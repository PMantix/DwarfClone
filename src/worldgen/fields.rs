//! The world's basic fields, one value per world cell: elevation,
//! temperature, rainfall, drainage, vegetation, savagery, good/evil and
//! salinity. The biomes are read off them (`classify`); nothing places a
//! biome directly. See `docs/worldgen.md`.

use crate::noise::Noise;
use crate::overworld::Biome;
use crate::rng::Rng;
use serde::{Deserialize, Serialize};

/// Sea level, the hills line and the mountain line, on the 0..255
/// elevation scale `Cell::elev` uses.
pub const SEA: f32 = 92.0;
pub const HILLS: f32 = 148.0;
pub const MOUNTAIN: f32 = 170.0;

/// The world-creation knobs that shape the land: how much of it is sea,
/// how much mountain, how wet, warm and wooded. The default is the world
/// as it has always been made (every offset zero, the usual shares).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mix {
    /// Share of the map below sea level.
    pub sea: f64,
    /// Where on the land's heights the hills and the mountains begin
    /// (quantiles of the land).
    pub hills: f64,
    pub mountains: f64,
    /// Added to every cell's rain and warmth (0..100 scales).
    pub wet: f32,
    pub warm: f32,
    /// The rain above which temperate and warm land grows forest.
    pub forest: f32,
}

impl Default for Mix {
    fn default() -> Self {
        Mix { sea: 0.4, hills: 0.8, mountains: 0.93, wet: 0.0, warm: 0.0, forest: 50.0 }
    }
}

impl Mix {
    /// The mix for the setup screen's levels (0..=4 each, 2 the usual).
    pub fn from_levels(ocean: u8, mountains: u8, rain: u8, heat: u8, forest: u8) -> Mix {
        let l = |v: u8| v.min(4) as usize;
        let (hills, mts) = [(0.90, 0.985), (0.85, 0.96), (0.80, 0.93), (0.74, 0.89), (0.66, 0.84)][l(mountains)];
        Mix {
            sea: [0.22, 0.31, 0.4, 0.5, 0.6][l(ocean)],
            hills,
            mountains: mts,
            wet: [-22.0, -11.0, 0.0, 11.0, 22.0][l(rain)],
            warm: [-20.0, -10.0, 0.0, 10.0, 20.0][l(heat)],
            forest: [62.0, 56.0, 50.0, 44.0, 38.0][l(forest)],
        }
    }
}

/// The fields, as floats while the world is made.
#[derive(Clone, Debug)]
#[cfg_attr(not(test), allow(dead_code))]
pub struct Grids {
    pub w: i32,
    pub h: i32,
    /// 0..255, sea level `SEA`.
    pub elev: Vec<f32>,
    /// The rest 0..100.
    pub temp: Vec<f32>,
    pub rain: Vec<f32>,
    pub drain: Vec<f32>,
    pub veg: Vec<f32>,
    pub savage: Vec<f32>,
    /// Low is evil, high is good.
    pub good: Vec<f32>,
    pub salt: Vec<f32>,
}

impl Grids {
    #[inline]
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn idx(&self, x: i32, y: i32) -> usize {
        (y * self.w + x) as usize
    }
}

/// The fields as kept with the world: one byte a cell each (elevation on
/// its 0..255 scale, the rest 0..100), saved as base64.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Fields {
    #[serde(with = "b64")]
    pub temp: Vec<u8>,
    #[serde(with = "b64")]
    pub rain: Vec<u8>,
    #[serde(with = "b64")]
    pub drain: Vec<u8>,
    #[serde(with = "b64")]
    pub veg: Vec<u8>,
    #[serde(with = "b64")]
    pub savage: Vec<u8>,
    #[serde(with = "b64")]
    pub good: Vec<u8>,
    #[serde(with = "b64")]
    pub salt: Vec<u8>,
    /// Water gathered from upstream, `28 ln(1 + flow)`.
    #[serde(with = "b64", default)]
    pub flow: Vec<u8>,
}

impl Fields {
    pub fn from_grids(g: &Grids, flow: &[f32]) -> Fields {
        let b = |v: &Vec<f32>| v.iter().map(|&x| x.round().clamp(0.0, 255.0) as u8).collect();
        let flow = flow.iter().map(|&f| flow_byte(f)).collect();
        Fields { temp: b(&g.temp), rain: b(&g.rain), drain: b(&g.drain), veg: b(&g.veg), savage: b(&g.savage), good: b(&g.good), salt: b(&g.salt), flow }
    }

    pub fn is_empty(&self) -> bool {
        self.rain.is_empty()
    }
}

/// Flow as kept in a byte.
pub fn flow_byte(f: f32) -> u8 {
    ((1.0 + f.max(0.0)).ln() * 28.0).round().clamp(0.0, 255.0) as u8
}

/// The flow a byte stands for.
pub fn byte_flow(b: u8) -> f32 {
    (b as f32 / 28.0).exp() - 1.0
}

/// One cell's fields, read back.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sample {
    pub elev: f32,
    pub temp: f32,
    pub rain: f32,
    pub drain: f32,
    pub veg: f32,
    pub savage: f32,
    pub good: f32,
    pub salt: f32,
}

/// The biome the fields call for (see the table in `docs/worldgen.md`).
/// Lakes come from the river phase, not from here.
pub fn classify(s: &Sample) -> Biome {
    classify_with(s, 50.0)
}

/// `classify`, with forest beginning at `forest` rain (`Mix::forest`).
pub fn classify_with(s: &Sample, forest: f32) -> Biome {
    let (e, t, r, d) = (s.elev, s.temp, s.rain, s.drain);
    if e < SEA {
        return Biome::Ocean;
    }
    if e >= MOUNTAIN {
        return if t < 12.0 { Biome::Glacier } else { Biome::Mountain };
    }
    if t < 8.0 {
        return Biome::Glacier;
    }
    if t < 22.0 {
        return if r >= 45.0 && t >= 14.0 { Biome::Taiga } else { Biome::Tundra };
    }
    if r >= 66.0 && d < 33.0 {
        return Biome::Swamp;
    }
    if r >= 66.0 && d < 50.0 {
        return Biome::Marsh;
    }
    if e >= HILLS {
        return if r < 20.0 && t >= 40.0 { Biome::Badlands } else { Biome::Hills };
    }
    if r < 12.0 {
        return if d >= 66.0 { Biome::Badlands } else { Biome::Desert };
    }
    if r < 30.0 {
        return if d >= 50.0 {
            Biome::Shrubland
        } else if t >= 62.0 {
            Biome::Savanna
        } else {
            Biome::Grassland
        };
    }
    if r < forest.max(30.0) {
        return if t >= 62.0 { Biome::Savanna } else { Biome::Grassland };
    }
    if t >= 62.0 {
        Biome::Jungle
    } else if t < 32.0 {
        Biome::Taiga
    } else {
        Biome::Forest
    }
}

/// Midpoint displacement (diamond-square) over a `w`×`h` grid, from a
/// lattice of points `step` apart (seeded by `coarse`), the displacement
/// shrinking by `rough` at each level.
pub fn midpoint(w: i32, h: i32, step: i32, amp: f32, rough: f32, rng: &mut Rng, coarse: &dyn Fn(i32, i32, &mut Rng) -> f32) -> Vec<f32> {
    let gw = ((w - 1 + step - 1) / step) * step + 1;
    let gh = ((h - 1 + step - 1) / step) * step + 1;
    let gi = |x: i32, y: i32| (y * gw + x) as usize;
    let mut g = vec![0.0f32; (gw * gh) as usize];
    for y in (0..gh).step_by(step as usize) {
        for x in (0..gw).step_by(step as usize) {
            g[gi(x, y)] = coarse(x, y, rng);
        }
    }
    let mut s = step;
    let mut a = amp;
    while s > 1 {
        let half = s / 2;
        // Diamonds: the middle of each square.
        for y in (half..gh).step_by(s as usize) {
            for x in (half..gw).step_by(s as usize) {
                let m = (g[gi(x - half, y - half)] + g[gi(x + half, y - half)] + g[gi(x - half, y + half)] + g[gi(x + half, y + half)]) * 0.25;
                g[gi(x, y)] = m + (rng.f64() as f32 - 0.5) * 2.0 * a;
            }
        }
        // Squares: the middle of each edge.
        for y in (0..gh).step_by(half as usize) {
            let x0 = if (y / half) % 2 == 0 { half } else { 0 };
            for x in (x0..gw).step_by(s as usize) {
                let (mut sum, mut n) = (0.0, 0.0);
                for (dx, dy) in [(-half, 0), (half, 0), (0, -half), (0, half)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx >= 0 && ny >= 0 && nx < gw && ny < gh {
                        sum += g[gi(nx, ny)];
                        n += 1.0;
                    }
                }
                g[gi(x, y)] = sum / n + (rng.f64() as f32 - 0.5) * 2.0 * a;
            }
        }
        s = half;
        a *= rough;
    }
    let mut out = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            out.push(g[gi(x, y)]);
        }
    }
    out
}

/// Rescale so the values are spread evenly over 0..100 (by rank).
pub fn uniform(v: &mut [f32]) {
    let mut order: Vec<usize> = (0..v.len()).collect();
    order.sort_by(|&a, &b| v[a].partial_cmp(&v[b]).unwrap().then(a.cmp(&b)));
    let n = (v.len().max(2) - 1) as f32;
    for (rank, &i) in order.iter().enumerate() {
        v[i] = rank as f32 / n * 100.0;
    }
}

/// Remap elevation, keeping its order, so the shares of sea, lowland,
/// hills and mountains come out the same in every world: 40% below the
/// sea, the land's 80th percentile at the hills line, its 93rd at the
/// mountain line.
#[allow(dead_code)]
pub fn proportion(elev: &mut [f32]) {
    proportion_with(elev, &Mix::default())
}

/// `proportion` with the shares of `mix`.
pub fn proportion_with(elev: &mut [f32], mix: &Mix) {
    let mut s: Vec<f32> = elev.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = s.len();
    let at = |q: f64| s[((n - 1) as f64 * q).round() as usize];
    let land = |q: f64| at(mix.sea + (1.0 - mix.sea) * q);
    let knots = [(s[0], 0.0f32), (at(mix.sea), SEA), (land(mix.hills), HILLS), (land(mix.mountains), MOUNTAIN), (s[n - 1], 255.0)];
    for v in elev.iter_mut() {
        let x = *v;
        let mut out = 255.0;
        for k in 0..knots.len() - 1 {
            let ((x0, y0), (x1, y1)) = (knots[k], knots[k + 1]);
            if x <= x1 {
                let t = if x1 > x0 { ((x - x0) / (x1 - x0)).clamp(0.0, 1.0) } else { 1.0 };
                out = y0 + (y1 - y0) * t;
                break;
            }
        }
        // Keep the sea strictly below the line and land strictly above it.
        if x < knots[1].0 {
            out = out.min(SEA - 0.5);
        } else {
            out = out.max(SEA + 0.5);
        }
        *v = out;
    }
}

/// Elevation: midpoint displacement over a continent-shaped lattice, with
/// ranges lifted along ridged noise, proportioned.
pub fn elevation(seed: u64, w: i32, h: i32) -> Vec<f32> {
    elevation_with(seed, w, h, &Mix::default())
}

/// `elevation` with the shares of `mix`.
pub fn elevation_with(seed: u64, w: i32, h: i32, mix: &Mix) -> Vec<f32> {
    let mut rng = Rng::new(seed ^ 0xE1E7_0001);
    let (wf, hf) = (w as f32, h as f32);
    // A few masses of land, placed at random, raise the lattice round them.
    let masses: Vec<(f32, f32, f32)> = (0..3 + rng.range(0, 3))
        .map(|_| (0.2 + rng.f64() as f32 * 0.6, 0.25 + rng.f64() as f32 * 0.5, 0.18 + rng.f64() as f32 * 0.16))
        .collect();
    let coarse = |x: i32, y: i32, rng: &mut Rng| -> f32 {
        let (u, v) = (x as f32 / wf, y as f32 / hf);
        let edge = ((u - 0.5).abs() * 2.0).max((v - 0.5).abs() * 2.0).powi(3);
        let lift: f32 = masses.iter().map(|&(mx, my, r)| (1.0 - (((u - mx) * 2.0).powi(2) + (v - my).powi(2)).sqrt() / r).max(0.0) * 0.35).sum();
        0.35 + rng.f64() as f32 * 0.35 + lift - edge * 0.75
    };
    let mut e = midpoint(w, h, 32, 0.28, 0.55, &mut rng, &coarse);
    // Ranges: where the ridge noise crosses its middle, bent a little.
    let ridge = Noise::new(seed ^ 0x81D6E);
    let bend = Noise::new(seed ^ 0x4EA7);
    let mut sorted = e.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let sea = sorted[sorted.len() * 2 / 5];
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f64 / 24.0, y as f64 / 12.0);
            let warp = (bend.fbm2(fx * 0.7 + 9.1, fy * 0.7, 2) - 0.5) * 1.6;
            let rn = ridge.fbm2(fx * 0.9 + warp * 0.6, fy * 0.9, 4);
            let r = (1.0 - (rn - 0.5).abs() * 9.0).max(0.0) as f32;
            let i = (y * w + x) as usize;
            let inland = ((e[i] - sea) * 8.0).clamp(0.0, 1.0);
            e[i] += r * 0.22 * inland;
        }
    }
    proportion_with(&mut e, mix);
    e
}

/// The prevailing wind at a latitude (0 equator .. 1 pole): +1 blowing
/// east (westerlies), -1 blowing west (trades and polar easterlies).
pub fn wind(lat: f32) -> i32 {
    if (0.33..0.66).contains(&lat) {
        1
    } else {
        -1
    }
}

/// Everything but the elevation, which must be final (after erosion) so
/// the temperature, rain and salt follow the land as it ends up.
#[cfg_attr(not(test), allow(dead_code))]
pub fn climate(seed: u64, w: i32, h: i32, elev: Vec<f32>) -> Grids {
    climate_with(seed, w, h, elev, &Mix::default())
}

/// `climate`, wetter or drier and warmer or colder by `mix`.
pub fn climate_with(seed: u64, w: i32, h: i32, elev: Vec<f32>, mix: &Mix) -> Grids {
    let n = (w * h) as usize;
    let mut rng = Rng::new(seed ^ 0xC11A_7E00);
    let heat = Noise::new(seed ^ 0x4EA7_0002);
    let flat = |rng: &mut Rng| -> Vec<f32> {
        let mut v = midpoint(w, h, 16, 0.5, 0.6, rng, &|_, _, r| r.f64() as f32);
        uniform(&mut v);
        v
    };
    let rain_base = flat(&mut rng);
    let drain = flat(&mut rng);
    let savage = flat(&mut rng);
    let good = flat(&mut rng);
    let lat_of = |y: i32| ((y as f32 + 0.5) / h as f32 - 0.5).abs() * 2.0;

    // Temperature: latitude, a little noise, colder with height.
    let mut temp = vec![0.0f32; n];
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            let noise = (heat.fbm2(x as f64 / 24.0, y as f64 / 12.0, 3) as f32 - 0.5) * 16.0;
            let lapse = ((elev[i] - SEA).max(0.0) / (255.0 - SEA)) * 45.0;
            temp[i] = (102.0 - lat_of(y) * 100.0 + noise - lapse + mix.warm).clamp(0.0, 100.0);
        }
    }

    // Rain carried by the wind: picked up over the sea, wrung out on the
    // climb, little left past the crest.
    let mut oro = vec![0.0f32; n];
    for y in 0..h {
        let dir = wind(lat_of(y));
        let xs: Vec<i32> = if dir > 0 { (0..w).collect() } else { (0..w).rev().collect() };
        let mut m = 1.0f32;
        for x in xs {
            let i = (y * w + x) as usize;
            let e = elev[i];
            if e < SEA {
                m = (m + 0.25).min(1.0);
                oro[i] = m;
            } else {
                // Air holds less water the higher it's lifted: what it can't
                // hold falls here, and it crosses the crest with only what
                // the crest let it keep.
                let cap = 1.0 - ((e - SEA) / (255.0 - SEA)) * 0.95;
                let excess = (m - cap).max(0.0);
                m -= excess;
                oro[i] = (m * 0.75 + excess * 3.0).min(1.0);
                m *= if e >= MOUNTAIN { 0.85 } else { 0.993 };
            }
        }
    }
    let mut rain = vec![0.0f32; n];
    for y in 0..h {
        let lat = lat_of(y);
        // Wet by the equator, dry under the subtropical highs, a damp
        // middle latitude, dry poles.
        let belt = 16.0 * (1.0 - lat / 0.15).max(0.0) - 22.0 * (1.0 - ((lat - 0.33) / 0.13).abs()).max(0.0) + 6.0 * (1.0 - ((lat - 0.55) / 0.12).abs()).max(0.0) - 14.0 * ((lat - 0.8) / 0.2).max(0.0);
        for x in 0..w {
            let i = (y * w + x) as usize;
            rain[i] = (0.35 * rain_base[i] + 70.0 * oro[i] + belt + mix.wet).clamp(0.0, 100.0);
        }
    }
    blur(&mut rain, w, h, 1);
    lipschitz(&mut rain, w, h, 15.0);

    // Vegetation: water and warmth.
    let veg: Vec<f32> = (0..n)
        .map(|i| {
            let warm = ((temp[i] - 5.0) / 25.0).clamp(0.0, 1.0);
            if elev[i] < SEA {
                0.0
            } else {
                (rain[i] * 1.1 * warm * (1.0 - ((elev[i] - MOUNTAIN) / 60.0).clamp(0.0, 0.8))).clamp(0.0, 100.0)
            }
        })
        .collect();

    // Salt: full in the sea, gone within a few cells of the coast.
    let mut dist = vec![i32::MAX; n];
    let mut queue = std::collections::VecDeque::new();
    for i in 0..n {
        if elev[i] < SEA {
            dist[i] = 0;
            queue.push_back(i);
        }
    }
    while let Some(i) = queue.pop_front() {
        let (x, y) = (i as i32 % w, i as i32 / w);
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if nx >= 0 && ny >= 0 && nx < w && ny < h {
                let j = (ny * w + nx) as usize;
                if dist[j] == i32::MAX {
                    dist[j] = dist[i] + 1;
                    queue.push_back(j);
                }
            }
        }
    }
    let salt: Vec<f32> = dist.iter().map(|&d| if d == i32::MAX { 0.0 } else { (100.0 - 34.0 * d as f32).max(0.0) }).collect();

    Grids { w, h, elev, temp, rain, drain, veg, savage, good, salt }
}

/// Box blur, `passes` times over.
pub fn blur(v: &mut [f32], w: i32, h: i32, passes: usize) {
    for _ in 0..passes {
        let src = v.to_vec();
        for y in 0..h {
            for x in 0..w {
                let (mut s, mut n) = (0.0, 0.0);
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let (nx, ny) = (x + dx, y + dy);
                        if nx >= 0 && ny >= 0 && nx < w && ny < h {
                            s += src[(ny * w + nx) as usize];
                            n += 1.0;
                        }
                    }
                }
                v[(y * w + x) as usize] = s / n;
            }
        }
    }
}

/// Lower the field wherever it climbs faster than `l` a cell (in any of
/// the eight directions): the lower envelope of cones on every value, so
/// neighbours never differ by more than `l`.
pub fn lipschitz(v: &mut [f32], w: i32, h: i32, l: f32) {
    let idx = |x: i32, y: i32| (y * w + x) as usize;
    for y in 0..h {
        for x in 0..w {
            let mut m = v[idx(x, y)];
            for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && ny >= 0 && nx < w && ny < h {
                    m = m.min(v[idx(nx, ny)] + l);
                }
            }
            v[idx(x, y)] = m;
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let mut m = v[idx(x, y)];
            for (dx, dy) in [(1, 1), (0, 1), (-1, 1), (1, 0)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && ny >= 0 && nx < w && ny < h {
                    m = m.min(v[idx(nx, ny)] + l);
                }
            }
            v[idx(x, y)] = m;
        }
    }
}

/// Bytes as base64 text in a save.
pub mod b64 {
    use serde::{Deserialize, Deserializer, Serializer};

    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    pub fn encode(v: &[u8]) -> String {
        let mut s = String::with_capacity(v.len() * 4 / 3 + 4);
        for c in v.chunks(3) {
            let b = [c[0], *c.get(1).unwrap_or(&0), *c.get(2).unwrap_or(&0)];
            let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
            for k in 0..4 {
                if k <= c.len() {
                    s.push(ABC[(n >> (18 - 6 * k)) as usize & 63] as char);
                } else {
                    s.push('=');
                }
            }
        }
        s
    }

    pub fn decode(s: &str) -> Vec<u8> {
        let val = |c: u8| ABC.iter().position(|&a| a == c).unwrap_or(0) as u32;
        let bytes: Vec<u8> = s.bytes().filter(|&c| c != b'=').collect();
        let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
        for c in bytes.chunks(4) {
            let mut n = 0u32;
            for k in 0..4 {
                n = n << 6 | c.get(k).map_or(0, |&b| val(b));
            }
            let take = c.len() - 1;
            for k in 0..take {
                out.push((n >> (16 - 8 * k)) as u8);
            }
        }
        out
    }

    pub fn serialize<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&encode(v))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        Ok(decode(&s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ridge_casts_a_rain_shadow() {
        // Sea at both ends, plains between, a ridge across the middle.
        let (w, h) = (100, 60);
        let elev: Vec<f32> = (0..w * h)
            .map(|i| {
                let x = i % w;
                if x < 5 || x >= 95 {
                    60.0
                } else if (48..52).contains(&x) {
                    210.0
                } else {
                    120.0
                }
            })
            .collect();
        let g = climate(9, w, h, elev);
        let (mut wet, mut dry) = (0.0, 0.0);
        for y in 0..h {
            let lat = ((y as f32 + 0.5) / h as f32 - 0.5).abs() * 2.0;
            let dir = wind(lat);
            wet += g.rain[g.idx(50 - 6 * dir, y)];
            dry += g.rain[g.idx(50 + 6 * dir, y)];
        }
        eprintln!("windward {:.1} leeward {:.1}", wet / h as f32, dry / h as f32);
        assert!(wet > dry + 10.0 * h as f32);
    }

    #[test]
    fn base64_round_trips() {
        for n in 0..10 {
            let v: Vec<u8> = (0..n * 7 + n).map(|i| (i * 37 % 256) as u8).collect();
            assert_eq!(b64::decode(&b64::encode(&v)), v);
        }
    }

    #[test]
    fn neighbours_never_differ_by_more_than_the_limit() {
        let (w, h) = (40, 30);
        let mut rng = Rng::new(3);
        let mut v: Vec<f32> = (0..w * h).map(|_| rng.f64() as f32 * 100.0).collect();
        lipschitz(&mut v, w, h, 15.0);
        for y in 0..h {
            for x in 0..w {
                for (dx, dy) in [(1, 0), (0, 1), (1, 1), (-1, 1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx >= 0 && ny >= 0 && nx < w && ny < h {
                        assert!((v[(y * w + x) as usize] - v[(ny * w + nx) as usize]).abs() <= 15.001);
                    }
                }
            }
        }
    }
}
