use ratatui::style::Color;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Mat {
    #[default]
    Air,
    Grass,
    Loam,
    Clay,
    Sand,
    Sandstone,
    Limestone,
    Granite,
    Marble,
    Basalt,
    Hematite,
    Malachite,
    NativeGold,
    Coal,
    Amethyst,
    Bedrock,
    Wood,
    Stoutcap,
    Flesh,
    Copper,
    Bronze,
    Iron,
    Steel,
    Bone,
    Leather,
    Gold,
    /// Volcanic glass, left where magma meets water.
    Obsidian,
    Hollowgrain,
    Threadreed,
    Sunberry,
    Goldoat,
}

/// A farmed plant: where and when it grows and what it's good for.
pub struct Crop {
    /// The harvest ("sunberries").
    pub plant: &'static str,
    pub seed: &'static str,
    /// Some(true) underground only, Some(false) under the sky only, None anywhere.
    pub underground: Option<bool>,
    /// Spring, summer, autumn, winter.
    pub seasons: [bool; 4],
    pub edible: bool,
    /// What it brews into, if anything.
    pub brew: Option<&'static str>,
    /// Ground into flour at a millstone.
    pub mill: bool,
    /// Spun into thread.
    pub fiber: bool,
}

impl Crop {
    pub fn grows(&self, outside: bool, season: usize) -> bool {
        self.seasons[season % 4] && self.underground.map_or(true, |u| u != outside)
    }

    /// What the harvest is for: "eat, brew, spin".
    pub fn uses(&self) -> String {
        let mut v = Vec::new();
        if self.edible {
            v.push("eat");
        }
        if self.brew.is_some() {
            v.push("brew");
        }
        if self.mill {
            v.push("mill");
        }
        if self.fiber {
            v.push("spin");
        }
        v.join(", ")
    }

    pub fn where_when(&self) -> String {
        let names = ["spring", "summer", "autumn", "winter"];
        let s: Vec<&str> = (0..4).filter(|&i| self.seasons[i]).map(|i| names[i]).collect();
        let place = match self.underground {
            Some(true) => "underground",
            Some(false) => "on the surface",
            None => "anywhere",
        };
        let when = if s.len() == 4 { "all year".to_string() } else { s.join(", ") };
        format!("{}, {}", place, when)
    }
}

/// Every crop, in the order a farm cycles through them.
pub const CROPS: [Mat; 5] = [Mat::Stoutcap, Mat::Hollowgrain, Mat::Threadreed, Mat::Sunberry, Mat::Goldoat];

impl Mat {
    pub fn name(self) -> &'static str {
        match self {
            Mat::Air => "air",
            Mat::Grass => "grass",
            Mat::Loam => "loam",
            Mat::Clay => "clay",
            Mat::Sand => "sand",
            Mat::Sandstone => "sandstone",
            Mat::Limestone => "limestone",
            Mat::Granite => "granite",
            Mat::Marble => "marble",
            Mat::Basalt => "basalt",
            Mat::Hematite => "hematite",
            Mat::Malachite => "malachite",
            Mat::NativeGold => "native gold",
            Mat::Coal => "coal",
            Mat::Amethyst => "amethyst",
            Mat::Bedrock => "bedrock",
            Mat::Wood => "wood",
            Mat::Stoutcap => "stoutcap",
            Mat::Flesh => "flesh",
            Mat::Copper => "copper",
            Mat::Bronze => "bronze",
            Mat::Iron => "iron",
            Mat::Steel => "steel",
            Mat::Bone => "bone",
            Mat::Leather => "leather",
            Mat::Gold => "gold",
            Mat::Obsidian => "obsidian",
            Mat::Hollowgrain => "hollowgrain",
            Mat::Threadreed => "threadreed",
            Mat::Sunberry => "sunberry",
            Mat::Goldoat => "goldoat",
        }
    }

    /// Adjective form used for crafted objects ("wooden bed", "granite table").
    pub fn adj(self) -> &'static str {
        match self {
            Mat::Wood => "wooden",
            m => m.name(),
        }
    }

    pub fn color(self) -> Color {
        let (r, g, b) = match self {
            Mat::Air => (0, 0, 0),
            Mat::Grass => (70, 160, 60),
            Mat::Loam => (140, 100, 60),
            Mat::Clay => (170, 90, 60),
            Mat::Sand => (200, 180, 110),
            Mat::Sandstone => (190, 160, 110),
            Mat::Limestone => (190, 190, 170),
            Mat::Granite => (150, 140, 140),
            Mat::Marble => (225, 225, 230),
            Mat::Basalt => (90, 90, 100),
            Mat::Hematite => (200, 60, 50),
            Mat::Malachite => (40, 200, 120),
            Mat::NativeGold => (250, 210, 40),
            Mat::Coal => (60, 60, 60),
            Mat::Amethyst => (190, 90, 230),
            Mat::Bedrock => (110, 30, 30),
            Mat::Wood => (160, 110, 50),
            Mat::Stoutcap => (180, 110, 200),
            Mat::Flesh => (200, 140, 140),
            Mat::Copper => (220, 120, 60),
            Mat::Bronze => (200, 150, 70),
            Mat::Iron => (170, 170, 185),
            Mat::Steel => (200, 210, 230),
            Mat::Bone => (230, 225, 200),
            Mat::Leather => (160, 110, 70),
            Mat::Gold => (255, 215, 60),
            Mat::Obsidian => (70, 50, 90),
            Mat::Hollowgrain => (200, 180, 120),
            Mat::Threadreed => (150, 200, 140),
            Mat::Sunberry => (240, 110, 120),
            Mat::Goldoat => (230, 200, 80),
        };
        Color::Rgb(r, g, b)
    }

    pub fn crop(self) -> Option<Crop> {
        let c = |plant, seed, underground, seasons, edible, brew, mill, fiber| Crop { plant, seed, underground, seasons, edible, brew, mill, fiber };
        Some(match self {
            Mat::Stoutcap => c("stoutcap mushroom", "stoutcap spawn", None, [true; 4], true, Some("stoutcap ale"), false, true),
            Mat::Hollowgrain => c("hollowgrain ears", "hollowgrain seeds", Some(true), [false, false, true, true], false, Some("hollowgrain beer"), true, false),
            Mat::Threadreed => c("threadreed stalks", "threadreed seeds", Some(true), [true, true, false, false], false, None, false, true),
            Mat::Sunberry => c("sunberries", "sunberry seeds", Some(false), [true, true, false, false], true, Some("sunberry wine"), false, false),
            Mat::Goldoat => c("goldoat sheaf", "goldoat seeds", Some(false), [false, true, true, false], false, None, true, false),
            _ => return None,
        })
    }

    pub fn is_soil(self) -> bool {
        matches!(self, Mat::Grass | Mat::Loam | Mat::Clay | Mat::Sand)
    }

    pub fn is_ore(self) -> bool {
        matches!(self, Mat::Hematite | Mat::Malachite | Mat::NativeGold | Mat::Coal)
    }

    pub fn is_gem(self) -> bool {
        self == Mat::Amethyst
    }

    /// Plain stone usable for masonry (ores and gems are kept for later industries).
    pub fn is_building_stone(self) -> bool {
        matches!(
            self,
            Mat::Sandstone | Mat::Limestone | Mat::Granite | Mat::Marble | Mat::Basalt | Mat::Obsidian
        )
    }

    /// Survives being dropped in magma: stone, ore, gems and metal.
    pub fn fireproof(self) -> bool {
        self.is_building_stone()
            || self.is_ore()
            || self.is_gem()
            || matches!(self, Mat::Copper | Mat::Bronze | Mat::Iron | Mat::Steel | Mat::Gold | Mat::Bedrock)
    }

    /// The metal an ore boulder yields at the smelter.
    pub fn smelts_to(self) -> Option<Mat> {
        match self {
            Mat::Hematite => Some(Mat::Iron),
            Mat::Malachite => Some(Mat::Copper),
            Mat::NativeGold => Some(Mat::Gold),
            _ => None,
        }
    }

    /// Value multiplier for goods made of this material.
    pub fn value(self) -> u32 {
        match self {
            Mat::Gold => 12,
            Mat::Amethyst => 10,
            Mat::Steel => 6,
            Mat::Iron | Mat::Bronze => 4,
            Mat::Copper => 3,
            Mat::Marble => 2,
            _ => 1,
        }
    }

    /// Damage multiplier for weapons made of this material.
    pub fn weapon_mult(self) -> f32 {
        match self {
            Mat::Steel => 1.5,
            Mat::Iron => 1.25,
            Mat::Bronze => 1.15,
            Mat::Copper => 1.0,
            Mat::Bone => 0.8,
            Mat::Wood => 0.6,
            _ => 0.7,
        }
    }

    /// Damage absorbed by armor made of this material.
    pub fn armor_value(self) -> i32 {
        match self {
            Mat::Steel => 7,
            Mat::Iron => 5,
            Mat::Bronze => 4,
            Mat::Copper => 3,
            Mat::Bone | Mat::Leather => 2,
            _ => 1,
        }
    }

    pub fn diggable(self) -> bool {
        self != Mat::Bedrock
    }

    /// How hard this is to cut, soil 20 to obsidian 95 (O10, DIG-06). With
    /// the pick's `edge` it sets how long a dig takes.
    pub fn hardness(self) -> u32 {
        match self {
            Mat::Grass | Mat::Loam | Mat::Sand => 20,
            Mat::Clay => 26,
            Mat::Sandstone | Mat::Coal => 50,
            Mat::Limestone => 55,
            Mat::NativeGold => 60,
            Mat::Marble | Mat::Malachite => 65,
            Mat::Hematite => 70,
            Mat::Granite => 80,
            Mat::Basalt | Mat::Amethyst => 85,
            Mat::Obsidian => 95,
            _ => 57,
        }
    }

    /// How well a tool of this metal bites into rock: copper 60 to steel
    /// 120; anything that isn't a tool metal is as good as a bare hand (30).
    pub fn edge(self) -> u32 {
        match self {
            Mat::Steel => 120,
            Mat::Iron => 90,
            Mat::Bronze => 75,
            Mat::Copper => 60,
            Mat::Gold => 40,
            _ => 30,
        }
    }

    /// Base number of ticks a novice needs to dig through this material
    /// with a tool of the given edge (a copper pick on granite: 140).
    pub fn dig_ticks_with(self, edge: u32) -> u32 {
        (self.hardness() * 105 / edge.max(1)).max(10)
    }
}
