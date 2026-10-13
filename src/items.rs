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
    Seed,
    Meat,
    Meal,
    Bone,
    Leather,
    Charcoal,
    Bar,
    Bin,
    Craft,
    Gem,
    Mechanism,
    Cage,
    Crossbow,
    /// A bundle of ten crossbow bolts.
    Bolt,
}

/// Items that fit in a barrel or a bin, ten to a container.
pub const CONTAINER_CAP: usize = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Slot {
    Head,
    Body,
    Shield,
}

impl ItemKind {
    pub fn is_food(self) -> bool {
        matches!(self, ItemKind::Mushroom | ItemKind::Meat | ItemKind::Meal)
    }

    pub fn is_container(self) -> bool {
        matches!(self, ItemKind::Barrel | ItemKind::Bin)
    }

    /// Which container (if any) this kind of item can be stored in.
    pub fn stored_in(self) -> Option<ItemKind> {
        use ItemKind::*;
        match self {
            Mushroom | Drink | Seed | Meat | Meal => Some(Barrel),
            Bar | Charcoal | Craft | Gem | Bone | Leather | Block => Some(Bin),
            k if k.is_equipment() => Some(Bin),
            _ => None,
        }
    }

    pub fn base_value(self) -> u32 {
        use ItemKind::*;
        match self {
            Boulder | Log | Bone | Charcoal => 1,
            Seed | Corpse | Carcass => 0,
            Mushroom | Drink | Meat => 2,
            Block | Leather => 3,
            Barrel | Bin => 5,
            Meal => 6,
            Bed | Table | Chair | Door => 10,
            Bar => 10,
            Craft => 20,
            Pick | Axe | Sword | Spear | Hammer => 25,
            Helm | Shield => 25,
            Breastplate => 40,
            Gem => 40,
            Mechanism => 8,
            Cage => 6,
            Crossbow => 20,
            Bolt => 3,
        }
    }

    /// Base damage when swung, for anything usable as a weapon.
    pub fn weapon_damage(self) -> Option<f32> {
        match self {
            ItemKind::Pick => Some(6.0),
            ItemKind::Axe => Some(9.0),
            ItemKind::Sword => Some(8.0),
            ItemKind::Spear => Some(8.0),
            ItemKind::Hammer => Some(9.0),
            // Swung, a crossbow is just a club.
            ItemKind::Crossbow => Some(3.0),
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
        self.weapon_damage().is_some() || self.armor_slot().is_some() || self == ItemKind::Bolt
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
    Metal,
    Goods,
}

impl StockFilter {
    pub const ALL: [StockFilter; 9] = [
        StockFilter::All,
        StockFilter::Stone,
        StockFilter::Wood,
        StockFilter::Food,
        StockFilter::Furniture,
        StockFilter::Equipment,
        StockFilter::Metal,
        StockFilter::Goods,
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
            StockFilter::Metal => "bars & fuel",
            StockFilter::Goods => "finished goods",
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
            StockFilter::Metal => 'm',
            StockFilter::Goods => 'g',
        }
    }

    pub fn accepts(self, k: ItemKind) -> bool {
        use ItemKind::*;
        match self {
            StockFilter::All => true,
            StockFilter::Stone => matches!(k, Boulder | Block),
            StockFilter::Wood => k == Log,
            StockFilter::Food => matches!(k, Mushroom | Drink | Seed | Meat | Meal),
            StockFilter::Furniture => matches!(k, Bed | Table | Chair | Door | Barrel | Bin | Cage),
            StockFilter::Metal => matches!(k, Bar | Charcoal),
            StockFilter::Goods => matches!(k, Craft | Gem | Bone | Leather | Mechanism),
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
    /// Barrel or bin this item is stored in.
    #[serde(default)]
    pub container: Option<u32>,
    /// 0 ordinary .. 5 masterful, 6 artifact (named in `label`).
    #[serde(default)]
    pub quality: u8,
    /// Marked to be carried to the trade depot.
    #[serde(default)]
    pub to_depot: bool,
    /// Marked to be thrown on the garbage dump.
    #[serde(default)]
    pub dump: bool,
}

pub const ARTIFACT: u8 = 6;

pub fn quality_prefix(q: u8) -> &'static str {
    match q {
        1 => "well-crafted ",
        2 => "finely-crafted ",
        3 => "superior ",
        4 => "exceptional ",
        5 => "masterful ",
        _ => "",
    }
}

pub fn quality_mult(q: u8) -> u32 {
    match q {
        0 => 1,
        1 => 2,
        2 => 3,
        3 => 4,
        4 => 5,
        5 => 10,
        _ => 100,
    }
}

impl Item {
    pub fn name(&self) -> String {
        if self.quality >= ARTIFACT {
            let base = Item { quality: 0, label: None, ..self.clone() };
            return format!("{} ({})", self.label.as_deref().unwrap_or("an artifact"), base.base_name());
        }
        format!("{}{}", quality_prefix(self.quality), self.base_name())
    }

    fn base_name(&self) -> String {
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
            Seed => "stoutcap spawn".into(),
            Meat => format!("{} meat", self.label.as_deref().unwrap_or("animal")),
            Meal => "prepared meal".into(),
            Bone => format!("{} bone", self.label.as_deref().unwrap_or("animal")),
            Leather => format!("{} leather", self.label.as_deref().unwrap_or("animal")),
            Charcoal => "charcoal".into(),
            Bar => format!("{} bar", m.name()),
            Bin => format!("{} bin", m.adj()),
            Craft => format!("{} figurine", m.adj()),
            Gem => format!("cut {}", m.name()),
            Mechanism => format!("{} mechanism", m.adj()),
            Crossbow => format!("{} crossbow", m.adj()),
            Bolt => format!("{} bolts (10)", m.adj()),
            Cage => match self.label.as_deref() {
                Some(beast) => format!("{} cage (holding a {})", m.adj(), beast),
                None => format!("{} cage", m.adj()),
            },
        }
    }

    pub fn value(&self) -> u32 {
        self.kind.base_value() * self.mat.value() * quality_mult(self.quality)
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
            Seed => ':',
            Meat => '%',
            Meal => '%',
            Bone => '~',
            Leather => '~',
            Charcoal => '*',
            Bar => '=',
            Bin => '■',
            Craft => '☼',
            Gem => '☼',
            Mechanism => '*',
            Cage => '#',
            Crossbow => '}',
            Bolt => '/',
        };
        let col = match self.kind {
            Drink => Color::Rgb(230, 170, 60),
            Corpse => Color::Rgb(200, 60, 60),
            Carcass => Color::Rgb(190, 120, 90),
            Meat => Color::Rgb(220, 90, 90),
            Meal => Color::Rgb(250, 200, 110),
            Seed => Color::Rgb(160, 120, 170),
            Charcoal => Color::Rgb(90, 90, 90),
            Bin | Barrel => Mat::Wood.color(),
            _ => self.mat.color(),
        };
        (c, col)
    }
}
