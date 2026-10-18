//! Embark terrain: the land, rock, caverns and magma sea of a fortress map,
//! plus the features that make one site unlike the next (rivers, chasms,
//! volcanoes, mesas, peaks, oases, glaciers, bogs, coasts, tors, underground
//! rivers and gem grottoes).
//!
//! Noise is sampled by absolute position, so a map can be generated again
//! larger, from a shifted origin, and the new land continues the old.

use crate::geom::{Pos, DIRS8};
use crate::material::Mat;
use crate::noise::Noise;
use crate::overworld::{Biome, EmbarkParams};
use crate::rng::Rng;
use crate::world::{MapSize, Tile, TileKind, World};

/// Site features, as bits in `EmbarkParams::features`.
pub mod feat {
    pub const RIVER: u32 = 1;
    pub const CHASM: u32 = 2;
    pub const VOLCANO: u32 = 4;
    pub const MESAS: u32 = 8;
    pub const PEAKS: u32 = 16;
    pub const OASIS: u32 = 32;
    pub const GLACIER: u32 = 64;
    pub const BOG: u32 = 128;
    pub const COAST: u32 = 256;
    pub const TORS: u32 = 512;
    pub const UNDER_RIVER: u32 = 1024;
    pub const GROTTO: u32 = 2048;
}

/// (bit, name, what it means at the site).
pub const FEATURES: [(u32, &str, &str); 12] = [
    (feat::RIVER, "river", "a river winds across the land, fordable here and there"),
    (feat::CHASM, "chasm", "a chasm splits the ground, crossed by natural bridges"),
    (feat::VOLCANO, "volcano", "a volcano rises here, its crater full of magma"),
    (feat::MESAS, "mesas", "the land climbs in broad terraces"),
    (feat::PEAKS, "peaks", "sharp peaks and ridges"),
    (feat::OASIS, "oasis", "an oasis of water and trees"),
    (feat::GLACIER, "glacier", "a glacier: sheets of ice over the land"),
    (feat::BOG, "bog", "a bog of shallow pools and reeds"),
    (feat::COAST, "coast", "the sea washes one side of the land"),
    (feat::TORS, "tors", "crags of bare rock stand out of the ground"),
    (feat::UNDER_RIVER, "underground river", "a river runs through the rock far below"),
    (feat::GROTTO, "gem grotto", "a grotto of gems lies hidden in the deep"),
];

/// The names of a site's features.
pub fn feature_names(bits: u32) -> Vec<&'static str> {
    FEATURES.iter().filter(|f| bits & f.0 != 0).map(|f| f.1).collect()
}

/// What features a world cell has, fixed by the world seed.
pub fn site_features(seed: u64, sx: i32, sy: i32, biome: Biome) -> u32 {
    use feat::*;
    let mut rng = Rng::new(seed ^ (sx as u64).wrapping_mul(0x9E37_79B9) ^ ((sy as u64) << 32) ^ 0xFEA7_0000);
    let odds: &[(u32, f64)] = match biome {
        Biome::Mountain => &[(PEAKS, 0.75), (CHASM, 0.35), (VOLCANO, 0.15), (GLACIER, 0.15), (RIVER, 0.2), (TORS, 0.4)],
        Biome::Hills => &[(MESAS, 0.4), (RIVER, 0.4), (CHASM, 0.2), (TORS, 0.4), (VOLCANO, 0.05)],
        Biome::Forest => &[(RIVER, 0.6), (BOG, 0.2), (TORS, 0.2), (CHASM, 0.08)],
        Biome::Grassland => &[(RIVER, 0.5), (TORS, 0.3), (MESAS, 0.2), (CHASM, 0.1)],
        Biome::Desert => &[(MESAS, 0.6), (OASIS, 0.5), (TORS, 0.35), (VOLCANO, 0.06), (CHASM, 0.15)],
        Biome::Tundra => &[(GLACIER, 0.6), (TORS, 0.3), (RIVER, 0.2), (PEAKS, 0.2)],
        Biome::Swamp => &[(BOG, 0.9), (RIVER, 0.3)],
        Biome::Ocean => &[(COAST, 1.0), (TORS, 0.2), (RIVER, 0.2)],
        Biome::Jungle => &[(RIVER, 0.6), (BOG, 0.35), (CHASM, 0.1), (TORS, 0.15)],
        Biome::Taiga => &[(RIVER, 0.4), (BOG, 0.2), (TORS, 0.3), (GLACIER, 0.1)],
        Biome::Savanna => &[(RIVER, 0.3), (TORS, 0.4), (OASIS, 0.2), (MESAS, 0.15)],
        Biome::Badlands => &[(MESAS, 0.85), (CHASM, 0.35), (TORS, 0.3), (VOLCANO, 0.08)],
        Biome::Glacier => &[(GLACIER, 1.0), (PEAKS, 0.3), (CHASM, 0.15)],
        Biome::Shrubland => &[(RIVER, 0.3), (TORS, 0.4), (MESAS, 0.2), (CHASM, 0.1)],
        Biome::Marsh => &[(BOG, 0.9), (RIVER, 0.3)],
        Biome::Lake => &[(COAST, 1.0), (TORS, 0.2)],
    };
    let mut bits = 0;
    for &(b, p) in odds {
        if rng.chance(p) {
            bits |= b;
        }
    }
    for (b, p) in [(UNDER_RIVER, 0.35), (GROTTO, 0.3)] {
        if rng.chance(p) {
            bits |= b;
        }
    }
    bits
}

/// A line across the map for a river or chasm: horizontal or vertical,
/// wandering, in absolute coordinates over the original map of `base`.
struct Course {
    across: bool,
    noise: Noise,
    mid: f64,
    span: f64,
}

impl Course {
    fn new(seed: u64, salt: u64, base: (i32, i32), across: bool) -> Course {
        let (w0, h0) = base;
        let (mid, span) = if across { (h0 as f64 / 2.0, h0 as f64 * 0.55) } else { (w0 as f64 / 2.0, w0 as f64 * 0.55) };
        Course { across, noise: Noise::new(seed ^ salt), mid, span }
    }

    /// Distance of (ax, ay) from the course, and how far along it is.
    fn at(&self, ax: i32, ay: i32) -> (f64, i32) {
        let (along, off) = if self.across { (ax, ay) } else { (ay, ax) };
        let c = self.mid + (self.noise.fbm2(along as f64 / 38.0, 0.37, 3) - 0.5) * self.span;
        ((off as f64 - c).abs(), along)
    }
}

/// Generate a map `params.size` big whose tile (0, 0) sits at absolute
/// `origin`. `base` is the original map's size (where features are laid
/// out). With `keep`, an existing map is kept as it is at offset (dx, dy)
/// and the new land fitted round it.
pub fn generate_region(seed: u64, params: &EmbarkParams, origin: (i32, i32), base: (i32, i32), keep: Option<(&World, (i32, i32))>) -> World {
    generate_shaped(seed, params, origin, base, keep).0
}

/// `generate_region`, with the shaped land it was built from (rivers' courses,
/// lakes, streams) for looking into.
pub fn generate_shaped(seed: u64, params: &EmbarkParams, origin: (i32, i32), base: (i32, i32), keep: Option<(&World, (i32, i32))>) -> (World, crate::terrain_gen::Shape) {
    let MapSize { w, h, d } = params.size;
    let (ox, oy) = origin;
    let bits = params.features;
    let has = |b: u32| bits & b != 0;
    // Deeper maps add rock below the usual layers (and room for a second cavern).
    let deep = (d - 32).max(0);
    let soil_n = Noise::new(seed ^ 0x1111);
    let rock_n = Noise::new(seed ^ 0x2222);
    let cave_n = Noise::new(seed ^ 0x3333);
    let ore_n = Noise::new(seed ^ 0x4444);
    let forest_n = Noise::new(seed ^ 0x5555);
    let pool_n = Noise::new(seed ^ 0x6666);
    let feat_n = Noise::new(seed ^ 0xB2B2);
    let mut rng = Rng::new(seed ^ 0x7777);
    let mut frng = Rng::new(seed ^ 0xC3C3);
    let (w0, h0) = base;
    let chasm = has(feat::CHASM).then(|| Course::new(seed, 0x2B2B, base, frng.chance(0.5)));
    let volcano = has(feat::VOLCANO).then(|| (frng.range(w0 / 4, (3 * w0 / 4).max(w0 / 4 + 1)), frng.range(h0 / 4, (3 * h0 / 4).max(h0 / 4 + 1))));
    let oasis = has(feat::OASIS).then(|| (frng.range(w0 / 5, (4 * w0 / 5).max(w0 / 5 + 1)), frng.range(h0 / 5, (4 * h0 / 5).max(h0 / 5 + 1))));
    let coast_side = frng.range(0, 4);
    let top = d - 3;

    // Which world cell each column lies in (the site's own, or a neighbour's
    // toward the edges, with a ragged border), for its biome.
    const CELL: f64 = crate::terrain_gen::CELL;
    let biome_n = Noise::new(seed ^ 0xD4D4);
    let around = |c: (i32, i32)| params.around.iter().find(|a| (a.x, a.y) == c);
    let home_elev = around(params.home).map_or(128.0, |a| a.elev as f64);
    let local = |ax: i32, ay: i32| -> (Biome, bool) {
        if params.around.is_empty() {
            return (params.biome, false);
        }
        let jx = (biome_n.fbm2(ax as f64 / 11.0, ay as f64 / 11.0, 2) - 0.5) * 22.0;
        let jy = (biome_n.fbm2(ay as f64 / 11.0 + 40.0, ax as f64 / 11.0, 2) - 0.5) * 22.0;
        let c = (params.home.0 + ((ax as f64 + jx) / CELL).floor() as i32, params.home.1 + ((ay as f64 + jy) / CELL).floor() as i32);
        around(c).map_or((params.biome, false), |a| (a.biome, a.evil))
    };
    let sea = 14 + deep;
    let mut col = vec![0u8; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let (b, evil) = local(x + ox, y + oy);
            col[(y * w + x) as usize] = b.code() | if evil { 0x80 } else { 0 };
        }
    }
    // From the region tiles where there are any: their biomes, borders roughened.
    if let Some(rt) = params.region.as_deref() {
        col = crate::terrain_gen::region::biomes(seed, params.home, (ox, oy), (w, h), rt);
    }
    // A kept map keeps its biomes.
    if let Some((old, (dx, dy))) = keep {
        for y in 0..old.h {
            for x in 0..old.w {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && ny >= 0 && nx < w && ny < h {
                    if let Some(&c) = old.biomes.get((y * old.w + x) as usize) {
                        col[(ny * w + nx) as usize] = c;
                    }
                }
            }
        }
    }

    // The land itself: raised, weathered and watered (see `terrain_gen`).
    let ctx = crate::terrain_gen::Ctx {
        seed,
        p: params,
        ox,
        oy,
        w,
        h,
        base,
        lo: 9 + deep,
        top,
        mid: 17.0 + deep as f64,
        sea,
        col: &col,
        volcano,
        oasis,
        coast_side,
        home_elev,
    };
    let shape = crate::terrain_gen::shape(&ctx, keep);
    let ground = params.region.as_deref().map(|rt| crate::terrain_gen::region::ground(&ctx, rt));
    let surf = shape.surf.clone();
    let water_level = match keep {
        Some((old, _)) => old.water_level,
        None => shape.water_level,
    };
    let biome_of = |i: usize| Biome::from_code(col[i]).unwrap_or(params.biome);
    // How far each column stands above its lowest neighbour.
    let drop_at = |x: i32, y: i32| -> i32 {
        let s = surf[(y * w + x) as usize];
        DIRS8
            .iter()
            .filter(|&&(dx, dy)| x + dx >= 0 && y + dy >= 0 && x + dx < w && y + dy < h)
            .filter(|&&(dx, dy)| {
                let j = ((y + dy) * w + x + dx) as usize;
                shape.fill[j] <= surf[j] && shape.river[j] == 0
            })
            .map(|&(dx, dy)| s - surf[((y + dy) * w + x + dx) as usize])
            .max()
            .unwrap_or(0)
    };

    let mut world = World {
        w,
        h,
        d,
        tiles: vec![Tile::default(); (w * h * d) as usize],
        surface: surf.clone(),
        // Two cavern layers, three on deep maps; the upper ones only open out
        // where there's enough rock above them.
        caverns: if deep >= 12 {
            vec![(5, 8), (5 + deep / 2, 8 + deep / 2), (5 + deep, 8 + deep)]
        } else {
            vec![(5 + deep, 8 + deep), (11 + deep, 13 + deep)]
        },
        restrict: 0,
        aquifer: params.aquifer,
        origin,
        base: Some(base),
        water_level,
        features: bits,
        gen_seed: seed,
        biomes: col.clone(),
    };
    let bands = world.caverns.clone();
    // The rock at a level: granite and marble in the high and cold places,
    // sedimentary rock elsewhere, igneous rock deep down.
    let geo_rock = |geo: crate::worldgen::tiles::Geo, ax: i32, ay: i32, z: i32| -> Mat {
        use crate::worldgen::tiles::Geo;
        if z < 4 {
            return Mat::Basalt;
        }
        let r = rock_n.fbm3(ax as f64 / 12.0, ay as f64 / 12.0, z as f64 / 3.0, 2);
        match geo {
            Geo::Sedimentary => {
                if z as f64 >= 12.0 + deep as f64 - r * 3.0 {
                    // Beds of sandstone and limestone.
                    if ((z as f64 + r * 4.0) / 3.0).floor() as i32 % 2 == 0 { Mat::Sandstone } else { Mat::Limestone }
                } else if r > 0.7 {
                    Mat::Marble
                } else {
                    Mat::Granite
                }
            }
            Geo::Metamorphic => {
                if z >= 9 + deep && r > 0.25 { Mat::Marble } else { Mat::Granite }
            }
            Geo::Extrusive => {
                if z as f64 >= 12.0 + deep as f64 - r * 2.0 { Mat::Basalt } else { Mat::Granite }
            }
            Geo::Intrusive => {
                if r > 0.72 { Mat::Marble } else { Mat::Granite }
            }
        }
    };
    let rock_at = |ax: i32, ay: i32, z: i32, b: Biome| -> Mat {
        use Biome as B;
        if let Some(g) = &ground {
            let (x, y) = (ax - ox, ay - oy);
            if x >= 0 && y >= 0 && x < w && y < h {
                return geo_rock(g.geo[(y * w + x) as usize], ax, ay, z);
            }
        }
        if z >= 13 + deep {
            let r = rock_n.fbm3(ax as f64 / 20.0, ay as f64 / 20.0, z as f64 / 3.0, 2);
            match b {
                B::Mountain | B::Glacier | B::Tundra | B::Taiga => {
                    if r < 0.62 { Mat::Granite } else { Mat::Marble }
                }
                _ => {
                    if r < 0.5 { Mat::Sandstone } else { Mat::Limestone }
                }
            }
        } else if z >= 4 {
            let r = rock_n.fbm3(ax as f64 / 9.0, ay as f64 / 9.0, z as f64 / 2.0, 2);
            match b {
                B::Mountain | B::Glacier => {
                    if r > 0.68 { Mat::Basalt } else { Mat::Granite }
                }
                B::Desert | B::Badlands | B::Savanna | B::Shrubland => {
                    if r < 0.42 {
                        Mat::Sandstone
                    } else if r > 0.66 {
                        Mat::Marble
                    } else {
                        Mat::Granite
                    }
                }
                _ => {
                    if r > 0.66 { Mat::Marble } else { Mat::Granite }
                }
            }
        } else {
            Mat::Basalt
        }
    };

    for y in 0..h {
        for x in 0..w {
            let (ax, ay) = (x + ox, y + oy);
            let i = (y * w + x) as usize;
            let hh = surf[i];
            let b = biome_of(i);
            use Biome as B;
            let underwater = shape.fill[i] > hh || shape.river[i] != 0;
            let near_sea = shape.sea[i]
                || DIRS8.iter().any(|&(dx, dy)| {
                    let (nx, ny) = (x + dx * 2, y + dy * 2);
                    nx >= 0 && ny >= 0 && nx < w && ny < h && shape.sea[(ny * w + nx) as usize]
                });
            // Bare rock on cliffs and crags where the rock is hard; deep soil
            // where the water laid it down; sand on the beach.
            let steep = drop_at(x, y);
            let bare = !underwater && (steep >= 2 && shape.hard[i] > 0.5 || steep >= 3);
            let mut soil = match b {
                B::Mountain | B::Glacier => i32::from(hh <= 20 + deep),
                B::Badlands => 1,
                B::Tundra => 2,
                B::Desert => 4,
                _ => 3,
            };
            // The region tile's own depth, where the land came from them.
            if let Some(g) = &ground {
                soil = match b {
                    B::Glacier | B::Badlands => soil.min(g.soil[i] as i32),
                    _ => g.soil[i] as i32,
                };
            }
            if bare {
                soil = 0;
            } else if shape.alluvium[i] > 1.0 {
                soil += 1;
            } else if shape.alluvium[i] < -1.2 && shape.hard[i] > 0.5 || shape.scree[i] > 0.8 {
                soil = soil.min(1);
            }
            let sv = soil_n.fbm2(ax as f64 / 14.0, ay as f64 / 14.0, 2);
            let soil_mat = match b {
                B::Desert => Mat::Sand,
                B::Badlands => {
                    if sv < 0.6 { Mat::Clay } else { Mat::Sand }
                }
                B::Swamp | B::Marsh | B::Jungle => {
                    if sv < 0.55 { Mat::Loam } else { Mat::Clay }
                }
                _ if near_sea && hh <= sea + 1 => Mat::Sand,
                _ => {
                    if sv < 0.42 {
                        Mat::Loam
                    } else if sv < 0.62 {
                        Mat::Clay
                    } else {
                        Mat::Sand
                    }
                }
            };
            for z in 0..d {
                let p = Pos::new(x, y, z);
                if z == 0 {
                    let t = world.t_mut(p);
                    t.kind = TileKind::Wall;
                    t.mat = Mat::Bedrock;
                } else if z < hh {
                    let depth = hh - z;
                    let mat = if depth <= soil { soil_mat } else { rock_at(ax, ay, z, b) };
                    let t = world.t_mut(p);
                    t.kind = TileKind::Wall;
                    t.mat = mat;
                } else if z == hh {
                    // Sand and clay under water, a beach along the sea, bare
                    // rock where there's no soil.
                    let mat = if underwater {
                        if matches!(b, B::Swamp | B::Marsh | B::Jungle) || shape.river[i] == 0 && sv < 0.4 { Mat::Clay } else { Mat::Sand }
                    } else if near_sea && hh <= sea + 1 {
                        Mat::Sand
                    } else if soil == 0 {
                        rock_at(ax, ay, z - 1, b)
                    } else {
                        match b {
                            B::Desert | B::Ocean | B::Lake => Mat::Sand,
                            B::Badlands => soil_mat,
                            _ => Mat::Grass,
                        }
                    };
                    let t = world.t_mut(p);
                    t.kind = TileKind::Floor;
                    t.mat = mat;
                }
            }
        }
    }

    // Natural ramps on every slope (a step of one level; anything higher is a cliff).
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
                let (ax, ay) = (x + ox, y + oy);
                let p = Pos::new(x, y, z);
                let t = *world.t(p);
                if t.kind != TileKind::Wall || !t.mat.is_building_stone() {
                    continue;
                }
                let v = ore_n.fbm3(ax as f64 / 7.0, ay as f64 / 7.0, z as f64 / 3.0, 2);
                if (v - 0.5).abs() < 0.016 * params.minerals {
                    world.t_mut(p).mat = match t.mat {
                        Mat::Sandstone | Mat::Limestone => {
                            if (ax + z).rem_euclid(3) == 0 { Mat::Coal } else { Mat::Hematite }
                        }
                        Mat::Granite | Mat::Marble => {
                            if v > 0.5 { Mat::Malachite } else { Mat::NativeGold }
                        }
                        _ => Mat::Hematite,
                    };
                } else if rng.chance(0.0015 * params.minerals) {
                    world.t_mut(p).mat = Mat::Amethyst;
                }
            }
        }
    }

    // Cavern layers deep underground.
    for z in bands.iter().flat_map(|&(lo, hi)| lo..=hi) {
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let (ax, ay) = (x + ox, y + oy);
                let hh = surf[(y * w + x) as usize];
                if z >= hh - 4 {
                    continue;
                }
                let v = cave_n.fbm3(ax as f64 / 11.0, ay as f64 / 11.0, z as f64 / 3.5, 3);
                if v > 0.6 {
                    let t = world.t_mut(Pos::new(x, y, z));
                    t.kind = TileKind::Open;
                    t.mat = Mat::Air;
                }
            }
        }
    }
    for z in bands.iter().flat_map(|&(lo, hi)| lo..=hi) {
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
    for z in bands.iter().flat_map(|&(lo, hi)| lo..=hi) {
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let p = Pos::new(x, y, z);
                if world.t(p).kind != TileKind::Floor || world.kind(p.up()) != Some(TileKind::Open) {
                    continue;
                }
                let step = DIRS8.iter().any(|&(dx, dy)| world.solid(p.offset(dx, dy, 0)) && world.walkable(p.offset(dx, dy, 1)));
                if step {
                    world.t_mut(p).kind = TileKind::Ramp;
                }
            }
        }
    }
    // Cavern pools and towering mushroom trees.
    for z in bands.iter().flat_map(|&(lo, hi)| lo..=hi) {
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let (ax, ay) = (x + ox, y + oy);
                let p = Pos::new(x, y, z);
                if world.t(p).kind != TileKind::Floor {
                    continue;
                }
                if pool_n.fbm2(ax as f64 / 6.0, ay as f64 / 6.0 + z as f64 * 10.0, 2) > 0.68 {
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

    // The magma sea: a molten layer just above the bedrock under a basalt
    // roof, with air above it, a few basalt islands and a shore around them.
    let sea_n = Noise::new(seed ^ 0x8888);
    if d > 8 {
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let (ax, ay) = (x + ox, y + oy);
                // The cave above is a little wider than the sea, leaving a shore.
                let v = sea_n.fbm2(ax as f64 / 12.0, ay as f64 / 12.0, 2);
                if v < 0.3 {
                    continue;
                }
                if v >= 0.36 {
                    let t = world.t_mut(Pos::new(x, y, 1));
                    t.kind = TileKind::Open;
                    t.mat = Mat::Air;
                    t.magma = 7;
                }
                let t = world.t_mut(Pos::new(x, y, 2));
                t.kind = TileKind::Open;
                t.mat = Mat::Air;
            }
        }
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let p = Pos::new(x, y, 2);
                if world.t(p).kind == TileKind::Open && world.solid(p.down()) {
                    let t = world.t_mut(p);
                    t.kind = TileKind::Floor;
                    t.mat = Mat::Basalt;
                }
            }
        }
    }

    // Water: the sea and the lakes to their brims, the rivers in their
    // beds (shallow at the fords), streams where the run-off gathers.
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            let hh = surf[i];
            if shape.fill[i] > hh {
                for z in hh..shape.fill[i].min(d - 1) {
                    world.t_mut(Pos::new(x, y, z)).water = 7;
                }
            } else if shape.river[i] != 0 {
                world.t_mut(Pos::new(x, y, hh)).water = shape.river[i];
            } else if shape.stream[i] {
                let t = world.t_mut(Pos::new(x, y, hh));
                t.water = 2;
                if t.mat == Mat::Grass {
                    t.mat = Mat::Sand;
                }
            }
        }
    }

    // Caves and tunnels in the upper rock (bored last, below): more of
    // them in broken, hilly land.
    let rough = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).filter(|&(x, y)| drop_at(x, y) >= 2).count() as f64 / (w * h) as f64;
    let hilly = matches!(params.biome, Biome::Hills | Biome::Mountain | Biome::Badlands | Biome::Glacier);
    let mut tunnels = ((w * h) as f64 / 1500.0 * (0.4 + rough * 25.0)).round() as usize;
    if hilly {
        tunnels = tunnels.max(3);
    } else if rough < 0.01 && frng.chance(0.5) {
        tunnels = tunnels.min(1);
    }
    let cave_floor = bands.iter().map(|b| b.1).max().unwrap_or(8) + 2;

    // An aquifer: rock two to four levels down that holds water, in broad
    // patches across the map, likelier under wet lowland than dry heights.
    if params.aquifer > 0 {
        let aq_n = Noise::new(seed ^ 0x9999);
        let mut sorted = surf.clone();
        sorted.sort_unstable();
        let median = sorted[sorted.len() / 2];
        for y in 0..h {
            for x in 0..w {
                let (ax, ay) = (x + ox, y + oy);
                let i = (y * w + x) as usize;
                let hh = surf[i];
                let low = ((median - hh) as f64 * 0.03).clamp(-0.12, 0.12);
                if aq_n.fbm2(ax as f64 / 18.0, ay as f64 / 18.0, 2) + shape.moist[i] as f64 * 0.12 + low < 0.5 {
                    continue;
                }
                for z in (hh - 4).max(1)..=hh - 2 {
                    let t = world.t_mut(Pos::new(x, y, z));
                    if t.kind == TileKind::Wall && !t.mat.is_ore() && !t.mat.is_gem() {
                        t.aquifer = true;
                    }
                }
            }
        }
    }

    // What grows: forests where the ground is wet (thick along the rivers,
    // thin on the ridges, with clearings), meadows on the flats, reeds by
    // still water, nothing on bare rock; snow on the heights of cold lands.
    let (lo_s, hi_s) = (*surf.iter().min().unwrap_or(&0), *surf.iter().max().unwrap_or(&0));
    for y in 0..h {
        for x in 0..w {
            let (ax, ay) = (x + ox, y + oy);
            let i = (y * w + x) as usize;
            let hh = surf[i];
            let p = Pos::new(x, y, hh);
            let b = biome_of(i);
            let snowline = match b {
                Biome::Glacier => i32::MIN,
                Biome::Mountain => lo_s + (hi_s - lo_s) * 7 / 10,
                Biome::Tundra | Biome::Taiga => hi_s - 3,
                _ => i32::MAX,
            };
            if hh >= snowline && hi_s > lo_s + 4 || b == Biome::Glacier {
                if world.t(p).water == 0 && world.t(p).kind != TileKind::Open {
                    world.t_mut(p).snow = true;
                }
            }
            if world.t(p).kind != TileKind::Floor || world.t(p).water > 0 {
                continue;
            }
            let m = shape.moist[i] as f64;
            let by_water = DIRS8.iter().any(|&(dx, dy)| {
                let q = p.offset(dx, dy, 0);
                world.in_bounds(q) && world.t(q).water > 3
            });
            let mat = world.t(p).mat;
            let mut trees = if mat == Mat::Grass || matches!(b, Biome::Desert | Biome::Badlands) && mat.is_soil() { b.params().forest } else { 0.0 };
            if let Some(g) = &ground {
                trees *= crate::terrain_gen::region::tree_factor(b, g.veg[i]);
            }
            // Ridges and slopes dry out; hollows and banks hold water.
            let ridge = (drop_at(x, y) as f64 * 0.12).min(0.4);
            let cluster = forest_n.fbm2(ax as f64 / 16.0, ay as f64 / 16.0, 3) + (m - 0.35) * 0.35 - ridge;
            let density = trees * (0.35 + 0.9 * m);
            if shape.pass[i] {
                // A way cut through stays open.
            } else if cluster > 0.52 && rng.chance((0.34 * density).min(0.8)) || rng.chance(0.012 * trees) {
                let t = world.t_mut(p);
                t.kind = TileKind::Tree;
                t.mat = Mat::Wood;
            } else if by_water && matches!(b, Biome::Swamp | Biome::Marsh | Biome::Jungle | Biome::Forest | Biome::Taiga | Biome::Tundra | Biome::Grassland) && rng.chance(0.35) {
                // Reeds and marsh by still water.
                if matches!(b, Biome::Swamp | Biome::Marsh) && rng.chance(0.4) {
                    world.t_mut(p).water = 2;
                } else {
                    world.t_mut(p).kind = TileKind::Shrub;
                }
            } else if rng.chance(0.04 + 0.05 * m) {
                world.t_mut(p).kind = TileKind::Shrub;
            }
        }
    }

    // ---- The site's features. ----
    let surf_at = |world: &World, x: i32, y: i32| world.surface[(y * world.w + x) as usize];
    // A chasm down through the rock, with bridges of rock left across it.
    if let Some(c) = &chasm {
        let bottom = bands.last().map_or(8, |b| b.1 + 1);
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let (dist, along) = c.at(x + ox, y + oy);
                if dist > 1.6 || along.rem_euclid(26) < 2 {
                    continue;
                }
                let hh = surf_at(&world, x, y);
                let floor = (bottom + (feat_n.fbm2(x as f64 / 9.0, y as f64 / 9.0, 2) * 4.0) as i32).min(hh - 4);
                for z in floor + 1..=hh {
                    let t = world.t_mut(Pos::new(x, y, z));
                    t.kind = TileKind::Open;
                    t.mat = Mat::Air;
                    t.water = 0;
                }
                let f = world.t_mut(Pos::new(x, y, floor));
                if f.kind == TileKind::Wall {
                    f.kind = TileKind::Floor;
                }
            }
        }
    }
    // A volcano: a crater full of magma, fed by a pipe from the sea below.
    if let Some((vx, vy)) = volcano {
        let (cx, cy) = (vx - ox, vy - oy);
        if cx > 2 && cy > 2 && cx < w - 3 && cy < h - 3 {
            let rim = surf_at(&world, cx, cy);
            for y in cy - 2..=cy + 2 {
                for x in cx - 2..=cx + 2 {
                    let r2 = (x - cx).pow(2) + (y - cy).pow(2);
                    if r2 > 5 {
                        continue;
                    }
                    for z in 1..=rim {
                        let t = world.t_mut(Pos::new(x, y, z));
                        t.kind = TileKind::Open;
                        t.mat = Mat::Air;
                        t.water = 0;
                        if z < rim - 1 || r2 <= 2 && z < rim {
                            t.magma = 7;
                        }
                    }
                }
            }
        }
    }
    // An oasis: a pool ringed by trees.
    if let Some((vx, vy)) = oasis {
        let (cx, cy) = (vx - ox, vy - oy);
        for y in (cy - 7).max(1)..(cy + 8).min(h - 1) {
            for x in (cx - 7).max(1)..(cx + 8).min(w - 1) {
                let r2 = (x - cx).pow(2) + (y - cy).pow(2);
                let hh = surf_at(&world, x, y);
                let p = Pos::new(x, y, hh);
                if world.t(p).kind != TileKind::Floor && world.t(p).kind != TileKind::Ramp {
                    continue;
                }
                if r2 <= 9 {
                    if world.t(p).water == 0 {
                        world.t_mut(p).water = 6;
                    }
                } else if r2 <= 40 && world.t(p).water == 0 {
                    let t = world.t_mut(p);
                    t.mat = Mat::Grass;
                    if frng.chance(0.35) && t.kind == TileKind::Floor {
                        t.kind = TileKind::Tree;
                        t.mat = Mat::Wood;
                    }
                }
            }
        }
    }
    // Glacier and bog: ice sheets, or shallow pools among the reeds.
    if has(feat::GLACIER) || has(feat::BOG) {
        for y in 0..h {
            for x in 0..w {
                let (ax, ay) = (x + ox, y + oy);
                let v = feat_n.fbm2(ax as f64 / 13.0, ay as f64 / 13.0, 3);
                let hh = surf_at(&world, x, y);
                let p = Pos::new(x, y, hh);
                if world.t(p).kind != TileKind::Floor || world.t(p).water > 0 {
                    continue;
                }
                if has(feat::GLACIER) && v > 0.55 {
                    let t = world.t_mut(p);
                    t.water = 7;
                    t.frozen = true;
                } else if has(feat::BOG) && v > 0.58 {
                    world.t_mut(p).water = 2;
                } else if has(feat::BOG) && v > 0.5 && frng.chance(0.4) {
                    world.t_mut(p).kind = TileKind::Shrub;
                }
            }
        }
    }
    // Tors: crags of bare rock standing out of the ground.
    if has(feat::TORS) {
        for _ in 0..(w * h / 260) {
            let (x, y) = (frng.range(2, w - 2), frng.range(2, h - 2));
            for _ in 0..frng.range(3, 9) {
                let (tx, ty) = (x + frng.range(-2, 3), y + frng.range(-2, 3));
                if tx < 1 || ty < 1 || tx >= w - 1 || ty >= h - 1 {
                    continue;
                }
                let hh = surf_at(&world, tx, ty);
                let p = Pos::new(tx, ty, hh);
                if world.t(p).kind == TileKind::Floor && world.t(p).water == 0 && !shape.pass[(ty * w + tx) as usize] {
                    let t = world.t_mut(p);
                    t.kind = TileKind::Wall;
                    t.mat = Mat::Granite;
                }
            }
        }
    }
    // An underground river: a channel of water winding through the rock
    // between the cavern layers.
    if has(feat::UNDER_RIVER) {
        let c = Course::new(seed, 0x3C3C, base, true);
        // Between the deepest cavern layer and the next.
        let z = bands.first().map_or(9, |b| b.1 + 2).min(d - 8);
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                if c.at(x + ox, y + oy).0 > 1.2 || z >= surf_at(&world, x, y) - 4 {
                    continue;
                }
                let t = world.t_mut(Pos::new(x, y, z));
                t.kind = TileKind::Open;
                t.mat = Mat::Air;
                t.water = 7;
                let b = world.t_mut(Pos::new(x, y, z - 1));
                if b.kind == TileKind::Wall {
                    b.mat = Mat::Sandstone;
                }
            }
        }
    }
    // A grotto: a pocket in the deep rock lined with gems.
    if has(feat::GROTTO) {
        let (gx, gy) = (frng.range(w0 / 4, (3 * w0 / 4).max(w0 / 4 + 1)) - ox, frng.range(h0 / 4, (3 * h0 / 4).max(h0 / 4 + 1)) - oy);
        let gz = bands.first().map_or(6, |b| b.0) + 1;
        if gx > 5 && gy > 5 && gx < w - 6 && gy < h - 6 {
            for y in gy - 4..=gy + 4 {
                for x in gx - 5..=gx + 5 {
                    for z in gz - 1..=gz + 1 {
                        let r = ((x - gx) * (x - gx)) as f64 / 25.0 + ((y - gy) * (y - gy)) as f64 / 16.0 + ((z - gz) * (z - gz)) as f64;
                        let t = world.t_mut(Pos::new(x, y, z));
                        if r <= 0.8 {
                            t.kind = if z == gz - 1 { TileKind::Floor } else { TileKind::Open };
                            t.mat = if z == gz - 1 { Mat::Marble } else { Mat::Air };
                        } else if r <= 1.6 && t.kind == TileKind::Wall {
                            t.mat = Mat::Amethyst;
                        }
                    }
                }
            }
        }
    }

    // A tree or a crag that shuts a narrow way between two pieces of land
    // falls (the land must stay one piece to walk).
    if keep.is_none() {
        open_ways(&mut world);
    }

    // The caves, kept clear of where the wagon will stop (the fortress
    // grows round it).
    if keep.is_none() && tunnels > 0 {
        let e = embark_spot(&world);
        let clear = (w.min(h) as f64 * 0.42) as i32;
        crate::terrain_gen::caves::bore(&mut world, seed, tunnels.min(14), (e.x, e.y), clear, cave_floor, volcano.map(|(vx, vy)| (vx - ox, vy - oy)));
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

    // The kept map goes back exactly as it was.
    if let Some((old, (dx, dy))) = keep {
        for z in 0..d.min(old.d) {
            for y in 0..old.h {
                for x in 0..old.w {
                    let to = Pos::new(x + dx, y + dy, z);
                    if world.in_bounds(to) {
                        *world.t_mut(to) = *old.t(Pos::new(x, y, z));
                    }
                }
            }
        }
    }
    (world, shape)
}

/// Clear the trees and crags standing between the walkable land's pieces:
/// any whose neighbours belong to more than one piece goes, until the
/// land is one (or nothing more can be opened).
fn open_ways(world: &mut World) {
    for _ in 0..6 {
        let (piece, main) = crate::terrain_gen::connect::surface_pieces(world);
        let pieces = piece.iter().filter(|&&p| p != u32::MAX && p != main).count();
        if pieces == 0 {
            return;
        }
        let (w, h) = (world.w, world.h);
        let mut opened = 0;
        for y in 0..h {
            for x in 0..w {
                let p = Pos::new(x, y, world.surface_z(x, y));
                let t = *world.t(p);
                if !(t.kind == TileKind::Tree || t.kind == TileKind::Wall && t.water == 0) {
                    continue;
                }
                let mut seen: Vec<u32> = Vec::new();
                for &(dx, dy) in DIRS8.iter() {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= w || ny >= h {
                        continue;
                    }
                    let k = piece[(ny * w + nx) as usize];
                    if k != u32::MAX && (world.surface_z(nx, ny) - p.z).abs() <= 1 && !seen.contains(&k) {
                        seen.push(k);
                    }
                }
                if seen.len() >= 2 {
                    let t = world.t_mut(p);
                    t.kind = TileKind::Floor;
                    if t.mat == Mat::Wood || t.mat.is_building_stone() {
                        t.mat = Mat::Grass;
                    }
                    opened += 1;
                }
            }
        }
        if opened == 0 {
            return;
        }
    }
}

/// Pick a dry, open spot near the middle of the map for the wagon: on the
/// land the map's edges can be walked to from, back from deep water, with
/// level ground round it and rock to dig into beside it.
pub fn embark_spot(w: &World) -> Pos {
    let (cx, cy) = (w.w / 2, w.h / 2);
    let (piece, main) = crate::terrain_gen::connect::surface_pieces(w);
    let deep = |x: i32, y: i32| x >= 0 && y >= 0 && x < w.w && y < w.h && w.t(Pos::new(x, y, w.surface_z(x, y))).water > 3;
    let mut best = None;
    let mut best_score = i32::MIN;
    // Open, dry ground at each column's top (not a tree, not water).
    let open = |x: i32, y: i32| -> bool {
        let p = Pos::new(x, y, w.surface_z(x, y));
        w.walkable(p) && w.t(p).water == 0 && w.t(p).kind != TileKind::Tree
    };
    let (mx, my) = ((w.w / 8).clamp(4, 12), (w.h / 6).clamp(4, 10));
    for y in my..w.h - my {
        for x in mx..w.w - mx {
            let p = Pos::new(x, y, w.surface_z(x, y));
            if !w.walkable(p) || w.t(p).water > 0 || piece[(y * w.w + x) as usize] != main {
                continue;
            }
            // Room round the wagon for stockpiles, farms and workshops:
            // open ground within a level of it, best of all level with it.
            let mut score = 0;
            for dy in (-10..=10).step_by(2) {
                for dx in (-10..=10).step_by(2) {
                    let (qx, qy) = (x + dx, y + dy);
                    if qx < 0 || qy < 0 || qx >= w.w || qy >= w.h || !open(qx, qy) {
                        continue;
                    }
                    let dz = (w.surface_z(qx, qy) - p.z).abs();
                    score += match dz {
                        0 => 4,
                        1 => 2,
                        _ => 0,
                    };
                }
            }
            if (-3..=3).any(|dy| (-3..=3).any(|dx| deep(x + dx, y + dy))) {
                score -= 30;
            }
            // Ground that holds level or rises to the east: halls dug there stay under rock.
            score += (1..=20).filter(|&dx| x + dx < w.w && w.surface_z(x + dx, y) >= p.z - 1).count() as i32;
            for dy in -2..=2 {
                for dx in -2..=2 {
                    let q = Pos::new(x + dx, y + dy, p.z);
                    if w.walkable(q) && w.t(q).water == 0 && w.t(q).kind != TileKind::Ramp {
                        score += 4;
                    }
                }
            }
            score -= (x - cx).abs() + (y - cy).abs();
            if score > best_score {
                best_score = score;
                best = Some(p);
            }
        }
    }
    best.unwrap_or(Pos::new(cx, cy, w.surface_z(cx, cy)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make(bits: u32, biome: Biome) -> World {
        let mut p = biome.params();
        p.features = bits;
        p.size = MapSize::MEDIUM;
        generate_region(42, &p, (0, 0), (p.size.w, p.size.h), None)
    }

    fn count(w: &World, f: impl Fn(&Tile, Pos) -> bool) -> usize {
        (0..w.tiles.len()).filter(|&i| f(&w.tiles[i], w.pos_of(i))).count()
    }

    #[test]
    fn the_neighbours_reach_onto_the_map() {
        use crate::overworld::Around;
        let mut p = Biome::Grassland.params();
        p.size = MapSize::MEDIUM;
        p.home = (10, 10);
        // Sea to the east, desert to the south, grassland elsewhere.
        for dy in -3..=3 {
            for dx in -3..=3 {
                let biome = if dx >= 1 && dy <= 0 { Biome::Ocean } else if dy >= 1 { Biome::Desert } else { Biome::Grassland };
                let elev = if biome == Biome::Ocean { 60 } else { 120 };
                p.around.push(Around { x: 10 + dx, y: 10 + dy, biome, elev, evil: false, river: 0 });
            }
        }
        let w = generate_region(5, &p, (0, 0), (96, 64), None);
        let share = |x0: i32, x1: i32, y0: i32, y1: i32, f: &dyn Fn(i32, i32) -> bool| {
            let n = ((x1 - x0) * (y1 - y0)) as f64;
            (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))).filter(|&(x, y)| f(x, y)).count() as f64 / n
        };
        let wet = |x: i32, y: i32| w.t(Pos::new(x, y, w.surface_z(x, y))).water > 0;
        let sandy = |x: i32, y: i32| w.t(Pos::new(x, y, w.surface_z(x, y))).mat == Mat::Sand;
        let east = share(70, 96, 0, 40, &wet);
        let west = share(0, 30, 0, 40, &wet);
        assert!(east > 0.6 && west < 0.3, "the sea should fill the east: east {:.2} west {:.2}", east, west);
        let south = share(0, 40, 54, 64, &sandy);
        assert!(south > 0.5, "the desert should reach the south edge: {:.2}", south);
        assert_eq!(w.biome_at(5, 5).map(|b| b.0), Some(Biome::Grassland));
        assert_eq!(w.biome_at(5, 62).map(|b| b.0), Some(Biome::Desert));
    }

    #[test]
    fn each_feature_shapes_the_land() {
        let plain = make(0, Biome::Forest);
        let surface_water = |w: &World| count(w, |t, p| t.water > 0 && p.z >= w.surface_z(p.x, p.y) && !t.frozen);
        let river = make(feat::RIVER, Biome::Forest);
        assert!(surface_water(&river) > surface_water(&plain) + 60, "a river: {} vs {}", surface_water(&river), surface_water(&plain));
        assert!(count(&river, |t, p| t.water == 2 && p.z >= river.surface_z(p.x, p.y)) > 0, "fords");
        let chasm = make(feat::CHASM, Biome::Mountain);
        let deep_open = count(&chasm, |t, p| t.kind == TileKind::Open && p.z < chasm.surface_z(p.x, p.y) - 5 && p.z > 12);
        assert!(deep_open > 100, "a chasm ({})", deep_open);
        let volcano = make(feat::VOLCANO, Biome::Mountain);
        assert!(count(&volcano, |t, p| t.magma > 0 && p.z > 10) > 5, "a magma pipe");
        let glacier = make(feat::GLACIER, Biome::Tundra);
        assert!(count(&glacier, |t, _| t.frozen) > 200);
        let bog = make(feat::BOG, Biome::Swamp);
        assert!(count(&bog, |t, p| t.water == 2 && p.z == bog.surface_z(p.x, p.y)) > 200);
        let tors = make(feat::TORS, Biome::Grassland);
        assert!(count(&tors, |t, p| t.kind == TileKind::Wall && p.z == tors.surface_z(p.x, p.y)) > 20);
        let under = make(feat::UNDER_RIVER, Biome::Forest);
        let z = under.caverns[0].1 + 2;
        assert!(count(&under, |t, p| t.water == 7 && t.kind == TileKind::Open && p.z == z) > 50);
        let grotto = make(feat::GROTTO, Biome::Forest);
        assert!(count(&grotto, |t, _| t.mat == Mat::Amethyst) > count(&plain, |t, _| t.mat == Mat::Amethyst) + 20);
        let coast = make(feat::COAST, Biome::Ocean);
        assert!(surface_water(&coast) > 1000, "the sea ({})", surface_water(&coast));
        let oasis = make(feat::OASIS, Biome::Desert);
        assert!(surface_water(&oasis) > surface_water(&make(0, Biome::Desert)));
        // Sites differ, and the same site is always the same.
        let a = site_features(7, 10, 10, Biome::Mountain);
        assert_eq!(a, site_features(7, 10, 10, Biome::Mountain));
        let kinds: std::collections::HashSet<u32> = (0..40).map(|i| site_features(7, i, 3, Biome::Hills)).collect();
        assert!(kinds.len() > 5, "hills should vary: {:?}", kinds);
        // No features: the map is just as it always was.
        let mut p = Biome::Forest.params();
        p.size = MapSize::MEDIUM;
        let old = crate::world::generate(42, &p);
        assert!(old.tiles == plain.tiles);
    }

    /// What the top of a column looks like, in plain ASCII: `~` deep water,
    /// `-` shallows, `#` ice, `T` tree, `"` shrub, `,` grass, `:` bare soil,
    /// `.` sand, `n` bare rock, `*` snow, `W` rock standing up, `M` magma,
    /// ` ` a drop.
    pub(crate) fn cover_char(w: &World, x: i32, y: i32) -> char {
        let s = w.surface_z(x, y);
        let mut p = Pos::new(x, y, s);
        while p.z + 1 < w.d && w.t(p.up()).water > 0 {
            p = p.up();
        }
        let t = w.t(p);
        if t.frozen {
            return '#';
        }
        if t.magma > 0 {
            return 'M';
        }
        if t.water > 3 || p.z > s {
            return '~';
        }
        if t.water > 0 {
            return '-';
        }
        match t.kind {
            TileKind::Tree => 'T',
            TileKind::Shrub => '"',
            TileKind::Wall => 'W',
            TileKind::Open => ' ',
            _ if t.snow => '*',
            _ => match t.mat {
                Mat::Grass => ',',
                Mat::Sand => '.',
                Mat::Loam | Mat::Clay => ':',
                _ => 'n',
            },
        }
    }

    /// The surface height as one character: 0-9 then a-z, counted up from
    /// the lowest column.
    pub(crate) fn height_char(w: &World, x: i32, y: i32, lo: i32) -> char {
        let v = (w.surface_z(x, y) - lo).clamp(0, 35) as u32;
        std::char::from_digit(v, 36).unwrap()
    }

    /// The sites `show_sites` draws: (label, test on a world cell).
    pub(crate) fn pick_sites(ow: &crate::overworld::Overworld, seed: u64) -> Vec<(&'static str, i32, i32)> {
        use crate::overworld::Biome as B;
        let cells: Vec<(i32, i32)> = (2..ow.h - 3).flat_map(|y| (2..ow.w - 3).map(move |x| (x, y))).filter(|&(x, y)| ow.embarkable(x, y)).collect();
        let same_round = |x: i32, y: i32| [(1, 0), (0, 1), (1, 1)].iter().all(|&(dx, dy)| ow.cell(x + dx, y + dy).biome == ow.cell(x, y).biome);
        let sea_next = |x: i32, y: i32| [(1, 0), (0, 1)].iter().any(|&(dx, dy)| ow.cell(x + dx, y + dy).biome == B::Ocean);
        let feats = |x: i32, y: i32| ow.site_features(seed, x, y);
        let tests: Vec<(&'static str, Box<dyn Fn(i32, i32) -> bool>)> = vec![
            ("plains (grassland)", Box::new(move |x, y| ow.cell(x, y).biome == B::Grassland && same_round(x, y) && feats(x, y) & (feat::RIVER | feat::MESAS | feat::CHASM) == 0)),
            ("rolling hills", Box::new(move |x, y| ow.cell(x, y).biome == B::Hills && same_round(x, y) && feats(x, y) & (feat::MESAS | feat::CHASM) == 0)),
            ("mountains", Box::new(move |x, y| ow.cell(x, y).biome == B::Mountain && feats(x, y) & feat::PEAKS != 0 && feats(x, y) & feat::CHASM == 0)),
            ("forest with a river", Box::new(move |x, y| ow.cell(x, y).biome == B::Forest && ow.cell(x, y).river != 0 && same_round(x, y))),
            ("coast", Box::new(move |x, y| matches!(ow.cell(x, y).biome, B::Grassland | B::Forest | B::Hills) && sea_next(x, y))),
            ("swamp", Box::new(move |x, y| ow.cell(x, y).biome == B::Swamp)),
            ("desert", Box::new(move |x, y| ow.cell(x, y).biome == B::Desert)),
            ("badlands", Box::new(move |x, y| ow.cell(x, y).biome == B::Badlands)),
        ];
        tests.iter().filter_map(|(name, f)| cells.iter().find(|&&(x, y)| f(x, y)).map(|&(x, y)| (*name, x, y))).collect()
    }

    /// Draw a handful of sites (cover, then heights), every second row,
    /// from world 42 or the next world that has one:
    /// `cargo test --release show_sites -- --ignored --nocapture` (SITE=name for one).
    #[test]
    #[ignore]
    fn show_sites() {
        let only = std::env::var("SITE").ok();
        let mut shown: Vec<&str> = Vec::new();
        for seed in [42u64, 7, 5, 11, 13] {
        let ow = crate::overworld::Overworld::generate(seed);
        for (name, sx, sy) in pick_sites(&ow, seed) {
            if shown.contains(&name) || only.as_deref().map_or(false, |o| !name.contains(o)) {
                continue;
            }
            shown.push(name);
            let params = ow.embark_params(seed, sx, sy, MapSize::MEDIUM);
            let t0 = std::time::Instant::now();
            let w = crate::world::generate(seed ^ ((sx as u64) << 20) ^ ((sy as u64) << 40), &params);
            let ms = t0.elapsed().as_secs_f64() * 1000.0;
            let lo = (0..w.h).flat_map(|y| (0..w.w).map(move |x| (x, y))).map(|(x, y)| w.surface_z(x, y)).min().unwrap();
            let hi = (0..w.h).flat_map(|y| (0..w.w).map(move |x| (x, y))).map(|(x, y)| w.surface_z(x, y)).max().unwrap();
            println!("== {} at ({}, {}): {:?}, features {:?}, river links {}, z {}..{}, {:.0} ms", name, sx, sy, params.biome, feature_names(params.features), params.river, lo, hi, ms);
            for y in (0..w.h).step_by(2) {
                let cover: String = (0..w.w).map(|x| cover_char(&w, x, y)).collect();
                println!("  {}", cover);
            }
            println!("  -- heights (0 = z {})", lo);
            for y in (0..w.h).step_by(2) {
                let hts: String = (0..w.w).map(|x| height_char(&w, x, y, lo)).collect();
                println!("  {}", hts);
            }
        }
        }
    }

    /// A site from a generated world, made as `Game::embark` makes it.
    fn site_map(ow: &crate::overworld::Overworld, seed: u64, x: i32, y: i32, size: MapSize) -> (EmbarkParams, World, crate::terrain_gen::Shape) {
        let p = ow.embark_params(seed, x, y, size);
        let (w, s) = generate_shaped(seed ^ ((x as u64) << 20) ^ ((y as u64) << 40), &p, (0, 0), (size.w, size.h), None);
        (p, w, s)
    }

    fn named_site(name: &str) -> (EmbarkParams, World, crate::terrain_gen::Shape) {
        for seed in [42u64, 7, 5] {
            let ow = crate::overworld::Overworld::generate(seed);
            if let Some(&(_, x, y)) = pick_sites(&ow, seed).iter().find(|s| s.0.contains(name)) {
                return site_map(&ow, seed, x, y, MapSize::MEDIUM);
            }
        }
        panic!("no {} site", name);
    }

    /// Each column's steepest step to a neighbour, as a histogram (0, 1, 2,
    /// 3 or more levels) and its mean.
    fn slopes(w: &World) -> ([usize; 4], f64) {
        let mut hist = [0usize; 4];
        let mut sum = 0i64;
        for y in 0..w.h {
            for x in 0..w.w {
                let s = w.surface_z(x, y);
                let d = crate::geom::DIRS4.iter().filter(|&&(dx, dy)| x + dx >= 0 && y + dy >= 0 && x + dx < w.w && y + dy < w.h).map(|&(dx, dy)| (w.surface_z(x + dx, y + dy) - s).abs()).max().unwrap_or(0);
                hist[(d as usize).min(3)] += 1;
                sum += d as i64;
            }
        }
        (hist, sum as f64 / (w.w * w.h) as f64)
    }

    #[test]
    fn rivers_run_downhill_the_way_the_world_draws_them() {
        let mut checked = 0;
        for seed in [42u64, 7] {
            let ow = crate::overworld::Overworld::generate(seed);
            let sites: Vec<(i32, i32)> = (2..ow.h - 3).flat_map(|y| (2..ow.w - 3).map(move |x| (x, y))).filter(|&(x, y)| ow.embarkable(x, y) && ow.cell(x, y).river != 0).take(8).collect();
            for (sx, sy) in sites {
                let (p, w, s) = site_map(&ow, seed, sx, sy, MapSize::MEDIUM);
                // Never uphill along any reach.
                for c in &s.courses {
                    for k in 1..c.len() {
                        assert!(c[k].2 <= c[k - 1].2, "seed {} site {:?}: the river climbs from {:?} to {:?}", seed, (sx, sy), c[k - 1], c[k]);
                    }
                }
                // Water in its bed.
                let river: Vec<(i32, i32)> = (0..w.h).flat_map(|y| (0..w.w).map(move |x| (x, y))).filter(|&(x, y)| s.river[(y * w.w + x) as usize] != 0).collect();
                assert!(river.len() > 40, "seed {} site {:?}: hardly any river ({})", seed, (sx, sy), river.len());
                for &(x, y) in &river {
                    assert!(w.t(Pos::new(x, y, w.surface_z(x, y))).water > 0, "a dry river bed at {:?}", (x, y));
                }
                // In and out across the sides the world map links.
                let near = |f: &dyn Fn(i32, i32) -> bool| river.iter().any(|&(x, y)| f(x, y));
                if p.river & 1 != 0 {
                    assert!(near(&|x, y| y == 0 && x < 48), "seed {} site {:?}: no river at the north edge", seed, (sx, sy));
                }
                if p.river & 8 != 0 {
                    assert!(near(&|x, y| x == 0 && y < 48), "seed {} site {:?}: no river at the west edge", seed, (sx, sy));
                }
                if p.river & 2 != 0 {
                    assert!(near(&|x, y| (46..=50).contains(&x) && y < 48), "seed {} site {:?}: no river across the east side", seed, (sx, sy));
                }
                if p.river & 4 != 0 {
                    assert!(near(&|x, y| (46..=50).contains(&y) && x < 48), "seed {} site {:?}: no river across the south side", seed, (sx, sy));
                }
                checked += 1;
            }
        }
        assert!(checked >= 8, "only {} river sites", checked);
    }

    #[test]
    fn lakes_lie_in_hollows() {
        let mut lakes = 0;
        for name in ["forest", "swamp", "mountains", "hills", "plains"] {
            let (_, w, s) = named_site(name);
            for y in 0..w.h {
                for x in 0..w.w {
                    let i = (y * w.w + x) as usize;
                    if s.fill[i] <= w.surface_z(x, y) || s.sea[i] {
                        continue;
                    }
                    lakes += 1;
                    // Held in on every side: by the lake itself, or by ground at least as high as its water.
                    for &(dx, dy) in DIRS8.iter() {
                        let (nx, ny) = (x + dx, y + dy);
                        if nx < 0 || ny < 0 || nx >= w.w || ny >= w.h {
                            continue;
                        }
                        let j = (ny * w.w + nx) as usize;
                        let held = s.fill[j] >= s.fill[i] || w.surface_z(nx, ny) >= s.fill[i];
                        assert!(held, "{}: the lake at {:?} (to {}) spills over {:?} (ground {})", name, (x, y), s.fill[i], (nx, ny), w.surface_z(nx, ny));
                    }
                    // And the water itself is there, to its brim.
                    assert_eq!(w.t(Pos::new(x, y, s.fill[i] - 1)).water, 7);
                }
            }
        }
        // The land from region tiles is smoother than the old invented
        // relief, with fewer closed hollows; a few lakes still form.
        assert!(lakes > 20, "hardly any lakes ({})", lakes);
    }

    #[test]
    fn mountains_are_steeper_than_plains() {
        let (_, plains, _) = named_site("plains");
        let (_, peaks, _) = named_site("mountains");
        let (_, hills, _) = named_site("hills");
        let (ph, pm) = slopes(&plains);
        let (mh, mm) = slopes(&peaks);
        let (hh, hm) = slopes(&hills);
        eprintln!("slopes 0/1/2/3+ and mean: plains {:?} {:.2}, hills {:?} {:.2}, mountains {:?} {:.2}", ph, pm, hh, hm, mh, mm);
        assert!(mm > pm * 2.0, "mountains {:.2} vs plains {:.2}", mm, pm);
        assert!(mh[2] + mh[3] > (ph[2] + ph[3]) * 3, "cliffs: mountains {:?} plains {:?}", mh, ph);
        assert!(ph[0] > mh[0], "the plains are flatter");
        let range = |w: &World| w.surface.iter().max().unwrap() - w.surface.iter().min().unwrap();
        assert!(range(&peaks) > range(&plains) + 4, "relief: mountains {} plains {}", range(&peaks), range(&plains));
    }

    #[test]
    fn caves_wind_under_hilly_land() {
        let mut found = 0;
        for name in ["hills", "mountains", "badlands"] {
            let (p, w, _) = named_site(name);
            if p.features & (feat::CHASM | feat::VOLCANO) != 0 {
                continue;
            }
            let top_band = w.caverns.iter().map(|b| b.1).max().unwrap();
            let caves = count(&w, |t, q| q.z > top_band + 1 && q.z < w.surface_z(q.x, q.y) - 1 && !t.kind.solid() && t.water == 0 && t.magma == 0);
            eprintln!("{}: {} tiles of cave", name, caves);
            assert!(caves > 40, "{}: no caves ({})", name, caves);
            found += 1;
        }
        assert!(found >= 2);
        // And none where the wagon stops.
        let (_, w, _) = named_site("hills");
        let e = embark_spot(&w);
        let top_band = w.caverns.iter().map(|b| b.1).max().unwrap();
        let near = count(&w, |t, q| (q.x - e.x).abs() <= 12 && (q.y - e.y).abs() <= 12 && q.z > top_band + 1 && q.z < w.surface_z(q.x, q.y) - 1 && !t.kind.solid());
        assert_eq!(near, 0, "a cave under the wagon");
    }

    #[test]
    fn land_is_made_quickly() {

        let ow = crate::overworld::Overworld::generate(42);
        let (_, x, y) = pick_sites(&ow, 42).into_iter().find(|s| s.0 == "mountains").unwrap();
        let p = ow.embark_params(42, x, y, MapSize::MEDIUM);
        let slow = if cfg!(debug_assertions) { 15.0 } else { 1.0 };
        for (size, bound) in [(MapSize::MEDIUM, 1.0), (MapSize::ALL[2], 3.0), (MapSize { w: 320, h: 256, d: 48 }, 6.0)] {
            let mut q = p.clone();
            q.size = size;
            let t0 = std::time::Instant::now();
            let w = generate_region(9, &q, (0, 0), (size.w, size.h), None);
            let s = t0.elapsed().as_secs_f64();
            eprintln!("{}x{}x{}: {:.0} ms", w.w, w.h, w.d, s * 1000.0);
            assert!(s < bound * slow, "{}x{} took {:.2}s", size.w, size.h, s);
        }
    }

    #[test]
    fn the_wagon_stops_on_dry_ground_the_whole_map_can_reach() {
        for name in ["plains", "hills", "mountains", "forest", "coast", "swamp", "desert", "badlands"] {
            let (_, w, _) = named_site(name);
            let e = embark_spot(&w);
            let t = w.t(e);
            assert!(w.walkable(e) && t.water == 0 && t.kind != TileKind::Tree, "{}: the wagon at {:?} on {:?}", name, e, t.kind);
            assert!(DIRS8.iter().all(|&(dx, dy)| w.get(e.offset(dx, dy, 0)).map_or(true, |q| q.water <= 3)), "{}: deep water beside the wagon", name);
            // Every bit of walkable ground is one piece with the wagon's.
            let (piece, main) = crate::terrain_gen::connect::surface_pieces(&w);
            assert_eq!(piece[(e.y * w.w + e.x) as usize], main, "{}: the wagon is cut off", name);
            let walk = (0..w.w * w.h).filter(|&i| piece[i as usize] != u32::MAX).count();
            let cut_off = (0..w.w * w.h).filter(|&i| piece[i as usize] != u32::MAX && piece[i as usize] != main).count();
            assert!(cut_off * 100 <= walk, "{}: {} of {} tiles cut off", name, cut_off, walk);
            // And a walker from the map's edge gets there.
            let edge = (0..w.w).map(|x| Pos::new(x, 0, w.surface_z(x, 0))).find(|&p| w.walkable(p) && piece[p.x as usize] == main);
            if let Some(from) = edge {
                assert!(crate::path::astar(&w, from, e, 200_000, crate::world::Mover::Goblin).is_some(), "{}: no way in from the edge", name);
            }
        }
    }

    #[test]
    fn sites_differ() {
        // Slope, cliffs, relief and water cover differ from site to site.
        let mut seen = Vec::new();
        for name in ["plains", "hills", "mountains", "forest", "coast", "swamp", "badlands", "desert"] {
            let (_, w, s) = named_site(name);
            let (hist, mean_slope) = slopes(&w);
            let cliffs = (hist[2] + hist[3]) as f64 / w.surface.len() as f64;
            // Water cover: lakes and the sea, rivers, streams, bog pools.
            let _ = &s;
            let wet = (0..w.h).flat_map(|y| (0..w.w).map(move |x| (x, y))).filter(|&(x, y)| w.t(Pos::new(x, y, w.surface_z(x, y))).water > 0).count() as f64 / w.surface.len() as f64;
            let relief = w.surface.iter().max().unwrap() - w.surface.iter().min().unwrap();
            seen.push((name, mean_slope, cliffs, wet, relief));
        }
        eprintln!("{:?}", seen);
        for i in 0..seen.len() {
            for j in i + 1..seen.len() {
                let (a, b) = (seen[i], seen[j]);
                let differ = (a.1 - b.1).abs() > 0.03 || (a.2 - b.2).abs() > 0.02 || (a.3 - b.3).abs() > 0.01 || (a.4 - b.4).abs() >= 3;
                assert!(differ, "{} and {} look alike: {:?} {:?}", a.0, b.0, a, b);
            }
        }
    }

    /// A site's map with the region tiles and mapping it was made from.
    fn tiled_site(ow: &crate::overworld::Overworld, seed: u64, x: i32, y: i32) -> (EmbarkParams, World, crate::terrain_gen::Shape) {
        let (p, w, s) = site_map(ow, seed, x, y, MapSize::MEDIUM);
        assert!(p.region.is_some() && s.zmap.is_some(), "no region tiles at {:?}", (x, y));
        (p, w, s)
    }

    /// The map column range of a region tile (absolute tile coords) on a
    /// map embarked at cell `home`.
    fn tile_cols(home: (i32, i32), rx: i32, ry: i32) -> (i32, i32) {
        (rx * 3 - home.0 * 48, ry * 3 - home.1 * 48)
    }

    #[test]
    fn the_land_follows_the_region_tiles() {
        let mut diffs: Vec<f32> = Vec::new();
        let (mut sum_e, mut sum_z, mut sum_ee, mut sum_zz, mut sum_ez) = (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
        for seed in [42u64, 7] {
            let ow = crate::overworld::Overworld::generate(seed);
            for (_, sx, sy) in pick_sites(&ow, seed) {
                let (p, w, s) = tiled_site(&ow, seed, sx, sy);
                let rt = p.region.as_deref().unwrap();
                let zm = s.zmap.unwrap();
                for ry in rt.y0..rt.y0 + rt.h {
                    for rx in rt.x0..rt.x0 + rt.w {
                        let (x0, y0) = tile_cols(p.home, rx, ry);
                        if x0 < 0 || y0 < 0 || x0 + 3 > w.w || y0 + 3 > w.h {
                            continue;
                        }
                        let i = rt.idx(rx, ry);
                        if rt.water[i] != crate::worldgen::tiles::LAND || rt.river[i] != 0 {
                            continue;
                        }
                        let mut z = 0.0;
                        for dy in 0..3 {
                            for dx in 0..3 {
                                z += w.surface_z(x0 + dx, y0 + dy) as f32 / 9.0;
                            }
                        }
                        let want = zm.z(rt.elev[i]);
                        diffs.push((z - want).abs());
                        let e = want as f64;
                        sum_e += e;
                        sum_z += z as f64;
                        sum_ee += e * e;
                        sum_zz += z as f64 * z as f64;
                        sum_ez += e * z as f64;
                    }
                }
            }
        }
        diffs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = diffs.len() as f64;
        let corr = (sum_ez / n - sum_e / n * sum_z / n) / ((sum_ee / n - (sum_e / n).powi(2)).sqrt() * (sum_zz / n - (sum_z / n).powi(2)).sqrt());
        let median = diffs[diffs.len() / 2];
        let p90 = diffs[diffs.len() * 9 / 10];
        eprintln!("{} tiles: median off by {:.2} levels, 90% within {:.2}, correlation {:.3}", diffs.len(), median, p90, corr);
        assert!(median < 1.0 && p90 < 2.5, "the land strays from its tiles: median {:.2}, 90% {:.2}", median, p90);
        assert!(corr > 0.85, "the land doesn't follow its tiles (correlation {:.3})", corr);
    }

    #[test]
    fn rivers_and_brooks_follow_the_region_tiles() {
        let (mut river_tiles, mut wet, mut brook_tiles, mut brook_wet, mut stray, mut river_cols) = (0, 0, 0, 0, 0, 0);
        for seed in [42u64, 7] {
            let ow = crate::overworld::Overworld::generate(seed);
            let sites: Vec<(i32, i32)> = (2..ow.h - 3).flat_map(|y| (2..ow.w - 3).map(move |x| (x, y))).filter(|&(x, y)| ow.embarkable(x, y) && (ow.cell(x, y).river != 0 || ow.cell(x, y).brook != 0)).step_by(7).take(6).collect();
            for (sx, sy) in sites {
                let (p, w, s) = tiled_site(&ow, seed, sx, sy);
                let rt = p.region.as_deref().unwrap();
                let water_near = |x0: i32, y0: i32, r: i32| (y0 - r..y0 + 3 + r).any(|y| (x0 - r..x0 + 3 + r).any(|x| x >= 0 && y >= 0 && x < w.w && y < w.h && w.t(Pos::new(x, y, w.surface_z(x, y))).water > 0));
                for ry in rt.y0..rt.y0 + rt.h {
                    for rx in rt.x0..rt.x0 + rt.w {
                        let (x0, y0) = tile_cols(p.home, rx, ry);
                        if x0 < 2 || y0 < 2 || x0 + 5 > w.w || y0 + 5 > w.h {
                            continue;
                        }
                        let i = rt.idx(rx, ry);
                        if rt.water[i] != crate::worldgen::tiles::LAND {
                            continue;
                        }
                        if rt.river[i] != 0 {
                            river_tiles += 1;
                            if water_near(x0, y0, 1) {
                                wet += 1;
                            }
                        } else if rt.brook[i] != 0 {
                            brook_tiles += 1;
                            if water_near(x0, y0, 1) {
                                brook_wet += 1;
                            }
                        }
                    }
                }
                // Every river on the map lies along a river tile.
                for y in 0..w.h {
                    for x in 0..w.w {
                        if s.river[(y * w.w + x) as usize] == 0 {
                            continue;
                        }
                        river_cols += 1;
                        let (ax, ay) = (x + p.home.0 * 48, y + p.home.1 * 48);
                        let near = (-1..=1).any(|dy| (-1..=1).any(|dx| {
                            let j = rt.idx(ax.div_euclid(3) + dx, ay.div_euclid(3) + dy);
                            rt.river[j] != 0 || rt.water[j] != crate::worldgen::tiles::LAND
                        }));
                        if !near {
                            stray += 1;
                        }
                    }
                }
            }
        }
        eprintln!("river tiles {} ({} wet), brook tiles {} ({} wet), river columns {} ({} away from river tiles)", river_tiles, wet, brook_tiles, brook_wet, river_cols, stray);
        assert!(river_tiles > 50 && brook_tiles > 50, "too few courses to judge");
        assert!(wet * 100 >= river_tiles * 95, "rivers missing from their tiles: {} of {}", wet, river_tiles);
        assert!(brook_wet * 100 >= brook_tiles * 85, "brooks missing from their tiles: {} of {}", brook_wet, brook_tiles);
        assert!(stray * 100 <= river_cols, "{} of {} river columns stray from the tiles", stray, river_cols);
    }

    #[test]
    fn cliffs_stand_where_region_tiles_jump() {
        let (mut jumps, mut cliffs, mut flats, mut flat_cliffs) = (0, 0, 0, 0);
        for seed in [42u64, 7, 5] {
            let ow = crate::overworld::Overworld::generate(seed);
            for (name, sx, sy) in pick_sites(&ow, seed) {
                if !matches!(name, "mountains" | "rolling hills" | "badlands") {
                    continue;
                }
                let (p, w, s) = tiled_site(&ow, seed, sx, sy);
                let rt = p.region.as_deref().unwrap();
                let zm = s.zmap.unwrap();
                for ry in rt.y0..rt.y0 + rt.h {
                    for rx in rt.x0..rt.x0 + rt.w - 1 {
                        let (x0, y0) = tile_cols(p.home, rx, ry);
                        if x0 < 0 || y0 < 0 || x0 + 6 > w.w || y0 + 3 > w.h {
                            continue;
                        }
                        let (a, b) = (rt.idx(rx, ry), rt.idx(rx + 1, ry));
                        if rt.water[a] != 0 || rt.water[b] != 0 || rt.river[a] != 0 || rt.river[b] != 0 || rt.brook[a] != 0 || rt.brook[b] != 0 {
                            continue;
                        }
                        let dz = (zm.z(rt.elev[a]) - zm.z(rt.elev[b])).abs();
                        // The steepest step across the border, row by row.
                        let step = (y0..y0 + 3).map(|y| (x0 + 1..x0 + 5).map(|x| (w.surface_z(x, y) - w.surface_z(x + 1, y)).abs()).max().unwrap()).max().unwrap();
                        if dz >= 3.0 {
                            jumps += 1;
                            if step >= 2 {
                                cliffs += 1;
                            }
                        } else if dz < 0.5 {
                            flats += 1;
                            if step >= 2 {
                                flat_cliffs += 1;
                            }
                        }
                    }
                }
            }
        }
        eprintln!("{} jumps, {} with a cliff; {} flat borders, {} with a cliff", jumps, cliffs, flats, flat_cliffs);
        assert!(jumps > 30, "too few jumps to judge ({})", jumps);
        assert!(cliffs * 10 >= jumps * 7, "only {} of {} jumps make a cliff", cliffs, jumps);
        assert!((flat_cliffs as f64 / flats.max(1) as f64) < (cliffs as f64 / jumps as f64) / 3.0, "cliffs everywhere: {} of {} flat borders", flat_cliffs, flats);
    }

    #[test]
    fn neighbouring_embarks_meet_at_their_shared_edge() {
        let mut checked = 0;
        for seed in [42u64, 7] {
            let ow = crate::overworld::Overworld::generate(seed);
            // A medium map is two cells wide, so the map two cells east
            // starts where this one ends.
            let pairs: Vec<(i32, i32)> = (2..ow.h - 3).flat_map(|y| (2..ow.w - 5).map(move |x| (x, y))).filter(|&(x, y)| ow.embarkable(x, y) && ow.embarkable(x + 2, y) && (0..3).all(|d| !ow.cell(x + d, y).biome.is_water() && !ow.cell(x + d, y + 1).biome.is_water())).step_by(97).take(4).collect();
            for (sx, sy) in pairs {
                let (pa, a, sa) = tiled_site(&ow, seed, sx, sy);
                let (pb, b, sb) = tiled_site(&ow, seed, sx + 2, sy);
                assert_eq!(pa.home.0 * 48 + a.w, pb.home.0 * 48);
                let (za, zb) = (sa.zmap.unwrap(), sb.zmap.unwrap());
                // In the world's own heights, the two edges agree.
                let mut off = Vec::new();
                for y in 0..a.h {
                    let ea = za.e(a.surface_z(a.w - 1, y) as f32);
                    let eb = zb.e(b.surface_z(0, y) as f32);
                    off.push(((ea - eb) * za.k).abs());
                }
                off.sort_by(|p, q| p.partial_cmp(q).unwrap());
                let med = off[off.len() / 2];
                eprintln!("seed {} {:?}|{:?}: edges differ by {:.2} levels (median), {:.2} (90%)", seed, (sx, sy), (sx + 2, sy), med, off[off.len() * 9 / 10]);
                assert!(med <= 1.5, "seed {}: the maps at {:?} and {:?} don't meet ({:.2} levels)", seed, (sx, sy), (sx + 2, sy), med);
                // The same tiles hold water on both sides.
                let rt_a = pa.region.as_deref().unwrap();
                let rt_b = pb.region.as_deref().unwrap();
                for ry in sy * 16..sy * 16 + 21 {
                    let rx = (sx + 2) * 16;
                    assert_eq!(rt_a.river[rt_a.idx(rx, ry)], rt_b.river[rt_b.idx(rx, ry)]);
                    assert_eq!(rt_a.elev[rt_a.idx(rx, ry)], rt_b.elev[rt_b.idx(rx, ry)]);
                }
                // A river crossing the edge crosses on both maps at the same place.
                let wet_a: Vec<i32> = (0..a.h).filter(|&y| sa.river[(y * a.w + a.w - 1) as usize] != 0).collect();
                let wet_b: Vec<i32> = (0..b.h).filter(|&y| sb.river[(y * b.w) as usize] != 0).collect();
                for y in &wet_a {
                    assert!(wet_b.iter().any(|v| (v - y).abs() <= 3), "seed {}: a river leaves {:?} at y {} but doesn't come into {:?}", seed, (sx, sy), y, (sx + 2, sy));
                }
                checked += 1;
            }
        }
        assert!(checked >= 4);
    }
}
