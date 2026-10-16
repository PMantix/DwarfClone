use crate::geom::{Pos, DIRS8};
use crate::items::ItemKind;
use crate::material::Mat;

use crate::overworld::EmbarkParams;

use serde::{Deserialize, Serialize};

/// Fortress map dimensions, chosen at embark.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapSize {
    pub w: i32,
    pub h: i32,
    pub d: i32,
}

impl MapSize {
    pub const ALL: [MapSize; 3] = [
        MapSize { w: 64, h: 48, d: 32 },
        MapSize { w: 96, h: 64, d: 32 },
        MapSize { w: 144, h: 96, d: 48 },
    ];
    pub const NAMES: [&'static str; 3] = ["Small", "Medium", "Large"];
    pub const MEDIUM: MapSize = MapSize::ALL[1];
}

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
/// A raised drawbridge: nobody passes and it can't be battered down.
pub const BRIDGE_RAISED: u8 = 3;
pub const DOOR_LOCKED: u8 = 2;
/// A shut floodgate: holds back walkers and water alike.
pub const FLOODGATE_SHUT: u8 = 4;

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
    /// Smooth rough natural stone (walls or floors).
    Smooth,
    /// Engrave smoothed stone.
    Engrave,
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
            Desig::Smooth => "smooth stone",
            Desig::Engrave => "engrave",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
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
    /// DOOR_NONE / DOOR_OPEN / DOOR_LOCKED / BRIDGE_RAISED / FLOODGATE_SHUT.
    #[serde(default)]
    pub door: u8,
    /// What a construction here was built from (None = natural).
    #[serde(default)]
    pub built_from: Option<ItemKind>,
    /// One bit per burrow this tile belongs to.
    #[serde(default)]
    pub burrows: u8,
    /// 0 normal, 1 high, 2 low, 3 restricted (dwarves only).
    #[serde(default)]
    pub traffic: u8,
    /// Depth of magma, 0-7, like `water`.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub magma: u8,
    /// Water-bearing rock: seeps into anything dug beside it.
    #[serde(default, skip_serializing_if = "is_false")]
    pub aquifer: bool,
    /// What's planted in a farm plot (None in older saves means stoutcap).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crop: Option<Mat>,
    /// 0 rough, 1 smoothed, 2+ engraved (2 + quality).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub smooth: u8,
    /// The legend an engraving shows (1-based event index; 0 none).
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub engraving: u32,
    /// The water here has frozen: it holds still and can be walked on.
    #[serde(default, skip_serializing_if = "is_false")]
    pub frozen: bool,
    /// Snow lies on the ground here.
    #[serde(default, skip_serializing_if = "is_false")]
    pub snow: bool,
    /// Burning: how much longer the fire here lasts (0 none).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub fire: u8,
    /// Miasma from rotting bodies, 0..7.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub miasma: u8,
    /// Dug out by dwarves (natural caverns aren't), so its ceiling needs support.
    #[serde(default, skip_serializing_if = "is_false")]
    pub dug: bool,
}

fn is_zero(v: &u8) -> bool {
    *v == 0
}

fn is_zero_u32(v: &u32) -> bool {
    *v == 0
}

fn is_false(v: &bool) -> bool {
    !*v
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
            burrows: 0,
            traffic: 0,
            magma: 0,
            aquifer: false,
            crop: None,
            smooth: 0,
            engraving: 0,
            frozen: false,
            snow: false,
            fire: 0,
            dug: false,
            miasma: 0,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct World {
    pub w: i32,
    pub h: i32,
    pub d: i32,
    /// Saved run-length encoded; plain arrays from older saves still load.
    #[serde(with = "tiles_rle")]
    pub tiles: Vec<Tile>,
    pub surface: Vec<i32>,
    /// Cavern layers as (lowest z, highest z), deepest first.
    #[serde(default = "default_caverns")]
    pub caverns: Vec<(i32, i32)>,
    /// Burrow bits the dwarf currently moving is kept to (0 = anywhere).
    #[serde(skip)]
    pub restrict: u8,
    /// 0 none, 1 light, 2 heavy.
    #[serde(default)]
    pub aquifer: u8,
    /// Where tile (0, 0) sits in the land's own coordinates (moves when the
    /// map grows north or west).
    #[serde(default)]
    pub origin: (i32, i32),
    /// The size the map was first made (features are laid out over it).
    #[serde(default)]
    pub base: Option<(i32, i32)>,
    /// The level lakes fill to.
    #[serde(default)]
    pub water_level: i32,
    /// The site's features (`terrain::feat` bits).
    #[serde(default)]
    pub features: u32,
    /// The seed the land was made from (0 in older saves).
    #[serde(default)]
    pub gen_seed: u64,
}

fn default_caverns() -> Vec<(i32, i32)> {
    vec![(5, 8)]
}

/// Most of a map is long runs of identical rock, so saves store (count, tile) pairs.
mod tiles_rle {
    use super::Tile;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize)]
    struct Out<'a> {
        rle: Vec<(u32, &'a Tile)>,
    }

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum In {
        Rle { rle: Vec<(u32, Tile)> },
        Plain(Vec<Tile>),
    }

    pub fn serialize<S: Serializer>(tiles: &[Tile], s: S) -> Result<S::Ok, S::Error> {
        let mut rle: Vec<(u32, &Tile)> = Vec::new();
        for t in tiles {
            match rle.last_mut() {
                Some((n, last)) if *last == t => *n += 1,
                _ => rle.push((1, t)),
            }
        }
        Out { rle }.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Tile>, D::Error> {
        Ok(match In::deserialize(d)? {
            In::Plain(v) => v,
            In::Rle { rle } => {
                let mut v = Vec::with_capacity(rle.iter().map(|(n, _)| *n as usize).sum());
                for (n, t) in rle {
                    v.extend(std::iter::repeat(t).take(n as usize));
                }
                v
            }
        })
    }
}

impl World {
    /// A one-cell stand-in used before a fortress exists.
    pub fn empty() -> World {
        World { w: 1, h: 1, d: 1, tiles: vec![Tile::default()], surface: vec![0], caverns: Vec::new(), restrict: 0, aquifer: 0, origin: (0, 0), base: None, water_level: 0, features: 0, gen_seed: 0 }
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

    /// Room value a tile adds for its detailing: smoothed 2, engraved 6 plus
    /// 3 per quality level.
    pub fn detail_value(&self, p: Pos) -> u32 {
        match self.get(p).map_or(0, |t| t.smooth) {
            0 => 0,
            1 => 2,
            s => 6 + 3 * (s - 2) as u32,
        }
    }

    /// Water can't enter: rock, a door, a raised bridge or a shut floodgate.
    pub fn blocks_water(&self, p: Pos) -> bool {
        self.get(p).map_or(true, |t| t.kind.solid() || t.door != DOOR_NONE || t.frozen)
    }

    /// Something overhead: rock, or a floor or wall built above.
    pub fn roofed(&self, p: Pos) -> bool {
        (p.z + 1..self.d).any(|z| self.kind(Pos::new(p.x, p.y, z)).map_or(false, |k| k != TileKind::Open))
    }

    pub fn walkable(&self, p: Pos) -> bool {
        self.get(p).map_or(false, |t| t.frozen || (t.kind.has_floor() && t.water <= 3 && t.magma == 0))
    }

    pub fn passable(&self, p: Pos, m: Mover) -> bool {
        match self.get(p) {
            Some(t) if t.frozen || (t.kind.has_floor() && t.water <= 3 && t.magma == 0) => match t.door {
                DOOR_LOCKED | BRIDGE_RAISED | FLOODGATE_SHUT => false,
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
        while p.z > 0 && !self.get(p).map_or(true, |t| t.kind.has_floor() || t.frozen) && !self.solid(p.down()) {
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
        // Dwarves keep to their burrow and follow traffic designations.
        if m == Mover::Dwarf {
            if self.restrict != 0 {
                out.retain(|(q, _)| self.t(*q).burrows & self.restrict != 0);
            }
            for (q, c) in out.iter_mut() {
                match self.t(*q).traffic {
                    1 => *c = *c * 7 / 10,
                    2 => *c *= 3,
                    3 => *c *= 12,
                    _ => {}
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
    let MapSize { w, h, .. } = params.size;
    crate::terrain::generate_region(seed, params, (0, 0), (w, h), None)
}
