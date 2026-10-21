//! Clockwork (O12, steps 2 and 3): automatons and luxury trinkets.
//!
//! A skilled mechanic builds an automaton at the Mechanic's workshop: a
//! guard that stands at its post and goes for whatever hostile comes near,
//! a hauler that carries loose goods to the stockpiles, or a gatekeeper
//! that holds the gate and never leaves it. They are creatures
//! (`Species::Automaton`, their state in `Creature.clockwork`), tame and
//! bloodless. They need upkeep: a spring runs down in a season, and gears
//! wear. A mechanic winds and oils one at the workshop (`Recipe::Tune`,
//! a mechanism each time), queued here when one is due. One left too long,
//! or badly made, may seize up, or go haywire and turn on the fort until
//! its spring runs out.
//!
//! The luxuries (music boxes, clockwork birds, orreries) are `ItemKind::
//! Trinket`. Nobles demand them; one set out in a dwarf's room adds to its
//! value and cheers its owner each day.

use super::*;
use crate::buildings::Input;
use crate::creature::{AutoKind, Clockwork, Creature, Species};
use crate::dwarf::Noble;
use super::names::{self, NameTheme};
use super::overseer::{Overseer, Style};

/// The label of an automaton fresh off the bench, before it walks.
pub const FRAME: &str = "automaton frame";

/// The luxuries, cheapest first.
pub const TRINKETS: [&str; 3] = ["music box", "clockwork bird", "orrery"];

/// A noble's demand for a trinket in their quarters.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrinketWant {
    pub noble: u32,
    pub what: String,
    pub deadline: u64,
}

/// The fort's clockwork (`Game.inspirations.clockwork`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Clockworks {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wants: Vec<TrinketWant>,
    /// When each automaton was finished.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub built: Vec<u64>,
    /// Trinkets to carry into someone's room: (item, where).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub homes: Vec<(u32, Pos)>,
}

fn near(a: Pos, b: Pos, r: i32) -> bool {
    a.z == b.z && (a.x - b.x).abs() <= r && (a.y - b.y).abs() <= r
}

/// The recipe that builds an automaton of `kind`.
pub fn recipe_of(kind: AutoKind) -> Recipe {
    match kind {
        AutoKind::Guard => Recipe::ClockGuard,
        AutoKind::Hauler => Recipe::ClockHauler,
        AutoKind::Gatekeeper => Recipe::ClockGate,
    }
}

fn kind_of(r: Recipe) -> Option<AutoKind> {
    match r {
        Recipe::ClockGuard => Some(AutoKind::Guard),
        Recipe::ClockHauler => Some(AutoKind::Hauler),
        Recipe::ClockGate => Some(AutoKind::Gatekeeper),
        _ => None,
    }
}

impl Game {
    /// Automatons the fort has, going or not.
    pub fn automatons(&self) -> impl Iterator<Item = &Creature> {
        self.creatures.values().filter(|c| c.alive && c.clockwork.is_some())
    }

    /// Its own glyph and colour: by what it does, dimmed when it has run
    /// down or seized up, red-hot when it has gone haywire.
    pub fn clockwork_look(&self, c: &Creature) -> Option<(char, (u8, u8, u8))> {
        let cw = c.clockwork.as_ref()?;
        let glyph = match cw.kind {
            AutoKind::Guard => 'Ä',
            AutoKind::Hauler => 'ä',
            AutoKind::Gatekeeper => 'Å',
        };
        let color = if cw.haywire_until > self.tick {
            (255, 80, 40)
        } else if cw.broken || self.tick >= cw.wound_until {
            (125, 115, 95)
        } else {
            (215, 175, 90)
        };
        Some((glyph, color))
    }

    /// What the look panel says of an automaton.
    pub fn clockwork_lines(&self, c: &Creature) -> Vec<String> {
        let Some(cw) = c.clockwork.as_ref() else { return Vec::new() };
        let mut v = vec![format!("A {}, {}.", cw.kind.name(), crate::items::quality_prefix(cw.quality).trim().to_string() + " work")];
        if let Some(m) = cw.maker.and_then(|m| self.dwarves.iter().find(|d| d.id == m)) {
            v.push(format!("Built by {}.", m.name));
        }
        if cw.haywire_until > self.tick {
            v.push("It has gone haywire!".into());
        } else if cw.broken {
            v.push("It has seized up, and waits on a mechanic.".into());
        } else if self.tick >= cw.wound_until {
            v.push("Its spring has run down.".into());
        } else {
            let days = (cw.wound_until - self.tick) / DAY;
            v.push(format!("Wound for {} more day{}; its gears {}.", days, if days == 1 { "" } else { "s" }, wear_words(cw.wear)));
        }
        if cw.kills > 0 {
            v.push(format!("It has struck down {} foe{}.", cw.kills, if cw.kills == 1 { "" } else { "s" }));
        }
        v
    }

    // ------------------------------------------------------------------
    // Made at the workshop
    // ------------------------------------------------------------------

    /// Something has just been made at a workshop (from the craft task):
    /// an automaton's frame gets up and walks, a mechanic's tuning winds
    /// one, an orrery is set with its gem.
    pub(super) fn clockwork_made(&mut self, d: &mut Dwarf, recipe: Recipe, used: &[Item], made: &[u32]) {
        if recipe == Recipe::Tune {
            self.tune_automaton(d);
            return;
        }
        if recipe == Recipe::Orrery {
            let gem = used.iter().find(|it| it.kind == ItemKind::Gem).map(|it| it.mat);
            for id in made {
                if let Some(it) = self.items.get_mut(id) {
                    it.decor = gem;
                }
            }
        }
        if matches!(recipe, Recipe::MusicBox | Recipe::ClockBird | Recipe::Orrery) {
            self.count("m2.trinkets", made.len() as u32);
        }
        let Some(kind) = kind_of(recipe) else { return };
        for &id in made {
            let Some(it) = self.items.get(&id).filter(|it| it.label.as_deref() == Some(FRAME)) else { continue };
            let (pos, quality, mat) = (it.pos, it.quality, it.mat);
            self.items.remove(&id);
            self.build_automaton(kind, pos, quality, mat, Some(d));
        }
    }

    /// An automaton of `kind` stands up at `pos`.
    pub(super) fn build_automaton(&mut self, kind: AutoKind, pos: Pos, quality: u8, mat: Mat, maker: Option<&mut Dwarf>) -> u32 {
        let id = self.spawn_creature(Species::Automaton, pos, u64::MAX);
        let fort = self.fortress_name.clone();
        let name = names::coin(names::hash(&[stories::hash(&fort, 0), id as u64, 12]), NameTheme::Automaton);
        let wind = self.knob("clockwork.wind", SEASON as f64) as u64;
        let post = self.automaton_post(kind).unwrap_or(pos);
        let tick = self.tick;
        let maker_id = maker.as_ref().map(|d| d.id);
        let maker_name = maker.as_ref().map(|d| d.name.clone());
        if let Some(c) = self.creatures.get_mut(&id) {
            c.tame = true;
            c.label = Some(name.clone());
            c.desc = Some(format!("a {} of {}", kind.name(), mat.name()));
            if kind == AutoKind::Gatekeeper {
                // Built heavy, to hold a doorway.
                c.body = crate::body::Body::new(1.6);
            }
            c.clockwork = Some(Box::new(Clockwork {
                kind,
                post: (kind != AutoKind::Hauler).then_some(post),
                wound_until: tick + wind,
                wear: 0,
                quality,
                maker: maker_id,
                haywire_until: 0,
                broken: false,
                load: None,
                dest: None,
                kills: 0,
                noted: false,
            }));
        }
        self.inspirations.clockwork.built.push(tick);
        self.count("m2.automatons", 1);
        let by = maker_name.clone().map_or(String::new(), |m| format!("{} has built ", m));
        let what = match kind {
            AutoKind::Guard => "It ticks, whirs, and takes up its post.",
            AutoKind::Hauler => "It ticks, whirs, and looks about for something to carry.",
            AutoKind::Gatekeeper => "It clanks to the gate and stands there, ticking.",
        };
        let text = if by.is_empty() { format!("{}, a {}, stirs. {}", name, kind.name(), what) } else { format!("{}{}, a {}. {}", by, name, kind.name(), what) };
        self.msg_at(&text, (215, 175, 90), Some(pos), logkind::LogKind::Job);
        // The fort's first is a moment.
        if self.inspirations.clockwork.built.len() == 1 {
            self.spotlight("A thing of brass", &text, Some(pos), (215, 175, 90));
        }
        if let Some(d) = maker {
            d.think(&format!("is proud of {}", name), -25, tick);
            let did = d.id;
            self.record(memory::MemoryKind::Feat, &[did], Some(pos), format!("{} built {}, a {}.", d.name, name, kind.name()), 5);
            self.legend(format!("{} built {}, a {}, at {}.", d.name, name, kind.name(), fort));
        }
        id
    }

    /// Where an automaton of `kind` stands: the gatekeeper inside the gate,
    /// a guard at the head of the stair down into the fort.
    fn automaton_post(&self, kind: AutoKind) -> Option<Pos> {
        let o = self.overseer.as_ref()?;
        let want = match kind {
            AutoKind::Gatekeeper => o.base?.offset(3, 0, 0),
            AutoKind::Guard => o.top.or(o.base)?,
            AutoKind::Hauler => return None,
        };
        // The spot itself, or the nearest one it can stand on.
        (0..4).flat_map(|r| (-r..=r).flat_map(move |dx| (-r..=r).map(move |dy| want.offset(dx, dy, 0)))).find(|&p| self.world.in_bounds(p) && self.world.walkable(p))
    }

    /// A mechanic's tuning: wind, oil and mend the automaton most in need.
    fn tune_automaton(&mut self, d: &mut Dwarf) {
        let tick = self.tick;
        let need = self
            .creatures
            .values()
            .filter(|c| c.alive)
            .filter_map(|c| c.clockwork.as_ref().map(|cw| (c.id, cw)))
            .filter(|(_, cw)| cw.haywire_until <= tick)
            .min_by_key(|(id, cw)| (!cw.broken, cw.wound_until.saturating_sub(cw.wear as u64 * DAY / 2), *id))
            .map(|(id, _)| id);
        let Some(cid) = need else { return };
        let wind = self.knob("clockwork.wind", SEASON as f64) as u64;
        let Some(c) = self.creatures.get_mut(&cid) else { return };
        let scale = if c.clockwork.as_ref().map_or(false, |cw| cw.kind == AutoKind::Gatekeeper) { 1.6 } else { 1.1 };
        c.body = crate::body::Body::new(scale);
        let was_broken;
        {
            let cw = c.clockwork.as_mut().unwrap();
            was_broken = cw.broken;
            cw.wound_until = tick + wind;
            cw.wear /= 3;
            cw.broken = false;
            cw.noted = false;
        }
        let name = c.title();
        d.gain_xp(Labor::Mechanics, 20);
        if was_broken {
            self.msg(&format!("{} has mended {}, and it ticks again.", d.name, name), (215, 175, 90));
        }
    }

    // ------------------------------------------------------------------
    // Its turn
    // ------------------------------------------------------------------

    /// An automaton's turn (from `creature_tick`, before its body's): None
    /// for anything else, or for one gone haywire, which then hunts the
    /// fort like any beast; otherwise whether it's still there.
    pub(super) fn clockwork_tick(&mut self, c: &mut Creature) -> Option<bool> {
        if c.species != Species::Automaton {
            return None;
        }
        // No blood to lose, nothing to fester.
        c.body.bleeds = [0; 7];
        c.body.bleeding = 0;
        c.body.blood = c.body.max_blood;
        c.body.dirty = false;
        c.body.infection = 0;
        if c.body.dead {
            self.creature_died(c.clone());
            return Some(false);
        }
        let mut cw = c.clockwork.take()?;
        let keep = self.clockwork_turn(c, &mut cw);
        c.clockwork = Some(cw);
        keep
    }

    fn clockwork_turn(&mut self, c: &mut Creature, cw: &mut Clockwork) -> Option<bool> {
        let tick = self.tick;
        if tick % 10 == 0 {
            c.body.tick_with(&self.bleed_params(), false);
        }
        if cw.haywire_until > 0 {
            if tick < cw.haywire_until {
                // Its gears hunt the fort (combat.rs, as any beast's).
                return None;
            }
            // The spring gives out at last: it seizes up where it stands.
            cw.haywire_until = 0;
            cw.broken = true;
            cw.noted = true;
            c.tame = true;
            c.target = None;
            c.foe = None;
            c.path.clear();
            let name = c.title();
            self.msg_at(&format!("{} shudders, grinds, and falls still.", name), (215, 175, 90), Some(c.pos), logkind::LogKind::Combat);
            return Some(true);
        }
        c.move_cd = c.move_cd.saturating_sub(1);
        c.attack_cd = c.attack_cd.saturating_sub(1);
        // Run down or seized up: it stands where it is.
        if cw.broken || tick >= cw.wound_until {
            if !cw.noted {
                cw.noted = true;
                let why = if cw.broken { "has seized up" } else { "has run down" };
                self.msg_at(&format!("{} {}.", c.title(), why), (200, 170, 110), Some(c.pos), logkind::LogKind::Job);
            }
            if let Some(item) = cw.load.take() {
                self.set_down(item, c.pos);
            }
            c.path.clear();
            return Some(true);
        }
        if c.body.unconscious() {
            return Some(true);
        }
        match cw.kind {
            AutoKind::Guard | AutoKind::Gatekeeper => self.guard_turn(c, cw),
            AutoKind::Hauler => self.hauler_turn(c, cw),
        }
        Some(true)
    }

    /// A guard goes for anything hostile near its post, and otherwise
    /// stands there; a gatekeeper keeps closer.
    fn guard_turn(&mut self, c: &mut Creature, cw: &mut Clockwork) {
        let post = cw.post.unwrap_or(c.pos);
        let reach = if cw.kind == AutoKind::Gatekeeper { 3 } else { 6 };
        let foe = self
            .creatures
            .values()
            .filter(|o| o.hostile() && !o.sneaking && self.creature_active(o) && near(o.pos, post, reach))
            .min_by_key(|o| ((o.pos.x - c.pos.x).abs() + (o.pos.y - c.pos.y).abs(), o.id))
            .map(|o| (o.id, o.pos));
        if let Some((fid, fpos)) = foe {
            if super::combat::adjacent(fpos, c.pos) {
                if c.attack_cd == 0 {
                    c.attack_cd = c.species.info().attack_delay + c.body.limp();
                    c.foe = Some(fid);
                    self.creature_maul(c, fid);
                    cw.wear = (cw.wear + 1).min(100);
                    if !self.creatures.contains_key(&fid) {
                        cw.kills += 1;
                        self.count("m2.automaton_kills", 1);
                    }
                }
                return;
            }
            if c.move_cd == 0 && c.path.first() != Some(&fpos) {
                if let Some(p) = path::astar(&self.world, c.pos, fpos, 2000, Mover::Dwarf) {
                    c.path = p.into_iter().rev().collect();
                }
            }
            self.creature_step(c, c.species.info().move_delay);
            return;
        }
        c.foe = None;
        if near(c.pos, post, 0) {
            c.path.clear();
            c.move_cd = c.move_cd.max(10);
            return;
        }
        if c.path.is_empty() && c.move_cd == 0 {
            match path::astar(&self.world, c.pos, post, 4000, Mover::Dwarf) {
                Some(p) => c.path = p.into_iter().rev().collect(),
                None => {
                    // No way back to its post: this is its post now.
                    cw.post = Some(c.pos);
                    c.move_cd = 30;
                    return;
                }
            }
        }
        self.creature_step(c, c.species.info().move_delay);
    }

    /// A hauler carries loose goods, one at a time, to a stockpile that
    /// takes them.
    fn hauler_turn(&mut self, c: &mut Creature, cw: &mut Clockwork) {
        if let Some(item) = cw.load {
            let ok = self.items.get(&item).map_or(false, |it| it.carried_by == Some(c.id));
            let Some(dest) = cw.dest.filter(|_| ok) else {
                cw.load = None;
                cw.dest = None;
                if ok {
                    self.set_down(item, c.pos);
                }
                return;
            };
            if c.pos == dest {
                self.set_down(item, dest);
                self.reserved.remove(&dest);
                cw.load = None;
                cw.dest = None;
                c.move_cd = 6;
                return;
            }
            if c.path.is_empty() && c.move_cd == 0 {
                match path::astar(&self.world, c.pos, dest, 6000, Mover::Dwarf) {
                    Some(p) => c.path = p.into_iter().rev().collect(),
                    None => {
                        self.reserved.remove(&dest);
                        self.set_down(item, c.pos);
                        cw.load = None;
                        cw.dest = None;
                        return;
                    }
                }
            }
            self.creature_step(c, c.species.info().move_delay);
            if let Some(it) = self.items.get_mut(&item) {
                it.pos = c.pos;
            }
            return;
        }
        // On its way to something to pick up.
        if let Some(dest) = cw.dest {
            let target = self.items.values().find(|it| it.claimed_by == Some(c.id) && it.carried_by.is_none()).map(|it| (it.id, it.pos));
            let Some((item, ipos)) = target else {
                self.reserved.remove(&dest);
                cw.dest = None;
                return;
            };
            if c.pos == ipos {
                if let Some(it) = self.items.get_mut(&item) {
                    it.carried_by = Some(c.id);
                    it.container = None;
                }
                cw.load = Some(item);
                c.path.clear();
                return;
            }
            if c.path.is_empty() && c.move_cd == 0 {
                match path::astar(&self.world, c.pos, ipos, 6000, Mover::Dwarf) {
                    Some(p) => c.path = p.into_iter().rev().collect(),
                    None => {
                        self.reserved.remove(&dest);
                        if let Some(it) = self.items.get_mut(&item) {
                            it.claimed_by = None;
                        }
                        cw.dest = None;
                        c.move_cd = 60;
                        return;
                    }
                }
            }
            self.creature_step(c, c.species.info().move_delay);
            return;
        }
        if c.move_cd > 0 || c.think_cd > 0 {
            c.think_cd = c.think_cd.saturating_sub(1);
            return;
        }
        c.think_cd = 40;
        if let Some((item, dest)) = self.haul_job_near(c.pos) {
            if let Some(it) = self.items.get_mut(&item) {
                it.claimed_by = Some(c.id);
            }
            self.reserved.insert(dest);
            cw.dest = Some(dest);
        }
    }

    /// The nearest loose thing a stockpile wants (not food or a corpse: it
    /// is not trusted with those), and a free stockpile tile for it.
    fn haul_job_near(&self, from: Pos) -> Option<(u32, Pos)> {
        let piles: Vec<&Zone> = self.zones.values().filter(|z| matches!(z.kind, ZoneKind::Stockpile(_))).collect();
        if piles.is_empty() {
            return None;
        }
        let taken: HashSet<Pos> = self.items.values().filter(|it| it.carried_by.is_none()).map(|it| it.pos).collect();
        let stored = |it: &Item| self.zones.get(&self.world.t(it.pos).zone).map_or(false, |z| z.takes(it.kind, it.mat));
        let mut cands: Vec<&Item> = self
            .items
            .values()
            .filter(|it| it.free() && it.container.is_none() && !it.to_depot && !it.dump && !it.kind.is_food() && !matches!(it.kind, ItemKind::Corpse | ItemKind::BodyPart | ItemKind::Trinket))
            .filter(|it| near(it.pos, from, 30) && !stored(it))
            .collect();
        cands.sort_by_key(|it| ((it.pos.x - from.x).abs() + (it.pos.y - from.y).abs(), it.id));
        for it in cands.into_iter().take(6) {
            let dest = piles
                .iter()
                .filter(|z| z.takes(it.kind, it.mat))
                .flat_map(|z| z.tiles.iter().copied())
                .filter(|p| !taken.contains(p) && !self.reserved.contains(p) && self.world.walkable(*p))
                .min_by_key(|p| ((p.x - it.pos.x).abs() + (p.y - it.pos.y).abs() + (p.z - it.pos.z).abs() * 8, p.x, p.y));
            if let Some(dest) = dest {
                return Some((it.id, dest));
            }
        }
        None
    }

    /// Put down something an automaton carried.
    fn set_down(&mut self, item: u32, at: Pos) {
        if let Some(it) = self.items.get_mut(&item) {
            it.carried_by = None;
            it.claimed_by = None;
            it.pos = at;
        }
    }

    /// An automaton has been smashed (from `creature_died`): what it was
    /// carrying falls, and some of its works can be saved.
    pub(super) fn clockwork_died(&mut self, c: &Creature) {
        let Some(cw) = c.clockwork.as_ref() else { return };
        if let Some(item) = cw.load {
            self.set_down(item, c.pos);
        }
        if let Some(dest) = cw.dest {
            self.reserved.remove(&dest);
        }
        for it in self.items.values_mut().filter(|it| it.claimed_by == Some(c.id)) {
            it.claimed_by = None;
        }
        self.spawn_item(ItemKind::Mechanism, Mat::Iron, c.pos);
        let name = c.title();
        self.msg_at(&format!("{} has been smashed to pieces; its gears scatter across the floor.", name), (215, 175, 90), Some(c.pos), logkind::LogKind::Combat);
        let fort = self.fortress_name.clone();
        if cw.kills > 0 {
            self.legend(format!("{}, a {} that struck down {} foe{}, was smashed at {}.", name, cw.kind.name(), cw.kills, if cw.kills == 1 { "" } else { "s" }, fort));
        }
    }

    // ------------------------------------------------------------------
    // The daily round: wear, faults, upkeep, and the trinkets
    // ------------------------------------------------------------------

    /// Daily (schedule.rs): gears wear, a worn one may fault, upkeep is
    /// queued, nobles want their trinkets, and trinkets find their rooms.
    pub(super) fn clockwork_daily(&mut self) {
        let ids: Vec<u32> = self.automatons().map(|c| c.id).collect();
        let tick = self.tick;
        for id in ids {
            let Some((wear, quality, ok)) = self.creatures.get_mut(&id).and_then(|c| {
                let cw = c.clockwork.as_mut()?;
                let running = cw.haywire_until == 0 && !cw.broken && tick < cw.wound_until;
                if running {
                    cw.wear = (cw.wear + if cw.quality <= 1 { 2 } else { 1 }).min(100);
                }
                Some((cw.wear, cw.quality, running))
            }) else {
                continue;
            };
            if ok {
                self.maybe_fault(id, wear, quality);
            }
        }
        self.queue_tuning();
        self.trinket_wants_daily();
        self.trinket_homes_daily();
    }

    /// A gear slips: worn or shoddy clockwork may seize up, or go haywire.
    fn maybe_fault(&mut self, id: u32, wear: u32, quality: u8) {
        let k = self.knob("clockwork.fault", 1.0);
        if k <= 0.0 {
            return;
        }
        // A slip now and then even when well kept; far likelier past half worn.
        let shoddy = (6 - quality.min(5) as i32) as f64 / 3.0;
        let p = (0.0008 * shoddy + (wear as f64 - 45.0).max(0.0) / 900.0) * k;
        if !self.rng.chance(p) {
            return;
        }
        let haywire = self.rng.chance(0.25 + 0.08 * (5 - quality.min(5) as i32) as f64);
        if haywire {
            self.go_haywire(id);
        } else if let Some(c) = self.creatures.get_mut(&id) {
            if let Some(cw) = c.clockwork.as_mut() {
                cw.broken = true;
            }
        }
    }

    /// An automaton goes haywire: it turns on the fort until its spring
    /// gives out. A moment with a card.
    pub(super) fn go_haywire(&mut self, id: u32) {
        let tick = self.tick;
        let last = (self.knob("clockwork.haywire_ticks", (DAY / 3) as f64) as u64).max(1);
        let Some(c) = self.creatures.get_mut(&id) else { return };
        let Some(cw) = c.clockwork.as_mut() else { return };
        cw.haywire_until = tick + last;
        if let Some(dest) = cw.dest.take() {
            self.reserved.remove(&dest);
        }
        let load = cw.load.take();
        let (kind, maker) = (cw.kind, cw.maker);
        c.tame = false;
        c.path.clear();
        c.foe = None;
        c.think_cd = 0;
        let (name, pos) = (c.title(), c.pos);
        if let Some(item) = load {
            self.set_down(item, pos);
        }
        self.count("m2.haywire", 1);
        let by = maker.and_then(|m| self.dwarves.iter().find(|d| d.id == m)).map(|d| d.name.clone());
        let text = match &by {
            Some(m) => format!("{}, the {} {} built, has gone haywire! Its gears shriek, and it turns on the fort.", name, kind.name(), m),
            None => format!("{}, the {}, has gone haywire! Its gears shriek, and it turns on the fort.", name, kind.name()),
        };
        self.msg_at(&text, (255, 90, 40), Some(pos), logkind::LogKind::Combat);
        self.spotlight("Clockwork gone haywire", &text, Some(pos), (255, 90, 40));
        let who: Vec<u32> = maker.into_iter().collect();
        self.record(memory::MemoryKind::Other, &who, Some(pos), format!("{} went haywire.", name), 4);
        if let Some(m) = maker.and_then(|m| self.dwarves.iter_mut().find(|d| d.id == m && d.alive)) {
            m.think(&format!("is mortified that {} went haywire", name), 25, tick);
        }
    }

    /// Queue a mechanic's tuning for each automaton that is due (a month
    /// before it runs down, or worn, or seized up), at a Mechanic's workshop.
    fn queue_tuning(&mut self) {
        let tick = self.tick;
        let due = self
            .automatons()
            .filter_map(|c| c.clockwork.as_ref())
            .filter(|cw| cw.haywire_until <= tick && (cw.broken || cw.wear >= 35 || cw.wound_until <= tick + MONTH))
            .count();
        let queued = self.queued(Recipe::Tune);
        if due <= queued {
            return;
        }
        let shop = self.buildings.values().filter(|b| b.kind == BuildingKind::Mechanic && b.built).min_by_key(|b| (b.orders.len(), b.id)).map(|b| b.id);
        if let Some(b) = shop.and_then(|id| self.buildings.get_mut(&id)) {
            // Upkeep goes before whatever else the bench has waiting.
            b.orders.insert(0, crate::buildings::Order { recipe: Recipe::Tune, repeat: false, mat: None });
        }
    }

    // ------------------------------------------------------------------
    // Trinkets
    // ------------------------------------------------------------------

    /// The tiles of each dwarf's own rooms: bedroom and office zones they
    /// own, and the enclosed room round the bed they sleep in.
    fn owned_rooms(&self) -> Vec<(u32, HashSet<Pos>)> {
        let mut rooms: HashMap<u32, HashSet<Pos>> = HashMap::new();
        for z in self.zones.values().filter(|z| matches!(z.kind, ZoneKind::Bedroom | ZoneKind::Office)) {
            if let Some(o) = z.owner {
                rooms.entry(o).or_default().extend(z.tiles.iter().copied());
            }
        }
        for b in self.buildings.values().filter(|b| b.kind == BuildingKind::Bed && b.built) {
            if let Some(o) = b.owner {
                if self.zone_at(b.pos).map_or(true, |z| !matches!(z.kind, ZoneKind::Bedroom | ZoneKind::Dormitory)) {
                    if let Some((room, _)) = self.room_at(b.pos) {
                        rooms.entry(o).or_default().extend(room);
                    }
                }
            }
        }
        let mut v: Vec<(u32, HashSet<Pos>)> = rooms.into_iter().collect();
        v.sort_by_key(|(o, _)| *o);
        v
    }

    /// What trinkets add to a room's value: twice their own.
    pub(super) fn trinket_value(&self, tiles: &HashSet<Pos>) -> u32 {
        self.items.values().filter(|it| it.kind == ItemKind::Trinket && it.carried_by.is_none() && tiles.contains(&it.pos)).map(|it| it.value() * 2).sum()
    }

    /// Each season a titled noble may want a trinket; a want is judged daily.
    fn trinket_wants_daily(&mut self) {
        let tick = self.tick;
        let rooms = self.owned_rooms();
        let has = |g: &Game, noble: u32, what: &str| {
            rooms.iter().find(|(o, _)| *o == noble).map_or(false, |(_, t)| {
                g.items.values().any(|it| it.kind == ItemKind::Trinket && it.carried_by.is_none() && it.label.as_deref() == Some(what) && t.contains(&it.pos))
            })
        };
        let wants = std::mem::take(&mut self.inspirations.clockwork.wants);
        for w in wants {
            let Some(i) = self.dwarves.iter().position(|d| d.id == w.noble && d.alive) else { continue };
            if has(self, w.noble, &w.what) {
                let name = self.dwarves[i].name.clone();
                self.dwarves[i].think(&format!("is delighted with the {} in their quarters", w.what), -30, tick);
                self.msg(&format!("{} is delighted with the {} in their quarters.", name, w.what), (150, 255, 150));
            } else if tick > w.deadline {
                let name = self.dwarves[i].name.clone();
                self.dwarves[i].think(&format!("is enraged that no {} was made for them", w.what), 30, tick);
                self.msg(&format!("{} is enraged that no {} was made for them.", name, w.what), (255, 120, 90));
            } else {
                self.inspirations.clockwork.wants.push(w);
            }
        }
        if tick % SEASON >= DAY {
            return;
        }
        let nobles: Vec<(u32, String, &'static str)> = self
            .dwarves
            .iter()
            .filter(|d| d.alive && !d.insane && matches!(d.noble, Some(Noble::Baron | Noble::Count | Noble::Duke)))
            .filter(|d| !self.inspirations.clockwork.wants.iter().any(|w| w.noble == d.id))
            .map(|d| (d.id, d.name.clone(), d.noble.map_or("noble", |n| n.title())))
            .collect();
        for (id, name, title) in nobles {
            // (By a hash, not the game's dice: a titled fort's other rolls stay as they were.)
            let roll = (names::hash(&[id as u64, tick / SEASON, 41]) % 1000) as f64 / 1000.0;
            if roll >= self.knob("clockwork.want", 0.4) {
                continue;
            }
            // The grander the noble, the grander the want.
            let rank = (title != "Baron") as usize + (title == "Duke") as usize;
            let what = TRINKETS[(names::hash(&[id as u64, tick / SEASON]) % (rank as u64 + 2)).min(2) as usize];
            if has(self, id, what) {
                continue;
            }
            self.msg(&format!("{} the {} demands a {} for their quarters.", name, title.to_lowercase(), what), (255, 215, 60));
            self.inspirations.clockwork.wants.push(TrinketWant { noble: id, what: what.to_string(), deadline: tick + YEAR / 2 });
        }
    }

    /// Find each loose trinket a room to stand in: a noble who wants that
    /// one first, then any noble, then whoever has a room and none yet.
    /// One in a room already cheers its owner.
    fn trinket_homes_daily(&mut self) {
        let tick = self.tick;
        let rooms = self.owned_rooms();
        let mut cheer: Vec<(u32, String)> = Vec::new();
        let mut placed: HashSet<u32> = HashSet::new();
        for it in self.items.values().filter(|it| it.kind == ItemKind::Trinket && it.carried_by.is_none()) {
            if let Some((o, _)) = rooms.iter().find(|(_, t)| t.contains(&it.pos)) {
                cheer.push((*o, it.label.clone().unwrap_or_else(|| "trinket".into())));
                placed.insert(*o);
            }
        }
        for (o, what) in cheer {
            if let Some(d) = self.dwarves.iter_mut().find(|d| d.id == o && d.alive) {
                let text = match what.as_str() {
                    "music box" => "was soothed by the tune of their music box".to_string(),
                    "clockwork bird" => "was cheered by the song of their clockwork bird".to_string(),
                    "orrery" => "watched the spheres turn in their orrery".to_string(),
                    w => format!("admired their {}", w),
                };
                d.think(&text, -8, tick);
            }
        }
        // A trinket put out of the way (forbidden on its way to a room) that
        // never got there may be taken again.
        let in_room = |p: Pos| rooms.iter().any(|(_, t)| t.contains(&p));
        let homes = &self.inspirations.clockwork.homes;
        let stray: Vec<u32> = self
            .items
            .values()
            .filter(|it| it.kind == ItemKind::Trinket && it.forbidden && it.carried_by.is_none() && it.claimed_by.is_none() && !in_room(it.pos) && !homes.iter().any(|h| h.0 == it.id))
            .map(|it| it.id)
            .collect();
        for id in stray {
            if let Some(it) = self.items.get_mut(&id) {
                it.forbidden = false;
            }
        }
        // Where each loose one should go.
        let mut homes: Vec<(u32, Pos)> = self.inspirations.clockwork.homes.iter().copied().filter(|(id, _)| self.items.get(id).map_or(false, |it| it.carried_by.is_none() && it.claimed_by.is_none())).collect();
        let loose: Vec<(u32, String)> = self
            .items
            .values()
            .filter(|it| it.kind == ItemKind::Trinket && it.free() && it.label.as_deref() != Some(FRAME) && !in_room(it.pos) && !homes.iter().any(|h| h.0 == it.id))
            .map(|it| (it.id, it.label.clone().unwrap_or_default()))
            .collect();
        let mut loose = loose;
        loose.sort_by_key(|(id, _)| *id);
        let wants = self.inspirations.clockwork.wants.clone();
        let nobles: HashSet<u32> = self.dwarves.iter().filter(|d| d.alive && d.noble.is_some()).map(|d| d.id).collect();
        let mut spoken: HashSet<u32> = placed.clone();
        spoken.extend(homes.iter().filter_map(|(_, p)| rooms.iter().find(|(_, t)| t.contains(p)).map(|(o, _)| *o)));
        let occupied: HashSet<Pos> = self.items.values().filter(|it| it.carried_by.is_none()).map(|it| it.pos).collect();
        for (id, what) in loose {
            let owner = wants
                .iter()
                .find(|w| w.what == what && !spoken.contains(&w.noble))
                .map(|w| w.noble)
                .or_else(|| rooms.iter().map(|(o, _)| *o).find(|o| nobles.contains(o) && !spoken.contains(o)))
                .or_else(|| rooms.iter().map(|(o, _)| *o).find(|o| !spoken.contains(o)));
            let Some(owner) = owner else { break };
            let Some((_, tiles)) = rooms.iter().find(|(o, _)| *o == owner) else { continue };
            let mut spots: Vec<Pos> = tiles.iter().copied().filter(|&p| self.world.walkable(p) && self.world.t(p).building == 0 && !occupied.contains(&p) && !self.reserved.contains(&p)).collect();
            spots.sort_by_key(|p| (p.z, p.y, p.x));
            if let Some(&spot) = spots.first() {
                homes.push((id, spot));
                spoken.insert(owner);
            }
        }
        self.inspirations.clockwork.homes = homes;
    }

    /// The finder (finders.rs): carry a trinket to the room it was meant for.
    pub(super) fn find_trinket_haul(&mut self, d: &mut Dwarf) -> bool {
        while let Some((item, dest)) = self.inspirations.clockwork.homes.first().copied() {
            self.inspirations.clockwork.homes.remove(0);
            let ok = self.items.get(&item).map_or(false, |it| it.carried_by.is_none() && it.claimed_by.is_none() && it.kind == ItemKind::Trinket);
            if !ok || self.reserved.contains(&dest) || !self.world.walkable(dest) {
                continue;
            }
            if let Some(it) = self.items.get_mut(&item) {
                it.claimed_by = Some(d.id);
                // Set out on show, not for the stockpile haulers to take back.
                it.forbidden = true;
            }
            self.reserved.insert(dest);
            d.task = Task::Haul { item, dest, into: None };
            return true;
        }
        false
    }

    // ------------------------------------------------------------------
    // The overseer
    // ------------------------------------------------------------------

    /// The overseer has its mechanic build an automaton now and then (one
    /// a year, a few in all), and trinkets for the nobles who want them and
    /// for trade when it means to prosper.
    pub(super) fn ov_clockwork(&mut self, o: &mut Overseer) {
        let Some(shop) = self.buildings.values().filter(|b| b.kind == BuildingKind::Mechanic && b.built).min_by_key(|b| (b.orders.len(), b.id)).map(|b| b.id) else { return };
        let busy = |g: &Game| g.buildings.get(&shop).map_or(true, |b| b.orders.len() >= 4);
        let mech = self.have_input(Input::Mechanism);
        let bars = self.have_input(Input::Bar);
        // Trinkets nobles have asked for, and not yet made or on order.
        let wants: Vec<String> = self.inspirations.clockwork.wants.iter().map(|w| w.what.clone()).collect();
        for what in wants {
            let r = match what.as_str() {
                "music box" => Recipe::MusicBox,
                "clockwork bird" => Recipe::ClockBird,
                _ => Recipe::Orrery,
            };
            let have = self.items.values().any(|it| it.kind == ItemKind::Trinket && it.label.as_deref() == Some(what.as_str()) && it.free());
            if !have && self.queued(r) == 0 && !busy(self) && mech >= 2 {
                self.add_order(shop, r);
            }
        }
        // For trade, when it means to prosper.
        if o.ambition == Some(super::ambitions::Ambition::Prosper) && mech >= 4 && bars >= 2 && !busy(self) && self.queued(Recipe::ClockBird) + self.queued(Recipe::MusicBox) == 0 {
            let loose = self.items.values().filter(|it| it.kind == ItemKind::Trinket && it.free()).count();
            if loose < 2 {
                self.add_order(shop, Recipe::ClockBird);
            }
        }
        // An automaton: a skilled mechanic, parts to spare, and room in the fort for one more.
        let tick = self.tick;
        let skill = self.dwarves.iter().filter(|d| d.alive && d.has(Labor::Mechanics)).map(|d| d.skill(Labor::Mechanics)).max().unwrap_or(0);
        let cap = self.knob("clockwork.cap", 3.0) as usize;
        let have = self.automatons().count();
        let recent = self.inspirations.clockwork.built.iter().any(|&t| tick < t + YEAR);
        let queued: usize = [AutoKind::Guard, AutoKind::Hauler, AutoKind::Gatekeeper].iter().map(|&k| self.queued(recipe_of(k))).sum();
        if queued > 0 || recent || have >= cap || busy(self) || (skill as f64) < self.knob("clockwork.skill", 2.0) || self.alive_count() < 8 || o.style == Style::Reckless {
            return;
        }
        let gate = o.base.map_or(false, |b| self.gate_lever(b).is_some());
        let count = |k: AutoKind| self.automatons().filter(|c| c.clockwork.as_ref().map_or(false, |cw| cw.kind == k)).count();
        // A warlike or careful overseer wants the gate held first; the rest a hauler.
        let martial = o.style == Style::Warlord || o.has(super::personality::Trait::Cautious);
        let order: [AutoKind; 3] = if martial { [AutoKind::Gatekeeper, AutoKind::Guard, AutoKind::Hauler] } else { [AutoKind::Hauler, AutoKind::Gatekeeper, AutoKind::Guard] };
        let kind = order.into_iter().find(|&k| count(k) == 0 && (k != AutoKind::Gatekeeper || gate)).unwrap_or(AutoKind::Guard);
        let need_bars = match kind {
            AutoKind::Hauler => 1,
            AutoKind::Guard => 2,
            AutoKind::Gatekeeper => 3,
        };
        // Keep a couple of mechanisms back for traps and levers.
        if mech < 5 || bars < need_bars + 1 {
            return;
        }
        self.add_order(shop, recipe_of(kind));
        Self::note_as(o, tick, super::personality::NoteKind::Plain, format!("Ordered a {} from the mechanic.", kind.name()));
    }
}

fn wear_words(w: u32) -> &'static str {
    match w {
        0..=15 => "run smooth",
        16..=40 => "are a little worn",
        41..=70 => "grind and click",
        _ => "are badly worn",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Mechanic's workshop near the embark, built.
    fn workshop(g: &mut Game) -> u32 {
        let e = g.embark;
        let site = g.ov_spot(BuildingKind::Mechanic, e.offset(3, 0, 0), 25).expect("room for a workshop");
        assert!(g.place_building(BuildingKind::Mechanic, site));
        let id = g.world.t(site).building;
        g.buildings.get_mut(&id).unwrap().built = true;
        id
    }

    /// Open ground beside the embark an automaton can stand on.
    fn ground(g: &Game, r: i32) -> Pos {
        let e = g.embark;
        (r..r + 10)
            .flat_map(|r| (-r..=r).flat_map(move |k| [e.offset(k, -r, 0), e.offset(k, r, 0), e.offset(-r, k, 0), e.offset(r, k, 0)]))
            .find(|&p| g.world.in_bounds(p) && g.world.walkable(p) && g.world.t(p).water == 0 && path::astar(&g.world, e, p, 20_000, Mover::Dwarf).is_some())
            .expect("no ground")
    }

    #[test]
    fn a_mechanic_builds_an_automaton_that_walks() {
        let mut g = Game::new(77);
        g.creatures.retain(|_, c| c.tame);
        let shop = workshop(&mut g);
        let e = g.embark;
        for _ in 0..2 {
            g.spawn_item(ItemKind::Bar, Mat::Copper, e);
        }
        for _ in 0..4 {
            g.spawn_item(ItemKind::Mechanism, Mat::Granite, e);
        }
        for d in g.dwarves.iter_mut() {
            d.labors[Labor::Mechanics.idx()] = true;
        }
        g.add_order(shop, Recipe::ClockGuard);
        for _ in 0..DAY * 3 {
            g.step();
            if g.automatons().next().is_some() {
                break;
            }
        }
        let c = g.automatons().next().expect("no automaton was built").clone();
        let cw = c.clockwork.as_ref().unwrap();
        assert_eq!(cw.kind, AutoKind::Guard);
        assert!(c.tame && c.label.is_some());
        assert!(cw.maker.is_some());
        assert!(!g.items.values().any(|it| it.label.as_deref() == Some(FRAME)), "the frame got up and walked");
        assert!(!g.items.values().any(|it| it.kind == ItemKind::Mechanism), "its parts went into it");
        assert_eq!(g.clockwork_look(&c).map(|l| l.0), Some('Ä'));
    }

    #[test]
    fn an_automaton_guards_its_post_and_does_not_bleed() {
        let mut g = Game::new(77);
        g.creatures.clear();
        let post = ground(&g, 6);
        let id = g.build_automaton(AutoKind::Guard, post, 3, Mat::Bronze, None);
        // A goblin comes near the post.
        let gp = (1..4).flat_map(|r| [post.offset(r, 0, 0), post.offset(-r, 0, 0), post.offset(0, r, 0), post.offset(0, -r, 0)]).find(|&p| g.world.walkable(p)).expect("room for a goblin");
        let gob = g.spawn_creature(Species::Goblin, gp, DAY);
        let mut struck = false;
        for _ in 0..600 {
            g.step();
            let a = &g.creatures[&id];
            assert_eq!(a.body.blood, a.body.max_blood, "an automaton has no blood to lose");
            assert!(a.body.bleeds.iter().all(|&b| b == 0));
            if g.creatures.get(&gob).map_or(true, |o| o.body.hp != o.body.max) {
                struck = true;
            }
            if !g.creatures.contains_key(&gob) {
                break;
            }
        }
        assert!(struck, "the guard never went for the goblin");
        // Back to its post once the fight is done.
        g.creatures.remove(&gob);
        for _ in 0..400 {
            g.step();
        }
        let a = &g.creatures[&id];
        assert!(near(a.pos, post, 1), "it wandered off its post: {:?} vs {:?}", a.pos, post);
        assert!(g.clockwork_lines(a).iter().any(|l| l.contains("clockwork guard")));
    }

    #[test]
    fn a_neglected_automaton_runs_down_and_a_mechanic_winds_it() {
        let mut g = Game::new(77);
        g.creatures.retain(|_, c| c.tame);
        let shop = workshop(&mut g);
        let post = ground(&g, 6);
        let id = g.build_automaton(AutoKind::Hauler, post, 3, Mat::Bronze, None);
        // Its spring nearly out: a tuning is queued at the workshop.
        g.creatures.get_mut(&id).unwrap().clockwork.as_mut().unwrap().wound_until = g.tick + 5;
        for _ in 0..20 {
            g.step();
        }
        assert!(g.log.iter().any(|l| l.text.contains("has run down")), "nobody was told it ran down");
        g.clockwork_daily();
        assert_eq!(g.buildings[&shop].orders.first().map(|o| o.recipe), Some(Recipe::Tune));
        g.spawn_item(ItemKind::Mechanism, Mat::Granite, g.embark);
        for d in g.dwarves.iter_mut() {
            d.labors[Labor::Mechanics.idx()] = true;
        }
        for _ in 0..DAY * 2 {
            g.step();
            if g.creatures[&id].clockwork.as_ref().unwrap().wound_until > g.tick {
                break;
            }
        }
        assert!(g.creatures[&id].clockwork.as_ref().unwrap().wound_until > g.tick + MONTH, "the mechanic never wound it");
    }

    #[test]
    fn a_haywire_automaton_turns_on_the_fort_and_then_seizes_up() {
        let mut g = Game::new(77);
        g.creatures.retain(|_, c| c.tame);
        let post = ground(&g, 2);
        let id = g.build_automaton(AutoKind::Guard, post, 0, Mat::Copper, None);
        g.go_haywire(id);
        assert!(g.creatures[&id].hostile(), "a haywire automaton is a danger");
        assert!(g.spotlights.iter().any(|s| s.title == "Clockwork gone haywire"), "no card for it");
        assert_eq!(g.clockwork_look(&g.creatures[&id]).map(|l| l.1), Some((255, 80, 40)));
        for _ in 0..DAY {
            g.step();
            if !g.creatures.contains_key(&id) {
                break;
            }
        }
        if let Some(c) = g.creatures.get(&id) {
            assert!(c.tame && !c.hostile(), "it should fall still once its spring gives out");
            assert!(c.clockwork.as_ref().unwrap().broken);
        }
    }

    #[test]
    fn a_trinket_in_a_room_lifts_its_value_and_cheers_its_owner() {
        let mut g = Game::new(77);
        g.creatures.retain(|_, c| c.tame);
        let a = ground(&g, 4);
        let tiles: Vec<Pos> = (0..3).flat_map(|dx| (0..3).map(move |dy| a.offset(dx, dy, 0))).filter(|&p| g.world.walkable(p)).collect();
        let owner = g.dwarves[0].id;
        assert!(g.add_zone_tiles(tiles.clone(), ZoneKind::Bedroom) > 0);
        let zid = g.world.t(tiles[0]).zone;
        g.zones.get_mut(&zid).unwrap().owner = Some(owner);
        let set: HashSet<Pos> = g.zones[&zid].tiles.iter().copied().collect();
        let before = g.area_value(&set);
        let box_id = g.spawn_item(ItemKind::Trinket, Mat::Wood, g.embark);
        g.items.get_mut(&box_id).unwrap().label = Some("music box".into());
        // A noble wants one.
        g.dwarves[0].noble = Some(crate::dwarf::Noble::Baron);
        g.inspirations.clockwork.wants.push(TrinketWant { noble: owner, what: "music box".into(), deadline: g.tick + YEAR });
        for d in g.dwarves.iter_mut() {
            d.labors[Labor::Hauling.idx()] = true;
        }
        g.clockwork_daily();
        assert!(!g.inspirations.clockwork.homes.is_empty(), "the music box was given no room");
        for _ in 0..DAY {
            g.step();
            if set.contains(&g.items[&box_id].pos) && g.items[&box_id].carried_by.is_none() {
                break;
            }
        }
        assert!(set.contains(&g.items[&box_id].pos), "the music box never reached the room");
        assert!(g.area_value(&set) > before, "a trinket should lift the room's value");
        g.clockwork_daily();
        let d = g.dwarves.iter().find(|d| d.id == owner).unwrap();
        assert!(d.thoughts.iter().any(|t| t.text.contains("music box")), "{:?}", d.thoughts.iter().map(|t| t.text.clone()).collect::<Vec<_>>());
        assert!(d.thoughts.iter().any(|t| t.text == "is delighted with the music box in their quarters"), "the demand is met");
        assert!(!g.inspirations.clockwork.wants.iter().any(|w| w.what == "music box" && w.deadline == YEAR), "and cleared");
    }
}
