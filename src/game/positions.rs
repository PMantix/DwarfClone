//! Appointed positions: the fortress's administrators, each with a duty.

use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Position {
    Manager,
    Bookkeeper,
    Broker,
    ChiefMedic,
    Commander,
}

impl Position {
    pub const ALL: [Position; 5] = [Position::Manager, Position::Bookkeeper, Position::Broker, Position::ChiefMedic, Position::Commander];

    pub fn title(self) -> &'static str {
        match self {
            Position::Manager => "Manager",
            Position::Bookkeeper => "Bookkeeper",
            Position::Broker => "Broker",
            Position::ChiefMedic => "Chief medical dwarf",
            Position::Commander => "Militia commander",
        }
    }

    /// What having one does, for the nobles screen.
    pub fn duty(self) -> &'static str {
        match self {
            Position::Manager => "work orders are carried out",
            Position::Bookkeeper => "stocks for work orders are counted often",
            Position::Broker => "better prices from traders",
            Position::ChiefMedic => "doctors treat a quarter faster",
            Position::Commander => "soldiers learn a quarter faster",
        }
    }
}

impl Game {
    /// Who holds a position, if they're alive and in their right mind.
    pub fn holder(&self, p: Position) -> Option<&Dwarf> {
        let id = self.positions.iter().find(|(q, _)| *q == p).map(|(_, id)| *id)?;
        self.dwarves.iter().find(|d| d.id == id && d.alive && !d.insane)
    }

    pub fn position_of(&self, dwarf: u32) -> Option<Position> {
        self.positions.iter().find(|(_, id)| *id == dwarf).map(|(p, _)| *p)
    }

    /// Give a position to a dwarf (taking any other position off them), or
    /// leave it vacant.
    pub fn appoint(&mut self, p: Position, dwarf: Option<u32>) {
        self.positions.retain(|(q, id)| *q != p && Some(*id) != dwarf);
        if let Some(id) = dwarf {
            self.positions.push((p, id));
        }
    }

    /// Hand a position to the next adult in line, or leave it vacant after
    /// the last. Returns a message.
    pub fn cycle_position(&mut self, p: Position) -> String {
        let cands: Vec<u32> = self.dwarves.iter().filter(|d| d.alive && !d.insane && !d.is_child()).map(|d| d.id).collect();
        let cur = self.positions.iter().find(|(q, _)| *q == p).map(|(_, id)| *id);
        let next = match cur.and_then(|c| cands.iter().position(|&x| x == c)) {
            None => cands.first().copied(),
            Some(i) => cands.get(i + 1).copied(),
        };
        self.appoint(p, next);
        match next.and_then(|id| self.dwarves.iter().find(|d| d.id == id)) {
            Some(d) => format!("{} is now {}.", d.name, p.title().to_lowercase()),
            None => format!("The post of {} is vacant.", p.title().to_lowercase()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_go_to_one_dwarf_each_and_cycle_to_vacant() {
        let mut g = Game::new(81);
        let a = g.dwarves[0].id;
        g.appoint(Position::Manager, Some(a));
        assert_eq!(g.holder(Position::Manager).map(|d| d.id), Some(a));
        g.appoint(Position::Broker, Some(a));
        assert!(g.holder(Position::Manager).is_none(), "one position per dwarf");
        assert_eq!(g.position_of(a), Some(Position::Broker));
        let n = g.dwarves.iter().filter(|d| d.alive && !d.is_child()).count();
        let mut vacant = false;
        for _ in 0..=n {
            g.cycle_position(Position::ChiefMedic);
            vacant |= g.holder(Position::ChiefMedic).is_none();
        }
        assert!(vacant, "cycling should pass through vacant");
    }

    #[test]
    fn a_chief_medical_dwarf_and_a_commander_speed_things_up() {
        let mut g = Game::new(81);
        let d = g.dwarves[1].clone();
        let four = |g: &mut Game| {
            (0..4u64).map(|t| {
                g.tick = 1000 + t;
                g.drill_xp(&d)
            }).sum::<u32>()
        };
        let plain = four(&mut g);
        let cmd = g.dwarves[0].id;
        g.appoint(Position::Commander, Some(cmd));
        assert_eq!(four(&mut g), plain + 1, "commander should add a quarter more drill");
        assert_eq!(g.treat_time(100), 100);
        g.appoint(Position::ChiefMedic, Some(g.dwarves[2].id));
        assert_eq!(g.treat_time(100), 75);
    }
}
