//! Fortress plans: halls first, rooms opening off them. Each layout is a
//! fixed plan for a level (a great hall with rooms either side, mirrored
//! wings on cross halls, rooms round a pillared atrium, a grid of lanes,
//! a hub with spokes), worked out the same way every time from the stair,
//! so the overseer only has to remember which slots it has taken. Rooms
//! open onto their hall through a door; bedrooms come in rows of cells.

use super::layout::{Facing, Layout, Shape};
use super::overseer::{Overseer, Room, RoomUse};
use super::*;
use serde::{Deserialize, Serialize};

/// What a level of halls is for: the first holds a bit of everything;
/// those dug below it each keep to a trade, as in the old forts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LevelRole {
    #[default]
    Main,
    Living,
    Workshops,
    Stores,
    Grand,
}

impl LevelRole {
    pub fn name(self) -> &'static str {
        match self {
            LevelRole::Main => "halls",
            LevelRole::Living => "bedrooms",
            LevelRole::Workshops => "workshops",
            LevelRole::Stores => "stores",
            LevelRole::Grand => "great halls",
        }
    }

    /// Whether rooms of `kind` belong on a level of this trade.
    pub fn takes(self, kind: RoomUse) -> bool {
        use RoomUse::*;
        match self {
            LevelRole::Main => true,
            LevelRole::Living => matches!(kind, Bedrooms | Dormitory | Hospital | Suite | Parlour | Office),
            LevelRole::Workshops => matches!(kind, Workshops | Industry | Stockpile),
            LevelRole::Stores => matches!(kind, Stockpile | Tomb | Jail),
            LevelRole::Grand => matches!(kind, Grand | Dining | Tavern | Temple | Meeting | Library | Barracks),
        }
    }

    /// The trade of a level opened for a room of `kind`.
    pub fn for_kind(kind: RoomUse) -> LevelRole {
        use RoomUse::*;
        match kind {
            Bedrooms | Dormitory | Suite | Parlour => LevelRole::Living,
            Workshops | Industry => LevelRole::Workshops,
            Stockpile | Tomb => LevelRole::Stores,
            Grand | Dining | Tavern | Temple | Meeting | Library => LevelRole::Grand,
            _ => LevelRole::Main,
        }
    }

    /// The layouts a level of this trade leans to.
    pub(super) fn layouts(self) -> &'static [Layout] {
        match self {
            LevelRole::Main => &[],
            LevelRole::Living => &[Layout::Grid, Layout::Orderly, Layout::Symmetric],
            LevelRole::Workshops => &[Layout::Grid, Layout::Spokes, Layout::Orderly],
            LevelRole::Stores => &[Layout::Orderly, Layout::Grid, Layout::Spokes],
            LevelRole::Grand => &[Layout::Radial, Layout::Symmetric, Layout::Spokes],
        }
    }
}

/// One level's plan: its layout, which way it runs, and how far.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelPlan {
    pub level: i32,
    pub layout: Layout,
    #[serde(default)]
    pub facing: Facing,
    /// How far the main hall runs.
    pub len: i32,
    /// How far the cross halls run (0: each layout's own length, as
    /// older plans had it).
    #[serde(default)]
    pub cross: i32,
    #[serde(default)]
    pub role: LevelRole,
}

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

/// A spot the plan keeps for something fine: a statue, or the monument
/// at the heart of the level (its top-left corner; it is 3×3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feature {
    Statue,
    /// A statue along a great hall, set up once the fort is established.
    HallStatue,
    Monument,
}

#[derive(Default)]
pub(super) struct Blueprint {
    pub segs: Vec<Seg>,
    pub slots: Vec<Slot>,
    /// Statues and the monument: the atrium's and the hub's hearts, statues
    /// along a great hall.
    pub features: Vec<(Pos, Feature)>,
    /// Rooms drawn so far, checked against the halls once all are drawn.
    pending: Vec<Slot>,
    /// What the level is for: which rooms its halls are lined with.
    role: LevelRole,
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
    /// The piece a level of this trade puts in its place (the same length
    /// along the hall, so the plan keeps its rhythm): rows of bedrooms on a
    /// level of bedrooms, big rooms on a level of workshops or stores.
    fn for_role(self, role: LevelRole) -> Piece {
        use Piece::*;
        match (role, self) {
            (LevelRole::Living, Large | Pillared) => Cells(3),
            (LevelRole::Workshops | LevelRole::Stores, Cells(n)) => {
                if n >= 3 {
                    Large
                } else {
                    Medium
                }
            }
            (LevelRole::Grand, Cells(n)) => {
                if n >= 3 {
                    Pillared
                } else {
                    Medium
                }
            }
            (LevelRole::Grand, Large) => Pillared,
            _ => self,
        }
    }

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

    /// Statues every six tiles down a great hall, on alternate sides.
    fn hall_statues(&mut self, run: &Run, from: i32, to: i32) {
        let mut s = from + 3;
        let mut q = 0;
        while s < to {
            self.features.push((run.at(s, q), Feature::HallStatue));
            s += 6;
            q = run.width - 1 - q;
        }
    }

    /// An open space (an atrium, a hub) as a segment of its own.
    fn space(&mut self, tiles: Vec<Pos>, parent: Option<usize>) -> usize {
        self.segs.push(Seg { tiles, parent });
        self.segs.len() - 1
    }

    /// Rooms down one side of a hall (`side` -1 or +1 across it), from
    /// `start` along it, cycling through `pattern`. Returns the slots in order.
    fn side(&self, run: &Run, side: i32, start: i32, pattern: &[Piece]) -> Vec<Slot> {
        let pattern: Vec<Piece> = pattern.iter().map(|p| p.for_role(self.role)).collect();
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

    /// Turn the plan (drawn facing east from (0, 0)) to face `facing` from
    /// `start`: every hall, room, door and bed. Boxes and shapes follow.
    fn turn(&mut self, facing: Facing, start: Pos) {
        let f = |p: Pos| {
            let q = facing.at(start, p.x, p.y);
            Pos::new(q.x, q.y, start.z)
        };
        for s in self.segs.iter_mut() {
            for p in s.tiles.iter_mut() {
                *p = f(*p);
            }
        }
        // (The monument's 3×3 turns about its centre, so its corner moves.)
        for (p, k) in self.features.iter_mut() {
            *p = if *k == Feature::Monument { f(p.offset(1, 1, 0)).offset(-1, -1, 0) } else { f(*p) };
        }
        for s in self.pending.iter_mut() {
            for p in s.tiles.iter_mut().chain(s.doors.iter_mut()).chain(s.beds.iter_mut()) {
                *p = f(*p);
            }
            let x0 = s.tiles.iter().map(|p| p.x).min().unwrap();
            let y0 = s.tiles.iter().map(|p| p.y).min().unwrap();
            s.a = Pos::new(x0, y0, start.z);
            let side = matches!(facing, Facing::North | Facing::South);
            s.shape = match s.shape {
                Shape::Rect { w, h } if side => Shape::Rect { w: h, h: w },
                Shape::Pillared { w, h } => {
                    let (w, h) = if side { (h, w) } else { (w, h) };
                    let shape = Shape::Pillared { w, h };
                    // (The pillars on the box's own grid, as a pillared room is cut.)
                    s.tiles = shape.tiles().into_iter().map(|(x, y)| s.a.offset(x, y, 0)).collect();
                    shape
                }
                other => other,
            };
        }
    }

    /// Keep only rooms that leave rock between themselves, the halls and
    /// each other (earlier rooms win), and none in `keep_out` (the
    /// gatehouse and its walls, the stairs).
    fn finish(&mut self, keep_out: &[(Pos, Pos)]) {
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
        let kept_out = |p: &Pos| keep_out.iter().any(|(a, b)| (a.x..=b.x).contains(&p.x) && (a.y..=b.y).contains(&p.y));
        for slot in slots {
            if slot.tiles.iter().any(kept_out) {
                continue;
            }
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

/// Where a level's plan begins, and what it must keep clear of.
pub(super) struct Start {
    /// Where its first hall begins, and how far along it rooms may start.
    pub at: Pos,
    pub skip: i32,
    /// A passage from the gatehouse to where the plan begins, if it
    /// doesn't begin right at its end.
    pub lead: Vec<Pos>,
    /// Boxes no room may reach into.
    pub keep_out: Vec<(Pos, Pos)>,
}

/// Where level `k`'s plan begins facing `facing`, for a fort whose stair
/// comes down at `b` (level `k` at depth `z`). The main level's halls
/// leave from the inner end of the gatehouse (which always runs east of
/// the stair); those below, from beside the inner stair.
pub(super) fn plan_start(b: Pos, k: i32, z: i32, facing: Facing) -> Start {
    let at = |x: i32, y: i32| Pos::new(b.x + x, b.y + y, z);
    if k == 0 {
        // The gatehouse and its walls, and round the foot of the stair.
        let keep_out = vec![(at(-2, -2), at(14, 3))];
        let (start, lead) = match facing {
            Facing::East => (at(15, 0), Vec::new()),
            Facing::North => (at(13, -1), Vec::new()),
            Facing::South => (at(14, 2), Vec::new()),
            // Up from the gatehouse's end, then back west above it, far
            // enough out for rooms on both sides.
            Facing::West => (at(14, -12), (-11..=-1).flat_map(|y| [at(13, y), at(14, y)]).collect()),
        };
        Start { at: start, skip: 1, lead, keep_out }
    } else {
        // The inner stair and the shaft beside it.
        let keep_out = vec![(at(6, -2), at(13, 4))];
        let (start, skip) = match facing {
            Facing::East => (at(10, 0), 6),
            Facing::West => (at(8, 1), 6),
            Facing::North => (at(9, 0), 4),
            Facing::South => (at(9, 2), 4),
        };
        Start { at: start, skip, lead: Vec::new(), keep_out }
    }
}

/// The plan for one level, facing its way from `start`.
pub(super) fn plan(spec: &LevelPlan, start: &Start) -> Blueprint {
    use Piece::*;
    let mut bp = Blueprint { role: spec.role, ..Default::default() };
    let (layout, skip, len) = (spec.layout, start.skip, spec.len);
    // Drawn facing east from (0, 0), then turned.
    let (x, y, z) = (0, 0, start.at.z);
    let origin = Pos::new(x, y, z);
    let cross = |own: i32| if spec.cross > 0 { spec.cross } else { own };
    match layout {
        Layout::Orderly => {
            // A great hall, rooms either side, then a cross hall at its end.
            let (_, run) = bp.hall(origin, 'E', len, 2, None);
            bp.hall_statues(&run, skip, len - 3);
            let north = bp.side(&run, -1, skip, &[Large, Medium, Large, Medium, Small]);
            let south = bp.side(&run, 1, skip, &[Cells(4), Medium, Cells(4), Large, Small]);
            let first = interleave(vec![north, south]);
            bp.settle(first);
            let end = run.at(len - 2, 0);
            let (_, r1) = bp.hall(end.offset(0, -1, 0), 'N', cross(20), 2, Some(run.chunk(len - 1)));
            let (_, r2) = bp.hall(end.offset(0, 2, 0), 'S', cross(20), 2, Some(run.chunk(len - 1)));
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
            let (_, run) = bp.hall(origin, 'E', len, 2, None);
            bp.hall_statues(&run, skip, len - 3);
            let first = interleave(vec![bp.side(&run, -1, skip, &[Pillared]), bp.side(&run, 1, skip, &[Pillared])]);
            bp.settle(first);
            let mut k = 0;
            let mut at = skip + 13;
            while at + 2 < len {
                let base = run.at(at, 0);
                let (_, r1) = bp.hall(base.offset(0, -1, 0), 'N', cross(18), 2, Some(run.chunk(at + 1)));
                let (_, r2) = bp.hall(base.offset(0, 2, 0), 'S', cross(18), 2, Some(run.chunk(at + 1)));
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
            let (_, lead) = bp.hall(origin, 'E', skip + 1, 2, None);
            let ax = x + skip + 1;
            let (aw, ah) = (17, 9);
            let a0 = Pos::new(ax, y - 3, z);
            let mut atrium: Vec<Pos> = Shape::Pillared { w: aw, h: ah }.tiles().into_iter().map(|(dx, dy)| a0.offset(dx, dy, 0)).collect();
            // The heart of the atrium: the pillar there left out for the
            // monument, statues by the pillars round it.
            atrium.push(a0.offset(7, 4, 0));
            bp.features.push((a0.offset(7, 3, 0), Feature::Monument));
            for (dx, dy) in [(5, 4), (11, 4), (4, 2), (10, 2), (4, 6), (10, 6)] {
                bp.features.push((a0.offset(dx, dy, 0), Feature::Statue));
            }
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
            let (_, run) = bp.hall(origin, 'E', len, 2, None);
            let south = bp.side(&run, 1, skip, &[Large, Large, Medium]);
            bp.settle(south);
            let mut lanes = Vec::new();
            let mut at = skip + 1;
            while at < len - 3 {
                let (_, r) = bp.hall(run.at(at, 0).offset(0, -1, 0), 'N', cross(22), 1, Some(run.chunk(at)));
                lanes.push(bp.side(&r, 1, 1, &[Medium, Cells(2)]));
                lanes.push(bp.side(&r, -1, 1, &[Medium, Cells(2)]));
                at += 14;
            }
            let rooms = interleave(lanes);
            bp.settle(rooms);
        }
        Layout::Spokes => {
            // A round hub, and spokes north, east and south.
            let (_, lead) = bp.hall(origin, 'E', skip + 1, 2, None);
            let r = 4;
            let c = Pos::new(x + skip + 1 + r, y, z);
            let hub: Vec<Pos> = Shape::Round { r }.tiles().into_iter().map(|(dx, dy)| c.offset(dx - r, dy - r, 0)).collect();
            let hub = bp.space(hub, Some(lead.chunk(skip)));
            // The monument at the hub's heart, a statue in each quarter round it.
            bp.features.push((c.offset(-1, -1, 0), Feature::Monument));
            for (dx, dy) in [(-3, -3), (3, -3), (-3, 3), (3, 3)] {
                bp.features.push((c.offset(dx, dy, 0), Feature::Statue));
            }
            let (_, rn) = bp.hall(c.offset(0, -r - 1, 0), 'N', cross(22), 2, Some(hub));
            let (_, rs) = bp.hall(c.offset(0, r + 1, 0), 'S', cross(22), 2, Some(hub));
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
    bp.turn(spec.facing, start.at);
    // The passage in from the gatehouse comes first; the plan hangs off it.
    if !start.lead.is_empty() && !bp.segs.is_empty() {
        let i = bp.segs.len();
        for s in bp.segs.iter_mut().filter(|s| s.parent.is_none()) {
            s.parent = Some(i);
        }
        bp.segs.push(Seg { tiles: start.lead.clone(), parent: None });
    }
    bp.finish(&start.keep_out);
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
        RoomUse::Suite | RoomUse::Industry => &[Large, Medium],
        RoomUse::Library => &[Medium, Large, Small],
        RoomUse::Parlour => &[Small, Medium],
    }
}

/// What the ghost of the plan shows on a tile still to be dug.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ghost {
    Hall,
    Door,
    /// A room nobody has taken yet (its size's letter at its centre:
    /// c bedrooms, s small, m middling, L large).
    Room(Option<char>),
    /// A room taken and still being cut (what it's for, at its centre).
    Taken(Option<char>),
}

impl Game {
    /// Where level `k`'s plan begins, facing its way.
    fn level_start(o: &Overseer, k: i32, facing: Facing) -> Start {
        plan_start(o.base.unwrap(), k, Self::level_z(o, k), facing)
    }

    /// The blueprint of level `k`, if it has been planned.
    pub(super) fn plan_of(o: &Overseer, k: i32) -> Option<Blueprint> {
        o.base?;
        let lp = o.levels.iter().find(|l| l.level == k)?;
        Some(plan(lp, &Self::level_start(o, k, lp.facing)))
    }

    /// Good rock for halls: solid, diggable, dry, under cover.
    fn plan_rock(&self, p: Pos) -> bool {
        self.world.get(p).map_or(false, |t| t.kind == TileKind::Wall && t.mat.diggable() && t.mat != Mat::Bedrock && t.water == 0 && !t.aquifer && t.built_from.is_none())
            && !self.world.outside(p)
    }

    /// How far the rock runs from `start` along `facing` (the main hall's
    /// length) and across it (the cross halls'), so a plan stretches to
    /// the room there is and no further.
    pub(super) fn run_space(&self, start: Pos, facing: Facing) -> (i32, i32) {
        let ok = |p: Pos| self.plan_rock(p) || self.world.get(p).map_or(false, |t| t.dug && t.kind.has_floor());
        // Out along the hall until the rock gives out (three tiles running).
        let reach = |from: Pos, fwd: Facing, side: i32, max: i32| -> i32 {
            let mut last = 0;
            let mut miss = 0;
            for f in 0..max {
                let good = (0..2).all(|q| ok(fwd.at(from, f, q * side)));
                if good {
                    miss = 0;
                    last = f + 1;
                } else {
                    miss += 1;
                    if miss >= 3 {
                        break;
                    }
                }
            }
            last
        };
        // (Short of where it gives out, by a room's depth: the cross halls
        // at the far end want rooms beyond them too.)
        let len = (reach(start, facing, 1, 104) - 8).clamp(30, 96);
        // Across, from a few points along it: the widest it gets.
        let turn = |f: Facing, left: bool| -> Facing {
            match (f, left) {
                (Facing::East, true) | (Facing::West, false) => Facing::North,
                (Facing::East, false) | (Facing::West, true) => Facing::South,
                (Facing::North, true) | (Facing::South, false) => Facing::West,
                (Facing::North, false) | (Facing::South, true) => Facing::East,
            }
        };
        let mut cross = 0;
        for along in [len / 4, len / 2, len - 4] {
            let mid = facing.at(start, along, 0);
            for left in [true, false] {
                cross = cross.max(reach(mid, turn(facing, left), 1, 30) - 2);
            }
        }
        (len, cross.clamp(12, 26))
    }

    /// How many of the plan's rooms (of its first 24) could be dug now:
    /// solid rock round them, and halls the miners can cut to them.
    pub(super) fn plan_fit(&self, o: &Overseer, bp: &Blueprint) -> u32 {
        bp.slots.iter().take(24).filter(|s| self.slot_fits(o, s) && Self::slot_halls(bp, s).iter().all(|&p| self.tunnel_ok(p) && !self.world.outside(p))).count() as u32
    }

    /// The layout a new level of this trade is laid out in: one the trade
    /// leans to, not the same as the level above if it can help it.
    fn layout_for(o: &Overseer, role: LevelRole, k: i32) -> Layout {
        let options = role.layouts();
        if options.is_empty() || !o.layout.planned() {
            return o.layout;
        }
        let b = o.base.unwrap();
        let above = o.levels.iter().find(|l| l.level == k - 1).map_or(o.layout, |l| l.layout);
        let i = (b.x * 7 + b.y * 13 + k * 31).rem_euclid(options.len() as i32) as usize;
        (0..options.len()).map(|j| options[(i + j) % options.len()]).find(|&l| l != above).unwrap_or(options[i])
    }

    /// Level `k`'s plan: the one drawn up when it was opened, or a new one,
    /// facing whichever way finds the most rooms in good rock.
    pub(super) fn level_plan(&self, o: &Overseer, k: i32, kind: RoomUse) -> LevelPlan {
        if let Some(lp) = o.levels.iter().find(|l| l.level == k) {
            return lp.clone();
        }
        // A plan drawn before levels had plans of their own: east, as it was.
        if o.plan_len > 0 && (k == 0 || o.plan_taken.iter().any(|t| t.0 == k)) {
            return LevelPlan { level: k, layout: o.layout, facing: Facing::East, len: o.plan_len, cross: 0, role: LevelRole::Main };
        }
        let role = if k == 0 { LevelRole::Main } else { LevelRole::for_kind(kind) };
        let layout = Self::layout_for(o, role, k);
        let mut best: Option<(u32, LevelPlan)> = None;
        for facing in Facing::ALL {
            let start = Self::level_start(o, k, facing);
            let (len, cross) = self.run_space(start.at, facing);
            let lp = LevelPlan { level: k, layout, facing, len, cross, role };
            // (A passage in to where the plan begins is hall to cut before
            // any room: it counts against the side, a room for every six tiles.)
            let lead = (start.lead.len() as u32 / 2 + 5) / 6;
            let fit = self.plan_fit(o, &plan(&lp, &start)).saturating_sub(lead);
            // (East, the way the gatehouse runs, unless another way is
            // clearly better: a quarter more rooms and then some, as
            // its halls are longer to cut.)
            let beats = |b: &(u32, LevelPlan)| if b.1.facing == Facing::East { fit > b.0 + b.0 / 4 + 1 } else { fit > b.0 };
            if best.as_ref().map_or(true, beats) {
                best = Some((fit, lp));
            }
        }
        best.unwrap().1
    }

    /// Solid, untouched rock all round the slot's tiles (doors aside).
    fn slot_fits(&self, _o: &Overseer, s: &Slot) -> bool {
        s.tiles.iter().all(|p| {
            p.x >= 2 && p.y >= 2 && p.x < self.world.w - 2 && p.y < self.world.h - 2
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
    /// Also returns the halls from the stair to it (for the shelter).
    pub(super) fn plan_slot(&mut self, o: &mut Overseer, kind: RoomUse, k: i32) -> Option<(Room, Vec<Pos>)> {
        let lp = self.level_plan(o, k, kind);
        // A level kept to a trade takes only rooms of that trade (while
        // there may be another level below for the rest).
        if !lp.role.takes(kind) && self.level_allowed(o, k + 1) {
            return None;
        }
        let start = Self::level_start(o, k, lp.facing);
        let bp = plan(&lp, &start);
        if bp.slots.is_empty() {
            return None;
        }
        // The sizes it wants, best first, each in the plan's order; a tomb
        // the nearest that will do (the dead can't wait while the far end
        // of the halls is cut).
        let classes = super::architect::classes(o, kind);
        let mut order: Vec<usize> = Vec::new();
        for &class in &classes {
            order.extend(bp.slots.iter().enumerate().filter(|(_, s)| s.class == class).map(|(i, _)| i));
        }
        if kind == RoomUse::Tomb {
            order.sort_by_key(|&i| Self::slot_halls(&bp, &bp.slots[i]).len());
        }
        {
            for i in order {
                let s = &bp.slots[i];
                if o.plan_taken.contains(&(k, i as u32)) {
                    continue;
                }
                if !self.slot_fits(o, s) {
                    // Something's in the way (a cave, water, another room): skip it for good.
                    o.plan_taken.push((k, i as u32));
                    continue;
                }
                let halls = Self::slot_halls(&bp, s);
                // (Never out under the sky: a hall that comes out of the
                // hillside is a way in round the gate.)
                if !halls.iter().chain(s.doors.iter()).all(|&p| self.tunnel_ok(p) && !self.world.outside(p)) {
                    continue;
                }
                if !o.levels.iter().any(|l| l.level == k) {
                    if k == 0 {
                        o.plan_len = lp.len;
                        Self::note_ov(o, self.tick, format!("It faces its halls {}, where the rock runs deepest: {} tiles of it.", lp.facing.name(), lp.len));
                    }
                    o.levels.push(lp.clone());
                }
                o.plan_taken.push((k, i as u32));
                let (w, h) = s.shape.size();
                let kind = if kind == RoomUse::Bedrooms && s.beds.is_empty() { RoomUse::Dormitory } else { kind };
                // A grand taste leaves pillars standing in a big hall.
                let (shape, tiles) = super::architect::restyle(o, kind, s.shape, s.a, &s.tiles);
                for &p in halls.iter().chain(s.doors.iter()).chain(tiles.iter()) {
                    if self.world.get(p).map_or(false, |t| t.kind.solid()) {
                        self.designate(p, p, Desig::Dig);
                    }
                }
                let mut way = halls;
                way.extend(s.doors.iter().copied());
                return Some((
                    Room {
                        kind,
                        a: s.a,
                        b: s.a.offset(w - 1, h - 1, 0),
                        stage: 0,
                        tiles,
                        shape: Some(shape),
                        since: self.tick,
                        doors: s.doors.clone(),
                        beds: s.beds.clone(),
                    },
                    way,
                ));
            }
        }
        None
    }

    /// The overseer's plan for the level of halls at depth `z`, if any.
    pub fn plan_at(&self, z: i32) -> Option<&LevelPlan> {
        let o = self.overseer.as_ref()?;
        o.base?;
        o.levels.iter().find(|l| Self::level_z(o, l.level) == z)
    }

    /// The plan of the level at depth `z` as a ghost: its halls, doors and
    /// rooms still to be dug (rooms given up on aren't shown).
    pub fn plan_ghost(&self, z: i32) -> Vec<(Pos, Ghost)> {
        let Some(lp) = self.plan_at(z).cloned() else { return Vec::new() };
        let o = self.overseer.as_ref().unwrap();
        let bp = plan(&lp, &Self::level_start(o, lp.level, lp.facing));
        let undug = |p: Pos| self.world.get(p).map_or(false, |t| t.kind.solid() && !t.dug);
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut push = |p: Pos, g: Ghost, out: &mut Vec<(Pos, Ghost)>| {
            if undug(p) && seen.insert(p) {
                out.push((p, g));
            }
        };
        for (i, s) in bp.slots.iter().enumerate() {
            let taken = o.plan_taken.contains(&(lp.level, i as u32));
            let (w, h) = s.shape.size();
            let centre = s.a.offset(w / 2, h / 2, 0);
            let ghost = |label: bool| -> Option<Ghost> {
                if !taken {
                    let c = match s.class {
                        Class::Cells => 'c',
                        Class::Small => 's',
                        Class::Medium => 'm',
                        Class::Large => 'L',
                    };
                    Some(Ghost::Room(label.then_some(c)))
                } else {
                    let r = o.rooms.iter().find(|r| r.a == s.a && r.stage < 2)?;
                    Some(Ghost::Taken(label.then(|| format!("{:?}", r.kind).chars().next().unwrap_or('?'))))
                }
            };
            if ghost(false).is_none() || !taken && !self.slot_fits(o, s) {
                continue;
            }
            for &d in &s.doors {
                push(d, Ghost::Door, &mut out);
            }
            for &p in &s.tiles {
                push(p, ghost(p == centre).unwrap(), &mut out);
            }
            for p in Self::slot_halls(&bp, s) {
                push(p, Ghost::Hall, &mut out);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLANNED: [Layout; 5] = [Layout::Orderly, Layout::Symmetric, Layout::Radial, Layout::Grid, Layout::Spokes];

    fn spec(layout: Layout, facing: Facing, k: i32) -> LevelPlan {
        LevelPlan { level: k, layout, facing, len: 64, cross: 0, role: LevelRole::Main }
    }

    /// The plan's invariants: rooms in rock, apart from the halls and each
    /// other, doors between a room and a hall, none in the gatehouse, and
    /// the halls one piece reaching back to where the level is entered.
    fn check_sealed(what: &str, bp: &Blueprint, start: &Start, entry: &[Pos]) {
        assert!(bp.slots.len() >= 10, "{}: only {} rooms in the plan", what, bp.slots.len());
        assert!(bp.slots.iter().any(|s| s.class == Class::Cells), "{}: no bedrooms", what);
        assert!(bp.slots.iter().any(|s| s.class == Class::Large), "{}: no big rooms", what);
        let halls: std::collections::HashSet<Pos> = bp.segs.iter().flat_map(|s| s.tiles.iter().copied()).collect();
        let mut seen: std::collections::HashMap<Pos, usize> = std::collections::HashMap::new();
        for (i, s) in bp.slots.iter().enumerate() {
            for p in &s.tiles {
                assert!(!halls.contains(p), "{}: a room cut into a hall", what);
                assert!(seen.insert(*p, i).is_none(), "{}: rooms overlap", what);
                assert!(!start.keep_out.iter().any(|(a, b)| (a.x..=b.x).contains(&p.x) && (a.y..=b.y).contains(&p.y)), "{}: a room in the gatehouse's walls", what);
            }
            // Its box holds its tiles, and its shape is its box.
            let (w, h) = s.shape.size();
            assert!(s.tiles.iter().all(|p| p.x >= s.a.x && p.y >= s.a.y && p.x < s.a.x + w && p.y < s.a.y + h), "{}: a room outside its box", what);
            // Every door sits between its room and a hall.
            for d in &s.doors {
                let touches = |set: &dyn Fn(Pos) -> bool| [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| set(d.offset(dx, dy, 0)));
                assert!(touches(&|q| halls.contains(&q)), "{}: a door onto nothing", what);
                assert!(touches(&|q| s.tiles.contains(&q)), "{}: a door into nothing", what);
            }
            for b in &s.beds {
                assert!(s.tiles.contains(b), "{}: a bed outside its room", what);
            }
        }
        // Sealed: rock all round each room but at its doors.
        for (i, s) in bp.slots.iter().enumerate() {
            for p in &s.tiles {
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let q = p.offset(dx, dy, 0);
                        if s.doors.contains(&q) {
                            continue;
                        }
                        assert!(!halls.contains(&q), "{}: room {} touches a hall at {:?}", what, i, q);
                        assert!(seen.get(&q).map_or(true, |&j| j == i), "{}: rooms {} and {} touch", what, i, seen[&q]);
                    }
                }
            }
        }
        // The halls are one piece, and reach where the level is entered.
        let mut todo: Vec<Pos> = halls.iter().copied().filter(|p| entry.iter().any(|e| (e.x - p.x).abs() + (e.y - p.y).abs() == 1)).collect();
        assert!(!todo.is_empty(), "{}: the halls don't reach the way in", what);
        let mut reached: std::collections::HashSet<Pos> = todo.iter().copied().collect();
        while let Some(p) = todo.pop() {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let q = p.offset(dx, dy, 0);
                if halls.contains(&q) && reached.insert(q) {
                    todo.push(q);
                }
            }
        }
        assert_eq!(reached.len(), halls.len(), "{}: halls cut off from the rest", what);
    }

    /// The atrium and the hub keep their hearts for the monument (its 3×3
    /// all hall floor) with statues round it, whichever way the plan faces;
    /// a great hall has statues down its length.
    #[test]
    fn the_atrium_and_hub_have_a_heart() {
        let b = Pos::new(120, 120, 20);
        for layout in PLANNED {
            for facing in Facing::ALL {
                let bp = plan(&spec(layout, facing, 0), &plan_start(b, 0, b.z, facing));
                let halls: std::collections::HashSet<Pos> = bp.segs.iter().flat_map(|s| s.tiles.iter().copied()).collect();
                let what = format!("{:?} facing {:?}", layout, facing);
                let hearts: Vec<Pos> = bp.features.iter().filter(|f| f.1 == Feature::Monument).map(|f| f.0).collect();
                let statues = bp.features.iter().filter(|f| f.1 != Feature::Monument).count();
                match layout {
                    Layout::Radial | Layout::Spokes => {
                        assert_eq!(hearts.len(), 1, "{}: no heart", what);
                        assert!(box_iter(hearts[0], hearts[0].offset(2, 2, 0)).all(|p| halls.contains(&p)), "{}: the monument's place isn't all floor", what);
                        assert!(statues >= 4, "{}: {} statues", what, statues);
                    }
                    Layout::Orderly | Layout::Symmetric => assert!(hearts.is_empty() && statues >= 5, "{}: {} statues down the hall", what, statues),
                    _ => assert!(bp.features.is_empty(), "{}", what),
                }
                for (p, _) in &bp.features {
                    assert!(halls.contains(p), "{}: a statue off the halls at {:?}", what, p);
                }
            }
        }
    }

    #[test]
    fn plans_turn_cleanly() {
        // Every layout, facing every way, on the main level (off the end of
        // the gatehouse) and below (off the inner stair): a valid, sealed plan.
        let b = Pos::new(120, 120, 20);
        for layout in PLANNED {
            let east = plan(&spec(layout, Facing::East, 0), &plan_start(b, 0, b.z, Facing::East)).slots.len();
            for facing in Facing::ALL {
                for k in [0, 1] {
                    let z = if k == 0 { b.z } else { b.z - 2 };
                    let start = plan_start(b, k, z, facing);
                    let bp = plan(&spec(layout, facing, k), &start);
                    let what = format!("{:?} facing {:?} on level {}", layout, facing, k);
                    // The gatehouse's inner end, or the inner stair.
                    let entry: Vec<Pos> = if k == 0 { vec![b.offset(14, 0, 0), b.offset(14, 1, 0)] } else { vec![Pos::new(b.x + 9, b.y + 1, z)] };
                    check_sealed(&what, &bp, &start, &entry);
                    if k == 0 {
                        // Turned, it holds (nearly) as many rooms as facing east.
                        assert!(bp.slots.len() + 3 >= east, "{}: {} rooms, {} facing east", what, bp.slots.len(), east);
                    }
                    // And it faces the way it says.
                    let (fx, fy) = facing.axes().0;
                    let ahead = bp
                        .slots
                        .iter()
                        .filter(|s| {
                            let (w, h) = s.shape.size();
                            (s.a.x + w / 2 - start.at.x) * fx + (s.a.y + h / 2 - start.at.y) * fy > 0
                        })
                        .count();
                    assert!(ahead * 10 >= bp.slots.len() * 7, "{}: only {} of {} rooms ahead", what, ahead, bp.slots.len());
                }
            }
        }
    }

    #[test]
    fn east_plans_are_as_they_were() {
        // Facing east, a plan is the one drawn before plans could turn (saves
        // hold their slots by number): a few slots, checked by position.
        let b = Pos::new(40, 60, 10);
        let bp = plan(&spec(Layout::Orderly, Facing::East, 0), &plan_start(b, 0, b.z, Facing::East));
        assert_eq!(bp.slots[0].a, Pos::new(56, 52, 10));
        assert_eq!(bp.slots[0].shape, Shape::Rect { w: 11, h: 7 });
        assert_eq!(bp.slots[1].class, Class::Cells);
        assert_eq!(bp.slots[1].a, Pos::new(56, 63, 10));
    }

    #[test]
    fn levels_keep_to_their_trade() {
        let b = Pos::new(120, 120, 20);
        let start = plan_start(b, 1, b.z - 2, Facing::South);
        let living = plan(&LevelPlan { role: LevelRole::Living, ..spec(Layout::Orderly, Facing::South, 1) }, &start);
        let shops = plan(&LevelPlan { role: LevelRole::Workshops, ..spec(Layout::Orderly, Facing::South, 1) }, &start);
        let count = |bp: &Blueprint, c: Class| bp.slots.iter().filter(|s| s.class == c).count();
        assert!(count(&living, Class::Cells) > 2 * count(&living, Class::Large), "a level of bedrooms is rows of bedrooms");
        assert_eq!(count(&shops, Class::Cells), 0, "no bedrooms on a level of workshops");
        assert!(count(&shops, Class::Large) >= 6);
        assert!(LevelRole::Living.takes(RoomUse::Bedrooms) && !LevelRole::Living.takes(RoomUse::Workshops));
        assert!(LevelRole::Main.takes(RoomUse::Workshops));
    }

    /// A fort on seed `seed` built over plain granite to the west of `cut`
    /// (just east of the wagon) and bedrock east of it, no caves or water
    /// below ground: only the west has rock to dig.
    fn rock_only_west(seed: u64) -> (Game, i32) {
        let mut g = Game::new(seed);
        let cut = g.embark.x + 4;
        g.world.aquifer = 0;
        g.world.caverns.clear();
        for z in 0..g.world.d {
            for y in 0..g.world.h {
                for x in 0..g.world.w {
                    let p = Pos::new(x, y, z);
                    if z < g.world.surface_z(x, y) {
                        let t = g.world.t_mut(p);
                        t.kind = TileKind::Wall;
                        t.mat = if x > cut { Mat::Bedrock } else { Mat::Granite };
                        t.water = 0;
                        t.magma = 0;
                        t.aquifer = false;
                    }
                }
            }
        }
        (g, cut)
    }

    #[test]
    fn a_fort_with_rock_only_to_the_west_still_fills_its_plan() {
        let (mut g, cut) = rock_only_west(5);
        g.set_overseer(Some(super::super::overseer::Style::Steady));
        for _ in 0..30 {
            g.step();
        }
        {
            let o = g.overseer.as_mut().expect("an overseer");
            assert!(o.base.is_some(), "dug in");
            o.layout = Layout::Orderly;
            o.nudges = vec![0, 0, 0, 0, 0, 3];
        }
        for _ in 0..(SEASON * 2) {
            g.step();
        }
        let o = g.overseer.as_ref().unwrap();
        let b = o.base.unwrap();
        let main = o.levels.iter().find(|l| l.level == 0).expect("a plan for the main level");
        assert_ne!(main.facing, Facing::East, "it faced its halls into the bedrock");
        let lp = main.clone();
        let bp = plan(&lp, &Game::level_start(o, 0, lp.facing));
        let planned: Vec<&Room> = o.rooms.iter().filter(|r| r.a.z == b.z && bp.slots.iter().any(|s| s.a == r.a)).collect();
        assert!(planned.len() >= 6, "only {} rooms on the main level: {:?} (plan {:?})", planned.len(), o.rooms.iter().map(|r| (r.kind, r.a)).collect::<Vec<_>>(), lp);
        assert!(planned.iter().filter(|r| r.stage >= 1).count() >= 3, "rooms dug and fitted");
        for r in &o.rooms {
            assert!(r.b.x <= cut, "a room in the bedrock: {:?} at {:?}", r.kind, r.a);
        }
        // Levels opened below keep to a trade of their own.
        assert!(o.levels.iter().filter(|l| l.level > 0).all(|l| l.role != LevelRole::Main), "{:?}", o.levels);
        // Its plan is mostly taken (the rest may run out under the hillside).
        let covered = bp.slots.iter().filter(|s| Game::slot_halls(&bp, s).iter().chain(s.tiles.iter()).all(|&p| g.world.in_bounds(p) && !g.world.outside(p))).count();
        assert!(planned.len() * 3 >= covered * 2, "only {} of the plan's {} rooms under cover taken", planned.len(), covered);
    }

    #[test]
    fn the_ghost_marks_free_and_taken_rooms() {
        let (mut g, _) = rock_only_west(5);
        g.set_overseer(Some(super::super::overseer::Style::Steady));
        for _ in 0..30 {
            g.step();
        }
        g.overseer.as_mut().unwrap().layout = Layout::Grid;
        for _ in 0..(DAY * 3) {
            g.step();
        }
        let o = g.overseer.as_ref().unwrap();
        let Some(lp) = o.levels.first() else { panic!("no plan yet") };
        let z = Game::level_z(o, lp.level);
        let ghost = g.plan_ghost(z);
        assert!(ghost.iter().any(|(_, k)| matches!(k, Ghost::Room(Some(_)))), "rooms still to come");
        assert!(ghost.iter().any(|(_, k)| *k == Ghost::Hall), "halls still to come");
        assert!(ghost.iter().all(|(p, _)| g.world.t(*p).kind.solid()), "only on rock");
    }
}
