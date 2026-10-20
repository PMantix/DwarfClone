//! Travel the world map (A3): a cell at a time, by the land's walking
//! time, through weather and night, with encounters on the way and camps.
//!
//! A step across a cell runs the clock on by the time it takes to walk 48
//! tiles of that land (knobs `travel.*_days`), slower by night, with a
//! limp or into a storm. The hero's body ticks all the way, so an open
//! wound keeps bleeding on the road: the long walk home with a gash is the
//! slice's best drama, and it can end in the grass two cells from home.
//!
//! Every step writes the travel log: the land crossed and the hour, the
//! weather, what can be seen from the road (smoke over a burned town, the
//! lights of a friendly one, a dragon's lair), and how the hero's body
//! holds up. The line naming the cell carries it in `LogEntry.at`, so the
//! log is also the road walked, which the travel view draws.

use super::peoples::{self, Stance};
use super::sitegen::{self, lore};
use super::{AdvEnd, Adventure, SiteKey, HERO_ID};
use crate::body::BleedParams;
use crate::creature::Species;
use crate::game::{clock, LogEntry, Weather, DAY, SEASON, YEAR};
use crate::geom::Pos;
use crate::items::ItemKind;
use crate::material::Mat;
use crate::overworld::{Biome, Overworld, Race, SiteKind};
use crate::rng::Rng;

/// What a step on the world map came to.
#[derive(Clone, Debug, PartialEq)]
pub enum TravelOutcome {
    Arrived,
    Encounter(Encounter),
    Blocked(String),
    /// The hero died on the road (bled out): the cause.
    Died(String),
}

/// Who is met on the road.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncKind {
    /// The land's own hunters: a pack, or one rare and terrible beast.
    Beasts,
    /// Soldiers out of a hostile site near by.
    Patrol,
    /// Outlaws, anywhere.
    Bandits,
    /// The dead that walk on tainted land at night.
    Dead,
    /// A beast of legend from a lair near by (`figure`).
    Legend,
}

/// Something met on the road (A3).
#[derive(Clone, Debug, PartialEq)]
pub struct Encounter {
    pub kind: EncKind,
    /// The announcement ("Wolves! Six of them, coming out of the pines.").
    pub text: String,
    pub figure: Option<u32>,
    /// What comes, and how many of each.
    pub foes: Vec<(Species, u8)>,
    /// The side of the map they come from.
    pub dir: (i32, i32),
    /// The side the hero walked in from (none for a camp).
    pub came: Option<(i32, i32)>,
    /// They fell on the camp in the night.
    pub ambush: bool,
    /// A patrol's people, for its arms.
    pub race: Option<Race>,
}

/// The eight ways, clockwise from north.
pub const DIRS: [(i32, i32); 8] = [(0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1)];

impl Adventure {
    /// One cell in direction (dx, dy), at the usual pace.
    pub fn travel(&mut self, dx: i32, dy: i32) -> TravelOutcome {
        self.travel_with(dx, dy, false)
    }

    /// One cell in direction (dx, dy); `quiet` keeps to cover, slower but
    /// seen by fewer.
    pub fn travel_with(&mut self, dx: i32, dy: i32, quiet: bool) -> TravelOutcome {
        if self.over.is_some() {
            return TravelOutcome::Blocked("Your journey is over.".into());
        }
        if self.visit.is_some() {
            self.leave();
        }
        let from = self.at;
        let to = (from.0 + dx, from.1 + dy);
        if !self.in_world(to) {
            return TravelOutcome::Blocked("There is nothing that way but the edge of the world.".into());
        }
        let Some(days) = self.crossing_days(to.0, to.1) else {
            let what = if self.ow.cell(to.0, to.1).biome == Biome::Lake { "A lake" } else { "The sea" };
            return TravelOutcome::Blocked(format!("{} bars the way.", what));
        };
        let t0 = self.clock;
        let w0 = self.weather;
        let ticks = (days * self.pace(quiet) * DAY as f64).round().max(10.0) as u64;
        self.at = to;
        self.stats.cells += 1;
        let died = self.walk(ticks, true);
        self.stats.days = ((self.clock - clock::DAWN_TICK.min(self.clock)) / DAY) as u32;
        self.road_log(from, to, t0, w0);
        if died {
            let cause = self.road_death(from, to);
            self.died_on_road(&cause);
            return TravelOutcome::Died(cause);
        }
        let mult = if quiet { self.knob("travel.quiet_enc", 0.5) } else { 1.0 };
        match self.roll_encounter_with(mult) {
            Some(mut e) => {
                e.came = Some((-dx, -dy));
                self.say(&e.text, (255, 110, 80));
                TravelOutcome::Encounter(e)
            }
            None => TravelOutcome::Arrived,
        }
    }

    /// How much longer than usual a step takes now: by night, with a limp,
    /// into a storm, or keeping to cover.
    pub fn pace(&self, quiet: bool) -> f64 {
        let mut m = 1.0;
        if clock::is_night(self.clock) {
            m *= self.knob("travel.night_mult", 1.5);
        }
        if self.hero.dwarf.body.limp() > 0 {
            m *= self.knob("travel.limp_mult", 1.5);
        }
        if matches!(self.weather, Weather::Blizzard | Weather::Storm) {
            m *= self.knob("travel.storm_mult", 2.0);
        }
        if quiet {
            m *= self.knob("travel.quiet_mult", 1.25);
        }
        m
    }

    /// Days to walk across a cell of the world (None: water).
    pub fn crossing_days(&self, x: i32, y: i32) -> Option<f64> {
        let ow = self.ow();
        let c = ow.cell(x, y);
        let base = match c.biome {
            Biome::Ocean | Biome::Lake => return None,
            Biome::Grassland | Biome::Savanna | Biome::Shrubland | Biome::Tundra | Biome::Desert => self.knob("travel.grass_days", 0.5),
            Biome::Forest | Biome::Hills | Biome::Taiga | Biome::Marsh => self.knob("travel.forest_days", 0.75),
            Biome::Mountain | Biome::Swamp | Biome::Jungle | Biome::Badlands => self.knob("travel.mountain_days", 1.0),
            Biome::Glacier => self.knob("travel.glacier_days", 1.5),
        };
        // A river with no ford: a settlement keeps a bridge or a ferry.
        let river = if c.river != 0 && c.site == 0 { self.knob("travel.river_days", 0.5) } else { 0.0 };
        Some(base + river)
    }

    /// The clock runs on `ticks` with the hero on the road (or lying still
    /// in camp): their wounds bleed and clot every 10 ticks and heal a
    /// little every 200, and the sky is read again each half day. True if
    /// they bled to death.
    fn walk(&mut self, ticks: u64, moving: bool) -> bool {
        let bp = BleedParams::default();
        let end = self.clock + ticks;
        while self.clock < end {
            let step = 10.min(end - self.clock);
            let before = self.clock;
            self.clock += step;
            let d = &mut self.hero.dwarf;
            d.body.tick_with(&bp, moving);
            if d.body.dead || d.body.blood <= 0 {
                return true;
            }
            if self.clock / 200 != before / 200 {
                d.body.heal(1);
            }
            for c in self.company.iter_mut().filter(|c| c.dwarf.alive) {
                c.dwarf.body.tick_with(&bp, moving);
                if c.dwarf.body.dead {
                    c.dwarf.alive = false;
                }
            }
            if self.clock / (DAY / 2) != before / (DAY / 2) {
                let (x, y) = self.at;
                let (biome, evil) = (self.ow().cell(x, y).biome, self.ow().cell(x, y).evil);
                let season = ((self.clock % YEAR) / SEASON) as usize;
                self.weather = roll_weather(biome, evil, season, self.rng());
            }
        }
        false
    }

    /// The hero bled out on the road: the end, and why.
    fn died_on_road(&mut self, cause: &str) {
        let name = self.hero.dwarf.name.clone();
        self.hero.dwarf.alive = false;
        self.hero.dwarf.body.dead = true;
        self.over = Some(AdvEnd::Died { cause: cause.to_string(), tick: self.clock });
        self.say(&format!("{} {}.", name, cause), (235, 60, 60));
        self.hero_died(cause);
    }

    /// "bled to death on the road to Copper Gate": the nearest place they
    /// were walking towards, else the land they fell in.
    fn road_death(&self, from: (i32, i32), to: (i32, i32)) -> String {
        let ow = self.ow();
        let toward = ow
            .sites
            .iter()
            .filter(|s| !s.ruined && s.kind != SiteKind::Lair)
            .filter(|s| cheb((s.x, s.y), to) < cheb((s.x, s.y), from) && cheb((s.x, s.y), to) <= 6)
            .min_by_key(|s| (cheb((s.x, s.y), to), s.id));
        match toward {
            Some(s) => format!("bled to death on the road to {}", s.name),
            None => format!("bled to death on the road, in {}", land_name(ow, to.0, to.1)),
        }
    }

    /// The chance of meeting something on a step into `cell` (or of being
    /// found in camp there): the land's savagery and taint, the lairs and
    /// hostile sites near it, the night, and the hero's fame and infamy.
    pub fn encounter_chance(&self, cell: (i32, i32)) -> f64 {
        let on = self.knob("travel.encounters", 1.0);
        if on <= 0.0 || !self.in_world(cell) {
            return 0.0;
        }
        let ow = self.ow();
        let c = ow.cell(cell.0, cell.1);
        let mut ch = self.knob("travel.enc_base", 0.03);
        ch += c.biome.params().savagery * ow.savagery * self.knob("travel.enc_savage", 0.25);
        if c.evil {
            ch += self.knob("travel.enc_evil", 0.08);
        }
        ch += ow.beasts_near(cell.0, cell.1, 4).len().min(3) as f64 * self.knob("travel.enc_lair", 0.04);
        ch += self.hostile_near(cell, 3).len().min(3) as f64 * self.knob("travel.enc_enemy", 0.05);
        let renown = (self.fame.renown.max(0) as f64 / 100.0).min(1.0);
        let infamy = (self.fame.infamy.max(0) as f64 / 100.0).min(1.0);
        ch += renown * self.knob("travel.enc_fame", 0.04) + infamy * self.knob("travel.enc_infamy", 0.06);
        if clock::is_night(self.now()) {
            ch *= self.knob("travel.enc_night", 1.6);
        }
        (ch * on).min(0.9)
    }

    /// A roll for an encounter where the hero stands.
    pub fn roll_encounter(&mut self) -> Option<Encounter> {
        self.roll_encounter_with(1.0)
    }

    fn roll_encounter_with(&mut self, mult: f64) -> Option<Encounter> {
        let p = self.encounter_chance(self.at) * mult;
        if p <= 0.0 || !self.rng().chance(p) {
            return None;
        }
        self.pick_encounter()
    }

    /// What comes, weighed by the land: its hunters, patrols out of hostile
    /// sites, bandits, the dead on tainted land at night, and (rarely) a
    /// beast of legend whose lair is near.
    fn pick_encounter(&mut self) -> Option<Encounter> {
        let (x, y) = self.at;
        let night = clock::is_night(self.now());
        let ow = self.ow();
        let c = ow.cell(x, y);
        let mut options: Vec<(f64, EncKind)> = vec![(4.0 + 10.0 * c.biome.params().savagery, EncKind::Beasts), (1.0, EncKind::Bandits)];
        let evil = c.evil;
        let legend = ow.beasts_near(x, y, 4).first().copied();
        let biome = c.biome;
        let hostile = self.hostile_near((x, y), 3);
        if !hostile.is_empty() {
            options.push((3.0 * hostile.len().min(3) as f64, EncKind::Patrol));
        }
        if evil && night {
            options.push((4.0, EncKind::Dead));
        }
        if legend.is_some() {
            options.push((self.knob("travel.legend_weight", 1.0), EncKind::Legend));
        }
        let total: f64 = options.iter().map(|o| o.0).sum();
        let mut r = self.rng().f64() * total;
        let mut kind = EncKind::Beasts;
        for (w, k) in options {
            if r < w {
                kind = k;
                break;
            }
            r -= w;
        }
        let dir = DIRS[self.rng().range(0, 8) as usize];
        let cover = cover(biome);
        let way = dir_words(dir.0, dir.1);
        let mut e = Encounter { kind, text: String::new(), figure: None, foes: Vec::new(), dir, came: None, ambush: false, race: None };
        match kind {
            EncKind::Beasts => {
                let (sp, n) = self.hunters(night);
                let info = sp.info();
                e.text = if n == 1 {
                    format!("{}! It comes out of the {} to the {}, and it has your scent.", cap(&lore::a_or_an(info.name)), cover, way)
                } else {
                    format!("{}! {} of them, coming out of the {} to the {}.", cap(info.plural), cap(number(n)), cover, way)
                };
                e.foes.push((sp, n));
            }
            EncKind::Patrol => {
                let site = hostile[self.rng().range(0, hostile.len() as i32) as usize];
                let n = self.rng().range(2, 6) as u8;
                let ow = self.ow();
                let s = &ow.sites[site];
                let race = sitegen::civ_race(ow, s.civ).unwrap_or(Race::Goblin);
                e.text = format!("A {} patrol out of {}, {} strong, marching from the {}. They have seen you.", lore::race_adj(race), s.name, number(n), way);
                e.foes.push((sitegen::soldier(race), n));
                e.race = Some(race);
            }
            EncKind::Bandits => {
                let n = self.rng().range(2, 5) as u8;
                e.text = format!("Bandits! {} of them rise from the {} to the {}, blades out.", cap(number(n)), cover, way);
                e.foes.push((Species::Raider, n));
            }
            EncKind::Dead => {
                let n = self.rng().range(2, 6) as u8;
                e.text = format!("Shapes stagger out of the dark to the {}: {} husks, the dead that walk on tainted land.", way, number(n));
                e.foes.push((Species::Husk, n));
            }
            EncKind::Legend => {
                let f = legend?;
                let ow = self.ow();
                let fig = &ow.figures[f as usize];
                let lair = fig.lair.and_then(|l| ow.sites.get(l as usize)).map_or("its lair".to_string(), |s| s.name.clone());
                e.text = format!("{} the {}! The terror of {} has caught your scent, and comes from the {}.", fig.name, fig.kind, lair, way);
                e.figure = Some(f);
                e.foes.push((Species::Megabeast, 1));
            }
        }
        Some(e)
    }

    /// The land's hunters to meet: a pack of its hunters, or now and then
    /// its rare beast alone; by night, those that hunt by night first.
    fn hunters(&mut self, night: bool) -> (Species, u8) {
        use crate::worldgen::regions::{role, Role};
        let (x, y) = self.at;
        let fauna: Vec<Species> = self.ow().region_at(x, y).map(|r| r.fauna.clone()).unwrap_or_default();
        let nocturnal = |s: &Species| s.beast().map_or(false, |b| b.has(crate::bestiary::NOCTURNAL));
        let pack: Vec<Species> = fauna.iter().copied().filter(|s| role(*s) == Some(Role::Hunter)).collect();
        let rare: Vec<Species> = fauna.iter().copied().filter(|s| role(*s) == Some(Role::Rare)).collect();
        let lone = !rare.is_empty() && (pack.is_empty() || self.rng().chance(0.25));
        let from = if lone { &rare } else { &pack };
        let by_night: Vec<Species> = from.iter().copied().filter(nocturnal).collect();
        let list = if night && !by_night.is_empty() { by_night } else { from.clone() };
        let sp = if list.is_empty() { Species::Wolf } else { list[self.rng().range(0, list.len() as i32) as usize] };
        if lone {
            return (sp, 1);
        }
        let (lo, hi) = sp.beast().map_or((2, 5), |b| (b.group.0.max(2), b.group.1.clamp(2, 6)));
        (sp, self.rng().range(lo as i32, hi.max(lo) as i32 + 1) as u8)
    }

    /// Living sites within `r` cells of `cell` whose folk would come for
    /// the hero (indices into `ow.sites`), nearest first.
    pub fn hostile_near(&self, cell: (i32, i32), r: i32) -> Vec<usize> {
        let ow = self.ow();
        let mut v: Vec<(i32, usize)> = ow
            .sites
            .iter()
            .enumerate()
            .filter(|(_, s)| !s.ruined && s.kind != SiteKind::Lair && s.civ.map_or(false, |c| ow.civs.get(c as usize).map_or(false, |c| c.alive)))
            .filter(|(_, s)| cheb((s.x, s.y), cell) <= r)
            .filter(|(_, s)| peoples::stance(ow, self.hero.race, self.hero.civ, s) == Stance::Hostile)
            .map(|(i, s)| (cheb((s.x, s.y), cell), i))
            .collect();
        v.sort();
        v.into_iter().map(|(_, i)| i).collect()
    }

    /// Whether the hero, seeing them first, slips away unseen: likelier
    /// keeping to cover, in the dark, and seldom from a beast of legend
    /// that has the scent.
    pub fn evade(&mut self, e: &Encounter, quiet: bool) -> bool {
        let mut p = self.knob("travel.evade", 0.4);
        if quiet {
            p += 0.2;
        }
        if clock::is_night(self.now()) {
            p += 0.15;
        }
        if e.ambush {
            p -= 0.3;
        }
        if e.kind == EncKind::Legend {
            p -= 0.25;
        }
        if self.hero.dwarf.body.limp() > 0 {
            p -= 0.2;
        }
        let ok = self.rng().chance(p.clamp(0.05, 0.9));
        if ok {
            self.say("You keep low and slip away before they find you.", (170, 210, 150));
        } else {
            self.say("They have seen you! There is no slipping away now.", (255, 110, 80));
        }
        ok
    }

    /// Stand and face it: into the wild land of the cell, the hero on the
    /// side they walked in from (or in the middle of their camp), and what
    /// comes on the map's edge on its own side, out for blood.
    pub fn meet(&mut self, e: &Encounter) -> Result<(), String> {
        let (x, y) = self.at;
        let came = if e.ambush { None } else { e.came };
        self.enter_from(SiteKey::Wild(x, y), came)?;
        let Some(v) = self.visit.as_mut() else { return Err("No land to stand on.".into()) };
        let g = &mut v.g;
        let me = g.dwarves[v.hero].pos;
        let (w, h) = (g.world.w, g.world.h);
        // Where they come in: the edge on their side, level with the hero,
        // and never right on top of them.
        let mut ex = if e.dir.0 < 0 { 0 } else if e.dir.0 > 0 { w - 1 } else { me.x };
        let mut ey = if e.dir.1 < 0 { 0 } else if e.dir.1 > 0 { h - 1 } else { me.y };
        if (ex - me.x).abs().max((ey - me.y).abs()) < 12 {
            if e.dir.0 == 0 {
                ex = if me.x < w / 2 { (me.x + 16).min(w - 1) } else { (me.x - 16).max(0) };
            } else {
                ey = if me.y < h / 2 { (me.y + 16).min(h - 1) } else { (me.y - 16).max(0) };
            }
        }
        let mut at = Pos::new(ex, ey, g.world.surface_z(ex, ey));
        if !g.world.walkable(at) {
            at = g.edge_entry().unwrap_or(me.offset(10, 0, 0));
        }
        for &(sp, n) in &e.foes {
            for _ in 0..n {
                let p = g.spot_near(at, 2);
                let id = match sp {
                    Species::Beast(_) => g.spawn_beast(sp, p, DAY * 3),
                    _ => g.spawn_creature(sp, p, DAY * 3),
                };
                let figure = e.figure.and_then(|f| g.overworld.figures.get(f as usize)).map(|f| (f.id, f.name.clone(), f.kind.clone()));
                let Some(c) = g.creatures.get_mut(&id) else { continue };
                c.siege = true;
                c.target = Some(HERO_ID);
                if let Some((f, name, kind)) = figure {
                    c.figure = Some(f);
                    c.label = Some(format!("{} the {}", name, kind));
                    c.breathes_fire = kind == "dragon";
                }
                match (e.kind, e.race) {
                    (EncKind::Patrol, Some(race)) => {
                        let (weapon, mat) = match race {
                            Race::Dwarf => (ItemKind::Axe, Mat::Iron),
                            Race::Human => (ItemKind::Sword, Mat::Iron),
                            Race::Elf => (ItemKind::Spear, Mat::Wood),
                            Race::Goblin => (ItemKind::Spear, Mat::Iron),
                        };
                        sitegen::arm(g, id, weapon, mat);
                        sitegen::arm(g, id, ItemKind::Helm, if race == Race::Elf { Mat::Wood } else { Mat::Copper });
                    }
                    (EncKind::Bandits, _) => sitegen::arm(g, id, if id % 2 == 0 { ItemKind::Sword } else { ItemKind::Axe }, Mat::Copper),
                    _ => {}
                }
            }
        }
        let text = e.text.clone();
        g.msg(&text, (255, 110, 80));
        if e.kind == EncKind::Legend {
            g.spotlight("A beast of legend!", &text, Some(at), (255, 60, 60));
        }
        Ok(())
    }

    /// Make camp where the hero stands and rest until dawn, healing. The
    /// night may bring an ambush (half the chance of meeting something on
    /// the road; less near a friendly site, less with a companion keeping
    /// watch): then the camp is woken at that hour and it is returned.
    pub fn camp(&mut self) -> Option<Encounter> {
        if self.over.is_some() || self.visit.is_some() {
            return None;
        }
        let now = self.clock;
        let mut dawn = now - now % DAY + clock::DAWN_TICK;
        if dawn <= now + clock::HOUR {
            dawn += DAY;
        }
        let (x, y) = self.at;
        let land = land_name(self.ow(), x, y);
        let mut p = self.encounter_chance((x, y)) * self.knob("travel.camp_enc", 0.5);
        if self.friendly_near((x, y), 1) {
            p *= 0.5;
        }
        if self.company.iter().any(|c| c.dwarf.alive) {
            p *= 0.6;
        }
        let ambush = self.rng().chance(p.min(1.0));
        let hurt = !self.hero.dwarf.body.healed();
        let h = names_hash(&[self.clock, x as u64, y as u64]);
        let fire = crate::game::stories::pick(h, 1, &["You make camp", "You build a small fire", "You make a cold camp", "You bed down"]);
        let moon = format!(" under {}", clock::moon_words(now));
        self.say_at(&format!("{} in {}{}.", fire, land, moon), (200, 190, 170), None);
        if ambush {
            let at = now + (dawn - now) * self.rng().range(40, 90) as u64 / 100;
            if self.walk(at - now, false) {
                let cause = format!("bled to death in camp, in {}", land);
                self.died_on_road(&cause);
                return None;
            }
            let mut e = self.pick_encounter()?;
            e.ambush = true;
            let text = format!("You wake to a sound in the dark... {}", e.text);
            e.text = text;
            self.say(&e.text, (255, 110, 80));
            return Some(e);
        }
        if self.walk(dawn - now, false) {
            let cause = format!("bled to death in camp, in {}", land);
            self.died_on_road(&cause);
            return None;
        }
        let rise = if hurt && self.hero.dwarf.body.healed() {
            "You wake at dawn, your wounds closed."
        } else if hurt {
            "You wake at dawn, stiff and sore."
        } else {
            "You break camp at dawn."
        };
        self.say(rise, (255, 215, 140));
        None
    }

    /// Leave the site the hero is in for the world map.
    pub fn take_road(&mut self) {
        let Some(key) = self.leave() else { return };
        let name = super::sites::site_name(self.ow(), key);
        let (x, y) = self.at;
        let land = land_name(self.ow(), x, y);
        let text = match key {
            SiteKey::Wild(..) => format!("You take to the road in {}.", land),
            _ => format!("You leave {} behind and take to the road.", name),
        };
        self.say_at(&text, (255, 215, 140), Some((x, y)));
    }

    /// The road walked: the cells the travel log names, oldest first.
    pub fn road(&self) -> Vec<(i32, i32)> {
        self.log.iter().filter_map(|e| e.at.map(|p| (p.x, p.y))).collect()
    }

    /// A line in the travel log, naming a cell of the road.
    fn say_at(&mut self, text: &str, color: (u8, u8, u8), cell: Option<(i32, i32)>) {
        if self.visit.is_some() {
            return self.say(text, color);
        }
        let tick = self.clock;
        self.log.push(LogEntry { text: text.to_string(), color, tick, at: cell.map(|(x, y)| Pos::new(x, y, 0)), kind: Default::default() });
    }

    fn in_world(&self, (x, y): (i32, i32)) -> bool {
        let ow = self.ow();
        ow.w > 1 && x >= 0 && y >= 0 && x < ow.w && y < ow.h
    }

    /// A living site of a people who would take the hero in, within `r`.
    fn friendly_near(&self, cell: (i32, i32), r: i32) -> bool {
        let ow = self.ow();
        ow.sites.iter().filter(|s| !s.ruined && s.kind != SiteKind::Lair && cheb((s.x, s.y), cell) <= r).any(|s| peoples::stance(ow, self.hero.race, self.hero.civ, s) <= Stance::Friendly)
    }

    /// The travel log of one step: the land and the hour, the weather,
    /// a ford, nightfall or dawn on the way, what is seen from the road,
    /// and how the hero's body holds up.
    fn road_log(&mut self, from: (i32, i32), to: (i32, i32), t0: u64, w0: Weather) {
        let (main, extra) = {
            let ow = self.ow();
            let c = ow.cell(to.0, to.1);
            let h = names_hash(&[t0, to.0 as u64, to.1 as u64]);
            let new_region = ow.cell(from.0, from.1).region != c.region;
            let land = land_name(ow, to.0, to.1);
            let when = when_words(t0);
            let verb = if new_region {
                crate::game::stories::pick(h, 2, &["You come into", "The road brings you into", "You enter"])
            } else {
                crate::game::stories::pick(h, 2, verbs(c.biome))
            };
            let sky = match self.weather {
                Weather::Clear => "",
                Weather::Rain => ", in the rain",
                Weather::Snow => ", through falling snow",
                Weather::Storm => ", under a sky of thunder",
                Weather::Blizzard => ", head down into a blizzard",
                Weather::BloodRain => ", and the rain that falls is red",
            };
            let main = format!("{} {} {}{}.", verb, land, when, sky);
            let mut extra: Vec<(String, (u8, u8, u8))> = Vec::new();
            // Nightfall or daybreak on the way.
            let (n0, n1) = (clock::is_night(t0), clock::is_night(self.clock));
            if !n0 && n1 {
                extra.push((format!("Night falls on the road. {} rises.", cap(clock::moon_words(self.clock))), (150, 160, 210)));
            } else if n0 && !n1 {
                extra.push((format!("Dawn breaks over {}.", land), (255, 215, 140)));
            }
            if self.weather != w0 {
                let t = match self.weather {
                    Weather::Clear => "The sky clears.",
                    Weather::Rain => "Rain sets in.",
                    Weather::Snow => "Snow begins to fall.",
                    Weather::Storm => "Thunder rolls; a storm breaks over you.",
                    Weather::Blizzard => "The wind rises and the snow comes sideways.",
                    Weather::BloodRain => "The rain that falls here is the colour of blood.",
                };
                extra.push((t.to_string(), (160, 190, 230)));
            }
            if c.river != 0 && c.site == 0 {
                let r = ow.river_at(to.0, to.1).map_or("the river".to_string(), |r| r.name.clone());
                extra.push((format!("You ford {}, waist-deep and cold.", r), (90, 200, 240)));
            }
            if let Some(sight) = self.sight(to) {
                extra.push(sight);
            }
            (main, extra)
        };
        self.say_at(&main, (220, 210, 190), Some(to));
        for (t, col) in extra {
            self.say(&t, col);
        }
        // The body on the road.
        let b = &self.hero.dwarf.body;
        let faint = b.blood * 2 < b.max_blood;
        let bleeding = b.bleed_rate() > 0;
        let limp = b.limp() > 0;
        let part = b.worst_bleed().map_or("wound", |p| p.name());
        if b.dead {
        } else if faint {
            self.say("You are growing faint; the road swims before your eyes.", (235, 60, 60));
        } else if bleeding {
            self.say(&format!("Your {} bleeds with every step.", part), (235, 90, 80));
        } else if limp {
            self.say("You limp on.", (240, 150, 110));
        }
    }

    /// Something seen from the road at `at`: the most striking place within
    /// three cells not already told of in the last stretch of the log.
    fn sight(&self, at: (i32, i32)) -> Option<(String, (u8, u8, u8))> {
        let ow = self.ow();
        let night = clock::is_night(self.clock);
        let told = |name: &str| self.log.iter().rev().take(40).any(|e| e.text.contains(name));
        let mut best: Option<(i32, i32, String, (u8, u8, u8))> = None;
        let mut offer = |rank: i32, d: i32, text: String, col: (u8, u8, u8)| {
            if best.as_ref().map_or(true, |b| (rank, -d) > (b.0, -b.1)) {
                best = Some((rank, d, text, col));
            }
        };
        // A fallen fort first: the crimson ruin.
        for f in ow.forts.iter() {
            let d = cheb(f.site, at);
            if d > 3 || told(&f.name) {
                continue;
            }
            let way = way_to(at, f.site);
            let fell = match (f.fell, &f.killer) {
                (Some(y), Some(k)) => format!(", which fell in {} to {}", y, lore::a_or_an(k)),
                (Some(y), None) => format!(", which fell in {}", y),
                _ => String::new(),
            };
            offer(5, d, format!("{}, the broken gate of {}{}.", cap(&way), f.name, fell), (220, 60, 70));
        }
        for (i, s) in ow.sites.iter().enumerate() {
            let d = cheb((s.x, s.y), at);
            if d > 3 || told(&s.name) {
                continue;
            }
            let way = way_to(at, (s.x, s.y));
            if s.ruined {
                let text = match lore::fall_of(ow, s) {
                    Some((lore::Fell::Beast { name, kind }, _, _)) if kind == "dragon" => format!("Smoke still rises {}, where {} burned under {}'s fire.", way, s.name, name),
                    Some((lore::Fell::Beast { name, kind }, year, _)) => format!("{}, the broken walls of {}, laid waste by {} the {} in {}.", cap(&way), s.name, name, kind, year),
                    Some((lore::Fell::Army { who }, year, _)) => format!("Crows circle {}, over the ruins of {}, razed by {} in {}.", way, s.name, who, year),
                    None => format!("{}, the tumbled stones of {}.", cap(&way), s.name),
                };
                offer(4, d, text, (200, 170, 150));
                continue;
            }
            match s.kind {
                SiteKind::Lair => {
                    let Some(f) = ow.figures.iter().find(|f| f.lair == Some(i as u32)) else { continue };
                    if f.alive() {
                        offer(4, d, format!("{}, the lair of {} the {}. Nothing grazes near it.", cap(&way), f.name, f.kind), (255, 110, 230));
                    } else {
                        offer(1, d, format!("{}, the empty lair of {} the {}.", cap(&way), f.name, f.kind), (200, 150, 200));
                    }
                }
                SiteKind::DarkFortress => offer(3, d, format!("The black towers of {} loom {}.", s.name, way), (200, 90, 90)),
                SiteKind::Retreat => offer(2, d, format!("Somewhere among the trees {} lies {}, a retreat of the elves.", way, s.name), (120, 220, 140)),
                _ => {
                    let hostile = peoples::stance(ow, self.hero.race, self.hero.civ, s) == Stance::Hostile;
                    let text = if hostile {
                        format!("The walls of {} stand {}. Its folk would not welcome you.", s.name, way)
                    } else if night {
                        format!("The lights of {} glimmer {}.", s.name, way)
                    } else {
                        format!("Hearth-smoke rises from {}, {}.", s.name, way)
                    };
                    offer(if hostile { 3 } else { 2 }, d, text, if hostile { (240, 150, 110) } else { (255, 230, 150) });
                }
            }
        }
        best.map(|b| (b.2, b.3))
    }
}

/// The weather for half a day in a land: wet lands rain, cold ones (and
/// all but the hot lands in winter) snow, summer brings storms, and
/// tainted land sometimes rains blood.
pub fn roll_weather(biome: Biome, evil: bool, season: usize, rng: &mut Rng) -> Weather {
    if evil && rng.chance(0.12) {
        return Weather::BloodRain;
    }
    let hot = matches!(biome, Biome::Desert | Biome::Jungle | Biome::Savanna | Biome::Badlands);
    let cold = biome.cold() || (season == 3 && !hot);
    let wet = match biome {
        Biome::Swamp | Biome::Marsh | Biome::Jungle => 0.45,
        Biome::Forest | Biome::Taiga => 0.3,
        Biome::Desert | Biome::Badlands => 0.03,
        Biome::Glacier | Biome::Tundra | Biome::Mountain => 0.3,
        _ => 0.2,
    };
    if !rng.chance(wet) {
        return Weather::Clear;
    }
    if cold {
        if rng.chance(0.25) {
            Weather::Blizzard
        } else {
            Weather::Snow
        }
    } else if rng.chance(if season == 1 { 0.35 } else { 0.1 }) {
        Weather::Storm
    } else {
        Weather::Rain
    }
}

/// Chebyshev distance between cells.
pub fn cheb(a: (i32, i32), b: (i32, i32)) -> i32 {
    (a.0 - b.0).abs().max((a.1 - b.1).abs())
}

/// "north-east", from a direction.
pub fn dir_words(dx: i32, dy: i32) -> &'static str {
    match (dx.signum(), dy.signum()) {
        (0, -1) => "north",
        (1, -1) => "north-east",
        (1, 0) => "east",
        (1, 1) => "south-east",
        (0, 1) => "south",
        (-1, 1) => "south-west",
        (-1, 0) => "west",
        (-1, -1) => "north-west",
        _ => "here",
    }
}

/// "to the east" (or "close by") of `to` from `from`, a long way off
/// along one axis counting as straight.
pub fn way_to(from: (i32, i32), to: (i32, i32)) -> String {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    if dx == 0 && dy == 0 {
        return "close by".to_string();
    }
    let sx = if dx.abs() * 2 < dy.abs() { 0 } else { dx.signum() };
    let sy = if dy.abs() * 2 < dx.abs() { 0 } else { dy.signum() };
    format!("to the {}", dir_words(sx, sy))
}

/// What a cell of land is called in the log: its region, else its land.
pub fn land_name(ow: &Overworld, x: i32, y: i32) -> String {
    if ow.w <= 1 || x < 0 || y < 0 || x >= ow.w || y >= ow.h {
        return "the wilds".to_string();
    }
    match ow.region_at(x, y) {
        Some(r) if r.name.starts_with("the ") => r.name.clone(),
        Some(r) if r.name.starts_with("The ") => format!("the {}", &r.name[4..]),
        Some(r) => format!("the {}", r.name),
        None => format!("the {}", ow.cell(x, y).biome.name().to_lowercase()),
    }
}

/// "at dusk", "by night": when a step began.
fn when_words(tick: u64) -> &'static str {
    match clock::hour(tick) {
        0..=2 => "in the dead of night",
        3..=4 => "before dawn",
        5..=6 => "at dawn",
        7..=10 => "in the morning",
        11..=13 => "at midday",
        14..=17 => "in the afternoon",
        18..=19 => "at dusk",
        _ => "by night",
    }
}

/// Ways to say the hero goes on through a land.
fn verbs(b: Biome) -> &'static [&'static str] {
    match b {
        Biome::Mountain | Biome::Badlands => &["You climb through", "You pick your way across", "You scramble over the rocks of"],
        Biome::Hills => &["You cross", "You walk the ridges of", "You climb on through"],
        Biome::Forest | Biome::Taiga | Biome::Jungle => &["You push on through", "You walk beneath the trees of", "You follow a deer-track through"],
        Biome::Swamp | Biome::Marsh => &["You wade on through", "You slog across", "You pick a path through"],
        Biome::Glacier | Biome::Tundra => &["You trudge across", "You cross", "You lean into the wind across"],
        Biome::Desert => &["You trudge across", "You cross the sands of", "You walk on across"],
        _ => &["You cross", "You walk on across", "You make good time across"],
    }
}

/// What foes come out of, in a land.
fn cover(b: Biome) -> &'static str {
    match b {
        Biome::Taiga => "pines",
        Biome::Forest => "trees",
        Biome::Jungle => "undergrowth",
        Biome::Mountain | Biome::Hills | Biome::Badlands => "rocks",
        Biome::Swamp | Biome::Marsh => "reeds",
        Biome::Desert => "dunes",
        Biome::Tundra | Biome::Glacier => "blowing snow",
        Biome::Shrubland => "scrub",
        _ => "long grass",
    }
}

/// A number in words, up to twelve.
pub fn number(n: u8) -> &'static str {
    ["none", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve"].get(n as usize).copied().unwrap_or("many")
}

fn cap(s: &str) -> String {
    lore::cap(s)
}

fn names_hash(parts: &[u64]) -> u64 {
    crate::game::names::hash(parts)
}

#[cfg(test)]
mod tests {
    use super::super::tests::test_hero;
    use super::*;
    use crate::body::Part;

    fn world() -> Overworld {
        static WORLD: std::sync::OnceLock<Overworld> = std::sync::OnceLock::new();
        WORLD.get_or_init(|| Overworld::generate(5)).clone()
    }

    /// An adventure on the road (no site), meeting nothing unless asked.
    fn on_road(seed: u64) -> Adventure {
        let mut a = Adventure::new(world(), seed, test_hero(seed), None);
        a.knobs.set.insert("travel.encounters".into(), 0.0);
        a
    }

    /// A land cell with `n` cells of `want` to the east of it.
    fn row_of(a: &Adventure, want: Biome, n: i32) -> Option<(i32, i32)> {
        let ow = a.ow();
        (0..ow.h).flat_map(|y| (0..ow.w - n - 1).map(move |x| (x, y))).find(|&(x, y)| {
            !ow.cell(x, y).biome.is_water() && (1..=n).all(|i| ow.cell(x + i, y).biome == want && ow.cell(x + i, y).river == 0)
        })
    }

    #[test]
    fn a_day_across_the_grass_and_three_across_the_mountains() {
        let mut a = on_road(31);
        let start = row_of(&a, Biome::Grassland, 2).expect("grassland");
        a.at = start;
        a.clock = clock::DAWN_TICK;
        let t = a.clock;
        assert_eq!(a.travel(1, 0), TravelOutcome::Arrived);
        assert_eq!(a.travel(1, 0), TravelOutcome::Arrived);
        let storms = a.log.iter().any(|e| e.text.contains("storm") || e.text.contains("blizzard"));
        if !storms {
            assert_eq!(a.clock - t, DAY, "two cells of grass by day are a day");
        }
        assert_eq!(a.at, (start.0 + 2, start.1));
        // Three mountain cells: a day each by day, longer by night.
        let mut a = on_road(32);
        let start = row_of(&a, Biome::Mountain, 3).expect("mountains");
        a.at = start;
        a.clock = clock::DAWN_TICK;
        for _ in 0..3 {
            let before = a.clock;
            let pace = a.pace(false);
            assert_eq!(a.travel(1, 0), TravelOutcome::Arrived);
            assert_eq!(a.clock - before, (DAY as f64 * pace).round() as u64);
        }
        assert!(a.clock - clock::DAWN_TICK >= DAY * 3);
        // The road is logged, cell by cell, and the sea bars the way.
        assert_eq!(a.road().len(), 3);
        assert!(a.log.iter().any(|e| e.text.starts_with("You")), "{:?}", a.log.iter().map(|e| &e.text).collect::<Vec<_>>());
        let ow = a.ow();
        let shore = (0..ow.h).flat_map(|y| (0..ow.w - 1).map(move |x| (x, y))).find(|&(x, y)| !ow.cell(x, y).biome.is_water() && ow.cell(x + 1, y).biome == Biome::Ocean).expect("a shore");
        a.at = shore;
        assert!(matches!(a.travel(1, 0), TravelOutcome::Blocked(_)));
        assert_eq!(a.at, shore);
        // Night is slower.
        a.clock = clock::DAWN_TICK - clock::HOUR * 3;
        a.weather = Weather::Clear;
        assert_eq!(a.pace(false), 1.5);
        // The weather follows the land.
        let mut kinds = std::collections::BTreeSet::new();
        let mut rng = Rng::new(3);
        for i in 0..400 {
            kinds.insert(format!("{:?}", roll_weather(Biome::Swamp, false, i % 4, &mut rng)));
        }
        assert!(kinds.contains("Rain") && kinds.contains("Snow") && kinds.contains("Clear"), "{:?}", kinds);
        let dry = (0..200).filter(|_| roll_weather(Biome::Desert, false, 1, &mut rng) == Weather::Clear).count();
        assert!(dry > 180, "{}", dry);
    }

    #[test]
    fn savage_land_is_dangerous() {
        let mut a = on_road(41);
        a.knobs.set.insert("travel.encounters".into(), 1.0);
        a.clock = clock::DAWN_TICK + clock::HOUR * 6;
        // The calmest land cell and the most savage one.
        let ow = a.ow();
        let land: Vec<(i32, i32)> = (0..ow.h).flat_map(|y| (0..ow.w).map(move |x| (x, y))).filter(|&(x, y)| !ow.cell(x, y).biome.is_water()).collect();
        let calm = *land.iter().min_by(|p, q| a.encounter_chance(**p).partial_cmp(&a.encounter_chance(**q)).unwrap()).unwrap();
        let wild = *land.iter().max_by(|p, q| a.encounter_chance(**p).partial_cmp(&a.encounter_chance(**q)).unwrap()).unwrap();
        let rate = |a: &mut Adventure, at: (i32, i32)| {
            a.at = at;
            (0..10_000).filter(|_| a.roll_encounter().is_some()).count()
        };
        let (c, w) = (rate(&mut a, calm), rate(&mut a, wild));
        assert!(w >= 3 * c, "savage {} vs calm {}", w, c);
        assert!(c > 0);
        a.clock += clock::HOUR * 14;
        assert!(clock::is_night(a.clock));
        let n = rate(&mut a, calm);
        assert!(n > c, "night {} vs day {}", n, c);
        // Every encounter is told, and something comes.
        a.at = wild;
        for _ in 0..200 {
            if let Some(e) = a.roll_encounter() {
                assert!(!e.text.is_empty() && !e.foes.is_empty() && e.foes.iter().all(|f| f.1 > 0), "{:?}", e);
            }
        }
    }

    #[test]
    fn a_wounded_hero_bleeds_on_the_road() {
        let mut a = on_road(51);
        let start = row_of(&a, Biome::Grassland, 3).or_else(|| row_of(&a, Biome::Forest, 3)).expect("land");
        a.at = start;
        let b = &mut a.hero.dwarf.body;
        // (A mangled arm: it bleeds for a day's walk before it clots.)
        b.wound(Part::LeftArm, 20, true);
        let blood0 = b.blood;
        assert!(b.bleed_rate() > 0);
        for _ in 0..3 {
            if !matches!(a.travel(1, 0), TravelOutcome::Arrived) {
                break;
            }
        }
        assert!(a.hero.dwarf.body.blood < blood0, "blood lost on the road");
        assert!(a.over.is_none(), "an arm wound is survivable");
        // A deep wound, walked on: the road ends in the grass.
        let mut a = on_road(52);
        a.at = start;
        let b = &mut a.hero.dwarf.body;
        b.wound(Part::UpperBody, 40, true);
        b.wound(Part::LeftLeg, 20, true);
        let mut end = None;
        for _ in 0..3 {
            match a.travel(1, 0) {
                TravelOutcome::Died(c) => {
                    end = Some(c);
                    break;
                }
                TravelOutcome::Arrived => {}
                o => panic!("{:?}", o),
            }
        }
        let cause = end.expect("bled out on the road");
        assert!(cause.starts_with("bled to death"), "{}", cause);
        assert!(matches!(&a.over, Some(AdvEnd::Died { cause: c, .. }) if c == &cause));
        assert!(!a.hero.dwarf.alive);
        assert!(a.log.iter().any(|e| e.text.contains("bled to death")));
    }

    #[test]
    fn camping_heals_and_the_night_can_bring_foes() {
        let mut a = on_road(61);
        let start = row_of(&a, Biome::Grassland, 1).expect("land");
        a.at = start;
        a.hero.dwarf.body.hp[Part::LeftArm.idx()] -= 3;
        let hp = a.hero.dwarf.body.hp[Part::LeftArm.idx()];
        assert!(a.camp().is_none());
        assert_eq!(clock::hour(a.clock), 6, "up at dawn");
        assert!(a.hero.dwarf.body.hp[Part::LeftArm.idx()] > hp, "healed in the night");
        // With the dice loaded, they come in the night.
        a.knobs.set.insert("travel.encounters".into(), 1.0);
        a.knobs.set.insert("travel.camp_enc".into(), 50.0);
        let e = a.camp().expect("an ambush");
        assert!(e.ambush && e.came.is_none());
        assert!(clock::hour(a.clock) != 6, "woken before dawn");
    }

    #[test]
    fn an_encounter_comes_from_the_edge() {
        let mut a = on_road(71);
        let start = row_of(&a, Biome::Grassland, 1).or_else(|| row_of(&a, Biome::Forest, 1)).expect("land");
        a.at = start;
        a.knobs.set.insert("site.flat".into(), 1.0);
        let e = Encounter {
            kind: EncKind::Beasts,
            text: "Wolves! Three of them, coming out of the long grass to the north.".into(),
            figure: None,
            foes: vec![(Species::Wolf, 3)],
            dir: (0, -1),
            came: Some((-1, 0)),
            ambush: false,
            race: None,
        };
        a.meet(&e).unwrap();
        let v = a.visit.as_ref().unwrap();
        let g = &v.g;
        let me = g.dwarves[v.hero].pos;
        assert_eq!(me.x, 0, "the hero comes in on the west edge, the way they walked");
        let wolves: Vec<Pos> = g.creatures.values().filter(|c| c.species == Species::Wolf && c.target == Some(HERO_ID)).map(|c| c.pos).collect();
        assert_eq!(wolves.len(), 3);
        assert!(wolves.iter().all(|p| p.y <= 3), "from the north edge: {:?}", wolves);
        assert!(g.log.iter().any(|l| l.text.starts_with("Wolves!")));
    }
}
