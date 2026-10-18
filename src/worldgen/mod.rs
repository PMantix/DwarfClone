//! The world map's land, made from fields (see `docs/worldgen.md`):
//! elevation and the climate fields, biomes read off them, the erosion
//! phase, rivers and lakes, and (`regions`) named regions with their
//! wildlife. `tiles` makes the finer region tiles the fortress map is
//! built from.

pub mod fields;
pub mod myth;
pub mod regions;
pub mod rivers;
pub mod tiles;

use crate::overworld::Biome;
use fields::{Grids, Sample};

/// The land of a whole world, before any civilisation.
pub struct Land {
    pub grids: Grids,
    pub biomes: Vec<Biome>,
    pub evil: Vec<bool>,
    /// River and brook links per cell (1 north, 2 east, 4 south, 8 west).
    pub river: Vec<u8>,
    pub brook: Vec<u8>,
    /// Summed rain from upstream.
    pub flow: Vec<f32>,
    /// The rivers, split into stems (main rivers and tributaries).
    pub stems: Vec<rivers::Stem>,
    /// Lake number + 1 per cell, and which lakes have no outflow.
    #[cfg_attr(not(test), allow(dead_code))]
    pub lake: Vec<u32>,
    #[cfg_attr(not(test), allow(dead_code))]
    pub terminal: Vec<bool>,
    /// What the making of the land went through, when asked for (to show
    /// the world taking shape); `None` otherwise.
    pub trace: Option<Trace>,
}

/// The land's making, kept to be played back: the elevation as first
/// raised, the trial rivers of the erosion phase in the order they ran,
/// and the elevation they left. Keeping it draws no random numbers.
#[derive(Clone, Debug, Default)]
pub struct Trace {
    /// Elevation before the erosion phase (already proportioned).
    pub raw: Vec<f32>,
    /// Every trial river, cell by cell from where it started.
    pub paths: Vec<Vec<u32>>,
    /// Elevation after the erosion phase, before the hollows fill.
    pub eroded: Vec<f32>,
}

/// Below this on the good/evil field the land is evil.
pub const EVIL: f32 = 10.0;

impl Grids {
    pub fn sample(&self, i: usize) -> Sample {
        Sample {
            elev: self.elev[i],
            temp: self.temp[i],
            rain: self.rain[i],
            drain: self.drain[i],
            veg: self.veg[i],
            savage: self.savage[i],
            good: self.good[i],
            salt: self.salt[i],
        }
    }
}

/// Make the land of a `w`×`h` world: elevation, the erosion phase, the
/// depressions filled (lakes where deep), the climate on the land as it
/// ends up, biomes, rivers and brooks.
#[cfg_attr(not(test), allow(dead_code))]
pub fn build(seed: u64, w: i32, h: i32) -> Land {
    build_traced(seed, w, h, false)
}

/// `build`, keeping a `Trace` of the making when `trace` is set. The land
/// comes out the same either way.
pub fn build_traced(seed: u64, w: i32, h: i32, trace: bool) -> Land {
    let n = (w * h) as usize;
    let mut elev = fields::elevation(seed, w, h);
    let mut tr = if trace { Some(Trace { raw: elev.clone(), ..Default::default() }) } else { None };
    rivers::erode_traced(&mut elev, w, h, seed, tr.as_mut().map(|t| &mut t.paths));
    fields::proportion(&mut elev);
    if let Some(t) = tr.as_mut() {
        t.eroded = elev.clone();
    }
    let (spill, recv) = rivers::route(&elev, w, h);
    let depth: Vec<f32> = (0..n).map(|i| if elev[i] < fields::SEA { 0.0 } else { spill[i] - elev[i] }).collect();
    let filled: Vec<f32> = (0..n).map(|i| if elev[i] < fields::SEA { elev[i] } else { spill[i] }).collect();
    let grids = fields::climate(seed, w, h, filled);
    let water = rivers::water(&grids.elev, &depth, recv, &grids.rain, w, h);
    let biomes: Vec<Biome> = (0..n).map(|i| if water.lake[i] != 0 { Biome::Lake } else { fields::classify(&grids.sample(i)) }).collect();
    let evil = (0..n).map(|i| !biomes[i].is_water() && grids.good[i] < EVIL).collect();
    let mut river = vec![0u8; n];
    let mut brook = vec![0u8; n];
    for i in 0..n {
        let r = water.recv[i];
        if r < 0 || !(water.river[i] || water.brook[i]) {
            continue;
        }
        let r = r as usize;
        let (out, back) = rivers::link(w, i, r);
        if water.river[i] {
            river[i] |= out;
            if water.river[r] {
                river[r] |= back;
            }
        } else {
            brook[i] |= out;
            if water.river[r] || water.brook[r] {
                brook[r] |= back;
            }
        }
    }
    let stems = rivers::stems(&water);
    Land { grids, biomes, evil, river, brook, flow: water.flow, stems, lake: water.lake, terminal: water.terminal, trace: tr }
}

#[cfg(test)]
mod tests {
    use super::fields::*;
    use super::*;

    fn worlds() -> Vec<(u64, Land)> {
        [1u64, 42, 7].iter().map(|&s| (s, build(s, 192, 80))).collect()
    }

    const N8: [(i32, i32); 8] = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];

    #[test]
    fn biomes_follow_the_fields() {
        for (seed, land) in worlds() {
            let g = &land.grids;
            let n = (g.w * g.h) as usize;
            let mut kinds = std::collections::HashSet::new();
            for i in 0..n {
                let b = land.biomes[i];
                kinds.insert(b);
                if b != Biome::Lake {
                    assert_eq!(b, classify(&g.sample(i)), "seed {}: cell {} isn't what its fields say", seed, i);
                }
            }
            assert!(kinds.len() >= 11, "seed {}: only {} biomes", seed, kinds.len());
            // Neighbouring cells differ only a little in rain and warmth, so
            // a border between two biomes is a smooth field crossing a line.
            for y in 0..g.h {
                for x in 0..g.w - 1 {
                    let (i, j) = (g.idx(x, y), g.idx(x + 1, y));
                    assert!((g.rain[i] - g.rain[j]).abs() <= 15.01, "seed {}: rain jumps at {:?}", seed, (x, y));
                    assert!((g.temp[i] - g.temp[j]).abs() <= 25.0, "seed {}: heat jumps at {:?}", seed, (x, y));
                }
            }
        }
    }

    #[test]
    fn no_desert_beside_jungle_or_swamp() {
        for (seed, land) in worlds() {
            let g = &land.grids;
            for y in 0..g.h {
                for x in 0..g.w {
                    if land.biomes[g.idx(x, y)] != Biome::Desert {
                        continue;
                    }
                    for (dx, dy) in N8 {
                        let (nx, ny) = (x + dx, y + dy);
                        if nx >= 0 && ny >= 0 && nx < g.w && ny < g.h {
                            let b = land.biomes[g.idx(nx, ny)];
                            assert!(!matches!(b, Biome::Jungle | Biome::Swamp | Biome::Marsh), "seed {}: desert at {:?} beside {:?}", seed, (x, y), b);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn rain_shadows_lie_leeward_of_ranges() {
        let (mut wet_side, mut dry_side, mut drier, mut pairs) = (0.0, 0.0, 0, 0);
        for (_, land) in worlds() {
            let g = &land.grids;
            for y in 0..g.h {
                let lat = ((y as f32 + 0.5) / g.h as f32 - 0.5).abs() * 2.0;
                let dir = wind(lat);
                for x in 3..g.w - 3 {
                    // A range: mountains here, lower land upwind and downwind.
                    if g.elev[g.idx(x, y)] < MOUNTAIN {
                        continue;
                    }
                    let (up, down) = (g.idx(x - 3 * dir, y), g.idx(x + 3 * dir, y));
                    if g.elev[up] < SEA || g.elev[down] < SEA || g.elev[up] >= MOUNTAIN || g.elev[down] >= MOUNTAIN {
                        continue;
                    }
                    wet_side += g.rain[up];
                    dry_side += g.rain[down];
                    pairs += 1;
                    if g.rain[down] < g.rain[up] {
                        drier += 1;
                    }
                }
            }
        }
        assert!(pairs > 30, "too few ranges with land either side ({})", pairs);
        let (wet, dry) = (wet_side / pairs as f32, dry_side / pairs as f32);
        eprintln!("{} ranges: windward rain {:.1}, leeward {:.1}, leeward drier in {}", pairs, wet, dry, drier);
        assert!(wet > dry + 4.0, "no rain shadow: windward {:.1} leeward {:.1}", wet, dry);
        assert!(drier * 5 > pairs * 3, "the lee is drier in only {} of {}", drier, pairs);
    }

    #[test]
    fn salt_only_near_the_sea() {
        for (seed, land) in worlds() {
            let g = &land.grids;
            for y in 0..g.h {
                for x in 0..g.w {
                    let i = g.idx(x, y);
                    if g.elev[i] < SEA {
                        assert_eq!(g.salt[i], 100.0);
                        continue;
                    }
                    if g.salt[i] > 0.0 {
                        let near = (-3..=3).any(|dy: i32| (-3..=3).any(|dx: i32| {
                            let (nx, ny) = (x + dx, y + dy);
                            dx.abs() + dy.abs() <= 3 && nx >= 0 && ny >= 0 && nx < g.w && ny < g.h && g.elev[g.idx(nx, ny)] < SEA
                        }));
                        assert!(near, "seed {}: salt {} far from the sea at {:?}", seed, g.salt[i], (x, y));
                    }
                }
            }
            // And salt marshes only on the coast.
            let salt_marsh = (0..(g.w * g.h) as usize).filter(|&i| matches!(land.biomes[i], Biome::Marsh | Biome::Swamp) && g.salt[i] >= 66.0).count();
            let marsh = (0..(g.w * g.h) as usize).filter(|&i| matches!(land.biomes[i], Biome::Marsh | Biome::Swamp)).count();
            eprintln!("seed {}: {} of {} wetland cells salt", seed, salt_marsh, marsh);
        }
    }

    #[test]
    fn land_is_made_quickly() {
        let t0 = std::time::Instant::now();
        let _ = build(5, 192, 80);
        let s = t0.elapsed().as_secs_f64();
        eprintln!("medium land: {:.0} ms", s * 1000.0);
        assert!(s < if cfg!(debug_assertions) { 8.0 } else { 0.8 }, "medium land took {:.2}s", s);
    }
}

/// Print a world's fields as digits (0-9 for 0..100) and its biomes:
/// `cargo test print_fields -- --ignored --nocapture` (SEED=n).
#[cfg(test)]
#[test]
#[ignore]
fn print_fields() {
    let seed: u64 = std::env::var("SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(42);
    let land = build(seed, 192, 80);
    let g = &land.grids;
    for (name, f) in [("rain", &g.rain), ("temp", &g.temp), ("drain", &g.drain)] {
        println!("-- {}", name);
        for y in 0..g.h {
            let row: String = (0..g.w).map(|x| {
                let i = g.idx(x, y);
                if g.elev[i] < fields::SEA { '~' } else { std::char::from_digit((f[i] / 10.0).min(9.0) as u32, 10).unwrap() }
            }).collect();
            println!("{}", row);
        }
    }
    let land_cells: Vec<usize> = (0..g.elev.len()).filter(|&i| g.elev[i] >= fields::SEA).collect();
    let mean = |f: &Vec<f32>| land_cells.iter().map(|&i| f[i]).sum::<f32>() / land_cells.len() as f32;
    println!("land means: rain {:.1} temp {:.1} drain {:.1} veg {:.1}", mean(&g.rain), mean(&g.temp), mean(&g.drain), mean(&g.veg));
}

#[cfg(test)]
mod water_tests {
    use super::fields::SEA;
    use super::*;

    #[test]
    fn rivers_gather_and_end_in_water() {
        let mut lakes = 0;
        for seed in [1u64, 42, 7, 99] {
            let land = build(seed, 192, 80);
            let g = &land.grids;
            let w = g.w;
            for s in &land.stems {
                // Downhill and gathering all the way.
                for k in 1..s.course.len() {
                    let (a, b) = (s.course[k - 1], s.course[k]);
                    assert!(g.elev[b] < g.elev[a], "seed {}: river climbs at {}", seed, b);
                    assert!(land.flow[b] >= land.flow[a], "seed {}: river loses water at {}", seed, b);
                }
                let into = s.into as usize;
                let end_ok = g.elev[into] < SEA || land.lake[into] != 0 || land.river[into] != 0;
                assert!(end_ok, "seed {}: a river ends on dry land at {:?}", seed, (into as i32 % w, into as i32 / w));
            }
            // A lake with no outflow lies in a dry basin; the others pass their water on.
            for (k, &t) in land.terminal.iter().enumerate() {
                let cells: Vec<usize> = (0..land.lake.len()).filter(|&i| land.lake[i] == k as u32 + 1).collect();
                let rain = cells.iter().map(|&i| g.rain[i]).sum::<f32>() / cells.len() as f32;
                if t {
                    assert!(rain < 35.0, "seed {}: a terminal lake in wet land (rain {:.0})", seed, rain);
                }
            }
            lakes += land.terminal.len();
            // Only the bigger water is a river; the rest are brooks.
            let rivers = land.river.iter().filter(|&&r| r != 0).count();
            let brooks = land.brook.iter().filter(|&&b| b != 0).count();
            eprintln!("seed {}: {} river cells in {} stems, {} brook cells, {} lakes", seed, rivers, land.stems.len(), brooks, land.terminal.len());
            assert!(rivers > 100 && brooks > rivers, "seed {}: {} river cells, {} brook cells", seed, rivers, brooks);
        }
        assert!(lakes >= 4, "only {} lakes in four worlds", lakes);
    }

    #[test]
    fn tracing_changes_nothing() {
        for seed in [3u64, 42] {
            let (a, b) = (build(seed, 128, 56), build_traced(seed, 128, 56, true));
            assert_eq!(a.grids.elev, b.grids.elev);
            assert_eq!(a.grids.rain, b.grids.rain);
            assert_eq!(a.biomes, b.biomes);
            assert_eq!(a.river, b.river);
            assert_eq!(a.flow, b.flow);
            let t = b.trace.expect("a trace");
            assert!(a.trace.is_none());
            assert!(t.paths.len() > 100, "{} trial rivers", t.paths.len());
            assert!(t.raw != t.eroded, "erosion left no mark");
        }
    }

    #[test]
    fn erosion_cuts_channels_through_the_heights() {
        let (w, h) = (192, 80);
        let mut e = fields::elevation(3, w, h);
        let before = e.clone();
        rivers::erode(&mut e, w, h, 3);
        let worn = (0..e.len()).filter(|&i| before[i] >= fields::HILLS && e[i] < before[i] - 2.0).count();
        eprintln!("{} high cells worn more than 2", worn);
        assert!(worn > 100);
        // Pits are fewer: cells with every neighbour higher.
        let pits = |v: &[f32]| (1..h - 1).flat_map(|y| (1..w - 1).map(move |x| (x, y))).filter(|&(x, y)| {
            let i = (y * w + x) as usize;
            v[i] >= SEA && [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().all(|&(dx, dy)| v[((y + dy) * w + x + dx) as usize] > v[i])
        }).count();
        eprintln!("pits before {} after {}", pits(&before), pits(&e));
        assert!(pits(&e) < pits(&before));
    }
}
