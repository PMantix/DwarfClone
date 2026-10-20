//! Travel the world map (A3): a cell at a time, by the land's walking
//! time, through weather and night, with encounters on the way and camps.

use super::Adventure;

/// What a step on the world map came to.
#[derive(Clone, Debug, PartialEq)]
pub enum TravelOutcome {
    Arrived,
    Encounter(Encounter),
    Blocked(String),
}

/// Something met on the road (A3).
#[derive(Clone, Debug, PartialEq)]
pub struct Encounter {
    pub text: String,
    pub figure: Option<u32>,
}

impl Adventure {
    /// One cell in direction (dx, dy) (A3). W0: refused.
    pub fn travel(&mut self, _dx: i32, _dy: i32) -> TravelOutcome {
        TravelOutcome::Blocked("Travel comes later (A3).".into())
    }

    /// Camp until dawn or until healed (A3).
    pub fn camp(&mut self) {}
}
