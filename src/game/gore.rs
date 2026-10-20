//! Gore: what a fight leaves behind. Blood spattered where blows land,
//! pools where someone bled hard or died, trails behind the wounded who
//! walk on, severed parts lying where they fell, smoke over burning ground,
//! furniture smashed by heavy blows, and what the dwarves who saw it think.
//!
//! All of it is sparse and capped: stains and smoke live in small maps keyed
//! by position (never on `Tile`), and fade by themselves. Cosmetic chance
//! (which way blood sprays) comes from a hash, not the game's dice, so the
//! mess never changes how a fight goes.

use super::combat::{near, Blow};
use super::*;
use crate::body::{Hurt, Part};
use crate::creature::{Creature, Species};
use std::collections::VecDeque;

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
/// Most bloodied walls kept.
pub const MAX_WALLS: usize = 800;
/// Most effects (flashes, flying drops and parts) at once.
pub const MAX_FX: usize = 200;
/// Fights kept for the fight log, and lines kept for each.
pub const MAX_FIGHTS: usize = 12;
pub const MAX_FIGHT_LINES: usize = 200;
/// Most people kept covered in blood, or with bloody feet.
const MAX_COATS: usize = 400;
/// Blows this far apart in ticks (or tiles) are separate fights.
const FIGHT_GAP: u64 = 400;
const FIGHT_SPREAD: i32 = 20;

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
        Some(Species::Beast(i)) => match crate::bestiary::beast(i).class {
            crate::bestiary::Class::Bug | crate::bestiary::Class::Mollusc | crate::bestiary::Class::Worm => Blood::Ichor,
            crate::bestiary::Class::Strange if crate::bestiary::beast(i).cave_only() => Blood::Violet,
            _ => Blood::Red,
        },
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

/// What a stain is of.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fluid {
    #[default]
    Blood,
    /// Someone retched here.
    Vomit,
}

/// Blood on one tile: whose, how much (1..=9), and when it was last added to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stain {
    /// None for a dwarf.
    pub who: Option<Species>,
    pub amt: u8,
    pub at: u64,
    /// Blood, or vomit.
    #[serde(default)]
    pub fluid: Fluid,
    /// Only footprints, tracked here from a pool somewhere else.
    #[serde(default)]
    pub prints: bool,
}

impl Stain {
    pub fn blood(&self) -> Blood {
        blood_of(self.who)
    }

    /// "goblin blood", "vomit".
    pub fn stuff(&self) -> String {
        match self.fluid {
            Fluid::Blood => format!("{} {}", bleeder(self.who), self.blood().fluid()),
            Fluid::Vomit => "vomit".to_string(),
        }
    }

    /// "a pool of goblin blood", "spatters of dwarf blood", "bloody footprints".
    pub fn describe(&self) -> String {
        let what = self.stuff();
        if self.prints {
            return format!("footprints in {}", what);
        }
        if self.fluid == Fluid::Vomit {
            return if self.amt >= POOL { "a puddle of vomit".to_string() } else { "spatters of vomit".to_string() };
        }
        match self.amt {
            0..=1 => format!("a few drops of {}", what),
            2..=4 => format!("spatters of {}", what),
            5..=7 => format!("a pool of {}", what),
            _ => format!("a great pool of {}", what),
        }
    }

    /// The colour to draw it in: darker as it thins.
    pub fn rgb(&self) -> (u8, u8, u8) {
        let (r, g, b) = if self.fluid == Fluid::Vomit { (150, 140, 60) } else { self.blood().rgb() };
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
    /// Blood dashed up the walls (a wall tile shows its stain).
    #[serde(with = "pos_map", default)]
    pub walls: BTreeMap<Pos, Stain>,
    /// What's flashing and flying just now (not saved).
    #[serde(skip)]
    pub fx: Vec<Fx>,
    /// The fights lately, blow by blow, for the fight log.
    #[serde(default)]
    pub fights: VecDeque<Fight>,
    /// Who is covered in whose blood (by dwarf or creature id).
    #[serde(default)]
    pub coats: BTreeMap<u32, Coat>,
    /// Whose feet are bloody, and how many more steps leave prints.
    #[serde(default)]
    pub feet: BTreeMap<u32, Feet>,
    /// Rooms told to run red, by zone id, with when.
    #[serde(default)]
    pub red: BTreeMap<u32, u64>,
    /// Cosmetic rolls so far (with the tick, they seed the hash).
    #[serde(default)]
    pub rolls: u64,
    /// Weapons bloodied: item id to (how much, when last).
    #[serde(default)]
    pub soaked: BTreeMap<u32, (u8, u64)>,
}

/// Something brief drawn over the map: a blow's flash, a drop of blood or
/// a severed part flying (`from` to `to` over `len` ticks; `from == to`
/// for a flash in place).
#[derive(Clone, Debug)]
pub struct Fx {
    pub from: Pos,
    pub to: Pos,
    pub at: u64,
    pub len: u64,
    pub glyph: char,
    pub rgb: (u8, u8, u8),
    pub bg: Option<(u8, u8, u8)>,
    /// Flies in an arc (drawn a row higher mid-flight).
    pub arc: bool,
    /// The item flying (it isn't drawn where it lands until it gets there).
    pub item: Option<u32>,
}

impl Fx {
    /// Where it is at `tick`, if it's showing.
    pub fn pos_at(&self, tick: u64) -> Option<Pos> {
        if tick < self.at || tick >= self.at + self.len {
            return None;
        }
        let k = (tick - self.at) as f32 / self.len.max(1) as f32;
        let x = self.from.x as f32 + (self.to.x - self.from.x) as f32 * k;
        let y = self.from.y as f32 + (self.to.y - self.from.y) as f32 * k;
        let lift = self.arc && k > 0.2 && k < 0.8;
        Some(Pos::new(x.round() as i32, y.round() as i32 - lift as i32, self.from.z))
    }
}

/// One line of a fight's log: what happened, how many times running (for
/// misses), and how much it mattered (0 a miss or a glance, 1 a wound,
/// 2 a telling blow, 3 a death).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FightLine {
    pub tick: u64,
    pub text: String,
    #[serde(default)]
    pub n: u16,
    #[serde(default)]
    pub grade: u8,
}

/// A fight: blows close together in time and place, from the first to the last.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Fight {
    pub start: u64,
    pub last: u64,
    pub at: Pos,
    /// Where it was ("the Dining Hall", "the surface").
    #[serde(default)]
    pub place: String,
    #[serde(default)]
    pub lines: Vec<FightLine>,
    #[serde(default)]
    pub blows: u32,
    #[serde(default)]
    pub wounds: u32,
    #[serde(default)]
    pub severed: u32,
    #[serde(default)]
    pub deaths: u32,
}

/// Blood (or vomit) someone is covered in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coat {
    pub who: Option<Species>,
    #[serde(default)]
    pub fluid: Fluid,
    /// 1 spattered, 2 covered, 3 soaked.
    pub amt: u8,
    pub at: u64,
}

/// Bloody feet: whose blood, and steps left that print.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Feet {
    pub who: Option<Species>,
    #[serde(default)]
    pub fluid: Fluid,
    pub steps: u8,
    pub last: Pos,
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
        self.stain_on(p, who, amt, Fluid::Blood, false);
    }

    /// Add a stain of `fluid` on `p` (footprints only where nothing else lies).
    pub(super) fn stain_on(&mut self, p: Pos, who: Option<Species>, amt: u8, fluid: Fluid, prints: bool) {
        if amt == 0 || !self.world.in_bounds(p) {
            return;
        }
        let t = self.world.t(p);
        if t.kind.solid() || t.water > 2 || t.magma > 0 {
            return;
        }
        if prints && self.gore.stains.contains_key(&p) {
            return;
        }
        let tick = self.tick;
        if !self.gore.stains.contains_key(&p) && self.gore.stains.len() >= MAX_STAINS {
            self.forget_old_stains();
        }
        let s = self.gore.stains.entry(p).or_insert(Stain { who, amt: 0, at: tick, fluid, prints });
        // The freshest and the most blood colour the tile.
        if s.amt <= amt || s.prints && !prints {
            s.who = who;
            s.fluid = fluid;
        }
        s.prints &= prints;
        s.amt = (s.amt + amt).min(MAX_AMT);
        s.at = tick;
        if fluid == Fluid::Blood && !prints && self.world.t(p).zone != 0 {
            self.blood_on_holy_ground(p);
        }
    }

    /// Blood dashed on the wall at `p`.
    pub(super) fn stain_wall(&mut self, p: Pos, who: Option<Species>, amt: u8) {
        if amt == 0 || !self.world.in_bounds(p) || !self.world.solid(p) {
            return;
        }
        let tick = self.tick;
        if !self.gore.walls.contains_key(&p) && self.gore.walls.len() >= MAX_WALLS {
            let mut ages: Vec<(u64, Pos)> = self.gore.walls.iter().map(|(&q, s)| (s.at, q)).collect();
            ages.sort_unstable();
            for (_, q) in ages.into_iter().take(MAX_WALLS / 10 + 1) {
                self.gore.walls.remove(&q);
            }
        }
        let s = self.gore.walls.entry(p).or_insert(Stain { who, amt: 0, at: tick, fluid: Fluid::Blood, prints: false });
        if s.amt <= amt {
            s.who = who;
        }
        s.amt = (s.amt + amt).min(MAX_AMT);
        s.at = tick;
    }

    /// The blood on the wall at `p`, if any.
    pub fn wall_stain(&self, p: Pos) -> Option<&Stain> {
        self.gore.walls.get(&p)
    }

    /// A cosmetic roll: a hash of the moment and a running count (never the game's dice).
    pub(super) fn gore_roll(&mut self, salt: u64) -> u64 {
        self.gore.rolls = self.gore.rolls.wrapping_add(1);
        mix(Pos::new((self.gore.rolls & 0xFFFF) as i32, (salt & 0xFFFF) as i32, (self.gore.rolls >> 16) as i32), self.tick ^ salt.wrapping_mul(0x9E37_79B9))
    }

    /// At the cap: forget the oldest tenth of the stains in one go.
    fn forget_old_stains(&mut self) {
        let mut ages: Vec<(u64, Pos)> = self.gore.stains.iter().map(|(&p, s)| (s.at, p)).collect();
        ages.sort_unstable();
        for (_, p) in ages.into_iter().take(MAX_STAINS / 10 + 1) {
            self.gore.stains.remove(&p);
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
        if self.gore.stains.is_empty() && self.gore.walls.is_empty() {
            if !self.gore.coats.is_empty() || !self.gore.feet.is_empty() {
                self.coats_tick();
            }
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
            // Footprints are thin and scuff away sooner.
            let period = if s.prints { (period / 4).max(GORE_TICK) } else { period };
            if outside && raining {
                // The rain washes it away.
                s.amt = s.amt.saturating_sub(2);
            } else if age >= period && age % period < GORE_TICK {
                s.amt -= 1;
            }
            s.amt > 0
        });
        // Blood on the walls lasts twice as long, but the rain still washes it.
        let wall_fade = fade_in * 2;
        self.gore.walls.retain(|&p, s| {
            let outside = w.outside(p);
            let age = tick.saturating_sub(s.at);
            if outside && raining {
                s.amt = s.amt.saturating_sub(2);
            } else if age >= wall_fade && age % wall_fade < GORE_TICK {
                s.amt -= 1;
            }
            s.amt > 0 && w.solid(p)
        });
        if tick % (DAY / 4) < GORE_TICK {
            self.coats_tick();
            self.rooms_run_red();
        }
    }

    /// Scrub the blood off `p` (the cleaning job). Returns whether there was any.
    pub fn clean_stain(&mut self, p: Pos) -> bool {
        let mut any = self.gore.stains.remove(&p).is_some();
        // The walls beside it are scrubbed down too.
        if !self.gore.walls.is_empty() {
            for (dx, dy) in DIRS8 {
                any |= self.gore.walls.remove(&p.offset(dx, dy, 0)).is_some();
            }
        }
        any
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
        // A floor beside a bloodied wall, to scrub the wall from.
        let mut spots = spots;
        for (q, s) in self.gore.walls.iter().take(200) {
            if tick < s.at + 100 || w.outside(*q) {
                continue;
            }
            if let Some(f) = DIRS4.iter().map(|&(dx, dy)| q.offset(dx, dy, 0)).find(|f| w.in_bounds(*f) && w.walkable(*f) && w.t(*f).revealed && !self.claimed.contains(f)) {
                spots.insert(f);
            }
        }
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
        let wall = DIRS8.iter().any(|&(dx, dy)| self.gore.walls.contains_key(&pos.offset(dx, dy, 0)));
        if !self.gore.stains.contains_key(&pos) && !wall {
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
        // Seen to land hard where they fall.
        self.flash(q, 2, 4, '!', (255, 225, 130), Some((90, 40, 0)));
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
        let mut retched = Vec::new();
        for d in self.dwarves.iter_mut().filter(|d| d.alive && !d.asleep && Some(d.id) != except && near(d.pos, at, 6) && d.pos.z == at.z) {
            if d.body.unconscious() || tick < d.gore_at + 150 {
                continue;
            }
            d.gore_at = tick;
            let seen = d.gore_seen.min(12) as i32;
            d.gore_seen = d.gore_seen.saturating_add(1);
            let mut stress = weight * (14 - seen) / 14;
            let veteran = d.fight_level() >= 3;
            if veteran {
                stress /= 2;
            }
            if stress <= 1 {
                d.think(if veteran && seen >= 8 { "has grown hardened to the slaughter" } else { "was unmoved by the carnage" }, 0, tick);
            } else {
                d.think(what, stress, tick);
                // The green and squeamish retch at the worst of it.
                if stress >= 10 && mix(d.pos, tick ^ d.id as u64) % 100 < 30 {
                    retched.push((d.pos, d.called()));
                }
            }
        }
        for (p, name) in retched {
            self.stain_on(p, None, 2, Fluid::Vomit, false);
            self.count("gore.retched", 1);
            self.flash(p, 0, 4, '~', (170, 160, 70), None);
            let _ = name;
        }
    }

    /// Every body tick for a dwarf: a bleeding dwarf leaves a trail; one
    /// who walks through a pool of blood minds it.
    pub(super) fn gore_underfoot(&mut self, d: &mut Dwarf) {
        let rate = d.body.bleed_rate();
        self.walk_gore(d.id, d.pos, None, rate);
        if d.path.is_empty() || self.tick < d.gore_at + 300 {
            return;
        }
        let Some(s) = self.gore.stains.get(&d.pos).copied() else { return };
        if s.amt >= POOL && s.who != None && !s.prints && s.fluid == Fluid::Blood {
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

/// The eight ways round, in turning order (for spraying a little to either side).
const TURN: [(i32, i32); 8] = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];

/// The way from `from` to `at` turned by `k` eighths (0 straight on).
fn turned(from: Pos, at: Pos, k: i32, h: u64) -> (i32, i32) {
    let d = ((at.x - from.x).signum(), (at.y - from.y).signum());
    let i = match TURN.iter().position(|&t| t == d) {
        Some(i) => i as i32,
        None => (h % 8) as i32,
    };
    TURN[(i + k).rem_euclid(8) as usize]
}

/// The glyph of a swing seen from `from` to `at`.
fn swing_glyph(from: Pos, at: Pos) -> char {
    match ((at.x - from.x).signum(), (at.y - from.y).signum()) {
        (0, 0) => '*',
        (_, 0) => '-',
        (0, _) => '|',
        (dx, dy) if dx == dy => '\\',
        _ => '/',
    }
}

fn brighten((r, g, b): (u8, u8, u8)) -> (u8, u8, u8) {
    let up = |v: u8| (v as u16 * 3 / 2 + 60).min(255) as u8;
    (up(r), up(g), up(b))
}

impl Game {
    // ------------------------------------------------------------ effects

    fn push_fx(&mut self, fx: Fx) {
        let tick = self.tick;
        if self.gore.fx.len() >= MAX_FX / 2 {
            self.gore.fx.retain(|f| f.at + f.len > tick);
        }
        if self.gore.fx.len() < MAX_FX {
            self.gore.fx.push(fx);
        }
    }

    /// A mark shown on `p` for `len` ticks, `wait` ticks from now.
    fn flash(&mut self, p: Pos, wait: u64, len: u64, glyph: char, rgb: (u8, u8, u8), bg: Option<(u8, u8, u8)>) {
        let at = self.tick + wait;
        self.push_fx(Fx { from: p, to: p, at, len, glyph, rgb, bg, arc: false, item: None });
    }

    /// What's flashing and flying on level `z` now: (where, glyph, colour, background).
    pub fn gore_fx(&self, z: i32) -> Vec<(Pos, char, (u8, u8, u8), Option<(u8, u8, u8)>)> {
        let t = self.tick;
        self.gore.fx.iter().filter(|f| f.from.z == z).filter_map(|f| f.pos_at(t).map(|p| (p, f.glyph, f.rgb, f.bg))).collect()
    }

    /// Items still flying (drawn in the air, not yet where they land).
    pub fn flying_items(&self) -> HashSet<u32> {
        let t = self.tick;
        self.gore.fx.iter().filter(|f| t < f.at + f.len).filter_map(|f| f.item).collect()
    }

    /// A blow seen on the map: the swing across the one struck, then the
    /// impact (a red burst where it drew blood, a grey spark off armour).
    pub(super) fn show_blow(&mut self, from: Pos, at: Pos, b: &Blow) {
        if from == at || from.z != at.z {
            return;
        }
        let hit = b.blood > 0;
        let heavy = b.notable;
        let swing = swing_glyph(from, at);
        self.flash(at, 0, 2, swing, (255, 255, 235), Some(if hit { (170, 20, 20) } else { (70, 70, 80) }));
        if b.msg.is_some() {
            let (g, c, bg) = if hit {
                (if heavy { '☼' } else { '*' }, (255, 90, 60), if heavy { Some((120, 0, 0)) } else { None })
            } else {
                ('+', (200, 200, 210), None)
            };
            self.flash(at, 2, if heavy { 4 } else { 2 }, g, c, bg);
        }
    }

    // ------------------------------------------------------------ the spray

    /// Blood thrown by a blow at `at` struck from `from`: a splash on the
    /// tile, and drops flying one to three tiles on, away from the blow and
    /// a little to either side, that land on the floor or dash the walls.
    pub(super) fn spatter(&mut self, at: Pos, from: Pos, who: Option<Species>, amt: u8) {
        self.bleed_on(at, who, amt);
        let drops = match amt {
            0 => 0,
            1 => 1,
            2..=3 => 2,
            4..=5 => 4,
            _ => 6,
        };
        let reach = 1 + (amt as u64 / 2).min(2);
        for k in 0..drops {
            let h = self.gore_roll(k);
            let turn = match h % 6 {
                0..=2 => 0,
                3 => 1,
                4 => -1,
                _ => if h & 64 == 0 { 2 } else { -2 },
            };
            let dist = 1 + (h >> 8) % reach;
            self.fling_drop(at, turned(from, at, turn, h >> 16), dist as i32, who, if amt >= 4 { 2 } else { 1 });
        }
        // A heavy blow splashes back on the one who struck it.
        if amt >= 3 && from != at && near(from, at, 1) {
            self.bleed_on(from, who, 1);
        }
    }

    /// One drop of blood flying from `at` along `dir` for up to `dist`
    /// tiles: it lands on the last floor it reaches, or on the wall in the way.
    fn fling_drop(&mut self, at: Pos, dir: (i32, i32), dist: i32, who: Option<Species>, amt: u8) {
        let mut q = at;
        let mut wall = None;
        for _ in 0..dist {
            let n = q.offset(dir.0, dir.1, 0);
            if !self.world.in_bounds(n) {
                break;
            }
            if self.world.solid(n) {
                wall = Some(n);
                break;
            }
            if !self.world.walkable(n) {
                break;
            }
            q = n;
        }
        let end = wall.unwrap_or(q);
        if end == at {
            return;
        }
        match wall {
            Some(w) => self.stain_wall(w, who, amt),
            None => self.bleed_on(q, who, amt),
        }
        let steps = (end.x - at.x).abs().max((end.y - at.y).abs()) as u64;
        let rgb = brighten(blood_of(who).rgb());
        self.push_fx(Fx { from: at, to: end, at: self.tick, len: 1 + steps * 2, glyph: if amt >= 2 { '•' } else { '·' }, rgb, bg: None, arc: false, item: None });
    }

    /// Something cut off flies from `at`, away from `from`, two or three
    /// tiles in an arc, leaving a trail of blood: where it lands (short of
    /// any wall, which it bloodies).
    fn fling(&mut self, at: Pos, from: Pos, far: i32, who: Option<Species>, salt: u64) -> Pos {
        let h = self.gore_roll(salt);
        let turn = [0, 0, 1, -1][(h % 4) as usize];
        let dir = turned(from, at, turn, h >> 8);
        let mut q = at;
        for _ in 0..far {
            let n = q.offset(dir.0, dir.1, 0);
            if !self.world.in_bounds(n) {
                break;
            }
            if self.world.solid(n) {
                self.stain_wall(n, who, 2);
                break;
            }
            if !self.world.walkable(n) {
                break;
            }
            q = n;
            self.bleed_on(q, who, 1);
        }
        q
    }

    /// Drop a severed part (or teeth) as an item near `at`, flung away from
    /// `from`: "Urist's left arm". It flies there over a few frames.
    pub(super) fn drop_part(&mut self, owner: &str, part: Part, teeth: u8, at: Pos, from: Pos) -> u32 {
        let (label, mat, far) = if teeth > 0 {
            let n = if teeth == 1 { "tooth".to_string() } else { format!("{} teeth", teeth) };
            (format!("{} {}", possessive(owner), n), Mat::Bone, 1 + (teeth > 1) as i32)
        } else {
            (format!("{} {}", possessive(owner), part.name()), Mat::Flesh, if part == Part::Head { 3 } else { 2 })
        };
        self.drop_bit(label, mat, at, from, far, None)
    }

    /// A bit of someone (a part, teeth, an ear) flung from `at` to land as
    /// an item labelled `label`.
    pub(super) fn drop_bit(&mut self, label: String, mat: Mat, at: Pos, from: Pos, far: i32, who: Option<Species>) -> u32 {
        let lands = self.fling(at, from, far, who, label.len() as u64);
        let id = self.spawn_item(ItemKind::BodyPart, mat, lands);
        if let Some(it) = self.items.get_mut(&id) {
            it.label = Some(label);
        }
        if lands != at && lands.z == at.z {
            let steps = (lands.x - at.x).abs().max((lands.y - at.y).abs()) as u64;
            let (glyph, rgb) = if mat == Mat::Bone { ('\'', (240, 235, 210)) } else { ('⌐', (220, 60, 60)) };
            let tick = self.tick;
            self.push_fx(Fx { from: at, to: lands, at: tick, len: 2 + steps * 3, glyph, rgb, bg: None, arc: steps >= 2, item: Some(id) });
        }
        id
    }

    /// The mess of a blow's extra hurts: an ear or fingers on the floor,
    /// guts spilled, an artery spurting, vomit; and what the witnesses saw.
    pub(super) fn hurt_mess(&mut self, b: &Blow, from: Pos, at: Pos, owner: &str, seen: &str, who: Option<Species>) {
        let whose = possessive(owner);
        for h in b.hurts.clone() {
            match h {
                Hurt::Ear => {
                    self.drop_bit(format!("{} ear", whose), Mat::Flesh, at, from, 1, who);
                }
                Hurt::Nose => {
                    self.drop_bit(format!("{} nose", whose), Mat::Flesh, at, from, 1, who);
                }
                Hurt::Eye(true) => {
                    self.drop_bit(format!("{} eye", whose), Mat::Flesh, at, from, 1, who);
                    self.witness(at, &format!("saw {} eye put out", possessive(seen)), if who.is_none() { 14 } else { 6 }, None);
                }
                Hurt::Fingers(n) => {
                    let what = if n == 1 { "finger".to_string() } else { format!("{} fingers", n) };
                    self.drop_bit(format!("{} {}", whose, what), Mat::Flesh, at, from, 1, who);
                }
                Hurt::Guts => {
                    self.bleed_on(at, who, 4);
                    let h = self.gore_roll(77);
                    self.fling_drop(at, turned(from, at, 0, h), 1, who, 3);
                    self.count("gore.guts", 1);
                    self.witness(at, &format!("saw {} guts spill out", possessive(seen)), if who.is_none() { 18 } else { 9 }, None);
                }
                Hurt::Artery => {
                    for k in 0..3 {
                        let h = self.gore_roll(90 + k);
                        self.fling_drop(at, turned(from, at, (h % 3) as i32 - 1, h), 2 + (h >> 9) as i32 % 2, who, 2);
                    }
                    self.count("gore.artery", 1);
                }
                Hurt::Vomit => {
                    self.stain_on(at, None, 3, Fluid::Vomit, false);
                }
                Hurt::Teeth(_) | Hurt::Eye(false) | Hurt::Bone | Hurt::Skull | Hurt::Ribs | Hurt::KnockedOut | Hurt::Fainted => {}
            }
        }
        if b.hurts.iter().any(|h| matches!(h, Hurt::KnockedOut | Hurt::Fainted)) {
            self.flash(at, 3, 6, 'z', (200, 200, 255), None);
        }
        if b.severed.is_some() {
            self.flash(at, 1, 3, '☼', (255, 40, 40), Some((140, 0, 0)));
        }
        let _ = from;
    }

    // ------------------------------------------------------------ the fight log

    /// Note a blow in the fight it belongs to (one close by in time and
    /// place, or a new one). Repeated misses are counted, not repeated.
    pub(super) fn fight_note(&mut self, at: Pos, b: &Blow) {
        let Some(text) = b.msg.clone().or_else(|| b.report.clone()) else { return };
        let grade = if b.fatal {
            3
        } else if b.notable {
            2
        } else if b.blood > 0 {
            1
        } else {
            0
        };
        let tick = self.tick;
        let fresh = self.gore.fights.back().map_or(true, |f| tick > f.last + FIGHT_GAP || !near(f.at, at, FIGHT_SPREAD) || f.at.z != at.z);
        if fresh {
            let place = self.place_name(at).unwrap_or_else(|| if self.world.outside(at) { "the open".to_string() } else { "the halls".to_string() });
            if self.gore.fights.len() >= MAX_FIGHTS {
                self.gore.fights.pop_front();
            }
            self.gore.fights.push_back(Fight { start: tick, last: tick, at, place, ..Fight::default() });
        }
        let f = self.gore.fights.back_mut().unwrap();
        f.last = tick;
        f.blows += 1;
        f.wounds += (grade >= 1) as u32;
        f.severed += b.severed.is_some() as u32;
        f.deaths += b.fatal as u32;
        if grade == 0 {
            let n = f.lines.len();
            if let Some(l) = f.lines[n.saturating_sub(4)..].iter_mut().rev().find(|l| l.text == text) {
                l.n = l.n.saturating_add(1);
                l.tick = tick;
                return;
            }
        }
        if f.lines.len() >= MAX_FIGHT_LINES {
            // Keep the telling blows: drop the oldest miss, else the oldest line.
            let i = f.lines.iter().position(|l| l.grade == 0).unwrap_or(0);
            f.lines.remove(i);
        }
        f.lines.push(FightLine { tick, text, n: 1, grade });
    }

    /// A fight's lines as read: "Urist misses the goblin (3 times)."
    pub fn fight_lines(f: &Fight) -> Vec<(String, u8)> {
        f.lines
            .iter()
            .map(|l| {
                let t = if l.n > 1 { format!("{} ({} times)", l.text.trim_end_matches('.'), l.n) } else { l.text.clone() };
                (t, l.grade)
            })
            .collect()
    }

    /// A fight's heading: "The fight in the Dining Hall: 34 blows, 12 wounds, 2 dead."
    pub fn fight_title(&self, f: &Fight) -> String {
        let mut s = format!("{}, {}: {} blows, {} drew blood", self.short_date(f.start), f.place, f.blows, f.wounds);
        if f.severed > 0 {
            s.push_str(&format!(", {} cut off", f.severed));
        }
        if f.deaths > 0 {
            s.push_str(&format!(", {} slain", f.deaths));
        }
        s
    }

    // ------------------------------------------------------------ underfoot

    /// Every body tick for anyone on the move: a bleeding one leaves a
    /// trail (an opened artery spurts); one who steps in blood or vomit
    /// carries it and leaves fading footprints; fresh spray where they
    /// stand covers them in it; water and rain wash it off. `own` is their
    /// own blood (None for a dwarf).
    pub(super) fn walk_gore(&mut self, id: u32, p: Pos, own: Option<Species>, rate: i32) {
        if rate > 0 {
            self.blood_trail(p, own, rate);
            if rate >= 25 {
                let h = self.gore_roll(id as u64);
                let d = TURN[(h % 8) as usize];
                self.fling_drop(p, d, 1, own, 1);
            }
        }
        if self.gore.stains.is_empty() && self.gore.feet.is_empty() && self.gore.coats.is_empty() {
            return;
        }
        let tick = self.tick;
        // Washed clean in water or out in the rain.
        let t = self.world.t(p);
        if t.water > 0 || self.weather.wet() && self.world.outside(p) {
            self.gore.coats.remove(&id);
            self.gore.feet.remove(&id);
            return;
        }
        let here = self.gore.stains.get(&p).copied();
        // Prints from feet already bloody.
        if let Some(f) = self.gore.feet.get(&id).copied() {
            if f.last != p {
                if f.steps == 0 {
                    self.gore.feet.remove(&id);
                } else {
                    self.stain_on(p, f.who, 1, f.fluid, true);
                    if let Some(f) = self.gore.feet.get_mut(&id) {
                        f.steps -= 1;
                        f.last = p;
                    }
                }
            }
        }
        let Some(s) = here.filter(|s| !s.prints && s.amt >= 2) else { return };
        // Stepping in it.
        if self.gore.feet.len() < MAX_COATS || self.gore.feet.contains_key(&id) {
            let steps = if s.amt >= POOL { 6 } else { 3 };
            let f = self.gore.feet.entry(id).or_insert(Feet { who: s.who, fluid: s.fluid, steps: 0, last: p });
            if f.steps < steps {
                *f = Feet { who: s.who, fluid: s.fluid, steps, last: p };
            }
        }
        // Fresh spray, or a pool lain in, covers them.
        let fresh = tick <= s.at + 20 && (s.fluid == Fluid::Vomit || s.who != own || s.amt >= POOL);
        if fresh {
            if !self.gore.coats.contains_key(&id) && self.gore.coats.len() >= MAX_COATS {
                if let Some((&old, _)) = self.gore.coats.iter().min_by_key(|(_, c)| c.at) {
                    self.gore.coats.remove(&old);
                }
            }
            let c = self.gore.coats.entry(id).or_insert(Coat { who: s.who, fluid: s.fluid, amt: 0, at: tick });
            c.who = s.who;
            c.fluid = s.fluid;
            c.amt = (c.amt + 1).min(3);
            c.at = tick;
        }
    }

    /// "Covered in goblin blood", if they are.
    pub fn coat_words(&self, id: u32) -> Option<String> {
        let c = self.gore.coats.get(&id)?;
        let stuff = match c.fluid {
            Fluid::Blood => format!("{} {}", bleeder(c.who), blood_of(c.who).fluid()),
            Fluid::Vomit => "vomit".to_string(),
        };
        Some(match c.amt {
            0..=1 => format!("Spattered with {}", stuff),
            2 => format!("Covered in {}", stuff),
            _ => format!("Soaked in {}", stuff),
        })
    }

    /// Coats dry and flake off after a few days; the dead and gone are forgotten.
    fn coats_tick(&mut self) {
        let tick = self.tick;
        let alive: HashSet<u32> = self.dwarves.iter().filter(|d| d.alive).map(|d| d.id).chain(self.creatures.keys().copied()).collect();
        self.gore.coats.retain(|id, c| alive.contains(id) && tick < c.at + 3 * DAY);
        self.gore.feet.retain(|id, f| alive.contains(id) && f.steps > 0);
        let items = &self.items;
        self.gore.soaked.retain(|id, s| items.contains_key(id) && tick < s.1 + SEASON);
    }

    // ------------------------------------------------------------ aftermath

    /// Now and then: a room whose floor is soaked in blood is told of, once
    /// a season: "The floor of the Dining Hall runs red with goblin blood."
    fn rooms_run_red(&mut self) {
        let mut by_zone: BTreeMap<u32, (u32, u32, Pos, Option<Species>)> = BTreeMap::new();
        for (&p, s) in self.gore.stains.iter() {
            if s.prints || s.fluid != Fluid::Blood {
                continue;
            }
            let z = self.world.t(p).zone;
            if z == 0 {
                continue;
            }
            let e = by_zone.entry(z).or_insert((0, 0, p, s.who));
            e.0 += s.amt as u32;
            e.1 += 1;
        }
        let tick = self.tick;
        let need = self.knob("gore.red_room", 40.0) as u32;
        for (z, (amt, tiles, p, who)) in by_zone {
            if amt < need || tiles < 10 || self.gore.red.get(&z).map_or(false, |&t| tick < t + SEASON) {
                continue;
            }
            self.gore.red.insert(z, tick);
            let place = self.place_name(p).or_else(|| self.zones.get(&z).map(|zz| format!("the {}", zz.kind.name().to_lowercase()))).unwrap_or_else(|| "the hall".into());
            let text = format!("The floor of {} runs red with {} {}.", place, bleeder(who), blood_of(who).fluid());
            self.msg_at(&text, (220, 60, 60), Some(p), super::logkind::LogKind::Combat);
            self.count("gore.red_rooms", 1);
        }
    }
}

// ------------------------------------------------------------ the telling

/// A fighter's lasting hurts on their blows: lost fingers and a broken
/// weapon arm weaken them, a lost eye spoils the aim.
pub(super) fn hurt_fighter(f: &mut super::combat::Fighter, body: &crate::body::Body) {
    let (m, s) = body.fight_mods();
    f.dmg *= m;
    f.skill = f.skill.saturating_sub(s);
}

/// What follows "The goblin slashes Urist in the left arm" for a blow
/// that landed: the weapon, the layers it went through, and every extra
/// hurt, DF-style: ", tearing apart the muscle and chipping the bone! An
/// artery has been opened!" `h` picks among wordings.
#[allow(clippy::too_many_arguments)]
pub(super) fn blow_words(
    att: &super::combat::Fighter,
    def: &super::combat::Fighter,
    part: Part,
    dmg: i32,
    max: i32,
    w: crate::body::Wound,
    decap: bool,
    cleave: bool,
    devastating: bool,
    sprawl: bool,
    hurts: &[Hurt],
    h: u64,
) -> String {
    use crate::body::Wound;
    let pick = |k: u64, opts: &[&'static str]| opts[((h >> (k * 5)) % opts.len() as u64) as usize];
    let with = match &att.weapon {
        Some(wp) if att.verb != "shoots" => format!(" with {}", wp),
        _ => String::new(),
    };
    let layers = crate::body::layers(part, dmg, max, att.edged, w, h >> 3);
    let them = cap(&def.name);
    let mut s = with;
    let core = match w {
        Wound::Sever if att.edged => format!(", severing {}!", pick(1, &["it", "it at the joint", "it clean through"])),
        Wound::Sever => format!(", {}!", pick(1, &["tearing it clean off", "ripping it away"])),
        Wound::Fatal if decap => format!(", {}!", pick(1, &["cutting off the head", "striking the head from the shoulders", "sending the head flying"])),
        Wound::Fatal if part == Part::Head && !att.edged => format!(", {}!", pick(1, &["caving in the skull", "shattering the skull", "crushing the skull like an egg"])),
        Wound::Fatal => format!(" {}", pick(1, &["— a fatal blow!", "— a mortal wound!", "— and the wound is mortal!"])),
        _ if layers.is_empty() => String::new(),
        Wound::Mangle => format!(", {}!", layers),
        _ => format!(", {}", layers),
    };
    if cleave && w != Wound::Bruise {
        s.push_str(", cutting through the armor and");
        s.push_str(core.trim_start_matches(','));
    } else if cleave {
        s.push_str(", cutting through the armor");
        if !core.is_empty() {
            s.push_str(" and");
            s.push_str(core.trim_start_matches(','));
        }
    } else {
        s.push_str(&core);
    }
    let mut extra: Vec<String> = Vec::new();
    for hurt in hurts {
        let line = match *hurt {
            Hurt::Teeth(1) => pick(2, &["A tooth goes flying!", "A tooth is knocked out!"]).to_string(),
            Hurt::Teeth(n) => format!("{} teeth go flying!", cap(num(n))),
            Hurt::Eye(true) => pick(3, &["An eye is gouged out!", "An eye is put out!"]).to_string(),
            Hurt::Eye(false) => "An eye is burst!".to_string(),
            Hurt::Ear => pick(3, &["An ear is torn off!", "An ear is cut away!", "An ear flies off!"]).to_string(),
            Hurt::Nose => pick(3, &["The nose is cut off!", "The nose is torn away!"]).to_string(),
            Hurt::Fingers(1) => "A finger is lopped off!".to_string(),
            Hurt::Fingers(n) => format!("{} fingers are lopped off!", cap(num(n))),
            Hurt::Bone if w != Wound::Mangle => pick(4, &["The bone breaks!", "There is a sickening crack!"]).to_string(),
            Hurt::Skull if w != Wound::Mangle => "The skull is fractured!".to_string(),
            Hurt::Ribs => pick(4, &["Ribs are broken!", "Ribs crack!"]).to_string(),
            Hurt::Guts => pick(5, &["The belly is opened and the guts spill out!", "The guts spill out onto the ground!"]).to_string(),
            Hurt::Artery => pick(6, &["An artery has been opened!", "An artery is cut, and blood spurts!"]).to_string(),
            Hurt::KnockedOut => format!("{} is knocked unconscious!", them),
            Hurt::Fainted => format!("{} faints from the pain!", them),
            Hurt::Vomit => format!("{} {}.", them, pick(7, &["vomits", "retches", "doubles over, retching"])),
            _ => continue,
        };
        extra.push(line);
    }
    if matches!(w, Wound::Sever) || decap {
        extra.push(if decap { "The head sails off in an arc!" } else { pick(8, &["The severed part sails off in an arc!", "The severed part flies off!", "The severed part tumbles away in a spray of blood!"]) }.to_string());
    }
    if devastating && w != Wound::Fatal {
        extra.push("A devastating blow!".to_string());
    }
    if sprawl {
        extra.push(format!("{} is sent sprawling!", them));
    }
    if extra.is_empty() {
        if !s.is_empty() && !s.ends_with('!') {
            s.push('.');
        }
    } else {
        if !s.ends_with('!') {
            s.push(if s.is_empty() { '.' } else { '!' });
        }
        s.push(' ');
        s.push_str(&extra.join(" "));
    }
    s
}

fn num(n: u8) -> &'static str {
    ["no", "one", "two", "three", "four", "five"].get(n as usize).copied().unwrap_or("many")
}

fn cap(s: &str) -> String {
    super::combat::cap(s)
}

impl Game {
    /// "Urist swings at the goblin and misses", in a few ways.
    pub(super) fn miss_words(&mut self, att: &super::combat::Fighter, def: &super::combat::Fighter) -> String {
        let h = mix(Pos::new(att.name.len() as i32, def.name.len() as i32, 0), self.tick / 40);
        let how = match att.verb {
            "shoots" => "shoots at {} and misses",
            v if v.starts_with("bite") => "snaps at {} and misses",
            _ => ["swings at {} and misses", "strikes at {}, but misses", "lunges at {}, who dodges"][(h % 3) as usize],
        };
        format!("{} {}.", cap(&att.name), how.replace("{}", &def.name))
    }

    /// A weapon as told in a blow: "blood-soaked iron axe".
    pub fn weapon_words(&self, it: &crate::items::Item) -> String {
        match self.gore.soaked.get(&it.id) {
            Some(&(n, _)) if n >= 6 => format!("blood-soaked {}", it.name()),
            Some(_) => format!("blood-spattered {}", it.name()),
            None => it.name(),
        }
    }

    /// A weapon takes the blood it draws.
    pub(super) fn soak(&mut self, item: u32, amt: u8) {
        let tick = self.tick;
        if !self.gore.soaked.contains_key(&item) && self.gore.soaked.len() >= MAX_COATS {
            if let Some((&old, _)) = self.gore.soaked.iter().min_by_key(|(_, s)| s.1) {
                self.gore.soaked.remove(&old);
            }
        }
        let e = self.gore.soaked.entry(item).or_insert((0, tick));
        e.0 = e.0.saturating_add(amt).min(20);
        e.1 = tick;
    }
}

// ------------------------------------------------------------ the looks

/// A plain bit of floor (blood shows on it as drops and pools).
pub fn plain_floor(ch: char) -> bool {
    matches!(ch, '.' | ',' | '`' | '\'' | '·' | '"' | '=' | ';' | ' ')
}

/// How a stain looks on a floor drawn as `ch` in `fg` on `bg`: drops and
/// pools of blood, vomit, faint footprints.
pub fn floor_look(s: &Stain, ch: char, fg: (u8, u8, u8), bg: (u8, u8, u8)) -> (char, (u8, u8, u8), (u8, u8, u8)) {
    let col = s.rgb();
    let plain = plain_floor(ch);
    if s.prints {
        let (r, g, b) = brighten(col);
        let mix = |a: u8, b: u8| ((a as u16 + b as u16) / 2) as u8;
        return if plain { (':', (r, g, b), (mix(bg.0, col.0), mix(bg.1, col.1), mix(bg.2, col.2))) } else { (ch, fg, bg) };
    }
    if !plain {
        return (ch, fg, col);
    }
    let glyph = if s.amt >= POOL {
        '≈'
    } else if s.fluid == Fluid::Vomit {
        '~'
    } else if s.amt >= 2 {
        '·'
    } else {
        ch
    };
    (glyph, brighten(col), col)
}

/// How blood dashed on a wall drawn as `ch` in `fg` looks: a solid wall
/// shows it in streaks of red through the stone.
pub fn wall_look(s: &Stain, ch: char, fg: (u8, u8, u8)) -> (char, (u8, u8, u8), (u8, u8, u8)) {
    let col = s.rgb();
    match ch {
        '█' | '▓' | '#' | ' ' => (if s.amt >= 4 { '▒' } else { '▓' }, fg, col),
        _ => {
            let mix = |a: u8, b: u8| ((a as u16 + 2 * b as u16) / 3) as u8;
            (ch, (mix(fg.0, col.0), mix(fg.1, col.1), mix(fg.2, col.2)), col)
        }
    }
}

impl Game {
    /// A dwarf who lost something for good to a blow (an eye, teeth,
    /// fingers, an ear, the nose) remembers it: in the medical history, in
    /// their thoughts, and in the fort's memory for the worst of it.
    pub(super) fn remember_losses(&mut self, d: &mut Dwarf, b: &Blow, by: &str) {
        let tick = self.tick;
        let a = if by.starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" };
        for h in &b.hurts {
            let (what, stress, big) = match *h {
                Hurt::Eye(_) => ("lost an eye".to_string(), 12, true),
                Hurt::Teeth(1) => ("lost a tooth".to_string(), 3, false),
                Hurt::Teeth(n) => (format!("lost {} teeth", n), 5, false),
                Hurt::Ear => ("lost an ear".to_string(), 6, false),
                Hurt::Nose => ("lost their nose".to_string(), 10, true),
                Hurt::Fingers(1) => ("lost a finger".to_string(), 6, false),
                Hurt::Fingers(n) => (format!("lost {} fingers", n), 9, true),
                Hurt::Guts => ("had their belly torn open".to_string(), 12, true),
                _ => continue,
            };
            super::medicine::chart(d, tick, format!("{} to {} {}", what, a, by));
            d.think(&format!("{} to {} {}", what, a, by), stress, tick);
            if big {
                let text = format!("{} {} to {} {}.", d.called(), what, a, by);
                self.record(super::memory::MemoryKind::Wounded, &[d.id], Some(d.pos), text, 2);
            }
        }
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

    /// A spot on open floor with a wall `gap` tiles to the east (built if need be).
    fn by_a_wall(g: &mut Game, gap: i32) -> Pos {
        let p = clear_ground(g);
        for k in 1..=gap {
            let q = p.offset(k, 0, 0);
            g.world.t_mut(q).kind = TileKind::Floor;
        }
        let w = p.offset(gap + 1, 0, 0);
        g.world.t_mut(w).kind = TileKind::Wall;
        g.world.t_mut(w).mat = Mat::Granite;
        p
    }

    #[test]
    fn blows_spray_blood_across_the_floor_and_up_the_walls() {
        let mut g = Game::new(5);
        g.weather = Weather::Clear;
        let p = by_a_wall(&mut g, 2);
        let from = p.offset(-1, 0, 0);
        for k in 0..12 {
            g.tick += 1 + k;
            g.spatter(p, from, Some(Species::Goblin), 5);
        }
        let wall = p.offset(3, 0, 0);
        assert!(g.wall_stain(wall).is_some(), "no blood reached the wall");
        assert!(g.stain_at(p.offset(2, 0, 0)).is_some() || g.stain_at(p.offset(2, 1, 0)).is_some() || g.stain_at(p.offset(2, -1, 0)).is_some(), "no drop flew two tiles");
        assert!(g.stain_at(from).is_some(), "nothing splashed back on the striker");
        // The drops are seen flying, and the wall is described.
        assert!(g.gore.fx.iter().any(|f| f.glyph == '•' || f.glyph == '·'));
        let (c, _, bg) = wall_look(g.wall_stain(wall).unwrap(), '█', (120, 120, 120));
        assert!(c == '▓' || c == '▒');
        assert!(bg.0 > bg.1, "a wall stain should show red");
        // A hauler scrubbing the floor beside it cleans the wall too.
        assert!(g.clean_stain(p.offset(2, 0, 0)) || g.clean_stain(p.offset(2, 1, 0)) || g.clean_stain(p.offset(2, -1, 0)));
        assert!(g.wall_stain(wall).is_none(), "the wall wasn't scrubbed");
        // Stains of old saves (without fluid or prints) still load.
        let old = r#"{"stains":[[{"x":1,"y":2,"z":3},{"who":null,"amt":3,"at":5}]]}"#;
        let back: Gore = serde_json::from_str(old).unwrap();
        let s = back.stains.values().next().unwrap();
        assert_eq!((s.fluid, s.prints), (Fluid::Blood, false));
        assert!(back.walls.is_empty() && back.fights.is_empty());
    }

    #[test]
    fn a_severed_part_flies_in_an_arc_and_lands_as_an_item() {
        let mut g = Game::new(5);
        let p = by_a_wall(&mut g, 4);
        let from = p.offset(-1, 0, 0);
        let id = g.drop_part("Urist", Part::LeftArm, 0, p, from);
        let it = &g.items[&id];
        assert_eq!(it.name(), "Urist's left arm");
        assert!(it.pos != p && near(it.pos, p, 3), "landed at {:?} from {:?}", it.pos, p);
        assert!(g.flying_items().contains(&id), "it should be seen flying first");
        let f = g.gore.fx.iter().find(|f| f.item == Some(id)).unwrap().clone();
        let mid = f.pos_at(f.at + f.len / 2).unwrap();
        assert!(f.arc && mid.y < p.y.max(it.pos.y), "no arc: {:?}", mid);
        g.tick += f.len;
        assert!(!g.flying_items().contains(&id), "still flying");
        // Blood all along the way.
        assert!(g.stain_at(it.pos).is_some());
        // Teeth land as "Urist's tooth".
        let t = g.drop_part("Urist", Part::Head, 1, p, from);
        assert_eq!(g.items[&t].name(), "Urist's tooth");
    }

    #[test]
    fn injuries_go_beyond_hit_points_and_matter() {
        use crate::body::{Body, Hurt, Wound};
        // Over many hashes, every kind of extra hurt turns up.
        let mut seen: HashSet<String> = HashSet::new();
        for h in 0..4000u64 {
            let h = mix(Pos::new(h as i32, 0, 0), h);
            for (part, w, edged) in [
                (Part::Head, Wound::Gash, true),
                (Part::Head, Wound::Mangle, false),
                (Part::LowerBody, Wound::Mangle, true),
                (Part::RightArm, Wound::Gash, true),
                (Part::LeftLeg, Wound::Mangle, false),
                (Part::UpperBody, Wound::Mangle, false),
            ] {
                let mut b = Body::new(1.0);
                for hurt in b.harm_from(part, w, 12, edged, h, 14) {
                    seen.insert(format!("{:?}", hurt).split('(').next().unwrap().to_string());
                }
            }
        }
        for k in ["Eye", "Ear", "Nose", "Fingers", "Bone", "Skull", "Ribs", "Guts", "Artery", "KnockedOut", "Fainted", "Vomit"] {
            assert!(seen.contains(k), "never: {} (saw {:?})", k, seen);
        }
        // An opened artery bleeds hard; fingers lost weaken the grip; an eye
        // lost spoils the aim; one knocked out comes to.
        let find = |part: Part, w: Wound, edged: bool, want: &dyn Fn(&Hurt) -> bool| {
            (0..4000u64).map(|h| mix(Pos::new(h as i32, 1, 0), h)).find_map(|h| {
                let mut b = Body::new(1.0);
                let hs = b.harm_from(part, w, 12, edged, h, 14);
                hs.iter().any(|x| want(x)).then_some(b)
            })
        };
        let b = find(Part::LeftArm, Wound::Gash, true, &|h| *h == Hurt::Artery).unwrap();
        assert!(b.bleeds[Part::LeftArm.idx()] >= 14 && b.hurts().iter().any(|s| s.contains("artery")));
        let b = find(Part::RightArm, Wound::Gash, true, &|h| matches!(h, Hurt::Fingers(_))).unwrap();
        assert!(b.fight_mods().0 < 1.0 && b.losses().iter().any(|s| s.contains("finger")));
        let b = find(Part::Head, Wound::Gash, true, &|h| *h == Hurt::Eye(true)).unwrap();
        assert!(b.fight_mods().1 >= 1 && b.losses().iter().any(|s| s.contains("eye")));
        let mut b = find(Part::Head, Wound::Mangle, false, &|h| *h == Hurt::KnockedOut).unwrap();
        b.hp = b.max;
        assert!(b.unconscious(), "knocked out, yet awake");
        for _ in 0..20 {
            b.tick();
        }
        assert!(!b.unconscious(), "never came to");
        // Losses stay; a broken bone knits as the part heals.
        let mut b = find(Part::LeftLeg, Wound::Mangle, false, &|h| *h == Hurt::Bone).unwrap();
        assert!(b.limp() > 0);
        b.harm.teeth = 3;
        b.heal(100);
        assert!(!b.harm.broken[Part::LeftLeg.idx()] && b.limp() == 0);
        assert!(b.losses().contains(&"missing 3 teeth".to_string()));
        // And it all saves.
        let back: Body = serde_json::from_str(&serde_json::to_string(&b).unwrap()).unwrap();
        assert_eq!(back.harm, b.harm);
    }

    #[test]
    fn the_blows_are_told_in_layers_and_the_fight_log_keeps_them_all() {
        let mut g = Game::new(5);
        let p = clear_ground(&g);
        let gob = g.spawn_creature(Species::Goblin, p, u64::MAX);
        let mut d = g.dwarves[0].clone();
        d.female = Some(true);
        let axe = g.spawn_item(ItemKind::Axe, Mat::Iron, p);
        g.items.get_mut(&axe).unwrap().carried_by = Some(d.id);
        d.held = Some(axe);
        d.fight_xp = 1200;
        d.pos = p.offset(-1, 0, 0);
        let att = g.dwarf_fighter(&d);
        let def = g.creature_fighter(&g.creatures[&gob]);
        let mut said: Vec<String> = Vec::new();
        for k in 0..600 {
            let mut body = crate::body::Body::new(0.9);
            g.tick += 3;
            let b = g.strike(&att, &def, &mut body);
            g.aftermath(&b, d.pos, p, "goblin", "a goblin", Some(Species::Goblin), false);
            said.extend(b.msg.clone().or(b.report.clone()));
            if k % 100 == 99 {
                g.tick += FIGHT_GAP + 1;
            }
        }
        let has = |w: &str| said.iter().any(|s| s.contains(w));
        assert!(has("with her iron"), "no weapon told: {:?}", &said[..10]);
        assert!(has("the skin") && has("the muscle") && has("bone"), "no layers");
        assert!(has("misses") || has("dodges"));
        assert!(has("artery") || has("ear") || has("finger") || has("guts"), "no extra hurts in 600 blows");
        // The axe is bloodied by now.
        assert!(g.weapon_words(&g.items[&axe]).starts_with("blood-"));
        // Six fights, each with its blows; repeated misses are counted.
        assert_eq!(g.gore.fights.len(), 6);
        let f = &g.gore.fights[0];
        assert!(f.blows >= 90 && f.lines.len() < f.blows as usize, "{} lines for {} blows", f.lines.len(), f.blows);
        assert!(Game::fight_lines(f).iter().any(|(t, _)| t.contains("times)")));
        assert!(g.fight_title(f).contains("blows"));
    }

    #[test]
    fn blood_is_tracked_about_and_covers_those_in_the_thick_of_it() {
        let mut g = Game::new(5);
        g.weather = Weather::Clear;
        let p = by_a_wall(&mut g, 6);
        let id = 4242;
        g.bleed_on(p, Some(Species::Goblin), 6);
        // Standing in fresh spray: covered in it.
        g.walk_gore(id, p, None, 0);
        assert_eq!(g.coat_words(id).as_deref(), Some("Spattered with goblin blood"));
        // Walking on: footprints for a few tiles, then clean feet.
        for k in 1..=6 {
            g.tick += 10;
            g.walk_gore(id, p.offset(k, 0, 0), None, 0);
        }
        let prints: Vec<i32> = (1..=6).filter(|k| g.stain_at(p.offset(*k, 0, 0)).map_or(false, |s| s.prints)).collect();
        assert!(prints.len() >= 4, "prints on {:?}", prints);
        assert_eq!(g.stain_at(p.offset(1, 0, 0)).unwrap().describe(), "footprints in goblin blood");
        // Footprints scuff away before the pool does.
        let start = g.tick;
        while g.stain_at(p.offset(1, 0, 0)).is_some() && g.tick < start + SEASON {
            g.tick += GORE_TICK;
            g.gore_tick();
        }
        assert!(g.stain_at(p).is_some(), "the pool went first");
        // Rain washes them clean.
        g.weather = Weather::Rain;
        g.walk_gore(id, p.offset(2, 0, 0), None, 0);
        assert!(g.coat_words(id).is_none());
    }

    #[test]
    fn a_hall_soaked_in_blood_is_told_once() {
        let mut g = Game::new(5);
        let p = clear_ground(&g);
        let zid = 9999;
        for dx in 0..4 {
            for dy in 0..4 {
                let q = p.offset(dx, dy, 0);
                g.world.t_mut(q).zone = zid;
                g.bleed_on(q, Some(Species::Goblin), 6);
            }
        }
        let before = g.log.len();
        g.rooms_run_red();
        g.rooms_run_red();
        let told: Vec<&String> = g.log[before..].iter().map(|l| &l.text).filter(|t| t.contains("runs red")).collect();
        assert_eq!(told.len(), 1, "{:?}", told);
        assert!(told[0].contains("goblin blood"));
    }

    #[test]
    fn an_arena_bout_is_violent_on_the_map_and_leaves_a_scene() {
        let m = crate::game::arena::presets().remove(0);
        let mut b = crate::game::arena::Battle::new(&m, 3);
        let z = b.pos(0).z;
        let mut most = 0;
        while b.outcome.is_none() {
            b.step();
            most = most.max(b.g.gore_fx(z).len());
        }
        assert!(most >= 3, "at most {} effects at once", most);
        let g = &b.g;
        assert!(g.gore.stains.len() >= 20, "only {} stains", g.gore.stains.len());
        assert!(g.gore.stains.values().any(|s| s.prints) || !g.gore.feet.is_empty(), "nobody stepped in the blood");
        let blows: u32 = g.gore.fights.iter().map(|f| f.blows).sum();
        assert!(blows >= 20, "{} blows in the fight log", blows);
        assert!(g.items.values().any(|it| it.kind == ItemKind::BodyPart) || g.gore.fights.iter().all(|f| f.severed == 0));
    }
}
