//! Cliffs may stand where the rock is hard, but every piece of land must
//! still be reachable on foot: a plateau ringed by cliffs, a ledge, a
//! pinnacle or a bank cut off by a gorge gets a path cut down to the rest,
//! the way a gully wears a way through a scarp (a ford across a river,
//! a causeway across a lake when nothing else will do).

use std::cmp::Reverse;
use std::collections::BinaryHeap;

const N8: [(i32, i32); 8] = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];

/// What a column holds, for walking.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Wet {
    Dry,
    /// A river's deep channel (a ford can be made).
    River,
    /// A lake or the sea, filled to its brim.
    Lake,
}

pub enum Change {
    Ford(usize),
    Causeway(usize),
}

/// Label the walkable surface: columns joined when both are dry (or
/// shallow) and their heights differ by at most one level.
pub fn components(surf: &[i32], wet: &[Wet], w: i32, h: i32) -> (Vec<u32>, Vec<usize>) {
    let n = (w * h) as usize;
    let mut label = vec![u32::MAX; n];
    let mut sizes = Vec::new();
    let mut stack = Vec::new();
    for s in 0..n {
        if label[s] != u32::MAX || wet[s] != Wet::Dry {
            continue;
        }
        let id = sizes.len() as u32;
        label[s] = id;
        stack.push(s);
        let mut size = 0;
        while let Some(i) = stack.pop() {
            size += 1;
            let (x, y) = (i as i32 % w, i as i32 / w);
            for &(dx, dy) in N8.iter() {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                let j = (ny * w + nx) as usize;
                if label[j] == u32::MAX && wet[j] == Wet::Dry && (surf[j] - surf[i]).abs() <= 1 {
                    label[j] = id;
                    stack.push(j);
                }
            }
        }
        sizes.push(size);
    }
    (label, sizes)
}

/// The pieces of a finished map's surface that can be walked between
/// (each column's top tile, joined to its neighbours a level apart at
/// most), and which piece is the largest.
pub fn surface_pieces(world: &crate::world::World) -> (Vec<u32>, u32) {
    let (w, h) = (world.w, world.h);
    let wet: Vec<Wet> = (0..w * h)
        .map(|i| {
            let p = crate::geom::Pos::new(i % w, i / w, world.surface[i as usize]);
            if world.walkable(p) {
                Wet::Dry
            } else {
                Wet::Lake
            }
        })
        .collect();
    let (label, sizes) = components(&world.surface, &wet, w, h);
    let main = (0..sizes.len()).max_by_key(|&k| sizes[k]).unwrap_or(0) as u32;
    (label, main)
}

/// Join every piece of land to the largest. `lowest` is how far each
/// column may be cut down (not below the water beside it); `fixed`
/// columns can't change. Returns the fords and causeways made.
pub fn join(surf: &mut [i32], wet: &mut [Wet], brim: &[i32], lowest: &[i32], fixed: &[bool], w: i32, h: i32) -> Vec<Change> {
    let n = (w * h) as usize;
    let mut changes = Vec::new();
    for _round in 0..8 {
        let (label, sizes) = components(surf, wet, w, h);
        if sizes.len() <= 1 {
            break;
        }
        let main = (0..sizes.len()).max_by_key(|&k| sizes[k]).unwrap() as u32;
        // Cheapest ways out from the main land: climbing a cliff costs by
        // its height, wading by the water.
        let mut dist = vec![u32::MAX; n];
        let mut from = vec![u32::MAX; n];
        let mut heap = BinaryHeap::new();
        for i in 0..n {
            if label[i] == main {
                dist[i] = 0;
                heap.push(Reverse((0u32, i as u32)));
            }
        }
        while let Some(Reverse((d, i))) = heap.pop() {
            let i = i as usize;
            if d > dist[i] {
                continue;
            }
            // Don't walk on through other land: reaching it is enough.
            if label[i] != main && label[i] != u32::MAX && d > 0 {
                continue;
            }
            let (x, y) = (i as i32 % w, i as i32 / w);
            for &(dx, dy) in N8.iter() {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx >= w || ny >= h {
                    continue;
                }
                let j = (ny * w + nx) as usize;
                let step = (surf[j] - surf[i]).abs();
                if fixed[j] && step > 1 {
                    continue;
                }
                let mut c = if dx != 0 && dy != 0 { 14 } else { 10 };
                c += 40 * (step - 1).max(0) as u32;
                match wet[j] {
                    Wet::River => c += 120,
                    Wet::Lake => c += 600,
                    Wet::Dry => {}
                }
                let nd = d + c;
                if nd < dist[j] {
                    dist[j] = nd;
                    from[j] = i as u32;
                    heap.push(Reverse((nd, j as u32)));
                }
            }
        }
        // The nearest point of each other piece, and the way to it.
        let mut best: Vec<(u32, usize)> = vec![(u32::MAX, 0); sizes.len()];
        for i in 0..n {
            let l = label[i];
            if l != u32::MAX && l != main && dist[i] < best[l as usize].0 {
                best[l as usize] = (dist[i], i);
            }
        }
        let mut cut = false;
        for (k, &(d, end)) in best.iter().enumerate() {
            if k as u32 == main || d == u32::MAX {
                continue;
            }
            let mut path = vec![end];
            let mut c = end;
            while from[c] != u32::MAX {
                c = from[c] as usize;
                path.push(c);
            }
            // A slope of one level a step, cutting down the higher ground.
            let mut t: Vec<i32> = path.iter().map(|&i| surf[i]).collect();
            for i in 1..t.len() {
                t[i] = t[i].min(t[i - 1] + 1);
            }
            for i in (0..t.len().saturating_sub(1)).rev() {
                t[i] = t[i].min(t[i + 1] + 1);
            }
            for (pi, &i) in path.iter().enumerate() {
                if fixed[i] {
                    continue;
                }
                match wet[i] {
                    Wet::River => {
                        wet[i] = Wet::Dry;
                        changes.push(Change::Ford(i));
                        cut = true;
                    }
                    Wet::Lake => {
                        wet[i] = Wet::Dry;
                        surf[i] = brim[i];
                        changes.push(Change::Causeway(i));
                        cut = true;
                    }
                    Wet::Dry => {
                        let nt = t[pi].max(lowest[i]);
                        if nt < surf[i] {
                            surf[i] = nt;
                            cut = true;
                        }
                    }
                }
            }
        }
        if !cut {
            break;
        }
    }
    changes
}
