//! A made people at the fort's door (`overworld/peoples.rs` makes them in
//! history). The warlike come as besiegers: their own kind, in their own
//! colour and letter on the map, each with their maker's gift (webs, a
//! roar, a hide like armour...), led by one of their own with a name in
//! their tongue. The rest, at peace with us, send traders in the dead of
//! winter when nobody else comes, with strange goods of their own.

use super::horrors::Power;
use super::*;
use crate::creature::{Creature, Species};
use crate::overworld::{MadeRace, Temper};

impl Game {
    fn made(&self, civ: u32) -> Option<&MadeRace> {
        self.overworld.made_of(civ)
    }

    /// What to call an army of `civ`'s ("vorlaks", or the species' plural).
    pub(super) fn army_plural(&self, civ: u32, species: Species) -> String {
        match self.made(civ) {
            Some(m) => m.plural.clone(),
            None => species.info().plural.to_string(),
        }
    }

    /// The species an army of `civ` is made of: a made people's own, else `base`.
    pub(super) fn army_kind(&self, civ: u32, base: Species) -> Species {
        if self.made(civ).is_some() {
            Species::Spawn
        } else {
            base
        }
    }

    /// The kind of figure that leads `civ`'s armies ("goblin", "vorlak").
    pub(super) fn leader_kind(&self, civ: u32, race: Race) -> String {
        match self.made(civ) {
            Some(m) => m.singular.clone(),
            None => format!("{:?}", race).to_lowercase(),
        }
    }

    /// A new name for one of `civ`'s, in their tongue.
    pub(super) fn leader_name(&mut self, civ: u32, race: Race) -> String {
        let Some(m) = self.made(civ).cloned() else { return crate::overworld::person_name(&mut self.rng, race) };
        format!("{} {}", m.word(&mut self.rng, 1), m.word(&mut self.rng, 2))
    }

    /// One of a made people's soldiers, just spawned: their own size and
    /// look, and for the leader and about a third of the rest (knob
    /// kin.gifted), their maker's gift: a whole army breathing fire or
    /// roaring would be too much for a young fort.
    pub(super) fn dress_kin(&mut self, id: u32, civ: u32, leads: bool) {
        let Some(m) = self.made(civ).cloned() else { return };
        let known = self.horrors.kin_known.iter().filter(|k| k.0 == civ).map(|k| k.1).collect::<Vec<Power>>();
        if let Some(c) = self.creatures.get_mut(&id) {
            c.body = crate::body::Body::new(m.size * if leads { 1.3 } else { 1.0 });
            if !leads {
                c.label = Some(format!("the {}", m.singular));
                c.desc = Some(format!("One of the {}: {}.", m.plural, m.body));
            } else {
                c.desc = Some(format!("{} of the {}, {}", if c.general.is_some() { "warleader" } else { "elder" }, m.plural, m.body));
            }
        }
        self.horrors.kin.insert(id, civ);
        let mut rng = self.horror_rng(id, 0x6B1D);
        if !leads && !rng.chance(self.knob("kin.gifted", 0.34)) {
            return;
        }
        self.gift_powers(id, vec![m.power], &mut rng);
        if let Some(g) = self.horrors.gifts.get_mut(&id) {
            g.known = known;
        }
    }

    /// "the vorlaks", when one of a made people shows its gift: the fort
    /// learns it of all of them.
    pub(super) fn kin_of(&self, id: u32) -> Option<(u32, String)> {
        let civ = *self.horrors.kin.get(&id)?;
        Some((civ, format!("every one of the {}", self.made(civ)?.plural)))
    }

    /// The whole people knows it now: every one of them on the map, and
    /// those that come later.
    pub(super) fn kin_learned(&mut self, civ: u32, p: Power) {
        if !self.horrors.kin_known.contains(&(civ, p)) {
            self.horrors.kin_known.push((civ, p));
        }
        let ids: Vec<u32> = self.horrors.kin.iter().filter(|(_, c)| **c == civ).map(|(id, _)| *id).collect();
        for id in ids {
            if let Some(g) = self.horrors.gifts.get_mut(&id) {
                if !g.known.contains(&p) {
                    g.known.push(p);
                }
            }
        }
    }

    /// Who fled when a made people's siege broke ("vorlaks"), if it was theirs.
    pub(super) fn routed_kin(&self) -> Option<String> {
        let civ = self.annals.sieges.last()?.civ?;
        Some(self.made(civ)?.plural.clone())
    }

    /// Their own letter and colour on the map.
    pub fn kin_look(&self, c: &Creature) -> Option<(char, (u8, u8, u8))> {
        let civ = self.horrors.kin.get(&c.id)?;
        let m = self.made(*civ)?;
        Some((m.glyph, m.color))
    }

    /// One of a made people has fallen: their body is named for them.
    pub(super) fn kin_died(&mut self, c: &Creature) {
        let Some(civ) = self.horrors.kin.remove(&c.id) else { return };
        let Some(word) = self.made(civ).map(|m| m.singular.clone()) else { return };
        let at = c.pos;
        for it in self.items.values_mut().filter(|it| it.pos == at && it.carried_by.is_none() && it.kind == ItemKind::Corpse && it.label.as_deref() == Some("spawn")) {
            it.label = Some(word.clone());
        }
    }

    /// In the dead of winter, when nobody else comes, a made people given
    /// to trade and at peace with us sends traders (the wary keep to
    /// themselves).
    pub(super) fn kin_caravan(&mut self, season: u64) {
        if season != 3 || self.caravan.is_some() {
            return;
        }
        let pick = self
            .neighbors
            .iter()
            .filter(|n| n.dist <= 30 && !self.at_war_with(n))
            .find(|n| self.made(n.civ).map_or(false, |m| m.temper == Temper::Mercantile))
            .cloned();
        let Some(n) = pick else { return };
        let m = self.made(n.civ).cloned().unwrap();
        let civ_name = self.overworld.civ(n.civ).name.clone();
        let Some(depot) = self.depot().map(|d| d.id) else {
            self.msg(&format!("A caravan of {} came out of the snow, but with no trade depot it moved on.", m.plural), (230, 200, 90));
            return;
        };
        let Some(entry) = self.edge_entry() else { return };
        let mut goods = self.caravan_goods(Race::Elf);
        // Strange goods of their own making.
        let strange = [(ItemKind::Craft, Mat::Bone, "idol"), (ItemKind::Craft, Mat::Obsidian, "charm"), (ItemKind::Leather, Mat::Leather, "hide")];
        for (k, mat, what) in strange {
            let id = self.new_id();
            goods.push(Item {
                id,
                kind: k,
                mat,
                pos: Pos::default(),
                carried_by: None,
                claimed_by: None,
                label: Some(format!("{} {}", m.singular, what)),
                container: None,
                quality: 3,
                to_depot: false,
                dump: false,
                wear: 0,
                forbidden: false,
            });
        }
        let mut merchants = Vec::new();
        for _ in 0..2 {
            let p = self.spot_near(entry, 1);
            let id = self.spawn_creature(Species::Merchant, p, DAY * 9);
            if let Some(c) = self.creatures.get_mut(&id) {
                c.label = Some(format!("a {} merchant", m.singular));
                c.desc = Some(format!("One of the {}: {}.", m.plural, m.body));
                c.body = crate::body::Body::new(m.size);
            }
            self.horrors.kin.insert(id, n.civ);
            merchants.push(id);
        }
        self.count("kin.caravans", 1);
        let text = format!("A caravan of the {} comes out of the snow: {}, {}, with strange goods to sell.", civ_name, crate::game::combat::cap(&m.plural), m.body);
        self.msg(&text, (240, 240, 160));
        self.spotlight("Strange traders", &text, Some(entry), (240, 240, 160));
        self.caravan = Some(Caravan { civ: n.civ, race: Race::Human, civ_name, goods, merchants, depot, arrived: false, leave_tick: self.tick + DAY * 9 });
        self.pause_request = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overworld::Neighbor;

    /// A fort with a made people of `temper` next door.
    pub(crate) fn with_kin(temper: Temper) -> (Game, Neighbor) {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let year = g.overworld.year;
        let mut r = Rng::new(9);
        let mut civ = None;
        for _ in 0..40 {
            if let Some(c) = g.overworld.make_people(&mut r, year) {
                civ = Some(c);
                break;
            }
        }
        let civ = civ.expect("no people made");
        let m = g.overworld.made.iter_mut().find(|m| m.civ == civ).unwrap();
        m.temper = temper;
        let race = if temper == Temper::Warlike { Race::Goblin } else { Race::Human };
        g.overworld.civs[civ as usize].race = race;
        g.overworld.civs[civ as usize].alive = true;
        if temper != Temper::Warlike {
            for c in g.overworld.civs.iter_mut() {
                c.enemies.retain(|&e| e != civ);
            }
            g.overworld.civs[civ as usize].enemies.clear();
        }
        let n = Neighbor { civ, race, dist: 8 };
        g.neighbors.retain(|n| n.race != Race::Goblin);
        g.neighbors.insert(0, n.clone());
        (g, n)
    }

    #[test]
    fn a_made_people_besieges_as_their_own_kind() {
        let (mut g, n) = with_kin(Temper::Warlike);
        let m = g.overworld.made_of(n.civ).unwrap().clone();
        assert!(g.at_war_with(&n));
        g.lay_siege_of(&n, 5, None);
        let army: Vec<&Creature> = g.creatures.values().filter(|c| c.siege).collect();
        assert_eq!(army.len(), 5);
        // The leader and some of the rest carry the gift, not all of them.
        let gifted: Vec<u32> = army.iter().filter(|c| g.horrors.gifts.contains_key(&c.id)).map(|c| c.id).collect();
        assert!(army.iter().filter(|c| c.general.is_some()).all(|c| gifted.contains(&c.id)));
        assert!(!gifted.is_empty() && gifted.len() < army.len(), "{} of {} gifted", gifted.len(), army.len());
        assert!(army.iter().all(|c| c.species == Species::Spawn));
        // Their own look and their maker's gift.
        for c in &army {
            assert_eq!(g.kin_look(c), Some((m.glyph, m.color)));
            if let Some(gift) = g.horrors.gifts.get(&c.id) {
                assert_eq!(gift.powers, vec![m.power]);
            }
        }
        assert!(army.iter().any(|c| c.label.as_deref() == Some(&format!("the {}", m.singular))));
        // Told by their own name.
        assert!(g.log.iter().any(|l| l.text.contains(&format!(" {},", m.plural))), "{:?}", g.log.iter().rev().take(3).map(|l| &l.text).collect::<Vec<_>>());
        assert_eq!(g.annals.sieges.last().unwrap().foe, m.plural);
        // When one shows the gift, the fort knows it of them all.
        let ids: Vec<u32> = army.iter().map(|c| c.id).collect();
        let first = gifted[0];
        let title = g.kin_of(first).unwrap().1;
        g.learn_power(first, &title, m.power);
        assert!(gifted.iter().all(|id| g.horrors.gifts[id].known.contains(&m.power)));
        assert!(g.overworld.events.iter().any(|e| e.text.contains(&format!("every one of the {}", m.plural))));
        // Those slain leave bodies with their name.
        let mut c = g.creatures.remove(&ids[if ids[0] == first { 1 } else { 0 }]).unwrap();
        c.body.dead = true;
        g.creature_died(c);
        assert!(g.items.values().any(|it| it.kind == ItemKind::Corpse && it.label.as_deref() == Some(m.singular.as_str())));
        // The next army comes already known.
        g.lay_siege_of(&n, 2, None);
        let fresh = g.creatures.values().filter(|c| c.siege && !ids.contains(&c.id) && g.horrors.gifts.contains_key(&c.id)).map(|c| c.id).next().unwrap();
        assert!(g.horrors.gifts[&fresh].known.contains(&m.power));
    }

    #[test]
    fn a_peaceful_made_people_trades_in_winter() {
        let (mut g, n) = with_kin(Temper::Mercantile);
        let m = g.overworld.made_of(n.civ).unwrap().clone();
        // No depot: they move on.
        g.kin_caravan(3);
        assert!(g.caravan.is_none());
        assert!(g.log.iter().any(|l| l.text.contains("came out of the snow")));
        let e = g.embark;
        let mut depot = None;
        'p: for r in 3..25 {
            for dy in -r..=r {
                for dx in -r..=r {
                    let p = e.offset(dx, dy, 0);
                    if g.place_building(BuildingKind::Depot, p) {
                        depot = Some(g.world.t(p).building);
                        break 'p;
                    }
                }
            }
        }
        let depot = depot.expect("no room for a depot");
        g.buildings.get_mut(&depot).unwrap().built = true;
        // Not in summer: that's the humans' season.
        g.kin_caravan(1);
        assert!(g.caravan.is_none());
        g.kin_caravan(3);
        let car = g.caravan.as_ref().expect("no caravan in winter");
        assert_eq!(car.civ, n.civ);
        assert!(car.goods.iter().any(|it| it.label.as_deref().map_or(false, |l| l.starts_with(&m.singular))));
        let merchants = car.merchants.clone();
        assert!(merchants.iter().all(|id| g.creatures[id].label.as_deref().unwrap().contains(&m.singular)));
        assert!(g.spotlights.iter().any(|s| s.title == "Strange traders"));
        // A warlike people never trades, nor does a wary one.
        for t in [Temper::Warlike, Temper::Wary] {
            let (mut g2, _) = with_kin(t);
            g2.kin_caravan(3);
            assert!(g2.caravan.is_none(), "{:?} traded", t);
            assert!(!g2.log.iter().any(|l| l.text.contains("came out of the snow")));
        }
    }

    #[test]
    fn kin_who_leave_the_map_are_forgotten() {
        let (mut g, n) = with_kin(Temper::Warlike);
        g.lay_siege_of(&n, 3, None);
        let ids: Vec<u32> = g.creatures.values().filter(|c| c.siege).map(|c| c.id).collect();
        assert!(ids.iter().all(|id| g.horrors.kin.contains_key(id)));
        // One flees off the map (gone without dying).
        g.creatures.remove(&ids[0]);
        g.horrors_tick();
        assert!(!g.horrors.kin.contains_key(&ids[0]));
        assert!(g.horrors.kin.contains_key(&ids[1]));
    }
}
