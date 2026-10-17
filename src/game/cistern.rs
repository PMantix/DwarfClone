//! Water a fort can rely on (ENG-18): the overseer sinks a well inside the
//! shelter over water it finds below or, failing that, cuts a cistern by
//! water outside and fills it through a floodgate, so dwarves sheltering
//! from a siege can still drink when the barrels run dry.
//!
//! Water here only ever falls and spreads sideways, never rises, so a shaft
//! down to water can't flood the hall it opens from. What can flood is
//! digging *into* water sideways: shafts and chambers are only planned
//! through sound rock with nothing dug or wet beside them, the cistern's
//! breach is dug last behind a shut gate, and the gate opens only while
//! nothing but the chamber can take the water.

use super::overseer::{Overseer, RoomUse, Style};
use super::water::WELL_DEPTH;
use super::*;
use serde::{Deserialize, Serialize};

/// A well being sunk: the shaft under `top` reaches water at `water_z`, dug
/// as channels level by level from a stair beside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WellPlan {
    /// Where the well goes, in a corner of a hall.
    pub top: Pos,
    /// The stair beside the shaft the diggers work from (none when the
    /// water lies right under the floor).
    pub stair: Option<Pos>,
    /// The level the water is at (or seeps in at, for an aquifer).
    pub water_z: i32,
    /// When it was begun.
    pub since: u64,
}

/// A cistern by the water: a chamber cut two levels under the halls beside
/// water outside (a lake, a river) whose surface is one level under them.
/// The water comes in through a floodgate at the water's edge and falls down
/// a drop into the chamber; the well stands over the drop, at the end of a
/// tunnel from the halls.
///
/// It sits by the water rather than under a hall because water levels out a
/// step lower with every tile it spreads: carried far through a tunnel it
/// arrives as a trickle and dries away. Falling, it arrives whole.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CisternPlan {
    /// The well, on the halls' level, over the drop.
    pub well: Pos,
    /// The stair at the end of the tunnel, down past the drop into the chamber.
    pub stair: Pos,
    /// The stair's middle, beside the drop, where the diggers work from.
    pub shaft: Pos,
    /// The hole behind the gate the water falls down into the chamber.
    pub drop: Pos,
    /// The chamber's corners, two levels below the halls.
    pub chamber: (Pos, Pos),
    /// The floodgate, between the breach and the drop.
    pub gate: Pos,
    /// The rock between the gate and the water: broken through last, by a
    /// channel from the ground above it, once the gate is shut.
    pub breach: Pos,
    /// The tunnel from the halls to the stair, the hall's end first.
    pub way: Vec<Pos>,
    /// The lever that works the gate, once placed.
    #[serde(default)]
    pub lever: Option<Pos>,
    /// The breach has been ordered: water stands at the gate.
    #[serde(default)]
    pub breached: bool,
    /// Something was dug into the chamber: the gate stays shut.
    #[serde(default)]
    pub leak: bool,
    pub since: u64,
}

impl CisternPlan {
    fn chamber_tiles(&self) -> impl Iterator<Item = Pos> {
        box_iter(self.chamber.0, self.chamber.1)
    }

    /// Where water let in at the gate may go.
    fn wet(&self) -> HashSet<Pos> {
        self.chamber_tiles().chain([self.gate, self.drop, self.shaft]).collect()
    }

    /// What each planned tile is dug as (the drop and the well are
    /// channelled later, once the chamber under them is cut).
    fn digs(&self) -> Vec<(Pos, Desig)> {
        let mut v: Vec<(Pos, Desig)> = self.way.iter().map(|&p| (p, Desig::Dig)).collect();
        v.push((self.stair, Desig::StairDown));
        v.push((self.shaft, Desig::StairUD));
        v.extend(self.chamber_tiles().map(|c| (c, if c == self.shaft.down() { Desig::StairUp } else { Desig::Dig })));
        v.push((self.gate, Desig::Dig));
        v
    }
}

/// The fort's waterworks (`Game.waterworks`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Waterworks {
    /// A well being sunk.
    #[serde(default)]
    pub well: Option<WellPlan>,
    /// The cistern, being dug or in use.
    #[serde(default)]
    pub cistern: Option<CisternPlan>,
    /// When to look for water again, after finding none.
    #[serde(skip)]
    pub next_scan: u64,
}

impl WellPlan {
    /// The tiles that become open shaft, top down.
    fn shaft(&self) -> impl Iterator<Item = Pos> + '_ {
        (self.water_z + 1..=self.top.z).rev().map(move |z| Pos::new(self.top.x, self.top.y, z))
    }
}

/// Rooms a well may go in, best first.
fn well_room(kind: RoomUse) -> Option<u8> {
    Some(match kind {
        RoomUse::Dining => 0,
        RoomUse::Meeting | RoomUse::Tavern | RoomUse::Grand => 1,
        RoomUse::Hospital => 2,
        RoomUse::Stockpile | RoomUse::Workshops | RoomUse::Barracks | RoomUse::Temple => 3,
        _ => return None,
    })
}

impl Game {
    /// Sound rock to dig a shaft through: a natural wall, no aquifer.
    fn shaft_rock(&self, p: Pos) -> bool {
        self.world.get(p).map_or(false, |t| t.kind == TileKind::Wall && t.mat.diggable() && t.built_from.is_none() && !t.aquifer && t.magma == 0)
    }

    /// Nothing open, wet, seeping or marked for digging beside `p` on its
    /// level, except `but`.
    fn sealed(&self, p: Pos, but: Option<Pos>) -> bool {
        DIRS8.iter().map(|&(dx, dy)| p.offset(dx, dy, 0)).filter(|&q| Some(q) != but).all(|q| {
            self.world.get(q).map_or(true, |t| {
                t.kind.solid() && t.water == 0 && t.magma == 0 && t.desig == Desig::None && !(t.aquifer && self.world.aquifer >= 2)
            })
        })
    }

    /// Water under `p` within a well's reach, through sound rock (and beside
    /// `stair`'s column, when there is one): the level it's at.
    fn water_below(&self, p: Pos, stair: Option<Pos>) -> Option<i32> {
        for k in 1..=WELL_DEPTH {
            let q = p.offset(0, 0, -k);
            let t = self.world.get(q)?;
            let heavy = t.aquifer && self.world.aquifer >= 2 && t.kind == TileKind::Wall;
            if (t.water >= 3 && !t.frozen && t.magma == 0) || heavy {
                // Right below the floor needs no stair; deeper needs one, its
                // foot on rock.
                return match (k, stair) {
                    (1, _) => Some(q.z),
                    (_, Some(s)) if self.world.solid(Pos::new(s.x, s.y, q.z)) => Some(q.z),
                    _ => None,
                };
            }
            let s = stair.map(|s| Pos::new(s.x, s.y, q.z));
            if !self.shaft_rock(q) || !self.sealed(q, s) {
                return None;
            }
            if let Some(s) = s {
                if !self.shaft_rock(s) || !self.sealed(s, Some(q)) {
                    return None;
                }
            }
        }
        None
    }

    /// Free floor in a hall: nothing built, planned or wet on it, not a door
    /// or a way in.
    fn free_floor(&self, o: &Overseer, p: Pos) -> bool {
        self.world.get(p).map_or(false, |t| t.kind == TileKind::Floor && t.building == 0 && t.water == 0 && t.desig == Desig::None && !self.world.outside(p))
            && !o.anchors.contains(&p)
            && !o.rooms.iter().any(|r| r.doors.contains(&p))
    }

    /// The free corners of the fitted halls a well may go in, best hall
    /// first: (rank, corner, which way the room lies in x and in y). A well
    /// in a corner is out of everyone's way.
    fn hall_corners(&self, o: &Overseer) -> Vec<(u8, Pos, i32, i32)> {
        let mut rooms: Vec<(u8, &super::overseer::Room)> = o.rooms.iter().filter(|r| r.stage >= 1).filter_map(|r| well_room(r.kind).map(|k| (k, r))).collect();
        rooms.sort_by_key(|(k, _)| *k);
        let mut out = Vec::new();
        for (rank, r) in rooms {
            let tiles: Vec<Pos> = if r.tiles.is_empty() { box_iter(r.a, r.b).collect() } else { r.tiles.clone() };
            for p in tiles {
                if !self.free_floor(o, p) {
                    continue;
                }
                // Walls on two sides, the room on the other two.
                let solid = |dx: i32, dy: i32| self.world.solid(p.offset(dx, dy, 0));
                let ox = match (solid(-1, 0), solid(1, 0)) {
                    (true, false) => 1,
                    (false, true) => -1,
                    _ => continue,
                };
                let oy = match (solid(0, -1), solid(0, 1)) {
                    (true, false) => 1,
                    (false, true) => -1,
                    _ => continue,
                };
                if [(ox, 0), (0, oy), (ox, oy)].iter().all(|&(dx, dy)| self.world.walkable(p.offset(dx, dy, 0))) {
                    out.push((rank, p, ox, oy));
                }
            }
        }
        out
    }

    /// Where a well could be sunk inside the fort, over water a shaft can
    /// reach: a corner of a fitted hall in the shelter, the dining hall first.
    pub(super) fn find_well_spot(&self, o: &Overseer) -> Option<WellPlan> {
        let mut best: Option<(i32, u8, WellPlan)> = None;
        for (rank, p, ox, oy) in self.hall_corners(o) {
            for s in [p.offset(ox, 0, 0), p.offset(0, oy, 0)] {
                let stair = Some(s).filter(|&s| self.free_floor(o, s));
                let Some(wz) = self.water_below(p, stair) else { continue };
                let depth = p.z - wz;
                let plan = WellPlan { top: p, stair: if depth > 1 { stair } else { None }, water_z: wz, since: self.tick };
                if best.as_ref().map_or(true, |b| (depth, rank) < (b.0, b.1)) {
                    best = Some((depth, rank, plan));
                }
                break;
            }
        }
        best.map(|b| b.2)
    }

    /// Lay out the dig for a well: the stair beside the shaft, and the shaft
    /// channelled down level by level.
    fn begin_well(&mut self, plan: WellPlan) {
        let top = plan.top;
        if let Some(s) = plan.stair {
            let foot = Pos::new(s.x, s.y, plan.water_z + 1);
            self.designate(s, s, Desig::StairDown);
            if foot.z < s.z - 1 {
                self.designate(s.down(), Pos::new(s.x, s.y, foot.z + 1), Desig::StairUD);
            }
            self.designate(foot, foot, Desig::StairUp);
        }
        for p in plan.shaft().collect::<Vec<_>>() {
            self.designate(p, p, Desig::Channel);
        }
        self.waterworks.well = Some(plan);
        let _ = top;
    }

    /// Start on water inside the fort, if there's none yet and none under
    /// way and the overseer can find some: whether it began.
    pub(super) fn start_waterworks(&mut self, o: &mut Overseer) -> bool {
        if o.style == Style::Reckless || o.did("well") || self.waterworks.well.is_some() || self.waterworks.cistern.is_some() {
            return false;
        }
        if self.tick < self.waterworks.next_scan {
            return false;
        }
        let found = (!o.did("nowell") && self.sink_well(o)) || (!o.did("nocistern") && self.plan_cistern(o));
        if !found {
            self.waterworks.next_scan = self.tick + self.knob("water.scan_days", 7.0) as u64 * DAY;
        }
        found
    }

    /// Start sinking a well if there's a spot for one. Whether one was begun.
    fn sink_well(&mut self, o: &mut Overseer) -> bool {
        let Some(plan) = self.find_well_spot(o) else { return false };
        self.begin_well(plan);
        // It needs a bucket to draw with.
        if self.stock_of(ItemKind::Bucket) == 0 && !o.boosts.iter().any(|b| b.0 == crate::buildings::Recipe::Bucket) {
            o.boosts.push((crate::buildings::Recipe::Bucket, 2, self.tick + DAY * 20));
        }
        let room = o.rooms.iter().find(|r| r.tiles.contains(&plan.top) || (r.tiles.is_empty() && box_iter(r.a, r.b).any(|p| p == plan.top))).map_or("the halls".to_string(), |r| format!("the {:?}", r.kind).to_lowercase());
        let text = o.voiced(&format!("Set miners to sink a well in {}, {} levels down to water.", room.replace("dining", "dining hall"), plan.top.z - plan.water_z));
        Self::note_ov(o, self.tick, text);
        true
    }

    /// Once a day (`overseer_hooks.rs`): see the waterworks along.
    pub(super) fn ov_water(&mut self, o: &mut Overseer) {
        if o.style == Style::Reckless {
            return;
        }
        // Once there's a shelter to keep water in, find some.
        if o.did("burrow") {
            self.start_waterworks(o);
        }
        self.tend_well(o);
        self.tend_cistern(o);
    }

    /// The well under way: put the well over the shaft once it's open, and
    /// call it done when it draws water.
    fn tend_well(&mut self, o: &mut Overseer) {
        let Some(plan) = self.waterworks.well else { return };
        let top = plan.top;
        let tile = *self.world.t(top);
        let bld = self.buildings.get(&tile.building).filter(|b| b.kind == BuildingKind::Well);
        match bld {
            Some(b) if b.built && self.well_has_water(top) => {
                self.waterworks.well = None;
                o.done.push("well".into());
                self.count("wells", 1);
                let fort = self.fortress_name.clone();
                let text = o.voiced("Sank a well inside the fort: water from below, even in a siege.");
                Self::note_ov(o, self.tick, text);
                self.msg(&format!("A well has been sunk in {}: clean water inside the walls.", fort), (110, 170, 255));
                self.legend(format!("A well was sunk in {}.", fort));
                return;
            }
            Some(_) => {}
            None if tile.kind == TileKind::Open => {
                self.place_building(BuildingKind::Well, top);
            }
            None => {
                // Channels pruned (or never valid): mark the shaft again.
                if tile.desig == Desig::None && tile.kind == TileKind::Floor {
                    self.designate(top, top, Desig::Channel);
                }
            }
        }
        // Given up on after two months (a spot that wasn't what it seemed).
        if self.tick > plan.since + self.knob("well.give_up_days", 60.0) as u64 * DAY {
            self.waterworks.well = None;
            if !o.did("nowell") {
                o.done.push("nowell".into());
            }
            Self::note_ov(o, self.tick, "Gave up on the well: the shaft never reached water.".into());
        }
    }

    // ------------------------------------------------------------- cistern

    /// The shelter's tiles' mask, if there is a shelter.
    fn shelter_mask(&self) -> Option<u8> {
        self.burrows.iter().find(|b| Some(b.bit) == self.alert_burrow).map(|b| b.mask())
    }

    /// Where a cistern could go when there's no water below but some
    /// outside, a lake or a river whose surface is one level under the
    /// halls: the breach a wall beside the water, under ground a dwarf can
    /// stand on and channel; the gate behind it; the drop behind that, with
    /// the stair beside it down to the chamber; the well over the drop; and
    /// a tunnel through sound rock from the halls to the stair.
    pub(super) fn find_cistern_spot(&self, o: &Overseer) -> Option<CisternPlan> {
        let base = o.base?;
        let mask = self.shelter_mask()?;
        let z0 = base.z;
        let lz = z0 - 1;
        let reach = self.knob("cistern.max_tunnel", 30.0) as i32;
        let w = &self.world;
        // (Clear of the inner stair down to the deeper halls.)
        let near_inner = |q: Pos| (q.x - base.x - 9).abs() <= 1 && (q.y - base.y - 1).abs() <= 1;
        let rock = |q: Pos| self.shaft_rock(q) && self.sealed(q, None) && !w.outside(q) && !near_inner(q);
        let shelter = |q: Pos| w.walkable(q) && !w.outside(q) && w.get(q).map_or(false, |t| t.burrows & mask != 0 && t.water == 0);
        // The tunnel's reach from the halls, through rock with nothing open
        // beside it but the halls it starts from.
        let tunnel_rock = |q: Pos| {
            self.shaft_rock(q)
                && !w.outside(q)
                && !near_inner(q)
                && DIRS8.iter().all(|&(dx, dy)| {
                    let n = q.offset(dx, dy, 0);
                    shelter(n) || w.get(n).map_or(true, |t| t.kind.solid() && t.water == 0 && t.desig == Desig::None && !(t.aquifer && w.aquifer >= 2))
                })
        };
        let (x0, x1) = (base.x - 30 - reach, base.x + 60 + reach);
        let (y0, y1) = (base.y - 30 - reach, base.y + 30 + reach);
        let inside = |x: i32, y: i32| x >= x0.max(1) && x <= x1.min(w.w - 2) && y >= y0.max(1) && y <= y1.min(w.h - 2);
        let mut prev: HashMap<(i32, i32), ((i32, i32), i32)> = HashMap::new();
        let mut todo: std::collections::VecDeque<(i32, i32)> = std::collections::VecDeque::new();
        for y in y0.max(1)..=y1.min(w.h - 2) {
            for x in x0.max(1)..=x1.min(w.w - 2) {
                let q = Pos::new(x, y, z0);
                if tunnel_rock(q) && DIRS4.iter().any(|&(dx, dy)| shelter(q.offset(dx, dy, 0))) {
                    prev.insert((x, y), ((x, y), 1));
                    todo.push_back((x, y));
                }
            }
        }
        if prev.is_empty() {
            return None;
        }
        while let Some((x, y)) = todo.pop_front() {
            let dist = prev[&(x, y)].1;
            if dist >= reach {
                continue;
            }
            for &(dx, dy) in DIRS4.iter() {
                let n = (x + dx, y + dy);
                if inside(n.0, n.1) && !prev.contains_key(&n) && tunnel_rock(Pos::new(n.0, n.1, z0)) {
                    prev.insert(n, ((x, y), dist + 1));
                    todo.push_back(n);
                }
            }
        }
        // The water, and the cistern behind each wall beside it. Only a
        // lake or a river will do: a pond would drain into the cistern and
        // dry up in the first drought.
        let min_water = self.knob("cistern.min_water", 40.0) as usize;
        let mut bodies: HashMap<Pos, usize> = HashMap::new();
        let mut body_size = |r: Pos| -> usize {
            if let Some(&n) = bodies.get(&r) {
                return n;
            }
            let mut seen: Vec<Pos> = vec![r];
            let mut set: HashSet<Pos> = HashSet::from([r]);
            let mut i = 0;
            while i < seen.len() && seen.len() < min_water {
                let p = seen[i];
                i += 1;
                for &(dx, dy) in DIRS4.iter() {
                    let q = p.offset(dx, dy, 0);
                    if w.get(q).map_or(false, |t| t.water >= 4 && !t.frozen) && set.insert(q) {
                        seen.push(q);
                    }
                }
            }
            let n = seen.len();
            for p in seen {
                bodies.insert(p, n);
            }
            n
        };
        let mut best: Option<(i32, CisternPlan)> = None;
        for y in y0.max(1)..=y1.min(w.h - 2) {
            for x in x0.max(1)..=x1.min(w.w - 2) {
                let r = Pos::new(x, y, lz);
                let Some(t) = w.get(r) else { continue };
                if t.water < 6 || t.frozen || t.magma > 0 || !w.outside(r) {
                    continue;
                }
                for &(dx, dy) in DIRS4.iter() {
                    // Away from the water: breach, gate, the drop; the shaft
                    // beside the drop, to one side or the other.
                    let br = r.offset(dx, dy, 0);
                    let gate = br.offset(dx, dy, 0);
                    let drop = gate.offset(dx, dy, 0);
                    if !w.in_bounds(drop.offset(2 * dx, 2 * dy, 0)) || !self.shaft_rock(br) || w.surface_z(br.x, br.y) != z0 || !rock(gate) || !rock(drop) {
                        continue;
                    }
                    // Beside the breach only rock or more of the water.
                    let side_ok = [(dy, dx), (-dy, -dx)].iter().all(|&(sx, sy)| {
                        let q = br.offset(sx, sy, 0);
                        w.solid(q) || w.get(q).map_or(false, |t| t.water > 0 && w.outside(q))
                    });
                    // Ground above the breach to channel it from, and somewhere to stand.
                    let up = br.up();
                    let up_ok = w.get(up).map_or(false, |t| {
                        matches!(t.kind, TileKind::Floor | TileKind::Ramp) && t.water == 0 && t.building == 0 && t.desig == Desig::None && t.zone == 0 && t.mat.diggable()
                    });
                    let stand = DIRS8.iter().any(|&(sx, sy)| w.walkable(up.offset(sx, sy, 0)));
                    if !side_ok || !up_ok || !stand {
                        continue;
                    }
                    for (px, py) in [(dy, dx), (-dy, -dx)] {
                        let shaft = drop.offset(px, py, 0);
                        let (well, stair) = (drop.up(), shaft.up());
                        let Some(&(_, cost)) = prev.get(&(stair.x, stair.y)) else { continue };
                        if best.as_ref().map_or(false, |b| b.0 <= cost) || !rock(shaft) {
                            continue;
                        }
                        // The chamber: under the drop and the shaft. Small, so
                        // what comes in fills it deep: a well only needs water
                        // there, and water two deep never dries away.
                        let (c, f) = (drop.down(), shaft.down());
                        let (a, b) = (Pos::new(c.x.min(f.x), c.y.min(f.y), lz - 1), Pos::new(c.x.max(f.x), c.y.max(f.y), lz - 1));
                        // The well and the stair in rock under the ground,
                        // nothing open beside the well but the stair and the halls.
                        let alcove_ok = [well, stair].iter().all(|&q| self.shaft_rock(q) && !w.outside(q) && !near_inner(q))
                            && DIRS8.iter().map(|&(sx, sy)| well.offset(sx, sy, 0)).all(|q| q == stair || w.solid(q) || shelter(q));
                        if !alcove_ok || !box_iter(a, b).all(|q| rock(q)) || body_size(r) < min_water {
                            continue;
                        }
                        let mut way = Vec::new();
                        let mut at = prev[&(stair.x, stair.y)].0;
                        while at != (stair.x, stair.y) {
                            way.push(Pos::new(at.0, at.1, z0));
                            let next = prev[&at].0;
                            if next == at {
                                break;
                            }
                            at = next;
                        }
                        way.reverse();
                        if way.contains(&well) {
                            continue;
                        }
                        let plan = CisternPlan { well, stair, shaft, drop, chamber: (a, b), gate, breach: br, way, lever: None, breached: false, leak: false, since: self.tick };
                        best = Some((cost, plan));
                    }
                }
            }
        }
        best.map(|b| b.1)
    }

    /// Lay out a cistern's dig, if there's a place for one. Whether it began.
    fn plan_cistern(&mut self, o: &mut Overseer) -> bool {
        let Some(plan) = self.find_cistern_spot(o) else { return false };
        for (p, d) in plan.digs() {
            self.designate(p, p, d);
        }
        // The way to it is part of the shelter.
        if let Some(i) = self.burrows.iter().position(|b| Some(b.bit) == self.alert_burrow) {
            let mut tiles = plan.way.clone();
            tiles.push(plan.stair);
            self.burrow_add(i, &tiles);
        }
        let text = o.voiced(&format!(
            "Set miners to cut a cistern by the water outside, {} tiles of tunnel from the halls, a floodgate at the water's edge.",
            plan.way.len() + 1
        ));
        Self::note_ov(o, self.tick, text);
        self.waterworks.cistern = Some(plan);
        true
    }

    /// The building of `kind` standing on `p`, if any.
    fn building_on(&self, p: Pos, kind: BuildingKind) -> Option<&Building> {
        self.buildings.get(&self.world.t(p).building).filter(|b| b.kind == kind)
    }

    /// Once a day: see the cistern dug, gated, levered, broken through to
    /// the water, and its well drawing.
    fn tend_cistern(&mut self, o: &mut Overseer) {
        let Some(mut plan) = self.waterworks.cistern.clone() else { return };
        let tick = self.tick;
        let dug = |g: &Game, p: Pos| !g.world.solid(p);
        // The drop once the chamber under it is cut, then the well over it.
        let open = |g: &Game, p: Pos| g.world.t(p).kind == TileKind::Open;
        let unmarked = |g: &Game, p: Pos| g.world.t(p).desig == Desig::None && g.world.t(p).building == 0;
        if !open(self, plan.drop) && dug(self, plan.drop.down()) && unmarked(self, plan.drop) {
            self.designate(plan.drop, plan.drop, Desig::Channel);
        }
        if self.building_on(plan.well, BuildingKind::Well).is_none() {
            if open(self, plan.well) {
                self.place_building(BuildingKind::Well, plan.well);
            } else if open(self, plan.drop) && dug(self, plan.stair) && unmarked(self, plan.well) {
                self.designate(plan.well, plan.well, Desig::Channel);
            }
        }
        if !plan.breached {
            // Mechanisms for the gate and its lever, after the drawbridge's.
            let spare = o.did("linked") || self.stock_of(ItemKind::Mechanism) >= 4;
            if spare && dug(self, plan.gate) && self.world.t(plan.gate).building == 0 {
                self.place_building(BuildingKind::Floodgate, plan.gate);
            }
            if spare && plan.lever.is_none() {
                // In the hall, by the tunnel's mouth.
                let mouth = plan.way.first().copied().unwrap_or(plan.stair);
                let spot = (1..=3)
                    .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| (dx, dy))))
                    .map(|(dx, dy)| mouth.offset(dx, dy, 0))
                    .find(|&q| !plan.way.contains(&q) && self.free_floor(o, q) && self.world.walkable(q));
                if let Some(q) = spot {
                    if self.place_building(BuildingKind::Lever, q) {
                        plan.lever = Some(q);
                    }
                }
            }
            let gate = self.building_on(plan.gate, BuildingKind::Floodgate).filter(|b| b.built).map(|b| b.id);
            let lever = plan.lever.and_then(|q| self.building_on(q, BuildingKind::Lever)).filter(|b| b.built).map(|b| b.id);
            if let (Some(gate), Some(lever)) = (gate, lever) {
                if !self.buildings[&lever].links.contains(&gate) {
                    let _ = self.link_lever(lever, gate);
                }
                let linked = self.buildings[&lever].links.contains(&gate);
                let shut = self.world.t(plan.gate).door == crate::world::FLOODGATE_SHUT;
                let all_dug = plan.way.iter().chain([plan.stair, plan.shaft].iter()).all(|&p| dug(self, p)) && plan.chamber_tiles().all(|p| dug(self, p)) && open(self, plan.drop);
                let well = self.building_on(plan.well, BuildingKind::Well).is_some();
                if linked && shut && all_dug && well {
                    // Last of all, break through to the water, from the ground above.
                    self.designate(plan.breach.up(), plan.breach.up(), Desig::Channel);
                    plan.breached = true;
                    let text = o.voiced("Had the rock between the cistern's floodgate and the water channelled away; the shut gate holds it back.");
                    Self::note_ov(o, tick, text);
                }
            }
            if !plan.breached && tick > plan.since + self.knob("cistern.give_up_days", 120.0) as u64 * DAY {
                // Never finished: leave it dry.
                for (p, _) in plan.digs() {
                    if self.world.t(p).desig != Desig::None {
                        self.designate(p, p, Desig::None);
                    }
                }
                self.waterworks.cistern = None;
                o.done.push("nocistern".into());
                Self::note_ov(o, tick, "Gave up on the cistern: it was never finished.".into());
                return;
            }
        } else {
            // Whatever was dug since into the chamber would flood.
            if !plan.leak && !self.cistern_sealed(&plan) {
                plan.leak = true;
                Self::note_ov(o, tick, "Saw digging break into the cistern: its floodgate stays shut.".into());
            }
            // The breach's channel, pruned before it was dug: mark it again.
            if self.world.t(plan.breach).kind == TileKind::Wall && self.world.t(plan.breach.up()).desig == Desig::None {
                self.designate(plan.breach.up(), plan.breach.up(), Desig::Channel);
            }
            let full = self.world.t(plan.drop.down()).water >= self.knob("cistern.full", 4.0) as u8;
            if !o.did("cistern") && full && self.building_on(plan.well, BuildingKind::Well).map_or(false, |b| b.built) {
                o.done.push("cistern".into());
                o.done.push("well".into());
                self.count("cisterns", 1);
                self.count("wells", 1);
                let fort = self.fortress_name.clone();
                let text = o.voiced("Filled the cistern by the water: a well inside the walls, even in a siege.");
                Self::note_ov(o, tick, text);
                self.msg(&format!("The cistern of {} has filled: its well draws clean water inside the walls.", fort), (110, 170, 255));
                self.legend(format!("A cistern was cut beneath {} and filled from the water outside.", fort));
            }
        }
        self.waterworks.cistern = Some(plan);
    }

    /// Whether water let in at the gate could only reach the chamber (it
    /// spreads and falls, never rises).
    fn cistern_sealed(&self, plan: &CisternPlan) -> bool {
        let allowed = plan.wet();
        let mut seen: HashSet<Pos> = HashSet::new();
        let mut todo = vec![plan.gate];
        while let Some(p) = todo.pop() {
            if !seen.insert(p) {
                continue;
            }
            if !allowed.contains(&p) {
                return false;
            }
            for &(dx, dy) in DIRS4.iter() {
                let q = p.offset(dx, dy, 0);
                if q != plan.breach && !seen.contains(&q) && (q == plan.gate || !self.world.blocks_water(q)) {
                    todo.push(q);
                }
            }
            let falls = matches!(self.world.kind(p), Some(TileKind::Open | TileKind::StairDown | TileKind::StairUD));
            if falls && !self.world.blocks_water(p.down()) {
                todo.push(p.down());
            }
        }
        true
    }

    /// Every ten ticks (`overseer_hooks.rs`): open the cistern's floodgate
    /// when its well runs low, shut it when it's full or enemies come.
    pub(super) fn ov_cistern_gate(&mut self, o: &mut Overseer) {
        let _ = o;
        let Some(plan) = self.waterworks.cistern.as_ref() else { return };
        if !plan.breached {
            return;
        }
        let Some((gate, shut)) = self.building_on(plan.gate, BuildingKind::Floodgate).filter(|b| b.built).map(|b| (b.id, b.locked)) else { return };
        let lever = plan.lever.and_then(|q| self.building_on(q, BuildingKind::Lever)).filter(|b| b.built && b.links.contains(&gate)).map(|b| (b.id, b.pull));
        let Some((lever, pulling)) = lever else { return };
        let level = self.world.t(plan.drop.down()).water;
        let pull = if !shut {
            self.alert || plan.leak || level >= self.knob("cistern.full", 4.0) as u8
        } else if !self.alert && !plan.leak && level < 2 && self.tick % 50 == 3 && self.world.t(plan.breach).water > 0 {
            // Nobody in the chamber to be caught by the water, and nowhere it can get out.
            let wet = plan.wet();
            !self.dwarves.iter().any(|d| d.alive && wet.contains(&d.pos)) && self.cistern_sealed(plan)
        } else {
            false
        };
        if pull && !pulling {
            self.request_pull(lever);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fort with its halls dug and its shelter marked, and a pocket of
    /// water put two levels under a corner of one of them.
    fn fort_over_water(seed: u64) -> (Game, Pos) {
        let mut g = trial_game(seed, Style::Steady, 40);
        // (Whatever it began by the water outside is put aside.)
        g.waterworks = Waterworks::default();
        let o = g.overseer.clone().unwrap();
        assert!(o.did("burrow") && !o.did("well"));
        assert!(g.find_well_spot(&o).is_none(), "already water below");
        let tiles: Vec<Pos> = o.rooms.iter().filter(|r| r.stage >= 1).flat_map(|r| if r.tiles.is_empty() { box_iter(r.a, r.b).collect() } else { r.tiles.clone() }).collect();
        for p in tiles {
            let q = p.offset(0, 0, -2);
            if !g.world.solid(q) {
                continue;
            }
            let before = *g.world.t(q);
            g.world.t_mut(q).kind = TileKind::Open;
            g.world.t_mut(q).water = 7;
            if g.find_well_spot(&o).map_or(false, |w| w.top == p) {
                g.water_cells = None;
                return (g, p);
            }
            *g.world.t_mut(q) = before;
        }
        panic!("no corner to put water under");
    }

    #[test]
    fn the_overseer_sinks_a_well_over_water() {
        let (mut g, top) = fort_over_water(5020);
        let e = g.embark;
        g.spawn_item(ItemKind::Bucket, Mat::Wood, e);
        g.spawn_item(ItemKind::Block, Mat::Granite, e);
        let start = g.tick;
        while g.tick < start + 3 * DAY && g.working_wells().is_empty() {
            g.step();
        }
        assert_eq!(g.working_wells(), vec![top], "no well after {} ticks; plan {:?}", g.tick - start, g.waterworks.well);
        // Done, told, and counted.
        for _ in 0..DAY {
            g.step();
        }
        let o = g.overseer.as_ref().unwrap();
        assert!(o.did("well"), "the overseer didn't call it done");
        assert!(o.notes.iter().any(|n| n.1.contains("Sank a well")));
        assert_eq!(g.annals.counters.get("wells"), Some(&1));
        assert!(g.waterworks.well.is_none());
        // And never under water itself: the hall is dry.
        assert_eq!(g.world.t(top.offset(1, 0, 0)).water + g.world.t(top.offset(-1, 0, 0)).water, 0);
    }

    /// Carve a cistern by hand deep in the rock, as the overseer would lay
    /// it out: a pool behind an open breach, a shut floodgate, the drop and
    /// the stair beside it down to a two-tile chamber, and a built well over
    /// the drop; a lever up by the wagon linked to the gate.
    fn carved_cistern(g: &mut Game) -> CisternPlan {
        let e = g.embark;
        let solid_block = |g: &Game, c: Pos| (-8..=4).all(|dx| (-3..=4).all(|dy| (-2..=2).all(|dz| g.world.solid(c.offset(dx, dy, dz)))));
        let (w, h) = (g.world.w, g.world.h);
        let spots: Vec<Pos> = (3..e.z - 4).rev().flat_map(|z| (6..h - 6).flat_map(move |y| (10..w - 6).map(move |x| Pos::new(x, y, z)))).collect();
        let c = spots.into_iter().find(|&c| (c.x + c.y) % 3 == 0 && solid_block(g, c)).expect("solid rock for a cistern");
        let open = |g: &mut Game, p: Pos, kind: TileKind, water: u8| {
            let t = g.world.t_mut(p);
            t.kind = kind;
            t.water = water;
            t.revealed = true;
            t.dug = true;
        };
        // The pool, 6 by 3, full; the breach already broken through.
        for dx in -7..=-2 {
            for dy in -1..=1 {
                open(g, c.offset(dx, dy, 0), TileKind::Floor, 7);
            }
        }
        let (breach, gate, drop) = (c.offset(-1, 0, 0), c, c.offset(1, 0, 0));
        let shaft = drop.offset(0, 1, 0);
        open(g, breach, TileKind::Floor, 0);
        open(g, gate, TileKind::Floor, 0);
        open(g, drop, TileKind::Open, 0);
        open(g, shaft, TileKind::StairUD, 0);
        open(g, drop.down(), TileKind::Floor, 0);
        open(g, shaft.down(), TileKind::StairUp, 0);
        open(g, drop.up(), TileKind::Open, 0);
        open(g, shaft.up(), TileKind::StairDown, 0);
        g.water_cells = None;
        g.dig_version += 1;
        // The gate, built and shut.
        assert!(g.place_building(BuildingKind::Floodgate, gate));
        let gid = g.world.t(gate).building;
        let b = g.buildings.get_mut(&gid).unwrap();
        b.built = true;
        b.locked = true;
        g.world.t_mut(gate).door = crate::world::FLOODGATE_SHUT;
        // The well over the drop.
        assert!(g.place_building(BuildingKind::Well, drop.up()));
        let wid = g.world.t(drop.up()).building;
        g.buildings.get_mut(&wid).unwrap().built = true;
        // The lever where dwarves can reach it.
        let lp = g.ov_spot(BuildingKind::Lever, e, 20).unwrap();
        assert!(g.place_building(BuildingKind::Lever, lp));
        let lid = g.world.t(lp).building;
        g.buildings.get_mut(&lid).unwrap().built = true;
        g.spawn_item(ItemKind::Mechanism, Mat::Granite, e);
        g.link_lever(lid, gid).unwrap();
        CisternPlan {
            well: drop.up(),
            stair: shaft.up(),
            shaft,
            drop,
            chamber: (Pos::new(drop.x, drop.y, drop.z - 1), Pos::new(shaft.x, shaft.y, drop.z - 1)),
            gate,
            breach,
            way: Vec::new(),
            lever: Some(lp),
            breached: true,
            leak: false,
            since: g.tick,
        }
    }

    #[test]
    fn a_cistern_fills_and_its_gate_shuts() {
        let mut g = trial_game(5000, Style::Steady, 2);
        g.creatures.retain(|_, c| c.tame);
        let plan = carved_cistern(&mut g);
        assert!(g.cistern_sealed(&plan), "the carved cistern leaks");
        let gid = g.world.t(plan.gate).building;
        g.waterworks.cistern = Some(plan.clone());
        assert!(!g.well_has_water(plan.well));
        let mut opened = false;
        let start = g.tick;
        while g.tick < start + 3 * DAY {
            g.step();
            opened |= !g.buildings[&gid].locked;
            if opened && g.buildings[&gid].locked {
                break;
            }
        }
        assert!(opened, "the overseer never let the water in");
        assert!(g.buildings[&gid].locked, "the gate was never shut again: chamber {:?}", plan.chamber_tiles().map(|p| g.world.t(p).water).collect::<Vec<_>>());
        let deep = g.world.t(plan.drop.down()).water;
        assert!(deep >= 4, "the chamber only filled to {}", deep);
        assert!(g.working_wells().contains(&plan.well));
        // It stays full behind the shut gate, nothing beyond it got wet, and
        // the overseer calls it done.
        for _ in 0..2 * DAY {
            g.step();
        }
        assert!(g.buildings[&gid].locked);
        assert!(g.world.t(plan.drop.down()).water >= 3);
        assert!(g.world.t(plan.well).water == 0 && g.world.t(plan.stair).water == 0, "water rose to the halls");
        assert!(g.cistern_sealed(&plan));
        let o = g.overseer.as_ref().unwrap();
        assert!(o.did("cistern") && o.did("well"), "not called done: {:?}", o.done);
        assert_eq!(g.annals.counters.get("cisterns"), Some(&1));
        // An army at the gates shuts an open floodgate.
        g.pull_lever(g.world.t(g.waterworks.cistern.as_ref().unwrap().lever.unwrap()).building);
        assert!(!g.buildings[&gid].locked);
        let mut o = g.overseer.take().unwrap();
        g.alert = true;
        g.ov_cistern_gate(&mut o);
        g.overseer = Some(o);
        assert!(g.buildings.values().any(|b| b.kind == BuildingKind::Lever && b.pull), "no pull asked for with enemies about");
    }

    /// Watch one fort's waterworks go up (SEED, STYLE, DAYS).
    #[test]
    #[ignore]
    fn watch_the_waterworks() {
        let env = |k: &str, d: &str| std::env::var(k).unwrap_or(d.into());
        let seed: u64 = env("SEED", "5022").parse().unwrap();
        let style = Style::parse(&env("STYLE", "delver")).unwrap();
        let days: u64 = env("DAYS", "200").parse().unwrap();
        let mut g = Game::new(seed);
        g.set_overseer(Some(style));
        let mut last = String::new();
        while g.tick < days * DAY && g.alive_count() > 0 {
            g.step();
            if g.tick % (DAY / 4) != 0 {
                continue;
            }
            let state = match g.waterworks.cistern.as_ref() {
                None => format!("well {:?}", g.waterworks.well.map(|w| w.top)),
                Some(c) => {
                    let dug = c.way.iter().filter(|p| !g.world.solid(**p)).count();
                    let ch: Vec<u8> = c.chamber_tiles().map(|p| g.world.t(p).water).collect();
                    let gate = g.building_on(c.gate, BuildingKind::Floodgate).map(|b| (b.built, b.locked));
                    let lever = c.lever.and_then(|q| g.building_on(q, BuildingKind::Lever)).map(|b| (b.built, b.links.len(), b.pull));
                    format!(
                        "way {}/{} chamber dug {} water {:?} gate {:?} lever {:?} well {:?} breached {} breach water {} leak {} mech {}",
                        dug,
                        c.way.len(),
                        c.chamber_tiles().filter(|p| !g.world.solid(*p)).count(),
                        ch,
                        gate,
                        lever,
                        g.building_on(c.well, BuildingKind::Well).map(|b| b.built),
                        c.breached,
                        g.world.t(c.breach).water,
                        c.leak,
                        g.stock_of(ItemKind::Mechanism)
                    )
                }
            };
            if state != last {
                eprintln!("day {} {}", g.tick / DAY, state);
                last = state;
            }
        }
        if let Some(c) = g.waterworks.cistern.as_ref() {
            let b = c.breach;
            eprintln!("gate {:?} breach {:?} well {:?} way {:?}", c.gate, c.breach, c.well, c.way);
            for z in [b.z + 1, b.z] {
                for y in b.y - 6..=b.y + 6 {
                    let row: String = (b.x - 12..=b.x + 12)
                        .map(|x| {
                            let q = Pos::new(x, y, z);
                            let t = g.world.t(q);
                            if q == c.gate { 'G' } else if q == b { 'B' } else if t.water > 0 { char::from_digit(t.water as u32, 10).unwrap() } else if t.kind.solid() { '#' } else if t.kind == TileKind::Open { ' ' } else { '.' }
                        })
                        .collect();
                    eprintln!("z{} {}", z, row);
                }
            }
        }
        let o = g.overseer.as_ref().unwrap();
        for n in o.notes.iter().filter(|n| n.1.contains("well") || n.1.contains("cistern")) {
            eprintln!("  {} {}", n.0 / DAY, n.1);
        }
        eprintln!("deaths {:?}", g.annals.deaths.iter().map(|d| d.cause.clone()).collect::<Vec<_>>());
        eprintln!("counters {:?}", g.annals.counters);
    }

    /// How many of the batch sites have water a well could reach, once the
    /// halls are dug.
    #[test]
    #[ignore]
    fn well_spots_at_the_batch_sites() {
        let mut found = 0;
        for seed in 5000..5025u64 {
            let style = [Style::Steady, Style::Builder, Style::Delver, Style::Warlord, Style::Reckless][(seed % 5) as usize];
            let g = trial_game(seed, style, 40);
            let Some(o) = g.overseer.as_ref() else { continue };
            let spot = g.find_well_spot(o);
            let t = std::time::Instant::now();
            let cistern = g.find_cistern_spot(o);
            let took = t.elapsed();
            found += (spot.is_some() || cistern.is_some()) as u32;
            eprintln!(
                "seed {} {:?} aquifer {} rooms {} well {:?} cistern {:?} ({:?})",
                seed,
                style,
                g.world.aquifer,
                o.rooms.len(),
                spot.map(|w| w.top.z - w.water_z),
                cistern.map(|c| c.way.len()),
                took
            );
        }
        eprintln!("{} of 25 have a well or cistern spot", found);
    }
}
