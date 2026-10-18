//! The wider world: biomes, civilizations, sites, historical figures and
//! a couple of centuries of generated history.

use crate::rng::Rng;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};


#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Biome {
    Ocean,
    Mountain,
    Hills,
    Forest,
    Grassland,
    Desert,
    Tundra,
    Swamp,
    Jungle,
    Taiga,
    Savanna,
    Badlands,
    Glacier,
    /// Dry scrub on free-draining ground.
    Shrubland,
    /// Grassy wetland (salt marsh by the sea).
    Marsh,
    /// Fresh (or, in dry basins, salt) standing water inland.
    Lake,
}

/// A world cell near the site, as the embark map sees it.
#[derive(Clone, Copy, Debug)]
pub struct Around {
    pub x: i32,
    pub y: i32,
    pub biome: Biome,
    pub elev: u8,
    pub evil: bool,
    /// Its river links (1 north, 2 east, 4 south, 8 west), 0 for none.
    pub river: u8,
}

/// How a biome shapes the embark region.
#[derive(Clone, Debug)]
pub struct EmbarkParams {
    /// Multiplier on hill height.
    pub relief: f64,
    /// Multiplier on tree density.
    pub forest: f64,
    /// Fraction of the map that lies under water.
    pub lake: f64,
    /// Chance of hostile animals among wildlife.
    pub savagery: f64,
    /// Multiplier on ore veins and gems.
    pub minerals: f64,
    /// Fortress map dimensions.
    pub size: crate::world::MapSize,
    /// 0 none, 1 light, 2 heavy: water-bearing rock a few levels down.
    pub aquifer: u8,
    /// The site's features (`terrain::feat` bits).
    pub features: u32,
    /// The site's own biome.
    pub biome: Biome,
    /// Its world cell, and the cells round it (empty: just the one biome
    /// everywhere, as before).
    pub home: (i32, i32),
    pub around: Vec<Around>,
    /// The site cell's river links on the world map (1 north, 2 east,
    /// 4 south, 8 west), so the river on the fortress map runs the same way.
    pub river: u8,
    /// The region tiles under the map and round it (none for a world made
    /// before the fields were kept, or a map made by hand): the land is
    /// then built from them.
    pub region: Option<std::sync::Arc<crate::worldgen::tiles::RegionTiles>>,
}

impl Biome {
    pub const ALL: [Biome; 16] = [
        Biome::Ocean,
        Biome::Mountain,
        Biome::Hills,
        Biome::Forest,
        Biome::Grassland,
        Biome::Desert,
        Biome::Tundra,
        Biome::Swamp,
        Biome::Jungle,
        Biome::Taiga,
        Biome::Savanna,
        Biome::Badlands,
        Biome::Glacier,
        Biome::Shrubland,
        Biome::Marsh,
        Biome::Lake,
    ];

    /// 1 + its place in `ALL` (0 means unknown).
    pub fn code(self) -> u8 {
        Biome::ALL.iter().position(|&b| b == self).unwrap_or(0) as u8 + 1
    }

    pub fn from_code(c: u8) -> Option<Biome> {
        match (c & 0x7f) as usize {
            0 => None,
            i => Biome::ALL.get(i - 1).copied(),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Biome::Ocean => "ocean",
            Biome::Mountain => "mountains",
            Biome::Hills => "hills",
            Biome::Forest => "forest",
            Biome::Grassland => "grassland",
            Biome::Desert => "desert",
            Biome::Tundra => "tundra",
            Biome::Swamp => "swamp",
            Biome::Jungle => "jungle",
            Biome::Taiga => "taiga",
            Biome::Savanna => "savanna",
            Biome::Badlands => "badlands",
            Biome::Glacier => "glacier",
            Biome::Shrubland => "shrubland",
            Biome::Marsh => "marsh",
            Biome::Lake => "lake",
        }
    }

    /// The sea or a lake: no land to stand on.
    pub fn is_water(self) -> bool {
        matches!(self, Biome::Ocean | Biome::Lake)
    }

    /// Whether winter (or the whole year) is cold enough for ice and snow.
    pub fn cold(self) -> bool {
        matches!(self, Biome::Tundra | Biome::Mountain | Biome::Taiga | Biome::Glacier)
    }

    /// The biome's typical mark on the world map (the map itself varies it).
    pub fn glyph(self) -> (char, Color) {
        match self {
            Biome::Ocean => ('≈', Color::Rgb(40, 90, 200)),
            Biome::Mountain => ('▲', Color::Rgb(170, 170, 180)),
            Biome::Hills => ('∩', Color::Rgb(80, 170, 70)),
            Biome::Forest => ('♣', Color::Rgb(40, 160, 60)),
            Biome::Grassland => ('"', Color::Rgb(110, 200, 85)),
            Biome::Desert => ('~', Color::Rgb(230, 205, 120)),
            Biome::Tundra => ('"', Color::Rgb(190, 210, 215)),
            Biome::Swamp => ('%', Color::Rgb(70, 150, 125)),
            Biome::Jungle => ('♠', Color::Rgb(30, 200, 90)),
            Biome::Taiga => ('↑', Color::Rgb(60, 140, 110)),
            Biome::Savanna => ('"', Color::Rgb(205, 195, 90)),
            Biome::Badlands => ('∩', Color::Rgb(205, 115, 65)),
            Biome::Glacier => ('░', Color::Rgb(210, 235, 255)),
            Biome::Shrubland => ('τ', Color::Rgb(170, 160, 95)),
            Biome::Marsh => ('"', Color::Rgb(90, 165, 150)),
            Biome::Lake => ('≈', Color::Rgb(70, 140, 230)),
        }
    }

    pub fn params(self) -> EmbarkParams {
        let p = |relief, forest, lake, savagery| EmbarkParams {
            relief,
            forest,
            lake,
            savagery,
            minerals: 1.0,
            size: crate::world::MapSize::MEDIUM,
            aquifer: 0,
            features: 0,
            biome: Biome::Grassland,
            home: (0, 0),
            around: Vec::new(),
            river: 0,
            region: None,
        };
        let mut e = match self {
            Biome::Mountain => p(1.6, 0.3, 0.03, 0.3),
            Biome::Hills => p(1.2, 0.7, 0.06, 0.2),
            Biome::Forest => p(0.8, 1.7, 0.08, 0.25),
            Biome::Grassland => p(0.5, 0.25, 0.08, 0.15),
            Biome::Desert => p(0.6, 0.05, 0.0, 0.2),
            Biome::Tundra => p(0.6, 0.2, 0.05, 0.3),
            Biome::Swamp => p(0.3, 0.9, 0.25, 0.3),
            Biome::Ocean => p(0.4, 0.2, 0.5, 0.2),
            Biome::Jungle => p(0.7, 2.0, 0.1, 0.4),
            Biome::Taiga => p(0.7, 1.4, 0.06, 0.3),
            Biome::Savanna => p(0.4, 0.4, 0.04, 0.3),
            Biome::Badlands => p(1.4, 0.03, 0.0, 0.25),
            Biome::Glacier => p(0.8, 0.0, 0.02, 0.2),
            Biome::Shrubland => p(0.6, 0.35, 0.03, 0.25),
            Biome::Marsh => p(0.25, 0.3, 0.3, 0.25),
            Biome::Lake => p(0.4, 0.2, 0.5, 0.2),
        };
        e.biome = self;
        e
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Race {
    Dwarf,
    Human,
    Elf,
    Goblin,
}

impl Race {
    pub fn plural(self) -> &'static str {
        match self {
            Race::Dwarf => "dwarves",
            Race::Human => "humans",
            Race::Elf => "elves",
            Race::Goblin => "goblins",
        }
    }

    pub fn color(self) -> Color {
        match self {
            Race::Dwarf => Color::Rgb(255, 210, 90),
            Race::Human => Color::Rgb(240, 240, 240),
            Race::Elf => Color::Rgb(120, 255, 160),
            Race::Goblin => Color::Rgb(230, 70, 70),
        }
    }

    fn home(self, b: Biome, evil: bool) -> bool {
        match self {
            Race::Dwarf => matches!(b, Biome::Mountain | Biome::Hills | Biome::Badlands),
            Race::Human => matches!(b, Biome::Grassland | Biome::Forest | Biome::Hills | Biome::Savanna | Biome::Shrubland),
            Race::Elf => matches!(b, Biome::Forest | Biome::Jungle | Biome::Taiga) && !evil,
            Race::Goblin => evil && !b.is_water() || matches!(b, Biome::Tundra | Biome::Mountain | Biome::Glacier),
        }
    }

    /// Typical lifespan in years.
    fn lifespan(self) -> i32 {
        match self {
            Race::Dwarf => 150,
            Race::Human => 70,
            Race::Elf => 400,
            Race::Goblin => 60,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SiteKind {
    Fortress,
    Town,
    Hamlet,
    Retreat,
    DarkFortress,
    Lair,
}

impl SiteKind {
    pub fn name(self) -> &'static str {
        match self {
            SiteKind::Fortress => "fortress",
            SiteKind::Town => "town",
            SiteKind::Hamlet => "hamlet",
            SiteKind::Retreat => "forest retreat",
            SiteKind::DarkFortress => "dark fortress",
            SiteKind::Lair => "lair",
        }
    }

    pub fn glyph(self) -> char {
        match self {
            SiteKind::Fortress => 'Ω',
            SiteKind::Town => '#',
            SiteKind::Hamlet => '+',
            SiteKind::Retreat => '♠',
            SiteKind::DarkFortress => 'Ψ',
            SiteKind::Lair => '&',
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cell {
    pub biome: Biome,
    #[serde(default, skip_serializing_if = "is_false")]
    pub evil: bool,
    /// Site id + 1, or 0.
    #[serde(default, skip_serializing_if = "is_zero32")]
    pub site: u32,
    /// Region id + 1, or 0 for a patch too small to name.
    #[serde(default)]
    pub region: u32,
    /// Height, 0..=255, for shading the map.
    #[serde(default)]
    pub elev: u8,
    /// Where a river runs: 1 north, 2 east, 4 south, 8 west.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub river: u8,
    #[serde(default, skip_serializing_if = "is_false")]
    pub volcano: bool,
    /// Where a brook runs (too small for the world map), the same way.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub brook: u8,
    /// Features deep under the cell: 1 a cave river.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub deep: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegionKind {
    Ocean,
    Sea,
    Lake,
    Mountains,
    Hills,
    Forest,
    Plains,
    Desert,
    Tundra,
    Swamp,
    Jungle,
    Taiga,
    Savanna,
    Badlands,
    Glacier,
}

impl RegionKind {
    pub fn is_water(self) -> bool {
        matches!(self, RegionKind::Ocean | RegionKind::Sea | RegionKind::Lake)
    }

    pub fn name(self) -> &'static str {
        match self {
            RegionKind::Ocean => "ocean",
            RegionKind::Sea => "sea",
            RegionKind::Lake => "lake",
            RegionKind::Mountains => "mountain range",
            RegionKind::Hills => "hills",
            RegionKind::Forest => "forest",
            RegionKind::Plains => "plains",
            RegionKind::Desert => "desert",
            RegionKind::Tundra => "tundra",
            RegionKind::Swamp => "swamp",
            RegionKind::Jungle => "jungle",
            RegionKind::Taiga => "taiga",
            RegionKind::Savanna => "savanna",
            RegionKind::Badlands => "badlands",
            RegionKind::Glacier => "glacier",
        }
    }
}

/// Where a river ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mouth {
    Sea,
    Lake,
    /// It joins another river (a tributary).
    River,
}

/// A named river: a main stem or a tributary.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct River {
    pub id: u32,
    pub name: String,
    /// Its cells, spring first.
    pub course: Vec<(i32, i32)>,
    /// The cell it runs into (the sea, a lake, or the river it joins).
    pub into: (i32, i32),
    pub mouth: Mouth,
    /// The river it feeds, for a tributary.
    pub joins: Option<u32>,
    /// How much water it carries at its mouth (summed rain upstream).
    pub flow: u32,
}

/// A named stretch of land or water: a sea, a mountain range, a forest...
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Region {
    pub id: u32,
    pub kind: RegionKind,
    pub name: String,
    /// Number of world cells.
    pub size: u32,
    /// One of its cells, for "show me where".
    pub x: i32,
    pub y: i32,
    /// Its biomes and how many cells of each, the commonest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub biomes: Vec<(Biome, u32)>,
    /// The wild animals that live there, the most widespread first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fauna: Vec<crate::creature::Species>,
    /// Its commonest plants.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plants: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Civ {
    pub id: u32,
    pub name: String,
    pub race: Race,
    pub alive: bool,
    pub enemies: Vec<u32>,
    pub ruler: Option<u32>,
    #[serde(default)]
    pub capital: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Site {
    pub id: u32,
    pub name: String,
    pub kind: SiteKind,
    pub civ: Option<u32>,
    pub x: i32,
    pub y: i32,
    pub pop: u32,
    pub ruined: bool,
    pub founded: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Figure {
    pub id: u32,
    pub name: String,
    /// "dwarf", "dragon", ...
    pub kind: String,
    pub civ: Option<u32>,
    pub born: i32,
    pub died: Option<i32>,
    pub cause: Option<String>,
    pub title: Option<String>,
    pub kills: u32,
    /// Lair site for a megabeast.
    pub lair: Option<u32>,
}

impl Figure {
    pub fn alive(&self) -> bool {
        self.died.is_none()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub year: i32,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Neighbor {
    pub civ: u32,
    pub race: Race,
    pub dist: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Overworld {
    pub w: i32,
    pub h: i32,
    pub cells: Vec<Cell>,
    pub civs: Vec<Civ>,
    pub sites: Vec<Site>,
    pub figures: Vec<Figure>,
    pub events: Vec<Event>,
    pub year: i32,
    pub name: String,
    /// Wildlife savagery multiplier chosen at world creation.
    #[serde(default = "one")]
    pub savagery: f64,
    /// Ore and gem abundance multiplier chosen at world creation.
    #[serde(default = "one")]
    pub minerals: f64,
    #[serde(default)]
    pub regions: Vec<Region>,
    /// The world seed (for the region tiles under an embark; 0 in old saves).
    #[serde(default)]
    pub seed: u64,
    /// The fields the land was made from (empty in old saves).
    #[serde(default, skip_serializing_if = "crate::worldgen::fields::Fields::is_empty")]
    pub fields: crate::worldgen::fields::Fields,
    /// The named rivers (empty in old saves).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rivers: Vec<River>,
    /// Battles and beasts' raids as history makes them, kept only while a
    /// world is made to be watched (`None` otherwise; never saved).
    #[serde(skip)]
    pub clashes: Option<Vec<Clash>>,
    /// The gods the peoples worship (empty in old saves; see `worldgen::myth`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gods: Vec<crate::worldgen::myth::God>,
}

/// How a clash ended for the place attacked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClashEnd {
    /// Attacked and still standing.
    Held,
    /// Razed or laid waste.
    Fell,
    /// Conquered and held by the attacker.
    Taken,
    /// The attackers were beaten back.
    Repulsed,
    /// A hero slew the beast.
    Slain,
}

/// One battle or raid: from the attacker's site (a beast's lair) to the
/// site it struck.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clash {
    pub year: i32,
    pub from: u32,
    pub to: u32,
    pub end: ClashEnd,
}

fn is_zero(v: &u8) -> bool {
    *v == 0
}

fn is_zero32(v: &u32) -> bool {
    *v == 0
}

fn is_false(v: &bool) -> bool {
    !*v
}

fn one() -> f64 {
    1.0
}

/// Settings chosen on the world creation screen.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldParams {
    pub seed: u64,
    /// Years of history before the expedition sets out.
    pub history: i32,
    /// 0 few, 1 normal, 2 many civilizations.
    pub civs: u8,
    /// 0 calm, 1 normal, 2 savage wildlife.
    pub savagery: u8,
    /// 0 sparse, 1 normal, 2 rich ore and gems.
    pub minerals: u8,
    /// 0 small (128×56), 1 medium (192×80), 2 large (288×120).
    #[serde(default)]
    pub size: u8,
}

impl Default for WorldParams {
    fn default() -> Self {
        WorldParams { seed: 0, history: 200, civs: 1, savagery: 1, minerals: 1, size: 0 }
    }
}

impl WorldParams {
    pub fn civ_plan(&self) -> Vec<Race> {
        use Race::*;
        let one: Vec<Race> = match self.civs {
            0 => vec![Dwarf, Human, Elf, Goblin],
            1 => vec![Dwarf, Dwarf, Human, Human, Elf, Goblin, Goblin],
            _ => vec![Dwarf, Dwarf, Dwarf, Human, Human, Human, Elf, Elf, Goblin, Goblin, Goblin],
        };
        // Bigger worlds hold more peoples.
        one.iter().copied().cycle().take(one.len() * self.scale()).collect()
    }

    pub fn dims(&self) -> (i32, i32) {
        // One character a cell, so a cell is twice as tall as it is wide.
        [(128, 56), (192, 80), (288, 120)][self.size.min(2) as usize]
    }

    /// How many times more land than a small world, roughly (for civs and beasts).
    pub fn scale(&self) -> usize {
        [2, 3, 6][self.size.min(2) as usize]
    }

    pub fn savagery_mult(&self) -> f64 {
        [0.5, 1.0, 1.7][self.savagery.min(2) as usize]
    }

    pub fn minerals_mult(&self) -> f64 {
        [0.5, 1.0, 2.0][self.minerals.min(2) as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum GenStage {
    Terrain,
    Civs,
    Beasts,
    History,
    Done,
}

/// World generation that can be advanced a little at a time, so the
/// creation screen can draw the world as it takes shape.
pub struct WorldGen {
    pub params: WorldParams,
    pub ow: Overworld,
    rng: Rng,
    /// The land, made all at once on the first step and then shown a few
    /// rows at a time.
    land: Option<crate::worldgen::Land>,
    row: i32,
    stage: GenStage,
    /// Each river's course, spring first (for checking they run downhill).
    pub rivers: Vec<Vec<(i32, i32)>>,
    /// What generation did, kept to be played back (`WorldGen::traced`).
    pub trace: Option<GenTrace>,
}

/// A site's standing at some moment of history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SiteState {
    pub civ: Option<u32>,
    pub kind: SiteKind,
    pub ruined: bool,
}

/// The record of a world's making, for watching it: the land's trace, each
/// site as it was founded, every later change to a site with its year, and
/// the clashes of history. Recording draws no random numbers.
#[derive(Clone, Debug, Default)]
pub struct GenTrace {
    pub land: Option<crate::worldgen::Trace>,
    /// Per site (by id): as founded.
    pub first: Vec<SiteState>,
    /// (year, site, its new state).
    pub changes: Vec<(i32, u32, SiteState)>,
    pub clashes: Vec<Clash>,
    last: Vec<SiteState>,
}

impl WorldGen {
    pub fn new(params: WorldParams) -> WorldGen {
        let seed = params.seed;
        let mut rng = Rng::new(seed ^ 0xBADC0DE);
        let (w, h) = params.dims();
        // The land has random streams of its own, so naming the world first
        // draws the same random numbers as naming it afterwards.
        let name = format!("The {} {}", word(&mut rng, Race::Elf, 2), rng.pick(&["Continent", "Expanse", "Reaches", "Wilds"]));
        let ow = Overworld {
            w,
            h,
            cells: Vec::with_capacity((w * h) as usize),
            civs: Vec::new(),
            sites: Vec::new(),
            figures: Vec::new(),
            events: Vec::new(),
            year: 0,
            name,
            savagery: params.savagery_mult(),
            minerals: params.minerals_mult(),
            regions: Vec::new(),
            seed,
            fields: Default::default(),
            rivers: Vec::new(),
            clashes: None,
            gods: Vec::new(),
        };
        WorldGen { params, ow, rng, land: None, row: 0, stage: GenStage::Terrain, rivers: Vec::new(), trace: None }
    }

    /// A generator that also keeps a `GenTrace` of what it does, so the
    /// making can be played back. The world comes out the same.
    pub fn traced(params: WorldParams) -> WorldGen {
        let mut g = WorldGen::new(params);
        g.trace = Some(GenTrace::default());
        g.ow.clashes = Some(Vec::new());
        g
    }

    /// Whether the land (fields, rivers, volcanoes, regions) is all made.
    pub fn land_ready(&self) -> bool {
        self.stage != GenStage::Terrain
    }

    /// Whether the civilisations and beasts are placed and history has begun.
    pub fn history_ready(&self) -> bool {
        matches!(self.stage, GenStage::History | GenStage::Done)
    }

    /// Note sites new since the last look as they were founded, and any
    /// site whose holder, kind or ruin changed in `year`.
    fn note_sites(&mut self, year: i32) {
        let Some(t) = self.trace.as_mut() else { return };
        for s in &self.ow.sites[t.first.len()..] {
            t.first.push(SiteState { civ: s.civ, kind: s.kind, ruined: s.ruined });
            t.last.push(SiteState { civ: s.civ, kind: s.kind, ruined: s.ruined });
        }
        for (k, s) in self.ow.sites.iter().enumerate() {
            let now = SiteState { civ: s.civ, kind: s.kind, ruined: s.ruined };
            if now != t.last[k] {
                t.changes.push((year, k as u32, now));
                t.last[k] = now;
            }
        }
        if let Some(c) = self.ow.clashes.as_mut() {
            t.clashes.append(c);
        }
    }

    pub fn done(&self) -> bool {
        self.stage == GenStage::Done
    }

    /// Advance by up to `rows` rows of terrain or `years` years of history.
    /// Returns true once the world is finished.
    pub fn step(&mut self, rows: usize, years: usize) -> bool {
        match self.stage {
            GenStage::Terrain => {
                for _ in 0..rows {
                    if self.row >= self.ow.h {
                        break;
                    }
                    let y = self.row;
                    self.terrain_row(y);
                    self.row += 1;
                }
                if self.row >= self.ow.h {
                    self.rivers_and_fire();
                    self.name_regions();
                    self.cave_rivers();
                    self.stage = GenStage::Civs;
                }
            }
            GenStage::Civs => {
                self.found_civs();
                self.note_sites(0);
                self.stage = GenStage::Beasts;
            }
            GenStage::Beasts => {
                self.place_beasts();
                self.note_sites(0);
                self.stage = GenStage::History;
            }
            GenStage::History => {
                for _ in 0..years {
                    if self.ow.year >= self.params.history {
                        break;
                    }
                    let year = self.ow.year + 1;
                    self.ow.simulate_year(&mut self.rng, year);
                    self.ow.year = year;
                    self.note_sites(year);
                }
                if self.ow.year >= self.params.history {
                    self.ow.gods = crate::worldgen::myth::pantheon(&self.ow);
                    self.stage = GenStage::Done;
                }
            }
            GenStage::Done => {}
        }
        self.done()
    }

    /// Stage name and fraction done (terrain counts for the first 15%).
    pub fn progress(&self) -> (String, f32) {
        match self.stage {
            GenStage::Terrain => ("Raising mountains and filling the seas".into(), 0.1 * self.row as f32 / self.ow.h as f32),
            GenStage::Civs => ("Founding the first civilizations".into(), 0.1),
            GenStage::Beasts => ("Waking ancient beasts".into(), 0.12),
            GenStage::History => (
                format!("Recording history: year {} of {}", self.ow.year, self.params.history),
                0.15 + 0.85 * self.ow.year as f32 / self.params.history.max(1) as f32,
            ),
            GenStage::Done => ("History is written".into(), 1.0),
        }
    }

    pub fn finish(mut self) -> Overworld {
        while !self.step(usize::MAX, usize::MAX) {}
        self.ow.clashes = None;
        self.ow
    }

    fn terrain_row(&mut self, y: i32) {
        let (w, h) = (self.ow.w, self.ow.h);
        let traced = self.trace.is_some();
        let land = self.land.get_or_insert_with(|| crate::worldgen::build_traced(self.params.seed, w, h, traced));
        for x in 0..w {
            let i = (y * w + x) as usize;
            let elev = land.grids.elev[i].round().clamp(0.0, 255.0) as u8;
            self.ow.cells.push(Cell { biome: land.biomes[i], evil: land.evil[i], site: 0, region: 0, elev, river: 0, volcano: false, brook: 0, deep: 0 });
        }
    }

    /// The rivers and brooks the land was made with, named, and a few
    /// mountains that smoulder; the fields kept with the world. Its own
    /// random stream, so the history that follows is the same with or
    /// without them.
    fn rivers_and_fire(&mut self) {
        let (w, h) = (self.ow.w, self.ow.h);
        let mut rng = Rng::new(self.params.seed ^ 0x51FE5);
        let Some(mut land) = self.land.take() else { return };
        if let Some(t) = self.trace.as_mut() {
            t.land = land.trace.take();
        }
        self.ow.fields = crate::worldgen::fields::Fields::from_grids(&land.grids, &land.flow);
        for i in 0..(w * h) as usize {
            self.ow.cells[i].river = land.river[i];
            self.ow.cells[i].brook = land.brook[i];
        }
        let at = |i: usize| (i as i32 % w, i as i32 / w);
        let mut names: Vec<String> = Vec::new();
        for (k, st) in land.stems.iter().enumerate() {
            let course: Vec<(i32, i32)> = st.course.iter().map(|&i| at(i)).collect();
            let into = at(st.into.max(0) as usize);
            let mouth = if st.joins.is_some() {
                Mouth::River
            } else if land.biomes[st.into.max(0) as usize] == Biome::Lake {
                Mouth::Lake
            } else {
                Mouth::Sea
            };
            let flow = land.flow[*st.course.last().unwrap()].round() as u32;
            let mut name = river_name(&mut rng, flow);
            for attempt in 0..12 {
                if !names.contains(&name) {
                    break;
                }
                name = if attempt < 11 { river_name(&mut rng, flow) } else { format!("{} {}", name, k + 1) };
            }
            names.push(name.clone());
            let mut path = course.clone();
            path.push(into);
            self.rivers.push(path);
            self.ow.rivers.push(River { id: k as u32, name, course, into, mouth, joins: st.joins.map(|j| j as u32), flow });
        }
        // Volcanoes on the high peaks.
        let peaks: Vec<usize> = (0..(w * h) as usize).filter(|&i| self.ow.cells[i].biome == Biome::Mountain && self.ow.cells[i].elev > 175 && self.ow.cells[i].river == 0 && self.ow.cells[i].brook == 0).collect();
        for _ in 0..(2 * self.params.scale()).min(peaks.len()) {
            let i = *rng.pick(&peaks);
            self.ow.cells[i].volcano = true;
        }
    }

    fn found_civs(&mut self) {
        let (w, h) = (self.ow.w, self.ow.h);
        let params = self.params;
        // Civilizations and their capitals.
        let plan = params.civ_plan();
        for race in plan {
            let mut spot = None;
            for _ in 0..400 {
                let (x, y) = (self.rng.range(1, w - 1), self.rng.range(1, h - 1));
                let c = self.ow.cell(x, y);
                if c.site == 0 && race.home(c.biome, c.evil) && self.ow.sites.iter().all(|s| (s.x - x).abs() + (s.y - y).abs() > 8) {
                    spot = Some((x, y));
                    break;
                }
            }
            let Some((x, y)) = spot else { continue };
            let id = self.ow.civs.len() as u32;
            let name = civ_name(&mut self.rng, race);
            self.ow.civs.push(Civ { id, name, race, alive: true, enemies: Vec::new(), ruler: None, capital: 0 });
            let sid = self.ow.add_site(&mut self.rng, Overworld::site_kind_for(race, true), Some(id), x, y, 0);
            self.ow.civs[id as usize].capital = sid;
            let (cname, sname) = (self.ow.civs[id as usize].name.clone(), self.ow.sites[sid as usize].name.clone());
            self.ow.log(0, format!("The {} of the {} founded {}.", race.plural(), cname, sname));
            self.ow.crown(&mut self.rng, id, 0);
        }
        // Goblins hate everyone.
        let ids: Vec<u32> = self.ow.civs.iter().map(|c| c.id).collect();
        for &a in &ids {
            for &b in &ids {
                if a != b && (self.ow.civs[a as usize].race == Race::Goblin || self.ow.civs[b as usize].race == Race::Goblin)
                    && self.ow.civs[a as usize].race != self.ow.civs[b as usize].race
                {
                    self.ow.civs[a as usize].enemies.push(b);
                }
            }
        }
    }

    /// Group connected cells of one broad kind of land (a forest of taiga,
    /// forest and jungle is one forest) into regions, name the bigger ones
    /// and give each its wildlife and plants. Naming has its own random
    /// stream so history comes out the same.
    fn name_regions(&mut self) {
        use crate::worldgen::regions;
        let (w, h) = (self.ow.w, self.ow.h);
        let mut rng = Rng::new(self.params.seed ^ 0x2E610);
        let ranges = regions::ranges(self.params.seed, w, h);
        let class = |c: &Cell| regions::broad(c.biome, c.elev);
        let mut seen = vec![false; (w * h) as usize];
        for start in 0..(w * h) as usize {
            if seen[start] {
                continue;
            }
            let k = class(&self.ow.cells[start]);
            let mut cells = vec![start];
            let mut stack = vec![start];
            seen[start] = true;
            let mut edge = false;
            while let Some(i) = stack.pop() {
                let (x, y) = (i as i32 % w, i as i32 / w);
                if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
                    edge = true;
                }
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= w || ny >= h {
                        continue;
                    }
                    let j = (ny * w + nx) as usize;
                    if !seen[j] && class(&self.ow.cells[j]) == k {
                        seen[j] = true;
                        cells.push(j);
                        stack.push(j);
                    }
                }
            }
            if cells.len() < 3 {
                continue;
            }
            cells.sort_unstable();
            let kind = match k {
                RegionKind::Ocean if edge && cells.len() > 400 => RegionKind::Ocean,
                RegionKind::Ocean if edge => RegionKind::Sea,
                RegionKind::Ocean => RegionKind::Lake,
                other => other,
            };
            let mut biomes: Vec<(Biome, u32)> = Vec::new();
            for &i in &cells {
                let b = self.ow.cells[i].biome;
                match biomes.iter_mut().find(|e| e.0 == b) {
                    Some(e) => e.1 += 1,
                    None => biomes.push((b, 1)),
                }
            }
            biomes.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.code().cmp(&b.0.code())));
            // Named for the land that makes up most of it.
            let flavour = match (kind, biomes[0].0) {
                (RegionKind::Forest, Biome::Jungle) => RegionKind::Jungle,
                (RegionKind::Forest, Biome::Taiga) => RegionKind::Taiga,
                (RegionKind::Plains, Biome::Savanna) => RegionKind::Savanna,
                (RegionKind::Desert, Biome::Badlands) => RegionKind::Badlands,
                (k, _) => k,
            };
            let id = self.ow.regions.len() as u32;
            // No frozen jungles: a warm land's names don't speak of ice.
            let warmth = cells.iter().map(|&i| *self.ow.fields.temp.get(i).unwrap_or(&50) as u32).sum::<u32>() / cells.len() as u32;
            let suit = |n: String| if warmth >= 50 { n.replace("Frozen", "Burning") } else { n };
            // Names are unique; after a few tries fall back to a numbered one.
            let mut name = suit(region_name(&mut rng, flavour));
            for attempt in 0..12 {
                if !self.ow.regions.iter().any(|r| r.name == name) {
                    break;
                }
                name = if attempt < 11 { suit(region_name(&mut rng, flavour)) } else { format!("{} {}", name, id + 1) };
            }
            let climate: Vec<regions::CellClimate> = cells
                .iter()
                .map(|&i| regions::CellClimate {
                    x: i as i32 % w,
                    y: i as i32 / w,
                    temp: *self.ow.fields.temp.get(i).unwrap_or(&50) as f32,
                    rain: *self.ow.fields.rain.get(i).unwrap_or(&50) as f32,
                    elev: self.ow.cells[i].elev,
                    biome: self.ow.cells[i].biome,
                })
                .collect();
            let (fauna, plants) = if kind.is_water() { (Vec::new(), Vec::new()) } else { (regions::fauna(&climate, &ranges), regions::plants(&climate)) };
            let mid = cells[cells.len() / 2];
            for &i in &cells {
                self.ow.cells[i].region = id + 1;
            }
            self.ow.regions.push(Region { id, kind, name, size: cells.len() as u32, x: mid as i32 % w, y: mid as i32 / w, biomes, fauna, plants });
        }
    }

    /// Rivers in the deep rock, running long courses under the land: a
    /// site has an underground river exactly when one passes beneath it.
    fn cave_rivers(&mut self) {
        let (w, h) = (self.ow.w, self.ow.h);
        let mut rng = Rng::new(self.params.seed ^ 0xCA7E_0001);
        for _ in 0..3 * self.params.scale() {
            let (mut x, mut y) = (rng.range(4, w - 4) as f64, rng.range(4, h - 4) as f64);
            let mut ang = rng.f64() * std::f64::consts::TAU;
            for _ in 0..rng.range(25, 70) {
                let (cx, cy) = (x.round() as i32, y.round() as i32);
                if cx < 0 || cy < 0 || cx >= w || cy >= h {
                    break;
                }
                self.ow.cells[(cy * w + cx) as usize].deep |= 1;
                ang += (rng.f64() - 0.5) * 0.6;
                // A cell is twice as tall as it is wide.
                x += ang.cos();
                y += ang.sin() * 0.5;
            }
        }
    }

    fn place_beasts(&mut self) {
        let (w, h) = (self.ow.w, self.ow.h);
        // Megabeasts in remote lairs.
        for i in 0..4 * self.params.scale() {
            let (kind, part) = BEASTS[(i + self.rng.range(0, 5) as usize) % BEASTS.len()];
            let mut spot = None;
            for _ in 0..300 {
                let (x, y) = (self.rng.range(0, w), self.rng.range(0, h));
                let c = self.ow.cell(x, y);
                if c.site == 0 && matches!(c.biome, Biome::Mountain | Biome::Swamp | Biome::Marsh | Biome::Desert | Biome::Tundra | Biome::Badlands | Biome::Glacier | Biome::Jungle) {
                    spot = Some((x, y));
                    break;
                }
            }
            let Some((x, y)) = spot else { continue };
            let born = -self.rng.range(200, 900);
            let f = self.ow.add_figure(&mut self.rng, kind, None, None, born);
            let name = format!("{}{}", self.ow.figures[f as usize].name, part);
            self.ow.figures[f as usize].name = name.clone();
            let lair = self.ow.add_site(&mut self.rng, SiteKind::Lair, None, x, y, 0);
            self.ow.sites[lair as usize].name = format!("{}'s lair", name);
            self.ow.figures[f as usize].lair = Some(lair);
            self.ow.log(0, format!("{} the {} made its lair in the {}.", name, kind, self.ow.cell(x, y).biome.name()));
        }

    }
}

// ---------------------------------------------------------------- names

const SYL: [&[&str]; 4] = [
    // dwarf
    &["ur", "ist", "dol", "bem", "kad", "ast", "tob", "zan", "mor", "thr", "eth", "rak", "gol", "nim", "ost", "lun"],
    // human
    &["al", "bar", "cen", "dor", "ed", "far", "gar", "hal", "ia", "jor", "ken", "lor", "mar", "nor", "ra", "wyn"],
    // elf
    &["ae", "li", "na", "the", "ri", "el", "va", "sil", "lo", "mia", "ne", "ira", "fa", "lua", "si", "ten"],
    // goblin
    &["snag", "ug", "zot", "grak", "bub", "nok", "ruz", "gash", "ok", "dug", "kra", "zub", "mog", "rat", "gro", "sk"],
];

fn race_idx(r: Race) -> usize {
    match r {
        Race::Dwarf => 0,
        Race::Human => 1,
        Race::Elf => 2,
        Race::Goblin => 3,
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

const REGION_ADJ: [&str; 16] = [
    "Shimmering", "Endless", "Howling", "Silent", "Golden", "Grey", "Bitter", "Frozen", "Whispering", "Ashen",
    "Verdant", "Sunken", "Broken", "Iron", "Crimson", "Pale",
];

fn region_name(rng: &mut Rng, kind: RegionKind) -> String {
    let adj = *rng.pick(&REGION_ADJ);
    let race = if rng.chance(0.5) { Race::Elf } else { Race::Human };
    let w = word(rng, race, 2);
    let pick = |rng: &mut Rng, a: String, b: String| if rng.chance(0.5) { a } else { b };
    match kind {
        RegionKind::Ocean => pick(rng, format!("The {} Ocean", adj), format!("The Ocean of {}", w)),
        RegionKind::Sea => pick(rng, format!("The {} Sea", adj), format!("The Sea of {}", w)),
        RegionKind::Lake => format!("Lake {}", w),
        RegionKind::Mountains => pick(rng, format!("The {} Peaks", w), format!("The {} Mountains", adj)),
        RegionKind::Hills => pick(rng, format!("The {} Hills", w), format!("The {} Downs", adj)),
        RegionKind::Forest => pick(rng, format!("The {} Wood", adj), format!("{} Forest", w)),
        RegionKind::Plains => pick(rng, format!("The {} Plains", w), format!("The {} Steppe", adj)),
        RegionKind::Desert => pick(rng, format!("The {} Waste", adj), format!("The {} Sands", w)),
        RegionKind::Tundra => pick(rng, format!("The {} Barrens", adj), format!("{} Tundra", w)),
        RegionKind::Swamp => pick(rng, format!("The {} Mire", w), format!("The {} Fen", adj)),
        RegionKind::Jungle => pick(rng, format!("The {} Jungle", adj), format!("The Tangles of {}", w)),
        RegionKind::Taiga => pick(rng, format!("The {} Pinewood", w), format!("The {} Taiga", adj)),
        RegionKind::Savanna => pick(rng, format!("The {} Savanna", w), format!("The {} Grasslands", adj)),
        RegionKind::Badlands => pick(rng, format!("The {} Badlands", adj), format!("The Breaks of {}", w)),
        RegionKind::Glacier => pick(rng, format!("The {} Ice", adj), format!("The {} Glacier", w)),
    }
}

fn river_name(rng: &mut Rng, flow: u32) -> String {
    let adj = *rng.pick(&REGION_ADJ);
    let race = if rng.chance(0.5) { Race::Elf } else { Race::Human };
    let w = word(rng, race, 2);
    let k = rng.range(0, 3);
    if flow >= 120 {
        match k {
            0 => format!("The River {}", w),
            1 => format!("The {} River", w),
            _ => format!("The {} River", adj),
        }
    } else {
        match k {
            0 => format!("The {} Water", w),
            1 => format!("The {} Run", adj),
            _ => format!("{} Creek", w),
        }
    }
}

fn word(rng: &mut Rng, r: Race, syllables: i32) -> String {
    let set = SYL[race_idx(r)];
    let mut s = String::new();
    for _ in 0..syllables {
        s.push_str(*rng.pick(set));
    }
    cap(&s)
}

const PLACE_A: [&str; 12] = [
    "Iron", "Stone", "Ash", "Raven", "Silver", "Oak", "Frost", "Ember", "Gloom", "Bright", "Copper", "Thorn",
];
const PLACE_B: [&str; 12] = [
    "hold", "hall", "gate", "vale", "ford", "spire", "deep", "crag", "mere", "wood", "watch", "barrow",
];
fn civ_name(rng: &mut Rng, r: Race) -> String {
    let (a, b): (&[&str], &[&str]) = match r {
        Race::Dwarf => (
            &["Kingdom", "Clans", "Hold", "Union"],
            &["of the Anvil", "of the Deep Roads", "of the Iron Oath", "of Stone and Fire", "of the Hidden Vaults", "of Ashen Banners"],
        ),
        Race::Human => (
            &["Realm", "League", "Crown", "Commonwealth"],
            &["of the Golden Sheaf", "of the Long Road", "of Silver Rivers", "of the Morning Bell", "of Seven Towers"],
        ),
        Race::Elf => (
            &["Fellowship", "Covenant", "Circle"],
            &["of Whispering Leaves", "of Silver Lanterns", "of the Last Light", "of Falling Stars", "of the Green Song"],
        ),
        Race::Goblin => (
            &["Horde", "Brood", "Dominion", "Pit"],
            &["of Broken Teeth", "of Smoke", "of Red Knives", "of the Long Night", "of Rusted Chains", "of Screams"],
        ),
    };
    format!("{} {}", rng.pick(a), rng.pick(b))
}

const BEASTS: [(&str, &str); 5] = [
    ("dragon", "wing"),
    ("titan", "fist"),
    ("hydra", "maw"),
    ("roc", "talon"),
    ("colossus", "stride"),
];

pub fn person_name(rng: &mut Rng, r: Race) -> String {
    format!("{} {}", word(rng, r, 2), word(rng, r, 2))
}

fn place_name(rng: &mut Rng, r: Race) -> String {
    match r {
        Race::Dwarf | Race::Human => format!("{}{}", rng.pick(&PLACE_A), rng.pick(&PLACE_B)),
        _ => word(rng, r, 3),
    }
}

// ---------------------------------------------------------------- generation

/// A river cell's thin line, from its links (1 north, 2 east, 4 south, 8 west).
pub fn river_glyph(links: u8) -> char {
    match links & 15 {
        1 | 4 | 5 => '│',
        2 | 8 | 10 => '─',
        3 => '└',
        9 => '┘',
        6 => '┌',
        12 => '┐',
        7 => '├',
        13 => '┤',
        11 => '┴',
        14 => '┬',
        _ => '┼',
    }
}

impl Overworld {
    pub fn cell(&self, x: i32, y: i32) -> &Cell {
        &self.cells[(y * self.w + x) as usize]
    }

    /// Whether a site has an aquifer (0 none, 1 light, 2 heavy). Wet lands
    /// usually do, dry and stony ones rarely; fixed for each site.
    pub fn aquifer(&self, x: i32, y: i32) -> u8 {
        let chance = match self.cell(x, y).biome {
            Biome::Swamp => 85,
            Biome::Ocean => 70,
            Biome::Forest => 50,
            Biome::Grassland => 45,
            Biome::Hills => 30,
            Biome::Tundra => 10,
            Biome::Mountain => 5,
            Biome::Desert => 0,
            Biome::Jungle => 70,
            Biome::Taiga => 35,
            Biome::Savanna => 25,
            Biome::Badlands => 3,
            Biome::Glacier => 5,
            Biome::Shrubland => 20,
            Biome::Marsh => 80,
            Biome::Lake => 70,
        };
        let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ (self.year as u64);
        h ^= h >> 29;
        h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        h ^= h >> 32;
        let roll = h % 100;
        if roll >= chance {
            0
        } else if roll < chance / 3 {
            2
        } else {
            1
        }
    }

    /// Everything the embark map takes from the world: the site's biome and
    /// features, and the cells round it (their biomes and heights blend in
    /// toward the map's edges).
    pub fn embark_params(&self, seed: u64, sx: i32, sy: i32, size: crate::world::MapSize) -> EmbarkParams {
        let mut p = self.cell(sx, sy).biome.params();
        p.minerals = self.minerals;
        p.size = size;
        p.aquifer = self.aquifer(sx, sy);
        p.features = self.site_features(seed, sx, sy);
        p.home = (sx, sy);
        p.river = self.cell(sx, sy).river;
        for dy in -3..=3 {
            for dx in -3..=3 {
                let (x, y) = (sx + dx, sy + dy);
                if x >= 0 && y >= 0 && x < self.w && y < self.h {
                    let c = self.cell(x, y);
                    p.around.push(Around { x, y, biome: c.biome, elev: c.elev, evil: c.evil, river: c.river });
                }
            }
        }
        if !self.fields.is_empty() {
            // The map stands on the site's cell at its top left; take its
            // tiles and enough all round for the map to grow either way.
            use crate::worldgen::tiles::{region_tiles, SUB, TILE};
            let m = size.w.max(size.h) / TILE + SUB;
            let (hx, hy) = (sx * SUB, sy * SUB);
            p.region = Some(std::sync::Arc::new(region_tiles(self, hx - m, hy - m, 2 * m + SUB, 2 * m + SUB)));
        }
        p
    }

    /// The site's features: its own roll, plus what the world map shows
    /// (a volcano on the cell). A river is there exactly when the map
    /// draws one through the cell.
    pub fn site_features(&self, seed: u64, x: i32, y: i32) -> u32 {
        let c = self.cell(x, y);
        let mut bits = crate::terrain::site_features(seed, x, y, c.biome) & !crate::terrain::feat::RIVER;
        if c.river != 0 {
            bits |= crate::terrain::feat::RIVER;
        }
        // A world made from fields lays out its cave rivers; an older one
        // keeps the old roll.
        if !self.fields.is_empty() {
            bits &= !crate::terrain::feat::UNDER_RIVER;
            if c.deep & 1 != 0 {
                bits |= crate::terrain::feat::UNDER_RIVER;
            }
        }
        if c.volcano {
            bits |= crate::terrain::feat::VOLCANO;
        }
        bits
    }

    /// How a cell looks on the world map, in the manner of an old atlas:
    /// each biome a dense texture of one glyph family in two or three shades
    /// (picked by a hash of the cell, so it never changes), rivers as thin
    /// lines from cell to cell, and smoking peaks in red.
    pub fn look(&self, x: i32, y: i32) -> (char, Color) {
        let c = self.cell(x, y);
        let mut hsh = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
        hsh ^= hsh >> 31;
        hsh = hsh.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        hsh ^= hsh >> 29;
        let roll = (hsh % 1000) as u32;
        // One of a few glyphs and, independently, one of a few shades.
        let pick = |opts: &[char]| opts[(hsh >> 12) as usize % opts.len()];
        let tone = |opts: &[(u8, u8, u8)]| {
            let (r, g, b) = opts[(hsh >> 24) as usize % opts.len()];
            Color::Rgb(r, g, b)
        };
        let e = c.elev as i32;
        if c.volcano {
            return ('▲', Color::Rgb(240, 80, 40));
        }
        if c.river != 0 {
            // The great rivers a deeper blue than the streams that feed them.
            let i = (y * self.w + x) as usize;
            let big = self.fields.flow.get(i).map_or(false, |&f| crate::worldgen::fields::byte_flow(f) >= 120.0);
            return (river_glyph(c.river), if big { tone(&[(60, 150, 245), (75, 165, 250)]) } else { tone(&[(90, 200, 240), (120, 215, 250)]) });
        }
        match c.biome {
            // Deep water darker, the shallows by the coast lighter.
            Biome::Ocean => ('≈', if e > 75 { Color::Rgb(50, 100, 210) } else { Color::Rgb(30, 70, 180) }),
            Biome::Mountain => {
                if e > 205 {
                    ('▲', tone(&[(240, 240, 250), (215, 215, 225)]))
                } else if e > 185 {
                    (pick(&['▲', '▲', '^']), tone(&[(180, 180, 190), (160, 160, 170)]))
                } else {
                    (pick(&['▲', '^', '^']), tone(&[(140, 135, 130), (120, 115, 110)]))
                }
            }
            Biome::Hills => (pick(&['∩', 'n', '∩']), tone(&[(80, 170, 70), (50, 130, 50)])),
            Biome::Forest => (pick(&['♣', '♣', '♣', '♠']), tone(&[(40, 160, 60), (30, 120, 45), (60, 185, 75)])),
            Biome::Grassland => ('"', tone(&[(110, 200, 85), (80, 170, 60)])),
            Biome::Desert => ('~', tone(&[(230, 205, 120), (210, 180, 95), (240, 220, 150)])),
            Biome::Tundra => ('"', tone(&[(190, 210, 215), (160, 185, 190)])),
            Biome::Swamp => (pick(&['%', '%', '"']), tone(&[(70, 150, 125), (50, 120, 100)])),
            Biome::Jungle => (pick(&['♠', '♠', '♣']), tone(&[(30, 200, 90), (20, 160, 70), (70, 220, 110)])),
            Biome::Taiga => (pick(&['↑', '↑', '↑', '♠']), tone(&[(60, 140, 110), (40, 110, 90)])),
            Biome::Savanna => {
                if roll % 11 == 0 {
                    ('♣', Color::Rgb(160, 165, 60))
                } else {
                    ('"', tone(&[(205, 195, 90), (180, 170, 70)]))
                }
            }
            Biome::Badlands => (pick(&['∩', 'n', '∩']), tone(&[(205, 115, 65), (175, 90, 50)])),
            Biome::Glacier => (pick(&['░', '▒', '░']), tone(&[(210, 235, 255), (180, 210, 240)])),
            Biome::Shrubland => (pick(&['τ', '"', 'τ']), tone(&[(170, 160, 95), (150, 140, 80), (185, 175, 110)])),
            Biome::Marsh => (pick(&['"', '"', '%']), tone(&[(90, 165, 150), (70, 140, 130)])),
            Biome::Lake => ('≈', Color::Rgb(70, 140, 230)),
        }
    }

    /// A cell as the finished world map draws it: the land's glyph and
    /// colour (a site's mark over it), on a dark ground tinted where the
    /// land is evil.
    pub fn map_look(&self, x: i32, y: i32) -> (char, Color, Color) {
        let c = self.cell(x, y);
        let (mut ch, mut fg) = self.look(x, y);
        let bg = if c.evil { Color::Rgb(45, 10, 55) } else { Color::Rgb(0, 0, 0) };
        if c.site > 0 {
            (ch, fg) = self.site_look(&self.sites[(c.site - 1) as usize]);
        }
        (ch, fg, bg)
    }

    /// A site's mark on the world map: small and bright, so it stands out
    /// of the land round it.
    pub fn site_look(&self, s: &Site) -> (char, Color) {
        if s.ruined {
            return ('×', Color::Rgb(170, 170, 170));
        }
        let ch = s.kind.glyph();
        let col = match (s.kind, s.civ) {
            (SiteKind::Lair, _) => Color::Rgb(255, 110, 230),
            (SiteKind::Town, _) => Color::Rgb(255, 255, 255),
            (SiteKind::Hamlet, _) => Color::Rgb(255, 240, 120),
            (_, Some(civ)) => self.civs[civ as usize].race.color(),
            _ => Color::Rgb(200, 200, 200),
        };
        (ch, col)
    }

    fn cell_mut(&mut self, x: i32, y: i32) -> &mut Cell {
        let w = self.w;
        &mut self.cells[(y * w + x) as usize]
    }

    pub fn site_at(&self, x: i32, y: i32) -> Option<&Site> {
        let s = self.cell(x, y).site;
        (s > 0).then(|| &self.sites[(s - 1) as usize])
    }

    pub fn civ(&self, id: u32) -> &Civ {
        &self.civs[id as usize]
    }

    /// The biggest named river running through a cell, if any.
    pub fn river_at(&self, x: i32, y: i32) -> Option<&River> {
        self.rivers.iter().filter(|r| r.course.contains(&(x, y))).max_by_key(|r| r.flow)
    }

    /// The named region a cell lies in, if any.
    pub fn region_at(&self, x: i32, y: i32) -> Option<&Region> {
        match self.cell(x, y).region {
            0 => None,
            r => self.regions.get(r as usize - 1),
        }
    }

    pub fn embarkable(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h && !self.cell(x, y).biome.is_water() && self.cell(x, y).site == 0
    }

    /// Living civilizations with a site within `r` cells, nearest first.
    pub fn neighbors(&self, x: i32, y: i32, r: i32) -> Vec<Neighbor> {
        let mut best: Vec<Neighbor> = Vec::new();
        for s in self.sites.iter().filter(|s| !s.ruined) {
            let Some(civ) = s.civ else { continue };
            if !self.civs[civ as usize].alive {
                continue;
            }
            let d = (s.x - x).abs().max((s.y - y).abs());
            if d > r {
                continue;
            }
            match best.iter_mut().find(|n| n.civ == civ) {
                Some(n) => n.dist = n.dist.min(d),
                None => best.push(Neighbor { civ, race: self.civs[civ as usize].race, dist: d }),
            }
        }
        best.sort_by_key(|n| n.dist);
        best
    }

    pub fn log(&mut self, year: i32, text: String) {
        self.events.push(Event { year, text });
    }

    fn free_spot_near(&self, rng: &mut Rng, x: i32, y: i32, r: i32, race: Race) -> Option<(i32, i32)> {
        for _ in 0..60 {
            let nx = x + rng.range(-r, r + 1);
            let ny = y + rng.range(-r, r + 1);
            if nx < 0 || ny < 0 || nx >= self.w || ny >= self.h {
                continue;
            }
            let c = self.cell(nx, ny);
            if c.site == 0 && !c.biome.is_water() && race.home(c.biome, c.evil) {
                return Some((nx, ny));
            }
        }
        None
    }

    fn add_site(&mut self, rng: &mut Rng, kind: SiteKind, civ: Option<u32>, x: i32, y: i32, year: i32) -> u32 {
        let id = self.sites.len() as u32;
        let race = civ.map(|c| self.civs[c as usize].race).unwrap_or(Race::Goblin);
        let name = place_name(rng, race);
        let pop = match kind {
            SiteKind::Town | SiteKind::DarkFortress => 300,
            SiteKind::Fortress | SiteKind::Retreat => 200,
            SiteKind::Hamlet => 60,
            SiteKind::Lair => 0,
        };
        self.sites.push(Site { id, name, kind, civ, x, y, pop, ruined: false, founded: year });
        self.cell_mut(x, y).site = id + 1;
        id
    }

    fn add_figure(&mut self, rng: &mut Rng, kind: &str, race: Option<Race>, civ: Option<u32>, born: i32) -> u32 {
        let id = self.figures.len() as u32;
        let name = match race {
            Some(r) => person_name(rng, r),
            None => format!("{}{}", word(rng, Race::Goblin, 1), word(rng, Race::Dwarf, 1).to_lowercase()),
        };
        self.figures.push(Figure {
            id,
            name,
            kind: kind.to_string(),
            civ,
            born,
            died: None,
            cause: None,
            title: None,
            kills: 0,
            lair: None,
        });
        id
    }

    fn site_kind_for(race: Race, capital: bool) -> SiteKind {
        match race {
            Race::Dwarf => SiteKind::Fortress,
            Race::Human => {
                if capital {
                    SiteKind::Town
                } else {
                    SiteKind::Hamlet
                }
            }
            Race::Elf => SiteKind::Retreat,
            Race::Goblin => SiteKind::DarkFortress,
        }
    }

    fn crown(&mut self, rng: &mut Rng, civ: u32, year: i32) {
        let race = self.civs[civ as usize].race;
        let born = year - rng.range(20, 60);
        let f = self.add_figure(rng, &format!("{:?}", race).to_lowercase(), Some(race), Some(civ), born);
        let title = match race {
            Race::Dwarf => "king",
            Race::Human => "queen",
            Race::Elf => "druid",
            Race::Goblin => "warlord",
        };
        self.figures[f as usize].title = Some(format!("{} of the {}", title, self.civs[civ as usize].name));
        self.civs[civ as usize].ruler = Some(f);
        let fname = self.figures[f as usize].name.clone();
        let cname = self.civs[civ as usize].name.clone();
        self.log(year, format!("{} became {} of the {}.", fname, title, cname));
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn generate(seed: u64) -> Overworld {
        Self::generate_with(seed, &|_, _| {})
    }

    /// A stand-in before any world exists (the title screen).
    pub fn empty() -> Overworld {
        Overworld {
            w: 1,
            h: 1,
            cells: vec![Cell { biome: Biome::Ocean, evil: false, site: 0, region: 0, elev: 0, river: 0, volcano: false, brook: 0, deep: 0 }],
            civs: Vec::new(),
            sites: Vec::new(),
            figures: Vec::new(),
            events: Vec::new(),
            year: 0,
            name: String::new(),
            savagery: 1.0,
            minerals: 1.0,
            regions: Vec::new(),
            seed: 0,
            fields: Default::default(),
            rivers: Vec::new(),
            clashes: None,
            gods: Vec::new(),
        }
    }

    /// Generate with default settings, reporting each stage and how far along it is (0..1).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn generate_with(seed: u64, report: &dyn Fn(&str, f32)) -> Overworld {
        let mut g = WorldGen::new(WorldParams { seed, ..WorldParams::default() });
        while !g.step(usize::MAX, 5) {
            let (stage, frac) = g.progress();
            report(&stage, frac);
        }
        g.finish()
    }

    fn living_sites(&self, civ: u32) -> Vec<u32> {
        self.sites.iter().filter(|s| s.civ == Some(civ) && !s.ruined).map(|s| s.id).collect()
    }

    /// Living megabeasts whose lairs lie within `r` cells of (x, y), nearest first.
    pub fn beasts_near(&self, x: i32, y: i32, r: i32) -> Vec<u32> {
        let mut v: Vec<(i32, u32)> = self
            .figures
            .iter()
            .filter(|f| f.alive())
            .filter_map(|f| {
                let lair = &self.sites[f.lair? as usize];
                let d = (lair.x - x).abs().max((lair.y - y).abs());
                (d <= r).then_some((d, f.id))
            })
            .collect();
        v.sort();
        v.into_iter().map(|(_, id)| id).collect()
    }

    pub fn kill_figure(&mut self, f: u32, year: i32, cause: String) {
        let fig = &mut self.figures[f as usize];
        if fig.died.is_some() {
            return;
        }
        fig.died = Some(year);
        fig.cause = Some(cause);
    }

    fn simulate_year(&mut self, rng: &mut Rng, year: i32) {
        let civ_ids: Vec<u32> = self.civs.iter().filter(|c| c.alive).map(|c| c.id).collect();
        for &c in &civ_ids {
            let race = self.civs[c as usize].race;
            let sites = self.living_sites(c);
            for &s in &sites {
                let site = &mut self.sites[s as usize];
                site.pop = (site.pop as f64 * 1.015) as u32 + 1;
            }
            // Expansion.
            if !sites.is_empty() && rng.chance(0.18) {
                let from = self.sites[*rng.pick(&sites) as usize].clone();
                if let Some((x, y)) = self.free_spot_near(rng, from.x, from.y, 6, race) {
                    let sid = self.add_site(rng, Overworld::site_kind_for(race, false), Some(c), x, y, year);
                    let text = format!(
                        "The {} of {} founded the {} of {}.",
                        race.plural(),
                        from.name,
                        self.sites[sid as usize].kind.name(),
                        self.sites[sid as usize].name
                    );
                    self.log(year, text);
                }
            }
            // Rulers age and die.
            if let Some(r) = self.civs[c as usize].ruler {
                let fig = &self.figures[r as usize];
                if fig.died.is_some() || year - fig.born > race.lifespan() + rng.range(-10, 10) {
                    if fig.died.is_none() {
                        let name = fig.name.clone();
                        self.kill_figure(r, year, "died of old age".into());
                        self.log(year, format!("{} died of old age.", name));
                    }
                    self.crown(rng, c, year);
                }
            }
            // Masterworks.
            if race == Race::Dwarf && rng.chance(0.04) && !sites.is_empty() {
                let born = year - rng.range(40, 100);
                let smith = self.add_figure(rng, "dwarf", Some(race), Some(c), born);
                let art = crate::dwarf::artifact_name(rng);
                let what = rng.pick(&["steel axe", "gold crown", "marble statue", "iron shield", "amethyst ring"]);
                let sname = self.sites[*rng.pick(&sites) as usize].name.clone();
                let fname = self.figures[smith as usize].name.clone();
                self.log(year, format!("{} created {}, a {}, in {}.", fname, art, what, sname));
            }
        }
        // Diplomacy among the non-goblins.
        for &a in &civ_ids {
            for &b in &civ_ids {
                if a >= b || self.civs[a as usize].race == Race::Goblin || self.civs[b as usize].race == Race::Goblin {
                    continue;
                }
                let at_war = self.civs[a as usize].enemies.contains(&b);
                if !at_war && rng.chance(0.01) {
                    self.civs[a as usize].enemies.push(b);
                    self.civs[b as usize].enemies.push(a);
                    let (an, bn) = (self.civs[a as usize].name.clone(), self.civs[b as usize].name.clone());
                    self.log(year, format!("The {} declared war on the {}.", an, bn));
                } else if at_war && rng.chance(0.12) {
                    self.civs[a as usize].enemies.retain(|&e| e != b);
                    self.civs[b as usize].enemies.retain(|&e| e != a);
                    let (an, bn) = (self.civs[a as usize].name.clone(), self.civs[b as usize].name.clone());
                    self.log(year, format!("The {} and the {} made peace.", an, bn));
                }
            }
        }
        // Battles.
        for &a in &civ_ids {
            if !self.civs[a as usize].alive {
                continue;
            }
            let enemies = self.civs[a as usize].enemies.clone();
            for b in enemies {
                if !self.civs[b as usize].alive || !rng.chance(0.12) {
                    continue;
                }
                self.battle(rng, a, b, year);
            }
        }
        // Megabeasts.
        let beasts: Vec<u32> = self.figures.iter().filter(|f| f.lair.is_some() && f.alive()).map(|f| f.id).collect();
        for f in beasts {
            if !rng.chance(0.07) {
                continue;
            }
            let lair = self.sites[self.figures[f as usize].lair.unwrap() as usize].clone();
            let target = self
                .sites
                .iter()
                .filter(|s| !s.ruined && s.civ.is_some() && (s.x - lair.x).abs().max((s.y - lair.y).abs()) <= 12)
                .min_by_key(|s| (s.x - lair.x).abs() + (s.y - lair.y).abs())
                .map(|s| s.id);
            let Some(t) = target else { continue };
            let (bname, bkind) = (self.figures[f as usize].name.clone(), self.figures[f as usize].kind.clone());
            let sname = self.sites[t as usize].name.clone();
            if rng.chance(0.2) {
                let civ = self.sites[t as usize].civ.unwrap();
                let race = self.civs[civ as usize].race;
                let born = year - rng.range(20, 60);
                let hero = self.add_figure(rng, &format!("{:?}", race).to_lowercase(), Some(race), Some(civ), born);
                self.figures[hero as usize].kills += 1;
                self.figures[hero as usize].title = Some(format!("slayer of {}", bname));
                self.kill_figure(f, year, format!("slain by {}", self.figures[hero as usize].name));
                let hname = self.figures[hero as usize].name.clone();
                self.clash(year, lair.id, t, ClashEnd::Slain);
                self.log(year, format!("{} slew {} the {} at {}.", hname, bname, bkind, sname));
            } else {
                self.figures[f as usize].kills += rng.range(5, 40) as u32;
                let protected = self.civs.iter().any(|c| c.race == Race::Dwarf && c.capital == t);
                let s = &mut self.sites[t as usize];
                s.pop = s.pop * 2 / 3;
                if !protected && (s.pop < 40 || rng.chance(0.3)) {
                    s.ruined = true;
                    self.clash(year, lair.id, t, ClashEnd::Fell);
                    self.log(year, format!("{} the {} laid waste to {}.", bname, bkind, sname));
                    self.check_extinct(year);
                } else {
                    self.clash(year, lair.id, t, ClashEnd::Held);
                    self.log(year, format!("{} the {} attacked {} and was driven off.", bname, bkind, sname));
                }
            }
        }
    }

    fn battle(&mut self, rng: &mut Rng, att: u32, def: u32, year: i32) {
        let mine = self.living_sites(att);
        let theirs = self.living_sites(def);
        if mine.is_empty() || theirs.is_empty() {
            return;
        }
        // Strike the closest enemy site within reach.
        let mut best: Option<(i32, u32, u32)> = None;
        for &m in &mine {
            for &t in &theirs {
                let (ms, ts) = (&self.sites[m as usize], &self.sites[t as usize]);
                let d = (ms.x - ts.x).abs().max((ms.y - ts.y).abs());
                if d <= 18 && best.map_or(true, |b| d < b.0) {
                    best = Some((d, m, t));
                }
            }
        }
        let Some((_, from, target)) = best else { return };
        let a_pow = self.sites[from as usize].pop as f64 * (0.5 + rng.f64());
        let d_pow = self.sites[target as usize].pop as f64 * (0.6 + rng.f64());
        let (an, ar) = (self.civs[att as usize].name.clone(), self.civs[att as usize].race);
        let tname = self.sites[target as usize].name.clone();
        let label = format!("{} {}", rng.pick(&["The Battle of", "The Siege of", "The Sack of"]), tname);
        // A dwarven mountainhome is never taken.
        let mountainhome = self.civs[def as usize].race == Race::Dwarf && self.civs[def as usize].capital == target;
        if a_pow > d_pow && mountainhome {
            let t = &mut self.sites[target as usize];
            t.pop = t.pop * 2 / 3;
            self.clash(year, from, target, ClashEnd::Held);
            self.log(year, format!("{}: the {} of the {} broke upon the gates of the mountainhome.", label, ar.plural(), an));
        } else if a_pow > d_pow {
            let t = &mut self.sites[target as usize];
            t.pop = t.pop / 3;
            if ar == Race::Goblin && rng.chance(0.5) {
                t.civ = Some(att);
                t.kind = SiteKind::DarkFortress;
                self.clash(year, from, target, ClashEnd::Taken);
                self.log(year, format!("{}: the {} of the {} conquered {}.", label, ar.plural(), an, tname));
            } else {
                t.ruined = true;
                self.clash(year, from, target, ClashEnd::Fell);
                self.log(year, format!("{}: the {} of the {} razed {}.", label, ar.plural(), an, tname));
            }
            // Sometimes a ruler falls.
            if rng.chance(0.25) {
                if let Some(r) = self.civs[def as usize].ruler {
                    if self.figures[r as usize].alive() {
                        let rn = self.figures[r as usize].name.clone();
                        self.kill_figure(r, year, format!("fell defending {}", tname));
                        self.log(year, format!("{} fell defending {}.", rn, tname));
                        self.civs[def as usize].ruler = None;
                    }
                }
            }
            self.check_extinct(year);
        } else {
            let f = &mut self.sites[from as usize];
            f.pop = f.pop * 4 / 5;
            self.clash(year, from, target, ClashEnd::Repulsed);
            self.log(year, format!("{}: the {} of the {} were repulsed.", label, ar.plural(), an));
        }
    }

    /// Keep a clash for watching, when a world is being made to be watched.
    fn clash(&mut self, year: i32, from: u32, to: u32, end: ClashEnd) {
        if let Some(c) = self.clashes.as_mut() {
            c.push(Clash { year, from, to, end });
        }
    }

    fn check_extinct(&mut self, year: i32) {
        for i in 0..self.civs.len() {
            let id = self.civs[i].id;
            if self.civs[i].alive && self.living_sites(id).is_empty() {
                self.civs[i].alive = false;
                let text = format!("The {} of the {} have vanished from the world.", self.civs[i].race.plural(), self.civs[i].name);
                self.log(year, text);
            }
        }
        for i in 0..self.civs.len() {
            if self.civs[i].alive && self.civs[i].ruler.is_none() {
                // Crowned lazily in the next year's succession pass.
            }
        }
    }

    /// A sensible default embark: open land near a dwarven civilization.
    pub fn default_site(&self) -> (i32, i32) {
        let dwarf_sites: Vec<&Site> = self
            .sites
            .iter()
            .filter(|s| !s.ruined && s.civ.map_or(false, |c| self.civs[c as usize].race == Race::Dwarf && self.civs[c as usize].alive))
            .collect();
        let mut best = None;
        let mut best_score = i32::MIN;
        for y in 0..self.h {
            for x in 0..self.w {
                if !self.embarkable(x, y) {
                    continue;
                }
                let c = self.cell(x, y);
                let mut score = match c.biome {
                    Biome::Hills | Biome::Forest => 20,
                    Biome::Mountain | Biome::Grassland => 12,
                    _ => 0,
                };
                if c.evil {
                    score -= 30;
                }
                // The map spreads east and south into the neighbours: a first
                // fortress wants land there, not the sea.
                for (dx, dy, cost) in [(1, 0, 14), (0, 1, 14), (1, 1, 14), (-1, 0, 5), (0, -1, 5), (-1, -1, 3), (1, -1, 3), (-1, 1, 3)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= self.w || ny >= self.h || self.cell(nx, ny).biome.is_water() {
                        score -= cost;
                    }
                }
                // A river makes a kind first fortress; the frozen far north and south don't.
                if c.river != 0 {
                    score += 30;
                }
                // Water-bearing rock makes a hard first dig.
                score -= 12 * self.aquifer(x, y) as i32;
                let lat = (y as f64 / self.h as f64 - 0.5).abs();
                if lat > 0.32 {
                    score -= 25;
                }
                let d = dwarf_sites.iter().map(|s| (s.x - x).abs().max((s.y - y).abs())).min().unwrap_or(40);
                score -= (d - 4).abs() * 2;
                if score > best_score {
                    best_score = score;
                    best = Some((x, y));
                }
            }
        }
        best.unwrap_or((self.w / 2, self.h / 2))
    }

    /// The dwarven civilization that sends an expedition to (x, y).
    pub fn parent_civ(&self, x: i32, y: i32) -> Option<u32> {
        self.sites
            .iter()
            .filter(|s| !s.ruined)
            .filter_map(|s| s.civ.map(|c| (c, s)))
            .filter(|(c, _)| self.civs[*c as usize].race == Race::Dwarf && self.civs[*c as usize].alive)
            .min_by_key(|(_, s)| (s.x - x).abs() + (s.y - y).abs())
            .map(|(c, _)| c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stepwise_generation_matches_one_go() {
        for seed in [3u64, 99] {
            let params = WorldParams { seed, history: 60, civs: 2, savagery: 0, minerals: 2, size: 1 };
            let mut g = WorldGen::new(params);
            let mut steps = 0;
            while !g.step(1, 1) {
                steps += 1;
            }
            assert!(steps > 60, "generation wasn't actually stepped ({} steps)", steps);
            let stepped = serde_json::to_string(&g.finish()).unwrap();
            let one_go = serde_json::to_string(&WorldGen::new(params).finish()).unwrap();
            assert!(stepped == one_go, "seed {}: stepped world differs from one-go world", seed);
        }
        // Settings matter.
        let a = WorldGen::new(WorldParams { seed: 5, ..WorldParams::default() }).finish();
        let b = WorldGen::new(WorldParams { seed: 5, civs: 2, ..WorldParams::default() }).finish();
        assert!(b.civs.len() > a.civs.len());
        assert_eq!(a.year, 200);
    }

    #[test]
    fn world_map_reads_as_solid_texture() {
        let thin = ['·', '.', ',', '`', '\'', ' ', '∙'];
        let heavy = ['═', '║', '╔', '╗', '╚', '╝', '╠', '╣', '╦', '╩', '╬'];
        let lines = ['─', '│', '┌', '┐', '└', '┘', '├', '┤', '┬', '┴', '┼'];
        for seed in [1u64, 42, 777] {
            let ow = Overworld::generate(seed);
            let mut ocean = None;
            for y in 0..ow.h {
                for x in 0..ow.w {
                    let c = ow.cell(x, y);
                    let (ch, _) = ow.look(x, y);
                    assert!(!heavy.contains(&ch), "seed {}: heavy line {} at {:?}", seed, ch, (x, y));
                    if c.biome == Biome::Ocean {
                        assert_eq!(*ocean.get_or_insert(ch), ch, "seed {}: ocean glyphs differ at {:?}", seed, (x, y));
                        continue;
                    }
                    assert!(!thin.contains(&ch), "seed {}: near-empty {:?} on {} at {:?}", seed, ch, c.biome.name(), (x, y));
                    if c.river != 0 && !c.volcano {
                        assert!(lines.contains(&ch), "seed {}: river drawn as {:?} at {:?}", seed, ch, (x, y));
                    }
                }
            }
            assert_eq!(ocean, Some('≈'));
            // Every site stands out from the land under it.
            for s in &ow.sites {
                let (ch, _) = ow.site_look(s);
                assert!(!thin.contains(&ch) && ch != ow.look(s.x, s.y).0 || s.kind == SiteKind::Retreat);
            }
        }
    }
    #[test]
    fn rivers_run_downhill_to_the_sea_or_a_lake() {
        for seed in [1u64, 7, 42, 99, 5003] {
            let mut g = WorldGen::new(WorldParams { seed, ..WorldParams::default() });
            while g.stage == GenStage::Terrain {
                g.step(usize::MAX, 0);
            }
            let ow = &g.ow;
            assert!(g.rivers.len() >= 5, "seed {}: only {} rivers", seed, g.rivers.len());
            let flow = |x: i32, y: i32| ow.fields.flow[(y * ow.w + x) as usize];
            let mut on_course = std::collections::HashSet::new();
            for (r, named) in g.rivers.iter().zip(&ow.rivers) {
                for i in 1..r.len() {
                    let (a, b) = (ow.cell(r[i - 1].0, r[i - 1].1), ow.cell(r[i].0, r[i].1));
                    assert!(b.elev <= a.elev, "seed {}: river climbs from {} to {} at {:?}", seed, a.elev, b.elev, r[i]);
                }
                // More water the further down it runs.
                for i in 1..named.course.len() {
                    let ((ax, ay), (bx, by)) = (named.course[i - 1], named.course[i]);
                    assert!(flow(bx, by) >= flow(ax, ay), "seed {}: {} loses water at {:?}", seed, named.name, (bx, by));
                }
                // It ends in the sea, a lake, or the river it feeds.
                let end = ow.cell(named.into.0, named.into.1);
                match named.mouth {
                    Mouth::Sea => assert_eq!(end.biome, Biome::Ocean, "seed {}: {} ends on land", seed, named.name),
                    Mouth::Lake => assert_eq!(end.biome, Biome::Lake),
                    Mouth::River => {
                        assert!(end.river != 0, "seed {}: {} joins nothing", seed, named.name);
                        let j = named.joins.unwrap() as usize;
                        assert!(ow.rivers[j].course.contains(&named.into));
                    }
                }
                on_course.extend(named.course.iter().copied());
            }
            // Every river cell on the map lies on one of those courses.
            for y in 0..ow.h {
                for x in 0..ow.w {
                    if ow.cell(x, y).river != 0 {
                        assert!(on_course.contains(&(x, y)), "seed {}: stray river at {:?}", seed, (x, y));
                    }
                }
            }
            // Tributaries, names, brooks (which the map doesn't draw).
            assert!(ow.rivers.iter().any(|r| r.joins.is_some()), "seed {}: no tributaries", seed);
            let names: std::collections::HashSet<&str> = ow.rivers.iter().map(|r| r.name.as_str()).collect();
            assert_eq!(names.len(), ow.rivers.len(), "seed {}: river names repeat", seed);
            let brooks: Vec<(i32, i32)> = (0..ow.h).flat_map(|y| (0..ow.w).map(move |x| (x, y))).filter(|&(x, y)| ow.cell(x, y).brook != 0 && ow.cell(x, y).river == 0).collect();
            assert!(brooks.len() > g.rivers.len(), "seed {}: only {} brook cells", seed, brooks.len());
            for &(x, y) in &brooks {
                assert!(!"─│┌┐└┘├┤┬┴┼".contains(ow.look(x, y).0) || ow.cell(x, y).volcano, "seed {}: a brook drawn at {:?}", seed, (x, y));
            }
            // A site has a river exactly where the map draws one, and the
            // default site still likes one.
            let ow = g.finish();
            for y in 0..ow.h {
                for x in 0..ow.w {
                    let has = ow.site_features(seed, x, y) & crate::terrain::feat::RIVER != 0;
                    assert_eq!(has, ow.cell(x, y).river != 0, "seed {}: river feature at {:?}", seed, (x, y));
                }
            }
            let (x, y) = ow.default_site();
            assert!(ow.cell(x, y).river != 0, "seed {}: default site without a river", seed);
            assert_eq!(ow.embark_params(seed, x, y, crate::world::MapSize::MEDIUM).river, ow.cell(x, y).river);
            assert!(ow.river_at(x, y).is_some());
        }
    }

    #[test]
    fn regions_are_connected_lands_of_one_kind_with_wildlife_of_their_own() {
        use crate::worldgen::regions::broad;
        let mut mixed_forest = Vec::new();
        let mut lands: Vec<(RegionKind, u32, Vec<crate::creature::Species>)> = Vec::new();
        for seed in [1u64, 42, 7] {
            let ow = Overworld::generate(seed);
            let kind_at = |x: i32, y: i32| broad(ow.cell(x, y).biome, ow.cell(x, y).elev);
            for r in &ow.regions {
                let cells: Vec<(i32, i32)> = (0..ow.h).flat_map(|y| (0..ow.w).map(move |x| (x, y))).filter(|&(x, y)| ow.cell(x, y).region == r.id + 1).collect();
                assert_eq!(cells.len() as u32, r.size, "seed {}: {} size", seed, r.name);
                assert_eq!(r.biomes.iter().map(|b| b.1).sum::<u32>(), r.size);
                // One broad kind of land...
                let k = kind_at(cells[0].0, cells[0].1);
                assert!(cells.iter().all(|&(x, y)| kind_at(x, y) == k), "seed {}: {} mixes kinds", seed, r.name);
                // ...all of a piece.
                let mut seen = std::collections::HashSet::new();
                let mut stack = vec![cells[0]];
                seen.insert(cells[0]);
                while let Some((x, y)) = stack.pop() {
                    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        let (nx, ny) = (x + dx, y + dy);
                        if nx >= 0 && ny >= 0 && nx < ow.w && ny < ow.h && ow.cell(nx, ny).region == r.id + 1 && seen.insert((nx, ny)) {
                            stack.push((nx, ny));
                        }
                    }
                }
                assert_eq!(seen.len(), cells.len(), "seed {}: {} is in pieces", seed, r.name);
                if r.kind == RegionKind::Forest && r.biomes.len() >= 2 && r.biomes[1].1 >= 5 {
                    mixed_forest.push(format!("{}: {:?}", r.name, r.biomes));
                }
                if !r.kind.is_water() && r.size >= 20 {
                    assert!(!r.fauna.is_empty() && !r.plants.is_empty(), "seed {}: nothing lives in {} {:?}", seed, r.name, r.biomes);
                    lands.push((r.kind, r.size, r.fauna.clone()));
                }
            }
        }
        // One forest can be taiga in the north and jungle in the south.
        assert!(!mixed_forest.is_empty(), "no forest spans two kinds of forest");
        eprintln!("mixed forests: {:?}", &mixed_forest[..mixed_forest.len().min(3)]);
        // Wildlife has ranges: the same kind of land elsewhere holds other beasts.
        let differ = lands.iter().any(|a| lands.iter().any(|b| a.0 == b.0 && a.2 != b.2));
        assert!(differ, "every region of a kind has the same wildlife");
        // And the fortress's animals come from its region.
        let g = crate::game::Game::new(42);
        let r = g.overworld.region_at(g.site.0, g.site.1).unwrap();
        let (calm, pack, rare) = g.fauna();
        for s in [calm, pack, rare] {
            let role = crate::worldgen::regions::role(s);
            assert!(r.fauna.contains(&s) || !r.fauna.iter().any(|&f| crate::worldgen::regions::role(f) == role), "{:?} isn't of {}", s, r.name);
        }
    }

    #[test]
    fn world_has_history() {
        for seed in 1..4 {
            let ow = Overworld::generate(seed);
            assert!(ow.civs.len() >= 4, "too few civs");
            assert!(ow.events.len() > 50, "history too thin: {}", ow.events.len());
            let (x, y) = ow.default_site();
            assert!(ow.embarkable(x, y));
            assert!(ow.parent_civ(x, y).is_some() || ow.civs.iter().all(|c| c.race != Race::Dwarf || !c.alive));
            if seed == 1 {
                for e in ow.events.iter().rev().take(10) {
                    eprintln!("{:4}: {}", e.year, e.text);
                }
                eprintln!("{} events, {} sites, {} figures", ow.events.len(), ow.sites.len(), ow.figures.len());
            }
        }
    }

    /// Print a world map (glyphs only) and how long it took:
    /// `cargo test --release print_world -- --ignored --nocapture` (SEED=n).
    #[test]
    #[ignore]
    fn print_world() {
        let seeds: Vec<u64> = std::env::var("SEED").ok().map(|s| s.split(',').filter_map(|v| v.parse().ok()).collect()).unwrap_or(vec![42, 7]);
        for seed in seeds {
            let t0 = std::time::Instant::now();
            let mut g = WorldGen::new(WorldParams { seed, size: 1, ..WorldParams::default() });
            while g.stage == GenStage::Terrain {
                g.step(usize::MAX, 0);
            }
            let land = t0.elapsed().as_secs_f64() * 1000.0;
            let ow = g.finish();
            let all = t0.elapsed().as_secs_f64() * 1000.0;
            println!("== world {} ({}x{}): land {:.0} ms, with history {:.0} ms, {} regions, {} sites", seed, ow.w, ow.h, land, all, ow.regions.len(), ow.sites.len());
            for y in 0..ow.h {
                let sites = std::env::var("SITES").is_ok();
                let row: String = (0..ow.w).map(|x| match ow.site_at(x, y) { Some(s) if sites => ow.site_look(s).0, _ => ow.look(x, y).0 }).collect();
                println!("{}", row);
            }
            if std::env::var("REGIONS").is_ok() {
                let mut rs: Vec<&Region> = ow.regions.iter().filter(|r| !r.kind.is_water()).collect();
                rs.sort_by(|a, b| b.size.cmp(&a.size));
                for r in rs.iter().take(12) {
                    let beasts: Vec<&str> = r.fauna.iter().map(|s| s.info().plural).collect();
                    println!("  {:<28} {:<15} {:>5} cells {:?}  wildlife: {}  plants: {}", r.name, r.kind.name(), r.size, r.biomes.iter().take(3).map(|b| b.0.name()).collect::<Vec<_>>(), beasts.join(", "), r.plants.join(", "));
                }
                for r in ow.rivers.iter().filter(|r| r.joins.is_none()).take(6) {
                    println!("  {} ({} cells, flow {}, {:?}, {} tributaries)", r.name, r.course.len(), r.flow, r.mouth, ow.rivers.iter().filter(|t| t.joins == Some(r.id)).count());
                }
            }
            let mut counts: Vec<(String, usize)> = Biome::ALL.iter().map(|b| (b.name().to_string(), ow.cells.iter().filter(|c| c.biome == *b).count())).collect();
            counts.retain(|c| c.1 > 0);
            println!("biomes: {:?}", counts);
        }
    }
}
