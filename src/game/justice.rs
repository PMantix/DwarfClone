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
        self.crimes.push(Crime { kind, culprit, victim, what, tick: self.tick, punished: false });
        // The record keeps the last few dozen.
        if self.crimes.len() > 40 {
            self.crimes.remove(0);
        }
    }

    pub fn unpunished(&self) -> usize {
        self.crimes.iter().filter(|c| !c.punished).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
