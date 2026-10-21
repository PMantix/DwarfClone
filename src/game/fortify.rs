//! The fort's outside and its grand works. As a fortress grows (its people,
//! its wealth, the armies it has seen, and its overseer's temper), the
//! overseer raises:
//!
//! 1. a curtain wall round the stair and the fields, with two gate towers
//!    flanking an open gate, then battlements along its top, a walkway
//!    behind them, an arch over the gate, and at last a drawbridge in the
//!    gate worked from a lever in a gate tower;
//! 2. watchtowers at its corners (see `towers.rs` for the watch, the
//!    stations and shooting down from them);
//! 3. a wizard's tower with a laboratory at the top, for a vain, pious or
//!    greedy overseer or a very rich fort;
//! 4. grand works: a pillared portico over the stair and pillars round the
//!    monument.
//!
//! Everything is built through ordinary construction jobs, from stone the
//! fort has, by its builders, a stretch or a storey at a time. Every work has
//! a name ("the North-West Watchtower") that the look panel and the director
//! use. The gate of the curtain wall is never built over, so an army always
//! has a way to it (and the drawbridge there only rises with the gate's own).

use super::overseer::{Overseer, Style};
use super::personality::Trait;
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkKind {
    Curtain,
    GateTower,
    Watchtower,
    WizardTower,
    Portico,
    Pillars,
    Bridge,
}

impl WorkKind {
    pub fn is_tower(self) -> bool {
        matches!(self, WorkKind::GateTower | WorkKind::Watchtower | WorkKind::WizardTower)
    }
}

/// One work: what it is, its name, where, and the constructions it is made
/// of in the order they go up (each step waits for the one before).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Work {
    pub kind: WorkKind,
    pub name: String,
    /// A tower's centre at the foot (the curtain: the middle of its gate).
    pub at: Pos,
    /// Storeys above the ground (towers).
    #[serde(default)]
    pub height: i32,
    /// A tower's doorway at the foot.
    #[serde(default)]
    pub door: Option<Pos>,
    /// Which way is out (a tower's lookout side, the curtain's outside).
    #[serde(default)]
    pub outward: (i32, i32),
    pub steps: Vec<Vec<(Pos, ConstructKind)>>,
    pub step: usize,
    /// Steps from here on wait until the fort wants battlements (the curtain).
    #[serde(default)]
    pub hold: usize,
    /// Tiles done at the last look, and since when that hasn't changed.
    #[serde(default)]
    pub mark: usize,
    #[serde(default)]
    pub since: u64,
    pub begun: u64,
    /// When it was finished (0: not yet).
    #[serde(default)]
    pub finished: u64,
    /// Tiles given up on (no ground for them).
    #[serde(default)]
    pub skipped: u32,
}

impl Work {
    pub fn done(&self) -> bool {
        self.step >= self.steps.len()
    }

    /// Whether `p` is part of it.
    pub fn covers(&self, p: Pos) -> bool {
        if self.kind.is_tower() {
            let roof = if self.kind == WorkKind::WizardTower { 1 } else { 0 };
            return (p.x - self.at.x).abs() <= 2 && (p.y - self.at.y).abs() <= 2 && p.z >= self.at.z && p.z <= self.at.z + self.height + roof;
        }
        self.steps.iter().flatten().any(|&(q, _)| q == p)
    }

    /// The top platform's middle (towers).
    pub fn top(&self) -> Pos {
        self.at.offset(0, 0, self.height)
    }

    /// How far along: tiles of the finished steps over all tiles.
    pub fn progress(&self) -> f64 {
        let all: usize = self.steps.iter().map(|s| s.len()).sum();
        let done: usize = self.steps.iter().take(self.step).map(|s| s.len()).sum();
        done as f64 / all.max(1) as f64
    }
}

/// Everything built outside, and what the watch is doing.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Works {
    pub list: Vec<Work>,
    /// The curtain wall has been laid out (or found impossible).
    #[serde(default)]
    pub laid: bool,
    /// The curtain's gate (the middle of the opening), which way runs along
    /// the wall there, and which way is in.
    #[serde(default)]
    pub gate: Option<Pos>,
    #[serde(default)]
    pub along: (i32, i32),
    #[serde(default)]
    pub inward: (i32, i32),
    /// Corners of the curtain where a watchtower can stand (not yet begun).
    #[serde(default)]
    pub corners: Vec<(Pos, (i32, i32), String)>,
    /// The soldier keeping watch from a watchtower in peacetime.
    #[serde(default)]
    pub watchman: Option<u32>,
    /// When someone last stood on a watchtower's top.
    #[serde(default)]
    pub watched_at: u64,
    /// The laboratory's zone (0: none yet), and when it was last studied in.
    #[serde(default)]
    pub lab: u32,
    #[serde(default)]
    pub studied: u64,
    /// The curtain's drawbridge: the bridges and the lever.
    #[serde(default)]
    pub bridges: Vec<Pos>,
    #[serde(default)]
    pub lever: Option<Pos>,
}

/// Ring and inner tiles of a 5×5 tower, as offsets from its centre.
pub(super) fn ring() -> Vec<(i32, i32)> {
    let mut v = Vec::new();
    for dy in -2..=2i32 {
        for dx in -2..=2i32 {
            if dx.abs().max(dy.abs()) == 2 {
                v.push((dx, dy));
            }
        }
    }
    v
}

pub(super) fn inner() -> Vec<(i32, i32)> {
    let mut v = Vec::new();
    for dy in -1..=1i32 {
        for dx in -1..=1i32 {
            if (dx, dy) != (0, 0) {
                v.push((dx, dy));
            }
        }
    }
    v
}

/// What crowns a tower.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Crown {
    /// A platform behind fortifications, to watch and shoot from.
    Battlements,
    /// A walled room with windows and a roof: the laboratory.
    Laboratory,
}

/// The steps a 5×5 tower goes up in, centred on `c` (on the ground), `h`
/// storeys tall: the foot (walls round, the doorway left open, an up stair
/// in the middle), then each storey (its walls and stair, then its floor,
/// which needs them to rest on). `openings` are wall tiles of the first
/// storey left as floor, onto a wall's walkway or the arch over a gate.
pub(super) fn tower_steps(c: Pos, h: i32, door: (i32, i32), openings: &[(i32, i32)], crown: Crown) -> Vec<Vec<(Pos, ConstructKind)>> {
    let at = |(dx, dy): (i32, i32), k: i32| c.offset(dx, dy, k);
    let mut steps = Vec::new();
    let mut foot: Vec<(Pos, ConstructKind)> = ring().into_iter().filter(|&o| o != door).map(|o| (at(o, 0), ConstructKind::Wall)).collect();
    foot.push((c, ConstructKind::StairUp));
    steps.push(foot);
    for k in 1..=h {
        let top = k == h;
        let mut walls = Vec::new();
        for o in ring() {
            let kind = if k == 1 && openings.contains(&o) {
                ConstructKind::Floor
            } else if top {
                match crown {
                    Crown::Battlements => ConstructKind::Fortification,
                    // Windows in the middle of each side.
                    Crown::Laboratory if o.0 == 0 || o.1 == 0 => ConstructKind::Fortification,
                    Crown::Laboratory => ConstructKind::Wall,
                }
            } else {
                ConstructKind::Wall
            };
            walls.push((at(o, k), kind));
        }
        walls.push((at((0, 0), k), if top { ConstructKind::StairDown } else { ConstructKind::StairUD }));
        steps.push(walls);
        steps.push(inner().into_iter().map(|o| (at(o, k), ConstructKind::Floor)).collect());
    }
    if crown == Crown::Laboratory {
        // A parapet round the roof, then the roof over the room.
        steps.push(ring().into_iter().map(|o| (at(o, h + 1), ConstructKind::Fortification)).collect());
        let mut roof: Vec<(Pos, ConstructKind)> = inner().into_iter().map(|o| (at(o, h + 1), ConstructKind::Floor)).collect();
        roof.push((at((0, 0), h + 1), ConstructKind::Floor));
        steps.push(roof);
    }
    steps
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

fn cheb(a: Pos, b: Pos) -> i32 {
    (a.x - b.x).abs().max((a.y - b.y).abs())
}

/// How many constructions the works may have waiting at once.
const MAX_PENDING: usize = 40;
/// Stone kept back for everything else.
const STONE_RESERVE: usize = 12;

impl Game {
    /// Daily, from the overseer's round: lay out what the fort is ready
    /// for, and build what has been laid out.
    pub(super) fn ov_fortify(&mut self, o: &mut Overseer) {
        if o.top.is_none() || o.base.is_none() {
            return;
        }
        let tick = self.tick;
        if !self.works.laid && self.curtain_due(o) {
            self.lay_out_curtain(o);
        }
        self.more_watchtowers(o);
        if self.wizard_due(o) {
            self.raise_wizard_tower(o);
        }
        self.grand_works(o);
        if self.works.list.is_empty() {
            return;
        }
        self.works_builders(o);
        // Build: each unfinished work moves on (none while an army is about,
        // the builders sheltering; nor while food or drink run short, when
        // every hand is wanted for those).
        if !self.alert && self.works_can_wait() {
            for i in 0..self.works.list.len() {
                if !self.works.list[i].done() {
                    self.advance_work(o, i);
                }
            }
        }
        self.hang_tower_doors();
        self.curtain_bridge(o);
        self.laboratory(o);
        self.pick_watchman(o);
        let _ = tick;
    }

    /// Whether the fort has food and drink enough to spare hands for its walls.
    fn works_can_wait(&self) -> bool {
        let pop = self.alive_count().max(1) as f64;
        let drink = self.stock_of(ItemKind::Drink) as f64 / pop;
        let food = [ItemKind::Meal, ItemKind::Plant, ItemKind::Meat, ItemKind::Fish].iter().map(|&k| self.stock_of(k)).sum::<usize>() as f64 / pop;
        drink >= 3.0 && food >= 3.0
    }

    /// Whether the fort is ready for a curtain wall: grown, with its gate
    /// finished, and rich enough or attacked already. Warlords, the
    /// cautious, an overseer set on fortifying or one a siege taught to wall
    /// the yard start sooner.
    fn curtain_due(&self, o: &Overseer) -> bool {
        if o.style == Style::Reckless || !o.did("linked") {
            return false;
        }
        let keen = o.style == Style::Warlord
            || o.has(Trait::Cautious)
            || o.ambition == Some(super::ambitions::Ambition::Fortify)
            || o.lessons.iter().any(|l| l.1 == super::lessons::Lesson::WallTheYard);
        let (day, pop) = if keen { (self.knob("works.keen_day", 160.0), 12) } else { (self.knob("works.curtain_day", 300.0), 16) };
        (self.tick / DAY) as f64 >= day && self.alive_count() >= pop && (self.wealth() >= 10_000 || !self.annals.sieges.is_empty())
    }

    /// Whether the curtain is ready for battlements and its walkway: once an
    /// army has come, or the fort is big or rich.
    fn battlements_due(&self) -> bool {
        !self.annals.sieges.is_empty() || self.alive_count() >= 20 || self.wealth() >= 30_000
    }

    /// A 5×5 tower `h` storeys tall can stand centred on (x, y): level
    /// ground, nothing on it (trees will be felled), open sky above, away
    /// from the stair. Returns the centre on the ground.
    pub(super) fn tower_site(&self, x: i32, y: i32, h: i32, avoid: Pos) -> Option<Pos> {
        let w = &self.world;
        if x < 4 || y < 4 || x >= w.w - 4 || y >= w.h - 4 {
            return None;
        }
        let z = w.surface_z(x, y);
        if z + h + 3 >= w.d {
            return None;
        }
        if (avoid.x - x).abs() <= 3 && (avoid.y - y).abs() <= 3 {
            return None;
        }
        for dy in -2..=2i32 {
            for dx in -2..=2 {
                let p = Pos::new(x + dx, y + dy, z);
                if w.surface_z(p.x, p.y) != z {
                    return None;
                }
                let t = w.t(p);
                if !matches!(t.kind, TileKind::Floor | TileKind::Shrub | TileKind::Tree)
                    || t.water > 0
                    || t.zone != 0
                    || t.building != 0
                    || !matches!(t.desig, Desig::None | Desig::Chop)
                    || t.built_from.is_some()
                {
                    return None;
                }
                if (1..=h + 2).any(|k| w.kind(p.offset(0, 0, k)) != Some(TileKind::Open)) {
                    return None;
                }
                if self.construction_at(p).is_some() {
                    return None;
                }
            }
        }
        Some(Pos::new(x, y, z))
    }

    /// Fell any trees on a work's ground.
    fn clear_trees(&mut self, tiles: &[Pos]) {
        for &p in tiles {
            if self.world.kind(p) == Some(TileKind::Tree) && self.world.t(p).desig == Desig::None {
                self.designate(p, p, Desig::Chop);
            }
        }
    }

    /// Lay out the curtain wall: a rectangle round the stair, the fields,
    /// the stockpiles and workshops by the wagon, with its gate in the
    /// middle of a side between two gate towers, and the corners kept for
    /// watchtowers.
    fn lay_out_curtain(&mut self, o: &mut Overseer) {
        self.works.laid = true;
        let top = o.top.unwrap();
        let e = self.embark;
        let tick = self.tick;
        // (The stair with room round it for its walled yard.)
        let (mut x0, mut y0, mut x1, mut y1) = ((top.x - 5).min(e.x - 21), (top.y - 5).min(e.y - 19), (top.x + 5).max(e.x + 20), (top.y + 5).max(e.y + 20));
        let mut grow = |p: Pos| {
            x0 = x0.min(p.x);
            y0 = y0.min(p.y);
            x1 = x1.max(p.x);
            y1 = y1.max(p.y);
        };
        for z in self.zones.values() {
            for &p in &z.tiles {
                if cheb(p, e) <= 34 && self.world.outside(p) {
                    grow(p);
                }
            }
        }
        for b in self.buildings.values() {
            if cheb(b.pos, e) <= 34 && self.world.outside(b.pos) {
                for p in b.tiles() {
                    grow(p);
                }
            }
        }
        let m = 3;
        let (w, h) = (self.world.w, self.world.h);
        let (x0, y0, x1, y1) = ((x0 - m).max(6), (y0 - m).max(6), (x1 + m).min(w - 7), (y1 + m).min(h - 7));
        if x1 - x0 < 20 || y1 - y0 < 20 {
            Self::note_ov(o, tick, "Meant to raise a curtain wall round the fields, but there is no room for one.".into());
            return;
        }
        // The sides: (where the gate's middle could go, which way runs along, which way is in, names).
        let mid = ((x0 + x1) / 2, (y0 + y1) / 2);
        let sides = [
            ((mid.0, y1), (1, 0), (0, -1), ["West", "East"]),
            ((x1, mid.1), (0, 1), (-1, 0), ["North", "South"]),
            ((x0, mid.1), (0, 1), (1, 0), ["North", "South"]),
            ((mid.0, y0), (1, 0), (0, 1), ["West", "East"]),
        ];
        let mut found = None;
        'sides: for &(m, along, inward, names) in &sides {
            for k in 0..40 {
                let off = if k % 2 == 0 { k / 2 } else { -(k / 2 + 1) };
                let (gx, gy) = (m.0 + along.0 * off, m.1 + along.1 * off);
                let Some(t1) = self.tower_site(gx - along.0 * 4, gy - along.1 * 4, 2, top) else { continue };
                let Some(t2) = self.tower_site(gx + along.0 * 4, gy + along.1 * 4, 2, top) else { continue };
                let ground = |x: i32, y: i32| Pos::new(x, y, self.world.surface_z(x, y));
                let g = ground(gx, gy);
                if (-1..=1).any(|i| !self.world.walkable(ground(gx + along.0 * i, gy + along.1 * i))) {
                    continue;
                }
                let out = ground(gx - inward.0 * 2, gy - inward.1 * 2);
                let inn = ground(gx + inward.0 * 3, gy + inward.1 * 3);
                if !self.world.walkable(out) || !self.world.walkable(inn) {
                    continue;
                }
                if path::astar(&self.world, out, top, 60_000, Mover::Goblin).is_none() || path::astar(&self.world, inn, top, 60_000, Mover::Dwarf).is_none() {
                    continue;
                }
                found = Some((g, along, inward, names, t1, t2));
                break 'sides;
            }
        }
        let Some((g, along, inward, names, t1, t2)) = found else {
            // No curtain on such broken land: a lone watchtower on the
            // highest ground near the stair instead.
            if let Some(c) = self.lone_tower_site(top) {
                self.works.corners.push((c, (0, 1), "the Watchtower".into()));
            }
            Self::note_ov(o, tick, "Meant to raise a curtain wall, but the land gives its gate no level footing; a watchtower on the high ground will have to do.".into());
            return;
        };
        self.works.gate = Some(g);
        self.works.along = along;
        self.works.inward = inward;
        // The corners, for watchtowers later.
        let corners = [((x0, y0), (1, 1), "North-West"), ((x1, y0), (-1, 1), "North-East"), ((x0, y1), (1, -1), "South-West"), ((x1, y1), (-1, -1), "South-East")];
        let mut keep: Vec<Pos> = vec![t1, t2];
        for &((cx, cy), sgn, name) in &corners {
            if let Some(c) = self.tower_site(cx, cy, 3, top) {
                self.works.corners.push((c, sgn, format!("the {} Watchtower", name)));
                keep.push(c);
            }
        }
        // The wall line: round the rectangle from the gate, leaving the gate
        // and the towers' ground.
        let mut perim: Vec<(i32, i32)> = Vec::new();
        for x in x0..=x1 {
            perim.push((x, y0));
        }
        for y in y0 + 1..=y1 {
            perim.push((x1, y));
        }
        for x in (x0..x1).rev() {
            perim.push((x, y1));
        }
        for y in (y0 + 1..y1).rev() {
            perim.push((x0, y));
        }
        let start = perim.iter().position(|&q| q == (g.x, g.y)).unwrap_or(0);
        perim.rotate_left(start);
        let gate_tiles: Vec<(i32, i32)> = (-1..=1).map(|i| (g.x + along.0 * i, g.y + along.1 * i)).collect();
        let mut line: Vec<(Pos, (i32, i32))> = Vec::new();
        for &(x, y) in &perim {
            if gate_tiles.contains(&(x, y)) || keep.iter().any(|c| (c.x - x).abs() <= 2 && (c.y - y).abs() <= 2) {
                continue;
            }
            let p = Pos::new(x, y, self.world.surface_z(x, y));
            let t = self.world.t(p);
            if t.zone != 0 || t.building != 0 || t.water > 0 || (p.x, p.y) == (top.x, top.y) {
                continue;
            }
            // Which way is in, from this stretch of the wall.
            let inw = if y == y0 {
                (0, 1)
            } else if y == y1 {
                (0, -1)
            } else if x == x0 {
                (1, 0)
            } else {
                (-1, 0)
            };
            line.push((p, inw));
        }
        let trees: Vec<Pos> = line.iter().map(|l| l.0).collect();
        self.clear_trees(&trees);
        let mut steps: Vec<Vec<(Pos, ConstructKind)>> = line.chunks(16).map(|c| c.iter().map(|&(p, _)| (p, ConstructKind::Wall)).collect()).collect();
        let hold = steps.len();
        // Battlements on the wall top, then the walkway behind them.
        steps.extend(line.chunks(24).map(|c| c.iter().map(|&(p, _)| (p.up(), ConstructKind::Fortification)).collect()));
        steps.extend(line.chunks(24).map(|c| c.iter().map(|&(p, i)| (p.offset(i.0, i.1, 1), ConstructKind::Floor)).collect()));
        // The arch over the gate, with its own battlements (when the gate
        // and its towers stand on one level).
        let level = t1.z == g.z && t2.z == g.z && (-1..=1).all(|i| self.world.surface_z(g.x + along.0 * i, g.y + along.1 * i) == g.z);
        let mut arch: Vec<(Pos, ConstructKind)> = (-1..=1).map(|i| (g.offset(along.0 * i, along.1 * i, 1), ConstructKind::Floor)).collect();
        arch.extend((-1..=1).map(|i| (g.offset(along.0 * i - inward.0, along.1 * i - inward.1, 1), ConstructKind::Fortification)));
        if level {
            steps.push(arch);
        }
        // The gate towers: each opens onto the arch and onto the walkway.
        let out = (-inward.0, -inward.1);
        for (c, sign, name) in [(t1, 1, names[0]), (t2, -1, names[1])] {
            let to_gate = (along.0 * 2 * sign, along.1 * 2 * sign);
            let to_walk = (-along.0 * 2 * sign + inward.0, -along.1 * 2 * sign + inward.1);
            let door = (inward.0 * 2, inward.1 * 2);
            let steps = tower_steps(c, 2, door, &[to_gate, to_walk], Crown::Battlements);
            self.clear_trees(&box_iter(c.offset(-2, -2, 0), c.offset(2, 2, 0)).collect::<Vec<_>>());
            self.works.list.push(Work {
                kind: WorkKind::GateTower,
                name: format!("the {} Gate Tower", name),
                at: c,
                height: 2,
                door: Some(c.offset(door.0, door.1, 0)),
                outward: out,
                steps,
                step: 0,
                hold: usize::MAX,
                mark: 0,
                since: tick,
                begun: tick,
                finished: 0,
                skipped: 0,
            });
        }
        let n = line.len();
        self.works.list.push(Work {
            kind: WorkKind::Curtain,
            name: "the Curtain Wall".into(),
            at: g,
            height: 1,
            door: None,
            outward: out,
            steps,
            step: 0,
            hold,
            mark: 0,
            since: tick,
            begun: tick,
            finished: 0,
            skipped: 0,
        });
        let side = match inward {
            (0, -1) => "south",
            (0, 1) => "north",
            (-1, 0) => "east",
            _ => "west",
        };
        let why = if o.has(Trait::Cautious) {
            "Uneasy with the fields lying open to anyone, it"
        } else if o.style == Style::Warlord {
            "Meaning to hold the fields as well as the halls, it"
        } else if o.has(Trait::Vain) {
            "Wanting the fort to look like a fortress from afar, it"
        } else if o.has(Trait::Greedy) {
            "Counting the stone in the stockpiles, it"
        } else {
            "The fort grown, it"
        };
        Self::note_ov(
            o,
            tick,
            format!("{} laid out a curtain wall round the stair and the fields ({} stretches of wall, {} by {}), its gate on the {} side between two gate towers.", why, n, x1 - x0 + 1, y1 - y0 + 1, side),
        );
        self.msg(&format!("The overseer lays out a curtain wall round {}, with gate towers on the {} side.", self.fortress_name.clone(), side), (200, 200, 140));
    }

    /// The highest ground near the stair (8 to 28 tiles out) a lone
    /// watchtower can stand on, its door to the south.
    fn lone_tower_site(&self, top: Pos) -> Option<Pos> {
        let mut best: Option<Pos> = None;
        for r in 8..28i32 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs().max(dy.abs()) != r {
                        continue;
                    }
                    if let Some(c) = self.tower_site(top.x + dx, top.y + dy, 3, top) {
                        if self.works.list.iter().any(|w| (w.at.x - c.x).abs() <= 6 && (w.at.y - c.y).abs() <= 6) {
                            continue;
                        }
                        let door = c.offset(0, 3, 0);
                        if best.map_or(true, |b| c.z > b.z) && self.world.walkable(Pos::new(door.x, door.y, self.world.surface_z(door.x, door.y))) {
                            best = Some(c);
                        }
                    }
                }
            }
        }
        best
    }

    /// Watchtowers at the corners, as the fort grows: the first once the
    /// curtain's walls stand, more with more people and every army.
    fn more_watchtowers(&mut self, o: &mut Overseer) {
        if self.alert || !self.works.laid {
            return;
        }
        // (Once half the walls stand, or a season after they were laid out.)
        let walls_up = self.works.list.iter().any(|w| w.kind == WorkKind::Curtain && (w.step * 2 >= w.hold || self.tick > w.begun + SEASON)) || self.works.gate.is_none();
        if !walls_up {
            return;
        }
        let have = self.works.list.iter().filter(|w| w.kind == WorkKind::Watchtower).count();
        let building = self.works.list.iter().any(|w| w.kind == WorkKind::Watchtower && !w.done());
        let pop = self.alive_count();
        let keen = o.style == Style::Warlord || o.has(Trait::Cautious);
        let want = 1 + (pop >= 22) as usize + (pop >= 30 || (keen && self.annals.sieges.len() >= 2)) as usize + (self.annals.sieges.len() >= 3) as usize;
        if building || have >= want {
            return;
        }
        // The corner on the highest ground first (it sees furthest), of
        // those still clear.
        let top = o.top.unwrap();
        let clear: Vec<usize> = (0..self.works.corners.len()).filter(|&i| {
            let c = self.works.corners[i].0;
            self.tower_site(c.x, c.y, 3, top).is_some()
        }).collect();
        let pick = clear.iter().copied().max_by_key(|&i| (self.works.corners[i].0.z, -(i as i32)));
        let (c, sgn, name) = match pick {
            Some(i) => self.works.corners.remove(i),
            // No corner will take one: the first stands on high ground
            // inside the walls instead.
            None if have == 0 => match self.lone_tower_site(top) {
                Some(c) => {
                    self.works.corners.clear();
                    (c, (0, 1), "the Watchtower".to_string())
                }
                None => return,
            },
            None => return,
        };
        let (sx, sy) = sgn;
        let door = (2 * sx, 2 * sy);
        // (A lone tower with no wall to open onto: no openings.)
        let openings = if sx == 0 || sy == 0 { vec![] } else { vec![(2 * sx, sy), (sx, 2 * sy)] };
        let steps = tower_steps(c, 3, door, &openings, Crown::Battlements);
        self.clear_trees(&box_iter(c.offset(-2, -2, 0), c.offset(2, 2, 0)).collect::<Vec<_>>());
        let tick = self.tick;
        self.works.list.push(Work {
            kind: WorkKind::Watchtower,
            name: name.clone(),
            at: c,
            height: 3,
            door: Some(c.offset(door.0, door.1, 0)),
            outward: (-sx, -sy),
            steps,
            step: 0,
            hold: usize::MAX,
            mark: 0,
            since: tick,
            begun: tick,
            finished: 0,
            skipped: 0,
        });
        let why = if have == 0 { "to see an army coming before it is at the gate" } else { "so no side of the wall goes unwatched" };
        Self::note_ov(o, tick, format!("Began {} at the corner of the curtain wall, {}.", name, why));
    }

    /// Whether to raise a wizard's tower: a vain, pious or greedy overseer
    /// once the fort is comfortable, anyone once it is very rich.
    pub(super) fn wizard_due(&self, o: &Overseer) -> bool {
        if self.works.list.iter().any(|w| w.kind == WorkKind::WizardTower) || o.did("wizard") {
            return false;
        }
        let wealth = self.wealth() as f64;
        let day = (self.tick / DAY) as f64;
        let minded = o.has(Trait::Vain) || o.has(Trait::Pious) || o.has(Trait::Greedy) || o.style == Style::Builder;
        let pop = self.alive_count();
        (minded && wealth >= self.knob("works.wizard_wealth", 20_000.0) && day >= self.knob("works.wizard_day", 240.0) && pop >= 12)
            || (wealth >= 80_000.0 && day >= 240.0 && pop >= 12)
            // A resident wizard asking for a laboratory (`wizard.rs`).
            || (self.wizard_wants_lab() && pop >= 10)
    }

    /// A tall slender tower with a laboratory at the top, inside the
    /// curtain if there is one, else near the stair.
    fn raise_wizard_tower(&mut self, o: &mut Overseer) {
        let top = o.top.unwrap();
        let tick = self.tick;
        o.done.push("wizard".into());
        let mut site = None;
        'look: for r in 9..26i32 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs().max(dy.abs()) != r {
                        continue;
                    }
                    let (x, y) = (top.x + dx, top.y + dy);
                    if self.works.list.iter().any(|w| (w.at.x - x).abs() <= 7 && (w.at.y - y).abs() <= 7) {
                        continue;
                    }
                    if self.works.corners.iter().any(|c| (c.0.x - x).abs() <= 7 && (c.0.y - y).abs() <= 7) {
                        continue;
                    }
                    if let Some(gate) = self.works.gate {
                        if (gate.x - x).abs() <= 9 && (gate.y - y).abs() <= 9 {
                            continue;
                        }
                    }
                    let Some(c) = self.tower_site(x, y, 6, top) else { continue };
                    // A door on the side that faces the stair, with a way to it.
                    let dir = if (top.x - x).abs() >= (top.y - y).abs() { ((top.x - x).signum(), 0) } else { (0, (top.y - y).signum()) };
                    let outside = c.offset(dir.0 * 3, dir.1 * 3, 0);
                    if !self.world.walkable(outside) || path::astar(&self.world, outside, top, 40_000, Mover::Dwarf).is_none() {
                        continue;
                    }
                    site = Some((c, dir));
                    break 'look;
                }
            }
        }
        let Some((c, dir)) = site else {
            Self::note_ov(o, tick, "Wanted a tower for learning, but found no level ground for it.".into());
            return;
        };
        let door = (dir.0 * 2, dir.1 * 2);
        let steps = tower_steps(c, 5, door, &[], Crown::Laboratory);
        self.clear_trees(&box_iter(c.offset(-2, -2, 0), c.offset(2, 2, 0)).collect::<Vec<_>>());
        let name = if o.has(Trait::Vain) {
            format!("the Spire of {}", o.name.split(' ').next().unwrap_or("the Overseer"))
        } else if o.has(Trait::Pious) {
            "the Star Tower".to_string()
        } else if o.has(Trait::Greedy) {
            "the Counting Tower".to_string()
        } else {
            "the Wizard's Tower".to_string()
        };
        let why = if o.has(Trait::Vain) {
            "Wanting something of its own on the skyline, it"
        } else if o.has(Trait::Pious) {
            "To be nearer the heavens, it"
        } else if o.has(Trait::Greedy) {
            "Wanting a high room to keep the ledgers and the secrets in, it"
        } else {
            "The fort rich enough for idle learning, it"
        };
        self.works.list.push(Work {
            kind: WorkKind::WizardTower,
            name: name.clone(),
            at: c,
            height: 5,
            door: Some(c.offset(door.0, door.1, 0)),
            outward: (-dir.0, -dir.1),
            steps,
            step: 0,
            hold: usize::MAX,
            mark: 0,
            since: tick,
            begun: tick,
            finished: 0,
            skipped: 0,
        });
        Self::note_ov(o, tick, format!("{} began {}, a tall tower with a laboratory at the top.", why, name));
    }

    /// The grand works, for a fort that is rich and whose walls are done.
    fn grand_works(&mut self, o: &mut Overseer) {
        let top = o.top.unwrap();
        let tick = self.tick;
        let wealth = self.wealth();
        // A bridge over a chasm that makes the fort walk the long way round.
        if !o.did("chasmbridge") && wealth >= 20_000 && (self.tick / DAY) >= 200 && !self.alert {
            o.done.push("chasmbridge".into());
            self.plan_chasm_bridge(o);
        }
        let walls_done = self.works.list.iter().filter(|w| w.kind != WorkKind::WizardTower).all(|w| w.done());
        let rich = wealth >= if o.style == Style::Builder || o.has(Trait::Vain) { 30_000 } else { 50_000 };
        if !rich || !walls_done || self.alert || (self.tick / DAY) < 200 {
            return;
        }
        // A pillared portico over the stair: four pillars, a roof between.
        if !o.did("portico") {
            o.done.push("portico".into());
            let z = top.z;
            let flat = (-3..=3).all(|dy| (-3..=3).all(|dx| self.world.surface_z(top.x + dx, top.y + dy) == z));
            let open = (-3..=3).all(|dy| (-3..=3).all(|dx| self.world.kind(top.offset(dx, dy, 1)) == Some(TileKind::Open)));
            let pillars: Vec<Pos> = [(-2, -2), (2, -2), (-2, 2), (2, 2)].iter().map(|&(dx, dy)| top.offset(dx, dy, 0)).collect();
            let free = pillars.iter().all(|&p| self.can_construct(p, ConstructKind::Wall));
            if flat && open && free {
                let roof: Vec<(Pos, ConstructKind)> = box_iter(top.offset(-2, -2, 1), top.offset(2, 2, 1)).map(|p| (p, ConstructKind::Floor)).collect();
                self.works.list.push(Work {
                    kind: WorkKind::Portico,
                    name: "the Great Portico".into(),
                    at: top,
                    height: 1,
                    door: None,
                    outward: (0, 0),
                    steps: vec![pillars.iter().map(|&p| (p, ConstructKind::Wall)).collect(), roof],
                    step: 0,
                    hold: usize::MAX,
                    mark: 0,
                    since: tick,
                    begun: tick,
                    finished: 0,
                    skipped: 0,
                });
                Self::note_ov(o, tick, "Began the Great Portico over the stair: four pillars and a roof, so the way in looks the way in to a great fortress.".into());
            }
        }
        // Pillars round the monument, two storeys tall.
        if !o.did("pillars") {
            let mon = self.buildings.values().find(|b| b.kind == BuildingKind::Monument && b.built).map(|b| b.pos);
            if let (Some(m), Some(name)) = (mon, self.monument.clone()) {
                o.done.push("pillars".into());
                // (The monument is 3×3 from `m`: pillars a step out from its corners.)
                let spots: Vec<Pos> = [(-2, -2), (4, -2), (-2, 4), (4, 4)].iter().map(|&(dx, dy)| m.offset(dx, dy, 0)).filter(|&p| self.can_construct(p, ConstructKind::Wall)).collect();
                if spots.len() >= 2 {
                    let up: Vec<(Pos, ConstructKind)> = spots.iter().map(|&p| (p.up(), ConstructKind::Wall)).collect();
                    self.works.list.push(Work {
                        kind: WorkKind::Pillars,
                        name: format!("the Pillars of {}", name),
                        at: m,
                        height: 2,
                        door: None,
                        outward: (0, 0),
                        steps: vec![spots.iter().map(|&p| (p, ConstructKind::Wall)).collect(), up],
                        step: 0,
                        hold: usize::MAX,
                        mark: 0,
                        since: tick,
                        begun: tick,
                        finished: 0,
                        skipped: 0,
                    });
                    Self::note_ov(o, tick, format!("Began tall pillars round {}, to frame it.", name));
                }
            }
        }
    }

    /// The best place for a bridge over a chasm near the stair: a straight
    /// run of open air between two banks on one level, where walking round
    /// is longest for its length. Two tiles wide where it can be.
    fn plan_chasm_bridge(&mut self, o: &mut Overseer) {
        let top = o.top.unwrap();
        let w = &self.world;
        let mut best: Option<(i32, Vec<Pos>)> = None;
        for dy in -30..=30 {
            for dx in -30..=30 {
                let (x, y) = (top.x + dx, top.y + dy);
                if !w.in_bounds(Pos::new(x, y, 0)) {
                    continue;
                }
                let a = Pos::new(x, y, w.surface_z(x, y));
                if !w.walkable(a) || w.t(a).water > 0 {
                    continue;
                }
                for (ux, uy) in [(1, 0), (0, 1)] {
                    let span: Vec<Pos> = (1..=10).map(|k| a.offset(ux * k, uy * k, 0)).take_while(|&p| w.kind(p) == Some(TileKind::Open) && w.t(p).water == 0 && !w.walkable(p)).collect();
                    let n = span.len() as i32;
                    if n < 2 || n >= 10 {
                        continue;
                    }
                    let b = a.offset(ux * (n + 1), uy * (n + 1), 0);
                    if !w.walkable(b) || w.t(b).water > 0 {
                        continue;
                    }
                    // How far round on foot (or no way at all).
                    let round = path::astar(w, a, b, 30_000, Mover::Dwarf).map_or(400, |p| p.len() as i32);
                    let gain = round - (n + 1);
                    if gain < 20 || best.as_ref().map_or(false, |b| b.0 >= gain) {
                        continue;
                    }
                    // A second lane beside it, if the chasm is as wide there.
                    let mut tiles = span.clone();
                    let side: Vec<Pos> = span.iter().map(|p| p.offset(uy, ux, 0)).collect();
                    if side.iter().all(|&p| w.kind(p) == Some(TileKind::Open) && w.t(p).water == 0) && w.walkable(a.offset(uy, ux, 0)) && w.walkable(b.offset(uy, ux, 0)) {
                        tiles.extend(side);
                    }
                    best = Some((gain, tiles));
                }
            }
        }
        let Some((gain, tiles)) = best else { return };
        let tick = self.tick;
        let at = tiles[tiles.len() / 4];
        self.works.list.push(Work {
            kind: WorkKind::Bridge,
            name: "the Chasm Bridge".into(),
            at,
            height: 0,
            door: None,
            outward: (0, 0),
            steps: vec![tiles.iter().map(|&p| (p, ConstructKind::Floor)).collect()],
            step: 0,
            hold: usize::MAX,
            mark: 0,
            since: tick,
            begun: tick,
            finished: 0,
            skipped: 0,
        });
        Self::note_ov(o, tick, format!("Began a bridge across the chasm, to save a walk of {} paces round it.", gain));
    }

    /// Whether `p` already is what `k` makes it.
    fn tile_is(&self, p: Pos, k: ConstructKind) -> bool {
        k.tile().map_or(false, |t| self.world.kind(p) == Some(t))
    }

    /// What to build the next stretch from: stone if there is plenty, else
    /// blocks if there are plenty, else nothing yet.
    fn works_material(&self, n: usize, o: &Overseer) -> Option<BuildMat> {
        let waiting = self.constructions.len();
        let loose = self.items.values().filter(|it| it.free() && BuildMat::Stone.accepts(it)).count();
        let reserve = if self.gate_wants_stone(o) { STONE_RESERVE * 2 } else { STONE_RESERVE };
        if loose >= n + waiting + reserve {
            return Some(BuildMat::Stone);
        }
        let blocks = self.items.values().filter(|it| it.free() && BuildMat::Blocks.accepts(it)).count();
        if blocks >= n + 30 {
            return Some(BuildMat::Blocks);
        }
        // A fort with no stone to spare raises its walls in timber, if wood is plenty.
        let logs = self.items.values().filter(|it| it.free() && BuildMat::Wood.accepts(it)).count();
        if logs >= n + waiting + 25 {
            return Some(BuildMat::Wood);
        }
        None
    }

    /// Stone for the works: natural rock cut where it shows above ground
    /// near the fort (the shortest haul to the walls), else on the levels
    /// just under the halls, else far out on the main level; a batch at a
    /// time.
    fn works_quarry(&mut self, o: &mut Overseer) {
        let (Some(top), Some(base)) = (o.top, o.base) else { return };
        let stone = |t: &crate::world::Tile| t.kind == TileKind::Wall && t.mat.is_building_stone() && !t.aquifer && t.built_from.is_none();
        let cutting = self.designations.iter().filter(|p| {
            let t = self.world.t(**p);
            t.desig == Desig::Dig && stone(t)
        });
        if cutting.count() >= 8 {
            return;
        }
        let e = self.embark;
        let w = &self.world;
        let open_beside = |p: Pos| DIRS8.iter().any(|&(dx, dy)| w.get(p.offset(dx, dy, 0)).map_or(false, |q| !q.kind.solid() && q.kind.has_floor() && q.water == 0));
        let wet = |p: Pos| DIRS8.iter().any(|&(dx, dy)| w.get(p.offset(dx, dy, 0)).map_or(true, |q| q.water > 0 || q.magma > 0));
        let mut seen: Vec<(i32, Pos)> = Vec::new();
        // Above ground: outcrops and hillsides near the fort.
        for dy in -36..=36 {
            for dx in -36..=36 {
                let (x, y) = (e.x + dx, e.y + dy);
                if !w.in_bounds(Pos::new(x, y, 0)) {
                    continue;
                }
                let z = w.surface_z(x, y);
                for k in -1..=1 {
                    let p = Pos::new(x, y, z + k);
                    let Some(t) = w.get(p) else { continue };
                    // (Beside open ground, or ground already cut into the hill.)
                    let face = DIRS8.iter().any(|&(ex, ey)| {
                        let q = p.offset(ex, ey, 0);
                        w.get(q).map_or(false, |u| u.kind.has_floor() && !u.kind.solid() && u.water == 0 && (w.outside(q) || (u.dug && q.z >= top.z - 1)))
                    });
                    if t.revealed && t.desig == Desig::None && stone(t) && face && !wet(p) && self.work_at(p).is_none() {
                        seen.push((dx.abs() + dy.abs(), p));
                    }
                }
            }
        }
        // Below: beside what's dug, under the halls or far out from them
        // (never in or beside the rooms laid out).
        let rooms: Vec<(Pos, Pos)> = o.rooms.iter().map(|r| (r.a, r.b)).collect();
        let by_room = |p: Pos| rooms.iter().any(|&(a, b)| p.z >= a.z.min(b.z) - 1 && p.z <= a.z.max(b.z) + 1 && p.x >= a.x.min(b.x) - 3 && p.x <= a.x.max(b.x) + 3 && p.y >= a.y.min(b.y) - 3 && p.y <= a.y.max(b.y) + 3);
        if seen.len() < 8 {
            for dz in -4..=0 {
                for dy in -40..=40 {
                    for dx in -40..=40 {
                        let p = base.offset(dx, dy, dz);
                        // (Never close by the stair, and further out on the
                        // halls' own level.)
                        if Self::gatehouse(base, p) || dx.abs() + dy.abs() < if dz == 0 { 25 } else { 15 } || by_room(p) {
                            continue;
                        }
                        let Some(t) = w.get(p) else { continue };
                        if t.revealed && t.desig == Desig::None && stone(t) && !w.outside(p) && open_beside(p) && !wet(p) {
                            seen.push((100 + dx.abs() + dy.abs() - dz * 2 + if dz == 0 { 40 } else { 0 }, p));
                        }
                    }
                }
            }
        }
        if seen.is_empty() {
            return;
        }
        seen.sort();
        let n = seen.len().min(16);
        for &(_, p) in seen.iter().take(n) {
            self.designate(p, p, Desig::Dig);
        }
        self.count("works.quarry", 1);
        if !o.did("worksquarry") {
            o.done.push("worksquarry".into());
            let where_ = if seen[0].0 < 100 { "from the rock that shows above ground" } else { "below" };
            Self::note_ov(o, self.tick, format!("Had stone cut {} for the walls and towers.", where_));
        }
        let _ = top;
    }

    /// Move a work on: plan what can be planned of its current step, and
    /// when the step stands (or what's left of it can't be built), the next.
    fn advance_work(&mut self, o: &mut Overseer, i: usize) {
        let tick = self.tick;
        loop {
            let w = &self.works.list[i];
            if w.done() {
                break;
            }
            if w.step >= w.hold && !self.battlements_due() {
                break;
            }
            let tiles = w.steps[w.step].clone();
            let mut done = 0;
            let mut pending = 0;
            let mut ready = Vec::new();
            for &(p, k) in &tiles {
                if self.tile_is(p, k) {
                    done += 1;
                } else if self.construction_at(p).is_some() {
                    pending += 1;
                } else if self.can_construct(p, k) {
                    ready.push((p, k));
                }
            }
            if done > self.works.list[i].mark {
                let w = &mut self.works.list[i];
                w.mark = done;
                w.since = tick;
            }
            if done == tiles.len() {
                let w = &mut self.works.list[i];
                w.step += 1;
                w.mark = 0;
                w.since = tick;
                if w.done() {
                    self.work_finished(o, i);
                    break;
                }
                continue;
            }
            let busy = self.constructions.len();
            if !ready.is_empty() && busy < MAX_PENDING {
                let room = MAX_PENDING - busy;
                ready.truncate(room);
                if let Some(mat) = self.works_material(ready.len(), o) {
                    for &(p, k) in &ready {
                        pending += self.plan_constructions(p, p, k, mat);
                    }
                    self.works.list[i].since = tick;
                } else {
                    let name = self.works.list[i].name.clone();
                    Self::note_ov(o, tick, format!("Work on {} waits for stone.", name));
                    self.works_quarry(o);
                    break;
                }
            }
            // Nothing building and nothing more can be: give up on what's
            // left of this step after a while (no ground for it).
            // Constructions that never get built (nowhere to stand) are
            // called off after a long while.
            if pending > 0 && tick > self.works.list[i].since + DAY * 20 {
                for &(p, _) in &tiles {
                    self.cancel_constructions(p, p);
                }
                pending = 0;
            }
            // (Trees in the way are felled first: give the woodcutters time.)
            let trees: Vec<Pos> = tiles.iter().map(|t| t.0).filter(|&p| self.world.kind(p) == Some(TileKind::Tree)).collect();
            self.clear_trees(&trees);
            let patience = if trees.is_empty() { DAY * 6 } else { DAY * 20 };
            let w = &mut self.works.list[i];
            if pending == 0 && ready.is_empty() && tick > w.since + patience {
                w.skipped += (tiles.len() - done) as u32;
                w.step += 1;
                w.mark = 0;
                w.since = tick;
                if w.done() {
                    self.work_finished(o, i);
                    break;
                }
                continue;
            }
            break;
        }
    }

    fn work_finished(&mut self, o: &mut Overseer, i: usize) {
        let tick = self.tick;
        let w = &mut self.works.list[i];
        w.finished = tick;
        let (kind, name, days) = (w.kind, w.name.clone(), (tick - w.begun) / DAY);
        let text = match kind {
            WorkKind::Curtain => format!("{} stands finished, battlements, walkway and all, after {} days of work.", cap(&name), days),
            WorkKind::GateTower => format!("{} stands over the gate.", cap(&name)),
            WorkKind::Watchtower => format!("{} is finished: a guard on its top will see an army long before it reaches the gate.", cap(&name)),
            WorkKind::WizardTower => format!("{} is finished, its laboratory at the top waiting for a scholar.", cap(&name)),
            WorkKind::Portico => format!("{} stands over the stair.", cap(&name)),
            WorkKind::Pillars => format!("{} are raised.", cap(&name)),
            WorkKind::Bridge => format!("{} spans the chasm.", cap(&name)),
        };
        Self::note_ov(o, tick, text.clone());
        self.msg(&text, (200, 200, 140));
        if matches!(kind, WorkKind::Curtain | WorkKind::WizardTower) {
            let fort = self.fortress_name.clone();
            self.legend(format!("The dwarves of {} finished {}.", fort, name));
        }
        if kind == WorkKind::Curtain {
            self.milestone("curtain", "A curtain wall round the fort");
        }
        if matches!(kind, WorkKind::Curtain | WorkKind::WizardTower) {
            let at = self.works.list[i].top();
            self.spotlight("A great work", &text, Some(at), (200, 200, 140));
        }
    }

    /// A few more builders while there is building to do.
    fn works_builders(&mut self, o: &Overseer) {
        if self.works.list.iter().all(|w| w.done()) {
            return;
        }
        let want = 2 + self.alive_count() / 15;
        let have = self.dwarves.iter().filter(|d| d.alive && !d.is_child() && !d.soldier && d.has(Labor::Building)).count();
        if have >= want {
            return;
        }
        let hands = self.field_hands();
        let mut cands: Vec<(u32, usize, u32)> = self
            .dwarves
            .iter()
            .filter(|d| d.alive && !d.is_child() && !d.soldier && !d.insane && !d.has(Labor::Building) && d.noble.is_none() && !hands.contains(&d.id))
            .map(|d| (d.skills[Labor::Building.idx()], LABORS.iter().filter(|l| d.has(**l)).count(), d.id))
            .collect();
        cands.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        for (_, _, id) in cands.into_iter().take(want - have) {
            self.set_labor(id, Labor::Building, true);
        }
        let _ = o;
    }

    /// Doors in the towers' doorways once their foot stands (hung again if
    /// broken down).
    fn hang_tower_doors(&mut self) {
        let doors: Vec<Pos> = self.works.list.iter().filter(|w| w.kind.is_tower() && w.step >= 1).filter_map(|w| w.door).collect();
        for p in doors {
            if self.world.t(p).building == 0 && self.stock_of(ItemKind::Door) > 0 {
                self.place_building(BuildingKind::Door, p);
            }
        }
    }

    /// The curtain's drawbridge: once the walls and gate towers are done and
    /// there are mechanisms to spare, a bridge across the gate and a lever in
    /// the first gate tower, linked.
    fn curtain_bridge(&mut self, o: &mut Overseer) {
        let Some(g) = self.works.gate else { return };
        let all_done = self.works.list.iter().filter(|w| matches!(w.kind, WorkKind::Curtain | WorkKind::GateTower)).all(|w| w.done());
        if !all_done || !o.did("linked") || self.gate_parts_missing(o) > 0 {
            return;
        }
        let along = self.works.along;
        if self.works.bridges.is_empty() {
            if self.stock_of(ItemKind::Mechanism) < 5 || self.alert {
                return;
            }
            let Some(t1) = self.works.list.iter().find(|w| w.kind == WorkKind::GateTower).map(|w| w.at) else { return };
            // The lever inside the tower, beside the stair.
            let inw = self.works.inward;
            let lever = t1.offset(inw.0, inw.1, 0);
            let spots: Vec<Pos> = (-1..=1).map(|i| g.offset(along.0 * i, along.1 * i, 0)).collect();
            if !spots.iter().all(|&p| self.can_place(BuildingKind::Bridge, p)) || !self.can_place(BuildingKind::Lever, lever) {
                o.done.push("nobridge".into());
                return;
            }
            for &p in &spots {
                self.place_building(BuildingKind::Bridge, p);
            }
            self.place_building(BuildingKind::Lever, lever);
            self.works.bridges = spots;
            self.works.lever = Some(lever);
            Self::note_ov(o, self.tick, "Began a drawbridge in the gate of the curtain wall, worked from a lever in the gate tower.".into());
            return;
        }
        // Link it once it's built (and again after a rebuild).
        let Some(lp) = self.works.lever else { return };
        let Some(lever) = self.buildings.values().find(|b| b.kind == BuildingKind::Lever && b.pos == lp && b.built).map(|b| b.id) else { return };
        let bridges: Vec<u32> = self.buildings.values().filter(|b| b.kind == BuildingKind::Bridge && b.built && self.works.bridges.contains(&b.pos)).map(|b| b.id).collect();
        if bridges.len() < self.works.bridges.len() {
            return;
        }
        let unlinked: Vec<u32> = bridges.iter().copied().filter(|id| !self.buildings[&lever].links.contains(id)).collect();
        for id in &unlinked {
            let _ = self.link_lever(lever, *id);
        }
        if !unlinked.is_empty() && !o.did("curtainbridge") {
            o.done.push("curtainbridge".into());
            Self::note_ov(o, self.tick, "Linked the lever to the drawbridge in the curtain's gate: when the gate below is barred, so is the wall.".into());
        }
    }

    /// The laboratory: a zone over the top room of the wizard's tower, with
    /// a table and a chair; dwarves keen to learn go up to study.
    fn laboratory(&mut self, o: &mut Overseer) {
        let Some(w) = self.works.list.iter().find(|w| w.kind == WorkKind::WizardTower && w.done()) else { return };
        let room = w.top();
        let name = w.name.clone();
        if self.works.lab != 0 && self.zones.contains_key(&self.works.lab) {
            return;
        }
        let tiles: Vec<Pos> = inner().into_iter().map(|(dx, dy)| room.offset(dx, dy, 0)).filter(|&p| self.world.walkable(p)).collect();
        if tiles.is_empty() {
            return;
        }
        if self.add_zone_tiles(tiles.clone(), ZoneKind::Laboratory) == 0 {
            return;
        }
        self.works.lab = self.world.t(tiles[0]).zone;
        for (k, p) in [(BuildingKind::Table, tiles[0]), (BuildingKind::Chair, tiles[1.min(tiles.len() - 1)])] {
            if self.stock_of(if k == BuildingKind::Table { ItemKind::Table } else { ItemKind::Chair }) > 0 {
                self.place_building(k, p);
            }
        }
        Self::note_ov(o, self.tick, format!("Set the top of {} aside as a laboratory, for whoever wants to learn.", name));
    }

    /// The name of whatever work `p` is part of.
    pub fn work_at(&self, p: Pos) -> Option<String> {
        if let Some(g) = self.works.gate {
            let a = self.works.along;
            if (-1..=1).any(|i| g.offset(a.0 * i, a.1 * i, 0) == p) {
                return Some("the gate of the Curtain Wall".into());
            }
        }
        if self.works.lab != 0 && self.world.get(p).map_or(false, |t| t.zone == self.works.lab) {
            let tower = self.works.list.iter().find(|w| w.kind == WorkKind::WizardTower).map_or(String::new(), |w| format!(" atop {}", w.name));
            return Some(format!("the Laboratory{}", tower));
        }
        self.works.list.iter().find(|w| w.covers(p)).map(|w| w.name.clone())
    }

    /// Works worth the camera's time: a finished tower with someone on top,
    /// or one going up. (who, place, what).
    pub fn works_sights(&self) -> Vec<(Option<u32>, Pos, String)> {
        let mut v = Vec::new();
        for w in &self.works.list {
            let top = w.top();
            let on_top = self.dwarves.iter().find(|d| d.alive && w.kind.is_tower() && d.pos.z == top.z && cheb(d.pos, top) <= 1);
            if w.done() {
                if let Some(d) = on_top {
                    let what = match w.kind {
                        WorkKind::WizardTower => format!("{} in the laboratory atop {}", d.name, w.name),
                        _ => format!("{} keeping watch from {}", d.name, w.name),
                    };
                    v.push((Some(d.id), d.pos, what));
                } else if w.kind != WorkKind::Curtain {
                    v.push((None, top, format!("{} over the fort", cap(&w.name))));
                }
            } else if w.step > 0 {
                v.push((None, if w.kind.is_tower() { w.at.offset(0, 0, (w.step as i32 / 2).min(w.height)) } else { w.at }, format!("{} going up ({}% built)", cap(&w.name), (w.progress() * 100.0) as u32)));
            }
        }
        v
    }

    /// Tests: raise every work now, as if the builders had done it all
    /// (material from nowhere), then hang doors.
    #[cfg(test)]
    pub(super) fn raise_works_now(&mut self) {
        for i in 0..self.works.list.len() {
            let steps = self.works.list[i].steps.clone();
            for step in steps {
                for (p, k) in step {
                    if let Some(t) = k.tile() {
                        let tt = self.world.t_mut(p);
                        tt.kind = t;
                        tt.built_from = Some(ItemKind::Boulder);
                        tt.mat = crate::material::Mat::Granite;
                    }
                }
            }
            let w = &mut self.works.list[i];
            w.step = w.steps.len();
            w.finished = self.tick.max(1);
        }
        self.constructions.retain(|_, c| c.worker.is_some());
        self.dig_version += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A top-down view of the fort's surface round the curtain: the highest
    /// thing built on each column.
    pub(crate) fn surface_view(g: &Game) -> String {
        let (mut x0, mut y0, mut x1, mut y1) = (g.embark.x - 30, g.embark.y - 28, g.embark.x + 30, g.embark.y + 28);
        for w in &g.works.list {
            for &(p, _) in w.steps.iter().flatten() {
                x0 = x0.min(p.x - 2);
                y0 = y0.min(p.y - 2);
                x1 = x1.max(p.x + 2);
                y1 = y1.max(p.y + 2);
            }
        }
        let (x0, y0, x1, y1) = (x0.max(0), y0.max(0), x1.min(g.world.w - 1), y1.min(g.world.h - 1));
        let top = g.overseer.as_ref().and_then(|o| o.top);
        let mut s = String::new();
        for y in y0..=y1 {
            for x in x0..=x1 {
                let z = g.world.surface_z(x, y);
                let mut ch = None;
                for k in (0..=7).rev() {
                    let p = Pos::new(x, y, z + k);
                    let Some(t) = g.world.get(p) else { continue };
                    if t.built_from.is_none() && k > 0 {
                        continue;
                    }
                    let c = match t.kind {
                        TileKind::Fortification => Some('%'),
                        TileKind::Wall if t.built_from.is_some() => Some('#'),
                        TileKind::Floor if t.built_from.is_some() && t.zone != 0 && g.zones.get(&t.zone).map_or(false, |zn| zn.kind == ZoneKind::Laboratory) => Some('L'),
                        TileKind::Floor if t.built_from.is_some() => Some('='),
                        TileKind::StairUp | TileKind::StairDown | TileKind::StairUD if t.built_from.is_some() => Some('X'),
                        _ => None,
                    };
                    if c.is_some() {
                        ch = c;
                        break;
                    }
                }
                let p = Pos::new(x, y, z);
                let t = g.world.t(p);
                let ch = ch.unwrap_or_else(|| {
                    if top.map_or(false, |q| (q.x, q.y) == (x, y)) {
                        '>'
                    } else if t.building != 0 {
                        match g.buildings.get(&t.building).map(|b| b.kind) {
                            Some(BuildingKind::Door) => 'D',
                            Some(BuildingKind::Bridge) => 'H',
                            Some(BuildingKind::Lever) => 'l',
                            Some(BuildingKind::Monument) => 'M',
                            _ => 'b',
                        }
                    } else if g.construction_at(p).is_some() || g.construction_at(p.up()).is_some() {
                        '?'
                    } else if t.water > 0 {
                        '~'
                    } else if t.kind == TileKind::Tree {
                        'T'
                    } else if t.kind == TileKind::Wall {
                        '^'
                    } else if t.zone != 0 {
                        match g.zones.get(&t.zone).map(|z| z.kind) {
                            Some(ZoneKind::Farm) => '"',
                            _ => ',',
                        }
                    } else if g.creatures.values().any(|c| c.alive && c.pos == p && c.hostile()) {
                        'g'
                    } else if t.kind.has_floor() {
                        '.'
                    } else {
                        ' '
                    }
                });
                s.push(ch);
            }
            s.push('\n');
        }
        s
    }

    fn report(g: &Game) -> String {
        let mut s = String::new();
        if let Some(o) = g.overseer.as_ref() {
            s.push_str(&format!("  linked {} laid {} gate {:?} corners {}
", o.did("linked"), g.works.laid, g.works.gate, g.works.corners.len()));
        }
        for w in &g.works.list {
            s.push_str(&format!(
                "  {:<28} {:?} step {}/{} done {} skipped {} begun d{} finished d{}\n",
                w.name,
                w.kind,
                w.step,
                w.steps.len(),
                w.done(),
                w.skipped,
                w.begun / DAY,
                w.finished / DAY
            ));
        }
        s
    }

    /// A fort with its stair dug and its overseer's first day done.
    fn begun(seed: u64, style: Style) -> Game {
        let mut g = Game::new(seed);
        g.set_overseer(Some(style));
        while g.overseer.as_ref().map_or(true, |o| o.base.is_none()) && g.tick < DAY * 3 {
            g.step();
        }
        assert!(g.overseer.as_ref().unwrap().base.is_some(), "no stair");
        g
    }

    /// From how many of a dozen spots on the edge of the map an army could
    /// walk to the stair.
    fn army_reaches_the_stair(g: &mut Game) -> usize {
        let top = g.overseer.as_ref().unwrap().top.unwrap();
        let mut ok = 0;
        for _ in 0..12 {
            if let Some(p) = g.edge_entry() {
                if path::astar(&g.world, p, top, 400_000, Mover::Goblin).is_some() {
                    ok += 1;
                }
            }
        }
        ok
    }

    #[test]
    fn the_curtain_wall_rises_round_the_fields_and_leaves_the_gate_open() {
        let (mut g, mut o) = (0..6)
            .find_map(|k| {
                let mut g = begun(301 + k, Style::Warlord);
                let mut o = g.overseer.take().unwrap();
                g.lay_out_curtain(&mut o);
                if g.works.gate.is_some() && !g.works.corners.is_empty() {
                    Some((g, o))
                } else {
                    None
                }
            })
            .expect("no site took a curtain wall");
        let gate = g.works.gate.unwrap();
        g.overseer = Some(o.clone());
        let before = army_reaches_the_stair(&mut g);
        assert!(before > 0, "the stair was unreachable to begin with");
        // Nothing is ever built in the gate itself, at its foot.
        let a = g.works.along;
        let opening: Vec<Pos> = (-1..=1).map(|i| gate.offset(a.0 * i, a.1 * i, 0)).collect();
        for w in &g.works.list {
            assert!(w.steps.iter().flatten().all(|(p, _)| !opening.contains(p)), "{} builds in the gate", w.name);
        }
        let kinds: Vec<WorkKind> = g.works.list.iter().map(|w| w.kind).collect();
        assert_eq!(kinds, vec![WorkKind::GateTower, WorkKind::GateTower, WorkKind::Curtain]);
        g.raise_works_now();
        g.alert = false;
        g.more_watchtowers(&mut o);
        assert!(g.works.list.iter().any(|w| w.kind == WorkKind::Watchtower), "no watchtower once the walls stood");
        g.raise_works_now();
        g.overseer = Some(o);
        // Battlements along the top, a walkway behind them, towers to climb.
        let forts = g.world.tiles.iter().filter(|t| t.kind == TileKind::Fortification && t.built_from.is_some()).count();
        assert!(forts > 60, "only {} fortifications", forts);
        for w in g.works.list.iter().filter(|w| w.kind.is_tower()) {
            let door = w.door.unwrap();
            let out = door.offset((door.x - w.at.x).signum(), (door.y - w.at.y).signum(), 0);
            assert!(path::astar(&g.world, out, w.top(), 20_000, Mover::Dwarf).is_some(), "no way up {}", w.name);
        }
        // The walkway runs from a gate tower's first storey along the wall.
        let t1 = g.works.list[0].at;
        let walk = g.world.tiles.iter().filter(|t| t.kind == TileKind::Floor && t.built_from.is_some()).count();
        assert!(walk > 40, "only {} walkway floors", walk);
        let first = t1.offset(0, 0, 1);
        let reach = path::search(&g.world, first, 20_000, Mover::Dwarf, |p| p.z == first.z && (p.x - t1.x).abs().max((p.y - t1.y).abs()) >= 8);
        assert!(reach.is_some(), "the walkway doesn't lead out of the gate tower");
        // And the army can still come in at the gate.
        let after = army_reaches_the_stair(&mut g);
        assert!(after > 0, "the curtain shut the army out entirely");
        let top = g.overseer.as_ref().unwrap().top.unwrap();
        let out = gate.offset(-g.works.inward.0 * 2, -g.works.inward.1 * 2, 0);
        assert!(path::astar(&g.world, out, top, 400_000, Mover::Goblin).is_some(), "no way from the gate to the stair");
        // The look panel names it.
        assert_eq!(g.work_at(g.works.list[2].steps[0][0].0).as_deref(), Some("the Curtain Wall"));
        assert!(g.work_at(gate).unwrap().contains("gate"));
        assert!(g.work_at(g.works.list[3].top()).unwrap().ends_with("Watchtower"));
    }

    #[test]
    fn a_tower_goes_up_storey_by_storey_from_real_stone() {
        let mut g = begun(311, Style::Steady);
        let mut o = g.overseer.take().unwrap();
        let e = g.embark;
        let top = o.top.unwrap();
        let c = (8..40).flat_map(|r| [(r, 0), (-r, 0), (0, r), (0, -r)]).find_map(|(dx, dy)| g.tower_site(e.x + dx, e.y + dy, 3, top)).expect("no tower site");
        for p in box_iter(c.offset(-2, -2, 0), c.offset(2, 2, 0)) {
            let t = g.world.t_mut(p);
            if t.kind == TileKind::Tree {
                t.kind = TileKind::Floor;
                t.desig = Desig::None;
            }
        }
        g.designations.retain(|p| (p.x - c.x).abs() > 2 || (p.y - c.y).abs() > 2 || p.z != c.z);
        let steps = tower_steps(c, 3, (0, 2), &[], Crown::Battlements);
        let tiles: usize = steps.iter().map(|s| s.len()).sum();
        g.works.list.push(Work {
            kind: WorkKind::Watchtower,
            name: "the Test Watchtower".into(),
            at: c,
            height: 3,
            door: Some(c.offset(0, 2, 0)),
            outward: (0, -1),
            steps,
            step: 0,
            hold: usize::MAX,
            mark: 0,
            since: 0,
            begun: g.tick,
            finished: 0,
            skipped: 0,
        });
        // Stone by the tower, and builders.
        for k in 0..(tiles + 30) {
            let p = c.offset(4 + (k % 5) as i32, -2 + (k / 30) as i32, 0);
            let p = Pos::new(p.x, p.y, g.world.surface_z(p.x, p.y));
            g.spawn_item(ItemKind::Boulder, crate::material::Mat::Granite, p);
        }
        let stone = |g: &Game| g.items.values().filter(|it| it.kind == ItemKind::Boulder).count();
        let before = stone(&g);
        let ids: Vec<u32> = g.dwarves.iter().map(|d| d.id).collect();
        for id in ids {
            g.set_labor(id, Labor::Building, true);
        }
        let mut seen_steps = Vec::new();
        for _ in 0..80 {
            g.advance_work(&mut o, 0);
            seen_steps.push(g.works.list[0].step);
            if g.works.list[0].done() {
                break;
            }
            for _ in 0..DAY / 2 {
                g.step();
            }
        }
        let w = g.works.list[0].clone();
        assert!(w.done(), "the tower stalled at step {} of {} ({:?})", w.step, w.steps.len(), seen_steps);
        assert_eq!(w.skipped, 0, "parts of the tower were given up");
        // Every tile built of the stone, by the builders.
        for &(p, k) in w.steps.iter().flatten() {
            let t = g.world.t(p);
            assert_eq!(Some(t.kind), k.tile(), "{:?} at {:?}", k, p);
            assert_eq!(t.built_from, Some(ItemKind::Boulder));
        }
        assert!(before - stone(&g) >= tiles, "{} tiles from {} stones", tiles, before - stone(&g));
        // Storey by storey: the step only ever went up.
        assert!(seen_steps.windows(2).all(|p| p[0] <= p[1]));
    }

    #[test]
    fn a_vain_overseer_raises_a_spire_with_a_laboratory_at_the_top() {
        let mut g = begun(321, Style::Steady);
        let mut o = g.overseer.take().unwrap();
        o.traits = vec![Trait::Vain, Trait::Cheerful];
        o.name = "Urist Testmaker".into();
        // Not while the fort is new and poor.
        assert!(!g.wizard_due(&o));
        g.knobs.set.insert("works.wizard_wealth".into(), 0.0);
        g.knobs.set.insert("works.wizard_day".into(), 0.0);
        while g.alive_count() < 12 {
            let mut d = g.dwarves[0].clone();
            d.id = g.new_id();
            g.dwarves.push(d);
        }
        assert!(g.wizard_due(&o), "a vain overseer of a comfortable fort wants its spire");
        g.raise_wizard_tower(&mut o);
        let w = g.works.list.iter().find(|w| w.kind == WorkKind::WizardTower).expect("no tower").clone();
        assert_eq!(w.name, "the Spire of Urist");
        assert_eq!(w.height, 5);
        assert!(!g.wizard_due(&o), "only one");
        g.raise_works_now();
        g.laboratory(&mut o);
        let lab = g.zones.get(&g.works.lab).expect("no laboratory").clone();
        assert_eq!(lab.kind, ZoneKind::Laboratory);
        assert!(lab.tiles.iter().all(|p| p.z == w.at.z + 5));
        // Walled, with windows, and roofed over.
        assert!(lab.tiles.iter().all(|p| g.world.kind(p.up()) == Some(TileKind::Floor)), "the laboratory has no roof");
        let ring_kinds: Vec<TileKind> = ring().into_iter().map(|(dx, dy)| g.world.kind(w.at.offset(dx, dy, 5)).unwrap()).collect();
        assert!(ring_kinds.iter().all(|k| matches!(k, TileKind::Wall | TileKind::Fortification)));
        assert!(ring_kinds.contains(&TileKind::Fortification), "no windows");
        assert!(g.work_at(lab.tiles[0]).unwrap().starts_with("the Laboratory atop the Spire"));
        // Someone keen to learn climbs up to study there.
        let door = w.door.unwrap();
        let foot = door.offset((door.x - w.at.x).signum(), (door.y - w.at.y).signum(), 0);
        assert!(path::astar(&g.world, foot, lab.tiles[0], 30_000, Mover::Dwarf).is_some(), "no way up to the laboratory");
        g.overseer = Some(o);
        let id = g.dwarves[1].id;
        {
            let d = &mut g.dwarves[1];
            d.needs = vec![crate::dwarf::Need { kind: crate::dwarf::NeedKind::Learn, strength: 3, level: -900, low: true }];
            d.pos = foot;
            d.path.clear();
            d.task = Task::Idle;
        }
        let mut studied = false;
        for _ in 0..DAY * 2 {
            g.step();
            let d = g.dwarves.iter().find(|d| d.id == id).unwrap();
            if g.zone_kind_at(d.pos) == Some(ZoneKind::Laboratory) {
                studied = true;
                break;
            }
        }
        assert!(studied, "nobody went up to study");
    }

    /// A grown Warlord fort under a cautious overseer walls its fields:
    /// gate towers, a curtain wall with battlements, a watchtower; and an
    /// army can still walk to its gate. (The first of a few forts that
    /// lives that long. Few do by day 440, and which ones shifts whenever
    /// the year's events do: of seeds 5000-5023, 5003 and 5009 did before
    /// MAGIC, HORRORS, CULTURE and FLAIR came in, 5013 after; the
    /// cisterns moved it on again, and the trap hall that kills (O04) once
    /// more, so a few more are tried.)
    #[test]
    // (Plays up to a dozen forts for well over a year: run it with `--ignored`.)
    #[ignore]
    fn a_wary_warlord_walls_its_fields_and_raises_a_watchtower() {
        let day = 440;
        let mut tried = Vec::new();
        for seed in [5013u64, 5003, 5009, 5018, 5023, 5008, 5028, 5033, 5038, 5043, 5048, 5053, 5058, 5063, 5068, 5073, 5078, 5083] {
            let mut g = Game::new(seed);
            // (Its walls, not its sallies: sally.rs.)
            g.knobs.set.insert("sally.min_soldiers".into(), 999.0);
            g.set_overseer(Some(Style::Warlord));
            g.overseer.as_mut().unwrap().traits = vec![Trait::Cautious, Trait::Vain];
            while g.tick < day * DAY && g.alive_count() > 0 {
                g.step();
            }
            let curtain = g.works.list.iter().find(|w| w.kind == WorkKind::Curtain).cloned();
            let tower = g.works.list.iter().any(|w| w.kind == WorkKind::Watchtower && w.done());
            // (A curtain still going up is a fort that started late: try the next.)
            let Some(c) = curtain.filter(|c| tower && g.alive_count() > 0 && c.step >= c.hold) else {
                tried.push((seed, g.alive_count(), g.works.list.iter().map(|w| (w.kind, w.step, w.steps.len())).collect::<Vec<_>>()));
                continue;
            };
            // Gate towers up, the walls standing, battlements along them.
            assert!(g.works.list.iter().filter(|w| w.kind == WorkKind::GateTower).all(|w| w.done()));
            assert!(c.step >= c.hold, "the curtain's walls aren't up: step {} of {}", c.step, c.hold);
            let walls = c.steps[..c.hold].iter().flatten().filter(|&&(p, k)| g.tile_is(p, k)).count();
            let forts = c.steps[c.hold..].iter().flatten().filter(|&&(p, k)| k == ConstructKind::Fortification && g.tile_is(p, k)).count();
            assert!(walls >= 60, "only {} stretches of wall", walls);
            // (The battlements come last, and may still be going up.)
            assert!(forts >= 20 || !c.done(), "only {} battlements", forts);
            // Built of the fort's own stone or timber.
            let (p, _) = c.steps[0].iter().copied().find(|&(p, k)| g.tile_is(p, k)).unwrap();
            assert!(g.world.t(p).built_from.is_some());
            // And the way in stays open (with the drawbridges down and the
            // doors unbarred): from the edge of the map, in at the gate, to
            // the stair.
            let gate = g.works.gate.unwrap();
            let top = g.overseer.as_ref().unwrap().top.unwrap();
            let out = gate.offset(-g.works.inward.0 * 2, -g.works.inward.1 * 2, 0);
            let out = Pos::new(out.x, out.y, g.world.surface_z(out.x, out.y));
            let mut w = g.world.clone();
            for t in w.tiles.iter_mut() {
                if t.door == crate::world::BRIDGE_RAISED || t.door == crate::world::DOOR_LOCKED {
                    t.door = crate::world::DOOR_NONE;
                }
            }
            assert!(path::astar(&w, out, top, 400_000, Mover::Goblin).is_some(), "no way in at the gate");
            let edges: Vec<Pos> = (0..12).filter_map(|_| g.edge_entry()).collect();
            assert!(edges.iter().any(|&p| path::astar(&w, p, top, 400_000, Mover::Goblin).is_some()), "the army can't reach the stair");
            eprintln!("{}", surface_view(&g));
            return;
        }
        panic!("no wary warlord walled its fields by day {}: {:?}", day, tried);
    }

    /// `SEED=5003 STYLE=Warlord DAYS=900 cargo test --release a_fort_from_above -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn a_fort_from_above() {
        let seed: u64 = std::env::var("SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(5003);
        let days: u64 = std::env::var("DAYS").ok().and_then(|s| s.parse().ok()).unwrap_or(900);
        let style = std::env::var("STYLE").ok().and_then(|s| Style::parse(&s)).unwrap_or(Style::Warlord);
        let mut g = Game::new(seed);
        g.set_overseer(Some(style));
        if std::env::var("WARY").is_ok() {
            g.overseer.as_mut().unwrap().traits = vec![Trait::Cautious, Trait::Vain];
        }
        let mut last = 0;
        while g.tick < days * DAY && g.alive_count() > 0 {
            g.step();
            if g.tick / (DAY * 60) != last {
                last = g.tick / (DAY * 60);
                let stone = g.items.values().filter(|it| it.free() && BuildMat::Stone.accepts(it)).count();
                let logs = g.items.values().filter(|it| it.free() && BuildMat::Wood.accepts(it)).count();
                let cut = g.annals.counters.get("works.quarry").copied().unwrap_or(0);
                let waiting: Vec<Pos> = g.designations.iter().copied().filter(|p| g.world.t(*p).desig == Desig::Dig && g.world.t(*p).mat.is_building_stone() && g.world.t(*p).kind == TileKind::Wall).collect();
                let pop = g.alive_count().max(1) as f64;
                let drink = g.stock_of(ItemKind::Drink) as f64 / pop;
                let food = [ItemKind::Meal, ItemKind::Plant, ItemKind::Meat, ItemKind::Fish].iter().map(|&k| g.stock_of(k)).sum::<usize>() as f64 / pop;
                eprintln!("  stone digs waiting {} drink {:.1} food {:.1} can wait {} alert {}", waiting.len(), drink, food, g.works_can_wait(), g.alert);
                eprintln!("day {} pop {} wealth {} stone {} logs {} quarried {} cons {} works {}", g.tick / DAY, g.alive_count(), g.wealth(), stone, logs, cut, g.constructions.len(), g.works.list.len());
                eprint!("{}", report(&g));
            }
        }
        let o = g.overseer.as_ref().unwrap();
        eprintln!("{} ({}) day {} pop {} wealth {} sieges {}", g.fortress_name, o.title(), g.tick / DAY, g.alive_count(), g.wealth(), g.annals.sieges.len());
        eprint!("{}", report(&g));
        for n in o.notes.iter().filter(|n| n.1.contains("wall") || n.1.contains("ower") || n.1.contains("Portico") || n.1.contains("abor") || n.1.contains("watch") || n.1.contains("drawbridge")) {
            eprintln!("  d{} {}", n.0 / DAY, n.1);
        }
        eprintln!("{}", surface_view(&g));
    }
}
