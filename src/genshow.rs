//! Watching a world being made. The generator keeps a trace of what it did
//! (`WorldGen::traced`): the elevation as first raised, the erosion phase's
//! trial rivers in the order they ran, the elevation they left, and each
//! site's founding and fate with the clashes of history. `Show` plays that
//! back in stages, each frame a pure function of the time since the start:
//!
//! 1. an empty sea; 2. land rising as the sea falls; 3. mountains thrust up
//! (volcanoes erupting where the world has them); 4. weathering, the trial
//! rivers running down the heights; 5. rivers growing from their springs to
//! the sea and lakes filling their basins; 6. rain and warmth bringing the
//! biomes in (a wildfire or two sweeping dry land and greening back);
//! 7. the first peoples founding their capitals one by one, and the great
//! beasts waking in their lairs; 8. history, age by age: settlements
//! springing up, raiders running between them, ruins smouldering where they
//! fell, each age given the same few seconds whatever its length so its
//! theme can be read (`chronicle`), and the great events (`overworld/epics.rs`)
//! held on screen while their shockwave spreads. The last frame is the
//! finished map. Each stage fades into the next rather than cutting.
//!
//! Fires, lava and smoke are flourishes of the playback only.

pub mod chronicle;

use crate::overworld::epics::{Epic, EpicKind};
use crate::overworld::spread::gap_between;
use crate::overworld::{river_glyph, Biome, ClashEnd, Overworld, SiteKind, SiteState, WorldGen};
use chronicle::Era;
use crate::worldgen::fields::{HILLS, MOUNTAIN, SEA};
use ratatui::style::Color;

/// A cell as drawn: glyph, foreground, background.
pub type Look = (char, Color, Color);

const BLACK: Color = Color::Rgb(0, 0, 0);

/// The stages of the show, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    Ocean,
    Rising,
    Mountains,
    Weathering,
    Rivers,
    Life,
    Founding,
    History,
    Done,
}

impl Stage {
    pub const ALL: [Stage; 9] = [Stage::Ocean, Stage::Rising, Stage::Mountains, Stage::Weathering, Stage::Rivers, Stage::Life, Stage::Founding, Stage::History, Stage::Done];

    /// What the stage shows, for the caption.
    pub fn caption(self) -> &'static str {
        match self {
            Stage::Ocean => "An empty sea rolls under the sky",
            Stage::Rising => "Land rises as the sea falls away",
            Stage::Mountains => "Mountains are thrust up",
            Stage::Weathering => "Wind and water wear the heights down",
            Stage::Rivers => "Rivers find their way to the sea",
            Stage::Life => "Rain and warmth bring the land to life",
            Stage::Founding => "The first peoples raise their halls",
            Stage::History => "The peoples spread across the land",
            Stage::Done => "History is written",
        }
    }

    /// A short name for the list of stages.
    pub fn short(self) -> &'static str {
        match self {
            Stage::Ocean => "Ocean",
            Stage::Rising => "Land rising",
            Stage::Mountains => "Mountains forming",
            Stage::Weathering => "Weathering",
            Stage::Rivers => "Rivers forming",
            Stage::Life => "Climate and life",
            Stage::Founding => "The first peoples",
            Stage::History => "History",
            Stage::Done => "The finished world",
        }
    }
}

/// Seconds each stage before history takes at normal speed.
const DUR: [f64; 7] = [2.5, 6.0, 6.0, 5.5, 6.0, 6.5, 6.0];
/// The stage index of history.
const HIST: usize = 7;
/// The finished world held still after the last year, before it's done.
const HOLD: f64 = 2.0;
/// How fast a trial river's head runs, cells a second.
const PATH_SPEED: f64 = 45.0;
/// How long a flash (a founding, a clash) lasts, seconds.
const FLASH: f64 = 1.2;
/// How long one stage takes to fade into the next, seconds.
const XF: f64 = 1.0;
/// How long the show lingers on a great event, seconds.
pub const EPIC_HOLD: f64 = 5.0;

/// History's pacing, worked out once it's all written: when each year
/// begins (seconds from the start of history), the ages, and the great
/// events with when the show reaches them.
struct Timeline {
    ys: Vec<f64>,
    eras: Vec<Era>,
}

/// One frame: every cell's look, and which cells hold a site (so a
/// shrunken map lets them show through).
pub struct ShowFrame {
    pub looks: Vec<Look>,
    pub site: Vec<bool>,
}

struct Fire {
    /// Cells it reaches and how many steps from where it started.
    cells: Vec<(u32, u8)>,
    reach: u8,
    /// When it starts, in the life stage (0..1).
    start: f64,
}

/// The playback of a world's making.
pub struct Show {
    w: i32,
    h: i32,
    /// Elevation as first raised, after the erosion phase, and as kept.
    raw: Vec<f32>,
    eroded: Vec<f32>,
    fin: Vec<f32>,
    raw_max: f32,
    /// Trial rivers to draw (a sample of them) and when each starts, in
    /// the weathering stage (0..1).
    paths: Vec<(f64, Vec<u32>)>,
    /// When each river cell, and each lake cell, appears in the rivers
    /// stage, and each land cell comes to life in the life stage (0..1).
    river_key: Vec<f32>,
    lake_key: Vec<f32>,
    life_key: Vec<f32>,
    /// Volcano cells and when each erupts in the mountains stage (0..1).
    volcanoes: Vec<(usize, f64)>,
    fires: Vec<Fire>,
    history: i32,
    /// Start of each stage, seconds; `starts[8]` is when it's all done.
    starts: [f64; 9],
    /// History's pacing, once history is written (`sync`).
    timeline: Option<Timeline>,
    /// How fast the show plays (the + and - keys).
    pub speed: f64,
}

fn hash(x: i32, y: i32, salt: u64) -> u64 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ salt.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^= h >> 31;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 29;
    h
}

fn hash01(x: i32, y: i32, salt: u64) -> f32 {
    (hash(x, y, salt) % 10_000) as f32 / 10_000.0
}

fn smooth(p: f64) -> f64 {
    let p = p.clamp(0.0, 1.0);
    p * p * (3.0 - 2.0 * p)
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

pub fn blend(a: Color, b: Color, t: f32) -> Color {
    match (a, b) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
            let t = t.clamp(0.0, 1.0);
            let m = |p: u8, q: u8| (p as f32 + (q as f32 - p as f32) * t).round() as u8;
            Color::Rgb(m(r1, r2), m(g1, g2), m(b1, b2))
        }
        _ => b,
    }
}

impl Show {
    /// Build the playback once the generator's land is made (it reads the
    /// land and the trace; history is read as it's written, each frame).
    pub fn new(g: &WorldGen) -> Show {
        let ow = &g.ow;
        let (w, h) = (ow.w, ow.h);
        let n = (w * h) as usize;
        let fin: Vec<f32> = ow.cells.iter().map(|c| c.elev as f32).collect();
        let tr = g.trace.as_ref().and_then(|t| t.land.as_ref()).filter(|t| t.raw.len() == n);
        let raw = tr.map_or_else(|| fin.clone(), |t| t.raw.clone());
        let eroded = tr.map_or_else(|| fin.clone(), |t| t.eroded.clone());
        let raw_max = raw.iter().copied().fold(SEA, f32::max);

        // A sample of the trial rivers: enough to see the heights worn,
        // few enough to follow each one.
        let all: Vec<&Vec<u32>> = tr.map_or(Vec::new(), |t| t.paths.iter().filter(|p| p.len() >= 4).collect());
        let stride = (all.len() / 500).max(1);
        let picked: Vec<&Vec<u32>> = all.into_iter().step_by(stride).collect();
        let m = picked.len().max(1) as f64;
        let paths = picked.into_iter().enumerate().map(|(k, p)| (0.78 * k as f64 / m, p.clone())).collect();

        // Rivers grow with their flow: springs first, the great mouths last,
        // so tributaries reach their stems as the stems pass.
        let flow = |i: usize| *ow.fields.flow.get(i).unwrap_or(&150) as f32;
        let rcells: Vec<usize> = (0..n).filter(|&i| ow.cells[i].river != 0).collect();
        let fmin = rcells.iter().map(|&i| flow(i)).fold(f32::MAX, f32::min);
        let fmax = rcells.iter().map(|&i| flow(i)).fold(f32::MIN, f32::max);
        let mut river_key = vec![2.0f32; n];
        for &i in &rcells {
            river_key[i] = 0.04 + 0.82 * (flow(i) - fmin) / (fmax - fmin).max(1.0);
        }

        // Lakes fill from the bottom of each basin to its rim.
        let mut lake_key = vec![2.0f32; n];
        let mut seen = vec![false; n];
        for s in 0..n {
            if seen[s] || ow.cells[s].biome != Biome::Lake {
                continue;
            }
            let mut comp = vec![s];
            seen[s] = true;
            let mut k = 0;
            while k < comp.len() {
                let i = comp[k];
                k += 1;
                let (x, y) = (i as i32 % w, i as i32 / w);
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx >= 0 && ny >= 0 && nx < w && ny < h {
                        let j = (ny * w + nx) as usize;
                        if !seen[j] && ow.cells[j].biome == Biome::Lake {
                            seen[j] = true;
                            comp.push(j);
                        }
                    }
                }
            }
            let lo = comp.iter().map(|&i| eroded[i]).fold(f32::MAX, f32::min);
            let hi = comp.iter().map(|&i| eroded[i].max(fin[i])).fold(f32::MIN, f32::max);
            for &i in &comp {
                lake_key[i] = 0.2 + 0.65 * (eroded[i] - lo) / (hi - lo).max(0.01);
            }
        }

        // Life comes first to the cold heights (snow), then spreads from
        // the wettest land to the driest.
        let field = |v: &Vec<u8>, i: usize| *v.get(i).unwrap_or(&50) as f32;
        let mut life_key = vec![0.0f32; n];
        for i in 0..n {
            let (x, y) = (i as i32 % w, i as i32 / w);
            let (temp, rain) = (field(&ow.fields.temp, i), field(&ow.fields.rain, i));
            life_key[i] = match ow.cells[i].biome {
                Biome::Glacier | Biome::Tundra => 0.02 + 0.12 * (temp / 22.0).clamp(0.0, 1.0),
                Biome::Mountain => 0.05 + 0.1 * hash01(x, y, 7),
                _ => 0.14 + 0.36 * (1.0 - rain / 100.0) + 0.04 * hash01(x, y, 11),
            };
        }

        let vents: Vec<usize> = (0..n).filter(|&i| ow.cells[i].volcano).collect();
        let nv = vents.len().max(2) - 1;
        let volcanoes = vents.into_iter().enumerate().map(|(k, i)| (i, 0.38 + 0.3 * k as f64 / nv as f64)).collect();

        let fires = Show::fires(ow);
        let history = g.params.history.max(1);
        let mut starts = [0.0; 9];
        for k in 0..HIST {
            starts[k + 1] = starts[k] + DUR[k];
        }
        starts[8] = starts[HIST] + Show::history_secs(history) + HOLD;
        let mut show = Show { w, h, raw, eroded, fin, raw_max, paths, river_key, lake_key, life_key, volcanoes, fires, history, starts, timeline: None, speed: 1.0 };
        show.sync(g);
        show
    }

    /// Seconds the years of history take before the ages are known.
    fn history_secs(history: i32) -> f64 {
        (history as f64 * 0.09).clamp(8.0, 20.0)
    }

    /// Once history is all written, pace it: every age gets the same few
    /// seconds whatever its length (so a long quiet age passes as quickly
    /// as a short bloody one), and each great event holds the year it
    /// happens in for `EPIC_HOLD`.
    pub fn sync(&mut self, g: &WorldGen) {
        if self.timeline.is_some() || !g.done() {
            return;
        }
        let none = Vec::new();
        let clashes = g.trace.as_ref().map_or(&none, |t| &t.clashes);
        let eras = chronicle::eras(&g.ow, clashes, self.history);
        // A few seconds an age, a little more for a long one.
        let per_era = |len: i32| (6.0 + 0.05 * len as f64).min(15.0);
        let hy = self.history as usize;
        let mut ys = vec![0.0; hy + 2];
        let mut t = 0.0;
        for y in 0..=hy {
            ys[y] = t;
            let len = eras.iter().find(|e| e.from <= y as i32 && y as i32 <= e.to).map_or(10, |e| e.to - e.from + 1);
            t += per_era(len) / len as f64;
            if g.ow.epics.iter().any(|e| e.year == y as i32) {
                t += EPIC_HOLD;
            }
        }
        ys[hy + 1] = t;
        self.starts[8] = self.starts[HIST] + t + HOLD;
        self.timeline = Some(Timeline { ys, eras });
    }

    /// The ages of history, once it's written.
    pub fn eras(&self) -> &[Era] {
        self.timeline.as_ref().map_or(&[], |t| &t.eras)
    }

    /// The age the show is in at `t`, and its index.
    pub fn era_at(&self, t: f64) -> Option<(usize, &Era)> {
        let y = self.year_at(t)?;
        self.eras().iter().enumerate().find(|(_, e)| e.from <= y && y <= e.to)
    }

    /// The great event the show is lingering on at `t`, and how far through.
    pub fn epic_at<'a>(&self, t: f64, g: &'a WorldGen) -> Option<(&'a Epic, f64)> {
        self.timeline.as_ref()?;
        g.ow.epics.iter().find_map(|e| {
            let age = t - self.time_of_year(e.year);
            (0.0..EPIC_HOLD).contains(&age).then_some((e, age / EPIC_HOLD))
        })
    }

    /// The next place worth skipping to after `t`: the next stage, age or
    /// great event.
    pub fn next_mark(&self, t: f64) -> f64 {
        let mut marks: Vec<f64> = self.starts.to_vec();
        for e in self.eras() {
            marks.push(self.time_of_year(e.from));
        }
        marks.into_iter().filter(|&m| m > t + 0.05).fold(self.end(), f64::min)
    }

    /// A wildfire or two on dry, warm grass and scrub, spreading into the
    /// burnable land round it. Picked from the cells by a hash, so the
    /// generator's random streams are untouched.
    fn fires(ow: &Overworld) -> Vec<Fire> {
        let (w, h) = (ow.w, ow.h);
        let n = (w * h) as usize;
        let burns = |i: usize| ow.cells[i].river == 0 && matches!(ow.cells[i].biome, Biome::Grassland | Biome::Savanna | Biome::Shrubland | Biome::Forest | Biome::Taiga);
        let field = |v: &Vec<u8>, i: usize| *v.get(i).unwrap_or(&50) as f32;
        let mut cand: Vec<usize> = (0..n)
            .filter(|&i| burns(i) && matches!(ow.cells[i].biome, Biome::Grassland | Biome::Savanna | Biome::Shrubland) && field(&ow.fields.rain, i) < 40.0 && field(&ow.fields.temp, i) >= 40.0)
            .collect();
        cand.sort_by_key(|&i| hash(i as i32 % w, i as i32 / w, ow.seed ^ 0xF1AE));
        let mut origins: Vec<usize> = Vec::new();
        for &c in &cand {
            let far = origins.iter().all(|&o| (o as i32 % w - c as i32 % w).abs() + (o as i32 / w - c as i32 / w).abs() > 30);
            if far {
                origins.push(c);
            }
            if origins.len() >= 2 {
                break;
            }
        }
        origins
            .iter()
            .enumerate()
            .map(|(k, &o)| {
                let reach = 8u8;
                let mut dist = vec![u8::MAX; n];
                dist[o] = 0;
                let mut cells = vec![(o as u32, 0u8)];
                let mut q = 0;
                while q < cells.len() {
                    let (i, d) = cells[q];
                    q += 1;
                    if d >= reach {
                        continue;
                    }
                    let (x, y) = (i as i32 % w, i as i32 / w);
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            let (nx, ny) = (x + dx, y + dy);
                            if nx < 0 || ny < 0 || nx >= w || ny >= h {
                                continue;
                            }
                            let j = (ny * w + nx) as usize;
                            // Not every cell catches: the front is ragged.
                            if dist[j] == u8::MAX && burns(j) && hash(nx, ny, 0xB0B) % 5 != 0 {
                                dist[j] = d + 1;
                                cells.push((j as u32, d + 1));
                            }
                        }
                    }
                }
                Fire { cells, reach, start: 0.56 + 0.07 * k as f64 }
            })
            .collect()
    }

    /// When the whole show is over (seconds).
    pub fn end(&self) -> f64 {
        self.starts[8]
    }

    /// The stage at time `t` and how far through it (0..1).
    pub fn stage_at(&self, t: f64) -> (Stage, f64) {
        if t >= self.starts[8] {
            return (Stage::Done, 1.0);
        }
        for k in (0..8).rev() {
            if t >= self.starts[k] {
                let len = if k == HIST { self.starts[8] - self.starts[HIST] } else { DUR[k] };
                return (Stage::ALL[k], ((t - self.starts[k]) / len).clamp(0.0, 1.0));
            }
        }
        (Stage::Ocean, 0.0)
    }

    /// When a stage starts (seconds).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn start_of(&self, s: Stage) -> f64 {
        self.starts[Stage::ALL.iter().position(|&x| x == s).unwrap()]
    }

    /// The year of history at `t`, once history has begun.
    pub fn year_at(&self, t: f64) -> Option<i32> {
        if t < self.starts[HIST] {
            return None;
        }
        let rel = t - self.starts[HIST];
        match &self.timeline {
            Some(tl) => Some((tl.ys.partition_point(|&s| s <= rel) as i32 - 1).clamp(0, self.history)),
            None => {
                let q = (rel / Show::history_secs(self.history)).clamp(0.0, 1.0);
                Some(((q * self.history as f64).floor() as i32).min(self.history))
            }
        }
    }

    /// When year `y` begins on the show's clock.
    pub fn time_of_year(&self, y: i32) -> f64 {
        match &self.timeline {
            Some(tl) => self.starts[HIST] + tl.ys[y.clamp(0, self.history + 1) as usize],
            None => self.starts[HIST] + Show::history_secs(self.history) * y as f64 / self.history as f64,
        }
    }

    /// How far the show can run with what the generator has made so far:
    /// the land until the first peoples are placed, then up to history,
    /// which waits until it's all written and paced (`sync`).
    pub fn ready_until(&self, g: &WorldGen) -> f64 {
        if g.done() && self.timeline.is_some() {
            f64::INFINITY
        } else if !g.history_ready() {
            self.starts[HIST - 1] - 1e-3
        } else {
            self.starts[HIST] - 1e-3
        }
    }

    fn sea(&self, x: i32, y: i32, e: f32, t: f64, amp: f64) -> Look {
        let base = if e > 75.0 { rgb(50, 100, 210) } else { rgb(30, 70, 180) };
        if amp > 0.0 {
            let (fx, fy) = (x as f64, y as f64);
            let ph = (fx * 0.31 + fy * 0.83 - t * 1.7).sin() + 0.6 * (fx * 0.11 - fy * 0.27 + t * 0.9).sin();
            if ph > 1.12 + (1.0 - amp) * 2.0 {
                return ('~', rgb(110, 160, 235), BLACK);
            }
        }
        ('≈', base, BLACK)
    }

    /// Land with nothing living on it yet, by its height.
    fn bare(&self, x: i32, y: i32, e: f32) -> Look {
        let hs = hash(x, y, 3);
        let alt = hs % 3 == 0;
        if e >= MOUNTAIN {
            // Grey rock whitening towards the peaks.
            let snow = ((e - 185.0) / 40.0).clamp(0.0, 1.0);
            let rock = if alt { rgb(135, 125, 115) } else { rgb(115, 108, 100) };
            let ch = if e > 195.0 || hs % 3 != 1 { '▲' } else { '^' };
            (ch, blend(rock, rgb(240, 240, 250), snow), BLACK)
        } else if e >= HILLS {
            (if alt { 'n' } else { '∩' }, if alt { rgb(150, 115, 80) } else { rgb(130, 100, 70) }, BLACK)
        } else {
            let tint = ((e - SEA) / (HILLS - SEA)).clamp(0.0, 1.0);
            let soil = blend(rgb(125, 100, 70), rgb(150, 120, 85), tint);
            (if hs % 4 == 0 { ':' } else { '·' }, if alt { blend(soil, rgb(0, 0, 0), 0.15) } else { soil }, BLACK)
        }
    }

    /// The kept look of a cell, without its site (the land under it).
    fn land_look(ow: &Overworld, x: i32, y: i32) -> Look {
        let (ch, fg) = ow.look(x, y);
        (ch, fg, if ow.cell(x, y).evil { rgb(45, 10, 55) } else { BLACK })
    }

    /// The frame at `t` seconds into the show: the stage's own, faded in
    /// over the end of the one before for its first moment.
    pub fn frame(&self, t: f64, g: &WorldGen) -> ShowFrame {
        let (stage, p) = self.stage_at(t);
        let mut fr = self.stage_frame(stage, p, t, g);
        let k = Stage::ALL.iter().position(|&s| s == stage).unwrap_or(0);
        if k >= 1 && stage != Stage::Done && t - self.starts[k] < XF {
            let old = self.stage_frame(Stage::ALL[k - 1], 1.0, t, g);
            let q = smooth((t - self.starts[k]) / XF) as f32;
            for (new, old) in fr.looks.iter_mut().zip(&old.looks) {
                if new != old {
                    *new = (if q < 0.5 { old.0 } else { new.0 }, blend(old.1, new.1, q), blend(old.2, new.2, q));
                }
            }
        }
        fr
    }

    /// A stage's frame `p` of the way through it, at clock time `t` (for
    /// the waves and flicker).
    fn stage_frame(&self, stage: Stage, p: f64, t: f64, g: &WorldGen) -> ShowFrame {
        let ow = &g.ow;
        let (w, h) = (self.w, self.h);
        let n = (w * h) as usize;
        let mut looks: Vec<Look> = vec![(' ', BLACK, BLACK); n];
        let mut site = vec![false; n];
        let at = |i: usize| (i as i32 % w, i as i32 / w);
        match stage {
            Stage::Ocean => {
                for i in 0..n {
                    let (x, y) = at(i);
                    looks[i] = self.sea(x, y, 40.0, t, 1.0);
                }
            }
            Stage::Rising => {
                // The sea level falls from above the highest peak to its own.
                let level = SEA + (self.raw_max - SEA + 2.0) * (1.0 - smooth(p)) as f32;
                for i in 0..n {
                    let (x, y) = at(i);
                    let r = self.raw[i];
                    looks[i] = if r > level && r >= SEA {
                        if r - level < 3.0 && p < 1.0 {
                            // New shore, wet sand.
                            ('░', if hash(x, y, 5) % 2 == 0 { rgb(215, 195, 130) } else { rgb(195, 175, 115) }, BLACK)
                        } else {
                            self.bare(x, y, SEA + (r - SEA) * 0.3)
                        }
                    } else {
                        self.sea(x, y, SEA - (level - r).max(0.5), t, 1.0)
                    };
                }
            }
            Stage::Mountains => {
                let s = 0.3 + 0.7 * smooth(p) as f32;
                for i in 0..n {
                    let (x, y) = at(i);
                    let r = self.raw[i];
                    looks[i] = if r >= SEA { self.bare(x, y, SEA + (r - SEA) * s) } else { self.sea(x, y, r, t, 1.0) };
                }
                self.eruptions(&mut looks, p, t);
            }
            Stage::Weathering => {
                let k = smooth(p) as f32;
                for i in 0..n {
                    let (x, y) = at(i);
                    let e = self.raw[i] + (self.eroded[i] - self.raw[i]) * k;
                    looks[i] = if e >= SEA { self.bare(x, y, e) } else { self.sea(x, y, e, t, 1.0) };
                }
                self.vents(ow, &mut looks);
                self.trial_rivers(&mut looks, p);
            }
            Stage::Rivers | Stage::Life => {
                let rivers_p = if stage == Stage::Rivers { p } else { 1.0 };
                let calm = if stage == Stage::Life { 1.0 - smooth(p / 0.9) } else { 1.0 };
                let shown = |i: usize| self.river_key[i] as f64 <= rivers_p;
                for i in 0..n {
                    let (x, y) = at(i);
                    let c = &ow.cells[i];
                    looks[i] = if c.biome == Biome::Ocean {
                        self.sea(x, y, self.fin[i], t, calm)
                    } else if c.biome == Biome::Lake {
                        if self.lake_key[i] as f64 <= rivers_p {
                            Show::land_look(ow, x, y)
                        } else {
                            self.bare(x, y, self.eroded[i].min(self.fin[i]))
                        }
                    } else if c.river != 0 {
                        if shown(i) {
                            // Only the links to water already there.
                            let mut links = 0u8;
                            for (bit, dx, dy) in [(1u8, 0, -1), (2, 1, 0), (4, 0, 1), (8, -1, 0)] {
                                let (nx, ny) = (x + dx, y + dy);
                                if c.river & bit == 0 || nx < 0 || ny < 0 || nx >= w || ny >= h {
                                    continue;
                                }
                                let j = (ny * w + nx) as usize;
                                let nc = &ow.cells[j];
                                if (nc.river != 0 && shown(j)) || nc.biome == Biome::Ocean || nc.biome == Biome::Lake {
                                    links |= bit;
                                }
                            }
                            let (_, fg, bg) = Show::land_look(ow, x, y);
                            (river_glyph(if links == 0 { c.river } else { links }), fg, bg)
                        } else {
                            self.bare(x, y, self.fin[i])
                        }
                    } else if c.volcano {
                        Show::land_look(ow, x, y)
                    } else if stage == Stage::Life && self.life_key[i] as f64 <= p {
                        let fin = Show::land_look(ow, x, y);
                        let q = ((p - self.life_key[i] as f64) / 0.1).min(1.0) as f32;
                        if q >= 1.0 {
                            fin
                        } else {
                            let bare = self.bare(x, y, self.fin[i]);
                            (fin.0, blend(bare.1, fin.1, q), blend(BLACK, fin.2, q))
                        }
                    } else {
                        self.bare(x, y, self.fin[i])
                    };
                }
                if stage == Stage::Life {
                    self.burn(ow, &mut looks, p, t);
                }
            }
            Stage::Founding => self.founding(g, p, t, &mut looks, &mut site),
            Stage::History => self.history(g, t, &mut looks, &mut site),
            Stage::Done => {
                for i in 0..n {
                    let (x, y) = at(i);
                    looks[i] = ow.map_look(x, y);
                    site[i] = ow.cells[i].site > 0;
                }
            }
        }
        ShowFrame { looks, site }
    }

    /// Volcano vents, once they've erupted, as the finished map draws them.
    fn vents(&self, ow: &Overworld, looks: &mut [Look]) {
        for &(i, _) in &self.volcanoes {
            looks[i] = Show::land_look(ow, i as i32 % self.w, i as i32 / self.w);
        }
    }

    /// Volcanoes erupting: the vent flaring, lava spreading round it and
    /// cooling, smoke drifting off.
    fn eruptions(&self, looks: &mut [Look], p: f64, t: f64) {
        let (w, h) = (self.w, self.h);
        let flick = ((t * 9.0) as i64) % 2 == 0;
        for &(v, start) in &self.volcanoes {
            if p < start {
                continue;
            }
            let a = (p - start) / 0.28;
            let (vx, vy) = (v as i32 % w, v as i32 / w);
            let r = 3.2 * (a * 1.6).min(1.0);
            let hot = (1.0 - (a - 1.0).max(0.0) / 0.6).max(0.0) as f32;
            for dy in -4..=4 {
                for dx in -8..=8 {
                    let (x, y) = (vx + dx, vy + dy);
                    if x < 0 || y < 0 || x >= w || y >= h {
                        continue;
                    }
                    let j = (y * w + x) as usize;
                    let d = (((dx as f64) * 0.5).powi(2) + (dy as f64).powi(2)).sqrt();
                    if j == v || d > r || self.raw[j] < SEA || hash(x, y, 0x1A7A) % 4 == 0 && d > 1.0 {
                        continue;
                    }
                    let glow = if (hash(x, y, (t * 6.0) as u64) % 3 == 0) == flick { rgb(255, 150, 40) } else { rgb(230, 60, 25) };
                    let cool = rgb(80, 45, 40);
                    looks[j] = (if d < 1.5 { '▒' } else { '≈' }, blend(cool, glow, hot), BLACK);
                }
            }
            // The vent.
            looks[v] = ('▲', if a < 1.0 && flick { rgb(255, 230, 90) } else { rgb(240, 80, 40) }, if a < 1.0 { rgb(90, 20, 0) } else { BLACK });
            // Smoke, drifting up and away while it erupts.
            if a < 1.3 {
                for k in 1..=4 {
                    let drift = ((a * 4.0) as i32 + k) / 2;
                    let (x, y) = (vx + drift, vy - k);
                    if x < 0 || y < 0 || x >= w || y >= h {
                        continue;
                    }
                    let fade = (1.0 - k as f32 * 0.18) * (1.3 - a as f32).min(1.0);
                    let j = (y * w + x) as usize;
                    looks[j] = (if k <= 2 { '▓' } else { '░' }, blend(rgb(40, 40, 40), rgb(170, 165, 160), fade), BLACK);
                }
            }
        }
    }

    /// The erosion phase's trial rivers running down the heights, each a
    /// bright head with a trail that fades to the sediment it leaves.
    fn trial_rivers(&self, looks: &mut [Look], p: f64) {
        let span = PATH_SPEED * DUR[3];
        for (start, path) in &self.paths {
            if p < *start {
                continue;
            }
            let head = ((p - start) * span) as usize;
            let tail = head.saturating_sub((1.2 * PATH_SPEED) as usize);
            let last = path.len() - 1;
            for j in tail..=head.min(last) {
                let i = path[j] as usize;
                let age = (head - j) as f32 / PATH_SPEED as f32;
                if age < 0.15 {
                    looks[i] = ('~', rgb(170, 225, 255), BLACK);
                } else {
                    let water = blend(rgb(80, 150, 230), rgb(150, 125, 85), age / 1.2);
                    looks[i] = (if age < 0.6 { '~' } else { '·' }, water, BLACK);
                }
            }
            // Silt where it came out, till the stage ends.
            if head > last && p < 0.97 && last > 0 {
                let i = path[last - 1] as usize;
                if looks[i].0 != '~' {
                    looks[i] = ('░', rgb(185, 160, 105), BLACK);
                }
            }
        }
    }

    /// Wildfires: a burning front sweeping out, black ground behind it,
    /// greening back to what grew there before the stage ends.
    fn burn(&self, ow: &Overworld, looks: &mut [Look], p: f64, t: f64) {
        let speed = 6.0 * DUR[5];
        for f in &self.fires {
            if p < f.start {
                continue;
            }
            let r = (p - f.start) * speed;
            let over = f.start + (f.reach as f64 + 2.0) / speed;
            for &(i, d) in &f.cells {
                let i = i as usize;
                let (x, y) = (i as i32 % self.w, i as i32 / self.w);
                if (self.life_key[i] as f64) > p {
                    continue;
                }
                let d = d as f64;
                if d > r {
                    continue;
                }
                let fin = Show::land_look(ow, x, y);
                let ash = ('"', rgb(55, 50, 45), BLACK);
                if r - d < 1.5 && p < over {
                    let c = match hash(x, y, (t * 8.0) as u64) % 3 {
                        0 => rgb(255, 210, 70),
                        1 => rgb(250, 120, 30),
                        _ => rgb(210, 45, 20),
                    };
                    looks[i] = (if hash(x, y, (t * 5.0) as u64) % 2 == 0 { '*' } else { '^' }, c, rgb(60, 15, 0));
                } else if p < over {
                    looks[i] = ash;
                } else {
                    // Growing back, the near cells first.
                    let q = ((p - over) / (0.97 - over).max(0.01) * (1.0 + (f.reach as f64 - d) * 0.05)).clamp(0.0, 1.0) as f32;
                    looks[i] = if q >= 1.0 { fin } else { (if q > 0.4 { fin.0 } else { ash.0 }, blend(ash.1, fin.1, q), blend(ash.2, fin.2, q)) };
                }
            }
        }
    }

    /// The first peoples' capitals rising one by one, each with a ring of
    /// its people's colour spreading out over the land it will claim; then
    /// the great beasts waking in their lairs, a dark pulse round each.
    fn founding(&self, g: &WorldGen, p: f64, t: f64, looks: &mut [Look], site: &mut [bool]) {
        let ow = &g.ow;
        let w = self.w;
        for i in 0..(w * self.h) as usize {
            looks[i] = Show::land_look(ow, i as i32 % w, i as i32 / w);
        }
        let first: Vec<&crate::overworld::Site> = ow.sites.iter().filter(|s| s.founded <= 0).collect();
        let caps: Vec<&&crate::overworld::Site> = first.iter().filter(|s| s.civ.is_some()).collect();
        let lairs: Vec<&&crate::overworld::Site> = first.iter().filter(|s| s.civ.is_none()).collect();
        let mut rise: Vec<(&crate::overworld::Site, f64, Color)> = Vec::new();
        for (k, s) in caps.iter().enumerate() {
            let col = ow.site_look(s).1;
            let col = s.civ.map_or(col, |c| ow.civ(c).race.color());
            rise.push((s, 0.06 + 0.56 * k as f64 / caps.len().max(2).saturating_sub(1).max(1) as f64, col));
        }
        for (k, s) in lairs.iter().enumerate() {
            rise.push((s, 0.68 + 0.16 * k as f64 / lairs.len().max(2).saturating_sub(1).max(1) as f64, rgb(170, 40, 150)));
        }
        let pulse = 0.5 + 0.5 * (t * 5.0).sin() as f32;
        for (s, at, col) in rise {
            if p < at || s.x < 0 || s.y < 0 || s.x >= w || s.y >= self.h {
                continue;
            }
            let age = (p - at) * DUR[6];
            // The ring: out to a few cells over a second, fading as it goes.
            if age < 1.4 {
                let r = age * 6.0;
                let fade = (1.0 - age / 1.4) as f32;
                self.ring(looks, (s.x, s.y), r, blend(BLACK, col, 0.55 * fade));
            }
            let i = (s.y * w + s.x) as usize;
            let mut then = s.clone();
            if let Some(f) = g.trace.as_ref().and_then(|t| t.first.get(s.id as usize)) {
                (then.civ, then.kind, then.ruined) = (f.civ, f.kind, f.ruined);
            }
            let (ch, fg) = ow.site_look(&then);
            let land = looks[i];
            let q = (age / 0.5).min(1.0) as f32;
            let bg = if s.kind == SiteKind::Lair { blend(rgb(60, 0, 50), BLACK, 1.0 - pulse * (1.0 - q * 0.6)) } else { land.2 };
            let bg = if age < FLASH { blend(blend(BLACK, col, 0.6), bg, (age / FLASH) as f32) } else { bg };
            looks[i] = (if q < 0.3 { land.0 } else { ch }, blend(land.1, fg, q), if age >= 2.0 || p >= 0.999 { land.2 } else { bg });
            site[i] = true;
        }
    }

    /// Tint the background of the cells about `r` from `c` (map distance).
    fn ring(&self, looks: &mut [Look], c: (i32, i32), r: f64, col: Color) {
        let ri = r.ceil() as i32 + 1;
        for dy in -(ri / 2 + 1)..=(ri / 2 + 1) {
            for dx in -ri..=ri {
                let (x, y) = (c.0 + dx, c.1 + dy);
                if x < 0 || y < 0 || x >= self.w || y >= self.h {
                    continue;
                }
                let d = gap_between((x, y), c) as f64;
                if (d - r).abs() < 0.8 {
                    let i = (y * self.w + x) as usize;
                    looks[i].2 = blend(looks[i].2, col, 0.85);
                }
            }
        }
    }

    /// Each site's standing in year `y` (or `None` before it's founded).
    fn sites_in(&self, g: &WorldGen, y: i32) -> Vec<Option<SiteState>> {
        let ow = &g.ow;
        let mut st: Vec<Option<SiteState>> = ow
            .sites
            .iter()
            .enumerate()
            .map(|(k, s)| {
                if s.founded > y {
                    return None;
                }
                let first = g.trace.as_ref().and_then(|t| t.first.get(k).copied());
                Some(first.unwrap_or(SiteState { civ: s.civ, kind: s.kind, ruined: s.ruined }))
            })
            .collect();
        if let Some(t) = &g.trace {
            for &(yr, sid, now) in &t.changes {
                if yr > y {
                    break;
                }
                if let Some(Some(s)) = st.get_mut(sid as usize) {
                    *s = now;
                }
            }
        }
        st
    }

    /// Settlements standing and ruins in year `y`.
    pub fn tally(&self, g: &WorldGen, y: i32) -> (usize, usize) {
        let st = self.sites_in(g, y);
        let towns = st.iter().flatten().filter(|s| s.civ.is_some() && !s.ruined).count();
        let ruins = st.iter().flatten().filter(|s| s.ruined).count();
        (towns, ruins)
    }

    /// Living peoples in year `y` (those with a standing site).
    pub fn peoples(&self, g: &WorldGen, y: i32) -> usize {
        let mut civs: Vec<u32> = self.sites_in(g, y).into_iter().flatten().filter(|s| !s.ruined).filter_map(|s| s.civ).collect();
        civs.sort_unstable();
        civs.dedup();
        civs.len()
    }

    fn history(&self, g: &WorldGen, t: f64, looks: &mut [Look], site: &mut [bool]) {
        let ow = &g.ow;
        let (w, h) = (self.w, self.h);
        for i in 0..(w * h) as usize {
            looks[i] = Show::land_look(ow, i as i32 % w, i as i32 / w);
        }
        let y = self.year_at(t).unwrap_or(0);
        let states = self.sites_in(g, y);
        // When each site last changed hands or fell, up to now.
        let mut changed: Vec<Option<i32>> = vec![None; ow.sites.len()];
        if let Some(tr) = &g.trace {
            for &(yr, sid, _) in tr.changes.iter().take_while(|c| c.0 <= y) {
                if let Some(c) = changed.get_mut(sid as usize) {
                    *c = Some(yr);
                }
            }
        }
        let flick = ((t * 8.0) as i64) % 2 == 0;
        for (k, s) in ow.sites.iter().enumerate() {
            let Some(st) = states[k] else { continue };
            if s.x < 0 || s.y < 0 || s.x >= w || s.y >= h {
                continue;
            }
            let i = (s.y * w + s.x) as usize;
            let mut then = s.clone();
            (then.civ, then.kind, then.ruined) = (st.civ, st.kind, st.ruined);
            let (mut ch, mut fg) = ow.site_look(&then);
            let land = looks[i];
            let mut bg = land.2;
            // Newly founded: it grows out of the land with a warm glow (the
            // first peoples' were shown rising already).
            let age = t - self.time_of_year(s.founded);
            if s.founded > 0 && age < FLASH {
                let q = (age / FLASH) as f32;
                bg = blend(rgb(150, 125, 30), bg, q);
                fg = blend(land.1, fg, (q * 2.0).min(1.0));
            }
            // Fallen lately: it burns, then cools to grey ruin.
            if let Some(yr) = changed[k] {
                let age = t - self.time_of_year(yr);
                if st.ruined && age < FLASH * 1.5 {
                    let q = (age / (FLASH * 1.5)) as f32;
                    if q < 0.5 {
                        ch = if flick { '*' } else { '☼' };
                    }
                    fg = blend(if flick { rgb(255, 170, 40) } else { rgb(230, 70, 30) }, fg, q);
                    bg = blend(rgb(90, 20, 0), bg, q);
                }
            }
            looks[i] = (ch, fg, bg);
            site[i] = true;
        }
        // Raiders run from the attacker to the place attacked; the place
        // flashes when they reach it.
        if let Some(tr) = &g.trace {
            for c in &tr.clashes {
                let age = t - self.time_of_year(c.year);
                if !(0.0..FLASH).contains(&age) {
                    continue;
                }
                let (Some(a), Some(b)) = (ow.sites.get(c.from as usize), ow.sites.get(c.to as usize)) else { continue };
                let hue = match c.end {
                    ClashEnd::Fell | ClashEnd::Taken => rgb(255, 70, 40),
                    ClashEnd::Slain => rgb(120, 220, 255),
                    _ => rgb(255, 170, 60),
                };
                let steps = (b.x - a.x).abs().max((b.y - a.y).abs());
                let run = FLASH * 0.5;
                let head = (age / run).min(1.0) * steps as f64;
                for k in 1..steps {
                    let behind = head - k as f64;
                    if !(0.0..4.0).contains(&behind) {
                        continue;
                    }
                    let x = a.x + ((b.x - a.x) as f64 * k as f64 / steps as f64).round() as i32;
                    let yy = a.y + ((b.y - a.y) as f64 * k as f64 / steps as f64).round() as i32;
                    let i = (yy * w + x) as usize;
                    if site[i] {
                        continue;
                    }
                    let base = looks[i];
                    let fade = (behind / 4.0) as f32;
                    looks[i] = (if behind < 1.0 { '•' } else { '·' }, blend(hue, base.1, fade), base.2);
                }
                if age >= run {
                    let i = (b.y * w + b.x) as usize;
                    looks[i].2 = blend(rgb(170, 30, 15), looks[i].2, ((age - run) / (FLASH - run)) as f32);
                }
            }
        }
        // A great event: a shockwave out from where it struck, the places it
        // touched pulsing, for as long as the show lingers on it.
        if let Some((e, q)) = self.epic_at(t, g) {
            let col = epic_color(e.kind);
            let age = q * EPIC_HOLD;
            let fade = (1.0 - q) as f32;
            for k in 0..3 {
                let r = age * 7.0 - k as f64 * 4.0;
                if r > 0.0 {
                    self.ring(looks, (e.x, e.y), r, blend(BLACK, col, 0.7 * fade));
                }
            }
            let pulse = 0.5 + 0.5 * (t * 6.0).sin() as f32;
            for &s in &e.sites {
                if let Some(s) = ow.sites.get(s as usize) {
                    let i = (s.y * w + s.x) as usize;
                    looks[i].2 = blend(looks[i].2, col, 0.7 * pulse * fade);
                }
            }
            if e.x >= 0 && e.y >= 0 && e.x < w && e.y < h {
                let i = (e.y * w + e.x) as usize;
                if !site[i] {
                    looks[i] = ('☼', blend(rgb(255, 255, 200), col, pulse), blend(BLACK, col, 0.5 * fade));
                }
            }
        }
    }
}

/// The colour of a great event, for its shockwave and banner.
pub fn epic_color(k: EpicKind) -> Color {
    match k {
        EpicKind::Eruption => rgb(255, 110, 30),
        EpicKind::Starfall => rgb(200, 160, 255),
        EpicKind::Rampage => rgb(230, 50, 120),
        EpicKind::Plague => rgb(150, 200, 60),
        EpicKind::Sundering => rgb(120, 170, 255),
        EpicKind::Empire => rgb(255, 210, 80),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overworld::WorldParams;

    fn made(seed: u64, history: i32) -> WorldGen {
        let mut g = WorldGen::traced(WorldParams { seed, history, ..WorldParams::default() });
        while !g.step(usize::MAX, 50) {}
        g
    }

    fn text(f: &ShowFrame, w: i32, x0: i32, y0: i32, cols: i32, rows: i32) -> String {
        let mut s = String::new();
        for y in y0..y0 + rows {
            for x in x0..x0 + cols {
                s.push(f.looks[(y * w + x) as usize].0);
            }
            s.push('\n');
        }
        s
    }

    #[test]
    fn recording_changes_nothing() {
        for seed in [7u64, 1234] {
            let params = WorldParams { seed, history: 80, civs: 2, ..WorldParams::default() };
            let plain = serde_json::to_string(&WorldGen::new(params).finish()).unwrap();
            let traced = serde_json::to_string(&WorldGen::traced(params).finish()).unwrap();
            assert!(plain == traced, "seed {}: recording changed the world", seed);
        }
    }

    #[test]
    fn the_stages_come_in_order_and_end_on_the_finished_map() {
        let g = made(31, 120);
        let show = Show::new(&g);
        let mut last = Stage::Ocean;
        let mut seen = Vec::new();
        let mut t = 0.0;
        while t <= show.end() + 0.5 {
            let (s, p) = show.stage_at(t);
            assert!(s >= last, "stage went back from {:?} to {:?} at {:.1}s", last, s, t);
            assert!((0.0..=1.0).contains(&p));
            if seen.last() != Some(&s) {
                seen.push(s);
            }
            last = s;
            t += 1.0 / 30.0;
        }
        assert_eq!(seen, Stage::ALL.to_vec(), "every stage, in order");
        assert!((45.0..=150.0).contains(&show.end()), "the show runs {:.1}s", show.end());

        // The last frames are the finished map, cell for cell.
        let ow = &g.ow;
        let fin: Vec<Look> = (0..ow.cells.len()).map(|i| ow.map_look(i as i32 % ow.w, i as i32 / ow.w)).collect();
        let done = show.frame(show.end(), &g);
        assert!(done.looks == fin, "the end isn't the finished map");
        // And history's last moment, rebuilt from the record, agrees with it.
        let held = show.frame(show.end() - 0.01, &g);
        assert_eq!(show.stage_at(show.end() - 0.01).0, Stage::History);
        for i in 0..fin.len() {
            assert_eq!(held.looks[i], fin[i], "cell {:?} differs at the end of history", (i as i32 % ow.w, i as i32 / ow.w));
            assert_eq!(held.site[i], ow.cells[i].site > 0);
        }
        // Early on there's only the sea; the land rises out of it.
        let sea = show.frame(1.0, &g);
        assert!(sea.looks.iter().all(|l| l.0 == '≈' || l.0 == '~'));
        let rising = show.frame(show.start_of(Stage::Rising) + DUR[1] * 0.4, &g);
        let land = |f: &ShowFrame| f.looks.iter().filter(|l| l.0 != '≈' && l.0 != '~').count();
        let risen = show.frame(show.start_of(Stage::Mountains), &g);
        assert!(land(&rising) > 0 && land(&rising) < land(&risen), "land should grow as the sea falls");
        // Rivers come in during their stage, not before.
        let rivers = |f: &ShowFrame| ow.cells.iter().enumerate().filter(|(i, c)| c.river != 0 && f.looks[*i].0 != '·' && f.looks[*i].0 != ':' && !matches!(f.looks[*i].0, '∩' | 'n' | '▲' | '^')).count();
        let before = show.frame(show.start_of(Stage::Rivers) + 0.01, &g);
        let after = show.frame(show.start_of(Stage::Life), &g);
        assert!(rivers(&before) * 4 < rivers(&after), "rivers {} then {}", rivers(&before), rivers(&after));
    }

    #[test]
    fn sites_appear_only_after_their_founding() {
        let g = made(5, 150);
        let show = Show::new(&g);
        let ow = &g.ow;
        assert!(ow.sites.iter().any(|s| s.founded > 20), "history founded nothing");
        let mut years_seen = 0;
        let mut t = show.start_of(Stage::History);
        while t < show.end() {
            let y = show.year_at(t).unwrap();
            let f = show.frame(t, &g);
            for s in &ow.sites {
                let i = (s.y * ow.w + s.x) as usize;
                assert_eq!(f.site[i], s.founded <= y, "{} (founded {}) in year {}", s.name, s.founded, y);
            }
            years_seen += 1;
            t += 0.5;
        }
        assert!(years_seen > 10);
        // Ruins show in their year, not before.
        let tr = g.trace.as_ref().unwrap();
        if let Some(&(yr, sid, _)) = tr.changes.iter().find(|c| c.2.ruined && c.0 > 1) {
            let s = &ow.sites[sid as usize];
            let i = (s.y * ow.w + s.x) as usize;
            let before = show.frame(show.time_of_year(yr - 1) + 0.01, &g);
            let burning = show.frame(show.time_of_year(yr) + 0.01, &g);
            let after = show.frame(show.time_of_year(yr) + FLASH * 1.6, &g);
            assert_ne!(before.looks[i].0, '×');
            assert!(matches!(burning.looks[i].0, '*' | '☼'), "a fallen place burns first, not {:?}", burning.looks[i].0);
            assert_eq!(after.looks[i].0, '×');
        }
        assert!(!tr.clashes.is_empty(), "no battles recorded");
    }

    #[test]
    fn the_show_waits_for_history() {
        let mut g = WorldGen::traced(WorldParams { seed: 9, history: 100, ..WorldParams::default() });
        g.step(usize::MAX, 0);
        let mut show = Show::new(&g);
        assert!(show.ready_until(&g) < show.start_of(Stage::Founding));
        while !g.history_ready() {
            g.step(usize::MAX, 0);
        }
        g.step(usize::MAX, 10);
        show.sync(&g);
        let r = show.ready_until(&g);
        assert!(r > show.start_of(Stage::Founding) && r < show.start_of(Stage::History));
        while !g.step(usize::MAX, 50) {}
        show.sync(&g);
        assert!(show.ready_until(&g) > show.end());
    }

    /// Every age gets its few seconds whatever its length, a great event
    /// holds its year on screen, and history reads slowly enough to follow.
    #[test]
    fn history_is_paced_by_its_ages_and_lingers_on_great_events() {
        let mut found_epic = false;
        for seed in [2u64, 11, 23] {
            let mut g = WorldGen::traced(WorldParams { seed, history: 250, epics: 2, ..WorldParams::default() });
            while !g.step(usize::MAX, 50) {}
            let show = Show::new(&g);
            let eras = show.eras().to_vec();
            assert!(!eras.is_empty());
            let secs = show.end() - show.start_of(Stage::History);
            let epics = g.ow.epics.len() as f64;
            assert!(secs >= 12.0 + epics * EPIC_HOLD, "seed {}: history over in {:.1}s", seed, secs);
            for e in &eras {
                let span = show.time_of_year(e.to + 1) - show.time_of_year(e.from);
                let held = g.ow.epics.iter().filter(|p| p.year >= e.from && p.year <= e.to).count() as f64 * EPIC_HOLD;
                assert!((5.99..=15.01).contains(&(span - held)), "seed {}: {} lasts {:.1}s", seed, e.name, span);
                let mid = show.time_of_year((e.from + e.to) / 2) + 0.01;
                assert_eq!(show.era_at(mid).map(|x| x.1.name.clone()), Some(e.name.clone()));
            }
            for p in &g.ow.epics {
                found_epic = true;
                let t = show.time_of_year(p.year) + 1.0;
                assert_eq!(show.year_at(t), Some(p.year));
                assert_eq!(show.year_at(t + EPIC_HOLD * 0.8), Some(p.year), "the year holds through the event");
                assert_eq!(show.epic_at(t, &g).map(|x| x.0.name.clone()), Some(p.name.clone()));
                // The shockwave shows on the map.
                let f = show.frame(t, &g);
                let tinted = f.looks.iter().filter(|l| l.2 != BLACK).count();
                let calm = show.frame(show.time_of_year(p.year) - 0.01, &g).looks.iter().filter(|l| l.2 != BLACK).count();
                assert!(tinted > calm, "seed {}: no shockwave for {}", seed, p.name);
            }
            // Tab-like skipping lands on each age in turn.
            let t0 = show.start_of(Stage::History);
            assert!(show.next_mark(t0 - 0.5) <= t0 + 1e-9);
        }
        assert!(found_epic, "no great events in three worlds at 'common'");
    }

    /// Stages fade into each other: no frame changes more than a fraction
    /// of the map from the one a thirtieth of a second before, except
    /// where a stage cuts to its own first picture.
    #[test]
    fn the_show_moves_smoothly() {
        let g = made(31, 60);
        let show = Show::new(&g);
        let n = g.ow.cells.len();
        let mut worst = (0usize, 0.0f64);
        let mut last = show.frame(0.0, &g);
        let mut t = 0.0;
        while t < show.start_of(Stage::History) + 2.0 {
            t += 1.0 / 30.0;
            let f = show.frame(t, &g);
            let jumped = f.looks.iter().zip(&last.looks).filter(|(a, b)| a.0 != b.0 && a.1 != b.1).count();
            if jumped > worst.0 {
                worst = (jumped, t);
            }
            last = f;
        }
        eprintln!("worst frame-to-frame change: {} of {} cells at {:.2}s", worst.0, n, worst.1);
        assert!(worst.0 * 4 < n, "{} of {} cells jumped at {:.2}s ({:?})", worst.0, n, worst.1, show.stage_at(worst.1));
        // The first peoples rise during their stage, not all at once.
        let caps = |f: &ShowFrame| f.site.iter().filter(|&&s| s).count();
        let early = show.frame(show.start_of(Stage::Founding) + 1.5, &g);
        let late = show.frame(show.start_of(Stage::History) - 0.01, &g);
        assert!(caps(&early) < caps(&late) && caps(&early) > 0, "{} then {}", caps(&early), caps(&late));
    }

    /// One frame from each stage: `cargo test --release print_show_stages -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn print_show_stages() {
        let g = made(42, 200);
        let show = Show::new(&g);
        let w = g.ow.w;
        for (k, s) in Stage::ALL.iter().enumerate() {
            let t = if *s == Stage::Done { show.end() } else { show.start_of(*s) + [0.6, 0.55, 0.75, 0.5, 0.6, 0.65, 0.8, 0.55][k] * if k < HIST { DUR[k] } else { show.end() - show.start_of(Stage::History) } };
            let f = show.frame(t, &g);
            let y = show.year_at(t).map_or(String::new(), |y| format!(", year {}", y));
            println!("-- {} ({}{}), t {:.1}s", s.short(), s.caption(), y, t);
            print!("{}", text(&f, w, 40, 20, 64, 18));
        }
    }
}
