//! How far apart the peoples settle. A world's settings say how many
//! civilizations it holds and how much room each site keeps round it; the
//! gap grows with the size of the map, so a big world is not a crowded one.
//! The capitals stand well apart, every later site keeps its distance from
//! all the others, and a people stops spreading once it holds as many
//! sites as its world allows.

use super::{Overworld, Race, WorldParams};
use crate::rng::Rng;

/// Distance between two cells as the map shows it: a cell is twice as tall
/// as it is wide, so a row counts double.
pub fn gap_between(a: (i32, i32), b: (i32, i32)) -> i32 {
    (a.0 - b.0).abs().max(2 * (a.1 - b.1).abs())
}

impl WorldParams {
    /// How many times wider than a small world, roughly (the square root of
    /// the area ratio): the yardstick distances and counts grow by.
    pub fn reach(&self) -> f64 {
        let (w, h) = self.dims();
        ((w * h) as f64 / (128.0 * 56.0)).sqrt()
    }

    /// The least distance (`gap_between`) a new site keeps from every other.
    pub fn site_gap(&self) -> i32 {
        let base = [3.0, 5.0, 8.0][self.spacing.min(2) as usize];
        (base * self.reach()).round().max(2.0) as i32
    }

    /// How many standing sites a people may hold before it stops spreading.
    pub fn sites_per_civ(&self) -> usize {
        let base = [3.0, 4.0, 5.0, 6.0, 7.0][self.civs.min(4) as usize];
        (base * self.reach().sqrt()).round() as usize
    }
}

impl Overworld {
    /// Whether (x, y) keeps `gap` from every site of the world.
    pub(super) fn clear_of_sites(&self, x: i32, y: i32, gap: i32) -> bool {
        self.sites.iter().all(|s| gap_between((s.x, s.y), (x, y)) >= gap)
    }

    /// The world's own settings, or the old ones for a world made before
    /// they were kept.
    pub(super) fn spread_rules(&self) -> Option<(i32, usize)> {
        self.made_with.map(|p| (p.site_gap(), p.sites_per_civ()))
    }

    /// Room for a people of `race` holding `held` sites to found another
    /// near (x, y): a spot that keeps the world's gap from every site, or
    /// None when it holds all it may. A world made before the rules were
    /// kept settles as it always did.
    pub(super) fn spread_spot(&self, rng: &mut Rng, x: i32, y: i32, race: Race, held: usize) -> Option<(i32, i32)> {
        let Some((gap, cap)) = self.spread_rules() else { return self.free_spot_near(rng, x, y, 6, race) };
        if held >= cap {
            return None;
        }
        let r = (gap * 2).max(6);
        for _ in 0..60 {
            let nx = x + rng.range(-r, r + 1);
            let ny = y + rng.range(-r / 2, r / 2 + 1);
            if nx < 0 || ny < 0 || nx >= self.w || ny >= self.h {
                continue;
            }
            let c = self.cell(nx, ny);
            if c.site == 0 && !c.biome.is_water() && race.home(c.biome, c.evil) && self.clear_of_sites(nx, ny, gap) {
                return Some((nx, ny));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overworld::WorldGen;

    fn counts(p: WorldParams) -> (usize, usize) {
        let ow = WorldGen::new(p).finish();
        let civ_sites = ow.sites.iter().filter(|s| s.civ.is_some()).count();
        (ow.civs.len(), civ_sites)
    }

    /// The civ setting really sets how many peoples there are, and the
    /// sites keep their distance, across seeds and sizes.
    #[test]
    fn few_civs_mean_few_civs_and_sites_keep_apart() {
        let mut by_level = [0usize; 5];
        let mut sites_by_level = [0usize; 5];
        for seed in [1u64, 7, 42, 99] {
            for civs in 0..5u8 {
                let p = WorldParams { seed, civs, history: 150, ..WorldParams::default() };
                let (c, s) = counts(p);
                by_level[civs as usize] += c;
                sites_by_level[civs as usize] += s;
            }
            // Every civilized site keeps its gap from every other, as founded.
            let p = WorldParams { seed, ..WorldParams::default() };
            let ow = WorldGen::new(p).finish();
            let gap = p.site_gap();
            // (The villains' made peoples settle by their own rules, peoples.rs:
            // their history is told after the rest, so their sites come last,
            // where the founding years start again from the beginning.)
            let cut = (1..ow.sites.len()).find(|&k| ow.sites[k].founded < ow.sites[k - 1].founded).unwrap_or(ow.sites.len()) as u32;
            let ours = |s: &&crate::overworld::Site| s.civ.is_some() && s.id < cut;
            for a in ow.sites.iter().filter(ours).filter(|s| s.founded > 0) {
                for b in ow.sites.iter().filter(ours).filter(|b| b.id < a.id) {
                    assert!(gap_between((a.x, a.y), (b.x, b.y)) >= gap, "seed {}: {} and {} too close", seed, a.name, b.name);
                }
            }
        }
        eprintln!("civs by level (4 seeds): {:?}, civ sites: {:?}", by_level, sites_by_level);
        // "Very few" is a handful: two peoples founded on a small world, and
        // perhaps one a villain makes or a realm that splits.
        assert!(by_level[0] <= 4 * 4, "very few civs gave {:?}", by_level);
        for k in 1..5 {
            assert!(by_level[k] > by_level[k - 1], "civ counts don't rise with the setting: {:?}", by_level);
            assert!(sites_by_level[k] >= sites_by_level[k - 1], "sites don't rise with the setting: {:?}", sites_by_level);
        }
        // (Every site ever founded, ruins too: the old normal world founded 450.)
        assert!(sites_by_level[2] <= 4 * 60, "a normal small world holds {} sites", sites_by_level[2] / 4);
        // Further apart means fewer.
        let near = counts(WorldParams { seed: 3, spacing: 0, ..WorldParams::default() }).1;
        let far = counts(WorldParams { seed: 3, spacing: 2, ..WorldParams::default() }).1;
        assert!(far < near, "spacing far {} vs near {}", far, near);
    }

    /// Before/after numbers: `cargo test --release print_civ_counts -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn print_civ_counts() {
        for size in [4u8, 0, 1, 2, 3] {
            for civs in 0..5u8 {
                let (mut c, mut s, mut all) = (0, 0, 0);
                let seeds = [1u64, 7, 42, 99, 5000];
                for &seed in &seeds {
                    let ow = WorldGen::new(WorldParams { seed, civs, size, ..WorldParams::default() }).finish();
                    c += ow.civs.len();
                    s += ow.sites.iter().filter(|s| s.civ.is_some()).count();
                    all += ow.sites.len();
                }
                let n = seeds.len() as f64;
                println!("size {} civs {}: {:.1} civs, {:.1} civ sites, {:.1} sites in all", size, civs, c as f64 / n, s as f64 / n, all as f64 / n);
            }
        }
    }
}
