//! Crime and punishment: tantrums and failed mandates are crimes on the
//! record, waiting for justice.

use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrimeKind {
    /// Struck another dwarf.
    Assault,
    /// Wrecked furniture.
    Vandalism,
    /// Blamed for a mandate that went unmet.
    Disobedience,
}

impl CrimeKind {
    pub fn name(self) -> &'static str {
        match self {
            CrimeKind::Assault => "Assault",
            CrimeKind::Vandalism => "Vandalism",
            CrimeKind::Disobedience => "Disobedience",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Crime {
    pub kind: CrimeKind,
    pub culprit: u32,
    pub victim: Option<u32>,
    /// What happened, for the record ("punched Urist", "toppled a granite table").
    pub what: String,
    pub tick: u64,
    pub punished: bool,
}

impl Game {
    /// Put a crime on the record.
    pub(super) fn report_crime(&mut self, kind: CrimeKind, culprit: u32, victim: Option<u32>, what: String) {
        self.year_log.crimes += 1;
        self.crimes.push(Crime { kind, culprit, victim, what, tick: self.tick, punished: false });
        // The record keeps the last few dozen.
        if self.crimes.len() > 40 {
            self.crimes.remove(0);
        }
    }

    pub fn unpunished(&self) -> usize {
        self.crimes.iter().filter(|c| !c.punished).count()
    }

    /// The sheriff takes up the oldest unpunished crime whose culprit is
    /// still about (never their own).
    pub(super) fn find_punishment(&mut self, d: &mut Dwarf) -> bool {
        if self.position_of(d.id) != Some(super::Position::Sheriff) {
            return false;
        }
        let culprit = self.crimes.iter().filter(|c| !c.punished && c.culprit != d.id).map(|c| c.culprit).find(|&id| {
            self.dwarves.iter().any(|o| o.id == id && o.alive && !o.insane && !matches!(o.task, Task::Punish { .. }))
        });
        let Some(culprit) = culprit else { return false };
        d.task = Task::Punish { culprit };
        true
    }

    pub(super) fn do_punish(&mut self, d: &mut Dwarf, culprit: u32) {
        let Some(cpos) = self.dwarves.iter().find(|o| o.id == culprit && o.alive).map(|o| o.pos) else {
            d.task = Task::Idle;
            return;
        };
        if !super::combat::adjacent(cpos, d.pos) && cpos != d.pos {
            if let Travel::Failed = self.travel(d, cpos) {
                d.task = Task::Idle;
                d.idle_cd = 200;
            }
            return;
        }
        if !Self::labor(d, 30, 0) {
            return;
        }
        d.task = Task::Idle;
        // A sheriff who can't fight or keep their head lets culprits slip away.
        let slip = match self.post_fit(super::Position::Sheriff) {
            Some(super::Fit::Good) => 0.0,
            Some(super::Fit::Fair) => 0.15,
            _ => 0.4,
        };
        if self.rng.chance(slip) {
            let who = self.dwarves.iter().find(|o| o.id == culprit).map_or("the culprit".to_string(), |o| o.name.clone());
            self.msg(&format!("{} slipped away from {}.", who, d.name), (200, 170, 255));
            d.idle_cd = 150;
            return;
        }
        // A hot-tempered sheriff beats harder, an even-tempered one more lightly.
        let force = if d.personality.temper >= 20 {
            4
        } else if d.personality.temper <= -20 {
            2
        } else {
            3
        };
        self.punish(culprit, &d.name, force);
    }

    /// Justice is done: jail if the fortress has one (three days a charge,
    /// at most ten), otherwise a beating; every open charge is closed.
    pub(super) fn punish(&mut self, culprit: u32, sheriff: &str, force: i32) {
        let tick = self.tick;
        let mut victims = Vec::new();
        let mut what = Vec::new();
        for c in self.crimes.iter_mut().filter(|c| c.culprit == culprit && !c.punished) {
            c.punished = true;
            victims.extend(c.victim);
            what.push(c.kind.name().to_lowercase());
        }
        if what.is_empty() {
            return;
        }
        let charges = what.len() as u64;
        what.dedup();
        let jail = self.zones.values().any(|z| z.kind == ZoneKind::Jail);
        let Some(i) = self.dwarves.iter().position(|o| o.id == culprit) else { return };
        let part = self.dwarves[i].body.pick_part(&mut self.rng);
        let o = &mut self.dwarves[i];
        let name = o.name.clone();
        if jail {
            let days = (3 * charges).min(10);
            o.jailed_until = tick + days * DAY;
            o.think(&format!("was jailed for {}", what.join(" and ")), 12, tick);
            self.msg(&format!("{} has sentenced {} to {} days in jail for {}.", sheriff, name, days, what.join(" and ")), (200, 170, 255));
        } else {
            o.body.wound(part, force, false);
            o.think(&format!("was beaten by the sheriff for {}", what.join(" and ")), 15, tick);
            self.msg(&format!("{} has punished {} for {}.", sheriff, name, what.join(" and ")), (200, 170, 255));
        }
        for v in victims {
            if let Some(vd) = self.dwarves.iter_mut().find(|x| x.id == v && x.alive) {
                vd.think("was glad to see justice done", -10, tick);
            }
        }
    }

    /// A prisoner stays in the jail (they may still go and eat, drink and
    /// sleep). True while serving.
    pub(super) fn serve_sentence(&mut self, d: &mut Dwarf) -> bool {
        if d.jailed_until == 0 {
            return false;
        }
        if self.tick >= d.jailed_until {
            d.jailed_until = 0;
            self.msg(&format!("{} has been released from jail.", d.name), (170, 170, 200));
            return false;
        }
        let cells: Vec<Pos> = self
            .zones
            .values()
            .filter(|z| z.kind == ZoneKind::Jail)
            .flat_map(|z| z.tiles.iter().copied())
            .filter(|&p| self.world.walkable(p))
            .collect();
        if cells.is_empty() {
            // The jail is gone; the sentence lapses.
            d.jailed_until = 0;
            return false;
        }
        let inside = self.zone_kind_at(d.pos) == Some(ZoneKind::Jail);
        if inside && !self.rng.chance(0.1) {
            d.idle_cd = 50;
            return true;
        }
        let q = cells[self.rng.range(0, cells.len() as i32) as usize];
        if q != d.pos {
            d.task = Task::Wander { target: q };
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sheriff_punishes_the_culprit_and_closes_the_case() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let (culprit, victim, sheriff) = (g.dwarves[0].id, g.dwarves[1].id, g.dwarves[2].id);
        g.dwarves[2].fight_xp = 2000;
        g.report_crime(CrimeKind::Assault, culprit, Some(victim), "punched someone".into());
        g.report_crime(CrimeKind::Vandalism, culprit, None, "toppled a chair".into());
        for _ in 0..DAY {
            g.step();
        }
        assert_eq!(g.unpunished(), 2, "no sheriff, no justice");
        g.appoint(super::super::Position::Sheriff, Some(sheriff));
        for _ in 0..DAY {
            g.step();
            if g.unpunished() == 0 {
                break;
            }
        }
        assert_eq!(g.unpunished(), 0, "the sheriff never punished anyone");
        let c = g.dwarves.iter().find(|d| d.id == culprit).unwrap();
        assert!(c.thoughts.iter().any(|t| t.text.starts_with("was beaten by the sheriff")));
        let v = g.dwarves.iter().find(|d| d.id == victim).unwrap();
        assert!(v.thoughts.iter().any(|t| t.text == "was glad to see justice done"));
    }

    #[test]
    fn with_a_jail_the_culprit_serves_time_inside() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let e = g.embark;
        let jail_tiles = g.add_zone(e.offset(-8, -8, -1), e.offset(-5, -5, 1), ZoneKind::Jail);
        assert!(jail_tiles > 0);
        let (culprit, sheriff) = (g.dwarves[0].id, g.dwarves[2].id);
        g.dwarves[2].fight_xp = 2000;
        g.report_crime(CrimeKind::Vandalism, culprit, None, "toppled a chair".into());
        g.appoint(super::super::Position::Sheriff, Some(sheriff));
        for _ in 0..DAY {
            g.step();
            if g.unpunished() == 0 {
                break;
            }
        }
        let until = g.dwarves.iter().find(|d| d.id == culprit).unwrap().jailed_until;
        assert!(until > g.tick, "not jailed");
        let mut inside = 0;
        for _ in 0..DAY {
            g.step();
            let d = g.dwarves.iter().find(|d| d.id == culprit).unwrap();
            inside += (g.zone_kind_at(d.pos) == Some(ZoneKind::Jail)) as u32;
        }
        assert!(inside > DAY as u32 / 2, "the prisoner was in jail only {} ticks of a day", inside);
        g.tick = until;
        let mut d = std::mem::take(g.dwarves.iter_mut().find(|d| d.id == culprit).unwrap());
        assert!(!g.serve_sentence(&mut d));
        assert_eq!(d.jailed_until, 0, "should be released");
    }

    #[test]
    fn tantrums_and_failed_mandates_go_on_the_record() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        // Two dwarves side by side; one throws a tantrum until they punch the other.
        let a = g.dwarves[0].pos;
        g.dwarves[1].pos = a.offset(1, 0, 0);
        g.dwarves[1].path.clear();
        let mut d = std::mem::take(&mut g.dwarves[0]);
        for _ in 0..200 {
            g.tantrum_mischief(&mut d);
            if g.crimes.iter().any(|c| c.kind == CrimeKind::Assault) {
                break;
            }
        }
        let culprit = d.id;
        g.dwarves[0] = d;
        let c = g.crimes.iter().find(|c| c.kind == CrimeKind::Assault).expect("no assault recorded");
        assert_eq!(c.culprit, culprit);
        assert!(c.victim.is_some() && !c.punished);

        // A mandate that runs out blames someone.
        let noble = g.dwarves[2].id;
        g.mandate = Some(Mandate { noble, kind: ItemKind::Bed, mat: None, count: 50, baseline: 0, deadline: 0 });
        g.tick = 10;
        g.check_mandate();
        assert!(g.crimes.iter().any(|c| c.kind == CrimeKind::Disobedience), "nobody was blamed");
        assert!(g.unpunished() >= 2);
    }
}
