//! The erosion phase and the world's rivers: temporary rivers from the
//! bases of the mountains wear the land and open channels to the sea; then
//! the depressions are filled (lakes where they're deep), the water is
//! routed downhill to the sea or a lake, its flow summed, and the cells it
//! gathers in become rivers (named, with their tributaries) and brooks.

use super::fields::{HILLS, SEA};
use crate::rng::Rng;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

const DIRS: [(i32, i32, u8); 4] = [(0, -1, 1), (1, 0, 2), (0, 1, 4), (-1, 0, 8)];

/// Flow (summed rain) at which water is a river, and a brook.
pub const RIVER_FLOW: f32 = 45.0;
pub const BROOK_FLOW: f32 = 6.0;

/// Wear the land with temporary rivers run from every cell at or above
/// the hills line: each steps to its lowest neighbour, wearing the cells it
/// passes, and digs through when every neighbour is higher, until it
/// reaches the sea or the world's edge (or gives up).
#[cfg_attr(not(test), allow(dead_code))]
pub fn erode(elev: &mut [f32], w: i32, h: i32, seed: u64) {
    erode_traced(elev, w, h, seed, None)
}

/// `erode`, noting each trial river's cells in `paths` when given (the
/// wearing is the same either way).
pub fn erode_traced(elev: &mut [f32], w: i32, h: i32, seed: u64, mut paths: Option<&mut Vec<Vec<u32>>>) {
    let mut rng = Rng::new(seed ^ 0xE205_10E0);
    let idx = |x: i32, y: i32| (y * w + x) as usize;
    for _pass in 0..2 {
        let mut starts: Vec<usize> = (0..elev.len()).filter(|&i| elev[i] >= HILLS).collect();
        // Shuffle, so no corner of the world wears first.
        for k in (1..starts.len()).rev() {
            let j = rng.range(0, k as i32 + 1) as usize;
            starts.swap(k, j);
        }
        for &s in &starts {
            if !rng.chance(0.5) {
                continue;
            }
            let (mut x, mut y) = (s as i32 % w, s as i32 / w);
            let mut path: Vec<u32> = Vec::new();
            for _ in 0..300 {
                let i = idx(x, y);
                if paths.is_some() {
                    path.push(i as u32);
                }
                if elev[i] < SEA || x == 0 || y == 0 || x == w - 1 || y == h - 1 {
                    break;
                }
                let mut best = None;
                for &(dx, dy, _) in DIRS.iter() {
                    let j = idx(x + dx, y + dy);
                    let v = elev[j] + rng.f64() as f32 * 0.01;
                    if best.map_or(true, |(_, b)| v < b) {
                        best = Some(((x + dx, y + dy), v));
                    }
                }
                let ((nx, ny), _) = best.unwrap();
                let j = idx(nx, ny);
                if elev[j] < elev[i] {
                    // Wear the cell the water leaves, more on the heights.
                    let drop = elev[i] - elev[j];
                    elev[i] -= (drop * 0.25).min(if elev[i] >= HILLS { 1.2 } else { 0.5 });
                } else {
                    // Stuck in a hollow: dig through the lowest side.
                    elev[j] = elev[i] - 0.3;
                }
                x = nx;
                y = ny;
            }
            if let Some(p) = paths.as_mut() {
                p.push(path);
            }
        }
    }
}

/// Where each cell's water goes and the level it spills at: a priority
/// flood from the sea. `recv[i]` is the neighbour it drains to (-1 for the
/// sea itself); `spill[i]` never rises along the way.
pub fn route(elev: &[f32], w: i32, h: i32) -> (Vec<f32>, Vec<i32>) {
    let n = elev.len();
    let mut spill = elev.to_vec();
    let mut recv = vec![-1i32; n];
    let mut done = vec![false; n];
    let key = |v: f32| (v * 4096.0) as i64;
    let mut heap = BinaryHeap::new();
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            if elev[i] < SEA {
                done[i] = true;
                heap.push(Reverse((key(spill[i]), i)));
            }
        }
    }
    while let Some(Reverse((_, i))) = heap.pop() {
        let (x, y) = (i as i32 % w, i as i32 / w);
        for &(dx, dy, _) in DIRS.iter() {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= w || ny >= h {
                continue;
            }
            let j = (ny * w + nx) as usize;
            if done[j] {
                continue;
            }
            done[j] = true;
            // A little above whatever it drains into, so the way down is strict.
            spill[j] = elev[j].max(spill[i] + 0.002);
            recv[j] = i as i32;
            heap.push(Reverse((key(spill[j]), j)));
        }
    }
    (spill, recv)
}

/// The direction bit from cell `a` to its neighbour `b`, and back.
pub fn link(w: i32, a: usize, b: usize) -> (u8, u8) {
    let (ax, ay, bx, by) = (a as i32 % w, a as i32 / w, b as i32 % w, b as i32 / w);
    let &(_, _, out) = DIRS.iter().find(|d| (d.0, d.1) == (bx - ax, by - ay)).expect("neighbours");
    let back = match out {
        1 => 4,
        2 => 8,
        4 => 1,
        _ => 2,
    };
    (out, back)
}

/// The water of the world, cell by cell.
pub struct Water {
    /// Where each cell drains to (-1: the sea or off the world).
    pub recv: Vec<i32>,
    /// Summed rain from everything upstream.
    pub flow: Vec<f32>,
    /// Lake number + 1 for lake cells, 0 elsewhere.
    pub lake: Vec<u32>,
    /// Lakes with no outflow (dry basins, where the water evaporates).
    pub terminal: Vec<bool>,
    pub river: Vec<bool>,
    pub brook: Vec<bool>,
}

/// Lakes in the deep hollows, then the flow. `elev` is the filled surface
/// (lake cells at their water level); `depth` how deep each hollow was.
pub fn water(elev: &[f32], depth: &[f32], recv: Vec<i32>, rain: &[f32], w: i32, h: i32) -> Water {
    let n = elev.len();
    // Hollows: connected cells filled more than a little.
    let mut lake = vec![0u32; n];
    let mut comp = vec![u32::MAX; n];
    let mut lakes: Vec<Vec<usize>> = Vec::new();
    for s in 0..n {
        if comp[s] != u32::MAX || depth[s] <= 0.5 || elev[s] < SEA {
            continue;
        }
        let id = lakes.len() as u32;
        let mut cells = vec![s];
        comp[s] = id;
        let mut k = 0;
        while k < cells.len() {
            let i = cells[k];
            k += 1;
            let (x, y) = (i as i32 % w, i as i32 / w);
            for &(dx, dy, _) in DIRS.iter() {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && ny >= 0 && nx < w && ny < h {
                    let j = (ny * w + nx) as usize;
                    if comp[j] == u32::MAX && depth[j] > 0.5 && elev[j] >= SEA {
                        comp[j] = id;
                        cells.push(j);
                    }
                }
            }
        }
        lakes.push(cells);
    }
    // Only the deepest hollows hold a lake (a few to a world); the rest
    // silt up level with their rims.
    let mut deep: Vec<(f32, Vec<usize>)> = lakes
        .into_iter()
        .map(|cells| (cells.iter().map(|&i| depth[i]).fold(0.0f32, f32::max), cells))
        .filter(|(d, cells)| *d >= 4.0 && (cells.len() >= 2 || *d >= 8.0))
        .collect();
    deep.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    deep.truncate((n / 1200).max(3));
    let mut kept: Vec<Vec<usize>> = Vec::new();
    for (_, cells) in deep {
        for &i in &cells {
            lake[i] = kept.len() as u32 + 1;
        }
        kept.push(cells);
    }

    // Catchment of each lake (for the dry ones), and the flow.
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| elev[b].partial_cmp(&elev[a]).unwrap().then(a.cmp(&b)));
    let unit = |i: usize| if elev[i] < SEA { 0.0 } else { 0.05 + rain[i] / 100.0 };
    // Rain over each lake's catchment, to tell the dry basins.
    let mut wet = vec![0.0f32; n];
    let mut area = vec![0.0f32; n];
    for &i in &order {
        wet[i] += rain[i];
        area[i] += 1.0;
        let r = recv[i];
        if r >= 0 && lake[i] == 0 {
            wet[r as usize] += wet[i];
            area[r as usize] += area[i];
        } else if r >= 0 {
            wet[r as usize] += wet[i];
            area[r as usize] += area[i];
        }
    }
    let mut terminal = vec![false; kept.len()];
    for (k, cells) in kept.iter().enumerate() {
        // The outlet: the lake cell draining out of the lake.
        if let Some(&o) = cells.iter().find(|&&i| recv[i] < 0 || lake[recv[i] as usize] != k as u32 + 1) {
            terminal[k] = wet[o] / area[o].max(1.0) < 20.0;
        }
    }
    let mut flow: Vec<f32> = (0..n).map(unit).collect();
    for &i in &order {
        let r = recv[i];
        if r < 0 {
            continue;
        }
        let r = r as usize;
        let leaving = lake[i] != 0 && lake[r] != lake[i];
        if leaving && terminal[lake[i] as usize - 1] {
            continue;
        }
        flow[r] += flow[i];
    }
    let land = |i: usize| elev[i] >= SEA && lake[i] == 0;
    let river: Vec<bool> = (0..n).map(|i| land(i) && flow[i] >= RIVER_FLOW).collect();
    let brook: Vec<bool> = (0..n).map(|i| land(i) && !river[i] && flow[i] >= BROOK_FLOW && rain[i] >= 20.0).collect();
    Water { recv, flow, lake, terminal, river, brook }
}

/// A river from its spring to where it ends.
pub struct Stem {
    /// Spring first.
    pub course: Vec<usize>,
    /// The cell it runs into: the sea, a lake, or the river it joins.
    pub into: i32,
    /// The stem it joins (an index into the list), if a tributary.
    pub joins: Option<usize>,
}

/// Split the rivers into stems: from each mouth the main stem follows the
/// biggest branch upstream; every other branch is a tributary.
pub fn stems(water: &Water) -> Vec<Stem> {
    let n = water.recv.len();
    let mut donors: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 0..n {
        if water.river[i] && water.recv[i] >= 0 {
            donors[water.recv[i] as usize].push(i);
        }
    }
    let mut out: Vec<Stem> = Vec::new();
    // (lowest cell, the cell it runs into, the stem it joins)
    let mut pending: Vec<(usize, i32, Option<usize>)> = (0..n)
        .filter(|&i| water.river[i] && (water.recv[i] < 0 || !water.river[water.recv[i] as usize]))
        .map(|i| (i, water.recv[i], None))
        .collect();
    // Biggest mouths first, so the great rivers get the first names.
    pending.sort_by(|a, b| water.flow[b.0].partial_cmp(&water.flow[a.0]).unwrap().then(a.0.cmp(&b.0)));
    pending.reverse();
    while let Some((low, into, joins)) = pending.pop() {
        let me = out.len();
        let mut course = vec![low];
        let mut cur = low;
        loop {
            let mut ds: Vec<usize> = donors[cur].clone();
            if ds.is_empty() {
                break;
            }
            ds.sort_by(|&a, &b| water.flow[b].partial_cmp(&water.flow[a]).unwrap().then(a.cmp(&b)));
            for &t in &ds[1..] {
                pending.insert(0, (t, cur as i32, Some(me)));
            }
            cur = ds[0];
            course.push(cur);
        }
        course.reverse();
        out.push(Stem { course, into, joins });
    }
    out
}
