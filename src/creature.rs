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
    /// A named beast out of the world's history (dragon, titan...).
    Megabeast,
    /// A one-of-a-kind horror that crawls up out of the caverns.
    CaveHorror,
    /// A sneak-thief that slips in unseen to steal something valuable.
    Skulker,
    /// Soldiers of a civilization at war with the fortress's.
    Human,
    Elf,
    /// Dwarves of a rival civilization.
    Raider,
    /// A guest at the tavern: a bard or a wandering warrior.
    Visitor,
    /// The walking dead, raised by a necromancer.
    Undead,
    // Wildlife of particular lands.
    Camel,
    Elk,
    Ibex,
    Antelope,
    Scorpion,
    Viper,
    Jaguar,
    Lion,
    Alligator,
    PolarBear,
    /// A dead thing that walks where evil lies on the land.
    Husk,
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
    pub const ALL: [Species; 18] = [
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
        Species::Megabeast,
        Species::CaveHorror,
        Species::Skulker,
        Species::Human,
        Species::Elf,
        Species::Raider,
        Species::Visitor,
        Species::Undead,
    ];

    /// People rather than beasts: they leave corpses, not carcasses.
    pub fn person(self) -> bool {
        matches!(self, Species::Goblin | Species::Merchant | Species::Human | Species::Elf | Species::Raider | Species::Visitor)
    }

    /// Animals that can be kept (people and cave folk can't).
    pub fn tameable(self) -> bool {
        !self.person() && !matches!(self, Species::Troglodyte | Species::Megabeast | Species::CaveHorror | Species::Skulker | Species::Undead)
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
            Species::Camel => SpeciesInfo {
                name: "camel",
                plural: "camels",
                glyph: 'C',
                color: (200, 165, 110),
                scale: 1.3,
                damage: 4.0,
                verb: "kicks",
                move_delay: 3,
                attack_delay: 12,
                aggro: 0,
                skill: 1,
                mover: Mover::Beast,
                edged: false,
            },
            Species::Elk => SpeciesInfo {
                name: "elk",
                plural: "elk",
                glyph: 'E',
                color: (150, 110, 70),
                scale: 1.4,
                damage: 5.0,
                verb: "gores",
                move_delay: 2,
                attack_delay: 12,
                aggro: 0,
                skill: 2,
                mover: Mover::Beast,
                edged: false,
            },
            Species::Ibex => SpeciesInfo {
                name: "ibex",
                plural: "ibex",
                glyph: 'i',
                color: (185, 170, 140),
                scale: 0.6,
                damage: 4.0,
                verb: "butts",
                move_delay: 2,
                attack_delay: 11,
                aggro: 0,
                skill: 2,
                mover: Mover::Beast,
                edged: false,
            },
            Species::Antelope => SpeciesInfo {
                name: "antelope",
                plural: "antelope",
                glyph: 'a',
                color: (205, 150, 80),
                scale: 0.7,
                damage: 3.0,
                verb: "kicks",
                move_delay: 2,
                attack_delay: 12,
                aggro: 0,
                skill: 1,
                mover: Mover::Beast,
                edged: false,
            },
            Species::Scorpion => SpeciesInfo {
                name: "giant scorpion",
                plural: "giant scorpions",
                glyph: 'x',
                color: (190, 120, 50),
                scale: 0.4,
                damage: 6.0,
                verb: "stings",
                move_delay: 2,
                attack_delay: 9,
                aggro: 3,
                skill: 3,
                mover: Mover::Beast,
                edged: true,
            },
            Species::Viper => SpeciesInfo {
                name: "viper",
                plural: "vipers",
                glyph: 'v',
                color: (120, 170, 60),
                scale: 0.2,
                damage: 5.0,
                verb: "bites",
                move_delay: 3,
                attack_delay: 8,
                aggro: 2,
                skill: 3,
                mover: Mover::Beast,
                edged: true,
            },
            Species::Jaguar => SpeciesInfo {
                name: "jaguar",
                plural: "jaguars",
                glyph: 'J',
                color: (220, 170, 60),
                scale: 0.9,
                damage: 8.0,
                verb: "mauls",
                move_delay: 2,
                attack_delay: 9,
                aggro: 5,
                skill: 4,
                mover: Mover::Beast,
                edged: true,
            },
            Species::Lion => SpeciesInfo {
                name: "lion",
                plural: "lions",
                glyph: 'L',
                color: (215, 175, 90),
                scale: 1.2,
                damage: 9.0,
                verb: "mauls",
                move_delay: 2,
                attack_delay: 10,
                aggro: 5,
                skill: 4,
                mover: Mover::Beast,
                edged: true,
            },
            Species::Alligator => SpeciesInfo {
                name: "alligator",
                plural: "alligators",
                glyph: 'A',
                color: (90, 130, 70),
                scale: 1.5,
                damage: 10.0,
                verb: "bites",
                move_delay: 4,
                attack_delay: 13,
                aggro: 3,
                skill: 3,
                mover: Mover::Beast,
                edged: true,
            },
            Species::PolarBear => SpeciesInfo {
                name: "polar bear",
                plural: "polar bears",
                glyph: 'P',
                color: (235, 235, 240),
                scale: 1.9,
                damage: 11.0,
                verb: "mauls",
                move_delay: 3,
                attack_delay: 12,
                aggro: 5,
                skill: 4,
                mover: Mover::Beast,
                edged: true,
            },
            Species::Husk => SpeciesInfo {
                name: "walking husk",
                plural: "walking husks",
                glyph: 'Z',
                color: (150, 120, 160),
                scale: 0.9,
                damage: 6.0,
                verb: "claws",
                move_delay: 3,
                attack_delay: 11,
                aggro: 6,
                skill: 2,
                mover: Mover::Beast,
                edged: true,
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
            Species::Megabeast => SpeciesInfo {
                name: "megabeast",
                plural: "megabeasts",
                glyph: 'M',
                color: (255, 170, 50),
                scale: 4.0,
                damage: 14.0,
                verb: "rends",
                move_delay: 3,
                attack_delay: 12,
                aggro: 40,
                skill: 7,
                mover: Mover::Goblin,
                edged: true,
            },
            Species::Skulker => SpeciesInfo {
                name: "skulker",
                plural: "skulkers",
                glyph: 's',
                color: (150, 170, 110),
                scale: 0.6,
                damage: 4.0,
                verb: "stabs",
                move_delay: 2,
                attack_delay: 10,
                aggro: 1,
                skill: 3,
                mover: Mover::Goblin,
                edged: true,
            },
            Species::CaveHorror => SpeciesInfo {
                name: "cave horror",
                plural: "cave horrors",
                glyph: 'H',
                color: (190, 120, 255),
                scale: 3.0,
                damage: 12.0,
                verb: "lashes",
                move_delay: 3,
                attack_delay: 11,
                aggro: 40,
                skill: 6,
                mover: Mover::Goblin,
                edged: false,
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
            Species::Undead => SpeciesInfo {
                name: "walking corpse",
                plural: "walking dead",
                glyph: 'z',
                color: (150, 175, 130),
                scale: 1.0,
                damage: 4.0,
                verb: "claws",
                move_delay: 5,
                attack_delay: 11,
                aggro: 999,
                skill: 2,
                mover: Mover::Goblin,
                edged: false,
            },
            Species::Visitor => SpeciesInfo {
                name: "visitor",
                plural: "visitors",
                glyph: '☻',
                color: (150, 210, 255),
                scale: 1.0,
                damage: 3.0,
                verb: "punches",
                move_delay: 3,
                attack_delay: 10,
                aggro: 0,
                skill: 2,
                mover: Mover::Dwarf,
                edged: false,
            },
            Species::Human => SpeciesInfo {
                name: "human soldier",
                plural: "human soldiers",
                glyph: 'h',
                color: (225, 205, 170),
                scale: 1.0,
                damage: 3.0,
                verb: "punches",
                move_delay: 3,
                attack_delay: 9,
                aggro: 999,
                skill: 3,
                mover: Mover::Goblin,
                edged: false,
            },
            Species::Elf => SpeciesInfo {
                name: "elf warrior",
                plural: "elf warriors",
                glyph: 'e',
                color: (120, 235, 150),
                scale: 0.85,
                damage: 2.5,
                verb: "strikes",
                move_delay: 2,
                attack_delay: 8,
                aggro: 999,
                skill: 3,
                mover: Mover::Goblin,
                edged: false,
            },
            Species::Raider => SpeciesInfo {
                name: "dwarf raider",
                plural: "dwarf raiders",
                glyph: 'd',
                color: (230, 170, 90),
                scale: 1.0,
                damage: 3.5,
                verb: "punches",
                move_delay: 4,
                attack_delay: 9,
                aggro: 999,
                skill: 3,
                mover: Mover::Goblin,
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
    /// The historical figure this is (megabeasts).
    #[serde(default)]
    pub figure: Option<u32>,
    /// Its name, if it has one ("Ashwing the dragon").
    #[serde(default)]
    pub label: Option<String>,
    /// What it looks like, for one-of-a-kind creatures.
    #[serde(default)]
    pub desc: Option<String>,
    /// Unseen until a dwarf comes near (thieves).
    #[serde(default)]
    pub sneaking: bool,
    /// What a thief has stolen.
    #[serde(default)]
    pub loot: Option<u32>,
    /// The historical figure leading a siege, if this is its general.
    #[serde(default)]
    pub general: Option<u32>,
    /// The dwarf this cat or dog has made its own.
    #[serde(default)]
    pub pet_of: Option<u32>,
    /// A tavern guest's calling (see `game::visitors`), 0 for everyone else.
    #[serde(default)]
    pub visitor: u8,
    /// A dragon: breathes fire and doesn't burn.
    #[serde(default)]
    pub breathes_fire: bool,
    /// Ticks until it can breathe fire again.
    #[serde(default)]
    pub breath_cd: u32,
    /// The hunting pack it came with (the first of them's id), 0 if none.
    #[serde(default)]
    pub pack: u32,
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
            figure: None,
            label: None,
            desc: None,
            sneaking: false,
            loot: None,
            general: None,
            pet_of: None,
            visitor: 0,
            breathes_fire: false,
            breath_cd: 0,
            pack: 0,
        }
    }

    /// Megabeasts, cave horrors and the walking dead: they hunt to the
    /// death and never break and run.
    pub fn legendary(&self) -> bool {
        self.figure.is_some() || matches!(self.species, Species::CaveHorror | Species::Undead)
    }

    /// Dangerous and known about: sneaks don't count until they're spotted.
    pub fn hostile(&self) -> bool {
        self.species.info().aggro > 0 && !self.tame && !self.sneaking
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

    /// Its own name if it has one, otherwise "the wolf".
    pub fn title(&self) -> String {
        self.label.clone().unwrap_or_else(|| format!("the {}", self.name()))
    }
}
