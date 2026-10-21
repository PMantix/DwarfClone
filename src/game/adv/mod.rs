//! Adventurer mode (G03): one hero against the world. See plan/adventure.md.
//!
//! The adventure is a small driver around many short-lived `Game`s, one per
//! site visit, the way the arena drives a `Battle`. `Adventure` owns the
//! overworld, the hero and the clock; entering a site builds a `Game` for
//! that map (`sites::enter`), puts the hero in its `dwarves` (`carry_in`)
//! and steps it with `adv_step` (turn.rs): a short, explicit list of the
//! systems an adventure wants. Fort systems never run.
//!
//! Wave 0 puts every shared type here; the wave-1 packages fill in the
//! stub modules (one file each), so they never touch the shared files.
// Wave 0: most of these are filled in and read by the wave-1 packages.
#![allow(dead_code)]

pub mod bot;
pub mod company;
pub mod create;
pub mod fame;
pub mod items;
pub mod nemesis;
pub mod peoples;
pub mod quests;
pub mod sitegen;
pub mod sites;
pub mod talk;
pub mod travel;
pub mod turn;

use super::clock;
use super::{Game, Knobs, LogEntry, Weather, YEAR};
use crate::dwarf::Dwarf;
use crate::geom::Pos;
use crate::items::Item;
use crate::overworld::{Overworld, Race};
use crate::rng::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// The hero's id in every visit, so sagas (keyed by id) and the legends
/// follow them from site to site. A game's own ids count up from 1 and never
/// reach it. Companions take the ids after it.
pub const HERO_ID: u32 = 4_000_000_000;

/// One adventure: the world, the hero, the clock, and the site they're in.
#[derive(Serialize, Deserialize)]
pub struct Adventure {
    pub seed: u64,
    /// The world (empty while a visit holds it: see `ow()`).
    pub ow: Overworld,
    pub start_year: i32,
    /// World time in ticks: starts at dawn (`clock::DAWN_TICK`) on the first
    /// day of `start_year`; a visit's `g.tick` runs on from it.
    pub clock: u64,
    /// The world cell the hero is in.
    pub at: (i32, i32),
    pub hero: Hero,
    #[serde(default)]
    pub company: Vec<Hero>,
    /// The site the hero is in, its `Game` whole.
    #[serde(default)]
    pub visit: Option<Box<Visit>>,
    /// What the hero changed in each site (a fort's save is its own memory).
    #[serde(default)]
    pub memos: BTreeMap<SiteKey, SiteMemo>,
    #[serde(default)]
    pub fame: Fame,
    #[serde(default)]
    pub quests: Vec<Quest>,
    #[serde(default)]
    pub nemesis: Option<Nemesis>,
    #[serde(default)]
    pub weather: Weather,
    /// The travel log; a visit uses its `Game`'s.
    #[serde(default)]
    pub log: Vec<LogEntry>,
    #[serde(default)]
    pub knobs: Knobs,
    /// Days, cells, slain, gold: for the end page and the batch.
    #[serde(default)]
    pub stats: AdvStats,
    #[serde(default)]
    pub over: Option<AdvEnd>,
    /// The adventure's own dice: travel, encounters, weather.
    rng: Rng,
    /// Where the fallen forts' saves are (A5; the app's saves folder).
    #[serde(skip)]
    pub saves_dir: Option<std::path::PathBuf>,
}

/// The hero, or a companion, between sites.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hero {
    /// Body, skills, fight_xp, epithet, kills (id `HERO_ID` for the hero).
    pub dwarf: Dwarf,
    /// Carried, held and worn items; their ids mean nothing outside a visit.
    #[serde(default)]
    pub pack: Vec<Item>,
    #[serde(default)]
    pub coins: u32,
    /// Home site id.
    #[serde(default)]
    pub home: Option<u32>,
    #[serde(default)]
    pub calling: Calling,
    /// The hero's people (a `Dwarf` struct of any race: plan, "Peoples").
    #[serde(default = "dwarf_race")]
    pub race: Race,
    #[serde(default)]
    pub civ: Option<u32>,
    /// Sites and fort marks heard of (A7).
    #[serde(default)]
    pub known: BTreeSet<u32>,
    /// For companions (A9).
    #[serde(default)]
    pub loyalty: i32,
}

fn dwarf_race() -> Race {
    Race::Dwarf
}

impl Hero {
    /// A hero with nothing yet but themselves.
    pub fn new(mut dwarf: Dwarf, race: Race) -> Hero {
        dwarf.id = HERO_ID;
        Hero { dwarf, pack: Vec::new(), coins: 0, home: None, calling: Calling::default(), race, civ: None, known: BTreeSet::new(), loyalty: 0 }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Calling {
    #[default]
    Warrior,
    Hunter,
    Wanderer,
    Peasant,
}

/// Which site: a cell of the wild, a world site, or a fallen fort.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SiteKey {
    Wild(i32, i32),
    Site(u32),
    /// Index into `ow.forts`.
    Fort(u32),
}

/// A site the hero is in: its game, and the hero's place in its dwarves.
#[derive(Serialize, Deserialize)]
pub struct Visit {
    pub key: SiteKey,
    pub g: Game,
    /// Index of the hero in `g.dwarves`.
    pub hero: usize,
    /// What the site was made with, for its memory (A4: `sitegen::remember`).
    #[serde(default)]
    pub placed: Vec<(u32, String)>,
}

/// What screen the adventure shows (App-side, never saved).
#[derive(Clone, Debug, PartialEq)]
pub enum View {
    /// Making the hero (A1).
    Create,
    Local,
    Travel,
    Look(Pos),
    Talk { who: u32, sel: usize },
    Inventory { sel: usize },
    Status,
    Quests,
    Company,
    Saga,
    Dead,
    Menu { sel: usize },
}

/// What the hero changed in a regenerated site, applied after it is made
/// again (A4).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SiteMemo {
    /// Figures and creature kinds killed.
    #[serde(default)]
    pub slain: Vec<String>,
    /// Labels of items taken.
    #[serde(default)]
    pub taken: Vec<String>,
    #[serde(default)]
    pub cleared: bool,
    #[serde(default)]
    pub opened: bool,
    #[serde(default)]
    pub corpses: Vec<(Pos, String)>,
    #[serde(default)]
    pub visited_tick: u64,
}

/// Renown and infamy (A10).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Fame {
    #[serde(default)]
    pub renown: i32,
    #[serde(default)]
    pub infamy: i32,
    /// Standing with each civ.
    #[serde(default)]
    pub by_civ: BTreeMap<u32, i32>,
    #[serde(default)]
    pub deeds: Vec<Deed>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Deed {
    pub tick: u64,
    pub year: i32,
    pub text: String,
    pub weight: i32,
}

/// A task given by someone (A8).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Quest {
    pub id: u32,
    pub kind: QuestKind,
    /// Who gave it (a person's name).
    pub giver: String,
    pub at_site: Option<u32>,
    /// What it is about: a figure, an artifact's name, a site.
    pub target: String,
    pub reward: u32,
    pub state: QuestState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestKind {
    Slay,
    Recover,
    Clear,
    Avenge,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestState {
    #[default]
    Open,
    Done,
    Failed,
}

/// A sworn enemy on the hero's trail (A12).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Nemesis {
    #[serde(default)]
    pub figure: Option<u32>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub at: (i32, i32),
}

/// The run's numbers, for the end page and the batch.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AdvStats {
    #[serde(default)]
    pub days: u32,
    #[serde(default)]
    pub cells: u32,
    #[serde(default)]
    pub slain: u32,
    #[serde(default)]
    pub gold: u32,
}

/// How the adventure ended.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AdvEnd {
    Died { cause: String, tick: u64 },
    Retired { tick: u64 },
}

impl Adventure {
    /// An adventure starting at dawn at the hero's home site (or, with no
    /// home, the world's default site).
    pub fn new(ow: Overworld, seed: u64, mut hero: Hero, home: Option<u32>) -> Adventure {
        let at = match home.and_then(|h| ow.sites.iter().find(|s| s.id == h)) {
            Some(s) => (s.x, s.y),
            None if ow.w > 1 => ow.default_site(),
            None => (0, 0),
        };
        hero.home = home;
        if hero.civ.is_none() {
            hero.civ = home.and_then(|h| ow.sites.iter().find(|s| s.id == h)).and_then(|s| s.civ);
        }
        // After the last fort the world remembers fell (A5).
        let start_year = ow.forts.iter().filter_map(|f| f.fell).fold(ow.year, i32::max) + 1;
        Adventure {
            seed,
            ow,
            start_year,
            clock: clock::DAWN_TICK,
            at,
            hero,
            company: Vec::new(),
            visit: None,
            memos: BTreeMap::new(),
            fame: Fame::default(),
            quests: Vec::new(),
            nemesis: None,
            weather: Weather::Clear,
            log: Vec::new(),
            knobs: Knobs::default(),
            stats: AdvStats::default(),
            over: None,
            rng: Rng::new(seed.wrapping_mul(0x9E37_79B9).wrapping_add(17)),
            saves_dir: None,
        }
    }

    /// The world: the visit's while inside one (it is moved there).
    pub fn ow(&self) -> &Overworld {
        match &self.visit {
            Some(v) => &v.g.overworld,
            None => &self.ow,
        }
    }

    pub fn ow_mut(&mut self) -> &mut Overworld {
        match &mut self.visit {
            Some(v) => &mut v.g.overworld,
            None => &mut self.ow,
        }
    }

    /// World time now: the visit's clock while inside one.
    pub fn now(&self) -> u64 {
        self.visit.as_ref().map_or(self.clock, |v| v.g.tick)
    }

    /// The year now.
    pub fn year(&self) -> i32 {
        // (Inside a visit by its game's own dating: a fallen fort keeps its own.)
        match &self.visit {
            Some(v) => v.g.start_year + (v.g.tick / YEAR) as i32,
            None => self.start_year + (self.clock / YEAR) as i32,
        }
    }

    /// A tunable number: `default` unless a batch set it.
    pub fn knob(&self, name: &'static str, default: f64) -> f64 {
        self.knobs.set.get(name).copied().unwrap_or(default)
    }

    /// The hour of the day, 0-23.
    pub fn hour(&self) -> u32 {
        clock::hour(self.now())
    }

    pub fn rng(&mut self) -> &mut Rng {
        &mut self.rng
    }

    /// The hero as they are now: in the visit while inside one.
    pub fn hero_dwarf(&self) -> &Dwarf {
        match &self.visit {
            Some(v) => &v.g.dwarves[v.hero],
            None => &self.hero.dwarf,
        }
    }

    /// A line in the travel log (or the visit's, while inside one).
    pub fn say(&mut self, text: &str, color: (u8, u8, u8)) {
        match &mut self.visit {
            Some(v) => v.g.msg(text, color),
            None => {
                let tick = self.clock;
                self.log.push(LogEntry { text: text.to_string(), color, tick, at: None, kind: Default::default() });
            }
        }
    }
}

/// A hero of the world's first dwarf civ (else any people's), with an axe,
/// set out from their home site: the title's "Adventure" and the window's
/// `--adventure` until the character maker (A1) replaces it.
pub fn quick_start(ow: Overworld, seed: u64) -> Adventure {
    let home = ow
        .sites
        .iter()
        .filter(|s| !s.ruined && s.civ.is_some())
        .min_by_key(|s| (ow.civs.get(s.civ.unwrap_or(0) as usize).map_or(true, |c| c.race != Race::Dwarf), s.id))
        .map(|s| (s.id, s.civ));
    let race = home.and_then(|(_, c)| c).and_then(|c| ow.civs.get(c as usize)).map_or(Race::Dwarf, |c| c.race);
    let mut rng = Rng::new(seed ^ 0xADE7);
    let mut d = crate::dwarf::new_dwarf(HERO_ID, Pos::new(0, 0, 0), &mut rng, &[]);
    d.name = crate::overworld::person_name(&mut rng, race);
    d.fight_xp = 300;
    let mut hero = Hero::new(d, race);
    let axe = crate::items::Item {
        id: 1,
        kind: crate::items::ItemKind::Axe,
        mat: crate::material::Mat::Copper,
        pos: Pos::new(0, 0, 0),
        carried_by: Some(HERO_ID),
        claimed_by: None,
        label: None,
        container: None,
        quality: 0,
        to_depot: false,
        dump: false,
        wear: 0,
        forbidden: false,
        kills: 0,
        maker: None,
        decor: None,
    };
    hero.pack.push(axe);
    hero.dwarf.held = Some(1);
    hero.civ = home.and_then(|(_, c)| c);
    let mut a = Adventure::new(ow, seed, hero, home.map(|(id, _)| id));
    let key = match home {
        Some((id, _)) => SiteKey::Site(id),
        None => SiteKey::Wild(a.at.0, a.at.1),
    };
    if a.enter(key).is_ok() {
        let (name, place) = (a.hero_dwarf().name.clone(), a.visit.as_ref().map_or(String::new(), |v| v.g.fortress_name.clone()));
        a.say(&format!("{} sets out from {} at dawn.", name, place), (255, 215, 90));
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A hero for tests: a dwarf with an iron axe in the pack.
    pub(crate) fn test_hero(seed: u64) -> Hero {
        let mut rng = Rng::new(seed);
        let d = crate::dwarf::new_dwarf(1, Pos::new(0, 0, 0), &mut rng, &[]);
        Hero::new(d, Race::Dwarf)
    }

    #[test]
    fn an_adventure_survives_a_save() {
        let ow = Game::new(5).overworld;
        let home = ow.sites.iter().find(|s| !s.ruined && s.civ.is_some()).map(|s| s.id);
        let mut a = Adventure::new(ow, 5, test_hero(5), home);
        a.say("Set out at dawn.", (200, 200, 200));
        a.fame.deeds.push(Deed { tick: a.clock, year: a.year(), text: "Set out".into(), weight: 1 });
        a.enter(SiteKey::Wild(a.at.0, a.at.1)).unwrap();
        let dir = std::env::temp_dir().join(format!("dwarfclone-adv-save-{}", std::process::id()));
        let meta = crate::saves::write_adventure(&dir, &a, false, &|_, _| {}).unwrap();
        assert_eq!(meta.kind, crate::saves::SaveKind::Adventure);
        assert!(crate::saves::list(&dir).iter().any(|m| m.file == meta.file && m.kind == crate::saves::SaveKind::Adventure));
        let b = crate::saves::read_adventure(&dir, &meta.file, &|_, _| {}).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!((b.seed, b.at, b.clock, b.start_year), (a.seed, a.at, a.clock, a.start_year));
        assert_eq!(b.hero.dwarf.name, a.hero.dwarf.name);
        assert_eq!(b.ow().name, a.ow().name);
        assert_eq!(b.ow().events.len(), a.ow().events.len());
        assert_eq!(b.fame.deeds.len(), 1);
        let (va, vb) = (a.visit.as_ref().unwrap(), b.visit.as_ref().unwrap());
        assert_eq!((vb.key, vb.hero, vb.g.tick), (va.key, va.hero, va.g.tick));
        assert_eq!(vb.g.dwarves[vb.hero].id, HERO_ID);
        assert!(b.ow.w <= 1, "the world travels inside the visit, not twice");
        // And out of the visit again, the world comes back.
        let mut b = b;
        b.leave();
        assert_eq!(b.ow.name, a.ow().name);
    }
}
