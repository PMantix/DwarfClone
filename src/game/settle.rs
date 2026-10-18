//! Before anyone arrives: the land's water is let run until it comes to
//! rest, so the fortress starts on the lakes, rivers and pools that last,
//! not on puddles that dry up in the first days.

use super::*;

/// Wet tiles and the water in them (frozen water left out).
pub(crate) fn water_census(w: &World) -> (usize, u64) {
    let mut wet = 0;
    let mut total = 0u64;
    for t in &w.tiles {
        if t.water > 0 && !t.frozen {
            wet += 1;
            total += t.water as u64;
        }
    }
    (wet, total)
}

/// How often (in water turns, one every three ticks) the settling looks to
/// see whether the water has come to rest.
const CHECK: u32 = 100;
/// The most water turns the settling runs: twenty days.
const MAX_TURNS: u32 = 8000;

/// What the settling did, for looking into.
#[derive(Clone, Copy, Debug, Default)]
pub struct Settled {
    pub turns: u32,
    pub before: (usize, u64),
    pub after: (usize, u64),
    pub ms: f64,
}

impl Game {
    /// Before anyone arrives, let the water run, fall and dry until it's at
    /// rest: brooks drain to their pools, cavern pools level out, damp films
    /// dry away. Only the water moves (and an aquifer seeps); the clock
    /// doesn't, and the game's own dice aren't touched.
    pub(super) fn settle_water(&mut self) -> Settled {
        let start = std::time::Instant::now();
        let before = water_census(&self.world);
        let dice = Rng::new(self.world.gen_seed ^ 0x5E77_1ED0_A7E5);
        let saved = std::mem::replace(&mut self.rng, dice);
        self.water_cells = None;
        let mut last = (wet_mask(&self.world), before.1);
        let mut calm = 0;
        let mut turns = 0;
        while turns < MAX_TURNS {
            self.fluids();
            if turns % 8 == 7 {
                self.aquifers();
            }
            turns += 1;
            if turns % CHECK != 0 {
                continue;
            }
            // At rest: hardly a tile has got wet or dry, and hardly any
            // water has gone, since the last look (twice running).
            let mask = wet_mask(&self.world);
            let (wet, total) = water_census(&self.world);
            let flips = mask.iter().zip(&last.0).filter(|(a, b)| a != b).count();
            let lost = (last.1 as i64 - total as i64).unsigned_abs();
            let quiet = flips <= (wet / 1000).max(2) && lost <= (total / 1000).max(4);
            calm = if quiet { calm + 1 } else { 0 };
            last = (mask, total);
            if calm >= 3 {
                break;
            }
        }
        self.rng = saved;
        self.water_cells = None;
        self.water_still.clear();
        self.fluid_calls = 0;
        Settled { turns, before, after: water_census(&self.world), ms: start.elapsed().as_secs_f64() * 1000.0 }
    }
}

/// Which tiles are wet (frozen water left out).
fn wet_mask(w: &World) -> Vec<bool> {
    w.tiles.iter().map(|t| t.water > 0 && !t.frozen).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::tests::pick_sites;

    const SIX: [&str; 6] = ["plains", "hills", "mountains", "river", "coast", "swamp"];

    /// The six sample sites: (name, seed, x, y).
    pub(crate) fn six_sites() -> Vec<(&'static str, u64, i32, i32)> {
        let mut out = Vec::new();
        for name in SIX {
            'found: for seed in [42u64, 7, 5] {
                let ow = Overworld::generate(seed);
                if let Some(&(_, x, y)) = pick_sites(&ow, seed).iter().find(|s| s.0.contains(name)) {
                    out.push((name, seed, x, y));
                    break 'found;
                }
            }
        }
        out
    }

    /// Wet tiles under the open sky.
    fn surface_wet(w: &World) -> usize {
        w.tiles.iter().enumerate().filter(|(i, t)| t.water > 0 && !t.frozen && w.outside(w.pos_of(*i))).count()
    }

    /// Wet tiles and total water at embark and after three days, for each of
    /// the six sample sites:
    /// `cargo test --release settle_report -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn settle_report() {
        for (name, seed, x, y) in six_sites() {
            let ow = Overworld::generate(seed);
            for size in world::MapSize::ALL {
                let params = ow.embark_params(seed, x, y, size);
                let world = world::generate(seed ^ ((x as u64) << 20) ^ ((y as u64) << 40), &params);
                let mut g = Game::from_world(world, seed, ow.clone(), (x, y), Pos::new(0, 0, 0));
                let st = g.settle_water();
                println!("   {}x{}x{}: settled in {} turns, {:.0} ms: wet {} -> {}, water {} -> {}", size.w, size.h, size.d, st.turns, st.ms, st.before.0, st.after.0, st.before.1, st.after.1);
            }
            let t0 = std::time::Instant::now();
            let mut g = Game::embark(seed, ow, x, y);
            let ms = t0.elapsed().as_secs_f64() * 1000.0;
            let (w0, t0w) = water_census(&g.world);
            let s0 = surface_wet(&g.world);
            for _ in 0..3 * DAY {
                g.step();
            }
            let (w1, t1w) = water_census(&g.world);
            let s1 = surface_wet(&g.world);
            println!("{:10} seed {:2} ({:2},{:2}) embark {:5.0} ms  surface wet {:5} -> {:5}  wet {:6} -> {:6} ({:+.1}%)  water {:7} -> {:7} ({:+.1}%)",
                name, seed, x, y, ms, s0, s1, w0, w1, (w1 as f64 - w0 as f64) * 100.0 / w0.max(1) as f64, t0w, t1w, (t1w as f64 - t0w as f64) * 100.0 / t0w.max(1) as f64);
        }
    }

    /// Every kind of sample site from two worlds: (name, seed, x, y).
    fn many_sites() -> Vec<(&'static str, u64, i32, i32)> {
        let mut out = Vec::new();
        for seed in [42u64, 7] {
            let ow = Overworld::generate(seed);
            for (name, x, y) in pick_sites(&ow, seed) {
                out.push((name, seed, x, y));
            }
        }
        out
    }

    #[test]
    fn a_new_site_s_water_barely_changes_in_its_first_three_days() {
        for (name, seed, x, y) in many_sites() {
            let ow = Overworld::generate(seed);
            let mut g = Game::embark(seed, ow, x, y);
            let mask0 = wet_mask(&g.world);
            let (wet0, total0) = water_census(&g.world);
            for _ in 0..3 * DAY {
                g.step();
            }
            let (wet1, total1) = water_census(&g.world);
            let flips = wet_mask(&g.world).iter().zip(&mask0).filter(|(a, b)| a != b).count();
            let dw = (total1 as i64 - total0 as i64).unsigned_abs();
            println!("{} ({} {},{}): wet {} -> {} ({} changed), water {} -> {}", name, seed, x, y, wet0, wet1, flips, total0, total1);
            assert!(flips <= (wet0 / 500).max(3), "{}: {} of {} tiles got wet or dry in three days", name, flips, wet0);
            assert!(dw <= (total0 / 1000).max(4), "{}: the water went from {} to {} in three days", name, total0, total1);
            assert!(g.world.walkable(g.embark), "{}: the wagon's spot isn't ground", name);
            let e = g.embark;
            assert_eq!(g.world.t(e).water, 0, "{}: the wagon stands in water", name);
            assert!(DIRS8.iter().all(|&(dx, dy)| g.world.get(e.offset(dx, dy, 0)).map_or(true, |t| t.water <= 3)), "{}: deep water beside the wagon", name);
        }
    }

    #[test]
    fn rivers_still_run_full_across_the_map() {
        let mut checked = 0;
        for seed in [42u64, 7] {
            let ow = Overworld::generate(seed);
            let sites: Vec<(i32, i32)> = (2..ow.h - 3).flat_map(|y| (2..ow.w - 3).map(move |x| (x, y))).filter(|&(x, y)| ow.embarkable(x, y) && ow.cell(x, y).river != 0).take(4).collect();
            for (x, y) in sites {
                let params = ow.embark_params(seed, x, y, world::MapSize::MEDIUM);
                let (_, shape) = crate::terrain::generate_shaped(seed ^ ((x as u64) << 20) ^ ((y as u64) << 40), &params, (0, 0), (params.size.w, params.size.h), None);
                let mut g = Game::embark(seed, ow.clone(), x, y);
                assert!(!g.world.river_beds.is_empty(), "site {:?}: the river's water isn't kept", (x, y));
                for _ in 0..3 * DAY {
                    g.step();
                }
                let w = &g.world;
                let bed: Vec<(i32, i32)> = (0..w.h).flat_map(|y| (0..w.w).map(move |x| (x, y))).filter(|&(x, y)| shape.river[(y * w.w + x) as usize] != 0).collect();
                assert!(bed.len() > 40, "site {:?}: hardly any river ({})", (x, y), bed.len());
                for &(bx, by) in &bed {
                    let t = w.t(Pos::new(bx, by, shape.surf[(by * w.w + bx) as usize]));
                    assert!(t.water > 0 || t.frozen, "seed {} site {:?}: the river bed at {:?} ran dry", seed, (x, y), (bx, by));
                }
                // It runs off the map (a river may rise or end in a lake on it).
                let edges = bed.iter().filter(|&&(bx, by)| bx == 0 || by == 0 || bx == w.w - 1 || by == w.h - 1).map(|&(bx, by)| if by == 0 { 0 } else if bx == w.w - 1 { 1 } else if by == w.h - 1 { 2 } else { 3 }).collect::<HashSet<_>>();
                assert!(!edges.is_empty(), "seed {} site {:?}: the river doesn't reach the map's edge", seed, (x, y));
                checked += 1;
            }
        }
        assert!(checked >= 6, "only {} river sites", checked);
    }

    #[test]
    fn settling_is_quick_and_leaves_the_clock_and_dice_alone() {
        for (name, seed, x, y) in six_sites() {
            let ow = Overworld::generate(seed);
            for (size, limit) in [(world::MapSize::MEDIUM, 1000.0), (world::MapSize::ALL[2], 3000.0)] {
                let params = ow.embark_params(seed, x, y, size);
                let world = world::generate(seed ^ ((x as u64) << 20) ^ ((y as u64) << 40), &params);
                let mut g = Game::from_world(world, seed, ow.clone(), (x, y), Pos::new(0, 0, 0));
                let dice = format!("{:?}", g.rng);
                let st = g.settle_water();
                assert_eq!(g.tick, 0);
                assert_eq!(format!("{:?}", g.rng), dice, "{}: settling used the game's dice", name);
                assert!(st.turns <= MAX_TURNS);
                // (Timed only in release builds.)
                if !cfg!(debug_assertions) {
                    assert!(st.ms < limit, "{} {}x{}: settling took {:.0} ms", name, size.w, size.h, st.ms);
                }
            }
        }
    }
}
