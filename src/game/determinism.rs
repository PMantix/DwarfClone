//! The simulation depends only on the game and its seed: two games from
//! the same seed, stepped the same, are the same game, whatever else the
//! machine is doing and whichever thread runs them. (A test that passed or
//! failed by what ran beside it was a fort that played differently.)

use super::*;
#[cfg(test)]
use std::hash::{Hash, Hasher};

impl Game {
    /// A fingerprint of the fort as it stands: the dice, the dwarves, the
    /// creatures, the items and buildings, the designations. Two games that
    /// differ in anything that matters soon differ here.
    #[cfg(test)]
    pub fn state_hash(&self) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.tick.hash(&mut h);
        self.next_id.hash(&mut h);
        self.rng.clone().next_u64().hash(&mut h);
        for d in &self.dwarves {
            (d.id, d.pos, d.alive, d.hunger, d.thirst, d.fatigue).hash(&mut h);
        }
        for c in self.creatures.values() {
            (c.id, c.pos, c.alive, c.path.len()).hash(&mut h);
        }
        for it in self.items.values() {
            (it.id, it.pos, it.carried_by).hash(&mut h);
        }
        for b in self.buildings.values() {
            (b.id, b.pos, b.built, b.raised).hash(&mut h);
        }
        let mut des: Vec<Pos> = self.designations.iter().copied().collect();
        des.sort();
        des.hash(&mut h);
        self.log.len().hash(&mut h);
        h.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::overseer::Style;

    /// One Warlord fort from `seed`, its fingerprint every few days.
    fn run(seed: u64, days: u64) -> Vec<(u64, u64)> {
        let mut g = Game::new(seed);
        g.set_overseer(Some(Style::Warlord));
        let mut out = Vec::new();
        while g.tick < days * DAY && g.alive_count() > 0 {
            g.step();
            if g.tick % (DAY / 2) == 0 {
                out.push((g.tick, g.state_hash()));
            }
        }
        out
    }

    /// The long look (`cargo test --release forts_twinned -- --ignored`):
    /// SEEDS forts from SEED0, each played twice at once on its own
    /// threads for DAYS days, and where each pair parted, if it did.
    #[test]
    #[ignore]
    fn forts_twinned() {
        let env = |k: &str, d: u64| std::env::var(k).ok().and_then(|s| s.parse().ok()).unwrap_or(d);
        let (seed0, seeds, days) = (env("SEED0", 7000), env("SEEDS", 4), env("DAYS", 510));
        let runs: Vec<Vec<(u64, u64)>> = std::thread::scope(|s| {
            let hs: Vec<_> = (0..seeds * 2).map(|k| s.spawn(move || run(seed0 + k / 2, days))).collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        });
        let mut parted = Vec::new();
        for k in 0..seeds as usize {
            let (a, b) = (&runs[2 * k], &runs[2 * k + 1]);
            let first = a.iter().zip(b).find(|(x, y)| x != y).map(|(x, _)| x.0);
            eprintln!("seed {}: {} looks, parted at {:?}", seed0 + k as u64, a.len(), first);
            if first.is_some() || a.len() != b.len() {
                parted.push(seed0 + k as u64);
            }
        }
        assert!(parted.is_empty(), "forts parted: {:?}", parted);
    }

    /// The same fort played alone and played while the machine is busy
    /// (a pool of threads playing other forts beside it) is the same fort.
    #[test]
    #[cfg_attr(debug_assertions, ignore)]
    fn a_fort_plays_the_same_alone_or_under_load() {
        let (seed, days) = (7000, 120);
        let alone = run(seed, days);
        let stop = std::sync::atomic::AtomicBool::new(false);
        let busy = std::thread::scope(|s| {
            let load: Vec<_> = (0..6u64)
                .map(|k| {
                    let stop = &stop;
                    s.spawn(move || {
                        while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                            let mut g = Game::new(9100 + k);
                            g.set_overseer(Some(Style::Warlord));
                            for _ in 0..DAY {
                                g.step();
                            }
                        }
                    })
                })
                .collect();
            let ours = s.spawn(move || run(seed, days)).join().unwrap();
            stop.store(true, std::sync::atomic::Ordering::Relaxed);
            for l in load {
                l.join().unwrap();
            }
            ours
        });
        let first = alone.iter().zip(&busy).find(|(a, b)| a != b);
        assert!(first.is_none(), "the forts parted at tick {:?}", first.map(|(a, _)| a.0));
        assert_eq!(alone.len(), busy.len());
    }
}
