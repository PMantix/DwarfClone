use ratatui::style::Color;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
}

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
        };
        Color::Rgb(r, g, b)
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

    /// Base number of ticks a novice needs to dig through this material.
    pub fn dig_ticks(self) -> u32 {
        match self {
            Mat::Grass | Mat::Loam | Mat::Sand => 35,
            Mat::Clay => 45,
            Mat::Sandstone | Mat::Limestone | Mat::Coal => 90,
            Mat::Marble => 110,
            Mat::Hematite | Mat::Malachite | Mat::NativeGold => 120,
            Mat::Granite | Mat::Basalt => 140,
            Mat::Obsidian => 160,
            Mat::Amethyst => 150,
            _ => 100,
        }
    }
}
