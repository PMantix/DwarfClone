//! What a siege teaches the overseer (SIE-01). While an army is on the map
//! it keeps a watch: where dwarves fell, whether the enemy got inside,
//! when the drawbridge went up. When the siege ends it turns the watch into
//! lessons, and acts on them.

use super::overseer::{Overseer, Style};
use super::*;
use crate::buildings::Recipe;
use serde::{Deserialize, Serialize};

/// What a siege can teach.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lesson {
    /// They got in: traps where they came, and where dwarves fell.
    TrapTheWay,
    /// One squad was not enough.
    SecondSquad,
    /// Dwarves died under the sky: wall the yard round the stair.
    WallTheYard,
    /// The bridge was down when they came: bar the gate sooner.
    BarSooner,
    /// The soldiers were beaten: better armour from the forge.
    ArmBetter,
    /// The wounded bled out: thread, cloth and a second doctor.
    MoreMedicine,
}

impl Lesson {
    pub fn name(self) -> &'static str {
        match self {
            Lesson::TrapTheWay => "trap the way in",
            Lesson::SecondSquad => "a second squad",
            Lesson::WallTheYard => "wall the yard",
            Lesson::BarSooner => "bar the gate sooner",
            Lesson::ArmBetter => "arm the soldiers better",
            Lesson::MoreMedicine => "more care for the wounded",
        }
    }
}

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

impl Game {
    /// The siege is over (`on_siege_end`): learn from the watch on it.
    pub(super) fn learn_from_siege(&mut self, i: usize) {
        let Some(w) = self.siege_watch.take() else { return };
        if w.siege != i || self.alive_count() == 0 {
            return;
        }
        let Some(mut o) = self.overseer.take() else { return };
        if o.base.is_some() {
            self.learn(&mut o, &w);
        }
        self.overseer = Some(o);
    }

    /// Turn a siege's watch into lessons, say them, and act on each.
    pub(super) fn learn(&mut self, o: &mut Overseer, w: &SiegeWatch) -> Vec<Lesson> {
        // (The reckless learn nothing.)
        if o.style == Style::Reckless {
            return Vec::new();
        }
        let slain = self.annals.sieges.get(w.siege).map_or(0, |s| s.foes_slain);
        let late = w.first_contact.is_some() && w.bridge_up_at.map_or(true, |t| t > w.contact_at);
        let mut learned: Vec<(Lesson, String)> = Vec::new();
        if w.foes_inside || late {
            let how = if w.foes_inside { "they got inside the halls" } else { "they came to the gate while the bridge was down" };
            learned.push((Lesson::TrapTheWay, format!("{}. Never again: traps in the corridor and where we fell.", how)));
            if o.did("linked") {
                learned.push((Lesson::BarSooner, "the bridge goes up sooner next time.".into()));
            }
        }
        if w.deaths_outside >= 2 {
            learned.push((Lesson::WallTheYard, format!("{} of ours died under the open sky. A wall round the yard, now.", w.deaths_outside)));
        }
        if (w.soldiers > 0 && w.soldiers_lost * 2 >= w.soldiers) || slain < 1 {
            let how = if w.soldiers_lost > 0 { format!("we lost {} of {} soldiers", w.soldiers_lost, w.soldiers) } else { "our soldiers slew none of them".to_string() };
            learned.push((Lesson::ArmBetter, format!("{}. Helms, breastplates and shields from the forge.", how)));
            if self.squads.len() < 2 {
                learned.push((Lesson::SecondSquad, "one squad is not enough.".into()));
            }
        }
        if w.bled >= 3 {
            learned.push((Lesson::MoreMedicine, format!("{} bled to death. Thread and cloth for the wounded, and a second doctor.", w.bled)));
        }
        let tick = self.tick;
        for (lesson, why) in &learned {
            o.lessons.push((tick, *lesson));
            Self::note_ov(o, tick, format!("After the siege, {}", why));
            self.apply_lesson(o, *lesson, w);
            self.count("lessons", 1);
        }
        if !learned.is_empty() {
            let names: Vec<&str> = learned.iter().map(|l| l.0.name()).collect();
            let text = format!("The overseer {} has learned from the siege: {}.", o.name, names.join(", "));
            self.msg(&text, (150, 210, 255));
            let fort = self.fortress_name.clone();
            for (lesson, _) in &learned {
                self.legend(format!("After a siege of {}, its overseer {} resolved to {}.", fort, o.name, lesson.name()));
            }
        }
        learned.into_iter().map(|l| l.0).collect()
    }

    /// Act on a lesson (again, if its work was lost).
    fn apply_lesson(&mut self, o: &mut Overseer, lesson: Lesson, w: &SiegeWatch) {
        let tick = self.tick;
        let base = o.base.unwrap();
        let until = tick + SEASON * 2;
        let boost = |o: &mut Overseer, r: Recipe, keep: u32| {
            o.boosts.retain(|b| b.0 != r);
            o.boosts.push((r, keep, until));
        };
        match lesson {
            Lesson::TrapTheWay => {
                self.set_stone_traps(o);
                // Weapon traps where dwarves fell near the gate, or else in
                // the corridor between the cage traps and the inner stair.
                let near = |p: Pos| (p.x - base.x).abs() <= 15 && (p.y - base.y).abs() <= 15 && (p.z - base.z).abs() <= 1;
                let mut spots: Vec<Pos> = w.kill_spots.iter().chain(w.first_contact.iter()).copied().filter(|&p| near(p)).collect();
                spots.extend([base.offset(8, 0, 0), base.offset(8, 1, 0), base.offset(14, 0, 0), base.offset(14, 1, 0)]);
                let mut n = 0;
                for p in spots {
                    if n >= 4 {
                        break;
                    }
                    let at = if self.can_place(BuildingKind::WeaponTrap, p) { Some(p) } else { self.ov_spot(BuildingKind::WeaponTrap, p, 1).filter(|&q| near(q)) };
                    if let Some(q) = at {
                        if self.place_building(BuildingKind::WeaponTrap, q) {
                            n += 1;
                        }
                    }
                }
                if n > 0 {
                    Self::note_ov(o, tick, format!("Set {} weapon traps where the enemy came in.", n));
                }
                boost(o, Recipe::Mechanism, 10);
            }
            Lesson::SecondSquad => {
                let adults = self.dwarves.iter().filter(|d| d.alive && !d.is_child()).count();
                if self.squads.len() >= 2 || adults < self.knob("lesson.squad_adults", 12.0) as usize {
                    return;
                }
                let want = self.squads.first().map_or(2, |s| s.members.len()).clamp(2, 4);
                let s = self.new_squad();
                let mut cands: Vec<(u32, u32)> = self
                    .dwarves
                    .iter()
                    .filter(|d| d.alive && !d.is_child() && !d.insane && d.noble.is_none() && self.position_of(d.id).is_none() && self.squad_of(d.id).is_none())
                    .map(|d| (d.fight_xp, d.id))
                    .collect();
                cands.sort_by(|a, b| b.cmp(a));
                for (_, id) in cands.into_iter().take(want) {
                    self.set_squad_member(s, id, true);
                }
                let name = self.squads[s].name.clone();
                Self::note_ov(o, tick, format!("Raised a second squad, {}, of {}.", name, self.squads[s].members.len()));
            }
            Lesson::WallTheYard => self.plan_yard(o),
            Lesson::BarSooner => {}
            Lesson::ArmBetter => {
                let n = (self.squads.iter().map(|s| s.members.len()).sum::<usize>() as u32).max(3);
                for r in [Recipe::ForgeHelm, Recipe::ForgeBreastplate, Recipe::ForgeShield, Recipe::LeatherHelm, Recipe::LeatherArmor] {
                    boost(o, r, n);
                }
            }
            Lesson::MoreMedicine => {
                boost(o, Recipe::SpinThread, 8);
                boost(o, Recipe::Weave, 6);
                let doctors = self.dwarves.iter().filter(|d| d.alive && !d.is_child() && d.has(Labor::Medicine)).count();
                if doctors < 2 {
                    let pick = self
                        .dwarves
                        .iter()
                        .filter(|d| d.alive && !d.is_child() && !d.soldier && !d.insane && !d.has(Labor::Medicine))
                        .max_by_key(|d| (d.skills[Labor::Medicine.idx()], d.id))
                        .map(|d| d.id);
                    if let Some(id) = pick {
                        self.set_labor(id, Labor::Medicine, true);
                    }
                }
            }
        }
    }

    /// A lesson asked for traps by the gate, and none stand there.
    pub(super) fn traps_wanted(&self, o: &Overseer) -> bool {
        let Some(base) = o.base else { return false };
        o.lessons.iter().any(|l| l.1 == Lesson::TrapTheWay)
            && !self.buildings.values().any(|b| {
                matches!(b.kind, BuildingKind::WeaponTrap | BuildingKind::StoneTrap) && (b.pos.x - base.x).abs() <= 15 && (b.pos.y - base.y).abs() <= 15 && (b.pos.z - base.z).abs() <= 1
            })
    }

    /// Every few days: what the lessons asked for, put back if it was lost
    /// (traps sprung and broken, a squad not raised for want of hands).
    pub(super) fn ov_lessons(&mut self, o: &mut Overseer) {
        if o.lessons.is_empty() || self.tick / DAY % 5 != 0 {
            return;
        }
        let w = SiegeWatch::default();
        if self.traps_wanted(o) && self.stock_of(ItemKind::Mechanism) >= 2 {
            o.done.retain(|d| d != "stonetraps");
            self.apply_lesson(o, Lesson::TrapTheWay, &w);
        }
        if o.lessons.iter().any(|l| l.1 == Lesson::SecondSquad) && self.squads.len() < 2 {
            self.apply_lesson(o, Lesson::SecondSquad, &w);
        }
    }

    /// `patience` (ticks before the bridge goes up regardless), less what
    /// sieges have taught: 60 ticks a lesson, never under 30.
    pub(super) fn bar_sooner(&self, o: &Overseer, patience: u64) -> u64 {
        let n = o.lessons.iter().filter(|l| l.1 == Lesson::BarSooner).count() as u64;
        if n == 0 {
            return patience;
        }
        patience.saturating_sub(n * self.knob("lesson.bar_sooner", 60.0) as u64).max(30)
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

    #[test]
    fn a_siege_that_got_in_brings_traps_and_a_second_squad() {
        let mut g = super::super::overseer::trial_game(5001, Style::Steady, 20);
        // Fourteen grown dwarves, one squad.
        while g.dwarves.iter().filter(|d| d.alive && !d.is_child()).count() < 14 {
            let id = g.new_id();
            let pos = g.embark;
            let d = crate::dwarf::new_dwarf(id, pos, &mut g.rng, &[]);
            g.dwarves.push(d);
        }
        if g.squads.is_empty() {
            g.new_squad();
        }
        g.squads.truncate(1);
        let mut o = g.overseer.take().unwrap();
        assert!(o.did("linked"), "seed 5001 has its gate by day 20");
        let base = o.base.unwrap();
        let traps = |g: &Game| {
            g.buildings.values().filter(|b| matches!(b.kind, BuildingKind::WeaponTrap | BuildingKind::StoneTrap) && (b.pos.x - base.x).abs() <= 15 && (b.pos.y - base.y).abs() <= 15).count()
        };
        let before = traps(&g);
        let patience = g.bar_patience(&o);
        // They got in, past a bridge still down, and two of three soldiers fell.
        let w = SiegeWatch { siege: 0, foes_inside: true, first_contact: Some(base.offset(5, 0, 0)), contact_at: g.tick, soldiers: 3, soldiers_lost: 2, kill_spots: vec![base.offset(6, 1, 0)], ..Default::default() };
        let learned = g.learn(&mut o, &w);
        for l in [Lesson::TrapTheWay, Lesson::BarSooner, Lesson::ArmBetter, Lesson::SecondSquad] {
            assert!(learned.contains(&l), "{:?} not learned (learned {:?})", l, learned);
        }
        assert!(traps(&g) > before, "no trap planned within 15 of the gate");
        assert_eq!(g.squads.len(), 2);
        assert!(g.squads[1].members.len() >= 2, "the second squad has nobody in it");
        assert!(o.notes.iter().any(|n| n.1.starts_with("After the siege, they got inside")), "no note: {:?}", o.notes);
        assert!(g.bar_patience(&o) < patience || g.bar_patience(&o) == 30, "the bridge doesn't go up sooner");
        assert!(o.boosts.iter().any(|b| b.0 == Recipe::ForgeHelm));
        assert_eq!(o.lessons.len(), learned.len());
        assert_eq!(g.annals.counters.get("lessons").copied(), Some(learned.len() as u32));
        g.overseer = Some(o);
        // The reckless learn nothing.
        let mut r = Overseer::new(Style::Reckless);
        r.base = Some(base);
        assert!(g.learn(&mut r, &w).is_empty());
    }
}
