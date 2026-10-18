//! How the land of an embark map comes to be, process by process: the
//! world map's heights and biomes raise it (`relief`), the world's rivers
//! cut their valleys (`rivers`), rain and frost wear it down (`erosion`),
//! water gathers into streams, ponds, lakes and the sea (`hydro`), every
//! piece of it is kept reachable (`connect`), and caves wind through the
//! rock beneath (`caves`). `terrain::generate_region` turns the result into
//! tiles.

pub mod caves;
pub mod connect;
pub mod erosion;
pub mod field;
pub mod hydro;
pub mod region;
pub mod relief;
pub mod rivers;

use crate::overworld::{Around, Biome, EmbarkParams};
use crate::rng::Rng;
use crate::world::World;
use connect::Wet;
use field::{fbm, Grid};

/// Tiles to a world cell.
pub const CELL: f64 = 48.0;

/// What every step needs to know about the map being made.
pub struct Ctx<'a> {
    pub seed: u64,
    pub p: &'a EmbarkParams,
    /// The map's place in the land's own coordinates, and its size.
    pub ox: i32,
    pub oy: i32,
    pub w: i32,
    pub h: i32,
    /// The original map's size (features are laid out over it).
    pub base: (i32, i32),
    /// Lowest and highest surface levels, the middle the land sits at, and the sea.
    pub lo: i32,
    pub top: i32,
    pub mid: f64,
    pub sea: i32,
    /// Each column's biome code (see `World::biomes`).
    pub col: &'a [u8],
    pub volcano: Option<(i32, i32)>,
    pub oasis: Option<(i32, i32)>,
    pub coast_side: i32,
    pub home_elev: f64,
}

impl<'a> Ctx<'a> {
    pub fn around(&self, x: i32, y: i32) -> Option<&Around> {
        self.p.around.iter().find(|a| (a.x, a.y) == (x, y))
    }

    pub fn cell_biome(&self, c: (i32, i32)) -> Biome {
        self.around(c.0, c.1).map_or(self.p.biome, |a| a.biome)
    }

    pub fn biome(&self, i: usize) -> Biome {
        Biome::from_code(self.col[i]).unwrap_or(self.p.biome)
    }

    /// The world map's height at a point, smoothly between cell centres.
    pub fn elev_at(&self, ax: f64, ay: f64) -> f64 {
        if self.p.around.is_empty() {
            return self.home_elev;
        }
        let (fx, fy) = (ax / CELL - 0.5, ay / CELL - 0.5);
        let (i0, j0) = (fx.floor(), fy.floor());
        let s = |t: f64| t * t * (3.0 - 2.0 * t);
        let (tx, ty) = (s(fx - i0), s(fy - j0));
        let e = |i: f64, j: f64| self.around(self.p.home.0 + i as i32, self.p.home.1 + j as i32).map_or(self.home_elev, |a| a.elev as f64);
        let top = e(i0, j0) + (e(i0 + 1.0, j0) - e(i0, j0)) * tx;
        let bottom = e(i0, j0 + 1.0) + (e(i0 + 1.0, j0 + 1.0) - e(i0, j0 + 1.0)) * tx;
        top + (bottom - top) * ty
    }

    /// The level the land sits at before hills are raised on it: higher
    /// toward high neighbours, lower toward low ones.
    pub fn base_z(&self, ax: f64, ay: f64) -> f64 {
        self.mid + ((self.elev_at(ax, ay) - self.home_elev) / 10.0).clamp(-7.0, 7.0)
    }
}

/// The shaped land, column by column.
pub struct Shape {
    pub surf: Vec<i32>,
    /// Lakes and the sea: water stands from the surface up to (not
    /// including) this level; 0 where there's none.
    pub fill: Vec<i32>,
    /// A river's water at the surface: 6 deep, 2 at a ford, 0 none.
    pub river: Vec<u8>,
    /// A stream: shallow water winding where the run-off gathers.
    pub stream: Vec<bool>,
    pub sea: Vec<bool>,
    pub hard: Vec<f32>,
    /// Soil laid down by water (positive) or worn away (negative), in levels.
    pub alluvium: Vec<f32>,
    /// Rock crumbled down from above.
    pub scree: Vec<f32>,
    /// 0..1: how wet the ground is (run-off, rivers and lakes nearby).
    pub moist: Vec<f32>,
    pub water_level: i32,
    /// Columns on a way cut through to join the land up (no trees there).
    pub pass: Vec<bool>,
    /// The rivers' courses on the map, upstream first: (x, y, bed level).
    #[cfg_attr(not(test), allow(dead_code))]
    pub courses: Vec<Vec<(i32, i32, i32)>>,
    /// World elevation to levels, when the land came from region tiles.
    #[cfg_attr(not(test), allow(dead_code))]
    pub zmap: Option<region::ZMap>,
}

/// How likely a hollow in this biome holds water.
pub fn lake_odds(b: Biome) -> f64 {
    match b {
        Biome::Desert => 0.0,
        Biome::Badlands => 0.1,
        Biome::Savanna | Biome::Shrubland => 0.35,
        Biome::Glacier => 0.3,
        Biome::Mountain => 0.55,
        Biome::Grassland | Biome::Hills => 0.6,
        Biome::Tundra | Biome::Forest => 0.75,
        Biome::Taiga | Biome::Jungle => 0.85,
        Biome::Swamp | Biome::Marsh | Biome::Ocean | Biome::Lake => 1.0,
    }
}

/// How much rain falls, roughly (for streams and the green of the land).
pub fn rainfall(b: Biome) -> f32 {
    match b {
        Biome::Desert | Biome::Badlands => 0.1,
        Biome::Savanna | Biome::Glacier => 0.4,
        Biome::Shrubland => 0.3,
        Biome::Grassland | Biome::Tundra | Biome::Mountain => 0.6,
        Biome::Hills => 0.7,
        Biome::Forest | Biome::Taiga => 0.9,
        Biome::Jungle | Biome::Swamp | Biome::Marsh | Biome::Ocean | Biome::Lake => 1.0,
    }
}

/// Shape the land: heights, weathering, rivers, lakes, streams and the sea.
pub fn shape(cx: &Ctx, keep: Option<(&World, (i32, i32))>) -> Shape {
    let (w, h) = (cx.w, cx.h);
    let n = (w * h) as usize;
    let mut rng = Rng::new(cx.seed ^ 0x7E44);
    // From the region tiles where the world has them, else from the world
    // cells' forms.
    let rt = cx.p.region.as_deref();
    let zm = rt.map(|rt| region::zmap(cx, rt));
    let (mut hg, hard, ter, step) = match (rt, &zm) {
        (Some(rt), Some(zm)) => region::heights(cx, rt, zm),
        _ => relief::heights(cx, &mut rng),
    };
    let tile_water = rt.map(|rt| region::water(cx, rt));

    // The sea: ocean columns, and one side of a coast.
    let mut sea = vec![false; n];
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            let (ax, ay) = ((x + cx.ox) as f64, (y + cx.oy) as f64);
            let mut s = match &tile_water {
                Some(v) => v[i],
                None => cx.biome(i).is_water(),
            };
            if cx.p.features & crate::terrain::feat::COAST != 0 {
                let (bw, bh) = (cx.base.0 as f64, cx.base.1 as f64);
                let t = match cx.coast_side {
                    0 => ax / bw,
                    1 => 1.0 - ax / bw,
                    2 => ay / bh,
                    _ => 1.0 - ay / bh,
                };
                s |= t < 0.3;
            }
            sea[i] = s;
        }
    }
    let shore = shore_distance(&sea, w, h);
    fit(&mut hg, &sea, &shore, cx, rt.is_some());
    let bench_floor = if sea.iter().any(|&s| s) { cx.sea as f32 } else { cx.lo as f32 + 2.0 };
    relief::terrace(&mut hg, &hard, &ter, &step, bench_floor);
    if sea.iter().any(|&s| s) {
        relief::sea_floor(&mut hg, &Grid { w, h, v: hard.v.clone() }, &shore, cx.sea as f32);
    }

    // Ponds: shallow dips in wet lowland.
    let pond_n = crate::noise::Noise::new(cx.seed ^ 0x6A6A);
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            let wet = match cx.biome(i) {
                // The world's wetlands lie wherever its fields say, flat or
                // not: deeper pools so they hold on a tilt.
                Biome::Swamp | Biome::Marsh if rt.is_some() => 1.8,
                Biome::Swamp | Biome::Marsh => 1.0,
                Biome::Jungle | Biome::Taiga | Biome::Forest | Biome::Tundra => 0.45,
                Biome::Grassland => 0.3,
                _ => 0.0,
            };
            if wet > 0.0 && !sea[i] {
                let v = cx_noise(&pond_n, x + cx.ox, y + cx.oy);
                hg.v[i] -= field::smoothstep(0.55, 0.7, v) * 1.6 * wet;
            }
        }
    }

    // A grown map: the old land stays exactly as it is, and the new land
    // eases toward it.
    let mut fixed = vec![false; n];
    let mut seam = vec![i32::MAX; n];
    let mut fixed_wet = vec![false; n];
    if let Some((old, (dx, dy))) = keep {
        let mut near = vec![(i32::MAX, 0.0f32); n];
        let mut queue = std::collections::VecDeque::new();
        for y in 0..old.h {
            for x in 0..old.w {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && ny >= 0 && nx < w && ny < h {
                    let i = (ny * w + nx) as usize;
                    fixed[i] = true;
                    let top = old.t(crate::geom::Pos::new(x, y, old.surface_z(x, y)));
                    fixed_wet[i] = !old.walkable(crate::geom::Pos::new(x, y, old.surface_z(x, y))) && (top.water > 3 || top.kind.solid() || top.kind == crate::world::TileKind::Open);
                    hg.v[i] = old.surface_z(x, y) as f32;
                    near[i] = (0, hg.v[i]);
                    queue.push_back(i);
                }
            }
        }
        while let Some(i) = queue.pop_front() {
            let (x, y) = (i as i32 % w, i as i32 / w);
            for &(ddx, ddy) in crate::geom::DIRS8.iter() {
                let (nx, ny) = (x + ddx, y + ddy);
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                let j = (ny * w + nx) as usize;
                if near[j].0 == i32::MAX && near[i].0 < 14 {
                    near[j] = (near[i].0 + 1, near[i].1);
                    queue.push_back(j);
                }
            }
        }
        for i in 0..n {
            seam[i] = near[i].0;
            if !fixed[i] && near[i].0 != i32::MAX {
                let t = field::smoothstep(0.0, 14.0, near[i].0 as f32);
                hg.v[i] = near[i].1 + (hg.v[i] - near[i].1) * t;
            }
        }
    }
    let fixed_opt = keep.map(|_| &fixed[..]);

    // The world's rivers cut their valleys.
    let net = match (rt, &zm) {
        (Some(rt), Some(zm)) => region::network(cx, rt, zm, &hg),
        _ => rivers::network(cx, &hg),
    };
    let near = rivers::nearest(&net, w, h, 26.0);
    rivers::carve_valley(&net, &near, &mut hg, &hard, fixed_opt);

    // Weathering.
    let wear = erosion::rain(&mut hg, &hard, &mut rng, 1.1, fixed_opt);
    let scree = erosion::crumble(&mut hg, &hard, 40, fixed_opt);
    // Sediment settles in the hollows: deep basins silt up, leaving
    // shallower ones (more of them in wet country, where lakes last).
    let brim_f = hydro::brims_f(&hg.v, w, h);
    for i in 0..n {
        let d = brim_f[i] - hg.v[i];
        if d > 0.0 && !fixed[i] && !sea[i] && near.dist[i] > 3.0 {
            let keep = 0.2 + 0.35 * lake_odds(cx.biome(i)) as f32;
            hg.v[i] = brim_f[i] - d * keep;
        }
    }

    // Into whole levels.
    let mut surf: Vec<i32> = hg.v.iter().map(|&z| (z.round() as i32).clamp(cx.lo, cx.top - 1)).collect();
    if let Some((old, (dx, dy))) = keep {
        for y in 0..old.h {
            for x in 0..old.w {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && ny >= 0 && nx < w && ny < h {
                    surf[(ny * w + nx) as usize] = old.surface_z(x, y);
                }
            }
        }
        // Next to the old land, step by step outward: within a level of it.
        let mut order: Vec<usize> = (0..n).filter(|&i| !fixed[i] && seam[i] <= 10).collect();
        order.sort_by_key(|&i| seam[i]);
        for &i in &order {
            let (x, y) = (i as i32 % w, i as i32 / w);
            let (mut lo, mut hi) = (i32::MIN, i32::MAX);
            for &(ddx, ddy) in crate::geom::DIRS8.iter() {
                let (nx, ny) = (x + ddx, y + ddy);
                if nx >= 0 && ny >= 0 && nx < w && ny < h {
                    let j = (ny * w + nx) as usize;
                    if seam[j] < seam[i] {
                        lo = lo.max(surf[j] - 1);
                        hi = hi.min(surf[j] + 1);
                    }
                }
            }
            if lo != i32::MIN {
                surf[i] = if lo <= hi { surf[i].clamp(lo, hi) } else { lo };
            }
        }
    }

    // The river in its bed, its banks above its water.
    let mut river = vec![0u8; n];
    let mut bed = vec![0i32; n];
    for i in 0..n {
        if fixed[i] || near.at[i].0 == u32::MAX {
            continue;
        }
        let s = net.reaches[near.at[i].0 as usize].samples[near.at[i].1 as usize];
        if near.dist[i] <= s.half {
            bed[i] = (s.level.floor() as i32).clamp(cx.lo, cx.top - 1);
            surf[i] = bed[i];
            // Fords: a riffle a level up, shallow water over stones, that holds the
            // deep water back in pools between them.
            if (s.along as i32).rem_euclid(21) < 3 {
                river[i] = 2;
                surf[i] = bed[i] + 1;
            } else {
                river[i] = 6;
            }
        }
    }
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            if river[i] != 0 || fixed[i] {
                continue;
            }
            let mut need = i32::MIN;
            for &(dx, dy) in crate::geom::DIRS8.iter() {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && ny >= 0 && nx < w && ny < h && river[(ny * w + nx) as usize] != 0 {
                    need = need.max(bed[(ny * w + nx) as usize] + 1);
                }
            }
            if surf[i] < need {
                surf[i] = need;
            }
        }
    }
    // The sea floor under the sea.
    for i in 0..n {
        if sea[i] && !fixed[i] && river[i] == 0 {
            surf[i] = surf[i].min(cx.sea - 1);
        }
    }

    // Lakes: hollows that hold their water, more of them in wet country.
    let outlet: Vec<Option<i32>> = (0..n)
        .map(|i| {
            if river[i] != 0 {
                Some(bed[i] + 1)
            } else if sea[i] {
                Some(cx.sea)
            } else {
                None
            }
        })
        .collect();
    let brim = hydro::brims(&surf, w, h, &outlet);
    let mut fill = vec![0i32; n];
    for i in 0..n {
        if sea[i] && river[i] == 0 && surf[i] < cx.sea {
            fill[i] = cx.sea;
        }
    }
    let mut lakes: Vec<(usize, i32)> = Vec::new();
    for hol in hydro::hollows(&surf, &brim, w, h) {
        if hol.cells.iter().any(|&i| fixed[i] || river[i] != 0 || sea[i]) {
            // Low land by the sea floods with it.
            if hol.cells.iter().any(|&i| sea[i]) && hol.level <= cx.sea {
                for &i in &hol.cells {
                    if !fixed[i] && river[i] == 0 {
                        fill[i] = hol.level;
                    }
                }
            }
            continue;
        }
        let deepest = *hol.cells.iter().max_by_key(|&&i| brim[i] - surf[i]).unwrap();
        let (dx, dy) = ((deepest as i32 % w + cx.ox), (deepest as i32 / w + cx.oy));
        let oasis = cx.oasis.map_or(false, |(vx, vy)| (vx - dx).abs() <= 10 && (vy - dy).abs() <= 10);
        let mut odds = if oasis { 1.0 } else { lake_odds(cx.biome(deepest)) };
        if hol.cells.len() < 4 && hol.depth <= 1 {
            odds *= 0.4;
        }
        if rng.chance(odds) {
            // In drier country the water stands below the brim (it leaks or
            // dries away faster than it fills), leaving a dry shore round it.
            let wet = oasis || lake_odds(cx.biome(deepest)) >= 0.75;
            let mut drop = if wet { 0 } else { (hol.depth - 1) / 2 };
            if !wet && hol.cells.len() > n / 12 {
                drop += 1;
            }
            let level = hol.level - drop.min(hol.depth - 1).max(0);
            let mut size = 0;
            for &i in &hol.cells {
                if surf[i] < level {
                    fill[i] = level;
                    size += 1;
                }
            }
            lakes.push((size, level));
        }
    }

    // Run-off: where the water from all the land above gathers.
    let rain: Vec<f32> = (0..n).map(|i| rainfall(cx.biome(i))).collect();
    let out: Vec<bool> = (0..n).map(|i| river[i] != 0 || fill[i] > surf[i]).collect();
    let acc = hydro::flow(&hg.v, w, h, &out, &rain);
    let stream_at = (n as f32 * 0.035).max(60.0);
    let mut stream = vec![false; n];
    for i in 0..n {
        if !out[i] && !fixed[i] && acc[i] >= stream_at && rainfall(cx.biome(i)) >= 0.5 {
            stream[i] = true;
        }
    }
    // The world's brooks, along their courses on the tiles.
    if let Some(rt) = rt {
        let brook = region::brooks(cx, rt);
        for i in 0..n {
            if brook[i] && !out[i] && !fixed[i] && !sea[i] {
                stream[i] = true;
            }
        }
    }

    // Every piece of land within reach.
    let mut wet: Vec<Wet> = (0..n)
        .map(|i| {
            if fixed_wet[i] {
                Wet::Lake
            } else if river[i] == 6 {
                Wet::River
            } else if fill[i] > surf[i] {
                Wet::Lake
            } else {
                Wet::Dry
            }
        })
        .collect();
    let lowest: Vec<i32> = (0..n)
        .map(|i| {
            let (x, y) = (i as i32 % w, i as i32 / w);
            let mut lo = cx.lo;
            for &(dx, dy) in crate::geom::DIRS8.iter() {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && ny >= 0 && nx < w && ny < h {
                    let j = (ny * w + nx) as usize;
                    if fill[j] > surf[j] {
                        lo = lo.max(fill[j]);
                    }
                    if river[j] != 0 {
                        lo = lo.max(bed[j] + 1);
                    }
                }
            }
            lo
        })
        .collect();
    let mut pass = vec![false; n];
    for c in connect::join(&mut surf, &mut wet, &brim, &lowest, &fixed, w, h) {
        match c {
            connect::Change::Ford(i) => {
                river[i] = 2;
                surf[i] = bed[i] + 1;
            }
            connect::Change::Causeway(i) => {
                fill[i] = 0;
                pass[i] = true;
            }
            connect::Change::Path(i) => pass[i] = true,
        }
    }

    // A grown map: whatever the joining-up did, dry land beside the old
    // edge stays within a level of it.
    if keep.is_some() {
        let mut order: Vec<usize> = (0..n).filter(|&i| !fixed[i] && seam[i] <= 6 && river[i] == 0 && fill[i] <= surf[i]).collect();
        order.sort_by_key(|&i| seam[i]);
        for &i in &order {
            let (x, y) = (i as i32 % w, i as i32 / w);
            let (mut lo, mut hi) = (i32::MIN, i32::MAX);
            for &(ddx, ddy) in crate::geom::DIRS8.iter() {
                let (nx, ny) = (x + ddx, y + ddy);
                if nx >= 0 && ny >= 0 && nx < w && ny < h {
                    let j = (ny * w + nx) as usize;
                    if seam[j] < seam[i] {
                        lo = lo.max(surf[j] - 1);
                        hi = hi.min(surf[j] + 1);
                    }
                }
            }
            if lo != i32::MIN {
                surf[i] = if lo <= hi { surf[i].clamp(lo, hi) } else { lo };
            }
        }
    }

    // How wet the ground is: run-off, and water close by.
    let mut moist = vec![0.0f32; n];
    let mut water_d = vec![u16::MAX; n];
    let mut queue = std::collections::VecDeque::new();
    for i in 0..n {
        if river[i] != 0 || fill[i] > surf[i] || stream[i] {
            water_d[i] = 0;
            queue.push_back(i);
        }
    }
    while let Some(i) = queue.pop_front() {
        let (x, y) = (i as i32 % w, i as i32 / w);
        for &(dx, dy) in crate::geom::DIRS8.iter() {
            let (nx, ny) = (x + dx, y + dy);
            if nx >= 0 && ny >= 0 && nx < w && ny < h {
                let j = (ny * w + nx) as usize;
                if water_d[j] == u16::MAX && water_d[i] < 12 {
                    water_d[j] = water_d[i] + 1;
                    queue.push_back(j);
                }
            }
        }
    }
    for i in 0..n {
        let near_water = if water_d[i] == u16::MAX { 0.0 } else { 1.0 - water_d[i] as f32 / 13.0 };
        let runoff = ((acc[i] / rain[i].max(0.05)).ln() / (stream_at.ln() + 1.0)).clamp(0.0, 1.0);
        moist[i] = (0.55 * near_water + 0.45 * runoff).clamp(0.0, 1.0);
    }

    let water_level = if sea.iter().any(|&s| s) {
        cx.sea
    } else {
        lakes.iter().max_by_key(|l| l.0).map_or(0, |l| l.1)
    };
    let courses = net
        .reaches
        .iter()
        .map(|r| {
            let mut v: Vec<(i32, i32, i32)> = Vec::new();
            for s in &r.samples {
                let (x, y) = (s.x.round() as i32, s.y.round() as i32);
                if x >= 0 && y >= 0 && x < w && y < h && v.last().map_or(true, |l| (l.0, l.1) != (x, y)) {
                    v.push((x, y, s.level.floor() as i32));
                }
            }
            v
        })
        .collect();
    Shape { surf, fill, river, stream, sea, hard: hard.v, alluvium: wear.deposit.v, scree: scree.v, moist, water_level, pass, courses, zmap: zm }
}

/// Fit the land between the sea (or the lowest level) and the sky: lifted
/// if it lies too low, its relief pressed flatter if it stands too tall,
/// and dry land kept above the sea, flattening into a shore beside it.
fn fit(hg: &mut Grid, sea: &[bool], shore: &[f32], cx: &Ctx, mapped: bool) {
    let any_sea = sea.iter().any(|&s| s);
    let mut land: Vec<f32> = (0..hg.v.len()).filter(|&i| !sea[i]).map(|i| hg.v[i]).collect();
    if land.is_empty() {
        return;
    }
    land.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let low = land[land.len() / 50];
    let high = *land.last().unwrap();
    let lo_w = if any_sea { cx.sea as f32 + 1.0 } else { cx.lo as f32 + 1.0 };
    let hi_w = (cx.top - 2) as f32;
    // Land from region tiles comes already fitted (see `region::zmap`),
    // but for easing onto the shore.
    let lift = if mapped { 0.0 } else { (lo_w - low).max(0.0) };
    let high = high + lift;
    let k = if high > hi_w && !mapped { (hi_w - lo_w) / (high - lo_w) } else { 1.0 };
    for i in 0..hg.v.len() {
        let mut z = hg.v[i] + lift;
        if z > lo_w {
            z = lo_w + (z - lo_w) * k;
        }
        if !sea[i] && any_sea {
            // Never below the sea, easing onto the shore.
            let f = cx.sea as f32 + 0.6 + (shore[i] * 0.08).min(0.6);
            let x = z - f;
            z = f + if x > 4.0 { x } else { (1.0 + x.exp()).ln() };
        }
        hg.v[i] = z;
    }
}

fn cx_noise(n: &crate::noise::Noise, ax: i32, ay: i32) -> f32 {
    (fbm(n, ax as f64 / 9.0, ay as f64 / 9.0, 3) * 0.5 + 0.5) as f32
}

/// Each column's distance from the shore (8-way steps): positive on land,
/// negative out at sea.
fn shore_distance(sea: &[bool], w: i32, h: i32) -> Vec<f32> {
    let n = (w * h) as usize;
    let mut d = vec![i32::MAX; n];
    let mut queue = std::collections::VecDeque::new();
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            let edge = crate::geom::DIRS8.iter().any(|&(dx, dy)| {
                let (nx, ny) = (x + dx, y + dy);
                nx >= 0 && ny >= 0 && nx < w && ny < h && sea[(ny * w + nx) as usize] != sea[i]
            });
            if edge {
                d[i] = 0;
                queue.push_back(i);
            }
        }
    }
    while let Some(i) = queue.pop_front() {
        let (x, y) = (i as i32 % w, i as i32 / w);
        for &(dx, dy) in crate::geom::DIRS8.iter() {
            let (nx, ny) = (x + dx, y + dy);
            if nx >= 0 && ny >= 0 && nx < w && ny < h {
                let j = (ny * w + nx) as usize;
                if d[j] == i32::MAX {
                    d[j] = d[i] + 1;
                    queue.push_back(j);
                }
            }
        }
    }
    (0..n)
        .map(|i| {
            let v = if d[i] == i32::MAX { 99 } else { d[i] } as f32;
            if sea[i] {
                -v - 0.5
            } else {
                v + 0.5
            }
        })
        .collect()
}
