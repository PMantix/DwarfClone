use crate::body::Body;
use crate::geom::Pos;
use crate::rng::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Labor {
    Mining,
    Woodcutting,
    Carpentry,
    Masonry,
    Farming,
    Brewing,
    Cooking,
    Butchery,
    Smelting,
    Smithing,
    Crafting,
    Hauling,
    Building,
}

pub const NUM_LABORS: usize = 13;

pub const LABORS: [Labor; NUM_LABORS] = [
    Labor::Mining,
    Labor::Woodcutting,
    Labor::Carpentry,
    Labor::Masonry,
    Labor::Farming,
    Labor::Brewing,
    Labor::Cooking,
    Labor::Butchery,
    Labor::Smelting,
    Labor::Smithing,
    Labor::Crafting,
    Labor::Hauling,
    Labor::Building,
];

impl Labor {
    pub fn idx(self) -> usize {
        LABORS.iter().position(|&l| l == self).unwrap()
    }

    pub fn name(self) -> &'static str {
        match self {
            Labor::Mining => "Mining",
            Labor::Woodcutting => "Woodcutting",
            Labor::Carpentry => "Carpentry",
            Labor::Masonry => "Masonry",
            Labor::Farming => "Farming",
            Labor::Brewing => "Brewing",
            Labor::Cooking => "Cooking",
            Labor::Butchery => "Butchery",
            Labor::Smelting => "Smelting",
            Labor::Smithing => "Smithing",
            Labor::Crafting => "Stonecrafting",
            Labor::Hauling => "Hauling",
            Labor::Building => "Construction",
        }
    }

    pub fn profession(self) -> &'static str {
        match self {
            Labor::Mining => "Miner",
            Labor::Woodcutting => "Woodcutter",
            Labor::Carpentry => "Carpenter",
            Labor::Masonry => "Mason",
            Labor::Farming => "Farmer",
            Labor::Brewing => "Brewer",
            Labor::Cooking => "Cook",
            Labor::Butchery => "Butcher",
            Labor::Smelting => "Furnace operator",
            Labor::Smithing => "Smith",
            Labor::Crafting => "Craftsdwarf",
            Labor::Hauling | Labor::Building => "Peasant",
        }
    }
}

const SKILL_RANKS: [&str; 11] = [
    "Dabbling",
    "Novice",
    "Adequate",
    "Competent",
    "Skilled",
    "Proficient",
    "Talented",
    "Adept",
    "Expert",
    "Master",
    "Legendary",
];

pub fn skill_level(xp: u32) -> u32 {
    (xp / 400).min(SKILL_RANKS.len() as u32 - 1)
}

pub fn skill_rank(xp: u32) -> &'static str {
    SKILL_RANKS[skill_level(xp) as usize]
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Task {
    #[default]
    Idle,
    Wander { target: Pos },
    Dig { target: Pos, stand: Pos },
    Haul { item: u32, dest: Pos },
    Construct { bld: u32, item: u32 },
    Craft { bld: u32, item: u32 },
    Plant { pos: Pos },
    Harvest { pos: Pos },
    Eat { item: u32 },
    DrinkItem { item: u32 },
    DrinkWater { stand: Pos },
    Sleep { bed: Option<u32>, stand: Pos },
    /// Recover from wounds, in a bed if one is free.
    Rest { bed: Option<u32>, stand: Pos },
    /// Pick up a tool, weapon or piece of armor and keep it.
    Equip { item: u32 },
    Attack { target: u32 },
    Station { pos: Pos },
    Train,
    Flee,
}

impl Task {
    pub fn is_need(&self) -> bool {
        matches!(
            self,
            Task::Eat { .. }
                | Task::DrinkItem { .. }
                | Task::DrinkWater { .. }
                | Task::Sleep { .. }
                | Task::Rest { .. }
                | Task::Flee
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Thought {
    pub text: String,
    pub stress: i32,
    pub tick: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Dwarf {
    pub id: u32,
    pub name: String,
    pub pos: Pos,
    pub alive: bool,
    pub labors: [bool; NUM_LABORS],
    pub skills: [u32; NUM_LABORS],
    pub hunger: u32,
    pub thirst: u32,
    pub fatigue: u32,
    pub stress: i32,
    pub thoughts: Vec<Thought>,
    pub task: Task,
    /// Remaining path, reversed: the last element is the next step.
    pub path: Vec<Pos>,
    pub move_cd: u32,
    pub work: u32,
    pub working: bool,
    pub idle_cd: u32,
    pub carrying: Option<u32>,
    pub color: (u8, u8, u8),
    pub tantrum: u32,
    pub asleep: bool,
    pub drown: u32,
    pub body: Body,
    /// Tool or weapon in hand.
    pub held: Option<u32>,
    pub armor: Vec<u32>,
    pub soldier: bool,
    pub fight_xp: u32,
    pub attack_cd: u32,
    /// Creature that last hit this dwarf, and when.
    pub attacker: Option<(u32, u64)>,
}

impl Dwarf {
    pub fn has(&self, l: Labor) -> bool {
        self.labors[l.idx()]
    }

    pub fn skill(&self, l: Labor) -> u32 {
        skill_level(self.skills[l.idx()])
    }

    pub fn gain_xp(&mut self, l: Labor, xp: u32) {
        self.skills[l.idx()] += xp;
    }

    pub fn profession(&self) -> &'static str {
        let mut best: Option<(u32, Labor)> = None;
        for &l in LABORS.iter() {
            if matches!(l, Labor::Hauling | Labor::Building) {
                continue;
            }
            let xp = self.skills[l.idx()];
            if xp >= 400 && best.map_or(true, |(b, _)| xp > b) {
                best = Some((xp, l));
            }
        }
        if self.soldier {
            return if self.fight_xp >= 1200 { "Veteran" } else { "Recruit" };
        }
        best.map_or("Peasant", |(_, l)| l.profession())
    }

    pub fn fight_level(&self) -> u32 {
        skill_level(self.fight_xp)
    }

    pub fn mood(&self) -> (&'static str, (u8, u8, u8)) {
        if self.body.unconscious() {
            ("Unconscious", (255, 60, 60))
        } else if self.tantrum > 0 {
            ("Throwing a tantrum", (255, 60, 60))
        } else if self.stress < -150 {
            ("Ecstatic", (80, 255, 255))
        } else if self.stress < -40 {
            ("Happy", (80, 220, 80))
        } else if self.stress < 80 {
            ("Content", (220, 220, 220))
        } else if self.stress < 200 {
            ("Unhappy", (230, 200, 60))
        } else if self.stress < 350 {
            ("Very unhappy", (240, 130, 40))
        } else {
            ("Miserable", (255, 60, 60))
        }
    }

    pub fn think(&mut self, text: &str, stress: i32, tick: u64) {
        self.stress += stress;
        self.thoughts.push(Thought { text: text.to_string(), stress, tick });
        if self.thoughts.len() > 8 {
            self.thoughts.remove(0);
        }
    }
}

const FIRST: [&str; 32] = [
    "Dagna", "Kolvar", "Ulmek", "Ostra", "Brumli", "Tharva", "Grenda", "Morik", "Halvi", "Esk",
    "Dorna", "Fenrik", "Ragni", "Borra", "Thekla", "Vigdor", "Ingra", "Sorli", "Agnor", "Bekka",
    "Gruni", "Hilda", "Jorvik", "Kelda", "Lofar", "Mira", "Nothar", "Odda", "Perrin", "Runa",
    "Sigga", "Tovin",
];
const PREFIX: [&str; 16] = [
    "Stone", "Iron", "Deep", "Ash", "Copper", "Granite", "Ember", "Hammer", "Gold", "Coal", "Rune",
    "Frost", "Ale", "Anvil", "Silver", "Flint",
];
const SUFFIX: [&str; 14] = [
    "beard", "delver", "fist", "forge", "helm", "mantle", "shaper", "brew", "vein", "hearth",
    "song", "grip", "hold", "crown",
];
const COLORS: [(u8, u8, u8); 8] = [
    (255, 220, 80),
    (120, 220, 255),
    (255, 140, 200),
    (160, 255, 140),
    (255, 170, 90),
    (200, 160, 255),
    (255, 255, 255),
    (90, 255, 200),
];

pub fn random_name(rng: &mut Rng) -> String {
    format!("{} {}{}", rng.pick(&FIRST), rng.pick(&PREFIX), rng.pick(&SUFFIX))
}

pub fn fortress_name(rng: &mut Rng) -> String {
    format!("{}{}", rng.pick(&PREFIX), rng.pick(&SUFFIX))
}

pub fn new_dwarf(id: u32, pos: Pos, rng: &mut Rng, specialties: &[Labor]) -> Dwarf {
    let mut labors = [false; NUM_LABORS];
    let mut skills = [0u32; NUM_LABORS];
    labors[Labor::Hauling.idx()] = true;
    labors[Labor::Building.idx()] = true;
    for (i, &l) in specialties.iter().enumerate() {
        labors[l.idx()] = true;
        skills[l.idx()] = if i == 0 { 900 + rng.range(0, 600) as u32 } else { 400 + rng.range(0, 300) as u32 };
    }
    Dwarf {
        id,
        name: random_name(rng),
        pos,
        alive: true,
        labors,
        skills,
        hunger: rng.range(0, 1500) as u32,
        thirst: rng.range(0, 1000) as u32,
        fatigue: rng.range(0, 800) as u32,
        color: *rng.pick(&COLORS),
        idle_cd: rng.range(0, 20) as u32,
        ..Default::default()
    }
}
