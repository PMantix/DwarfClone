//! History as the creation show tells it: not every raid and coronation
//! but the ages of the world, each with the theme that ran through it
//! ("the dwarves of the Anvil and the goblins of Smoke war for forty
//! years"), its great deeds picked out and the small ones folded into
//! counts. The ages are the legends' own (`game::ages::derive`), so the
//! names the show gives them are the names the world remembers.

use crate::game::ages::theme_of;
use crate::overworld::{Clash, ClashEnd, Overworld};

/// One age of history, as the show tells it.
#[derive(Clone, Debug, PartialEq)]
pub struct Era {
    /// First and last year (inclusive).
    pub from: i32,
    pub to: i32,
    /// "The Age of Red Spears".
    pub name: String,
    /// What ran through it, a sentence.
    pub theme: String,
}

/// Small deeds of an age, counted rather than told.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    pub battles: usize,
    pub razed: usize,
    pub founded: usize,
    pub crowned: usize,
    pub works: usize,
    pub slain: usize,
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

fn years(n: i32) -> String {
    match n {
        n if n <= 1 => "a year".to_string(),
        n if n >= 95 && n <= 110 => "a century".to_string(),
        n if n > 110 => format!("{} years", n),
        n if n % 10 == 0 => format!("{} years", ["", "ten", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"][(n / 10) as usize]),
        n => format!("{} years", n),
    }
}

/// Whether an event is one the show tells in full: the end of a people, a
/// beast slain or a town laid waste by one, a conquest, the shadow's deeds.
/// Wars declared and ended, battles, foundings and crownings are counted.
pub fn notable(text: &str) -> bool {
    text.contains("vanished from the world")
        || text.contains(" slew ")
        || text.contains("laid waste")
        || text.contains("conquered")
        || text.contains("swore war")
        || text.contains("raised the Tower")
        || text.contains("turned on their maker")
}

/// The small deeds among `events` with years in `from..=to`, and the clashes.
pub fn tally(ow: &Overworld, clashes: &[Clash], from: i32, to: i32) -> Tally {
    let mut t = Tally::default();
    for e in ow.events.iter().filter(|e| e.year >= from && e.year <= to && !e.bard) {
        let s = &e.text;
        if s.contains(" founded ") || s.contains("bred more") {
            t.founded += 1;
        }
        if s.contains(" became ") {
            t.crowned += 1;
        }
        if s.contains(" created ") {
            t.works += 1;
        }
        if s.contains(" slew ") {
            t.slain += 1;
        }
    }
    for c in clashes.iter().filter(|c| c.year >= from && c.year <= to) {
        t.battles += 1;
        if matches!(c.end, ClashEnd::Fell | ClashEnd::Taken) {
            t.razed += 1;
        }
    }
    t
}

impl Tally {
    /// "12 battles · 3 razed · 5 founded", leaving out the zeros.
    pub fn line(&self) -> String {
        let mut v = Vec::new();
        for (n, one, many) in [
            (self.battles, "battle", "battles"),
            (self.razed, "fallen", "fallen"),
            (self.founded, "founding", "foundings"),
            (self.crowned, "ruler crowned", "rulers crowned"),
            (self.works, "masterwork", "masterworks"),
            (self.slain, "beast slain", "beasts slain"),
        ] {
            if n > 0 {
                v.push(format!("{} {}", n, if n == 1 { one } else { many }));
            }
        }
        v.join(" · ")
    }
}

/// The people a site belongs to, as "the dwarves of the Kingdom of the Anvil".
fn people_of(ow: &Overworld, civ: u32) -> String {
    format!("the {} of the {}", ow.people(civ), ow.civ(civ).name)
}

/// The sentence for an age of `key` over `from..=to`.
fn theme(ow: &Overworld, clashes: &[Clash], key: &str, from: i32, to: i32) -> String {
    let span = years(to - from + 1);
    let t = tally(ow, clashes, from, to);
    match key {
        "war" | "heroes" if t.battles > 0 => {
            // The two peoples who fought most.
            let mut pairs: Vec<((u32, u32), usize)> = Vec::new();
            for c in clashes.iter().filter(|c| c.year >= from && c.year <= to) {
                let (Some(a), Some(b)) = (ow.sites.get(c.from as usize).and_then(|s| s.civ), ow.sites.get(c.to as usize).and_then(|s| s.civ)) else { continue };
                if a == b {
                    continue;
                }
                let k = (a.min(b), a.max(b));
                match pairs.iter_mut().find(|p| p.0 == k) {
                    Some(p) => p.1 += 1,
                    None => pairs.push((k, 1)),
                }
            }
            pairs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            match pairs.first() {
                Some(&((a, b), n)) if key == "war" => format!(
                    "{} and {} war for {}: {} {} between them, {} {} fallen in all.",
                    cap(&people_of(ow, a)),
                    people_of(ow, b),
                    span,
                    n,
                    if n == 1 { "battle" } else { "battles" },
                    t.razed,
                    if t.razed == 1 { "place" } else { "places" }
                ),
                _ => format!("An age of heroes and new crowns: {} beasts slain, {} rulers crowned, {} battles fought.", t.slain, t.crowned, t.battles),
            }
        }
        "beasts" | "dragons" => {
            let worst = ow
                .figures
                .iter()
                .filter(|f| f.lair.is_some())
                .map(|f| (ow.events.iter().filter(|e| e.year >= from && e.year <= to && e.text.starts_with(&f.name) && e.text.contains("laid waste")).count(), f))
                .filter(|(n, _)| *n > 0)
                .max_by_key(|(n, f)| (*n, std::cmp::Reverse(f.id)));
            match worst {
                Some((n, f)) => format!("Great beasts stalk the land for {}: {} the {} alone lays waste to {} {}.", span, f.name, f.kind, n, if n == 1 { "place" } else { "places" }),
                None => format!("Great beasts stalk the land for {}, and the peoples hide behind their walls.", span),
            }
        }
        "founding" => {
            let mut by: Vec<(u32, usize)> = Vec::new();
            for s in ow.sites.iter().filter(|s| s.founded >= from && s.founded <= to) {
                if let Some(c) = s.civ {
                    match by.iter_mut().find(|b| b.0 == c) {
                        Some(b) => b.1 += 1,
                        None => by.push((c, 1)),
                    }
                }
            }
            by.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            match by.first() {
                Some(&(c, _)) => format!("The peoples spread across the land for {}: {} new settlements, {} foremost among the builders.", span, t.founded, people_of(ow, c)),
                None => format!("New hearths are lit across the land for {}.", span),
            }
        }
        "craft" => format!("An age of making: {} masterworks are wrought in {}.", t.works, span),
        "shadow" => format!("A shadow falls on the world for {}: a tower rises, and things are made that should not be.", span),
        "fort" => "A fortress grows so great the world speaks its name.".to_string(),
        k if k.starts_with("civ:") => {
            let c = k[4..].parse::<u32>().unwrap_or(0);
            format!("{} hold half the world for {}.", cap(&people_of(ow, c)), span)
        }
        _ => format!("{} of quiet: the peoples grow and the roads stay open.", cap(&span)),
    }
}

/// The ages of `ow`'s history up to `end`, each with its theme.
pub fn eras(ow: &Overworld, clashes: &[Clash], end: i32) -> Vec<Era> {
    let ages = crate::game::ages::derive(ow);
    let mut out: Vec<Era> = Vec::new();
    for (k, a) in ages.iter().enumerate() {
        let from = if k == 0 { 0 } else { a.from.max(0) };
        let to = ages.get(k + 1).map_or(end, |n| n.from - 1).min(end);
        if to < from {
            continue;
        }
        // An age of a few years at the end is part of the one before.
        if to - from < 5 && to == end {
            if let Some(last) = out.last_mut() {
                last.to = to;
                continue;
            }
        }
        out.push(Era { from, to, name: cap(&a.name), theme: theme(ow, clashes, &a.key, from, to) });
    }
    if out.is_empty() {
        out.push(Era { from: 0, to: end, name: "The First Age".into(), theme: theme(ow, clashes, "peace", 0, end) });
    }
    out
}

/// The notable events of `from..=to` that have happened by `year`, oldest
/// first, and how many lesser ones there were.
pub fn deeds(ow: &Overworld, from: i32, to: i32, year: i32) -> (Vec<(i32, String)>, usize) {
    let mut great = Vec::new();
    let mut lesser = 0;
    for e in ow.events.iter().filter(|e| e.year >= from && e.year <= to.min(year) && !e.bard) {
        if ow.epics.iter().any(|p| p.year == e.year && p.text == e.text) {
            continue;
        }
        if notable(&e.text) {
            great.push((e.year, e.text.clone()));
        } else if theme_of(&e.text).is_some() || e.text.contains(" founded ") {
            lesser += 1;
        }
    }
    (great, lesser)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overworld::{WorldGen, WorldParams};

    /// The ages cover the history end to end, each says what it was about,
    /// and the show tells far fewer deeds than history holds.
    #[test]
    fn history_is_told_in_ages_with_their_themes() {
        for seed in [3u64, 17, 40] {
            let mut g = WorldGen::traced(WorldParams { seed, history: 200, ..WorldParams::default() });
            while !g.step(usize::MAX, 50) {}
            let clashes = g.trace.as_ref().unwrap().clashes.clone();
            let ow = &g.ow;
            let eras = eras(ow, &clashes, 200);
            assert_eq!(eras[0].from, 0);
            assert_eq!(eras.last().unwrap().to, 200);
            for w in eras.windows(2) {
                assert_eq!(w[0].to + 1, w[1].from, "seed {}: ages leave a gap", seed);
            }
            assert!(!eras.is_empty() && eras.len() <= 12, "seed {}: {} ages", seed, eras.len());
            let told: usize = eras.iter().map(|e| deeds(ow, e.from, e.to, 200).0.len()).sum();
            eprintln!("seed {}: {} events, {} told in full", seed, ow.events.len(), told);
            let mut all: Vec<&str> = Vec::new();
            for e in &eras {
                for (_, t) in deeds(ow, e.from, e.to, 200).0 {
                    all.push(["vanished", " slew ", "laid waste", "conquered", "swore war", "Tower", "maker"].iter().copied().find(|k| t.contains(k)).unwrap_or("?"));
                }
            }
            all.sort();
            eprintln!("  told: {:?}", all);
            for e in &eras {
                eprintln!("  {} ({}-{}): {}", e.name, e.from, e.to, e.theme);
                assert!(e.name.starts_with("The "), "{}", e.name);
                assert!(e.theme.len() > 20 && !e.theme.contains("{}"));
            }
            assert!(told * 3 < ow.events.len(), "seed {}: {} of {} told", seed, told, ow.events.len());
        }
    }
}
