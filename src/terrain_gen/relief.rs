//! The lie of the land before weather touches it: the world map's heights
//! set the level and the slope across the map, and each world cell's biome
//! (and its own character) picks the landform: flat plains, gentle rolling
//! hills, steep hills, ridged mountain ranges, stepped mesas, dunes, the
//! sea floor. A fault may lift one side of the map into an escarpment.

use super::field::{cell_hash, fbm, ridged, smoothstep, warp, Grid};
use super::Ctx;
use crate::noise::Noise;
use crate::overworld::Biome;
use crate::rng::Rng;
use crate::terrain::feat;

/// How a stretch of land is shaped.
#[derive(Clone, Copy, Debug, Default)]
pub struct Form {
    /// Rolling hills: how high, in levels.
    pub amp: f64,
    /// Ridged ranges: how high their crests stand.
    pub ridge: f64,
    /// Width of the hills, in tiles.
    pub scale: f64,
    /// 0..1: how much the land climbs in flat steps (mesas).
    pub terrace: f64,
    /// Height of each step.
    pub step: f64,
    /// 0..1: how hard the rock is (hard rock holds a cliff, soft slumps).
    pub hard: f64,
    /// Sand dunes: how high.
    pub dunes: f64,
}

impl Form {
    fn add(&mut self, o: &Form, k: f64) {
        self.amp += o.amp * k;
        self.ridge += o.ridge * k;
        self.scale += o.scale * k;
        self.terrace += o.terrace * k;
        self.step += o.step * k;
        self.hard += o.hard * k;
        self.dunes += o.dunes * k;
    }
}

/// The usual shape of a biome's land.
pub fn biome_form(b: Biome) -> Form {
    let f = |amp, ridge, scale, terrace, step, hard, dunes| Form { amp, ridge, scale, terrace, step, hard, dunes };
    match b {
        Biome::Grassland => f(2.4, 0.0, 44.0, 0.0, 3.0, 0.3, 0.0),
        Biome::Savanna => f(2.0, 0.0, 46.0, 0.0, 3.0, 0.35, 0.0),
        Biome::Tundra => f(2.8, 0.0, 38.0, 0.0, 3.0, 0.5, 0.0),
        Biome::Forest => f(4.5, 0.0, 34.0, 0.0, 3.0, 0.35, 0.0),
        Biome::Taiga => f(4.5, 1.0, 34.0, 0.0, 3.0, 0.5, 0.0),
        Biome::Jungle => f(5.5, 0.0, 30.0, 0.0, 3.0, 0.3, 0.0),
        Biome::Hills => f(8.0, 2.5, 28.0, 0.15, 3.0, 0.5, 0.0),
        Biome::Mountain => f(4.0, 10.0, 18.0, 0.6, 3.0, 0.85, 0.0),
        Biome::Glacier => f(4.0, 7.0, 22.0, 0.35, 3.0, 0.7, 0.0),
        Biome::Badlands => f(6.5, 0.0, 24.0, 0.95, 3.0, 0.65, 0.0),
        Biome::Desert => f(1.6, 0.0, 40.0, 0.0, 3.0, 0.3, 2.2),
        Biome::Swamp => f(1.1, 0.0, 30.0, 0.0, 3.0, 0.15, 0.0),
        Biome::Ocean | Biome::Lake => f(2.0, 0.0, 40.0, 0.0, 3.0, 0.3, 0.0),
        Biome::Shrubland => f(2.6, 0.0, 42.0, 0.0, 3.0, 0.4, 0.0),
        Biome::Marsh => f(1.0, 0.0, 30.0, 0.0, 3.0, 0.15, 0.0),
    }
}

/// A world cell's own landform: its biome's, given a character of its own
/// (this stretch of grassland flat as a table, that one rolling), and
/// roughened where the world's heights change fast round it.
pub fn cell_form(b: Biome, cx: i32, cy: i32, spread: f64) -> Form {
    let mut f = biome_form(b);
    let c = cell_hash(cx, cy, 0x1A9D);
    match b {
        Biome::Grassland | Biome::Savanna | Biome::Shrubland | Biome::Tundra | Biome::Forest | Biome::Taiga | Biome::Jungle => {
            if c < 0.35 {
                // Flat plains.
                f.amp *= 0.35;
            } else if c > 0.72 {
                // Rolling hill country.
                f.amp *= 2.0;
                f.scale *= 0.8;
            }
        }
        Biome::Hills => {
            if c < 0.3 {
                f.amp *= 0.7;
                f.scale *= 1.3;
            } else if c > 0.72 {
                f.amp *= 1.35;
                f.ridge += 4.0;
                f.hard += 0.2;
            }
        }
        Biome::Mountain | Biome::Glacier => f.ridge *= 0.8 + c * 0.5,
        Biome::Badlands => f.step = if c < 0.5 { 3.0 } else { 4.0 },
        _ => {}
    }
    f.amp += (spread / 22.0).min(4.0);
    f
}

/// Height of every column, as a float level, before any weathering, and
/// how hard the rock is under each.
pub fn heights(cx: &Ctx, rng: &mut Rng) -> (Grid, Grid, Grid, Grid) {
    let (w, h) = (cx.w, cx.h);
    let p = cx.p;
    let warp_n = Noise::new(cx.seed ^ 0x3A01);
    let roll_n = Noise::new(cx.seed);
    let detail_n = Noise::new(cx.seed ^ 0x3A02);
    let ridge_n = Noise::new(cx.seed ^ 0xA1A1);
    let dune_n = Noise::new(cx.seed ^ 0x3A03);
    let hard_n = Noise::new(cx.seed ^ 0x3A04);
    let esc_n = Noise::new(cx.seed ^ 0x3A05);
    let has = |b: u32| p.features & b != 0;

    // Each world cell's form, cached.
    let mut forms: Vec<((i32, i32), Form)> = Vec::new();
    let mut form_of = |c: (i32, i32)| -> Form {
        if let Some(f) = forms.iter().find(|f| f.0 == c) {
            return f.1;
        }
        let b = cx.cell_biome(c);
        let mut lo = 255.0f64;
        let mut hi = 0.0f64;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if let Some(a) = cx.around(c.0 + dx, c.1 + dy) {
                    if !a.biome.is_water() {
                        lo = lo.min(a.elev as f64);
                        hi = hi.max(a.elev as f64);
                    }
                }
            }
        }
        let mut f = cell_form(b, c.0, c.1, (hi - lo).max(0.0));
        if p.around.is_empty() {
            f = biome_form(b);
            f.amp *= p.relief.max(0.3) / biome_relief(b);
        }
        if has(feat::PEAKS) {
            f.ridge += 6.0;
            f.hard = f.hard.max(0.8);
        }
        if has(feat::MESAS) {
            f.terrace = f.terrace.max(0.9);
            f.hard = f.hard.max(0.65);
            f.amp = f.amp.max(4.0);
        }
        forms.push((c, f));
        f
    };

    // An escarpment: a fault across the map with one side lifted, broken
    // here and there where the scarp has worn down.
    let esc_odds = match p.biome {
        Biome::Hills | Biome::Badlands => 0.45,
        Biome::Mountain | Biome::Savanna => 0.35,
        Biome::Grassland | Biome::Forest | Biome::Tundra | Biome::Taiga | Biome::Desert => 0.22,
        _ => 0.0,
    };
    let escarp = (rng.chance(esc_odds) && !p.around.is_empty()).then(|| {
        let ang = rng.f64() * std::f64::consts::TAU;
        let (bw, bh) = (cx.base.0 as f64, cx.base.1 as f64);
        let off = (0.22 + rng.f64() * 0.16) * bw.min(bh);
        let (nx, ny) = (ang.cos(), ang.sin());
        let (px, py) = (bw / 2.0 + nx * off, bh / 2.0 + ny * off);
        let rise = 3.0 + rng.f64() * 3.5;
        // Which side stands high: away from the middle, so the middle stays
        // on the low, open side.
        (px, py, nx, ny, rise)
    });

    let mut hg = Grid::new(w, h, 0.0);
    let mut hard = Grid::new(w, h, 0.0);
    let mut ter = Grid::new(w, h, 0.0);
    let mut step = Grid::new(w, h, 3.0);
    for y in 0..h {
        for x in 0..w {
            let (ax, ay) = ((x + cx.ox) as f64, (y + cx.oy) as f64);
            // Blend the forms of the four nearest world cells.
            let (fx, fy) = (ax / super::CELL - 0.5, ay / super::CELL - 0.5);
            let (i0, j0) = (fx.floor(), fy.floor());
            let s = |t: f64| t * t * (3.0 - 2.0 * t);
            let (tx, ty) = (s(fx - i0), s(fy - j0));
            let mut f = Form::default();
            for (di, dj, k) in [(0, 0, (1.0 - tx) * (1.0 - ty)), (1, 0, tx * (1.0 - ty)), (0, 1, (1.0 - tx) * ty), (1, 1, tx * ty)] {
                let c = (cx.p.home.0 + i0 as i32 + di, cx.p.home.1 + j0 as i32 + dj);
                f.add(&form_of(c), k);
            }
            let (wx, wy) = warp(&warp_n, ax, ay, 56.0, 20.0);
            let mut z = cx.base_z(ax, ay);
            z += fbm(&roll_n, wx / f.scale, wy / f.scale, 5) * f.amp * 1.7;
            z += fbm(&detail_n, ax / 6.0, ay / 6.0, 2) * 0.5 * (f.amp / 4.0).min(1.2);
            if f.ridge > 0.0 {
                // Ranges stand on broad massifs, cut by valleys.
                let r = ridged(&ridge_n, wx / (f.scale * 1.25), wy / (f.scale * 1.25), 5);
                let massif = smoothstep(-0.25, 0.3, fbm(&ridge_n, wx / 70.0 + 50.0, wy / 70.0, 3) as f32) as f64;
                z += ((r - 0.3) * 1.4 + massif * 0.8) * f.ridge;
            }
            if f.dunes > 0.0 {
                let ph = (wx * 0.8 + wy * 0.6) / 4.2 + fbm(&dune_n, ax / 20.0, ay / 20.0, 2) * 5.0;
                let crest = 0.5 + 0.5 * ph.sin();
                let field = (fbm(&dune_n, ax / 30.0 + 9.0, ay / 30.0, 2) + 0.6).clamp(0.0, 1.0);
                z += crest * crest * f.dunes * field;
            }
            let mut hd = f.hard + fbm(&hard_n, ax / 22.0, ay / 22.0, 3) * 0.35;
            if let Some((px, py, nx, ny, rise)) = escarp {
                let along = (ax - px) * -ny + (ay - py) * nx;
                let sd = (ax - px) * nx + (ay - py) * ny + fbm(&esc_n, along / 26.0, 3.7, 3) * 9.0;
                let worn = smoothstep(-0.15, 0.25, fbm(&esc_n, along / 17.0, 11.1, 2) as f32) as f64;
                let up = smoothstep(-1.0, 1.0, sd as f32) as f64;
                z += rise * up * (0.35 + 0.65 * worn);
                if sd > -1.0 && sd < 6.0 {
                    hd += 0.45 * worn;
                }
            }
            if let Some((vx, vy)) = cx.volcano {
                let r = ((ax - vx as f64).powi(2) + (ay - vy as f64).powi(2)).sqrt();
                if r < 18.0 {
                    z += (1.0 - r / 18.0).powf(1.4) * 13.0;
                    hd = hd.max(0.75);
                }
            }
            if let Some((vx, vy)) = cx.oasis {
                let r = ((ax - vx as f64).powi(2) + (ay - vy as f64).powi(2)).sqrt();
                if r < 9.0 {
                    z -= (1.0 - r / 9.0) * 3.0;
                }
            }
            let i = hg.idx(x, y);
            hg.v[i] = z as f32;
            hard.v[i] = hd.clamp(0.05, 1.0) as f32;
            ter.v[i] = f.terrace.min(1.0) as f32;
            step.v[i] = f.step.max(2.0) as f32;
        }
    }
    (hg, hard, ter, step)
}

/// Hard rock weathers in steps: flat benches and tops between faces (the
/// mesas of the badlands, the crags of the mountains), done once the land
/// has its final heights so a step is a few whole levels.
pub fn terrace(hg: &mut Grid, hard: &Grid, ter: &Grid, step: &Grid, floor: f32) {
    for i in 0..hg.v.len() {
        let k = ter.v[i] * smoothstep(0.35, 0.7, hard.v[i]).max(0.5);
        if k < 0.05 || hg.v[i] <= floor {
            continue;
        }
        let st = step.v[i];
        let z = hg.v[i] - floor;
        let t = z / st;
        let fl = t.floor();
        let fr = smoothstep(0.55, 0.8, t - fl);
        hg.v[i] = floor + z + ((fl + fr) * st - z) * k.min(1.0);
    }
}

/// The relief the old fixed table gave a biome (to honour a hand-set
/// `relief` when there's no world round the map).
fn biome_relief(b: Biome) -> f64 {
    b.params().relief
}

/// Push the sea floor under the sea, shelving down from the shore, and
/// soften the land just above it into a shore (cliffs where the rock is
/// hard). `dist` is each column's distance from the shoreline, in tiles,
/// negative out to sea.
pub fn sea_floor(hg: &mut Grid, hard: &Grid, dist: &[f32], sea: f32) {
    for i in 0..hg.v.len() {
        let d = dist[i];
        if d < 0.0 {
            let floor = sea - 1.2 - (-d / 3.0).min(4.0);
            hg.v[i] = hg.v[i].min(floor);
        } else if d < 6.0 && hard.v[i] < 0.6 {
            // A beach: low and gentle by the water.
            let cap = sea + 0.2 + d * 0.45;
            hg.v[i] = hg.v[i].min(cap);
        }
    }
}
