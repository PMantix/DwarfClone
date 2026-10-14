use crate::body::Body;
use crate::geom::Pos;
use crate::items::ItemKind;
use crate::material::Mat;
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
    Mechanics,
    Medicine,
    Leatherworking,
    Clothesmaking,
    Soapmaking,
    Fishing,
}

pub const NUM_LABORS: usize = 19;

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
    Labor::Mechanics,
    Labor::Medicine,
    Labor::Leatherworking,
    Labor::Clothesmaking,
    Labor::Soapmaking,
    Labor::Fishing,
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
            Labor::Mechanics => "Mechanics",
            Labor::Medicine => "Medicine",
            Labor::Leatherworking => "Leatherworking",
            Labor::Clothesmaking => "Weaving and tailoring",
            Labor::Soapmaking => "Soapmaking",
            Labor::Fishing => "Fishing",
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
            Labor::Mechanics => "Mechanic",
            Labor::Medicine => "Doctor",
            Labor::Leatherworking => "Leatherworker",
            Labor::Clothesmaking => "Clothier",
            Labor::Soapmaking => "Soaper",
            Labor::Fishing => "Fisherdwarf",
            Labor::Hauling | Labor::Building => "Peasant",
        }
    }

    /// Three-letter column heading for the labor screen.
    pub fn short(self) -> &'static str {
        match self {
            Labor::Mining => "Min",
            Labor::Woodcutting => "Woo",
            Labor::Carpentry => "Car",
            Labor::Masonry => "Mas",
            Labor::Farming => "Frm",
            Labor::Brewing => "Brw",
            Labor::Cooking => "Coo",
            Labor::Butchery => "But",
            Labor::Smelting => "Sme",
            Labor::Smithing => "Smi",
            Labor::Crafting => "Cra",
            Labor::Hauling => "Hau",
            Labor::Building => "Con",
            Labor::Mechanics => "Mec",
            Labor::Medicine => "Med",
            Labor::Leatherworking => "Lea",
            Labor::Clothesmaking => "Clo",
            Labor::Soapmaking => "Soa",
            Labor::Fishing => "Fsh",
        }
    }

    /// Tool labors are exclusive: a dwarf carries a pick or an axe, not both.
    pub fn exclusive_with(self) -> Option<Labor> {
        match self {
            Labor::Mining => Some(Labor::Woodcutting),
            Labor::Woodcutting => Some(Labor::Mining),
            _ => None,
        }
    }

    /// Labors that can be seized by a strange mood.
    pub fn moody(self) -> bool {
        matches!(self, Labor::Carpentry | Labor::Masonry | Labor::Smithing | Labor::Crafting)
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
    /// Carry an item to a stockpile tile, optionally into a barrel or bin there.
    Haul { item: u32, dest: Pos, into: Option<u32> },
    Construct { bld: u32, item: u32 },
    /// Bring every input to the workshop, then work.
    Craft { bld: u32, items: Vec<u32>, delivered: usize },
    Plant { pos: Pos, seed: u32 },
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
    /// Possessed by a strange mood: gather materials and make an artifact.
    Mood,
    /// Eat at a seat (chair beside a table) if one was found.
    Dine { item: u32, seat: u32 },
    /// Build or take down a construction, standing beside it.
    BuildTile { con: u32, item: Option<u32>, stand: Option<Pos> },
    /// Go and throw a lever.
    PullLever { bld: u32 },
    /// Kill a tame animal marked for slaughter.
    Slaughter { creature: u32 },
    /// Treat a patient, bringing supplies if needed.
    Treat { patient: u32, what: crate::game::Treatment, item: Option<u32> },
    /// Carry a citizen's corpse to a coffin.
    Bury { item: u32, coffin: u32 },
    /// Pick a wild shrub in a gathering zone.
    Gather { pos: Pos },
    /// Fish from the bank at `stand`.
    Fish { stand: Pos },
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Personality {
    /// Each trait runs from -50 to 50.
    pub anxiety: i32,
    pub sociability: i32,
    pub temper: i32,
    pub diligence: i32,
    pub artistry: i32,
}

impl Personality {
    pub fn random(rng: &mut Rng) -> Personality {
        let mut t = || rng.range(-50, 51);
        Personality { anxiety: t(), sociability: t(), temper: t(), diligence: t(), artistry: t() }
    }

    pub fn describe(&self) -> Vec<&'static str> {
        let mut v = Vec::new();
        let pick = |x: i32, hi: &'static str, lo: &'static str| {
            if x >= 25 {
                Some(hi)
            } else if x <= -25 {
                Some(lo)
            } else {
                None
            }
        };
        v.extend(pick(self.anxiety, "is easily stressed", "is calm under pressure"));
        v.extend(pick(self.sociability, "loves company", "prefers to be alone"));
        v.extend(pick(self.temper, "has a hot temper", "is slow to anger"));
        v.extend(pick(self.diligence, "is a tireless worker", "is rather lazy"));
        v.extend(pick(self.artistry, "is moved by fine craftsmanship", "cares little for art"));
        v
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NeedKind {
    Socialize,
    Drink,
    Craft,
    Fight,
    Learn,
    Pray,
    Party,
}

pub const NEED_KINDS: [NeedKind; 7] =
    [NeedKind::Socialize, NeedKind::Drink, NeedKind::Craft, NeedKind::Fight, NeedKind::Learn, NeedKind::Pray, NeedKind::Party];

impl NeedKind {
    pub fn name(self) -> &'static str {
        match self {
            NeedKind::Socialize => "Socialize",
            NeedKind::Drink => "Drink",
            NeedKind::Craft => "Craft",
            NeedKind::Fight => "Fight",
            NeedKind::Learn => "Learn",
            NeedKind::Pray => "Pray",
            NeedKind::Party => "Make merry",
        }
    }

    /// "is unable to ..." / "was able to ... at last".
    pub fn verb(self) -> &'static str {
        match self {
            NeedKind::Socialize => "spend time with others",
            NeedKind::Drink => "have a drink",
            NeedKind::Craft => "make something",
            NeedKind::Fight => "pick a fight",
            NeedKind::Learn => "learn something",
            NeedKind::Pray => "pray",
            NeedKind::Party => "make merry",
        }
    }
}

/// Level a need starts to weigh on a dwarf's mind, and where it hurts.
pub const NEED_UNMET: i32 = -300;
pub const NEED_BADLY: i32 = -600;
pub const NEED_MIN: i32 = -1000;
pub const NEED_MAX: i32 = 400;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Need {
    pub kind: NeedKind,
    /// 1 to 3: how much it matters to this dwarf.
    pub strength: u8,
    /// From NEED_MIN (long unmet) to NEED_MAX (just satisfied).
    pub level: i32,
    /// Set once the need went badly unmet, until it is met again.
    #[serde(default)]
    pub low: bool,
}

impl Need {
    pub fn status(&self) -> &'static str {
        match self.level {
            l if l >= 150 => "satisfied",
            l if l > NEED_UNMET => "not a concern",
            l if l > NEED_BADLY => "unfulfilled",
            _ => "badly unfulfilled",
        }
    }
}

/// Which needs a dwarf has, shaped by personality.
pub fn random_needs(p: &Personality, rng: &mut Rng) -> Vec<Need> {
    let by = |x: i32| -> u8 {
        if x >= 25 {
            3
        } else if x >= 0 {
            2
        } else if x >= -25 {
            1
        } else {
            0
        }
    };
    let mut v = Vec::new();
    for k in NEED_KINDS {
        let strength = match k {
            NeedKind::Socialize => by(p.sociability),
            NeedKind::Drink => rng.range(1, 4) as u8,
            NeedKind::Craft => by(p.artistry),
            NeedKind::Fight => by(p.temper).saturating_sub(1),
            NeedKind::Learn => rng.range(0, 3) as u8,
            NeedKind::Pray => {
                if rng.chance(0.4) {
                    0
                } else {
                    rng.range(1, 4) as u8
                }
            }
            NeedKind::Party => {
                if p.sociability >= 10 || rng.chance(0.4) {
                    rng.range(1, 3) as u8
                } else {
                    0
                }
            }
        };
        if strength > 0 {
            v.push(Need { kind: k, strength, level: rng.range(100, 300), low: false });
        }
    }
    v
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Noble {
    Leader,
    Mayor,
    /// Titles granted by the civilization as the fortress grows rich.
    Baron,
    Count,
    Duke,
}

impl Noble {
    pub fn title(self) -> &'static str {
        match self {
            Noble::Leader => "Expedition leader",
            Noble::Mayor => "Mayor",
            Noble::Baron => "Baron",
            Noble::Count => "Count",
            Noble::Duke => "Duke",
        }
    }

    /// Minimum bedroom value this noble expects.
    pub fn room_demand(self) -> u32 {
        match self {
            Noble::Leader => 40,
            Noble::Mayor => 100,
            Noble::Baron => 200,
            Noble::Count => 400,
            Noble::Duke => 800,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoodNeed {
    Log,
    Stone,
    Bar,
    Bone,
    Gem,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mood {
    pub labor: Labor,
    pub shop: Option<u32>,
    pub needs: Vec<MoodNeed>,
    pub items: Vec<u32>,
    pub delivered: usize,
    pub started: u64,
    pub work_left: u32,
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
    /// Older saves have fewer labors; missing ones start off.
    #[serde(deserialize_with = "pad_labors")]
    pub labors: [bool; NUM_LABORS],
    #[serde(deserialize_with = "pad_labors")]
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
    /// Shirt, trousers and shoes being worn.
    #[serde(default)]
    pub clothes: Vec<u32>,
    /// Something fresh to wear is in stock.
    #[serde(default)]
    pub want_clothes: bool,
    pub soldier: bool,
    pub fight_xp: u32,
    pub attack_cd: u32,
    /// Creature that last hit this dwarf, and when.
    pub attacker: Option<(u32, u64)>,
    #[serde(default)]
    pub personality: Personality,
    #[serde(default)]
    pub likes_mat: Mat,
    #[serde(default)]
    pub likes_food: Option<ItemKind>,
    /// (other dwarf id, regard from -100 to 100)
    #[serde(default)]
    pub relations: Vec<(u32, i32)>,
    #[serde(default)]
    pub last_chat: u64,
    #[serde(default)]
    pub insane: bool,
    /// Ticks spent miserable; sustained misery breaks a dwarf.
    #[serde(default)]
    pub misery: u32,
    #[serde(default)]
    pub mood: Option<Mood>,
    #[serde(default)]
    pub noble: Option<Noble>,
    /// Only does their own skilled labors: no hauling.
    #[serde(default)]
    pub specialist: bool,
    /// Crossbow bolts in the quiver.
    #[serde(default)]
    pub ammo: u32,
    /// Personal needs; empty in older saves until first checked.
    #[serde(default)]
    pub needs: Vec<Need>,
    /// Taking a break to pray or carouse until this tick.
    #[serde(default)]
    pub break_until: u64,
    /// In years; missing in older saves until first checked.
    #[serde(default)]
    pub age: Option<u32>,
    #[serde(default)]
    pub female: Option<bool>,
    #[serde(default)]
    pub spouse: Option<u32>,
    /// (mother, father) for those born in the fortress.
    #[serde(default)]
    pub parents: Option<(u32, u32)>,
    /// Expecting a child, due at this tick.
    #[serde(default)]
    pub due: Option<u64>,
    /// When they died (unknown for deaths in older saves).
    #[serde(default)]
    pub died: Option<u64>,
    /// Buried or remembered on a slab.
    #[serde(default)]
    pub rested: bool,
    /// Walks the halls as a ghost (position is the ghost's).
    #[serde(default)]
    pub ghost: bool,
}

impl Dwarf {
    pub fn has(&self, l: Labor) -> bool {
        self.labors[l.idx()]
    }

    pub fn skill(&self, l: Labor) -> u32 {
        skill_level(self.skills[l.idx()])
    }

    pub fn gain_xp(&mut self, l: Labor, xp: u32) {
        let before = skill_level(self.skills[l.idx()]);
        self.skills[l.idx()] += xp;
        // Picking up a new trade feeds the urge to learn; plain hauling doesn't.
        if l != Labor::Hauling {
            if before < 5 {
                self.satisfy(NeedKind::Learn, xp as i32 * 2);
            }
            if skill_level(self.skills[l.idx()]) > before {
                self.satisfy(NeedKind::Learn, 300);
            }
        }
    }

    pub fn is_child(&self) -> bool {
        self.age.map_or(false, |a| a < crate::game::ADULT_AGE)
    }

    /// `other` is this dwarf's parent.
    pub fn family_of(&self, other: u32) -> bool {
        self.parents.map_or(false, |(m, f)| m == other || f == other)
    }

    pub fn need(&self, k: NeedKind) -> Option<&Need> {
        self.needs.iter().find(|n| n.kind == k)
    }

    pub fn satisfy(&mut self, k: NeedKind, amount: i32) {
        if let Some(n) = self.needs.iter_mut().find(|n| n.kind == k) {
            n.level = (n.level + amount).min(NEED_MAX);
        }
    }

    /// Work focus in percent: unmet needs distract (down to 70%), met ones
    /// sharpen (up to 120%).
    pub fn focus(&self) -> i32 {
        let total: i32 = self.needs.iter().map(|n| n.strength as i32).sum();
        if total == 0 {
            return 100;
        }
        let avg = self.needs.iter().map(|n| n.level.clamp(NEED_MIN, NEED_MAX) * n.strength as i32).sum::<i32>() / total;
        if avg < 0 {
            100 + avg * 3 / 100
        } else {
            100 + avg / 20
        }
    }

    pub fn focus_word(&self) -> &'static str {
        match self.focus() {
            f if f >= 112 => "very focused",
            f if f >= 104 => "focused",
            f if f > 92 => "",
            f if f > 76 => "distracted",
            _ => "badly distracted",
        }
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
        if let Some(n) = self.noble {
            return n.title();
        }
        if self.is_child() {
            return if self.age == Some(0) { "Baby" } else { "Child" };
        }
        if self.soldier {
            return if self.fight_xp >= 1200 { "Veteran" } else { "Recruit" };
        }
        best.map_or("Peasant", |(_, l)| l.profession())
    }

    pub fn fight_level(&self) -> u32 {
        skill_level(self.fight_xp)
    }

    pub fn regard(&self, other: u32) -> i32 {
        self.relations.iter().find(|(id, _)| *id == other).map_or(0, |r| r.1)
    }

    pub fn adjust_regard(&mut self, other: u32, delta: i32) {
        match self.relations.iter_mut().find(|(id, _)| *id == other) {
            Some(r) => r.1 = (r.1 + delta).clamp(-100, 100),
            None => self.relations.push((other, delta.clamp(-100, 100))),
        }
    }

    pub fn tantrum_threshold(&self) -> i32 {
        450 - self.personality.temper * 2
    }

    pub fn mood(&self) -> (&'static str, (u8, u8, u8)) {
        if self.insane {
            ("Stricken by melancholy", (150, 150, 255))
        } else if self.mood.is_some() {
            ("Possessed by a strange mood", (255, 120, 255))
        } else if self.body.unconscious() {
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
        // Anxious dwarves take bad news harder and good news less to heart.
        let stress = if stress > 0 {
            stress * (100 + self.personality.anxiety) / 100
        } else {
            stress * (100 - self.personality.anxiety / 2) / 100
        };
        // Happiness doesn't bank forever: a contented dwarf can still be brought low.
        self.stress = (self.stress + stress).max(STRESS_FLOOR);
        self.thoughts.push(Thought { text: text.to_string(), stress, tick });
        if self.thoughts.len() > 8 {
            self.thoughts.remove(0);
        }
    }
}

fn pad_labors<'de, D, T>(d: D) -> Result<[T; NUM_LABORS], D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default + Copy,
{
    let v: Vec<T> = Vec::deserialize(d)?;
    let mut a = [T::default(); NUM_LABORS];
    for (i, x) in v.into_iter().take(NUM_LABORS).enumerate() {
        a[i] = x;
    }
    Ok(a)
}

/// The happiest a dwarf's stress can get.
pub const STRESS_FLOOR: i32 = -400;

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

const LIKED_MATS: [Mat; 10] = [
    Mat::Granite,
    Mat::Marble,
    Mat::Limestone,
    Mat::Wood,
    Mat::Copper,
    Mat::Iron,
    Mat::Steel,
    Mat::Gold,
    Mat::Amethyst,
    Mat::Bone,
];

const ART_A: [&str; 16] = [
    "Ember", "Oath", "Echo", "Anvil", "Gleam", "Hollow", "Thunder", "Silence", "Dawn", "Ash",
    "Crown", "Vigil", "Storm", "Marrow", "Lantern", "Root",
];
const ART_B: [&str; 12] = [
    "of Echoes", "of the Deep", "Unbroken", "of Ages", "the Patient", "of Sorrows", "Everlasting",
    "of the Mountainhome", "the Bright", "of Iron Rain", "the Quiet", "of First Fire",
];

pub fn artifact_name(rng: &mut Rng) -> String {
    format!("The {} {}", rng.pick(&ART_A), rng.pick(&ART_B))
}

pub fn random_name(rng: &mut Rng) -> String {
    format!("{} {}{}", rng.pick(&FIRST), rng.pick(&PREFIX), rng.pick(&SUFFIX))
}

pub fn fortress_name(rng: &mut Rng) -> String {
    format!("{}{}", rng.pick(&PREFIX), rng.pick(&SUFFIX))
}

/// A baby born in the fortress: no labors, the mother's family name.
pub fn new_child(id: u32, pos: Pos, rng: &mut Rng, mother: &Dwarf, father: u32) -> Dwarf {
    let mut d = new_dwarf(id, pos, rng, &[]);
    d.labors = [false; NUM_LABORS];
    d.age = Some(0);
    d.parents = Some((mother.id, father));
    d.hunger = 0;
    d.thirst = 0;
    d.fatigue = 0;
    let first = d.name.split(' ').next().unwrap_or("").to_string();
    if let Some(family) = mother.name.split(' ').nth(1) {
        d.name = format!("{} {}", first, family);
    }
    d
}

pub fn new_dwarf(id: u32, pos: Pos, rng: &mut Rng, specialties: &[Labor]) -> Dwarf {
    let mut labors = [false; NUM_LABORS];
    let mut skills = [0u32; NUM_LABORS];
    labors[Labor::Hauling.idx()] = true;
    labors[Labor::Building.idx()] = true;
    for (i, &l) in specialties.iter().enumerate() {
        if l.exclusive_with().map_or(false, |r| labors[r.idx()]) {
            continue;
        }
        labors[l.idx()] = true;
        skills[l.idx()] = if i == 0 { 900 + rng.range(0, 600) as u32 } else { 400 + rng.range(0, 300) as u32 };
    }
    let personality = Personality::random(rng);
    let needs = random_needs(&personality, rng);
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
        personality,
        needs,
        age: Some(rng.range(18, 100) as u32),
        female: Some(rng.chance(0.5)),
        likes_mat: *rng.pick(&LIKED_MATS),
        likes_food: Some(*rng.pick(&[ItemKind::Plant, ItemKind::Meat, ItemKind::Meal])),
        ..Default::default()
    }
}
