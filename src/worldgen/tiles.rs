//! Region tiles: each world cell is 16×16 of them, and a fortress map is
//! built from the ones under it (a region tile is 3×3 map columns). They
//! are never stored: any rectangle is worked out from the world's fields
//! and seed on demand, so a tile comes out the same whoever asks, and two
//! embarks side by side meet on the same land.
//!
//! Each tile has the world's fields interpolated smoothly between cell
//! centres, plus detail of its own: a fractal elevation (ridged in the
//! mountains, stepped where the rock is hard, so neighbouring tiles jump
//! into cliffs), water where the sea or a lake lies (the shore wandering
//! across the cells, not along their borders), the courses of the world's
//! rivers and brooks, a soil depth, and the geology of its province.

use super::fields::{classify, Sample, HILLS, MOUNTAIN, SEA};
use crate::noise::Noise;
use crate::overworld::{Biome, Overworld};
use crate::terrain_gen::field::{cell_hash, fbm, ridged};

/// Region tiles to a world cell, each way.
pub const SUB: i32 = 16;
/// Map columns to a region tile, each way.
pub const TILE: i32 = 3;

/// The stack of rock under a region tile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Geo {
    /// Sandstone and limestone over granite.
    Sedimentary,
    /// Marble over granite.
    Metamorphic,
    /// Basalt flows over granite.
    Extrusive,
    /// Granite all the way down.
    Intrusive,
}

/// What a tile has standing on it.
pub const LAND: u8 = 0;
pub const SEA_WATER: u8 = 1;
pub const LAKE_WATER: u8 = 2;

/// A rectangle of region tiles, in absolute region-tile coordinates.
#[derive(Clone, Debug)]
pub struct RegionTiles {
    pub x0: i32,
    pub y0: i32,
    pub w: i32,
    pub h: i32,
    /// The ground (the bed under water), in world elevation units.
    pub elev: Vec<f32>,
    /// The water's surface on water tiles; the river's on river tiles.
    pub level: Vec<f32>,
    pub water: Vec<u8>,
    pub biome: Vec<Biome>,
    pub evil: Vec<bool>,
    /// River and brook links between tiles (1 north, 2 east, 4 south, 8 west).
    pub river: Vec<u8>,
    pub brook: Vec<u8>,
    /// Water carried, on river tiles.
    pub flow: Vec<f32>,
    /// The next tile downstream on a river's (or brook's) course.
    pub down: Vec<Option<(i32, i32)>>,
    pub bdown: Vec<Option<(i32, i32)>>,
    /// Levels of soil over the rock.
    pub soil: Vec<u8>,
    pub veg: Vec<f32>,
    pub geo: Vec<Geo>,
}

impl RegionTiles {
    #[inline]
    pub fn inside(&self, rx: i32, ry: i32) -> bool {
        rx >= self.x0 && ry >= self.y0 && rx < self.x0 + self.w && ry < self.y0 + self.h
    }

    /// The index of an absolute tile, clamped into the rectangle.
    #[inline]
    pub fn idx(&self, rx: i32, ry: i32) -> usize {
        let x = (rx - self.x0).clamp(0, self.w - 1);
        let y = (ry - self.y0).clamp(0, self.h - 1);
        (y * self.w + x) as usize
    }
}

/// The world's fields at a cell, clamped to the world (None in a world made
/// before the fields were kept).
fn cell_sample(ow: &Overworld, cx: i32, cy: i32) -> Sample {
    let (x, y) = (cx.clamp(0, ow.w - 1), cy.clamp(0, ow.h - 1));
    let i = (y * ow.w + x) as usize;
    let f = &ow.fields;
    let g = |v: &Vec<u8>| *v.get(i).unwrap_or(&50) as f32;
    Sample { elev: ow.cells[i].elev as f32, temp: g(&f.temp), rain: g(&f.rain), drain: g(&f.drain), veg: g(&f.veg), savage: g(&f.savage), good: g(&f.good), salt: g(&f.salt) }
}

fn cell_at(ow: &Overworld, cx: i32, cy: i32) -> &crate::overworld::Cell {
    ow.cell(cx.clamp(0, ow.w - 1), cy.clamp(0, ow.h - 1))
}

/// How rough a biome's land is at the region scale: fractal amplitude,
/// ridges, and how much it climbs in steps (in elevation units, about seven
/// to a level).
fn rough(b: Biome) -> (f32, f32, f32) {
    match b {
        Biome::Ocean => (6.0, 0.0, 0.0),
        Biome::Lake => (4.0, 0.0, 0.0),
        Biome::Swamp | Biome::Marsh => (3.0, 0.0, 0.0),
        Biome::Grassland | Biome::Savanna => (8.0, 0.0, 0.0),
        Biome::Desert | Biome::Tundra => (8.0, 0.0, 0.0),
        Biome::Shrubland => (10.0, 0.0, 0.1),
        Biome::Forest => (12.0, 0.0, 0.0),
        Biome::Taiga | Biome::Jungle => (13.0, 0.0, 0.0),
        Biome::Hills => (40.0, 20.0, 0.5),
        Biome::Badlands => (30.0, 0.0, 0.9),
        Biome::Glacier => (40.0, 40.0, 0.5),
        Biome::Mountain => (55.0, 80.0, 0.7),
    }
}

/// Catmull-Rom through four values.
fn cubic(p: [f32; 4], t: f32) -> f32 {
    let (a, b, c, d) = (p[0], p[1], p[2], p[3]);
    b + 0.5 * t * (c - a + t * (2.0 * a - 5.0 * b + 4.0 * c - d + t * (3.0 * (b - c) + d - a)))
}

/// A field over world cells, sampled smoothly at a point in cell units
/// (cell centres at whole numbers + 0.5).
fn smooth(grid: &dyn Fn(i32, i32) -> f32, u: f32, v: f32) -> f32 {
    let (fu, fv) = (u - 0.5, v - 0.5);
    let (i0, j0) = (fu.floor() as i32, fv.floor() as i32);
    let (tu, tv) = (fu - i0 as f32, fv - j0 as f32);
    let mut rows = [0.0f32; 4];
    for (k, row) in rows.iter_mut().enumerate() {
        let j = j0 - 1 + k as i32;
        *row = cubic([grid(i0 - 1, j), grid(i0, j), grid(i0 + 1, j), grid(i0 + 2, j)], tu);
    }
    cubic(rows, tv)
}

/// Bilinear (smoothstepped) blend of a per-cell value.
fn blend(grid: &dyn Fn(i32, i32) -> f32, u: f32, v: f32) -> f32 {
    let (fu, fv) = (u - 0.5, v - 0.5);
    let (i0, j0) = (fu.floor() as i32, fv.floor() as i32);
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (tu, tv) = (s(fu - i0 as f32), s(fv - j0 as f32));
    let top = grid(i0, j0) + (grid(i0 + 1, j0) - grid(i0, j0)) * tu;
    let bot = grid(i0, j0 + 1) + (grid(i0 + 1, j0 + 1) - grid(i0, j0 + 1)) * tu;
    top + (bot - top) * tv
}

/// Region tiles over `w`×`h` tiles from absolute tile (x0, y0).
pub fn region_tiles(ow: &Overworld, x0: i32, y0: i32, w: i32, h: i32) -> RegionTiles {
    let n = (w * h) as usize;
    let seed = ow.seed;
    // The cells the rectangle touches, and two more all round (for the
    // smooth interpolation and the rivers that cross in from outside).
    let (cx0, cy0) = (x0.div_euclid(SUB) - 2, y0.div_euclid(SUB) - 2);
    let (cx1, cy1) = ((x0 + w - 1).div_euclid(SUB) + 2, (y0 + h - 1).div_euclid(SUB) + 2);
    let (cw, ch) = (cx1 - cx0 + 1, cy1 - cy0 + 1);
    let samples: Vec<Sample> = (cy0..=cy1).flat_map(|cy| (cx0..=cx1).map(move |cx| (cx, cy))).map(|(cx, cy)| cell_sample(ow, cx, cy)).collect();
    let cs = |cx: i32, cy: i32| -> &Sample { &samples[((cy.clamp(cy0, cy1) - cy0) * cw + (cx.clamp(cx0, cx1) - cx0)) as usize] };
    let _ = ch;
    let water_cell = |cx: i32, cy: i32| cell_at(ow, cx, cy).biome.is_water();
    let elev_n = Noise::new(seed ^ 0x7E1E_0001);
    let ridge_n = Noise::new(seed ^ 0x7E1E_0002);
    let shore_n = Noise::new(seed ^ 0x7E1E_0003);

    let mut t = RegionTiles {
        x0,
        y0,
        w,
        h,
        elev: vec![0.0; n],
        level: vec![0.0; n],
        water: vec![LAND; n],
        biome: vec![Biome::Grassland; n],
        evil: vec![false; n],
        river: vec![0; n],
        brook: vec![0; n],
        flow: vec![0.0; n],
        down: vec![None; n],
        bdown: vec![None; n],
        soil: vec![0; n],
        veg: vec![0.0; n],
        geo: vec![Geo::Sedimentary; n],
    };
    let provinces = provinces(ow);
    let warp_n = Noise::new(seed ^ 0x7E1E_0004);

    for ty in 0..h {
        for tx in 0..w {
            let (rx, ry) = (x0 + tx, y0 + ty);
            let i = (ty * w + tx) as usize;
            let (u, v) = ((rx as f32 + 0.5) / SUB as f32, (ry as f32 + 0.5) / SUB as f32);
            let (cx, cy) = (rx.div_euclid(SUB), ry.div_euclid(SUB));
            let f = |pick: fn(&Sample) -> f32| smooth(&|a, b| pick(cs(a, b)), u, v);
            let s = Sample {
                elev: f(|s| s.elev),
                temp: f(|s| s.temp).clamp(0.0, 100.0),
                rain: f(|s| s.rain).clamp(0.0, 100.0),
                drain: f(|s| s.drain).clamp(0.0, 100.0),
                veg: f(|s| s.veg).clamp(0.0, 100.0),
                savage: 0.0,
                good: f(|s| s.good).clamp(0.0, 100.0),
                salt: f(|s| s.salt).clamp(0.0, 100.0),
            };
            // Detail: fractal, ridged in the ranges, stepped in hard rock.
            let r = |k: usize| blend(&|a, b| { let q = rough(cell_at(ow, a, b).biome); [q.0, q.1, q.2][k] }, u, v);
            let (amp, rid, ter) = (r(0), r(1), r(2));
            let (fx, fy) = (rx as f64, ry as f64);
            let mut e = s.elev + amp * fbm(&elev_n, fx / 7.0, fy / 7.0, 4) as f32 * 2.0;
            if rid > 0.0 {
                e += rid * (ridged(&ridge_n, fx / 11.0, fy / 11.0, 4) as f32 - 0.45);
            }
            if ter > 0.0 {
                let step = 20.0;
                let q = (e / step).floor() * step;
                let frac = (e - q) / step;
                let stepped = q + step * ((frac - 0.88) * 8.0).clamp(0.0, 1.0);
                e += (stepped - e) * ter;
            }

            // Water: the shore lies `shore` tiles into the water cell (or,
            // negative, into the land), wandering with the noise.
            let shore = (fbm(&shore_n, fx / 9.0, fy / 9.0, 3) * 10.0) as f32;
            let wet = water_cell(cx, cy);
            let (sx, sy) = (rx - cx * SUB, ry - cy * SUB);
            let mut dmin = f32::MAX;
            let mut near_water: Option<(i32, i32)> = None;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if (dx, dy) == (0, 0) || water_cell(cx + dx, cy + dy) == wet {
                        continue;
                    }
                    // Distance from the tile's centre to that cell's square.
                    let gx = if dx < 0 { sx as f32 + 0.5 } else if dx > 0 { (SUB - sx) as f32 - 0.5 } else { 0.0 };
                    let gy = if dy < 0 { sy as f32 + 0.5 } else if dy > 0 { (SUB - sy) as f32 - 0.5 } else { 0.0 };
                    let d = (gx * gx + gy * gy).sqrt();
                    if d < dmin {
                        dmin = d;
                        near_water = Some((cx + dx, cy + dy));
                    }
                }
            }
            let is_water = if wet { dmin > shore } else { dmin < -shore };
            let water_of = if wet { Some((cx, cy)) } else { near_water };
            let water_level = |c: (i32, i32)| -> (u8, f32) {
                let wc = cell_at(ow, c.0, c.1);
                if wc.biome == Biome::Lake { (LAKE_WATER, wc.elev as f32) } else { (SEA_WATER, SEA) }
            };
            if is_water {
                let (kind, lv) = water_level(water_of.unwrap());
                t.water[i] = kind;
                t.level[i] = lv;
                // The bed shelves down from the shore.
                let into = if wet { dmin - shore } else { -shore - dmin };
                t.elev[i] = e.min(lv - 1.5 - into.clamp(0.0, 8.0) * 0.8);
                t.biome[i] = if kind == LAKE_WATER { Biome::Lake } else { Biome::Ocean };
            } else {
                // Land stands above whatever water is near.
                let floor = match water_of {
                    Some(c) if dmin < 10.0 => water_level(c).1 + 0.5,
                    _ => SEA + 0.5,
                };
                t.elev[i] = e.max(floor);
                let mut cs2 = s;
                cs2.elev = s.elev.max(floor);
                let b = classify(&cs2);
                t.biome[i] = if b == Biome::Ocean { Biome::Grassland } else { b };
            }
            t.evil[i] = t.water[i] == LAND && s.good < super::EVIL + 2.0;
            t.veg[i] = if t.water[i] == LAND { s.veg } else { 0.0 };
            // Soil: deep in wet lowland, thin on drained and high ground.
            let mut soil: i32 = if s.elev >= MOUNTAIN { 0 } else if s.elev >= HILLS { 1 } else { 2 };
            if s.veg > 45.0 && s.elev < MOUNTAIN {
                soil += 1;
            }
            if s.veg > 80.0 && s.drain < 40.0 {
                soil += 1;
            }
            if s.drain > 70.0 {
                soil -= 1;
            }
            if matches!(t.biome[i], Biome::Desert) {
                soil = 4;
            }
            t.soil[i] = soil.clamp(0, 5) as u8;
            // Geology: the nearest province centre, its borders bent.
            let (wu, wv) = (u + (fbm(&warp_n, fx / 20.0, fy / 20.0, 3) as f32) * 1.6, v + (fbm(&warp_n, fx / 20.0 + 31.0, fy / 20.0, 3) as f32) * 1.6);
            t.geo[i] = provinces.iter().min_by(|a, b| ((a.0 - wu).powi(2) + (a.1 - wv).powi(2)).partial_cmp(&((b.0 - wu).powi(2) + (b.1 - wv).powi(2))).unwrap()).map_or(Geo::Intrusive, |p| p.2);
        }
    }

    // Rivers and brooks, cell by cell.
    for cy in cy0 + 1..cy1 {
        for cx in cx0 + 1..cx1 {
            if cx < 0 || cy < 0 || cx >= ow.w || cy >= ow.h {
                continue;
            }
            let c = ow.cell(cx, cy);
            if c.river != 0 {
                course(ow, &mut t, cx, cy, c.river, true);
            }
            if c.brook != 0 {
                course(ow, &mut t, cx, cy, c.brook, false);
            }
        }
    }
    t
}

/// The geological provinces: seeded centres over the world (in cell
/// units) and the stack of rock each holds.
fn provinces(ow: &Overworld) -> Vec<(f32, f32, Geo)> {
    let mut rng = crate::rng::Rng::new(ow.seed ^ 0x6E0_6E0);
    let count = ((ow.w * ow.h) / 400).max(8);
    (0..count)
        .map(|_| {
            let (x, y) = (rng.range(0, ow.w), rng.range(0, ow.h));
            let c = ow.cell(x, y);
            let roll = rng.f64();
            let volcanic = (-2..=2).any(|dy: i32| (-2..=2).any(|dx: i32| x + dx >= 0 && y + dy >= 0 && x + dx < ow.w && y + dy < ow.h && ow.cell(x + dx, y + dy).volcano));
            let geo = if volcanic {
                Geo::Extrusive
            } else if c.elev as f32 >= HILLS {
                if roll < 0.4 { Geo::Metamorphic } else if roll < 0.8 { Geo::Intrusive } else { Geo::Extrusive }
            } else if roll < 0.65 {
                Geo::Sedimentary
            } else if roll < 0.8 {
                Geo::Metamorphic
            } else if roll < 0.9 {
                Geo::Extrusive
            } else {
                Geo::Intrusive
            };
            (x as f32 + 0.5, y as f32 + 0.5, geo)
        })
        .collect()
}

const DIRS: [(i32, i32, u8); 4] = [(0, -1, 1), (1, 0, 2), (0, 1, 4), (-1, 0, 8)];

fn back(bit: u8) -> u8 {
    match bit {
        1 => 4,
        2 => 8,
        4 => 1,
        _ => 2,
    }
}

/// Which of two linked cells lies downstream: the one carrying more water,
/// then the lower, then a fixed order, so both cells agree.
fn downstream_of(ow: &Overworld, a: (i32, i32), b: (i32, i32)) -> bool {
    let key = |c: (i32, i32)| {
        let cell = cell_at(ow, c.0, c.1);
        let i = (c.1.clamp(0, ow.h - 1) * ow.w + c.0.clamp(0, ow.w - 1)) as usize;
        let flow = if cell.biome.is_water() { 256 } else { *ow.fields.flow.get(i).unwrap_or(&0) as i32 };
        (flow, 255 - cell.elev as i32, c.1 * 4096 + c.0)
    };
    key(b) > key(a)
}

/// The tile where a river crosses the side between two cells, on `c`'s
/// side of it (its place along the side hashed from the pair).
fn side_tile(c: (i32, i32), n: (i32, i32), seed: u64) -> (i32, i32) {
    let (lo, hi) = if (c.1, c.0) < (n.1, n.0) { (c, n) } else { (n, c) };
    let off = 3 + (cell_hash(lo.0 * 3 + hi.0, lo.1 * 3 + hi.1, seed ^ 0x51DE) * 10.0) as i32;
    let (bx, by) = (c.0 * SUB, c.1 * SUB);
    match (n.0 - c.0, n.1 - c.1) {
        (1, 0) => (bx + SUB - 1, by + off),
        (-1, 0) => (bx, by + off),
        (0, 1) => (bx + off, by + SUB - 1),
        _ => (bx + off, by),
    }
}

/// A wandering 4-connected line of tiles from `a` to `b` inside the cell
/// whose corner is `corner`, the same every time for the same ends.
fn wander(a: (i32, i32), b: (i32, i32), corner: (i32, i32), salt: u64) -> Vec<(i32, i32)> {
    // Midpoint displacement of the straight line, two levels deep.
    let mut pts: Vec<(f32, f32)> = vec![(a.0 as f32, a.1 as f32), (b.0 as f32, b.1 as f32)];
    for depth in 0..2 {
        let mut next = vec![pts[0]];
        for k in 0..pts.len() - 1 {
            let (p, q) = (pts[k], pts[k + 1]);
            let (dx, dy) = (q.0 - p.0, q.1 - p.1);
            let len = (dx * dx + dy * dy).sqrt().max(1.0);
            let jit = (cell_hash(a.0 * 31 + b.0 * 7 + k as i32, a.1 * 17 + b.1 * 5 + depth, salt) as f32 - 0.5) * len * 0.5;
            let m = ((p.0 + q.0) / 2.0 - dy / len * jit, (p.1 + q.1) / 2.0 + dx / len * jit);
            let m = (m.0.clamp(corner.0 as f32 + 1.0, (corner.0 + SUB - 2) as f32), m.1.clamp(corner.1 as f32 + 1.0, (corner.1 + SUB - 2) as f32));
            next.push(m);
            next.push(q);
        }
        pts = next;
    }
    // Into tiles, one step at a time.
    let mut out: Vec<(i32, i32)> = vec![a];
    for k in 1..pts.len() {
        let target = if k == pts.len() - 1 { b } else { (pts[k].0.round() as i32, pts[k].1.round() as i32) };
        let mut cur = *out.last().unwrap();
        while cur != target {
            let (dx, dy) = (target.0 - cur.0, target.1 - cur.1);
            // Alternate so the line runs diagonally as a staircase.
            if dx != 0 && (dy == 0 || (out.len() % 2 == 0)) {
                cur.0 += dx.signum();
            } else {
                cur.1 += dy.signum();
            }
            out.push(cur);
        }
    }
    // Erase loops: if the line comes back to a tile, cut out the loop.
    let mut clean: Vec<(i32, i32)> = Vec::with_capacity(out.len());
    for p in out {
        if let Some(k) = clean.iter().position(|&q| q == p) {
            clean.truncate(k + 1);
        } else {
            clean.push(p);
        }
    }
    clean
}

/// Lay a cell's river (or brook) out on the tiles: from the middle of each
/// side it links across to a point near the cell's middle, the water level
/// falling all the way, and the tiles beneath cut down to it.
fn course(ow: &Overworld, t: &mut RegionTiles, cx: i32, cy: i32, links: u8, river: bool) {
    let seed = ow.seed;
    let c = (cx, cy);
    let corner = (cx * SUB, cy * SUB);
    let centre = (corner.0 + 8 + ((cell_hash(cx, cy, seed ^ 0xCE47) * 7.0) as i32 - 3), corner.1 + 8 + ((cell_hash(cx, cy, seed ^ 0xCE48) * 7.0) as i32 - 3));
    let here = ow.cell(cx, cy).elev as f32;
    let flow = crate::worldgen::fields::byte_flow(*ow.fields.flow.get((cy * ow.w + cx) as usize).unwrap_or(&0));
    // The way out first, then the ways in, each stopping where it meets
    // water already laid (so the courses in a cell form a tree).
    let mut sides: Vec<(i32, i32, u8, bool)> = DIRS.iter().filter(|d| links & d.2 != 0).map(|&(dx, dy, bit)| (dx, dy, bit, downstream_of(ow, c, (cx + dx, cy + dy)))).collect();
    sides.sort_by_key(|s| !s.3);
    for (dx, dy, bit, down) in sides {
        let n = (cx + dx, cy + dy);
        let ncell = cell_at(ow, n.0, n.1);
        let into_water = ncell.biome.is_water();
        let edge = side_tile(c, n, seed);
        let side_level = if into_water {
            let surface = if ncell.biome == Biome::Lake { ncell.elev as f32 } else { SEA };
            ((here + ncell.elev as f32) / 2.0).max(surface)
        } else {
            (here + ncell.elev as f32) / 2.0
        };
        let mut path = wander(edge, centre, corner, seed ^ ((bit as u64) << 8) ^ 0xC0_75E);
        let (mut l0, mut l1) = (side_level, here);
        if down {
            path.reverse();
            std::mem::swap(&mut l0, &mut l1);
            if into_water {
                // On into the lake or the sea until the water is reached.
                let mut cur = edge;
                for _ in 0..SUB {
                    cur = (cur.0 + dx, cur.1 + dy);
                    path.push(cur);
                    if t.inside(cur.0, cur.1) && t.water[t.idx(cur.0, cur.1)] != LAND {
                        break;
                    }
                }
            } else {
                // A step across the side into the next cell's own course.
                path.push((edge.0 + dx, edge.1 + dy));
            }
        }
        let steps = path.len().max(2) - 1;
        for k in 0..path.len() {
            let p = path[k];
            if k > 0 {
                link(t, path[k - 1], p, river);
            }
            if !t.inside(p.0, p.1) {
                continue;
            }
            let i = t.idx(p.0, p.1);
            if t.water[i] != LAND {
                break;
            }
            let laid = if river { t.down[i].is_some() } else { t.bdown[i].is_some() };
            if !down && k > 0 && laid {
                // Joined a course already laid.
                break;
            }
            let next = path.get(k + 1).copied();
            if river {
                let lv = l0 + (l1 - l0) * (k.min(steps) as f32 / steps as f32);
                if t.down[i].is_none() {
                    t.level[i] = lv;
                    t.down[i] = next;
                } else {
                    t.level[i] = t.level[i].min(lv);
                }
                t.elev[i] = t.elev[i].min(t.level[i] - 1.0);
                t.flow[i] = t.flow[i].max(flow);
            } else if t.bdown[i].is_none() {
                t.bdown[i] = next;
            }
        }
    }
}

fn link(t: &mut RegionTiles, a: (i32, i32), b: (i32, i32), river: bool) {
    let Some(&(_, _, bit)) = DIRS.iter().find(|d| (d.0, d.1) == (b.0 - a.0, b.1 - a.1)) else { return };
    for (p, bt) in [(a, bit), (b, back(bit))] {
        if t.inside(p.0, p.1) {
            let i = t.idx(p.0, p.1);
            if river {
                t.river[i] |= bt;
            } else {
                t.brook[i] |= bt;
            }
        }
    }
}
