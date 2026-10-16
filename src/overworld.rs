//! The wider world: biomes, civilizations, sites, historical figures and
//! a couple of centuries of generated history.

use crate::noise::Noise;
use crate::rng::Rng;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};


#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
}

/// How a biome shapes the embark region.
#[derive(Clone, Copy, Debug)]
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
}

impl Biome {
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
        }
    }

    /// Whether winter (or the whole year) is cold enough for ice and snow.
    pub fn cold(self) -> bool {
        matches!(self, Biome::Tundra | Biome::Mountain | Biome::Taiga | Biome::Glacier)
    }

    pub fn glyph(self) -> (char, Color) {
        match self {
            Biome::Ocean => ('≈', Color::Rgb(40, 90, 200)),
            Biome::Mountain => ('▲', Color::Rgb(170, 170, 180)),
            Biome::Hills => ('∩', Color::Rgb(150, 120, 80)),
            Biome::Forest => ('♣', Color::Rgb(40, 160, 60)),
            Biome::Grassland => ('"', Color::Rgb(120, 200, 90)),
            Biome::Desert => ('~', Color::Rgb(220, 200, 110)),
            Biome::Tundra => ('·', Color::Rgb(220, 230, 240)),
            Biome::Swamp => ('%', Color::Rgb(70, 150, 130)),
            Biome::Jungle => ('♠', Color::Rgb(30, 200, 70)),
            Biome::Taiga => ('↑', Color::Rgb(60, 130, 100)),
            Biome::Savanna => ('"', Color::Rgb(190, 190, 80)),
            Biome::Badlands => ('∩', Color::Rgb(200, 110, 60)),
            Biome::Glacier => ('░', Color::Rgb(200, 230, 255)),
        }
    }

    pub fn params(self) -> EmbarkParams {
        let p = |relief, forest, lake, savagery| EmbarkParams { relief, forest, lake, savagery, minerals: 1.0, size: crate::world::MapSize::MEDIUM, aquifer: 0, features: 0 };
        match self {
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
        }
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
            Race::Human => matches!(b, Biome::Grassland | Biome::Forest | Biome::Hills | Biome::Savanna),
            Race::Elf => matches!(b, Biome::Forest | Biome::Jungle | Biome::Taiga) && !evil,
            Race::Goblin => evil || matches!(b, Biome::Tundra | Biome::Mountain | Biome::Glacier),
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
    elev: Noise,
    ridge: Noise,
    rain: Noise,
    heat: Noise,
    evil: Noise,
    row: i32,
    stage: GenStage,
    /// Raises or lowers the land so about the same share of every world is sea.
    sea_shift: f64,
}

impl WorldGen {
    pub fn new(params: WorldParams) -> WorldGen {
        let seed = params.seed;
        let mut rng = Rng::new(seed ^ 0xBADC0DE);
        let (w, h) = params.dims();
        // The terrain uses only noise, so naming the world first draws the
        // same random numbers as naming it afterwards.
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
        };
        WorldGen {
            params,
            ow,
            rng,
            elev: Noise::new(seed ^ 0xE1E7),
            ridge: Noise::new(seed ^ 0x81D6E),
            rain: Noise::new(seed ^ 0x7A1),
            heat: Noise::new(seed ^ 0x4EA7),
            evil: Noise::new(seed ^ 0xE711),
            row: 0,
            stage: GenStage::Terrain,
            sea_shift: 0.0,
        }
        .with_sea_level()
    }

    /// Sample the land coarsely and set the sea so about two-fifths is water.
    fn with_sea_level(mut self) -> WorldGen {
        let (w, h) = (self.ow.w, self.ow.h);
        let mut hs: Vec<f64> = (0..h).step_by(3).flat_map(|y| (0..w).step_by(3).map(move |x| (x, y))).map(|(x, y)| self.height(x, y)).collect();
        hs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        self.sea_shift = hs[hs.len() * 2 / 5] - 0.36;
        self
    }

    pub fn done(&self) -> bool {
        self.stage == GenStage::Done
    }

    /// Rows of terrain generated so far (the rest of the map is still blank).
    pub fn rows_done(&self) -> i32 {
        self.row
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
                    self.stage = GenStage::Civs;
                }
            }
            GenStage::Civs => {
                self.found_civs();
                self.stage = GenStage::Beasts;
            }
            GenStage::Beasts => {
                self.place_beasts();
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
                }
                if self.ow.year >= self.params.history {
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
        self.ow
    }

    /// Height at a cell, about 0..1: rolling land, lower toward the edges,
    /// with long ridges of mountains running through it.
    fn height(&self, x: i32, y: i32) -> f64 {
        let (w, h) = (self.ow.w, self.ow.h);
        // A cell is twice as tall as it is wide.
        let (fx, fy) = (x as f64 / 24.0, y as f64 / 12.0);
        let edge = {
            let dx = (x as f64 / w as f64 - 0.5).abs() * 2.0;
            let dy = (y as f64 / h as f64 - 0.5).abs() * 2.0;
            dx.max(dy).powi(3) * 0.35
        };
        // Bend the land a little so coasts aren't blobs.
        let warp = (self.heat.fbm2(fx * 0.7 + 9.1, fy * 0.7, 2) - 0.5) * 1.6;
        let e = self.elev.fbm2(fx + warp, fy - warp * 0.5, 5) - edge;
        // Ridges: where the ridge noise crosses its middle.
        let rn = self.ridge.fbm2(fx * 0.9 + warp * 0.6, fy * 0.9, 4);
        let ridge = (1.0 - (rn - 0.5).abs() * 9.0).max(0.0);
        let inland = ((e - self.sea_shift - 0.38) * 8.0).clamp(0.0, 1.0);
        e + ridge * 0.24 * inland - self.sea_shift
    }

    fn terrain_row(&mut self, y: i32) {
        let (w, h) = (self.ow.w, self.ow.h);
        for x in 0..w {
            let (fx, fy) = (x as f64 / 24.0, y as f64 / 12.0);
            let e = self.height(x, y);
            let r = self.rain.fbm2(fx * 1.2, fy * 1.2, 3) + (self.rain.fbm2(fx * 4.0, fy * 4.0, 2) - 0.5) * 0.25;
            // Cold toward both poles, and up in the heights.
            let lat = (y as f64 / h as f64 - 0.5).abs() * 2.0;
            let t = 1.0 - lat * 0.95 + (self.heat.fbm2(fx, fy, 3) - 0.5) * 0.5 - (e - 0.6).max(0.0) * 1.5;
            let biome = if e < 0.36 {
                Biome::Ocean
            } else if t < 0.08 {
                Biome::Glacier
            } else if e > 0.66 {
                Biome::Mountain
            } else if e > 0.58 {
                if r < 0.42 && t > 0.55 { Biome::Badlands } else { Biome::Hills }
            } else if t < 0.26 {
                if r > 0.5 { Biome::Taiga } else { Biome::Tundra }
            } else if t < 0.38 && r > 0.47 {
                Biome::Taiga
            } else if r < 0.37 && t > 0.55 {
                if r > 0.34 && t > 0.62 { Biome::Badlands } else { Biome::Desert }
            } else if t > 0.66 && r > 0.55 {
                if r > 0.66 { Biome::Swamp } else { Biome::Jungle }
            } else if r > 0.65 {
                Biome::Swamp
            } else if r > 0.5 {
                Biome::Forest
            } else if t > 0.62 && r < 0.47 {
                Biome::Savanna
            } else {
                Biome::Grassland
            };
            let ev = biome != Biome::Ocean && self.evil.fbm2(fx * 1.6, fy * 1.6, 2) > 0.66;
            let elev = (e * 255.0).clamp(0.0, 255.0) as u8;
            self.ow.cells.push(Cell { biome, evil: ev, site: 0, region: 0, elev, river: 0, volcano: false });
        }
    }

    /// Rivers run from the wet heights down to the sea (or pool into lakes),
    /// and a few mountains smoulder. Its own random stream, so the history
    /// that follows is the same with or without them.
    fn rivers_and_fire(&mut self) {
        let (w, h) = (self.ow.w, self.ow.h);
        let mut rng = Rng::new(self.params.seed ^ 0x51FE5);
        let hgt: Vec<f64> = (0..w * h).map(|i| self.ow.cells[i as usize].elev as f64 + rng.range(0, 100) as f64 * 0.01).collect();
        let idx = |x: i32, y: i32| (y * w + x) as usize;
        let dirs = [(0, -1, 1u8, 4u8), (1, 0, 2, 8), (0, 1, 4, 1), (-1, 0, 8, 2)];
        let sources = (w * h / 160) as usize;
        for _ in 0..sources * 4 {
            let (x, y) = (rng.range(1, w - 1), rng.range(1, h - 1));
            let c = &self.ow.cells[idx(x, y)];
            if c.biome == Biome::Ocean || c.river != 0 || c.elev < 150 || matches!(c.biome, Biome::Desert | Biome::Badlands | Biome::Glacier) {
                continue;
            }
            let mut path = vec![(x, y)];
            let mut seen = std::collections::HashSet::new();
            seen.insert((x, y));
            let mut end = None;
            for _ in 0..300 {
                let (cx, cy) = *path.last().unwrap();
                let here = hgt[idx(cx, cy)];
                // The lowest neighbour not yet on this river.
                let next = dirs
                    .iter()
                    .map(|&(dx, dy, _, _)| (cx + dx, cy + dy))
                    .filter(|&(nx, ny)| nx >= 0 && ny >= 0 && nx < w && ny < h && !seen.contains(&(nx, ny)))
                    .min_by(|a, b| hgt[idx(a.0, a.1)].partial_cmp(&hgt[idx(b.0, b.1)]).unwrap());
                let Some((nx, ny)) = next else { break };
                if hgt[idx(nx, ny)] > here + 6.0 {
                    // A hollow with no way out: it fills into a lake.
                    end = Some(false);
                    break;
                }
                let nc = &self.ow.cells[idx(nx, ny)];
                path.push((nx, ny));
                seen.insert((nx, ny));
                if nc.biome == Biome::Ocean || nc.river != 0 {
                    end = Some(true);
                    break;
                }
            }
            if path.len() < 6 || end.is_none() {
                continue;
            }
            for i in 0..path.len() - 1 {
                let ((ax, ay), (bx, by)) = (path[i], path[i + 1]);
                let &(_, _, out, inn) = dirs.iter().find(|d| (d.0, d.1) == (bx - ax, by - ay)).unwrap();
                self.ow.cells[idx(ax, ay)].river |= out;
                if self.ow.cells[idx(bx, by)].biome != Biome::Ocean {
                    self.ow.cells[idx(bx, by)].river |= inn;
                }
            }
            if end == Some(false) {
                let &(lx, ly) = path.last().unwrap();
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1), (-1, 0)] {
                    let (px, py) = (lx + dx, ly + dy);
                    let inside = px > 0 && py > 0 && px < w - 1 && py < h - 1;
                    if inside && ((dx, dy) == (0, 0) || rng.chance(0.5)) {
                        let c = &mut self.ow.cells[idx(px, py)];
                        if c.biome != Biome::Mountain {
                            c.biome = Biome::Ocean;
                            c.river = 0;
                            c.evil = false;
                        }
                    }
                }
            }
        }
        // Drop links that point at a cell that is no longer river or water.
        for y in 0..h {
            for x in 0..w {
                let r = self.ow.cells[idx(x, y)].river;
                if r == 0 {
                    continue;
                }
                let mut keep = 0;
                for &(dx, dy, bit, back) in &dirs {
                    let (nx, ny) = (x + dx, y + dy);
                    if r & bit != 0 && nx >= 0 && ny >= 0 && nx < w && ny < h {
                        let n = &self.ow.cells[idx(nx, ny)];
                        if n.biome == Biome::Ocean || n.river & back != 0 {
                            keep |= bit;
                        }
                    }
                }
                self.ow.cells[idx(x, y)].river = if self.ow.cells[idx(x, y)].biome == Biome::Ocean { 0 } else { keep };
            }
        }
        // Volcanoes on the high peaks.
        let peaks: Vec<usize> = (0..(w * h) as usize).filter(|&i| self.ow.cells[i].biome == Biome::Mountain && self.ow.cells[i].elev > 175 && self.ow.cells[i].river == 0).collect();
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

    /// Flood-fill stretches of similar land or water and name the bigger ones.
    /// Naming has its own random stream so history comes out the same.
    fn name_regions(&mut self) {
        let (w, h) = (self.ow.w, self.ow.h);
        let mut rng = Rng::new(self.params.seed ^ 0x2E610);
        let class = |b: Biome| match b {
            Biome::Ocean => 0,
            Biome::Mountain => 1,
            Biome::Hills => 2,
            Biome::Forest => 3,
            Biome::Grassland => 4,
            Biome::Desert => 5,
            Biome::Tundra => 6,
            Biome::Swamp => 7,
            Biome::Jungle => 8,
            Biome::Taiga => 9,
            Biome::Savanna => 10,
            Biome::Badlands => 11,
            Biome::Glacier => 12,
        };
        let mut seen = vec![false; (w * h) as usize];
        for start in 0..(w * h) as usize {
            if seen[start] {
                continue;
            }
            let k = class(self.ow.cells[start].biome);
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
                    if !seen[j] && class(self.ow.cells[j].biome) == k {
                        seen[j] = true;
                        cells.push(j);
                        stack.push(j);
                    }
                }
            }
            if cells.len() < 3 {
                continue;
            }
            let kind = match self.ow.cells[start].biome {
                Biome::Ocean if edge && cells.len() > 400 => RegionKind::Ocean,
                Biome::Ocean if edge => RegionKind::Sea,
                Biome::Ocean => RegionKind::Lake,
                Biome::Mountain => RegionKind::Mountains,
                Biome::Hills => RegionKind::Hills,
                Biome::Forest => RegionKind::Forest,
                Biome::Grassland => RegionKind::Plains,
                Biome::Desert => RegionKind::Desert,
                Biome::Tundra => RegionKind::Tundra,
                Biome::Swamp => RegionKind::Swamp,
                Biome::Jungle => RegionKind::Jungle,
                Biome::Taiga => RegionKind::Taiga,
                Biome::Savanna => RegionKind::Savanna,
                Biome::Badlands => RegionKind::Badlands,
                Biome::Glacier => RegionKind::Glacier,
            };
            let id = self.ow.regions.len() as u32;
            // Names are unique; after a few tries fall back to a numbered one.
            let mut name = region_name(&mut rng, kind);
            for attempt in 0..12 {
                if !self.ow.regions.iter().any(|r| r.name == name) {
                    break;
                }
                name = if attempt < 11 { region_name(&mut rng, kind) } else { format!("{} {}", name, id + 1) };
            }
            let mid = cells[cells.len() / 2];
            self.ow.regions.push(Region { id, kind, name, size: cells.len() as u32, x: mid as i32 % w, y: mid as i32 / w });
            for i in cells {
                self.ow.cells[i].region = id + 1;
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
                if c.site == 0 && matches!(c.biome, Biome::Mountain | Biome::Swamp | Biome::Desert | Biome::Tundra | Biome::Badlands | Biome::Glacier | Biome::Jungle) {
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

    /// The site's features: its own roll, plus what the world map shows
    /// (a river through the cell, a volcano on it).
    pub fn site_features(&self, seed: u64, x: i32, y: i32) -> u32 {
        let c = self.cell(x, y);
        let mut bits = crate::terrain::site_features(seed, x, y, c.biome);
        if c.river != 0 {
            bits |= crate::terrain::feat::RIVER;
        }
        if c.volcano {
            bits |= crate::terrain::feat::VOLCANO;
        }
        bits
    }

    /// How a cell looks on the world map: biome, height and a little
    /// variety from cell to cell, with rivers and volcanoes drawn in.
    pub fn look(&self, x: i32, y: i32) -> (char, Color) {
        let c = self.cell(x, y);
        let mut hsh = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
        hsh ^= hsh >> 31;
        hsh = hsh.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        hsh ^= hsh >> 29;
        let roll = (hsh % 1000) as u32;
        let pick = |opts: &[char]| opts[(hsh >> 12) as usize % opts.len()];
        let e = c.elev as i32;
        let shade = |(r, g, b): (u8, u8, u8), k: i32| {
            // k in percent; plus a flicker from cell to cell.
            let k = k + (roll % 13) as i32 - 6;
            let f = |v: u8| (v as i32 * k / 100).clamp(0, 255) as u8;
            Color::Rgb(f(r), f(g), f(b))
        };
        if c.volcano {
            return ('▲', shade((240, 80, 40), 100));
        }
        if c.river != 0 {
            let ch = match c.river {
                1 | 4 | 5 => '║',
                2 | 8 | 10 => '═',
                3 => '╚',
                9 => '╝',
                6 => '╔',
                12 => '╗',
                7 => '╠',
                13 => '╣',
                11 => '╩',
                14 => '╦',
                _ => '╬',
            };
            return (ch, shade((80, 140, 240), 100));
        }
        let near = |pred: &dyn Fn(&Cell) -> bool| {
            [(0, -1), (1, 0), (0, 1), (-1, 0)].iter().any(|&(dx, dy)| {
                let (nx, ny) = (x + dx, y + dy);
                // While the world is being made, rows below aren't there yet.
                nx >= 0 && ny >= 0 && nx < self.w && ((ny * self.w + nx) as usize) < self.cells.len() && pred(self.cell(nx, ny))
            })
        };
        match c.biome {
            Biome::Ocean => {
                if near(&|n: &Cell| n.biome != Biome::Ocean) {
                    (pick(&['≈', '~']), shade((70, 130, 220), 100))
                } else if e > 75 {
                    (pick(&['≈', '~', '≈']), shade((40, 85, 190), 100))
                } else {
                    (pick(&['≈', '~', '≈']), shade((25, 55, 150), 100))
                }
            }
            Biome::Mountain => {
                if e > 205 {
                    ('▲', shade((235, 235, 245), 100))
                } else if e > 185 {
                    (pick(&['▲', '▲', '^']), shade((175, 175, 185), 100))
                } else {
                    (pick(&['▲', '^', '^']), shade((140, 130, 120), 100))
                }
            }
            Biome::Hills => (pick(&['∩', 'n', '∩']), shade((150, 125, 80), 85 + (e - 150) / 2)),
            Biome::Forest => (pick(&['♣', '♣', '♠', '♣', '"']), shade((40, 160, 60), 95 - (roll % 3) as i32 * 10)),
            Biome::Grassland => (pick(&['"', ',', '\'', '"', '.', '`']), shade((110, 195, 85), 100)),
            Biome::Desert => (pick(&['~', '≈', '.', '~', ',']), shade((225, 200, 115), 100)),
            Biome::Tundra => (pick(&['·', '.', ',', '·', '"']), shade((200, 215, 220), 100)),
            Biome::Swamp => (pick(&['%', '"', '%', '≈', '♣']), shade((70, 150, 125), 100)),
            Biome::Jungle => (pick(&['♠', '♣', '♠', '¶', '♠']), shade((30, 190, 70), 100)),
            Biome::Taiga => (pick(&['↑', '↑', '♠', '↑']), shade((60, 140, 105), 100)),
            Biome::Savanna => {
                if roll % 9 == 0 {
                    ('♣', shade((150, 160, 60), 100))
                } else {
                    (pick(&['"', ',', '"', '.']), shade((200, 190, 85), 100))
                }
            }
            Biome::Badlands => (pick(&['∩', 'n', '.', '∩', '¬']), shade((205, 115, 65), 100)),
            Biome::Glacier => (pick(&['░', '▒', '░', '·']), shade((205, 230, 250), 100)),
        }
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

    /// The named region a cell lies in, if any.
    pub fn region_at(&self, x: i32, y: i32) -> Option<&Region> {
        match self.cell(x, y).region {
            0 => None,
            r => self.regions.get(r as usize - 1),
        }
    }

    pub fn embarkable(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h && self.cell(x, y).biome != Biome::Ocean && self.cell(x, y).site == 0
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
            if c.site == 0 && c.biome != Biome::Ocean && race.home(c.biome, c.evil) {
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
            cells: vec![Cell { biome: Biome::Ocean, evil: false, site: 0, region: 0, elev: 0, river: 0, volcano: false }],
            civs: Vec::new(),
            sites: Vec::new(),
            figures: Vec::new(),
            events: Vec::new(),
            year: 0,
            name: String::new(),
            savagery: 1.0,
            minerals: 1.0,
            regions: Vec::new(),
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
                self.log(year, format!("{} slew {} the {} at {}.", hname, bname, bkind, sname));
            } else {
                self.figures[f as usize].kills += rng.range(5, 40) as u32;
                let protected = self.civs.iter().any(|c| c.race == Race::Dwarf && c.capital == t);
                let s = &mut self.sites[t as usize];
                s.pop = s.pop * 2 / 3;
                if !protected && (s.pop < 40 || rng.chance(0.3)) {
                    s.ruined = true;
                    self.log(year, format!("{} the {} laid waste to {}.", bname, bkind, sname));
                    self.check_extinct(year);
                } else {
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
            self.log(year, format!("{}: the {} of the {} broke upon the gates of the mountainhome.", label, ar.plural(), an));
        } else if a_pow > d_pow {
            let t = &mut self.sites[target as usize];
            t.pop = t.pop / 3;
            if ar == Race::Goblin && rng.chance(0.5) {
                t.civ = Some(att);
                t.kind = SiteKind::DarkFortress;
                self.log(year, format!("{}: the {} of the {} conquered {}.", label, ar.plural(), an, tname));
            } else {
                t.ruined = true;
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
            self.log(year, format!("{}: the {} of the {} were repulsed.", label, ar.plural(), an));
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
                // A river makes a kind first fortress; the frozen far north and south don't.
                if c.river != 0 {
                    score += 15;
                }
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
}
