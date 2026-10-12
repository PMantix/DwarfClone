use crate::geom::Pos;
use crate::world::World;
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
pub fn search<F: FnMut(Pos) -> bool>(
    world: &World,
    start: Pos,
    limit: usize,
    mut goal: F,
) -> Option<(Pos, Vec<Pos>)> {
    let mut came: HashMap<Pos, (u32, Pos)> = HashMap::new();
    let mut heap = BinaryHeap::new();
    came.insert(start, (0, start));
    heap.push(Reverse((0u32, start)));
    let mut nb = Vec::with_capacity(16);
    let mut expanded = 0;
    while let Some(Reverse((cost, p))) = heap.pop() {
        if came[&p].0 < cost {
            continue;
        }
        if goal(p) {
            return Some((p, rebuild(&came, start, p)));
        }
        expanded += 1;
        if expanded > limit {
            return None;
        }
        world.neighbors(p, &mut nb);
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
pub fn astar(world: &World, start: Pos, goal: Pos, limit: usize) -> Option<Vec<Pos>> {
    if start == goal {
        return Some(Vec::new());
    }
    let mut came: HashMap<Pos, (u32, Pos)> = HashMap::new();
    let mut heap = BinaryHeap::new();
    came.insert(start, (0, start));
    heap.push(Reverse((heuristic(start, goal), start)));
    let mut nb = Vec::with_capacity(16);
    let mut expanded = 0;
    while let Some(Reverse((_, p))) = heap.pop() {
        if p == goal {
            return Some(rebuild(&came, start, p));
        }
        expanded += 1;
        if expanded > limit {
            return None;
        }
        let cost = came[&p].0;
        world.neighbors(p, &mut nb);
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
