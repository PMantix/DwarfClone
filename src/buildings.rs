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
    Mechanic,
    Lever,
    CageTrap,
    WeaponTrap,
    StoneTrap,
    Bridge,
    ArcheryTarget,
    Coffin,
    Slab,
    Well,
    Floodgate,
    MagmaSmelter,
    MagmaForge,
    Tannery,
    Leatherworks,
    Loom,
    Clothier,
    Soapmaker,
    Millstone,
    /// A megaproject: a great monument raised block by block.
    Monument,
    /// Bedroom furniture: a cabinet for clothes, a chest for belongings.
    Cabinet,
    Chest,
    /// A carved stone statue, for halls and temples.
    Statue,
    /// A low stone stage in a tavern, where bards perform.
    Stage,
}

/// Stone blocks a monument takes.
pub const MONUMENT_BLOCKS: u8 = 30;

impl BuildingKind {
    pub const ALL: [BuildingKind; 38] = [
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
        BuildingKind::Mechanic,
        BuildingKind::Lever,
        BuildingKind::CageTrap,
        BuildingKind::WeaponTrap,
        BuildingKind::StoneTrap,
        BuildingKind::Bridge,
        BuildingKind::ArcheryTarget,
        BuildingKind::Coffin,
        BuildingKind::Slab,
        BuildingKind::Well,
        BuildingKind::Floodgate,
        BuildingKind::MagmaSmelter,
        BuildingKind::MagmaForge,
        BuildingKind::Tannery,
        BuildingKind::Leatherworks,
        BuildingKind::Loom,
        BuildingKind::Clothier,
        BuildingKind::Soapmaker,
        BuildingKind::Millstone,
        BuildingKind::Monument,
        BuildingKind::Cabinet,
        BuildingKind::Chest,
        BuildingKind::Statue,
        BuildingKind::Stage,
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
            BuildingKind::Mechanic => "Mechanic's workshop",
            BuildingKind::Lever => "Lever",
            BuildingKind::CageTrap => "Cage trap",
            BuildingKind::WeaponTrap => "Weapon trap",
            BuildingKind::StoneTrap => "Stone-fall trap",
            BuildingKind::Bridge => "Drawbridge",
            BuildingKind::ArcheryTarget => "Archery target",
            BuildingKind::Coffin => "Coffin",
            BuildingKind::Slab => "Memorial slab",
            BuildingKind::Well => "Well",
            BuildingKind::Floodgate => "Floodgate",
            BuildingKind::MagmaSmelter => "Magma smelter",
            BuildingKind::MagmaForge => "Magma forge",
            BuildingKind::Tannery => "Tanner's shop",
            BuildingKind::Leatherworks => "Leather works",
            BuildingKind::Loom => "Loom",
            BuildingKind::Clothier => "Clothier's shop",
            BuildingKind::Soapmaker => "Soap maker's workshop",
            BuildingKind::Millstone => "Millstone",
            BuildingKind::Monument => "Great monument",
            BuildingKind::Cabinet => "Cabinet",
            BuildingKind::Chest => "Chest",
            BuildingKind::Statue => "Statue",
            BuildingKind::Stage => "Stage",
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
            BuildingKind::Mechanic => 'n',
            BuildingKind::Lever => 'l',
            BuildingKind::CageTrap => 'a',
            BuildingKind::WeaponTrap => 'v',
            BuildingKind::StoneTrap => 'o',
            BuildingKind::Bridge => 'i',
            BuildingKind::ArcheryTarget => 'z',
            BuildingKind::Coffin => 'y',
            BuildingKind::Slab => 'g',
            BuildingKind::Well => 'q',
            BuildingKind::Floodgate => 'j',
            BuildingKind::MagmaSmelter => 'E',
            BuildingKind::MagmaForge => 'F',
            BuildingKind::Tannery => 'T',
            BuildingKind::Leatherworks => 'L',
            BuildingKind::Loom => 'W',
            BuildingKind::Clothier => 'K',
            BuildingKind::Soapmaker => 'P',
            BuildingKind::Millstone => 'M',
            BuildingKind::Monument => 'G',
            BuildingKind::Cabinet => 'x',
            BuildingKind::Chest => 'H',
            BuildingKind::Statue => 'S',
            BuildingKind::Stage => 'A',
        }
    }

    /// Workshops heated by magma below instead of fuel.
    pub fn uses_magma(self) -> bool {
        matches!(self, BuildingKind::MagmaSmelter | BuildingKind::MagmaForge)
    }

    pub fn is_workshop(self) -> bool {
        !matches!(
            self,
            BuildingKind::Bed
                | BuildingKind::Table
                | BuildingKind::Chair
                | BuildingKind::Door
                | BuildingKind::Lever
                | BuildingKind::CageTrap
                | BuildingKind::WeaponTrap
                | BuildingKind::StoneTrap
                | BuildingKind::Bridge
                | BuildingKind::ArcheryTarget
                | BuildingKind::Coffin
                | BuildingKind::Slab
                | BuildingKind::Well
                | BuildingKind::Floodgate
                | BuildingKind::Monument
                | BuildingKind::Cabinet
                | BuildingKind::Chest
                | BuildingKind::Statue
                | BuildingKind::Stage
        )
    }

    pub fn is_trap(self) -> bool {
        matches!(self, BuildingKind::CageTrap | BuildingKind::WeaponTrap | BuildingKind::StoneTrap)
    }

    /// How many items go into building it, delivered one at a time.
    pub fn parts(self) -> u8 {
        match self {
            BuildingKind::CageTrap | BuildingKind::WeaponTrap | BuildingKind::StoneTrap | BuildingKind::Bridge | BuildingKind::Well | BuildingKind::Floodgate => 2,
            BuildingKind::Monument => MONUMENT_BLOCKS,
            _ => 1,
        }
    }

    /// Whether `item` fits part number `part` (0-based).
    pub fn needs_part(self, part: u8, item: &Item) -> bool {
        use BuildingKind::*;
        let mechanism = item.kind == ItemKind::Mechanism;
        match (self, part) {
            (Lever, _) | (CageTrap, 0) | (WeaponTrap, 0) | (StoneTrap, 0) | (Bridge, 1) | (Floodgate, 1) => mechanism,
            (CageTrap, _) => item.kind == ItemKind::Cage && item.label.is_none(),
            (WeaponTrap, _) => matches!(item.kind, ItemKind::Sword | ItemKind::Spear | ItemKind::Axe | ItemKind::Hammer),
            (StoneTrap, _) => item.kind == ItemKind::Boulder && item.mat.is_building_stone(),
            (Bridge, _) => matches!(item.kind, ItemKind::Log | ItemKind::Block),
            (Well, 0) => item.kind == ItemKind::Block || (item.kind == ItemKind::Boulder && item.mat.is_building_stone()),
            (Well, _) => item.kind == ItemKind::Bucket,
            (Floodgate, _) => item.kind == ItemKind::Block || (item.kind == ItemKind::Boulder && item.mat.is_building_stone()),
            (Monument, _) => item.kind == ItemKind::Block,
            _ => self.needs(item),
        }
    }

    pub fn part_desc(self, part: u8) -> &'static str {
        use BuildingKind::*;
        match (self, part) {
            (Lever, _) | (CageTrap, 0) | (WeaponTrap, 0) | (StoneTrap, 0) | (Bridge, 1) | (Floodgate, 1) => "a mechanism",
            (CageTrap, _) => "an empty cage",
            (WeaponTrap, _) => "a weapon",
            (StoneTrap, _) => "a stone boulder",
            (Bridge, _) => "a log or blocks",
            (Well, 0) => "stone blocks or a boulder",
            (Well, _) => "a bucket",
            (Floodgate, _) => "stone blocks or a boulder",
            (Monument, _) => "stone blocks",
            _ => self.needs_desc(),
        }
    }

    pub fn size(self) -> i32 {
        if self.is_workshop() || self == BuildingKind::Monument {
            3
        } else if self == BuildingKind::Stage {
            2
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
            BuildingKind::Coffin => item.kind == ItemKind::Coffin,
            BuildingKind::Slab => item.kind == ItemKind::Slab,
            BuildingKind::Cabinet => item.kind == ItemKind::Cabinet,
            BuildingKind::Chest => item.kind == ItemKind::Chest,
            BuildingKind::Statue => item.kind == ItemKind::Statue,
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
            BuildingKind::Coffin => "a coffin",
            BuildingKind::Slab => "a slab",
            BuildingKind::Cabinet => "a cabinet",
            BuildingKind::Chest => "a chest",
            BuildingKind::Statue => "a statue",
            BuildingKind::Lever => "a mechanism",
            BuildingKind::CageTrap => "a mechanism + an empty cage",
            BuildingKind::WeaponTrap => "a mechanism + a weapon",
            BuildingKind::StoneTrap => "a mechanism + a boulder",
            BuildingKind::Bridge => "a log or blocks + a mechanism",
            BuildingKind::Well => "blocks or a boulder + a bucket",
            BuildingKind::Floodgate => "blocks or a boulder + a mechanism",
            BuildingKind::Monument => "30 stone blocks",
            _ => "a stone boulder",
        }
    }

    pub fn recipes(self) -> &'static [Recipe] {
        use Recipe::*;
        match self {
            BuildingKind::Carpenter => &[WoodBed, WoodTable, WoodChair, WoodDoor, Barrel, Bin, WoodCage, WoodCrossbow, Bolts, Splint, Crutch, WoodCoffin, Bucket, WoodCabinet, WoodChest],
            BuildingKind::Mechanic => &[Mechanism],
            BuildingKind::Mason => &[Blocks, StoneTable, StoneChair, StoneDoor, StoneCoffin, Slab, Statue],
            BuildingKind::Still => &[Brew],
            BuildingKind::Kitchen => &[Cook],
            BuildingKind::Butcher => &[Butcher],
            BuildingKind::WoodFurnace => &[Charcoal, Ash],
            BuildingKind::Soapmaker => &[MakeSoap],
            BuildingKind::Millstone => &[Mill],
            BuildingKind::Smelter | BuildingKind::MagmaSmelter => &[SmeltOre, MakeSteel],
            BuildingKind::Forge | BuildingKind::MagmaForge => &[
                ForgePick,
                ForgeAxe,
                ForgeSword,
                ForgeSpear,
                ForgeHammer,
                ForgeHelm,
                ForgeBreastplate,
                ForgeShield,
                ForgeCrossbow,
                ForgeBolts,
            ],
            BuildingKind::Craftsdwarf => &[StoneCraft, BoneCraft, CutGem, SpinThread],
            BuildingKind::Tannery => &[Tan],
            BuildingKind::Leatherworks => &[LeatherArmor, LeatherHelm, LeatherCraft, LeatherShoes],
            BuildingKind::Loom => &[Weave],
            BuildingKind::Clothier => &[ClothShirt, ClothTrousers],
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
            BuildingKind::Mechanic => Color::Rgb(170, 190, 210),
            BuildingKind::Lever => Color::Rgb(220, 200, 120),
            BuildingKind::CageTrap | BuildingKind::WeaponTrap | BuildingKind::StoneTrap => Color::Rgb(230, 120, 120),
            BuildingKind::Well => Color::Rgb(110, 170, 255),
            BuildingKind::Floodgate => Color::Rgb(150, 190, 230),
            BuildingKind::MagmaSmelter => Color::Rgb(255, 120, 30),
            BuildingKind::MagmaForge => Color::Rgb(255, 70, 20),
            BuildingKind::Tannery => Color::Rgb(170, 120, 80),
            BuildingKind::Leatherworks => Color::Rgb(200, 150, 100),
            BuildingKind::Loom => Color::Rgb(200, 170, 220),
            BuildingKind::Clothier => Color::Rgb(220, 160, 200),
            BuildingKind::Soapmaker => Color::Rgb(240, 240, 200),
            BuildingKind::Millstone => Color::Rgb(210, 200, 170),
            BuildingKind::Monument => Color::Rgb(255, 215, 90),
            BuildingKind::Cabinet | BuildingKind::Chest => Color::Rgb(200, 160, 110),
            BuildingKind::Statue => Color::Rgb(235, 235, 245),
            BuildingKind::Stage => Color::Rgb(210, 170, 120),
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
            BuildingKind::Mechanic => 'N',
            BuildingKind::Lever => '¬',
            BuildingKind::CageTrap | BuildingKind::WeaponTrap | BuildingKind::StoneTrap => '^',
            BuildingKind::Bridge => '≡',
            BuildingKind::ArcheryTarget => '◎',
            BuildingKind::Coffin => 'Ω',
            BuildingKind::Slab => '▬',
            BuildingKind::Well => '○',
            BuildingKind::Floodgate => '╫',
            BuildingKind::MagmaSmelter => 'E',
            BuildingKind::MagmaForge => 'F',
            BuildingKind::Tannery => 'T',
            BuildingKind::Leatherworks => 'L',
            BuildingKind::Loom => 'W',
            BuildingKind::Clothier => 'K',
            BuildingKind::Soapmaker => 'P',
            BuildingKind::Millstone => 'M',
            BuildingKind::Monument => '▲',
            BuildingKind::Cabinet => 'π',
            BuildingKind::Chest => '■',
            BuildingKind::Statue => '♀',
            BuildingKind::Stage => '▒',
        }
    }
}

/// One ingredient slot of a recipe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    Log,
    Stone,
    Brewable,
    Fiber,
    Edible,
    Carcass,
    Ore,
    Fuel,
    Bar,
    IronBar,
    Flux,
    Bone,
    RoughGem,
    Hide,
    Leather,
    Thread,
    Cloth,
    Ash,
    Tallow,
    Grain,
}

impl Input {
    pub fn accepts(self, it: &Item) -> bool {
        match self {
            Input::Log => it.kind == ItemKind::Log,
            Input::Stone => it.kind == ItemKind::Boulder && it.mat.is_building_stone(),
            Input::Brewable => it.kind == ItemKind::Plant && it.mat.crop().map_or(true, |c| c.brew.is_some()),
            Input::Fiber => it.kind == ItemKind::Plant && it.mat.crop().map_or(true, |c| c.fiber),
            Input::Edible => (it.kind == ItemKind::Plant && it.is_food()) || matches!(it.kind, ItemKind::Meat | ItemKind::Fish | ItemKind::Flour),
            Input::Carcass => it.kind == ItemKind::Carcass,
            Input::Ore => it.kind == ItemKind::Boulder && it.mat.smelts_to().is_some(),
            Input::Fuel => it.kind == ItemKind::Charcoal || (it.kind == ItemKind::Boulder && it.mat == Mat::Coal),
            Input::Bar => it.kind == ItemKind::Bar,
            Input::IronBar => it.kind == ItemKind::Bar && it.mat == Mat::Iron,
            Input::Flux => it.kind == ItemKind::Boulder && it.mat == Mat::Limestone,
            Input::Bone => it.kind == ItemKind::Bone,
            Input::RoughGem => it.kind == ItemKind::Boulder && it.mat.is_gem(),
            Input::Hide => it.kind == ItemKind::Hide,
            Input::Leather => it.kind == ItemKind::Leather,
            Input::Thread => it.kind == ItemKind::Thread,
            Input::Cloth => it.kind == ItemKind::Cloth,
            Input::Ash => it.kind == ItemKind::Ash,
            Input::Tallow => it.kind == ItemKind::Tallow,
            Input::Grain => it.kind == ItemKind::Plant && it.mat.crop().map_or(false, |c| c.mill),
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Input::Log => "log",
            Input::Stone => "stone boulder",
            Input::Brewable => "brewable plant",
            Input::Fiber => "fiber plant",
            Input::Edible => "edible plant, meat, fish or flour",
            Input::Carcass => "carcass",
            Input::Ore => "metal ore",
            Input::Fuel => "fuel (charcoal or coal)",
            Input::Bar => "metal bar",
            Input::IronBar => "iron bar",
            Input::Flux => "limestone (flux)",
            Input::Bone => "bone",
            Input::RoughGem => "rough gem",
            Input::Hide => "hide",
            Input::Leather => "leather",
            Input::Thread => "thread",
            Input::Cloth => "cloth",
            Input::Ash => "ash",
            Input::Tallow => "tallow",
            Input::Grain => "grain (hollowgrain or goldoat)",
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
    Mechanism,
    WoodCage,
    WoodCrossbow,
    Bolts,
    Splint,
    Crutch,
    SpinThread,
    WoodCoffin,
    StoneCoffin,
    Slab,
    Bucket,
    Tan,
    LeatherArmor,
    LeatherHelm,
    LeatherCraft,
    LeatherShoes,
    Weave,
    ClothShirt,
    ClothTrousers,
    Ash,
    MakeSoap,
    Mill,
    ForgeCrossbow,
    ForgeBolts,
    WoodCabinet,
    WoodChest,
    Statue,
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
            Brew => "Brew a drink",
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
            Mechanism => "Make mechanism",
            WoodCage => "Make wooden cage",
            WoodCrossbow => "Make wooden crossbow",
            Bolts => "Make wooden bolts",
            Splint => "Make splint",
            Crutch => "Make crutch",
            SpinThread => "Spin thread",
            WoodCoffin => "Make wooden coffin",
            StoneCoffin => "Carve stone coffin",
            Slab => "Carve memorial slab",
            Bucket => "Make bucket",
            Tan => "Tan a hide",
            LeatherArmor => "Make leather armor",
            LeatherHelm => "Make leather cap",
            LeatherCraft => "Make leather crafts",
            LeatherShoes => "Make leather shoes",
            Weave => "Weave cloth",
            ClothShirt => "Sew a shirt",
            ClothTrousers => "Sew trousers",
            Ash => "Burn wood to ash",
            MakeSoap => "Make soap",
            Mill => "Mill grain into flour",
            ForgeCrossbow => "Forge crossbow",
            ForgeBolts => "Forge bolts",
            WoodCabinet => "Make wooden cabinet",
            WoodChest => "Make wooden chest",
            Statue => "Carve a statue",
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
            Mechanism => 'm',
            WoodCage => 'g',
            WoodCrossbow => 'x',
            Bolts => 'l',
            Splint => 's',
            Crutch => 'c',
            SpinThread => 't',
            WoodCoffin | StoneCoffin => 'o',
            Slab => 's',
            Bucket => 'u',
            Tan => 't',
            LeatherArmor => 'a',
            LeatherHelm => 'h',
            LeatherCraft => 'c',
            LeatherShoes => 'o',
            Weave => 'w',
            ClothShirt => 's',
            ClothTrousers => 't',
            Ash => 'a',
            MakeSoap => 's',
            Mill => 'm',
            ForgeCrossbow => 'x',
            ForgeBolts => 'l',
            WoodCabinet => 'i',
            WoodChest => 'e',
            Statue => 'a',
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
            Mechanism => Labor::Mechanics,
            WoodCage | WoodCrossbow | Bolts | Splint | Crutch => Labor::Carpentry,
            SpinThread => Labor::Crafting,
            WoodCoffin | Bucket => Labor::Carpentry,
            StoneCoffin | Slab => Labor::Masonry,
            Tan | LeatherArmor | LeatherHelm | LeatherCraft | LeatherShoes => Labor::Leatherworking,
            Weave | ClothShirt | ClothTrousers => Labor::Clothesmaking,
            Ash => Labor::Smelting,
            MakeSoap => Labor::Soapmaking,
            Mill => Labor::Farming,
            ForgeCrossbow | ForgeBolts => Labor::Smithing,
            WoodCabinet | WoodChest => Labor::Carpentry,
            Statue => Labor::Masonry,
        }
    }

    pub fn inputs(self) -> &'static [Input] {
        use Recipe::*;
        match self {
            WoodBed | WoodTable | WoodChair | WoodDoor | Barrel | Bin | Charcoal | WoodCage | WoodCrossbow | Bolts | Splint | Crutch | WoodCoffin | Bucket | WoodCabinet | WoodChest => {
                &[Input::Log]
            }
            SpinThread => &[Input::Fiber],
            Blocks | StoneTable | StoneChair | StoneDoor | StoneCraft | Mechanism | StoneCoffin | Slab | Statue => &[Input::Stone],
            Brew => &[Input::Brewable],
            Cook => &[Input::Edible],
            Butcher => &[Input::Carcass],
            SmeltOre => &[Input::Ore, Input::Fuel],
            MakeSteel => &[Input::IronBar, Input::Fuel, Input::Flux],
            ForgeBreastplate => &[Input::Bar, Input::Bar, Input::Fuel],
            ForgePick | ForgeAxe | ForgeSword | ForgeSpear | ForgeHammer | ForgeHelm | ForgeShield | ForgeCrossbow | ForgeBolts => {
                &[Input::Bar, Input::Fuel]
            }
            BoneCraft => &[Input::Bone],
            CutGem => &[Input::RoughGem],
            Tan => &[Input::Hide],
            LeatherArmor => &[Input::Leather, Input::Leather],
            LeatherHelm | LeatherCraft | LeatherShoes => &[Input::Leather],
            Weave => &[Input::Thread, Input::Thread],
            ClothShirt | ClothTrousers => &[Input::Cloth],
            Ash => &[Input::Log],
            MakeSoap => &[Input::Ash, Input::Tallow],
            Mill => &[Input::Grain],
        }
    }

    /// Inputs at a given workshop: magma workshops need no fuel.
    pub fn inputs_at(self, shop: BuildingKind) -> Vec<Input> {
        self.inputs().iter().copied().filter(|&i| !(shop.uses_magma() && i == Input::Fuel)).collect()
    }

    pub fn input_desc_at(self, shop: BuildingKind) -> String {
        self.inputs_at(shop).iter().map(|i| i.desc()).collect::<Vec<_>>().join(" + ")
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
                // Brewing leaves the plant's seed behind.
                let mut v = vec![(ItemKind::Drink, m, None); 4];
                v.push((ItemKind::Seed, m, None));
                v
            }
            Cook => {
                // Flour stretches further: bread and dumplings.
                let flour = inputs.first().map_or(false, |i| i.kind == ItemKind::Flour);
                let mut v = vec![(ItemKind::Meal, m, None); if flour { 3 } else { 2 }];
                // Cooking a stoutcap leaves its spawn, like eating one does.
                if inputs.first().map_or(false, |i| i.kind == ItemKind::Plant) {
                    v.push((ItemKind::Seed, m, None));
                }
                v
            }
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
                v.push((ItemKind::Hide, Mat::Leather, label.clone()));
                v.push((ItemKind::Tallow, Mat::Flesh, label));
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
            Mechanism => one(ItemKind::Mechanism),
            WoodCage => one(ItemKind::Cage),
            WoodCrossbow => one(ItemKind::Crossbow),
            Bolts => vec![(ItemKind::Bolt, m, None); 2],
            Splint => one(ItemKind::Splint),
            Crutch => one(ItemKind::Crutch),
            SpinThread => vec![(ItemKind::Thread, m, None), (ItemKind::Thread, m, None), (ItemKind::Seed, m, None)],
            WoodCoffin | StoneCoffin => one(ItemKind::Coffin),
            Slab => one(ItemKind::Slab),
            Bucket => one(ItemKind::Bucket),
            Tan => vec![(ItemKind::Leather, Mat::Leather, inputs.first().and_then(|i| i.label.clone()))],
            LeatherArmor => vec![(ItemKind::Breastplate, Mat::Leather, None)],
            LeatherHelm => vec![(ItemKind::Helm, Mat::Leather, None)],
            LeatherCraft => vec![(ItemKind::Craft, Mat::Leather, None)],
            LeatherShoes => vec![(ItemKind::Shoes, Mat::Leather, None)],
            Weave => vec![(ItemKind::Cloth, m, None)],
            ClothShirt => one(ItemKind::Shirt),
            ClothTrousers => one(ItemKind::Trousers),
            Ash => vec![(ItemKind::Ash, Mat::Wood, None)],
            Mill => vec![(ItemKind::Flour, m, None), (ItemKind::Flour, m, None), (ItemKind::Seed, m, None)],
            ForgeCrossbow => one(ItemKind::Crossbow),
            ForgeBolts => vec![(ItemKind::Bolt, m, None); 3],
            WoodCabinet => one(ItemKind::Cabinet),
            WoodChest => one(ItemKind::Chest),
            Statue => one(ItemKind::Statue),
            MakeSoap => {
                let fat = inputs.get(1).and_then(|i| i.label.clone());
                vec![(ItemKind::Soap, Mat::Flesh, fat.clone()), (ItemKind::Soap, Mat::Flesh, fat)]
            }
        }
    }

    pub fn work(self) -> u32 {
        use Recipe::*;
        match self {
            Blocks | Charcoal | Tan | Ash => 90,
            Weave => 120,
            Brew | Cook => 110,
            Butcher => 120,
            SmeltOre | MakeSteel => 160,
            ForgeBreastplate => 260,
            ForgePick | ForgeAxe | ForgeSword | ForgeSpear | ForgeHammer | ForgeHelm | ForgeShield | ForgeCrossbow => 200,
            ForgeBolts => 180,
            CutGem | Statue => 180,
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
    /// Parts delivered so far (traps and bridges take more than one).
    #[serde(default)]
    pub parts: u8,
    /// What a trap is loaded with: (kind, material, quality) of its last part.
    #[serde(default)]
    pub load: Option<(ItemKind, Mat, u8)>,
    /// A trap can't fire again before this tick.
    #[serde(default)]
    pub trap_cd: u64,
    /// Doors and bridges a lever works.
    #[serde(default)]
    pub links: Vec<u32>,
    /// Someone has been asked to pull this lever.
    #[serde(default)]
    pub pull: bool,
    /// A drawbridge is raised (impassable).
    #[serde(default)]
    pub raised: bool,
    /// Who may work at this workshop (empty: anyone with the labor).
    #[serde(default)]
    pub workers: Vec<u32>,
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
        if self.kind == BuildingKind::Monument {
            // Scaffolding while it rises, then an obelisk on its plinth.
            return if !self.built {
                if (dx + dy) % 2 == 0 { '#' } else { '░' }
            } else {
                match (dx, dy) {
                    (1, 1) => '▲',
                    (1, 0) | (1, 2) | (0, 1) | (2, 1) => '▓',
                    _ => '▄',
                }
            };
        }
        if self.kind == BuildingKind::Stage {
            return if self.built { '▒' } else { '░' };
        }
        if self.kind == BuildingKind::Bridge && self.raised {
            return '▀';
        }
        if self.kind == BuildingKind::Floodgate && self.built && !self.locked {
            return '¦';
        }
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
