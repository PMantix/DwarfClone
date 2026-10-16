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

/// The whole report, as text.
pub fn report(trials: &[Trial], years: usize) -> String {
    let mut s = String::new();
    let n = trials.len();
    let deaths: Vec<&DeathNote> = trials.iter().flat_map(|t| t.deaths.iter()).collect();
    let nd = deaths.len();

    let _ = writeln!(s, "\n  Forts standing at the end of each year (of {}):", n);
    let surv = survival(trials, years);
    let _ = writeln!(s, "    {}", surv.iter().enumerate().map(|(i, k)| format!("year {}: {} ({})", i + 1, k, pct(*k, n))).collect::<Vec<_>>().join("   "));

    let _ = writeln!(s, "\n  Deaths by cause ({} dwarves):", nd);
    for (c, k) in causes(trials) {
        let _ = writeln!(s, "    {:<22} {:>5}  {:>4}", c, k, pct(k, nd));
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
    let _ = writeln!(s, "    {:<8} {:>5} {:>7} {:>6} {:>8} {:>8} {:>7} {:>7}  {}", "", "count", "day", "size", "dwarves", "foes", "shelter", "bridge", "outcomes");
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
            "    {:<8} {:>5} {:>7.0} {:>6.1} {:>8.1} {:>8.1} {:>7} {:>7}  {}",
            ["first", "second", "later"][i],
            list.len(),
            avg(&|x| x.tick as f64 / crate::game::DAY as f64),
            avg(&|x| x.size as f64),
            avg(&|x| x.dwarves_lost as f64),
            avg(&|x| x.foes_slain as f64),
            pct(list.iter().filter(|x| x.shelter).count(), list.len()),
            pct(list.iter().filter(|x| x.bridge).count(), list.len()),
            outcomes.iter().map(|(o, c)| format!("{} {}", o, c)).collect::<Vec<_>>().join(", ")
        );
    }

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

/// Two batches side by side: survival, causes, sieges, and the forts whose
/// fate changed.
pub fn compare(a: &[Trial], b: &[Trial], years: usize) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "\n  {:<24} {:>12} {:>12}", "", "before", "after");
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
            epitaph: String::new(),
            deaths: deaths
                .into_iter()
                .map(|(c, k)| DeathNote { cause: c.into(), killer: k.map(|k| k.to_string()), siege: Some(0), ..Default::default() })
                .collect(),
            sieges: sieges.into_iter().map(|(lost, outcome)| SiegeNote { size: 5, dwarves_lost: lost, outcome: outcome.into(), ..Default::default() }).collect(),
            alive_by_year: alive,
            story: vec![],
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
}
