//! The fortress map's land taken from the region tiles under it (see
//! `worldgen::tiles`): one mapping from world elevation to levels for the
//! whole map, the surface drawn through the tiles (smooth between tiles of
//! about the same height, a cliff at the border where they jump), the sea
//! and lakes where the tiles hold water, and the rivers and brooks along
//! the tiles' courses. The usual processes then finish the land.

use super::field::{fbm, ridged, smoothstep, Grid};
use super::rivers::{Net, Reach, Sample};
use super::Ctx;
use crate::noise::Noise;
use crate::overworld::Biome;
use crate::terrain::feat;
use crate::worldgen::tiles::{RegionTiles, LAND, TILE};

/// Map columns to a world cell.
pub const CELL: i32 = 48;

/// World elevation to z-levels, for one map.
#[derive(Clone, Copy, Debug)]
pub struct ZMap {
    pub e_ref: f32,
    pub base: f32,
    pub k: f32,
}

impl ZMap {
    pub fn z(&self, e: f32) -> f32 {
        self.base + (e - self.e_ref) * self.k
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn e(&self, z: f32) -> f32 {
        self.e_ref + (z - self.base) / self.k
    }
}

impl<'a> Ctx<'a> {
    /// A map column in absolute map coordinates (the world's (0, 0) cell
    /// starts at 0), the same on every map that covers it.
    pub fn abs(&self, x: i32, y: i32) -> (i32, i32) {
        (x + self.ox + self.p.home.0 * CELL, y + self.oy + self.p.home.1 * CELL)
    }
}

/// The region tile under a map column.
pub fn tile_of(cx: &Ctx, rt: &RegionTiles, x: i32, y: i32) -> usize {
    let (ax, ay) = cx.abs(x, y);
    rt.idx(ax.div_euclid(TILE), ay.div_euclid(TILE))
}

/// The tile's centre in map coordinates.
fn centre(cx: &Ctx, rx: i32, ry: i32) -> (f32, f32) {
    let (bx, by) = cx.abs(0, 0);
    ((rx * TILE) as f32 + TILE as f32 / 2.0 - 0.5 - bx as f32, (ry * TILE) as f32 + TILE as f32 / 2.0 - 0.5 - by as f32)
}

/// The mapping for this map: water (if any) at the sea's level, the land
/// rising a level for every seven units of elevation, pressed flatter only
/// if it wouldn't fit under the sky.
pub fn zmap(cx: &Ctx, rt: &RegionTiles) -> ZMap {
    let mut land: Vec<f32> = Vec::new();
    let mut water: Vec<(f32, usize)> = Vec::new();
    for y in (0..cx.h).step_by(TILE as usize) {
        for x in (0..cx.w).step_by(TILE as usize) {
            let i = tile_of(cx, rt, x, y);
            if rt.water[i] == LAND {
                land.push(rt.elev[i]);
            } else {
                match water.iter_mut().find(|w| (w.0 - rt.level[i]).abs() < 0.5) {
                    Some(w) => w.1 += 1,
                    None => water.push((rt.level[i], 1)),
                }
            }
        }
    }
    land.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let (e_ref, base) = match water.iter().max_by_key(|w| w.1) {
        Some(w) => (w.0, cx.sea as f32),
        None if !land.is_empty() => (land[land.len() / 2], cx.mid as f32),
        None => (0.0, cx.mid as f32),
    };
    let mut k = 0.15f32;
    if let (Some(&lo), Some(&hi)) = (land.first(), land.last()) {
        let up = (cx.top - 3) as f32 - base;
        if hi > e_ref {
            k = k.min(up / (hi - e_ref));
        }
        let down = base - (cx.lo + 1) as f32;
        if lo < e_ref && water.is_empty() {
            k = k.min(down / (e_ref - lo));
        }
    }
    ZMap { e_ref, base, k: k.max(0.02) }
}

/// Which columns lie under the sea or a lake (the tiles' water, its edge
/// roughened a little).
pub fn water(cx: &Ctx, rt: &RegionTiles) -> Vec<bool> {
    let n = Noise::new(cx.seed ^ 0x4E61_0001);
    let mut v = vec![false; (cx.w * cx.h) as usize];
    for y in 0..cx.h {
        for x in 0..cx.w {
            let (ax, ay) = cx.abs(x, y);
            let jx = (fbm(&n, ax as f64 / 5.0, ay as f64 / 5.0, 2) * 2.4) as i32;
            let jy = (fbm(&n, ax as f64 / 5.0 + 17.0, ay as f64 / 5.0, 2) * 2.4) as i32;
            let i = rt.idx((ax + jx).div_euclid(TILE), (ay + jy).div_euclid(TILE));
            v[(y * cx.w + x) as usize] = rt.water[i] != LAND;
        }
    }
    v
}

/// Each column's biome code (with the evil bit), from its region tile, the
/// borders roughened.
pub fn biomes(seed: u64, home: (i32, i32), (ox, oy): (i32, i32), (w, h): (i32, i32), rt: &RegionTiles) -> Vec<u8> {
    let n = Noise::new(seed ^ 0x4E61_0002);
    let mut v = vec![0u8; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let (ax, ay) = (x + ox + home.0 * CELL, y + oy + home.1 * CELL);
            let jx = (fbm(&n, ax as f64 / 6.0, ay as f64 / 6.0, 2) * 4.0) as i32;
            let jy = (fbm(&n, ax as f64 / 6.0 + 23.0, ay as f64 / 6.0, 2) * 4.0) as i32;
            let i = rt.idx((ax + jx).div_euclid(TILE), (ay + jy).div_euclid(TILE));
            v[(y * w + x) as usize] = rt.biome[i].code() | if rt.evil[i] { 0x80 } else { 0 };
        }
    }
    v
}

/// The surface through the tiles, before any weathering, and how hard the
/// rock is (as `relief::heights`).
pub fn heights(cx: &Ctx, rt: &RegionTiles, zm: &ZMap) -> (Grid, Grid, Grid, Grid) {
    let (w, h) = (cx.w, cx.h);
    let p = cx.p;
    let has = |b: u32| p.features & b != 0;
    let cliff_n = Noise::new(cx.seed ^ 0x4E61_0003);
    let detail_n = Noise::new(cx.seed ^ 0x4E61_0004);
    let hard_n = Noise::new(cx.seed ^ 0x3A04);
    let dune_n = Noise::new(cx.seed ^ 0x3A03);
    let peak_n = Noise::new(cx.seed ^ 0xA1A1);
    let mut hg = Grid::new(w, h, 0.0);
    let mut hard = Grid::new(w, h, 0.0);
    let mut ter = Grid::new(w, h, 0.0);
    let mut step = Grid::new(w, h, 3.0);
    let z_at = |rx: i32, ry: i32| zm.z(rt.elev[rt.idx(rx, ry)]);
    for y in 0..h {
        for x in 0..w {
            let (ax, ay) = cx.abs(x, y);
            let (fx, fy) = ((ax as f32 + 0.5) / TILE as f32 - 0.5, (ay as f32 + 0.5) / TILE as f32 - 0.5);
            let (i0, j0) = (fx.floor() as i32, fy.floor() as i32);
            let (tx, ty) = (fx - i0 as f32, fy - j0 as f32);
            // Smooth between tiles of about one height; where they differ
            // by two levels or more, a cliff at their border (wandering a
            // little).
            let jit = fbm(&cliff_n, ax as f64 / 4.0, ay as f64 / 4.0, 2) as f32 * 0.3;
            let cliff = std::cell::Cell::new(false);
            let mix = |a: f32, b: f32, t: f32| -> f32 {
                let wgt = if (b - a).abs() >= 2.0 {
                    // A face stands where the rock is hard enough to hold it.
                    if (t - 0.5 - jit).abs() < 0.4 {
                        cliff.set(true);
                    }
                    if t > 0.5 + jit { 1.0 } else { 0.0 }
                } else {
                    smoothstep(0.0, 1.0, t)
                };
                a + (b - a) * wgt
            };
            let top = mix(z_at(i0, j0), z_at(i0 + 1, j0), tx);
            let bot = mix(z_at(i0, j0 + 1), z_at(i0 + 1, j0 + 1), tx);
            let mut z = mix(top, bot, ty);
            let i = hg.idx(x, y);
            let b = cx.biome(i);
            let form = super::relief::biome_form(b);
            // A little roughness of the map's own.
            z += fbm(&detail_n, ax as f64 / 5.0, ay as f64 / 5.0, 2) as f32 * 0.35 * (form.amp as f32 / 4.0).min(1.2);
            if has(feat::PEAKS) {
                z += (ridged(&peak_n, ax as f64 / 14.0, ay as f64 / 14.0, 4) as f32 - 0.35) * 5.0;
            }
            if form.dunes > 0.0 {
                let (wx, wy) = (ax as f64, ay as f64);
                let ph = (wx * 0.8 + wy * 0.6) / 4.2 + fbm(&dune_n, wx / 20.0, wy / 20.0, 2) * 5.0;
                let crest = 0.5 + 0.5 * ph.sin();
                let field = (fbm(&dune_n, wx / 30.0 + 9.0, wy / 30.0, 2) + 0.6).clamp(0.0, 1.0);
                z += (crest * crest * form.dunes * field) as f32;
            }
            let mut hd = form.hard as f32 + fbm(&hard_n, ax as f64 / 22.0, ay as f64 / 22.0, 3) as f32 * 0.35;
            hd += match rt.geo[tile_of(cx, rt, x, y)] {
                crate::worldgen::tiles::Geo::Intrusive | crate::worldgen::tiles::Geo::Extrusive => 0.1,
                crate::worldgen::tiles::Geo::Sedimentary => -0.05,
                _ => 0.0,
            };
            let (lx, ly) = ((x + cx.ox) as f64, (y + cx.oy) as f64);
            if let Some((vx, vy)) = cx.volcano {
                let r = ((lx - vx as f64).powi(2) + (ly - vy as f64).powi(2)).sqrt();
                if r < 18.0 {
                    z += ((1.0 - r / 18.0).powf(1.4) * 13.0) as f32;
                    hd = hd.max(0.75);
                }
            }
            if let Some((vx, vy)) = cx.oasis {
                let r = ((lx - vx as f64).powi(2) + (ly - vy as f64).powi(2)).sqrt();
                if r < 9.0 {
                    z -= ((1.0 - r / 9.0) * 3.0) as f32;
                }
            }
            if cliff.get() {
                hd = hd.max(0.85);
            }
            let mut t = form.terrace as f32;
            if has(feat::MESAS) {
                t = t.max(0.9);
                hd = hd.max(0.65);
            }
            hg.v[i] = z;
            hard.v[i] = hd.clamp(0.05, 1.0);
            ter.v[i] = t.min(1.0);
            step.v[i] = form.step.max(2.0) as f32;
        }
    }
    (hg, hard, ter, step)
}

/// Chaikin's corner cutting, ends kept: smooths a line of tile centres
/// (with their levels) into a curve that stays within a tile of them.
fn chaikin(pts: &[(f32, f32, f32)]) -> Vec<(f32, f32, f32)> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let mut out = vec![pts[0]];
    for k in 0..pts.len() - 1 {
        let (a, b) = (pts[k], pts[k + 1]);
        let q = (a.0 * 0.75 + b.0 * 0.25, a.1 * 0.75 + b.1 * 0.25, a.2 * 0.75 + b.2 * 0.25);
        let r = (a.0 * 0.25 + b.0 * 0.75, a.1 * 0.25 + b.1 * 0.75, a.2 * 0.25 + b.2 * 0.75);
        if k > 0 {
            out.push(q);
        }
        if k < pts.len() - 2 {
            out.push(r);
        }
    }
    out.push(*pts.last().unwrap());
    out
}

/// The tiles' courses as chains of tiles, each from a spring or a junction
/// down to the next junction, the water, or the edge of the tiles.
fn chains(rt: &RegionTiles, down: &[Option<(i32, i32)>], on: &dyn Fn(usize) -> bool) -> Vec<Vec<usize>> {
    let n = rt.elev.len();
    let mut ups = vec![0u8; n];
    for i in 0..n {
        if !on(i) {
            continue;
        }
        if let Some((nx, ny)) = down[i] {
            if rt.inside(nx, ny) {
                let j = rt.idx(nx, ny);
                ups[j] = ups[j].saturating_add(1);
            }
        }
    }
    let mut out = Vec::new();
    for s in 0..n {
        if !on(s) || ups[s] == 1 {
            continue;
        }
        let mut chain = vec![s];
        let mut cur = s;
        while let Some((nx, ny)) = down[cur] {
            if !rt.inside(nx, ny) || chain.len() > n {
                break;
            }
            let j = rt.idx(nx, ny);
            if chain.contains(&j) {
                break;
            }
            chain.push(j);
            if rt.water[j] != LAND || ups[j] >= 2 || !on(j) {
                break;
            }
            cur = j;
        }
        if chain.len() >= 2 {
            out.push(chain);
        }
    }
    out
}

/// The rivers of the tiles, as the network the valleys are cut along.
pub fn network(cx: &Ctx, rt: &RegionTiles, zm: &ZMap, hg: &Grid) -> Net {
    let mut reaches = Vec::new();
    let half_base = if cx.w * cx.h > 12_000 { 1.2 } else { 1.0 };
    for chain in chains(rt, &rt.down, &|i| rt.river[i] != 0 && rt.water[i] == LAND && rt.down[i].is_some()) {
        let pts: Vec<(f32, f32, f32)> = chain
            .iter()
            .map(|&i| {
                let (rx, ry) = (rt.x0 + i as i32 % rt.w, rt.y0 + i as i32 / rt.w);
                let (x, y) = centre(cx, rx, ry);
                let lv = if rt.water[i] != LAND { rt.level[i] } else { rt.level[i] };
                (x, y, zm.z(lv))
            })
            .collect();
        let flow = chain.iter().map(|&i| rt.flow[i]).fold(0.0f32, f32::max);
        let smooth = chaikin(&chaikin(&pts));
        let mut samples = Vec::new();
        let mut along = 0.0;
        let mut run = f32::MAX;
        for k in 0..smooth.len() {
            let (a, b) = (smooth[k], *smooth.get(k + 1).unwrap_or(&smooth[k]));
            let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
            let steps = ((len / 0.5).ceil() as usize).max(1);
            let last = k + 1 == smooth.len();
            for s in 0..if last { 1 } else { steps } {
                let t = s as f32 / steps as f32;
                let (x, y, l) = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, a.2 + (b.2 - a.2) * t);
                // Never above the land it runs over (a grown map's eased
                // edge, a tile's own dip).
                run = run.min(l).min(hg.at_clamped(x.round() as i32, y.round() as i32) + 0.5);
                let half = half_base + 0.45 * (1.0 + flow / 45.0).ln();
                samples.push(Sample { x, y, level: run, half: half.clamp(1.1, 3.0), along });
                along += len / steps as f32;
            }
        }
        reaches.push(Reach { samples });
    }
    Net { reaches }
}

/// The brooks: columns along the tiles' brook courses.
pub fn brooks(cx: &Ctx, rt: &RegionTiles) -> Vec<bool> {
    let (w, h) = (cx.w, cx.h);
    let mut v = vec![false; (w * h) as usize];
    for chain in chains(rt, &rt.bdown, &|i| rt.brook[i] != 0 && rt.water[i] == LAND && rt.bdown[i].is_some()) {
        let pts: Vec<(f32, f32, f32)> = chain
            .iter()
            .map(|&i| {
                let (x, y) = centre(cx, rt.x0 + i as i32 % rt.w, rt.y0 + i as i32 / rt.w);
                (x, y, 0.0)
            })
            .collect();
        let smooth = chaikin(&chaikin(&pts));
        for k in 0..smooth.len().saturating_sub(1) {
            let (a, b) = (smooth[k], smooth[k + 1]);
            let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
            let steps = (len / 0.4).ceil().max(1.0) as usize;
            for s in 0..=steps {
                let t = s as f32 / steps as f32;
                let (x, y) = ((a.0 + (b.0 - a.0) * t).round() as i32, (a.1 + (b.1 - a.1) * t).round() as i32);
                if x >= 0 && y >= 0 && x < w && y < h {
                    v[(y * w + x) as usize] = true;
                }
            }
        }
    }
    v
}

/// Soil depth, vegetation and rock of each column, from its tile.
pub struct Ground {
    pub soil: Vec<u8>,
    pub veg: Vec<f32>,
    pub geo: Vec<crate::worldgen::tiles::Geo>,
}

pub fn ground(cx: &Ctx, rt: &RegionTiles) -> Ground {
    let n = (cx.w * cx.h) as usize;
    let mut g = Ground { soil: vec![0; n], veg: vec![0.0; n], geo: vec![crate::worldgen::tiles::Geo::Intrusive; n] };
    for y in 0..cx.h {
        for x in 0..cx.w {
            let i = tile_of(cx, rt, x, y);
            let k = (y * cx.w + x) as usize;
            g.soil[k] = rt.soil[i];
            g.veg[k] = rt.veg[i];
            g.geo[k] = rt.geo[i];
        }
    }
    g
}

/// How a biome's trees thicken or thin with the vegetation the world gives it.
pub fn tree_factor(b: Biome, veg: f32) -> f64 {
    let typical = match b {
        Biome::Jungle => 85.0,
        Biome::Forest | Biome::Swamp => 70.0,
        Biome::Taiga => 55.0,
        Biome::Marsh | Biome::Hills => 45.0,
        Biome::Grassland | Biome::Tundra => 35.0,
        Biome::Savanna | Biome::Shrubland => 25.0,
        _ => 30.0,
    };
    ((veg / typical) as f64).clamp(0.5, 1.5)
}
