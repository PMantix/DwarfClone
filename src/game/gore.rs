//! Gore: what a fight leaves behind. Blood spattered where blows land,
//! pools where someone bled hard or died, trails behind the wounded who
//! walk on, severed parts lying where they fell, smoke over burning ground,
//! furniture smashed by heavy blows, and what the dwarves who saw it think.
//!
//! All of it is sparse and capped: stains and smoke live in small maps keyed
//! by position (never on `Tile`), and fade by themselves. Cosmetic chance
//! (which way blood sprays) comes from a hash, not the game's dice, so the
//! mess never changes how a fight goes.

use super::combat::near;
use super::*;
use crate::body::Part;
use crate::creature::{Creature, Species};

/// Most blood stains kept; past this the oldest are forgotten first.
pub const MAX_STAINS: usize = 2500;
/// Most smoky tiles kept.
pub const MAX_SMOKE: usize = 800;
/// Ticks between stain upkeep (fading, washing).
pub const GORE_TICK: u64 = 100;
/// Ticks between smoke drifting and thinning.
pub const SMOKE_TICK: u64 = 10;
/// A stain this big is a pool.
pub const POOL: u8 = 5;
/// Most blood one tile can hold.
const MAX_AMT: u8 = 9;

/// The colour family of what bleeds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blood {
    Red,
    /// Yellow-green: spiders, scorpions.
    Ichor,
    /// Black and clotted: the walking dead.
    Black,
    /// Violet: things from the deep caverns.
    Violet,
}

impl Blood {
    pub fn fluid(self) -> &'static str {
        match self {
            Blood::Red => "blood",
            Blood::Ichor => "ichor",
            Blood::Black => "black ichor",
            Blood::Violet => "ichor",
        }
    }

    /// Colour at full strength.
    pub fn rgb(self) -> (u8, u8, u8) {
        match self {
            Blood::Red => (150, 10, 10),
            Blood::Ichor => (120, 140, 20),
            Blood::Black => (40, 30, 30),
            Blood::Violet => (110, 30, 140),
        }
    }
}

/// What bleeds this colour: None for a dwarf.
pub fn blood_of(who: Option<Species>) -> Blood {
    match who {
        Some(Species::Undead | Species::Husk) => Blood::Black,
        Some(Species::GiantSpider | Species::Scorpion) => Blood::Ichor,
        Some(Species::CaveHorror) => Blood::Violet,
        _ => Blood::Red,
    }
}

/// "dwarf", "goblin", "wolf".
pub fn bleeder(who: Option<Species>) -> &'static str {
    match who {
        None => "dwarf",
        Some(Species::Human) => "human",
        Some(Species::Elf) => "elf",
        Some(Species::Raider) => "dwarf",
        Some(Species::Undead) => "corpse",
        Some(s) => s.info().name,
    }
}

/// Blood on one tile: whose, how much (1..=9), and when it was last added to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stain {
    /// None for a dwarf.
    pub who: Option<Species>,
    pub amt: u8,
    pub at: u64,
}

impl Stain {
    pub fn blood(&self) -> Blood {
        blood_of(self.who)
    }

    /// "a pool of goblin blood", "spatters of dwarf blood".
    pub fn describe(&self) -> String {
        let what = format!("{} {}", bleeder(self.who), self.blood().fluid());
        match self.amt {
            0..=1 => format!("a few drops of {}", what),
            2..=4 => format!("spatters of {}", what),
            5..=7 => format!("a pool of {}", what),
            _ => format!("a great pool of {}", what),
        }
    }

    /// The colour to draw it in: darker as it thins.
    pub fn rgb(&self) -> (u8, u8, u8) {
        let (r, g, b) = self.blood().rgb();
        let k = 0.45 + 0.55 * (self.amt.min(MAX_AMT) as f32 / MAX_AMT as f32);
        ((r as f32 * k) as u8, (g as f32 * k) as u8, (b as f32 * k) as u8)
    }
}

/// The stains, the smoke, and where the last fight was.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Gore {
    #[serde(with = "pos_map", default)]
    pub stains: BTreeMap<Pos, Stain>,
    /// Smoke density (1..=6) on a tile; it drifts and thins, and isn't saved.
    #[serde(skip)]
    pub smoke: BTreeMap<Pos, u8>,
    /// The freshest battlefield: where the last notable blow fell, and when.
    #[serde(default)]
    pub field: Option<(Pos, u64)>,
    /// After finding nothing to clean, nobody looks again until this tick.
    #[serde(skip)]
    clean_retry: u64,
}

/// A map keyed by position, saved as a list (JSON keys must be strings).
mod pos_map {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S: Serializer, V: Serialize>(m: &BTreeMap<Pos, V>, s: S) -> Result<S::Ok, S::Error> {
        let v: Vec<(&Pos, &V)> = m.iter().collect();
        v.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>, V: Deserialize<'de>>(d: D) -> Result<BTreeMap<Pos, V>, D::Error> {
        let v: Vec<(Pos, V)> = Vec::deserialize(d)?;
        Ok(v.into_iter().collect())
    }
}

/// A cheap, steady hash of a place and a moment (cosmetic chance only).
pub(super) fn mix(p: Pos, salt: u64) -> u64 {
    let mut h = (p.x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (p.y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ (p.z as u64).wrapping_mul(0x1656_67B1_9E37_79F9) ^ salt.wrapping_mul(0x2545_F491_4F6C_DD1D);
    h ^= h >> 29;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^ (h >> 32)
}

/// "Urist's", "Ilus'".
pub fn possessive(name: &str) -> String {
    if name.ends_with('s') {
        format!("{}'", name)
    } else {
        format!("{}'s", name)
    }
}

/// Who a part item belongs to, for its label: a dwarf's first name, a
/// named creature's name, or "goblin".
pub fn owner_of_creature(c: &Creature) -> String {
    c.label.clone().unwrap_or_else(|| bleeder(Some(c.species)).to_string())
}

impl Game {
    /// The stain on `p`, if any.
    pub fn stain_at(&self, p: Pos) -> Option<&Stain> {
        self.gore.stains.get(&p)
    }

    /// Smoke density on `p` (0 for none).
    pub fn smoke_at(&self, p: Pos) -> u8 {
        self.gore.smoke.get(&p).copied().unwrap_or(0)
    }

    /// Add `amt` of `who`'s blood on `p` (it soaks into whatever it lands on;
    /// it doesn't hang in the air or soak into rock).
    pub(super) fn bleed_on(&mut self, p: Pos, who: Option<Species>, amt: u8) {
        if amt == 0 || !self.world.in_bounds(p) {
            return;
        }
        let t = self.world.t(p);
        if t.kind.solid() || t.water > 2 || t.magma > 0 {
            return;
        }
        let tick = self.tick;
        if !self.gore.stains.contains_key(&p) && self.gore.stains.len() >= MAX_STAINS {
            self.forget_old_stains();
        }
        let s = self.gore.stains.entry(p).or_insert(Stain { who, amt: 0, at: tick });
        // The freshest and the most blood colour the tile.
        if s.amt <= amt {
            s.who = who;
        }
        s.amt = (s.amt + amt).min(MAX_AMT);
        s.at = tick;
    }

    /// At the cap: forget the oldest tenth of the stains in one go.
    fn forget_old_stains(&mut self) {
        let mut ages: Vec<(u64, Pos)> = self.gore.stains.iter().map(|(&p, s)| (s.at, p)).collect();
        ages.sort_unstable();
        for (_, p) in ages.into_iter().take(MAX_STAINS / 10 + 1) {
            self.gore.stains.remove(&p);
        }
    }

    /// Blood thrown by a blow at `at` struck from `from`: on the tile, and
    /// for a heavy blow on the tile beyond as well.
    pub(super) fn spatter(&mut self, at: Pos, from: Pos, who: Option<Species>, amt: u8) {
        self.bleed_on(at, who, amt);
        if amt >= 2 {
            let beyond = self.beyond(at, from, amt as u64);
            self.bleed_on(beyond, who, amt / 2);
        }
    }

    /// A tile just past `at`, away from `from` (and a little to either
    /// side, by the hash), that something could land on; `at` if none.
    fn beyond(&self, at: Pos, from: Pos, salt: u64) -> Pos {
        let (dx, dy) = ((at.x - from.x).signum(), (at.y - from.y).signum());
        let h = mix(at, self.tick ^ salt.wrapping_mul(977));
        let (dx, dy) = match (dx, dy, h % 4) {
            (0, 0, _) => DIRS8[(h % 8) as usize],
            (dx, dy, 0) if dx == 0 => (if h & 8 == 0 { 1 } else { -1 }, dy),
            (dx, dy, 0) if dy == 0 => (dx, if h & 8 == 0 { 1 } else { -1 }),
            (dx, dy, _) => (dx, dy),
        };
        let q = at.offset(dx, dy, 0);
        if self.world.in_bounds(q) && self.world.walkable(q) {
            q
        } else {
            at
        }
    }

    /// The wounded leave a trail where they walk, and a pool where they
    /// lie bleeding (from the body tick: `rate` is the body's bleed rate).
    pub(super) fn blood_trail(&mut self, p: Pos, who: Option<Species>, rate: i32) {
        if rate < 3 {
            return;
        }
        let amt = if rate >= 30 { 2 } else { 1 };
        self.bleed_on(p, who, amt);
    }

    /// Where someone died: a pool beneath them.
    pub(super) fn death_pool(&mut self, p: Pos, who: Option<Species>, big: bool) {
        self.bleed_on(p, who, if big { 7 } else { 5 });
    }

    /// Every `GORE_TICK`: blood dries and fades, slowly under a roof, faster
    /// in the open, fast in the rain, and at once under water.
    pub(super) fn gore_tick(&mut self) {
        if self.gore.stains.is_empty() {
            return;
        }
        let tick = self.tick;
        let raining = self.weather.wet();
        let fade_out = self.knob("gore.fade_outside", (DAY / 2) as f64).max(GORE_TICK as f64) as u64;
        let fade_in = self.knob("gore.fade_inside", SEASON as f64).max(GORE_TICK as f64) as u64;
        let w = &self.world;
        self.gore.stains.retain(|&p, s| {
            let Some(t) = w.get(p) else { return false };
            if t.water > 0 || t.magma > 0 || t.kind.solid() {
                return false;
            }
            let outside = w.outside(p);
            let age = tick.saturating_sub(s.at);
            let period = if outside { fade_out } else { fade_in };
            if outside && raining {
                // The rain washes it away.
                s.amt = s.amt.saturating_sub(2);
            } else if age >= period && age % period < GORE_TICK {
                s.amt -= 1;
            }
            s.amt > 0
        });
    }

    /// Scrub the blood off `p` (the cleaning job). Returns whether there was any.
    pub fn clean_stain(&mut self, p: Pos) -> bool {
        self.gore.stains.remove(&p).is_some()
    }

    /// The cleaning job, for a hauler with nothing else to carry: scrub the
    /// blood off a floor inside the fort (out in the open the weather sees
    /// to it), once the fighting is over.
    pub(super) fn find_clean(&mut self, d: &mut Dwarf) -> bool {
        if self.tick < self.gore.clean_retry || self.gore.stains.is_empty() {
            return false;
        }
        let tick = self.tick;
        let w = &self.world;
        let spots: HashSet<Pos> = self
            .gore
            .stains
            .iter()
            .filter(|(p, s)| tick >= s.at + 100 && w.t(**p).revealed && !w.outside(**p) && !self.claimed.contains(*p))
            .map(|(p, _)| *p)
            .take(400)
            .collect();
        if spots.is_empty() || self.hostile_count() > 0 {
            self.gore.clean_retry = tick + 300;
            return false;
        }
        let Some((p, path)) = path::search(&self.world, d.pos, SEARCH_LIMIT, Mover::Dwarf, |q| spots.contains(&q)) else {
            self.gore.clean_retry = tick + 300;
            return false;
        };
        self.claimed.insert(p);
        d.task = Task::Clean { pos: p };
        Self::set_path(d, path);
        true
    }

    pub(super) fn do_clean(&mut self, d: &mut Dwarf, pos: Pos) {
        if !self.gore.stains.contains_key(&pos) {
            self.release_task(d);
            return;
        }
        match self.travel(d, pos) {
            Travel::Moving => {}
            Travel::Failed => self.release_task(d),
            Travel::Arrived => {
                let amt = self.gore.stains.get(&pos).map_or(1, |s| s.amt) as u32;
                if Self::labor(d, 10 + 6 * amt, 0) {
                    self.clean_stain(pos);
                    self.claimed.remove(&pos);
                    d.task = Task::Idle;
                    self.count("gore.cleaned", 1);
                }
            }
        }
    }

    /// Smoke rises from `p` (fire, dragon's breath).
    pub(super) fn puff(&mut self, p: Pos, density: u8) {
        if !self.world.in_bounds(p) || self.world.solid(p) {
            return;
        }
        if !self.gore.smoke.contains_key(&p) && self.gore.smoke.len() >= MAX_SMOKE {
            return;
        }
        let s = self.gore.smoke.entry(p).or_insert(0);
        *s = (*s).max(density.min(6));
    }

    /// Every `SMOKE_TICK`: smoke drifts a little and thins, quickly under
    /// the open sky, slowly in the halls.
    pub(super) fn smoke_tick(&mut self) {
        if self.gore.smoke.is_empty() {
            return;
        }
        let tick = self.tick;
        let mut drift: Vec<(Pos, u8)> = Vec::new();
        let w = &self.world;
        self.gore.smoke.retain(|&p, d| {
            let outside = w.outside(p);
            let h = mix(p, tick);
            if *d >= 3 {
                let (dx, dy) = DIRS8[(h % 8) as usize];
                let q = if outside && h % 5 == 0 { p.up() } else { p.offset(dx, dy, 0) };
                if w.in_bounds(q) && !w.solid(q) {
                    drift.push((q, *d - 2));
                }
            }
            let thin = if outside { 1 + (h % 3 == 0) as u8 } else { (h % 3 == 0) as u8 };
            *d = d.saturating_sub(thin);
            *d > 0
        });
        for (q, d) in drift {
            if d > 0 && (self.gore.smoke.contains_key(&q) || self.gore.smoke.len() < MAX_SMOKE) {
                let s = self.gore.smoke.entry(q).or_insert(0);
                *s = (*s).max(d);
            }
        }
    }

    /// Drop a severed part (or teeth) as an item near `at`, flung away from
    /// `from`: "Urist's left arm".
    pub(super) fn drop_part(&mut self, owner: &str, part: Part, teeth: u8, at: Pos, from: Pos) -> u32 {
        let lands = self.beyond(at, from, 31 + teeth as u64);
        let (label, mat) = if teeth > 0 {
            let n = if teeth == 1 { "tooth".to_string() } else { format!("{} teeth", teeth) };
            (format!("{} {}", possessive(owner), n), Mat::Bone)
        } else {
            (format!("{} {}", possessive(owner), part.name()), Mat::Flesh)
        };
        let id = self.spawn_item(ItemKind::BodyPart, mat, lands);
        if let Some(it) = self.items.get_mut(&id) {
            it.label = Some(label);
        }
        id
    }

    /// Something big enough sends the one struck sprawling a tile back:
    /// where they land (`None` if there's nowhere to fall). Furniture or a
    /// door where they land is smashed.
    pub(super) fn knock_back(&mut self, at: Pos, from: Pos, big: bool) -> Option<Pos> {
        let (dx, dy) = ((at.x - from.x).signum(), (at.y - from.y).signum());
        if dx == 0 && dy == 0 {
            return None;
        }
        let q = at.offset(dx, dy, 0);
        if !self.world.in_bounds(q) || !self.world.walkable(q) || self.world.t(q).door == world::DOOR_LOCKED {
            return None;
        }
        let bid = self.world.t(q).building;
        if big && bid != 0 {
            let smash = self.buildings.get(&bid).map_or(false, |b| b.built && matches!(b.kind, BuildingKind::Table | BuildingKind::Chair | BuildingKind::Bed | BuildingKind::Door));
            if smash {
                let name = self.buildings[&bid].name().to_lowercase();
                if let Some(mut b) = self.buildings.remove(&bid) {
                    // Splinters: nothing to salvage.
                    b.built = false;
                    self.buildings.insert(bid, b);
                }
                self.remove_building(bid);
                self.count("gore.smashed", 1);
                self.msg(&format!("The {} is smashed to pieces in the fighting.", name), (230, 150, 110));
            }
        }
        Some(q)
    }

    /// Mark a fresh battlefield (for the director's camera).
    pub(super) fn mark_battlefield(&mut self, p: Pos) {
        self.gore.field = Some((p, self.tick));
    }

    /// The freshest battlefield, if the blood there is still wet (a day).
    pub fn fresh_battlefield(&self) -> Option<Pos> {
        self.gore.field.filter(|&(_, t)| self.tick <= t + DAY).map(|(p, _)| p)
    }

    /// Dwarves who saw something terrible at `at` think about it (`what`:
    /// "saw a goblin's arm cut off"). Each sight hardens them a little; a
    /// seasoned soldier hardly notices. `weight` is the stress to the
    /// green and squeamish.
    pub(super) fn witness(&mut self, at: Pos, what: &str, weight: i32, except: Option<u32>) {
        let tick = self.tick;
        for d in self.dwarves.iter_mut().filter(|d| d.alive && !d.asleep && Some(d.id) != except && near(d.pos, at, 6) && d.pos.z == at.z) {
            if d.body.unconscious() || tick < d.gore_at + 150 {
                continue;
            }
            d.gore_at = tick;
            let seen = d.gore_seen.min(12) as i32;
            d.gore_seen = d.gore_seen.saturating_add(1);
            let mut stress = weight * (14 - seen) / 14;
            if d.fight_level() >= 3 {
                stress /= 2;
            }
            if stress <= 1 {
                d.think("was unmoved by the carnage", 0, tick);
            } else {
                d.think(what, stress, tick);
            }
        }
    }

    /// Every body tick for a dwarf: a bleeding dwarf leaves a trail; one
    /// who walks through a pool of blood minds it.
    pub(super) fn gore_underfoot(&mut self, d: &mut Dwarf) {
        let rate = d.body.bleed_rate();
        if rate > 0 {
            self.blood_trail(d.pos, None, rate);
        }
        if d.path.is_empty() || self.tick < d.gore_at + 300 {
            return;
        }
        let Some(s) = self.gore.stains.get(&d.pos).copied() else { return };
        if s.amt >= POOL && s.who != None {
            d.gore_at = self.tick;
            let seen = d.gore_seen.min(12) as i32;
            let stress = 6 * (14 - seen) / 14;
            d.think(&format!("waded through {}", s.describe()), stress, self.tick);
        }
    }

    /// Whether a fighter is standing beside `p` (for picking a free tile beside a foe).
    pub(super) fn someone_at(&self, p: Pos, except: u32) -> bool {
        self.creatures.values().any(|o| o.id != except && o.alive && o.pos == p)
            || self.dwarves.iter().any(|d| d.alive && d.id != except && d.pos == p && !d.body.unconscious())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clear_ground(g: &Game) -> Pos {
        let e = g.embark;
        for r in 0..20 {
            for dx in -r..=r {
                for dy in -r..=r {
                    let x = e.x + dx;
                    let y = e.y + dy;
                    if x < 2 || y < 2 || x >= g.world.w - 2 || y >= g.world.h - 2 {
                        continue;
                    }
                    let p = Pos::new(x, y, g.world.surface_z(x, y));
                    if g.world.walkable(p) && g.world.outside(p) && g.world.t(p).water == 0 && g.world.t(p).building == 0 {
                        return p;
                    }
                }
            }
        }
        e
    }

    #[test]
    fn blood_fades_outside_and_rain_washes_it() {
        let mut g = Game::new(5);
        g.weather = Weather::Clear;
        let p = clear_ground(&g);
        g.bleed_on(p, Some(Species::Goblin), 6);
        assert_eq!(g.stain_at(p).map(|s| s.describe()), Some("a pool of goblin blood".to_string()));
        let mut gone = None;
        for k in 1..(DAY * 6 / GORE_TICK) {
            g.tick += GORE_TICK;
            g.gore_tick();
            if g.stain_at(p).is_none() {
                gone = Some(k * GORE_TICK);
                break;
            }
        }
        let gone = gone.expect("blood outside never faded");
        assert!(gone >= DAY, "faded too fast: {} ticks", gone);
        // Rain washes a pool away within a few hours.
        g.bleed_on(p, None, 9);
        g.weather = Weather::Rain;
        for _ in 0..6 {
            g.tick += GORE_TICK;
            g.gore_tick();
        }
        assert!(g.stain_at(p).is_none(), "the rain didn't wash it away");
    }

    #[test]
    fn stains_are_capped_and_cheap() {
        let mut g = Game::new(5);
        let z = g.embark.z;
        let mut n = 0;
        for x in 0..g.world.w {
            for y in 0..g.world.h {
                for dz in -2..=2 {
                    g.tick += 1;
                    g.bleed_on(Pos::new(x, y, z + dz), None, 3);
                    n += 1;
                }
            }
        }
        assert!(n > MAX_STAINS * 3);
        assert!(g.gore.stains.len() <= MAX_STAINS, "{} stains kept", g.gore.stains.len());
        // Upkeep over a full map of stains stays quick.
        let t = std::time::Instant::now();
        for _ in 0..100 {
            g.tick += GORE_TICK;
            g.gore_tick();
        }
        let ms = t.elapsed().as_secs_f64() * 1000.0 / 100.0;
        assert!(ms < 5.0, "gore_tick took {:.2} ms", ms);
        // And they save and load.
        let back: Gore = serde_json::from_str(&serde_json::to_string(&g.gore).unwrap()).unwrap();
        assert_eq!(back.stains, g.gore.stains);
    }

    #[test]
    fn a_bleeding_walker_leaves_a_trail() {
        let mut g = Game::new(5);
        g.creatures.clear();
        g.weather = Weather::Clear;
        let start = g.dwarves[0].pos;
        {
            let d = &mut g.dwarves[0];
            d.body.wound(Part::LeftLeg, 9, true);
            assert!(d.body.bleed_rate() > 0);
        }
        // Walk a few steps: every tile walked while bleeding is marked.
        let mut walked = vec![start];
        for _ in 0..6 {
            let mut nb = Vec::new();
            let at = g.dwarves[0].pos;
            g.world.neighbors(at, &mut nb, Mover::Dwarf);
            let Some(&(next, _)) = nb.iter().find(|(q, _)| !walked.contains(q) && q.z == at.z) else { break };
            let mut d = std::mem::take(&mut g.dwarves[0]);
            d.pos = next;
            d.path = vec![next];
            g.gore_underfoot(&mut d);
            g.dwarves[0] = d;
            walked.push(next);
        }
        let marked = walked[1..].iter().filter(|p| g.stain_at(**p).is_some()).count();
        assert!(marked >= 4, "only {} of {} steps left blood", marked, walked.len() - 1);
        assert!(walked[1..].iter().all(|p| g.stain_at(*p).map_or(true, |s| s.amt < POOL)), "a trail, not pools");
    }

    #[test]
    fn smoke_rises_with_fire_and_clears() {
        let mut g = Game::new(5);
        g.weather = Weather::Clear;
        // A tree to burn.
        let tree = (0..g.world.tiles.len()).map(|i| g.world.pos_of(i)).find(|&p| g.world.t(p).kind == TileKind::Tree).expect("a tree");
        assert!(g.ignite(tree));
        let mut most = 0;
        for _ in 0..DAY * 2 {
            g.step();
            most = most.max(g.gore.smoke.len());
            if most > 0 && g.world.tiles.iter().all(|t| t.fire == 0) {
                break;
            }
        }
        assert!(most > 0, "no smoke from the fire");
        // Put any fire out and let the smoke thin.
        for t in g.world.tiles.iter_mut() {
            t.fire = 0;
        }
        g.fire_cells = None;
        for _ in 0..400 {
            g.step();
        }
        assert!(g.gore.smoke.is_empty(), "smoke lingered: {}", g.gore.smoke.len());
    }

    #[test]
    fn a_heavy_blow_sends_one_sprawling_into_the_furniture() {
        let mut g = Game::new(5);
        let p = clear_ground(&g);
        let q = p.offset(1, 0, 0);
        if !g.world.walkable(q) {
            return;
        }
        let from = p.offset(-1, 0, 0);
        assert_eq!(g.knock_back(p, from, false), Some(q));
        assert_eq!(g.knock_back(p, p, true), None, "nowhere to be knocked from");
    }
}
