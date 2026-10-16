//! Wells (clean water drawn up from below, for drinking and for patients),
//! floodgates (lever-worked gates that hold water back), and magma: the sea
//! far below, how it flows, and the workshops it heats.

use super::*;

/// How far down a well's rope reaches.
pub const WELL_DEPTH: i32 = 10;

impl Game {
    /// A well goes over a channel (open space) with somewhere to stand beside it.
    pub(super) fn can_place_well(&self, p: Pos) -> bool {
        let Some(t) = self.world.get(p) else { return false };
        t.revealed
            && t.building == 0
            && t.water == 0
            && t.kind == TileKind::Open
            && DIRS8.iter().any(|&(dx, dy)| self.world.walkable(p.offset(dx, dy, 0)))
    }

    /// Whether a bucket lowered from `p` would come up full: water lies below
    /// with nothing but open space in between.
    pub fn well_has_water(&self, p: Pos) -> bool {
        let mut q = p;
        for _ in 0..=WELL_DEPTH {
            match self.world.get(q) {
                Some(t) if t.water > 0 && !t.frozen => return true,
                Some(t) if t.kind == TileKind::Open => q = q.down(),
                _ => return false,
            }
        }
        false
    }

    /// Built wells that have water.
    pub(super) fn working_wells(&self) -> Vec<Pos> {
        self.buildings
            .values()
            .filter(|b| b.kind == BuildingKind::Well && b.built && self.well_has_water(b.pos))
            .map(|b| b.pos)
            .collect()
    }

    pub(super) fn beside_well(&self, p: Pos) -> bool {
        self.working_wells().iter().any(|w| w.z == p.z && (w.x - p.x).abs() <= 1 && (w.y - p.y).abs() <= 1)
    }

    /// Where to stand to work on `p`: the tile itself, or a walkable
    /// neighbour when it has no floor (a well over its shaft).
    pub(super) fn work_spot(&self, p: Pos) -> Pos {
        if self.world.walkable(p) {
            return p;
        }
        DIRS8.iter().map(|&(dx, dy)| p.offset(dx, dy, 0)).find(|&q| self.world.walkable(q)).unwrap_or(p)
    }

    /// A walkable neighbour of `p` to step aside to (or `p` if there's none).
    pub(super) fn work_spot_off(&self, p: Pos) -> Pos {
        DIRS8.iter().map(|&(dx, dy)| p.offset(dx, dy, 0)).find(|&q| self.world.walkable(q)).unwrap_or(p)
    }

    // ------------------------------------------------------------------ magma

    /// Every few ticks: magma falls and levels out like water, only slower.
    /// Where it meets water both turn to obsidian; what it touches burns.
    pub(super) fn magma(&mut self) {
        if self.magma_seen != self.dig_version {
            self.magma_cells = None;
            self.magma_seen = self.dig_version;
        }
        let mut cells = match self.magma_cells.take() {
            Some(c) => c,
            None => self.world.tiles.iter().enumerate().filter(|(_, t)| t.magma > 0).map(|(i, _)| i).collect(),
        };
        if cells.is_empty() {
            self.magma_cells = Some(cells);
            return;
        }
        for i in (1..cells.len()).rev() {
            let j = self.rng.range(0, i as i32 + 1) as usize;
            cells.swap(i, j);
        }
        let mut active: HashSet<usize> = HashSet::new();
        let mut moved = false;
        for i in cells {
            let p = self.world.pos_of(i);
            let t = self.world.tiles[i];
            if t.magma == 0 {
                continue;
            }
            let falls = matches!(t.kind, TileKind::Open | TileKind::StairDown | TileKind::StairUD);
            let b = p.down();
            if falls && !self.world.blocks_water(b) {
                if self.world.t(b).water > 0 {
                    self.quench(b);
                    active.insert(i);
                    continue;
                }
                let room = 7 - self.world.t(b).magma;
                if room > 0 {
                    let amt = room.min(t.magma);
                    self.world.t_mut(b).magma += amt;
                    self.magma_ignites(b);
                    self.world.tiles[i].magma -= amt;
                    active.insert(self.world.idx(b));
                    if self.world.tiles[i].magma > 0 {
                        active.insert(i);
                    }
                    moved = true;
                    continue;
                }
            }
            let (dx, dy) = DIRS4[self.rng.range(0, 4) as usize];
            let q = p.offset(dx, dy, 0);
            if !self.world.blocks_water(q) {
                if self.world.t(q).water > 0 {
                    self.quench(q);
                } else {
                    let qm = self.world.t(q).magma;
                    let here = self.world.tiles[i].magma;
                    if qm + 1 < here {
                        self.world.t_mut(q).magma += 1;
                        self.magma_ignites(q);
                        self.world.tiles[i].magma -= 1;
                        active.insert(self.world.idx(q));
                        moved = true;
                    }
                }
            }
            // Stay on the list while there's anywhere left to go.
            let here = self.world.tiles[i].magma;
            let restless = here > 0
                && ((falls && !self.world.blocks_water(b) && self.world.t(b).magma < 7)
                    || DIRS4.iter().any(|&(dx, dy)| {
                        let q = p.offset(dx, dy, 0);
                        !self.world.blocks_water(q) && (self.world.t(q).magma + 1 < here || self.world.t(q).water > 0)
                    }));
            if restless {
                active.insert(i);
            }
        }
        if moved {
            self.burn_items();
        }
        let mut next: Vec<usize> = active.into_iter().collect();
        next.sort_unstable();
        self.magma_cells = Some(next);
    }

    /// Every so often an aquifer seeps water. A heavy one fills every open
    /// cell beside it or dug out of it; constructed walls hold it back. A
    /// light one only dampens floors dug out of it (never a stair shaft).
    pub(super) fn aquifers(&mut self) {
        if self.world.aquifer == 0 {
            return;
        }
        let stale = self.aquifer_leaks.as_ref().map_or(true, |(v, _)| *v != self.dig_version);
        if stale {
            if self.aquifer_tiles.is_none() {
                self.aquifer_tiles = Some(self.world.tiles.iter().enumerate().filter(|(_, t)| t.aquifer).map(|(i, _)| i).collect());
            }
            let w = &self.world;
            let heavy = w.aquifer >= 2;
            let mut leaks: HashSet<usize> = HashSet::new();
            for &i in self.aquifer_tiles.as_ref().unwrap() {
                let t = &w.tiles[i];
                let p = w.pos_of(i);
                if !t.kind.solid() {
                    let holds = matches!(t.kind, TileKind::Floor | TileKind::Ramp | TileKind::StairUp | TileKind::Shrub);
                    if !w.blocks_water(p) && (heavy || holds) {
                        leaks.insert(i);
                    }
                } else if heavy && t.built_from.is_none() {
                    for &(dx, dy) in DIRS4.iter() {
                        let q = p.offset(dx, dy, 0);
                        if w.in_bounds(q) && !w.blocks_water(q) && w.t(q).magma == 0 {
                            leaks.insert(w.idx(q));
                        }
                    }
                }
            }
            let mut v: Vec<usize> = leaks.into_iter().collect();
            v.sort_unstable();
            self.aquifer_leaks = Some((self.dig_version, v));
        }
        // A light aquifer only dampens the floor; a heavy one floods.
        let (chance, cap) = if self.world.aquifer >= 2 { (0.5, 7) } else { (0.02, 1) };
        let leaks = self.aquifer_leaks.as_ref().map(|(_, v)| v.clone()).unwrap_or_default();
        for i in leaks {
            if self.world.tiles[i].water < cap && self.rng.chance(chance) {
                self.world.tiles[i].water += 1;
                if let Some(c) = self.water_cells.as_mut() {
                    c.push(i);
                }
            }
        }
    }

    /// Fresh magma sets anything burnable beside or above it alight.
    fn magma_ignites(&mut self, p: Pos) {
        for q in DIRS8.iter().map(|&(dx, dy)| p.offset(dx, dy, 0)).chain([p.up()]) {
            self.ignite(q);
        }
    }

    /// Magma and water meet: the tile sets as obsidian.
    pub(super) fn quench(&mut self, p: Pos) {
        let t = self.world.t_mut(p);
        t.kind = TileKind::Wall;
        t.mat = Mat::Obsidian;
        t.magma = 0;
        t.water = 0;
        self.dig_version += 1;
        self.water_cells = None;
    }

    /// Anything lying in magma that isn't stone or metal is destroyed.
    fn burn_items(&mut self) {
        let burnt: Vec<u32> = self
            .items
            .values()
            .filter(|it| it.carried_by.is_none() && !it.mat.fireproof() && self.world.get(it.pos).map_or(false, |t| t.magma > 0))
            .map(|it| it.id)
            .collect();
        for id in &burnt {
            self.items.remove(id);
        }
        for it in self.items.values_mut() {
            if it.container.map_or(false, |c| burnt.contains(&c)) {
                it.container = None;
            }
        }
    }

    /// Magma in the tile below `p`, or in `p` itself, heats a workshop.
    pub fn magma_below(&self, p: Pos) -> bool {
        [p, p.down()].iter().any(|&q| self.world.get(q).map_or(false, |t| t.magma > 0))
    }

    /// Whether freshly dug `p` looks out on magma.
    pub(super) fn near_magma(&self, p: Pos) -> bool {
        (-2..=1).any(|dz| {
            (-2..=2).any(|dy| (-2..=2).any(|dx| self.world.get(p.offset(dx, dy, dz)).map_or(false, |t| t.magma > 0 && t.revealed)))
        })
    }

    /// Like any workshop, except the middle tile may be open space, and it
    /// must have magma beneath.
    pub(super) fn can_place_magma_shop(&self, kind: BuildingKind, top_left: Pos) -> bool {
        let s = kind.size();
        let mid = top_left.offset(s / 2, s / 2, 0);
        for dy in 0..s {
            for dx in 0..s {
                let p = top_left.offset(dx, dy, 0);
                let Some(t) = self.world.get(p) else { return false };
                let ok_kind = matches!(t.kind, TileKind::Floor | TileKind::Shrub) || (p == mid && t.kind == TileKind::Open);
                if !t.revealed || t.building != 0 || t.water > 0 || t.magma > 0 || !ok_kind {
                    return false;
                }
            }
        }
        self.magma_below(mid)
    }

    /// Where to stand to draw from the nearest working well.
    pub(super) fn well_stand(&self, from: Pos) -> Option<Pos> {
        let mut wells = self.working_wells();
        wells.sort_by_key(|w| ((w.x - from.x).abs() + (w.y - from.y).abs() + 4 * (w.z - from.z).abs(), w.x, w.y, w.z));
        wells.into_iter().map(|w| self.work_spot(w)).find(|&s| self.world.walkable(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::Part;

    /// Dig a shaft beside the embark down to a pocket of water and return it.
    fn dig_shaft(g: &mut Game) -> Pos {
        let e = g.embark;
        for r in 2..20 {
            for dx in -r..=r {
                let p = e.offset(dx, r, 0);
                let (a, b) = (p.down(), p.down().down());
                let floor_ok = g.world.walkable(p) && g.world.t(p).building == 0 && !g.items.values().any(|it| it.pos == p);
                if floor_ok && g.world.solid(a) && g.world.solid(b) && g.world.walkable(p.offset(-1, 0, 0)) {
                    g.world.t_mut(p).kind = TileKind::Open;
                    g.world.t_mut(a).kind = TileKind::Open;
                    g.world.t_mut(a).water = 7;
                    g.water_cells = None;
                    return p;
                }
            }
        }
        panic!("nowhere to dig a well");
    }

    fn built_well(g: &mut Game) -> Pos {
        let p = dig_shaft(g);
        assert!(g.can_place(BuildingKind::Well, p));
        assert!(g.place_building(BuildingKind::Well, p));
        let id = g.world.t(p).building;
        let b = g.buildings.get_mut(&id).unwrap();
        b.built = true;
        b.mat = Mat::Granite;
        p
    }

    /// Carve a sealed tunnel deep in the rock: a full reservoir, a floodgate
    /// spot, then dry floor. Returns (reservoir, gate, far end).
    fn tunnel(g: &mut Game) -> (Pos, Pos, Pos) {
        let e = g.embark;
        for z in (2..e.z - 3).rev() {
            for (x, y) in (3..g.world.h - 3).flat_map(|y| [(e.x, y), (e.x - 12, y), (e.x + 12, y)]) {
                let start = Pos::new(x, y, z);
                let row: Vec<Pos> = (-1..9).map(|dx| start.offset(dx, 0, 0)).collect();
                let rock = row.iter().all(|&p| {
                    [p, p.down(), p.up(), p.offset(0, 1, 0), p.offset(0, -1, 0)].iter().all(|&q| g.world.solid(q))
                });
                if !rock {
                    continue;
                }
                for &p in &row[1..9] {
                    let t = g.world.t_mut(p);
                    t.kind = TileKind::Floor;
                    t.revealed = true;
                }
                for &p in &row[1..4] {
                    g.world.t_mut(p).water = 7;
                }
                g.water_cells = None;
                return (row[1], row[4], row[8]);
            }
        }
        panic!("no solid rock for a tunnel");
    }

    #[test]
    fn a_floodgate_holds_water_until_its_lever_is_pulled() {
        let mut g = Game::new(81);
        let (_, gate, far) = tunnel(&mut g);
        assert!(g.place_building(BuildingKind::Floodgate, gate));
        let gid = g.world.t(gate).building;
        {
            let b = g.buildings.get_mut(&gid).unwrap();
            b.built = true;
            b.locked = true;
        }
        g.world.t_mut(gate).door = crate::world::FLOODGATE_SHUT;
        let e = g.embark;
        let lp = (2..20).map(|r| e.offset(r, 2, 0)).find(|&p| g.can_place(BuildingKind::Lever, p)).unwrap();
        assert!(g.place_building(BuildingKind::Lever, lp));
        let lid = g.world.t(lp).building;
        g.buildings.get_mut(&lid).unwrap().built = true;
        g.spawn_item(ItemKind::Mechanism, Mat::Granite, e);
        g.link_lever(lid, gid).unwrap();
        let wet_beyond = |g: &Game| (0..5).any(|i| g.world.t(far.offset(-i, 0, 0)).water > 0);
        for _ in 0..600 {
            g.step();
        }
        assert_eq!(g.world.t(gate).water, 0, "water seeped into a shut floodgate");
        assert!(!wet_beyond(&g), "water got past a shut floodgate");
        g.pull_lever(lid);
        assert!(!g.buildings[&gid].locked);
        for _ in 0..600 {
            g.step();
        }
        assert!(wet_beyond(&g), "water never flowed through the open floodgate");
        g.pull_lever(lid);
        assert!(g.world.t(gate).door == crate::world::FLOODGATE_SHUT && !g.world.passable(gate, Mover::Dwarf));
    }

    #[test]
    fn a_new_floodgate_starts_shut_with_nobody_inside() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let e = g.embark;
        let p = (2..20).map(|r| e.offset(r, -2, 0)).find(|&p| g.can_place(BuildingKind::Floodgate, p)).unwrap();
        assert!(g.place_building(BuildingKind::Floodgate, p));
        g.spawn_item(ItemKind::Block, Mat::Granite, e);
        g.spawn_item(ItemKind::Mechanism, Mat::Granite, e);
        let id = g.world.t(p).building;
        for _ in 0..DAY {
            g.step();
            if g.buildings[&id].built {
                break;
            }
        }
        assert!(g.buildings[&id].built, "the floodgate was never built");
        assert_eq!(g.world.t(p).door, crate::world::FLOODGATE_SHUT);
        assert!(g.dwarves.iter().all(|d| d.pos != p), "the builder was left standing in the gate");
    }

    #[test]
    fn the_world_has_a_magma_sea_above_the_bedrock() {
        let g = Game::new(81);
        let w = &g.world;
        let (mut sea, mut shore) = (0, 0);
        for y in 0..w.h {
            for x in 0..w.w {
                sea += (w.t(Pos::new(x, y, 1)).magma == 7) as i32;
                shore += (w.t(Pos::new(x, y, 2)).kind == TileKind::Floor) as i32;
            }
        }
        assert!(sea > w.w * w.h / 2, "only {} magma tiles", sea);
        assert!(shore > 0, "no shore to stand on");
        assert!(g.dwarves.iter().all(|d| d.pos.z > 4));
    }

    #[test]
    fn magma_flows_burns_wood_and_sets_into_obsidian_against_water() {
        let mut g = Game::new(81);
        let (res, gate, far) = tunnel(&mut g);
        for i in 0..3 {
            let p = res.offset(i, 0, 0);
            g.world.t_mut(p).water = 0;
            g.world.t_mut(p).magma = 7;
        }
        g.world.t_mut(far).water = 7;
        g.water_cells = None;
        g.dig_version += 1;
        let log = g.spawn_item(ItemKind::Log, Mat::Wood, gate);
        let rock = g.spawn_item(ItemKind::Boulder, Mat::Granite, gate);
        for _ in 0..6000 {
            g.step();
        }
        assert!(!g.items.contains_key(&log), "the log didn't burn");
        assert!(g.items.contains_key(&rock), "stone shouldn't burn");
        let obsidian = (0..10).any(|i| {
            let t = g.world.t(res.offset(i - 1, 0, 0));
            t.kind == TileKind::Wall && t.mat == Mat::Obsidian
        });
        assert!(obsidian, "magma met water without making obsidian");
    }

    #[test]
    fn a_magma_forge_needs_magma_below_but_no_fuel() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        g.items.retain(|_, it| !matches!(it.kind, ItemKind::Charcoal) && it.mat != Mat::Coal);
        let e = g.embark;
        let tl = (2..20)
            .flat_map(|r| [e.offset(r, 0, 0), e.offset(-r, 0, 0), e.offset(0, r, 0)])
            .find(|&p| g.can_place(BuildingKind::Forge, p) && g.world.solid(p.offset(1, 1, -1)) && g.world.solid(p.offset(1, 1, -2)))
            .expect("room for a forge");
        assert!(!g.can_place(BuildingKind::MagmaForge, tl), "a magma forge needs magma");
        let below = tl.offset(1, 1, -1);
        g.world.t_mut(below).kind = TileKind::Open;
        g.world.t_mut(below).magma = 7;
        g.dig_version += 1;
        assert!(g.can_place(BuildingKind::MagmaForge, tl));
        assert!(g.place_building(BuildingKind::MagmaForge, tl));
        let id = g.world.t(tl).building;
        g.buildings.get_mut(&id).unwrap().built = true;
        g.spawn_item(ItemKind::Bar, Mat::Iron, e);
        g.add_order(id, Recipe::ForgePick);
        for d in g.dwarves.iter_mut() {
            d.labors[Labor::Smithing.idx()] = true;
        }
        let picks = |g: &Game| g.items.values().filter(|it| it.kind == ItemKind::Pick && it.mat == Mat::Iron).count();
        let before = picks(&g);
        for _ in 0..2 * DAY {
            g.step();
        }
        assert_eq!(picks(&g), before + 1, "the magma forge made no pick");
    }

    /// A natural aquifer wall with dry rooms dug either side of it at depth 3,
    /// one walled in with a construction. Returns (open room, walled room).
    fn aquifer_rooms(g: &mut Game, level: u8) -> (Pos, Pos) {
        g.world.aquifer = level;
        let e = g.embark;
        let p = (3..30)
            .flat_map(|r| [e.offset(r, 3, 0), e.offset(-r, 3, 0), e.offset(3, r, 0)])
            .map(|p| Pos::new(p.x, p.y, g.world.surface_z(p.x, p.y) - 3))
            .find(|p| (-3..=3).all(|dx| (-2..=2).all(|dy| g.world.solid(p.offset(dx, dy, 0)) && g.world.solid(p.offset(dx, dy, -1)))))
            .expect("solid rock");
        for dx in -3..=3 {
            for dy in -2..=2 {
                g.world.t_mut(p.offset(dx, dy, 0)).aquifer = false;
            }
        }
        g.world.t_mut(p).aquifer = true;
        let (open, walled) = (p.offset(1, 0, 0), p.offset(-2, 0, 0));
        for q in [open, walled] {
            let t = g.world.t_mut(q);
            t.kind = TileKind::Floor;
            t.revealed = true;
        }
        // Between the aquifer and the walled room: a constructed wall.
        let between = p.offset(-1, 0, 0);
        g.world.t_mut(between).built_from = Some(ItemKind::Block);
        g.world.t_mut(between).aquifer = true;
        g.aquifer_tiles = None;
        g.dig_version += 1;
        (open, walled)
    }

    #[test]
    fn a_light_aquifer_only_dampens_floors_dug_out_of_it() {
        let mut g = Game::new(81);
        let (open, _) = aquifer_rooms(&mut g, 1);
        let rock = open.offset(-1, 0, 0);
        g.world.t_mut(rock).kind = TileKind::Floor;
        g.dig_version += 1;
        let mut damp = false;
        // A drip now and then (one chance in fifty every 25 ticks).
        for _ in 0..20000 {
            g.step();
            let w = g.world.t(rock).water;
            assert!(w <= 1, "a light aquifer flooded to {}", w);
            damp |= w > 0;
        }
        assert!(damp, "the dug aquifer floor never got damp");
        // A stair dug through a light aquifer stays dry.
        g.world.t_mut(rock).kind = TileKind::StairUD;
        for q in [rock, open] {
            g.world.t_mut(q).water = 0;
        }
        g.dig_version += 1;
        for _ in 0..3000 {
            g.step();
            assert_eq!(g.world.t(rock).water, 0, "water seeped into a stair shaft");
        }
    }

    #[test]
    fn a_heavy_aquifer_floods_but_a_built_wall_keeps_it_out() {
        let mut g = Game::new(81);
        let (open, walled) = aquifer_rooms(&mut g, 2);
        for _ in 0..3000 {
            g.step();
        }
        assert!(g.world.t(open).water >= 4, "heavy aquifer only left {}", g.world.t(open).water);
        assert_eq!(g.world.t(walled).water, 0, "water seeped through a constructed wall");
    }

    #[test]
    fn wet_sites_have_aquifers_and_the_world_marks_them() {
        let ow = Overworld::generate(7);
        let mut kinds = [0; 3];
        for y in 0..ow.h {
            for x in 0..ow.w {
                kinds[ow.aquifer(x, y) as usize] += 1;
            }
        }
        assert!(kinds.iter().all(|&k| k > 0), "aquifer kinds {:?}", kinds);
        let (x, y) = (0..ow.w * ow.h).map(|i| (i % ow.w, i / ow.w)).find(|&(x, y)| ow.aquifer(x, y) == 1 && ow.cell(x, y).biome != crate::overworld::Biome::Ocean).unwrap();
        let g = Game::embark(7, ow, x, y);
        assert_eq!(g.world.aquifer, 1);
        assert!(g.world.tiles.iter().any(|t| t.aquifer));
    }

    #[test]
    fn a_well_needs_a_channel_over_water() {
        let mut g = Game::new(81);
        let e = g.embark;
        assert!(!g.can_place(BuildingKind::Well, e), "a well can't go on solid floor");
        let p = dig_shaft(&mut g);
        assert!(g.can_place(BuildingKind::Well, p));
        assert!(g.well_has_water(p));
        g.world.t_mut(p.down()).water = 0;
        assert!(!g.well_has_water(p), "a dry shaft shouldn't count");
    }

    #[test]
    fn dwarves_build_a_well_and_drink_from_it() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        g.items.retain(|_, it| it.kind != ItemKind::Drink);
        let p = dig_shaft(&mut g);
        assert!(g.place_building(BuildingKind::Well, p));
        let e = g.embark;
        g.spawn_item(ItemKind::Block, Mat::Granite, e);
        g.spawn_item(ItemKind::Bucket, Mat::Wood, e);
        let id = g.world.t(p).building;
        for _ in 0..DAY {
            g.step();
            if g.buildings[&id].built {
                break;
            }
        }
        assert!(g.buildings[&id].built, "the well was never built");
        for d in g.dwarves.iter_mut() {
            d.thirst = THIRSTY + 1;
        }
        for _ in 0..DAY / 2 {
            g.step();
        }
        let text = "drank clean water from the well; there was no ale";
        assert!(g.dwarves.iter().any(|d| d.thoughts.iter().any(|t| t.text == text)), "nobody drank from the well");
    }

    #[test]
    fn a_doctor_brings_well_water_to_a_patient_in_bed() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        built_well(&mut g);
        let e = g.embark;
        let bucket = g.spawn_item(ItemKind::Bucket, Mat::Wood, e);
        for d in g.dwarves.iter_mut() {
            d.labors[Labor::Medicine.idx()] = false;
        }
        g.dwarves[1].labors[Labor::Medicine.idx()] = true;
        let id = g.dwarves[0].id;
        let arm = Part::LeftArm.idx();
        {
            let d = &mut g.dwarves[0];
            d.body.hp[arm] = 1;
            d.thirst = THIRSTY / 2 + 50;
            d.task = Task::Rest { bed: None, stand: d.pos };
            d.asleep = true;
        }
        let mut brought = false;
        for _ in 0..DAY / 2 {
            g.step();
            let d = g.dwarves.iter_mut().find(|d| d.id == id).unwrap();
            d.body.hp[arm] = 1;
            d.hunger = 0;
            assert!(matches!(d.task, Task::Rest { .. }), "the patient got up: {:?}", d.task);
            if d.thirst < 100 {
                brought = true;
                break;
            }
        }
        assert!(brought, "nobody brought the patient water");
        let b = &g.items[&bucket];
        assert!(b.label.is_none() && b.carried_by.is_none(), "the bucket should be set down empty");
    }
}
