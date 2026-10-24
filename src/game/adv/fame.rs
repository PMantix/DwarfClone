//! Fame, the hero's saga, death and the world's memory (A10). The fort's
//! fall leaves its mark on the world (`mark_fallen`, called once from
//! `sagas_fall`); a fort embarked where an adventurer fell finds them
//! (`place_remains`, A14).
//!
//! The adventure's side: `notice`, run after every act, counts what the
//! hero did since it last looked (a beast of legend slain, a name won,
//! townsfolk murdered, a treasure taken and a treasure brought home, a
//! ruin dared, a wound that nearly killed them) and makes each a `deed`:
//! renown or infamy, a chapter of the hero's Wander saga, and for the
//! great ones a line in the legends. Death (`hero_died`) and retirement
//! (`retire`) close the saga and write the hero into the world: a figure,
//! a legend, the saga, and their `Remains` where they fell, for a later
//! adventurer to find (`lay_remains`).

use super::super::*;
use super::{AdvEnd, Adventure, Deed, Hero, SiteKey, HERO_ID};
use crate::game::sagas::{title_for, Beat, SagaKind, WorldSaga};
use crate::items::ARTIFACT;
use crate::overworld::{Figure, FortMark, Race, Remains, SiteKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A deed this heavy is told in the legends.
const LEGEND_WEIGHT: i32 = 3;
/// Murdering one of the folk.
const MURDER_INFAMY: i32 = 3;
/// What a murder costs the hero with the town's people.
const MURDER_STANDING: i32 = 10;
/// The wound that nearly killed them: blood below this share.
const NEAR_DEATH: f64 = 0.4;

/// How the adventure ended, as the world keeps it: who, how, where, and
/// what was written (A19 records it in the shared world file).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ending {
    /// "Urist the Dragonslayer".
    pub called: String,
    /// The hero as they ended (body, kills, epithet, looks).
    pub dwarf: Dwarf,
    #[serde(default = "dwarf_race")]
    pub race: Race,
    /// Their figure in `ow.figures`.
    pub figure: u32,
    /// False: retired.
    pub died: bool,
    pub year: i32,
    /// "was slain by a goblin in the ruin of Copper Gate", "retired to
    /// Copper Gate".
    pub cause: String,
    /// Where: the site's name, or the road.
    pub place: String,
    /// The home they set out from.
    #[serde(default)]
    pub home: String,
    /// The line written in the legends.
    pub legend: String,
    /// The hero's saga as the world keeps it (None: too little happened).
    #[serde(default)]
    pub saga: Option<WorldSaga>,
    /// Where their bones lie (the dead only).
    #[serde(default)]
    pub remains: Option<Remains>,
    /// A line to remember them by.
    #[serde(default)]
    pub eulogy: String,
    /// Their life, in paragraphs.
    #[serde(default)]
    pub story: Vec<String>,
}

fn dwarf_race() -> Race {
    Race::Dwarf
}

/// What `notice` has counted already, so each deed is told once.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Seen {
    /// Set once the first look has been taken (nothing before it is a deed).
    #[serde(default)]
    pub primed: bool,
    /// Where the hero was at the last look.
    #[serde(default)]
    pub site: Option<SiteKey>,
    #[serde(default)]
    pub kills: u32,
    #[serde(default)]
    pub epithet: Option<String>,
    /// Figures of the legends dead at the last look.
    #[serde(default)]
    pub dead: BTreeSet<u32>,
    /// The folk of the town the hero is in, alive at the last look.
    #[serde(default)]
    pub folk: Vec<(u32, String)>,
    /// Treasures taken (labels), and where from.
    #[serde(default)]
    pub taken: Vec<(String, String)>,
    /// Treasures brought home.
    #[serde(default)]
    pub brought: Vec<String>,
    /// The wound that nearly killed them has been told.
    #[serde(default)]
    pub turned: bool,
    /// The saga's weight (how much it has been through).
    #[serde(default)]
    pub weight: u32,
    /// A saga card waiting to be shown (a turn).
    #[serde(default)]
    pub card: bool,
}

impl Game {
    /// The fort has fallen: the world remembers where, when, to what, and
    /// what it held. Draws no dice (it runs inside the fort's last tick).
    pub(crate) fn mark_fallen(&mut self, killer: Option<String>) {
        let artifacts: Vec<String> = self.items.values().filter(|it| it.quality >= ARTIFACT).map(|it| it.label.clone().unwrap_or_else(|| it.name())).collect();
        let mark = FortMark {
            name: self.fortress_name.clone(),
            site: self.site,
            civ: self.civ,
            founded: self.start_year,
            fell: Some(self.start_year + (self.tick / YEAR) as i32),
            killer,
            artifacts,
            save_file: crate::saves::fort_file(&self.fortress_name),
            peak_pop: self.dwarves.len(),
            journal: self.journal_last_page(),
            occupant: None,
            robbed: Vec::new(),
        };
        let ow = &mut self.overworld;
        match ow.forts.iter().position(|f| f.save_file == mark.save_file && f.site == mark.site) {
            Some(i) => ow.forts[i] = mark,
            None => ow.forts.push(mark),
        }
    }

    /// A fort embarked where an adventurer fell finds their bones and gear
    /// (A14 fills it; called from `embark_prepared`).
    pub(crate) fn place_remains(&mut self) {}

    /// The bones and gear of the adventurers who fell in this place, where
    /// they fell (once: a place that kept them, like a fallen fort's save,
    /// is not given them twice). How many were laid.
    pub(crate) fn lay_remains(&mut self, key: SiteKey, cell: (i32, i32)) -> usize {
        let here: Vec<Remains> = self
            .overworld
            .remains
            .iter()
            .filter(|r| (r.x, r.y) == cell)
            .filter(|r| match key {
                SiteKey::Site(id) => r.site == Some(id),
                SiteKey::Wild(..) | SiteKey::Fort(_) => r.site.is_none(),
            })
            .cloned()
            .collect();
        let mut n = 0;
        for r in here {
            let bones = bones_label(&r.name);
            if self.items.values().any(|it| it.label.as_deref() == Some(bones.as_str())) {
                continue;
            }
            let at = Pos::new(r.pos.0, r.pos.1, r.pos.2);
            // (A wild place is made again from the adventure's own seed,
            // so the very spot may be rock now: the nearest open one.)
            let at = if self.world.in_bounds(at) && self.world.walkable(at) {
                at
            } else if self.world.in_bounds(at) {
                self.spot_near(at, 4)
            } else {
                self.spot_near(self.embark, 4)
            };
            let id = self.spawn_item(ItemKind::Bone, Mat::Bone, at);
            if let Some(it) = self.items.get_mut(&id) {
                it.label = Some(bones);
            }
            for (kind, mat, label) in &r.gear {
                let id = self.spawn_item(*kind, *mat, at);
                if let Some(it) = self.items.get_mut(&id) {
                    it.label = Some(label.clone());
                    it.wear = it.wear.max(1);
                }
            }
            n += 1;
        }
        n
    }
}

/// What a dead hero's bones are called where they lie.
pub fn bones_label(called: &str) -> String {
    format!("the bones of {}", called)
}

fn first(name: &str) -> String {
    name.split(' ').next().unwrap_or(name).to_string()
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

/// "a goblin", "an elf".
fn a_an(w: &str) -> String {
    let an = w.starts_with(['a', 'e', 'i', 'o', 'u']);
    format!("{} {}", if an { "an" } else { "a" }, w)
}

/// What the item is called: its label, else its name.
fn item_called(it: &crate::items::Item) -> String {
    it.label.clone().unwrap_or_else(|| it.name())
}

impl Adventure {
    /// The hero's home site's name.
    pub fn home_name(&self) -> String {
        let ow = self.ow();
        self.hero.home.and_then(|h| ow.sites.iter().find(|s| s.id == h)).map_or_else(|| "nowhere".to_string(), |s| s.name.clone())
    }

    /// The date in words, for a saga chapter: "Spring 214".
    fn when(&self) -> String {
        format!("{} {}", SEASONS[((self.now() % YEAR) / SEASON) as usize % 4], self.year())
    }

    /// Where the hero is now, in words.
    fn place(&self) -> String {
        match self.visit.as_ref().map(|v| v.key) {
            Some(SiteKey::Fort(i)) => format!("the ruin of {}", self.ow().forts.get(i as usize).map_or("a fallen fort".to_string(), |f| f.name.clone())),
            Some(key) => super::sites::site_name(self.ow(), key),
            None => super::sites::site_name(self.ow(), SiteKey::Wild(self.at.0, self.at.1)),
        }
    }

    /// The hero's saga, opened with their setting out if it hasn't been.
    fn saga_mut(&mut self) -> &mut WorldSaga {
        if self.fame.saga.is_none() {
            let d = self.hero_dwarf();
            let (name, called) = (d.name.clone(), d.called());
            let home = self.home_name();
            let title = title_for(SagaKind::Wander, &home, self.seed, &first(&name));
            let set_out = format!("{} set out from {} in the spring of {}.", first(&name), home, self.start_year);
            self.fame.saga = Some(WorldSaga {
                title,
                kind: SagaKind::Wander,
                fort: format!("{} of {}", called, home),
                year: self.start_year,
                chapters: vec![(format!("Spring {}", self.start_year), Beat::Cause, set_out)],
                closing: String::new(),
                doom: false,
                fell: false,
            });
        }
        self.fame.saga.as_mut().unwrap()
    }

    /// A chapter of the hero's saga. A long saga keeps its first chapters
    /// and its latest, as the fort's do.
    fn chapter(&mut self, beat: Beat, text: String, weight: u32) {
        let when = self.when();
        let s = self.saga_mut();
        if s.chapters.len() >= 9 {
            s.chapters.remove(2);
        }
        s.chapters.push((when, beat, text));
        self.fame.seen.weight += weight;
        if beat == Beat::Turn && saga_told(self.fame.saga.as_ref().unwrap(), self.fame.seen.weight, false) {
            self.fame.seen.card = true;
        }
    }

    /// A line in the world's legends, this year.
    fn legend(&mut self, text: String) {
        let year = self.year();
        let ow = self.ow_mut();
        ow.year = ow.year.max(year);
        ow.log(year, text);
    }

    /// A deed: `text` is what the hero did ("slew Emberwing the dragon").
    /// It raises their renown by `weight`, is a chapter of their saga, and
    /// at weight 3 a line in the legends.
    pub fn deed(&mut self, text: &str, weight: i32) {
        self.record(text, weight, Beat::Escalation, weight >= LEGEND_WEIGHT);
    }

    /// A deed the world holds against the hero: infamy, and standing lost
    /// with `civ`'s people.
    pub fn misdeed(&mut self, text: &str, weight: i32, civ: Option<u32>) {
        self.fame.infamy += weight;
        if let Some(c) = civ {
            *self.fame.by_civ.entry(c).or_default() -= MURDER_STANDING;
        }
        self.record(text, -weight, Beat::Escalation, weight >= LEGEND_WEIGHT);
    }

    fn record(&mut self, text: &str, weight: i32, beat: Beat, legend: bool) {
        let d = self.hero_dwarf();
        let (name, called) = (d.name.clone(), d.called());
        if weight > 0 {
            self.fame.renown += weight;
            if let Some(c) = self.hero.civ {
                *self.fame.by_civ.entry(c).or_default() += weight;
            }
        }
        let (tick, year) = (self.now(), self.year());
        self.fame.deeds.push(Deed { tick, year, text: text.to_string(), weight });
        self.chapter(beat, format!("{} {}.", first(&name), text), weight.unsigned_abs());
        if legend {
            let home = self.home_name();
            self.legend(format!("{} of {} {}.", called, home, text));
        }
    }

    /// What the hero has done since the last look: run after every act,
    /// every step of the road and every arrival. Deeds are told once;
    /// the hero's death in a site ends the adventure.
    pub fn notice(&mut self) {
        if self.over.is_some() {
            return;
        }
        if let Some(v) = &self.visit {
            if !v.g.dwarves[v.hero].alive {
                let cause = self.death_in_site();
                self.over = Some(AdvEnd::Died { cause: cause.clone(), tick: self.now() });
                self.hero_died(&cause);
                return;
            }
        }
        let key = self.visit.as_ref().map(|v| v.key);
        let primed = self.fame.seen.primed;
        if !primed {
            let (kills, epithet) = (self.hero_dwarf().kills, self.hero_dwarf().epithet.clone());
            self.fame.seen.kills = kills;
            self.fame.seen.epithet = epithet;
            self.fame.seen.dead = self.ow().figures.iter().filter(|f| !f.alive()).map(|f| f.id).collect();
            self.fame.seen.taken = self.carried_treasures().into_iter().map(|t| (t, "home".to_string())).collect();
            self.fame.seen.primed = true;
        }
        // A new place: arrivals and leavings, and the dead who lie here.
        if key != self.fame.seen.site || !primed {
            let was = std::mem::replace(&mut self.fame.seen.site, key);
            self.lay_here();
            self.fame.seen.folk = self.folk_here();
            if primed {
                self.arrived(was, key);
            }
        }
        self.notice_kills();
        self.notice_treasure(key);
        self.notice_wounds();
        self.stats.slain = self.hero_dwarf().kills;
        // Companions (A9) and quests (A8): after the deeds they count on.
        self.company_check();
        self.check_quests();
    }

    /// Lay the remains of the fallen in the site just entered.
    fn lay_here(&mut self) {
        let Some(v) = self.visit.as_mut() else { return };
        let cell = match v.key {
            SiteKey::Wild(x, y) => (x, y),
            SiteKey::Site(id) => v.g.overworld.sites.iter().find(|s| s.id == id).map_or(self.at, |s| (s.x, s.y)),
            SiteKey::Fort(i) => v.g.overworld.forts.get(i as usize).map_or(self.at, |f| f.site),
        };
        let key = v.key;
        let n = v.g.lay_remains(key, cell);
        if n > 0 {
            let r = v.g.overworld.remains.iter().find(|r| (r.x, r.y) == cell).map(|r| r.name.clone()).unwrap_or_default();
            v.g.msg(&format!("Old bones lie here, and gear left where it fell. They say {} died in this place.", r), (200, 190, 160));
        }
    }

    /// The folk of the town the hero is in, alive.
    fn folk_here(&self) -> Vec<(u32, String)> {
        let Some(v) = &self.visit else { return Vec::new() };
        let mut f: Vec<(u32, String)> = v.g.creatures.values().filter(|c| c.alive && c.visitor == super::sitegen::town::FOLK).map(|c| (c.id, c.title())).collect();
        f.sort();
        f
    }

    /// Into a new place, or out of one.
    fn arrived(&mut self, was: Option<SiteKey>, now: Option<SiteKey>) {
        if let Some(SiteKey::Fort(i)) = was {
            if now != was {
                let name = self.ow().forts.get(i as usize).map_or("a fallen fort".to_string(), |f| f.name.clone());
                self.deed(&format!("came out of the ruin of {} alive", name), 1);
            }
        }
        match now {
            Some(SiteKey::Fort(i)) => {
                let Some(m) = self.ow().forts.get(i as usize).cloned() else { return };
                let fell = match (m.fell, &m.killer) {
                    (Some(y), Some(k)) => format!(", which fell to the {} in {}", k, y),
                    (Some(y), None) => format!(", which fell in {}", y),
                    _ => String::new(),
                };
                self.deed(&format!("went down into the ruin of {}{}", m.name, fell), 1);
            }
            Some(SiteKey::Site(id)) => {
                let Some(site) = self.ow().sites.iter().find(|s| s.id == id).cloned() else { return };
                if site.kind == SiteKind::Lair {
                    self.deed(&format!("went into {}, a beast's lair", site.name), 1);
                    return;
                }
                // Home, or among friends, with treasure from afar.
                let stance = super::peoples::stance(self.ow(), self.hero.race, self.hero.civ, &site);
                if site.ruined || !matches!(stance, super::peoples::Stance::Home | super::peoples::Stance::Friendly) {
                    return;
                }
                let home: Vec<(String, String)> = self.fame.seen.taken.iter().filter(|(t, from)| from != "home" && !self.fame.seen.brought.contains(t)).cloned().collect();
                let carried = self.carried_treasures();
                let home: Vec<(String, String)> = home.into_iter().filter(|(t, _)| carried.contains(t)).collect();
                if home.is_empty() {
                    return;
                }
                let what: Vec<String> = home.iter().map(|(t, from)| format!("{} out of {}", t, from)).collect();
                for (t, _) in &home {
                    self.fame.seen.brought.push(t.clone());
                }
                self.deed(&format!("came home to {} bearing {}", site.name, what.join(" and ")), LEGEND_WEIGHT);
            }
            _ => {}
        }
    }

    /// Figures of legend slain, a name won, folk murdered.
    fn notice_kills(&mut self) {
        let d = self.hero_dwarf();
        let (kills, epithet, deed) = (d.kills, d.epithet.clone(), d.deed.clone());
        let gained = kills.saturating_sub(self.fame.seen.kills);
        self.fame.seen.kills = kills;
        let named = epithet != self.fame.seen.epithet;
        let before = std::mem::replace(&mut self.fame.seen.epithet, epithet.clone());
        if gained == 0 && !named {
            return;
        }
        // The legends' figures who died as the hero struck.
        let fresh: Vec<Figure> = self.ow().figures.iter().filter(|f| !f.alive() && !self.fame.seen.dead.contains(&f.id)).cloned().collect();
        for f in &fresh {
            self.fame.seen.dead.insert(f.id);
        }
        let mut left = gained as usize;
        if named {
            // A new name says the deed itself ("slew Emberwing the dragon").
            let was = before.map_or(String::new(), |e| format!(" no longer {} but", e));
            let text = format!("{}, and was hailed{} as {}", deed.unwrap_or_else(|| "did great deeds".into()), was, self.hero_dwarf().called());
            self.deed(&text, LEGEND_WEIGHT);
            left = left.saturating_sub(1);
        }
        for f in fresh.iter().skip(if named { 1 } else { 0 }) {
            if left == 0 {
                break;
            }
            left -= 1;
            let title = f.title.as_ref().map_or(String::new(), |t| format!(" {}", t));
            self.deed(&format!("slew {}{} the {}, of the old legends", f.name, title, f.kind), LEGEND_WEIGHT);
        }
        // Townsfolk struck down: murder.
        let now = self.folk_here();
        let gone: Vec<(u32, String)> = self.fame.seen.folk.iter().filter(|(id, _)| !now.iter().any(|(n, _)| n == id)).cloned().collect();
        self.fame.seen.folk = now;
        if left == 0 || gone.is_empty() {
            return;
        }
        let (town, civ) = match self.visit.as_ref().map(|v| v.key) {
            Some(SiteKey::Site(id)) => {
                let s = self.ow().sites.iter().find(|s| s.id == id);
                (s.map_or("the town".to_string(), |s| s.name.clone()), s.and_then(|s| s.civ))
            }
            _ => ("the town".to_string(), None),
        };
        for (_, who) in gone.into_iter().take(left) {
            self.misdeed(&format!("murdered {} in the streets of {}", who, town), MURDER_INFAMY, civ);
        }
    }

    /// The treasures (works of legend) the hero carries, by name.
    pub fn carried_treasures(&self) -> Vec<String> {
        match &self.visit {
            Some(v) => v.g.items.values().filter(|it| it.carried_by == Some(HERO_ID) && it.quality >= ARTIFACT).map(item_called).collect(),
            None => self.hero.pack.iter().filter(|it| it.quality >= ARTIFACT).map(item_called).collect(),
        }
    }

    /// A treasure taken up; the gear of a fallen adventurer found.
    fn notice_treasure(&mut self, key: Option<SiteKey>) {
        let place = self.place();
        for t in self.carried_treasures() {
            if self.fame.seen.taken.iter().any(|(x, _)| x == &t) {
                continue;
            }
            self.fame.seen.taken.push((t.clone(), place.clone()));
            self.deed(&format!("took up {} in {}", t, place), 2);
        }
        // Gear of the fallen, taken from their bones.
        let Some(v) = &self.visit else { return };
        let carried: Vec<String> = v.g.items.values().filter(|it| it.carried_by == Some(HERO_ID)).filter_map(|it| it.label.clone()).collect();
        let cell = self.at;
        let site = match key {
            Some(SiteKey::Site(id)) => Some(id),
            _ => None,
        };
        let mut found: Vec<(String, String)> = Vec::new();
        for r in self.ow_mut().remains.iter_mut().filter(|r| (r.x, r.y) == cell && r.site == site) {
            let before = r.gear.len();
            r.gear.retain(|(_, _, l)| !carried.contains(l));
            if r.gear.len() < before {
                found.push((r.name.clone(), cell_word(before - r.gear.len())));
            }
        }
        for (who, n) in found {
            self.deed(&format!("found the bones of {} and took up {} of their gear", who, n), 1);
        }
    }

    /// The wound that nearly killed them: the turn of their saga.
    fn notice_wounds(&mut self) {
        if self.fame.seen.turned {
            return;
        }
        let d = self.hero_dwarf();
        let b = &d.body;
        if !(d.alive && (b.unconscious() || (b.blood as f64) < b.max_blood as f64 * NEAR_DEATH)) {
            return;
        }
        let by = self.visit.as_ref().and_then(|v| v.g.annals.wounded_by.iter().rev().find(|w| w.0 == HERO_ID).map(|w| w.1.clone()));
        let name = first(&d.name);
        let place = self.place();
        let text = match by {
            Some(k) => format!("{} lay near death in {}, cut down by {}.", name, place, a_an(&k)),
            None => format!("{} lay near death in {}.", name, place),
        };
        self.fame.seen.turned = true;
        self.chapter(Beat::Turn, text, 1);
    }

    /// The saga card waiting to be shown, as a page.
    pub fn take_card(&mut self) -> Option<super::sitegen::fallen::Writing> {
        if !std::mem::take(&mut self.fame.seen.card) {
            return None;
        }
        self.fame.saga.as_ref().map(|s| writing(s, false))
    }

    /// How the hero died, inside a site: what the site's annals say.
    fn death_in_site(&self) -> String {
        let place = self.place();
        let Some(v) = &self.visit else { return format!("died in {}", place) };
        let name = &v.g.dwarves[v.hero].name;
        let note = v.g.annals.deaths.iter().rev().find(|n| &n.name == name);
        let how = match note.map(|n| (n.cause.as_str(), n.killer.as_ref())) {
            Some(("struck down", Some(k))) => format!("was slain by {}", a_an(k)),
            Some(("bled to death", Some(k))) => format!("bled to death from {}'s wounds", a_an(k)),
            Some(("struck down", None)) => "was struck down".to_string(),
            Some((c, _)) if c.contains(' ') => c.to_string(),
            Some((c, _)) => format!("died of {}", c),
            None => "died".to_string(),
        };
        format!("{} in {}", how, place)
    }

    /// The hero is dead (`cause`: "was slain by a goblin in the ruin of
    /// Copper Gate"). The world remembers: their figure, a legend, their
    /// saga ended in doom, and their bones and gear where they fell.
    pub fn hero_died(&mut self, cause: &str) {
        if self.fame.end.is_some() {
            return;
        }
        if self.over.is_none() {
            self.over = Some(AdvEnd::Died { cause: cause.to_string(), tick: self.now() });
        }
        let year = self.year();
        let mut d = self.hero_dwarf().clone();
        d.alive = false;
        let called = d.called();
        let home = self.home_name();
        let place = self.place();
        let figure = self.write_figure(&d, Some(year), cause);
        // Their gear, where they lie.
        let (pos, gear) = match &self.visit {
            Some(v) => {
                let p = v.g.dwarves[v.hero].pos;
                let gear = v.g.items.values().filter(|it| it.carried_by == Some(HERO_ID) || (it.pos == p && it.carried_by.is_none() && !matches!(it.kind, ItemKind::Corpse | ItemKind::Carcass))).map(|it| (it.kind, it.mat, item_called(it))).collect();
                ((p.x, p.y, p.z), gear)
            }
            None => ((0, 0, 0), self.hero.pack.iter().map(|it| (it.kind, it.mat, item_called(it))).collect()),
        };
        let (cell, site) = match self.visit.as_ref().map(|v| v.key) {
            Some(SiteKey::Site(id)) => (self.ow().sites.iter().find(|s| s.id == id).map_or(self.at, |s| (s.x, s.y)), Some(id)),
            Some(SiteKey::Fort(i)) => (self.ow().forts.get(i as usize).map_or(self.at, |f| f.site), None),
            _ => (self.at, None),
        };
        let remains = Remains { name: called.clone(), x: cell.0, y: cell.1, site, pos, year, gear, figure };
        self.ow_mut().remains.push(remains.clone());
        // Their life, told by the place they died in (or by no place).
        let (eulogy, story) = {
            let none;
            let g = match &self.visit {
                Some(v) => &v.g,
                None => {
                    none = Game::placeholder(crate::overworld::Overworld::empty());
                    &none
                }
            };
            // The eulogy's own cause is the fort's short word: ours instead,
            // and their last words after it.
            let base = g.eulogy(&d);
            let base = base.split(';').next().unwrap_or(&base).trim_end_matches('.').to_string();
            let words = g.annals.deaths.iter().rev().find(|n| n.name == d.name).and_then(|n| n.last_words.clone());
            let said = words.map_or(String::new(), |w| format!(", saying \u{201c}{}\u{201d}", w));
            (format!("{}; {}{}.", base, cause, said), g.life_story(&d))
        };
        let saga = self.end_saga(true, format!("{} {}.", first(&d.name), cause), eulogy.clone());
        let legend = format!("{} of {} {}, in {}.", called, home, cause, year);
        self.legend(legend.clone());
        // Oaths unkept (A8), and companions who carry the news home (A9).
        self.oaths_at_death();
        self.company_at_death();
        let race = self.hero.race;
        self.fame.end = Some(Ending { called, dwarf: d, race, figure, died: true, year, cause: cause.to_string(), place, home, legend, saga, remains: Some(remains), eulogy, story });
    }

    /// The hero's figure in the legends (alive if `died` is None).
    fn write_figure(&mut self, d: &Dwarf, died: Option<i32>, cause: &str) -> u32 {
        let year = self.year();
        let kind = format!("{} adventurer", format!("{:?}", self.hero.race).to_lowercase());
        let born = year - d.age.unwrap_or(30) as i32;
        let civ = self.hero.civ;
        self.ow_mut().push_figure(Figure { id: 0, name: d.name.clone(), kind, civ, born, died, cause: died.map(|_| cause.to_string()), title: d.epithet.clone(), kills: d.kills, lair: None })
    }

    /// The saga's last chapter and closing line; kept by the world if it
    /// was worth telling.
    fn end_saga(&mut self, doom: bool, text: String, closing: String) -> Option<WorldSaga> {
        let when = self.when();
        let called = self.hero_dwarf().called();
        let home = self.home_name();
        let weight = self.fame.seen.weight;
        let s = self.saga_mut();
        s.chapters.push((when, Beat::Ending, text));
        s.closing = closing;
        s.doom = doom;
        s.fort = format!("{} of {}", called, home);
        let s = s.clone();
        if !saga_told(&s, weight, true) {
            return None;
        }
        self.ow_mut().sagas.push(s.clone());
        Some(s)
    }

    /// Whether the hero may settle down here: in a town of friends, with
    /// no foe about.
    pub fn can_retire(&self) -> Result<(), String> {
        let Some(v) = &self.visit else { return Err("You can only settle down in a town.".into()) };
        let SiteKey::Site(id) = v.key else { return Err("You can only settle down in a town.".into()) };
        let Some(site) = self.ow().sites.iter().find(|s| s.id == id) else { return Err("You can only settle down in a town.".into()) };
        if site.ruined || site.kind == SiteKind::Lair || site.civ.is_none() {
            return Err("No one lives here to settle among.".into());
        }
        if !matches!(super::peoples::stance(self.ow(), self.hero.race, self.hero.civ, site), super::peoples::Stance::Home | super::peoples::Stance::Friendly) {
            return Err(format!("The folk of {} would not have you.", site.name));
        }
        if let Some(why) = v.g.alarm(v.hero, v.g.tick) {
            return Err(format!("Not now: {}", why));
        }
        Ok(())
    }

    /// The hero settles down in a friendly town: the saga ends in gold,
    /// and they live on in the legends.
    pub fn retire(&mut self) {
        if self.over.is_some() || self.can_retire().is_err() {
            return;
        }
        let year = self.year();
        let d = self.hero_dwarf().clone();
        let called = d.called();
        let home = self.home_name();
        let place = self.place();
        let cause = format!("settled down in {}", place);
        let figure = self.write_figure(&d, None, &cause);
        let deeds = self.fame.deeds.iter().filter(|x| x.weight > 0).count();
        let closing = match deeds {
            0 => format!("{} hung up their pack, and the road went on without them.", first(&d.name)),
            _ => format!("{} hung up their pack, and the songs of the road are sung of them still.", first(&d.name)),
        };
        let saga = self.end_saga(false, format!("{} {}.", first(&d.name), cause), closing.clone());
        let legend = format!("{} of {} {}, in {}, to wander no more.", called, home, cause, year);
        self.legend(legend.clone());
        self.over = Some(AdvEnd::Retired { tick: self.now() });
        let race = self.hero.race;
        let story = self.visit.as_ref().map_or(Vec::new(), |v| v.g.life_story(&d));
        self.fame.end = Some(Ending { called, dwarf: d, race, figure, died: false, year, cause, place, home, legend, saga, remains: None, eulogy: closing, story });
    }

    /// The world, taken out of the adventure (the death page's `n` and
    /// `a`): its newest year is the adventure's.
    pub fn take_world(&mut self) -> crate::overworld::Overworld {
        let year = self.year();
        let mut ow = std::mem::replace(self.ow_mut(), crate::overworld::Overworld::empty());
        ow.year = ow.year.max(year);
        ow
    }
}

/// "one piece", "three pieces".
fn cell_word(n: usize) -> String {
    match n {
        1 => "a piece".into(),
        2 => "two pieces".into(),
        3 => "three pieces".into(),
        n => format!("{} pieces", n),
    }
}

/// Worth telling (as `Saga::told` for a Wander saga): two chapters that
/// mattered, or a death after a deed.
pub fn saga_told(s: &WorldSaga, weight: u32, ended: bool) -> bool {
    let n = s.chapters.len();
    (n >= 2 && weight >= 2) || (s.doom && ended && s.chapters.iter().any(|c| c.1 == Beat::Escalation))
}

/// A hero's saga as a page to read.
pub fn writing(s: &WorldSaga, ended: bool) -> super::sitegen::fallen::Writing {
    super::sitegen::fallen::Writing {
        title: s.title.clone(),
        sub: format!("a saga of {} · {}, {}", s.kind.name(), s.fort, s.year),
        doom: s.doom,
        chapters: s.chapters.clone(),
        closing: if ended { s.closing.clone() } else { "The road goes on...".to_string() },
        fell: false,
    }
}

/// The hero as they ended, for the end page's portrait.
pub fn end_hero(e: &Ending) -> Hero {
    Hero::new(e.dwarf.clone(), e.race)
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::turn::{Action, Outcome};
    use crate::creature::Species;
    use crate::overworld::Overworld;

    fn world() -> Overworld {
        static WORLD: std::sync::OnceLock<Overworld> = std::sync::OnceLock::new();
        WORLD.get_or_init(|| Overworld::generate(5)).clone()
    }

    /// A hero set out from home, in the wild land of their home's cell,
    /// with nothing about.
    fn in_the_wild(seed: u64) -> Adventure {
        let mut a = super::super::quick_start(world(), seed);
        let (x, y) = a.at;
        a.enter(SiteKey::Wild(x, y)).unwrap();
        a.visit.as_mut().unwrap().g.creatures.clear();
        a.notice();
        a
    }

    /// The hero strikes the last blow on a beast of the legends (as
    /// combat does: the kill credited, the figure dead in the legends).
    fn slay_figure(a: &mut Adventure, f: u32) {
        let year = a.year();
        let v = a.visit.as_mut().unwrap();
        let h = v.hero;
        let p = v.g.dwarves[h].pos;
        let id = v.g.spawn_creature(Species::Megabeast, p, DAY);
        let mut c = v.g.creatures.remove(&id).unwrap();
        let fig = &v.g.overworld.figures[f as usize];
        c.label = Some(format!("{} the {}", fig.name, fig.kind));
        c.figure = Some(f);
        let mut d = std::mem::take(&mut v.g.dwarves[h]);
        v.g.credit_kill(&mut d, &c);
        v.g.dwarves[h] = d;
        let fig = &mut v.g.overworld.figures[f as usize];
        fig.died = Some(year);
        fig.cause = Some("slain".into());
    }

    #[test]
    fn slaying_a_figure_is_a_deed_and_a_legend() {
        let mut a = in_the_wild(5);
        assert!(a.fame.deeds.is_empty(), "setting out is no deed");
        let beasts: Vec<u32> = a.ow().figures.iter().filter(|f| f.alive() && f.civ.is_none()).map(|f| f.id).take(2).collect();
        assert_eq!(beasts.len(), 2, "the world has beasts of legend");
        slay_figure(&mut a, beasts[0]);
        a.notice();
        let name = a.hero_dwarf().name.clone();
        let first = name.split(' ').next().unwrap().to_string();
        assert!(a.hero_dwarf().epithet.is_some(), "a name won");
        assert_eq!(a.fame.deeds.len(), 1, "{:?}", a.fame.deeds);
        let d = &a.fame.deeds[0];
        assert!(d.weight >= LEGEND_WEIGHT && d.text.contains("hailed"), "{:?}", d);
        assert!(a.fame.renown >= LEGEND_WEIGHT);
        assert!(a.ow().events.iter().any(|e| e.text.contains(&first) && e.text.contains("hailed")), "in the legends");
        // The saga: setting out, then the deed.
        let s = a.fame.saga.as_ref().unwrap();
        assert_eq!(s.kind, SagaKind::Wander);
        assert_eq!(s.chapters.len(), 2);
        assert_eq!(s.chapters[0].1, Beat::Cause);
        assert!(s.chapters[1].2.starts_with(&first));
        // Told once: looking again finds nothing new.
        a.notice();
        assert_eq!(a.fame.deeds.len(), 1);
        // A second beast, no grander name: its own deed.
        slay_figure(&mut a, beasts[1]);
        a.notice();
        assert_eq!(a.fame.deeds.len(), 2);
        let fig = a.ow().figures[beasts[1] as usize].name.clone();
        assert!(a.fame.deeds[1].text.contains(&fig), "{:?}", a.fame.deeds[1]);
        assert!(a.ow().events.iter().any(|e| e.text.contains(&fig) && e.text.contains(&first)));
    }

    #[test]
    fn murdering_townsfolk_is_infamy() {
        let mut a = super::super::quick_start(world(), 6);
        a.notice();
        let civ = a.hero.civ;
        let v = a.visit.as_mut().unwrap();
        let folk = v.g.creatures.values().find(|c| c.visitor == super::super::sitegen::town::FOLK).map(|c| c.id).expect("folk at home");
        let h = v.hero;
        let c = v.g.creatures.remove(&folk).unwrap();
        let mut d = std::mem::take(&mut v.g.dwarves[h]);
        v.g.credit_kill(&mut d, &c);
        v.g.dwarves[h] = d;
        a.notice();
        assert!(a.fame.infamy >= MURDER_INFAMY, "{:?}", a.fame);
        assert!(a.fame.deeds.last().unwrap().text.starts_with("murdered"));
        assert!(a.fame.by_civ.get(&civ.unwrap()).copied().unwrap_or(0) < 0, "their own people think less of them");
        assert!(a.ow().events.iter().any(|e| e.text.contains("murdered")));
    }

    #[test]
    fn the_world_remembers_the_dead() {
        let ow = world();
        let lair = ow.sites.iter().find(|s| s.kind == SiteKind::Lair && !s.ruined).map(|s| s.id).expect("a lair");
        let mut a = super::super::quick_start(ow, 7);
        a.notice();
        a.enter(SiteKey::Site(lair)).unwrap();
        a.notice();
        assert!(a.fame.deeds.last().unwrap().text.contains("lair"), "{:?}", a.fame.deeds);
        // A name, then struck down by a goblin.
        let v = a.visit.as_mut().unwrap();
        let h = v.hero;
        v.g.dwarves[h].epithet = Some("Wolfbane".into());
        a.fame.seen.epithet = Some("Wolfbane".into());
        let v = a.visit.as_mut().unwrap();
        let tick = v.g.tick;
        v.g.annals.wounded_by.push((HERO_ID, "goblin".into(), tick));
        let mut d = std::mem::take(&mut v.g.dwarves[h]);
        let fell_at = d.pos;
        v.g.kill_for_test(&mut d, "was struck down");
        v.g.dwarves[h] = d;
        assert_eq!(v.g.act(h, Action::Wait(1)), Outcome::Died);
        a.notice();
        assert!(matches!(&a.over, Some(AdvEnd::Died { cause, .. }) if cause.contains("slain by a goblin")), "{:?}", a.over);
        let e = a.fame.end.clone().expect("the ending is written");
        assert!(e.died && e.called.contains("Wolfbane"));
        let ow = a.ow();
        let f = &ow.figures[e.figure as usize];
        assert_eq!(f.title.as_deref(), Some("Wolfbane"));
        assert!(f.died.is_some() && f.kind.contains("adventurer"));
        let r = ow.remains.last().expect("remains");
        assert_eq!((r.site, r.pos), (Some(lair), (fell_at.x, fell_at.y, fell_at.z)));
        assert!(r.gear.iter().any(|g| g.0 == ItemKind::Axe), "{:?}", r.gear);
        let s = ow.sagas.last().expect("a saga");
        assert!(s.kind == SagaKind::Wander && s.doom && s.chapters.last().unwrap().1 == Beat::Ending, "{:?}", s);
        assert!(ow.events.iter().any(|ev| ev.text == e.legend && ev.text.contains("slain by a goblin")), "{}", e.legend);
        // Told once.
        let n = a.ow().figures.len();
        a.hero_died("died again");
        assert_eq!(a.ow().figures.len(), n);
        // Another adventurer comes to the lair: the bones and the axe lie
        // where the first fell, and the axe can be taken up.
        let ow = a.take_world();
        let mut b = super::super::quick_start(ow, 8);
        b.notice();
        b.enter(SiteKey::Site(lair)).unwrap();
        b.notice();
        let v = b.visit.as_mut().unwrap();
        let bones = bones_label(&e.called);
        let at = v.g.items.values().find(|it| it.label.as_deref() == Some(bones.as_str())).map(|it| it.pos).expect("the bones lie here");
        assert!((at.x - fell_at.x).abs() <= 4 && (at.y - fell_at.y).abs() <= 4, "where they fell: {:?} {:?}", at, fell_at);
        let axe = v.g.items.values().find(|it| it.pos == at && it.kind == ItemKind::Axe && it.carried_by.is_none()).map(|it| it.id).expect("the axe by them");
        let h = v.hero;
        v.g.dwarves[h].pos = at;
        v.g.creatures.clear();
        assert!(!matches!(v.g.act(h, Action::Get(vec![axe])), Outcome::Refused(_)));
        b.notice();
        assert!(b.fame.deeds.last().unwrap().text.contains("found the bones"), "{:?}", b.fame.deeds);
        assert!(b.ow().remains.last().unwrap().gear.iter().all(|g| g.0 != ItemKind::Axe), "taken from the remains");
    }

    #[test]
    fn a_hero_settles_down_in_glory() {
        let mut a = super::super::quick_start(world(), 9);
        a.notice();
        a.deed("drove the wolves from the hills", 3);
        assert!(a.can_retire().is_ok(), "{:?}", a.can_retire());
        a.retire();
        assert!(matches!(a.over, Some(AdvEnd::Retired { .. })));
        let e = a.fame.end.clone().unwrap();
        assert!(!e.died && e.remains.is_none());
        assert!(a.ow().figures[e.figure as usize].alive());
        let s = a.ow().sagas.last().expect("the saga is kept");
        assert!(!s.doom && s.chapters.last().unwrap().1 == Beat::Ending);
        // Not in the wild.
        let mut b = in_the_wild(10);
        assert!(b.can_retire().is_err());
        b.retire();
        assert!(b.over.is_none());
    }

    #[test]
    fn from_death_to_a_new_fort() {
        use crate::app::{App, Mode};
        use crate::game::adv::View;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut app = App::new(5);
        app.save_dir = std::env::temp_dir().join("dwarfclone-no-saves-here");
        let mut a = in_the_wild(11);
        a.deed("crossed the Grey Waste alone", 2);
        let v = a.visit.as_mut().unwrap();
        let h = v.hero;
        let mut d = std::mem::take(&mut v.g.dwarves[h]);
        v.g.kill_for_test(&mut d, "has bled to death");
        v.g.dwarves[h] = d;
        let year = a.year();
        app.adventure_ready(Box::new(a));
        let s = app.adv.as_ref().unwrap();
        assert_eq!(s.view, View::Dead, "straight to the end page");
        let called = s.adv.fame.end.as_ref().unwrap().called.clone();
        let title = s.adv.fame.saga.as_ref().unwrap().title.clone();
        // Drawn, at a small terminal and a large one.
        for (w, h) in [(80, 24), (140, 44)] {
            let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
            term.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
            let text: String = term.backend().buffer().content().iter().map(|c| c.symbol().to_string()).collect();
            assert!(text.contains("is dead"), "{}x{}", w, h);
            if w > 100 {
                assert!(text.contains("wanderer"), "the saga leads at {}x{}", w, h);
            }
        }
        app.on_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
        assert!(matches!(app.mode, Mode::WorldMap { .. }), "{:?}", app.mode);
        assert!(app.adv.is_none());
        let ow = app.game.overworld.clone();
        assert!(ow.sagas.iter().any(|s| s.title == title && s.doom));
        let first = called.split(' ').next().unwrap().to_string();
        assert!(ow.figures.iter().any(|f| f.name.starts_with(&first) && f.died.is_some()));
        let (x, y) = app.game.site;
        let g = Game::embark_with(5, ow, x, y, crate::world::MapSize::ALL[0], &|_, _| {});
        assert!(g.start_year > year, "the fort comes after the hero's death");
        assert!(g.overworld.sagas.iter().any(|s| s.title == title));
    }

    /// One step along the way to `to` in the site (the hero's own feet: a
    /// step, or a stair), or a wait if there is no way.
    fn step_toward(v: &super::super::Visit, to: Pos) -> Action {
        let me = v.g.dwarves[v.hero].pos;
        match crate::path::astar(&v.g.world, me, to, 40_000, Mover::Dwarf).and_then(|p| p.first().copied()) {
            Some(p) if p.z != me.z => Action::Climb(p.z - me.z),
            Some(p) => Action::Move(p.x - me.x, p.y - me.y),
            None => Action::Wait(10),
        }
    }

    /// Fight it out with the foes `foes` picks, walking to them; false if
    /// the hero died.
    fn fight(a: &mut Adventure, foes: &dyn Fn(&super::super::Visit) -> Vec<u32>) -> bool {
        for n in 0..800 {
            let Some(v) = a.visit.as_mut() else { return a.over.is_none() };
            let h = v.hero;
            let me = v.g.dwarves[h].pos;
            let near = foes(v).into_iter().filter_map(|id| v.g.creatures.get(&id)).filter(|c| c.alive).min_by_key(|c| (c.pos.x - me.x).abs().max((c.pos.y - me.y).abs()) + 10 * (c.pos.z - me.z).abs()).map(|c| (c.id, c.pos));
            let Some((id, fp)) = near else {
                mend(a);
                return a.over.is_none();
            };
            let close = fp.z == me.z && (fp.x - me.x).abs() <= 1 && (fp.y - me.y).abs() <= 1;
            // Bind a bleeding wound while no foe is at hand; let them come
            // a while, then go to them.
            let act = if close {
                Action::Attack { target: super::super::turn::Target::Creature(id), aim: None }
            } else if v.g.dwarves[h].body.bleed_rate() > 0 {
                Action::Bind
            } else if n < 40 {
                Action::Wait(10)
            } else {
                step_toward(v, fp)
            };
            let out = v.g.act(h, act);
            a.notice();
            if out == Outcome::Died || a.over.is_some() {
                return false;
            }
        }
        true
    }

    /// Bind and rest until whole, as a careful hero does between fights.
    fn mend(a: &mut Adventure) {
        for _ in 0..30 {
            let Some(v) = a.visit.as_mut() else { return };
            let h = v.hero;
            let b = &v.g.dwarves[h].body;
            let act = if b.bleed_rate() > 0 {
                Action::Bind
            } else if !b.healed() {
                Action::Wait(300)
            } else {
                return;
            };
            let out = v.g.act(h, act);
            a.notice();
            if out == Outcome::Died || matches!(out, Outcome::Refused(_)) {
                return;
            }
        }
    }

    /// Walk the world to cell `to`, round what blocks the way; what comes
    /// on the road is slipped away from, or fought. The careful hero keeps
    /// to cover. False if the hero
    /// didn't get there.
    fn walk_to(a: &mut Adventure, to: (i32, i32)) -> bool {
        use super::super::travel::TravelOutcome;
        let hostile = |v: &super::super::Visit| v.g.creatures.values().filter(|c| c.hostile()).map(|c| c.id).collect::<Vec<u32>>();
        for _ in 0..300 {
            if a.over.is_some() {
                return false;
            }
            if a.at == to {
                return true;
            }
            let Some((dx, dy)) = way(a, to) else { return false };
            {
                match a.travel_with(dx, dy, true) {
                    TravelOutcome::Blocked(_) => return false,
                    TravelOutcome::Arrived => {}
                    TravelOutcome::Died(_) => return false,
                    TravelOutcome::Encounter(e) => {
                        if !a.evade(&e, true) && a.meet(&e).is_ok() {
                            a.notice();
                            if !fight(a, &hostile) {
                                return false;
                            }
                            a.take_road();
                        }
                    }
                }
                a.notice();
            }
        }
        a.at == to
    }

    /// The first step of the shortest way over land from where the hero
    /// stands to `to` (cell by cell, round lakes and seas).
    fn way(a: &Adventure, to: (i32, i32)) -> Option<(i32, i32)> {
        use super::super::travel::DIRS;
        let ow = a.ow();
        let (w, h) = (ow.w, ow.h);
        let idx = |(x, y): (i32, i32)| (y * w + x) as usize;
        let mut from: Vec<Option<(i32, i32)>> = vec![None; (w * h) as usize];
        let mut q = std::collections::VecDeque::new();
        from[idx(to)] = Some(to);
        q.push_back(to);
        // Searched from the goal, so the first step is read off at the start.
        while let Some(c) = q.pop_front() {
            if c == a.at {
                let n = from[idx(c)]?;
                return Some((n.0 - c.0, n.1 - c.1));
            }
            for (dx, dy) in DIRS {
                let n = (c.0 + dx, c.1 + dy);
                if n.0 < 0 || n.1 < 0 || n.0 >= w || n.1 >= h || from[idx(n)].is_some() {
                    continue;
                }
                if n != a.at && a.crossing_days(n.0, n.1).is_none() {
                    continue;
                }
                from[idx(n)] = Some(c);
                q.push_back(n);
            }
        }
        None
    }

    /// G03's Done-when: a hero leaves a town, walks the world to the ruin
    /// of a fallen fort, fights what lives there, takes its treasure, and
    /// walks home with it, and the legends tell of it. Every hero leaves a
    /// deed; the dead leave their bones and a line in the legends. (Picking
    /// the target and the way is the bot's, A18: here the script walks
    /// straight at it.)
    #[test]
    fn a_hero_walks_home_with_the_treasure_of_a_fallen_fort() {
        use super::super::sitegen::fallen::tests::{fallen_fort, tmp, PRIZE};
        let base = tmp("a10-g03");
        let ow = fallen_fort(37, |_| Some("goblin".into()), &base);
        let file = ow.forts[0].save_file.clone();
        let fort = ow.forts[0].site;
        let mut won = 0;
        for seed in 40..60u64 {
            let dir = tmp(&format!("a10-g03-{}", seed));
            std::fs::create_dir_all(&dir).unwrap();
            for f in [file.clone(), format!("{}.meta.json", file.trim_end_matches(".json"))] {
                std::fs::copy(base.join(&f), dir.join(&f)).unwrap();
            }
            let mut a = super::super::quick_start(ow.clone(), seed);
            a.saves_dir = Some(dir.clone());
            a.knobs.set.insert("fallen.band".into(), 1.0);
            let Some(SiteKey::Site(home)) = a.visit.as_ref().map(|v| v.key) else { panic!("set out from a town") };
            let home_cell = a.ow().sites.iter().find(|s| s.id == home).map(|s| (s.x, s.y)).unwrap();
            // Armour from home.
            let v = a.visit.as_mut().unwrap();
            let (h, p) = (v.hero, v.g.dwarves[v.hero].pos);
            for kind in [ItemKind::Helm, ItemKind::Breastplate] {
                let id = v.g.spawn_item(kind, Mat::Iron, p);
                v.g.items.get_mut(&id).unwrap().carried_by = Some(HERO_ID);
                v.g.dwarves[h].armor.push(id);
            }
            a.notice();
            let name = a.hero_dwarf().name.clone();
            let first = first(&name);
            // Out of the town, and over the land to the ruin.
            a.take_road();
            let mut ok = walk_to(&mut a, fort);
            let mut why = "the road";
            if ok {
                why = "the ruin's keepers";
                a.enter(SiteKey::Fort(0)).unwrap();
                a.notice();
                assert!(a.fame.deeds.last().map_or(false, |d| d.text.contains("ruin of")), "seed {}: daring the ruin is a deed", seed);
                ok = fight(&mut a, &|v| v.placed.iter().filter(|(_, k)| k.starts_with('@')).map(|(id, _)| *id).collect());
            }
            // Its treasure, taken up where it lies.
            if ok {
                why = "the treasure out of reach";
                let mut last = Pos::new(-1, -1, -1);
                for _ in 0..400 {
                    let v = a.visit.as_mut().unwrap();
                    let h = v.hero;
                    let Some(it) = v.g.items.values().find(|it| it.label.as_deref() == Some(PRIZE)).map(|it| (it.id, it.pos, it.carried_by)) else { break };
                    if it.2 == Some(HERO_ID) {
                        break;
                    }
                    let mut act = if v.g.dwarves[h].pos == it.1 { Action::Get(vec![it.0]) } else { step_toward(v, it.1) };
                    let stuck = v.g.dwarves[h].pos == last;
                    last = v.g.dwarves[h].pos;
                    if matches!(act, Action::Wait(_)) || (stuck && !matches!(act, Action::Get(_))) {
                        // (No way found by the script's walker: stood by it,
                        // as A5's version of this test did.)
                        v.g.dwarves[h].pos = it.1;
                        act = Action::Get(vec![it.0]);
                    }
                    if v.g.act(h, act) == Outcome::Died {
                        break;
                    }
                    a.notice();
                }
                ok = a.over.is_none() && a.carried_treasures().iter().any(|t| t == PRIZE);
            }
            // And home again.
            if ok {
                why = "the road home";
                a.take_road();
                a.notice();
                ok = walk_to(&mut a, home_cell);
            }
            if ok {
                a.enter(SiteKey::Site(home)).unwrap();
                a.notice();
            }
            if a.over.is_some() {
                // The dead are remembered where they fell.
                let e = a.fame.end.as_ref().expect("an ending");
                assert!(a.ow().remains.iter().any(|r| r.name == e.called), "seed {}: no bones", seed);
                assert!(a.ow().events.iter().any(|ev| ev.text == e.legend), "seed {}: no legend", seed);
                eprintln!("seed {}: {}", seed, e.legend);
            }
            let _ = std::fs::remove_dir_all(&dir);
            if !ok || a.over.is_some() {
                eprintln!("seed {}: undone by {}", seed, why);
                continue;
            }
            let v = a.visit.as_ref().unwrap();
            assert_eq!(v.key, SiteKey::Site(home));
            assert!(v.g.items.values().any(|it| it.carried_by == Some(HERO_ID) && it.label.as_deref() == Some(PRIZE)), "brought home");
            let deed = a.fame.deeds.last().unwrap();
            assert!(deed.text.starts_with("came home") && deed.text.contains(PRIZE), "{:?}", a.fame.deeds);
            assert!(a.ow().events.iter().any(|e| e.text.contains(PRIZE) && e.text.contains(&first) && e.text.contains("came home")), "in the legends");
            assert!(a.ow().forts[0].robbed.last().map_or(false, |r| r.by.contains(&first)), "the ruin remembers who robbed it");
            let saga = a.fame.saga.as_ref().unwrap();
            assert!(saga.chapters.iter().any(|c| c.2.contains("ruin of")), "{:?}", saga.chapters);
            won += 1;
            break;
        }
        let _ = std::fs::remove_dir_all(&base);
        assert!(won > 0, "no hero came home");
    }

    #[test]
    fn a_fallen_fort_leaves_its_mark() {
        let mut g = Game::new(5);
        for _ in 0..DAY {
            g.step();
        }
        assert!(g.overworld.forts.is_empty());
        for i in 0..g.dwarves.len() {
            let mut d = std::mem::take(&mut g.dwarves[i]);
            if d.alive {
                g.kill_for_test(&mut d, "was struck down");
            }
            g.dwarves[i] = d;
        }
        for _ in 0..10 {
            g.step();
        }
        let ow = &g.overworld;
        assert_eq!(ow.forts.len(), 1, "one mark");
        let m = &ow.forts[0];
        assert_eq!((m.name.as_str(), m.site, m.founded), (g.fortress_name.as_str(), g.site, g.start_year));
        assert_eq!(m.fell, Some(g.start_year));
        assert_eq!(m.save_file, crate::saves::fort_file(&g.fortress_name));
        assert_eq!(m.peak_pop, g.dwarves.len());
        assert_eq!(ow.fort_at(g.site.0, g.site.1), Some(0));
        assert_eq!(ow.map_look(g.site.0, g.site.1).0, '×');
    }
}
