use crate::body::Body;
use crate::geom::Pos;
use crate::world::Mover;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Species {
    Deer,
    Wolf,
    Bear,
    GiantSpider,
    Troglodyte,
    Goblin,
}

pub struct SpeciesInfo {
    pub name: &'static str,
    pub plural: &'static str,
    pub glyph: char,
    pub color: (u8, u8, u8),
    pub scale: f32,
    pub damage: f32,
    pub verb: &'static str,
    pub move_delay: u32,
    pub attack_delay: u32,
    /// Attacks dwarves that come within this many tiles (0 = never hostile).
    pub aggro: i32,
    pub skill: u32,
    pub mover: Mover,
    pub edged: bool,
}

impl Species {
    pub fn info(self) -> SpeciesInfo {
        match self {
            Species::Deer => SpeciesInfo {
                name: "deer",
                plural: "deer",
                glyph: 'd',
                color: (190, 150, 100),
                scale: 0.9,
                damage: 3.0,
                verb: "kicks",
                move_delay: 2,
                attack_delay: 12,
                aggro: 0,
                skill: 1,
                mover: Mover::Beast,
                edged: false,
            },
            Species::Wolf => SpeciesInfo {
                name: "wolf",
                plural: "wolves",
                glyph: 'w',
                color: (160, 160, 175),
                scale: 0.6,
                damage: 6.0,
                verb: "bites",
                move_delay: 2,
                attack_delay: 9,
                aggro: 4,
                skill: 2,
                mover: Mover::Beast,
                edged: true,
            },
            Species::Bear => SpeciesInfo {
                name: "bear",
                plural: "bears",
                glyph: 'B',
                color: (150, 100, 60),
                scale: 1.6,
                damage: 10.0,
                verb: "mauls",
                move_delay: 3,
                attack_delay: 12,
                aggro: 5,
                skill: 4,
                mover: Mover::Beast,
                edged: true,
            },
            Species::GiantSpider => SpeciesInfo {
                name: "giant cave spider",
                plural: "giant cave spiders",
                glyph: 'S',
                color: (190, 90, 230),
                scale: 1.3,
                damage: 9.0,
                verb: "bites",
                move_delay: 3,
                attack_delay: 10,
                aggro: 12,
                skill: 5,
                mover: Mover::Beast,
                edged: true,
            },
            Species::Troglodyte => SpeciesInfo {
                name: "troglodyte",
                plural: "troglodytes",
                glyph: 't',
                color: (190, 190, 150),
                scale: 0.8,
                damage: 5.0,
                verb: "punches",
                move_delay: 3,
                attack_delay: 10,
                aggro: 10,
                skill: 2,
                mover: Mover::Beast,
                edged: false,
            },
            Species::Goblin => SpeciesInfo {
                name: "goblin",
                plural: "goblins",
                glyph: 'g',
                color: (110, 210, 80),
                scale: 0.9,
                damage: 3.0,
                verb: "punches",
                move_delay: 3,
                attack_delay: 9,
                aggro: 999,
                skill: 2,
                mover: Mover::Goblin,
                edged: false,
            },
        }
    }

    pub fn dwells_underground(self) -> bool {
        matches!(self, Species::GiantSpider | Species::Troglodyte)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Creature {
    pub id: u32,
    pub species: Species,
    pub pos: Pos,
    pub body: Body,
    pub alive: bool,
    /// Reversed path; last element is the next step.
    pub path: Vec<Pos>,
    pub move_cd: u32,
    pub attack_cd: u32,
    pub think_cd: u32,
    pub target: Option<u32>,
    pub weapon: Option<u32>,
    pub armor: Vec<u32>,
    pub leave_tick: u64,
    pub leaving: bool,
    pub siege: bool,
    /// Door being battered down.
    pub bash: Option<Pos>,
    pub bash_work: u32,
}

impl Creature {
    pub fn new(id: u32, species: Species, pos: Pos, leave_tick: u64) -> Creature {
        Creature {
            id,
            species,
            pos,
            body: Body::new(species.info().scale),
            alive: true,
            path: Vec::new(),
            move_cd: 0,
            attack_cd: 0,
            think_cd: (id % 20) as u32,
            target: None,
            weapon: None,
            armor: Vec::new(),
            leave_tick,
            leaving: false,
            siege: false,
            bash: None,
            bash_work: 0,
        }
    }

    pub fn hostile(&self) -> bool {
        self.species.info().aggro > 0
    }

    pub fn name(&self) -> &'static str {
        self.species.info().name
    }
}
