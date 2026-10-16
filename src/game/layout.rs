//! The overseer's room planner. Each fortress gets a temperament (neat rows,
//! a sprawl, rings round a rotunda, mirrored wings or a warren), rooms come
//! in different shapes, and the Architect raises grand works. Rooms are
//! only cut where solid rock surrounds them, then tunnelled back to the
//! halls, always behind the gate.

use super::overseer::{Overseer, Room, RoomUse, Style};
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Layout {
    /// A great hall with rooms opening off either side, bedrooms in rows.
    #[default]
    Orderly,
    /// Rooms of every size, scattered, joined by tunnels.
    Sprawling,
    /// A pillared atrium at the heart, rooms opening onto it.
    Radial,
    /// Cross halls off a spine; every room has its twin across it.
    Symmetric,
    /// Small round and crooked rooms on winding tunnels.
    Warren,
    /// Lanes off a spine, rooms back to back between them.
    Grid,
    /// A round hub with halls running out from it.
    Spokes,
}

impl Layout {
    #[allow(dead_code)]
    pub const ALL: [Layout; 7] = [Layout::Orderly, Layout::Sprawling, Layout::Radial, Layout::Symmetric, Layout::Warren, Layout::Grid, Layout::Spokes];

    pub fn name(self) -> &'static str {
        match self {
            Layout::Orderly => "a great hall with rooms off it",
            Layout::Sprawling => "a sprawl",
            Layout::Radial => "rooms round a pillared atrium",
            Layout::Symmetric => "mirrored wings",
            Layout::Warren => "a warren",
            Layout::Grid => "a grid of lanes",
            Layout::Spokes => "a hub with spokes",
        }
    }

    /// Whether this layout works from a plan of halls (the rest just dig).
    pub fn planned(self) -> bool {
        !matches!(self, Layout::Sprawling | Layout::Warren)
    }

    /// What a style tends to dig.
    pub fn pick(style: Style, roll: u32) -> Layout {
        let options: &[Layout] = match style {
            // A tidy mind likes halls; a builder likes a show; a delver digs
            // where the rock leads; a warlord wants barracks lanes.
            Style::Steady => &[Layout::Orderly, Layout::Grid, Layout::Symmetric, Layout::Radial, Layout::Spokes],
            Style::Builder => &[Layout::Radial, Layout::Symmetric, Layout::Spokes, Layout::Orderly],
            Style::Delver => &[Layout::Warren, Layout::Spokes, Layout::Sprawling, Layout::Grid],
            Style::Warlord => &[Layout::Grid, Layout::Orderly, Layout::Spokes, Layout::Symmetric],
            Style::Reckless => &[Layout::Sprawling, Layout::Warren, Layout::Orderly],
        };
        options[roll as usize % options.len()]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Shape {
    Rect { w: i32, h: i32 },
    /// A rectangle with rock pillars left standing every third tile.
    Pillared { w: i32, h: i32 },
    Round { r: i32 },
    /// A plus: two crossing bars.
    Cross { arm: i32, width: i32 },
    /// An L: a w×h box with its far quarter left as rock.
    Ell { w: i32, h: i32 },
}

impl Shape {
    pub fn size(self) -> (i32, i32) {
        match self {
            Shape::Rect { w, h } | Shape::Pillared { w, h } | Shape::Ell { w, h } => (w, h),
            Shape::Round { r } => (2 * r + 1, 2 * r + 1),
            Shape::Cross { arm, .. } => (2 * arm + 1, 2 * arm + 1),
        }
    }

    /// The tiles dug, relative to the top-left of its box.
    pub fn tiles(self) -> Vec<(i32, i32)> {
        let (w, h) = self.size();
        let mut v = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let keep = match self {
                    Shape::Rect { .. } => true,
                    // Pillars on the inner grid, never on the edge.
                    Shape::Pillared { .. } => !(x > 0 && y > 0 && x < w - 1 && y < h - 1 && x % 3 == 1 && y % 3 == 1),
                    Shape::Round { r } => {
                        let (dx, dy) = (x - r, y - r);
                        dx * dx + dy * dy <= r * r + r / 2
                    }
                    Shape::Cross { arm, width } => (x - arm).abs() <= width / 2 || (y - arm).abs() <= width / 2,
                    Shape::Ell { w, h } => !(x >= w / 2 + 1 && y >= h / 2 + 1),
                };
                if keep {
                    v.push((x, y));
                }
            }
        }
        v
    }

    pub fn describe(self) -> &'static str {
        match self {
            Shape::Rect { .. } => "hall",
            Shape::Pillared { .. } => "pillared hall",
            Shape::Round { .. } => "round hall",
            Shape::Cross { .. } => "cross-shaped hall",
            Shape::Ell { .. } => "crooked hall",
        }
    }
}

impl Game {
    /// The level `k` halls are cut on: the main level, then two below it,
    /// then every four (between the exploring shaft's tunnels).
    pub(super) fn level_z(o: &Overseer, k: i32) -> i32 {
        let b = o.base.unwrap();
        if k == 0 {
            b.z
        } else {
            b.z - 2 - 4 * (k - 1)
        }
    }

    /// Where a level's rooms tunnel back to: the end of the gatehouse on
    /// the main level, the foot of the inner stair below.
    fn level_hub(o: &Overseer, k: i32) -> Pos {
        let b = o.base.unwrap();
        if k == 0 {
            b.offset(14, 0, 0)
        } else {
            Pos::new(b.x + 9, b.y + 1, Self::level_z(o, k))
        }
    }

    fn pick_shape(&mut self, o: &Overseer, kind: RoomUse) -> Shape {
        let r = |g: &mut Game, a: i32, b: i32| g.rng.range(a, b + 1);
        if kind == RoomUse::Grand {
            return match self.rng.range(0, 4) {
                0 => Shape::Pillared { w: 15, h: 11 },
                1 => Shape::Round { r: 6 },
                2 => Shape::Rect { w: 3, h: 19 },
                _ => Shape::Cross { arm: 6, width: 3 },
            };
        }
        // Workshops need room for 3×3 shops.
        if kind == RoomUse::Workshops {
            return Shape::Rect { w: 7, h: if o.layout == Layout::Orderly { 7 } else { r(self, 7, 8) } };
        }
        if o.style == Style::Reckless {
            return Shape::Rect { w: r(self, 9, 13), h: r(self, 7, 11) };
        }
        let hall = matches!(kind, RoomUse::Dining | RoomUse::Tavern | RoomUse::Temple | RoomUse::Meeting);
        match o.layout {
            Layout::Orderly | Layout::Grid | Layout::Spokes => Shape::Rect { w: 7, h: 6 },
            Layout::Symmetric => {
                if hall {
                    Shape::Pillared { w: 9, h: 7 }
                } else {
                    Shape::Rect { w: 7, h: 6 }
                }
            }
            Layout::Sprawling => match self.rng.range(0, 4) {
                0 if hall => Shape::Pillared { w: r(self, 9, 11), h: r(self, 7, 9) },
                1 => Shape::Ell { w: r(self, 7, 9), h: r(self, 6, 8) },
                _ => Shape::Rect { w: r(self, 5, 9), h: r(self, 4, 8) },
            },
            Layout::Radial => {
                if hall {
                    Shape::Round { r: r(self, 3, 4) }
                } else {
                    Shape::Rect { w: r(self, 5, 7), h: r(self, 5, 6) }
                }
            }
            Layout::Warren => match self.rng.range(0, 3) {
                0 => Shape::Round { r: r(self, 2, 3) },
                1 => Shape::Cross { arm: 3, width: 3 },
                _ => Shape::Ell { w: r(self, 6, 7), h: r(self, 5, 6) },
            },
        }
    }

    /// Whether a room of `shape` fits with its top-left at `o0`: solid,
    /// untouched rock all round, and well clear of the gatehouse and shaft.
    fn room_fits(&self, o: &Overseer, shape: Shape, o0: Pos) -> bool {
        let b = o.base.unwrap();
        let (w, h) = shape.size();
        if o0.x - 1 < b.x + 15 || o0.x + w + 1 >= self.world.w - 1 || o0.y - 1 < 1 || o0.y + h + 1 >= self.world.h - 1 {
            return false;
        }
        // Clear of every other room's whole box, with rock between.
        let (a2, b2) = (o0.offset(-1, -1, 0), o0.offset(w, h, 0));
        if o.rooms.iter().any(|r| r.a.z == o0.z && !(b2.x < r.a.x - 1 || a2.x > r.b.x + 1 || b2.y < r.a.y - 1 || a2.y > r.b.y + 1)) {
            return false;
        }
        for y in -1..=h {
            for x in -1..=w {
                let Some(t) = self.world.get(o0.offset(x, y, 0)) else { return false };
                if t.kind != TileKind::Wall || t.dug || t.built_from.is_some() || t.desig != Desig::None || t.water > 0 || t.aquifer || t.mat == Mat::Bedrock {
                    return false;
                }
            }
        }
        true
    }

    /// Candidate top-left corners for the next room on level `k`, best first.
    fn room_spots(&mut self, o: &Overseer, shape: Shape, k: i32) -> Vec<Pos> {
        let b = o.base.unwrap();
        let z = Self::level_z(o, k);
        let (w, h) = shape.size();
        let anchors: Vec<Pos> = o.anchors.iter().copied().filter(|a| a.z == z).chain(std::iter::once(Self::level_hub(o, k))).collect();
        let near = |p: Pos| anchors.iter().map(|a| (a.x - p.x).abs() + (a.y - p.y).abs()).min().unwrap_or(0);
        let mut out: Vec<(i32, Pos)> = Vec::new();
        match o.layout {
            Layout::Orderly | Layout::Symmetric | Layout::Grid | Layout::Spokes => {
                // Rows either side of the axis, moving outwards.
                let on_level = o.rooms.iter().filter(|r| r.a.z == z).count() as i32;
                for col in 0..12 {
                    let x = b.x + 16 + col * (w.max(7) + 2);
                    for (row, y) in [(0, b.y - 2 - h), (1, b.y + 3), (2, b.y - 4 - 2 * h), (3, b.y + 5 + h)] {
                        let mut score = col * 10 + row * 3;
                        if o.layout == Layout::Symmetric && on_level % 2 == 1 {
                            // The twin of the last room, across the axis.
                            if let Some(last) = o.rooms.last().filter(|r| r.a.z == z) {
                                let mirror = Pos::new(last.a.x, 2 * b.y + 1 - last.b.y, z);
                                score = if Pos::new(x, y, z) == mirror { -100 } else { score + 50 };
                            }
                        }
                        out.push((score, Pos::new(x, y, z)));
                    }
                }
                if o.layout == Layout::Symmetric {
                    if let Some(last) = o.rooms.last().filter(|r| r.a.z == z && on_level % 2 == 1) {
                        out.push((-100, Pos::new(last.a.x, 2 * b.y + 1 - last.b.y, z)));
                    }
                }
            }
            Layout::Radial => {
                let c = Pos::new(b.x + 28, b.y, z);
                for (ring, radius) in [(0, 0), (1, 12), (2, 19), (3, 26)] {
                    let n = if ring == 0 { 1 } else { 6 + ring * 2 };
                    for i in 0..n {
                        let ang = i as f64 / n as f64 * std::f64::consts::TAU + ring as f64 * 0.4;
                        let p = Pos::new(c.x + (radius as f64 * ang.cos()) as i32 - w / 2, c.y + (radius as f64 * ang.sin() * 0.8) as i32 - h / 2, z);
                        out.push((ring * 100 + i, p));
                    }
                }
            }
            Layout::Sprawling | Layout::Warren => {
                let spread = if o.layout == Layout::Warren { 3 } else { 9 };
                for _ in 0..400 {
                    let x = self.rng.range(b.x + 15, (self.world.w - 3 - w).max(b.x + 16));
                    let y = self.rng.range(2, (self.world.h - 3 - h).max(3));
                    let p = Pos::new(x, y, z);
                    let jitter = self.rng.range(0, spread * 4);
                    out.push((near(p) + jitter, p));
                }
            }
        }
        out.sort_by_key(|x| x.0);
        // When the pattern has no room left, anywhere near the halls will do.
        let mut extra: Vec<(i32, Pos)> = (0..300)
            .map(|_| {
                let x = self.rng.range(b.x + 15, (self.world.w - 3 - w).max(b.x + 16));
                let y = self.rng.range(2, (self.world.h - 3 - h).max(3));
                let p = Pos::new(x, y, z);
                (near(p), p)
            })
            .collect();
        extra.sort_by_key(|x| x.0);
        out.into_iter().chain(extra).map(|x| x.1).collect()
    }

    /// Cut a tunnel from `from` to `to` on one level: straight lines with a
    /// bend, or (in a warren) with a wander in the middle.
    fn tunnel(&mut self, o: &Overseer, from: Pos, to: Pos) -> Vec<Pos> {
        let mut pts = vec![from];
        let b = o.base.unwrap();
        if o.layout == Layout::Warren {
            let mut mid = Pos::new((from.x + to.x) / 2 + self.rng.range(-4, 5), (from.y + to.y) / 2 + self.rng.range(-4, 5), from.z);
            // Never a bend inside the gatehouse: the tunnel couldn't get there.
            if mid.z == b.z {
                mid.x = mid.x.max(b.x + 14);
            }
            pts.push(mid);
        }
        pts.push(to);
        let mut path = Vec::new();
        for w in pts.windows(2) {
            let (a, c) = (w[0], w[1]);
            let x_first = self.rng.chance(0.5);
            let mut p = a;
            let step = |p: &mut Pos, tx: i32, ty: i32, xf: bool| {
                if xf && p.x != tx || p.y == ty {
                    p.x += (tx - p.x).signum();
                } else {
                    p.y += (ty - p.y).signum();
                }
            };
            // (A bound, in case the target lies where the tunnel may not go.)
            let mut steps = 0;
            while p != c && steps < 400 {
                steps += 1;
                step(&mut p, c.x, c.y, x_first);
                // Never into the gatehouse or past it towards the stair.
                if p.x < b.x + 14 && p.z == b.z {
                    p.x = b.x + 14;
                }
                path.push(p);
            }
        }
        path
    }

    /// Whether a tunnel can go through `p`: open dry ground already, or rock
    /// a miner can dig (no water, no aquifer, no bedrock).
    pub(super) fn tunnel_ok(&self, p: Pos) -> bool {
        let Some(t) = self.world.get(p) else { return false };
        if t.water > 0 || t.magma > 0 || t.aquifer {
            return false;
        }
        if t.kind.solid() {
            t.kind == TileKind::Wall && t.mat.diggable() && t.built_from.is_none() && t.mat != Mat::Bedrock
        } else {
            t.kind.has_floor() || t.kind == TileKind::Open && !t.revealed
        }
    }

    /// Find a place for a room of `kind`, dig it and its tunnel. None if
    /// there's nowhere left to put it.
    pub(super) fn plan_room(&mut self, o: &mut Overseer, kind: RoomUse) -> Option<Room> {
        let shape = self.pick_shape(o, kind);
        let (w, h) = shape.size();
        let b = o.base.unwrap();
        let mut level = o.level;
        loop {
            let z = Self::level_z(o, level);
            if z < 3 {
                return None;
            }
            // The plan's next room, if the layout has a plan and room left in it.
            if o.layout.planned() {
                if let Some(room) = self.plan_slot(o, kind, level) {
                    let halls: Vec<Pos> = room.doors.clone();
                    let door = room.doors.first().copied().unwrap_or(room.a);
                    self.open_room(o, level, room.a, room.b, &halls, door);
                    return Some(room);
                }
                // Bedrooms only come in the plan's rows.
                if kind == RoomUse::Bedrooms {
                    if level > o.level + 1 || Self::level_z(o, level + 1) < 3 {
                        return None;
                    }
                    level += 1;
                    continue;
                }
            }
            // The first spot that fits and that a tunnel can reach.
            let hub = Self::level_hub(o, level);
            let anchors: Vec<Pos> = o.anchors.iter().copied().filter(|a| a.z == z).chain(std::iter::once(hub)).collect();
            let dist = |p: Pos, a: Pos| (a.x - p.x).abs() + (a.y - p.y).abs();
            let mut found = None;
            let spots = self.room_spots(o, shape, level);
            for o0 in spots {
                if !self.room_fits(o, shape, o0) {
                    continue;
                }
                let tiles: Vec<Pos> = shape.tiles().into_iter().map(|(x, y)| o0.offset(x, y, 0)).collect();
                // The door: the room's tile nearest the halls.
                let (door, anchor) = tiles
                    .iter()
                    .flat_map(|&t| anchors.iter().map(move |&a| (t, a)))
                    .min_by_key(|&(t, a)| dist(t, a))
                    .unwrap();
                let path = self.tunnel(o, door, anchor);
                if path.iter().all(|&p| self.tunnel_ok(p)) {
                    found = Some((o0, tiles, door, path));
                    break;
                }
            }
            let Some((o0, tiles, door, path)) = found else {
                if level > o.level + 1 {
                    return None;
                }
                level += 1;
                continue;
            };
            for &p in &tiles {
                self.designate(p, p, Desig::Dig);
            }
            for &p in &path {
                self.designate(p, p, Desig::Dig);
            }
            let _ = b;
            self.open_room(o, level, o0, o0.offset(w - 1, h - 1, 0), &path, door);
            let kind = if kind == RoomUse::Bedrooms { RoomUse::Dormitory } else { kind };
            return Some(Room { kind, a: o0, b: o0.offset(w - 1, h - 1, 0), stage: 0, tiles, shape: Some(shape), since: self.tick, doors: Vec::new(), beds: Vec::new() });
        }
    }

    /// A room is under way on level `level`: open the level if it's new,
    /// take it into the shelter, and remember the way to it.
    fn open_room(&mut self, o: &mut Overseer, level: i32, a: Pos, b: Pos, path: &[Pos], door: Pos) {
        let base = o.base.unwrap();
        let z = Self::level_z(o, level);
        if level > o.level {
            o.level = level;
        }
        // The inner stair down to a new level.
        if level > 0 && !o.did(&format!("level{}", level)) {
            let top = Pos::new(base.x + 9, base.y + 1, base.z);
            self.designate(top, Pos::new(top.x, top.y, z), Desig::StairUD);
            o.done.push(format!("level{}", level));
            Self::note_ov(o, self.tick, format!("Opened a new level of halls, at depth {}.", base.z - z));
        }
        // The shelter reaches it.
        if let Some(i) = self.burrows.iter().position(|bw| Some(bw.bit) == self.alert_burrow) {
            self.paint_burrow(i, a.offset(-1, -1, 0), b.offset(1, 1, 0), true);
            for &p in path {
                self.paint_burrow(i, p, p, true);
            }
        }
        o.anchors.push(door);
        o.anchors.push(Pos::new((a.x + b.x) / 2, (a.y + b.y) / 2, a.z));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn show_layouts() {
        let only = std::env::var("LAYOUT").ok();
        for &layout in Layout::ALL.iter() {
            if only.as_deref().map_or(false, |l| !layout.name().contains(l)) {
                continue;
            }
            let seed = std::env::var("SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(5);
            let ow = crate::overworld::Overworld::generate(seed);
            let (sx, sy) = ow.default_site();
            let mut g = Game::embark_with(seed, ow, sx, sy, crate::world::MapSize { w: 144, h: 96, d: 48 }, &|_, _| {});
            g.set_overseer(Some(Style::Builder));
            g.overseer.as_mut().unwrap().nudges = vec![0, 0, 0, 0, 0, 3, 0, 0, 3];
            for _ in 0..30 {
                g.step();
            }
            g.overseer.as_mut().unwrap().layout = layout;
            for _ in 0..(SEASON * 3 / 2) {
                g.step();
            }
            let o = g.overseer.as_ref().unwrap();
            let b = o.base.unwrap();
            eprintln!("== {} ({} rooms: {:?})", layout.name(), o.rooms.len(), o.rooms.iter().map(|r| r.kind).collect::<Vec<_>>());
            let mut s = String::new();
            for y in (b.y - 30).max(0)..(b.y + 31).min(g.world.h) {
                for x in (b.x + 8).max(0)..(b.x + 90).min(g.world.w) {
                    let t = g.world.t(Pos::new(x, y, b.z));
                    s.push(if t.building != 0 {
                        match g.buildings.get(&t.building).map(|b| b.kind) {
                            Some(BuildingKind::Door) => 'D',
                            Some(BuildingKind::Bed) => 'B',
                            Some(BuildingKind::Coffin) => 'C',
                            _ => 'b',
                        }
                    } else if t.desig != Desig::None {
                        '+'
                    } else if !t.kind.solid() {
                        '.'
                    } else {
                        '#'
                    });
                }
                s.push(char::from(10u8));
            }
            eprintln!("{}", s);
        }
    }

    #[test]
    fn shapes_have_their_shapes() {
        let p = Shape::Pillared { w: 15, h: 11 }.tiles();
        assert!(p.len() < 15 * 11 && p.len() > 15 * 11 - 20, "pillars left standing");
        assert!(p.contains(&(0, 0)) && !p.contains(&(1, 1)) && !p.contains(&(4, 4)));
        let r = Shape::Round { r: 6 }.tiles();
        assert!(r.contains(&(6, 6)) && !r.contains(&(0, 0)), "round, not square");
        let c = Shape::Cross { arm: 6, width: 3 }.tiles();
        assert!(c.contains(&(6, 0)) && c.contains(&(0, 6)) && !c.contains(&(0, 0)));
        let l = Shape::Ell { w: 7, h: 6 }.tiles();
        assert!(l.contains(&(0, 0)) && !l.contains(&(6, 5)));
    }

    #[test]
    fn each_layout_places_rooms_that_fit_behind_the_gate() {
        for (i, &layout) in Layout::ALL.iter().enumerate() {
            let mut g = Game::new(5);
            g.set_overseer(Some(Style::Steady));
            for _ in 0..300 {
                g.step();
            }
            let mut o = g.overseer.take().unwrap();
            o.layout = layout;
            o.rooms.clear();
            o.anchors.clear();
            let base = o.base.expect("dug in");
            let mut boxes: Vec<(Pos, Pos)> = Vec::new();
            for k in [RoomUse::Dining, RoomUse::Dormitory, RoomUse::Stockpile, RoomUse::Grand, RoomUse::Tavern, RoomUse::Workshops] {
                let r = g.plan_room(&mut o, k).unwrap_or_else(|| panic!("{:?}: no room for {:?}", layout, k));
                assert!(r.a.x >= base.x + 15, "{:?}: room in front of the gate", layout);
                assert!(r.b.x < g.world.w && r.b.y < g.world.h && r.a.y > 0, "{:?}: off the map", layout);
                for (a, b) in &boxes {
                    let apart = r.b.x < a.x || r.a.x > b.x || r.b.y < a.y || r.a.y > b.y || r.a.z != a.z;
                    assert!(apart, "{:?}: rooms overlap", layout);
                }
                boxes.push((r.a, r.b));
                o.rooms.push(r);
            }
            let _ = i;
        }
    }
}
