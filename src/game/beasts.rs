//! The bestiary at play (`crate::bestiary` has the animals). Each kind
//! of animal comes where it lives and acts its part:
//!
//! - who comes: the site's region populations (`worldgen::regions`), the
//!   fish of its waters, the sea's beasts on a coast, the cave things of
//!   each cavern layer (there when the miners break into it);
//! - grazers keep with their herd and run from hunters and dwarves; prey
//!   bolts; predators hunt prey their size (and, the big ones, dwarves:
//!   `wildlife::predator_think`); scavengers make for carcasses and
//!   battlefields; vermin go for the food stores (and cats catch them);
//!   ambushers lie still until something comes close; the territorial
//!   drive off whoever comes too near;
//! - fliers cross the sky from one edge of the map to the other (or drop
//!   to carrion and then go), swimmers keep to the water, burrowers come
//!   up in the fort's tunnels;
//! - kept animals give their eggs and wool; butchered, they give what the
//!   table says (`bestiary::carcass_yields`).

use super::combat::{adjacent, dist2, faction, hostile_to, near, Faction};
use super::*;
use crate::bestiary::{self, Beast, Class, Temper, BURROW, SWIM};
use crate::creature::{Creature, Species};

/// Most bestiary animals about the surface at once.
const CAP: usize = 30;

/// The side a table animal takes among the creatures (`combat::faction`).
pub(super) fn beast_faction(b: &Beast) -> Faction {
    match b.temper {
        Temper::Predator | Temper::Ambusher => Faction::Predator,
        Temper::Territorial if b.aggro() > 0 => Faction::Predator,
        _ => Faction::Prey,
    }
}

/// The first id of the bestiary's animals: far above everything else's.
pub const BEAST_IDS: u32 = 0xB000_0000;

/// A bestiary animal that's dangerous only to whoever comes close (the
/// territorial, ambushers, fish): it doesn't prowl about the fort, so it
/// doesn't keep the dwarves indoors or send the hunters out (`prowlers`).
pub(super) fn keeps_to_itself(c: &Creature) -> bool {
    c.species.beast().map_or(false, |b| matches!(b.temper, Temper::Territorial | Temper::Ambusher) || b.water_only())
}

/// The bestiary's animals wander by their own ways (`beast_think`), not
/// the usual way (`combat::creature_tick`).
pub(super) fn own_ways(c: &Creature) -> bool {
    matches!(c.species, Species::Beast(_))
}

fn on_edge(p: Pos, w: i32, h: i32) -> bool {
    p.x <= 0 || p.y <= 0 || p.x >= w - 1 || p.y >= h - 1
}

/// The nearest point of the map's edge.
fn nearest_edge(p: Pos, w: i32, h: i32) -> (i32, i32) {
    let d = [(p.x, (0, p.y)), (w - 1 - p.x, (w - 1, p.y)), (p.y, (p.x, 0)), (h - 1 - p.y, (p.x, h - 1))];
    d.iter().min_by_key(|e| e.0).map(|e| e.1).unwrap()
}

/// A straight line over the land from `a` to `(x, y)` at the height of the
/// ground below, as a reversed path (the next step last).
fn sky_route(world: &crate::world::World, a: Pos, to: (i32, i32)) -> Vec<Pos> {
    let (dx, dy) = (to.0 - a.x, to.1 - a.y);
    let n = dx.abs().max(dy.abs());
    let mut v = Vec::with_capacity(n as usize);
    for i in 1..=n {
        let x = (a.x as f32 + dx as f32 * i as f32 / n as f32).round() as i32;
        let y = (a.y as f32 + dy as f32 * i as f32 / n as f32).round() as i32;
        let (x, y) = (x.clamp(0, world.w - 1), y.clamp(0, world.h - 1));
        v.push(Pos::new(x, y, world.surface_z(x, y)));
    }
    v.reverse();
    v
}

/// One step from `a` towards `b` over the land (fliers).
fn sky_step(world: &crate::world::World, a: Pos, b: Pos) -> Pos {
    let x = (a.x + (b.x - a.x).signum()).clamp(0, world.w - 1);
    let y = (a.y + (b.y - a.y).signum()).clamp(0, world.h - 1);
    Pos::new(x, y, world.surface_z(x, y))
}

impl Game {
    // ------------------------------------------------------------------
    // Who lives here
    // ------------------------------------------------------------------

    /// The site's warmth, from the world's fields.
    pub(super) fn site_temp(&self) -> Option<f32> {
        let (x, y) = self.site;
        let ow = &self.overworld;
        if x < 0 || y < 0 || x >= ow.w || y >= ow.h {
            return None;
        }
        ow.fields.temp.get((y * ow.w + x) as usize).map(|&t| t as f32)
    }

    /// The sea is within a couple of cells of the site.
    pub(super) fn site_coastal(&self) -> bool {
        let (sx, sy) = self.site;
        let ow = &self.overworld;
        if ow.cells.len() <= 1 {
            return self.biome == Biome::Ocean;
        }
        (-2..=2).any(|dy| {
            (-2..=2).any(|dx| {
                let (x, y) = (sx + dx, sy + dy);
                x >= 0 && y >= 0 && x < ow.w && y < ow.h && ow.cell(x, y).biome == Biome::Ocean
            })
        })
    }

    /// The bestiary's animals that may come to this site: the region's
    /// populations (or, with no region, the biome's), the fish and
    /// waterfowl of fresh water, and on a coast the sea's. Not the cave
    /// dwellers (`populate_cavern_beasts`) or the kept animals.
    pub fn site_beasts(&self) -> Vec<u16> {
        let temp = self.site_temp();
        let fits = |b: &Beast| temp.map_or(true, |t| b.fits_temp(t));
        let mut v: Vec<u16> = Vec::new();
        let (sx, sy) = self.site;
        let region = if sx >= 0 && sy >= 0 && sx < self.overworld.w && sy < self.overworld.h && self.overworld.cells.len() > 1 {
            self.overworld.region_at(sx, sy)
        } else {
            None
        };
        match region {
            Some(r) => v.extend(r.fauna.iter().filter_map(|s| match s {
                Species::Beast(i) => Some(*i),
                _ => None,
            })),
            None => {
                let bit = bestiary::biome_bit(self.biome);
                v.extend(bestiary::BEASTS.iter().enumerate().filter(|(_, b)| b.home & bit != 0 && !b.water_only() && fits(b)).map(|(i, _)| i as u16));
            }
        }
        let mut water = bestiary::RIVER | bestiary::LAKE;
        if self.site_coastal() {
            water |= bestiary::SEA | bestiary::COAST;
        }
        for (i, b) in bestiary::BEASTS.iter().enumerate() {
            if b.home & water != 0 && fits(b) && !b.cave_only() && !b.domestic() {
                v.push(i as u16);
            }
        }
        v.retain(|&i| {
            let b = bestiary::beast(i);
            !b.cave_only() && !b.domestic()
        });
        v.sort_unstable();
        v.dedup();
        v
    }

    /// The fish a fisher may land here (`farming::do_fish`): those of the
    /// site's waters under the sky, the cave fish below.
    pub(super) fn fish_of_waters(&self, outside: bool) -> Vec<&'static str> {
        let temp = self.site_temp();
        let mut bits = if outside { bestiary::RIVER | bestiary::LAKE } else { bestiary::CAVES };
        if outside && self.site_coastal() {
            bits |= bestiary::SEA;
        }
        bestiary::BEASTS
            .iter()
            .filter(|b| b.class == Class::Fish && b.home & bits != 0 && b.size <= 1.0 && b.rarity <= 3 && (!outside || temp.map_or(true, |t| b.fits_temp(t))))
            .map(|b| b.name)
            .collect()
    }

    /// Remember that this kind has been about the fort (the bestiary page).
    pub(super) fn note_seen(&mut self, i: u16) {
        if let Err(k) = self.beasts_seen.binary_search(&i) {
            self.beasts_seen.insert(k, i);
        }
    }

    // ------------------------------------------------------------------
    // Coming and going
    // ------------------------------------------------------------------

    /// One of the bestiary's animals arrives, numbered among its own kind.
    pub(super) fn spawn_beast(&mut self, sp: Species, pos: Pos, stay: u64) -> u32 {
        let mut id = BEAST_IDS.wrapping_add(self.beast_ids);
        while self.creatures.contains_key(&id) || self.items.contains_key(&id) {
            self.beast_ids = self.beast_ids.wrapping_add(1);
            id = BEAST_IDS.wrapping_add(self.beast_ids);
        }
        self.beast_ids = self.beast_ids.wrapping_add(1);
        let leave = self.tick.saturating_add(stay);
        self.creatures.insert(id, Creature::new(id, sp, pos, leave));
        id
    }

    /// A roll of the bestiary's own dice for creature `id` this tick.
    fn beast_roll(&self, id: u32, p: f64) -> bool {
        let seed = self.world.gen_seed ^ self.tick.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (id as u64).wrapping_mul(0xD1B5_4A32_D192_ED03);
        crate::rng::Rng::new(seed).chance(p)
    }

    /// Run `f` with dice of the bestiary's own (fixed by the world, the
    /// tick and `salt`), leaving the game's own dice as they were: the
    /// animals' comings don't reshuffle everything else that happens.
    fn with_own_dice<R>(&mut self, salt: u64, f: impl FnOnce(&mut Game) -> R) -> R {
        let seed = self.world.gen_seed ^ self.tick.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ salt;
        let main = std::mem::replace(&mut self.rng, crate::rng::Rng::new(seed));
        let r = f(self);
        self.rng = main;
        r
    }

    /// Once a day (from `wildlife`): now and then some of the site's
    /// animals come by, the commoner more often. Nothing dangerous in the
    /// first season.
    pub(super) fn beast_visit(&mut self) {
        self.with_own_dice(0xBE57, |g| g.beast_visit_now());
    }

    fn beast_visit_now(&mut self) {
        if !self.rng.chance(self.knob("beasts.chance", 0.35)) {
            return;
        }
        let about = self.creatures.values().filter(|c| matches!(c.species, Species::Beast(_)) && !c.tame && !c.species.dwells_underground()).count();
        if about >= CAP {
            return;
        }
        let early = self.tick < SEASON;
        let pool: Vec<(u16, f64)> = self
            .site_beasts()
            .into_iter()
            .map(|i| (i, bestiary::beast(i)))
            .filter(|(_, b)| !(early && b.aggro() > 0))
            // The dangerous come less often than the rest (the land's
            // usual hunters come as ever, `wildlife`).
            .map(|(i, b)| (i, b.weight() * if b.aggro() > 0 { self.knob("beasts.danger", 0.4) } else { 1.0 }))
            .collect();
        let total: f64 = pool.iter().map(|p| p.1).sum();
        if total <= 0.0 {
            return;
        }
        let mut roll = self.rng.f64() * total;
        let mut pick = pool[0].0;
        for &(i, w) in &pool {
            if roll < w {
                pick = i;
                break;
            }
            roll -= w;
        }
        let ids = self.spawn_beasts(pick);
        if ids.is_empty() {
            return;
        }
        self.count("beasts.visits", 1);
        let b = bestiary::beast(pick);
        let n = ids.len();
        if b.aggro() > 0 {
            let what = if n > 1 { format!("A {} of {}", if b.temper == Temper::Grazer { "herd" } else { "pack" }, b.plural) } else { format!("{} {}", an(b.name), b.name) };
            self.msg(&format!("{} has been spotted nearby.", super::combat::cap(&what)), (255, 170, 80));
        } else if b.rarity >= 4 {
            self.msg(&format!("{} {} has been seen near the fort.", super::combat::cap(an(b.name)), b.name), (170, 220, 255));
        }
    }

    /// A party of beast `i` arrives, each by its own way: fish in the
    /// water, fliers over the edge of the map, burrowers up in the tunnels,
    /// the rest out of their lair or in from the edge. Their ids.
    pub(super) fn spawn_beasts(&mut self, i: u16) -> Vec<u32> {
        let b = bestiary::beast(i);
        let sp = Species::Beast(i);
        let n = self.rng.range(b.group.0 as i32, b.group.1 as i32 + 1).max(1);
        let stay = DAY * self.rng.range(1, 4) as u64;
        let mut ids = Vec::new();
        if b.water_only() {
            let Some(at) = self.surface_water(800) else { return ids };
            for _ in 0..n {
                let p = self.water_near(at, 3).unwrap_or(at);
                ids.push(self.spawn_beast(sp, p, stay));
            }
        } else if b.flier() {
            let (w, h) = (self.world.w, self.world.h);
            let side = self.rng.range(0, 4);
            let (fx, fy, tx, ty) = match side {
                0 => (0, self.rng.range(0, h), w - 1, self.rng.range(0, h)),
                1 => (w - 1, self.rng.range(0, h), 0, self.rng.range(0, h)),
                2 => (self.rng.range(0, w), 0, self.rng.range(0, w), h - 1),
                _ => (self.rng.range(0, w), h - 1, self.rng.range(0, w), 0),
            };
            let carrion = if b.temper == Temper::Scavenger { self.carrion_outside() } else { None };
            for k in 0..n {
                let (ox, oy) = if side < 2 { (0, k) } else { (k, 0) };
                let (x, y) = ((fx + ox).clamp(0, w - 1), (fy + oy).clamp(0, h - 1));
                let start = Pos::new(x, y, self.world.surface_z(x, y));
                // A crossing takes a day at most; carrion keeps it half a day.
                let id = self.spawn_beast(sp, start, DAY);
                let route = match carrion {
                    Some(p) => sky_route(&self.world, start, (p.x + ox % 2, p.y + oy % 2)),
                    None => sky_route(&self.world, start, ((tx + ox).clamp(0, w - 1), (ty + oy).clamp(0, h - 1))),
                };
                if let Some(c) = self.creatures.get_mut(&id) {
                    c.path = route;
                }
                ids.push(id);
            }
        } else {
            let tunnel = if b.can(BURROW) && self.rng.chance(0.5) { self.tunnel_spot() } else { None };
            let shore = if tunnel.is_none() && b.can(SWIM) && b.home & (bestiary::FRESH | bestiary::SEA | bestiary::COAST) != 0 && self.rng.chance(0.7) {
                self.surface_water(400).map(|p| self.spot_near(p, 2))
            } else {
                None
            };
            let (entry, n, lair) = match tunnel.or(shore) {
                Some(p) => (p, n, None),
                None => match self.lair_sends(sp, n as u32) {
                    None => return ids,
                    Some(Some((li, m))) => (self.lairs[li].pos, m as i32, Some(li as u32)),
                    Some(None) => match self.edge_entry() {
                        Some(p) => (p, n, None),
                        None => return ids,
                    },
                },
            };
            for _ in 0..n {
                let p = self.spot_near(entry, 2);
                let id = self.spawn_beast(sp, p, stay);
                if let Some(c) = self.creatures.get_mut(&id) {
                    c.lair = lair;
                }
                ids.push(id);
            }
        }
        // Those that came together keep together.
        if ids.len() > 1 {
            let first = ids[0];
            for id in &ids {
                if let Some(c) = self.creatures.get_mut(id) {
                    c.pack = first;
                }
            }
        }
        ids
    }

    /// A tile of open water under the sky, by looking about at random.
    pub(super) fn surface_water(&mut self, tries: usize) -> Option<Pos> {
        let (w, h) = (self.world.w, self.world.h);
        for _ in 0..tries {
            let (x, y) = (self.rng.range(0, w), self.rng.range(0, h));
            let z = self.world.surface_z(x, y);
            for p in [Pos::new(x, y, z), Pos::new(x, y, z - 1)] {
                if self.world.get(p).map_or(false, |t| t.water > 0 && !t.kind.solid() && !t.frozen) {
                    return Some(p);
                }
            }
        }
        None
    }

    /// A water tile within `r` of `at` on its level.
    fn water_near(&mut self, at: Pos, r: i32) -> Option<Pos> {
        for _ in 0..30 {
            let q = at.offset(self.rng.range(-r, r + 1), self.rng.range(-r, r + 1), 0);
            if self.world.get(q).map_or(false, |t| t.water > 0 && !t.kind.solid()) {
                return Some(q);
            }
        }
        None
    }

    /// A tunnel the fort has dug, somewhere under the surface.
    fn tunnel_spot(&mut self) -> Option<Pos> {
        let (w, h) = (self.world.w, self.world.h);
        for _ in 0..300 {
            let (x, y) = (self.rng.range(1, w - 1), self.rng.range(1, h - 1));
            let z = self.world.surface_z(x, y) - self.rng.range(1, 16);
            let p = Pos::new(x, y, z);
            if self.world.in_bounds(p) && self.world.t(p).dug && self.world.walkable(p) && !self.world.outside(p) {
                return Some(p);
            }
        }
        None
    }

    /// The nearest carcass or corpse under the sky, or the fresh battlefield.
    fn carrion_outside(&self) -> Option<Pos> {
        let mid = Pos::new(self.world.w / 2, self.world.h / 2, 0);
        self.items
            .values()
            .filter(|it| matches!(it.kind, ItemKind::Carcass | ItemKind::Corpse) && it.carried_by.is_none() && self.world.outside(it.pos))
            .map(|it| it.pos)
            .min_by_key(|p| (p.x - mid.x).abs() + (p.y - mid.y).abs())
            .or_else(|| self.fresh_battlefield().filter(|&p| self.world.outside(p)))
    }

    /// The miners have broken into a cavern at `at`: the first time in its
    /// layer, the layer's animals are found there (on its floors away from
    /// the breach, and the fish in its pools).
    pub(super) fn wake_cavern(&mut self, at: Pos) {
        let mut bands = self.world.caverns.clone();
        bands.sort();
        let Some(layer) = bands.iter().position(|&(lo, hi)| at.z >= lo - 1 && at.z <= hi + 1) else { return };
        if layer >= 8 || self.beast_layers & (1 << layer) != 0 {
            return;
        }
        self.beast_layers |= 1 << layer;
        let (lo, hi) = bands[layer];
        let mut floors = Vec::new();
        for z in lo..=hi {
            for y in 0..self.world.h {
                for x in 0..self.world.w {
                    let p = Pos::new(x, y, z);
                    let t = self.world.t(p);
                    if t.kind == TileKind::Floor && t.water == 0 && !t.dug && t.building == 0 && !near(p, at, 10) {
                        floors.push(p);
                    }
                }
            }
        }
        self.populate_cavern_beasts(layer, &floors, lo, hi);
    }

    /// A few kinds of the layer's animals in cavern layer `layer` (0 the
    /// first), on `floors` and, for the fish, in its pools.
    pub(super) fn populate_cavern_beasts(&mut self, layer: usize, floors: &[Pos], lo: i32, hi: i32) {
        if floors.is_empty() {
            return;
        }
        self.with_own_dice(0xCA7E + layer as u64, |g| g.populate_layer(layer, floors, lo, hi));
    }

    fn populate_layer(&mut self, layer: usize, floors: &[Pos], lo: i32, hi: i32) {
        let bit = bestiary::cave_bit(layer);
        // Only the cave dwellers proper: they sleep until the cavern is found.
        let mut pool: Vec<u16> = bestiary::of_habitat(bit, None).filter(|&i| bestiary::beast(i).cave_only()).collect();
        let mut pools: Vec<Pos> = Vec::new();
        for z in lo..=hi {
            for y in 0..self.world.h {
                for x in 0..self.world.w {
                    let p = Pos::new(x, y, z);
                    let t = self.world.t(p);
                    if t.water > 0 && !t.kind.solid() {
                        pools.push(p);
                    }
                }
            }
        }
        let kinds = self.knob("beasts.cave_kinds", 5.0) as usize;
        for _ in 0..kinds {
            if pool.is_empty() {
                break;
            }
            let total: f64 = pool.iter().map(|&i| bestiary::beast(i).weight()).sum();
            let mut roll = self.rng.f64() * total;
            let mut k = 0;
            for (j, &i) in pool.iter().enumerate() {
                let w = bestiary::beast(i).weight();
                if roll < w {
                    k = j;
                    break;
                }
                roll -= w;
            }
            let i = pool.remove(k);
            let b = bestiary::beast(i);
            let spots: &[Pos] = if b.water_only() { &pools } else { floors };
            if spots.is_empty() {
                continue;
            }
            let at = *self.rng.pick(spots);
            let n = self.rng.range(b.group.0 as i32, b.group.1 as i32 + 1).max(1);
            let mut first = 0;
            for _ in 0..n {
                let p = if b.water_only() { self.water_near(at, 2).unwrap_or(at) } else { self.spot_near(at, 2) };
                let id = self.spawn_beast(Species::Beast(i), p, u64::MAX);
                if first == 0 {
                    first = id;
                }
                if n > 1 {
                    if let Some(c) = self.creatures.get_mut(&id) {
                        c.pack = first;
                    }
                }
            }
            self.count("beasts.cave_kinds", 1);
        }
    }

    // ------------------------------------------------------------------
    // Fliers and swimmers (every tick, from `creature_tick`)
    // ------------------------------------------------------------------

    /// A wild flier of the open sky or a fish: its whole turn, done here
    /// (`Some(false)`: it's gone). Everything else goes the usual way.
    pub(super) fn beast_tick(&mut self, c: &mut Creature) -> Option<bool> {
        let b = c.species.beast()?;
        if c.tame || c.siege {
            return None;
        }
        if b.water_only() {
            return Some(self.swim_tick(c, b));
        }
        // (Under a roof, fliers get about like everything else.)
        if b.flier() && !b.cave_only() && self.world.outside(c.pos) {
            // Come down after a dwarf: it fights on the ground like any beast.
            if c.target.is_some() && self.world.walkable(c.pos) {
                return None;
            }
            return Some(self.fly_tick(c, b));
        }
        None
    }

    fn fly_tick(&mut self, c: &mut Creature, b: &'static Beast) -> bool {
        c.move_cd = c.move_cd.saturating_sub(1);
        c.attack_cd = c.attack_cd.saturating_sub(1);
        let (w, h) = (self.world.w, self.world.h);
        if !c.leaving && self.tick >= c.leave_tick {
            c.leaving = true;
            c.foe = None;
            c.target = None;
            c.path.clear();
        }
        if c.leaving && c.path.is_empty() {
            if on_edge(c.pos, w, h) {
                return false;
            }
            c.path = sky_route(&self.world, c.pos, nearest_edge(c.pos, w, h));
        }
        if c.think_cd == 0 {
            c.think_cd = 15;
            if !c.leaving {
                self.flier_think(c, b);
            }
        } else {
            c.think_cd -= 1;
        }
        // After something: swoop on it, and strike when beside it.
        let quarry = c
            .foe
            .and_then(|f| self.creatures.get(&f).filter(|o| o.alive).map(|o| o.pos))
            .or_else(|| c.target.and_then(|t| self.dwarves.iter().find(|d| d.id == t && d.alive).map(|d| d.pos)));
        if let Some(qp) = quarry {
            if !near(qp, c.pos, 14) {
                c.foe = None;
                c.target = None;
            } else if near(qp, c.pos, 1) {
                if c.attack_cd == 0 {
                    c.attack_cd = b.attack_delay();
                    if let Some(f) = c.foe {
                        self.creature_maul(c, f);
                    } else if let Some(di) = c.target.and_then(|t| self.dwarves.iter().position(|d| d.id == t)) {
                        self.creature_attack(c, di);
                    }
                }
                return true;
            } else if c.move_cd == 0 {
                c.pos = sky_step(&self.world, c.pos, qp);
                c.move_cd = b.speed as u32;
                return true;
            }
        } else {
            c.foe = None;
            c.target = None;
        }
        if c.move_cd == 0 {
            match c.path.pop() {
                Some(next) => {
                    c.pos = next;
                    c.move_cd = b.speed as u32;
                }
                // Across: gone over the far edge.
                None if on_edge(c.pos, w, h) && !c.leaving => return false,
                // Come down at carrion: it stays a while.
                None => {}
            }
        }
        true
    }

    /// A flier looks about: a hunter for prey below (and the big ones for
    /// a dwarf on their own), a scavenger for carrion.
    fn flier_think(&mut self, c: &mut Creature, b: &'static Beast) {
        if let Species::Beast(i) = c.species {
            self.note_seen(i);
        }
        match b.temper {
            Temper::Scavenger if c.path.is_empty() => {
                // Picked clean: off again, unless there is more.
                match self.carrion_outside().filter(|&p| !near(p, c.pos, 1)) {
                    Some(p) => c.path = sky_route(&self.world, c.pos, (p.x, p.y)),
                    None if c.leave_tick > self.tick + DAY / 2 => c.leave_tick = self.tick + DAY / 2,
                    None => {}
                }
            }
            Temper::Predator if c.foe.is_none() && c.target.is_none() => {
                let pos = c.pos;
                let me = c.id;
                let prey = self
                    .creatures
                    .values()
                    .filter(|o| o.id != me && o.alive && !o.tame && !o.sneaking && near(o.pos, pos, 8) && faction(o) == Faction::Prey && self.creature_active(o))
                    .filter(|o| o.species.info().scale <= (b.size * 0.8).max(0.05))
                    .min_by_key(|o| (dist2(o.pos, pos), o.id))
                    .map(|o| o.id);
                if let Some(id) = prey {
                    c.foe = Some(id);
                    self.count("beasts.swoops", 1);
                    return;
                }
                if b.aggro() > 0 {
                    let r = b.aggro() * 2;
                    let lone = self
                        .dwarves
                        .iter()
                        .filter(|d| d.alive && self.world.outside(d.pos) && near(d.pos, pos, r))
                        .find(|d| self.prey_odds(d).1 == 0.0)
                        .map(|d| d.id);
                    if let Some(id) = lone {
                        c.target = Some(id);
                        self.count("beasts.swoops", 1);
                    }
                }
            }
            _ => {}
        }
    }

    fn swim_tick(&mut self, c: &mut Creature, b: &'static Beast) -> bool {
        c.move_cd = c.move_cd.saturating_sub(1);
        c.attack_cd = c.attack_cd.saturating_sub(1);
        if self.world.get(c.pos).map_or(true, |t| t.water == 0) {
            // Out of the water (it drained, or froze): back in, or it dies.
            match self.water_beside(c.pos) {
                Some(q) => c.pos = q,
                None => {
                    self.count("beasts.stranded", 1);
                    self.creature_died(c.clone());
                    return false;
                }
            }
        }
        if self.tick >= c.leave_tick {
            // Off downstream, or out to sea.
            return false;
        }
        if c.think_cd == 0 {
            c.think_cd = 15;
            if let Species::Beast(i) = c.species {
                self.note_seen(i);
            }
            if b.temper == Temper::Predator && c.foe.is_none() && self.beast_roll(c.id, 0.3) {
                let (pos, me) = (c.pos, c.id);
                c.foe = self
                    .creatures
                    .values()
                    .filter(|o| o.id != me && o.alive && near(o.pos, pos, 5) && faction(o) == Faction::Prey)
                    .filter(|o| o.species.beast().map_or(false, |ob| ob.can(SWIM)) && o.species.info().scale <= b.size.max(0.05))
                    .min_by_key(|o| (dist2(o.pos, pos), o.id))
                    .map(|o| o.id);
            }
        } else {
            c.think_cd -= 1;
        }
        // A big biter takes a dwarf at the water's edge.
        if b.aggro() > 0 && c.attack_cd == 0 {
            if let Some(di) = self.dwarves.iter().position(|d| d.alive && near(d.pos, c.pos, 1)) {
                c.attack_cd = b.attack_delay();
                self.creature_attack(c, di);
                return true;
            }
        }
        let goal = match c.foe.and_then(|f| self.creatures.get(&f).filter(|o| o.alive).map(|o| o.pos)) {
            Some(fp) if near(fp, c.pos, 1) => {
                if c.attack_cd == 0 {
                    c.attack_cd = b.attack_delay();
                    let f = c.foe.unwrap();
                    self.creature_maul(c, f);
                }
                return true;
            }
            Some(fp) if near(fp, c.pos, 8) => Some(fp),
            _ => {
                c.foe = None;
                None
            }
        };
        if c.move_cd > 0 {
            return true;
        }
        // Swimming about: towards a spot of water, picked now and then.
        let goal = goal.or_else(|| c.path.first().copied());
        let goal = match goal {
            Some(g) if g != c.pos => g,
            _ => {
                let at = c.pos;
                let Some(g) = self.with_own_dice(c.id as u64, |g| g.water_near(at, 8)) else { return true };
                c.path = vec![g];
                g
            }
        };
        let here = dist2(c.pos, goal);
        let mut best: Option<(i32, Pos)> = None;
        for &(dx, dy) in DIRS8.iter() {
            let q = c.pos.offset(dx, dy, 0);
            if self.world.get(q).map_or(false, |t| t.water > 0 && !t.kind.solid() && !t.frozen) {
                let d = dist2(q, goal);
                if d < here && best.map_or(true, |(bd, _)| d < bd) {
                    best = Some((d, q));
                }
            }
        }
        match best {
            Some((_, q)) => {
                c.pos = q;
                c.move_cd = b.speed as u32 + 1;
            }
            // No way on through the water: pick somewhere else.
            None => {
                c.path.clear();
                c.move_cd = 5;
            }
        }
        true
    }

    fn water_beside(&self, p: Pos) -> Option<Pos> {
        DIRS8.iter().map(|&(dx, dy)| p.offset(dx, dy, 0)).chain([p.down()]).find(|&q| self.world.get(q).map_or(false, |t| t.water > 0 && !t.kind.solid()))
    }

    // ------------------------------------------------------------------
    // Walkers (every 15 ticks or so, from `creature_think`)
    // ------------------------------------------------------------------

    /// A table animal on its feet decides what to do, by its temperament.
    /// False for anything else (it thinks the usual way).
    pub(super) fn beast_think(&mut self, c: &mut Creature) -> bool {
        let Species::Beast(i) = c.species else { return false };
        if c.siege {
            return false;
        }
        let b = bestiary::beast(i);
        self.note_seen(i);
        match b.temper {
            Temper::Grazer | Temper::Prey => {
                if !self.bolt(c, b, 4) {
                    self.keep_with_herd(c);
                }
            }
            Temper::Vermin => {
                if !self.bolt(c, b, 3) {
                    self.raid_stores(c);
                }
            }
            Temper::Scavenger => {
                if !self.bolt(c, b, 3) {
                    self.seek_carrion(c);
                }
            }
            Temper::Ambusher => self.lie_in_wait(c, b),
            Temper::Territorial => self.hold_ground(c, b),
            Temper::Predator => self.prowl(c, b),
        }
        // Nothing to do: it roams about a little (ambushers stay put).
        if c.path.is_empty() && c.foe.is_none() && c.target.is_none() && b.temper != Temper::Ambusher && !c.leaving {
            let hostile = b.aggro() > 0;
            if self.beast_roll(c.id ^ 2, if hostile { 0.5 } else { 0.35 }) {
                self.with_own_dice(c.id as u64, |g| g.creature_wander(c, if hostile { 12 } else { 7 }));
            }
        }
        true
    }

    /// Run from dwarves within `r`, from whatever hunts it, and (grazers
    /// and prey) from predators about: well clear, not a step. True if it ran.
    fn bolt(&mut self, c: &mut Creature, b: &Beast, r: i32) -> bool {
        let (me, pos) = (c.id, c.pos);
        let shy = matches!(b.temper, Temper::Grazer | Temper::Prey);
        let mut threats: Vec<Pos> = self.dwarves.iter().filter(|d| d.alive && near(d.pos, pos, r)).map(|d| d.pos).collect();
        threats.extend(
            self.creatures
                .values()
                .filter(|o| o.alive && o.id != me && near(o.pos, pos, 7))
                .filter(|o| o.foe == Some(me) || shy && !o.tame && faction(o) == Faction::Predator && o.species.info().scale >= b.size * 0.4)
                .map(|o| o.pos),
        );
        if threats.is_empty() {
            return false;
        }
        let clear = |q: Pos| threats.iter().all(|t| (t.x - q.x).abs().max((t.y - q.y).abs()) >= 8);
        let w = &self.world;
        let open_air = w.outside(pos);
        if !c.path.first().map_or(false, |&q| clear(q)) {
            match path::search(w, pos, 400, c.mover(), |q| clear(q) && (!open_air || w.outside(q))) {
                Some((_, p)) => c.path = p.into_iter().rev().collect(),
                None => {
                    let score = |q: Pos| threats.iter().map(|t| dist2(*t, q)).min().unwrap_or(0);
                    let mut nb = Vec::new();
                    w.neighbors(pos, &mut nb, c.mover());
                    if let Some(&(best, _)) = nb.iter().max_by_key(|(q, _)| score(*q)) {
                        c.path = vec![best];
                    }
                }
            }
            self.count("beasts.bolts", 1);
        }
        c.think_cd = 5;
        true
    }

    /// A herd animal strayed from the one it came with goes back to it.
    fn keep_with_herd(&mut self, c: &mut Creature) {
        if c.pack == 0 || c.pack == c.id || !c.path.is_empty() {
            return;
        }
        let Some(lead) = self.creatures.get(&c.pack).filter(|o| o.alive).map(|o| o.pos) else { return };
        if near(lead, c.pos, 4) {
            return;
        }
        match path::search(&self.world, c.pos, 500, c.mover(), |q| near(q, lead, 2)) {
            Some((_, p)) => c.path = p.into_iter().rev().collect(),
            None => c.think_cd = 120,
        }
    }

    /// Vermin make for food lying about or stored, and eat it; a cat
    /// beside one catches it.
    fn raid_stores(&mut self, c: &mut Creature) {
        c.think_cd = 40;
        let pos = c.pos;
        let caught = self.creatures.values().any(|o| o.alive && o.tame && near(o.pos, pos, 1) && (o.species == Species::Cat || o.species.beast().map_or(false, |b| b.has(bestiary::HUNTBEAST))));
        if caught {
            self.count("beasts.vermin_caught", 1);
            let name = c.name();
            self.warn(&format!("caught:{}", name), &format!("A cat has caught one of the {}.", c.species.info().plural), (200, 230, 160));
            c.body.dead = true;
            return;
        }
        if !c.path.is_empty() {
            return;
        }
        // Loose food only: barrels keep the vermin out.
        let loose = |it: &crate::items::Item| it.carried_by.is_none() && it.container.is_none() && it.is_food();
        let here = self.items.values().find(|it| it.pos == pos && loose(it)).map(|it| it.id);
        if let Some(id) = here {
            if self.beast_roll(c.id, 0.35) {
                let what = self.items.get(&id).map(|it| it.name()).unwrap_or_default();
                self.items.remove(&id);
                self.count("beasts.vermin_ate", 1);
                // Fed, it makes off.
                c.leaving = true;
                c.path.clear();
                let plural = c.species.info().plural;
                self.warn(&format!("vermin:{}", plural), &format!("{} are at the food: they have eaten {}.", super::combat::cap(plural), what), (230, 190, 110));
            }
            return;
        }
        let food: std::collections::HashSet<Pos> = self.items.values().filter(|it| loose(it) && near(it.pos, pos, 30)).map(|it| it.pos).collect();
        if food.is_empty() {
            return;
        }
        match path::search(&self.world, pos, 2000, c.mover(), |q| food.contains(&q)) {
            Some((_, p)) => c.path = p.into_iter().rev().collect(),
            None => c.think_cd = 300,
        }
    }

    /// Scavengers make for the nearest carcass or corpse, or the
    /// battlefield, and stay by it.
    fn seek_carrion(&mut self, c: &mut Creature) {
        if !c.path.is_empty() {
            return;
        }
        let pos = c.pos;
        let at = |it: &crate::items::Item| matches!(it.kind, ItemKind::Carcass | ItemKind::Corpse) && it.carried_by.is_none();
        if self.items.values().any(|it| at(it) && near(it.pos, pos, 1)) {
            c.think_cd = 60;
            return;
        }
        let mut food: std::collections::HashSet<Pos> = self.items.values().filter(|it| at(it) && near(it.pos, pos, 40)).map(|it| it.pos).collect();
        if let Some(p) = self.fresh_battlefield().filter(|&p| near(p, pos, 40)) {
            food.insert(p);
        }
        if food.is_empty() {
            return;
        }
        match path::search(&self.world, pos, 3000, c.mover(), |q| food.contains(&q)) {
            Some((_, p)) => {
                c.path = p.into_iter().rev().collect();
                self.count("beasts.carrion", 1);
            }
            // No way there: don't look again for a while.
            None => c.think_cd = 300,
        }
    }

    /// An ambusher stays put and strikes at whatever comes close: prey,
    /// or a dwarf if it's big or venomous enough to try.
    fn lie_in_wait(&mut self, c: &mut Creature, b: &Beast) {
        if c.foe.is_some() || c.target.is_some() {
            return;
        }
        c.path.clear();
        let pos = c.pos;
        if b.aggro() > 0 {
            if let Some(d) = self.dwarves.iter().filter(|d| d.alive && near(d.pos, pos, b.aggro())).min_by_key(|d| dist2(d.pos, pos)) {
                c.target = Some(d.id);
                self.count("beasts.ambushes", 1);
                return;
            }
        }
        let me = c.id;
        c.foe = self
            .creatures
            .values()
            .filter(|o| o.id != me && o.alive && !o.tame && near(o.pos, pos, 2) && faction(o) == Faction::Prey && o.species.info().scale <= b.size * 1.5)
            .min_by_key(|o| (dist2(o.pos, pos), o.id))
            .map(|o| o.id);
        if c.foe.is_some() {
            self.count("beasts.ambushes", 1);
        }
    }

    /// The territorial go for a dwarf who comes too close (and give up
    /// when they've backed off); the small ones just make off.
    fn hold_ground(&mut self, c: &mut Creature, b: &Beast) {
        if b.aggro() == 0 {
            self.bolt(c, b, 2);
            return;
        }
        if c.target.is_some() {
            return;
        }
        let pos = c.pos;
        if let Some(d) = self.dwarves.iter().filter(|d| d.alive && near(d.pos, pos, b.aggro())).min_by_key(|d| dist2(d.pos, pos)) {
            c.target = Some(d.id);
            c.path.clear();
            self.count("beasts.territorial", 1);
        }
    }

    /// A predator: hunts prey its size now and then, fights armies and the
    /// dead that come near (if it's big enough), and the big ones size up
    /// the dwarves as wolves do; the small keep clear of dwarves.
    fn prowl(&mut self, c: &mut Creature, b: &Beast) {
        if c.foe.is_none() && !c.leaving && c.wary_until <= self.tick {
            let (me, pos) = (c.id, c.pos);
            let hunting = self.beast_roll(c.id, self.knob("beasts.hunt", 0.15));
            let found = self
                .creatures
                .values()
                .filter(|o| o.id != me && o.alive && !o.sneaking && o.pos.z == pos.z && self.creature_active(o))
                .filter(|o| {
                    let f = faction(o);
                    if f == Faction::Prey {
                        hunting && !o.tame && near(o.pos, pos, 12) && o.species.info().scale <= (b.size * 1.3).max(0.05)
                    } else {
                        b.size >= 0.5 && hostile_to(Faction::Predator, f) && near(o.pos, pos, 3)
                    }
                })
                .min_by_key(|o| (dist2(o.pos, pos), o.id))
                .map(|o| (o.id, o.pos));
            if let Some((id, p)) = found {
                if adjacent(p, pos) || self.can_reach(c, p, 600) {
                    c.foe = Some(id);
                    c.path.clear();
                    self.count("beasts.hunts", 1);
                }
            }
        }
        if c.foe.is_some() {
            return;
        }
        if b.aggro() > 0 {
            self.predator_think(c);
        } else {
            self.bolt(c, b, 3);
        }
    }

    // ------------------------------------------------------------------
    // Kept animals
    // ------------------------------------------------------------------

    /// Once a day, a kept animal of the bestiary gives what it gives alive:
    /// a hen her eggs, a sheep its wool now and then.
    pub(super) fn kept_products(&mut self, c: &Creature) {
        let Some(b) = c.species.beast() else { return };
        // (Every tenth tick is a full turn for every tame animal.)
        if !c.adult || self.tick % 10 != 0 || (self.tick / 10 + c.id as u64) % (DAY / 10) != 0 {
            return;
        }
        if b.gives(bestiary::EGGS) && c.female && self.beast_roll(c.id, 0.5) {
            let id = self.spawn_item(ItemKind::Meat, Mat::Flesh, c.pos);
            if let Some(it) = self.items.get_mut(&id) {
                it.label = Some(format!("{} eggs", b.name));
            }
            self.count("beasts.eggs", 1);
        }
        if b.gives(bestiary::WOOL) && self.beast_roll(c.id ^ 1, 1.0 / 20.0) {
            let id = self.spawn_item(ItemKind::Product, bestiary::product_mat(bestiary::WOOL), c.pos);
            if let Some(it) = self.items.get_mut(&id) {
                it.label = Some(bestiary::product_label(b, bestiary::WOOL));
            }
            self.count("beasts.wool", 1);
        }
    }
}

/// "a" or "an".
fn an(name: &str) -> &'static str {
    if name.starts_with(|ch: char| "aeiou".contains(ch)) {
        "an"
    } else {
        "a"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bestiary::{find, BEASTS};
    use crate::buildings::Recipe;

    fn sp(name: &str) -> Species {
        Species::Beast(find(name).unwrap_or_else(|| panic!("no {}", name)))
    }

    /// Open ground with no creatures about and the dwarves sent well off.
    fn field(seed: u64) -> (Game, Pos) {
        let mut g = Game::new(seed);
        g.creatures.clear();
        let a = g.flat_patch(30, 30, 4).offset(15, 15, 0);
        for d in g.dwarves.iter_mut() {
            if near(d.pos, a, 40) {
                d.alive = false;
            }
        }
        (g, a)
    }

    fn run(g: &mut Game, ticks: u64, mut each: impl FnMut(&Game)) {
        for _ in 0..ticks {
            g.step();
            each(g);
        }
    }

    #[test]
    fn a_grazer_flees_a_predator() {
        let (mut g, a) = field(21);
        let bison = g.spawn_creature(sp("bison"), a, u64::MAX);
        let start = g.creatures[&bison].pos;
        let tiger = g.spawn_creature(Species::Lion, a.offset(3, 0, 0), u64::MAX);
        g.creatures.get_mut(&tiger).unwrap().wary_until = u64::MAX; // it doesn't chase
        let mut far = 0;
        run(&mut g, 300, |g| {
            if let (Some(b), Some(t)) = (g.creatures.get(&bison), g.creatures.get(&tiger)) {
                far = far.max((b.pos.x - t.pos.x).abs().max((b.pos.y - t.pos.y).abs()));
            }
        });
        let b = &g.creatures[&bison];
        assert!(far >= 7, "the bison stayed within {} of the lion", far);
        assert!(dist2(b.pos, start) > 9, "the bison never moved");
        assert!(g.annals.counters.get("beasts.bolts").copied().unwrap_or(0) > 0);
    }

    #[test]
    fn a_predator_hunts_prey() {
        let (mut g, a) = field(22);
        g.knobs.set.insert("beasts.hunt".into(), 1.0);
        let fox = g.spawn_creature(sp("red fox"), a, u64::MAX);
        let rabbit = g.spawn_creature(sp("rabbit"), a.offset(4, 0, 0), u64::MAX);
        let mut chased = false;
        run(&mut g, 600, |g| {
            chased |= g.creatures.get(&fox).map_or(false, |f| f.foe == Some(rabbit));
        });
        assert!(chased, "the fox never went after the rabbit");
        assert!(g.annals.counters.get("beasts.hunts").copied().unwrap_or(0) > 0);
        // A fox doesn't go after an elk.
        let (mut g, a) = field(22);
        g.knobs.set.insert("beasts.hunt".into(), 1.0);
        let fox = g.spawn_creature(sp("red fox"), a, u64::MAX);
        let elk = g.spawn_creature(Species::Elk, a.offset(4, 0, 0), u64::MAX);
        run(&mut g, 300, |g| assert!(g.creatures.get(&fox).map_or(true, |f| f.foe != Some(elk)), "a fox hunting an elk"));
    }

    #[test]
    fn a_scavenger_goes_to_a_corpse() {
        let (mut g, a) = field(23);
        let hyena = g.spawn_creature(sp("spotted hyena"), a, u64::MAX);
        let body = a.offset(12, 3, 0);
        let body = g.spot_near(body, 0);
        let id = g.spawn_item(ItemKind::Carcass, Mat::Flesh, body);
        g.items.get_mut(&id).unwrap().label = Some("deer".into());
        let mut best = i32::MAX;
        run(&mut g, 500, |g| {
            if let Some(h) = g.creatures.get(&hyena) {
                best = best.min(dist2(h.pos, body));
            }
        });
        assert!(best <= 2, "the hyena got no nearer than {} to the carcass", best);
    }

    #[test]
    fn a_scavenger_flier_drops_to_carrion() {
        let (mut g, a) = field(24);
        let body = g.spot_near(a, 0);
        g.spawn_item(ItemKind::Carcass, Mat::Flesh, body);
        let vulture = find("griffon vulture").unwrap();
        let ids = g.spawn_beasts(vulture);
        assert!(!ids.is_empty());
        let mut best = i32::MAX;
        run(&mut g, 2000, |g| {
            for id in &ids {
                if let Some(c) = g.creatures.get(id) {
                    best = best.min((c.pos.x - body.x).abs().max((c.pos.y - body.y).abs()));
                }
            }
        });
        assert!(best <= 1, "no vulture came within {} of the carcass", best);
    }

    #[test]
    fn a_swimmer_stays_in_water() {
        let (mut g, a) = field(25);
        // A pond in the field, kept full and kept from spreading.
        let pond = |g: &mut Game| {
            for dx in -1..9 {
                for dy in -1..7 {
                    let inside = (0..8).contains(&dx) && (0..6).contains(&dy);
                    g.world.t_mut(a.offset(dx, dy, 0)).water = if inside { 3 } else { 0 };
                }
            }
        };
        pond(&mut g);
        let trout = g.spawn_creature(sp("brown trout"), a.offset(3, 3, 0), u64::MAX);
        let start = g.creatures[&trout].pos;
        let mut moved = false;
        for _ in 0..600 {
            pond(&mut g);
            g.step();
            let c = g.creatures.get(&trout).expect("the trout died or left");
            // (The pond may lap a tile over its edge before it's bailed.)
            assert!(g.world.t(c.pos).water > 0, "the trout is out of the water at {:?}", c.pos);
            assert!(c.pos.x >= a.x - 1 && c.pos.x <= a.x + 8 && c.pos.y >= a.y - 1 && c.pos.y <= a.y + 6 && c.pos.z == a.z, "the trout left the pond for {:?}", c.pos);
            moved |= c.pos != start;
        }
        assert!(moved, "the trout never swam about");
        // Drained, it dies (and doesn't walk off).
        for dx in -1..9 {
            for dy in -1..7 {
                g.world.t_mut(a.offset(dx, dy, 0)).water = 0;
            }
        }
        g.step();
        g.step();
        assert!(!g.creatures.contains_key(&trout), "a trout lived on dry land");
    }

    #[test]
    fn a_flier_crosses_the_map() {
        let (mut g, _) = field(26);
        let i = find("starling").unwrap();
        let ids = g.spawn_beasts(i);
        assert!(!ids.is_empty());
        let start: Vec<Pos> = ids.iter().map(|id| g.creatures[id].pos).collect();
        let mut furthest = 0;
        let mut gone_at = None;
        for t in 0..3000 {
            g.step();
            for (k, id) in ids.iter().enumerate() {
                if let Some(c) = g.creatures.get(id) {
                    furthest = furthest.max((c.pos.x - start[k].x).abs().max((c.pos.y - start[k].y).abs()));
                }
            }
            if ids.iter().all(|id| !g.creatures.contains_key(id)) {
                gone_at = Some(t);
                break;
            }
        }
        let span = g.world.w.min(g.world.h);
        assert!(furthest >= span - 4, "the flock got only {} of {} across", furthest, span);
        assert!(gone_at.is_some(), "the flock never left over the far edge");
    }

    #[test]
    fn vermin_raid_the_food_and_cats_catch_them() {
        let (mut g, a) = field(27);
        let food = g.spot_near(a.offset(8, 0, 0), 0);
        for _ in 0..6 {
            g.spawn_item(ItemKind::Meat, Mat::Flesh, food);
        }
        let rat = g.spawn_creature(sp("brown rat"), a, u64::MAX);
        let before = g.items.values().filter(|it| it.kind == ItemKind::Meat).count();
        run(&mut g, 1500, |_| {});
        let after = g.items.values().filter(|it| it.kind == ItemKind::Meat).count();
        assert_eq!(after, before - 1, "a rat eats one meal and makes off");
        assert!(g.creatures.get(&rat).map_or(true, |c| c.leaving), "the fed rat stayed");
        // Food in a barrel is safe from them.
        let (mut g2, a2) = field(27);
        let spot = g2.spot_near(a2.offset(6, 0, 0), 0);
        let barrel = g2.spawn_item(ItemKind::Barrel, Mat::Wood, spot);
        for _ in 0..4 {
            let id = g2.spawn_item(ItemKind::Meat, Mat::Flesh, spot);
            g2.items.get_mut(&id).unwrap().container = Some(barrel);
        }
        g2.spawn_creature(sp("brown rat"), a2, u64::MAX);
        run(&mut g2, 1500, |_| {});
        assert_eq!(g2.items.values().filter(|it| it.kind == ItemKind::Meat).count(), 4, "a rat got into the barrel");
        // A cat beside a rat catches it.
        let rat = g.spawn_creature(sp("brown rat"), a, u64::MAX);
        let cat = g.spawn_tame(Species::Cat, a.offset(1, 0, 0), true, true);
        let _ = cat;
        let mut c = g.creatures.remove(&rat).unwrap();
        c.think_cd = 0;
        g.raid_stores(&mut c);
        assert!(c.body.dead, "the cat let the rat be");
    }

    #[test]
    fn an_ambusher_waits_and_strikes() {
        let (mut g, a) = field(28);
        let croc = g.spawn_creature(sp("crocodile"), a, u64::MAX);
        let start = g.creatures[&croc].pos;
        run(&mut g, 400, |g| assert_eq!(g.creatures[&croc].pos, start, "the crocodile wandered off"));
        let deer = g.spawn_creature(Species::Deer, a.offset(1, 0, 0), u64::MAX);
        g.creatures.get_mut(&deer).unwrap().think_cd = 200;
        let mut struck = false;
        run(&mut g, 60, |g| struck |= g.creatures.get(&croc).map_or(false, |c| c.foe == Some(deer)) || !g.creatures.contains_key(&deer));
        assert!(struck, "the crocodile let the deer walk by");
    }

    #[test]
    fn butchering_yields_the_tables_products() {
        let butcher = |name: &str| {
            let it = crate::items::Item {
                id: 1,
                kind: ItemKind::Carcass,
                mat: Mat::Flesh,
                pos: Pos::new(0, 0, 0),
                carried_by: None,
                claimed_by: None,
                label: Some(name.into()),
                container: None,
                quality: 0,
                to_depot: false,
                dump: false,
                wear: 0,
                forbidden: false,
            };
            Recipe::Butcher.produce(&[it])
        };
        let sheep = butcher("sheep");
        assert!(sheep.iter().filter(|o| o.0 == ItemKind::Meat).count() >= 2);
        assert!(sheep.iter().any(|o| o.0 == ItemKind::Hide) && sheep.iter().any(|o| o.0 == ItemKind::Tallow) && sheep.iter().any(|o| o.0 == ItemKind::Bone));
        assert!(sheep.iter().any(|o| o.0 == ItemKind::Product && o.2.as_deref() == Some("sheep wool")));
        let walrus = butcher("walrus");
        assert!(walrus.iter().any(|o| o.0 == ItemKind::Product && o.2.as_deref() == Some("walrus ivory")));
        let trout = butcher("brown trout");
        assert!(trout.iter().all(|o| o.0 != ItemKind::Meat) && trout.iter().any(|o| o.0 == ItemKind::Fish));
        let crab = butcher("giant crab");
        assert!(crab.iter().any(|o| o.2.as_deref() == Some("giant crab chitin")) && crab.iter().all(|o| o.0 != ItemKind::Hide));
        // Every animal gives something, and what its products say.
        for (i, b) in BEASTS.iter().enumerate() {
            let out = crate::bestiary::carcass_yields(i as u16);
            assert!(!out.is_empty(), "{} gives nothing", b.name);
            if b.gives(crate::bestiary::IVORY) {
                assert!(out.iter().any(|o| o.0 == ItemKind::Product), "{}", b.name);
            }
        }
        // The old named animals still butcher as before.
        assert_eq!(butcher("bear").iter().filter(|o| o.0 == ItemKind::Meat).count(), 7);
        // A slain table animal's carcass is labelled for the butcher.
        let (mut g, a) = field(29);
        let id = g.spawn_creature(sp("walrus"), a, u64::MAX);
        let c = g.creatures.remove(&id).unwrap();
        g.creature_died(c);
        assert!(g.items.values().any(|it| it.kind == ItemKind::Carcass && it.label.as_deref() == Some("walrus")));
    }

    #[test]
    fn kept_hens_lay_eggs() {
        let (mut g, a) = field(30);
        let hen = g.spawn_tame(Species::Beast(crate::bestiary::CHICKEN), a, true, true);
        let _ = hen;
        run(&mut g, DAY * 6, |_| {});
        let eggs = g.items.values().filter(|it| it.kind == ItemKind::Meat && it.label.as_deref() == Some("chicken eggs")).count();
        assert!(eggs >= 1, "no eggs in six days");
        let it = g.items.values().find(|it| it.label.as_deref() == Some("chicken eggs")).unwrap();
        assert_eq!(it.name(), "chicken eggs");
    }

    #[test]
    fn the_site_draws_its_animals_from_the_region() {
        let g = Game::new(31);
        let v = g.site_beasts();
        assert!(v.len() >= 8, "only {} kinds about the site", v.len());
        let (sx, sy) = g.site;
        let r = g.overworld.region_at(sx, sy).expect("a region");
        let regional: Vec<u16> = r.fauna.iter().filter_map(|s| if let Species::Beast(i) = s { Some(*i) } else { None }).collect();
        assert!(!regional.is_empty(), "no bestiary animals in {}", r.name);
        for i in &regional {
            assert!(v.contains(i), "{} of the region can't come", crate::bestiary::beast(*i).name);
        }
        // Everything that may come fits the land or the water.
        let bit = crate::bestiary::biome_bit(g.biome);
        let wet = crate::bestiary::FRESH | crate::bestiary::SEA | crate::bestiary::COAST;
        let lands = r.biomes.iter().fold(0, |m, b| m | crate::bestiary::biome_bit(b.0));
        for &i in &v {
            let b = crate::bestiary::beast(i);
            assert!(b.home & (bit | lands | wet) != 0, "{} doesn't belong here", b.name);
        }
    }

    #[test]
    fn saves_keep_the_beasts_and_old_saves_load() {
        let (mut g, a) = field(33);
        let id = g.spawn_creature(sp("musk ox"), a, u64::MAX);
        g.note_seen(find("musk ox").unwrap());
        let json = serde_json::to_string(&g).unwrap();
        let back: Game = serde_json::from_str(&json).unwrap();
        assert_eq!(back.creatures[&id].species, sp("musk ox"));
        assert_eq!(back.beasts_seen, g.beasts_seen);
        // A save from before the bestiary: no beasts_seen at all.
        let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
        v.as_object_mut().unwrap().remove("beasts_seen");
        let old: Game = serde_json::from_value(v).unwrap();
        assert!(old.beasts_seen.is_empty());
    }

    #[test]
    fn the_sites_animals_come_by_over_a_month() {
        let mut g = Game::new(34);
        let mut kinds: Vec<u16> = Vec::new();
        for _ in 0..30 {
            g.tick = g.tick / DAY * DAY + DAY / 2;
            g.beast_visit();
            for c in g.creatures.values() {
                if let Species::Beast(i) = c.species {
                    if !kinds.contains(&i) {
                        kinds.push(i);
                    }
                }
            }
            g.tick += DAY;
        }
        let visits = g.annals.counters.get("beasts.visits").copied().unwrap_or(0);
        assert!(visits >= 5 && kinds.len() >= 3, "{} visits, {} kinds in a month", visits, kinds.len());
    }

    /// What the bestiary's comings cost: a season of a fort with and
    /// without them (`cargo test --release beasts_cost -- --ignored --nocapture`).
    #[test]
    #[ignore]
    fn beasts_cost() {
        for seed in [5u64, 6, 7] {
            for chance in [0.0, 0.35, 0.0, 0.35] {
                let mut g = Game::new(seed);
                g.knobs.set.insert("beasts.chance".into(), chance);
                g.set_overseer(Some(crate::game::overseer::Style::Steady));
                let t = std::time::Instant::now();
                for _ in 0..SEASON {
                    g.step();
                }
                let beasts = g.creatures.values().filter(|c| matches!(c.species, Species::Beast(_))).count();
                eprintln!("seed {} chance {}: {:?}, {} creatures ({} beasts)", seed, chance, t.elapsed(), g.creatures.len(), beasts);
            }
        }
    }

    #[test]
    fn spawning_stays_cheap_and_bounded() {
        let mut g = Game::new(32);
        let t = std::time::Instant::now();
        for _ in 0..200 {
            g.beast_visit();
        }
        let el = t.elapsed();
        assert!(el.as_millis() < 3000, "200 visits took {:?}", el);
        let about = g.creatures.values().filter(|c| matches!(c.species, Species::Beast(_)) && !c.species.dwells_underground()).count();
        assert!(about <= CAP + 12, "{} beasts about", about);
        // Each cavern layer has its animals once the miners break in, the
        // first time only.
        assert!(!g.creatures.values().any(|c| c.species.dwells_underground() && matches!(c.species, Species::Beast(_))), "cave beasts before the caverns were found");
        let mut bands = g.world.caverns.clone();
        bands.sort();
        for (k, &(lo, hi)) in bands.iter().enumerate() {
            let before = g.creatures.len();
            let at = Pos::new(g.world.w / 2, g.world.h / 2, (lo + hi) / 2);
            g.wake_cavern(at);
            let mut kinds: Vec<Species> = g.creatures.values().filter(|c| c.species.dwells_underground() && c.pos.z >= lo && c.pos.z <= hi).map(|c| c.species).collect();
            kinds.dedup_by_key(|s| format!("{:?}", s));
            kinds.sort_by_key(|s| format!("{:?}", s));
            kinds.dedup();
            assert!(g.creatures.len() > before && kinds.len() >= 2, "layer {} woke with {} kinds", k, kinds.len());
            for i in kinds.iter().filter_map(|s| s.beast()) {
                assert!(i.home & crate::bestiary::cave_bit(k) != 0, "{} in layer {}", i.name, k);
            }
            let n = g.creatures.len();
            g.wake_cavern(at);
            assert_eq!(g.creatures.len(), n, "woke twice");
        }
    }
}
