//! The necromancer: a dark figure in a tower near some sites, who now and
//! then raises the dead against the fortress, its own unburied dead first.
//! Destroy the necromancer and the dead lie still for good.

use super::*;
use crate::creature::Species;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Necromancer {
    pub name: String,
    pub tower: String,
    /// The fortress has learned of them (after their first raid).
    pub known: bool,
    pub alive: bool,
}

const TOWERS: [&str; 6] = ["the Bone Spire", "the Pale Tower", "the Ashen Keep", "the Hollow Tower", "the Black Steeple", "the Tower of Whispers"];

impl Game {
    /// At embark (or when an older save loads): some sites have a
    /// necromancer for a neighbour.
    pub(super) fn place_necromancer(&mut self) {
        self.necro_rolled = true;
        if !self.rng.chance(0.4) {
            return;
        }
        let name = crate::overworld::person_name(&mut self.rng, Race::Human);
        let tower = self.rng.pick(&TOWERS).to_string();
        self.necromancer = Some(Necromancer { name, tower, known: false, alive: true });
    }

    /// Each season after the first year: the necromancer may raise the dead,
    /// the more readily the more of ours lie unburied.
    pub(super) fn maybe_undead(&mut self) {
        let Some(n) = self.necromancer.as_ref() else { return };
        if !n.alive || self.tick < YEAR {
            return;
        }
        let unburied = self.items.values().filter(|it| it.free() && self.burial_corpse(it)).count();
        if self.rng.chance((0.2 + 0.1 * unburied as f64).min(0.8)) {
            self.raise_dead();
        }
    }

    /// The dead walk: our own unburied dead where they lie, and a horde
    /// from the necromancer's tower at the edge of the map.
    pub(super) fn raise_dead(&mut self) {
        let Some(n) = self.necromancer.clone() else { return };
        let tick = self.tick;
        let corpses: Vec<(u32, Pos, String)> = self
            .items
            .values()
            .filter(|it| it.free() && self.burial_corpse(it))
            .map(|it| (it.id, it.pos, it.label.clone().unwrap_or_default()))
            .collect();
        for (item, pos, name) in &corpses {
            self.items.remove(item);
            let id = self.spawn_creature(Species::Undead, *pos, u64::MAX);
            let c = self.creatures.get_mut(&id).unwrap();
            c.siege = true;
            c.label = Some(format!("the corpse of {}", name));
            c.desc = Some(format!("risen:{}", name));
            let dead = self.dwarves.iter().find(|d| &d.name == name).map(|d| d.id);
            for o in self.dwarves.iter_mut().filter(|o| o.alive) {
                if dead.map_or(false, |dd| o.regard(dd) >= 30 || o.family_of(dd) || o.spouse == Some(dd)) {
                    o.think(&format!("saw {} walk again", name), 30, tick);
                }
            }
        }
        let horde = 3 + self.alive_count() / 5;
        if let Some(entry) = self.edge_entry() {
            for _ in 0..horde {
                let p = self.spot_near(entry, 3);
                let id = self.spawn_creature(Species::Undead, p, u64::MAX);
                self.creatures.get_mut(&id).unwrap().siege = true;
            }
            // Sometimes the necromancer comes to see the work done.
            if !n.known || self.rng.chance(0.5) {
                let p = self.spot_near(entry, 2);
                let id = self.spawn_creature(Species::Undead, p, u64::MAX);
                let c = self.creatures.get_mut(&id).unwrap();
                c.siege = true;
                c.general = Some(u32::MAX);
                c.label = Some(format!("{} the necromancer", n.name));
                c.desc = Some(format!("master of {}", n.tower));
                c.body = crate::body::Body::new(1.3);
            }
        }
        self.necromancer.as_mut().unwrap().known = true;
        let fort = self.fortress_name.clone();
        let ours = if corpses.is_empty() { String::new() } else { format!(" {} of our own dead rise where they lay!", corpses.len()) };
        self.msg(&format!("{} of {} raises the dead against {}!{}", n.name, n.tower, fort, ours), (190, 120, 230));
        self.legend(format!("{} the necromancer raised the dead against {}.", n.name, fort));
        self.year_log.undead = true;
        self.pause_request = true;
    }

    /// One of the walking dead is destroyed: our own dead fall back as
    /// corpses to be buried, the rest crumble. The necromancer's end lays
    /// them all to rest.
    pub(super) fn undead_destroyed(&mut self, c: &crate::creature::Creature) {
        if let Some(name) = c.desc.as_deref().and_then(|d| d.strip_prefix("risen:")) {
            let id = self.spawn_item(ItemKind::Corpse, Mat::Flesh, c.pos);
            if let Some(it) = self.items.get_mut(&id) {
                it.label = Some(name.to_string());
            }
        }
        if c.general.is_none() {
            return;
        }
        if let Some(n) = self.necromancer.as_mut() {
            n.alive = false;
        }
        self.milestone("necromancer", "The necromancer destroyed");
        let fort = self.fortress_name.clone();
        let label = c.title();
        self.msg(&format!("{} is destroyed! The dead fall still.", label), (255, 215, 60));
        self.legend(format!("{} was destroyed at {}, and the dead fell still.", label, fort));
        let ids: Vec<u32> = self.creatures.values().filter(|o| o.species == Species::Undead).map(|o| o.id).collect();
        for id in ids {
            if let Some(mut o) = self.creatures.remove(&id) {
                o.general = None;
                self.undead_destroyed(&o);
            }
        }
        let tick = self.tick;
        for d in self.dwarves.iter_mut().filter(|d| d.alive) {
            d.think("rejoiced that the dead lie still", -20, tick);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_unburied_dead_rise_and_fall_with_their_master() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        g.necromancer = Some(Necromancer { name: "Morvane Ash".into(), tower: "the Pale Tower".into(), known: false, alive: true });
        // One of ours dies and lies unburied.
        let mut d = std::mem::take(&mut g.dwarves[1]);
        let name = d.name.clone();
        g.kill(&mut d, "was crushed");
        g.dwarves[1] = d;
        assert!(g.items.values().any(|it| it.kind == ItemKind::Corpse && it.label.as_deref() == Some(name.as_str())));
        g.raise_dead();
        assert!(!g.items.values().any(|it| it.kind == ItemKind::Corpse && it.label.as_deref() == Some(name.as_str())), "the corpse should have risen");
        let risen = g.creatures.values().find(|c| c.label.as_deref() == Some(format!("the corpse of {}", name).as_str())).expect("no risen corpse").id;
        let master = g.creatures.values().find(|c| c.general.is_some()).expect("the necromancer should come").id;
        assert!(g.creatures.values().filter(|c| c.species == Species::Undead).count() >= 5);
        assert!(g.log.iter().any(|l| l.text.contains("raises the dead")));
        // Strike down the risen one: the corpse is back, to be buried.
        let c = g.creatures.remove(&risen).unwrap();
        g.creature_died(c);
        assert!(g.items.values().any(|it| it.kind == ItemKind::Corpse && it.label.as_deref() == Some(name.as_str())));
        // Destroy the master: they all fall.
        let mut killer = std::mem::take(&mut g.dwarves[0]);
        let c = g.creatures.remove(&master).unwrap();
        g.credit_kill(&mut killer, &c);
        g.creature_died(c);
        g.dwarves[0] = killer;
        assert!(!g.creatures.values().any(|c| c.species == Species::Undead), "the dead should fall still");
        assert!(!g.necromancer.as_ref().unwrap().alive);
        assert_eq!(g.dwarves[0].epithet.as_deref(), Some("Deathbane"));
        g.maybe_undead();
        assert!(!g.creatures.values().any(|c| c.species == Species::Undead), "no more raids");
    }
}
