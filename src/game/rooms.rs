//! Rooms made from zones: bedrooms and offices with owners, dormitories,
//! dining halls, and the garbage dump.

use super::{Game, ZoneKind, SEARCH_LIMIT};
use crate::buildings::BuildingKind;
use crate::dwarf::{Dwarf, Task};
use crate::geom::{Pos, DIRS4};
use crate::items::ItemKind;
use crate::material::Mat;
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
            ZoneKind::Gathering => "Gathering area".into(),
            ZoneKind::Jail => "Jail".into(),
            ZoneKind::Laboratory => "Laboratory".into(),
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
                | ZoneKind::Laboratory
        )
    }

    /// Zones that may share their tiles with another of these (O11): a
    /// dining hall that is also the meeting area, a tavern with a shrine.
    pub fn overlaps(self) -> bool {
        matches!(self, ZoneKind::Dining | ZoneKind::Meeting | ZoneKind::Tavern | ZoneKind::Temple)
    }

    /// Zones that belong to one dwarf.
    pub fn ownable(self) -> bool {
        matches!(self, ZoneKind::Bedroom | ZoneKind::Office)
    }
}

/// The low (or high) corner of the box around some tiles.
fn bounds(tiles: &[Pos], low: bool) -> Option<Pos> {
    let pick = |f: fn(i32, i32) -> i32| {
        let mut it = tiles.iter();
        let first = *it.next()?;
        Some(it.fold(first, |m, p| Pos::new(f(m.x, p.x), f(m.y, p.y), f(m.z, p.z))))
    };
    if low {
        pick(i32::min)
    } else {
        pick(i32::max)
    }
}

/// One stockpile setting: a kind of item, or a material.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PileToggle {
    Kind(ItemKind),
    Mat(Mat),
}

impl Game {
    /// The zone at `p`, if any.
    pub fn zone_at(&self, p: Pos) -> Option<&super::Zone> {
        self.world.get(p).and_then(|t| self.zones.get(&t.zone))
    }

    /// The kind of the zone at `p` (the first laid, if several share it).
    pub fn zone_kind_at(&self, p: Pos) -> Option<ZoneKind> {
        self.zone_at(p).map(|z| z.kind)
    }

    /// Every zone at `p`: the first laid, then those laid over it (O11).
    pub fn zones_at(&self, p: Pos) -> impl Iterator<Item = &super::Zone> {
        let over = self.zone_over.get(&p).into_iter().flatten().filter_map(move |id| self.zones.get(id));
        self.zone_at(p).into_iter().chain(over)
    }

    /// Whether any zone at `p` is a `kind`. The overlay map is only asked
    /// when the tile's own zone doesn't answer.
    pub fn has_zone_kind(&self, p: Pos, kind: ZoneKind) -> bool {
        self.zone_kind_at(p) == Some(kind)
            || (!self.zone_over.is_empty()
                && self.zone_over.get(&p).map_or(false, |v| v.iter().any(|id| self.zones.get(id).map_or(false, |z| z.kind == kind))))
    }

    /// Whether a `kind` zone may be laid over the zone already at `p`:
    /// both share halls, and no zone of that kind is there yet.
    pub(super) fn can_overlay(&self, p: Pos, kind: ZoneKind) -> bool {
        kind.overlaps() && self.zone_at(p).map_or(false, |z| z.kind.overlaps()) && !self.zones_at(p).any(|z| z.kind == kind)
    }

    /// Rebuild `zone_over` from the zones (after loading): a zone's tiles
    /// that hold another zone are laid over it.
    pub fn rebuild_zone_over(&mut self) {
        self.zone_over.clear();
        let mut ids: Vec<u32> = self.zones.keys().copied().collect();
        ids.sort();
        for id in ids {
            for &p in &self.zones[&id].tiles {
                if self.world.get(p).map_or(false, |t| t.zone != id) {
                    self.zone_over.entry(p).or_default().push(id);
                }
            }
        }
    }

    /// Take overlay zones passing `which` off the tiles in the box. Returns how many tiles.
    pub(super) fn remove_overlays_where(&mut self, a: Pos, b: Pos, which: &dyn Fn(ZoneKind) -> bool) -> usize {
        let mut n = 0;
        for p in crate::geom::box_iter(a, b) {
            let Some(ids) = self.zone_over.get(&p).cloned() else { continue };
            let gone: Vec<u32> = ids.iter().copied().filter(|id| self.zones.get(id).map_or(true, |z| which(z.kind))).collect();
            for id in &gone {
                if let Some(z) = self.zones.get_mut(id) {
                    z.tiles.retain(|&q| q != p);
                    if z.tiles.is_empty() {
                        self.zones.remove(id);
                    }
                }
                n += 1;
            }
            let left: Vec<u32> = ids.into_iter().filter(|id| !gone.contains(id)).collect();
            if left.is_empty() {
                self.zone_over.remove(&p);
            } else {
                self.zone_over.insert(p, left);
            }
        }
        n
    }

    /// Bulk zoning (O11): each enclosed room in the box becomes a zone of
    /// its own (a row of bedrooms, one for each dwarf); floor in no room
    /// makes one more. Returns (zones made, tiles zoned).
    pub fn add_zone_rooms(&mut self, a: Pos, b: Pos, kind: ZoneKind) -> (usize, usize) {
        let lo = Pos::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z));
        let hi = Pos::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z));
        let inside = |p: &Pos| p.x >= lo.x && p.x <= hi.x && p.y >= lo.y && p.y <= hi.y && p.z >= lo.z && p.z <= hi.z;
        let mut done: HashSet<Pos> = HashSet::new();
        let mut rest = Vec::new();
        let (mut zones, mut tiles) = (0, 0);
        for p in crate::geom::box_iter(lo, hi) {
            if done.contains(&p) || !self.world.in_bounds(p) {
                continue;
            }
            let room = if self.world.walkable(p) { self.room_at(p) } else { None };
            match room {
                Some((room, _)) => {
                    let part: Vec<Pos> = room.into_iter().filter(inside).collect();
                    done.extend(part.iter().copied());
                    done.insert(p);
                    let n = self.add_zone_tiles(part, kind);
                    zones += (n > 0) as usize;
                    tiles += n;
                }
                None => {
                    done.insert(p);
                    rest.push(p);
                }
            }
        }
        let n = self.add_zone_tiles(rest, kind);
        zones += (n > 0) as usize;
        tiles += n;
        (zones, tiles)
    }

    /// The player names a zone, "the Hall of Ale" (O11): it becomes a named
    /// place no deed renames. An empty name takes the name away.
    pub fn name_zone(&mut self, zone: u32, name: &str) -> bool {
        let Some(z) = self.zones.get(&zone) else { return false };
        let (Some(a), Some(b)) = (bounds(&z.tiles, true), bounds(&z.tiles, false)) else { return false };
        let what = z.kind.name().to_lowercase();
        let name = name.trim();
        self.zones.get_mut(&zone).unwrap().name = (!name.is_empty()).then(|| name.to_string());
        self.name_by_player(super::places::PlaceKind::Hall, a, b, name, what);
        true
    }

    /// The player names a building, "Old Grumbler" for the still (O11).
    pub fn name_building(&mut self, bld: u32, name: &str) -> bool {
        let Some(b) = self.buildings.get(&bld) else { return false };
        let s = b.kind.size() - 1;
        let (a, c) = (b.pos, b.pos.offset(s, s, 0));
        let what = b.kind.name().to_lowercase();
        let name = name.trim();
        self.buildings.get_mut(&bld).unwrap().name = (!name.is_empty()).then(|| name.to_string());
        self.name_by_player(super::places::PlaceKind::Work, a, c, name, what);
        true
    }

    /// Register (or, with an empty name, forget) the player's name for the box `a`..`b`.
    fn name_by_player(&mut self, kind: super::places::PlaceKind, a: Pos, b: Pos, name: &str, what: String) {
        if name.is_empty() {
            self.stories.places.retain(|pl| !(pl.by_player && pl.at == a && pl.b == Some(b)));
            return;
        }
        let why = format!("the {} so named by order of the fort", what);
        self.name_place(kind, a, b, name.to_string(), why, 0, true);
    }

    /// The tile's own zone is gone: the first zone laid over it takes its place.
    pub(super) fn promote_overlay(&mut self, p: Pos) {
        let Some(mut v) = self.zone_over.remove(&p) else { return };
        // (Skipping any zone since taken away whole.)
        v.retain(|id| self.zones.contains_key(id));
        if v.is_empty() {
            return;
        }
        self.world.t_mut(p).zone = v.remove(0);
        if !v.is_empty() {
            self.zone_over.insert(p, v);
        }
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
        value
            + walls.iter().map(|q| (w.t(*q).mat.value() - 1) * 3 + w.detail_value(*q)).sum::<u32>()
            + tiles.iter().map(|q| w.detail_value(*q)).sum::<u32>()
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

    /// The office a dwarf owns, if any.
    pub fn office_of(&self, dwarf: u32) -> Option<&super::Zone> {
        self.zones.values().find(|z| z.kind == ZoneKind::Office && z.owner == Some(dwarf))
    }

    /// Who could own a bedroom or office: titled nobles first, then the
    /// holders of posts, then everyone else.
    pub fn owner_candidates(&self) -> Vec<u32> {
        let mut v: Vec<(u8, usize, u32)> = self
            .dwarves
            .iter()
            .enumerate()
            .filter(|(_, d)| d.alive && !d.insane && !d.is_child())
            .map(|(i, d)| {
                let rank = if d.noble.is_some() {
                    0
                } else if self.position_of(d.id).is_some() {
                    1
                } else {
                    2
                };
                (rank, i, d.id)
            })
            .collect();
        v.sort();
        v.into_iter().map(|(_, _, id)| id).collect()
    }

    /// Daily: offices nobody owns go to the nobles and post holders who
    /// have none, the highest first.
    pub(super) fn assign_offices(&mut self) {
        let mut free: Vec<u32> = self.zones.values().filter(|z| z.kind == ZoneKind::Office && z.owner.is_none()).map(|z| z.id).collect();
        if free.is_empty() {
            return;
        }
        free.sort();
        let wanting: Vec<u32> = self
            .owner_candidates()
            .into_iter()
            .filter(|&id| {
                let d = self.dwarves.iter().find(|d| d.id == id).unwrap();
                (d.noble.is_some() || self.position_of(id).is_some()) && self.office_of(id).is_none()
            })
            .collect();
        for (zone, id) in free.into_iter().zip(wanting) {
            self.set_zone_owner(zone, Some(id));
            let name = self.dwarves.iter().find(|d| d.id == id).map_or(String::new(), |d| d.name.clone());
            self.msg(&format!("{} has moved into an office.", name), (200, 220, 255));
        }
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
                    // The room's first bed is its owner's; a second is for
                    // the owner's husband or wife (couples share).
                    let zid = self.world.t(pos).zone;
                    let first = self.buildings.values().filter(|b| b.kind == BuildingKind::Bed && b.built && self.world.t(b.pos).zone == zid).map(|b| b.id).min();
                    let o = if first == Some(bid) {
                        owner.unwrap()
                    } else {
                        let o = owner.unwrap();
                        match self.dwarves.iter().find(|d| d.id == o).and_then(|d| d.spouse).filter(|s| self.dwarves.iter().any(|d| d.id == *s && d.alive) && !self.zones.values().any(|z| z.kind == ZoneKind::Bedroom && z.owner == Some(*s))) {
                            Some(s) => s,
                            None => {
                                self.buildings.get_mut(&bid).unwrap().owner = None;
                                continue;
                            }
                        }
                    };
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
        self.has_zone_kind(p, ZoneKind::Dining)
    }

    /// Mark one item, or every loose item on a tile, for the garbage dump.
    pub fn toggle_dump(&mut self, item: u32) -> Option<bool> {
        let it = self.items.get_mut(&item)?;
        // Tools and weapons in someone's hands stay there.
        if it.carried_by.is_some() {
            return None;
        }
        it.dump = !it.dump;
        it.forbidden = false;
        it.to_depot = false;
        it.container = None;
        Some(it.dump)
    }

    /// A stockpile's settings, one row each for the kinds of item its
    /// category takes and for the materials of such items the fortress has
    /// (or that it already refuses): (label, allowed, what toggling flips).
    pub fn pile_rows(&self, zone: u32) -> Vec<(String, bool, PileToggle)> {
        let Some(z) = self.zones.get(&zone) else { return Vec::new() };
        let ZoneKind::Stockpile(f) = z.kind else { return Vec::new() };
        let mut v = Vec::new();
        for k in ItemKind::ALL {
            if super::stock_ok(f, k) {
                v.push((k.plural().to_string(), !z.deny.contains(&k), PileToggle::Kind(k)));
            }
        }
        let mut mats: Vec<Mat> = self
            .items
            .values()
            .filter(|it| super::stock_ok(f, it.kind) && !it.kind.is_container())
            .map(|it| it.mat)
            .chain(z.deny_mats.iter().copied())
            .collect();
        mats.sort();
        mats.dedup();
        for m in mats {
            v.push((format!("made of {}", m.name()), !z.deny_mats.contains(&m), PileToggle::Mat(m)));
        }
        v
    }

    /// Flip one stockpile setting.
    pub fn toggle_pile(&mut self, zone: u32, t: PileToggle) {
        let Some(z) = self.zones.get_mut(&zone) else { return };
        match t {
            PileToggle::Kind(k) => {
                if let Some(i) = z.deny.iter().position(|&d| d == k) {
                    z.deny.remove(i);
                } else {
                    z.deny.push(k);
                }
            }
            PileToggle::Mat(m) => {
                if let Some(i) = z.deny_mats.iter().position(|&d| d == m) {
                    z.deny_mats.remove(i);
                } else {
                    z.deny_mats.push(m);
                }
            }
        }
        self.haul_retry = 0;
    }

    /// Forbid or allow an item. None if someone is carrying it.
    pub fn toggle_forbid(&mut self, item: u32) -> Option<bool> {
        let it = self.items.get_mut(&item)?;
        if it.carried_by.is_some() {
            return None;
        }
        it.forbidden = !it.forbidden;
        Some(it.forbidden)
    }

    /// Forbid everything loose on a tile, or allow it all again if it's all
    /// forbidden already. Returns (how many, now forbidden).
    pub fn forbid_tile(&mut self, p: Pos) -> (usize, bool) {
        let here: Vec<u32> = self.items.values().filter(|it| it.pos == p && it.carried_by.is_none()).map(|it| it.id).collect();
        let forbid = here.iter().any(|id| !self.items[id].forbidden);
        for id in &here {
            self.items.get_mut(id).unwrap().forbidden = forbid;
        }
        (here.len(), forbid)
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

    #[test]
    fn a_dining_hall_that_is_also_the_meeting_area_works_for_both() {
        let mut g = Game::new(23);
        let place = |g: &mut Game, kind: BuildingKind, p: Pos| {
            assert!(g.place_building(kind, p));
            let id = *g.buildings.keys().max().unwrap();
            g.buildings.get_mut(&id).unwrap().built = true;
            id
        };
        // A table and chair near the food, and the hall further off.
        let near = open_patch(&g, 2, 1, &HashSet::new());
        place(&mut g, BuildingKind::Table, near);
        place(&mut g, BuildingKind::Chair, near.offset(1, 0, 0));
        let skip: HashSet<Pos> = box_iter(near.offset(-6, -6, 0), near.offset(7, 6, 0)).collect();
        let hall = open_patch(&g, 3, 3, &skip);
        place(&mut g, BuildingKind::Table, hall.offset(1, 1, 0));
        let hall_chair = place(&mut g, BuildingKind::Chair, hall.offset(2, 1, 0));
        let far = hall.offset(2, 2, 0);
        // The meeting area first, then the dining hall laid over the same tiles.
        assert_eq!(g.add_zone(hall, far, ZoneKind::Meeting), 9);
        assert_eq!(g.add_zone(hall, far, ZoneKind::Dining), 9, "the dining hall wasn't laid over the meeting area");
        assert_eq!(g.add_zone(hall, far, ZoneKind::Dining), 0, "two dining halls on one tile");
        assert_eq!(g.add_zone(hall, far, ZoneKind::Stockpile(crate::items::StockFilter::All)), 0, "a stockpile shares a hall");
        for p in box_iter(hall, far) {
            assert!(g.in_dining_hall(p) && g.in_meeting_area(p), "at {:?}", p);
            assert_eq!(g.zones_at(p).count(), 2);
        }
        // It survives a save: the overlay is rebuilt from the zones.
        let mut back: Game = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        back.after_load();
        assert!(back.in_dining_hall(hall) && back.in_meeting_area(hall), "lost on loading");

        // A hungry dwarf eats at its table.
        g.dwarves[0].hunger = crate::game::HUNGRY + 10;
        g.dwarves[0].idle_cd = 0;
        for _ in 0..50 {
            g.step();
            if matches!(g.dwarves[0].task, Task::Dine { .. } | Task::Eat { .. }) {
                break;
            }
        }
        assert!(matches!(g.dwarves[0].task, Task::Dine { seat, .. } if seat == hall_chair), "went for {:?}", g.dwarves[0].task);
        // Idle dwarves gather there.
        assert!(box_iter(hall, far).all(|p| g.meeting_tiles().contains(&p)));
        let mut met = false;
        for _ in 0..DAY {
            g.step();
            met |= g.dwarves.iter().skip(1).any(|d| d.alive && g.in_dining_hall(d.pos) && g.in_meeting_area(d.pos) && !matches!(d.task, Task::Dine { .. } | Task::Eat { .. }));
            if met {
                break;
            }
        }
        assert!(met, "nobody gathered in the hall");

        // Taking the meeting area away leaves the dining hall whole.
        assert_eq!(g.remove_zones_where(hall, far, |k| k == ZoneKind::Meeting), 9);
        assert!(box_iter(hall, far).all(|p| g.in_dining_hall(p) && !g.in_meeting_area(p)));
        assert!(g.zone_over.is_empty());
    }

    #[test]
    fn bulk_zoning_gives_each_room_its_own_zone() {
        let mut g = Game::new(21);
        // Three 2x2 cells side by side, walled apart and roofed over.
        let a = open_patch(&g, 10, 4, &HashSet::new());
        for p in box_iter(a, a.offset(9, 3, 0)) {
            let inner = (p.y == a.y + 1 || p.y == a.y + 2) && (p.x - a.x) % 3 != 0;
            if !inner {
                g.world.t_mut(p).kind = TileKind::Wall;
            }
            g.world.t_mut(p.up()).kind = TileKind::Floor;
            g.world.t_mut(p.up()).built_from = Some(ItemKind::Block);
        }
        let (zones, tiles) = g.add_zone_rooms(a, a.offset(9, 3, 0), ZoneKind::Bedroom);
        assert_eq!((zones, tiles), (3, 12), "each cell its own bedroom");
        let ids: HashSet<u32> = [1, 4, 7].iter().map(|&dx| g.world.t(a.offset(dx, 1, 0)).zone).collect();
        assert_eq!(ids.len(), 3);
    }

    /// A dwarf faint with hunger (or mad, and starving) who sits down to a
    /// meal at a table eats it: the meal was once dropped every tick as
    /// "ordinary work" that hunger interrupts, or as no task for the mad,
    /// and picked up again, and they starved at the table.
    #[test]
    fn a_starving_dwarf_eats_at_the_table() {
        let mut g = Game::new(23);
        g.creatures.retain(|_, c| c.tame);
        let place = |g: &mut Game, kind: BuildingKind, p: Pos| {
            assert!(g.place_building(kind, p));
            let id = *g.buildings.keys().max().unwrap();
            g.buildings.get_mut(&id).unwrap().built = true;
            id
        };
        let near = open_patch(&g, 4, 1, &HashSet::new());
        place(&mut g, BuildingKind::Table, near);
        place(&mut g, BuildingKind::Chair, near.offset(1, 0, 0));
        place(&mut g, BuildingKind::Table, near.offset(2, 0, 0));
        place(&mut g, BuildingKind::Chair, near.offset(3, 0, 0));
        for _ in 0..6 {
            g.spawn_item(ItemKind::Meal, Mat::Stoutcap, near.offset(0, 1, 0));
        }
        g.dwarves[0].hunger = crate::game::HUNGRY + 6000;
        g.dwarves[1].hunger = crate::game::HUNGRY + 13000;
        g.dwarves[1].insane = true;
        let mut ate = [false; 2];
        let mut dined = [false; 2];
        for _ in 0..DAY {
            g.step();
            for i in 0..2 {
                dined[i] |= matches!(g.dwarves[i].task, Task::Dine { .. });
                ate[i] |= g.dwarves[i].hunger < crate::game::HUNGRY;
            }
            if ate == [true, true] {
                break;
            }
        }
        assert!(dined[0] && ate[0], "the starving dwarf never finished a meal at the table");
        assert!(ate[1], "the mad dwarf starves with meals at hand (dined: {})", dined[1]);
    }

    #[test]
    fn forbidden_items_are_left_alone_until_allowed() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let e = g.embark;
        assert!(g.add_zone(e.offset(-8, 3, -1), e.offset(-1, 9, 1), ZoneKind::Stockpile(crate::items::StockFilter::Wood)) > 0);
        let spot = (4..20).map(|r| e.offset(r, -4, 0)).find(|p| g.world.walkable(*p) && g.world.t(*p).zone == 0).unwrap();
        let log = g.spawn_item(ItemKind::Log, crate::material::Mat::Wood, spot);
        let p0 = g.items[&log].pos;
        assert_eq!(g.toggle_forbid(log), Some(true));
        // Forbidden food isn't eaten either.
        for it in g.items.values_mut().filter(|it| it.kind.is_food()) {
            it.forbidden = true;
        }
        let food = g.items.values().filter(|it| it.kind.is_food()).count();
        for d in g.dwarves.iter_mut() {
            d.hunger = crate::game::HUNGRY + 1;
        }
        for _ in 0..DAY {
            g.step();
        }
        assert_eq!(g.items[&log].pos, p0, "a forbidden log was hauled");
        assert_eq!(g.items.values().filter(|it| it.kind.is_food()).count(), food, "forbidden food was eaten");
        assert_eq!(g.toggle_forbid(log), Some(false));
        for it in g.items.values_mut() {
            it.forbidden = false;
        }
        for _ in 0..2 * DAY {
            g.step();
        }
        let p = g.items[&log].pos;
        assert!(matches!(g.zone_kind_at(p), Some(ZoneKind::Stockpile(_))), "allowed log never reached the stockpile");
        let (n, on) = g.forbid_tile(p);
        assert!(n >= 1 && on);
        assert!(g.items[&log].forbidden);
    }

    #[test]
    fn a_stockpile_refuses_the_kinds_and_materials_it_is_told_to() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let e = g.embark;
        assert!(g.add_zone(e.offset(-8, 3, -1), e.offset(-1, 9, 1), ZoneKind::Stockpile(crate::items::StockFilter::Stone)) > 0);
        let zone = *g.zones.keys().last().unwrap();
        let rows = g.pile_rows(zone);
        assert!(rows.iter().any(|r| r.0 == "boulders") && rows.iter().any(|r| r.0 == "blocks"));
        let spot = (4..20).map(|r| e.offset(r, -4, 0)).find(|p| g.world.walkable(*p) && g.world.t(*p).zone == 0).unwrap();
        let granite = g.spawn_item(ItemKind::Boulder, Mat::Granite, spot);
        let marble = g.spawn_item(ItemKind::Boulder, Mat::Marble, spot);
        let block = g.spawn_item(ItemKind::Block, Mat::Marble, spot);
        g.toggle_pile(zone, PileToggle::Mat(Mat::Granite));
        g.toggle_pile(zone, PileToggle::Kind(ItemKind::Block));
        let rows = g.pile_rows(zone);
        assert!(rows.iter().any(|r| r.0 == "made of granite" && !r.1), "granite row: {:?}", rows.iter().map(|r| (&r.0, r.1)).collect::<Vec<_>>());
        for _ in 0..DAY {
            g.step();
        }
        let in_pile = |g: &Game, id: u32| g.zone_at(g.items[&id].pos).map_or(false, |z| z.id == zone);
        assert!(in_pile(&g, marble), "marble boulder should be stored");
        assert!(!in_pile(&g, granite), "granite was refused");
        assert!(!in_pile(&g, block), "blocks were refused");
        // Allowing granite again brings it in.
        g.toggle_pile(zone, PileToggle::Mat(Mat::Granite));
        for _ in 0..DAY {
            g.step();
        }
        assert!(in_pile(&g, granite), "granite should be stored once allowed");
    }
}
