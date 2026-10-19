//! The bestiary: the world's animals as data. Each beast has its looks,
//! size, where it lives (lands, waters, cavern layers, a band of warmth),
//! how it behaves (its temperament), how it moves, how many go about
//! together, how it fights, what it is good for, and how rare it is. The
//! game knows them all through one species, `Species::Beast(i)`, whose
//! `SpeciesInfo` is built from the table (`creature.rs`); the old named
//! species stay as they were.
//!
//! The table is a const array, so a mistake in it is a compile error, and
//! the tests below check every entry's fields.

use crate::items::ItemKind;
use crate::material::Mat;
use crate::overworld::Biome;

// ---------------------------------------------------------------------
// Habitats: the lands, the waters and the three cavern layers.
// ---------------------------------------------------------------------

pub const FOREST: u32 = 1 << 0;
pub const TAIGA: u32 = 1 << 1;
pub const JUNGLE: u32 = 1 << 2;
pub const GRASS: u32 = 1 << 3;
pub const SAVANNA: u32 = 1 << 4;
pub const SHRUB: u32 = 1 << 5;
pub const DESERT: u32 = 1 << 6;
pub const BADLANDS: u32 = 1 << 7;
pub const SWAMP: u32 = 1 << 8;
pub const MARSH: u32 = 1 << 9;
pub const TUNDRA: u32 = 1 << 10;
pub const GLACIER: u32 = 1 << 11;
pub const MOUNTAIN: u32 = 1 << 12;
pub const HILLS: u32 = 1 << 13;
/// The shore of the sea.
pub const COAST: u32 = 1 << 14;
pub const RIVER: u32 = 1 << 15;
pub const LAKE: u32 = 1 << 16;
pub const SEA: u32 = 1 << 17;
pub const CAVE1: u32 = 1 << 18;
pub const CAVE2: u32 = 1 << 19;
pub const CAVE3: u32 = 1 << 20;

pub const WOODS: u32 = FOREST | TAIGA | JUNGLE;
pub const OPEN: u32 = GRASS | SAVANNA | SHRUB;
pub const DRY: u32 = DESERT | BADLANDS;
pub const WET: u32 = SWAMP | MARSH;
pub const COLD: u32 = TUNDRA | GLACIER;
pub const HIGH: u32 = MOUNTAIN | HILLS;
pub const FRESH: u32 = RIVER | LAKE;
pub const CAVES: u32 = CAVE1 | CAVE2 | CAVE3;
/// Every land biome.
pub const LAND: u32 = WOODS | OPEN | DRY | WET | COLD | HIGH;

/// Habitat bits and their names, for the bestiary and the tests.
pub const HABITATS: [(u32, &str); 21] = [
    (FOREST, "temperate forest"),
    (TAIGA, "taiga"),
    (JUNGLE, "jungle"),
    (GRASS, "grassland"),
    (SAVANNA, "savanna"),
    (SHRUB, "shrubland"),
    (DESERT, "desert"),
    (BADLANDS, "badlands"),
    (SWAMP, "swamp"),
    (MARSH, "marsh"),
    (TUNDRA, "tundra"),
    (GLACIER, "glacier"),
    (MOUNTAIN, "mountains"),
    (HILLS, "hills"),
    (COAST, "coasts"),
    (RIVER, "rivers"),
    (LAKE, "lakes"),
    (SEA, "the sea"),
    (CAVE1, "first cavern"),
    (CAVE2, "second cavern"),
    (CAVE3, "third cavern"),
];

/// The habitat bit of a world biome (none for open water: see SEA, LAKE).
pub fn biome_bit(b: Biome) -> u32 {
    match b {
        Biome::Forest => FOREST,
        Biome::Taiga => TAIGA,
        Biome::Jungle => JUNGLE,
        Biome::Grassland => GRASS,
        Biome::Savanna => SAVANNA,
        Biome::Shrubland => SHRUB,
        Biome::Desert => DESERT,
        Biome::Badlands => BADLANDS,
        Biome::Swamp => SWAMP,
        Biome::Marsh => MARSH,
        Biome::Tundra => TUNDRA,
        Biome::Glacier => GLACIER,
        Biome::Mountain => MOUNTAIN,
        Biome::Hills => HILLS,
        Biome::Ocean => SEA,
        Biome::Lake => LAKE,
    }
}

/// The cavern layer's bit (0 the first, 1 the second, 2 and below the third).
pub fn cave_bit(layer: usize) -> u32 {
    match layer {
        0 => CAVE1,
        1 => CAVE2,
        _ => CAVE3,
    }
}

// ---------------------------------------------------------------------
// Movement.
// ---------------------------------------------------------------------

pub const WALK: u8 = 1;
pub const FLY: u8 = 2;
pub const SWIM: u8 = 4;
pub const BURROW: u8 = 8;
pub const CLIMB: u8 = 16;

// ---------------------------------------------------------------------
// Traits.
// ---------------------------------------------------------------------

pub const VENOMOUS: u16 = 1 << 0;
pub const NOCTURNAL: u16 = 1 << 1;
/// Keeps well clear of fire from the first sight of it.
pub const FIREFEAR: u16 = 1 << 2;
/// Can be tamed out of a cage.
pub const TAMEABLE: u16 = 1 << 3;
/// Can be trained for war.
pub const WARBEAST: u16 = 1 << 4;
/// Can be trained to hunt.
pub const HUNTBEAST: u16 = 1 << 5;
/// Big and steady enough to ride, some day.
pub const MOUNT: u16 = 1 << 6;
/// Takes to a dwarf as a pet.
pub const PETLIKE: u16 = 1 << 7;
/// Kept for what it gives: a choice for the wagon.
pub const LIVESTOCK: u16 = 1 << 8;
/// A giant kind of a common animal.
pub const GIANT: u16 = 1 << 9;
/// Gives off its own light (cave things).
pub const GLOWS: u16 = 1 << 10;

// ---------------------------------------------------------------------
// Products.
// ---------------------------------------------------------------------

pub const MEAT: u16 = 1 << 0;
pub const FAT: u16 = 1 << 1;
pub const BONE: u16 = 1 << 2;
pub const HIDE: u16 = 1 << 3;
pub const WOOL: u16 = 1 << 4;
pub const MILK: u16 = 1 << 5;
pub const EGGS: u16 = 1 << 6;
pub const FEATHERS: u16 = 1 << 7;
pub const VENOM: u16 = 1 << 8;
pub const CHITIN: u16 = 1 << 9;
pub const IVORY: u16 = 1 << 10;
pub const PEARL: u16 = 1 << 11;
pub const SILK: u16 = 1 << 12;
pub const SHELL: u16 = 1 << 13;
pub const HORN: u16 = 1 << 14;

/// Product bits and their names ("wool", "feathers").
pub const PRODUCTS: [(u16, &str); 15] = [
    (MEAT, "meat"),
    (FAT, "fat"),
    (BONE, "bones"),
    (HIDE, "hide"),
    (WOOL, "wool"),
    (MILK, "milk"),
    (EGGS, "eggs"),
    (FEATHERS, "feathers"),
    (VENOM, "venom"),
    (CHITIN, "chitin"),
    (IVORY, "ivory"),
    (PEARL, "pearls"),
    (SILK, "silk"),
    (SHELL, "shell"),
    (HORN, "horn"),
];

/// Products taken from a carcass as goods (not meat, fat, bones or hide,
/// and not what only a kept animal gives).
pub const GOODS: u16 = WOOL | FEATHERS | VENOM | CHITIN | IVORY | PEARL | SILK | SHELL | HORN;

// ---------------------------------------------------------------------
// The beast.
// ---------------------------------------------------------------------

/// How it behaves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Temper {
    /// Small and hunted: bolts from anything.
    Prey,
    /// Grazes in herds, and the herd runs together.
    Grazer,
    /// Hunts other animals; the big ones size up dwarves too.
    Predator,
    /// Comes to carcasses and battlefields.
    Scavenger,
    /// Raids the food stores.
    Vermin,
    /// Waits, still, for something to come close.
    Ambusher,
    /// Leaves others be, but drives off anyone who comes too near.
    Territorial,
}

impl Temper {
    #[cfg_attr(not(test), allow(dead_code))]
    pub const ALL: [Temper; 7] = [Temper::Prey, Temper::Grazer, Temper::Predator, Temper::Scavenger, Temper::Vermin, Temper::Ambusher, Temper::Territorial];

    pub fn name(self) -> &'static str {
        match self {
            Temper::Prey => "prey",
            Temper::Grazer => "grazer",
            Temper::Predator => "predator",
            Temper::Scavenger => "scavenger",
            Temper::Vermin => "vermin",
            Temper::Ambusher => "ambusher",
            Temper::Territorial => "territorial",
        }
    }
}

/// What kind of animal: for its blood, its young and what's in its carcass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Mammal,
    Bird,
    Reptile,
    Amphibian,
    Fish,
    /// Insects, spiders, scorpions and crabs.
    Bug,
    /// Snails, clams, octopuses.
    Mollusc,
    Worm,
    /// Nothing like anything else.
    Strange,
}

/// How it strikes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Strike {
    Bite,
    Claw,
    Gore,
    Kick,
    Sting,
    Constrict,
    Peck,
    Butt,
    Talons,
    Slam,
    Lash,
    Pinch,
    Trample,
}

impl Strike {
    pub fn verb(self) -> &'static str {
        match self {
            Strike::Bite => "bites",
            Strike::Claw => "claws",
            Strike::Gore => "gores",
            Strike::Kick => "kicks",
            Strike::Sting => "stings",
            Strike::Constrict => "squeezes",
            Strike::Peck => "pecks",
            Strike::Butt => "butts",
            Strike::Talons => "rakes",
            Strike::Slam => "slams",
            Strike::Lash => "lashes",
            Strike::Pinch => "pinches",
            Strike::Trample => "tramples",
        }
    }

    pub fn edged(self) -> bool {
        matches!(self, Strike::Bite | Strike::Claw | Strike::Gore | Strike::Sting | Strike::Peck | Strike::Talons | Strike::Pinch)
    }

    pub fn noun(self) -> &'static str {
        match self {
            Strike::Bite => "bite",
            Strike::Claw => "claws",
            Strike::Gore => "horns",
            Strike::Kick => "kick",
            Strike::Sting => "sting",
            Strike::Constrict => "coils",
            Strike::Peck => "beak",
            Strike::Butt => "head",
            Strike::Talons => "talons",
            Strike::Slam => "bulk",
            Strike::Lash => "tail",
            Strike::Pinch => "pincers",
            Strike::Trample => "feet",
        }
    }
}

/// One animal of the bestiary.
#[derive(Clone, Copy, Debug)]
pub struct Beast {
    pub name: &'static str,
    pub plural: &'static str,
    pub glyph: char,
    pub color: (u8, u8, u8),
    /// Body scale (a dwarf is 1).
    pub size: f32,
    pub class: Class,
    pub temper: Temper,
    /// WALK, FLY, SWIM, BURROW, CLIMB.
    pub moves: u8,
    /// Habitat bits; 0 for a kept animal that lives nowhere wild.
    pub home: u32,
    /// The warmth it lives in (0..100, as the world's fields have it).
    pub temp: (u8, u8),
    /// How many go about together.
    pub group: (u8, u8),
    /// Ticks between steps (1 fast .. 9 barely moves).
    pub speed: u8,
    pub strike: Strike,
    pub damage: f32,
    pub traits: u16,
    pub yields: u16,
    /// 1 common .. 5 very rare.
    pub rarity: u8,
    pub young: &'static str,
    pub note: &'static str,
}

const fn class_yields(c: Class) -> u16 {
    match c {
        Class::Mammal => MEAT | FAT | BONE | HIDE,
        Class::Bird => MEAT | BONE | FEATHERS,
        Class::Reptile => MEAT | BONE | HIDE,
        Class::Amphibian => MEAT | BONE,
        Class::Fish => MEAT | BONE,
        Class::Bug => CHITIN,
        Class::Mollusc => MEAT | SHELL,
        Class::Worm => MEAT,
        Class::Strange => MEAT | BONE | HIDE,
    }
}

const fn class_young(c: Class) -> &'static str {
    match c {
        Class::Mammal => "young",
        Class::Bird => "chick",
        Class::Reptile | Class::Amphibian => "hatchling",
        Class::Fish => "fry",
        Class::Bug | Class::Worm => "larva",
        Class::Mollusc => "spat",
        Class::Strange => "spawn",
    }
}

impl Beast {
    const fn new(name: &'static str, plural: &'static str, glyph: char, color: (u8, u8, u8), size: f32, class: Class, temper: Temper) -> Beast {
        Beast {
            name,
            plural,
            glyph,
            color,
            size,
            class,
            temper,
            moves: WALK,
            home: 0,
            temp: (0, 100),
            group: (1, 1),
            speed: 3,
            strike: Strike::Bite,
            damage: 2.0,
            traits: 0,
            yields: class_yields(class),
            rarity: 2,
            young: class_young(class),
            note: "",
        }
    }
    const fn lives(mut self, home: u32, lo: u8, hi: u8) -> Beast {
        self.home = home;
        self.temp = (lo, hi);
        self
    }
    const fn moves(mut self, m: u8) -> Beast {
        self.moves = m;
        self
    }
    const fn herd(mut self, lo: u8, hi: u8) -> Beast {
        self.group = (lo, hi);
        self
    }
    const fn speed(mut self, s: u8) -> Beast {
        self.speed = s;
        self
    }
    const fn hits(mut self, s: Strike, dmg: f32) -> Beast {
        self.strike = s;
        self.damage = dmg;
        self
    }
    const fn traits(mut self, t: u16) -> Beast {
        self.traits = t;
        self
    }
    const fn yields(mut self, y: u16) -> Beast {
        self.yields = y;
        self
    }
    const fn plus(mut self, y: u16) -> Beast {
        self.yields |= y;
        self
    }
    const fn rare(mut self, r: u8) -> Beast {
        self.rarity = r;
        self
    }
    const fn young(mut self, y: &'static str) -> Beast {
        self.young = y;
        self
    }
    const fn note(mut self, n: &'static str) -> Beast {
        self.note = n;
        self
    }

    pub fn has(&self, t: u16) -> bool {
        self.traits & t != 0
    }
    pub fn gives(&self, y: u16) -> bool {
        self.yields & y != 0
    }
    pub fn can(&self, m: u8) -> bool {
        self.moves & m != 0
    }
    /// Lives only in the water (fish, whales): it never leaves it.
    pub fn water_only(&self) -> bool {
        self.moves & (WALK | FLY) == 0 && self.can(SWIM)
    }
    /// Crosses the sky above the land.
    pub fn flier(&self) -> bool {
        self.can(FLY)
    }
    /// Lives in the caverns and nowhere else.
    pub fn cave_only(&self) -> bool {
        self.home & CAVES != 0 && self.home & !CAVES == 0
    }
    /// Kept by people, not found wild.
    pub fn domestic(&self) -> bool {
        self.home == 0
    }

    /// How close a dwarf may come before it attacks (0: never).
    pub fn aggro(&self) -> i32 {
        match self.temper {
            Temper::Predator if self.size >= 0.5 => 4 + (self.size >= 1.2) as i32,
            Temper::Territorial if self.size >= 0.5 => 2 + (self.size >= 1.5) as i32,
            Temper::Ambusher if self.size >= 0.3 => 2,
            Temper::Ambusher if self.has(VENOMOUS) => 1,
            _ => 0,
        }
    }

    /// Fighting skill: the hunters and the big are better at it.
    pub fn skill(&self) -> u32 {
        let base = match self.temper {
            Temper::Predator | Temper::Ambusher => 3,
            Temper::Territorial => 2,
            _ => 1,
        };
        base + (self.size >= 1.0) as u32 + (self.size >= 2.5) as u32
    }

    /// Ticks between blows.
    pub fn attack_delay(&self) -> u32 {
        (8.0 + self.size.min(3.0) * 1.7).round() as u32
    }

    /// The force of a blow, venom included.
    pub fn blow(&self) -> f32 {
        if self.has(VENOMOUS) {
            self.damage * 1.3
        } else {
            self.damage
        }
    }

    /// How common it is, as a weight for picking.
    pub fn weight(&self) -> f64 {
        match self.rarity {
            1 => 8.0,
            2 => 5.0,
            3 => 3.0,
            4 => 1.5,
            _ => 0.6,
        }
    }

    pub fn rarity_name(&self) -> &'static str {
        match self.rarity {
            1 => "common",
            2 => "usual",
            3 => "uncommon",
            4 => "rare",
            _ => "very rare",
        }
    }

    /// Lives in warmth `t` (0..100).
    pub fn fits_temp(&self, t: f32) -> bool {
        t >= self.temp.0 as f32 && t <= self.temp.1 as f32
    }

    /// Its habitats in words: "temperate forest, taiga, rivers".
    pub fn habitat_words(&self) -> String {
        if self.domestic() {
            return "kept by people".into();
        }
        let v: Vec<&str> = HABITATS.iter().filter(|(b, _)| self.home & b != 0).map(|(_, n)| *n).collect();
        v.join(", ")
    }

    /// How it gets about in words: "walks, swims".
    pub fn movement_words(&self) -> String {
        let all = [(WALK, "walks"), (FLY, "flies"), (SWIM, "swims"), (BURROW, "burrows"), (CLIMB, "climbs")];
        all.iter().filter(|(m, _)| self.can(*m)).map(|(_, n)| *n).collect::<Vec<_>>().join(", ")
    }

    /// What it's good for in words.
    pub fn product_words(&self) -> String {
        PRODUCTS.iter().filter(|(p, _)| self.gives(*p)).map(|(_, n)| *n).collect::<Vec<_>>().join(", ")
    }

    /// Its traits in words.
    pub fn trait_words(&self) -> Vec<&'static str> {
        let all = [
            (VENOMOUS, "venomous"),
            (NOCTURNAL, "nocturnal"),
            (FIREFEAR, "fears fire"),
            (TAMEABLE, "tameable"),
            (WARBEAST, "trainable for war"),
            (HUNTBEAST, "trainable for hunting"),
            (MOUNT, "could be ridden"),
            (PETLIKE, "makes a pet"),
            (LIVESTOCK, "kept as livestock"),
            (GIANT, "a giant kind"),
            (GLOWS, "glows"),
        ];
        all.iter().filter(|(t, _)| self.has(*t)).map(|(_, n)| *n).collect()
    }

    /// One line about it for the look panel and the bestiary.
    pub fn summary(&self) -> String {
        let group = if self.group.1 > 1 { format!(", in groups of {}-{}", self.group.0, self.group.1) } else { String::new() };
        format!("A {} {} that {}{}; it {} with its {}.", self.rarity_name(), self.temper.name(), self.movement_words(), group, self.strike.verb(), self.strike.noun())
    }
}

use Class::*;
use Strike::*;
use Temper::*;

/// Every animal of the bestiary. Append only: saves hold indexes into it.
pub const BEASTS: &[Beast] = &[
    // ---- Temperate forest -------------------------------------------
    Beast::new("red fox", "red foxes", 'f', (210, 110, 50), 0.2, Mammal, Predator).lives(FOREST | GRASS | HILLS | SHRUB | TAIGA, 10, 70).hits(Bite, 3.0).traits(FIREFEAR | TAMEABLE).young("kit").note("Hunts rabbits and mice at the edges of woods."),
    Beast::new("badger", "badgers", 'b', (160, 160, 150), 0.25, Mammal, Predator).lives(FOREST | GRASS | HILLS, 15, 65).moves(WALK | BURROW).hits(Claw, 3.0).traits(NOCTURNAL).young("cub").note("Digs its setts under the roots of old trees."),
    Beast::new("giant badger", "giant badgers", 'B', (150, 150, 140), 1.2, Mammal, Territorial).lives(FOREST | GRASS | HILLS, 15, 60).moves(WALK | BURROW).hits(Claw, 8.0).traits(GIANT | NOCTURNAL).rare(4).young("cub").note("A badger the size of a bear, and as short-tempered as any badger."),
    Beast::new("wild boar", "wild boars", 'q', (110, 90, 70), 0.9, Mammal, Territorial).lives(FOREST | JUNGLE | HILLS | SHRUB, 20, 85).herd(2, 5).hits(Gore, 6.0).traits(TAMEABLE).young("piglet").note("Roots for acorns; the tusked males charge without warning."),
    Beast::new("rabbit", "rabbits", 'r', (170, 140, 110), 0.05, Mammal, Prey).lives(FOREST | GRASS | SHRUB | HILLS, 10, 75).moves(WALK | BURROW).herd(2, 6).speed(2).traits(TAMEABLE | PETLIKE).young("kit").note("Everything in the woods eats rabbits."),
    Beast::new("red squirrel", "red squirrels", 's', (200, 100, 50), 0.04, Mammal, Prey).lives(FOREST | TAIGA, 5, 65).moves(WALK | CLIMB).speed(2).young("kit").note("Chatters from the branches at anyone passing."),
    Beast::new("roe deer", "roe deer", 'd', (180, 130, 80), 0.5, Mammal, Grazer).lives(FOREST | HILLS | GRASS, 10, 65).herd(2, 4).speed(2).hits(Kick, 2.0).young("fawn").note("A small, shy deer of the woodland glades."),
    Beast::new("lynx", "lynxes", 'l', (190, 160, 120), 0.4, Mammal, Predator).lives(FOREST | TAIGA | MOUNTAIN, 0, 55).hits(Claw, 5.0).traits(NOCTURNAL | FIREFEAR).young("kitten").note("Tufted ears, great soft paws; it hunts hares in deep snow."),
    Beast::new("wildcat", "wildcats", 'c', (150, 130, 100), 0.15, Mammal, Predator).lives(FOREST | HILLS | SHRUB, 15, 70).hits(Claw, 2.0).traits(NOCTURNAL | TAMEABLE | PETLIKE).young("kitten").note("Like a house cat, but it will not be stroked."),
    Beast::new("hedgehog", "hedgehogs", 'h', (140, 120, 100), 0.03, Mammal, Prey).lives(FOREST | GRASS | SHRUB, 15, 70).speed(4).hits(Sting, 1.0).traits(NOCTURNAL | TAMEABLE | PETLIKE).young("hoglet").note("Rolls into a ball of spines when frightened."),
    Beast::new("porcupine", "porcupines", 'p', (120, 100, 80), 0.2, Mammal, Prey).lives(FOREST | TAIGA | SHRUB, 10, 70).moves(WALK | CLIMB).speed(4).hits(Sting, 3.0).young("porcupette").note("Its quills come out in whatever bites it."),
    Beast::new("tawny owl", "tawny owls", 'o', (170, 120, 70), 0.08, Bird, Predator).lives(FOREST | HILLS, 10, 70).moves(FLY | WALK).hits(Talons, 2.0).traits(NOCTURNAL | HUNTBEAST).young("owlet").note("Calls through the night; takes mice from the forest floor."),
    Beast::new("raven", "ravens", 'v', (40, 40, 60), 0.1, Bird, Scavenger).lives(FOREST | TAIGA | MOUNTAIN | TUNDRA | HILLS | BADLANDS, 0, 70).moves(FLY | WALK).herd(1, 3).hits(Peck, 1.5).traits(TAMEABLE).note("Clever, and first to any carcass."),
    Beast::new("woodpecker", "woodpeckers", 'w', (200, 60, 60), 0.03, Bird, Prey).lives(FOREST | TAIGA | JUNGLE, 5, 80).moves(FLY | WALK | CLIMB).hits(Peck, 1.0).note("Drums on dead trunks for grubs."),
    Beast::new("wild turkey", "wild turkeys", 't', (120, 90, 70), 0.3, Bird, Grazer).lives(FOREST | GRASS | SHRUB, 20, 75).herd(3, 8).hits(Peck, 1.0).traits(TAMEABLE).plus(EGGS).young("poult").note("Struts in flocks through open woodland."),
    Beast::new("pine marten", "pine martens", 'm', (120, 80, 50), 0.08, Mammal, Predator).lives(FOREST | TAIGA, 0, 55).moves(WALK | CLIMB).speed(2).hits(Bite, 2.0).young("kit").note("Chases squirrels from tree to tree."),
    Beast::new("box turtle", "box turtles", 't', (130, 120, 70), 0.05, Reptile, Prey).lives(FOREST | GRASS | SWAMP, 25, 80).speed(7).hits(Bite, 0.5).yields(MEAT | SHELL).traits(TAMEABLE | PETLIKE).note("Shuts itself inside its shell."),
    Beast::new("black bear", "black bears", 'B', (50, 45, 45), 1.2, Mammal, Territorial).lives(FOREST | HILLS | TAIGA, 10, 60).moves(WALK | CLIMB).hits(Claw, 8.0).traits(FIREFEAR).young("cub").note("Smaller than the brown bear, and fonder of berries."),
    Beast::new("pheasant", "pheasants", 'p', (190, 110, 50), 0.08, Bird, Prey).lives(FOREST | GRASS | SHRUB, 15, 70).moves(WALK | FLY).herd(1, 3).hits(Peck, 0.5).traits(TAMEABLE).plus(EGGS).note("Bursts from the undergrowth with a clatter of wings."),
    Beast::new("carrion crow", "carrion crows", 'v', (30, 30, 35), 0.06, Bird, Scavenger).lives(FOREST | GRASS | HILLS | SHRUB | MARSH, 5, 75).moves(FLY | WALK).herd(2, 6).hits(Peck, 1.0).note("Gathers where anything has died."),
    Beast::new("stag beetle", "stag beetles", 'b', (90, 50, 40), 0.02, Bug, Prey).lives(FOREST, 20, 70).speed(4).hits(Pinch, 0.5).note("Its great jaws are mostly for show."),
    Beast::new("grass snake", "grass snakes", 'n', (90, 140, 70), 0.06, Reptile, Prey).lives(FOREST | GRASS | MARSH | SWAMP, 15, 70).moves(WALK | SWIM).hits(Bite, 0.5).note("Harmless; plays dead when caught."),
    Beast::new("timber rattler", "timber rattlers", 'n', (140, 110, 60), 0.12, Reptile, Ambusher).lives(FOREST | HILLS | SHRUB, 20, 70).speed(4).hits(Bite, 4.0).traits(VENOMOUS).plus(VENOM).note("Lies coiled in leaf litter and rattles once."),
    Beast::new("aurochs", "aurochs", 'O', (70, 50, 40), 2.0, Mammal, Territorial).lives(FOREST | GRASS, 15, 55).herd(3, 8).hits(Gore, 10.0).traits(TAMEABLE | MOUNT).rare(4).young("calf").note("The great wild ox, black and long-horned."),
    Beast::new("forest bat", "forest bats", 'b', (90, 70, 60), 0.02, Mammal, Prey).lives(FOREST | JUNGLE | HILLS, 15, 85).moves(FLY).herd(3, 10).speed(1).traits(NOCTURNAL).note("Flits after moths at dusk."),
    // ---- Taiga --------------------------------------------------------
    Beast::new("moose", "moose", 'M', (90, 70, 50), 1.8, Mammal, Territorial).lives(TAIGA | TUNDRA | MARSH | FOREST, 0, 40).moves(WALK | SWIM).hits(Kick, 8.0).traits(MOUNT).young("calf").note("Tall as a wagon at the shoulder, and it does not give way."),
    Beast::new("reindeer", "reindeer", 'R', (160, 140, 120), 0.9, Mammal, Grazer).lives(TAIGA | TUNDRA | GLACIER, 0, 35).herd(4, 10).hits(Gore, 4.0).traits(TAMEABLE | MOUNT).plus(MILK | HORN).young("calf").note("Wanders the snows in long herds, digging for moss."),
    Beast::new("wolverine", "wolverines", 'W', (80, 60, 40), 0.45, Mammal, Predator).lives(TAIGA | TUNDRA | MOUNTAIN, 0, 35).hits(Claw, 6.0).young("kit").note("Will drive a bear off its kill."),
    Beast::new("snowshoe hare", "snowshoe hares", 'r', (230, 230, 225), 0.05, Mammal, Prey).lives(TAIGA | TUNDRA, 0, 40).herd(1, 4).speed(2).young("leveret").note("White in winter, brown in summer."),
    Beast::new("sable", "sables", 'm', (70, 50, 30), 0.06, Mammal, Predator).lives(TAIGA, 0, 35).moves(WALK | CLIMB).speed(2).hits(Bite, 1.5).young("kit").note("Prized for a pelt like dark water."),
    Beast::new("capercaillie", "capercaillies", 'g', (60, 70, 60), 0.15, Bird, Prey).lives(TAIGA | FOREST, 0, 45).moves(WALK | FLY).hits(Peck, 1.0).plus(EGGS).note("A great black grouse that clicks and pops in spring."),
    Beast::new("great grey owl", "great grey owls", 'o', (170, 170, 170), 0.12, Bird, Predator).lives(TAIGA | TUNDRA, 0, 40).moves(FLY | WALK).hits(Talons, 2.0).traits(HUNTBEAST).young("owlet").note("Hears voles moving under the snow."),
    Beast::new("amur tiger", "amur tigers", 'T', (220, 140, 60), 1.5, Mammal, Predator).lives(TAIGA | FOREST, 0, 40).hits(Claw, 11.0).traits(FIREFEAR).rare(4).young("cub").note("The great tiger of the snow forests."),
    Beast::new("giant wolverine", "giant wolverines", 'W', (70, 50, 35), 1.1, Mammal, Predator).lives(TAIGA | TUNDRA, 0, 35).hits(Claw, 9.0).traits(GIANT).rare(4).young("kit").note("Every bit as savage as the small kind, and four times the size."),
    Beast::new("beaver", "beavers", 'b', (130, 90, 60), 0.25, Mammal, Prey).lives(FOREST | TAIGA | RIVER | LAKE, 0, 60).moves(WALK | SWIM).speed(4).hits(Bite, 2.0).young("kit").note("Fells trees and dams streams."),
    Beast::new("moss ox", "moss oxen", 'O', (90, 120, 70), 1.7, Strange, Grazer).lives(TAIGA, 0, 35).herd(2, 6).speed(4).hits(Butt, 6.0).traits(TAMEABLE | MOUNT).plus(WOOL).rare(4).young("calf").note("So slow that moss grows thick on its back; it eats what grows on the others."),
    // ---- Jungle -------------------------------------------------------
    Beast::new("tiger", "tigers", 'T', (240, 150, 50), 1.4, Mammal, Predator).lives(JUNGLE | SWAMP | FOREST, 50, 100).moves(WALK | SWIM).hits(Claw, 10.0).traits(FIREFEAR).rare(3).young("cub").note("Striped like the light through leaves; it hunts alone."),
    Beast::new("tapir", "tapirs", 'q', (70, 70, 80), 1.0, Mammal, Grazer).lives(JUNGLE | SWAMP | RIVER, 55, 100).moves(WALK | SWIM).hits(Bite, 2.0).young("calf").note("A shy browser with a short trunk."),
    Beast::new("capybara", "capybaras", 'q', (150, 110, 70), 0.4, Mammal, Grazer).lives(JUNGLE | SWAMP | SAVANNA | RIVER | LAKE, 50, 100).moves(WALK | SWIM).herd(3, 8).hits(Bite, 1.5).traits(TAMEABLE | PETLIKE).young("pup").note("The largest rodent, and the calmest."),
    Beast::new("howler monkey", "howler monkeys", 'y', (120, 70, 40), 0.1, Mammal, Prey).lives(JUNGLE, 55, 100).moves(WALK | CLIMB).herd(3, 8).hits(Bite, 1.0).young("infant").note("Its roar carries for miles at dawn."),
    Beast::new("macaque", "macaques", 'y', (180, 150, 110), 0.08, Mammal, Vermin).lives(JUNGLE | FOREST | MOUNTAIN, 35, 100).moves(WALK | CLIMB).herd(3, 8).speed(2).hits(Bite, 1.5).young("infant").note("Bold thieves of anything edible."),
    Beast::new("orangutan", "orangutans", 'Y', (200, 100, 50), 0.7, Mammal, Prey).lives(JUNGLE, 60, 100).moves(WALK | CLIMB).speed(4).hits(Slam, 4.0).rare(3).young("infant").note("A thoughtful red ape of the canopy."),
    Beast::new("gorilla", "gorillas", 'Y', (50, 50, 55), 1.5, Mammal, Territorial).lives(JUNGLE | MOUNTAIN, 55, 100).herd(2, 6).hits(Slam, 9.0).rare(4).young("infant").note("Peaceful unless its troop is threatened."),
    Beast::new("scarlet macaw", "scarlet macaws", 'p', (230, 40, 40), 0.05, Bird, Prey).lives(JUNGLE | SAVANNA, 60, 100).moves(FLY | WALK | CLIMB).herd(2, 6).hits(Peck, 1.5).traits(TAMEABLE | PETLIKE).note("Shrieks in flocks of red, blue and gold."),
    Beast::new("toucan", "toucans", 'p', (250, 170, 40), 0.04, Bird, Prey).lives(JUNGLE, 60, 100).moves(FLY | WALK).herd(1, 4).hits(Peck, 1.0).note("Its great bright bill weighs almost nothing."),
    Beast::new("python", "pythons", 'S', (150, 130, 80), 0.6, Reptile, Ambusher).lives(JUNGLE | SWAMP | SAVANNA, 55, 100).moves(WALK | SWIM | CLIMB).speed(4).hits(Constrict, 7.0).rare(3).note("Lies in wait for days, then coils and squeezes."),
    Beast::new("king cobra", "king cobras", 'n', (90, 90, 50), 0.15, Reptile, Ambusher).lives(JUNGLE | FOREST, 55, 100).hits(Bite, 5.0).traits(VENOMOUS).plus(VENOM).rare(3).note("Rears its hood as high as a dwarf's head."),
    Beast::new("poison frog", "poison frogs", 'f', (60, 160, 230), 0.01, Amphibian, Prey).lives(JUNGLE, 60, 100).hits(Bite, 0.2).traits(VENOMOUS).plus(VENOM).note("Bright as a jewel; its skin is death to touch."),
    Beast::new("leopard", "leopards", 'L', (220, 180, 90), 0.7, Mammal, Predator).lives(JUNGLE | SAVANNA | MOUNTAIN | FOREST, 40, 100).moves(WALK | CLIMB).hits(Claw, 7.0).traits(FIREFEAR | NOCTURNAL).rare(3).young("cub").note("Drags its kills up into the trees."),
    Beast::new("okapi", "okapis", 'G', (110, 60, 50), 1.0, Mammal, Grazer).lives(JUNGLE, 60, 100).hits(Kick, 3.0).rare(3).young("calf").note("Striped legs like a zebra, a neck like a short giraffe."),
    Beast::new("junglefowl", "junglefowl", 'g', (200, 80, 40), 0.06, Bird, Prey).lives(JUNGLE | FOREST, 50, 100).moves(WALK | FLY).herd(2, 6).hits(Peck, 1.0).traits(TAMEABLE).plus(EGGS).note("The wild bird every farmyard hen came from."),
    Beast::new("giant centipede", "giant centipedes", 'x', (170, 70, 40), 0.3, Bug, Ambusher).lives(JUNGLE | CAVE1, 50, 100).moves(WALK | CLIMB).speed(2).hits(Bite, 4.0).traits(VENOMOUS | GIANT | NOCTURNAL).plus(VENOM).rare(3).note("As long as a dwarf's arm, all legs and fangs."),
    Beast::new("army ant", "army ants", 'a', (120, 60, 40), 0.01, Bug, Vermin).lives(JUNGLE | SAVANNA, 60, 100).herd(5, 10).hits(Sting, 0.3).note("A marching column eats everything in its path."),
    Beast::new("sloth", "sloths", 'y', (130, 120, 90), 0.3, Mammal, Prey).lives(JUNGLE, 60, 100).moves(WALK | CLIMB | SWIM).speed(8).hits(Claw, 2.0).young("infant").note("Hangs so still that green things grow in its fur."),
    Beast::new("anteater", "anteaters", 'A', (140, 120, 100), 0.5, Mammal, Prey).lives(JUNGLE | SAVANNA | GRASS, 50, 100).hits(Claw, 4.0).young("pup").note("Its claws open termite mounds and, if pressed, jaguars."),
    Beast::new("harpy eagle", "harpy eagles", 'E', (180, 180, 180), 0.15, Bird, Predator).lives(JUNGLE, 60, 100).moves(FLY | WALK).hits(Talons, 4.0).traits(HUNTBEAST).rare(3).young("eaglet").note("Snatches monkeys from the canopy."),
    Beast::new("bird-eating spider", "bird-eating spiders", 'x', (90, 70, 60), 0.03, Bug, Ambusher).lives(JUNGLE | SAVANNA, 60, 100).moves(WALK | CLIMB).hits(Bite, 2.0).traits(VENOMOUS | NOCTURNAL).plus(VENOM | SILK).note("A hairy spider as wide as a hand."),
    Beast::new("caiman", "caimans", 'C', (100, 110, 80), 0.8, Reptile, Ambusher).lives(JUNGLE | SWAMP | RIVER | LAKE, 55, 100).moves(WALK | SWIM).hits(Bite, 7.0).note("Floats like a log in brown water."),
    Beast::new("crocodile", "crocodiles", 'C', (90, 110, 70), 1.7, Reptile, Ambusher).lives(JUNGLE | SAVANNA | SWAMP | RIVER | LAKE, 55, 100).moves(WALK | SWIM).speed(4).hits(Bite, 12.0).rare(3).note("Waits at the drinking places for something to stoop."),
    // ---- Savanna, grassland and shrubland -------------------------------
    Beast::new("bison", "bison", 'O', (100, 70, 50), 1.8, Mammal, Grazer).lives(GRASS | SHRUB | FOREST, 5, 60).herd(5, 12).hits(Gore, 7.0).traits(TAMEABLE).plus(WOOL | HORN).young("calf").note("The plains go dark with them."),
    Beast::new("pronghorn", "pronghorns", 'a', (200, 150, 90), 0.5, Mammal, Grazer).lives(GRASS | SHRUB | DESERT, 15, 70).herd(3, 8).speed(1).hits(Butt, 2.0).plus(HORN).young("fawn").note("Faster over open ground than anything that hunts it."),
    Beast::new("prairie dog", "prairie dogs", 'r', (190, 160, 110), 0.03, Mammal, Prey).lives(GRASS | SHRUB, 15, 70).moves(WALK | BURROW).herd(4, 10).hits(Bite, 0.5).young("pup").note("Whole towns of them yip warnings from their mounds."),
    Beast::new("coyote", "coyotes", 'j', (180, 150, 110), 0.3, Mammal, Predator).lives(GRASS | SHRUB | DESERT | BADLANDS | FOREST, 15, 85).herd(1, 3).hits(Bite, 3.5).traits(NOCTURNAL | FIREFEAR).young("pup").note("Sings at night and steals what it can."),
    Beast::new("wild horse", "wild horses", 'H', (140, 100, 70), 1.3, Mammal, Grazer).lives(GRASS | SHRUB | SAVANNA, 10, 70).herd(3, 8).speed(1).hits(Kick, 6.0).traits(TAMEABLE | MOUNT | WARBEAST).young("foal").note("Runs before the wind across the steppe."),
    Beast::new("zebra", "zebras", 'H', (230, 230, 230), 1.2, Mammal, Grazer).lives(SAVANNA | GRASS, 55, 100).herd(4, 10).speed(2).hits(Kick, 6.0).young("foal").note("Its stripes dazzle the eye of a charging lion."),
    Beast::new("wildebeest", "wildebeest", 'O', (110, 100, 100), 1.1, Mammal, Grazer).lives(SAVANNA | GRASS, 55, 100).herd(6, 12).speed(2).hits(Gore, 5.0).plus(HORN).young("calf").note("Follows the rains in herds beyond counting."),
    Beast::new("giraffe", "giraffes", 'G', (220, 170, 90), 2.5, Mammal, Grazer).lives(SAVANNA, 55, 100).herd(2, 5).hits(Kick, 9.0).rare(3).young("calf").note("Browses the tops of acacias; its kick breaks lions."),
    Beast::new("elephant", "elephants", 'E', (150, 150, 150), 4.5, Mammal, Territorial).lives(SAVANNA | JUNGLE | GRASS, 55, 100).herd(3, 6).speed(3).hits(Trample, 15.0).traits(TAMEABLE | MOUNT | WARBEAST).plus(IVORY).rare(4).young("calf").note("The largest beast that walks; it remembers."),
    Beast::new("rhinoceros", "rhinoceroses", 'R', (130, 130, 120), 2.6, Mammal, Territorial).lives(SAVANNA | GRASS | SHRUB, 50, 100).hits(Gore, 12.0).plus(HORN).rare(4).young("calf").note("Near-blind and quick to charge what it can't see."),
    Beast::new("hippopotamus", "hippopotamuses", 'H', (130, 100, 110), 3.0, Mammal, Territorial).lives(SAVANNA | SWAMP | RIVER | LAKE, 55, 100).moves(WALK | SWIM).herd(2, 6).hits(Bite, 13.0).plus(IVORY).rare(3).young("calf").note("Grazes by night and guards its river by day."),
    Beast::new("cheetah", "cheetahs", 'c', (230, 190, 100), 0.45, Mammal, Predator).lives(SAVANNA | GRASS | SHRUB, 50, 100).speed(1).hits(Claw, 5.0).traits(HUNTBEAST | TAMEABLE).rare(3).young("cub").note("The fastest runner of all, for a short way."),
    Beast::new("spotted hyena", "spotted hyenas", 'j', (180, 150, 100), 0.6, Mammal, Scavenger).lives(SAVANNA | SHRUB | DESERT, 45, 100).herd(3, 6).hits(Bite, 7.0).traits(NOCTURNAL).young("cub").note("Cackles over carcasses; its jaws crack bone."),
    Beast::new("griffon vulture", "griffon vultures", 'V', (190, 170, 140), 0.25, Bird, Scavenger).lives(SAVANNA | DESERT | BADLANDS | MOUNTAIN | SHRUB, 30, 100).moves(FLY | WALK).herd(2, 6).hits(Peck, 2.0).note("Circles high, and drops to wherever death is."),
    Beast::new("ostrich", "ostriches", 'O', (90, 80, 80), 1.0, Bird, Prey).lives(SAVANNA | DESERT | SHRUB, 50, 100).herd(1, 5).speed(1).hits(Kick, 7.0).traits(TAMEABLE | MOUNT).plus(EGGS).young("chick").note("Cannot fly, and need not: nothing outruns it."),
    Beast::new("warthog", "warthogs", 'q', (120, 100, 90), 0.6, Mammal, Territorial).lives(SAVANNA | GRASS, 50, 100).herd(2, 5).hits(Gore, 4.0).plus(IVORY).young("piglet").note("Kneels to graze, and runs with its tail straight up."),
    Beast::new("meerkat", "meerkats", 'r', (200, 170, 120), 0.02, Mammal, Prey).lives(SAVANNA | DESERT | SHRUB, 50, 100).moves(WALK | BURROW).herd(4, 10).hits(Bite, 0.5).young("pup").note("One stands sentry while the rest dig."),
    Beast::new("aardvark", "aardvarks", 'A', (170, 140, 120), 0.5, Mammal, Prey).lives(SAVANNA | GRASS, 50, 100).moves(WALK | BURROW).hits(Claw, 3.0).traits(NOCTURNAL).young("calf").note("Digs faster than a team of miners."),
    Beast::new("secretary bird", "secretary birds", 'e', (200, 200, 200), 0.12, Bird, Predator).lives(SAVANNA | GRASS, 50, 100).moves(WALK | FLY).hits(Kick, 3.0).note("Stamps snakes to death on long legs."),
    Beast::new("cape buffalo", "cape buffalo", 'O', (60, 50, 50), 2.0, Mammal, Territorial).lives(SAVANNA | SWAMP | GRASS, 50, 100).herd(4, 10).hits(Gore, 11.0).plus(HORN).young("calf").note("Lions think twice; hunters think three times."),
    Beast::new("gazelle", "gazelles", 'a', (210, 170, 110), 0.35, Mammal, Grazer).lives(SAVANNA | DESERT | SHRUB | GRASS, 45, 100).herd(4, 10).speed(1).hits(Butt, 1.5).plus(HORN).young("fawn").note("Leaps straight up when a hunter is near."),
    Beast::new("black mamba", "black mambas", 'n', (100, 100, 90), 0.15, Reptile, Ambusher).lives(SAVANNA | SHRUB | JUNGLE, 55, 100).speed(1).hits(Bite, 6.0).traits(VENOMOUS).plus(VENOM).rare(3).note("Fast as a running dwarf, and deadly."),
    Beast::new("skylark", "skylarks", 'p', (170, 150, 110), 0.02, Bird, Prey).lives(GRASS | SHRUB | TUNDRA, 5, 75).moves(FLY | WALK).herd(1, 4).hits(Peck, 0.2).note("Sings as it climbs straight up into the sky."),
    Beast::new("quail", "quail", 'g', (160, 130, 100), 0.03, Bird, Prey).lives(GRASS | SHRUB | SAVANNA | DESERT, 20, 90).herd(3, 8).hits(Peck, 0.2).traits(TAMEABLE).plus(EGGS).note("Runs through the grass in coveys."),
    Beast::new("brown hare", "brown hares", 'r', (170, 130, 90), 0.06, Mammal, Prey).lives(GRASS | SHRUB | HILLS, 10, 70).speed(1).young("leveret").note("Boxes with its rivals in spring."),
    Beast::new("kestrel", "kestrels", 'k', (190, 120, 70), 0.03, Bird, Predator).lives(GRASS | SHRUB | HILLS | FOREST | BADLANDS, 10, 85).moves(FLY | WALK).hits(Talons, 1.0).traits(HUNTBEAST).note("Hangs on the wind, then drops onto a mouse."),
    Beast::new("onager", "onagers", 'H', (190, 170, 140), 1.0, Mammal, Grazer).lives(DESERT | SHRUB | GRASS, 30, 90).herd(3, 8).speed(2).hits(Kick, 5.0).traits(TAMEABLE | MOUNT).young("foal").note("The wild ass of the dry steppe."),
    Beast::new("saiga", "saigas", 'a', (200, 180, 140), 0.4, Mammal, Grazer).lives(GRASS | SHRUB | DESERT, 5, 65).herd(4, 10).speed(2).hits(Butt, 1.5).plus(HORN).young("calf").note("Its swollen nose warms the winter air."),
    Beast::new("locust", "locusts", 'i', (160, 170, 60), 0.01, Bug, Vermin).lives(GRASS | SAVANNA | SHRUB | DESERT, 40, 100).moves(WALK | FLY).herd(6, 12).speed(2).hits(Bite, 0.1).note("A swarm strips a field in a day."),
    // ---- Desert and badlands ------------------------------------------
    Beast::new("fennec fox", "fennec foxes", 'f', (230, 210, 160), 0.06, Mammal, Predator).lives(DESERT, 55, 100).moves(WALK | BURROW).hits(Bite, 1.5).traits(NOCTURNAL | TAMEABLE | PETLIKE).young("kit").note("Its huge ears hear beetles under the sand."),
    Beast::new("jackrabbit", "jackrabbits", 'r', (180, 160, 120), 0.07, Mammal, Prey).lives(DESERT | SHRUB | BADLANDS | GRASS, 30, 95).speed(1).young("leveret").note("Long ears that shed the desert heat."),
    Beast::new("sidewinder", "sidewinders", 'n', (210, 190, 140), 0.07, Reptile, Ambusher).lives(DESERT, 55, 100).hits(Bite, 4.0).traits(VENOMOUS | NOCTURNAL).plus(VENOM).note("Throws itself sideways over the dunes."),
    Beast::new("gila monster", "gila monsters", 'l', (220, 120, 90), 0.08, Reptile, Ambusher).lives(DESERT | SHRUB | BADLANDS, 50, 100).speed(5).hits(Bite, 3.0).traits(VENOMOUS).plus(VENOM).note("Beaded pink and black; once it bites it chews."),
    Beast::new("desert tortoise", "desert tortoises", 't', (160, 140, 100), 0.1, Reptile, Prey).lives(DESERT | SHRUB, 45, 100).moves(WALK | BURROW).speed(8).hits(Bite, 0.5).yields(MEAT | SHELL).traits(TAMEABLE | PETLIKE).note("Digs a cool den for the hottest months."),
    Beast::new("roadrunner", "roadrunners", 'e', (150, 130, 100), 0.04, Bird, Predator).lives(DESERT | SHRUB | BADLANDS, 40, 100).speed(1).hits(Peck, 1.5).note("Runs down lizards and young snakes."),
    Beast::new("horned lizard", "horned lizards", 'l', (190, 160, 120), 0.01, Reptile, Prey).lives(DESERT | BADLANDS | SHRUB, 40, 100).hits(Bite, 0.2).note("Squirts blood from its eyes at foxes."),
    Beast::new("darkling beetle", "darkling beetles", 'b', (40, 40, 40), 0.01, Bug, Prey).lives(DESERT | BADLANDS | SHRUB, 30, 100).speed(4).hits(Bite, 0.1).note("Drinks the fog that rolls in from the sea."),
    Beast::new("sand cat", "sand cats", 'c', (220, 200, 150), 0.08, Mammal, Predator).lives(DESERT, 50, 100).hits(Claw, 1.5).traits(NOCTURNAL | TAMEABLE | PETLIKE).young("kitten").note("Never drinks; it gets its water from its prey."),
    Beast::new("addax", "addaxes", 'a', (230, 220, 200), 0.8, Mammal, Grazer).lives(DESERT, 55, 100).herd(2, 6).hits(Gore, 3.0).plus(HORN).rare(3).young("calf").note("Pale, spiral-horned, and at home in the deep sand."),
    Beast::new("oryx", "oryxes", 'a', (210, 200, 180), 0.9, Mammal, Grazer).lives(DESERT | SAVANNA | SHRUB, 50, 100).herd(3, 8).hits(Gore, 6.0).plus(HORN).young("calf").note("Its straight horns are longer than a spear."),
    Beast::new("rattlesnake", "rattlesnakes", 'n', (170, 140, 90), 0.1, Reptile, Ambusher).lives(DESERT | BADLANDS | SHRUB | GRASS | HILLS, 30, 100).hits(Bite, 5.0).traits(VENOMOUS).plus(VENOM).note("Warns once with its rattle."),
    Beast::new("bighorn sheep", "bighorn sheep", 'i', (160, 130, 100), 0.8, Mammal, Grazer).lives(MOUNTAIN | HILLS | BADLANDS, 5, 75).herd(3, 8).hits(Butt, 6.0).plus(HORN).young("lamb").note("The rams crash their horns together on the crags."),
    Beast::new("tarantula", "tarantulas", 'x', (100, 70, 50), 0.02, Bug, Ambusher).lives(DESERT | BADLANDS | SHRUB | SAVANNA, 45, 100).moves(WALK | BURROW).speed(4).hits(Bite, 1.0).traits(VENOMOUS | NOCTURNAL).plus(SILK).note("Waits at the mouth of its silk-lined burrow."),
    Beast::new("dromedary", "dromedaries", 'C', (210, 170, 110), 1.3, Mammal, Grazer).lives(0, 0, 100).hits(Kick, 4.0).traits(TAMEABLE | MOUNT | LIVESTOCK).plus(MILK | WOOL).young("calf").note("A camel bred by caravan folk; it carries and gives milk."),
    Beast::new("dune strider", "dune striders", 'D', (230, 200, 140), 1.6, Strange, Grazer).lives(DESERT, 60, 100).herd(2, 5).speed(2).hits(Kick, 6.0).traits(TAMEABLE | MOUNT).rare(4).young("foal").note("Stilt-legged and humpless; it walks the hottest sand on broad padded feet and drinks once a season."),
    Beast::new("glasswing", "glasswings", '¥', (180, 230, 230), 0.1, Strange, Prey).lives(DESERT | BADLANDS, 50, 100).moves(FLY).herd(3, 8).speed(1).hits(Bite, 0.5).yields(CHITIN).rare(4).note("A flier whose wings are clear as cut crystal; flocks of them throw rainbows on the dunes."),
    // ---- Swamp and marsh ----------------------------------------------
    Beast::new("leech", "leeches", '~', (80, 60, 50), 0.01, Worm, Prey).lives(SWAMP | MARSH | RIVER | LAKE, 20, 100).moves(SWIM).speed(4).hits(Bite, 0.2).note("Waits in the shallows for a warm leg."),
    Beast::new("snapping turtle", "snapping turtles", 't', (90, 90, 60), 0.3, Reptile, Ambusher).lives(SWAMP | MARSH | RIVER | LAKE, 25, 90).moves(WALK | SWIM).speed(5).hits(Bite, 5.0).yields(MEAT | SHELL).plus(BONE).note("Its beak can take off a toe."),
    Beast::new("grey heron", "grey herons", 'e', (160, 160, 170), 0.1, Bird, Predator).lives(MARSH | SWAMP | RIVER | LAKE | COAST, 10, 85).moves(WALK | FLY).hits(Peck, 2.0).note("Stands motionless in the shallows for fish."),
    Beast::new("common crane", "common cranes", 'e', (180, 180, 190), 0.3, Bird, Prey).lives(MARSH | GRASS | TUNDRA | LAKE, 5, 70).moves(WALK | FLY).herd(2, 6).hits(Peck, 2.0).note("Dances in spring with wings spread wide."),
    Beast::new("bullfrog", "bullfrogs", 'f', (90, 130, 60), 0.02, Amphibian, Prey).lives(SWAMP | MARSH | RIVER | LAKE | FOREST, 20, 90).moves(WALK | SWIM).hits(Bite, 0.3).note("Its croak sounds like a cow far off."),
    Beast::new("giant toad", "giant toads", 'F', (130, 120, 70), 0.6, Amphibian, Ambusher).lives(SWAMP | MARSH | JUNGLE | CAVE1, 25, 100).moves(WALK | SWIM).speed(4).hits(Bite, 4.0).traits(GIANT | NOCTURNAL).plus(VENOM).rare(3).note("Swallows anything that fits, and much that doesn't."),
    Beast::new("cottonmouth", "cottonmouths", 'n', (70, 70, 60), 0.1, Reptile, Ambusher).lives(SWAMP | MARSH | RIVER, 40, 95).moves(WALK | SWIM).hits(Bite, 5.0).traits(VENOMOUS).plus(VENOM).note("Gapes its white mouth before it strikes."),
    Beast::new("muskrat", "muskrats", 'r', (110, 80, 50), 0.03, Mammal, Prey).lives(SWAMP | MARSH | RIVER | LAKE, 5, 80).moves(WALK | SWIM).herd(1, 3).hits(Bite, 0.5).young("kit").note("Builds lodges of reeds in the marsh."),
    Beast::new("river otter", "river otters", 'o', (110, 80, 60), 0.15, Mammal, Predator).lives(RIVER | LAKE | MARSH | FOREST, 5, 75).moves(WALK | SWIM).herd(1, 3).hits(Bite, 2.0).traits(TAMEABLE | PETLIKE).young("pup").note("Plays in the rapids and fishes in the pools."),
    Beast::new("marsh harrier", "marsh harriers", 'k', (150, 110, 70), 0.06, Bird, Predator).lives(MARSH | SWAMP | GRASS, 15, 80).moves(FLY | WALK).hits(Talons, 1.5).note("Quarters low over the reedbeds."),
    Beast::new("nutria", "nutrias", 'r', (120, 90, 60), 0.08, Mammal, Vermin).lives(SWAMP | MARSH | RIVER, 30, 90).moves(WALK | SWIM).herd(2, 5).hits(Bite, 1.0).young("kit").note("Orange teeth; eats crops down to the root."),
    Beast::new("mosquito", "mosquitoes", 'i', (100, 90, 80), 0.01, Bug, Prey).lives(SWAMP | MARSH | JUNGLE | TUNDRA, 15, 100).moves(FLY).herd(6, 12).speed(1).hits(Sting, 0.1).note("A whining cloud over the still water."),
    Beast::new("bog snail", "bog snails", 's', (150, 130, 90), 0.01, Mollusc, Prey).lives(SWAMP | MARSH, 15, 85).speed(9).hits(Bite, 0.1).note("Leaves silver trails across the moss."),
    Beast::new("mirebeast", "mirebeasts", 'Ö', (100, 110, 70), 2.2, Strange, Ambusher).lives(SWAMP, 30, 100).moves(WALK | SWIM).speed(5).hits(Lash, 10.0).traits(NOCTURNAL).rare(5).note("A heap of weed-grown hide that lies in the mud like an island, until something steps on it."),
    Beast::new("bellow toad", "bellow toads", 'F', (140, 150, 70), 0.9, Strange, Territorial).lives(MARSH | SWAMP, 20, 90).moves(WALK | SWIM).speed(4).hits(Slam, 5.0).rare(4).note("Its throat sac swells to the size of a barrel; its bellow can be heard a mile off."),
    // ---- Tundra and glacier -------------------------------------------
    Beast::new("musk ox", "musk oxen", 'O', (80, 60, 50), 1.3, Mammal, Territorial).lives(TUNDRA | GLACIER, 0, 30).herd(4, 10).hits(Gore, 8.0).traits(TAMEABLE).plus(WOOL | HORN).young("calf").note("Forms a ring of horns around its young."),
    Beast::new("arctic fox", "arctic foxes", 'f', (240, 240, 245), 0.08, Mammal, Predator).lives(TUNDRA | GLACIER | TAIGA | COAST, 0, 30).moves(WALK | BURROW).hits(Bite, 1.5).traits(TAMEABLE).young("kit").note("Follows the bears and eats their leavings."),
    Beast::new("lemming", "lemmings", 'r', (170, 130, 90), 0.01, Mammal, Prey).lives(TUNDRA | TAIGA, 0, 35).moves(WALK | BURROW).herd(4, 10).hits(Bite, 0.1).young("pup").note("Some years there are a hundred under every stone."),
    Beast::new("snowy owl", "snowy owls", 'o', (245, 245, 245), 0.1, Bird, Predator).lives(TUNDRA | GLACIER | TAIGA, 0, 35).moves(FLY | WALK).hits(Talons, 2.0).traits(HUNTBEAST).young("owlet").note("Hunts by day through the long summer light."),
    Beast::new("ptarmigan", "ptarmigans", 'g', (235, 235, 230), 0.03, Bird, Prey).lives(TUNDRA | MOUNTAIN | GLACIER, 0, 35).moves(WALK | FLY).herd(2, 8).hits(Peck, 0.2).plus(EGGS).note("Feathered to the toes for the snow."),
    Beast::new("arctic hare", "arctic hares", 'r', (240, 240, 240), 0.07, Mammal, Prey).lives(TUNDRA | GLACIER, 0, 25).herd(1, 6).speed(1).young("leveret").note("Stands on its hind legs to look over the drifts."),
    Beast::new("walrus", "walruses", 'W', (170, 120, 100), 2.0, Mammal, Territorial).lives(COAST | GLACIER | SEA, 0, 30).moves(WALK | SWIM).herd(2, 8).speed(5).hits(Gore, 9.0).plus(IVORY).young("calf").note("Hauls out on the ice with its tusks."),
    Beast::new("harbour seal", "harbour seals", 'S', (130, 130, 120), 0.6, Mammal, Prey).lives(COAST | SEA, 0, 55).moves(WALK | SWIM).herd(2, 8).speed(5).hits(Bite, 2.0).young("pup").note("Lies on the rocks at low tide."),
    Beast::new("penguin", "penguins", 'p', (40, 40, 50), 0.2, Bird, Prey).lives(COAST | GLACIER | SEA, 0, 35).moves(WALK | SWIM).herd(4, 12).speed(4).hits(Peck, 1.0).yields(MEAT | FAT | BONE | FEATHERS).plus(EGGS).note("Flies only underwater."),
    Beast::new("woolly mammoth", "woolly mammoths", 'Ш', (130, 90, 60), 5.0, Mammal, Territorial).lives(TUNDRA | GLACIER | TAIGA, 0, 25).herd(2, 5).hits(Trample, 16.0).traits(MOUNT).plus(WOOL | IVORY).rare(5).young("calf").note("Shaggy, tusked, older than the hills."),
    Beast::new("arctic tern", "arctic terns", 'p', (230, 230, 230), 0.02, Bird, Prey).lives(TUNDRA | COAST | GLACIER, 0, 40).moves(FLY | WALK | SWIM).herd(3, 8).speed(1).hits(Peck, 0.5).plus(EGGS).note("Follows the summer from one end of the world to the other."),
    Beast::new("frostmane", "frostmanes", 'Λ', (210, 230, 250), 1.1, Strange, Predator).lives(TUNDRA | GLACIER | MOUNTAIN, 0, 25).herd(2, 4).speed(2).hits(Bite, 9.0).traits(FIREFEAR).plus(WOOL).rare(4).young("cub").note("A long-legged pack hunter with a lion's ruff of rimed white hair; its breath freezes in its mane."),
    Beast::new("rime crawler", "rime crawlers", '~', (200, 220, 240), 0.4, Strange, Prey).lives(GLACIER, 0, 15).moves(WALK | BURROW).speed(6).hits(Bite, 1.0).yields(MEAT | FAT).rare(4).note("A pale, blind worm-thing as long as a dwarf that bores tunnels through old ice."),
    // ---- Mountains and hills ------------------------------------------
    Beast::new("mountain goat", "mountain goats", 'g', (240, 240, 230), 0.6, Mammal, Grazer).lives(MOUNTAIN | HILLS | GLACIER, 0, 55).moves(WALK | CLIMB).herd(2, 6).hits(Butt, 4.0).traits(TAMEABLE).plus(WOOL | MILK | HORN).young("kid").note("Stands on ledges a dwarf could not."),
    Beast::new("chamois", "chamois", 'g', (150, 110, 70), 0.4, Mammal, Grazer).lives(MOUNTAIN | HILLS, 0, 55).moves(WALK | CLIMB).herd(2, 6).speed(2).hits(Butt, 2.0).plus(HORN).young("kid").note("Its hooked horns are finer than any goat's."),
    Beast::new("snow leopard", "snow leopards", 'L', (220, 220, 210), 0.6, Mammal, Predator).lives(MOUNTAIN | GLACIER, 0, 40).moves(WALK | CLIMB).hits(Claw, 7.0).traits(FIREFEAR).rare(4).young("cub").note("Grey ghost of the high passes."),
    Beast::new("golden eagle", "golden eagles", 'E', (170, 120, 60), 0.1, Bird, Predator).lives(MOUNTAIN | HILLS | GRASS | TUNDRA | BADLANDS, 0, 80).moves(FLY | WALK).hits(Talons, 3.0).traits(HUNTBEAST).young("eaglet").note("Takes hares, and sometimes lambs."),
    Beast::new("giant eagle", "giant eagles", 'E', (190, 140, 70), 1.4, Bird, Predator).lives(MOUNTAIN | HILLS, 0, 60).moves(FLY | WALK).hits(Talons, 10.0).traits(GIANT | MOUNT).rare(4).young("eaglet").note("Its wings blot out the sun; it has carried off oxen."),
    Beast::new("marmot", "marmots", 'r', (150, 120, 80), 0.06, Mammal, Prey).lives(MOUNTAIN | HILLS | GRASS, 0, 50).moves(WALK | BURROW).herd(2, 6).hits(Bite, 0.5).young("pup").note("Whistles from the boulders when an eagle passes."),
    Beast::new("pika", "pikas", 'r', (160, 140, 120), 0.01, Mammal, Prey).lives(MOUNTAIN, 0, 40).moves(WALK | BURROW).herd(2, 6).hits(Bite, 0.1).young("pup").note("Dries hay among the rocks for winter."),
    Beast::new("condor", "condors", 'V', (40, 40, 40), 0.3, Bird, Scavenger).lives(MOUNTAIN | HILLS | COAST | BADLANDS, 10, 80).moves(FLY | WALK).hits(Peck, 2.5).rare(3).note("Glides the length of a range without a wingbeat."),
    Beast::new("bearded vulture", "bearded vultures", 'V', (200, 140, 90), 0.2, Bird, Scavenger).lives(MOUNTAIN | HILLS, 0, 60).moves(FLY | WALK).hits(Peck, 2.0).rare(3).note("Drops bones on rocks to get at the marrow."),
    Beast::new("cougar", "cougars", 'L', (190, 150, 100), 0.6, Mammal, Predator).lives(MOUNTAIN | HILLS | FOREST | SHRUB | BADLANDS, 10, 80).moves(WALK | CLIMB).hits(Claw, 7.0).traits(FIREFEAR | NOCTURNAL).young("cub").note("Follows a lone traveller for miles."),
    Beast::new("yak", "yaks", 'Y', (60, 50, 50), 1.6, Mammal, Grazer).lives(MOUNTAIN | TUNDRA | GLACIER, 0, 40).herd(3, 8).speed(4).hits(Gore, 7.0).traits(TAMEABLE | MOUNT | LIVESTOCK).plus(WOOL | MILK | HORN).young("calf").note("Shaggy, sure-footed beast of the highest pastures."),
    Beast::new("llama", "llamas", 'L', (220, 200, 170), 1.0, Mammal, Grazer).lives(MOUNTAIN | GRASS | HILLS, 10, 60).herd(3, 8).hits(Kick, 3.0).traits(TAMEABLE | MOUNT | LIVESTOCK).plus(WOOL).young("cria").note("Carries loads over the passes, and spits."),
    Beast::new("red kite", "red kites", 'k', (190, 90, 50), 0.05, Bird, Scavenger).lives(HILLS | GRASS | FOREST, 10, 70).moves(FLY | WALK).hits(Peck, 1.0).note("Its forked tail turns it over the fields."),
    Beast::new("stoat", "stoats", 'm', (190, 140, 90), 0.02, Mammal, Predator).lives(HILLS | GRASS | FOREST | TUNDRA, 0, 60).speed(1).hits(Bite, 1.0).young("kit").note("Dances to bewilder a rabbit, then strikes."),
    Beast::new("adder", "adders", 'n', (120, 110, 80), 0.04, Reptile, Ambusher).lives(HILLS | FOREST | MARSH | SHRUB, 5, 60).hits(Bite, 3.0).traits(VENOMOUS).plus(VENOM).note("Zig-zag backed; basks on the heath."),
    Beast::new("stonehide", "stonehides", 'Ø', (140, 140, 130), 1.9, Strange, Grazer).lives(MOUNTAIN | HILLS | BADLANDS, 0, 70).herd(1, 3).speed(5).hits(Butt, 8.0).traits(TAMEABLE).plus(HORN).rare(4).young("calf").note("A grey, plated browser that licks minerals from the rock until its hide grows hard as slate."),
    // ---- Coasts -------------------------------------------------------
    Beast::new("herring gull", "herring gulls", 'v', (220, 220, 220), 0.05, Bird, Scavenger).lives(COAST | SEA | LAKE, 0, 80).moves(FLY | WALK | SWIM).herd(3, 8).hits(Peck, 1.0).plus(EGGS).note("Screams over anything washed up."),
    Beast::new("pelican", "pelicans", 'P', (230, 220, 210), 0.3, Bird, Predator).lives(COAST | LAKE | MARSH, 35, 100).moves(FLY | WALK | SWIM).herd(2, 6).hits(Peck, 1.0).note("Scoops up fish by the pouchful."),
    Beast::new("shore crab", "shore crabs", 'x', (180, 90, 50), 0.02, Bug, Scavenger).lives(COAST | SEA, 5, 90).moves(WALK | SWIM).herd(2, 6).hits(Pinch, 0.5).yields(MEAT | CHITIN).note("Scuttles sideways after whatever the tide leaves."),
    Beast::new("giant crab", "giant crabs", 'X', (200, 80, 50), 1.3, Bug, Territorial).lives(COAST | SEA, 15, 95).moves(WALK | SWIM).speed(4).hits(Pinch, 9.0).traits(GIANT).yields(MEAT | CHITIN).rare(4).note("A crab as broad as a cart, with one claw bigger than the other."),
    Beast::new("sea turtle", "sea turtles", 'T', (90, 140, 90), 1.0, Reptile, Prey).lives(COAST | SEA, 50, 100).moves(WALK | SWIM).speed(6).hits(Bite, 2.0).yields(MEAT | SHELL).plus(EGGS).rare(3).note("Comes ashore only to lay its eggs."),
    Beast::new("sea lion", "sea lions", 'S', (140, 100, 70), 1.2, Mammal, Prey).lives(COAST | SEA, 10, 70).moves(WALK | SWIM).herd(2, 8).speed(4).hits(Bite, 4.0).young("pup").note("Barks and squabbles on the rocks."),
    Beast::new("puffin", "puffins", 'p', (40, 40, 40), 0.03, Bird, Prey).lives(COAST | SEA, 0, 45).moves(FLY | WALK | SWIM).herd(3, 10).hits(Peck, 0.5).plus(EGGS).note("Brings home a beakful of little fish."),
    Beast::new("cormorant", "cormorants", 'v', (30, 40, 30), 0.1, Bird, Predator).lives(COAST | LAKE | RIVER, 10, 85).moves(FLY | WALK | SWIM).herd(1, 5).hits(Peck, 1.5).traits(HUNTBEAST).note("Dries its wings spread wide on the rocks."),
    Beast::new("albatross", "albatrosses", 'P', (240, 240, 240), 0.3, Bird, Prey).lives(SEA | COAST, 0, 60).moves(FLY | SWIM | WALK).hits(Peck, 2.0).rare(3).note("Rides the sea winds for years without landing."),
    Beast::new("sea otter", "sea otters", 'o', (100, 70, 50), 0.25, Mammal, Predator).lives(COAST | SEA, 0, 50).moves(SWIM | WALK).herd(1, 4).speed(4).hits(Bite, 2.0).young("pup").note("Floats on its back to crack shellfish."),
    Beast::new("coconut crab", "coconut crabs", 'x', (130, 70, 140), 0.1, Bug, Scavenger).lives(COAST | JUNGLE, 60, 100).moves(WALK | CLIMB).hits(Pinch, 3.0).yields(MEAT | CHITIN).note("Climbs palms and steals pots."),
    // ---- Rivers and lakes ---------------------------------------------
    Beast::new("mudperch", "mudperch", 'α', (150, 140, 90), 0.04, Fish, Prey).lives(RIVER | LAKE | SWAMP, 10, 90).moves(SWIM).herd(2, 6).speed(2).note("A dull little fish that feeds on the bottom."),
    Beast::new("silverfin", "silverfins", 'α', (210, 220, 230), 0.03, Fish, Prey).lives(RIVER | LAKE, 5, 85).moves(SWIM).herd(3, 8).speed(1).note("Flashes in shoals in the shallows."),
    Beast::new("river pike", "river pike", 'α', (110, 140, 90), 0.15, Fish, Predator).lives(RIVER | LAKE, 0, 70).moves(SWIM).speed(1).hits(Bite, 2.0).note("Lies in the weed and strikes like an arrow."),
    Beast::new("brown trout", "brown trout", 'α', (160, 120, 80), 0.06, Fish, Prey).lives(RIVER | LAKE, 0, 60).moves(SWIM).herd(1, 4).speed(1).note("Rises to flies in fast cold water."),
    Beast::new("carp", "carp", 'α', (190, 150, 60), 0.1, Fish, Prey).lives(RIVER | LAKE | SWAMP, 20, 90).moves(SWIM).herd(2, 5).speed(3).note("Grows fat and old in still ponds."),
    Beast::new("catfish", "catfish", 'α', (100, 100, 90), 0.2, Fish, Scavenger).lives(RIVER | LAKE | SWAMP, 30, 100).moves(SWIM).speed(3).hits(Bite, 1.0).note("Whiskered; eats what sinks."),
    Beast::new("river eel", "river eels", 'α', (90, 80, 60), 0.05, Fish, Prey).lives(RIVER | LAKE | SEA, 10, 80).moves(SWIM).speed(2).hits(Bite, 0.5).yields(MEAT | FAT).note("Slips through wet grass from one pond to the next."),
    Beast::new("sturgeon", "sturgeon", 'α', (120, 120, 130), 0.9, Fish, Prey).lives(RIVER | LAKE | SEA, 0, 60).moves(SWIM).speed(3).plus(EGGS).rare(4).note("Armoured and ancient; its roe is worth its weight in silver."),
    Beast::new("salmon", "salmon", 'α', (210, 120, 110), 0.12, Fish, Prey).lives(RIVER | SEA, 0, 50).moves(SWIM).herd(3, 8).speed(1).note("Leaps the falls to spawn where it hatched."),
    Beast::new("piranha", "piranhas", 'α', (180, 70, 60), 0.02, Fish, Predator).lives(RIVER | LAKE, 60, 100).moves(SWIM).herd(4, 10).speed(1).hits(Bite, 2.0).note("A shoal of them strips a carcass to the bone."),
    Beast::new("lake char", "lake char", 'α', (220, 150, 120), 0.07, Fish, Prey).lives(LAKE | RIVER, 0, 40).moves(SWIM).herd(2, 5).speed(2).note("Red-bellied fish of deep cold lakes."),
    Beast::new("gar", "gars", 'α', (130, 140, 100), 0.3, Fish, Predator).lives(RIVER | LAKE | SWAMP, 40, 100).moves(SWIM).speed(2).hits(Bite, 3.0).note("A long-snouted fish that gulps air in foul water."),
    Beast::new("lamprey", "lampreys", '~', (110, 100, 100), 0.03, Fish, Predator).lives(RIVER | LAKE | SEA, 0, 60).moves(SWIM).speed(2).hits(Bite, 1.0).note("Fixes its round sucking mouth on bigger fish."),
    Beast::new("newt", "newts", 'l', (200, 120, 50), 0.01, Amphibian, Prey).lives(RIVER | LAKE | FOREST | MARSH, 5, 70).moves(WALK | SWIM).speed(4).hits(Bite, 0.1).note("Grows back a lost leg."),
    Beast::new("giant salamander", "giant salamanders", 'l', (100, 90, 70), 0.5, Amphibian, Ambusher).lives(RIVER | MOUNTAIN, 5, 50).moves(WALK | SWIM).speed(5).hits(Bite, 3.0).traits(GIANT).rare(3).note("Lies under stones in cold torrents, wrinkled and huge."),
    Beast::new("mallard", "mallards", 'd', (100, 150, 90), 0.05, Bird, Prey).lives(RIVER | LAKE | MARSH | SWAMP, 0, 85).moves(FLY | WALK | SWIM).herd(2, 8).hits(Peck, 0.2).traits(TAMEABLE | LIVESTOCK).plus(EGGS).young("duckling").note("Dabbles in every pond in the world."),
    Beast::new("greylag goose", "greylag geese", 'g', (170, 170, 160), 0.15, Bird, Territorial).lives(LAKE | MARSH | GRASS | TUNDRA | RIVER, 0, 70).moves(FLY | WALK | SWIM).herd(3, 10).hits(Peck, 1.5).traits(TAMEABLE | LIVESTOCK).plus(EGGS).young("gosling").note("Hisses at dwarves twice its size."),
    Beast::new("mute swan", "mute swans", 'S', (250, 250, 250), 0.3, Bird, Territorial).lives(LAKE | RIVER | MARSH, 5, 65).moves(FLY | WALK | SWIM).herd(1, 2).hits(Slam, 2.0).young("cygnet").note("Can break a dwarf's arm with a wing, they say."),
    Beast::new("kingfisher", "kingfishers", 'k', (60, 150, 220), 0.01, Bird, Predator).lives(RIVER | LAKE | FOREST | JUNGLE, 10, 100).moves(FLY | WALK).speed(1).hits(Peck, 0.3).note("A blue spark along the river."),
    Beast::new("crayfish", "crayfish", 'x', (150, 70, 50), 0.01, Bug, Scavenger).lives(RIVER | LAKE | SWAMP, 5, 85).moves(WALK | SWIM).herd(2, 6).hits(Pinch, 0.3).yields(MEAT | CHITIN).note("Hides under stones by day."),
    Beast::new("dragonfly", "dragonflies", 'i', (80, 180, 200), 0.01, Bug, Predator).lives(RIVER | LAKE | MARSH | SWAMP | JUNGLE, 15, 100).moves(FLY).herd(2, 6).speed(1).hits(Bite, 0.1).note("Hawks gnats over the water."),
    Beast::new("pearl mussel", "pearl mussels", 'o', (60, 60, 70), 0.01, Mollusc, Prey).lives(RIVER | LAKE, 0, 60).moves(SWIM).herd(2, 6).speed(9).hits(Pinch, 0.1).plus(PEARL).rare(3).note("Lives a hundred years in clean gravel; one in a hundred holds a pearl."),
    Beast::new("water vole", "water voles", 'r', (110, 90, 70), 0.02, Mammal, Vermin).lives(RIVER | LAKE | MARSH | GRASS, 0, 65).moves(WALK | SWIM | BURROW).herd(2, 5).hits(Bite, 0.3).young("pup").note("Plops into the water when it hears a step."),
    Beast::new("platypus", "platypuses", 'q', (120, 90, 60), 0.05, Mammal, Predator).lives(RIVER | LAKE, 15, 70).moves(WALK | SWIM | BURROW).hits(Sting, 2.0).traits(VENOMOUS).plus(VENOM | EGGS).rare(4).note("Duck-billed, egg-laying, and a spur of venom on its heel."),
    Beast::new("river dolphin", "river dolphins", 'D', (230, 170, 180), 1.2, Mammal, Prey).lives(RIVER | LAKE, 60, 100).moves(SWIM).herd(1, 4).speed(2).hits(Butt, 2.0).rare(4).note("Pink, near-blind, and hunts by sound in muddy rivers."),
    // ---- The sea ------------------------------------------------------
    Beast::new("cod", "cod", 'α', (150, 160, 120), 0.2, Fish, Prey).lives(SEA, 0, 50).moves(SWIM).herd(3, 10).speed(2).yields(MEAT | BONE | FAT).note("Shoals in the cold grey water."),
    Beast::new("herring", "herring", 'α', (190, 200, 210), 0.02, Fish, Prey).lives(SEA, 0, 55).moves(SWIM).herd(6, 12).speed(1).note("Silver in shoals beyond counting."),
    Beast::new("mackerel", "mackerel", 'α', (80, 150, 140), 0.03, Fish, Prey).lives(SEA, 10, 80).moves(SWIM).herd(5, 12).speed(1).note("Barred blue-green and always hungry."),
    Beast::new("tuna", "tuna", 'α', (60, 80, 140), 1.0, Fish, Predator).lives(SEA, 25, 100).moves(SWIM).herd(2, 6).speed(1).hits(Bite, 3.0).note("A fish like a sea-going bull."),
    Beast::new("swordfish", "swordfish", 'α', (80, 90, 140), 1.3, Fish, Predator).lives(SEA, 30, 100).moves(SWIM).speed(1).hits(Gore, 8.0).rare(3).note("Slashes through shoals with its long bill."),
    Beast::new("great white shark", "great white sharks", 'Σ', (150, 160, 170), 2.5, Fish, Predator).lives(SEA, 15, 85).moves(SWIM).speed(2).hits(Bite, 15.0).yields(MEAT | BONE | FAT | HIDE).rare(4).note("Rolls its eyes white as it bites."),
    Beast::new("hammerhead shark", "hammerhead sharks", 'Σ', (130, 140, 140), 1.6, Fish, Predator).lives(SEA, 45, 100).moves(SWIM).herd(1, 5).speed(2).hits(Bite, 10.0).yields(MEAT | BONE | HIDE).rare(3).note("Its odd head reads the sea floor for buried rays."),
    Beast::new("manta ray", "manta rays", 'Ψ', (60, 60, 80), 2.0, Fish, Prey).lives(SEA, 50, 100).moves(SWIM).speed(2).hits(Slam, 3.0).rare(3).note("Flies through the water on great black wings."),
    Beast::new("stingray", "stingrays", 'Ψ', (140, 120, 100), 0.3, Fish, Ambusher).lives(SEA | COAST, 40, 100).moves(SWIM).hits(Sting, 4.0).traits(VENOMOUS).plus(VENOM).note("Lies buried in the sand of the shallows."),
    Beast::new("octopus", "octopuses", '§', (190, 100, 80), 0.2, Mollusc, Ambusher).lives(SEA | COAST, 10, 95).moves(SWIM | WALK).hits(Bite, 2.0).yields(MEAT).note("Clever, boneless, and changes colour at will."),
    Beast::new("giant squid", "giant squid", '§', (210, 90, 90), 4.0, Mollusc, Predator).lives(SEA, 0, 60).moves(SWIM).speed(2).hits(Lash, 12.0).traits(GIANT).yields(MEAT).rare(5).note("Seen only dead on a beach, or never again."),
    Beast::new("jellyfish", "jellyfish", '¤', (200, 160, 220), 0.05, Strange, Prey).lives(SEA | COAST, 10, 100).moves(SWIM).herd(3, 10).speed(5).hits(Sting, 1.0).traits(VENOMOUS).yields(VENOM).note("A drifting bell of stinging threads."),
    Beast::new("blue whale", "blue whales", 'W', (80, 110, 150), 9.0, Mammal, Prey).lives(SEA, 0, 70).moves(SWIM).herd(1, 2).speed(3).hits(Slam, 20.0).yields(MEAT | FAT | BONE).rare(4).young("calf").note("Larger than anything that ever lived; seen from shore as a plume on the horizon."),
    Beast::new("orca", "orcas", 'O', (30, 30, 30), 3.0, Mammal, Predator).lives(SEA | COAST, 0, 60).moves(SWIM).herd(2, 6).speed(1).hits(Bite, 14.0).yields(MEAT | FAT | BONE).rare(3).young("calf").note("Black and white wolves of the sea, hunting in families."),
    Beast::new("humpback whale", "humpback whales", 'W', (60, 70, 80), 7.0, Mammal, Prey).lives(SEA, 0, 80).moves(SWIM).herd(1, 3).speed(3).hits(Slam, 16.0).yields(MEAT | FAT | BONE).rare(3).young("calf").note("Sings in the deep and leaps clear of the waves."),
    Beast::new("narwhal", "narwhals", 'N', (190, 190, 180), 1.6, Mammal, Prey).lives(SEA | GLACIER, 0, 25).moves(SWIM).herd(2, 6).speed(2).hits(Gore, 5.0).yields(MEAT | FAT | IVORY).rare(4).young("calf").note("Its single spiral tusk is sold as a wonder."),
    Beast::new("dolphin", "dolphins", 'D', (150, 160, 180), 0.8, Mammal, Prey).lives(SEA | COAST, 20, 95).moves(SWIM).herd(3, 8).speed(1).hits(Butt, 2.0).young("calf").note("Rides the bow-wave of ships for the joy of it."),
    Beast::new("lobster", "lobsters", 'x', (40, 70, 120), 0.05, Bug, Scavenger).lives(SEA | COAST, 0, 70).moves(WALK | SWIM).speed(4).hits(Pinch, 1.0).yields(MEAT | CHITIN).note("Blue-black until it's cooked."),
    Beast::new("oyster", "oysters", 'o', (170, 170, 160), 0.01, Mollusc, Prey).lives(SEA | COAST, 10, 90).moves(SWIM).herd(3, 8).speed(9).hits(Pinch, 0.1).plus(PEARL).note("Grows a pearl round a grain of grit."),
    Beast::new("sea urchin", "sea urchins", '*', (90, 40, 90), 0.01, Strange, Prey).lives(SEA | COAST, 0, 95).moves(SWIM).herd(2, 6).speed(9).hits(Sting, 0.5).yields(MEAT | SHELL).note("A ball of black spines on the rocks."),
    Beast::new("moray eel", "moray eels", '~', (110, 130, 60), 0.3, Fish, Ambusher).lives(SEA, 45, 100).moves(SWIM).hits(Bite, 4.0).note("Gapes from a crack in the reef."),
    Beast::new("sea snake", "sea snakes", 'n', (60, 110, 160), 0.05, Reptile, Ambusher).lives(SEA | COAST, 55, 100).moves(SWIM).hits(Bite, 5.0).traits(VENOMOUS).plus(VENOM).note("Banded blue and black, and deadlier than any cobra."),
    Beast::new("lantern eel", "lantern eels", '~', (150, 240, 200), 0.4, Strange, Predator).lives(SEA, 0, 60).moves(SWIM).speed(2).hits(Bite, 3.0).traits(GLOWS | NOCTURNAL).yields(MEAT | FAT).rare(4).note("Lights a green lamp at the tip of its chin and swims up to whatever comes to look."),
    // ---- The sky ------------------------------------------------------
    Beast::new("starling", "starlings", 'v', (70, 60, 90), 0.01, Bird, Prey).lives(GRASS | FOREST | HILLS | SHRUB | MARSH, 10, 70).moves(FLY | WALK).herd(6, 12).speed(1).hits(Peck, 0.1).note("A murmuration twists over the fields at dusk."),
    Beast::new("rock dove", "rock doves", 'v', (160, 160, 180), 0.03, Bird, Prey).lives(MOUNTAIN | HILLS | COAST | BADLANDS, 10, 85).moves(FLY | WALK).herd(3, 10).speed(1).hits(Peck, 0.1).traits(TAMEABLE).plus(EGGS).note("Nests on cliffs, and finds its way home from anywhere."),
    Beast::new("sparrow", "sparrows", 'v', (150, 120, 90), 0.01, Bird, Vermin).lives(GRASS | FOREST | SHRUB | SAVANNA | HILLS, 10, 90).moves(FLY | WALK).herd(3, 10).speed(1).hits(Peck, 0.1).note("Steals grain from under the farmer's nose."),
    Beast::new("barn swallow", "barn swallows", 'v', (60, 70, 140), 0.01, Bird, Prey).lives(GRASS | MARSH | FOREST | HILLS | SAVANNA, 15, 90).moves(FLY).herd(3, 10).speed(1).hits(Peck, 0.1).note("Skims the fields for flies before rain."),
    Beast::new("red-tailed hawk", "red-tailed hawks", 'k', (170, 100, 60), 0.06, Bird, Predator).lives(GRASS | FOREST | DESERT | SHRUB | HILLS, 10, 90).moves(FLY | WALK).hits(Talons, 2.0).traits(HUNTBEAST).note("Its scream is the sound of open country."),
    Beast::new("peregrine falcon", "peregrine falcons", 'k', (110, 110, 130), 0.05, Bird, Predator).lives(MOUNTAIN | COAST | HILLS | TUNDRA | BADLANDS, 0, 85).moves(FLY | WALK).speed(1).hits(Talons, 2.0).traits(HUNTBEAST | TAMEABLE).note("Stoops on pigeons faster than any falling stone."),
    Beast::new("white stork", "white storks", 'e', (240, 240, 240), 0.2, Bird, Prey).lives(GRASS | MARSH | SAVANNA, 25, 90).moves(FLY | WALK).herd(1, 6).hits(Peck, 1.5).note("Clatters its bill from nests on high places."),
    Beast::new("flamingo", "flamingos", 'e', (250, 140, 160), 0.2, Bird, Prey).lives(LAKE | MARSH | SAVANNA | COAST, 40, 100).moves(FLY | WALK | SWIM).herd(5, 12).hits(Peck, 0.5).note("Pink from the shrimp it sieves from salt lakes."),
    Beast::new("hummingbird", "hummingbirds", 'v', (80, 220, 120), 0.005, Bird, Prey).lives(JUNGLE | FOREST | MOUNTAIN, 40, 100).moves(FLY).speed(1).hits(Peck, 0.1).yields(FEATHERS).note("Hovers at flowers on blurring wings."),
    Beast::new("flying fox", "flying foxes", 'b', (120, 80, 50), 0.05, Mammal, Prey).lives(JUNGLE | SAVANNA | COAST, 55, 100).moves(FLY | CLIMB).herd(4, 12).speed(2).hits(Bite, 0.5).traits(NOCTURNAL).young("pup").note("A fruit bat with a wingspan wider than a dwarf is tall."),
    Beast::new("vampire bat", "vampire bats", 'b', (90, 50, 60), 0.02, Mammal, Predator).lives(JUNGLE | SAVANNA | CAVE1, 50, 100).moves(FLY | WALK).herd(2, 8).speed(1).hits(Bite, 0.5).traits(NOCTURNAL).young("pup").note("Laps blood from sleeping cattle."),
    Beast::new("storm heron", "storm herons", 'Ê', (110, 130, 170), 0.6, Strange, Prey).lives(COAST | MARSH | SEA | LAKE, 10, 80).moves(FLY | WALK | SWIM).herd(1, 3).hits(Peck, 4.0).rare(4).note("A slate-blue heron with a wingspan like a sail; it flies only ahead of bad weather, and sailors curse it."),
    // ---- Vermin -------------------------------------------------------
    Beast::new("brown rat", "brown rats", 'r', (130, 100, 80), 0.03, Mammal, Vermin).lives(LAND | COAST | RIVER | CAVE1, 5, 100).moves(WALK | SWIM | CLIMB | BURROW).herd(2, 6).speed(2).hits(Bite, 0.5).rare(1).young("pup").note("Wherever there's food, sooner or later there are rats."),
    Beast::new("house mouse", "house mice", 'r', (160, 150, 140), 0.01, Mammal, Vermin).lives(GRASS | FOREST | SHRUB | SAVANNA | HILLS | DESERT | TAIGA, 5, 100).moves(WALK | CLIMB | BURROW).herd(2, 6).speed(2).hits(Bite, 0.1).rare(1).traits(TAMEABLE | PETLIKE).young("pup").note("A whisker of a thing in the grain bins."),
    Beast::new("cockroach", "cockroaches", 'i', (120, 70, 40), 0.005, Bug, Vermin).lives(JUNGLE | SWAMP | SAVANNA | CAVE1 | CAVE2, 40, 100).moves(WALK | CLIMB).herd(3, 8).speed(1).hits(Bite, 0.1).rare(1).note("Can live a week without its head."),
    Beast::new("blowfly", "blowflies", 'i', (60, 110, 80), 0.005, Bug, Scavenger).lives(LAND, 10, 100).moves(FLY).herd(4, 10).speed(1).hits(Bite, 0.1).rare(1).note("Finds a carcass within the hour."),
    Beast::new("fire ant", "fire ants", 'a', (200, 60, 30), 0.005, Bug, Vermin).lives(SAVANNA | GRASS | SHRUB | JUNGLE | SWAMP, 50, 100).herd(5, 10).speed(2).hits(Sting, 0.5).traits(VENOMOUS).note("Their sting burns like an ember."),
    Beast::new("grain weevil", "grain weevils", 'i', (100, 80, 60), 0.005, Bug, Vermin).lives(GRASS | SAVANNA | SHRUB | FOREST, 15, 100).herd(4, 10).speed(4).hits(Bite, 0.05).rare(1).note("Hollows out seeds from the inside."),
    Beast::new("silverfish", "silverfish", 'i', (190, 190, 200), 0.005, Bug, Vermin).lives(FOREST | CAVE1 | CAVE2 | SWAMP, 10, 100).herd(2, 6).speed(2).hits(Bite, 0.05).note("Eats paper, glue and flour in the dark."),
    Beast::new("giant rat", "giant rats", 'R', (120, 90, 70), 0.4, Mammal, Vermin).lives(SWAMP | FOREST | JUNGLE | CAVE1 | CAVE2, 20, 100).moves(WALK | SWIM | BURROW).herd(1, 3).speed(2).hits(Bite, 3.0).traits(GIANT).rare(3).young("pup").note("A rat the size of a dog, and much less shy."),
    // ---- The first cavern ---------------------------------------------
    Beast::new("cave cricket", "cave crickets", 'i', (190, 180, 150), 0.01, Bug, Prey).lives(CAVE1 | CAVE2, 0, 100).herd(3, 8).speed(1).hits(Bite, 0.1).rare(1).note("Long-legged and pale; it hops away from any light."),
    Beast::new("olm", "olms", 'l', (240, 210, 200), 0.02, Amphibian, Prey).lives(CAVE1 | CAVE2 | RIVER, 0, 100).moves(SWIM | WALK).speed(4).hits(Bite, 0.1).note("A pink, eyeless salamander that can wait ten years for a meal."),
    Beast::new("blind cave fish", "blind cave fish", 'α', (230, 220, 220), 0.02, Fish, Prey).lives(CAVE1 | CAVE2 | CAVE3, 0, 100).moves(SWIM).herd(3, 8).speed(2).note("No eyes at all, and no need of them."),
    Beast::new("blind eel", "blind eels", 'α', (200, 200, 190), 0.06, Fish, Prey).lives(CAVE1 | CAVE2, 0, 100).moves(SWIM).speed(2).hits(Bite, 0.3).yields(MEAT | FAT).note("Coils in the black pools of the upper caves."),
    Beast::new("glowfin", "glowfins", 'α', (120, 230, 200), 0.03, Fish, Prey).lives(CAVE1 | CAVE2 | CAVE3, 0, 100).moves(SWIM).herd(2, 6).speed(1).traits(GLOWS).note("Its fins shine faint green in the dark water."),
    Beast::new("cave carp", "cave carp", 'α', (170, 160, 130), 0.1, Fish, Prey).lives(CAVE1 | CAVE2, 0, 100).moves(SWIM).herd(1, 4).speed(3).note("A pale, slow carp that grazes cave slime."),
    Beast::new("great cave bat", "great cave bats", 'B', (110, 90, 100), 0.4, Mammal, Predator).lives(CAVE1 | CAVE2, 0, 100).moves(FLY | WALK).herd(2, 6).speed(1).hits(Bite, 3.0).traits(NOCTURNAL | GIANT).rare(2).young("pup").note("Wings wide as a door; it hunts by its own shrieks."),
    Beast::new("capgrazer", "capgrazers", 'Q', (170, 130, 160), 0.8, Strange, Grazer).lives(CAVE1 | CAVE2, 0, 100).herd(3, 6).speed(4).hits(Butt, 3.0).traits(TAMEABLE | LIVESTOCK).plus(MILK).rare(2).young("calf").note("A blunt-headed, shelled beast that grazes cave fungus down to the stalk and sleeps in heaps."),
    Beast::new("pale gecko", "pale geckos", 'l', (220, 220, 200), 0.01, Reptile, Prey).lives(CAVE1 | CAVE2, 0, 100).moves(WALK | CLIMB).herd(1, 4).speed(1).hits(Bite, 0.1).note("Runs upside-down across the cavern roof."),
    Beast::new("drip leech", "drip leeches", '~', (150, 60, 70), 0.02, Worm, Ambusher).lives(CAVE1 | CAVE2, 0, 100).moves(WALK | CLIMB).herd(2, 6).speed(5).hits(Bite, 0.5).note("Hangs from the cavern roof and drops on whatever passes beneath."),
    Beast::new("lanternmoth", "lanternmoths", 'i', (240, 230, 130), 0.02, Bug, Prey).lives(CAVE1 | CAVE2, 0, 100).moves(FLY | WALK).herd(2, 8).speed(1).traits(GLOWS).plus(SILK).note("Flutters in a soft yellow glow; miners follow them to open caves."),
    Beast::new("pale scuttler", "pale scuttlers", 'x', (230, 220, 210), 0.05, Bug, Scavenger).lives(CAVE1 | CAVE2 | CAVE3, 0, 100).moves(WALK | CLIMB).herd(2, 6).speed(1).hits(Pinch, 0.5).yields(MEAT | CHITIN).rare(1).note("Many-legged and colourless, it swarms over anything dead in the dark."),
    // ---- The second cavern --------------------------------------------
    Beast::new("tuskmole", "tuskmoles", 'M', (150, 120, 110), 0.9, Strange, Territorial).lives(CAVE2 | CAVE3, 0, 100).moves(WALK | BURROW).speed(4).hits(Gore, 7.0).plus(IVORY).rare(3).young("pup").note("A blind digger as heavy as a pony, with a pair of tusks for breaking rock."),
    Beast::new("rockgnawer", "rockgnawers", 'r', (130, 130, 140), 0.05, Strange, Vermin).lives(CAVE2 | CAVE3 | CAVE1, 0, 100).moves(WALK | BURROW).herd(2, 6).speed(2).hits(Bite, 1.0).rare(2).young("pup").note("A grey rodent with teeth of flint that gnaws soft stone and anything stored on it."),
    Beast::new("lamplight beetle", "lamplight beetles", 'b', (250, 200, 90), 0.03, Bug, Prey).lives(CAVE2 | CAVE3, 0, 100).moves(WALK | FLY).herd(2, 6).speed(3).hits(Bite, 0.2).traits(GLOWS).note("Its back glows like a lantern seen through horn."),
    Beast::new("stalk-crab", "stalk-crabs", 'x', (160, 120, 170), 0.2, Bug, Scavenger).lives(CAVE2 | CAVE3, 0, 100).moves(WALK | SWIM | CLIMB).herd(1, 4).speed(3).hits(Pinch, 2.0).yields(MEAT | CHITIN).rare(2).note("A crab on stilt legs that wades the cave pools for drowned things."),
    Beast::new("cave monitor", "cave monitors", 'L', (120, 130, 100), 0.6, Reptile, Predator).lives(CAVE1 | CAVE2, 0, 100).moves(WALK | SWIM | CLIMB).hits(Bite, 6.0).traits(VENOMOUS).rare(3).note("A long pale lizard that tastes the air for warm blood."),
    Beast::new("gloomhound", "gloomhounds", 'h', (90, 80, 110), 0.6, Strange, Predator).lives(CAVE2 | CAVE3, 0, 100).herd(3, 5).speed(2).hits(Bite, 6.0).traits(TAMEABLE | WARBEAST | HUNTBEAST).rare(3).young("pup").note("Eyeless, hairless hunting dogs of the deep that run in silent packs by smell."),
    Beast::new("silkweaver", "silkweavers", 'X', (220, 220, 230), 0.3, Bug, Ambusher).lives(CAVE2 | CAVE3, 0, 100).moves(WALK | CLIMB).speed(3).hits(Bite, 4.0).traits(VENOMOUS).plus(SILK | VENOM).rare(3).note("A spider that strings whole galleries with white silk, and waits."),
    Beast::new("spore drifter", "spore drifters", 'o', (170, 200, 140), 0.3, Strange, Prey).lives(CAVE2 | CAVE3, 0, 100).moves(FLY).herd(1, 4).speed(6).hits(Slam, 0.2).yields(MEAT).rare(3).note("A gas-swollen bladder that floats on the cave draughts, trailing spores."),
    // ---- The third cavern ---------------------------------------------
    Beast::new("ember salamander", "ember salamanders", 'l', (250, 120, 50), 0.15, Amphibian, Prey).lives(CAVE3, 0, 100).moves(WALK | SWIM).speed(3).hits(Bite, 1.0).traits(GLOWS).rare(3).note("Lives by the hot springs of the deep, and its skin is warm to the touch."),
    Beast::new("pallid stalker", "pallid stalkers", 'Æ', (230, 230, 220), 1.3, Strange, Ambusher).lives(CAVE3, 0, 100).moves(WALK | CLIMB).speed(2).hits(Claw, 10.0).rare(4).note("A tall, thin, colourless thing that stands so still against the rock it seems part of it."),
    Beast::new("thousand-foot", "thousand-feet", 'x', (150, 60, 50), 1.0, Bug, Ambusher).lives(CAVE3 | CAVE2, 0, 100).moves(WALK | CLIMB).speed(2).hits(Bite, 8.0).traits(VENOMOUS | GIANT).plus(VENOM).rare(4).note("A millipede as long as a mine cart; its rustle carries far in the deep."),
    Beast::new("gemback tortoise", "gemback tortoises", 'T', (140, 200, 190), 1.4, Strange, Grazer).lives(CAVE3, 0, 100).speed(8).hits(Butt, 4.0).yields(MEAT | BONE | SHELL | PEARL).rare(4).note("Grazes the mineral crusts of the deepest caves; little crystals grow in the plates of its shell."),
    Beast::new("veilwing", "veilwings", 'V', (120, 90, 150), 0.8, Strange, Predator).lives(CAVE3, 0, 100).moves(FLY | WALK).herd(1, 3).speed(1).hits(Talons, 7.0).traits(NOCTURNAL).yields(MEAT | BONE | HIDE).rare(4).note("A dark, membrane-winged hunter that glides down on its prey without a sound."),
    Beast::new("chasm eel", "chasm eels", '~', (110, 90, 140), 0.7, Fish, Ambusher).lives(CAVE3 | CAVE2, 0, 100).moves(SWIM).speed(2).hits(Bite, 7.0).yields(MEAT | FAT).rare(3).note("Lurks in the bottomless pools; anything that drinks there is in danger."),
    Beast::new("hollow crab", "hollow crabs", 'x', (180, 170, 150), 0.4, Bug, Prey).lives(CAVE3, 0, 100).moves(WALK | SWIM).herd(2, 5).speed(4).hits(Pinch, 2.0).yields(MEAT | CHITIN | SHELL).rare(3).note("Wears an empty stone shell; when it outgrows one it fights for another."),
    // ---- Giants of common animals --------------------------------------
    Beast::new("giant beaver", "giant beavers", 'B', (120, 80, 50), 1.2, Mammal, Territorial).lives(RIVER | LAKE | TAIGA | FOREST, 0, 50).moves(WALK | SWIM).speed(4).hits(Bite, 7.0).traits(GIANT).rare(4).young("kit").note("Its dams flood whole valleys."),
    Beast::new("giant mantis", "giant mantises", 'M', (130, 200, 90), 0.8, Bug, Ambusher).lives(JUNGLE | SAVANNA | GRASS, 50, 100).moves(WALK | FLY).hits(Claw, 7.0).traits(GIANT).rare(4).note("Sways like a stalk of grass, then lunges."),
    Beast::new("giant tortoise", "giant tortoises", 'T', (130, 120, 100), 1.6, Reptile, Grazer).lives(SAVANNA | SHRUB | COAST | GRASS, 45, 100).speed(9).hits(Bite, 2.0).traits(GIANT).yields(MEAT | BONE | SHELL).rare(4).note("Lives two hundred years, and is in no hurry."),
    Beast::new("giant otter", "giant otters", 'o', (100, 80, 60), 0.7, Mammal, Predator).lives(RIVER | LAKE | JUNGLE | SWAMP, 55, 100).moves(WALK | SWIM).herd(2, 6).hits(Bite, 5.0).traits(GIANT).rare(3).young("pup").note("Families of them shriek at caimans and drive them off."),
    Beast::new("giant anteater", "giant anteaters", 'A', (100, 90, 80), 0.8, Mammal, Territorial).lives(SAVANNA | GRASS | JUNGLE, 50, 100).speed(4).hits(Claw, 7.0).traits(GIANT).rare(3).young("pup").note("Walks on its knuckles to save its claws."),
    Beast::new("giant hedgehog", "giant hedgehogs", 'H', (150, 130, 100), 0.9, Mammal, Territorial).lives(FOREST | HILLS | SHRUB, 15, 65).speed(4).hits(Sting, 6.0).traits(GIANT | NOCTURNAL).rare(4).young("hoglet").note("A ball of spines as big as a barrel."),
    Beast::new("giant moth", "giant moths", 'i', (190, 170, 150), 0.6, Bug, Prey).lives(FOREST | JUNGLE | TAIGA, 15, 100).moves(FLY).herd(1, 3).speed(2).hits(Slam, 0.5).traits(GIANT | NOCTURNAL).plus(SILK).rare(4).note("Wings like painted shields, drawn to every fire."),
    Beast::new("giant weta", "giant wetas", 'i', (140, 110, 80), 0.2, Bug, Prey).lives(FOREST | HILLS | CAVE1, 5, 70).speed(4).hits(Bite, 1.5).traits(GIANT | NOCTURNAL).rare(3).note("A cricket as heavy as a sparrow."),
    // ---- Strange beasts ------------------------------------------------
    Beast::new("thornback", "thornbacks", 'Ŧ', (140, 150, 90), 1.6, Strange, Territorial).lives(GRASS | SHRUB | SAVANNA, 25, 90).herd(1, 4).speed(4).hits(Lash, 9.0).traits(TAMEABLE | WARBEAST).plus(HORN).rare(4).young("calf").note("A low-slung grazer whose back and tail bristle with woody thorns, which it sheds each spring."),
    Beast::new("hollow stag", "hollow stags", 'Ð', (170, 160, 140), 1.2, Strange, Grazer).lives(FOREST | TAIGA, 5, 55).speed(2).hits(Gore, 6.0).plus(HORN).rare(5).young("fawn").note("A pale stag whose antlers are like bare dead branches; it is seen at dusk, and rarely twice."),
    Beast::new("sunback lizard", "sunback lizards", 'l', (240, 190, 60), 0.5, Strange, Prey).lives(SAVANNA | DESERT | BADLANDS, 55, 100).speed(2).hits(Bite, 2.0).rare(4).note("Raises a golden frill to the sun every morning and will not move until it is warm."),
    Beast::new("reedwalker", "reedwalkers", 'Ŕ', (150, 170, 110), 0.5, Strange, Prey).lives(MARSH | SWAMP | LAKE, 15, 85).moves(WALK | SWIM).herd(2, 6).speed(2).hits(Kick, 1.5).plus(EGGS | FEATHERS).rare(3).note("A half-bird, half-lizard that stalks the reedbeds on long jointed legs and lays blue eggs."),
    Beast::new("ironjaw", "ironjaws", 'J', (110, 110, 120), 1.0, Strange, Predator).lives(BADLANDS | MOUNTAIN | HILLS, 20, 85).herd(1, 2).speed(2).hits(Bite, 11.0).rare(5).young("whelp").note("A short, heavy hunter with a jaw that never lets go; prospectors find its teeth in the scree."),
    Beast::new("skyray", "skyrays", 'Ψ', (110, 160, 200), 1.0, Strange, Prey).lives(SEA | COAST | OPEN, 20, 100).moves(FLY | SWIM).herd(1, 4).speed(2).hits(Lash, 2.0).rare(5).note("A ray that leaps from the sea and glides for miles on the warm air over the plains."),
    Beast::new("cinder hound", "cinder hounds", 'h', (200, 90, 50), 0.6, Strange, Predator).lives(BADLANDS | DESERT, 50, 100).herd(2, 4).speed(2).hits(Bite, 6.0).rare(4).young("whelp").note("A lean red dog of the badlands, with a coat that smells of smoke; fire does not frighten it."),
    // ---- Kept animals --------------------------------------------------
    Beast::new("chicken", "chickens", 'c', (230, 210, 160), 0.05, Bird, Prey).lives(0, 0, 100).hits(Peck, 0.3).traits(TAMEABLE | LIVESTOCK).plus(EGGS).note("Kept for eggs in every village."),
    Beast::new("sheep", "sheep", 's', (235, 235, 220), 0.6, Mammal, Grazer).lives(0, 0, 100).herd(2, 6).hits(Butt, 2.0).traits(TAMEABLE | LIVESTOCK).plus(WOOL | MILK).young("lamb").note("Kept for wool, milk and mutton."),
    Beast::new("pig", "pigs", 'q', (230, 170, 160), 0.7, Mammal, Grazer).lives(0, 0, 100).hits(Bite, 2.0).traits(TAMEABLE | LIVESTOCK).young("piglet").note("Eats anything, and is mostly bacon."),
    Beast::new("cow", "cows", 'C', (200, 190, 170), 1.6, Mammal, Grazer).lives(0, 0, 100).hits(Kick, 4.0).traits(TAMEABLE | LIVESTOCK).plus(MILK | HORN).young("calf").note("Milk, beef and leather, and no trouble."),
    Beast::new("horse", "horses", 'H', (160, 110, 70), 1.4, Mammal, Grazer).lives(0, 0, 100).speed(1).hits(Kick, 6.0).traits(TAMEABLE | LIVESTOCK | MOUNT | WARBEAST).young("foal").note("Bred for the plough and the cart."),
    Beast::new("donkey", "donkeys", 'H', (150, 140, 130), 1.0, Mammal, Grazer).lives(0, 0, 100).hits(Kick, 5.0).traits(TAMEABLE | LIVESTOCK | MOUNT).young("foal").note("Stubborn, sure-footed, and carries twice its weight."),
    Beast::new("water buffalo", "water buffalo", 'O', (70, 70, 75), 1.8, Mammal, Grazer).lives(0, 0, 100).moves(WALK | SWIM).hits(Gore, 7.0).traits(TAMEABLE | LIVESTOCK | MOUNT).plus(MILK | HORN).young("calf").note("Wallows in paddies and pulls the plough through them."),
    Beast::new("alpaca", "alpacas", 'L', (230, 210, 190), 0.7, Mammal, Grazer).lives(0, 0, 100).herd(2, 6).hits(Kick, 2.0).traits(TAMEABLE | LIVESTOCK).plus(WOOL).young("cria").note("Its fleece is softer than any sheep's."),
];

/// How many there are.
pub const COUNT: usize = BEASTS.len();

/// The beast at `i` (a missing index gives the first: a save from a later
/// game with more beasts in it still loads).
pub fn beast(i: u16) -> &'static Beast {
    BEASTS.get(i as usize).unwrap_or(&BEASTS[0])
}

/// The beast called `name`, if any.
pub fn find(name: &str) -> Option<u16> {
    BEASTS.iter().position(|b| b.name == name).map(|i| i as u16)
}

const fn same(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// The index of the beast called `name`, at compile time (a name not in
/// the table is a compile error where it's used in a const).
pub const fn idx(name: &str) -> u16 {
    let mut i = 0;
    while i < BEASTS.len() {
        if same(BEASTS[i].name, name) {
            return i as u16;
        }
        i += 1;
    }
    panic!("no such beast")
}

pub const SHEEP: u16 = idx("sheep");
pub const CHICKEN: u16 = idx("chicken");
pub const PIG: u16 = idx("pig");

/// What a product is made of, as an item.
pub fn product_mat(bit: u16) -> Mat {
    match bit {
        WOOL | FEATHERS | SILK => Mat::Leather,
        VENOM => Mat::Flesh,
        _ => Mat::Bone,
    }
}

/// "sheep wool", "walrus ivory".
pub fn product_label(b: &Beast, bit: u16) -> String {
    let noun = PRODUCTS.iter().find(|(p, _)| *p == bit).map_or("product", |p| p.1);
    format!("{} {}", b.name, noun)
}

/// What butchering one of them gives: meat (fish for fish) by its size,
/// fat, bones and hide where it has them, and its goods (wool, feathers,
/// ivory, venom...) as products. Eggs and milk come only from the living.
pub fn carcass_yields(i: u16) -> Vec<(ItemKind, Mat, Option<String>)> {
    let b = beast(i);
    let name = Some(b.name.to_string());
    let mut v = Vec::new();
    if b.gives(MEAT) {
        let n = ((b.size * 5.0).round() as usize).clamp(1, 12);
        let kind = if b.class == Class::Fish { ItemKind::Fish } else { ItemKind::Meat };
        v.extend(std::iter::repeat((kind, Mat::Flesh, name.clone())).take(n));
    }
    if b.gives(FAT) {
        let n = 1 + (b.size >= 1.5) as usize;
        v.extend(std::iter::repeat((ItemKind::Tallow, Mat::Flesh, name.clone())).take(n));
    }
    if b.gives(BONE) && b.size >= 0.02 {
        let n = 1 + (b.size >= 0.5) as usize;
        v.extend(std::iter::repeat((ItemKind::Bone, Mat::Bone, name.clone())).take(n));
    }
    if b.gives(HIDE) && b.size >= 0.04 {
        v.push((ItemKind::Hide, Mat::Leather, name.clone()));
    }
    for &(bit, _) in PRODUCTS.iter().filter(|(p, _)| GOODS & p != 0 && b.gives(*p)) {
        v.push((ItemKind::Product, product_mat(bit), Some(product_label(b, bit))));
    }
    v
}

/// The beasts of a habitat bit fitting warmth `t` (any warmth if None).
pub fn of_habitat(bit: u32, t: Option<f32>) -> impl Iterator<Item = u16> {
    BEASTS.iter().enumerate().filter(move |(_, b)| b.home & bit != 0 && t.map_or(true, |t| b.fits_temp(t))).map(|(i, _)| i as u16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn the_table_has_at_least_150_distinct_animals_with_valid_fields() {
        assert!(BEASTS.len() >= 150, "only {} beasts", BEASTS.len());
        let mut names = HashSet::new();
        let mut plurals = HashSet::new();
        for b in BEASTS.iter() {
            assert!(names.insert(b.name), "{} twice", b.name);
            assert!(plurals.insert(b.plural), "plural {} twice", b.plural);
            assert!(!b.name.is_empty() && b.name == b.name.to_lowercase(), "{}", b.name);
            assert!(b.size > 0.0 && b.size <= 10.0, "{} size {}", b.name, b.size);
            assert!(b.temp.0 <= b.temp.1 && b.temp.1 <= 100, "{} temp", b.name);
            assert!(b.group.0 >= 1 && b.group.0 <= b.group.1 && b.group.1 <= 12, "{} group", b.name);
            assert!(b.speed >= 1 && b.speed <= 9, "{} speed", b.name);
            assert!(b.damage > 0.0 && b.damage <= 20.0, "{} damage", b.name);
            assert!(b.rarity >= 1 && b.rarity <= 5, "{} rarity", b.name);
            assert!(b.moves != 0, "{} can't move", b.name);
            assert!(b.home != 0 || b.has(LIVESTOCK), "{} lives nowhere", b.name);
            assert!(b.yields != 0, "{} gives nothing", b.name);
            assert!(!b.note.is_empty() && b.note.ends_with('.'), "{} note", b.name);
            assert!(!b.glyph.is_whitespace(), "{} glyph", b.name);
            // Water-only beasts live in water; cave fliers in caves.
            if b.water_only() {
                assert!(b.home & (FRESH | SEA | CAVES | WET) != 0, "{} swims nowhere", b.name);
            }
            // The venomous have a venom to give or a sting to show.
            if b.has(VENOMOUS) {
                assert!(b.gives(VENOM) || matches!(b.strike, Strike::Sting | Strike::Bite), "{}", b.name);
            }
        }
        assert_eq!(beast(SHEEP).name, "sheep");
        assert_eq!(find("brown rat").map(|i| beast(i).name), Some("brown rat"));
        assert_eq!(beast(60000).name, BEASTS[0].name);
    }

    #[test]
    fn every_land_water_and_cavern_layer_has_several_animals() {
        for (bit, name) in HABITATS.iter() {
            let n = BEASTS.iter().filter(|b| b.home & bit != 0).count();
            assert!(n >= 6, "only {} beasts in {}", n, name);
        }
        // The sky over every land biome has fliers.
        for (bit, name) in HABITATS.iter().take(14) {
            let n = BEASTS.iter().filter(|b| b.home & bit != 0 && b.flier()).count();
            assert!(n >= 2, "only {} fliers over {}", n, name);
        }
        for t in Temper::ALL {
            let n = BEASTS.iter().filter(|b| b.temper == t).count();
            assert!(n >= 6, "only {} of temper {:?}", n, t);
        }
        // Several in each cavern layer that walk (not just fish).
        for bit in [CAVE1, CAVE2, CAVE3] {
            assert!(BEASTS.iter().filter(|b| b.home & bit != 0 && b.can(WALK)).count() >= 5);
            assert!(BEASTS.iter().any(|b| b.home & bit != 0 && b.water_only()));
        }
    }

    /// The census, by habitat and temperament
    /// (`cargo test --release bestiary_census -- --ignored --nocapture`).
    #[test]
    #[ignore]
    fn bestiary_census() {
        println!("{} animals", BEASTS.len());
        for (bit, name) in HABITATS.iter() {
            println!("{:>16}: {}", name, BEASTS.iter().filter(|b| b.home & bit != 0).count());
        }
        println!("{:>16}: {}", "sky (fliers)", BEASTS.iter().filter(|b| b.flier()).count());
        println!("{:>16}: {}", "kept only", BEASTS.iter().filter(|b| b.domestic()).count());
        for t in Temper::ALL {
            println!("{:>16}: {}", t.name(), BEASTS.iter().filter(|b| b.temper == t).count());
        }
        println!("{:>16}: {}", "giant kinds", BEASTS.iter().filter(|b| b.has(GIANT)).count());
        println!("{:>16}: {}", "strange", BEASTS.iter().filter(|b| b.class == Class::Strange).count());
    }

    #[test]
    fn the_strange_ones_are_original() {
        // Invented beasts mustn't borrow another game's invented names.
        let borrowed = ["crundle", "draltha", "jabberer", "gorlak", "rutherer", "elk bird", "nightwing", "molemarian", "voracious cave crawler", "giant cave swallow", "giant olm", "blind cave ogre", "cave floater", "pond grabber"];
        for b in BEASTS.iter() {
            assert!(!borrowed.contains(&b.name), "{}", b.name);
        }
    }
}
