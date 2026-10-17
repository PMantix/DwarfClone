use crate::geom::Pos;
use crate::world::{Mover, World};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// What a search knows of each tile it has reached: the best cost so far and
/// where it came from, in flat arrays the size of the map. A tile's entry
/// counts only if its stamp is this search's, so nothing need be cleared
/// between searches. (A hash map here was most of the cost of pathfinding.)
#[derive(Default)]
struct Came {
    stamp: Vec<u32>,
    cost: Vec<u32>,
    from: Vec<Pos>,
    now: u32,
}

impl Came {
    /// One for a search over `world`, reused from the last if there is one.
    fn take(world: &World) -> Came {
        let mut c = SPARE.with(|s| s.take()).unwrap_or_default();
        let n = (world.w * world.h * world.d) as usize;
        if c.stamp.len() != n {
            c.stamp = vec![0; n];
            c.cost = vec![0; n];
            c.from = vec![Pos::new(0, 0, 0); n];
            c.now = 0;
        }
        c.now = c.now.wrapping_add(1);
        if c.now == 0 {
            c.stamp.iter_mut().for_each(|s| *s = 0);
            c.now = 1;
        }
        c
    }

    /// Done with it: keep it for the next search on this thread. (A search
    /// begun inside another's goal gets its own, so they never clash.)
    fn give_back(self) {
        SPARE.with(|s| s.set(Some(self)));
    }

    fn get(&self, world: &World, p: Pos) -> Option<u32> {
        let i = world.idx(p);
        (self.stamp[i] == self.now).then(|| self.cost[i])
    }

    fn set(&mut self, world: &World, p: Pos, cost: u32, from: Pos) {
        let i = world.idx(p);
        self.stamp[i] = self.now;
        self.cost[i] = cost;
        self.from[i] = from;
    }

    fn path(&self, world: &World, start: Pos, end: Pos) -> Vec<Pos> {
        let mut path = Vec::new();
        let mut p = end;
        while p != start {
            path.push(p);
            p = self.from[world.idx(p)];
        }
        path.reverse();
        path
    }
}

thread_local! {
    static SPARE: std::cell::Cell<Option<Came>> = const { std::cell::Cell::new(None) };
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
    if !world.in_bounds(start) {
        return goal(start).then(|| (start, Vec::new()));
    }
    let mut came = Came::take(world);
    let mut heap = BinaryHeap::new();
    came.set(world, start, 0, start);
    heap.push(Reverse((0u32, start)));
    let mut nb = Vec::with_capacity(16);
    let mut expanded = 0;
    use std::sync::atomic::Ordering::Relaxed;
    SEARCHES.fetch_add(1, Relaxed);
    while let Some(Reverse((cost, p))) = heap.pop() {
        if came.get(world, p).map_or(false, |c| c < cost) {
            continue;
        }
        if goal(p) {
            SEARCH_NODES.fetch_add(expanded as u64, Relaxed);
            let path = came.path(world, start, p);
            came.give_back();
            return Some((p, path));
        }
        expanded += 1;
        if expanded > limit {
            SEARCH_NODES.fetch_add(expanded as u64, Relaxed);
            came.give_back();
            return None;
        }
        world.neighbors(p, &mut nb, m);
        for &(q, c) in &nb {
            let nc = cost + c;
            if came.get(world, q).map_or(true, |oc| nc < oc) {
                came.set(world, q, nc, p);
                heap.push(Reverse((nc, q)));
            }
        }
    }
    came.give_back();
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
    if !world.in_bounds(start) {
        return None;
    }
    let mut came = Came::take(world);
    let mut heap = BinaryHeap::new();
    came.set(world, start, 0, start);
    heap.push(Reverse((heuristic(start, goal), start)));
    let mut nb = Vec::with_capacity(16);
    let mut expanded = 0;
    use std::sync::atomic::Ordering::Relaxed;
    ASTARS.fetch_add(1, Relaxed);
    while let Some(Reverse((_, p))) = heap.pop() {
        if p == goal {
            ASTAR_NODES.fetch_add(expanded as u64, Relaxed);
            let path = came.path(world, start, p);
            came.give_back();
            return Some(path);
        }
        expanded += 1;
        if expanded > limit {
            ASTAR_NODES.fetch_add(expanded as u64, Relaxed);
            came.give_back();
            return None;
        }
        let cost = came.get(world, p).unwrap_or(0);
        world.neighbors(p, &mut nb, m);
        for &(q, c) in &nb {
            let nc = cost + c;
            if came.get(world, q).map_or(true, |oc| nc < oc) {
                came.set(world, q, nc, p);
                heap.push(Reverse((nc + heuristic(q, goal), q)));
            }
        }
    }
    came.give_back();
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::TileKind;

    #[test]
    fn searches_agree_however_they_are_nested() {
        let g = crate::game::Game::new(5);
        let w = &g.world;
        let from = g.embark;
        let far = |p: Pos| (p.x - from.x).abs() + (p.y - from.y).abs() >= 12 && w.walkable(p);
        let first = search(w, from, 20_000, Mover::Dwarf, far).expect("somewhere 12 tiles off");
        // Again, with the store reused: the same answer.
        let again = search(w, from, 20_000, Mover::Dwarf, far).unwrap();
        assert_eq!(first, again);
        // And with a search inside the goal, which takes a store of its own.
        let nested = search(w, from, 20_000, Mover::Dwarf, |p| far(p) && search(w, p, 50, Mover::Dwarf, |q| q == p).is_some()).unwrap();
        assert_eq!(first, nested);
        let a = astar(w, from, first.0, 20_000, Mover::Dwarf).unwrap();
        assert_eq!(a.last(), Some(&first.0));
        // From off the map: nothing, not a panic.
        assert!(search(w, Pos::new(-1, 0, 0), 100, Mover::Dwarf, |_| false).is_none());
    }

    #[test]
    fn digging_beside_the_sky_does_not_walk_the_whole_sky() {
        let mut g = crate::game::Game::new(5);
        let w = &mut g.world;
        // A buried pocket of open space, unknown, and a tile of the open air
        // beside the surface that somehow isn't known either.
        let e = g.embark;
        let deep = Pos::new(e.x, e.y, e.z - 6);
        for dz in -1..=1 {
            for dy in -2..=2 {
                for dx in -2..=2 {
                    w.t_mut(deep.offset(dx, dy, dz)).kind = TileKind::Wall;
                }
            }
        }
        for dx in -1..=1 {
            for dy in -1..=1 {
                let q = deep.offset(dx, dy, 0);
                w.t_mut(q).kind = TileKind::Open;
                w.t_mut(q).revealed = false;
            }
        }
        let walked = w.flood_reveal(deep);
        assert_eq!(walked, 9, "the pocket, all of it");
        assert!(w.t(deep.offset(1, 1, 0)).revealed && w.t(deep.offset(2, 0, 0)).revealed, "and the walls around it");
        let air = e.up();
        w.t_mut(air).revealed = false;
        let walked = w.flood_reveal(air);
        assert!(walked < 30, "only what wasn't known: {} tiles", walked);
    }
}
