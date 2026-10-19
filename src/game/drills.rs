//! Drills in the barracks (A04: MIL-38 teaching, MIL-39 sparring). A squad
//! on a training order doesn't just stand about: its routine turns through
//! sparring in pairs, a demonstration by its best fighter, marching in step
//! and wrestling, while marksdwarves shoot at the archery target (or a bare
//! wall). Practice blows are pulled: a bruise now and then, a real hurt very
//! rarely, and always announced. Bouts feed friendships, grudges, gossip and
//! an epithet. Swings, parries, dodges and bolts in flight show on the map
//! for a moment (`drill_fx`).
//!
//! It's cheap: one pass every other tick over the squads on the training
//! floor, places planned every ten ticks, and soldiers walk to their place
//! a step at a time with no path search (the barracks is one room).

use super::combat::{adjacent, dist2};
use super::*;
use serde::{Deserialize, Serialize};

/// Ticks each drill in the routine lasts.
pub const DRILL_SPAN: u64 = 400;
/// The routine, one drill after another (each squad starts at its own place in it).
const ROUTINE: [Drill; 5] = [Drill::Spar, Drill::March, Drill::Spar, Drill::Demonstrate, Drill::Wrestle];
/// Ticks between a sparring pair's blows.
const EXCHANGE: u64 = 24;
/// Blows to a bout.
const BOUT: u8 = 6;
/// Ticks a marching line takes over each step.
const MARCH_PACE: u64 = 10;
/// Ticks between a marksdwarf's practice shots.
const SHOT_EVERY: u64 = 40;
/// Ticks between the moves of a demonstration.
const SHOW_EVERY: u64 = 60;
/// What watching a move shown by a better fighter teaches.
const WATCH_XP: u32 = 4;
/// Sparring bouts won that earn a name.
const RING_WINS: u32 = 25;
/// Rivalries remembered at most (the oldest are forgotten).
const MAX_RIVALS: usize = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drill {
    Spar,
    Demonstrate,
    March,
    Wrestle,
}

/// What one soldier is doing in the drill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// Sparring with this partner.
    Spar(u32),
    /// Wrestling with this partner.
    Wrestle(u32),
    /// No partner: cuts and thrusts at the air.
    Shadow,
    /// Shooting at the target here.
    Shoot(Pos),
    /// Showing the squad a move.
    Demonstrate,
    /// Watching this soldier show a move.
    Watch(u32),
    /// In the marching line.
    March,
    /// The march is over: ranks broken, getting their breath back.
    AtEase,
}

impl Role {
    pub fn name(self) -> &'static str {
        match self {
            Role::Spar(_) => "sparring",
            Role::Wrestle(_) => "wrestling",
            Role::Shadow => "drilling alone",
            Role::Shoot(_) => "target practice",
            Role::Demonstrate => "demonstrating a move",
            Role::Watch(_) => "watching a demonstration",
            Role::March => "marching in step",
            Role::AtEase => "at ease",
        }
    }
}

/// A brief mark on the map: a swing, a parry, a dodge, a bolt striking.
#[derive(Clone, Copy, Debug)]
pub struct Mark {
    pub pos: Pos,
    pub glyph: char,
    pub rgb: (u8, u8, u8),
    pub from: u64,
    pub until: u64,
}

/// A bolt in flight, a tile every two ticks.
#[derive(Clone, Copy, Debug)]
pub struct Flight {
    pub from: Pos,
    pub to: Pos,
    pub at: u64,
}

impl Flight {
    fn steps(&self) -> i32 {
        (self.to.x - self.from.x).abs().max((self.to.y - self.from.y).abs())
    }

    /// When it strikes.
    pub fn lands(&self) -> u64 {
        self.at + 2 * self.steps() as u64
    }

    /// Where it is at `tick`, if still in the air (the same line as `clear_shot`).
    pub fn at_tick(&self, tick: u64) -> Option<Pos> {
        let steps = self.steps();
        if tick < self.at || steps == 0 {
            return None;
        }
        let i = ((tick - self.at) / 2) as i32 + 1;
        if i >= steps {
            return None;
        }
        let (dx, dy) = (self.to.x - self.from.x, self.to.y - self.from.y);
        let x = self.from.x + (dx * i + dx.signum() * steps / 2) / steps;
        let y = self.from.y + (dy * i + dy.signum() * steps / 2) / steps;
        Some(Pos::new(x, y, self.from.z))
    }

    pub fn glyph(&self) -> char {
        line_glyph(self.to.x - self.from.x, self.to.y - self.from.y)
    }
}

/// A stroke drawn along a direction.
fn line_glyph(dx: i32, dy: i32) -> char {
    if dy == 0 || dx.abs() > 2 * dy.abs() {
        '-'
    } else if dx == 0 || dy.abs() > 2 * dx.abs() {
        '|'
    } else if (dx > 0) == (dy > 0) {
        '\\'
    } else {
        '/'
    }
}

/// A bout under way between two sparring partners.
#[derive(Clone, Debug)]
struct Bout {
    a: u32,
    b: u32,
    blows: u8,
    points: (i32, i32),
}

/// Who has bested whom in the ring, and how many times running.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rivalry {
    pub a: u32,
    pub b: u32,
    /// Who won the last bout, and how many in a row.
    pub last: u32,
    pub run: u8,
    pub bouts: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Drills {
    #[serde(default)]
    pub rivals: Vec<Rivalry>,
    /// Where each soldier on the training floor should stand, and doing what.
    #[serde(skip)]
    pub spots: BTreeMap<u32, (Pos, Role)>,
    #[serde(skip)]
    bouts: Vec<Bout>,
    #[serde(skip)]
    pub marks: Vec<Mark>,
    #[serde(skip)]
    pub flights: Vec<Flight>,
}

/// Squares apart, on one level.
fn gap(a: Pos, b: Pos) -> i32 {
    (a.x - b.x).abs().max((a.y - b.y).abs())
}

/// Whether anything in `used` is on or next to `p`.
fn crowded(used: &HashSet<Pos>, p: Pos) -> bool {
    (-1..=1).any(|dx| (-1..=1).any(|dy| used.contains(&p.offset(dx, dy, 0))))
}

/// The tile around a centre, in turn round the clock.
const AROUND: [(i32, i32); 8] = [(0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1)];

impl Game {
    /// What the squad on the training floor does now.
    pub fn drill_now(&self, squad: u32) -> Drill {
        ROUTINE[((self.tick / DRILL_SPAN + squad as u64) % ROUTINE.len() as u64) as usize]
    }

    /// What a soldier is doing at drill, for the unit view.
    pub fn drill_role(&self, dwarf: u32) -> Option<Role> {
        self.drills.spots.get(&dwarf).map(|s| s.1)
    }

    /// The drills' marks and bolts on level `z` this tick: where, glyph, colour.
    pub fn drill_fx(&self, z: i32) -> Vec<(Pos, char, (u8, u8, u8))> {
        let t = self.tick;
        let mut v: Vec<(Pos, char, (u8, u8, u8))> =
            self.drills.marks.iter().filter(|m| m.pos.z == z && m.from <= t && t < m.until).map(|m| (m.pos, m.glyph, m.rgb)).collect();
        for f in self.drills.flights.iter().filter(|f| f.from.z == z) {
            if let Some(p) = f.at_tick(t) {
                v.push((p, f.glyph(), (215, 195, 145)));
            }
        }
        v
    }

    fn mark(&mut self, pos: Pos, glyph: char, rgb: (u8, u8, u8), from: u64, len: u64) {
        if self.drills.marks.len() < 256 {
            self.drills.marks.push(Mark { pos, glyph, rgb, from, until: from + len });
        }
    }

    /// A bolt seen to fly from `from` to `to` (practice or battle).
    pub(super) fn loose_bolt(&mut self, from: Pos, to: Pos) {
        if self.drills.flights.len() < 64 && from.z == to.z {
            self.drills.flights.push(Flight { from, to, at: self.tick });
        }
    }

    /// Word of the drills, but not every day: at most once in `every` ticks per key.
    fn now_and_then(&mut self, key: &str, text: &str, rgb: (u8, u8, u8), every: u64) {
        let last = self.warns.get(key).copied();
        if last.map_or(true, |t| self.tick >= t + every) {
            self.warns.insert(key.to_string(), self.tick);
            self.msg(text, rgb);
        }
    }

    /// Every other tick: each squad on the training floor at its drill.
    pub(super) fn drills_tick(&mut self) {
        let tick = self.tick;
        self.drills.marks.retain(|m| m.until > tick);
        self.drills.flights.retain(|f| f.lands() + 2 > tick);
        for s in 0..self.squads.len() {
            self.drill_squad(s);
        }
    }

    /// A soldier on the training floor takes a step toward their place in
    /// the drill, straight there (no path search: the barracks is one room).
    pub(super) fn drill_step(&self, d: &mut Dwarf) {
        let Some(&(spot, _)) = self.drills.spots.get(&d.id) else { return };
        if spot == d.pos || spot.z != d.pos.z {
            return;
        }
        if d.move_cd > 0 {
            d.move_cd -= 1;
            return;
        }
        let zone = self.world.t(d.pos).zone;
        if zone == 0 || self.world.t(spot).zone != zone {
            return;
        }
        let mut nb = Vec::new();
        self.world.neighbors(d.pos, &mut nb, Mover::Dwarf);
        let here = dist2(d.pos, spot);
        let best = nb.iter().map(|&(q, _)| q).filter(|q| q.z == d.pos.z && self.world.t(*q).zone == zone).min_by_key(|&q| dist2(q, spot));
        if let Some(q) = best.filter(|&q| dist2(q, spot) < here) {
            d.pos = q;
            d.path.clear();
            d.move_cd = MOVE_DELAY + d.body.limp();
        }
    }

    fn drill_squad(&mut self, s: usize) {
        let (sid, training) = (self.squads[s].id, self.squads[s].order == SquadOrder::Train);
        let members = self.squads[s].members.clone();
        // Who's on the training floor: at drill, awake, in one barracks.
        let mut zone = 0;
        let mut floor: Vec<usize> = Vec::new();
        for m in members {
            let Some(i) = self.dwarves.iter().position(|d| d.id == m) else { continue };
            let d = &self.dwarves[i];
            let zid = self.world.t(d.pos).zone;
            let ok = training
                && d.alive
                && d.task == Task::Train
                && !d.asleep
                && !d.body.unconscious()
                && zid != 0
                && (zone == 0 || zid == zone)
                && self.zones.get(&zid).map_or(false, |z| z.kind == ZoneKind::Barracks);
            if ok {
                zone = zid;
                floor.push(i);
            } else {
                self.drills.spots.remove(&m);
            }
        }
        if floor.is_empty() {
            return;
        }
        let drill = self.drill_now(sid);
        let tick = self.tick;
        if tick % 10 < 2 || floor.iter().any(|&i| !self.drills.spots.contains_key(&self.dwarves[i].id)) {
            self.plan_drill(sid, zone, &floor, drill);
        }
        self.run_drill(s, &floor);
    }

    /// Everyone's place in the drill.
    fn plan_drill(&mut self, sid: u32, zone: u32, floor: &[usize], drill: Drill) {
        let Some(z) = self.zones.get(&zone) else { return };
        let tiles: Vec<Pos> = z.tiles.iter().copied().filter(|&p| self.world.walkable(p) && self.world.t(p).building == 0).collect();
        if tiles.is_empty() {
            return;
        }
        let set: HashSet<Pos> = tiles.iter().copied().collect();
        let mut used: HashSet<Pos> = HashSet::new();
        let mut plan: Vec<(usize, Pos, Role)> = Vec::new();
        let slot = self.tick / DRILL_SPAN;
        match drill {
            Drill::March => self.plan_march(&tiles, &set, floor, &mut plan),
            Drill::Demonstrate => {
                // The best fighter shows the move, or the squad's leader if
                // they're nearly as good.
                let best = *floor.iter().max_by_key(|&&i| (self.dwarves[i].fight_xp, std::cmp::Reverse(i))).unwrap_or(&floor[0]);
                let lead = if self.dwarves[floor[0]].fight_level() + 1 >= self.dwarves[best].fight_level() { floor[0] } else { best };
                let (sx, sy) = tiles.iter().fold((0, 0), |a, p| (a.0 + p.x, a.1 + p.y));
                let n = tiles.len() as i32;
                let mid = Pos::new(sx / n, sy / n, tiles[0].z);
                let centre = *tiles.iter().min_by_key(|&&p| dist2(p, mid)).unwrap_or(&tiles[0]);
                plan.push((lead, centre, Role::Demonstrate));
                used.insert(centre);
                let lid = self.dwarves[lead].id;
                let mut ring: Vec<Pos> = tiles.iter().copied().filter(|&p| (2..=3).contains(&gap(p, centre))).collect();
                ring.sort_by_key(|&p| gap(p, centre));
                let mut ring = ring.into_iter();
                for &i in floor.iter().filter(|&&i| i != lead) {
                    let at = ring.next().unwrap_or(self.dwarves[i].pos);
                    plan.push((i, at, Role::Watch(lid)));
                }
            }
            Drill::Spar | Drill::Wrestle => {
                let target = self.practice_target(&tiles, &set);
                let mut fighters: Vec<usize> = Vec::new();
                for &i in floor {
                    let shot = target.filter(|_| self.holds_crossbow(&self.dwarves[i])).and_then(|t| {
                        let mut c: Vec<Pos> = tiles
                            .iter()
                            .copied()
                            .filter(|&p| !used.contains(&p) && (3..=8).contains(&gap(p, t)) && self.clear_shot(p, t))
                            .collect();
                        c.sort_by_key(|&p| (gap(p, t) - 6).abs());
                        c.first().map(|&p| (p, t))
                    });
                    match shot {
                        Some((p, t)) => {
                            used.insert(p);
                            plan.push((i, p, Role::Shoot(t)));
                        }
                        None => fighters.push(i),
                    }
                }
                // Partners change each time round the routine.
                if fighters.len() > 2 {
                    let k = (slot / ROUTINE.len() as u64) as usize % fighters.len();
                    fighters.rotate_left(k);
                }
                for pair in fighters.chunks(2) {
                    if let &[a, b] = pair {
                        let free = |p: Pos| set.contains(&p) && !crowded(&used, p);
                        let found = tiles.iter().find_map(|&p| {
                            [(1, 0), (0, 1)].iter().map(|&(dx, dy)| p.offset(dx, dy, 0)).find(|&q| free(p) && free(q)).map(|q| (p, q))
                        });
                        let (pa, pb) = found.unwrap_or((self.dwarves[a].pos, self.dwarves[b].pos));
                        used.insert(pa);
                        used.insert(pb);
                        let (ia, ib) = (self.dwarves[a].id, self.dwarves[b].id);
                        let (ra, rb) = if drill == Drill::Wrestle { (Role::Wrestle(ib), Role::Wrestle(ia)) } else { (Role::Spar(ib), Role::Spar(ia)) };
                        plan.push((a, pa, ra));
                        plan.push((b, pb, rb));
                    } else {
                        let a = pair[0];
                        let p = tiles.iter().copied().find(|&p| !crowded(&used, p)).unwrap_or(self.dwarves[a].pos);
                        used.insert(p);
                        plan.push((a, p, Role::Shadow));
                    }
                }
            }
        }
        let _ = sid;
        for (i, p, r) in plan {
            let id = self.dwarves[i].id;
            self.drills.spots.insert(id, (p, r));
        }
    }

    /// A line abreast marching back and forth along the barracks' long
    /// side, a step every `MARCH_PACE` ticks; at the end of the drill they
    /// break ranks.
    fn plan_march(&self, tiles: &[Pos], set: &HashSet<Pos>, floor: &[usize], plan: &mut Vec<(usize, Pos, Role)>) {
        let into = self.tick % DRILL_SPAN;
        if into >= DRILL_SPAN * 17 / 20 {
            for &i in floor {
                plan.push((i, self.dwarves[i].pos, Role::AtEase));
            }
            return;
        }
        let (x0, x1) = (tiles.iter().map(|p| p.x).min().unwrap_or(0), tiles.iter().map(|p| p.x).max().unwrap_or(0));
        let (y0, y1) = (tiles.iter().map(|p| p.y).min().unwrap_or(0), tiles.iter().map(|p| p.y).max().unwrap_or(0));
        let z = tiles[0].z;
        let along_x = x1 - x0 >= y1 - y0;
        let (len, width) = if along_x { (x1 - x0, y1 - y0 + 1) } else { (y1 - y0, x1 - x0 + 1) };
        let len = len.max(1);
        let k = (into / MARCH_PACE) as i32 % (2 * len);
        let (off, dir) = if k <= len { (k, 1) } else { (2 * len - k, -1) };
        let n = floor.len() as i32;
        let start = (width - n.min(width)) / 2;
        let mut taken: HashSet<Pos> = HashSet::new();
        for (r, &i) in floor.iter().enumerate() {
            let (rank, file) = (r as i32 / width, r as i32 % width);
            let along = (off - dir * rank).clamp(0, len);
            let want = if along_x { Pos::new(x0 + along, y0 + start + file, z) } else { Pos::new(x0 + start + file, y0 + along, z) };
            let at = if set.contains(&want) && !taken.contains(&want) {
                want
            } else {
                tiles.iter().copied().filter(|p| !taken.contains(p)).min_by_key(|&p| dist2(p, want)).unwrap_or(self.dwarves[i].pos)
            };
            taken.insert(at);
            plan.push((i, at, Role::March));
        }
    }

    /// Something for marksdwarves to shoot at: an archery target in or
    /// beside the barracks, or else the far wall down its longest line.
    fn practice_target(&self, tiles: &[Pos], set: &HashSet<Pos>) -> Option<Pos> {
        let z = tiles[0].z;
        let butt = self
            .buildings
            .values()
            .filter(|b| b.kind == BuildingKind::ArcheryTarget && b.built && b.pos.z == z)
            .map(|b| b.pos)
            .find(|&p| tiles.iter().any(|&t| gap(t, p) <= 2));
        if butt.is_some() {
            return butt;
        }
        let (sx, sy) = tiles.iter().fold((0, 0), |a, p| (a.0 + p.x, a.1 + p.y));
        let n = tiles.len() as i32;
        let mid = Pos::new(sx / n, sy / n, z);
        let centre = *tiles.iter().min_by_key(|&&p| dist2(p, mid))?;
        let mut best: Option<(i32, Pos)> = None;
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let mut p = centre;
            let mut run = 0;
            while set.contains(&p.offset(dx, dy, 0)) && run < 12 {
                p = p.offset(dx, dy, 0);
                run += 1;
            }
            let wall = p.offset(dx, dy, 0);
            if self.world.solid(wall) && run >= 2 && best.map_or(true, |b| run > b.0) {
                best = Some((run, wall));
            }
        }
        best.map(|b| b.1)
    }

    /// The drill itself: blows, shots and shown moves.
    fn run_drill(&mut self, s: usize, floor: &[usize]) {
        let tick = self.tick;
        let sid = self.squads[s].id;
        // Once each pass: due when the salted half-tick comes round.
        let due = |every: u64, salt: u64| (tick / 2 + salt) % (every / 2) == 0;
        let roles: Vec<(usize, Pos, Role)> = floor
            .iter()
            .filter_map(|&i| self.drills.spots.get(&self.dwarves[i].id).map(|&(p, r)| (i, p, r)))
            .collect();
        let index = |id: u32| floor.iter().copied().find(|&j| self.dwarves[j].id == id);
        let mut demo: Option<(usize, Vec<usize>)> = None;
        let mut pairs: Vec<(usize, usize, bool)> = Vec::new();
        let mut shots: Vec<(usize, Pos)> = Vec::new();
        let mut shadows: Vec<usize> = Vec::new();
        for &(i, spot, role) in &roles {
            let d = &self.dwarves[i];
            match role {
                Role::Spar(p) | Role::Wrestle(p) if d.id < p => {
                    if let Some(j) = index(p).filter(|&j| adjacent(self.dwarves[j].pos, d.pos)) {
                        if due(EXCHANGE, d.id as u64) {
                            pairs.push((i, j, matches!(role, Role::Wrestle(_))));
                        }
                    }
                }
                Role::Shoot(t) if d.pos == spot && due(SHOT_EVERY, d.id as u64) => shots.push((i, t)),
                Role::Shadow if due(EXCHANGE, d.id as u64) => shadows.push(i),
                Role::Demonstrate if d.pos == spot && due(SHOW_EVERY, sid as u64) => {
                    let lid = d.id;
                    let watching: Vec<usize> = roles
                        .iter()
                        .filter(|r| r.2 == Role::Watch(lid) && gap(self.dwarves[r.0].pos, d.pos) <= 3 && self.dwarves[r.0].pos.z == d.pos.z)
                        .map(|r| r.0)
                        .collect();
                    demo = Some((i, watching));
                }
                _ => {}
            }
        }
        for (i, j, wrestle) in pairs {
            self.exchange(i, j, wrestle);
        }
        for (i, t) in shots {
            self.practice_shot(i, t);
        }
        for i in shadows {
            let p = self.dwarves[i].pos;
            let (dx, dy) = AROUND[self.rng.range(0, 8) as usize];
            self.mark(p.offset(dx, dy, 0), line_glyph(dx, dy), (200, 200, 215), tick, 8);
            self.dwarves[i].satisfy(dwarf::NeedKind::Fight, 2);
        }
        if let Some((lead, watching)) = demo {
            if !watching.is_empty() {
                self.demonstrate(s, lead, &watching);
            }
        }
    }

    /// One blow between sparring (or wrestling) partners: struck, parried
    /// or dodged, and pulled.
    fn exchange(&mut self, i: usize, j: usize, wrestle: bool) {
        let tick = self.tick;
        let (iid, jid) = (self.dwarves[i].id, self.dwarves[j].id);
        let k = match self.drills.bouts.iter().position(|b| b.a == iid && b.b == jid) {
            Some(k) => k,
            None => {
                self.drills.bouts.retain(|b| b.a != iid && b.b != iid && b.a != jid && b.b != jid);
                self.drills.bouts.push(Bout { a: iid, b: jid, blows: 0, points: (0, 0) });
                self.drills.bouts.len() - 1
            }
        };
        let swap = self.drills.bouts[k].blows % 2 == 1;
        let (att, def) = if swap { (j, i) } else { (i, j) };
        let (ap, dp) = (self.dwarves[att].pos, self.dwarves[def].pos);
        let a = self.dwarves[att].fight_level() as i32 * 3 + 4 + self.rng.range(0, 10);
        let d = self.dwarves[def].fight_level() as i32 * 3 + self.rng.range(0, 10);
        let hit = a > d + 2;
        let parried = !hit && self.rng.chance(0.5);
        // The swing, then what came of it.
        if !wrestle {
            self.mark(ap, line_glyph(dp.x - ap.x, dp.y - ap.y), (235, 235, 245), tick, 10);
        }
        let (glyph, rgb) = match (hit, parried, wrestle) {
            (true, _, false) => ('*', (255, 235, 120)),
            (true, _, true) => ('&', (255, 200, 140)),
            (false, true, _) => ('×', (170, 200, 255)),
            (false, false, _) => ('~', (200, 230, 255)),
        };
        self.mark(dp, glyph, rgb, tick + 4, 12);
        self.blow_sound(hit && !wrestle, parried, dp);
        self.count("drill.blows", 1);
        // Both learn from every blow; the one who comes off better a little more.
        self.dwarves[att].fight_xp += 1 + hit as u32;
        self.dwarves[def].fight_xp += 1 + !hit as u32;
        self.dwarves[att].satisfy(dwarf::NeedKind::Fight, 4);
        self.dwarves[def].satisfy(dwarf::NeedKind::Fight, 4);
        let point = if hit { 2 } else { 1 };
        {
            let b = &mut self.drills.bouts[k];
            let scorer_is_a = (att == i) == hit;
            if scorer_is_a {
                b.points.0 += point;
            } else {
                b.points.1 += point;
            }
            b.blows += 1;
        }
        if hit {
            self.pulled_blow(att, def, wrestle);
        }
        if self.drills.bouts[k].blows >= BOUT {
            let b = self.drills.bouts.remove(k);
            self.settle_bout(i, j, b.points, wrestle);
        }
    }

    /// Practice blows are pulled, but not always far enough: a bruise or a
    /// nick now and then (more between hot heads), and very rarely a real
    /// hurt.
    fn pulled_blow(&mut self, att: usize, def: usize, wrestle: bool) {
        use crate::body::Part;
        let heat = (self.dwarves[att].personality.temper + self.dwarves[def].personality.temper) as f64 / 100.0;
        let care = (1.0 + heat).max(0.2);
        let (ap, dp) = (self.dwarves[att].pos, self.dwarves[def].pos);
        if self.rng.chance(0.0001 * care * self.knob("drill.accident", 1.0)) {
            let part = *self.rng.pick(&[Part::LeftArm, Part::RightArm, Part::LeftLeg, Part::RightLeg]);
            let pi = part.idx();
            let dmg = self.dwarves[def].body.max[pi] * 2 / 3;
            self.dwarves[def].body.wound(part, dmg, false);
            let (an, dn) = (self.dwarves[att].called(), self.dwarves[def].called());
            let aid = self.dwarves[att].id;
            let tick = self.tick;
            self.dwarves[def].adjust_regard(aid, -10);
            self.dwarves[def].think(&format!("was hurt in the sparring ring by {}", an), 15, tick);
            self.dwarves[att].think("hurt a squadmate in practice", 10, tick);
            let what = if wrestle { "threw" } else { "struck" };
            let text = format!("Sparring accident! {} {} {} too hard: {}'s {} is badly hurt.", an, what, dn, dn, part.name());
            self.msg(&text, (255, 150, 90));
            self.gossip(text);
            self.count("drill.accidents", 1);
        } else if self.rng.chance(0.015 * care) {
            let part = *self.rng.pick(&[Part::UpperBody, Part::LowerBody, Part::LeftArm, Part::RightArm, Part::LeftLeg, Part::RightLeg]);
            self.dwarves[def].body.wound(part, 1, false);
            // An edge that wasn't quite turned leaves a drop or two on the floor.
            let edged = self.dwarves[att].held.and_then(|id| self.items.get(&id)).map_or(false, |it| matches!(it.kind, ItemKind::Axe | ItemKind::Sword | ItemKind::Spear));
            if edged && !wrestle {
                self.spatter(dp, ap, None, 1);
            }
            let tick = self.tick;
            self.dwarves[def].think(if edged && !wrestle { "was nicked in the sparring ring" } else { "was bruised in the sparring ring" }, 2, tick);
            self.count("drill.bruises", 1);
        }
    }

    /// A bout is over: the better of the two has the bout, and the ring
    /// remembers.
    fn settle_bout(&mut self, i: usize, j: usize, points: (i32, i32), wrestle: bool) {
        let tick = self.tick;
        let (iid, jid) = (self.dwarves[i].id, self.dwarves[j].id);
        self.count("drill.bouts", 1);
        if points.0 == points.1 {
            self.dwarves[i].adjust_regard(jid, 1);
            self.dwarves[j].adjust_regard(iid, 1);
            return;
        }
        let (w, l) = if points.0 > points.1 { (i, j) } else { (j, i) };
        let (wid, lid) = (self.dwarves[w].id, self.dwarves[l].id);
        let (lo, hi) = (wid.min(lid), wid.max(lid));
        let r = match self.drills.rivals.iter().position(|r| r.a == lo && r.b == hi) {
            Some(r) => r,
            None => {
                if self.drills.rivals.len() >= MAX_RIVALS {
                    self.drills.rivals.remove(0);
                }
                self.drills.rivals.push(Rivalry { a: lo, b: hi, last: wid, run: 0, bouts: 0 });
                self.drills.rivals.len() - 1
            }
        };
        let run = {
            let rv = &mut self.drills.rivals[r];
            rv.run = if rv.last == wid { rv.run.saturating_add(1) } else { 1 };
            rv.last = wid;
            rv.bouts += 1;
            rv.run
        };
        // Sparring partners grow close, unless one keeps losing and has a temper.
        self.dwarves[w].adjust_regard(lid, 2);
        if run >= 3 && self.dwarves[l].personality.temper > 25 {
            self.dwarves[l].adjust_regard(wid, -6);
        } else {
            self.dwarves[l].adjust_regard(wid, 2);
        }
        let (wn, ln) = (self.dwarves[w].called(), self.dwarves[l].called());
        let ring = if wrestle { "on the wrestling mat" } else { "in the sparring ring" };
        if run == 3 || run == 10 {
            let times = if run == 3 { "three times" } else { "ten times" };
            let text = format!("{} bested {} {} running {}.", wn, ln, times, ring);
            self.dwarves[w].think(&format!("bested {} {}", ln, ring), -6, tick);
            let sting = if self.dwarves[l].personality.temper > 25 { 8 } else { 3 };
            self.dwarves[l].think(&format!("was bested again and again by {}", wn), sting, tick);
            self.gossip(text.clone());
            self.now_and_then(&format!("ring{}", wid), &text, (190, 210, 170), DAY * 7);
        }
        self.dwarves[w].spars_won += 1;
        if self.dwarves[w].spars_won == RING_WINS {
            let mut d = std::mem::take(&mut self.dwarves[w]);
            self.earn_epithet(&mut d, "Ringmaster", super::heroes::RANK_FEAT, "won twenty-five sparring bouts");
            self.dwarves[w] = d;
        }
    }

    /// A marksdwarf looses a practice bolt at the target. The bolts are
    /// blunt practice bolts, gathered up again after: the quiver isn't touched.
    fn practice_shot(&mut self, i: usize, target: Pos) {
        let from = self.dwarves[i].pos;
        if !self.clear_shot(from, target) {
            return;
        }
        let lvl = self.dwarves[i].fight_level() as f64;
        let hit = self.rng.chance((0.45 + 0.05 * lvl).min(0.95));
        let to = if hit {
            target
        } else {
            let (dx, dy) = AROUND[self.rng.range(0, 8) as usize];
            target.offset(dx, dy, 0)
        };
        self.loose_bolt(from, to);
        let lands = self.tick + 2 * gap(from, to) as u64;
        if hit {
            self.mark(to, '*', (255, 240, 150), lands, 10);
        } else {
            self.mark(to, '\'', (190, 160, 110), lands, 14);
        }
        self.dwarves[i].fight_xp += 1 + hit as u32;
        self.dwarves[i].satisfy(dwarf::NeedKind::Fight, 4);
        self.count("drill.shots", 1);
    }

    /// The leader shows a move: the blade sweeps round them, and those
    /// watching who aren't as good learn from it.
    fn demonstrate(&mut self, s: usize, lead: usize, watching: &[usize]) {
        let tick = self.tick;
        let p = self.dwarves[lead].pos;
        for (k, &(dx, dy)) in AROUND.iter().enumerate() {
            self.mark(p.offset(dx, dy, 0), line_glyph(dx, dy), (235, 235, 245), tick + 2 * k as u64, 4);
        }
        let lvl = self.dwarves[lead].fight_level();
        for &w in watching {
            let gain = if self.dwarves[w].fight_level() < lvl { WATCH_XP } else { 1 };
            self.dwarves[w].fight_xp += gain;
            self.dwarves[w].satisfy(dwarf::NeedKind::Learn, 20);
            self.count("drill.watch_xp", gain);
        }
        self.dwarves[lead].fight_xp += 1;
        self.dwarves[lead].satisfy(dwarf::NeedKind::Fight, 6);
        self.count("drill.demos", 1);
        let kind = self.dwarves[lead].held.and_then(|id| self.items.get(&id)).map(|it| it.kind);
        let move_name = match kind {
            Some(ItemKind::Axe) => "a hooking axe cut",
            Some(ItemKind::Sword) => "a low feint and thrust",
            Some(ItemKind::Spear) => "a long lunge with the spear",
            Some(ItemKind::Hammer) => "an overhead hammer blow",
            Some(ItemKind::Crossbow) => "how to span a crossbow on the run",
            Some(ItemKind::Pick) => "how to fight with a pick",
            _ => "a wrestling throw",
        };
        let text = format!("{} shows {} {}.", self.dwarves[lead].called(), self.squads[s].name, move_name);
        let key = format!("demo{}", self.squads[s].id);
        self.now_and_then(&key, &text, (170, 200, 230), DAY * 7);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::Mat;

    /// A walled barracks of `w` by `h` with soldiers of one squad in it, at
    /// drill. Returns the squad and the barracks' corner.
    fn barracks(seed: u64, w: i32, h: i32, soldiers: usize) -> (Game, usize, Pos) {
        let mut g = Game::new(seed);
        g.creatures.retain(|_, c| c.tame);
        g.alert = false;
        let a = g.open_patch(w + 2, h + 2, 6).offset(1, 1, 0);
        // Walls round it, so the drills have a room.
        for x in -1..=w {
            for y in -1..=h {
                let p = a.offset(x, y, 0);
                let t = g.world.t_mut(p);
                if x == -1 || y == -1 || x == w || y == h {
                    t.kind = TileKind::Wall;
                } else {
                    t.revealed = true;
                }
            }
        }
        g.add_zone(a, a.offset(w - 1, h - 1, 0), ZoneKind::Barracks);
        let s = g.new_squad();
        g.squads[s].armor = false;
        let ids: Vec<u32> = g.dwarves.iter().take(soldiers).map(|d| d.id).collect();
        for (k, &id) in ids.iter().enumerate() {
            g.set_squad_member(s, id, true);
            let i = g.dwarves.iter().position(|d| d.id == id).unwrap();
            let d = &mut g.dwarves[i];
            d.pos = a.offset(k as i32 % w, k as i32 / w, 0);
            d.task = Task::Train;
            d.hunger = 0;
            d.thirst = 0;
            d.fatigue = 0;
        }
        // The rest stay out of the way.
        g.dwarves.retain(|d| ids.contains(&d.id));
        (g, s, a)
    }

    /// Keep them fed, watered and rested, so only the drill goes on.
    fn keep_fresh(g: &mut Game) {
        for d in g.dwarves.iter_mut() {
            d.hunger = 0;
            d.thirst = 0;
            d.fatigue = 0;
        }
    }

    fn ctr(g: &Game, k: &str) -> u32 {
        g.annals.counters.get(k).copied().unwrap_or(0)
    }

    /// Two soldiers pair off, stand next to each other and trade blows for
    /// a day of sparring and wrestling, and nobody is badly hurt.
    #[test]
    fn a_sparring_pair_trades_blows_without_harm() {
        let (mut g, s, _) = barracks(91, 8, 5, 2);
        let sid = g.squads[s].id;
        // Start at the top of a sparring drill.
        while g.drill_now(sid) != Drill::Spar || g.tick % DRILL_SPAN != 0 {
            g.tick += 1;
        }
        let mut paired = false;
        let mut seen_fx = false;
        for _ in 0..DAY {
            g.step();
            keep_fresh(&mut g);
            let (a, b) = (&g.dwarves[0], &g.dwarves[1]);
            if matches!(g.drill_role(a.id), Some(Role::Spar(p)) if p == b.id) && adjacent(a.pos, b.pos) {
                paired = true;
            }
            seen_fx |= !g.drill_fx(a.pos.z).is_empty();
        }
        let blows = ctr(&g, "drill.blows");
        eprintln!("blows {} bouts {} bruises {} accidents {}", blows, ctr(&g, "drill.bouts"), ctr(&g, "drill.bruises"), ctr(&g, "drill.accidents"));
        assert!(paired, "they never squared up");
        assert!(blows >= 20, "only {} blows", blows);
        assert!(ctr(&g, "drill.bouts") >= 3);
        assert!(seen_fx, "nothing to see on the map");
        for d in &g.dwarves {
            assert!(d.alive && !d.body.unconscious() && d.body.bleeding == 0, "{} was hurt: {:?}", d.name, d.body.hp);
        }
    }

    /// Blows are pulled: in many thousands, bruises are uncommon and real
    /// hurts rarer still, and no blow is ever deadly.
    #[test]
    fn pulled_blows_rarely_hurt() {
        let (mut g, _, _) = barracks(92, 6, 4, 2);
        for _ in 0..20_000 {
            g.pulled_blow(0, 1, false);
            g.dwarves[1].body = g.dwarves[0].body.clone();
        }
        let (bruises, bad) = (ctr(&g, "drill.bruises"), ctr(&g, "drill.accidents"));
        eprintln!("bruises {} accidents {}", bruises, bad);
        assert!(bruises > 0 && bruises < 1000, "{} bruises", bruises);
        assert!(bad < 12, "{} accidents", bad);
        assert!(g.dwarves[1].alive);
        // Made common, a real hurt is announced, and still not deadly.
        g.knobs.set.insert("drill.accident".into(), 2000.0);
        for _ in 0..50 {
            g.pulled_blow(0, 1, false);
            assert!(g.dwarves[1].alive && !g.dwarves[1].body.unconscious());
            g.dwarves[1].body = g.dwarves[0].body.clone();
        }
        assert!(g.log.iter().any(|l| l.text.starts_with("Sparring accident!")), "an accident goes unannounced");
    }

    /// A marksdwarf at drill takes a place in range of the archery target
    /// and shoots at it; the bolts fly, and the quiver isn't emptied.
    #[test]
    fn marksdwarves_practise_at_the_target() {
        let (mut g, s, a) = barracks(93, 10, 5, 1);
        let sid = g.squads[s].id;
        g.squads[s].weapon = super::super::military::UniformWeapon::Crossbow;
        let id = g.dwarves[0].id;
        let bow = g.spawn_item(ItemKind::Crossbow, Mat::Wood, a);
        g.items.get_mut(&bow).unwrap().carried_by = Some(id);
        g.dwarves[0].held = Some(bow);
        g.dwarves[0].ammo = 20;
        // A target against the far wall.
        let t = a.offset(9, 2, 0);
        g.world.t_mut(t).zone = 0;
        assert!(g.place_building(BuildingKind::ArcheryTarget, t));
        let bid = g.world.t(t).building;
        g.buildings.get_mut(&bid).unwrap().built = true;
        g.zones.values_mut().for_each(|z| z.tiles.retain(|&p| p != t));
        while g.drill_now(sid) != Drill::Spar || g.tick % DRILL_SPAN != 0 {
            g.tick += 1;
        }
        let mut flew = false;
        for _ in 0..DRILL_SPAN - 20 {
            g.step();
            keep_fresh(&mut g);
            flew |= !g.drills.flights.is_empty() && g.drill_fx(t.z).iter().any(|f| matches!(f.1, '-' | '|' | '/' | '\\'));
        }
        let shots = ctr(&g, "drill.shots");
        assert!(matches!(g.drill_role(id), Some(Role::Shoot(p)) if p == t), "{:?}", g.drill_role(id));
        assert!(shots >= 3, "only {} practice shots", shots);
        assert!(flew, "no bolt was seen in flight");
        assert_eq!(g.dwarves[0].ammo, 20, "practice bolts are gathered up again");
    }

    /// The best fighter shows the squad a move; those watching learn from
    /// it, and the fort hears of it.
    #[test]
    fn a_demonstration_teaches_the_watchers() {
        let (mut g, s, _) = barracks(94, 9, 7, 4);
        let sid = g.squads[s].id;
        g.dwarves[0].fight_xp = 2400;
        for d in g.dwarves.iter_mut().skip(1) {
            d.fight_xp = 0;
        }
        while g.drill_now(sid) != Drill::Demonstrate || g.tick % DRILL_SPAN != 0 {
            g.tick += 1;
        }
        for _ in 0..DRILL_SPAN - 10 {
            g.step();
            keep_fresh(&mut g);
        }
        let lead = g.dwarves[0].id;
        assert_eq!(g.drill_role(lead), Some(Role::Demonstrate));
        assert!(g.dwarves[1..].iter().all(|d| g.drill_role(d.id) == Some(Role::Watch(lead))));
        assert!(ctr(&g, "drill.demos") >= 2, "{} demonstrations", ctr(&g, "drill.demos"));
        assert!(ctr(&g, "drill.watch_xp") >= 3 * WATCH_XP * 2, "watchers learned only {}", ctr(&g, "drill.watch_xp"));
        assert!(g.log.iter().any(|l| l.text.contains(&format!("shows {}", g.squads[s].name))), "no word of the demonstration");
    }

    /// The squad marches in a line across the barracks and back, then
    /// breaks ranks.
    #[test]
    fn the_squad_marches_in_step() {
        let (mut g, s, a) = barracks(95, 10, 4, 3);
        let sid = g.squads[s].id;
        while g.drill_now(sid) != Drill::March || g.tick % DRILL_SPAN != 0 {
            g.tick += 1;
        }
        let mut xs = HashSet::new();
        let mut in_line = 0;
        for k in 0..DRILL_SPAN {
            g.step();
            keep_fresh(&mut g);
            if k % 10 == 0 {
                let x: Vec<i32> = g.dwarves.iter().map(|d| d.pos.x).collect();
                if x.iter().all(|&v| v == x[0]) {
                    in_line += 1;
                    xs.insert(x[0] - a.x);
                }
            }
        }
        assert!(in_line >= 10, "in line only {} times", in_line);
        assert!(xs.len() >= 6, "the line didn't cross the room: {:?}", xs);
        assert!(g.dwarves.iter().all(|d| g.drill_role(d.id) == Some(Role::AtEase)), "they never broke ranks");
    }

    /// A partner who keeps winning is talked about, and the ring makes friends.
    #[test]
    fn a_winning_run_is_gossip() {
        let (mut g, _, _) = barracks(96, 6, 4, 2);
        let (aid, bid) = (g.dwarves[0].id, g.dwarves[1].id);
        for _ in 0..3 {
            g.settle_bout(0, 1, (6, 2), false);
        }
        assert!(g.gossip.iter().any(|(_, t)| t.contains("three times running in the sparring ring")), "{:?}", g.gossip);
        assert_eq!(g.dwarves[0].spars_won, 3);
        assert!(g.dwarves[0].regard(bid) > 0);
        let _ = aid;
    }
}
