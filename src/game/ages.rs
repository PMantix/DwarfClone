//! Ages named by the world (O09, WLD-13): the world's history falls into
//! named eras, "the Age of Ash and Iron", "the Age of Dragons", and a new
//! one begins when the world changes: wars give way to quiet, the great
//! beasts multiply or are slain, one people comes to hold half the world, or
//! a fortress grows so storied that the age takes its name.
//!
//! The ages of history before the embark are read from its events; in play
//! the turn of each year looks at the last ten years and at the world as it
//! stands. Names come from hashes, never the game's random numbers.

use super::*;
use crate::overworld::Overworld;
use serde::{Deserialize, Serialize};

/// One age of the world.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Age {
    /// The first year of the age.
    pub from: i32,
    /// "the Age of Ash and Iron".
    pub name: String,
    /// What began it, as the legends tell it.
    #[serde(default)]
    pub why: String,
    /// What the age is of ("war", "beasts", "civ:3", "fort" ...), so a
    /// new age is only called when the world has really changed.
    #[serde(default)]
    pub key: String,
}

/// Years of history to a stretch when ages are read from the past.
const STRETCH: i32 = 25;
/// Years looked back over at each new year in play.
const LOOKBACK: i32 = 10;
/// An age lasts at least this many years before the next is called.
const MIN_YEARS: i32 = 3;

const WAR: [&str; 6] = ["Ash and Iron", "Broken Banners", "Red Spears", "Iron and Blood", "Burning Gates", "the Long War"];
const BEASTS: [&str; 5] = ["Wyrms", "Fire and Claw", "the Burning Sky", "Teeth", "the Great Beasts"];
const FOUNDING: [&str; 5] = ["Hearths", "First Stones", "New Roofs", "Founding", "Open Roads"];
const CRAFT: [&str; 4] = ["Gold and Song", "the Anvil", "Bright Works", "Silver Hands"];
const SHADOW: [&str; 4] = ["Shadow", "the Black Tower", "Twisted Flesh", "Whispers"];
const HEROES: [&str; 4] = ["Heroes", "Crowns", "Bold Deeds", "Champions"];
const PEACE: [&str; 4] = ["the Long Quiet", "Still Waters", "Plenty", "Peace"];

/// What an event of the legends is about, by the shape of its sentence.
pub fn theme_of(text: &str) -> Option<&'static str> {
    let t = text;
    if t.contains("Tower of") || t.contains("twisted") || t.contains("raised the dead") || t.contains("necromancer") || t.contains("the dead walked") {
        Some("shadow")
    } else if t.contains("laid waste") || t.contains("and was driven off") || t.contains("fire-breath") {
        Some("beasts")
    } else if t.contains("razed") || t.contains("conquered") || t.contains("declared war") || t.contains("broke upon") || t.contains("repulsed") || t.contains("fell defending") || t.contains("fell upon") || t.contains("swore war") || t.contains("Battle of") || t.contains("Siege of") || t.contains("Massacre") || t.contains("Rout") {
        Some("war")
    } else if t.contains(" slew ") || t.contains(" became ") {
        Some("heroes")
    } else if t.contains("founded") || t.contains("bred more") {
        Some("founding")
    } else if t.contains(" created ") || t.contains("masterwork") || t.contains("artifact") || t.contains("monument") || t.contains("dedicated") {
        Some("craft")
    } else if t.contains("made peace") || t.contains("bought peace") {
        Some("peace")
    } else {
        None
    }
}

fn words_of(key: &str) -> &'static [&'static str] {
    match key {
        "war" => &WAR,
        "beasts" => &BEASTS,
        "founding" => &FOUNDING,
        "craft" => &CRAFT,
        "shadow" => &SHADOW,
        "heroes" => &HEROES,
        _ => &PEACE,
    }
}

/// The great beasts still living, and how many of them are dragons.
fn beasts_alive(ow: &Overworld) -> (usize, usize) {
    let alive: Vec<&crate::overworld::Figure> = ow.figures.iter().filter(|f| f.lair.is_some() && f.alive()).collect();
    (alive.len(), alive.iter().filter(|f| f.kind == "dragon").count())
}

/// The living people holding at least half of the world's standing sites.
fn dominant(ow: &Overworld) -> Option<u32> {
    let standing: Vec<u32> = ow.sites.iter().filter(|s| !s.ruined).filter_map(|s| s.civ).collect();
    if standing.len() < 4 {
        return None;
    }
    ow.civs.iter().filter(|c| c.alive).find(|c| standing.iter().filter(|&&s| s == c.id).count() * 2 >= standing.len()).map(|c| c.id)
}

/// The fortress, for the age that may take its name.
pub struct FortState<'a> {
    pub name: &'a str,
    /// Its legends, all told.
    pub legends: usize,
    /// Whether its great monument stands.
    pub monument: bool,
}

/// The age the world is in, judged from `events` (indices into
/// `ow.events`) and, if `now`, the world as it stands: (key, score, the
/// event that most speaks for it).
fn judge(ow: &Overworld, events: &[usize], now: Option<&FortState>) -> Vec<(String, i32, String)> {
    let mut score: Vec<(String, i32, String)> = Vec::new();
    let mut add = |key: &str, n: i32, why: &str| match score.iter_mut().find(|s| s.0 == key) {
        Some(s) => s.1 += n,
        None => score.push((key.to_string(), n, why.to_string())),
    };
    for &i in events {
        let e = &ow.events[i];
        if e.bard {
            continue;
        }
        if let Some(k) = theme_of(&e.text) {
            add(k, 1, &e.text);
        }
    }
    if let Some(f) = now {
        let (beasts, dragons) = beasts_alive(ow);
        if dragons >= 3 {
            add("dragons", 4, &format!("{} dragons lived in the world at once.", dragons));
        } else if beasts >= 3 {
            add("beasts", 4, &format!("{} great beasts lived in the world at once.", beasts));
        }
        if let Some(c) = dominant(ow) {
            add(&format!("civ:{}", c), 4, &format!("The {} held half the world.", ow.civ(c).name));
        }
        if f.legends >= 30 && f.monument {
            add("fort", 50, &format!("{} grew so great that the world spoke its name.", f.name));
        }
    }
    score.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    score
}

/// The name of an age of `key` beginning in `from`.
fn name_for(ow: &Overworld, key: &str, from: i32, fort: &str, before: &[Age]) -> String {
    let base = if key == "fort" {
        format!("the Age of {}", fort)
    } else if key == "dragons" {
        "the Age of Dragons".to_string()
    } else if let Some(c) = key.strip_prefix("civ:").and_then(|c| c.parse::<u32>().ok()) {
        format!("the Age of the {}", ow.civ(c).name)
    } else {
        let h = super::names::hash(&[ow.seed, from as u64, stories::hash(key, 13)]);
        format!("the Age of {}", stories::pick(h, 1, words_of(key)))
    };
    // A second age of the same name is told as such.
    let again = before.iter().filter(|a| a.name == base || a.name.ends_with(&base[4..])).count();
    if again == 0 {
        base
    } else {
        const ORD: [&str; 4] = ["Second", "Third", "Fourth", "Fifth"];
        format!("the {} {}", ORD[(again - 1).min(3)], &base[4..])
    }
}

/// The ages of `ow`: as stored, or read from its history if none are.
pub fn ages_of(ow: &Overworld) -> Vec<Age> {
    if ow.ages.is_empty() {
        derive(ow)
    } else {
        ow.ages.clone()
    }
}

/// The age the world is in now, by name.
pub fn current_age(ow: &Overworld) -> Option<String> {
    ages_of(ow).last().map(|a| a.name.clone())
}

/// Read the ages from the history: stretches of `STRETCH` years, each
/// named for what most of its legends tell of, with like stretches joined.
pub fn derive(ow: &Overworld) -> Vec<Age> {
    let Some(first) = ow.events.iter().map(|e| e.year).min() else { return Vec::new() };
    let last = ow.events.iter().map(|e| e.year).max().unwrap_or(first).max(ow.year);
    let mut ages: Vec<Age> = Vec::new();
    let mut y = first;
    while y <= last {
        let inside: Vec<usize> = (0..ow.events.len()).filter(|&i| ow.events[i].year >= y && ow.events[i].year < y + STRETCH).collect();
        let score = judge(ow, &inside, None);
        let (key, why) = match score.first() {
            Some((k, n, w)) if *n >= 3 => (k.clone(), w.clone()),
            _ => ("peace".to_string(), String::new()),
        };
        if ages.last().map_or(true, |a| a.key != key) {
            let name = name_for(ow, &key, y, "", &ages);
            ages.push(Age { from: y, name, why, key });
        }
        y += STRETCH;
    }
    ages
}

impl Game {
    /// Whether the fortress's great monument stands.
    fn monument_stands(&self) -> bool {
        self.monument.is_some()
    }

    /// At the turn of the year: does the world call a new age? Returns it.
    pub(super) fn ages_new_year(&mut self) -> Option<Age> {
        let year = self.start_year + (self.tick / YEAR) as i32;
        if self.overworld.ages.is_empty() {
            self.overworld.ages = derive(&self.overworld);
        }
        let fort = self.fortress_name.clone();
        let legends = self.overworld.events.iter().filter(|e| e.text.contains(&fort)).count();
        let state = FortState { name: &fort, legends, monument: self.monument_stands() };
        let recent: Vec<usize> = (0..self.overworld.events.len()).filter(|&i| self.overworld.events[i].year >= year - LOOKBACK).collect();
        let score = judge(&self.overworld, &recent, Some(&state));
        let (key, n, why) = score.first().cloned()?;
        let cur = self.overworld.ages.last().cloned().unwrap_or_default();
        if key == cur.key || n < 3 || (key != "fort" && year - cur.from < MIN_YEARS) {
            return None;
        }
        // The age holds while what it is of holds as strongly.
        let held = score.iter().find(|s| s.0 == cur.key).map_or(0, |s| s.1);
        if key != "fort" && n <= held {
            return None;
        }
        let name = name_for(&self.overworld, &key, year, &fort, &self.overworld.ages);
        let age = Age { from: year, name, why, key };
        let old = cur.name;
        let text = if old.is_empty() {
            format!("The world began to speak of {}.", age.name)
        } else {
            format!("{} passed, and {} began.", stories_cap(&old), age.name)
        };
        self.legend(text.clone());
        self.msg(&format!("{} {}", text, age.why), (230, 200, 120));
        self.overworld.ages.push(age.clone());
        self.count("o09.ages", 1);
        Some(age)
    }
}

fn stories_cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dragon(g: &mut Game, name: &str) {
        let id = g.overworld.figures.len() as u32;
        g.overworld.figures.push(crate::overworld::Figure {
            id,
            name: name.into(),
            kind: "dragon".into(),
            civ: None,
            born: 0,
            died: None,
            cause: None,
            title: None,
            kills: 0,
            lair: Some(0),
        });
    }

    #[test]
    fn the_history_falls_into_named_ages() {
        let g = Game::new(81);
        let ages = derive(&g.overworld);
        assert!(!ages.is_empty());
        assert!(ages.iter().all(|a| a.name.starts_with("the ") && a.name.contains("Age of")), "{:?}", ages);
        assert!(ages.windows(2).all(|w| w[0].from < w[1].from && w[0].key != w[1].key));
        // The same world, the same ages.
        assert_eq!(ages, derive(&Game::new(81).overworld));
    }

    #[test]
    fn three_dragons_make_an_age_of_dragons() {
        let mut g = Game::new(81);
        // The great beasts of the world slain, and quiet for ten years.
        for f in g.overworld.figures.iter_mut().filter(|f| f.lair.is_some()) {
            f.died = Some(1);
        }
        g.tick = 20 * YEAR;
        g.overworld.ages = vec![Age { from: 0, name: "the Age of Peace".into(), why: String::new(), key: "peace".into() }];
        assert!(g.ages_new_year().is_none(), "nothing has changed");
        for n in ["Ashmaw", "Gloomwing", "Cinderjaw"] {
            dragon(&mut g, n);
        }
        let age = g.ages_new_year().expect("a new age");
        assert_eq!(age.name, "the Age of Dragons");
        assert_eq!(current_age(&g.overworld).as_deref(), Some("the Age of Dragons"));
        assert!(g.overworld.events.last().unwrap().text.contains("the Age of Dragons began"));
        // Called once: the next year it simply goes on.
        g.tick += YEAR;
        assert!(g.ages_new_year().is_none());
    }
}
