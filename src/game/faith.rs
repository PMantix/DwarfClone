//! The gods in the fortress (MYT-10, MYT-11): every dwarf has a patron
//! among their people's gods, devout or not; temples are dedicated to the
//! god most of the fortress worships; dwarves at prayer pray to their own
//! god by name, and each festival is kept in some god's honour.

use super::festivals::FestivalKind;
use super::overseer::{Overseer, RoomUse};
use super::personality::Trait;
use super::*;
use crate::worldgen::myth::{God, Sphere};

impl Game {
    /// The gods the fortress's people worship: its civilization's, else
    /// every dwarven god, else any.
    pub fn fort_gods(&self) -> Vec<u32> {
        let ow = &self.overworld;
        let mine: Vec<u32> = ow.gods.iter().filter(|g| self.civ.map_or(false, |c| g.civs.contains(&c))).map(|g| g.id).collect();
        if !mine.is_empty() {
            return mine;
        }
        let dwarven: Vec<u32> = ow
            .gods
            .iter()
            .filter(|g| g.civs.iter().any(|&c| ow.civs.get(c as usize).map_or(false, |c| c.race == Race::Dwarf)))
            .map(|g| g.id)
            .collect();
        if !dwarven.is_empty() {
            return dwarven;
        }
        ow.gods.iter().map(|g| g.id).collect()
    }

    /// A dwarf's patron god and how devout they are.
    pub fn patron_of(&self, dwarf: u32) -> Option<(&God, u8)> {
        let &(g, dev) = self.magic.patrons.get(&dwarf)?;
        Some((self.god(g)?, dev))
    }

    /// The god most of the living worship (the first of equals).
    pub fn chief_god(&self) -> Option<u32> {
        let mut count: BTreeMap<u32, u32> = BTreeMap::new();
        for d in self.dwarves.iter().filter(|d| d.alive) {
            if let Some(&(g, dev)) = self.magic.patrons.get(&d.id) {
                *count.entry(g).or_insert(0) += dev as u32;
            }
        }
        count.into_iter().max_by_key(|&(g, n)| (n, std::cmp::Reverse(g))).map(|(g, _)| g).or_else(|| self.fort_gods().first().copied())
    }

    /// Daily: newcomers find their god, and (without an overseer to see to
    /// it) new temples are dedicated.
    pub(super) fn faith_daily(&mut self) {
        if self.overworld.gods.is_empty() {
            // Older worlds: their gods are found now.
            if self.overworld.civs.is_empty() {
                return;
            }
            self.overworld.gods = crate::worldgen::myth::pantheon(&self.overworld);
        }
        let gods = self.fort_gods();
        if gods.is_empty() {
            return;
        }
        let need: Vec<u32> = self.dwarves.iter().filter(|d| d.alive && !d.ghost && !self.magic.patrons.contains_key(&d.id)).map(|d| d.id).collect();
        for id in need {
            let mut r = self.mrng();
            // The first gods of a people are the most worshipped.
            let n = gods.len() as i32;
            let g = gods[r.range(0, n).min(r.range(0, n)) as usize];
            let dev = match r.range(0, 10) {
                0..=4 => 1,
                5..=8 => 2,
                _ => 3,
            };
            self.magic.patrons.insert(id, (g, dev));
        }
        let zones = &self.zones;
        self.magic.shrines.retain(|z, _| zones.get(z).map_or(false, |z| z.kind == ZoneKind::Temple));
        if self.overseer.is_none() {
            self.dedicate_temples();
        }
    }

    /// Each temple not yet dedicated is given to the god most of the
    /// fortress worships. The gods' names, one per temple.
    pub(super) fn dedicate_temples(&mut self) -> Vec<String> {
        let temples: Vec<u32> = self.zones.values().filter(|z| z.kind == ZoneKind::Temple && !self.magic.shrines.contains_key(&z.id)).map(|z| z.id).collect();
        let mut said = Vec::new();
        for z in temples {
            let Some(gid) = self.chief_god() else { break };
            let Some(god) = self.god(gid).cloned() else { break };
            self.magic.shrines.insert(z, gid);
            let fort = self.fortress_name.clone();
            self.msg(&format!("A temple of {} has been dedicated to {}, {}.", fort, god.called(), god.describe()), (255, 225, 150));
            if self.magic.shrines.values().filter(|&&g| g == gid).count() == 1 {
                self.legend(format!("The dwarves of {} dedicated a temple to {}, {}.", fort, god.called(), god.describe()));
            }
            // The faithful are glad of it.
            let tick = self.tick;
            for d in self.dwarves.iter_mut().filter(|d| d.alive) {
                if let Some(&(g, dev)) = self.magic.patrons.get(&d.id) {
                    if g == gid && dev >= 2 {
                        d.think(&format!("is glad to see a temple to {}", god.called()), -8 * dev as i32, tick);
                    }
                }
            }
            said.push(god.called());
        }
        said
    }

    /// Daily for the overseer: it dedicates new temples, and puts a temple
    /// into the plan when many of its dwarves are devout.
    pub(super) fn ov_faith(&mut self, o: &mut Overseer) {
        let tick = self.tick;
        for g in self.dedicate_temples() {
            let why = if o.has(Trait::Pious) { "may the god look kindly on us" } else { "whom most of our dwarves pray to" };
            Self::note_ov(o, tick, format!("Dedicated the new temple to {}, {}.", g, why));
        }
        if tick < SEASON
            || o.wants.contains(&RoomUse::Temple)
            || o.blocked_lately(RoomUse::Temple, tick)
            || o.rooms.iter().any(|r| r.kind == RoomUse::Temple)
            || self.zones.values().any(|z| z.kind == ZoneKind::Temple)
        {
            return;
        }
        let pop = self.alive_count().max(1);
        let devout = self.dwarves.iter().filter(|d| d.alive && self.magic.patrons.get(&d.id).map_or(false, |p| p.1 >= 2)).count();
        let enough = if o.has(Trait::Pious) { devout * 5 >= pop } else { devout * 2 >= pop };
        if !enough || devout < 3 {
            return;
        }
        let chief = self.chief_god().and_then(|g| self.god(g)).map_or("their gods".to_string(), |g| g.called());
        o.wants.push(RoomUse::Temple);
        Self::note_ov(o, tick, format!("{} of our dwarves are devout, most of them followers of {}: a temple goes into the plan.", devout, chief));
    }

    /// A dwarf at prayer in a temple: once a day, whom they pray to.
    pub(super) fn at_prayer(&mut self, d: &mut Dwarf) {
        self.atone(d);
        let tick = self.tick;
        if d.thoughts.iter().any(|t| t.text.starts_with("prayed to ") && tick < t.tick + DAY) {
            return;
        }
        let Some(&(gid, dev)) = self.magic.patrons.get(&d.id) else { return };
        let Some(god) = self.god(gid) else { return };
        let called = god.called();
        let shrine = self.world.get(d.pos).map(|t| t.zone).and_then(|z| self.magic.shrines.get(&z)).copied();
        let (text, s) = if shrine == Some(gid) {
            (format!("prayed to {} in {} own temple", called, if god.female { "her" } else { "his" }), -6 - 3 * dev as i32)
        } else {
            (format!("prayed to {}", called), -3 - 2 * dev as i32)
        };
        d.think(&text, s, tick);
    }

    /// The god a festival is kept for: one of the right sphere, else the
    /// fortress's chief god.
    pub(super) fn festival_god(&self, kind: FestivalKind) -> Option<u32> {
        let want: &[Sphere] = match kind {
            FestivalKind::Harvest => &[Sphere::Harvest, Sphere::Fortune],
            FestivalKind::Midwinter => &[Sphere::Fire, Sphere::Night],
            FestivalKind::Founding => &[Sphere::Stone, Sphere::Craft, Sphere::Deep],
            FestivalKind::Dedication => &[Sphere::Craft, Sphere::Stone],
            FestivalKind::Wedding => &[Sphere::Love, Sphere::Fortune],
            FestivalKind::Victory => &[Sphere::War, Sphere::Death],
        };
        let gods = self.fort_gods();
        for s in want {
            if let Some(&g) = gods.iter().find(|&&g| self.god(g).map_or(false, |god| god.has(*s))) {
                return Some(g);
            }
        }
        self.chief_god()
    }

    /// A festival begins: whose it is.
    pub(super) fn festival_blessing(&mut self, kind: FestivalKind) {
        let Some(god) = self.festival_god(kind).and_then(|g| self.god(g)).cloned() else { return };
        self.msg(&format!("The {} is kept in honour of {}, {}.", kind.name(), god.called(), god.describe()), (255, 215, 60));
        if let Some(last) = self.year_log.festivals.last_mut() {
            if last == kind.name() {
                *last = format!("{} of {}", kind.name(), god.name);
            }
        }
    }

    /// For the look panel: whom a dwarf worships, and how.
    pub fn faith_line(&self, dwarf: u32) -> Option<String> {
        let (god, dev) = self.patron_of(dwarf)?;
        let how = match dev {
            1 => "now and then",
            2 => "devoutly",
            _ => "zealously",
        };
        Some(format!("Worships {} ({}) {}.", god.called(), god.spheres_text(), how))
    }

    /// For the look panel: whose temple this is.
    pub fn shrine_line(&self, p: Pos) -> Option<String> {
        let z = self.world.get(p)?.zone;
        let god = self.god(*self.magic.shrines.get(&z)?)?;
        Some(format!("Temple of {}, {}; forbids {}.", god.called(), god.describe(), god.taboo.name()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temple(g: &mut Game) -> u32 {
        let e = g.embark;
        assert!(g.add_zone(e.offset(-3, -3, 0), e.offset(3, 3, 0), ZoneKind::Temple) > 0);
        g.zones.values().find(|z| z.kind == ZoneKind::Temple).unwrap().id
    }

    #[test]
    fn dwarves_worship_their_peoples_gods_and_a_temple_is_dedicated() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let gods = g.fort_gods();
        assert!(gods.len() >= 3, "a dwarven people has a few gods");
        let civ = g.civ.unwrap();
        assert!(gods.iter().all(|&id| g.god(id).unwrap().civs.contains(&civ)));
        g.faith_daily();
        for d in g.dwarves.iter().filter(|d| d.alive) {
            let (god, dev) = g.patron_of(d.id).expect("a dwarf without a god");
            assert!(gods.contains(&god.id) && (1..=3).contains(&dev));
            assert!(g.faith_line(d.id).unwrap().starts_with(&format!("Worships {}", god.called())));
        }
        let z = temple(&mut g);
        g.faith_daily();
        let chief = g.chief_god().unwrap();
        assert_eq!(g.magic.shrines.get(&z), Some(&chief));
        let god = g.god(chief).unwrap().clone();
        assert!(g.log.iter().any(|l| l.text.contains("has been dedicated to") && l.text.contains(&god.called())));
        assert!(g.overworld.events.iter().any(|e| e.text.contains("dedicated a temple to")));
        let line = g.shrine_line(g.zones[&z].tiles[0]).unwrap();
        assert!(line.contains(&god.called()) && line.contains(god.taboo.name()), "{}", line);
        // A temple dug out again is forgotten.
        g.zones.remove(&z);
        g.faith_daily();
        assert!(g.magic.shrines.is_empty());
    }

    #[test]
    fn a_dwarf_at_prayer_prays_to_their_god_by_name() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        g.faith_daily();
        let z = temple(&mut g);
        g.faith_daily();
        let shrine = g.magic.shrines[&z];
        let tile = g.zones[&z].tiles[0];
        let mut d = std::mem::take(&mut g.dwarves[0]);
        d.pos = tile;
        d.task = Task::Idle;
        d.asleep = false;
        d.needs = vec![crate::dwarf::Need { kind: crate::dwarf::NeedKind::Pray, strength: 3, level: 0, low: true }];
        g.needs_tick(&mut d);
        let (god, _) = g.patron_of(d.id).unwrap();
        let t = d.thoughts.iter().find(|t| t.text.starts_with("prayed to ")).expect("no prayer");
        assert!(t.text.contains(&god.called()), "{}", t.text);
        assert_eq!(t.text.contains("own temple"), god.id == shrine);
        // Once a day is enough.
        g.needs_tick(&mut d);
        assert_eq!(d.thoughts.iter().filter(|t| t.text.starts_with("prayed to ")).count(), 1);
        g.dwarves[0] = d;
    }

    #[test]
    fn festivals_are_kept_for_a_god() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        g.faith_daily();
        g.start_festival(FestivalKind::Harvest);
        let god = g.god(g.festival_god(FestivalKind::Harvest).unwrap()).unwrap().clone();
        assert!(g.log.iter().any(|l| l.text.starts_with("The harvest feast is kept in honour of") && l.text.contains(&god.called())));
        assert_eq!(g.year_log.festivals, vec![format!("harvest feast of {}", god.name)]);
    }

    #[test]
    fn a_devout_fort_gets_a_temple_from_the_overseer() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        g.set_overseer(Some(Style::Steady));
        g.faith_daily();
        for p in g.magic.patrons.values_mut() {
            p.1 = 3;
        }
        g.tick = SEASON + 5;
        let mut o = g.overseer.take().unwrap();
        g.ov_faith(&mut o);
        assert!(o.wants.contains(&RoomUse::Temple), "no temple wanted");
        assert!(o.notes.iter().any(|n| n.1.contains("a temple goes into the plan")));
        // Once there is one, it is dedicated and noted.
        temple(&mut g);
        g.ov_faith(&mut o);
        assert!(o.notes.iter().any(|n| n.1.starts_with("Dedicated the new temple to")));
        g.overseer = Some(o);
    }
}
