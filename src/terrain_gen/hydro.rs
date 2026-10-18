//! Where the water goes once the land is shaped: every column drains
//! toward the map's edge, a river or the sea (hollows fill on the way, as
//! water fills them), so the flow piling up along the way marks the
//! streams, and the hollows that hold their water become ponds and lakes.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Total-ordered float for the heap.
#[derive(Clone, Copy, PartialEq, PartialOrd)]
struct F(f32);
impl Eq for F {}
impl Ord for F {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.0.partial_cmp(&o.0).unwrap_or(std::cmp::Ordering::Equal)
    }
}

const N8: [(i32, i32); 8] = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];

/// Drainage over a float heightmap: priority-flood from the outlets (the
/// map's edges and any `outlet` column) so every column has somewhere
/// downhill to send its water. Returns how much water passes through each
/// column (in columns' worth of rain, weighted by `rain`).
pub fn flow(hgt: &[f32], w: i32, h: i32, outlet: &[bool], rain: &[f32]) -> Vec<f32> {
    let n = (w * h) as usize;
    let mut seen = vec![false; n];
    let mut parent = vec![u32::MAX; n];
    let mut order: Vec<u32> = Vec::with_capacity(n);
    let mut heap: BinaryHeap<Reverse<(F, u32)>> = BinaryHeap::new();
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            if x == 0 || y == 0 || x == w - 1 || y == h - 1 || outlet[i] {
                seen[i] = true;
                heap.push(Reverse((F(hgt[i]), i as u32)));
            }
        }
    }
    let mut fill = hgt.to_vec();
    while let Some(Reverse((F(lv), i))) = heap.pop() {
        order.push(i);
        let (x, y) = (i as i32 % w, i as i32 / w);
        for &(dx, dy) in N8.iter() {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= w || ny >= h {
                continue;
            }
            let j = (ny * w + nx) as usize;
            if seen[j] {
                continue;
            }
            seen[j] = true;
            parent[j] = i;
            // A hollow fills to its brim, with a tilt so water on the flat
            // still finds its way out.
            fill[j] = fill[j].max(lv + 1e-4);
            heap.push(Reverse((F(fill[j]), j as u32)));
        }
    }
    let mut acc: Vec<f32> = rain.to_vec();
    for &i in order.iter().rev() {
        let p = parent[i as usize];
        if p != u32::MAX {
            acc[p as usize] += acc[i as usize];
        }
    }
    acc
}

/// The brim of every hollow on a float heightmap (each column's level
/// once hollows are filled to spilling), draining to the map's edges.
pub fn brims_f(hgt: &[f32], w: i32, h: i32) -> Vec<f32> {
    let n = (w * h) as usize;
    let mut seen = vec![false; n];
    let mut fill = hgt.to_vec();
    let mut heap: BinaryHeap<Reverse<(F, u32)>> = BinaryHeap::new();
    for y in 0..h {
        for x in 0..w {
            if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
                let i = (y * w + x) as usize;
                seen[i] = true;
                heap.push(Reverse((F(hgt[i]), i as u32)));
            }
        }
    }
    while let Some(Reverse((F(lv), i))) = heap.pop() {
        let (x, y) = (i as i32 % w, i as i32 / w);
        for &(dx, dy) in N8.iter() {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= w || ny >= h {
                continue;
            }
            let j = (ny * w + nx) as usize;
            if !seen[j] {
                seen[j] = true;
                fill[j] = fill[j].max(lv);
                heap.push(Reverse((F(fill[j]), j as u32)));
            }
        }
    }
    fill
}

/// Fill the hollows of an integer surface from its outlets: each column's
/// brim (the level water standing there would reach before spilling).
/// `outlet` columns drain at `outlet_level`.
pub fn brims(surf: &[i32], w: i32, h: i32, outlet_level: &[Option<i32>]) -> Vec<i32> {
    let n = (w * h) as usize;
    let mut seen = vec![false; n];
    let mut fill = surf.to_vec();
    let mut heap: BinaryHeap<Reverse<(i32, u32)>> = BinaryHeap::new();
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            if let Some(l) = outlet_level[i] {
                seen[i] = true;
                fill[i] = l.max(surf[i]);
                heap.push(Reverse((fill[i], i as u32)));
            } else if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
                seen[i] = true;
                heap.push(Reverse((surf[i], i as u32)));
            }
        }
    }
    while let Some(Reverse((lv, i))) = heap.pop() {
        let (x, y) = (i as i32 % w, i as i32 / w);
        for &(dx, dy) in N8.iter() {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= w || ny >= h {
                continue;
            }
            let j = (ny * w + nx) as usize;
            if seen[j] {
                continue;
            }
            seen[j] = true;
            fill[j] = surf[j].max(lv);
            heap.push(Reverse((fill[j], j as u32)));
        }
    }
    fill
}

/// The hollows: groups of touching columns under their brim, each with
/// its columns, its brim level and its deepest point.
pub struct Hollow {
    pub cells: Vec<usize>,
    pub level: i32,
    pub depth: i32,
}

pub fn hollows(surf: &[i32], fill: &[i32], w: i32, h: i32) -> Vec<Hollow> {
    let n = (w * h) as usize;
    let mut done = vec![false; n];
    let mut out = Vec::new();
    for s in 0..n {
        if done[s] || fill[s] <= surf[s] {
            continue;
        }
        let level = fill[s];
        let mut cells = vec![s];
        done[s] = true;
        let mut k = 0;
        while k < cells.len() {
            let i = cells[k];
            k += 1;
            let (x, y) = (i as i32 % w, i as i32 / w);
            for &(dx, dy) in N8.iter() {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                let j = (ny * w + nx) as usize;
                if !done[j] && fill[j] > surf[j] && fill[j] == level {
                    done[j] = true;
                    cells.push(j);
                }
            }
        }
        let depth = cells.iter().map(|&i| level - surf[i]).max().unwrap_or(0);
        out.push(Hollow { cells, level, depth });
    }
    out
}
