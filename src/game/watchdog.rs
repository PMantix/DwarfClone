//! The overseer's watchdog: a fortress whose rooms have found nowhere to
//! go, or that has made nothing new for weeks while its dwarves stand
//! idle, is stuck. The watchdog notices and steps in, a stronger step each
//! time: dig wherever the rock allows instead of to a plan of halls, then
//! move the stair somewhere with rock for halls, and at last put by what it
//! can't have, saying why (the dead get coffins by the wagon instead).

use super::layout::Layout;
use super::overseer::{room_name, Overseer, RoomUse};
use super::*;

/// Days of no progress before the watchdog steps in.
pub(super) const STALL_DAYS: u64 = 8;

impl Game {
    /// What progress looks like: rooms fitted out and buildings built.
    /// (Rooms laid out don't count: one nobody can reach is given up.)
    fn progress_mark(&self, o: &Overseer) -> (usize, usize) {
        (o.rooms.iter().filter(|r| r.stage == 1).count(), self.buildings.values().filter(|b| b.built).count())
    }

    /// Whether the gatehouse can be walked to its end (where the main
    /// level's halls join), or could once it's dug: no water or magma in it.
    fn hub_open(&self, o: &Overseer) -> bool {
        let b = o.base.unwrap();
        (1..=14).all(|dx| {
            self.world.get(b.offset(dx, 0, 0)).map_or(false, |t| t.water == 0 && t.magma == 0 && (self.world.walkable(b.offset(dx, 0, 0)) || t.desig == Desig::Dig || t.kind.solid()))
        })
    }

    /// Daily: is the fortress stuck? Step in if so.
    pub(super) fn ov_watchdog(&mut self, o: &mut Overseer) {
        let tick = self.tick;
        let mark = self.progress_mark(o);
        if mark != o.stall_mark || o.stall_since == 0 {
            o.stall_mark = mark;
            o.stall_since = tick;
        }
        let alive = self.alive_count();
        let idle = self.dwarves.iter().filter(|d| d.alive && matches!(d.task, crate::dwarf::Task::Idle | crate::dwarf::Task::Wander { .. })).count();
        let want = 5 + alive / 3;
        // (Short of rooms, not just of some room a want asked for: a grown
        // fortress that has put a want by is not stuck.)
        let wanting = o.rooms.iter().filter(|r| r.stage < 2).count() < want;
        // The plan's next room has found nowhere to go for a week and more,
        // or nothing new for twice that while a third of the dwarves idle.
        let planning = o.stuck_since > 0 && tick >= o.stuck_since + STALL_DAYS * DAY;
        let idling = wanting && idle * 3 >= alive && alive > 0 && tick >= o.stall_since + 2 * STALL_DAYS * DAY;
        if !(planning || idling) {
            return;
        }
        self.count("overseer.stalls", 1);
        o.stalls += 1;
        o.stall_since = tick;
        self.ov_unstick(o);
    }

    /// Step in: the gentlest thing that gets a room under way.
    fn ov_unstick(&mut self, o: &mut Overseer) {
        let tick = self.tick;
        // What it means to dig next.
        let kind = o.wants.first().copied().or_else(|| o.blocked.iter().map(|b| b.0).find(|&k| k != RoomUse::Bedrooms)).or_else(|| super::overseer::plan(o.style).get(o.planned).copied()).unwrap_or(RoomUse::Stockpile);
        // 1. Its plan of halls has no room left in it: dig wherever the rock
        // allows round them (`anywhere`), if the halls can be got to at all.
        if self.hub_open(o) {
            let was = o.layout;
            o.layout = Layout::Sprawling;
            if let Some(room) = self.plan_room(o, kind) {
                let how = if was.planned() { format!("Its plan of {} had no room left", was.name()) } else { "Its halls had no room left to grow east".to_string() };
                Self::note_ov(o, tick, format!("{}: began digging wherever the rock allows, a {} first.", how, room_name(room.kind)));
                self.unstuck(o, room);
                return;
            }
            o.layout = was;
        }
        // 2. No rock for halls by this stair: a new stair, somewhere there
        // is. (Not for a fortress with halls in use: that one has just run
        // out of room, and puts by what won't fit.)
        let fitted = o.rooms.iter().filter(|r| r.stage == 1).count();
        if o.resites < 2 && fitted < 3 && self.ov_resite(o) {
            return;
        }
        // 3. Nothing for it: put by what it can't have, and say why (the
        // plan moves on past it).
        if o.wants.is_empty() && super::overseer::plan(o.style).get(o.planned).copied() == Some(kind) {
            o.planned += 1;
        }
        for k in std::mem::take(&mut o.wants) {
            o.blocked.retain(|b| b.0 != k);
            o.blocked.push((k, tick));
        }
        if !o.blocked.iter().any(|b| b.0 == kind) {
            o.blocked.push((kind, tick));
        }
        o.stuck_since = 0;
        let what: Vec<&str> = o.blocked.iter().filter(|b| tick < b.1 + SEASON).map(|b| room_name(b.0)).collect();
        Self::note_ov(o, tick, format!("Found no rock left to cut halls from: gave up on the {} for this season.", what.join(", ")));
    }

    /// A room is under way again.
    fn unstuck(&mut self, o: &mut Overseer, room: super::overseer::Room) {
        o.blocked.retain(|b| b.0 != room.kind);
        o.wants.retain(|&k| k != room.kind);
        if super::overseer::plan(o.style).get(o.planned).copied() == Some(room.kind) {
            o.planned += 1;
        }
        o.stuck_since = 0;
        o.rooms.push(room);
        o.slots = (o.rooms.len() as i32 + 1) / 2;
    }

    /// A new stair down, well away from the old one and with rock for
    /// halls: the old site's halls start over there. False if there's
    /// nowhere better.
    pub(super) fn ov_resite(&mut self, o: &mut Overseer) -> bool {
        let Some(old) = o.top else { return false };
        let Some((s, depth)) = self.stair_site(o.style, Some(old)) else { return false };
        let base = s.offset(0, 0, -depth);
        if base.z < 2 {
            return false;
        }
        // Rooms still to dig by the old stair are given up.
        for r in o.rooms.iter_mut().filter(|r| r.stage == 0) {
            let tiles: Vec<Pos> = if r.tiles.is_empty() { box_iter(r.a, r.b).collect() } else { r.tiles.clone() };
            for p in tiles {
                if self.world.get(p).map_or(false, |t| t.desig == Desig::Dig) {
                    self.designate(p, p, Desig::None);
                }
            }
            r.stage = 2;
        }
        self.designate(s, base, Desig::StairUD);
        self.designate(base.offset(1, 0, 0), base.offset(14, 1, 0), Desig::Dig);
        o.top = Some(s);
        o.base = Some(base);
        o.slots = 0;
        o.level = 0;
        o.anchors.clear();
        o.plan_taken.clear();
        o.plan_len = 0;
        o.stuck_since = 0;
        o.blocked.clear();
        o.resites += 1;
        // The gate, its bridge and traps, the shelter and the levels below
        // all go with the new stair.
        o.done.retain(|d| !matches!(d.as_str(), "gate" | "bridge" | "linked" | "traps" | "burrow" | "quarry") && !d.starts_with("level"));
        o.layout = self.pick_layout(o);
        let far = (s.x - old.x).abs().max((s.y - old.y).abs());
        Self::note_ov(
            o,
            self.tick,
            format!("The old stair's halls had no rock to grow into: struck a new stair {} tiles away, down {} levels, to lay out as {}.", far, depth, o.layout.name()),
        );
        true
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::super::overseer::{trial_game, Style};
    use super::*;

    /// A fort whose stair stands at the map's east edge (its halls run
    /// east, off the map) gets nowhere; the watchdog moves the stair, and
    /// rooms are dug and fitted.
    #[test]
    fn a_stair_with_no_room_for_halls_is_moved() {
        let mut g = trial_game(5002, Style::Steady, 0);
        while g.overseer.as_ref().unwrap().base.is_none() {
            g.step();
        }
        // Then, by hand: the stair right by the east edge.
        let mut o = g.overseer.take().unwrap();
        let w = g.world.w;
        let e = g.embark;
        let mut top = None;
        'find: for x in (w - 12..w - 6).rev() {
            for dy in 0..40 {
                for y in [e.y + dy, e.y - dy] {
                    let z = g.world.surface_z(x, y);
                    let p = Pos::new(x, y, z);
                    let solid = (1..=3).all(|k| g.world.get(p.offset(0, 0, -k)).map_or(false, |t| t.kind.solid() && !t.aquifer));
                    if g.world.in_bounds(p) && g.world.t(p).kind == TileKind::Floor && solid {
                        top = Some(p);
                        break 'find;
                    }
                }
            }
        }
        let top = top.expect("somewhere by the east edge");
        let base = top.offset(0, 0, -3);
        g.designate(top, base, Desig::StairUD);
        g.designate(base.offset(1, 0, 0), base.offset(14.min(w - 2 - base.x), 1, 0), Desig::Dig);
        o.top = Some(top);
        o.base = Some(base);
        o.layout = Layout::Orderly;
        o.done.push("depot".into());
        g.overseer = Some(o);
        let mut moved = None;
        for day in 0..60 {
            for _ in 0..DAY {
                g.step();
            }
            let o = g.overseer.as_ref().unwrap();
            if moved.is_none() && o.resites > 0 {
                moved = Some(day);
            }
            if o.rooms.iter().any(|r| r.stage == 1) {
                break;
            }
        }
        let o = g.overseer.as_ref().unwrap();
        let notes: Vec<_> = o.notes.iter().rev().take(12).collect();
        assert!(o.stalls > 0 && moved.is_some(), "the watchdog never moved the stair: {:?}", notes);
        assert!(o.top != Some(top), "still the old stair");
        assert!(o.rooms.iter().any(|r| r.stage == 1), "no room fitted out after the move (moved day {:?}): {:?}", moved, notes);
        assert_no_repeats(o);
    }

    /// A want that can't be had is put by for the season, not asked for
    /// again daily; and a line said over and over is noted once a half
    /// season at most.
    #[test]
    fn a_blocked_want_is_not_asked_for_daily() {
        let mut g = Game::new(5);
        let mut o = Overseer::new(Style::Steady);
        o.base = Some(g.embark.offset(0, 0, -3));
        g.tick = 100 * DAY;
        o.blocked.push((RoomUse::Hospital, g.tick));
        g.ov_hospital(&mut o);
        assert!(!o.wants.contains(&RoomUse::Hospital), "asked again for a hospital it has no rock for");
        g.tick += SEASON;
        g.ov_hospital(&mut o);
        assert!(o.wants.contains(&RoomUse::Hospital), "a season on, it tries again");
        for day in 0..120 {
            Game::note_ov(&mut o, g.tick + day * DAY, "The dead lie unburied. Ordered a tomb dug.".into());
        }
        let n = o.notes.iter().filter(|n| n.1.contains("unburied")).count();
        assert!(n <= 3, "said {} times in 120 days", n);
        assert_no_repeats(&o);
    }

    /// No line is noted twice within half a season.
    pub(crate) fn assert_no_repeats(o: &Overseer) {
        for (i, a) in o.notes.iter().enumerate() {
            for b in &o.notes[i + 1..] {
                assert!(!(a.1 == b.1 && b.0 < a.0 + SEASON / 2), "said twice in half a season: d{} and d{}: {}", a.0 / DAY, b.0 / DAY, a.1);
            }
        }
    }
}
