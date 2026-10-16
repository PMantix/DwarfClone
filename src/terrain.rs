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
    let MapSize { w, h, d } = params.size;
    let (ox, oy) = origin;
    let bits = params.features;
    let has = |b: u32| bits & b != 0;
    // Deeper maps add rock below the usual layers (and room for a second cavern).
    let deep = (d - 32).max(0);
    let height_n = Noise::new(seed);
    let soil_n = Noise::new(seed ^ 0x1111);
    let rock_n = Noise::new(seed ^ 0x2222);
    let cave_n = Noise::new(seed ^ 0x3333);
    let ore_n = Noise::new(seed ^ 0x4444);
    let forest_n = Noise::new(seed ^ 0x5555);
    let pool_n = Noise::new(seed ^ 0x6666);
    let ridge_n = Noise::new(seed ^ 0xA1A1);
    let feat_n = Noise::new(seed ^ 0xB2B2);
    let mut rng = Rng::new(seed ^ 0x7777);
    let mut frng = Rng::new(seed ^ 0xC3C3);
    let (w0, h0) = base;
    let river = has(feat::RIVER).then(|| Course::new(seed, 0x1A1A, base, frng.chance(0.5)));
    let chasm = has(feat::CHASM).then(|| Course::new(seed, 0x2B2B, base, frng.chance(0.5)));
    let volcano = has(feat::VOLCANO).then(|| (frng.range(w0 / 4, (3 * w0 / 4).max(w0 / 4 + 1)), frng.range(h0 / 4, (3 * h0 / 4).max(h0 / 4 + 1))));
    let oasis = has(feat::OASIS).then(|| (frng.range(w0 / 5, (4 * w0 / 5).max(w0 / 5 + 1)), frng.range(h0 / 5, (4 * h0 / 5).max(h0 / 5 + 1))));
    let coast_side = frng.range(0, 4);
    let top = d - 3;

    // Height map.
    let mut surf = vec![0i32; (w * h) as usize];
    let mut river_cell = vec![false; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let (ax, ay) = (x + ox, y + oy);
            let v = height_n.fbm2(ax as f64 / 34.0, ay as f64 / 34.0, 4);
            let lo = (9 + deep) as f64;
            let mut hh = 18.0 + deep as f64 + (v - 0.5) * 18.0 * params.relief;
            if has(feat::PEAKS) {
                let r = 1.0 - (2.0 * ridge_n.fbm2(ax as f64 / 22.0, ay as f64 / 22.0, 3) - 1.0).abs();
                hh += r * r * r * 12.0 * params.relief.max(0.6);
            }
            if has(feat::MESAS) {
                hh = (hh / 3.0).floor() * 3.0 + 1.0;
            }
            if has(feat::COAST) {
                let t = match coast_side {
                    0 => ax as f64 / w0 as f64,
                    1 => 1.0 - ax as f64 / w0 as f64,
                    2 => ay as f64 / h0 as f64,
                    _ => 1.0 - ay as f64 / h0 as f64,
                };
                hh -= ((0.45 - t).max(0.0) * 26.0).min(9.0);
            }
            if let Some((vx, vy)) = volcano {
                let r = (((ax - vx).pow(2) + (ay - vy).pow(2)) as f64).sqrt();
                if r < 14.0 {
                    hh += (1.0 - r / 14.0) * 11.0;
                }
            }
            let cap = (lo + 18.0).max(if has(feat::PEAKS) || volcano.is_some() { lo + 24.0 } else { lo + 18.0 });
            let mut hi = hh.round().clamp(lo, cap.min(top as f64)) as i32;
            if let Some(c) = &river {
                if c.at(ax, ay).0 <= 2.0 {
                    hi -= 2;
                    river_cell[(y * w + x) as usize] = true;
                }
            }
            surf[(y * w + x) as usize] = hi;
        }
    }
    // A kept map's land stays exactly as it is.
    let mut fixed = vec![false; (w * h) as usize];
    if let Some((old, (dx, dy))) = keep {
        for y in 0..old.h {
            for x in 0..old.w {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && ny >= 0 && nx < w && ny < h {
                    surf[(ny * w + nx) as usize] = old.surface_z(x, y);
                    fixed[(ny * w + nx) as usize] = true;
                    river_cell[(ny * w + nx) as usize] = false;
                }
            }
        }
    }
    // Relax so neighbours never differ by more than one level (every slope
    // gets a ramp and the whole surface is walkable).
    if keep.is_none() {
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
    } else {
        // Around kept land: within one of every neighbour, rising to meet
        // the old map where it must.
        for _ in 0..400 {
            let mut changed = false;
            for y in 0..h {
                for x in 0..w {
                    let i = (y * w + x) as usize;
                    if fixed[i] {
                        continue;
                    }
                    let (mut lo, mut hi) = (i32::MIN, i32::MAX);
                    for &(dx, dy) in DIRS8.iter() {
                        let (nx, ny) = (x + dx, y + dy);
                        if nx < 0 || ny < 0 || nx >= w || ny >= h {
                            continue;
                        }
                        let n = surf[(ny * w + nx) as usize];
                        lo = lo.max(n - 1);
                        hi = hi.min(n + 1);
                    }
                    let c = surf[i];
                    let nc = if lo > hi { lo } else { c.clamp(lo, hi) };
                    if nc != c {
                        surf[i] = nc;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
    }

    let water_level = match keep {
        Some((old, _)) => old.water_level,
        None => {
            let mut sorted = surf.clone();
            sorted.sort_unstable();
            if params.lake <= 0.0 {
                0
            } else {
                sorted[((sorted.len() as f64 * params.lake) as usize).min(sorted.len() - 1)]
            }
        }
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
    };
    let bands = world.caverns.clone();

    for y in 0..h {
        for x in 0..w {
            let (ax, ay) = (x + ox, y + oy);
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
                        let v = soil_n.fbm2(ax as f64 / 14.0, ay as f64 / 14.0, 2);
                        if v < 0.42 {
                            Mat::Loam
                        } else if v < 0.62 {
                            Mat::Clay
                        } else {
                            Mat::Sand
                        }
                    } else if z >= 13 + deep {
                        if rock_n.fbm3(ax as f64 / 20.0, ay as f64 / 20.0, z as f64 / 3.0, 2) < 0.5 {
                            Mat::Sandstone
                        } else {
                            Mat::Limestone
                        }
                    } else if z >= 4 {
                        if rock_n.fbm3(ax as f64 / 9.0, ay as f64 / 9.0, z as f64 / 2.0, 2) > 0.66 {
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

    // An aquifer: rock two to four levels down that holds water, in broad
    // patches across the map.
    if params.aquifer > 0 {
        let aq_n = Noise::new(seed ^ 0x9999);
        for y in 0..h {
            for x in 0..w {
                let (ax, ay) = (x + ox, y + oy);
                if aq_n.fbm2(ax as f64 / 18.0, ay as f64 / 18.0, 2) < 0.45 {
                    continue;
                }
                let hh = surf[(y * w + x) as usize];
                for z in (hh - 4).max(1)..=hh - 2 {
                    let t = world.t_mut(Pos::new(x, y, z));
                    if t.kind == TileKind::Wall && !t.mat.is_ore() && !t.mat.is_gem() {
                        t.aquifer = true;
                    }
                }
            }
        }
    }

    // Surface lakes, forests and shrubs.
    for y in 0..h {
        for x in 0..w {
            let (ax, ay) = (x + ox, y + oy);
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
            let forest = forest_n.fbm2(ax as f64 / 16.0, ay as f64 / 16.0, 3);
            if forest > 0.55 && rng.chance((0.30 * params.forest).min(0.8)) || rng.chance(0.015 * params.forest) {
                let t = world.t_mut(p);
                t.kind = TileKind::Tree;
                t.mat = Mat::Wood;
            } else if rng.chance(0.05) {
                world.t_mut(p).kind = TileKind::Shrub;
            }
        }
    }

    // ---- The site's features. ----
    let surf_at = |world: &World, x: i32, y: i32| world.surface[(y * world.w + x) as usize];
    // A river in its bed: deep water, with shallow fords every so often.
    if let Some(c) = &river {
        for y in 0..h {
            for x in 0..w {
                if !river_cell[(y * w + x) as usize] {
                    continue;
                }
                let (_, along) = c.at(x + ox, y + oy);
                let hh = surf_at(&world, x, y);
                let p = Pos::new(x, y, hh);
                let t = world.t_mut(p);
                if t.kind == TileKind::Tree || t.kind == TileKind::Shrub {
                    t.kind = TileKind::Floor;
                }
                t.mat = Mat::Sand;
                t.water = if along.rem_euclid(19) < 3 { 2 } else { 6 };
            }
        }
    }
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
                    world.t_mut(p).water = 6;
                } else if r2 <= 40 {
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
                if world.t(p).kind == TileKind::Floor && world.t(p).water == 0 {
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
    world
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
}
