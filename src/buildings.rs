use crate::dwarf::Labor;
use crate::geom::Pos;
use crate::items::{Item, ItemKind};
use crate::material::Mat;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildingKind {
    Carpenter,
    Mason,
    Still,
    Kitchen,
    Butcher,
    WoodFurnace,
    Smelter,
    Forge,
    Craftsdwarf,
    Depot,
    Bed,
    Table,
    Chair,
    Door,
}

impl BuildingKind {
    pub const ALL: [BuildingKind; 14] = [
        BuildingKind::Carpenter,
        BuildingKind::Mason,
        BuildingKind::Still,
        BuildingKind::Kitchen,
        BuildingKind::Butcher,
        BuildingKind::WoodFurnace,
        BuildingKind::Smelter,
        BuildingKind::Forge,
        BuildingKind::Craftsdwarf,
        BuildingKind::Depot,
        BuildingKind::Bed,
        BuildingKind::Table,
        BuildingKind::Chair,
        BuildingKind::Door,
    ];

    pub fn name(self) -> &'static str {
        match self {
            BuildingKind::Carpenter => "Carpenter's workshop",
            BuildingKind::Mason => "Mason's workshop",
            BuildingKind::Still => "Still",
            BuildingKind::Kitchen => "Kitchen",
            BuildingKind::Butcher => "Butcher's shop",
            BuildingKind::WoodFurnace => "Wood furnace",
            BuildingKind::Smelter => "Smelter",
            BuildingKind::Forge => "Metalsmith's forge",
            BuildingKind::Craftsdwarf => "Craftsdwarf's workshop",
            BuildingKind::Depot => "Trade depot",
            BuildingKind::Bed => "Bed",
            BuildingKind::Table => "Table",
            BuildingKind::Chair => "Chair",
            BuildingKind::Door => "Door",
        }
    }

    pub fn key(self) -> char {
        match self {
            BuildingKind::Carpenter => 'c',
            BuildingKind::Mason => 'm',
            BuildingKind::Still => 's',
            BuildingKind::Kitchen => 'k',
            BuildingKind::Butcher => 'u',
            BuildingKind::WoodFurnace => 'w',
            BuildingKind::Smelter => 'e',
            BuildingKind::Forge => 'f',
            BuildingKind::Craftsdwarf => 'r',
            BuildingKind::Depot => 'p',
            BuildingKind::Bed => 'b',
            BuildingKind::Table => 't',
            BuildingKind::Chair => 'h',
            BuildingKind::Door => 'd',
        }
    }

    pub fn is_workshop(self) -> bool {
        !matches!(self, BuildingKind::Bed | BuildingKind::Table | BuildingKind::Chair | BuildingKind::Door)
    }

    pub fn size(self) -> i32 {
        if self.is_workshop() {
            3
        } else {
            1
        }
    }

    /// The item consumed to construct this building.
    pub fn needs(self, item: &Item) -> bool {
        match self {
            BuildingKind::Carpenter | BuildingKind::Butcher | BuildingKind::Depot => item.kind == ItemKind::Log,
            BuildingKind::Bed => item.kind == ItemKind::Bed,
            BuildingKind::Table => item.kind == ItemKind::Table,
            BuildingKind::Chair => item.kind == ItemKind::Chair,
            BuildingKind::Door => item.kind == ItemKind::Door,
            _ => item.kind == ItemKind::Boulder && item.mat.is_building_stone(),
        }
    }

    pub fn needs_desc(self) -> &'static str {
        match self {
            BuildingKind::Carpenter | BuildingKind::Butcher | BuildingKind::Depot => "a log",
            BuildingKind::Bed => "a bed",
            BuildingKind::Table => "a table",
            BuildingKind::Chair => "a chair",
            BuildingKind::Door => "a door",
            _ => "a stone boulder",
        }
    }

    pub fn recipes(self) -> &'static [Recipe] {
        use Recipe::*;
        match self {
            BuildingKind::Carpenter => &[WoodBed, WoodTable, WoodChair, WoodDoor, Barrel, Bin],
            BuildingKind::Mason => &[Blocks, StoneTable, StoneChair, StoneDoor],
            BuildingKind::Still => &[Brew],
            BuildingKind::Kitchen => &[Cook],
            BuildingKind::Butcher => &[Butcher],
            BuildingKind::WoodFurnace => &[Charcoal],
            BuildingKind::Smelter => &[SmeltOre, MakeSteel],
            BuildingKind::Forge => &[
                ForgePick,
                ForgeAxe,
                ForgeSword,
                ForgeSpear,
                ForgeHammer,
                ForgeHelm,
                ForgeBreastplate,
                ForgeShield,
            ],
            BuildingKind::Craftsdwarf => &[StoneCraft, BoneCraft, CutGem],
            _ => &[],
        }
    }

    pub fn color(self) -> Color {
        match self {
            BuildingKind::Carpenter => Color::Rgb(190, 130, 60),
            BuildingKind::Mason => Color::Rgb(185, 185, 195),
            BuildingKind::Still => Color::Rgb(200, 120, 220),
            BuildingKind::Kitchen => Color::Rgb(240, 200, 120),
            BuildingKind::Butcher => Color::Rgb(220, 90, 90),
            BuildingKind::WoodFurnace => Color::Rgb(230, 120, 40),
            BuildingKind::Smelter => Color::Rgb(255, 150, 50),
            BuildingKind::Forge => Color::Rgb(255, 90, 40),
            BuildingKind::Craftsdwarf => Color::Rgb(120, 200, 220),
            BuildingKind::Depot => Color::Rgb(240, 240, 160),
            _ => Color::Rgb(200, 200, 200),
        }
    }

    pub fn letter(self) -> char {
        match self {
            BuildingKind::Carpenter => 'C',
            BuildingKind::Mason => 'M',
            BuildingKind::Still => 'S',
            BuildingKind::Kitchen => 'K',
            BuildingKind::Butcher => 'B',
            BuildingKind::WoodFurnace => 'W',
            BuildingKind::Smelter => 'E',
            BuildingKind::Forge => 'F',
            BuildingKind::Craftsdwarf => 'R',
            BuildingKind::Depot => 'D',
            BuildingKind::Bed => 'Θ',
            BuildingKind::Table => '╥',
            BuildingKind::Chair => 'h',
            BuildingKind::Door => '+',
        }
    }
}

/// One ingredient slot of a recipe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    Log,
    Stone,
    Mushroom,
    Edible,
    Carcass,
    Ore,
    Fuel,
    Bar,
    IronBar,
    Flux,
    Bone,
    RoughGem,
}

impl Input {
    pub fn accepts(self, it: &Item) -> bool {
        match self {
            Input::Log => it.kind == ItemKind::Log,
            Input::Stone => it.kind == ItemKind::Boulder && it.mat.is_building_stone(),
            Input::Mushroom => it.kind == ItemKind::Mushroom,
            Input::Edible => matches!(it.kind, ItemKind::Mushroom | ItemKind::Meat),
            Input::Carcass => it.kind == ItemKind::Carcass,
            Input::Ore => it.kind == ItemKind::Boulder && it.mat.smelts_to().is_some(),
            Input::Fuel => it.kind == ItemKind::Charcoal || (it.kind == ItemKind::Boulder && it.mat == Mat::Coal),
            Input::Bar => it.kind == ItemKind::Bar,
            Input::IronBar => it.kind == ItemKind::Bar && it.mat == Mat::Iron,
            Input::Flux => it.kind == ItemKind::Boulder && it.mat == Mat::Limestone,
            Input::Bone => it.kind == ItemKind::Bone,
            Input::RoughGem => it.kind == ItemKind::Boulder && it.mat.is_gem(),
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Input::Log => "log",
            Input::Stone => "stone boulder",
            Input::Mushroom => "stoutcap",
            Input::Edible => "mushroom or meat",
            Input::Carcass => "carcass",
            Input::Ore => "metal ore",
            Input::Fuel => "fuel (charcoal or coal)",
            Input::Bar => "metal bar",
            Input::IronBar => "iron bar",
            Input::Flux => "limestone (flux)",
            Input::Bone => "bone",
            Input::RoughGem => "rough gem",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Recipe {
    WoodBed,
    WoodTable,
    WoodChair,
    WoodDoor,
    Barrel,
    Bin,
    Blocks,
    StoneTable,
    StoneChair,
    StoneDoor,
    Brew,
    Cook,
    Butcher,
    Charcoal,
    SmeltOre,
    MakeSteel,
    ForgePick,
    ForgeAxe,
    ForgeSword,
    ForgeSpear,
    ForgeHammer,
    ForgeHelm,
    ForgeBreastplate,
    ForgeShield,
    StoneCraft,
    BoneCraft,
    CutGem,
}

impl Recipe {
    pub fn name(self) -> &'static str {
        use Recipe::*;
        match self {
            WoodBed => "Make wooden bed",
            WoodTable => "Make wooden table",
            WoodChair => "Make wooden chair",
            WoodDoor => "Make wooden door",
            Barrel => "Make barrel",
            Bin => "Make bin",
            Blocks => "Cut stone blocks",
            StoneTable => "Carve stone table",
            StoneChair => "Carve stone chair",
            StoneDoor => "Carve stone door",
            Brew => "Brew stoutcap ale",
            Cook => "Prepare meal",
            Butcher => "Butcher a carcass",
            Charcoal => "Make charcoal",
            SmeltOre => "Smelt ore",
            MakeSteel => "Make steel",
            ForgePick => "Forge pick",
            ForgeAxe => "Forge battle axe",
            ForgeSword => "Forge short sword",
            ForgeSpear => "Forge spear",
            ForgeHammer => "Forge war hammer",
            ForgeHelm => "Forge helm",
            ForgeBreastplate => "Forge breastplate",
            ForgeShield => "Forge shield",
            StoneCraft => "Make stone crafts",
            BoneCraft => "Make bone crafts",
            CutGem => "Cut a gem",
        }
    }

    pub fn key(self) -> char {
        use Recipe::*;
        match self {
            WoodBed | Brew | Cook | Butcher | Charcoal | BoneCraft => 'b',
            WoodTable | StoneTable => 't',
            WoodChair | StoneChair => 'h',
            WoodDoor | StoneDoor => 'd',
            Barrel => 'r',
            Bin => 'n',
            Blocks => 'k',
            SmeltOre => 'o',
            MakeSteel => 's',
            ForgePick => 'p',
            ForgeAxe => 'a',
            ForgeSword => 'w',
            ForgeSpear => 's',
            ForgeHammer => 'h',
            ForgeHelm => 'e',
            ForgeBreastplate => 'b',
            ForgeShield => 'i',
            StoneCraft => 'c',
            CutGem => 'g',
        }
    }

    pub fn labor(self) -> Labor {
        use Recipe::*;
        match self {
            WoodBed | WoodTable | WoodChair | WoodDoor | Barrel | Bin => Labor::Carpentry,
            Blocks | StoneTable | StoneChair | StoneDoor => Labor::Masonry,
            Brew => Labor::Brewing,
            Cook => Labor::Cooking,
            Butcher => Labor::Butchery,
            Charcoal | SmeltOre | MakeSteel => Labor::Smelting,
            ForgePick | ForgeAxe | ForgeSword | ForgeSpear | ForgeHammer | ForgeHelm | ForgeBreastplate
            | ForgeShield => Labor::Smithing,
            StoneCraft | BoneCraft | CutGem => Labor::Crafting,
        }
    }

    pub fn inputs(self) -> &'static [Input] {
        use Recipe::*;
        match self {
            WoodBed | WoodTable | WoodChair | WoodDoor | Barrel | Bin | Charcoal => &[Input::Log],
            Blocks | StoneTable | StoneChair | StoneDoor | StoneCraft => &[Input::Stone],
            Brew => &[Input::Mushroom],
            Cook => &[Input::Edible],
            Butcher => &[Input::Carcass],
            SmeltOre => &[Input::Ore, Input::Fuel],
            MakeSteel => &[Input::IronBar, Input::Fuel, Input::Flux],
            ForgeBreastplate => &[Input::Bar, Input::Bar, Input::Fuel],
            ForgePick | ForgeAxe | ForgeSword | ForgeSpear | ForgeHammer | ForgeHelm | ForgeShield => {
                &[Input::Bar, Input::Fuel]
            }
            BoneCraft => &[Input::Bone],
            CutGem => &[Input::RoughGem],
        }
    }

    pub fn input_desc(self) -> String {
        self.inputs().iter().map(|i| i.desc()).collect::<Vec<_>>().join(" + ")
    }

    /// What comes out, given the consumed inputs (in recipe order).
    pub fn produce(self, inputs: &[Item]) -> Vec<(ItemKind, Mat, Option<String>)> {
        use Recipe::*;
        let m = inputs.first().map_or(Mat::Wood, |i| i.mat);
        let one = |k: ItemKind| vec![(k, m, None)];
        match self {
            WoodBed => one(ItemKind::Bed),
            WoodTable | StoneTable => one(ItemKind::Table),
            WoodChair | StoneChair => one(ItemKind::Chair),
            WoodDoor | StoneDoor => one(ItemKind::Door),
            Barrel => one(ItemKind::Barrel),
            Bin => one(ItemKind::Bin),
            Blocks => vec![(ItemKind::Block, m, None); 4],
            Brew => {
                let mut v = vec![(ItemKind::Drink, Mat::Stoutcap, None); 4];
                v.push((ItemKind::Seed, Mat::Stoutcap, None));
                v
            }
            Cook => vec![(ItemKind::Meal, m, None); 2],
            Butcher => {
                let label = inputs.first().and_then(|i| i.label.clone());
                let meat = match label.as_deref() {
                    Some("bear") => 7,
                    Some("deer") => 4,
                    _ => 3,
                };
                let mut v = vec![(ItemKind::Meat, Mat::Flesh, label.clone()); meat];
                v.push((ItemKind::Bone, Mat::Bone, label.clone()));
                v.push((ItemKind::Bone, Mat::Bone, label.clone()));
                v.push((ItemKind::Leather, Mat::Leather, label));
                v
            }
            Charcoal => vec![(ItemKind::Charcoal, Mat::Coal, None)],
            SmeltOre => {
                let metal = m.smelts_to().unwrap_or(Mat::Iron);
                vec![(ItemKind::Bar, metal, None); 2]
            }
            MakeSteel => vec![(ItemKind::Bar, Mat::Steel, None)],
            ForgePick => one(ItemKind::Pick),
            ForgeAxe => one(ItemKind::Axe),
            ForgeSword => one(ItemKind::Sword),
            ForgeSpear => one(ItemKind::Spear),
            ForgeHammer => one(ItemKind::Hammer),
            ForgeHelm => one(ItemKind::Helm),
            ForgeBreastplate => one(ItemKind::Breastplate),
            ForgeShield => one(ItemKind::Shield),
            StoneCraft | BoneCraft => one(ItemKind::Craft),
            CutGem => one(ItemKind::Gem),
        }
    }

    pub fn work(self) -> u32 {
        use Recipe::*;
        match self {
            Blocks | Charcoal => 90,
            Brew | Cook => 110,
            Butcher => 120,
            SmeltOre | MakeSteel => 160,
            ForgeBreastplate => 260,
            ForgePick | ForgeAxe | ForgeSword | ForgeSpear | ForgeHammer | ForgeHelm | ForgeShield => 200,
            CutGem => 180,
            _ => 150,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Order {
    pub recipe: Recipe,
    pub repeat: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Building {
    pub id: u32,
    pub kind: BuildingKind,
    /// Top-left corner of the footprint.
    pub pos: Pos,
    pub built: bool,
    pub mat: Mat,
    pub orders: Vec<Order>,
    pub worker: Option<u32>,
    pub occupant: Option<u32>,
    pub retry_tick: u64,
    #[serde(default)]
    pub locked: bool,
    /// Bed owner.
    #[serde(default)]
    pub owner: Option<u32>,
    /// Quality of the item it was built from.
    #[serde(default)]
    pub quality: u8,
    /// Value of the enclosed room around this bed or table (0 = not a room).
    #[serde(default)]
    pub room_value: u32,
}

impl Building {
    pub fn center(&self) -> Pos {
        let s = self.kind.size() / 2;
        self.pos.offset(s, s, 0)
    }

    pub fn tiles(&self) -> Vec<Pos> {
        let s = self.kind.size();
        let mut v = Vec::new();
        for dy in 0..s {
            for dx in 0..s {
                v.push(self.pos.offset(dx, dy, 0));
            }
        }
        v
    }

    pub fn name(&self) -> String {
        if self.built && !self.kind.is_workshop() {
            format!("{} {}", self.mat.adj(), self.kind.name().to_lowercase())
        } else {
            self.kind.name().to_string()
        }
    }

    /// Glyph for the tile at offset (dx, dy) inside the footprint.
    pub fn glyph_at(&self, dx: i32, dy: i32) -> char {
        if !self.kind.is_workshop() {
            return self.kind.letter();
        }
        match (dx, dy) {
            (0, 0) => '┌',
            (2, 0) => '┐',
            (0, 2) => '└',
            (2, 2) => '┘',
            (1, 0) | (1, 2) => '─',
            (0, 1) | (2, 1) => '│',
            _ => self.kind.letter(),
        }
    }
}
