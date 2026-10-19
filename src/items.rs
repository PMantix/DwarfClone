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
    /// A harvested crop (stoutcap mushrooms, grain, berries...).
    #[serde(alias = "Mushroom")]
    Plant,
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
    Thread,
    Splint,
    Crutch,
    Coffin,
    /// A memorial slab.
    Slab,
    /// Labelled "water" while full.
    Bucket,
    /// An untanned skin, labelled with the animal.
    Hide,
    /// A bolt of woven cloth.
    Cloth,
    Shirt,
    Trousers,
    Shoes,
    /// Wood ash from the furnace, for lye.
    Ash,
    /// Rendered animal fat.
    Tallow,
    Soap,
    /// A caught fish, labelled with its kind.
    Fish,
    /// Ground grain, for the kitchen.
    Flour,
    /// A severed limb, head or teeth, labelled with whose ("Urist's left arm").
    BodyPart,
    Cabinet,
    Chest,
    Statue,
    /// A musical instrument, labelled with its people's name for it (`arts.rs`).
    Instrument,
    /// An animal product kept as goods (wool, feathers, ivory, pearls...),
    /// labelled with what it is ("sheep wool").
    Product,
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
    pub const ALL: [ItemKind; 56] = [
        ItemKind::Boulder,
        ItemKind::Log,
        ItemKind::Block,
        ItemKind::Bed,
        ItemKind::Table,
        ItemKind::Chair,
        ItemKind::Door,
        ItemKind::Barrel,
        ItemKind::Plant,
        ItemKind::Drink,
        ItemKind::Corpse,
        ItemKind::Carcass,
        ItemKind::Pick,
        ItemKind::Axe,
        ItemKind::Sword,
        ItemKind::Spear,
        ItemKind::Hammer,
        ItemKind::Helm,
        ItemKind::Breastplate,
        ItemKind::Shield,
        ItemKind::Seed,
        ItemKind::Meat,
        ItemKind::Meal,
        ItemKind::Bone,
        ItemKind::Leather,
        ItemKind::Charcoal,
        ItemKind::Bar,
        ItemKind::Bin,
        ItemKind::Craft,
        ItemKind::Gem,
        ItemKind::Mechanism,
        ItemKind::Cage,
        ItemKind::Crossbow,
        ItemKind::Bolt,
        ItemKind::Thread,
        ItemKind::Splint,
        ItemKind::Crutch,
        ItemKind::Coffin,
        ItemKind::Slab,
        ItemKind::Bucket,
        ItemKind::Hide,
        ItemKind::Cloth,
        ItemKind::Shirt,
        ItemKind::Trousers,
        ItemKind::Shoes,
        ItemKind::Ash,
        ItemKind::Tallow,
        ItemKind::Soap,
        ItemKind::Fish,
        ItemKind::Flour,
        ItemKind::BodyPart,
        ItemKind::Cabinet,
        ItemKind::Chest,
        ItemKind::Statue,
        ItemKind::Instrument,
        ItemKind::Product,
    ];

    /// Plural name for lists ("logs", "metal bars").
    pub fn plural(self) -> &'static str {
        match self {
            ItemKind::Boulder => "boulders",
            ItemKind::Log => "logs",
            ItemKind::Block => "blocks",
            ItemKind::Bed => "beds",
            ItemKind::Table => "tables",
            ItemKind::Chair => "chairs",
            ItemKind::Door => "doors",
            ItemKind::Barrel => "barrels",
            ItemKind::Plant => "plants",
            ItemKind::Drink => "drinks",
            ItemKind::Corpse => "corpses",
            ItemKind::Carcass => "carcasses",
            ItemKind::Pick => "picks",
            ItemKind::Axe => "battle axes",
            ItemKind::Sword => "swords",
            ItemKind::Spear => "spears",
            ItemKind::Hammer => "war hammers",
            ItemKind::Helm => "helms",
            ItemKind::Breastplate => "breastplates",
            ItemKind::Shield => "shields",
            ItemKind::Seed => "seeds",
            ItemKind::Meat => "meat",
            ItemKind::Meal => "meals",
            ItemKind::Bone => "bones",
            ItemKind::Leather => "leather",
            ItemKind::Charcoal => "charcoal",
            ItemKind::Bar => "metal bars",
            ItemKind::Bin => "bins",
            ItemKind::Craft => "crafts",
            ItemKind::Gem => "cut gems",
            ItemKind::Mechanism => "mechanisms",
            ItemKind::Cage => "cages",
            ItemKind::Crossbow => "crossbows",
            ItemKind::Bolt => "bolts",
            ItemKind::Thread => "thread",
            ItemKind::Splint => "splints",
            ItemKind::Crutch => "crutchs",
            ItemKind::Coffin => "coffins",
            ItemKind::Slab => "slabs",
            ItemKind::Bucket => "buckets",
            ItemKind::Hide => "hides",
            ItemKind::Cloth => "cloth",
            ItemKind::Shirt => "shirts",
            ItemKind::Trousers => "trousers",
            ItemKind::Shoes => "shoes",
            ItemKind::Ash => "ash",
            ItemKind::Tallow => "tallow",
            ItemKind::Soap => "soap",
            ItemKind::Fish => "fish",
            ItemKind::Flour => "flour",
            ItemKind::BodyPart => "body parts",
            ItemKind::Cabinet => "cabinets",
            ItemKind::Chest => "chests",
            ItemKind::Statue => "statues",
            ItemKind::Instrument => "musical instruments",
            ItemKind::Product => "animal products",
        }
    }

    pub fn is_food(self) -> bool {
        matches!(self, ItemKind::Plant | ItemKind::Meat | ItemKind::Meal | ItemKind::Fish)
    }

    pub fn is_container(self) -> bool {
        matches!(self, ItemKind::Barrel | ItemKind::Bin)
    }

    /// Which container (if any) this kind of item can be stored in.
    pub fn stored_in(self) -> Option<ItemKind> {
        use ItemKind::*;
        match self {
            Plant | Drink | Seed | Meat | Meal | Tallow | Fish | Flour => Some(Barrel),
            Bar | Charcoal | Craft | Instrument | Gem | Bone | Leather | Block | Cloth | Shirt | Trousers | Shoes | Ash | Soap | Product => Some(Bin),
            k if k.is_equipment() => Some(Bin),
            _ => None,
        }
    }

    pub fn base_value(self) -> u32 {
        use ItemKind::*;
        match self {
            Boulder | Log | Bone | Charcoal => 1,
            Seed | Corpse | Carcass | BodyPart => 0,
            Hide | Ash => 1,
            Tallow => 2,
            Soap => 5,
            Cloth => 5,
            Shirt | Trousers | Shoes => 10,
            Plant | Drink | Meat | Fish | Flour => 2,
            Block | Leather | Bucket => 3,
            Barrel | Bin => 5,
            Meal => 6,
            Bed | Table | Chair | Door | Cabinet | Chest => 10,
            Statue => 30,
            Product => 4,
            Bar => 10,
            Craft => 20,
            Instrument => 25,
            Pick | Axe | Sword | Spear | Hammer => 25,
            Helm | Shield => 25,
            Breastplate => 40,
            Gem => 40,
            Mechanism => 8,
            Cage => 6,
            Crossbow => 20,
            Bolt => 3,
            Thread => 2,
            Splint | Crutch => 4,
            Coffin | Slab => 10,
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

    /// Shirt 0, trousers 1, shoes 2.
    pub fn clothing_slot(self) -> Option<u8> {
        match self {
            ItemKind::Shirt => Some(0),
            ItemKind::Trousers => Some(1),
            ItemKind::Shoes => Some(2),
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
    /// The hospital's own: what a doctor dresses wounds and sets bones with.
    Medical,
}

impl StockFilter {
    pub const ALL: [StockFilter; 10] = [
        StockFilter::All,
        StockFilter::Stone,
        StockFilter::Wood,
        StockFilter::Food,
        StockFilter::Furniture,
        StockFilter::Equipment,
        StockFilter::Metal,
        StockFilter::Goods,
        StockFilter::Refuse,
        StockFilter::Medical,
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
            StockFilter::Medical => "medical supplies",
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
            StockFilter::Medical => 'h',
        }
    }

    pub fn accepts(self, k: ItemKind) -> bool {
        use ItemKind::*;
        match self {
            StockFilter::All => true,
            StockFilter::Stone => matches!(k, Boulder | Block),
            StockFilter::Wood => k == Log,
            StockFilter::Food => matches!(k, Plant | Drink | Seed | Meat | Meal | Fish | Flour),
            StockFilter::Furniture => matches!(k, Bed | Table | Chair | Door | Barrel | Bin | Cage | Coffin | Slab | Bucket | Cabinet | Chest | Statue),
            StockFilter::Metal => matches!(k, Bar | Charcoal),
            StockFilter::Goods => {
                matches!(
                    k,
                    Craft | Instrument | Gem | Bone | Leather | Mechanism | Thread | Splint | Crutch | Cloth | Shirt | Trousers | Shoes | Ash | Tallow | Product | Soap
                )
            }
            StockFilter::Refuse => matches!(k, Corpse | Carcass | Hide | BodyPart),
            StockFilter::Equipment => k.is_equipment(),
            StockFilter::Medical => matches!(k, Thread | Cloth | Splint | Crutch | Soap | Bucket),
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
    /// Days of wear on clothing; days of rot on a body.
    #[serde(default)]
    pub wear: u16,
    /// Left alone: nobody hauls, uses or eats it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub forbidden: bool,
    // Milestone 2 (wave 0), filled in by O03's crafts (P4):
    /// Foes it has killed (a weapon is named at five).
    #[serde(default, skip_serializing_if = "is_zero16")]
    pub kills: u16,
    /// Who made it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maker: Option<u32>,
    /// What it is encrusted or decorated with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decor: Option<Mat>,
}

fn is_zero16(n: &u16) -> bool {
    *n == 0
}

pub const ARTIFACT: u8 = 6;
/// Clothing this worn is due for replacing.
pub const WORN: u16 = 150;
/// Rags: an embarrassment to wear.
pub const TATTERED: u16 = 300;
/// Thrown away.
pub const FALLS_APART: u16 = 400;

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
        let worn = match self.wear {
            w if w >= TATTERED => "tattered ",
            w if w >= WORN => "worn ",
            _ => "",
        };
        format!("{}{}{}", worn, quality_prefix(self.quality), self.base_name())
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
            Plant => m.crop().map_or("stoutcap mushroom", |c| c.plant).into(),
            Drink => format!("barrel of {}", m.crop().and_then(|c| c.brew).unwrap_or("stoutcap ale")),
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
            Seed => m.crop().map_or("stoutcap spawn", |c| c.seed).into(),
            Meat if self.label.as_deref().map_or(false, |l| l.ends_with(" eggs")) => self.label.clone().unwrap_or_default(),
            Meat => format!("{} meat", self.label.as_deref().unwrap_or("animal")),
            Product => self.label.clone().unwrap_or_else(|| "animal product".into()),
            Meal => "prepared meal".into(),
            Bone => format!("{} bone", self.label.as_deref().unwrap_or("animal")),
            Leather => format!("{} leather", self.label.as_deref().unwrap_or("animal")),
            Hide => format!("{} hide", self.label.as_deref().unwrap_or("animal")),
            Cloth => format!("{} cloth", m.name()),
            Shirt => format!("{} shirt", m.name()),
            Trousers => format!("{} trousers", m.name()),
            Shoes => format!("{} shoes", m.name()),
            Ash => "wood ash".into(),
            Tallow => format!("{} tallow", self.label.as_deref().unwrap_or("animal")),
            Soap => format!("{} soap", self.label.as_deref().unwrap_or("tallow")),
            Fish => self.label.clone().unwrap_or_else(|| "fish".into()),
            Flour => format!("{} flour", m.name()),
            BodyPart => self.label.clone().unwrap_or_else(|| "severed limb".into()),
            Charcoal => "charcoal".into(),
            Bar => format!("{} bar", m.name()),
            Bin => format!("{} bin", m.adj()),
            Craft => format!("{} figurine", m.adj()),
            Gem => format!("cut {}", m.name()),
            Mechanism => format!("{} mechanism", m.adj()),
            Crossbow => format!("{} crossbow", m.adj()),
            Bolt => format!("{} bolts (10)", m.adj()),
            Thread => format!("{}-fiber thread", m.name()),
            Splint => format!("{} splint", m.adj()),
            Crutch => format!("{} crutch", m.adj()),
            Coffin => format!("{} coffin", m.adj()),
            Slab => format!("{} slab", m.adj()),
            Cabinet => format!("{} cabinet", m.adj()),
            Chest => format!("{} chest", m.adj()),
            Statue => format!("{} statue", m.adj()),
            Instrument => format!("{} {}", m.adj(), self.label.as_deref().unwrap_or("instrument")),
            Bucket if self.label.as_deref() == Some("water") => format!("{} bucket of water", m.adj()),
            Bucket => format!("{} bucket", m.adj()),
            Cage => match self.label.as_deref() {
                Some(beast) => format!("{} cage (holding a {})", m.adj(), beast),
                None => format!("{} cage", m.adj()),
            },
        }
    }

    /// Lying loose for anyone to take: not carried, claimed or forbidden.
    pub fn free(&self) -> bool {
        self.carried_by.is_none() && self.claimed_by.is_none() && !self.forbidden
    }

    /// Food a dwarf can eat as it is (grain has to be milled or brewed first).
    pub fn is_food(&self) -> bool {
        self.kind.is_food() && (self.kind != ItemKind::Plant || self.mat.crop().map_or(true, |c| c.edible))
    }

    pub fn value(&self) -> u32 {
        let v = self.kind.base_value() * self.mat.value() * quality_mult(self.quality);
        if self.wear >= WORN {
            v / 2
        } else {
            v
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
            Plant => '%',
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
            Thread => '§',
            Splint => '|',
            Crutch => '|',
            Coffin => 'Ω',
            Slab => '▬',
            Bucket => '¤',
            Hide => '~',
            Cloth => '≈',
            Shirt | Trousers => '(',
            Shoes => '{',
            Ash => '*',
            Tallow => '%',
            Soap => '=',
            Fish => 'α',
            Flour => '∙',
            BodyPart if self.mat == Mat::Bone => ',',
            BodyPart if self.label.as_deref().map_or(false, |l| l.ends_with("head")) => 'ö',
            BodyPart => '⌐',
            Cabinet => 'π',
            Chest => '■',
            Statue => '♀',
            Instrument => '♫',
            Product => '♦',
        };
        let col = match self.kind {
            Drink => Color::Rgb(230, 170, 60),
            Corpse => Color::Rgb(200, 60, 60),
            Carcass | Hide => Color::Rgb(190, 120, 90),
            Meat => Color::Rgb(220, 90, 90),
            Meal => Color::Rgb(250, 200, 110),
            Seed => Color::Rgb(160, 120, 170),
            Charcoal => Color::Rgb(90, 90, 90),
            Ash => Color::Rgb(170, 170, 170),
            Tallow => Color::Rgb(240, 230, 180),
            Soap => Color::Rgb(250, 250, 220),
            Fish => Color::Rgb(140, 190, 220),
            BodyPart if self.mat == Mat::Bone => Color::Rgb(235, 225, 200),
            BodyPart => Color::Rgb(200, 50, 50),
            Bin | Barrel => Mat::Wood.color(),
            _ => self.mat.color(),
        };
        (c, col)
    }
}
