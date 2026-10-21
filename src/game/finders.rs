//! The job finders: what an idle dwarf looks for once their needs are met,
//! in order of priority. `choose_task` walks the table and takes the first
//! finder that gives them a task. A new kind of work adds a row here.

use super::*;

/// Looks for work for a dwarf; true if it gave them a task.
pub(super) type Finder = fn(&mut Game, &mut Dwarf) -> bool;

/// Every finder, most urgent first.
pub(super) const FINDERS: &[(&str, Finder)] = &[
    ("punishment", |g, d| g.find_punishment(d)),
    ("rescue", |g, d| g.find_rescue(d)),
    ("lever", |g, d| g.find_lever_pull(d)),
    ("first aid", |g, d| g.find_first_aid(d)),
    ("patient", |g, d| d.has(Labor::Medicine) && g.find_patient(d)),
    ("coax", |g, d| g.find_coax(d)),
    ("slaughter", |g, d| d.has(Labor::Butchery) && g.find_slaughter(d)),
    ("burial", |g, d| d.has(Labor::Hauling) && g.find_burial(d)),
    // Drink and meals into the shelter's larder (shelter.rs), before the
    // fort's other work: it is what they will live on in a siege.
    ("larder", |g, d| d.has(Labor::Hauling) && !d.specialist && g.find_larder_haul(d)),
    ("construct", |g, d| d.has(Labor::Building) && (g.find_construct(d) || g.find_build_tile(d))),
    ("craft", |g, d| g.find_craft(d)),
    ("detail", |g, d| d.has(Labor::Engraving) && g.find_detail(d)),
    ("mine", |g, d| d.has(Labor::Mining) && g.find_dig(d, false)),
    // Outdoor work waits while beasts prowl near (hunts.rs).
    ("chop", |g, d| d.has(Labor::Woodcutting) && !g.outdoors_banned() && g.find_dig(d, true)),
    ("farm", |g, d| d.has(Labor::Farming) && (g.find_farm(d) || !g.outdoors_banned() && g.find_gather(d))),
    ("fish", |g, d| d.has(Labor::Fishing) && !g.outdoors_banned() && g.find_fishing(d)),
    // O12: a trinket to the room it was meant for (clockwork.rs).
    ("trinket", |g, d| d.has(Labor::Hauling) && !d.specialist && !g.inspirations.clockwork.homes.is_empty() && g.find_trinket_haul(d)),
    // Specialists don't haul; the depot and the dump come before the stockpiles.
    ("haul", |g, d| d.has(Labor::Hauling) && !d.specialist && (g.find_depot_haul(d) || g.find_dump_haul(d) || g.find_haul(d) || g.find_clean(d))),
];

/// The finders from here on are routine work: one that found nothing for a
/// dwarf rests a while for them (`finder_resting`). Those above (rescue,
/// first aid, the lever...) are asked every time.
pub(super) const FIRST_ROUTINE: usize = 6;

/// Whether finder `k` may rest: routine work, but not stocking the
/// shelter's larder, which can't wait on a siege.
fn routine(k: usize) -> bool {
    k >= FIRST_ROUTINE && FINDERS.get(k).map_or(false, |f| f.0 != "larder")
}

impl Game {
    /// Whether finder `k` found nothing for this dwarf a moment ago, with the
    /// fortress dug as it is now: skip it this time (OTH-06).
    pub(super) fn finder_resting(&self, dwarf: u32, k: usize) -> bool {
        routine(k)
            && self.finder_fail.get(&(dwarf, k as u8)).map_or(false, |&(until, ver, _)| self.tick < until && ver == self.dig_version)
            && self.knob("finder.backoff", 1.0) > 0.0
    }

    /// Remember how finder `k` did: a miss rests it for 40 ticks, doubling
    /// with each miss in a row (to 160); a find forgets the misses.
    pub(super) fn finder_done(&mut self, dwarf: u32, k: usize, hit: bool) {
        if !routine(k) {
            return;
        }
        let key = (dwarf, k as u8);
        if hit {
            self.finder_fail.remove(&key);
            return;
        }
        let ver = self.dig_version;
        let n = self.finder_fail.get(&key).map_or(0, |&(_, v, n)| if v == ver { n } else { 0 });
        let wait = (self.knob("finder.rest", 40.0) as u64) << n.min(2);
        self.finder_fail.insert(key, (self.tick + wait, ver, n.saturating_add(1)));
    }
}

thread_local! {
    /// TRIAL_PROF: time spent in each finder on this thread, with calls and
    /// how many gave a task.
    static PROF: std::cell::RefCell<BTreeMap<&'static str, (f64, u64, u64)>> = std::cell::RefCell::new(BTreeMap::new());
}

/// Note a finder's turn (only while profiling).
pub(super) fn prof_note(name: &'static str, since: std::time::Instant, hit: bool) {
    let ms = since.elapsed().as_secs_f64() * 1000.0;
    PROF.with(|p| {
        let mut p = p.borrow_mut();
        let e = p.entry(name).or_default();
        e.0 += ms;
        e.1 += 1;
        e.2 += hit as u64;
    });
}

/// The finders' times since the last call, slowest first: (name, ms, calls, hits).
pub fn prof_take() -> Vec<(&'static str, f64, u64, u64)> {
    let mut v: Vec<_> = PROF.with(|p| std::mem::take(&mut *p.borrow_mut())).into_iter().map(|(k, (ms, n, h))| (k, ms, n, h)).collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finders_are_named_once() {
        let mut names: Vec<&str> = FINDERS.iter().map(|f| f.0).collect();
        assert_eq!(names.first(), Some(&"punishment"));
        assert_eq!(names.last(), Some(&"haul"));
        names.sort();
        names.dedup();
        assert_eq!(names.len(), FINDERS.len());
        assert_eq!(FINDERS[FIRST_ROUTINE].0, "slaughter", "the urgent finders come before the routine ones");
    }

    #[test]
    fn a_finder_that_found_nothing_rests() {
        let mut g = Game::new(3);
        let k = FIRST_ROUTINE + 1;
        assert!(!g.finder_resting(7, k));
        g.finder_done(7, k, false);
        assert!(g.finder_resting(7, k), "it rests after a miss");
        assert!(!g.finder_resting(8, k), "only for that dwarf");
        assert!(!g.finder_resting(7, 1) && { g.finder_done(7, 1, false); !g.finder_resting(7, 1) }, "urgent finders never rest");
        let larder = FINDERS.iter().position(|f| f.0 == "larder").unwrap();
        g.finder_done(7, larder, false);
        assert!(!g.finder_resting(7, larder), "nor does the shelter's larder");
        g.tick += 41;
        assert!(!g.finder_resting(7, k), "for a while");
        g.finder_done(7, k, false);
        g.tick += 41;
        assert!(g.finder_resting(7, k), "longer after a second miss");
        g.dig_version += 1;
        assert!(!g.finder_resting(7, k), "a dig changes everything");
        g.finder_done(7, k, true);
        assert!(!g.finder_fail.contains_key(&(7, k as u8)), "a find forgets the misses");
    }
}
