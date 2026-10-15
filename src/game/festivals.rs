//! Festivals: a few days a year the fortress downs tools to feast, dance,
//! arm-wrestle and see who can drink the most.

use super::*;
use crate::dwarf::NeedKind;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FestivalKind {
    /// Early spring, from the second year: the day the fortress was founded.
    Founding,
    /// Mid-autumn, when the harvest is in.
    Harvest,
    /// Midwinter: a fire against the dark.
    Midwinter,
    /// A great monument is finished.
    Dedication,
}

impl FestivalKind {
    pub fn name(self) -> &'static str {
        match self {
            FestivalKind::Founding => "founding day",
            FestivalKind::Harvest => "harvest feast",
            FestivalKind::Midwinter => "midwinter fire",
            FestivalKind::Dedication => "dedication feast",
        }
    }

    /// Which festival falls on this day of the year (season, day of season), if any.
    pub fn on(season: u64, day: u64, year: u64) -> Option<FestivalKind> {
        match (season, day) {
            (0, 6) if year > 0 => Some(FestivalKind::Founding),
            (2, 13) => Some(FestivalKind::Harvest),
            (3, 13) => Some(FestivalKind::Midwinter),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Festival {
    pub kind: FestivalKind,
    pub until: u64,
    /// Where it's held.
    pub venue: Vec<Pos>,
    /// Who has turned up.
    pub attended: Vec<u32>,
    /// Mugs downed, per dwarf.
    pub mugs: Vec<(u32, u32)>,
    /// Contests announced so far (to keep the log readable).
    pub told: u32,
}

/// How long a festival lasts.
pub const FESTIVAL_LEN: u64 = DAY / 2;

impl Game {
    /// Each morning: start a festival if today is one.
    pub(super) fn festivals_daily(&mut self) {
        let day_of_year = (self.tick % YEAR) / DAY;
        let season = day_of_year / (SEASON / DAY);
        let day = day_of_year % (SEASON / DAY);
        if let Some(kind) = FestivalKind::on(season, day, self.tick / YEAR) {
            self.start_festival(kind);
        }
    }

    pub(super) fn start_festival(&mut self, kind: FestivalKind) {
        if self.festival.is_some() || self.alive_count() < 2 {
            return;
        }
        // The meeting hall or tavern, else a dining hall, else the open
        // ground by the wagon.
        let mut venue = self.meeting_tiles();
        if venue.is_empty() {
            venue = self
                .zones
                .values()
                .filter(|z| z.kind == ZoneKind::Dining)
                .flat_map(|z| z.tiles.iter().copied())
                .filter(|&p| self.world.walkable(p))
                .collect();
        }
        if venue.is_empty() {
            let e = self.embark;
            venue = (-3..=3)
                .flat_map(|dy| (-3..=3).map(move |dx| (dx, dy)))
                .map(|(dx, dy)| {
                    let p = e.offset(dx, dy, 0);
                    Pos::new(p.x, p.y, self.world.surface_z(p.x, p.y))
                })
                .filter(|&p| self.world.walkable(p))
                .collect();
        }
        if venue.is_empty() {
            return;
        }
        let until = self.tick + FESTIVAL_LEN;
        let alert = self.alert;
        for i in 0..self.dwarves.len() {
            let d = &self.dwarves[i];
            if !d.alive || d.insane || d.tantrum > 0 || d.mood.is_some() || d.jailed_until > 0 || (d.soldier && alert) {
                continue;
            }
            let mut d = std::mem::take(&mut self.dwarves[i]);
            if !d.task.is_need() && d.task != Task::Idle {
                self.release_task(&mut d);
            }
            d.break_until = until;
            self.dwarves[i] = d;
        }
        let text = match kind {
            FestivalKind::Founding => format!("It is founding day! {} remembers the day it was struck from the rock.", self.fortress_name),
            FestivalKind::Harvest => "The harvest is in! The fortress gathers for the harvest feast.".to_string(),
            FestivalKind::Midwinter => "Midwinter! The fortress gathers round the fire against the long dark.".to_string(),
            FestivalKind::Dedication => "The fortress gathers to dedicate its great monument!".to_string(),
        };
        self.msg(&text, (255, 215, 60));
        self.festival = Some(Festival { kind, until, venue, attended: Vec::new(), mugs: Vec::new(), told: 0 });
        self.year_log.festivals.push(kind.name().to_string());
        self.milestone("festival", "The fortress's first festival");
    }

    /// A dwarf at a festival: get there, then dance, drink and compete.
    /// False when there's no festival.
    pub(super) fn festival_activity(&mut self, d: &mut Dwarf) -> bool {
        let Some(f) = self.festival.as_ref() else { return false };
        if self.tick >= f.until || self.tick >= d.break_until {
            return false;
        }
        let kind = f.kind;
        let here = f.venue.contains(&d.pos);
        let pick = |g: &mut Game| {
            let v = &g.festival.as_ref().unwrap().venue;
            v[g.rng.range(0, v.len() as i32) as usize]
        };
        if !here {
            let q = pick(self);
            d.task = Task::Wander { target: q };
            return true;
        }
        let f = self.festival.as_mut().unwrap();
        if !f.attended.contains(&d.id) {
            f.attended.push(d.id);
        }
        let roll = self.rng.range(0, 100);
        if roll < 30 {
            // Dance: a few steps across the floor.
            let q = pick(self);
            if q != d.pos {
                d.task = Task::Wander { target: q };
            }
        } else if roll < 40 && !d.is_child() {
            self.festival_drink(d, kind);
        } else if roll < 43 && !d.is_child() {
            self.arm_wrestle(d, kind);
        }
        true
    }

    /// A mug from the stores, if there's any left.
    fn festival_drink(&mut self, d: &mut Dwarf, kind: FestivalKind) {
        let Some(id) = self.items.values().find(|it| it.kind == ItemKind::Drink && it.free()).map(|it| it.id) else { return };
        self.items.remove(&id);
        d.thirst = 0;
        d.satisfy(NeedKind::Drink, 400);
        let tick = self.tick;
        d.think(&format!("drank deep at the {}", kind.name()), -3, tick);
        let f = self.festival.as_mut().unwrap();
        match f.mugs.iter_mut().find(|(id, _)| *id == d.id) {
            Some(m) => m.1 += 1,
            None => f.mugs.push((d.id, 1)),
        }
    }

    /// Challenge someone nearby; the stronger arm usually wins.
    fn arm_wrestle(&mut self, d: &mut Dwarf, kind: FestivalKind) {
        let Some(j) = self.dwarves.iter().position(|o| {
            o.alive && !o.asleep && !o.is_child() && o.id != d.id && (o.pos.x - d.pos.x).abs() <= 2 && (o.pos.y - d.pos.y).abs() <= 2 && o.pos.z == d.pos.z
        }) else {
            return;
        };
        let tick = self.tick;
        let mine = crate::dwarf::skill_level(d.fight_xp) as i32 * 3 + self.rng.range(0, 10);
        let theirs = crate::dwarf::skill_level(self.dwarves[j].fight_xp) as i32 * 3 + self.rng.range(0, 10);
        let other = self.dwarves[j].name.clone();
        let (winner, loser) = if mine >= theirs {
            d.fight_xp += 20;
            d.think(&format!("won an arm-wrestling bout at the {}", kind.name()), -8, tick);
            self.dwarves[j].think(&format!("lost an arm-wrestling bout to {}", d.name), 1, tick);
            self.festival_feat(d, true);
            (d.name.clone(), other)
        } else {
            let mut o = std::mem::take(&mut self.dwarves[j]);
            o.fight_xp += 20;
            o.think(&format!("won an arm-wrestling bout at the {}", kind.name()), -8, tick);
            d.think(&format!("lost an arm-wrestling bout to {}", other), 1, tick);
            self.festival_feat(&mut o, true);
            self.dwarves[j] = o;
            (other, d.name.clone())
        };
        let f = self.festival.as_mut().unwrap();
        if f.told < 3 {
            f.told += 1;
            self.msg(&format!("{} beat {} at arm-wrestling!", winner, loser), (230, 200, 120));
        }
    }

    /// Every so often: end a festival that's run its course.
    pub(super) fn festival_tick(&mut self) {
        let Some(f) = self.festival.as_ref() else { return };
        if self.tick < f.until {
            return;
        }
        let f = self.festival.take().unwrap();
        let tick = self.tick;
        let name = f.kind.name();
        for d in self.dwarves.iter_mut().filter(|d| d.alive && f.attended.contains(&d.id)) {
            d.think(&format!("enjoyed the {}", name), -25, tick);
            d.satisfy(NeedKind::Party, 1000);
            d.satisfy(NeedKind::Socialize, 400);
        }
        self.msg(&format!("The {} is over. {} dwarves made merry.", name, f.attended.len()), (255, 215, 60));
        if let Some(&(id, n)) = f.mugs.iter().max_by_key(|(id, n)| (*n, u32::MAX - id)) {
            if n >= 2 {
                if let Some(i) = self.dwarves.iter().position(|d| d.id == id) {
                    let mut d = std::mem::take(&mut self.dwarves[i]);
                    d.think(&format!("won the drinking contest at the {}", name), -12, tick);
                    let who = d.name.clone();
                    self.msg(&format!("{} won the drinking contest, downing {} mugs!", who, n), (230, 200, 120));
                    self.festival_feat(&mut d, false);
                    self.dwarves[i] = d;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_harvest_feast_brings_everyone_together() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let e = g.embark;
        for _ in 0..30 {
            g.spawn_item(ItemKind::Drink, Mat::Stoutcap, e);
        }
        // The calendar knows its feasts.
        assert_eq!(FestivalKind::on(2, 13, 0), Some(FestivalKind::Harvest));
        assert_eq!(FestivalKind::on(0, 6, 0), None, "no founding day in the first year");
        assert_eq!(FestivalKind::on(0, 6, 1), Some(FestivalKind::Founding));
        let drinks = g.items.values().filter(|it| it.kind == ItemKind::Drink).count();
        g.start_festival(FestivalKind::Harvest);
        assert!(g.festival.is_some());
        assert!(g.log.iter().any(|l| l.text.contains("harvest feast")));
        let adults = g.dwarves.iter().filter(|d| d.alive).count();
        for _ in 0..FESTIVAL_LEN + 60 {
            g.step();
        }
        assert!(g.festival.is_none(), "it should be over");
        let merry = g.dwarves.iter().filter(|d| d.thoughts.iter().any(|t| t.text == "enjoyed the harvest feast")).count();
        assert!(merry * 2 >= adults, "only {} of {} came", merry, adults);
        assert!(g.items.values().filter(|it| it.kind == ItemKind::Drink).count() < drinks, "nobody drank");
        assert!(g.log.iter().any(|l| l.text.contains("is over")));
        for l in g.log.iter().rev().take(8) {
            println!("{}", l.text);
        }
    }
}
