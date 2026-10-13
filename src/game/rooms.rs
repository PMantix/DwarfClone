//! Rooms made from zones: bedrooms and offices with owners, dormitories,
//! dining halls, and the garbage dump.

use super::{Game, ZoneKind, SEARCH_LIMIT};
use crate::buildings::BuildingKind;
use crate::dwarf::{Dwarf, Task};
use crate::geom::{Pos, DIRS4};
use crate::path;
use crate::world::Mover;
use std::collections::HashSet;

impl ZoneKind {
    pub fn name(self) -> String {
        match self {
            ZoneKind::Stockpile(f) => format!("Stockpile: {}", f.name()),
            ZoneKind::Farm => "Farm plot".into(),
            ZoneKind::Meeting => "Meeting area".into(),
            ZoneKind::Bedroom => "Bedroom".into(),
            ZoneKind::Dormitory => "Dormitory".into(),
            ZoneKind::Dining => "Dining hall".into(),
            ZoneKind::Office => "Office".into(),
            ZoneKind::Dump => "Garbage dump".into(),
            ZoneKind::Barracks => "Barracks".into(),
            ZoneKind::Hospital => "Hospital".into(),
            ZoneKind::Pasture => "Pasture".into(),
            ZoneKind::Temple => "Temple".into(),
            ZoneKind::Tavern => "Tavern".into(),
            ZoneKind::Tomb => "Tomb".into(),
        }
    }

    /// Zones that can be laid over beds, tables and chairs.
    pub fn holds_furniture(self) -> bool {
        matches!(
            self,
            ZoneKind::Meeting
                | ZoneKind::Bedroom
                | ZoneKind::Dormitory
                | ZoneKind::Dining
                | ZoneKind::Office
                | ZoneKind::Barracks
                | ZoneKind::Hospital
                | ZoneKind::Temple
                | ZoneKind::Tavern
                | ZoneKind::Tomb
        )
    }

    /// Zones that belong to one dwarf.
    pub fn ownable(self) -> bool {
        matches!(self, ZoneKind::Bedroom | ZoneKind::Office)
    }
}

impl Game {
    /// The zone at `p`, if any.
    pub fn zone_at(&self, p: Pos) -> Option<&super::Zone> {
        self.world.get(p).and_then(|t| self.zones.get(&t.zone))
    }

    pub fn zone_kind_at(&self, p: Pos) -> Option<ZoneKind> {
        self.zone_at(p).map(|z| z.kind)
    }

    /// Value of an area: floor space, furniture standing in it and the stone
    /// of the walls around it.
    pub fn area_value(&self, tiles: &HashSet<Pos>) -> u32 {
        let w = &self.world;
        let mut value = tiles.len() as u32 * 2;
        let mut counted = HashSet::new();
        let mut walls = HashSet::new();
        for &q in tiles {
            let bid = w.t(q).building;
            if bid != 0 && counted.insert(bid) {
                if let Some(b) = self.buildings.get(&bid) {
                    value += self.building_value(b);
                }
            }
            for (dx, dy) in DIRS4 {
                let n = q.offset(dx, dy, 0);
                if w.get(n).map_or(false, |t| t.kind.encloses()) {
                    walls.insert(n);
                }
            }
        }
        value + walls.iter().map(|q| (w.t(*q).mat.value() - 1) * 3).sum::<u32>()
    }

    /// Value of the room around a bed or table: its zone if it has one,
    /// otherwise an enclosed underground room, otherwise nothing.
    pub(super) fn room_value_at(&self, p: Pos) -> u32 {
        match self.zone_at(p) {
            Some(z) if z.kind.holds_furniture() => self.area_value(&z.tiles.iter().copied().collect()),
            _ => self.room_at(p).map_or(0, |(_, v)| v),
        }
    }

    /// Give an ownable zone to a dwarf (or nobody). Bedrooms take effect at once.
    pub fn set_zone_owner(&mut self, zone: u32, owner: Option<u32>) -> bool {
        let Some(z) = self.zones.get(&zone) else { return false };
        if !z.kind.ownable() {
            return false;
        }
        let kind = z.kind;
        // One bedroom and one office per dwarf.
        if let Some(o) = owner {
            for other in self.zones.values_mut() {
                if other.kind == kind && other.owner == Some(o) {
                    other.owner = None;
                }
            }
        }
        self.zones.get_mut(&zone).unwrap().owner = owner;
        if kind == ZoneKind::Bedroom {
            // Free the beds in this room so assignment hands them to the new owner.
            let tiles: HashSet<Pos> = self.zones[&zone].tiles.iter().copied().collect();
            for b in self.buildings.values_mut() {
                if b.kind == BuildingKind::Bed && tiles.contains(&b.pos) {
                    b.owner = None;
                }
            }
            self.assign_rooms();
        }
        true
    }

    /// Next living dwarf after the current owner, wrapping to nobody.
    pub fn cycle_zone_owner(&mut self, zone: u32) -> Option<String> {
        let cur = self.zones.get(&zone)?.owner;
        let ids: Vec<u32> = self.dwarves.iter().filter(|d| d.alive && !d.insane).map(|d| d.id).collect();
        let next = match cur.and_then(|c| ids.iter().position(|&i| i == c)) {
            None => ids.first().copied(),
            Some(i) => ids.get(i + 1).copied(),
        };
        self.set_zone_owner(zone, next);
        Some(next.and_then(|id| self.dwarves.iter().find(|d| d.id == id)).map_or("nobody".into(), |d| d.name.clone()))
    }

    /// Beds that belong to zones: a bedroom's beds go to its owner, a dormitory's to nobody.
    /// Returns dwarves whose bed was given by a zone (automatic assignment leaves them be).
    pub(super) fn apply_zone_beds(&mut self) -> HashSet<u32> {
        let mut fixed = HashSet::new();
        let bed_ids: Vec<u32> = self.buildings.values().filter(|b| b.kind == BuildingKind::Bed && b.built).map(|b| b.id).collect();
        for bid in bed_ids {
            let pos = self.buildings[&bid].pos;
            let (kind, owner) = match self.zone_at(pos) {
                Some(z) => (Some(z.kind), z.owner),
                None => (None, None),
            };
            match kind {
                Some(ZoneKind::Dormitory | ZoneKind::Hospital) => self.buildings.get_mut(&bid).unwrap().owner = None,
                Some(ZoneKind::Bedroom) if owner.is_some() => {
                    let o = owner.unwrap();
                    // Their other beds go back into the pool.
                    for b in self.buildings.values_mut() {
                        if b.owner == Some(o) && b.id != bid && b.kind == BuildingKind::Bed {
                            let in_room = self.world.get(b.pos).map_or(false, |t| t.zone == self.world.t(pos).zone);
                            if !in_room {
                                b.owner = None;
                            }
                        }
                    }
                    self.buildings.get_mut(&bid).unwrap().owner = Some(o);
                    fixed.insert(o);
                }
                _ => {}
            }
        }
        fixed
    }

    /// Whether automatic assignment may hand out this bed.
    pub(super) fn bed_assignable(&self, pos: Pos) -> bool {
        match self.zone_at(pos) {
            Some(z) if z.kind == ZoneKind::Dormitory || z.kind == ZoneKind::Hospital => false,
            Some(z) if z.kind == ZoneKind::Bedroom => z.owner.is_none(),
            _ => true,
        }
    }

    pub fn in_dining_hall(&self, p: Pos) -> bool {
        self.zone_kind_at(p) == Some(ZoneKind::Dining)
    }

    /// Mark one item, or every loose item on a tile, for the garbage dump.
    pub fn toggle_dump(&mut self, item: u32) -> Option<bool> {
        let it = self.items.get_mut(&item)?;
        // Tools and weapons in someone's hands stay there.
        if it.carried_by.is_some() {
            return None;
        }
        it.dump = !it.dump;
        it.to_depot = false;
        it.container = None;
        Some(it.dump)
    }

    pub fn dump_tile(&mut self, p: Pos) -> usize {
        let mut n = 0;
        for it in self.items.values_mut() {
            if it.pos == p && it.carried_by.is_none() && it.container.is_none() && !it.dump {
                it.dump = true;
                it.to_depot = false;
                n += 1;
            }
        }
        n
    }

    pub(super) fn find_dump_haul(&mut self, d: &mut Dwarf) -> bool {
        let tiles: HashSet<Pos> = self
            .zones
            .values()
            .filter(|z| z.kind == ZoneKind::Dump)
            .flat_map(|z| z.tiles.iter().copied())
            .filter(|p| self.world.walkable(*p))
            .collect();
        if tiles.is_empty() {
            return false;
        }
        let Some((item, path)) = self.find_item(d.pos, |it| it.dump && !tiles.contains(&it.pos)) else { return false };
        let ipos = self.items[&item].pos;
        let Some((dest, _)) = path::search(&self.world, ipos, SEARCH_LIMIT, Mover::Dwarf, |p| tiles.contains(&p)) else {
            return false;
        };
        self.claim_item(item, d.id);
        d.task = Task::Haul { item, dest, into: None };
        super::Game::set_path(d, path);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{ItemKind, DAY};
    use crate::geom::box_iter;
    use crate::world::TileKind;

    /// A spot where a `w`×`h` patch of plain floor is free, near the wagon.
    fn open_patch(g: &Game, w: i32, h: i32, skip: &HashSet<Pos>) -> Pos {
        let e = g.embark;
        for r in 4..30 {
            for dy in -r..=r {
                for dx in -r..=r {
                    let a = e.offset(dx, dy, 0);
                    let a = Pos::new(a.x, a.y, g.world.surface_z(a.x, a.y));
                    let ok = box_iter(a, a.offset(w - 1, h - 1, 0)).all(|p| {
                        g.world.get(p).map_or(false, |t| t.kind == TileKind::Floor && t.water == 0 && t.building == 0 && t.zone == 0)
                            && !skip.contains(&p)
                            && g.items.values().all(|it| it.pos != p)
                    });
                    if ok {
                        return a;
                    }
                }
            }
        }
        panic!("no open patch");
    }

    fn bed_at(g: &mut Game, p: Pos) -> u32 {
        assert!(g.place_building(BuildingKind::Bed, p));
        let id = *g.buildings.keys().max().unwrap();
        g.buildings.get_mut(&id).unwrap().built = true;
        id
    }

    #[test]
    fn assigned_bedroom_is_used_over_a_nearer_bed() {
        let mut g = Game::new(21);
        // A free bed right by the wagon, and a bedroom zone further off.
        let near = open_patch(&g, 1, 1, &HashSet::new());
        let near_bed = bed_at(&mut g, near);
        let mut skip: HashSet<Pos> = box_iter(near.offset(-6, -6, 0), near.offset(6, 6, 0)).collect();
        skip.insert(near);
        let room = open_patch(&g, 3, 3, &skip);
        let room_bed = bed_at(&mut g, room.offset(1, 1, 0));
        assert!(g.add_zone(room, room.offset(2, 2, 0), ZoneKind::Bedroom) == 9);
        let zid = g.world.t(room).zone;

        let owner = g.dwarves[0].id;
        assert!(g.set_zone_owner(zid, Some(owner)));
        assert_eq!(g.buildings[&room_bed].owner, Some(owner));
        assert_ne!(g.buildings[&near_bed].owner, Some(owner));
        assert!(g.buildings[&room_bed].room_value > 0, "zoned bedroom has no value");

        // Make the owner sleepy, next to the near bed.
        let d = &mut g.dwarves[0];
        d.pos = near;
        d.fatigue = crate::game::TIRED + 10;
        d.idle_cd = 0;
        for _ in 0..DAY / 2 {
            g.step();
            if g.dwarves[0].asleep {
                break;
            }
        }
        let d = &g.dwarves[0];
        assert!(d.asleep, "owner never fell asleep");
        assert!(matches!(d.task, Task::Sleep { bed: Some(b), .. } if b == room_bed), "slept in {:?}", d.task);

        // Daily reassignment must not take the room away again.
        g.assign_rooms();
        assert_eq!(g.buildings[&room_bed].owner, Some(owner));
    }

    #[test]
    fn dormitory_beds_stay_shared_and_dump_collects_garbage() {
        let mut g = Game::new(22);
        let dorm = open_patch(&g, 3, 3, &HashSet::new());
        let b1 = bed_at(&mut g, dorm);
        let b2 = bed_at(&mut g, dorm.offset(2, 0, 0));
        g.add_zone(dorm, dorm.offset(2, 2, 0), ZoneKind::Dormitory);
        g.assign_rooms();
        assert_eq!(g.buildings[&b1].owner, None);
        assert_eq!(g.buildings[&b2].owner, None);

        // Mark a log for the dump; a hauler takes it there and it stays.
        let skip: HashSet<Pos> = box_iter(dorm.offset(-1, -1, 0), dorm.offset(3, 3, 0)).collect();
        let dump = open_patch(&g, 2, 2, &skip);
        g.add_zone(dump, dump.offset(1, 1, 0), ZoneKind::Dump);
        let log = g.items.values().find(|it| it.kind == ItemKind::Log || it.kind == ItemKind::Barrel).map(|it| it.id).unwrap();
        assert_eq!(g.toggle_dump(log), Some(true));
        for _ in 0..DAY {
            g.step();
        }
        let p = g.items[&log].pos;
        assert_eq!(g.zone_kind_at(p), Some(ZoneKind::Dump), "item not dumped (at {:?})", p);
        for _ in 0..DAY / 2 {
            g.step();
        }
        assert_eq!(g.items[&log].pos, p, "dumped item was hauled away again");
    }

    #[test]
    fn dining_hall_seats_come_first() {
        let mut g = Game::new(23);
        let place = |g: &mut Game, kind: BuildingKind, p: Pos| {
            assert!(g.place_building(kind, p));
            let id = *g.buildings.keys().max().unwrap();
            g.buildings.get_mut(&id).unwrap().built = true;
            id
        };
        // A table and chair near the food, and another pair in a dining hall further off.
        let near = open_patch(&g, 2, 1, &HashSet::new());
        place(&mut g, BuildingKind::Table, near);
        place(&mut g, BuildingKind::Chair, near.offset(1, 0, 0));
        let skip: HashSet<Pos> = box_iter(near.offset(-6, -6, 0), near.offset(7, 6, 0)).collect();
        let hall = open_patch(&g, 3, 3, &skip);
        place(&mut g, BuildingKind::Table, hall.offset(1, 1, 0));
        let hall_chair = place(&mut g, BuildingKind::Chair, hall.offset(2, 1, 0));
        g.add_zone(hall, hall.offset(2, 2, 0), ZoneKind::Dining);

        let d = &mut g.dwarves[0];
        d.hunger = crate::game::HUNGRY + 10;
        d.idle_cd = 0;
        for _ in 0..50 {
            g.step();
            if matches!(g.dwarves[0].task, Task::Dine { .. } | Task::Eat { .. }) {
                break;
            }
        }
        assert!(
            matches!(g.dwarves[0].task, Task::Dine { seat, .. } if seat == hall_chair),
            "went for {:?}",
            g.dwarves[0].task
        );
    }
}
