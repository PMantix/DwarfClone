//! The overseer's interior architect: how the inside of a fort grows up.
//!
//! After the first dormitory come bedrooms of their own (a cabinet and a
//! chest each as the fort can spare them; couples share, families sleep
//! side by side), and once every dwarf has one the dormitory is retired and
//! turned to another use. Nobles get suites (a bedchamber, an office and a
//! dining room behind their own doors). As the fort settles in it digs
//! gathering rooms: a parlour, a library, a barracks with a training floor,
//! a temple, a tavern with a bar and a stage, and a great dining hall with
//! rows of tables and statues; the halls are smoothed and engraved. Its
//! taste shows: a vain overseer builds grand pillared halls full of statues,
//! a cautious one keeps things compact, a pious one a great temple, the
//! reckless hardly bother. (`maturity.rs` has how far the fort has come,
//! and the rebuilding: the industry quarter, the hospital wing, tearing
//! down the rough early work.)

use super::blueprint::Class;
use super::construct::{BuildMat, ConstructKind};
use super::layout::Shape;
use super::maturity::Chain;
use super::overseer::{Overseer, Room, RoomUse, Style};
use super::personality::Trait;
use super::*;

/// What the architect remembers (kept on the overseer).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Interior {
    /// How far the fort has come: 0 founding, 1 settled, 2 established,
    /// 3 mature (`maturity.rs`), and since when.
    pub tier: u8,
    pub tier_since: u64,
    /// Rooms with names, for the look panel, the notes and the camera.
    pub places: Vec<Place>,
    /// The industry quarter: each room (by its corner) and its chain.
    pub quarter: Vec<(Pos, Chain)>,
    /// Nobles' suites: the room's corner, a tile of its bedchamber and of
    /// its office, and whose it is.
    pub suites: Vec<Suite>,
    /// The tavern's stage, where bards perform.
    pub stage: Vec<Pos>,
    /// The great dining hall (its corner), once there is one, and when one
    /// was wanted.
    pub great: Option<Pos>,
    pub great_wanted: u64,
    /// When it last made a storeroom over into something better.
    pub repurposed: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Place {
    pub a: Pos,
    pub b: Pos,
    pub name: String,
    /// When it took this name (a new hall draws the camera for a while).
    pub since: u64,
    /// Worth a visit from the camera.
    pub notable: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Suite {
    pub a: Pos,
    pub bed: Pos,
    pub office: Option<Pos>,
    pub owner: Option<u32>,
}

/// How an overseer likes its halls.
#[derive(Clone, Copy, Debug)]
pub struct Taste {
    /// 0 plain .. 3 lavish: statues, pillars and engraving.
    pub grand: u8,
    /// Small rooms, close together.
    pub compact: bool,
    pub pious: bool,
    /// Hardly bothers at all (the reckless).
    pub plain: bool,
}

pub fn taste(o: &Overseer) -> Taste {
    let mut grand: i32 = match o.style {
        Style::Builder => 2,
        Style::Reckless => 0,
        _ => 1,
    };
    if o.has(Trait::Vain) {
        grand += 1;
    }
    if o.has(Trait::Cautious) {
        grand -= 1;
    }
    let plain = o.style == Style::Reckless;
    if plain {
        grand = 0;
    }
    Taste { grand: grand.clamp(0, 3) as u8, compact: o.has(Trait::Cautious), pious: o.has(Trait::Pious), plain }
}

/// Halls where the fort gathers.
fn gathering(k: RoomUse) -> bool {
    matches!(k, RoomUse::Dining | RoomUse::Tavern | RoomUse::Temple | RoomUse::Meeting | RoomUse::Library)
}

/// Whether it houses its dwarves in rooms of their own now (past the first
/// dormitory). The reckless, digging where they please, keep to dormitories.
pub(super) fn sleeps_in_rooms(o: &Overseer) -> bool {
    (o.layout.planned() || o.style != Style::Reckless) && (o.rooms.iter().any(|r| r.kind == RoomUse::Dormitory) || o.did("dorm-retired"))
}

/// The sizes of room it would like for `kind`, best first: its taste
/// reorders the plan's own wishes.
pub(super) fn classes(o: &Overseer, kind: RoomUse) -> Vec<Class> {
    let mut v = super::blueprint::wants(kind).to_vec();
    let t = taste(o);
    let size = |c: &Class| match c {
        Class::Cells => 0,
        Class::Small => 1,
        Class::Medium => 2,
        Class::Large => 3,
    };
    if gathering(kind) || kind == RoomUse::Suite {
        if t.compact {
            // Snug rooms: the smallest that will do, first.
            v.sort_by_key(|c| if *c == Class::Small && kind != RoomUse::Library { 9 } else { size(c) });
        } else if t.grand >= 2 || (t.pious && kind == RoomUse::Temple) {
            v.sort_by_key(|c| -size(c));
        }
    }
    v
}

/// A big hall cut by a grand taste keeps pillars standing.
pub(super) fn restyle(o: &Overseer, kind: RoomUse, shape: Shape, a: Pos, tiles: &[Pos]) -> (Shape, Vec<Pos>) {
    let t = taste(o);
    if let Shape::Rect { w, h } = shape {
        let grand = t.grand >= 2 || (t.pious && kind == RoomUse::Temple && t.grand >= 1);
        if grand && gathering(kind) && w.min(h) >= 6 && w.max(h) >= 9 {
            let s = Shape::Pillared { w, h };
            return (s, s.tiles().into_iter().map(|(x, y)| a.offset(x, y, 0)).collect());
        }
    }
    (shape, tiles.to_vec())
}

/// Bedrooms of their own where the halls have no plan (one small room off
/// the tunnel each), and the architect's other rooms, in its taste.
pub(super) fn taste_shape(o: &Overseer, kind: RoomUse) -> Option<Shape> {
    let t = taste(o);
    match kind {
        RoomUse::Bedrooms if pods(o) => Some(Shape::Rect { w: 3, h: 3 }),
        RoomUse::Suite => Some(Shape::Rect { w: 11, h: if t.compact { 4 } else { 5 } }),
        RoomUse::Industry => Some(Shape::Rect { w: 8, h: 6 }),
        RoomUse::Library => Some(Shape::Rect { w: if t.compact { 5 } else { 7 }, h: 5 }),
        RoomUse::Parlour => Some(Shape::Rect { w: 5, h: 4 }),
        k if gathering(k) && !t.plain && t.grand >= 2 => Some(Shape::Pillared { w: 11, h: 9 }),
        k if gathering(k) && t.compact => Some(Shape::Rect { w: 6, h: 5 }),
        _ => None,
    }
}

/// Whether bedrooms may come one to a small room off the tunnels (where
/// there's no plan of halls, or its rows of bedrooms are used up).
pub(super) fn pods(o: &Overseer) -> bool {
    o.style != Style::Reckless
}

/// A room dug off a tunnel (no plan): a bedroom of its own gets a door at
/// the tunnel's mouth and its bed at the far end; anything else is as
/// asked. (The reckless still want dormitories.)
pub(super) fn pod(o: &Overseer, kind: RoomUse, tiles: &[Pos], path: &[Pos]) -> (RoomUse, Vec<Pos>, Vec<Pos>) {
    if kind != RoomUse::Bedrooms {
        return (kind, Vec::new(), Vec::new());
    }
    if !pods(o) || tiles.is_empty() {
        return (RoomUse::Dormitory, Vec::new(), Vec::new());
    }
    let door = path.first().copied();
    let from = door.unwrap_or(tiles[0]);
    let bed = *tiles.iter().max_by_key(|p| (p.x - from.x).abs() + (p.y - from.y).abs()).unwrap();
    (RoomUse::Bedrooms, door.into_iter().collect(), vec![bed])
}

pub(super) fn room_tiles(r: &Room) -> Vec<Pos> {
    if r.tiles.is_empty() {
        box_iter(r.a, r.b).collect()
    } else {
        r.tiles.clone()
    }
}

fn cheb(a: Pos, b: Pos) -> i32 {
    (a.x - b.x).abs().max((a.y - b.y).abs())
}

/// One bedroom of a row: its tiles, its bed, its door.
pub(super) struct Cell {
    pub tiles: Vec<Pos>,
    pub bed: Pos,
    pub door: Option<Pos>,
}

/// The bedrooms of a row (or the one room of a pod).
pub(super) fn cells_of(r: &Room) -> Vec<Cell> {
    let tiles = room_tiles(r);
    r.beds
        .iter()
        .enumerate()
        .map(|(i, &bed)| Cell {
            tiles: tiles.iter().copied().filter(|&t| cheb(t, bed) <= 2 && r.beds.iter().all(|&o| cheb(t, o) >= cheb(t, bed))).collect(),
            bed,
            door: r.doors.get(i).copied(),
        })
        .collect()
}

impl Cell {
    /// Beside the bed for a cabinet and a chest, the far side first.
    fn furniture(&self) -> Vec<Pos> {
        let from = self.door.unwrap_or(self.bed);
        let mut v: Vec<Pos> = DIRS4.iter().map(|&(dx, dy)| self.bed.offset(dx, dy, 0)).filter(|p| self.tiles.contains(p) && cheb(*p, from) > 1).collect();
        v.sort_by_key(|p| -((p.x - from.x).abs() + (p.y - from.y).abs()));
        // (A tile further along the back wall, when the bed has only one beside it.)
        for &t in &self.tiles {
            if v.len() >= 2 {
                break;
            }
            if t != self.bed && !v.contains(&t) && cheb(t, from) > 1 && cheb(t, self.bed) == 1 {
                v.push(t);
            }
        }
        v
    }

    /// Where a husband's or wife's bed goes: by the first, clear of the door.
    fn second_bed(&self) -> Option<Pos> {
        let from = self.door.unwrap_or(self.bed);
        let taken = self.furniture();
        self.tiles.iter().copied().filter(|&t| t != self.bed && !taken.iter().take(2).any(|f| *f == t) && cheb(t, from) > 1).min_by_key(|&t| (cheb(t, self.bed), t.x, t.y))
    }
}

/// "Urist" from "Urist Gemcutter".
fn first_name(n: &str) -> String {
    n.split(' ').next().unwrap_or(n).to_string()
}

/// A few words in its voice after a note on its halls, now and then.
fn flourish(o: &Overseer, text: &str) -> String {
    let Some(t) = o.traits.first() else { return text.to_string() };
    let tail = match t {
        Trait::Vain => " The visitors will talk of it.",
        Trait::Cautious => " Nothing more than needed.",
        Trait::Pious => " The gods will be pleased.",
        Trait::Greedy => " It cost little enough.",
        Trait::Sentimental => " They deserve no less.",
        Trait::Cheerful => " Splendid!",
        Trait::Bold => " About time.",
        Trait::Stubborn => " Just as it planned.",
    };
    let h = text.bytes().fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
    if h % 3 == 0 {
        format!("{}{}", text, tail)
    } else {
        text.to_string()
    }
}

const TAVERN_ADJ: [&str; 10] = ["Copper", "Drunken", "Gilded", "Laughing", "Bearded", "Rusty", "Golden", "Sleeping", "Merry", "Iron"];
const TAVERN_NOUN: [&str; 10] = ["Tankard", "Anvil", "Barrel", "Goblet", "Pickaxe", "Hearth", "Boar", "Mug", "Lantern", "Keg"];

impl Game {
    /// What stands on `p` (planned or built).
    pub(super) fn bld_kind_at(&self, p: Pos) -> Option<BuildingKind> {
        match self.world.get(p).map_or(0, |t| t.building) {
            0 => None,
            id => self.buildings.get(&id).map(|b| b.kind),
        }
    }

    /// Items of this kind not already spoken for by furniture waiting to be built.
    pub(super) fn spare(&self, kind: BuildingKind) -> usize {
        let item = match kind {
            BuildingKind::Bed => ItemKind::Bed,
            BuildingKind::Table => ItemKind::Table,
            BuildingKind::Chair => ItemKind::Chair,
            BuildingKind::Cabinet => ItemKind::Cabinet,
            BuildingKind::Chest => ItemKind::Chest,
            BuildingKind::Statue => ItemKind::Statue,
            BuildingKind::Door => ItemKind::Door,
            _ => return 1,
        };
        let waiting = self.buildings.values().filter(|b| b.kind == kind && !b.built).count();
        let have = self.items.values().filter(|it| it.kind == item && it.carried_by.map_or(true, |c| !self.dwarves.iter().any(|d| d.id == c && d.held == Some(it.id))) && !it.forbidden).count();
        have.saturating_sub(waiting)
    }

    /// Where a room is entered: its first door, else the tile outside it
    /// where its tunnel comes in.
    pub(super) fn entry(&self, r: &Room) -> Pos {
        if let Some(&d) = r.doors.first() {
            return d;
        }
        let tiles = room_tiles(r);
        for &t in &tiles {
            for &(dx, dy) in DIRS4.iter() {
                let q = t.offset(dx, dy, 0);
                if !tiles.contains(&q) && self.world.get(q).map_or(false, |x| x.dug && !x.kind.solid()) {
                    return q;
                }
            }
        }
        Pos::new((r.a.x + r.b.x) / 2, (r.a.y + r.b.y) / 2, r.a.z)
    }

    /// Room tiles along its walls, clear of its doors: corners first, then
    /// the far ones.
    pub(super) fn wall_spots(&self, r: &Room) -> Vec<Pos> {
        let tiles = room_tiles(r);
        let doors: Vec<Pos> = r.doors.iter().copied().chain(std::iter::once(self.entry(r))).collect();
        let solid = |p: Pos| self.world.get(p).map_or(true, |t| t.kind.solid()) && !tiles.contains(&p);
        let mut v: Vec<(i32, i32, Pos)> = tiles
            .iter()
            .copied()
            .filter(|&p| doors.iter().all(|&d| cheb(p, d) > 1))
            .filter(|&p| self.world.t(p).zone == 0 || !matches!(self.zone_kind_at(p), Some(ZoneKind::Stockpile(_))))
            .filter_map(|p| {
                let walls = DIRS4.iter().filter(|&&(dx, dy)| solid(p.offset(dx, dy, 0))).count() as i32;
                (walls > 0).then(|| (-walls, -doors.iter().map(|d| (p.x - d.x).abs() + (p.y - d.y).abs()).min().unwrap_or(0), p))
            })
            .collect();
        v.sort_by_key(|x| (x.0, x.1, x.2.x, x.2.y));
        v.into_iter().map(|x| x.2).collect()
    }

    /// Put up to `n` of `kind` in the room (counting those already there) on
    /// the given spots, as stock allows. Returns how many it placed.
    fn furnish(&mut self, r: &Room, kind: BuildingKind, spots: &[Pos], n: usize) -> usize {
        let tiles = room_tiles(r);
        let have = self.buildings.values().filter(|b| b.kind == kind && tiles.contains(&b.pos)).count();
        let mut spare = self.spare(kind);
        let mut placed = 0;
        for &p in spots {
            if have + placed >= n || spare == 0 {
                break;
            }
            if self.world.t(p).building == 0 && self.place_building(kind, p) {
                placed += 1;
                spare -= 1;
            }
        }
        placed
    }

    /// Daily: the inside of the fort grows up (from `ov_daily_hooks`).
    pub(super) fn ov_interior(&mut self, o: &mut Overseer) {
        if o.base.is_none() {
            return;
        }
        self.ov_tier(o);
        self.ov_bedrooms(o);
        self.ov_retire_dorm(o);
        self.ov_suites(o);
        self.ov_wishes(o);
        self.ov_dress(o);
        self.ov_finish(o);
        self.ov_halls(o);
        self.ov_mature(o);
        self.ov_repurpose(o);
        self.ov_places(o);
    }

    // ------------------------------------------------------------------
    // Living quarters
    // ------------------------------------------------------------------

    /// Each row of bedrooms zoned room by room and furnished as stock
    /// allows; every dwarf given a room (couples sharing, families side by
    /// side); and more rows dug while there are dwarves without one.
    fn ov_bedrooms(&mut self, o: &mut Overseer) {
        let t = taste(o);
        let rows: Vec<Room> = o.rooms.iter().filter(|r| r.stage == 1 && r.kind == RoomUse::Bedrooms).cloned().collect();
        let mut cells: Vec<(u32, Cell)> = Vec::new();
        for r in &rows {
            for c in cells_of(r) {
                if self.bld_kind_at(c.bed).is_none() {
                    self.place_building(BuildingKind::Bed, c.bed);
                }
                if self.world.t(c.bed).zone == 0 {
                    self.add_zone_tiles(c.tiles.clone(), ZoneKind::Bedroom);
                }
                let z = self.world.t(c.bed).zone;
                if z != 0 && self.zones.get(&z).map_or(false, |z| z.kind == ZoneKind::Bedroom) {
                    cells.push((z, c));
                }
            }
        }
        // A cabinet and a chest in each, once the fort has found its feet.
        if o.interior.tier >= 1 && !t.plain {
            for (_, c) in &cells {
                let spots = c.furniture();
                for (kind, spot) in [BuildingKind::Cabinet, BuildingKind::Chest].into_iter().zip(spots) {
                    if self.world.t(spot).building == 0 && self.spare(kind) > 0 {
                        self.place_building(kind, spot);
                    }
                }
            }
        }
        self.house(&cells);
        // More rooms while some go without (and one to spare for the next
        // migrant), once the essentials are dug.
        let people: Vec<&Dwarf> = self.dwarves.iter().filter(|d| d.alive && !d.insane).collect();
        let couples = people.iter().filter(|d| d.spouse.map_or(false, |s| people.iter().any(|x| x.id == s)) && d.id < d.spouse.unwrap()).count();
        let need = people.len() - couples;
        let planned: usize = o.rooms.iter().filter(|r| r.kind == RoomUse::Bedrooms && r.stage < 2).map(|r| r.beds.len()).sum();
        let suites = o.interior.suites.iter().filter(|s| s.owner.is_some()).count();
        if o.interior.tier >= 1
            && sleeps_in_rooms(o)
            && planned + suites < need + 2
            && !o.wants.contains(&RoomUse::Bedrooms)
            && !o.rooms.iter().any(|r| r.stage == 0 && r.kind == RoomUse::Bedrooms)
            && !o.blocked_lately(RoomUse::Bedrooms, self.tick)
        {
            o.wants.push(RoomUse::Bedrooms);
        }
    }

    /// Who sleeps where: everyone without a room of their own gets one, a
    /// husband or wife shares their spouse's (a second bed goes in), and
    /// the rest take a free room as near their family as can be.
    fn house(&mut self, cells: &[(u32, Cell)]) {
        let owner_of = |g: &Game, z: u32| g.zones.get(&z).and_then(|z| z.owner);
        let owns_room = |g: &Game, id: u32| g.zones.values().any(|z| z.kind == ZoneKind::Bedroom && z.owner == Some(id));
        let beds_in = |g: &Game, z: u32| g.buildings.values().filter(|b| b.kind == BuildingKind::Bed && g.world.t(b.pos).zone == z).count();
        let mut people: Vec<(u8, u32)> = self
            .dwarves
            .iter()
            .filter(|d| d.alive && !d.insane)
            .map(|d| (if d.noble.is_some() { 0 } else if d.is_child() { 2 } else { 1 }, d.id))
            .collect();
        people.sort();
        let mut moved = 0;
        for (_, id) in people {
            if moved >= 6 {
                break;
            }
            if owns_room(self, id) {
                continue;
            }
            let d = self.dwarves.iter().find(|d| d.id == id).unwrap();
            let spouse = d.spouse.filter(|s| self.dwarves.iter().any(|x| x.id == *s && x.alive));
            // Sharing with a husband or wife who has a room.
            if let Some(s) = spouse {
                if let Some(z) = self.zones.values().find(|z| z.kind == ZoneKind::Bedroom && z.owner == Some(s)).map(|z| z.id) {
                    if beds_in(self, z) < 2 {
                        let spot = match cells.iter().find(|c| c.0 == z) {
                            Some((_, c)) => c.second_bed(),
                            None => self.zones[&z].tiles.iter().copied().find(|&p| self.world.t(p).building == 0),
                        };
                        if let Some(p) = spot {
                            self.place_building(BuildingKind::Bed, p);
                        }
                    }
                    continue;
                }
            }
            let family: Vec<u32> = {
                let mut f: Vec<u32> = spouse.into_iter().collect();
                if let Some((a, b)) = d.parents {
                    f.extend([a, b]);
                }
                f.extend(self.dwarves.iter().filter(|x| x.alive && (x.parents.map_or(false, |(a, b)| a == id || b == id) || (d.parents.is_some() && x.parents == d.parents && x.id != id))).map(|x| x.id));
                f
            };
            let near: Vec<Pos> = cells.iter().filter(|(z, _)| owner_of(self, *z).map_or(false, |w| family.contains(&w))).map(|(_, c)| c.bed).collect();
            let free = cells
                .iter()
                .enumerate()
                .filter(|(_, (z, _))| owner_of(self, *z).is_none() && beds_in(self, *z) > 0)
                .min_by_key(|(i, (_, c))| if near.is_empty() { *i as i32 } else { near.iter().map(|&n| (n.x - c.bed.x).abs() + (n.y - c.bed.y).abs() + 50 * (n.z - c.bed.z).abs()).min().unwrap() })
                .map(|(_, (z, _))| *z);
            if let Some(z) = free {
                self.set_zone_owner(z, Some(id));
                moved += 1;
            }
        }
    }

    /// Once every dwarf has a room of their own (and one is free for the
    /// next to come), the dormitory is emptied and put to another use.
    fn ov_retire_dorm(&mut self, o: &mut Overseer) {
        let t = taste(o);
        if o.interior.tier < 1 || t.plain || !sleeps_in_rooms(o) {
            return;
        }
        let dorms: Vec<usize> = (0..o.rooms.len()).filter(|&i| o.rooms[i].kind == RoomUse::Dormitory && o.rooms[i].stage == 1).collect();
        if dorms.is_empty() {
            return;
        }
        let in_room = |g: &Game, id: u32| g.buildings.values().any(|b| b.kind == BuildingKind::Bed && b.built && b.owner == Some(id) && g.zone_kind_at(b.pos) == Some(ZoneKind::Bedroom));
        if self.dwarves.iter().any(|d| d.alive && !d.insane && !in_room(self, d.id)) {
            return;
        }
        let free = self.zones.values().filter(|z| z.kind == ZoneKind::Bedroom && z.owner.is_none() && z.tiles.iter().any(|p| self.buildings.get(&self.world.t(*p).building).map_or(false, |b| b.kind == BuildingKind::Bed && b.built))).count();
        // (And two to spare, for the next to come.)
        if free < 2 {
            return;
        }
        for i in dorms {
            let r = o.rooms[i].clone();
            let tiles = room_tiles(&r);
            let beds: Vec<u32> = self.buildings.values().filter(|b| b.kind == BuildingKind::Bed && tiles.contains(&b.pos)).map(|b| b.id).collect();
            // Never while someone sleeps in it.
            let asleep = self.dwarves.iter().any(|d| d.alive && matches!(d.task, Task::Sleep { bed: Some(b), .. } | Task::Rest { bed: Some(b), .. } if beds.contains(&b)));
            if asleep {
                continue;
            }
            for b in beds {
                self.remove_building(b);
            }
            self.remove_zones_where(r.a, r.b, |k| k == ZoneKind::Dormitory);
            let new = if t.grand >= 1 && !o.rooms.iter().any(|x| x.kind == RoomUse::Library && x.stage < 2) {
                RoomUse::Library
            } else if o.style == Style::Warlord && !o.rooms.iter().any(|x| x.kind == RoomUse::Barracks && x.stage < 2) {
                RoomUse::Barracks
            } else {
                RoomUse::Parlour
            };
            o.rooms[i].kind = new;
            o.rooms[i].since = self.tick;
            let room = o.rooms[i].clone();
            if let Some(z) = self.fit_interior(o, &room) {
                self.add_zone(room.a, room.b, z);
            }
            if !o.did("dorm-retired") {
                o.done.push("dorm-retired".into());
            }
            let what = flourish(o, &format!("Retired the old dormitory: every dwarf has a room of their own now. It will be a {}.", super::overseer::room_name(new)));
            Self::note_ov(o, self.tick, what);
            self.count("interior.dorm_retired", 1);
        }
    }

    /// Nobles get suites as their rank demands: a mayor and up (and, under
    /// a vain overseer, the expedition leader too, once the fort can afford it).
    fn ov_suites(&mut self, o: &mut Overseer) {
        let t = taste(o);
        if o.interior.tier < 1 || t.plain {
            return;
        }
        let alive = |g: &Game, id: u32| g.dwarves.iter().any(|d| d.id == id && d.alive && !d.insane);
        for s in o.interior.suites.iter_mut() {
            if s.owner.map_or(false, |id| !alive(self, id)) {
                s.owner = None;
            }
        }
        let worthy: Vec<u32> = {
            let mut v: Vec<(std::cmp::Reverse<u32>, u32)> = self
                .dwarves
                .iter()
                .filter(|d| d.alive && !d.insane)
                .filter_map(|d| d.noble.map(|n| (n, d.id)))
                .filter(|(n, _)| *n != crate::dwarf::Noble::Leader || (t.grand >= 2 && o.interior.tier >= 2))
                .map(|(n, id)| (std::cmp::Reverse(n.room_demand()), id))
                .collect();
            v.sort();
            v.into_iter().map(|x| x.1).collect()
        };
        // Empty suites go to whoever deserves one and has none.
        for i in 0..o.interior.suites.len() {
            if o.interior.suites[i].owner.is_some() {
                continue;
            }
            let Some(&id) = worthy.iter().find(|&&id| !o.interior.suites.iter().any(|s| s.owner == Some(id))) else { break };
            let s = o.interior.suites[i].clone();
            for p in std::iter::once(s.bed).chain(s.office) {
                let z = self.world.t(p).zone;
                if z != 0 {
                    self.set_zone_owner(z, Some(id));
                }
            }
            o.interior.suites[i].owner = Some(id);
            let who = self.dwarves.iter().find(|d| d.id == id).map(|d| (d.name.clone(), d.noble.map_or("noble", |n| n.title())));
            if let Some((name, title)) = who {
                let what = flourish(o, &format!("Gave {} ({}) a suite of rooms: a bedchamber, an office and a dining room.", name, title.to_lowercase()));
                Self::note_ov(o, self.tick, what);
            }
        }
        let homeless = worthy.iter().any(|id| !o.interior.suites.iter().any(|s| s.owner == Some(*id)));
        let pending = o.rooms.iter().any(|r| r.kind == RoomUse::Suite && r.stage == 0) || o.interior.suites.iter().any(|s| s.owner.is_none());
        if homeless && !pending && !o.wants.contains(&RoomUse::Suite) && !o.blocked_lately(RoomUse::Suite, self.tick) {
            o.wants.push(RoomUse::Suite);
            Self::note_ov(o, self.tick, "A noble wants rooms befitting their rank: ordered a suite dug.".into());
        }
    }

    // ------------------------------------------------------------------
    // Fitting out the architect's rooms
    // ------------------------------------------------------------------

    /// Zone and furnish a suite, a library, a parlour, a workshop quarter
    /// or a barracks (a dormitory put to new use). Returns the zone to lay
    /// over the whole room, if any (the caller lays it).
    pub(super) fn fit_interior(&mut self, o: &mut Overseer, r: &Room) -> Option<ZoneKind> {
        for &d in &r.doors {
            if self.bld_kind_at(d).is_none() {
                self.place_building(BuildingKind::Door, d);
            }
        }
        match r.kind {
            RoomUse::Suite => {
                self.fit_suite(o, r);
                None
            }
            RoomUse::Industry => {
                self.fit_quarter(o, r);
                None
            }
            RoomUse::Library => {
                // Desks in rows, cabinets of books along the walls.
                let tiles = room_tiles(r);
                let mut n = 0;
                for &p in &tiles {
                    let (dx, dy) = (p.x - r.a.x, p.y - r.a.y);
                    if dx % 3 == 1 && dy % 2 == 1 && n < 6 && tiles.contains(&p.offset(1, 0, 0)) && self.place_building(BuildingKind::Table, p) {
                        self.place_building(BuildingKind::Chair, p.offset(1, 0, 0));
                        n += 1;
                    }
                }
                let spots = self.wall_spots(r);
                self.furnish(r, BuildingKind::Cabinet, &spots, 4);
                Some(ZoneKind::Meeting)
            }
            RoomUse::Parlour => {
                let c = Pos::new((r.a.x + r.b.x) / 2, (r.a.y + r.b.y) / 2, r.a.z);
                if self.place_building(BuildingKind::Table, c) {
                    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                        self.place_building(BuildingKind::Chair, c.offset(dx, dy, 0));
                    }
                }
                Some(ZoneKind::Meeting)
            }
            RoomUse::Barracks => {
                self.place_building(BuildingKind::ArcheryTarget, r.a);
                Some(ZoneKind::Barracks)
            }
            // (A storeroom made over: see maturity.rs, `ov_repurpose`.)
            RoomUse::Dining | RoomUse::Tavern => {
                let (sx, sy, max) = if r.kind == RoomUse::Dining { (3, 2, 16) } else { (3, 3, 4) };
                let tiles = room_tiles(r);
                let entry = self.entry(r);
                let mut n = 0;
                for &p in &tiles {
                    let (dx, dy) = (p.x - r.a.x, p.y - r.a.y);
                    if n < max && dx % sx == 0 && dy % sy == 0 && cheb(p, entry) > 1 && self.place_building(BuildingKind::Table, p) {
                        if tiles.contains(&p.offset(1, 0, 0)) {
                            self.place_building(BuildingKind::Chair, p.offset(1, 0, 0));
                        }
                        n += 1;
                    }
                }
                Some(if r.kind == RoomUse::Dining { ZoneKind::Dining } else { ZoneKind::Tavern })
            }
            RoomUse::Temple => Some(ZoneKind::Temple),
            RoomUse::Meeting => Some(ZoneKind::Meeting),
            RoomUse::Hospital => {
                let tiles = room_tiles(r);
                let mut n = 0;
                for &p in &tiles {
                    let (dx, dy) = (p.x - r.a.x, p.y - r.a.y);
                    if n < 6 && dx % 3 == 0 && dy % 3 == 0 && p.y < r.b.y && self.place_building(BuildingKind::Bed, p) {
                        n += 1;
                    }
                }
                self.add_zone(Pos::new(r.a.x, r.b.y, r.a.z), r.b, ZoneKind::Stockpile(StockFilter::Medical));
                Some(ZoneKind::Hospital)
            }
            _ => None,
        }
    }

    /// A suite: walls with doors split the room into a bedchamber at the
    /// far end, an office, and (in a long room) a dining room where it's
    /// entered. Walls never go across the way in.
    fn fit_suite(&mut self, o: &mut Overseer, r: &Room) {
        let entry = self.entry(r);
        let (w, h) = (r.b.x - r.a.x + 1, r.b.y - r.a.y + 1);
        let tiles = room_tiles(r);
        // Where the walls go along x (or y): two in a long room, one in a
        // shorter; and where along it the way in comes.
        let cuts = |along_x: bool| -> Option<(i32, i32, Vec<i32>)> {
            let (l, el) = if along_x { (w, entry.x - r.a.x) } else { (h, entry.y - r.a.y) };
            if l >= 9 {
                let c = vec![l / 3, l - 1 - l / 3];
                if !c.contains(&el) {
                    return Some((l, el, c));
                }
            }
            if l >= 6 {
                for c in [l / 2, l / 2 - 1] {
                    if c != el {
                        return Some((l, el, vec![c]));
                    }
                }
            }
            None
        };
        let horiz = match (cuts(true), cuts(false)) {
            (Some(a), Some(b)) => (a.2.len(), w) >= (b.2.len(), h),
            (x, _) => x.is_some(),
        };
        let Some((l, el, cut)) = cuts(horiz) else {
            // Too small to divide: one fine bedchamber.
            let bed = *tiles.iter().max_by_key(|p| (p.x - entry.x).abs() + (p.y - entry.y).abs()).unwrap();
            self.place_building(BuildingKind::Bed, bed);
            self.add_zone(r.a, r.b, ZoneKind::Bedroom);
            o.interior.suites.push(Suite { a: r.a, bed, office: None, owner: None });
            return;
        };
        let at = |s: i32, q: i32| if horiz { r.a.offset(s, q, 0) } else { r.a.offset(q, s, 0) };
        let depth = if horiz { h } else { w };
        let mid = depth / 2;
        // The dividing walls, a door in each.
        for &c in &cut {
            for q in (0..depth).filter(|&q| q != mid) {
                let p = at(c, q);
                self.plan_constructions(p, p, ConstructKind::Wall, BuildMat::Any);
            }
        }
        // The rooms between them, and the one the way in opens on.
        let mut bounds = vec![0];
        for &c in &cut {
            bounds.push(c + 1);
        }
        let ranges: Vec<(i32, i32)> = (0..bounds.len()).map(|i| (bounds[i], if i < cut.len() { cut[i] - 1 } else { l - 1 })).collect();
        let part = |(s0, s1): (i32, i32)| -> Vec<Pos> { (s0..=s1).flat_map(|s| (0..depth).map(move |q| (s, q))).map(|(s, q)| at(s, q)).filter(|p| tiles.contains(p)).collect() };
        let e = el.clamp(0, l - 1);
        let ein = ranges.iter().position(|&(a, b)| e >= a && e <= b).unwrap_or(0);
        let bi = if ein == 0 { ranges.len() - 1 } else { 0 };
        let oi = if ranges.len() == 3 { 3 - ein - bi } else { ein };
        let (br, or) = (ranges[bi], ranges[oi]);
        // The far wall of a room, along the length.
        let far = |(a, b): (i32, i32)| if a == 0 { a } else { b };
        let near = |(a, b): (i32, i32)| if a == 0 { b } else { a };
        self.add_zone_tiles(part(br), ZoneKind::Bedroom);
        self.add_zone_tiles(part(or), ZoneKind::Office);
        if ranges.len() == 3 {
            self.add_zone_tiles(part(ranges[ein]), ZoneKind::Dining);
        }
        for &c in &cut {
            self.place_building(BuildingKind::Door, at(c, mid));
        }
        let bed = at(far(br), mid);
        self.place_building(BuildingKind::Bed, bed);
        let t = taste(o);
        for (k, p) in [(BuildingKind::Cabinet, at(far(br), mid - 1)), (BuildingKind::Chest, at(far(br), mid + 1)), (BuildingKind::Statue, at(near(br), 0))] {
            if part(br).contains(&p) && (k != BuildingKind::Statue || t.grand >= 2) {
                self.place_building(k, p);
            }
        }
        // The office: a desk and its chair, a cabinet.
        let (desk, seat) = if oi == ein { (at(far(or), 0), at(far(or), 1)) } else { (at(far(or), mid), at((far(or) + near(or)) / 2, mid)) };
        if part(or).contains(&desk) && cheb(desk, entry) > 1 && self.place_building(BuildingKind::Table, desk) {
            self.place_building(BuildingKind::Chair, seat);
        }
        let cab = at(far(or), depth - 1);
        if cheb(cab, entry) > 1 {
            self.place_building(BuildingKind::Cabinet, cab);
        }
        // The dining room: a table, a chair either side.
        if ranges.len() == 3 {
            let d = ranges[ein];
            let tp = at((d.0 + d.1) / 2, if mid >= 2 { mid - 1 } else { mid });
            if part(d).contains(&tp) && cheb(tp, entry) > 1 && self.place_building(BuildingKind::Table, tp) {
                let (a, b) = if horiz { (tp.offset(-1, 0, 0), tp.offset(1, 0, 0)) } else { (tp.offset(0, -1, 0), tp.offset(0, 1, 0)) };
                self.place_building(BuildingKind::Chair, a);
                self.place_building(BuildingKind::Chair, b);
            }
        }
        o.interior.suites.push(Suite { a: r.a, bed, office: part(or).first().copied(), owner: None });
    }

    // ------------------------------------------------------------------
    // Gathering rooms
    // ------------------------------------------------------------------

    /// One room at a time, once the plan's own are under way: the rooms a
    /// growing fort gathers in, as far as it has come and to its taste.
    fn ov_wishes(&mut self, o: &mut Overseer) {
        let t = taste(o);
        if t.plain || o.interior.tier == 0 || !o.wants.is_empty() || o.rooms.iter().any(|r| r.stage == 0) {
            return;
        }
        let tier = o.interior.tier;
        let pop = self.alive_count();
        let have = |k: RoomUse| o.rooms.iter().filter(|r| r.kind == k && r.stage < 2).count();
        let seats = self.buildings.values().filter(|b| b.kind == BuildingKind::Chair && self.in_dining_hall(b.pos)).count();
        let mut list: Vec<(RoomUse, bool, &str)> = Vec::new();
        if tier >= 1 {
            list.push((RoomUse::Temple, t.pious && have(RoomUse::Temple) == 0, "a temple, before anything grander"));
            list.push((RoomUse::Meeting, have(RoomUse::Meeting) == 0, "a meeting hall"));
            list.push((RoomUse::Parlour, have(RoomUse::Parlour) == 0 && pop >= 10, "a small parlour to talk in"));
        }
        if tier >= 2 {
            // (The quarter's first halls, food, wood and stone, now; metal and cloth once it's mature.)
            let quarter = self.quarter_wish(o).filter(|c| tier >= 3 || matches!(c, Chain::Food | Chain::Wood | Chain::Stone));
            list.push((RoomUse::Tavern, have(RoomUse::Tavern) == 0, "a tavern"));
            list.push((RoomUse::Dining, o.interior.great.is_none() && (seats < pop || t.grand >= 2) && have(RoomUse::Dining) < 3, "a great dining hall"));
            list.push((RoomUse::Temple, have(RoomUse::Temple) == 0, "a temple"));
            list.extend(quarter.map(|c| (RoomUse::Industry, true, c.wish())));
            list.push((RoomUse::Barracks, have(RoomUse::Barracks) == 0 && o.style != Style::Reckless, "a barracks with a training floor"));
            list.push((RoomUse::Library, have(RoomUse::Library) == 0, "a library"));
            list.push((RoomUse::Hospital, self.wing_wanted(o), "a hospital wing"));
        }
        if tier >= 3 {
            list.push((RoomUse::Parlour, have(RoomUse::Parlour) < 1 + pop / 20, "another parlour"));
            list.push((RoomUse::Temple, t.pious && have(RoomUse::Temple) < 2, "a great temple"));
            list.push((RoomUse::Library, t.grand >= 2 && have(RoomUse::Library) < 2, "a second library"));
            list.push((RoomUse::Meeting, t.grand >= 2 && have(RoomUse::Meeting) < 2, "a second meeting hall"));
        }
        let Some(&(kind, _, why)) = list.iter().find(|(k, ok, _)| *ok && !o.blocked_lately(*k, self.tick)) else { return };
        o.wants.push(kind);
        if kind == RoomUse::Dining {
            o.interior.great_wanted = self.tick.max(1);
        }
        if kind == RoomUse::Hospital {
            o.done.push("wing".into());
        }
        // (Said once: asking again after a season without rock is no news.)
        let key = format!("wished {}", why);
        if !o.did(&key) {
            o.done.push(key);
            let what = format!("Ordered {} dug.", why);
            Self::note_ov(o, self.tick, flourish(o, &what));
        }
    }

    /// Daily: furniture for the fitted halls as stock allows: chairs both
    /// sides of the tables and statues in the dining halls, a stage and a
    /// bar in the tavern, an altar and statues in the temple, targets and
    /// cabinets in the barracks, more beds in the hospital.
    fn ov_dress(&mut self, o: &mut Overseer) {
        let t = taste(o);
        if t.plain || o.interior.tier == 0 {
            return;
        }
        let pop = self.alive_count();
        let rooms: Vec<Room> = o.rooms.iter().filter(|r| r.stage == 1).cloned().collect();
        for r in rooms {
            let tiles = room_tiles(&r);
            let statues = t.grand as usize + (o.interior.tier >= 3) as usize;
            match r.kind {
                RoomUse::Dining | RoomUse::Grand if tiles.iter().any(|p| self.in_dining_hall(*p)) => {
                    // A chair either side of every table.
                    let tables: Vec<Pos> = self.buildings.values().filter(|b| b.kind == BuildingKind::Table && tiles.contains(&b.pos)).map(|b| b.pos).collect();
                    let mut chairs: Vec<Pos> = Vec::new();
                    for tp in tables {
                        for q in [tp.offset(-1, 0, 0), tp.offset(1, 0, 0)] {
                            if tiles.contains(&q) {
                                chairs.push(q);
                            }
                        }
                    }
                    let entry = self.entry(&r);
                    chairs.retain(|&c| cheb(c, entry) > 1);
                    let n = self.buildings.values().filter(|b| b.kind == BuildingKind::Chair && tiles.contains(&b.pos)).count() + chairs.len();
                    self.furnish(&r, BuildingKind::Chair, &chairs, n);
                    let great = o.interior.great == Some(r.a);
                    let spots = self.wall_spots(&r);
                    self.furnish(&r, BuildingKind::Statue, &spots, statues + if great { 2 } else { 0 });
                }
                RoomUse::Tavern => self.dress_tavern(o, &r, statues),
                RoomUse::Temple => {
                    // An altar at the far wall, statues round it.
                    let entry = self.entry(&r);
                    let altar = *tiles.iter().max_by_key(|p| ((p.x - entry.x).abs() + (p.y - entry.y).abs(), p.x, p.y)).unwrap();
                    if self.world.t(altar).building == 0 && self.spare(BuildingKind::Table) > 0 {
                        self.place_building(BuildingKind::Table, altar);
                    }
                    let spots = self.wall_spots(&r);
                    self.furnish(&r, BuildingKind::Statue, &spots, statues.max(1) + if t.pious { 2 } else { 0 });
                }
                RoomUse::Meeting | RoomUse::Library | RoomUse::Parlour => {
                    let spots = self.wall_spots(&r);
                    self.furnish(&r, BuildingKind::Statue, &spots, statues.saturating_sub(1).min(if r.kind == RoomUse::Parlour { 1 } else { 3 }));
                }
                RoomUse::Suite => {
                    let spots = self.wall_spots(&r);
                    self.furnish(&r, BuildingKind::Statue, &spots, if t.grand >= 2 { 2 } else { 0 });
                }
                RoomUse::Barracks if o.interior.tier >= 2 => {
                    // Targets along the far wall, cabinets and chests for
                    // the soldiers' kit; the middle left clear to drill on.
                    let entry = self.entry(&r);
                    let mut far: Vec<Pos> = self.wall_spots(&r);
                    far.sort_by_key(|p| -((p.x - entry.x).abs() + (p.y - entry.y).abs()));
                    let targets: Vec<Pos> = far.iter().copied().step_by(3).collect();
                    self.furnish(&r, BuildingKind::ArcheryTarget, &targets, 1 + pop / 15);
                    let side: Vec<Pos> = far.iter().copied().skip(1).step_by(2).collect();
                    self.furnish(&r, BuildingKind::Cabinet, &side, 2);
                    self.furnish(&r, BuildingKind::Chest, &side, 2);
                }
                RoomUse::Hospital => {
                    let big = tiles.len() >= 50;
                    let want = if big { 3 + pop / 6 } else { 3 + pop / 12 };
                    let free: Vec<Pos> = tiles.iter().copied().filter(|p| self.world.t(*p).zone == 0 || self.zone_kind_at(*p) == Some(ZoneKind::Hospital)).filter(|p| (p.x - r.a.x) % 2 == 0 && (p.y - r.a.y) % 2 == 0).collect();
                    self.furnish(&r, BuildingKind::Bed, &free, want.min(10));
                    let spots = self.wall_spots(&r);
                    if big {
                        self.furnish(&r, BuildingKind::Table, &spots, 1);
                        self.furnish(&r, BuildingKind::Chest, &spots, 1);
                        self.furnish(&r, BuildingKind::Cabinet, &spots, 1);
                    }
                }
                _ => {}
            }
        }
    }

    /// The tavern: a stage for the bards in a far corner, a bar counter
    /// along the far wall with stools before it and the drink kept behind
    /// it, chairs at the tables.
    fn dress_tavern(&mut self, o: &mut Overseer, r: &Room, statues: usize) {
        let tiles = room_tiles(r);
        let entry = self.entry(r);
        let far = |p: &Pos| (p.x - entry.x).abs() + (p.y - entry.y).abs();
        // The stage: the free 2×2 farthest from the door.
        let have_stage = self.buildings.values().any(|b| b.kind == BuildingKind::Stage && tiles.contains(&b.pos));
        if !have_stage {
            let spot = tiles
                .iter()
                .copied()
                .filter(|&p| (0..2).all(|dy| (0..2).all(|dx| tiles.contains(&p.offset(dx, dy, 0)) && cheb(p.offset(dx, dy, 0), entry) > 1)) && self.can_place(BuildingKind::Stage, p))
                .max_by_key(|p| (far(p), p.x, p.y));
            if let Some(p) = spot {
                if self.place_building(BuildingKind::Stage, p) {
                    o.interior.stage = (0..2).flat_map(|dy| (0..2).map(move |dx| p.offset(dx, dy, 0))).collect();
                }
            }
        }
        let stage = o.interior.stage.clone();
        // The bar: the row of tiles along the wall farthest from the door.
        let side = {
            let d = [(r.a.y - entry.y).abs(), (r.b.y - entry.y).abs(), (r.a.x - entry.x).abs(), (r.b.x - entry.x).abs()];
            (0..4).max_by_key(|&i| d[i]).unwrap()
        };
        let on_side = |p: &Pos| match side {
            0 => p.y == r.a.y,
            1 => p.y == r.b.y,
            2 => p.x == r.a.x,
            _ => p.x == r.b.x,
        };
        let inward = match side {
            0 => (0, 1),
            1 => (0, -1),
            2 => (1, 0),
            _ => (-1, 0),
        };
        let mut row: Vec<Pos> = tiles.iter().copied().filter(|p| on_side(p) && !stage.iter().any(|s| cheb(*s, *p) <= 1)).collect();
        row.sort_by_key(|p| (p.x, p.y));
        if row.len() >= 4 {
            // The drink behind the bar: its last two tiles a stockpile.
            let (bar, store) = row.split_at(row.len() - 2);
            if !store.iter().any(|p| matches!(self.zone_kind_at(*p), Some(ZoneKind::Stockpile(_)))) && store.iter().all(|p| self.world.t(*p).building == 0) {
                for &p in store {
                    self.remove_zones_where(p, p, |k| k == ZoneKind::Tavern);
                }
                self.add_zone_tiles(store.to_vec(), ZoneKind::Stockpile(StockFilter::Food));
            }
            let counter: Vec<Pos> = bar.iter().copied().take(4).collect();
            self.furnish(r, BuildingKind::Table, &counter, counter.len() + self.buildings.values().filter(|b| b.kind == BuildingKind::Table && tiles.contains(&b.pos) && !counter.contains(&b.pos)).count());
            let stools: Vec<Pos> = counter.iter().map(|p| p.offset(inward.0, inward.1, 0)).filter(|p| tiles.contains(p)).collect();
            let n = self.buildings.values().filter(|b| b.kind == BuildingKind::Chair && tiles.contains(&b.pos)).count() + stools.len();
            self.furnish(r, BuildingKind::Chair, &stools, n);
        }
        // Chairs at the other tables.
        let tables: Vec<Pos> = self.buildings.values().filter(|b| b.kind == BuildingKind::Table && tiles.contains(&b.pos) && !on_side(&b.pos)).map(|b| b.pos).collect();
        let chairs: Vec<Pos> = tables.iter().flat_map(|t| [t.offset(1, 0, 0), t.offset(-1, 0, 0)]).filter(|p| tiles.contains(p) && !stage.contains(p) && cheb(*p, entry) > 1).collect();
        let n = self.buildings.values().filter(|b| b.kind == BuildingKind::Chair && tiles.contains(&b.pos)).count() + chairs.len();
        self.furnish(r, BuildingKind::Chair, &chairs, n);
        let spots: Vec<Pos> = self.wall_spots(r).into_iter().filter(|p| !stage.contains(p) && !on_side(p)).collect();
        self.furnish(r, BuildingKind::Statue, &spots, statues.saturating_sub(1));
    }

    /// Smoothing and engraving as the fort comes up in the world: the
    /// halls first, then (for a grand taste) the bedrooms and suites.
    fn ov_finish(&mut self, o: &mut Overseer) {
        let t = taste(o);
        let tier = o.interior.tier;
        if t.plain || tier == 0 {
            return;
        }
        let detailing = self.designations.iter().filter(|p| matches!(self.world.t(**p).desig, Desig::Smooth | Desig::Engrave)).count();
        // (The rooms once established; the halls from settled, `halls.rs`.)
        if detailing < 40 && tier >= 2 {
            let smooth_kinds = |k: RoomUse| gathering(k) || matches!(k, RoomUse::Suite | RoomUse::Parlour | RoomUse::Grand) || (k == RoomUse::Bedrooms && t.grand >= 2 && tier >= 3);
            let carve = tier >= 3 && t.grand >= 1 || t.grand >= 3;
            let rooms: Vec<Room> = o.rooms.iter().filter(|r| r.stage == 1 && smooth_kinds(r.kind)).cloned().collect();
            for r in rooms {
                let key = r.a.x * 1000 + r.a.y + r.a.z * 1_000_000;
                let (s, c) = (format!("engrave{}", key), format!("carve{}", key));
                if !o.did(&s) {
                    self.designate(r.a.offset(-1, -1, 0), r.b.offset(1, 1, 0), Desig::Smooth);
                    o.done.push(s);
                    break;
                }
                if carve && !o.did(&c) && self.tick > o.interior.tier_since + DAY * 10 {
                    self.designate(r.a.offset(-1, -1, 0), r.b.offset(1, 1, 0), Desig::Engrave);
                    o.done.push(c);
                    break;
                }
            }
        }
        // Someone to do it (a builder's overseer sees to its own).
        if o.style != Style::Builder && self.designations.iter().any(|p| matches!(self.world.t(*p).desig, Desig::Smooth | Desig::Engrave)) {
            let want = 1 + self.alive_count() / 20;
            let have = self.dwarves.iter().filter(|d| d.alive && !d.soldier && d.has(Labor::Engraving)).count();
            if have < want {
                let pick: Vec<u32> = {
                    let mut c: Vec<(u32, usize, u32)> = self
                        .dwarves
                        .iter()
                        .filter(|d| d.alive && !d.is_child() && !d.soldier && !d.insane && !d.has(Labor::Engraving))
                        .map(|d| (d.skills[Labor::Engraving.idx()], LABORS.iter().filter(|l| d.has(**l)).count(), d.id))
                        .collect();
                    c.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
                    c.into_iter().take(want - have).map(|x| x.2).collect()
                };
                for id in pick {
                    self.set_labor(id, Labor::Engraving, true);
                }
            }
        }
    }

    /// Work orders for the furniture it means to put in.
    pub(super) fn interior_orders(&self, o: &Overseer, orders: &mut Vec<(Recipe, u32)>) {
        let t = taste(o);
        let tier = o.interior.tier as u32;
        if t.plain || tier == 0 {
            return;
        }
        let mut want: Vec<(Recipe, u32)> = Vec::new();
        if self.stock_of(ItemKind::Log) >= 3 {
            want.push((Recipe::WoodCabinet, 1 + tier));
            want.push((Recipe::WoodChest, 1 + tier));
            if tier >= 2 {
                want.push((Recipe::WoodTable, 4));
                want.push((Recipe::WoodChair, 6));
                want.push((Recipe::WoodBed, 3));
            }
        }
        if !self.gate_wants_stone(o) && (t.grand > 0 || tier >= 2) {
            want.push((Recipe::Statue, t.grand as u32 + tier.saturating_sub(1)));
        }
        for (r, keep) in want {
            match orders.iter_mut().find(|x| x.0 == r) {
                Some(x) => x.1 = x.1.max(keep),
                None => orders.push((r, keep)),
            }
        }
    }

    // ------------------------------------------------------------------
    // Names
    // ------------------------------------------------------------------

    /// What a fitted room is called (None: not worth a name).
    fn room_title(&self, o: &Overseer, r: &Room) -> Option<(String, bool)> {
        let t = taste(o);
        let h = ((r.a.x * 31 + r.a.y * 17 + r.a.z * 7) as usize) % 100;
        Some(match r.kind {
            RoomUse::Dining if o.interior.great == Some(r.a) => ("the Great Dining Hall".into(), true),
            RoomUse::Dining => ("the dining hall".into(), false),
            RoomUse::Tavern => (format!("the {} {}", TAVERN_ADJ[h % 10], TAVERN_NOUN[h / 10 % 10]), true),
            RoomUse::Temple if room_tiles(r).len() >= 60 => (if t.pious { "the Great Temple".into() } else { "the temple hall".into() }, true),
            RoomUse::Temple => ("the temple".into(), true),
            RoomUse::Library => ("the library".into(), true),
            RoomUse::Parlour => ("the parlour".into(), false),
            RoomUse::Meeting => ("the meeting hall".into(), false),
            RoomUse::Barracks => ("the barracks".into(), false),
            RoomUse::Hospital if room_tiles(r).len() >= 50 && o.did("wing") => ("the hospital wing".into(), true),
            RoomUse::Hospital => ("the hospital".into(), false),
            RoomUse::Industry => {
                let c = o.interior.quarter.iter().find(|q| q.0 == r.a).map(|q| q.1)?;
                (c.hall().to_string(), true)
            }
            RoomUse::Suite => {
                let s = o.interior.suites.iter().find(|s| s.a == r.a)?;
                let who = s.owner.and_then(|id| self.dwarves.iter().find(|d| d.id == id));
                match who {
                    Some(d) => (format!("{}'s suite", first_name(&d.name)), true),
                    None => ("an empty suite".into(), false),
                }
            }
            RoomUse::Dormitory => ("the dormitory".into(), false),
            RoomUse::Stockpile => ("the storeroom".into(), false),
            RoomUse::Workshops => ("the workshop hall".into(), false),
            RoomUse::Tomb => ("the tomb".into(), false),
            RoomUse::Office => ("the office".into(), false),
            RoomUse::Jail => ("the jail".into(), false),
            RoomUse::Grand => (format!("the grand {}", r.shape.map_or("hall", |s| s.describe())), true),
            RoomUse::Bedrooms => return None,
        })
    }

    /// Daily: the names of the fitted rooms (a new one is noted, and the
    /// camera may visit it). Also marks the great dining hall once it's done.
    fn ov_places(&mut self, o: &mut Overseer) {
        if o.interior.great.is_none() && o.interior.great_wanted > 0 {
            if let Some(r) = o.rooms.iter().filter(|r| r.kind == RoomUse::Dining && r.stage == 1 && r.since >= o.interior.great_wanted).max_by_key(|r| room_tiles(r).len()) {
                o.interior.great = Some(r.a);
            }
        }
        let mut places: Vec<Place> = Vec::new();
        let fitted: Vec<Room> = o.rooms.iter().filter(|r| r.stage == 1).cloned().collect();
        for r in &fitted {
            let Some((name, notable)) = self.room_title(o, r) else { continue };
            let since = o.interior.places.iter().find(|p| p.a == r.a && p.name == name).map(|p| p.since);
            let fresh = since.is_none() && notable && !o.interior.places.iter().any(|p| p.a == r.a && p.notable);
            places.push(Place { a: r.a, b: r.b, name: name.clone(), since: since.unwrap_or(self.tick), notable });
            if fresh && o.interior.tier >= 1 {
                let what = match r.kind {
                    RoomUse::Dining => format!("Finished {}, with {} seats.", name, self.buildings.values().filter(|b| b.kind == BuildingKind::Chair && box_iter(r.a, r.b).any(|p| p == b.pos)).count()),
                    RoomUse::Tavern if !o.interior.stage.is_empty() => format!("Opened a tavern, {}, with a bar and a stage for the bards.", name),
                    RoomUse::Tavern => format!("Opened a tavern, {}.", name),
                    _ => format!("Finished {}.", name),
                };
                let what = flourish(o, &what);
                Self::note_ov(o, self.tick, what);
            }
        }
        o.interior.places = places;
    }

    /// What the look panel calls the room at `p`: "Urist's bedroom", "the
    /// Great Dining Hall"...
    pub fn place_name(&self, p: Pos) -> Option<String> {
        if let Some(z) = self.zone_at(p) {
            let owner = z.owner.and_then(|id| self.dwarves.iter().find(|d| d.id == id));
            match (z.kind, owner) {
                (ZoneKind::Bedroom, Some(d)) => {
                    let mate = d.spouse.and_then(|s| self.dwarves.iter().find(|x| x.id == s && x.alive)).filter(|m| self.buildings.values().any(|b| b.kind == BuildingKind::Bed && b.owner == Some(m.id) && self.world.t(b.pos).zone == z.id));
                    return Some(match (mate, d.noble) {
                        (_, Some(n)) if n != crate::dwarf::Noble::Leader => format!("the {}'s bedchamber", n.title().to_lowercase()),
                        (Some(m), _) => format!("{} and {}'s bedroom", first_name(&d.name), first_name(&m.name)),
                        _ => format!("{}'s bedroom", first_name(&d.name)),
                    });
                }
                (ZoneKind::Office, Some(d)) => return Some(format!("{}'s office", first_name(&d.name))),
                _ => {}
            }
        }
        let o = self.overseer.as_ref()?;
        o.interior.places.iter().find(|pl| p.z == pl.a.z && p.x >= pl.a.x && p.x <= pl.b.x && p.y >= pl.a.y && p.y <= pl.b.y).map(|pl| pl.name.clone())
    }

    /// Halls named lately, for the camera: where, and what.
    pub fn new_halls(&self, within: u64) -> Vec<(Pos, String)> {
        let Some(o) = self.overseer.as_ref() else { return Vec::new() };
        o.interior
            .places
            .iter()
            .filter(|p| p.notable && self.tick <= p.since + within)
            .map(|p| (Pos::new((p.a.x + p.b.x) / 2, (p.a.y + p.b.y) / 2, p.a.z), p.name.clone()))
            .collect()
    }

    /// Where the bards perform, if the tavern has a stage.
    pub(super) fn stage_tiles(&self) -> Vec<Pos> {
        self.overseer.as_ref().map_or(Vec::new(), |o| o.interior.stage.iter().copied().filter(|&p| self.world.walkable(p)).collect())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::game::overseer::{trial_game, Overseer};

    /// The fort's main level (or `z`), drawn: rock #, floor ., doors +,
    /// beds Θ, tables ╥, chairs h, cabinets π, chests ■, statues ♀, the
    /// stage ▒, workshops by letter, zones as lower-case letters where
    /// nothing stands (b bedroom, d dining, t tavern, m meeting, o office,
    /// w hospital ward, s stockpile, r barracks, p temple, z dormitory).
    pub fn ascii_plan(g: &Game, z: i32, x0: i32, x1: i32, y0: i32, y1: i32) -> String {
        let mut s = String::new();
        for y in y0.max(0)..=y1.min(g.world.h - 1) {
            for x in x0.max(0)..=x1.min(g.world.w - 1) {
                let p = Pos::new(x, y, z);
                let t = g.world.t(p);
                let c = if t.building != 0 {
                    match g.buildings.get(&t.building) {
                        Some(b) => b.glyph_at(x - b.pos.x, y - b.pos.y),
                        None => '?',
                    }
                } else if t.desig != Desig::None {
                    '+'
                } else if t.kind.solid() {
                    if t.built_from.is_some() { 'W' } else if t.smooth > 0 { '▓' } else { '#' }
                } else if let Some(k) = g.zone_kind_at(p) {
                    match k {
                        ZoneKind::Bedroom => 'b',
                        ZoneKind::Dining => 'd',
                        ZoneKind::Tavern => 't',
                        ZoneKind::Meeting => 'm',
                        ZoneKind::Office => 'o',
                        ZoneKind::Hospital => 'w',
                        ZoneKind::Stockpile(_) => 's',
                        ZoneKind::Barracks => 'r',
                        ZoneKind::Temple => 'p',
                        ZoneKind::Dormitory => 'z',
                        ZoneKind::Tomb => 'c',
                        _ => ',',
                    }
                } else if t.kind.has_floor() {
                    '.'
                } else {
                    ' '
                };
                s.push(c);
            }
            s.push(char::from(10u8));
        }
        s
    }

    /// The fort's rooms, one line each.
    pub fn room_list(g: &Game, o: &Overseer) -> String {
        let mut s = String::new();
        for r in o.rooms.iter().filter(|r| r.stage == 1) {
            let name = g.room_title(o, r).map_or(super::super::overseer::room_name(r.kind).to_string(), |x| x.0);
            s.push_str(&format!("  {:<28} {:?} at ({},{},{}) {} tiles{}", name, r.kind, r.a.x, r.a.y, r.a.z, room_tiles(r).len(), char::from(10u8)));
        }
        s
    }

    /// A fort's insides after DAYS days: how far it came, its rooms, its
    /// notes on them, and its main level drawn.
    /// `SEED=5003 STYLE=builder DAYS=672 cargo test --release show_interior -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn show_interior() {
        let env = |k: &str, d: &str| std::env::var(k).unwrap_or(d.into());
        let seed: u64 = env("SEED", "5003").parse().unwrap();
        let style = Style::parse(&env("STYLE", "builder")).unwrap();
        let days: u64 = env("DAYS", "672").parse().unwrap();
        let mut g = Game::new(seed);
        g.set_overseer(Some(style));
        if let Ok(t) = std::env::var("TRAITS") {
            let traits: Vec<Trait> = t.split(',').filter_map(|n| Trait::ALL.iter().copied().find(|x| x.name() == n.trim())).collect();
            g.overseer.as_mut().unwrap().traits = traits;
        }
        let mut told: std::collections::HashSet<(u64, String)> = Default::default();
        let mut story: Vec<(u64, String)> = Vec::new();
        let words = ["bedroom", "ormitor", "suite", "Ordered", "Finished", "Opened", "Took down", "Cleared", "Moved", "fortress has", "fortress is", "Retired", "Gave", "Dug out", "Fitted"];
        while g.tick < days * DAY && g.alive_count() > 0 {
            g.step();
            if g.tick % DAY == 0 {
                if let Some(o) = g.overseer.as_ref() {
                    for n in &o.notes {
                        if told.insert(n.clone()) && words.iter().any(|w| n.1.contains(w)) {
                            story.push(n.clone());
                        }
                    }
                }
            }
            if g.tick % (DAY * 28) == 0 {
                let o = g.overseer.as_ref().unwrap();
                eprintln!("day {:>4} pop {:>2} wealth {:>6} tier {} rooms {} fitted {}", g.tick / DAY, g.alive_count(), g.wealth(), o.interior.tier, o.rooms.len(), o.rooms.iter().filter(|r| r.stage == 1).count());
            }
        }
        let o = g.overseer.as_ref().unwrap();
        eprintln!("== {} {} ({}), layout {:?}, tier {}", style.name(), seed, o.title(), o.layout, o.interior.tier);
        let owned = g.zones.values().filter(|z| z.kind == ZoneKind::Bedroom && z.owner.is_some()).count();
        let bedrooms = g.zones.values().filter(|z| z.kind == ZoneKind::Bedroom).count();
        let count = |k: BuildingKind| g.buildings.values().filter(|b| b.kind == k && b.built).count();
        eprintln!(
            "alive {} | bedrooms {} (owned {}) dorm retired {} | suites {:?} | quarter {:?} | statues {} cabinets {} chests {} stage {} | smoothed {}",
            g.alive_count(),
            bedrooms,
            owned,
            o.did("dorm-retired"),
            o.interior.suites.iter().map(|s| s.owner).collect::<Vec<_>>(),
            o.interior.quarter.iter().map(|q| q.1).collect::<Vec<_>>(),
            count(BuildingKind::Statue),
            count(BuildingKind::Cabinet),
            count(BuildingKind::Chest),
            count(BuildingKind::Stage),
            g.world.tiles.iter().filter(|t| t.smooth > 0).count()
        );
        eprint!("{}", room_list(&g, o));
        for n in story.iter() {
            eprintln!("  d{} {}", n.0 / DAY, n.1);
        }
        let b = o.base.unwrap();
        for k in 0..=o.level {
            let z = Game::level_z(o, k);
            eprintln!("== level {} (z {})", k, z);
            eprint!("{}", ascii_plan(&g, z, b.x - 3, g.world.w - 1, 0, g.world.h - 1));
        }
    }

    use crate::game::maturity::tests::empty_room;
    use crate::material::Mat;

    /// A copy of a game (through a save).
    fn copy(g: &Game) -> Game {
        let mut c: Game = serde_json::from_str(&serde_json::to_string(g).unwrap()).unwrap();
        c.after_load();
        c
    }

    fn stock(g: &mut Game, kind: ItemKind, n: usize) {
        let e = g.embark;
        let at = (0..10).map(|r| e.offset(r, 1, 0)).find(|p| g.world.walkable(*p)).unwrap_or(e);
        for _ in 0..n {
            g.spawn_item(kind, Mat::Granite, at);
        }
    }

    fn peace(g: &mut Game, days: u64) {
        for _ in 0..days * DAY {
            g.step();
            g.creatures.retain(|_, c| c.tame || !c.hostile());
            g.incoming = None;
        }
    }

    /// After the first dormitory every dwarf gets a bedroom of their own
    /// (a bed, a door, a cabinet and a chest), and once they all have one
    /// the dormitory is retired and put to another use. The first fort
    /// from seed 5003 on with a dormitory by day 30.
    #[test]
    fn rooms_of_their_own_and_the_dormitory_retires() {
        let dorm = |g: &Game| g.overseer.as_ref().map_or(false, |o| o.rooms.iter().any(|r| r.kind == RoomUse::Dormitory && r.stage == 1) && o.style == Style::Steady);
        let (found, passed) = crate::game::overseer::fort_where(Style::Steady, 5003, 6, 30, dorm);
        let Some((seed, mut g)) = found else { panic!("no fort with a dormitory by day 30 (passed over {:?})", passed) };
        let mut day = 30;
        while day < 140 && !g.overseer.as_ref().unwrap().did("dorm-retired") {
            peace(&mut g, 5);
            day += 5;
        }
        let o = g.overseer.as_ref().unwrap();
        let rooms: Vec<(RoomUse, u8)> = o.rooms.iter().map(|r| (r.kind, r.stage)).collect();
        assert!(o.did("dorm-retired"), "{}: the dormitory still in use at day {} (tier {}, rooms {:?})", seed, day, o.interior.tier, rooms);
        let owned = g.zones.values().filter(|z| z.kind == ZoneKind::Bedroom && z.owner.is_some()).count();
        assert!(owned >= 6, "{}: only {} bedrooms owned", seed, owned);
        // Everyone sleeps in a bed of their own in a bedroom (or their spouse's).
        for d in g.dwarves.iter().filter(|d| d.alive && !d.insane) {
            let bed = g.buildings.values().find(|b| b.kind == BuildingKind::Bed && b.owner == Some(d.id));
            assert!(bed.map_or(false, |b| g.zone_kind_at(b.pos) == Some(ZoneKind::Bedroom)), "{}: {} has no bedroom", seed, d.name);
        }
        assert!(!g.zones.values().any(|z| z.kind == ZoneKind::Dormitory), "{}: a dormitory zone left", seed);
        assert!(!o.rooms.iter().any(|r| r.kind == RoomUse::Dormitory && r.stage == 1), "{}: the dormitory wasn't put to new use", seed);
        assert!(o.notes.iter().any(|n| n.1.starts_with("Retired the old dormitory: every dwarf has a room of their own now.")));
        // Each bedroom has a door; some already a cabinet and a chest.
        let cells: Vec<Cell> = o.rooms.iter().filter(|r| r.kind == RoomUse::Bedrooms && r.stage == 1).flat_map(cells_of).collect();
        assert!(cells.iter().all(|c| c.door.map_or(false, |d| g.bld_kind_at(d) == Some(BuildingKind::Door))), "{}: a bedroom without a door", seed);
        let furnished = cells.iter().filter(|c| c.tiles.iter().any(|p| g.bld_kind_at(*p) == Some(BuildingKind::Cabinet)) && c.tiles.iter().any(|p| g.bld_kind_at(*p) == Some(BuildingKind::Chest))).count();
        assert!(furnished >= 1, "{}: no bedroom has a cabinet and a chest", seed);
        // The look panel names them.
        let c = &cells[0];
        let owner = g.zone_at(c.bed).and_then(|z| z.owner).and_then(|id| g.dwarves.iter().find(|d| d.id == id)).map(|d| first_name(&d.name));
        if let Some(n) = owner {
            assert!(g.place_name(c.bed).map_or(false, |s| s.contains(&n) && s.contains("bedroom")), "{:?}", g.place_name(c.bed));
        }
    }

    /// Husband and wife share a room: its second bed is the spouse's.
    #[test]
    fn a_couple_shares_a_bedroom() {
        let mut g = Game::new(21);
        let e = g.embark;
        let spot = (2..30)
            .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| (dx, dy))))
            .map(|(dx, dy)| e.offset(dx, dy, 0))
            .find(|a| box_iter(*a, a.offset(2, 2, 0)).all(|p| g.world.get(p).map_or(false, |t| t.kind == crate::world::TileKind::Floor && t.water == 0 && t.building == 0 && t.zone == 0)))
            .expect("open ground");
        for p in [spot, spot.offset(2, 0, 0)] {
            assert!(g.place_building(BuildingKind::Bed, p));
            let id = *g.buildings.keys().max().unwrap();
            g.buildings.get_mut(&id).unwrap().built = true;
        }
        g.add_zone(spot, spot.offset(2, 2, 0), ZoneKind::Bedroom);
        let (a, b) = (g.dwarves[0].id, g.dwarves[1].id);
        g.dwarves[0].spouse = Some(b);
        g.dwarves[1].spouse = Some(a);
        let z = g.world.t(spot).zone;
        g.set_zone_owner(z, Some(a));
        let owners: Vec<Option<u32>> = [spot, spot.offset(2, 0, 0)].iter().map(|p| g.buildings.values().find(|x| x.pos == *p).unwrap().owner).collect();
        assert!(owners.contains(&Some(a)) && owners.contains(&Some(b)), "both sleep there: {:?}", owners);
        let name = g.place_name(spot).unwrap();
        assert!(name.contains(" and "), "{}", name);
    }

    /// A noble gets a suite: a bedchamber, an office and a dining room,
    /// walled apart with doors between, the bedchamber and office theirs.
    #[test]
    fn a_noble_gets_a_suite() {
        let mut g = trial_game(5003, Style::Steady, 45);
        let mut o = g.overseer.take().unwrap();
        o.interior.tier = 2;
        let mayor = g.dwarves.iter().find(|d| d.alive && !d.is_child()).map(|d| d.id).unwrap();
        for d in g.dwarves.iter_mut() {
            if d.id == mayor {
                d.noble = Some(crate::dwarf::Noble::Mayor);
            } else if d.noble == Some(crate::dwarf::Noble::Leader) {
                d.noble = None;
            }
        }
        // It asks for one...
        g.ov_suites(&mut o);
        assert!(o.wants.contains(&RoomUse::Suite), "no suite wanted for the mayor");
        // ...and fits one out (here in a storeroom made over).
        let i = empty_room(&mut g, &o, 40);
        o.rooms[i].kind = RoomUse::Suite;
        let r = o.rooms[i].clone();
        assert_eq!(g.fit_interior(&mut o, &r), None);
        g.ov_suites(&mut o);
        let s = o.interior.suites.last().cloned().expect("a suite");
        assert_eq!(s.owner, Some(mayor));
        assert_eq!(g.zone_at(s.bed).map(|z| (z.kind, z.owner)), Some((ZoneKind::Bedroom, Some(mayor))), "the bedchamber is the mayor's");
        let office = s.office.expect("an office");
        assert_eq!(g.zone_at(office).map(|z| (z.kind, z.owner)), Some((ZoneKind::Office, Some(mayor))), "the office is the mayor's");
        let tiles = room_tiles(&r);
        // (A long room has a dining room between the two; a shorter one just the two.)
        let long = (r.b.x - r.a.x + 1).max(r.b.y - r.a.y + 1) >= 9;
        assert!(!long || tiles.iter().any(|p| g.zone_kind_at(*p) == Some(ZoneKind::Dining)), "a dining room");
        assert_eq!(g.bld_kind_at(s.bed), Some(BuildingKind::Bed));
        let walls = g.constructions.values().filter(|c| tiles.contains(&c.pos)).count();
        let doors = tiles.iter().filter(|p| g.bld_kind_at(**p) == Some(BuildingKind::Door)).count();
        assert!(walls >= 3 && doors >= 1, "walls {} and doors {} between its rooms", walls, doors);
        assert!(o.notes.iter().any(|n| n.1.contains("a suite of rooms")));
        g.overseer = Some(o);
        assert!(g.place_name(s.bed).map_or(false, |n| n.contains("mayor")), "{:?}", g.place_name(s.bed));
    }

    /// The great dining hall gets rows of tables with a chair either side,
    /// and statues along its walls; it takes its name.
    #[test]
    fn a_great_dining_hall_gets_tables_chairs_and_statues() {
        let mut g = trial_game(5003, Style::Steady, 45);
        let mut o = g.overseer.take().unwrap();
        o.interior.tier = 2;
        o.traits = vec![Trait::Vain, Trait::Cheerful];
        let i = empty_room(&mut g, &o, 40);
        o.rooms[i].kind = RoomUse::Dining;
        o.interior.great = Some(o.rooms[i].a);
        let r = o.rooms[i].clone();
        if let Some(z) = g.fit_interior(&mut o, &r) {
            g.add_zone(r.a, r.b, z);
        }
        stock(&mut g, ItemKind::Statue, 6);
        stock(&mut g, ItemKind::Chair, 24);
        stock(&mut g, ItemKind::Table, 12);
        g.ov_dress(&mut o);
        let tiles = room_tiles(&r);
        let n = |g: &Game, k: BuildingKind| g.buildings.values().filter(|b| b.kind == k && tiles.contains(&b.pos)).count();
        let (tables, chairs, statues) = (n(&g, BuildingKind::Table), n(&g, BuildingKind::Chair), n(&g, BuildingKind::Statue));
        assert!(tables >= 4, "{} tables", tables);
        assert!(chairs >= tables * 3 / 2, "{} chairs for {} tables", chairs, tables);
        assert!(statues >= 3, "{} statues", statues);
        g.ov_places(&mut o);
        g.overseer = Some(o);
        let mid = tiles[tiles.len() / 2];
        assert_eq!(g.place_name(mid).as_deref(), Some("the Great Dining Hall"));
        // Hauled in and set up.
        peace(&mut g, 12);
        let built = g.buildings.values().filter(|b| b.kind == BuildingKind::Statue && b.built && tiles.contains(&b.pos)).count();
        assert!(built >= 1, "no statue set up in the hall");
    }

    /// Two overseers, vain and cautious, take over the same fort: the vain
    /// one asks for bigger halls, leaves pillars standing and fills them
    /// with statues; the cautious one keeps them small and plain.
    #[test]
    fn vain_and_cautious_overseers_build_differently() {
        let base = trial_game(5003, Style::Steady, 45);
        let mut out = Vec::new();
        for traits in [vec![Trait::Vain, Trait::Pious], vec![Trait::Cautious, Trait::Greedy]] {
            let mut g = copy(&base);
            let mut o = g.overseer.take().unwrap();
            o.traits = traits.clone();
            o.interior.tier = 2;
            let mut tiles = 0;
            let mut pillared = 0;
            for k in [RoomUse::Dining, RoomUse::Temple, RoomUse::Library, RoomUse::Meeting] {
                if let Some(r) = g.plan_room(&mut o, k) {
                    tiles += room_tiles(&r).len();
                    pillared += matches!(r.shape, Some(Shape::Pillared { .. })) as usize;
                    o.rooms.push(r);
                }
            }
            // The same dining hall, dressed by each.
            let d = o.rooms.iter().position(|r| r.kind == RoomUse::Dining && r.stage == 1).expect("a fitted dining hall");
            let hall = o.rooms[d].clone();
            stock(&mut g, ItemKind::Statue, 8);
            g.ov_dress(&mut o);
            let statues = g.buildings.values().filter(|b| b.kind == BuildingKind::Statue && room_tiles(&hall).contains(&b.pos)).count();
            eprintln!("{:?}: {} tiles of new halls, {} pillared, {} statues", traits, tiles, pillared, statues);
            out.push((tiles, pillared, statues));
        }
        let (vain, cautious) = (out[0], out[1]);
        assert!(vain.0 > cautious.0, "the vain one's halls are bigger: {:?} vs {:?}", vain, cautious);
        assert!(vain.1 > cautious.1, "the vain one leaves pillars standing: {:?} vs {:?}", vain, cautious);
        assert!(vain.2 > cautious.2, "the vain one has more statues: {:?} vs {:?}", vain, cautious);
    }

    #[test]
    fn a_cell_has_room_for_furniture_and_a_second_bed() {
        // A row of two cells off a hall running east, rooms to the south.
        let mut tiles = Vec::new();
        for c in 0..2 {
            for s in 4 * c..4 * c + 3 {
                for q in 1..4 {
                    tiles.push(Pos::new(10 + s, 10 + q, 5));
                }
            }
        }
        let r = Room {
            kind: RoomUse::Bedrooms,
            a: Pos::new(10, 11, 5),
            b: Pos::new(16, 13, 5),
            stage: 1,
            tiles,
            shape: None,
            since: 0,
            doors: vec![Pos::new(11, 10, 5), Pos::new(15, 10, 5)],
            beds: vec![Pos::new(11, 13, 5), Pos::new(15, 13, 5)],
        };
        let cells = cells_of(&r);
        assert_eq!(cells.len(), 2);
        for c in &cells {
            assert_eq!(c.tiles.len(), 9, "a cell is 3x3");
            let f = c.furniture();
            assert!(f.len() >= 2 && f.iter().all(|p| c.tiles.contains(p) && *p != c.bed), "furniture beside the bed: {:?}", f);
            let s = c.second_bed().expect("room for a second bed");
            assert!(c.tiles.contains(&s) && s != c.bed && !f[..2].contains(&s));
            assert!(cheb(s, c.door.unwrap()) > 1, "not in the doorway");
        }
    }

    #[test]
    fn taste_orders_the_halls_it_asks_for() {
        let mut vain = Overseer::new(Style::Steady);
        vain.traits = vec![Trait::Vain, Trait::Greedy];
        let mut cautious = Overseer::new(Style::Steady);
        cautious.traits = vec![Trait::Cautious, Trait::Greedy];
        assert_eq!(classes(&vain, RoomUse::Dining)[0], Class::Large);
        assert_eq!(classes(&cautious, RoomUse::Dining)[0], Class::Medium);
        assert!(taste(&vain).grand > taste(&cautious).grand);
        let tiles: Vec<Pos> = box_iter(Pos::new(0, 0, 0), Pos::new(10, 6, 0)).collect();
        let (s, t) = restyle(&vain, RoomUse::Dining, Shape::Rect { w: 11, h: 7 }, Pos::new(0, 0, 0), &tiles);
        assert!(matches!(s, Shape::Pillared { .. }) && t.len() < tiles.len(), "a vain overseer leaves pillars standing");
        let (s, _) = restyle(&cautious, RoomUse::Dining, Shape::Rect { w: 11, h: 7 }, Pos::new(0, 0, 0), &tiles);
        assert!(matches!(s, Shape::Rect { .. }));
    }
}
