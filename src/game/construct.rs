//! Constructions: walls, floors, stairs, ramps and fortifications built from
//! stone, blocks or logs, and taken down again.

use super::{Game, Travel, SEARCH_LIMIT};
use crate::dwarf::{Dwarf, Labor, Task};
use crate::geom::{box_iter, Pos, DIRS8};
use crate::items::{Item, ItemKind};
use crate::path;
use crate::world::{Mover, TileKind};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstructKind {
    Wall,
    Floor,
    StairUp,
    StairDown,
    StairUD,
    Ramp,
    Fortification,
    /// Take down an existing construction and get the material back.
    Remove,
}

impl ConstructKind {
    pub const ALL: [ConstructKind; 8] = [
        ConstructKind::Wall,
        ConstructKind::Floor,
        ConstructKind::StairUp,
        ConstructKind::StairDown,
        ConstructKind::StairUD,
        ConstructKind::Ramp,
        ConstructKind::Fortification,
        ConstructKind::Remove,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ConstructKind::Wall => "Wall",
            ConstructKind::Floor => "Floor",
            ConstructKind::StairUp => "Up stair",
            ConstructKind::StairDown => "Down stair",
            ConstructKind::StairUD => "Up/down stair",
            ConstructKind::Ramp => "Ramp",
            ConstructKind::Fortification => "Fortification",
            ConstructKind::Remove => "Remove construction",
        }
    }

    pub fn key(self) -> char {
        match self {
            ConstructKind::Wall => 'w',
            ConstructKind::Floor => 'f',
            ConstructKind::StairUp => 'u',
            ConstructKind::StairDown => 'j',
            ConstructKind::StairUD => 'i',
            ConstructKind::Ramp => 'r',
            ConstructKind::Fortification => 'F',
            ConstructKind::Remove => 'x',
        }
    }

    /// The tile it turns into once built.
    pub fn tile(self) -> Option<TileKind> {
        match self {
            ConstructKind::Wall => Some(TileKind::Wall),
            ConstructKind::Floor => Some(TileKind::Floor),
            ConstructKind::StairUp => Some(TileKind::StairUp),
            ConstructKind::StairDown => Some(TileKind::StairDown),
            ConstructKind::StairUD => Some(TileKind::StairUD),
            ConstructKind::Ramp => Some(TileKind::Ramp),
            ConstructKind::Fortification => Some(TileKind::Fortification),
            ConstructKind::Remove => None,
        }
    }

    /// Map glyph for a planned construction.
    pub fn glyph(self) -> char {
        match self {
            ConstructKind::Wall => '█',
            ConstructKind::Floor => '+',
            ConstructKind::StairUp => '<',
            ConstructKind::StairDown => '>',
            ConstructKind::StairUD => 'X',
            ConstructKind::Ramp => '▲',
            ConstructKind::Fortification => '#',
            ConstructKind::Remove => 'x',
        }
    }
}

/// Which materials a construction may be built from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuildMat {
    Any,
    Stone,
    Blocks,
    Wood,
}

impl BuildMat {
    pub fn name(self) -> &'static str {
        match self {
            BuildMat::Any => "any material",
            BuildMat::Stone => "rough stone",
            BuildMat::Blocks => "stone blocks",
            BuildMat::Wood => "logs",
        }
    }

    pub fn next(self) -> BuildMat {
        match self {
            BuildMat::Any => BuildMat::Stone,
            BuildMat::Stone => BuildMat::Blocks,
            BuildMat::Blocks => BuildMat::Wood,
            BuildMat::Wood => BuildMat::Any,
        }
    }

    pub fn accepts(self, it: &Item) -> bool {
        let stone = it.kind == ItemKind::Boulder && it.mat.is_building_stone();
        let blocks = it.kind == ItemKind::Block;
        let wood = it.kind == ItemKind::Log;
        !it.dump
            && !it.to_depot
            && match self {
                BuildMat::Any => stone || blocks || wood,
                BuildMat::Stone => stone,
                BuildMat::Blocks => blocks,
                BuildMat::Wood => wood,
            }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Construction {
    pub id: u32,
    pub pos: Pos,
    pub kind: ConstructKind,
    pub mat: BuildMat,
    pub worker: Option<u32>,
    pub retry_tick: u64,
}

impl Game {
    pub fn construction_at(&self, p: Pos) -> Option<&Construction> {
        self.constructions.values().find(|c| c.pos == p)
    }

    /// Whether `kind` may be planned at `p`.
    pub fn can_construct(&self, p: Pos, kind: ConstructKind) -> bool {
        let Some(t) = self.world.get(p) else { return false };
        if !t.revealed || t.desig != crate::world::Desig::None || t.building != 0 || t.water > 0 {
            return false;
        }
        if self.construction_at(p).is_some() {
            return false;
        }
        match kind {
            ConstructKind::Remove => t.built_from.is_some(),
            ConstructKind::Floor => t.kind == TileKind::Open && self.supported(p),
            // Stairs can also climb through open space, as long as something holds them up.
            ConstructKind::StairUp | ConstructKind::StairDown | ConstructKind::StairUD => {
                t.zone == 0 && (matches!(t.kind, TileKind::Floor | TileKind::Shrub) || (t.kind == TileKind::Open && self.supported(p)))
            }
            _ => t.zone == 0 && matches!(t.kind, TileKind::Floor | TileKind::Shrub),
        }
    }

    /// Something to build against: a wall, floor or stair beside or below.
    fn supported(&self, p: Pos) -> bool {
        let w = &self.world;
        w.solid(p.down())
            || w.kind(p.down()).map_or(false, |k| matches!(k, TileKind::StairUp | TileKind::StairUD))
            || DIRS8.iter().any(|&(dx, dy)| {
                let q = p.offset(dx, dy, 0);
                w.in_bounds(q) && w.kind(q).map_or(false, |k| k != TileKind::Open)
            })
    }

    /// Plan `kind` on every suitable tile in the box. Returns how many were planned.
    pub fn plan_constructions(&mut self, a: Pos, b: Pos, kind: ConstructKind, mat: BuildMat) -> usize {
        let mut n = 0;
        for p in box_iter(a, b) {
            if self.can_construct(p, kind) {
                let id = self.new_id();
                self.constructions.insert(id, Construction { id, pos: p, kind, mat, worker: None, retry_tick: 0 });
                n += 1;
            }
        }
        n
    }

    /// Cancel planned constructions in the box. Returns how many were cancelled.
    pub fn cancel_constructions(&mut self, a: Pos, b: Pos) -> usize {
        let doomed: Vec<(u32, Option<u32>)> = self
            .constructions
            .values()
            .filter(|c| box_iter(a, b).any(|p| p == c.pos))
            .map(|c| (c.id, c.worker))
            .collect();
        // A worker notices their job is gone and drops it next tick.
        for &(id, _) in &doomed {
            self.constructions.remove(&id);
        }
        doomed.len()
    }

    pub(super) fn find_build_tile(&mut self, d: &mut Dwarf) -> bool {
        let tick = self.tick;
        let pending: Vec<u32> = self
            .constructions
            .values()
            .filter(|c| c.worker.is_none() && c.retry_tick <= tick)
            .map(|c| c.id)
            .collect();
        let mut missing: HashSet<BuildMat> = HashSet::new();
        for cid in pending {
            let c = self.constructions[&cid].clone();
            if c.kind == ConstructKind::Remove {
                if self.stand_for(d.pos, c.pos).is_some() {
                    self.constructions.get_mut(&cid).unwrap().worker = Some(d.id);
                    d.task = Task::BuildTile { con: cid, item: None, stand: None };
                    return true;
                }
                self.constructions.get_mut(&cid).unwrap().retry_tick = tick + 600;
                continue;
            }
            if missing.contains(&c.mat) {
                self.constructions.get_mut(&cid).unwrap().retry_tick = tick + 300;
                continue;
            }
            match self.find_item(d.pos, |it| c.mat.accepts(it)) {
                Some((item, path)) => {
                    self.claim_item(item, d.id);
                    self.constructions.get_mut(&cid).unwrap().worker = Some(d.id);
                    d.task = Task::BuildTile { con: cid, item: Some(item), stand: None };
                    Self::set_path(d, path);
                    return true;
                }
                None => {
                    missing.insert(c.mat);
                    self.constructions.get_mut(&cid).unwrap().retry_tick = tick + 300;
                    self.warn(
                        &format!("conmat{:?}", c.mat),
                        &format!("Constructions are waiting for {}.", c.mat.name()),
                        (230, 200, 90),
                    );
                }
            }
        }
        false
    }

    /// A walkable tile beside `target` (same level) that a dwarf at `from` can reach.
    fn stand_for(&self, from: Pos, target: Pos) -> Option<Pos> {
        let goal = |p: Pos| p != target && p.z == target.z && (p.x - target.x).abs() <= 1 && (p.y - target.y).abs() <= 1;
        if goal(from) {
            return Some(from);
        }
        path::search(&self.world, from, SEARCH_LIMIT, Mover::Dwarf, goal).map(|(p, _)| p)
    }

    pub(super) fn do_build_tile(&mut self, d: &mut Dwarf, con: u32, item: Option<u32>, stand: Option<Pos>) {
        let Some(target) = self.constructions.get(&con).map(|c| c.pos) else {
            self.release_task(d);
            return;
        };
        if let Some(item) = item {
            match self.fetch(d, item) {
                None => {
                    self.release_task(d);
                    return;
                }
                Some(false) => return,
                Some(true) => {}
            }
        }
        let stand = match stand {
            Some(s) => s,
            None => match self.stand_for(d.pos, target) {
                Some(s) => {
                    d.task = Task::BuildTile { con, item, stand: Some(s) };
                    s
                }
                None => {
                    self.release_task(d);
                    if let Some(c) = self.constructions.get_mut(&con) {
                        c.retry_tick = self.tick + 600;
                    }
                    return;
                }
            },
        };
        match self.travel(d, stand) {
            Travel::Moving => {}
            Travel::Failed => {
                self.release_task(d);
                if let Some(c) = self.constructions.get_mut(&con) {
                    c.retry_tick = self.tick + 600;
                }
            }
            Travel::Arrived => {
                if Self::labor(d, 90, d.skill(Labor::Building)) {
                    self.complete_construction(d, con, item);
                    d.gain_xp(Labor::Building, 20);
                    d.task = Task::Idle;
                }
            }
        }
    }

    fn complete_construction(&mut self, d: &mut Dwarf, con: u32, item: Option<u32>) {
        let Some(c) = self.constructions.remove(&con) else { return };
        let p = c.pos;
        self.dig_version += 1;
        match c.kind.tile() {
            None => {
                // Take it down: walls and stairs leave floor, a floor leaves open space.
                let t = *self.world.t(p);
                let Some(from) = t.built_from else { return };
                let tt = self.world.t_mut(p);
                tt.kind = if t.kind == TileKind::Floor { TileKind::Open } else { TileKind::Floor };
                tt.built_from = None;
                self.spawn_item(from, t.mat, d.pos);
                if self.world.t(p).kind == TileKind::Open {
                    self.drop_items_at(p);
                }
            }
            Some(kind) => {
                let Some(it) = item.and_then(|id| self.items.remove(&id)) else { return };
                d.carrying = None;
                let tt = self.world.t_mut(p);
                tt.kind = kind;
                tt.mat = it.mat;
                tt.built_from = Some(it.kind);
                tt.farm = 0;
                if !kind.has_floor() {
                    // Nothing stays inside a wall: shove items and creatures out to the builder.
                    let out = d.pos;
                    for i in self.items.values_mut() {
                        if i.pos == p && i.carried_by.is_none() {
                            i.pos = out;
                        }
                    }
                    for o in self.dwarves.iter_mut().filter(|o| o.alive && o.pos == p) {
                        o.pos = out;
                        o.path.clear();
                    }
                    for cr in self.creatures.values_mut().filter(|cr| cr.pos == p) {
                        cr.pos = out;
                        cr.path.clear();
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::DAY;
    use crate::geom::Pos;

    /// A straight run of plain floor, `len` long, near the wagon.
    fn floor_run(g: &Game, len: i32) -> Pos {
        let e = g.embark;
        for r in 3..30 {
            for dy in -r..=r {
                for dx in -r..=r {
                    let a = e.offset(dx, dy, 0);
                    let a = Pos::new(a.x, a.y, g.world.surface_z(a.x, a.y));
                    if (0..len).all(|i| {
                        let p = a.offset(i, 0, 0);
                        g.world.get(p).map_or(false, |t| t.kind == TileKind::Floor && t.water == 0 && t.building == 0 && t.zone == 0)
                    }) {
                        return a;
                    }
                }
            }
        }
        panic!("no floor run");
    }

    fn finish_all(g: &mut Game, days: u64) {
        for _ in 0..days * DAY {
            g.step();
            if g.constructions.is_empty() {
                return;
            }
        }
        panic!("constructions unfinished: {:?}", g.constructions.values().map(|c| (c.kind, c.pos, c.worker)).collect::<Vec<_>>());
    }

    #[test]
    fn walls_block_goblins_and_come_down_again() {
        let mut g = Game::new(31);
        // A tile with level open ground all round it.
        let start = g.open_patch(3, 3, 3).offset(1, 1, 0);
        // Enclose the tile with a ring of walls built from logs.
        let ring: Vec<Pos> = DIRS8.iter().map(|&(dx, dy)| start.offset(dx, dy, 0)).collect();
        // Somewhere a goblin could come from.
        let far = (8..30)
            .flat_map(|d| [start.offset(d, 0, 0), start.offset(-d, 0, 0), start.offset(0, d, 0), start.offset(0, -d, 0)])
            .filter(|p| g.world.in_bounds(*p))
            .map(|p| Pos::new(p.x, p.y, g.world.surface_z(p.x, p.y)))
            .find(|&p| g.world.walkable(p) && path::astar(&g.world, p, start, 40_000, Mover::Goblin).is_some())
            .expect("no path to begin with");
        // Plan four of the eight walls (the wagon has logs for that); wall off the
        // rest directly so the test is only about what gets built.
        let mut planned = 0;
        for &p in &ring {
            if planned < 4 && g.plan_constructions(p, p, ConstructKind::Wall, BuildMat::Any) == 1 {
                planned += 1;
            } else if g.construction_at(p).is_none() {
                g.world.t_mut(p).kind = TileKind::Wall;
            }
        }
        assert!(planned >= 3, "couldn't plan walls");
        finish_all(&mut g, 4);
        for &p in &ring {
            assert_eq!(g.world.t(p).kind, TileKind::Wall, "gap at {:?}", p);
        }
        let built: Vec<Pos> = ring.iter().copied().filter(|&p| g.world.t(p).built_from.is_some()).collect();
        assert_eq!(built.len(), planned);
        assert!(path::astar(&g.world, far, start, 40_000, Mover::Goblin).is_none(), "goblin can still reach the walled-in tile");

        // Take one of our walls down: the path opens and the material comes back.
        let logs_before = g.items.values().filter(|it| it.kind == ItemKind::Log || it.kind == ItemKind::Boulder || it.kind == ItemKind::Block).count();
        // A corner wall: once it's gone, a diagonal step between the two walls beside it gets in.
        let w = *built.iter().find(|p| p.x != start.x && p.y != start.y).unwrap_or(&built[0]);
        assert_eq!(g.plan_constructions(w, w, ConstructKind::Remove, BuildMat::Any), 1);
        finish_all(&mut g, 2);
        assert_eq!(g.world.t(w).kind, TileKind::Floor);
        assert!(g.world.t(w).built_from.is_none());
        let logs_after = g.items.values().filter(|it| it.kind == ItemKind::Log || it.kind == ItemKind::Boulder || it.kind == ItemKind::Block).count();
        assert_eq!(logs_after, logs_before + 1);
        assert!(path::astar(&g.world, far, start, 40_000, Mover::Goblin).is_some(), "removing the wall didn't reopen the path");
    }

    #[test]
    fn floors_bridge_open_space_and_fortifications_block_walkers() {
        let mut g = Game::new(32);
        let a = floor_run(&g, 3);
        // Turn the middle tile into a hole, then floor it over.
        let mid = a.offset(1, 0, 0);
        g.world.t_mut(mid).kind = TileKind::Open;
        g.world.t_mut(mid.down()).kind = TileKind::Wall;
        assert!(g.can_construct(mid, ConstructKind::Floor));
        assert!(!g.can_construct(a, ConstructKind::Floor), "floor planned on floor");
        g.plan_constructions(mid, mid, ConstructKind::Floor, BuildMat::Any);
        finish_all(&mut g, 3);
        assert_eq!(g.world.t(mid).kind, TileKind::Floor);
        assert!(g.world.walkable(mid));

        // A fortification can't be walked through but water flows into it.
        let f = a.offset(2, 0, 0);
        assert!(g.plan_constructions(f, f, ConstructKind::Fortification, BuildMat::Any) == 1);
        finish_all(&mut g, 3);
        assert_eq!(g.world.t(f).kind, TileKind::Fortification);
        assert!(!g.world.passable(f, Mover::Dwarf) && !g.world.passable(f, Mover::Goblin));
        assert!(!g.world.solid(f), "fortification should let water through");
    }

    #[test]
    fn a_roof_makes_a_surface_room() {
        let mut g = Game::new(33);
        let p = floor_run(&g, 1);
        for &(dx, dy) in DIRS8.iter() {
            g.world.t_mut(p.offset(dx, dy, 0)).kind = TileKind::Wall;
        }
        assert!(g.room_at(p).is_none(), "open to the sky but counted as a room");
        let above = p.up();
        g.world.t_mut(above).kind = TileKind::Floor;
        g.world.t_mut(above).built_from = Some(ItemKind::Block);
        assert!(g.room_at(p).is_some(), "roofed surface room not detected");
    }
}
