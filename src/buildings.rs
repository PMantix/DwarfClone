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
    Bed,
    Table,
    Chair,
    Door,
}

impl BuildingKind {
    pub const ALL: [BuildingKind; 7] = [
        BuildingKind::Carpenter,
        BuildingKind::Mason,
        BuildingKind::Still,
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
            BuildingKind::Bed => 'b',
            BuildingKind::Table => 't',
            BuildingKind::Chair => 'h',
            BuildingKind::Door => 'd',
        }
    }

    pub fn is_workshop(self) -> bool {
        matches!(self, BuildingKind::Carpenter | BuildingKind::Mason | BuildingKind::Still)
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
            BuildingKind::Carpenter => item.kind == ItemKind::Log,
            BuildingKind::Mason | BuildingKind::Still => {
                item.kind == ItemKind::Boulder && item.mat.is_building_stone()
            }
            BuildingKind::Bed => item.kind == ItemKind::Bed,
            BuildingKind::Table => item.kind == ItemKind::Table,
            BuildingKind::Chair => item.kind == ItemKind::Chair,
            BuildingKind::Door => item.kind == ItemKind::Door,
        }
    }

    pub fn needs_desc(self) -> &'static str {
        match self {
            BuildingKind::Carpenter => "a log",
            BuildingKind::Mason | BuildingKind::Still => "a stone boulder",
            BuildingKind::Bed => "a bed",
            BuildingKind::Table => "a table",
            BuildingKind::Chair => "a chair",
            BuildingKind::Door => "a door",
        }
    }

    pub fn recipes(self) -> &'static [Recipe] {
        match self {
            BuildingKind::Carpenter => &[
                Recipe::WoodBed,
                Recipe::WoodTable,
                Recipe::WoodChair,
                Recipe::WoodDoor,
                Recipe::Barrel,
            ],
            BuildingKind::Mason => &[
                Recipe::Blocks,
                Recipe::StoneTable,
                Recipe::StoneChair,
                Recipe::StoneDoor,
            ],
            BuildingKind::Still => &[Recipe::Brew],
            _ => &[],
        }
    }

    pub fn color(self) -> Color {
        match self {
            BuildingKind::Carpenter => Color::Rgb(190, 130, 60),
            BuildingKind::Mason => Color::Rgb(185, 185, 195),
            BuildingKind::Still => Color::Rgb(200, 120, 220),
            _ => Color::Rgb(200, 200, 200),
        }
    }

    pub fn letter(self) -> char {
        match self {
            BuildingKind::Carpenter => 'C',
            BuildingKind::Mason => 'M',
            BuildingKind::Still => 'S',
            BuildingKind::Bed => 'Θ',
            BuildingKind::Table => '╥',
            BuildingKind::Chair => 'h',
            BuildingKind::Door => '+',
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
    Blocks,
    StoneTable,
    StoneChair,
    StoneDoor,
    Brew,
}

impl Recipe {
    pub fn name(self) -> &'static str {
        match self {
            Recipe::WoodBed => "Make wooden bed",
            Recipe::WoodTable => "Make wooden table",
            Recipe::WoodChair => "Make wooden chair",
            Recipe::WoodDoor => "Make wooden door",
            Recipe::Barrel => "Make barrel",
            Recipe::Blocks => "Cut stone blocks",
            Recipe::StoneTable => "Carve stone table",
            Recipe::StoneChair => "Carve stone chair",
            Recipe::StoneDoor => "Carve stone door",
            Recipe::Brew => "Brew stoutcap ale",
        }
    }

    pub fn key(self) -> char {
        match self {
            Recipe::WoodBed | Recipe::Brew => 'b',
            Recipe::WoodTable | Recipe::StoneTable => 't',
            Recipe::WoodChair | Recipe::StoneChair => 'h',
            Recipe::WoodDoor | Recipe::StoneDoor => 'd',
            Recipe::Barrel => 'r',
            Recipe::Blocks => 'k',
        }
    }

    pub fn labor(self) -> Labor {
        match self {
            Recipe::WoodBed | Recipe::WoodTable | Recipe::WoodChair | Recipe::WoodDoor | Recipe::Barrel => {
                Labor::Carpentry
            }
            Recipe::Blocks | Recipe::StoneTable | Recipe::StoneChair | Recipe::StoneDoor => Labor::Masonry,
            Recipe::Brew => Labor::Brewing,
        }
    }

    pub fn accepts(self, item: &Item) -> bool {
        match self.labor() {
            Labor::Carpentry => item.kind == ItemKind::Log,
            Labor::Masonry => item.kind == ItemKind::Boulder && item.mat.is_building_stone(),
            _ => item.kind == ItemKind::Mushroom,
        }
    }

    pub fn input_desc(self) -> &'static str {
        match self.labor() {
            Labor::Carpentry => "log",
            Labor::Masonry => "stone boulder",
            _ => "stoutcap mushroom",
        }
    }

    pub fn output(self) -> (ItemKind, u32) {
        match self {
            Recipe::WoodBed => (ItemKind::Bed, 1),
            Recipe::WoodTable | Recipe::StoneTable => (ItemKind::Table, 1),
            Recipe::WoodChair | Recipe::StoneChair => (ItemKind::Chair, 1),
            Recipe::WoodDoor | Recipe::StoneDoor => (ItemKind::Door, 1),
            Recipe::Barrel => (ItemKind::Barrel, 1),
            Recipe::Blocks => (ItemKind::Block, 4),
            Recipe::Brew => (ItemKind::Drink, 4),
        }
    }

    pub fn work(self) -> u32 {
        match self {
            Recipe::Blocks => 90,
            Recipe::Brew => 110,
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
