//! Constructions: walls, floors, stairs, ramps and fortifications built from
//! stone, blocks or logs, and taken down again.

use super::{Game, Travel, SEARCH_LIMIT};
use crate::dwarf::{Dwarf, Labor, Task};
use crate::geom::{box_iter, Pos, DIRS8};
use crate::items::{Item, ItemKind};
use crate::path;
use crate::world::{Desig, Mover, TileKind};
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
        // (Smoothing or engraving waiting there gives way to building.)
        let detail = matches!(t.desig, crate::world::Desig::Smooth | crate::world::Desig::Engrave);
        if !t.revealed || t.desig != crate::world::Desig::None && !detail || t.building != 0 || t.water > 0 {
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
            // Walls and fortifications also go up on top of a wall, or out
            // over open space beside something to build against (a tower's
            // next storey, battlements along a wall top).
            ConstructKind::Wall | ConstructKind::Fortification => {
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
                if self.world.t(p).desig != crate::world::Desig::None && !self.claimed.contains(&p) {
                    self.designate(p, p, crate::world::Desig::None);
                }
                let id = self.new_id();
                self.constructions.insert(id, Construction { id, pos: p, kind, mat, worker: None, retry_tick: 0 });
                n += 1;
            }
        }
        n
    }

    /// Which stair a tile of a stair column from `lo` up to `hi` gets (O11):
    /// up at the foot, down at the head, both between, so the shaft doesn't
    /// end in a stair down into rock. A stair already there, or one dug or
    /// planned just above or below the box, keeps its way through.
    pub(super) fn column_stair(&self, p: Pos, lo: i32, hi: i32) -> Desig {
        let w = &self.world;
        let goes = |q: Pos, ways: [TileKind; 2], desigs: [Desig; 2]| {
            w.get(q).map_or(false, |t| ways.contains(&t.kind) || desigs.contains(&t.desig))
                || self.construction_at(q).map_or(false, |c| c.kind.tile().map_or(false, |k| ways.contains(&k)))
        };
        let here = w.kind(p);
        let up = p.z < hi
            || matches!(here, Some(TileKind::StairUp | TileKind::StairUD))
            || goes(p.up(), [TileKind::StairDown, TileKind::StairUD], [Desig::StairDown, Desig::StairUD]);
        let down = p.z > lo
            || matches!(here, Some(TileKind::StairDown | TileKind::StairUD))
            || goes(p.down(), [TileKind::StairUp, TileKind::StairUD], [Desig::StairUp, Desig::StairUD]);
        match (up, down) {
            (true, false) => Desig::StairUp,
            (false, true) => Desig::StairDown,
            _ => Desig::StairUD,
        }
    }

    /// Where a stair column crosses open air (a cavern, a shaft), or ends
    /// on a floor where an up stair can't be dug, plan a built stair of the
    /// kind `d` names instead (O11). In the air the stair below, dug or
    /// built, holds it up. True if one was planned.
    pub(super) fn plan_air_stair(&mut self, p: Pos, d: Desig, lo: i32) -> bool {
        let kind = match d {
            Desig::StairUp => ConstructKind::StairUp,
            Desig::StairDown => ConstructKind::StairDown,
            _ => ConstructKind::StairUD,
        };
        let Some(t) = self.world.get(p) else { return false };
        let held = p.z > lo && t.kind == TileKind::Open && t.revealed && t.zone == 0 && t.desig == Desig::None && t.building == 0 && t.water == 0 && self.construction_at(p).is_none();
        if !(self.can_construct(p, kind) || held) {
            return false;
        }
        let id = self.new_id();
        self.constructions.insert(id, Construction { id, pos: p, kind, mat: BuildMat::Any, worker: None, retry_tick: 0 });
        true
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
                    if let Some(l) = super::troubles::Lack::from_desc(&c.mat.name().to_lowercase()) {
                        self.trouble(l);
                    }
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

    /// A walkable tile beside `target` that a dwarf at `from` can reach: on
    /// the same level, or one level down (beside it or right under it), so a
    /// tower's next storey, a wall top or a roof can be built from below.
    ///
    /// A wall is built from a tile that isn't itself to be walled and that
    /// won't be shut in once the walls planned around it are up (O11), so
    /// a builder ringing a yard ends outside it. Only if there's no such
    /// tile does any stand do.
    fn stand_for(&self, from: Pos, target: Pos) -> Option<Pos> {
        let w = &self.world;
        let goal = |p: Pos| {
            let beside = (p.z == target.z || p.z == target.z - 1) && (p.x - target.x).abs() <= 1 && (p.y - target.y).abs() <= 1;
            // Or straight above, on a stair down: a stair column is built from the top as well.
            let over = p == target.up() && matches!(w.kind(p), Some(TileKind::StairDown | TileKind::StairUD));
            p != target && (beside || over)
        };
        let walls = self.walls_near(target);
        if !walls.is_empty() {
            let free = |p: Pos| goal(p) && !walls.contains(&p) && !self.shut_in(p, target, &walls);
            if free(from) {
                return Some(from);
            }
            if let Some((p, _)) = path::search(&self.world, from, SEARCH_LIMIT, Mover::Dwarf, free) {
                return Some(p);
            }
        }
        if goal(from) {
            return Some(from);
        }
        path::search(&self.world, from, SEARCH_LIMIT, Mover::Dwarf, goal).map(|(p, _)| p)
    }

    /// Walls and fortifications still to be built, when `target` is one and
    /// others are planned near it (empty otherwise: nothing to be shut in by).
    fn walls_near(&self, target: Pos) -> HashSet<Pos> {
        let blocks = |k: ConstructKind| matches!(k, ConstructKind::Wall | ConstructKind::Fortification);
        if !self.construction_at(target).map_or(false, |c| blocks(c.kind)) {
            return HashSet::new();
        }
        let set: HashSet<Pos> = self.constructions.values().filter(|c| blocks(c.kind) && c.pos.z == target.z).map(|c| c.pos).collect();
        if set.iter().any(|&p| p != target && (p.x - target.x).abs() <= 3 && (p.y - target.y).abs() <= 3) {
            set
        } else {
            HashSet::new()
        }
    }

    /// Whether a dwarf at `p` would be shut in once `walls` and `target`
    /// stand: a flood fill round them gives out within 200 tiles.
    fn shut_in(&self, p: Pos, target: Pos, walls: &HashSet<Pos>) -> bool {
        let mut seen: HashSet<Pos> = HashSet::from([p]);
        let mut todo = std::collections::VecDeque::from([p]);
        let mut out = Vec::new();
        while let Some(q) = todo.pop_front() {
            if seen.len() >= 200 {
                return false;
            }
            self.world.neighbors(q, &mut out, Mover::Dwarf);
            for &(n, _) in &out {
                if n != target && !walls.contains(&n) && seen.insert(n) {
                    todo.push_back(n);
                }
            }
        }
        true
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
                // A built floor is taken up first, and its block or log comes back (O11).
                let old = *self.world.t(p);
                if let (Some(from), TileKind::Floor) = (old.built_from, old.kind) {
                    if kind != TileKind::Floor {
                        self.spawn_item(from, old.mat, d.pos);
                    }
                }
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

    /// Step until every designation is dug and every construction built.
    fn work_all(g: &mut Game, days: u64) {
        for _ in 0..days * DAY {
            g.step();
            if g.constructions.is_empty() && g.designations.is_empty() {
                return;
            }
        }
        panic!("work unfinished: {} digs, {:?}", g.designations.len(), g.constructions.values().map(|c| (c.kind, c.pos, c.worker)).collect::<Vec<_>>());
    }

    #[test]
    fn a_stair_column_has_up_at_the_bottom_and_down_at_the_top() {
        let mut g = Game::new(42);
        let e = g.embark;
        let s = (1..10).map(|i| e.offset(i, 0, 0)).find(|&p| g.world.t(p).kind == TileKind::Floor && (0..=6).all(|dz| g.world.t(p.offset(0, 0, -dz)).water == 0)).expect("no flat tile");
        let foot = s.offset(0, 0, -6);
        // Part of the way down the rock gives way to a cavity: the column crosses it on built stairs.
        let air = [s.offset(0, 0, -3), s.offset(0, 0, -4)];
        for &p in &air {
            let t = g.world.t_mut(p);
            t.kind = TileKind::Open;
            t.revealed = true;
        }
        // (Damp stone near the column or not, dig on: O10's damp stone has its own test.)
        g.damp.dig_wet_ok = true;
        g.designate(s, foot, Desig::StairUD);
        assert_eq!(g.world.t(s).desig, Desig::StairDown, "the head");
        assert_eq!(g.world.t(foot).desig, Desig::StairUp, "the foot");
        for z in [-1, -2, -5] {
            assert_eq!(g.world.t(s.offset(0, 0, z)).desig, Desig::StairUD, "between, at {}", z);
        }
        for &p in &air {
            assert_eq!(g.construction_at(p).map(|c| c.kind), Some(ConstructKind::StairUD), "no built stair in the air at {:?}", p);
        }
        work_all(&mut g, 12);
        assert_eq!(g.world.t(s).kind, TileKind::StairDown);
        assert_eq!(g.world.t(foot).kind, TileKind::StairUp, "a stair down into rock at the bottom");
        for z in -5..=-1 {
            assert_eq!(g.world.t(s.offset(0, 0, z)).kind, TileKind::StairUD, "at {}", z);
        }
        assert!(path::astar(&g.world, s, foot, 40_000, Mover::Dwarf).is_some(), "can't walk down the column");
        // Carrying the column on down keeps its old foot connected.
        let deeper = foot.offset(0, 0, -2);
        g.designate(foot, deeper, Desig::StairUD);
        assert_eq!(g.world.t(foot).desig, Desig::StairUD, "the old foot must become a through stair");
        assert_eq!(g.world.t(deeper).desig, Desig::StairUp);
    }

    #[test]
    fn a_builder_walls_a_ring_and_ends_outside_it() {
        let mut g = Game::new(31);
        g.creatures.retain(|_, c| c.tame);
        let center = g.open_patch(3, 3, 3).offset(1, 1, 0);
        // Blocks for the eight walls, and the builder starting in the middle.
        for _ in 0..8 {
            g.spawn_item(ItemKind::Block, crate::material::Mat::Granite, center.offset(3, 0, 0));
        }
        for i in 0..g.dwarves.len() {
            let d = &mut g.dwarves[i];
            if i == 0 {
                d.pos = center;
            }
            d.path.clear();
        }
        let ring: Vec<Pos> = DIRS8.iter().map(|&(dx, dy)| center.offset(dx, dy, 0)).collect();
        for &p in &ring {
            assert_eq!(g.plan_constructions(p, p, ConstructKind::Wall, BuildMat::Blocks), 1, "couldn't plan at {:?}", p);
        }
        finish_all(&mut g, 6);
        for &p in &ring {
            assert_eq!(g.world.t(p).kind, TileKind::Wall, "gap at {:?}", p);
        }
        assert!(g.dwarves.iter().all(|d| d.pos != center), "a builder walled themselves in");
    }

    #[test]
    fn a_wall_goes_up_on_a_built_floor() {
        let mut g = Game::new(32);
        let p = floor_run(&g, 1);
        let m = crate::material::Mat::Marble;
        {
            let t = g.world.t_mut(p);
            t.built_from = Some(ItemKind::Block);
            t.mat = m;
        }
        let marble = |g: &Game| g.items.values().filter(|it| it.kind == ItemKind::Block && it.mat == m).count();
        let before = marble(&g);
        assert!(g.can_construct(p, ConstructKind::Wall));
        assert_eq!(g.plan_constructions(p, p, ConstructKind::Wall, BuildMat::Wood), 1);
        finish_all(&mut g, 3);
        assert_eq!(g.world.t(p).kind, TileKind::Wall);
        assert_eq!(g.world.t(p).built_from, Some(ItemKind::Log));
        assert_eq!(marble(&g), before + 1, "the floor's block wasn't given back");
    }

    #[test]
    fn digging_takes_the_nearest_designation_first() {
        let mut g = Game::new(42);
        let a = floor_run(&g, 12);
        let (near, far) = (a.offset(3, 0, 0), a.offset(10, 0, 0));
        for p in [far, near] {
            let t = g.world.t_mut(p);
            t.kind = TileKind::Wall;
            t.mat = crate::material::Mat::Granite;
            t.built_from = None;
            assert_eq!(g.designate(p, p, Desig::Dig), 1);
        }
        let pick = g.spawn_item(ItemKind::Pick, crate::material::Mat::Iron, a);
        let mut d = g.dwarves[0].clone();
        d.pos = a;
        d.held = Some(pick);
        assert!(g.find_dig(&mut d, false));
        assert!(matches!(d.task, Task::Dig { target, .. } if target == near), "dug {:?} first", d.task);
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
