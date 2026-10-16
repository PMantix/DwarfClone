use crate::geom::Pos;
use crate::world::{Mover, World};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

fn rebuild(came: &HashMap<Pos, (u32, Pos)>, start: Pos, end: Pos) -> Vec<Pos> {
    let mut path = Vec::new();
    let mut p = end;
    while p != start {
        path.push(p);
        p = came[&p].1;
    }
    path.reverse();
    path
}

/// Dijkstra flood from `start` to the cheapest position satisfying `goal`.
/// Returns that position and the path to it (excluding `start`).
/// Searches run and nodes expanded, for finding what makes the game slow.
pub static SEARCHES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static SEARCH_NODES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static ASTARS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static ASTAR_NODES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn search<F: FnMut(Pos) -> bool>(
    world: &World,
    start: Pos,
    limit: usize,
    m: Mover,
    mut goal: F,
) -> Option<(Pos, Vec<Pos>)> {
    let mut came: HashMap<Pos, (u32, Pos)> = HashMap::new();
    let mut heap = BinaryHeap::new();
    came.insert(start, (0, start));
    heap.push(Reverse((0u32, start)));
    let mut nb = Vec::with_capacity(16);
    let mut expanded = 0;
    use std::sync::atomic::Ordering::Relaxed;
    SEARCHES.fetch_add(1, Relaxed);
    while let Some(Reverse((cost, p))) = heap.pop() {
        if came[&p].0 < cost {
            continue;
        }
        if goal(p) {
            SEARCH_NODES.fetch_add(expanded as u64, Relaxed);
            return Some((p, rebuild(&came, start, p)));
        }
        expanded += 1;
        if expanded > limit {
            SEARCH_NODES.fetch_add(expanded as u64, Relaxed);
            return None;
        }
        world.neighbors(p, &mut nb, m);
        for &(q, c) in &nb {
            let nc = cost + c;
            if came.get(&q).map_or(true, |&(oc, _)| nc < oc) {
                came.insert(q, (nc, p));
                heap.push(Reverse((nc, q)));
            }
        }
    }
    None
}

fn heuristic(a: Pos, b: Pos) -> u32 {
    let dx = (a.x - b.x).unsigned_abs();
    let dy = (a.y - b.y).unsigned_abs();
    let dz = (a.z - b.z).unsigned_abs();
    10 * dx.max(dy) + 4 * dx.min(dy) + 10 * dz
}

/// A* from `start` to `goal`. Returns the path excluding `start`.
pub fn astar(world: &World, start: Pos, goal: Pos, limit: usize, m: Mover) -> Option<Vec<Pos>> {
    if start == goal {
        return Some(Vec::new());
    }
    let mut came: HashMap<Pos, (u32, Pos)> = HashMap::new();
    let mut heap = BinaryHeap::new();
    came.insert(start, (0, start));
    heap.push(Reverse((heuristic(start, goal), start)));
    let mut nb = Vec::with_capacity(16);
    let mut expanded = 0;
    use std::sync::atomic::Ordering::Relaxed;
    ASTARS.fetch_add(1, Relaxed);
    while let Some(Reverse((_, p))) = heap.pop() {
        if p == goal {
            ASTAR_NODES.fetch_add(expanded as u64, Relaxed);
            return Some(rebuild(&came, start, p));
        }
        expanded += 1;
        if expanded > limit {
            ASTAR_NODES.fetch_add(expanded as u64, Relaxed);
            return None;
        }
        let cost = came[&p].0;
        world.neighbors(p, &mut nb, m);
        for &(q, c) in &nb {
            let nc = cost + c;
            if came.get(&q).map_or(true, |&(oc, _)| nc < oc) {
                came.insert(q, (nc, p));
                heap.push(Reverse((nc + heuristic(q, goal), q)));
            }
        }
    }
    None
}
