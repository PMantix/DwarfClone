//! A mortal overseer (O07; packages P8 and P17 fill this in): the overseer
//! is a dwarf of the fort, trusted or doubted, and may fall and be
//! succeeded.
//!
//! - *Who it is* (`embody`): on hand-over it becomes the expedition leader
//!   (else the most sociable adult), and takes their name.
//! - *Its fall* (`ov_reign`, `ov_fallen`): killed, gone mad, lost, jailed,
//!   deposed or out-voted at the mayor's election, it is succeeded by the
//!   mayor or the dwarf the fort looks up to most, with traits of their
//!   own. Each reign goes in `o.reigns` and the year's chronicle.
//! - *Faith* (`faith`, `trust_daily`): deaths, an empty larder and a broken
//!   promise of shelter cost it; a siege beaten off, a feast, an artifact
//!   or the monument win it back. Low faith brings grumbling, then
//!   defiance and food riots, and in the end deposition. The brain feels
//!   it as `Sense::Doubt` and tries to win the fort back.
//!
//! Successors and their traits come from hashes, not the game's random
//! numbers; only defiance (a mason who won't cut blocks) is a roll.

use super::ambitions::{Ambition, Sense};
use super::overseer::Overseer;
use super::personality::Trait;
use super::sagas::Beat;
use super::troubles::Lack;
use super::*;
use serde::{Deserialize, Serialize};

/// How a reign ended.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReignEnd {
    /// Killed, with the cause.
    Died(String),
    Mad,
    Lost,
    Deposed,
    Elected,
}

impl ReignEnd {
    /// For the counters: `m2.reign_<key>`.
    fn key(&self) -> &'static str {
        match self {
            ReignEnd::Died(_) => "died",
            ReignEnd::Mad => "mad",
            ReignEnd::Lost => "lost",
            ReignEnd::Deposed => "deposed",
            ReignEnd::Elected => "elected",
        }
    }
}

/// One overseer's time in charge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reign {
    pub name: String,
    #[serde(default)]
    pub dwarf: Option<u32>,
    #[serde(default)]
    pub traits: Vec<super::personality::Trait>,
    pub from: u64,
    #[serde(default)]
    pub to: Option<u64>,
    #[serde(default)]
    pub end: Option<ReignEnd>,
    /// Faith was last reckoned up to here.
    #[serde(default)]
    pub checked: u64,
    /// Faith has been below the deposition mark since then (0: it isn't).
    #[serde(default)]
    pub low_since: u64,
    /// The dwarves last grumbled, and last rioted.
    #[serde(default)]
    pub grumbled: u64,
    #[serde(default)]
    pub riot: u64,
    /// Feasts already counted this year.
    #[serde(default)]
    pub feasts: usize,
    /// The fort has turned against it (a reign saga is open).
    #[serde(default)]
    pub doubted: bool,
}

impl Reign {
    fn new(name: String, dwarf: u32, traits: Vec<Trait>, from: u64) -> Reign {
        Reign { name, dwarf: Some(dwarf), traits, from, to: None, end: None, checked: from, low_since: 0, grumbled: from, riot: 0, feasts: 0, doubted: false }
    }

    /// "Urist the Cautious".
    pub fn styled(&self) -> String {
        let first = super::stories::first_name(&self.name).to_string();
        match self.traits.first() {
            Some(t) => format!("{} the {}", first, cap(t.name())),
            None => first,
        }
    }
}

/// A page of the overseer's journal (OVR-06), one a season.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JournalPage {
    pub tick: u64,
    pub lines: Vec<String>,
}

/// Faith in an overseer starts here (0..100).
pub fn sixty() -> i32 {
    60
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

/// "first", "second"... "twelfth", then "13th".
pub fn ordinal(n: usize) -> String {
    const W: [&str; 12] = ["first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth", "ninth", "tenth", "eleventh", "twelfth"];
    W.get(n.wrapping_sub(1)).map_or(format!("{}th", n), |w| w.to_string())
}

/// Two traits from a dwarf's personality: a strong leaning gives its
/// trait, the rest are filled from `seed` (a hash, not the game's dice).
pub fn traits_of(p: &crate::dwarf::Personality, seed: u64) -> Vec<Trait> {
    let mut leans: Vec<(i32, Trait)> = Vec::new();
    if p.anxiety > 20 {
        leans.push((p.anxiety, Trait::Cautious));
    } else if p.anxiety < -20 {
        leans.push((-p.anxiety, Trait::Bold));
    }
    if p.artistry > 25 {
        leans.push((p.artistry, Trait::Vain));
    }
    if p.sociability > 25 {
        leans.push((p.sociability, Trait::Cheerful));
    }
    if p.diligence > 25 {
        leans.push((p.diligence, Trait::Stubborn));
    }
    if p.temper < -25 {
        leans.push((-p.temper, Trait::Sentimental));
    }
    leans.sort_by(|a, b| b.0.cmp(&a.0));
    let mut out: Vec<Trait> = leans.into_iter().map(|l| l.1).take(2).collect();
    let mut k = 0;
    while out.len() < 2 {
        let h = super::names::hash(&[seed, k]);
        let t = Trait::ALL[(h % Trait::ALL.len() as u64) as usize];
        if !out.contains(&t) && !out.iter().any(|o| clash(*o, t)) {
            out.push(t);
        }
        k += 1;
    }
    out
}

fn clash(a: Trait, b: Trait) -> bool {
    matches!((a, b), (Trait::Cautious, Trait::Bold) | (Trait::Bold, Trait::Cautious))
}

/// "has been struck down" → "was struck down"; "has drowned" → "drowned".
fn past(cause: &str) -> String {
    if let Some(r) = cause.strip_prefix("has been ") {
        format!("was {}", r)
    } else if let Some(r) = cause.strip_prefix("has ") {
        r.to_string()
    } else {
        cause.to_string()
    }
}

fn a_an(w: &str) -> &'static str {
    if w.starts_with(['a', 'e', 'i', 'o', 'u', 'A', 'E', 'I', 'O', 'U']) {
        "an"
    } else {
        "a"
    }
}

impl Game {
    /// Faith knobs: where grumbling, defiance and deposition begin.
    fn faith_marks(&self) -> (i32, i32, i32) {
        (self.knob("faith.grumble", 40.0) as i32, self.knob("faith.defy", 25.0) as i32, self.knob("faith.depose", 10.0) as i32)
    }

    /// A steady number for this fort, to seed hashes with.
    fn fort_seed(&self) -> u64 {
        super::stories::hash(&self.fortress_name, 0x0e7)
    }

    /// Faith moves `by` because of `why`: logged (the last dozen), and the
    /// same reason twice in a row is one line with the sum.
    pub(super) fn faith(&mut self, o: &mut Overseer, by: i32, why: &str) {
        if by == 0 {
            return;
        }
        o.faith = (o.faith + by).clamp(0, 100);
        let tick = self.tick;
        match o.faith_log.last_mut() {
            Some(l) if l.2 == why && tick < l.0 + SEASON => {
                l.0 = tick;
                l.1 += by;
            }
            _ => o.faith_log.push((tick, by, why.to_string())),
        }
        if o.faith_log.len() > 12 {
            o.faith_log.remove(0);
        }
    }

    /// The overseer becomes a dwarf of the fort: the expedition leader,
    /// else the most sociable adult. Its name becomes theirs; traits it
    /// has already (rolled or picked) it keeps.
    pub(super) fn embody(&mut self, o: &mut Overseer) {
        let pick = self
            .dwarves
            .iter()
            .filter(|d| d.alive && !d.insane && !d.is_child())
            .max_by_key(|d| (d.noble == Some(crate::dwarf::Noble::Leader), d.personality.sociability, std::cmp::Reverse(d.id)))
            .map(|d| (d.id, d.name.clone(), d.personality.clone()));
        let Some((id, name, p)) = pick else { return };
        o.dwarf = Some(id);
        o.name = name.clone();
        if o.traits.is_empty() {
            o.traits = traits_of(&p, super::names::hash(&[self.fort_seed(), id as u64]));
        }
        if o.reigns.last().map_or(true, |r| r.to.is_some() || r.dwarf != Some(id)) {
            o.reigns.push(Reign::new(name, id, o.traits.clone(), self.tick));
        }
    }

    /// `kill`: if it was the overseer, its fall is told at once.
    pub(super) fn reign_kill(&mut self, d: &Dwarf, cause: &str) {
        if self.overseer.as_ref().map_or(false, |o| o.dwarf == Some(d.id)) {
            let by = self.annals.deaths.last().filter(|n| n.name == d.name).and_then(|n| n.killer.clone());
            let how = match by {
                Some(k) if cause.contains("struck") || cause.contains("bled") => format!("was cut down by {} {}", a_an(&k), k),
                _ => past(cause),
            };
            self.ov_fell = Some((d.id, how));
        }
    }

    /// Every ten ticks (`ov_alert_hooks`): the overseer has just been
    /// killed. Cheap when it hasn't.
    pub(super) fn ov_fallen(&mut self, o: &mut Overseer) {
        let Some((id, how)) = self.ov_fell.take() else { return };
        if o.dwarf == Some(id) {
            self.succeed(o, ReignEnd::Died(how), None);
        }
    }

    /// Daily (`ov_daily_hooks`): who the overseer is, whether it still
    /// stands, and what the fort thinks of it.
    pub(super) fn ov_reign(&mut self, o: &mut Overseer) {
        if o.dwarf.is_none() {
            self.embody(o);
            return;
        }
        if o.reigns.last().map_or(true, |r| r.to.is_some()) {
            // An old save, or a reign cut short with no heir: start the book again.
            let (name, id, traits, tick) = (o.name.clone(), o.dwarf.unwrap(), o.traits.clone(), self.tick);
            o.reigns.push(Reign::new(name, id, traits, tick));
        }
        if let Some(end) = self.reign_over(o) {
            self.succeed(o, end, None);
            return;
        }
        self.trust_daily(o);
        self.trust_effects(o);
    }

    /// Is the overseer dwarf still fit to rule? If not, how its reign ended.
    fn reign_over(&self, o: &Overseer) -> Option<ReignEnd> {
        let id = o.dwarf?;
        let Some(d) = self.dwarves.iter().find(|d| d.id == id) else { return Some(ReignEnd::Lost) };
        if !d.alive {
            let how = self.annals.deaths.iter().rev().find(|n| n.name == d.name).map_or("died".to_string(), |n| match &n.killer {
                Some(k) => format!("was cut down by {} {}", a_an(k), k),
                None => past(&n.cause),
            });
            return Some(ReignEnd::Died(how));
        }
        if d.insane {
            return Some(ReignEnd::Mad);
        }
        if d.jailed_until > self.tick + DAY {
            return Some(ReignEnd::Deposed);
        }
        None
    }

    /// The day's faith: what happened since yesterday.
    fn trust_daily(&mut self, o: &mut Overseer) {
        let tick = self.tick;
        let since = o.reigns.last().map_or(tick, |r| r.checked);
        // The dead: a promise of shelter broken costs twice.
        let mut lost = 0;
        let mut why = "dwarves died";
        for n in self.annals.deaths.iter().filter(|n| n.tick >= since && n.tick < tick) {
            if n.sheltered {
                lost += 6;
                why = "a broken promise: dwarves died in the shelter";
            } else {
                lost += 3;
            }
        }
        self.faith(o, -lost.min(15), why);
        // Sieges beaten off.
        let ended: Vec<(String, String)> = self.annals.sieges.iter().filter(|s| s.ended.map_or(false, |e| e >= since && e < tick)).map(|s| (s.outcome.clone(), s.foe.clone())).collect();
        for (outcome, foe) in ended {
            match outcome.as_str() {
                "routed" => self.faith(o, 8, &format!("the {} were routed", foe)),
                "withdrew" => self.faith(o, 3, &format!("the {} withdrew", foe)),
                _ => {}
            }
        }
        // Artifacts and the monument.
        let (mut art, mut monument) = (0, false);
        for m in self.memory_since(since).filter(|m| m.tick < tick) {
            match m.kind {
                super::memory::MemoryKind::Artifact => art += 1,
                super::memory::MemoryKind::Monument => monument = true,
                _ => {}
            }
        }
        self.faith(o, 2 * art.min(3), "an artifact was made");
        if monument {
            self.faith(o, 10, "the monument");
        }
        // Feasts.
        let feasts = self.year_log.festivals.len();
        let counted = o.reigns.last().map_or(0, |r| r.feasts);
        if feasts > counted {
            self.faith(o, 3 * (feasts - counted) as i32, "a feast");
        }
        // An empty larder or dry barrels, every day it lasts.
        let hungry = o.worries.iter().any(|w| matches!(w.lack, Lack::Food | Lack::Drink));
        if hungry {
            self.faith(o, -1, "the larder is bare");
        } else if o.faith < 60 && lost == 0 && tick / DAY % 3 == 0 {
            // Quiet days mend it, slowly.
            self.faith(o, 1, "quiet days");
        }
        if let Some(r) = o.reigns.last_mut() {
            r.checked = tick;
            r.feasts = feasts;
        }
    }

    /// What low faith does: grumbling, defiance, riots, and at the last
    /// deposition. A reign the fort turns against becomes a saga.
    fn trust_effects(&mut self, o: &mut Overseer) {
        let tick = self.tick;
        let (grumble, defy, depose) = self.faith_marks();
        let first = super::stories::first_name(&o.name).to_string();
        let id = o.dwarf.unwrap_or(0);
        let Some(r) = o.reigns.last().cloned() else { return };
        // Grumbling in the halls, weekly.
        if o.faith < grumble && tick >= r.grumbled + 7 * DAY {
            for d in self.dwarves.iter_mut().filter(|d| d.alive && !d.is_child() && d.id != id) {
                d.think(&format!("grumbled about the overseer {}", first), 5, tick);
            }
            Self::note_ov(o, tick, "Heard the dwarves grumbling about it in the halls.".into());
            if let Some(r) = o.reigns.last_mut() {
                r.grumbled = tick;
            }
        }
        // The fort turns against it: a saga begins.
        if o.faith < defy && !r.doubted {
            let fort = self.fortress_name.clone();
            self.msg(&format!("The dwarves of {} have lost faith in the overseer {}.", fort, first), (230, 150, 90));
            let why = o.faith_log.iter().rev().find(|l| l.1 < 0).map_or("hard times".to_string(), |l| l.2.clone());
            self.saga_reign(id, &first, Beat::Cause, format!("The dwarves of {} lost faith in the overseer {}: {}.", fort, o.name, why), None);
            if let Some(r) = o.reigns.last_mut() {
                r.doubted = true;
            }
        } else if r.doubted && o.faith >= 50 {
            self.msg(&format!("The overseer {} has won back the fort's faith.", first), (150, 220, 255));
            self.saga_reign(id, &first, Beat::Ending, format!("{} won the fort's faith back.", first), Some((false, format!("The grumbling died away, and {} kept the ledger.", first))));
            if let Some(r) = o.reigns.last_mut() {
                r.doubted = false;
            }
        }
        // Defiance: an order it gave yesterday, ignored.
        if o.faith < defy {
            let fresh: Vec<Recipe> = o.boosts.iter().filter(|b| b.2 + DAY >= tick + DAY * 20).map(|b| b.0).collect();
            for recipe in fresh {
                if self.rng.chance(self.knob("faith.defiance", 0.3)) {
                    o.boosts.retain(|b| b.0 != recipe);
                    Self::note_ov(o, tick, format!("Called for more {}, and nobody stirred to make it.", recipe.name().to_lowercase()));
                    self.count("m2.defied", 1);
                }
            }
            // A food riot, when hunger and doubt meet.
            let hungry = o.worries.iter().any(|w| matches!(w.lack, Lack::Food | Lack::Drink));
            if hungry && (r.riot == 0 || tick >= r.riot + SEASON) {
                self.food_riot(o, id, &first);
            }
        }
        // Deposition.
        if o.faith < depose {
            let since = o.reigns.last().map_or(0, |r| r.low_since);
            if since == 0 {
                if let Some(r) = o.reigns.last_mut() {
                    r.low_since = tick;
                }
                self.msg(&format!("There is talk of putting the overseer {} aside.", first), (230, 120, 90));
            } else if tick >= since + self.knob("faith.depose_days", 10.0) as u64 * DAY {
                self.succeed(o, ReignEnd::Deposed, None);
            }
        } else if let Some(r) = o.reigns.last_mut() {
            r.low_since = 0;
        }
    }

    /// Up to three of the most stressed dwarves riot over the empty larder.
    fn food_riot(&mut self, o: &mut Overseer, id: u32, first: &str) {
        let tick = self.tick;
        let mut angry: Vec<(i32, usize)> = self
            .dwarves
            .iter()
            .enumerate()
            .filter(|(_, d)| d.alive && !d.insane && !d.is_child() && d.id != id && d.tantrum == 0 && d.stress * 2 >= d.tantrum_threshold())
            .map(|(i, d)| (-d.stress, i))
            .collect();
        if angry.is_empty() {
            return;
        }
        angry.sort();
        let mut names = Vec::new();
        let mut at = None;
        for (_, i) in angry.into_iter().take(3) {
            let mut d = std::mem::take(&mut self.dwarves[i]);
            self.release_task(&mut d);
            d.tantrum = 900;
            d.think(&format!("rioted against the overseer {}", first), -20, tick);
            names.push(super::stories::first_name(&d.name).to_string());
            at = at.or(Some(d.pos));
            self.dwarves[i] = d;
        }
        if let Some(r) = o.reigns.last_mut() {
            r.riot = tick;
        }
        let who = names.join(", ");
        let text = format!("Hungry and done with {}, {} riot{} in the halls.", first, who, if names.len() == 1 { "s" } else { "" });
        self.msg(&text, (255, 90, 90));
        self.spotlight("A food riot", &text, at, (230, 110, 70));
        self.saga_reign(id, first, Beat::Escalation, format!("Hunger came, and {} rioted against {}.", who, first), None);
        Self::note_ov(o, tick, format!("Faced a riot over the empty larder: {}.", who));
        self.count("m2.riots", 1);
    }

    /// The mayor's election (`nobles_daily`): who wins. An overseer the
    /// fort still trusts is elected itself; else the winner succeeds it.
    pub(super) fn reign_election(&mut self, pick: Option<u32>) -> Option<u32> {
        let Some(mut o) = self.overseer.take() else { return pick };
        let mine = o.dwarf.filter(|&id| self.dwarves.iter().any(|d| d.id == id && d.alive && !d.insane && !d.is_child()));
        let won = match (mine, pick) {
            (Some(m), _) if o.faith >= self.knob("faith.reelect", 60.0) as i32 => {
                self.faith(&mut o, 3, "elected mayor");
                Self::note_ov(&mut o, self.tick, "Was elected mayor by the fort.".into());
                Some(m)
            }
            (Some(m), Some(p)) if p != m => {
                self.succeed(&mut o, ReignEnd::Elected, Some(p));
                Some(p)
            }
            _ => pick,
        };
        self.overseer = Some(o);
        won
    }

    /// Who the fort would follow: the mayor, else the dwarf it looks up to
    /// most (name, sociability, regard, rank).
    fn heir(&self, old: Option<u32>) -> Option<u32> {
        let fit = |d: &Dwarf| d.alive && !d.insane && !d.is_child() && Some(d.id) != old && d.jailed_until <= self.tick;
        if let Some(m) = self.dwarves.iter().find(|d| fit(d) && d.noble == Some(crate::dwarf::Noble::Mayor)) {
            return Some(m.id);
        }
        self.dwarves
            .iter()
            .filter(|d| fit(d))
            .map(|d| {
                let regard: i32 = self.dwarves.iter().filter(|o| o.alive && o.id != d.id).map(|o| o.regard(d.id)).sum();
                let s = d.epithet_rank as i32 * 10 + d.personality.sociability + regard / 10 + if d.noble.is_some() { 15 } else { 0 };
                (s, std::cmp::Reverse(d.id), d.id)
            })
            .max()
            .map(|x| x.2)
    }

    /// The reign ends; the heir (or `heir`) takes up the ledger. Told on a
    /// card, in the legends, the chronicle and the fort's memory.
    pub(super) fn succeed(&mut self, o: &mut Overseer, end: ReignEnd, heir: Option<u32>) {
        let tick = self.tick;
        let old_id = o.dwarf;
        if o.reigns.is_empty() {
            if let Some(id) = old_id {
                o.reigns.push(Reign::new(o.name.clone(), id, o.traits.clone(), 0));
            }
        }
        let old = o.reigns.last().cloned();
        let Some(new_id) = heir.or_else(|| self.heir(old_id)) else {
            // No one left to follow: the reign ends with the fort.
            if let Some(r) = o.reigns.last_mut().filter(|r| r.to.is_none()) {
                r.to = Some(tick);
                r.end = Some(end);
            }
            return;
        };
        let Some(d) = self.dwarves.iter().find(|d| d.id == new_id) else { return };
        let (name, p, pos) = (d.name.clone(), d.personality.clone(), d.pos);
        let mut traits = traits_of(&p, super::names::hash(&[self.fort_seed(), new_id as u64, tick]));
        if traits.iter().all(|t| o.traits.contains(t)) {
            // Not the same overseer again: swap the second trait.
            let mut k = 0;
            loop {
                let t = Trait::ALL[(super::names::hash(&[new_id as u64, tick, 77 + k]) % 8) as usize];
                if !traits.contains(&t) && !o.traits.contains(&t) && !clash(traits[0], t) {
                    traits[1] = t;
                    break;
                }
                k += 1;
            }
        }
        let old_name = o.name.clone();
        let old_first = super::stories::first_name(&old_name).to_string();
        let old_styled = old.as_ref().map_or(old_first.clone(), |r| r.styled());
        let first = super::stories::first_name(&name).to_string();
        let doubted = old.as_ref().map_or(false, |r| r.doubted);
        if let Some(r) = o.reigns.last_mut() {
            r.to = Some(tick);
            r.end = Some(end.clone());
        }
        // The new overseer.
        o.dwarf = Some(new_id);
        o.name = name.clone();
        o.traits = traits.clone();
        o.faith = self.knob("faith.new_reign", 55.0) as i32;
        o.faith_log.clear();
        o.nudges.clear();
        o.ambition = None;
        o.reigns.push(Reign::new(name.clone(), new_id, traits.clone(), tick));
        let n = o.reigns.len();
        let fort = self.fortress_name.clone();
        let tn: Vec<&str> = traits.iter().map(|t| t.name()).collect();
        let (how, short) = match &end {
            ReignEnd::Died(c) => (format!("ended when {} {}", old_first, c), format!("{} {}", old_first, c)),
            ReignEnd::Mad => (format!("ended when {} went mad", old_first), format!("{} has gone mad", old_first)),
            ReignEnd::Lost => (format!("ended when {} was lost", old_first), format!("{} is lost", old_first)),
            ReignEnd::Deposed => (format!("ended when the fort would follow {} no longer", old_first), format!("The fort has put {} aside", old_first)),
            ReignEnd::Elected => (format!("ended at the mayor's election, which {} lost", old_first), format!("{} has lost the election", old_first)),
        };
        let line = format!("The reign of {} {}; {}, {}, took up the ledger as the {} overseer of {}.", old_styled, how, first, tn.join(" and "), ordinal(n), fort);
        let (title, color) = match end {
            ReignEnd::Died(_) => ("The overseer has fallen", (220, 70, 70)),
            ReignEnd::Deposed => ("The overseer is deposed", (230, 140, 70)),
            ReignEnd::Elected => ("A new overseer is elected", (240, 200, 110)),
            _ => ("A new overseer", (220, 170, 90)),
        };
        let card = format!("{}\n\n{} now keeps the ledger, {}. The fort's faith in its new overseer stands at {}.", line, name, tn.join(" and "), o.faith);
        self.msg(&format!("{}. {} is the overseer now.", short, first), color);
        self.spotlight(title, &card, Some(pos), color);
        self.legend(format!("The reign of {} over {} {}; {} took up the ledger.", old_styled, fort, how, name));
        let who: Vec<u32> = std::iter::once(new_id).chain(old_id).collect();
        self.record(super::memory::MemoryKind::Overseer, &who, Some(pos), line.clone(), 7);
        self.year_log.reigns.push(line);
        Self::note_ov(o, tick, format!("Took up the ledger from {}.", old_first));
        // A reign the fort had turned against ends its saga, in grief.
        if doubted || matches!(end, ReignEnd::Deposed) {
            let close = match &end {
                ReignEnd::Deposed => format!("The fort put {} aside, and followed {}.", old_first, first),
                ReignEnd::Elected => format!("The fort chose {} over {}.", first, old_first),
                _ => format!("{} was gone, and {} took up the ledger.", old_first, first),
            };
            let text = format!("The reign of {} {}.", old_styled, how);
            self.saga_reign(old_id.unwrap_or(0), &old_first, Beat::Ending, text, Some((true, close)));
        }
        self.count("m2.reigns", 1);
        self.count(&format!("m2.reign_{}", end.key()), 1);
    }

    /// Senses (`extra_senses`): a doubted overseer tries to win the fort
    /// back with feasts and full stores, and stops sending dwarves away.
    pub(super) fn doubt_sense(&self, o: &Overseer) -> (Sense, f64, Vec<(Ambition, f64)>) {
        let level = if o.dwarf.is_some() { ((50 - o.faith) as f64 / 50.0).clamp(0.0, 1.0) } else { 0.0 };
        (Sense::Doubt, level, vec![(Ambition::Revel, 1.2), (Ambition::Provision, 0.8), (Ambition::Venture, -1.0), (Ambition::Architect, -0.8)])
    }

    /// The chronicle's lines for the year's changes of reign.
    pub(super) fn lines_reigns(&self, log: &chronicle::YearLog) -> Vec<String> {
        log.reigns.clone()
    }

    /// For the Autoplay screen: which overseer this is, and its faith.
    pub fn reign_lines(&self) -> Vec<String> {
        let Some(o) = &self.overseer else { return Vec::new() };
        if o.dwarf.is_none() {
            return Vec::new();
        }
        let n = o.reigns.len().max(1);
        let (grumble, defy, depose) = self.faith_marks();
        let mood = if o.faith < depose {
            "they would put it aside"
        } else if o.faith < defy {
            "defiant"
        } else if o.faith < grumble {
            "grumbling"
        } else if o.faith >= 70 {
            "devoted"
        } else {
            "content"
        };
        let mut v = vec![format!("The {} overseer of {} · faith {} ({})", ordinal(n), self.fortress_name, o.faith, mood)];
        let why: Vec<String> = o.faith_log.iter().rev().take(3).map(|l| format!("{:+} {}", l.1, l.2)).collect();
        if !why.is_empty() {
            v.push(format!("  lately: {}", why.join(" · ")));
        }
        for r in o.reigns.iter().rev().skip(1).take(2) {
            let end = match &r.end {
                Some(ReignEnd::Died(c)) => past(c),
                Some(ReignEnd::Mad) => "went mad".into(),
                Some(ReignEnd::Lost) => "was lost".into(),
                Some(ReignEnd::Deposed) => "was deposed".into(),
                Some(ReignEnd::Elected) => "lost the election".into(),
                None => "stepped aside".into(),
            };
            v.push(format!("  before: {}, who {}", r.styled(), end));
        }
        v
    }

    /// At the end of a trial (`m2_stats`): faith at the end.
    pub(super) fn reign_stats(&mut self) {
        if let Some(f) = self.overseer.as_ref().filter(|o| o.dwarf.is_some()).map(|o| o.faith) {
            self.count("m2.faith_end", f as u32);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::overseer::Style;

    fn fort(seed: u64) -> Game {
        let mut g = Game::new(seed);
        g.creatures.retain(|_, c| c.tame);
        g.set_overseer(Some(Style::Steady));
        g
    }

    fn overseer_dwarf(g: &Game) -> u32 {
        g.overseer.as_ref().and_then(|o| o.dwarf).expect("the overseer is a dwarf")
    }

    #[test]
    fn the_overseer_is_a_dwarf_of_the_fort() {
        let g = fort(5);
        let id = overseer_dwarf(&g);
        let o = g.overseer.as_ref().unwrap();
        let d = g.dwarves.iter().find(|d| d.id == id).unwrap();
        assert_eq!(o.name, d.name);
        assert_eq!(o.reigns.len(), 1);
        assert_eq!(o.traits.len(), 2);
    }

    #[test]
    fn an_overseer_killed_in_a_siege_is_succeeded() {
        let mut g = fort(5);
        for _ in 0..DAY {
            g.step();
        }
        g.overseer.as_mut().unwrap().traits = vec![Trait::Bold, Trait::Vain];
        g.note_siege("goblins", 12, "Snaga Ironmaw");
        let (id, name) = {
            let o = g.overseer.as_ref().unwrap();
            (o.dwarf.unwrap(), o.name.clone())
        };
        let i = g.dwarves.iter().position(|d| d.id == id).unwrap();
        let mut d = std::mem::take(&mut g.dwarves[i]);
        g.kill_for_test(&mut d, "has been struck down");
        g.dwarves[i] = d;
        assert!(g.ov_fell.is_some());
        g.oversee_alert();
        let o = g.overseer.as_ref().unwrap();
        assert_ne!(o.dwarf, Some(id));
        assert_ne!(o.name, name);
        assert_ne!(o.traits, vec![Trait::Bold, Trait::Vain]);
        assert_eq!(o.reigns.len(), 2);
        assert!(matches!(o.reigns[0].end, Some(ReignEnd::Died(_))), "{:?}", o.reigns[0]);
        let heir = g.dwarves.iter().find(|d| Some(d.id) == o.dwarf).unwrap();
        assert!(heir.alive && heir.name == o.name);
        assert!(g.spotlights.iter().any(|s| s.title == "The overseer has fallen"));
        assert!(g.memory.events.iter().any(|m| m.kind == super::super::memory::MemoryKind::Overseer));
        g.tick = YEAR;
        g.write_chronicle_for_test();
        let c = g.chronicles.last().unwrap();
        assert!(c.lines.iter().any(|l| l.contains("reign") && l.contains("second overseer")), "{:?}", c.lines);
    }

    #[test]
    fn deaths_in_the_shelter_cost_faith_and_a_rout_restores_it() {
        let mut g = fort(6);
        let mut o = g.overseer.take().unwrap();
        g.tick = 10 * DAY + 1;
        g.ov_reign(&mut o);
        let start = o.faith;
        let note = |g: &mut Game, sheltered: bool| {
            g.annals.deaths.push(annals::DeathNote { tick: g.tick - 10, name: "Somebody".into(), cause: "struck down".into(), sheltered, ..Default::default() });
        };
        g.tick += DAY;
        note(&mut g, true);
        g.ov_reign(&mut o);
        assert_eq!(o.faith, start - 6, "{:?}", o.faith_log);
        assert!(o.faith_log.last().unwrap().2.contains("broken promise"));
        g.tick += DAY;
        note(&mut g, false);
        g.ov_reign(&mut o);
        assert_eq!(o.faith, start - 9, "{:?}", o.faith_log);
        g.note_siege("goblins", 10, "Snaga");
        let t = g.tick;
        if let Some(s) = g.annals.sieges.last_mut() {
            s.ended = Some(t);
            s.outcome = "routed".into();
        }
        g.tick += DAY;
        g.ov_reign(&mut o);
        assert!(o.faith >= start - 1, "{} {:?}", o.faith, o.faith_log);
        assert!(o.faith_log.iter().any(|l| l.2.contains("routed")));
    }

    #[test]
    fn an_overseer_without_faith_is_deposed() {
        let mut g = fort(7);
        let mut o = g.overseer.take().unwrap();
        let id = o.dwarf.unwrap();
        g.tick = 10 * DAY + 1;
        g.ov_reign(&mut o);
        let mut days = 0;
        while o.dwarf == Some(id) && days < 30 {
            o.faith = o.faith.min(5);
            g.tick += DAY;
            g.ov_reign(&mut o);
            days += 1;
        }
        assert!((10..=12).contains(&days), "deposed after {} days", days);
        assert_ne!(o.dwarf, Some(id));
        assert_eq!(o.reigns[0].end, Some(ReignEnd::Deposed));
        assert_eq!(o.faith, 55);
        // The doubted reign is a saga, ended in grief.
        assert!(g.sagas.list.iter().any(|s| s.kind == super::super::sagas::SagaKind::Reign && !s.open() && s.doom), "{:?}", g.sagas.list);
        assert!(g.spotlights.iter().any(|s| s.title == "The overseer is deposed"));
        // Grumbling came first.
        assert!(g.dwarves.iter().any(|d| d.thoughts.iter().any(|t| t.text.contains("grumbled about the overseer"))));
    }

    #[test]
    fn a_trusted_overseer_wins_the_election_and_a_doubted_one_loses_it() {
        let mut g = fort(8);
        let id = overseer_dwarf(&g);
        let other = g.dwarves.iter().find(|d| d.id != id && !d.is_child()).unwrap().id;
        assert_eq!(g.reign_election(Some(other)), Some(id));
        g.overseer.as_mut().unwrap().faith = 30;
        assert_eq!(g.reign_election(Some(other)), Some(other));
        let o = g.overseer.as_ref().unwrap();
        assert_eq!(o.dwarf, Some(other));
        assert_eq!(o.reigns[0].end, Some(ReignEnd::Elected));
    }

    #[test]
    fn heirs_get_their_traits_from_who_they_are() {
        let p = crate::dwarf::Personality { anxiety: 40, sociability: 30, temper: 0, diligence: 0, artistry: 0 };
        assert_eq!(traits_of(&p, 1), vec![Trait::Cautious, Trait::Cheerful]);
        let p = crate::dwarf::Personality { anxiety: -40, sociability: 0, temper: 0, diligence: 0, artistry: 0 };
        for s in 0..20 {
            let t = traits_of(&p, s);
            assert_eq!(t.len(), 2);
            assert_eq!(t[0], Trait::Bold);
            assert_ne!(t[1], Trait::Cautious);
        }
    }
}
