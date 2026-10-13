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
    Merchant,
    Dog,
    Cat,
    Goat,
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
    pub const ALL: [Species; 10] = [
        Species::Deer,
        Species::Wolf,
        Species::Bear,
        Species::GiantSpider,
        Species::Troglodyte,
        Species::Goblin,
        Species::Merchant,
        Species::Dog,
        Species::Cat,
        Species::Goat,
    ];

    /// Animals that can be kept (people and cave folk can't).
    pub fn tameable(self) -> bool {
        !matches!(self, Species::Goblin | Species::Troglodyte | Species::Merchant)
    }

    /// Most young in one birth.
    pub fn litter(self) -> i32 {
        match self {
            Species::Dog | Species::Cat => 3,
            Species::Goat => 2,
            _ => 1,
        }
    }

    pub fn young(self) -> &'static str {
        match self {
            Species::Dog | Species::Wolf => "pup",
            Species::Cat => "kitten",
            Species::Goat => "kid",
            Species::Deer => "fawn",
            Species::Bear => "cub",
            _ => "young",
        }
    }

    pub fn info(self) -> SpeciesInfo {
        match self {
            Species::Dog => SpeciesInfo {
                name: "dog",
                plural: "dogs",
                glyph: 'd',
                color: (200, 170, 120),
                scale: 0.5,
                damage: 5.0,
                verb: "bites",
                move_delay: 2,
                attack_delay: 9,
                aggro: 0,
                skill: 3,
                mover: Mover::Beast,
                edged: false,
            },
            Species::Cat => SpeciesInfo {
                name: "cat",
                plural: "cats",
                glyph: 'c',
                color: (220, 200, 170),
                scale: 0.25,
                damage: 2.0,
                verb: "scratches",
                move_delay: 2,
                attack_delay: 8,
                aggro: 0,
                skill: 2,
                mover: Mover::Beast,
                edged: true,
            },
            Species::Goat => SpeciesInfo {
                name: "goat",
                plural: "goats",
                glyph: 'g',
                color: (215, 215, 200),
                scale: 0.6,
                damage: 3.0,
                verb: "butts",
                move_delay: 3,
                attack_delay: 12,
                aggro: 0,
                skill: 1,
                mover: Mover::Beast,
                edged: false,
            },
            Species::Deer => SpeciesInfo {
                name: "deer",
                plural: "deer",
                glyph: 'D',
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
            Species::Merchant => SpeciesInfo {
                name: "merchant",
                plural: "merchants",
                glyph: 'M',
                color: (240, 240, 160),
                scale: 1.0,
                damage: 3.0,
                verb: "punches",
                move_delay: 3,
                attack_delay: 10,
                aggro: 0,
                skill: 1,
                mover: Mover::Dwarf,
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
    /// Kept by the fortress: never hostile, never leaves, uses doors.
    #[serde(default)]
    pub tame: bool,
    #[serde(default)]
    pub female: bool,
    #[serde(default = "yes")]
    pub adult: bool,
    /// Tick of birth (tame young grow up after a season).
    #[serde(default)]
    pub born: u64,
    /// Tick the young are due, 0 if not expecting.
    #[serde(default)]
    pub pregnant: u64,
    /// The soldier a war dog guards.
    #[serde(default)]
    pub owner: Option<u32>,
    /// The pasture zone it's kept in.
    #[serde(default)]
    pub pasture: Option<u32>,
    #[serde(default)]
    pub slaughter: bool,
}

fn yes() -> bool {
    true
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
            tame: false,
            female: false,
            adult: true,
            born: 0,
            pregnant: 0,
            owner: None,
            pasture: None,
            slaughter: false,
        }
    }

    pub fn hostile(&self) -> bool {
        self.species.info().aggro > 0 && !self.tame
    }

    /// Tame animals go through doors like dwarves.
    pub fn mover(&self) -> Mover {
        if self.tame {
            Mover::Dwarf
        } else {
            self.species.info().mover
        }
    }

    pub fn name(&self) -> &'static str {
        self.species.info().name
    }
}
