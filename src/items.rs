use crate::geom::Pos;
use crate::material::Mat;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemKind {
    Boulder,
    Log,
    Block,
    Bed,
    Table,
    Chair,
    Door,
    Barrel,
    Mushroom,
    Drink,
    Corpse,
    Carcass,
    Pick,
    Axe,
    Sword,
    Spear,
    Hammer,
    Helm,
    Breastplate,
    Shield,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Slot {
    Head,
    Body,
    Shield,
}

impl ItemKind {
    pub fn is_food(self) -> bool {
        self == ItemKind::Mushroom
    }

    /// Base damage when swung, for anything usable as a weapon.
    pub fn weapon_damage(self) -> Option<f32> {
        match self {
            ItemKind::Pick => Some(6.0),
            ItemKind::Axe => Some(9.0),
            ItemKind::Sword => Some(8.0),
            ItemKind::Spear => Some(8.0),
            ItemKind::Hammer => Some(9.0),
            _ => None,
        }
    }

    pub fn weapon_verb(self) -> &'static str {
        match self {
            ItemKind::Pick => "strikes",
            ItemKind::Axe => "hacks",
            ItemKind::Sword => "slashes",
            ItemKind::Spear => "stabs",
            ItemKind::Hammer => "bashes",
            _ => "punches",
        }
    }

    pub fn edged(self) -> bool {
        matches!(self, ItemKind::Axe | ItemKind::Sword | ItemKind::Spear)
    }

    pub fn armor_slot(self) -> Option<Slot> {
        match self {
            ItemKind::Helm => Some(Slot::Head),
            ItemKind::Breastplate => Some(Slot::Body),
            ItemKind::Shield => Some(Slot::Shield),
            _ => None,
        }
    }

    pub fn is_equipment(self) -> bool {
        self.weapon_damage().is_some() || self.armor_slot().is_some()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StockFilter {
    Equipment,
    All,
    Stone,
    Wood,
    Food,
    Furniture,
    Refuse,
}

impl StockFilter {
    pub const ALL: [StockFilter; 7] = [
        StockFilter::All,
        StockFilter::Stone,
        StockFilter::Wood,
        StockFilter::Food,
        StockFilter::Furniture,
        StockFilter::Equipment,
        StockFilter::Refuse,
    ];

    pub fn name(self) -> &'static str {
        match self {
            StockFilter::All => "everything",
            StockFilter::Equipment => "weapons & armor",
            StockFilter::Stone => "stone",
            StockFilter::Wood => "wood",
            StockFilter::Food => "food & drink",
            StockFilter::Furniture => "furniture",
            StockFilter::Refuse => "refuse",
        }
    }

    pub fn key(self) -> char {
        match self {
            StockFilter::All => 'a',
            StockFilter::Equipment => 'e',
            StockFilter::Stone => 's',
            StockFilter::Wood => 'w',
            StockFilter::Food => 'f',
            StockFilter::Furniture => 'u',
            StockFilter::Refuse => 'r',
        }
    }

    pub fn accepts(self, k: ItemKind) -> bool {
        use ItemKind::*;
        match self {
            StockFilter::All => true,
            StockFilter::Stone => matches!(k, Boulder | Block),
            StockFilter::Wood => k == Log,
            StockFilter::Food => matches!(k, Mushroom | Drink),
            StockFilter::Furniture => matches!(k, Bed | Table | Chair | Door | Barrel),
            StockFilter::Refuse => matches!(k, Corpse | Carcass),
            StockFilter::Equipment => k.is_equipment(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    pub id: u32,
    pub kind: ItemKind,
    pub mat: Mat,
    pub pos: Pos,
    pub carried_by: Option<u32>,
    pub claimed_by: Option<u32>,
    pub label: Option<String>,
}

impl Item {
    pub fn name(&self) -> String {
        use ItemKind::*;
        let m = self.mat;
        match self.kind {
            Boulder if m.is_gem() => format!("rough {}", m.name()),
            Boulder if m.is_ore() => format!("{} ore", m.name()),
            Boulder => format!("{} boulder", m.name()),
            Log => "wooden log".into(),
            Block => format!("{} blocks", m.name()),
            Bed => format!("{} bed", m.adj()),
            Table => format!("{} table", m.adj()),
            Chair => format!("{} chair", m.adj()),
            Door => format!("{} door", m.adj()),
            Barrel => format!("{} barrel", m.adj()),
            Mushroom => "stoutcap mushroom".into(),
            Drink => "barrel of stoutcap ale".into(),
            Corpse => format!("corpse of {}", self.label.as_deref().unwrap_or("a dwarf")),
            Carcass => format!("{} carcass", self.label.as_deref().unwrap_or("animal")),
            Pick => format!("{} pick", m.adj()),
            Axe => format!("{} battle axe", m.adj()),
            Sword => format!("{} short sword", m.adj()),
            Spear => format!("{} spear", m.adj()),
            Hammer => format!("{} war hammer", m.adj()),
            Helm => format!("{} helm", m.adj()),
            Breastplate => format!("{} breastplate", m.adj()),
            Shield => format!("{} shield", m.adj()),
        }
    }

    pub fn glyph(&self) -> (char, Color) {
        use ItemKind::*;
        let c = match self.kind {
            Boulder if self.mat.is_gem() => '☼',
            Boulder if self.mat.is_ore() => '*',
            Boulder => '•',
            Log => '=',
            Block => '■',
            Bed => 'Θ',
            Table => '╥',
            Chair => 'h',
            Door => '+',
            Barrel => '0',
            Mushroom => '%',
            Drink => '!',
            Corpse | Carcass => '&',
            Pick | Axe | Sword | Spear | Hammer => '/',
            Helm | Breastplate => '[',
            Shield => ')',
        };
        let col = match self.kind {
            Drink => Color::Rgb(230, 170, 60),
            Corpse => Color::Rgb(200, 60, 60),
            Carcass => Color::Rgb(190, 120, 90),
            _ => self.mat.color(),
        };
        (c, col)
    }
}
