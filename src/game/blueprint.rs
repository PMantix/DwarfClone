//! Fortress plans: halls first, rooms opening off them. Each layout is a
//! fixed plan for a level (a great hall with rooms either side, mirrored
//! wings on cross halls, rooms round a pillared atrium, a grid of lanes,
//! a hub with spokes), worked out the same way every time from the stair,
//! so the overseer only has to remember which slots it has taken. Rooms
//! open onto their hall through a door; bedrooms come in rows of cells.

use super::layout::{Layout, Shape};
use super::overseer::{Overseer, Room, RoomUse};
use super::*;

/// One stretch of hall, dug as a whole, hanging off its parent.
pub(super) struct Seg {
    pub tiles: Vec<Pos>,
    pub parent: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Class {
    /// A row of 3×3 bedrooms, each with its own door.
    Cells,
    Small,
    Medium,
    Large,
}

pub(super) struct Slot {
    pub class: Class,
    pub shape: Shape,
    /// Top-left of the box.
    pub a: Pos,
    pub tiles: Vec<Pos>,
    pub doors: Vec<Pos>,
    /// Where the beds go, for a row of cells.
    pub beds: Vec<Pos>,
    pub seg: usize,
}

#[derive(Default)]
pub(super) struct Blueprint {
    pub segs: Vec<Seg>,
    pub slots: Vec<Slot>,
    /// Rooms drawn so far, checked against the halls once all are drawn.
    pending: Vec<Slot>,
}

/// A kind of room along a hall: how far along it runs, how deep it goes.
#[derive(Clone, Copy)]
enum Piece {
    Cells(i32),
    Small,
    Medium,
    Large,
    /// A large room with pillars.
    Pillared,
}

impl Piece {
    fn along(self) -> i32 {
        match self {
            Piece::Cells(n) => 4 * n - 1,
            Piece::Small => 4,
            Piece::Medium => 7,
            Piece::Large | Piece::Pillared => 11,
        }
    }

    fn depth(self) -> i32 {
        match self {
            Piece::Cells(_) => 3,
            Piece::Small => 4,
            Piece::Medium => 6,
            Piece::Large | Piece::Pillared => 7,
        }
    }
}

/// A hall: from `from`, running along `u` for `len`, `width` wide toward
/// `p`, dug in short sections (`chunks`) so only what's needed is cut.
#[derive(Clone)]
struct Run {
    from: Pos,
    u: (i32, i32),
    p: (i32, i32),
    len: i32,
    width: i32,
    chunks: Vec<usize>,
}

/// Tiles of hall in one section.
const CHUNK: i32 = 6;

impl Run {
    fn at(&self, s: i32, q: i32) -> Pos {
        self.from.offset(self.u.0 * s + self.p.0 * q, self.u.1 * s + self.p.1 * q, 0)
    }

    /// The section that reaches `s` along the hall.
    fn chunk(&self, s: i32) -> usize {
        self.chunks[((s.max(0) / CHUNK) as usize).min(self.chunks.len() - 1)]
    }
}

/// The unit vectors for a heading, and the side a hall widens toward.
fn axes(dir: char) -> ((i32, i32), (i32, i32)) {
    match dir {
        'E' => ((1, 0), (0, 1)),
        'W' => ((-1, 0), (0, 1)),
        'N' => ((0, -1), (1, 0)),
        _ => ((0, 1), (1, 0)),
    }
}

impl Blueprint {
    fn hall(&mut self, from: Pos, dir: char, len: i32, width: i32, parent: Option<usize>) -> (usize, Run) {
        let (u, p) = axes(dir);
        let mut run = Run { from, u, p, len, width, chunks: Vec::new() };
        let mut up = parent;
        let mut s0 = 0;
        while s0 < len {
            let tiles = (s0..(s0 + CHUNK).min(len)).flat_map(|s| (0..width).map(move |q| (s, q))).map(|(s, q)| run.at(s, q)).collect();
            self.segs.push(Seg { tiles, parent: up });
            up = Some(self.segs.len() - 1);
            run.chunks.push(self.segs.len() - 1);
            s0 += CHUNK;
        }
        (run.chunks[0], run)
    }

    /// An open space (an atrium, a hub) as a segment of its own.
    fn space(&mut self, tiles: Vec<Pos>, parent: Option<usize>) -> usize {
        self.segs.push(Seg { tiles, parent });
        self.segs.len() - 1
    }

    /// Rooms down one side of a hall (`side` -1 or +1 across it), from
    /// `start` along it, cycling through `pattern`. Returns the slots in order.
    fn side(&self, run: &Run, side: i32, start: i32, pattern: &[Piece]) -> Vec<Slot> {
        let mut out = Vec::new();
        let mut t = start;
        let mut i = 0;
        while !pattern.is_empty() {
            let piece = pattern[i % pattern.len()];
            let (al, dp) = (piece.along(), piece.depth());
            if t + al > run.len {
                break;
            }
            // The wall between hall and room, then the room beyond it.
            let (wall, q0) = if side < 0 { (-1, -1 - dp) } else { (run.width, run.width + 1) };
            let mut tiles = Vec::new();
            let mut doors = Vec::new();
            let mut beds = Vec::new();
            match piece {
                Piece::Cells(n) => {
                    for c in 0..n {
                        let s0 = t + 4 * c;
                        for s in s0..s0 + 3 {
                            for q in q0..q0 + 3 {
                                tiles.push(run.at(s, q));
                            }
                        }
                        doors.push(run.at(s0 + 1, wall));
                        // The bed against the back wall, out of the doorway.
                        beds.push(run.at(s0 + 1, if side < 0 { q0 } else { q0 + 2 }));
                    }
                }
                _ => {
                    for s in t..t + al {
                        for q in q0..q0 + dp {
                            tiles.push(run.at(s, q));
                        }
                    }
                    doors.push(run.at(t + al / 2, wall));
                }
            }
            let (x0, y0) = (tiles.iter().map(|p| p.x).min().unwrap(), tiles.iter().map(|p| p.y).min().unwrap());
            let (x1, y1) = (tiles.iter().map(|p| p.x).max().unwrap(), tiles.iter().map(|p| p.y).max().unwrap());
            let a = Pos::new(x0, y0, run.from.z);
            let (w, h) = (x1 - x0 + 1, y1 - y0 + 1);
            let shape = match piece {
                Piece::Pillared => Shape::Pillared { w, h },
                _ => Shape::Rect { w, h },
            };
            if let Piece::Pillared = piece {
                tiles = shape.tiles().into_iter().map(|(x, y)| a.offset(x, y, 0)).collect();
            }
            let class = match piece {
                Piece::Cells(_) => Class::Cells,
                Piece::Small => Class::Small,
                Piece::Medium => Class::Medium,
                Piece::Large | Piece::Pillared => Class::Large,
            };
            // The hall as far as the room's last door.
            let seg = run.chunk(if let Piece::Cells(n) = piece { t + 4 * (n - 1) + 1 } else { t + al / 2 });
            out.push(Slot { class, shape, a, tiles, doors, beds, seg });
            t += al + 1;
            i += 1;
        }
        out
    }

    /// Rooms to consider, in the order they should be taken.
    fn settle(&mut self, slots: Vec<Slot>) {
        self.pending.extend(slots);
    }

    /// Keep only rooms that leave rock between themselves, the halls and
    /// each other (earlier rooms win).
    fn finish(&mut self) {
        let slots = std::mem::take(&mut self.pending);
        let mut halls: std::collections::HashSet<(i32, i32)> = std::collections::HashSet::new();
        for s in &self.segs {
            for p in &s.tiles {
                halls.insert((p.x, p.y));
            }
        }
        let mut taken: std::collections::HashSet<(i32, i32)> = std::collections::HashSet::new();
        for s in &self.slots {
            for p in &s.tiles {
                taken.insert((p.x, p.y));
            }
        }
        for slot in slots {
            let clash = slot.tiles.iter().any(|p| {
                (-1..=1).any(|dy| (-1..=1).any(|dx| {
                    let q = (p.x + dx, p.y + dy);
                    halls.contains(&q) || taken.contains(&q)
                }))
            }) || slot.doors.iter().any(|d| taken.contains(&(d.x, d.y)));
            if clash {
                continue;
            }
            for p in &slot.tiles {
                taken.insert((p.x, p.y));
            }
            self.slots.push(slot);
        }
    }
}

/// Every other one from each list in turn: twins and spokes fill evenly.
fn interleave(lists: Vec<Vec<Slot>>) -> Vec<Slot> {
    let mut iters: Vec<std::vec::IntoIter<Slot>> = lists.into_iter().map(|l| l.into_iter()).collect();
    let mut out = Vec::new();
    loop {
        let mut any = false;
        for it in iters.iter_mut() {
            if let Some(s) = it.next() {
                out.push(s);
                any = true;
            }
        }
        if !any {
            return out;
        }
    }
}

/// The plan for one level. `start` is where its first hall begins (the
/// end of the gatehouse, or beside the inner stair below), facing east.
pub(super) fn plan(layout: Layout, start: Pos, skip: i32, len: i32) -> Blueprint {
    use Piece::*;
    let mut bp = Blueprint::default();
    let (x, y, z) = (start.x, start.y, start.z);
    match layout {
        Layout::Orderly => {
            // A great hall, rooms either side, then a cross hall at its end.
            let (_, run) = bp.hall(start, 'E', len, 2, None);
            let north = bp.side(&run, -1, skip, &[Large, Medium, Large, Medium, Small]);
            let south = bp.side(&run, 1, skip, &[Cells(4), Medium, Cells(4), Large, Small]);
            let first = interleave(vec![north, south]);
            bp.settle(first);
            let end = run.at(len - 2, 0);
            let (_, r1) = bp.hall(end.offset(0, -1, 0), 'N', 20, 2, Some(run.chunk(len - 1)));
            let (_, r2) = bp.hall(end.offset(0, 2, 0), 'S', 20, 2, Some(run.chunk(len - 1)));
            let more = interleave(vec![
                bp.side(&r1, -1, 1, &[Medium, Cells(3)]),
                bp.side(&r2, -1, 1, &[Medium, Cells(3)]),
                bp.side(&r1, 1, 2, &[Cells(3), Medium]),
                bp.side(&r2, 1, 2, &[Cells(3), Medium]),
            ]);
            bp.settle(more);
        }
        Layout::Symmetric => {
            // A spine with cross halls; every room has its twin across the spine.
            let (_, run) = bp.hall(start, 'E', len, 2, None);
            let first = interleave(vec![bp.side(&run, -1, skip, &[Pillared]), bp.side(&run, 1, skip, &[Pillared])]);
            bp.settle(first);
            let mut k = 0;
            let mut at = skip + 13;
            while at + 2 < len {
                let base = run.at(at, 0);
                let (_, r1) = bp.hall(base.offset(0, -1, 0), 'N', 18, 2, Some(run.chunk(at + 1)));
                let (_, r2) = bp.hall(base.offset(0, 2, 0), 'S', 18, 2, Some(run.chunk(at + 1)));
                let pat_e: &[Piece] = if k % 2 == 0 { &[Medium, Cells(2)] } else { &[Cells(3), Small] };
                let pat_w: &[Piece] = if k % 2 == 0 { &[Cells(3), Small] } else { &[Medium, Cells(2)] };
                let wings = interleave(vec![
                    bp.side(&r1, 1, 1, pat_e),
                    bp.side(&r2, 1, 1, pat_e),
                    bp.side(&r1, -1, 1, pat_w),
                    bp.side(&r2, -1, 1, pat_w),
                ]);
                bp.settle(wings);
                at += 18;
                k += 1;
            }
        }
        Layout::Radial => {
            // A pillared atrium, rooms opening onto it above and below, and a
            // hall on beyond it with more.
            let (_, lead) = bp.hall(start, 'E', skip + 1, 2, None);
            let ax = x + skip + 1;
            let (aw, ah) = (17, 9);
            let a0 = Pos::new(ax, y - 3, z);
            let atrium: Vec<Pos> = Shape::Pillared { w: aw, h: ah }.tiles().into_iter().map(|(dx, dy)| a0.offset(dx, dy, 0)).collect();
            let court = bp.space(atrium, Some(lead.chunk(skip)));
            let top = Run { from: a0, u: (1, 0), p: (0, 1), len: aw, width: ah, chunks: vec![court] };
            let north = bp.side(&top, -1, 1, &[Medium, Small, Medium]);
            let south = bp.side(&top, 1, 1, &[Cells(4)]);
            let first = interleave(vec![north, south]);
            bp.settle(first);
            let rest = len - skip - 1 - aw;
            if rest > 8 {
                let (_, run) = bp.hall(Pos::new(ax + aw, y, z), 'E', rest, 2, Some(court));
                let more = interleave(vec![bp.side(&run, -1, 1, &[Large, Medium]), bp.side(&run, 1, 1, &[Cells(3), Large, Medium])]);
                bp.settle(more);
            }
        }
        Layout::Grid => {
            // A spine, and lanes off it with rooms back to back between them.
            let (_, run) = bp.hall(start, 'E', len, 2, None);
            let south = bp.side(&run, 1, skip, &[Large, Large, Medium]);
            bp.settle(south);
            let mut lanes = Vec::new();
            let mut at = skip + 1;
            while at < len - 3 {
                let (_, r) = bp.hall(run.at(at, 0).offset(0, -1, 0), 'N', 22, 1, Some(run.chunk(at)));
                lanes.push(bp.side(&r, 1, 1, &[Medium, Cells(2)]));
                lanes.push(bp.side(&r, -1, 1, &[Medium, Cells(2)]));
                at += 14;
            }
            let rooms = interleave(lanes);
            bp.settle(rooms);
        }
        Layout::Spokes => {
            // A round hub, and spokes north, east and south.
            let (_, lead) = bp.hall(start, 'E', skip + 1, 2, None);
            let r = 4;
            let c = Pos::new(x + skip + 1 + r, y, z);
            let hub: Vec<Pos> = Shape::Round { r }.tiles().into_iter().map(|(dx, dy)| c.offset(dx - r, dy - r, 0)).collect();
            let hub = bp.space(hub, Some(lead.chunk(skip)));
            let (_, rn) = bp.hall(c.offset(0, -r - 1, 0), 'N', 22, 2, Some(hub));
            let (_, rs) = bp.hall(c.offset(0, r + 1, 0), 'S', 22, 2, Some(hub));
            let (_, re) = bp.hall(c.offset(r + 1, 0, 0), 'E', (len - skip - 2 * r - 3).max(8), 2, Some(hub));
            let spokes = interleave(vec![
                bp.side(&re, -1, 1, &[Large, Medium]),
                bp.side(&rn, 1, 2, &[Cells(3), Medium]),
                bp.side(&rs, 1, 2, &[Medium, Cells(3)]),
                bp.side(&re, 1, 1, &[Cells(4), Large]),
                bp.side(&rn, -1, 2, &[Medium, Small]),
                bp.side(&rs, -1, 2, &[Small, Medium]),
            ]);
            bp.settle(spokes);
        }
        // The sprawl and the warren have no plan: rooms find their own way.
        Layout::Sprawling | Layout::Warren => {}
    }
    bp.finish();
    bp
}

/// The sizes of room each use would like, best first.
pub(super) fn wants(kind: RoomUse) -> &'static [Class] {
    use Class::*;
    match kind {
        RoomUse::Bedrooms => &[Cells],
        RoomUse::Workshops => &[Large],
        RoomUse::Stockpile | RoomUse::Dining | RoomUse::Tavern | RoomUse::Temple | RoomUse::Meeting => &[Large, Medium],
        RoomUse::Dormitory => &[Medium, Large],
        RoomUse::Hospital | RoomUse::Barracks | RoomUse::Tomb => &[Medium, Large, Small],
        RoomUse::Office | RoomUse::Jail => &[Small, Medium],
        RoomUse::Grand => &[],
    }
}

impl Game {
    /// Where level `k`'s plan begins, and how far along its hall rooms may start.
    fn plan_start(o: &Overseer, k: i32) -> (Pos, i32) {
        let b = o.base.unwrap();
        if k == 0 {
            (b.offset(15, 0, 0), 1)
        } else {
            (Pos::new(b.x + 10, b.y, Self::level_z(o, k)), 6)
        }
    }

    /// Solid, untouched rock all round the slot's tiles (doors aside).
    fn slot_fits(&self, o: &Overseer, s: &Slot) -> bool {
        let b = o.base.unwrap();
        s.tiles.iter().all(|p| {
            p.x >= b.x + 15 && p.y >= 2 && p.x < self.world.w - 2 && p.y < self.world.h - 2
                && (-1..=1).all(|dy| (-1..=1).all(|dx| {
                    let q = p.offset(dx, dy, 0);
                    match self.world.get(q) {
                        Some(t) => s.doors.contains(&q) || t.kind == TileKind::Wall && !t.dug && t.built_from.is_none() && t.desig == Desig::None && t.water == 0 && !t.aquifer && t.mat != Mat::Bedrock,
                        None => false,
                    }
                }))
        })
    }

    /// The halls from the stair out to a slot: its segment and every one above it.
    fn slot_halls(bp: &Blueprint, s: &Slot) -> Vec<Pos> {
        let mut v = Vec::new();
        let mut seg = Some(s.seg);
        while let Some(i) = seg {
            v.extend(bp.segs[i].tiles.iter().copied());
            seg = bp.segs[i].parent;
        }
        v
    }

    /// Take the plan's next free slot that suits `kind` on level `k`.
    pub(super) fn plan_slot(&mut self, o: &mut Overseer, kind: RoomUse, k: i32) -> Option<Room> {
        let (start, skip) = Self::plan_start(o, k);
        // How far east the plan runs, fixed when first drawn up (so it stays
        // the same plan if the map grows).
        if o.plan_len == 0 {
            o.plan_len = (self.world.w - 3 - start.x).clamp(30, 64);
        }
        let bp = plan(o.layout, start, skip, o.plan_len);
        if bp.slots.is_empty() {
            return None;
        }
        for &class in wants(kind) {
            for (i, s) in bp.slots.iter().enumerate() {
                if s.class != class || o.plan_taken.contains(&(k, i as u32)) {
                    continue;
                }
                if !self.slot_fits(o, s) {
                    // Something's in the way (a cave, water, another room): skip it for good.
                    o.plan_taken.push((k, i as u32));
                    continue;
                }
                let halls = Self::slot_halls(&bp, s);
                if !halls.iter().chain(s.doors.iter()).all(|&p| self.tunnel_ok(p)) {
                    continue;
                }
                o.plan_taken.push((k, i as u32));
                for &p in halls.iter().chain(s.doors.iter()).chain(s.tiles.iter()) {
                    if self.world.get(p).map_or(false, |t| t.kind.solid()) {
                        self.designate(p, p, Desig::Dig);
                    }
                }
                let (w, h) = s.shape.size();
                let kind = if kind == RoomUse::Bedrooms && s.beds.is_empty() { RoomUse::Dormitory } else { kind };
                return Some(Room {
                    kind,
                    a: s.a,
                    b: s.a.offset(w - 1, h - 1, 0),
                    stage: 0,
                    tiles: s.tiles.clone(),
                    shape: Some(s.shape),
                    since: self.tick,
                    doors: s.doors.clone(),
                    beds: s.beds.clone(),
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_leave_rock_between_rooms_and_halls() {
        for layout in [Layout::Orderly, Layout::Symmetric, Layout::Radial, Layout::Grid, Layout::Spokes] {
            let bp = plan(layout, Pos::new(40, 60, 10), 1, 64);
            assert!(bp.slots.len() >= 10, "{:?}: only {} rooms in the plan", layout, bp.slots.len());
            assert!(bp.slots.iter().any(|s| s.class == Class::Cells), "{:?}: no bedrooms", layout);
            assert!(bp.slots.iter().any(|s| s.class == Class::Large), "{:?}: no big rooms", layout);
            let halls: std::collections::HashSet<Pos> = bp.segs.iter().flat_map(|s| s.tiles.iter().copied()).collect();
            let mut seen = std::collections::HashSet::new();
            for s in &bp.slots {
                for p in &s.tiles {
                    assert!(!halls.contains(p), "{:?}: a room cut into a hall", layout);
                    assert!(seen.insert(*p), "{:?}: rooms overlap", layout);
                }
                // Every door sits between its room and a hall.
                for d in &s.doors {
                    let touches = |set: &dyn Fn(Pos) -> bool| [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| set(d.offset(dx, dy, 0)));
                    assert!(touches(&|q| halls.contains(&q)), "{:?}: a door onto nothing", layout);
                    assert!(touches(&|q| s.tiles.contains(&q)), "{:?}: a door into nothing", layout);
                }
            }
        }
    }
}

