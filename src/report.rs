//! The autoplay batch's report: why forts fall. Survival year by year,
//! deaths by cause and by killer, where dwarves died, and how sieges went;
//! and the same set side by side for two batches (`--compare`).

use crate::game::annals::{DeathNote, SiegeNote};
use crate::game::{Style, Trial};
use std::collections::BTreeMap;
use std::fmt::Write;

fn pct(n: usize, of: usize) -> String {
    if of == 0 {
        "-".into()
    } else {
        format!("{:.0}%", 100.0 * n as f64 / of as f64)
    }
}

/// Forts still standing at the end of each year, 1..=years.
pub fn survival(trials: &[Trial], years: usize) -> Vec<usize> {
    (1..=years).map(|y| trials.iter().filter(|t| t.alive_by_year.get(y - 1).map_or(false, |&a| a > 0)).count()).collect()
}

/// Deaths counted by `key`, most first.
fn tally<'a>(deaths: impl Iterator<Item = &'a DeathNote>, key: impl Fn(&DeathNote) -> Option<String>) -> Vec<(String, usize)> {
    let mut m: BTreeMap<String, usize> = BTreeMap::new();
    for d in deaths {
        if let Some(k) = key(d) {
            *m.entry(k).or_default() += 1;
        }
    }
    let mut v: Vec<(String, usize)> = m.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v
}

fn causes(trials: &[Trial]) -> Vec<(String, usize)> {
    tally(trials.iter().flat_map(|t| t.deaths.iter()), |d| Some(d.cause.clone()))
}

fn killers(trials: &[Trial]) -> Vec<(String, usize)> {
    tally(trials.iter().flat_map(|t| t.deaths.iter()), |d| d.killer.clone())
}

/// Sieges by their order in each fort: the first, the second, the third and later.
fn sieges_by_order(trials: &[Trial]) -> [Vec<&SiegeNote>; 3] {
    let mut out: [Vec<&SiegeNote>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for t in trials {
        for (i, s) in t.sieges.iter().enumerate() {
            out[i.min(2)].push(s);
        }
    }
    out
}

/// Every counter, summed over the forts, by name.
pub fn counter_totals(trials: &[Trial]) -> BTreeMap<String, u64> {
    let mut m: BTreeMap<String, u64> = BTreeMap::new();
    for t in trials {
        for (k, v) in &t.counters {
            *m.entry(k.clone()).or_default() += *v as u64;
        }
    }
    m
}

/// The knobs a batch was run with ("bleed.gash=6 siege.cap=4"), or "defaults".
/// Forts run with different knobs are listed each way.
pub fn knobs_used(trials: &[Trial]) -> String {
    let mut sets: Vec<String> = trials.iter().map(|t| t.knobs.iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>().join(" ")).collect();
    sets.sort();
    sets.dedup();
    let sets: Vec<String> = sets.into_iter().map(|s| if s.is_empty() { "defaults".to_string() } else { s }).collect();
    if sets.is_empty() {
        "defaults".into()
    } else {
        sets.join(" | ")
    }
}

/// The whole report, as text.
pub fn report(trials: &[Trial], years: usize) -> String {
    let mut s = String::new();
    let n = trials.len();
    let deaths: Vec<&DeathNote> = trials.iter().flat_map(|t| t.deaths.iter()).collect();
    let nd = deaths.len();
    if trials.iter().any(|t| !t.knobs.is_empty()) {
        let _ = writeln!(s, "\n  Knobs: {}", knobs_used(trials));
    }

    let _ = writeln!(s, "\n  Forts standing at the end of each year (of {}):", n);
    let surv = survival(trials, years);
    let _ = writeln!(s, "    {}", surv.iter().enumerate().map(|(i, k)| format!("year {}: {} ({})", i + 1, k, pct(*k, n))).collect::<Vec<_>>().join("   "));

    let _ = writeln!(s, "\n  Deaths by cause ({} dwarves):", nd);
    for (c, k) in causes(trials) {
        let mad = deaths.iter().filter(|d| d.cause == c && d.mad).count();
        let sad = deaths.iter().filter(|d| d.cause == c && d.melancholy).count();
        let note = if mad > 0 { format!("  ({} of them mad, {} of those melancholic)", mad, sad) } else { String::new() };
        let _ = writeln!(s, "    {:<22} {:>5}  {:>4}{}", c, k, pct(k, nd), note);
    }
    let ks = killers(trials);
    if !ks.is_empty() {
        let _ = writeln!(s, "\n  Killed by (struck down, or bled from their wounds):");
        for (c, k) in ks.iter().take(10) {
            let _ = writeln!(s, "    {:<22} {:>5}  {:>4}", c, k, pct(*k, nd));
        }
    }
    let during = deaths.iter().filter(|d| d.siege.is_some()).count();
    let _ = writeln!(
        s,
        "\n  Where they died: {} during a siege, {} under the open sky, {} inside the shelter, {} soldiers.",
        pct(during, nd),
        pct(deaths.iter().filter(|d| d.outside).count(), nd),
        pct(deaths.iter().filter(|d| d.sheltered).count(), nd),
        pct(deaths.iter().filter(|d| d.soldier).count(), nd)
    );

    let all: usize = trials.iter().map(|t| t.sieges.len()).sum();
    let _ = writeln!(s, "\n  Sieges ({}):", all);
    let _ = writeln!(s, "    {:<8} {:>5} {:>7} {:>6} {:>8} {:>8} {:>7} {:>7} {:>7}  {}", "", "count", "day", "size", "dwarves", "foes", "caught", "shelter", "bridge", "outcomes");
    for (i, list) in sieges_by_order(trials).iter().enumerate() {
        if list.is_empty() {
            continue;
        }
        let k = list.len() as f64;
        let avg = |f: &dyn Fn(&SiegeNote) -> f64| list.iter().map(|x| f(x)).sum::<f64>() / k;
        let mut outcomes: BTreeMap<&str, usize> = BTreeMap::new();
        for x in list {
            *outcomes.entry(if x.outcome.is_empty() { "unfinished" } else { x.outcome.as_str() }).or_default() += 1;
        }
        let _ = writeln!(
            s,
            "    {:<8} {:>5} {:>7.0} {:>6.1} {:>8.1} {:>8.1} {:>7.1} {:>7} {:>7}  {}",
            ["first", "second", "later"][i],
            list.len(),
            avg(&|x| x.tick as f64 / crate::game::DAY as f64),
            avg(&|x| x.size as f64),
            avg(&|x| x.dwarves_lost as f64),
            avg(&|x| x.foes_slain as f64),
            avg(&|x| x.foes_caught as f64),
            pct(list.iter().filter(|x| x.shelter).count(), list.len()),
            pct(list.iter().filter(|x| x.bridge).count(), list.len()),
            outcomes.iter().map(|(o, c)| format!("{} {}", o, c)).collect::<Vec<_>>().join(", ")
        );
    }

    // Blows in sieges: who struck, how often it drew blood.
    let mut blows = [(0u32, 0u32); 5];
    for t in trials {
        for sg in &t.sieges {
            for i in 0..5 {
                blows[i].0 += sg.blows[i].0;
                blows[i].1 += sg.blows[i].1;
            }
        }
    }
    if blows.iter().any(|b| b.0 > 0) {
        let _ = writeln!(s, "\n  Blows in sieges (struck, drew blood):");
        for (i, what) in ["soldiers' melee", "civilians' melee", "bolts shot", "foes on soldiers", "foes on civilians"].iter().enumerate() {
            let _ = writeln!(s, "    {:<22} {:>6} {:>6}  {:>4}", what, blows[i].0, blows[i].1, pct(blows[i].1 as usize, blows[i].0 as usize));
        }
    }

    // Counters: the total over the batch and the average per fort.
    let counters = counter_totals(trials);
    if !counters.is_empty() {
        let _ = writeln!(s, "\n  Counters (total, per fort):");
        for (k, v) in &counters {
            let _ = writeln!(s, "    {:<22} {:>6} {:>8.1}", k, v, *v as f64 / n.max(1) as f64);
        }
    }

    // Milestone 2: the overseers (the `m2.*` counters, by style).
    s.push_str(&overseers(trials));

    let _ = writeln!(s, "\n  {:<9} {:>5} {:>6} {:>6} {:>8} {:>8} {:>10}", "style", "runs", "glory", "ruin", "standing", "avg yrs", "avg ☼peak");
    for st in Style::ALL {
        let mine: Vec<&Trial> = trials.iter().filter(|t| t.style == st).collect();
        if mine.is_empty() {
            continue;
        }
        let k = mine.len() as f64;
        let _ = writeln!(
            s,
            "  {:<9} {:>5} {:>6} {:>6} {:>8} {:>8.1} {:>10.0}",
            st.name(),
            mine.len(),
            mine.iter().filter(|t| t.verdict == "Glory").count(),
            mine.iter().filter(|t| t.verdict == "Ruin").count(),
            mine.iter().filter(|t| t.alive > 0).count(),
            mine.iter().map(|t| t.years).sum::<f64>() / k,
            mine.iter().map(|t| t.peak_wealth as f64).sum::<f64>() / k
        );
    }
    s
}

/// The "Overseers" section (Milestone 2): each `m2.*` counter (filled by
/// `Game::m2_stats` at the end of a fort) averaged per fort of each style.
/// Empty until an M2 package records one; `--compare` already shows them
/// among the counters.
pub fn overseers(trials: &[Trial]) -> String {
    let mut s = String::new();
    let keys: Vec<String> = counter_totals(trials).into_keys().filter(|k| k.starts_with("m2.")).collect();
    if keys.is_empty() {
        return s;
    }
    let styles: Vec<Style> = Style::ALL.iter().copied().filter(|st| trials.iter().any(|t| t.style == *st)).collect();
    let _ = write!(s, "\n  Overseers (per fort):\n    {:<22}", "");
    for st in &styles {
        let _ = write!(s, " {:>9}", st.name());
    }
    s.push('\n');
    for k in &keys {
        let _ = write!(s, "    {:<22}", &k[3..]);
        for st in &styles {
            let mine: Vec<&Trial> = trials.iter().filter(|t| t.style == *st).collect();
            let sum: u64 = mine.iter().map(|t| t.counters.get(k).copied().unwrap_or(0) as u64).sum();
            let _ = write!(s, " {:>9.1}", sum as f64 / mine.len().max(1) as f64);
        }
        s.push('\n');
    }
    s
}

/// Two batches side by side: survival, causes, sieges, and the forts whose
/// fate changed.
pub fn compare(a: &[Trial], b: &[Trial], years: usize) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "\n  {:<24} {:>12} {:>12}", "", "before", "after");
    if a.iter().chain(b.iter()).any(|t| !t.knobs.is_empty()) {
        let _ = writeln!(s, "  knobs before: {}", knobs_used(a));
        let _ = writeln!(s, "  knobs after:  {}", knobs_used(b));
    }
    let (sa, sb) = (survival(a, years), survival(b, years));
    for y in 0..years {
        let _ = writeln!(s, "  {:<24} {:>12} {:>12}", format!("standing after year {}", y + 1), format!("{}/{}", sa[y], a.len()), format!("{}/{}", sb[y], b.len()));
    }
    let (ca, cb) = (causes(a), causes(b));
    let mut names: Vec<String> = ca.iter().chain(cb.iter()).map(|c| c.0.clone()).collect();
    names.sort();
    names.dedup();
    let get = |v: &[(String, usize)], k: &str| v.iter().find(|c| c.0 == k).map_or(0, |c| c.1);
    let _ = writeln!(s, "\n  Deaths by cause:");
    for k in names {
        let _ = writeln!(s, "  {:<24} {:>12} {:>12}", k, get(&ca, &k), get(&cb, &k));
    }
    let lost = |t: &[Trial], i: usize| {
        let v: Vec<u32> = t.iter().filter_map(|t| t.sieges.get(i)).map(|s| s.dwarves_lost).collect();
        if v.is_empty() {
            "-".to_string()
        } else {
            format!("{:.1}", v.iter().sum::<u32>() as f64 / v.len() as f64)
        }
    };
    let _ = writeln!(s, "\n  {:<24} {:>12} {:>12}", "dwarves lost, 1st siege", lost(a, 0), lost(b, 0));
    let _ = writeln!(s, "  {:<24} {:>12} {:>12}", "dwarves lost, 2nd siege", lost(a, 1), lost(b, 1));
    // Counters, per fort (the batches may differ in size).
    let (na, nb) = (counter_totals(a), counter_totals(b));
    let mut keys: Vec<&String> = na.keys().chain(nb.keys()).collect();
    keys.sort();
    keys.dedup();
    if !keys.is_empty() {
        let per = |m: &BTreeMap<String, u64>, k: &str, n: usize| format!("{:.1}", m.get(k).copied().unwrap_or(0) as f64 / n.max(1) as f64);
        let _ = writeln!(s, "\n  Counters, per fort:");
        for k in keys {
            let _ = writeln!(s, "  {:<24} {:>12} {:>12}", k, per(&na, k, a.len()), per(&nb, k, b.len()));
        }
    }
    let changed: Vec<String> = a
        .iter()
        .filter_map(|x| {
            let y = b.iter().find(|y| y.seed == x.seed && y.style == x.style)?;
            (x.verdict != y.verdict).then(|| format!("    seed {} {}: {} ({:.1} yrs) → {} ({:.1} yrs)", x.seed, x.style.name(), x.verdict, x.years, y.verdict, y.years))
        })
        .collect();
    if !changed.is_empty() {
        let _ = writeln!(s, "\n  Forts whose fate changed:");
        for c in changed {
            let _ = writeln!(s, "{}", c);
        }
    }
    s
}

/// The adventure batch's report (A18): how the adventurers ended, by goal
/// and by leg of the journey, what killed them, and the days, kills, loot
/// and fame of the lot. Given a second batch, the two side by side.
pub fn adventures(a: &[crate::game::adv::bot::AdvTrial], b: Option<&[crate::game::adv::bot::AdvTrial]>) -> String {
    use crate::game::adv::bot::{AdvTrial, Fate, Goal, Leg};
    // Each batch as rows of (label, value); a label of "#…" heads a section.
    let rows = |t: &[AdvTrial]| -> Vec<(String, String)> {
        let mut v: Vec<(String, String)> = Vec::new();
        let mut row = |k: &str, val: String| v.push((k.to_string(), val));
        let n = t.len();
        let of = |f: &dyn Fn(&AdvTrial) -> bool| t.iter().filter(|x| f(x)).count();
        let frac = |k: usize, of: usize| format!("{}/{} {}", k, of, pct(k, of));
        let mean = |f: &dyn Fn(&AdvTrial) -> f64, among: &[&AdvTrial]| if among.is_empty() { "-".to_string() } else { format!("{:.1}", among.iter().map(|x| f(x)).sum::<f64>() / among.len() as f64) };
        let all: Vec<&AdvTrial> = t.iter().collect();
        let dead: Vec<&AdvTrial> = t.iter().filter(|x| x.fate == Fate::Died).collect();
        row("#How they ended", String::new());
        row("came home", frac(of(&|x| x.fate == Fate::Home), n));
        row("died", frac(dead.len(), n));
        row("lost (days ran out)", frac(of(&|x| x.fate == Fate::Lost), n));
        row("#By goal: home / reached / cleared", String::new());
        for g in Goal::ALL {
            let k = of(&|x| x.goal == g);
            if k > 0 {
                let home = of(&|x| x.goal == g && x.fate == Fate::Home);
                let (reached, cleared) = (of(&|x| x.goal == g && x.reached), of(&|x| x.goal == g && x.cleared));
                row(&format!("{} ({} runs)", g.name(), k), format!("{} {} / {} / {}", home, pct(home, k), reached, cleared));
            }
        }
        row("#Where the dead fell", String::new());
        for l in Leg::ALL {
            let k = dead.iter().filter(|x| x.leg == Some(l)).count();
            row(l.words(), format!("{} {}", k, pct(k, dead.len())));
        }
        // "struck down by a goblin in Copper Gate": the cause without the place.
        let mut causes: BTreeMap<String, usize> = BTreeMap::new();
        let mut killers: BTreeMap<String, usize> = BTreeMap::new();
        for x in &dead {
            let c = x.cause.as_deref().unwrap_or("died");
            let c = c.split(" in ").next().unwrap_or(c).split(" by ").next().unwrap_or(c);
            let c = c.split(" on the road").next().unwrap_or(c);
            *causes.entry(c.trim().to_string()).or_default() += 1;
            if let Some(k) = &x.killer {
                *killers.entry(k.clone()).or_default() += 1;
            }
        }
        row("#Causes of death", String::new());
        for (c, k) in causes {
            row(&c, k.to_string());
        }
        if !killers.is_empty() {
            row("#Killed by", String::new());
            for (c, k) in killers {
                row(&c, k.to_string());
            }
        }
        row("#Per adventurer", String::new());
        row("days out", mean(&|x| x.days as f64, &all));
        row("days, the dead", mean(&|x| x.days as f64, &dead));
        row("cells walked", mean(&|x| x.cells as f64, &all));
        row("road fights", mean(&|x| x.fights as f64, &all));
        row("encounters slipped", mean(&|x| x.evaded as f64, &all));
        row("kills", mean(&|x| x.kills as f64, &all));
        row("killed something", frac(of(&|x| x.kills > 0), n));
        let home: Vec<&AdvTrial> = t.iter().filter(|x| x.fate == Fate::Home).collect();
        row("brought loot home", frac(home.iter().filter(|x| !x.loot.is_empty()).count(), n));
        row("loot worth, home", mean(&|x| x.loot_value as f64, &home));
        row("a relic home", frac(home.iter().filter(|x| x.artifact).count(), n));
        row("renown", mean(&|x| x.renown as f64, &all));
        row("deeds", mean(&|x| x.deeds as f64, &all));
        row("epithets earned", of(&|x| x.epithet.is_some()).to_string());
        v
    };
    let knobs = |t: &[AdvTrial]| {
        let mut sets: Vec<String> = t.iter().map(|x| x.knobs.iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<_>>().join(" ")).collect();
        sets.sort();
        sets.dedup();
        sets.into_iter().map(|s| if s.is_empty() { "defaults".to_string() } else { s }).collect::<Vec<_>>().join(" | ")
    };
    let mut s = String::new();
    let line = |s: &mut String, k: &str, a: &str, b: Option<&str>| {
        let _ = match (k.strip_prefix('#'), b) {
            (Some(h), _) => writeln!(s, "\n  {}:", h),
            (None, Some(b)) => writeln!(s, "    {:<34} {:>14} {:>14}", k, a, b),
            (None, None) => writeln!(s, "    {:<34} {:>14}", k, a),
        };
    };
    let ra = rows(a);
    match b {
        None => {
            let _ = writeln!(s, "\n  Adventurers: {} (knobs: {})", a.len(), knobs(a));
            for (k, v) in &ra {
                line(&mut s, k, v, None);
            }
        }
        Some(b) => {
            let rb = rows(b);
            let _ = writeln!(s, "\n    {:<34} {:>14} {:>14}", "", "before", "after");
            let _ = writeln!(s, "    knobs before: {}\n    knobs after:  {}", knobs(a), knobs(b));
            // Every row of either, section by section, the first batch's order first.
            let sections = |r: &[(String, String)]| {
                let mut out: Vec<(String, Vec<String>)> = Vec::new();
                for (k, _) in r {
                    match k.strip_prefix('#') {
                        Some(_) => out.push((k.clone(), Vec::new())),
                        None => {
                            if let Some(last) = out.last_mut() {
                                last.1.push(k.clone());
                            }
                        }
                    }
                }
                out
            };
            let mut merged = sections(&ra);
            for (h, ks) in sections(&rb) {
                match merged.iter().position(|m| m.0 == h) {
                    Some(i) => {
                        for k in ks {
                            if !merged[i].1.contains(&k) {
                                merged[i].1.push(k);
                            }
                        }
                    }
                    None => merged.push((h, ks)),
                }
            }
            let get = |r: &[(String, String)], h: &str, k: &str| {
                let from = r.iter().position(|x| x.0 == h).map_or(r.len(), |i| i + 1);
                r[from..].iter().take_while(|x| !x.0.starts_with('#')).find(|x| x.0 == k).map_or("-".to_string(), |x| x.1.clone())
            };
            for (h, ks) in &merged {
                line(&mut s, h, "", None);
                for k in ks {
                    line(&mut s, k, &get(&ra, h, k), Some(&get(&rb, h, k)));
                }
            }
            let changed: Vec<String> = a
                .iter()
                .filter_map(|x| {
                    let y = b.iter().find(|y| y.seed == x.seed)?;
                    (x.fate != y.fate || x.leg != y.leg).then(|| format!("    seed {} ({}): {:?} {} → {:?} {}", x.seed, x.goal.name(), x.fate, x.leg.map_or("", |l| l.words()), y.fate, y.leg.map_or("", |l| l.words())))
                })
                .collect();
            if !changed.is_empty() {
                let _ = writeln!(s, "\n  Adventurers whose fate changed:");
                for c in changed {
                    let _ = writeln!(s, "{}", c);
                }
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trial(seed: u64, alive: Vec<usize>, deaths: Vec<(&str, Option<&str>)>, sieges: Vec<(u32, &str)>) -> Trial {
        Trial {
            seed,
            style: Style::Steady,
            fort: "Testhold".into(),
            years: alive.len() as f64,
            alive: *alive.last().unwrap_or(&0),
            peak_pop: 7,
            peak_wealth: 1000,
            milestones: vec![],
            heroes: vec![],
            ambitions: vec![],
            chronicles: vec![],
            verdict: if alive.last().map_or(true, |&a| a == 0) { "Ruin".into() } else { "Endured".into() },
            overseer: String::new(),
            epitaph: String::new(),
            deaths: deaths
                .into_iter()
                .map(|(c, k)| DeathNote { cause: c.into(), killer: k.map(|k| k.to_string()), siege: Some(0), ..Default::default() })
                .collect(),
            sieges: sieges.into_iter().map(|(lost, outcome)| SiegeNote { size: 5, dwarves_lost: lost, outcome: outcome.into(), ..Default::default() }).collect(),
            alive_by_year: alive,
            story: vec![],
            counters: BTreeMap::new(),
            knobs: BTreeMap::new(),
            sagas: vec![],
        }
    }

    #[test]
    fn the_report_tells_why_forts_fall() {
        let a = vec![
            trial(1, vec![7, 0], vec![("bled to death", Some("goblin")), ("bled to death", Some("goblin")), ("thirst", None)], vec![(3, "the fortress fell")]),
            trial(2, vec![9, 9], vec![("struck down", Some("troglodyte"))], vec![(1, "routed")]),
        ];
        let r = report(&a, 2);
        assert!(r.contains("year 1: 2 (100%)") && r.contains("year 2: 1 (50%)"), "{}", r);
        assert!(r.contains("bled to death") && r.contains("50%"), "{}", r);
        assert!(r.contains("goblin") && r.contains("troglodyte"), "{}", r);
        assert!(r.contains("first") && r.contains("routed 1"), "{}", r);
        let mut b = a.clone();
        b[0].alive_by_year = vec![7, 4];
        b[0].verdict = "Endured".into();
        let c = compare(&a, &b, 2);
        assert!(c.contains("1/2") && c.contains("2/2"), "{}", c);
        assert!(c.contains("seed 1 Steady: Ruin"), "{}", c);
    }

    #[test]
    fn the_overseers_section_shows_m2_counters_by_style() {
        let mut a = vec![trial(1, vec![7], vec![], vec![])];
        assert_eq!(overseers(&a), "", "nothing until a package records something");
        a[0].counters.insert("m2.named_places".into(), 3);
        a[0].counters.insert("rescues".into(), 1);
        let s = overseers(&a);
        assert!(s.contains("Overseers") && s.contains("named_places") && s.contains("3.0") && !s.contains("rescues"), "{}", s);
    }

    #[test]
    fn counters_are_summed_and_compared() {
        let mut a = vec![trial(1, vec![7], vec![], vec![]), trial(2, vec![9], vec![], vec![])];
        a[0].counters.insert("rescues".into(), 3);
        a[0].counters.insert("bandages".into(), 1);
        a[1].counters.insert("rescues".into(), 5);
        let r = report(&a, 1);
        assert_eq!(counter_totals(&a).get("rescues"), Some(&8));
        // Summed over the forts, and per fort, sorted by name.
        assert!(r.contains("rescues                     8      4.0"), "{}", r);
        assert!(r.contains("bandages                    1      0.5"), "{}", r);
        assert!(r.find("bandages").unwrap() < r.find("rescues").unwrap(), "{}", r);
        assert!(!r.contains("Knobs:"), "no knobs line for a default batch: {}", r);
        // The second batch was run with a knob, and rescued more.
        let mut b = a.clone();
        b[1].counters.insert("rescues".into(), 11);
        b[1].counters.insert("wells".into(), 2);
        for t in b.iter_mut() {
            t.knobs.insert("bleed.gash".into(), 6.0);
        }
        let c = compare(&a, &b, 1);
        assert!(c.contains("rescues                           4.0          7.0"), "{}", c);
        assert!(c.contains("wells                             0.0          1.0"), "{}", c);
        assert!(c.contains("knobs after:  bleed.gash=6") && c.contains("knobs before: defaults"), "{}", c);
        assert!(report(&b, 1).contains("Knobs: bleed.gash=6"));
        // An old batch file, from before counters, still loads.
        let old = serde_json::to_string(&a[0]).unwrap().replace(r#","counters":{"bandages":1,"rescues":3},"knobs":{}"#, "");
        assert!(!old.contains("counters"), "{}", old);
        let t: Trial = serde_json::from_str(&old).unwrap();
        assert!(t.counters.is_empty() && t.knobs.is_empty());
    }
}
