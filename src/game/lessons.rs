//! What a siege teaches the overseer (SIE-01). While an army is on the map
//! it keeps a watch: where dwarves fell, whether the enemy got inside,
//! when the drawbridge went up. When the siege ends it turns the watch into
//! lessons, and acts on them.

use super::*;
use serde::{Deserialize, Serialize};

/// What the overseer saw of the siege now on (or the last one, until it
/// has learned from it).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SiegeWatch {
    /// Which siege (index into `annals.sieges`).
    pub siege: usize,
    pub deaths_outside: u32,
    pub deaths_inside: u32,
    /// A besieger stood inside the shelter burrow.
    pub foes_inside: bool,
    pub bridge_up_at: Option<u64>,
    /// Where (and when) a besieger first came to the gate.
    pub first_contact: Option<Pos>,
    pub contact_at: u64,
    /// Where dwarves fell.
    pub kill_spots: Vec<Pos>,
    pub soldiers_lost: u32,
    /// The most soldiers under arms while it lasted.
    pub soldiers: u32,
    /// Of the dead, how many bled to death.
    pub bled: u32,
}

impl Game {
    /// Every 50 ticks while a siege is on: keep the watch.
    pub(super) fn watch_siege(&mut self) {
        let Some(i) = self.annals.sieges.iter().rposition(|s| s.ended.is_none()) else { return };
        let (top, base) = match self.overseer.as_ref() {
            Some(o) => (o.top.unwrap_or(self.embark), o.base.unwrap_or(self.embark)),
            None => (self.embark, self.embark),
        };
        let tick = self.tick;
        let soldiers = self.dwarves.iter().filter(|d| d.alive && d.soldier).count() as u32;
        let raised = self.buildings.values().any(|b| b.kind == BuildingKind::Bridge && b.raised);
        let besiegers: Vec<Pos> = self.creatures.values().filter(|c| c.alive && c.siege && !c.legendary() && c.hostile()).map(|c| c.pos).collect();
        let by = |p: Pos, q: Pos, r: i32| (p.x - q.x).abs() <= r && (p.y - q.y).abs() <= r && (p.z - q.z).abs() <= 1;
        let contact = besiegers.iter().copied().find(|&p| by(p, top, 10) || by(p, base, 12));
        let mask = self.burrows.iter().find(|b| Some(b.bit) == self.alert_burrow).map(|b| b.mask());
        let inside = mask.map_or(false, |m| besiegers.iter().any(|&p| self.world.get(p).map_or(false, |t| t.burrows & m != 0)));
        let w = self.watch_for(i);
        w.soldiers = w.soldiers.max(soldiers);
        if raised && w.bridge_up_at.is_none() {
            w.bridge_up_at = Some(tick);
        }
        if let (Some(p), None) = (contact, w.first_contact) {
            w.first_contact = Some(p);
            w.contact_at = tick;
        }
        w.foes_inside |= inside;
    }

    /// The watch on siege `i`, begun afresh if it was on another.
    fn watch_for(&mut self, i: usize) -> &mut SiegeWatch {
        if self.siege_watch.as_ref().map_or(true, |w| w.siege != i) {
            self.siege_watch = Some(SiegeWatch { siege: i, ..Default::default() });
        }
        self.siege_watch.as_mut().unwrap()
    }

    /// A dwarf died while siege `i` was on (from `note_death`).
    pub(super) fn watch_death(&mut self, i: usize, d: &Dwarf, cause: &str) {
        let outside = self.world.get(d.pos).map_or(false, |_| self.world.outside(d.pos));
        let w = self.watch_for(i);
        if outside {
            w.deaths_outside += 1;
        } else {
            w.deaths_inside += 1;
        }
        if w.kill_spots.len() < 24 {
            w.kill_spots.push(d.pos);
        }
        if d.soldier {
            w.soldiers_lost += 1;
        }
        if cause == "bled to death" {
            w.bled += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A siege on the map, with nothing else about.
    fn besieged(seed: u64) -> Game {
        let mut g = Game::new(seed);
        g.creatures.retain(|_, c| c.tame);
        let n = g.neighbors.iter().find(|n| g.at_war_with(n)).cloned().or_else(|| g.neighbors.first().cloned()).expect("a neighbour");
        let mut foe = n.clone();
        foe.race = crate::overworld::Race::Goblin;
        g.lay_siege(&foe);
        g
    }

    #[test]
    fn the_overseer_watches_a_siege() {
        let mut g = besieged(5);
        g.watch_siege();
        assert_eq!(g.siege_watch.as_ref().map(|w| w.siege), Some(0));
        // Two dwarves cut down out on the surface by the wagon.
        for k in 0..2 {
            assert!(g.world.outside(g.dwarves[k].pos), "the dwarves start under the sky");
            let mut d = std::mem::take(&mut g.dwarves[k]);
            g.kill_for_test(&mut d, "has been struck down");
            g.dwarves[k] = d;
        }
        g.watch_siege();
        let w = g.siege_watch.as_ref().unwrap();
        assert_eq!(w.deaths_outside, 2);
        assert_eq!(w.deaths_inside, 0);
        assert_eq!(w.kill_spots.len(), 2);
        assert_eq!(w.bled, 0);
        assert!(!w.foes_inside && w.bridge_up_at.is_none());
    }
}
