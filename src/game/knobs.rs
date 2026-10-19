//! Knobs and counters: the batch's measuring tape.
//!
//! A knob is a tunable number declared where it's used,
//! `self.knob("wildlife.chance", 0.18)`, so there's no shared list to edit.
//! It's the default unless the batch was run with `--knob name=value`.
//! Counters (`self.count("rescues", 1)`) go in the annals and are summed
//! over the forts in the batch report.

use super::*;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

/// The knobs a batch changed from their defaults (only what the command line set).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Knobs {
    pub set: BTreeMap<String, f64>,
}

impl Knobs {
    /// "name=v name2=v2", for the report header; empty when nothing was set.
    pub fn describe(&self) -> String {
        self.set.iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>().join(" ")
    }
}

thread_local! {
    /// Every knob read on this thread, with its default (for `--list-knobs`).
    static SEEN: RefCell<BTreeMap<&'static str, f64>> = RefCell::new(BTreeMap::new());
}

/// The knobs read so far on this thread, name → default.
pub fn seen() -> BTreeMap<&'static str, f64> {
    SEEN.with(|s| s.borrow().clone())
}

/// "bleed.gash=6" → ("bleed.gash", 6.0).
pub fn parse_knob(arg: &str) -> Result<(String, f64), String> {
    let (name, v) = arg.split_once('=').ok_or_else(|| format!("a knob is name=value, not \"{}\"", arg))?;
    let name = name.trim();
    if name.is_empty() {
        return Err(format!("a knob needs a name: \"{}\"", arg));
    }
    let v: f64 = v.trim().parse().map_err(|_| format!("knob {} needs a number, not \"{}\"", name, v.trim()))?;
    Ok((name.to_string(), v))
}

/// A flat JSON map of knobs, `{"bleed.gash": 6, "siege.cap": 12}`.
pub fn parse_knobs_file(text: &str) -> Result<BTreeMap<String, f64>, String> {
    serde_json::from_str(text).map_err(|e| format!("a knobs file is a flat {{name: number}} map: {}", e))
}

impl Game {
    /// A tunable number: `default` unless the batch was run with `--knob name=v`.
    pub fn knob(&self, name: &'static str, default: f64) -> f64 {
        SEEN.with(|s| {
            let mut s = s.borrow_mut();
            if !s.contains_key(name) {
                s.insert(name, default);
            }
        });
        self.knobs.set.get(name).copied().unwrap_or(default)
    }

    /// Add `n` to a counter for the batch report.
    pub(crate) fn count(&mut self, key: &str, n: u32) {
        let c = &mut self.annals.counters;
        match c.get_mut(key) {
            Some(v) => *v = v.saturating_add(n),
            None => {
                c.insert(key.to_string(), n);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_knob_changes_the_game() {
        // The same fort, wildlife certain and wildlife never, to the middle of day one.
        let run = |chance: f64| {
            let mut g = Game::new(11);
            g.knobs.set.insert("wildlife.chance".into(), chance);
            let before = g.creatures.len();
            while g.tick <= DAY / 2 {
                g.step();
            }
            (g.annals.counters.get("wildlife.visits").copied().unwrap_or(0), g.creatures.len() > before)
        };
        let (visits, spawned) = run(1.0);
        assert_eq!(visits, 1, "wildlife should come on day one");
        assert!(spawned, "animals should have been spawned");
        assert_eq!(run(0.0).0, 0, "and never with the chance at zero");
        assert!(seen().get("wildlife.chance") == Some(&0.18), "the knob and its default are listed");
    }

    #[test]
    fn siege_size_cap_is_a_knob() {
        // A fort that has broken many sieges draws big armies...
        let mut g = Game::new(21);
        for i in 0..12 {
            g.annals.sieges.push(annals::SiegeNote { tick: i, foe: "goblins".into(), size: 10, outcome: "routed".into(), ..Default::default() });
        }
        let civ = g.neighbors.first().map_or(0, |n| n.civ);
        assert!((0..20).map(|_| g.siege_strength(civ)).max().unwrap() > 4, "a dozen broken sieges bring more than four");
        // ...unless the batch caps them.
        g.knobs.set.insert("siege.cap".into(), 4.0);
        for _ in 0..20 {
            assert!(g.siege_strength(civ) <= 4);
        }
        // And the first raid's size is a knob too.
        let mut g = Game::new(21);
        g.knobs.set.insert("siege.first_min".into(), 5.0);
        g.knobs.set.insert("siege.first_max".into(), 5.0);
        assert_eq!(g.siege_strength(civ), 5);
    }

    #[test]
    fn knobs_parse() {
        assert_eq!(parse_knob("bleed.gash=6"), Ok(("bleed.gash".into(), 6.0)));
        assert_eq!(parse_knob(" siege.cap = 4.5 "), Ok(("siege.cap".into(), 4.5)));
        assert!(parse_knob("bleed.gash").is_err() && parse_knob("x=lots").is_err() && parse_knob("=3").is_err());
        let f = parse_knobs_file("{\"a.b\": 2, \"c\": 0.5}").unwrap();
        assert_eq!(f.get("a.b"), Some(&2.0));
        let mut k = Knobs::default();
        k.set.extend(f);
        assert_eq!(k.describe(), "a.b=2 c=0.5");
        // A knob nobody set is its default.
        let g = Game::placeholder(crate::overworld::Overworld::generate(3));
        assert_eq!(g.knob("test.unset", 1.25), 1.25);
    }
}
