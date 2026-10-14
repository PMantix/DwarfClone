//! Wells (clean water drawn up from below, for drinking and for patients)
//! and floodgates (lever-worked gates that hold water back).

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
                Some(t) if t.water > 0 => return true,
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
            for y in 3..g.world.h - 3 {
                let start = Pos::new(e.x, y, z);
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
