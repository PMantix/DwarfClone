//! Regions: connected stretches of one broad kind of land (a forest that's
//! taiga in its cold north and jungle in its warm south is still one
//! forest), each named, with the wildlife and plants that live there.
//! Wild species have a habitat (warmth, rain, height, wet ground) and a
//! geographic range (a few seeded centres with a reach), so the same kind of
//! land in two corners of the world can hold different animals.

use crate::creature::Species;
use crate::overworld::{Biome, RegionKind};
use crate::rng::Rng;

/// The broad kind of land a biome belongs to, for grouping into regions.
pub fn broad(b: Biome, elev: u8) -> RegionKind {
    match b {
        Biome::Ocean => RegionKind::Ocean,
        Biome::Lake => RegionKind::Lake,
        Biome::Mountain => RegionKind::Mountains,
        Biome::Glacier if elev >= 170 => RegionKind::Mountains,
        Biome::Glacier => RegionKind::Glacier,
        Biome::Hills => RegionKind::Hills,
        Biome::Forest | Biome::Taiga | Biome::Jungle => RegionKind::Forest,
        Biome::Grassland | Biome::Savanna | Biome::Shrubland => RegionKind::Plains,
        Biome::Desert | Biome::Badlands => RegionKind::Desert,
        Biome::Swamp | Biome::Marsh => RegionKind::Swamp,
        Biome::Tundra => RegionKind::Tundra,
    }
}

/// What part a wild animal plays in the land: something to graze,
/// something that hunts, something rare and dangerous.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Grazer,
    Hunter,
    Rare,
}

/// Where a wild species can live.
pub struct Habitat {
    pub species: Species,
    pub role: Role,
    /// Warmth and rain it likes (0..100 each).
    pub temp: (f32, f32),
    pub rain: (f32, f32),
    /// Only on high ground (hills and up), or only on wet ground.
    pub high: bool,
    pub wet: bool,
    /// How many range centres it has, and how far each reaches (a share of
    /// the world's width).
    pub ranges: usize,
    pub reach: f32,
}

pub const WILD: [Habitat; 13] = [
    Habitat { species: Species::Deer, role: Role::Grazer, temp: (25.0, 85.0), rain: (30.0, 100.0), high: false, wet: false, ranges: 3, reach: 0.3 },
    Habitat { species: Species::Elk, role: Role::Grazer, temp: (4.0, 45.0), rain: (25.0, 100.0), high: false, wet: false, ranges: 3, reach: 0.3 },
    Habitat { species: Species::Ibex, role: Role::Grazer, temp: (8.0, 80.0), rain: (0.0, 80.0), high: true, wet: false, ranges: 3, reach: 0.3 },
    Habitat { species: Species::Camel, role: Role::Grazer, temp: (40.0, 100.0), rain: (0.0, 25.0), high: false, wet: false, ranges: 2, reach: 0.35 },
    Habitat { species: Species::Antelope, role: Role::Grazer, temp: (45.0, 100.0), rain: (10.0, 55.0), high: false, wet: false, ranges: 2, reach: 0.3 },
    Habitat { species: Species::Wolf, role: Role::Hunter, temp: (4.0, 70.0), rain: (15.0, 100.0), high: false, wet: false, ranges: 4, reach: 0.35 },
    Habitat { species: Species::Lion, role: Role::Hunter, temp: (50.0, 100.0), rain: (10.0, 55.0), high: false, wet: false, ranges: 2, reach: 0.3 },
    Habitat { species: Species::Scorpion, role: Role::Hunter, temp: (40.0, 100.0), rain: (0.0, 30.0), high: false, wet: false, ranges: 2, reach: 0.35 },
    Habitat { species: Species::Jaguar, role: Role::Hunter, temp: (58.0, 100.0), rain: (45.0, 100.0), high: false, wet: false, ranges: 2, reach: 0.3 },
    Habitat { species: Species::Bear, role: Role::Rare, temp: (8.0, 60.0), rain: (30.0, 100.0), high: false, wet: false, ranges: 3, reach: 0.25 },
    Habitat { species: Species::PolarBear, role: Role::Rare, temp: (0.0, 22.0), rain: (0.0, 100.0), high: false, wet: false, ranges: 2, reach: 0.45 },
    Habitat { species: Species::Viper, role: Role::Rare, temp: (42.0, 100.0), rain: (0.0, 100.0), high: false, wet: false, ranges: 3, reach: 0.3 },
    Habitat { species: Species::Alligator, role: Role::Rare, temp: (52.0, 100.0), rain: (50.0, 100.0), high: false, wet: true, ranges: 2, reach: 0.3 },
];

/// Plants: (name, warmth, rain, wet ground only, high ground only).
pub const PLANTS: [(&str, (f32, f32), (f32, f32), bool, bool); 18] = [
    ("oak", (35.0, 70.0), (40.0, 100.0), false, false),
    ("maple", (30.0, 60.0), (45.0, 100.0), false, false),
    ("birch", (15.0, 50.0), (35.0, 100.0), false, false),
    ("pine", (10.0, 55.0), (30.0, 100.0), false, false),
    ("larch", (4.0, 35.0), (30.0, 100.0), false, false),
    ("palm", (62.0, 100.0), (45.0, 100.0), false, false),
    ("mahogany", (65.0, 100.0), (60.0, 100.0), false, false),
    ("mangrove", (55.0, 100.0), (60.0, 100.0), true, false),
    ("willow", (20.0, 70.0), (50.0, 100.0), true, false),
    ("acacia", (55.0, 100.0), (12.0, 50.0), false, false),
    ("baobab", (62.0, 100.0), (12.0, 45.0), false, false),
    ("cactus", (40.0, 100.0), (0.0, 22.0), false, false),
    ("sagebrush", (25.0, 70.0), (8.0, 35.0), false, false),
    ("heather", (8.0, 45.0), (25.0, 100.0), false, true),
    ("reeds", (12.0, 100.0), (50.0, 100.0), true, false),
    ("tall grass", (25.0, 80.0), (20.0, 60.0), false, false),
    ("moss", (0.0, 30.0), (20.0, 100.0), false, false),
    ("juniper", (20.0, 60.0), (10.0, 50.0), false, true),
];

/// What one cell offers a species or plant.
pub struct CellClimate {
    pub x: i32,
    pub y: i32,
    pub temp: f32,
    pub rain: f32,
    pub elev: u8,
    pub biome: Biome,
}

fn fits(t: f32, r: f32, temp: (f32, f32), rain: (f32, f32)) -> bool {
    t >= temp.0 && t <= temp.1 && r >= rain.0 && r <= rain.1
}

fn wet_ground(b: Biome) -> bool {
    matches!(b, Biome::Swamp | Biome::Marsh | Biome::Jungle)
}

/// Each species' range centres over the world, fixed by the seed.
pub fn ranges(seed: u64, w: i32, h: i32) -> Vec<Vec<(f32, f32, f32)>> {
    let mut rng = Rng::new(seed ^ 0xFA_0A_0A);
    WILD.iter()
        .map(|hab| {
            (0..hab.ranges)
                .map(|_| (rng.range(0, w) as f32, rng.range(0, h) as f32, hab.reach * w as f32 * (0.7 + rng.f64() as f32 * 0.6)))
                .collect()
        })
        .collect()
}

/// The species living over a region's cells, most widespread first: a
/// species is there if its habitat and its range cover enough of them.
pub fn fauna(cells: &[CellClimate], ranges: &[Vec<(f32, f32, f32)>]) -> Vec<Species> {
    let mut found: Vec<(usize, Species)> = Vec::new();
    for (k, hab) in WILD.iter().enumerate() {
        if hab.ranges == 0 {
            continue;
        }
        let n = cells
            .iter()
            .filter(|c| {
                let in_range = ranges[k].iter().any(|&(cx, cy, r)| {
                    // A cell is twice as tall as it is wide.
                    let (dx, dy) = (c.x as f32 - cx, (c.y as f32 - cy) * 2.0);
                    dx * dx + dy * dy <= r * r
                });
                in_range && fits(c.temp, c.rain, hab.temp, hab.rain) && (!hab.high || c.elev >= 148) && (!hab.wet || wet_ground(c.biome)) && !c.biome.is_water()
            })
            .count();
        if n >= 3 && n * 6 >= cells.len() || n * 3 >= cells.len() && n > 0 {
            if !found.iter().any(|f| f.1 == hab.species) {
                found.push((n, hab.species));
            }
        }
    }
    found.sort_by(|a, b| b.0.cmp(&a.0));
    found.into_iter().map(|f| f.1).collect()
}

/// The plants of a region's cells, the commonest first (at most five).
pub fn plants(cells: &[CellClimate]) -> Vec<String> {
    let mut found: Vec<(usize, &str)> = PLANTS
        .iter()
        .map(|&(name, temp, rain, wet, high)| {
            let n = cells.iter().filter(|c| !c.biome.is_water() && fits(c.temp, c.rain, temp, rain) && (!wet || wet_ground(c.biome)) && (!high || c.elev >= 148)).count();
            (n, name)
        })
        .filter(|&(n, _)| n > 0 && n * 5 >= cells.len())
        .collect();
    found.sort_by(|a, b| b.0.cmp(&a.0));
    found.into_iter().take(5).map(|f| f.1.to_string()).collect()
}

/// The role a species plays, if it's wild.
pub fn role(s: Species) -> Option<Role> {
    WILD.iter().find(|h| h.species == s && h.ranges > 0).map(|h| h.role)
}
