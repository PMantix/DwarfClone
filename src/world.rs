use crate::geom::{Pos, DIRS8};
use crate::items::ItemKind;
use crate::material::Mat;
use crate::noise::Noise;
use crate::overworld::EmbarkParams;
use crate::rng::Rng;
use serde::{Deserialize, Serialize};

pub const W: i32 = 96;
pub const H: i32 = 64;
pub const D: i32 = 32;

/// What occupies a cell. Cells with a floor (Floor, Ramp, stairs, Shrub) can be stood on;
/// Open cells have no floor, and Wall/Tree are solid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TileKind {
    Open,
    Floor,
    Wall,
    Ramp,
    StairUp,
    StairDown,
    StairUD,
    Tree,
    Shrub,
    /// Built wall with arrow slits: blocks walkers, lets water through.
    Fortification,
}

impl TileKind {
    pub fn solid(self) -> bool {
        matches!(self, TileKind::Wall | TileKind::Tree)
    }

    pub fn has_floor(self) -> bool {
        !matches!(self, TileKind::Open | TileKind::Wall | TileKind::Tree | TileKind::Fortification)
    }

    /// Bounds a room the way a wall does.
    pub fn encloses(self) -> bool {
        self.solid() || self == TileKind::Fortification
    }

    pub fn name(self) -> &'static str {
        match self {
            TileKind::Open => "open space",
            TileKind::Floor => "floor",
            TileKind::Wall => "wall",
            TileKind::Ramp => "ramp",
            TileKind::StairUp => "up stair",
            TileKind::StairDown => "down stair",
            TileKind::StairUD => "up/down stair",
            TileKind::Tree => "tree",
            TileKind::Shrub => "shrub",
            TileKind::Fortification => "fortification",
        }
    }
}

/// Who is moving: animals can't open doors, nobody passes a locked one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mover {
    Dwarf,
    Beast,
    Goblin,
}

pub const DOOR_NONE: u8 = 0;
pub const DOOR_OPEN: u8 = 1;
pub const DOOR_LOCKED: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Desig {
    None,
    Dig,
    Channel,
    StairUp,
    StairDown,
    StairUD,
    Ramp,
    Chop,
}

impl Desig {
    pub fn name(self) -> &'static str {
        match self {
            Desig::None => "remove designation",
            Desig::Dig => "mine",
            Desig::Channel => "channel",
            Desig::StairUp => "up stair",
            Desig::StairDown => "down stair",
            Desig::StairUD => "up/down stair",
            Desig::Ramp => "ramp",
            Desig::Chop => "chop trees",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Tile {
    pub kind: TileKind,
    pub mat: Mat,
    pub water: u8,
    pub revealed: bool,
    pub desig: Desig,
    /// Zone id (stockpile or farm), 0 = none.
    pub zone: u32,
    /// Building id, 0 = none.
    pub building: u32,
    /// Farm growth: 0 = fallow, otherwise ticks of growth.
    pub farm: u16,
    /// DOOR_NONE / DOOR_OPEN / DOOR_LOCKED.
    #[serde(default)]
    pub door: u8,
    /// What a construction here was built from (None = natural).
    #[serde(default)]
    pub built_from: Option<ItemKind>,
}

impl Default for Tile {
    fn default() -> Self {
        Tile {
            kind: TileKind::Open,
            mat: Mat::Air,
            water: 0,
            revealed: false,
            desig: Desig::None,
            zone: 0,
            building: 0,
            farm: 0,
            door: DOOR_NONE,
            built_from: None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct World {
    pub w: i32,
    pub h: i32,
    pub d: i32,
    pub tiles: Vec<Tile>,
    pub surface: Vec<i32>,
}

impl World {
    /// A one-cell stand-in used before a fortress exists.
    pub fn empty() -> World {
        World { w: 1, h: 1, d: 1, tiles: vec![Tile::default()], surface: vec![0] }
    }

    pub fn in_bounds(&self, p: Pos) -> bool {
        p.x >= 0 && p.y >= 0 && p.z >= 0 && p.x < self.w && p.y < self.h && p.z < self.d
    }

    pub fn idx(&self, p: Pos) -> usize {
        ((p.z * self.h + p.y) * self.w + p.x) as usize
    }

    pub fn pos_of(&self, i: usize) -> Pos {
        let i = i as i32;
        Pos::new(i % self.w, (i / self.w) % self.h, i / (self.w * self.h))
    }

    pub fn get(&self, p: Pos) -> Option<&Tile> {
        if self.in_bounds(p) {
            Some(&self.tiles[self.idx(p)])
        } else {
            None
        }
    }

    pub fn t(&self, p: Pos) -> &Tile {
        &self.tiles[self.idx(p)]
    }

    pub fn t_mut(&mut self, p: Pos) -> &mut Tile {
        let i = self.idx(p);
        &mut self.tiles[i]
    }

    pub fn kind(&self, p: Pos) -> Option<TileKind> {
        self.get(p).map(|t| t.kind)
    }

    /// Out-of-bounds counts as solid so nothing walks or flows off the map.
    pub fn solid(&self, p: Pos) -> bool {
        self.get(p).map_or(true, |t| t.kind.solid())
    }

    /// Something overhead: rock, or a floor or wall built above.
    pub fn roofed(&self, p: Pos) -> bool {
        (p.z + 1..self.d).any(|z| self.kind(Pos::new(p.x, p.y, z)).map_or(false, |k| k != TileKind::Open))
    }

    pub fn walkable(&self, p: Pos) -> bool {
        self.get(p)
            .map_or(false, |t| t.kind.has_floor() && t.water <= 3)
    }

    pub fn passable(&self, p: Pos, m: Mover) -> bool {
        match self.get(p) {
            Some(t) if t.kind.has_floor() && t.water <= 3 => match t.door {
                DOOR_LOCKED => false,
                DOOR_OPEN => m != Mover::Beast,
                _ => true,
            },
            _ => false,
        }
    }

    pub fn surface_z(&self, x: i32, y: i32) -> i32 {
        self.surface[(y * self.w + x) as usize]
    }

    pub fn outside(&self, p: Pos) -> bool {
        p.z >= self.surface_z(p.x, p.y)
    }

    /// Lowest position an object dropped at `p` comes to rest at.
    pub fn settle(&self, mut p: Pos) -> Pos {
        while p.z > 0 && !self.kind(p).map_or(true, |k| k.has_floor()) && !self.solid(p.down()) {
            p = p.down();
        }
        p
    }

    /// Legal single moves from `p`, with costs (10 = orthogonal step).
    pub fn neighbors(&self, p: Pos, out: &mut Vec<(Pos, u32)>, m: Mover) {
        out.clear();
        let here = match self.get(p) {
            Some(t) => t.kind,
            None => return,
        };
        for &(dx, dy) in DIRS8.iter() {
            let q = p.offset(dx, dy, 0);
            if self.passable(q, m) {
                // Diagonal steps are allowed even between two walls, as in DF.
                let diag = dx != 0 && dy != 0;
                out.push((q, if diag { 14 } else { 10 }));
            }
        }
        if matches!(here, TileKind::StairUp | TileKind::StairUD) {
            let q = p.up();
            if matches!(self.kind(q), Some(TileKind::StairDown | TileKind::StairUD)) && self.passable(q, m) {
                out.push((q, 12));
            }
        }
        if matches!(here, TileKind::StairDown | TileKind::StairUD) {
            let q = p.down();
            if matches!(self.kind(q), Some(TileKind::StairUp | TileKind::StairUD)) && self.passable(q, m) {
                out.push((q, 12));
            }
        }
        // Up a ramp: needs open space above the ramp and a wall to climb onto.
        if here == TileKind::Ramp && self.kind(p.up()) == Some(TileKind::Open) {
            for &(dx, dy) in DIRS8.iter() {
                let q = p.offset(dx, dy, 1);
                if self.solid(p.offset(dx, dy, 0)) && self.passable(q, m) {
                    out.push((q, 14));
                }
            }
        }
        // Down onto an adjacent ramp from the top of the wall it leans against.
        if self.solid(p.down()) {
            for &(dx, dy) in DIRS8.iter() {
                let r = p.offset(dx, dy, -1);
                if self.kind(r) == Some(TileKind::Ramp)
                    && self.kind(p.offset(dx, dy, 0)) == Some(TileKind::Open)
                    && self.passable(r, m)
                {
                    out.push((r, 14));
                }
            }
        }
    }

    /// Reveal the cells around a freshly dug position. Returns true if this
    /// broke into a previously unknown open area (e.g. a cavern).
    pub fn reveal_around(&mut self, p: Pos) -> bool {
        let mut seeds = Vec::new();
        let mut around: Vec<Pos> = DIRS8.iter().map(|&(dx, dy)| p.offset(dx, dy, 0)).collect();
        around.push(p);
        around.push(p.up());
        around.push(p.down());
        for q in around {
            if let Some(t) = self.get(q) {
                if !t.revealed {
                    let open = !t.kind.solid();
                    self.t_mut(q).revealed = true;
                    if open && q != p {
                        seeds.push(q);
                    }
                }
            }
        }
        let found = !seeds.is_empty();
        for s in seeds {
            self.flood_reveal(s);
        }
        found
    }

    /// Reveal a connected non-solid region plus the walls bounding it.
    pub fn flood_reveal(&mut self, start: Pos) {
        let mut stack = vec![start];
        let mut seen = std::collections::HashSet::new();
        seen.insert(start);
        while let Some(p) = stack.pop() {
            self.t_mut(p).revealed = true;
            for dz in -1..=1 {
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let q = p.offset(dx, dy, dz);
                        if !self.in_bounds(q) {
                            continue;
                        }
                        if self.t(q).kind.solid() {
                            if dz == 0 {
                                self.t_mut(q).revealed = true;
                            }
                        } else if seen.insert(q) && seen.len() < 60_000 {
                            stack.push(q);
                        }
                    }
                }
            }
        }
    }
}

/// Generate a fresh embark region.
pub fn generate(seed: u64, params: &EmbarkParams) -> World {
    let (w, h, d) = (W, H, D);
    let height_n = Noise::new(seed);
    let soil_n = Noise::new(seed ^ 0x1111);
    let rock_n = Noise::new(seed ^ 0x2222);
    let cave_n = Noise::new(seed ^ 0x3333);
    let ore_n = Noise::new(seed ^ 0x4444);
    let forest_n = Noise::new(seed ^ 0x5555);
    let pool_n = Noise::new(seed ^ 0x6666);
    let mut rng = Rng::new(seed ^ 0x7777);

    // Height map, then relaxed so neighbours never differ by more than one level
    // (every slope gets a ramp and the whole surface is walkable).
    let mut surf = vec![0i32; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let v = height_n.fbm2(x as f64 / 34.0, y as f64 / 34.0, 4);
            surf[(y * w + x) as usize] = (18.0 + (v - 0.5) * 18.0 * params.relief).round().clamp(9.0, 27.0) as i32;
        }
    }
    loop {
        let mut changed = false;
        for y in 0..h {
            for x in 0..w {
                for &(dx, dy) in DIRS8.iter() {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= w || ny >= h {
                        continue;
                    }
                    let n = surf[(ny * w + nx) as usize];
                    let c = &mut surf[(y * w + x) as usize];
                    if *c > n + 1 {
                        *c = n + 1;
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }

    let mut sorted = surf.clone();
    sorted.sort_unstable();
    let water_level = if params.lake <= 0.0 {
        0
    } else {
        sorted[((sorted.len() as f64 * params.lake) as usize).min(sorted.len() - 1)]
    };

    let mut world = World {
        w,
        h,
        d,
        tiles: vec![Tile::default(); (w * h * d) as usize],
        surface: surf.clone(),
    };

    for y in 0..h {
        for x in 0..w {
            let hh = surf[(y * w + x) as usize];
            for z in 0..d {
                let p = Pos::new(x, y, z);
                let t = world.t_mut(p);
                if z == 0 {
                    t.kind = TileKind::Wall;
                    t.mat = Mat::Bedrock;
                } else if z < hh {
                    t.kind = TileKind::Wall;
                    let depth = hh - z;
                    t.mat = if depth <= 3 {
                        let v = soil_n.fbm2(x as f64 / 14.0, y as f64 / 14.0, 2);
                        if v < 0.42 {
                            Mat::Loam
                        } else if v < 0.62 {
                            Mat::Clay
                        } else {
                            Mat::Sand
                        }
                    } else if z >= 13 {
                        if rock_n.fbm3(x as f64 / 20.0, y as f64 / 20.0, z as f64 / 3.0, 2) < 0.5 {
                            Mat::Sandstone
                        } else {
                            Mat::Limestone
                        }
                    } else if z >= 4 {
                        if rock_n.fbm3(x as f64 / 9.0, y as f64 / 9.0, z as f64 / 2.0, 2) > 0.66 {
                            Mat::Marble
                        } else {
                            Mat::Granite
                        }
                    } else {
                        Mat::Basalt
                    };
                } else if z == hh {
                    t.kind = TileKind::Floor;
                    t.mat = if hh < water_level { Mat::Sand } else { Mat::Grass };
                }
            }
        }
    }

    // Natural ramps on every slope.
    for y in 0..h {
        for x in 0..w {
            let hh = surf[(y * w + x) as usize];
            let higher = DIRS8.iter().any(|&(dx, dy)| {
                let (nx, ny) = (x + dx, y + dy);
                nx >= 0 && ny >= 0 && nx < w && ny < h && surf[(ny * w + nx) as usize] == hh + 1
            });
            if higher {
                world.t_mut(Pos::new(x, y, hh)).kind = TileKind::Ramp;
            }
        }
    }

    // Ore veins and gem clusters inside the rock layers.
    for z in 1..d {
        for y in 0..h {
            for x in 0..w {
                let p = Pos::new(x, y, z);
                let t = *world.t(p);
                if t.kind != TileKind::Wall || !t.mat.is_building_stone() {
                    continue;
                }
                let v = ore_n.fbm3(x as f64 / 7.0, y as f64 / 7.0, z as f64 / 3.0, 2);
                if (v - 0.5).abs() < 0.016 {
                    world.t_mut(p).mat = match t.mat {
                        Mat::Sandstone | Mat::Limestone => {
                            if (x + z) % 3 == 0 { Mat::Coal } else { Mat::Hematite }
                        }
                        Mat::Granite | Mat::Marble => {
                            if v > 0.5 { Mat::Malachite } else { Mat::NativeGold }
                        }
                        _ => Mat::Hematite,
                    };
                } else if rng.chance(0.0015) {
                    world.t_mut(p).mat = Mat::Amethyst;
                }
            }
        }
    }

    // A cavern layer deep underground.
    for z in 5..=8 {
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let hh = surf[(y * w + x) as usize];
                if z >= hh - 4 {
                    continue;
                }
                let v = cave_n.fbm3(x as f64 / 11.0, y as f64 / 11.0, z as f64 / 3.5, 3);
                if v > 0.6 {
                    let t = world.t_mut(Pos::new(x, y, z));
                    t.kind = TileKind::Open;
                    t.mat = Mat::Air;
                }
            }
        }
    }
    for z in 5..=8 {
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let p = Pos::new(x, y, z);
                if world.t(p).kind != TileKind::Open || !world.solid(p.down()) {
                    continue;
                }
                let floor_mat = world.t(p.down()).mat;
                let t = world.t_mut(p);
                t.kind = TileKind::Floor;
                t.mat = floor_mat;
            }
        }
    }
    // Cavern ramps where the cavern floor steps up by one level.
    for z in 5..=8 {
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let p = Pos::new(x, y, z);
                if world.t(p).kind != TileKind::Floor || world.kind(p.up()) != Some(TileKind::Open) {
                    continue;
                }
                let step = DIRS8.iter().any(|&(dx, dy)| {
                    world.solid(p.offset(dx, dy, 0)) && world.walkable(p.offset(dx, dy, 1))
                });
                if step {
                    world.t_mut(p).kind = TileKind::Ramp;
                }
            }
        }
    }
    // Cavern pools and towering mushroom trees.
    for z in 5..=8 {
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let p = Pos::new(x, y, z);
                if world.t(p).kind != TileKind::Floor {
                    continue;
                }
                if pool_n.fbm2(x as f64 / 6.0, y as f64 / 6.0 + z as f64 * 10.0, 2) > 0.68 {
                    world.t_mut(p).water = 7;
                } else if rng.chance(0.07) {
                    let t = world.t_mut(p);
                    t.kind = TileKind::Tree;
                    t.mat = Mat::Wood;
                } else if rng.chance(0.05) {
                    world.t_mut(p).kind = TileKind::Shrub;
                }
            }
        }
    }

    // Surface lakes, forests and shrubs.
    for y in 0..h {
        for x in 0..w {
            let hh = surf[(y * w + x) as usize];
            if hh < water_level {
                for z in hh..water_level {
                    world.t_mut(Pos::new(x, y, z)).water = 7;
                }
                continue;
            }
            let p = Pos::new(x, y, hh);
            if world.t(p).kind != TileKind::Floor {
                continue;
            }
            let forest = forest_n.fbm2(x as f64 / 16.0, y as f64 / 16.0, 3);
            if forest > 0.55 && rng.chance((0.30 * params.forest).min(0.8)) || rng.chance(0.015 * params.forest) {
                let t = world.t_mut(p);
                t.kind = TileKind::Tree;
                t.mat = Mat::Wood;
            } else if rng.chance(0.05) {
                world.t_mut(p).kind = TileKind::Shrub;
            }
        }
    }

    // Everything under the open sky is known, plus the cliff faces beside it.
    for y in 0..h {
        for x in 0..w {
            let hh = surf[(y * w + x) as usize];
            for z in hh..d {
                world.t_mut(Pos::new(x, y, z)).revealed = true;
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            let hh = surf[(y * w + x) as usize];
            for z in hh..d {
                for &(dx, dy) in DIRS8.iter() {
                    let q = Pos::new(x + dx, y + dy, z);
                    if world.in_bounds(q) {
                        world.t_mut(q).revealed = true;
                    }
                }
            }
        }
    }

    world
}
