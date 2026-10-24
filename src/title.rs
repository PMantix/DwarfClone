//! The title screen's living backdrops. Each launch picks one of several
//! scenes (never the one shown last time) and a seed that varies it: the
//! lie of the land, the hour, the weather, who turns up. Left open long
//! enough, the screen dissolves into another scene.

mod caravan;
mod caverns;
mod dragon;
mod festival;
mod flood;
mod forge;
mod fortress;
mod gorge;
mod harbour;
mod magma;
mod mine;
mod necro;
mod ruins;
#[cfg(test)]
mod snapshot;
mod siege;
mod tavern;
mod winter;
// Set B: underground, industry and dread.
mod b_kit;
mod b_cavein;
mod b_drowned;
mod b_clockwork;
mod b_smelter;
mod b_engraver;
mod b_traphall;
mod b_pitbridge;
mod b_beast;
mod b_spider;
mod b_crypt;
mod b_ghosts;
mod b_tantrum;
mod b_mood;
mod b_mushroom;
mod b_ambush;
mod b_thief;
// Set C: the world, history, adventure, sea and sky.
mod c_kit;
mod c_campfire;
mod c_slayer;
mod c_voyage;
mod c_lighthouse;
mod c_worldgen;
mod c_march;
mod c_elves;
mod c_castle;
mod c_darkfort;
mod c_fair;
mod c_ford;
mod c_comet;
mod c_eclipse;
mod c_ages;
mod c_skyisles;
mod c_volcano;
#[cfg(test)]
mod c_tests;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use std::path::Path;
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Frames (about 30 a second) since the title first showed.
pub fn now() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    (START.get_or_init(Instant::now).elapsed().as_millis() / 33) as u64
}

/// Frames per season of the fortress backdrop's little year.
pub const SEASON_FRAMES: u64 = 900;

/// Lines that turn over under the title menu.
pub const LINES: [&str; 20] = [
    "Dwarves who sleep on the floor wake up grumpy.",
    "A meeting area keeps idle dwarves out of the corridors.",
    "Goblins come from the nearest dark fortress. Closer means sooner.",
    "A dwarf in a strange mood will not be denied.",
    "Every artifact has a name, and the world will remember it.",
    "Magma heats a forge for free. Mind the floodgates.",
    "A sheriff who can't fight lets the guilty walk.",
    "Wells keep the hospital supplied. Rivers freeze.",
    "Engravings remember what the world has forgotten.",
    "Cave horrors climb towards the light once the caverns are open.",
    "Skulkers steal the shiniest thing lying about. Lock it up.",
    "Leaves fall in autumn. Fire loves dry grass.",
    "The broker's smile is worth a tenth of every bargain.",
    "A good manager sees to the orders twice a day.",
    "Strike the earth!",
    "Unburied dead do not rest quietly.",
    "Soap is the doctor's best friend.",
    "Ale first. Then everything else.",
    "A dwarf's room says who they are. Make it grand.",
    "The deeper you dig, the stranger it gets.",
];

/// A backdrop: how it's drawn and what's said under the banner.
struct Scene {
    #[cfg_attr(not(test), allow(dead_code))]
    name: &'static str,
    draw: fn(&mut Canvas),
    taglines: &'static [&'static str],
}

const SCENES: &[Scene] = &[
    Scene { name: "fortress", draw: fortress::draw, taglines: fortress::TAGLINES },
    Scene { name: "siege", draw: siege::draw, taglines: siege::TAGLINES },
    Scene { name: "dragon", draw: dragon::draw, taglines: dragon::TAGLINES },
    Scene { name: "caravan", draw: caravan::draw, taglines: caravan::TAGLINES },
    Scene { name: "caverns", draw: caverns::draw, taglines: caverns::TAGLINES },
    Scene { name: "tavern", draw: tavern::draw, taglines: tavern::TAGLINES },
    Scene { name: "winter", draw: winter::draw, taglines: winter::TAGLINES },
    Scene { name: "ruins", draw: ruins::draw, taglines: ruins::TAGLINES },
    Scene { name: "forge", draw: forge::draw, taglines: forge::TAGLINES },
    Scene { name: "mine", draw: mine::draw, taglines: mine::TAGLINES },
    Scene { name: "magma", draw: magma::draw, taglines: magma::TAGLINES },
    Scene { name: "harbour", draw: harbour::draw, taglines: harbour::TAGLINES },
    Scene { name: "necro", draw: necro::draw, taglines: necro::TAGLINES },
    Scene { name: "festival", draw: festival::draw, taglines: festival::TAGLINES },
    Scene { name: "gorge", draw: gorge::draw, taglines: gorge::TAGLINES },
    Scene { name: "flood", draw: flood::draw, taglines: flood::TAGLINES },
    // Set B: underground, industry and dread.
    Scene { name: "cavein", draw: b_cavein::draw, taglines: b_cavein::TAGLINES },
    Scene { name: "drowned", draw: b_drowned::draw, taglines: b_drowned::TAGLINES },
    Scene { name: "clockwork", draw: b_clockwork::draw, taglines: b_clockwork::TAGLINES },
    Scene { name: "smelter", draw: b_smelter::draw, taglines: b_smelter::TAGLINES },
    Scene { name: "engraver", draw: b_engraver::draw, taglines: b_engraver::TAGLINES },
    Scene { name: "traphall", draw: b_traphall::draw, taglines: b_traphall::TAGLINES },
    Scene { name: "pitbridge", draw: b_pitbridge::draw, taglines: b_pitbridge::TAGLINES },
    Scene { name: "beast", draw: b_beast::draw, taglines: b_beast::TAGLINES },
    Scene { name: "spider", draw: b_spider::draw, taglines: b_spider::TAGLINES },
    Scene { name: "crypt", draw: b_crypt::draw, taglines: b_crypt::TAGLINES },
    Scene { name: "ghosts", draw: b_ghosts::draw, taglines: b_ghosts::TAGLINES },
    Scene { name: "tantrum", draw: b_tantrum::draw, taglines: b_tantrum::TAGLINES },
    Scene { name: "mood", draw: b_mood::draw, taglines: b_mood::TAGLINES },
    Scene { name: "mushroom", draw: b_mushroom::draw, taglines: b_mushroom::TAGLINES },
    Scene { name: "ambush", draw: b_ambush::draw, taglines: b_ambush::TAGLINES },
    Scene { name: "thief", draw: b_thief::draw, taglines: b_thief::TAGLINES },
    // Set C: the world, history, adventure, sea and sky.
    Scene { name: "campfire", draw: c_campfire::draw, taglines: c_campfire::TAGLINES },
    Scene { name: "slayer", draw: c_slayer::draw, taglines: c_slayer::TAGLINES },
    Scene { name: "voyage", draw: c_voyage::draw, taglines: c_voyage::TAGLINES },
    Scene { name: "lighthouse", draw: c_lighthouse::draw, taglines: c_lighthouse::TAGLINES },
    Scene { name: "worldgen", draw: c_worldgen::draw, taglines: c_worldgen::TAGLINES },
    Scene { name: "march", draw: c_march::draw, taglines: c_march::TAGLINES },
    Scene { name: "elves", draw: c_elves::draw, taglines: c_elves::TAGLINES },
    Scene { name: "castle", draw: c_castle::draw, taglines: c_castle::TAGLINES },
    Scene { name: "darkfort", draw: c_darkfort::draw, taglines: c_darkfort::TAGLINES },
    Scene { name: "fair", draw: c_fair::draw, taglines: c_fair::TAGLINES },
    Scene { name: "ford", draw: c_ford::draw, taglines: c_ford::TAGLINES },
    Scene { name: "comet", draw: c_comet::draw, taglines: c_comet::TAGLINES },
    Scene { name: "eclipse", draw: c_eclipse::draw, taglines: c_eclipse::TAGLINES },
    Scene { name: "ages", draw: c_ages::draw, taglines: c_ages::TAGLINES },
    Scene { name: "skyisles", draw: c_skyisles::draw, taglines: c_skyisles::TAGLINES },
    Scene { name: "volcano", draw: c_volcano::draw, taglines: c_volcano::TAGLINES },
];

/// How many scenes there are to choose from.
pub const SCENE_COUNT: usize = SCENES.len();

/// Frames a scene holds the screen before the next dissolves in (~2.5 min).
pub const HOLD: u64 = 4500;
/// Frames the dissolve takes.
const FADE: u64 = 75;

/// Which scene is showing, and the seed that varies it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pick {
    pub scene: usize,
    pub seed: u64,
}

fn splitmix(z: u64) -> u64 {
    let mut z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A scene for this seed, never `last`.
pub fn choose(seed: u64, last: Option<usize>) -> usize {
    let n = SCENE_COUNT as u64;
    match last.filter(|&l| l < SCENE_COUNT) {
        Some(l) => {
            let k = (splitmix(seed) % (n - 1)) as usize;
            if k >= l {
                k + 1
            } else {
                k
            }
        }
        None => (splitmix(seed) % n) as usize,
    }
}

static PICK: OnceLock<Pick> = OnceLock::new();
const MEMO: &str = ".title";

fn clock_seed() -> u64 {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0);
    splitmix(nanos ^ ((std::process::id() as u64) << 32))
}

/// The scene shown last time, from the little note in the saves folder.
fn recall(dir: &Path) -> Option<usize> {
    std::fs::read_to_string(dir.join(MEMO)).ok()?.trim().parse().ok()
}

fn remember(dir: &Path, scene: usize) {
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(dir.join(MEMO), scene.to_string());
}

/// Choose this launch's scene (once): a fresh seed from the clock, and any
/// scene but the one noted in `dir` from last time. Failures to read or
/// write the note are shrugged off.
pub fn start(dir: &Path) -> Pick {
    *PICK.get_or_init(|| {
        let seed = clock_seed();
        let last = if cfg!(test) { None } else { recall(dir) };
        let p = Pick { scene: choose(seed, last), seed };
        if !cfg!(test) {
            remember(dir, p.scene);
        }
        p
    })
}

fn pick() -> Pick {
    *PICK.get_or_init(|| {
        let seed = clock_seed();
        Pick { scene: choose(seed, None), seed }
    })
}

/// The scene that follows `cur` as the `i`th change of a launch.
fn successor(base: u64, cur: Pick, i: u64) -> Pick {
    let seed = splitmix(base ^ i.wrapping_mul(0xD6E8_FEB8_6659_FD93));
    Pick { scene: choose(seed, Some(cur.scene)), seed }
}

/// What shows at frame `t` of a launch that began with `p`: the scene, and,
/// while it gives way, the one coming in and how far it has got (0..1).
pub fn showing(p: Pick, t: u64) -> (Pick, Option<(Pick, f32)>) {
    let k = t / HOLD;
    let mut cur = p;
    for i in 1..=k {
        cur = successor(p.seed, cur, i);
    }
    let into = t % HOLD;
    if into >= HOLD - FADE {
        let f = (into - (HOLD - FADE)) as f32 / FADE as f32;
        (cur, Some((successor(p.seed, cur, k + 1), f)))
    } else {
        (cur, None)
    }
}

/// How many rows of sky every scene keeps above the menu.
pub fn sky_height(h: u16) -> u16 {
    let h = h as i32;
    (h * 2 / 5).max(9).min((h - 6).max(1)) as u16
}

/// Paint the backdrop over all of `area` at frame `t`, leaving `keep`
/// (relative to `area`) free of glyphs for the title. Returns the height
/// of the sky, so the caller can lay the menu out beneath it.
pub fn draw(buf: &mut Buffer, area: Rect, t: u64, keep: Rect) -> u16 {
    draw_pick(buf, area, t, keep, pick())
}

/// As [`draw`], for a launch that began with `p`.
pub fn draw_pick(buf: &mut Buffer, area: Rect, t: u64, keep: Rect, p: Pick) -> u16 {
    let (cur, next) = showing(p, t);
    paint(buf, area, t, keep, cur);
    if let Some((n, f)) = next {
        let mut other = Buffer::empty(area);
        paint(&mut other, area, t, keep, n);
        // A dissolve that drifts down the screen.
        for y in 0..area.height {
            for x in 0..area.width {
                let r = (hash(x as i32, y as i32) % 1000) as f32 / 1000.0;
                let order = r * 0.7 + y as f32 / area.height.max(1) as f32 * 0.3;
                if order < f {
                    let (ax, ay) = (area.x + x, area.y + y);
                    *buf.get_mut(ax, ay) = other.get(ax, ay).clone();
                }
            }
        }
    }
    sky_height(area.height)
}

fn paint(buf: &mut Buffer, area: Rect, t: u64, keep: Rect, p: Pick) {
    let flip = roll(p.seed, 9999) % 2 == 1;
    let mut cv = Canvas { buf, area, clip: keep, t, seed: p.seed, sky: sky_height(area.height) as i32, flip };
    (SCENES[p.scene % SCENE_COUNT].draw)(&mut cv);
}

fn tagline_of(p: Pick) -> &'static str {
    let l = SCENES[p.scene % SCENE_COUNT].taglines;
    l[(splitmix(p.seed ^ 0x7A6) % l.len() as u64) as usize]
}

/// The line under the banner: it belongs to the scene on show.
pub fn tagline(t: u64) -> &'static str {
    let (cur, next) = showing(pick(), t);
    match next {
        Some((n, f)) if f > 0.5 => tagline_of(n),
        _ => tagline_of(cur),
    }
}

/// The tip turning over under the menu at frame `t`, starting somewhere
/// different each launch.
pub fn tip(t: u64) -> &'static str {
    let off = splitmix(pick().seed ^ 0x71B) % LINES.len() as u64;
    LINES[((t / 240 + off) % LINES.len() as u64) as usize]
}

// ---------------------------------------------------------------------------
// Drawing helpers shared by the scenes.

type Rgb = (u8, u8, u8);

fn hash(x: i32, y: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^ (h >> 12)
}

/// A number from the seed, one per `k`.
fn roll(seed: u64, k: u32) -> u32 {
    (splitmix(seed ^ (k as u64).wrapping_mul(0xA24B_AED4_963E_E407)) >> 16) as u32
}

fn rgb(c: Rgb) -> Color {
    Color::Rgb(c.0, c.1, c.2)
}

fn lerp(a: Rgb, b: Rgb, f: f32) -> Rgb {
    let f = f.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * f) as u8;
    (l(a.0, b.0), l(a.1, b.1), l(a.2, b.2))
}

fn mix(a: Rgb, b: Rgb, f: f32) -> Color {
    rgb(lerp(a, b, f))
}

/// Back and forth between `a` and `b`, one step every `every` frames.
fn pingpong(t: u64, every: u64, a: i32, b: i32) -> i32 {
    let span = (b - a).max(1) as u64;
    let k = (t / every.max(1)) % (2 * span);
    a + if k < span { k as i32 } else { (2 * span - k) as i32 }
}

/// A mountain-ish skyline height in 0..1 at column `x`, shaped by the seed.
fn ridge(x: i32, w: i32, seed: u64, salt: u32) -> f32 {
    let fx = x as f32 / w.max(1) as f32;
    let p = |k: u32| (roll(seed, salt + k) % 628) as f32 / 100.0;
    let r = (fx * 3.1 + p(0)).sin() * 0.45 + (fx * 9.0 + p(1)).sin() * 0.22 + (fx * 23.0 + p(2)).sin() * 0.1;
    ((r + 0.5) / 1.25).clamp(0.0, 1.0)
}

/// Pointed summits, 0..1 at column `x`: `n` seeded peaks with ragged
/// flanks.
fn peaks(x: i32, w: i32, seed: u64, salt: u32, n: u32) -> f32 {
    let wf = w.max(1) as f32;
    let mut m: f32 = 0.0;
    for k in 0..n {
        let r = |j: u32| (roll(seed, salt + k * 5 + j) % 1000) as f32 / 1000.0;
        let px = (r(0) * 1.2 - 0.1) * wf;
        let height = 0.45 + r(1) * 0.55;
        let half = wf * (0.1 + r(2) * 0.18) + 3.0;
        m = m.max(height * (1.0 - (x as f32 - px).abs() / half));
    }
    let jag = (hash(x, salt as i32) % 100) as f32 / 100.0 - 0.5;
    (m + jag * 0.06).clamp(0.0, 1.0)
}

/// The cells from `a` to `b` (both included), one step at a time.
fn line(a: (i32, i32), b: (i32, i32)) -> Vec<(i32, i32)> {
    let (mut x, mut y) = a;
    let (dx, dy) = ((b.0 - x).abs(), -(b.1 - y).abs());
    let (sx, sy) = (if x < b.0 { 1 } else { -1 }, if y < b.1 { 1 } else { -1 });
    let mut err = dx + dy;
    let mut v = vec![(x, y)];
    while (x, y) != b {
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
        v.push((x, y));
    }
    v
}

struct Canvas<'a> {
    buf: &'a mut Buffer,
    area: Rect,
    /// No glyphs here (the banner's place); colours still show through.
    clip: Rect,
    t: u64,
    seed: u64,
    sky: i32,
    /// Seen in a mirror: every scene can face either way.
    flip: bool,
}

/// A glyph as it looks in a mirror.
fn mirrored(ch: char) -> char {
    match ch {
        '/' => '\\',
        '\\' => '/',
        '►' => '◄',
        '◄' => '►',
        '▶' => '◀',
        '◀' => '▶',
        '╜' => '╙',
        '╙' => '╜',
        '«' => '»',
        '»' => '«',
        '▌' => '▐',
        '▐' => '▌',
        '(' => ')',
        ')' => '(',
        '╲' => '╱',
        '╱' => '╲',
        c => c,
    }
}

impl Canvas<'_> {
    fn w(&self) -> i32 {
        self.area.width as i32
    }

    fn h(&self) -> i32 {
        self.area.height as i32
    }

    fn roll(&self, k: u32) -> u32 {
        roll(self.seed, k)
    }

    /// A number from the seed to stir into layout hashes.
    fn salt(&self) -> i32 {
        (self.roll(777) % 100_003) as i32
    }

    /// Where column `x` lands on screen.
    fn col(&self, x: i32) -> i32 {
        if self.flip {
            self.w() - 1 - x
        } else {
            x
        }
    }

    fn inside(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w() && y < self.h()
    }

    fn clipped(&self, x: i32, y: i32) -> bool {
        let c = self.clip;
        x >= c.x as i32 && x < (c.x + c.width) as i32 && y >= c.y as i32 && y < (c.y + c.height) as i32
    }

    fn put(&mut self, x: i32, y: i32, ch: char, fg: Color, bg: Option<Color>) {
        if !self.inside(x, y) {
            return;
        }
        let x = self.col(x);
        let ch = if self.flip { mirrored(ch) } else { ch };
        let clipped = self.clipped(x, y);
        let c = self.buf.get_mut(self.area.x + x as u16, self.area.y + y as u16);
        if clipped {
            if let Some(bg) = bg {
                c.set_char(' ').set_bg(bg);
            }
            return;
        }
        c.set_char(ch).set_fg(fg);
        if let Some(bg) = bg {
            c.set_bg(bg);
        }
    }

    /// A glyph over whatever colour is already there.
    fn glyph(&mut self, x: i32, y: i32, ch: char, fg: Color) {
        self.put(x, y, ch, fg, None);
    }

    /// Fill a cell with colour only.
    fn fill(&mut self, x: i32, y: i32, bg: Color) {
        self.put(x, y, ' ', Color::Black, Some(bg));
    }

    /// Recolour a cell's background, keeping its glyph.
    fn tint(&mut self, x: i32, y: i32, bg: Color) {
        if self.inside(x, y) {
            let x = self.col(x);
            self.buf.get_mut(self.area.x + x as u16, self.area.y + y as u16).set_bg(bg);
        }
    }

    /// The background colour already at a cell.
    fn bg_at(&self, x: i32, y: i32) -> Rgb {
        if !self.inside(x, y) {
            return (0, 0, 0);
        }
        let x = self.col(x);
        match self.buf.get(self.area.x + x as u16, self.area.y + y as u16).bg {
            Color::Rgb(r, g, b) => (r, g, b),
            _ => (0, 0, 0),
        }
    }

    /// Blend `col` into a cell's background by `f`, keeping its glyph.
    fn glow(&mut self, x: i32, y: i32, col: Rgb, f: f32) {
        if self.inside(x, y) && f > 0.0 {
            let bg = self.bg_at(x, y);
            self.tint(x, y, mix(bg, col, f));
        }
    }

    /// A soft round light (twice as wide as tall, as cells are) blended
    /// into the backgrounds around (`cx`, `cy`).
    fn halo(&mut self, cx: i32, cy: i32, r: f32, col: Rgb, f: f32) {
        let r = r.max(0.5);
        let (rx, ry) = ((r * 2.0).ceil() as i32, r.ceil() as i32);
        for y in cy - ry..=cy + ry {
            for x in cx - rx..=cx + rx {
                let (dx, dy) = ((x - cx) as f32 * 0.5, (y - cy) as f32);
                let d = (dx * dx + dy * dy).sqrt() / r;
                if d < 1.0 {
                    self.glow(x, y, col, f * (1.0 - d) * (1.0 - d));
                }
            }
        }
    }

    /// A filled disc of background colour (a sun, a moon).
    fn disc(&mut self, cx: i32, cy: i32, r: f32, col: Rgb) {
        let (rx, ry) = ((r * 2.0).ceil() as i32, r.ceil() as i32);
        for y in cy - ry..=cy + ry {
            for x in cx - rx..=cx + rx {
                let (dx, dy) = ((x - cx) as f32 * 0.5, (y - cy) as f32);
                if dx * dx + dy * dy <= r * r {
                    self.tint(x, y, rgb(col));
                }
            }
        }
    }

    /// Rows of text drawn over the scene, spaces left see-through.
    fn sprite(&mut self, x: i32, y: i32, rows: &[&str], fg: Color) {
        for (dy, r) in rows.iter().enumerate() {
            for (dx, ch) in r.chars().enumerate() {
                if ch != ' ' {
                    self.glyph(x + dx as i32, y + dy as i32, ch, fg);
                }
            }
        }
    }

    /// A vertical gradient over rows `0..rows`.
    fn sky_fill(&mut self, rows: i32, top: Rgb, bottom: Rgb) {
        for y in 0..rows.min(self.h()) {
            let bg = mix(top, bottom, y as f32 / rows.max(1) as f32);
            for x in 0..self.w() {
                self.fill(x, y, bg);
            }
        }
    }

    /// Twinkling stars over rows `0..rows`, one cell in `every`.
    fn stars(&mut self, rows: i32, every: u32) {
        let t = self.t;
        for y in 0..rows.min(self.h()) {
            for x in 0..self.w() {
                let hs = hash(x * 7 + 3, y * 13 + 1);
                if hs % every.max(2) == 0 {
                    let twinkle = (t / 6 + hs as u64 / 53) % 23 == 0;
                    let (ch, fg) = if twinkle { ('+', (255, 255, 230)) } else { ('·', (150, 150, 190)) };
                    self.glyph(x, y, ch, rgb(fg));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZES: [(u16, u16); 5] = [(20, 8), (80, 24), (120, 40), (200, 60), (300, 100)];

    /// The banner's place as the title screen lays it out.
    fn keep(w: u16) -> Rect {
        let bw = 54u16;
        Rect::new((w.saturating_sub(bw) / 2).saturating_sub(1), 0, bw + 2, 6).intersection(Rect::new(0, 0, w, 60))
    }

    fn render(scene: usize, seed: u64, w: u16, h: u16, t: u64) -> Buffer {
        let area = Rect::new(0, 0, w, h);
        let mut b = Buffer::empty(area);
        let sky = draw_pick(&mut b, area, t, keep(w), Pick { scene, seed });
        assert!(sky < h.max(2), "sky {} on {}x{}", sky, w, h);
        b
    }

    fn text(b: &Buffer) -> String {
        let w = b.area.width as usize;
        let mut s = String::new();
        for (i, c) in b.content.iter().enumerate() {
            if i > 0 && i % w == 0 {
                s.push('\n');
            }
            s.push_str(c.symbol());
        }
        s
    }

    #[test]
    fn every_scene_fits_any_screen_and_moves() {
        for scene in 0..SCENE_COUNT {
            for seed in [1u64, 77, 123_456_789] {
                for (w, h) in SIZES {
                    for t in [0u64, 100, 4000] {
                        render(scene, seed, w, h, t);
                    }
                    if w >= 80 {
                        let a = render(scene, seed, w, h, 100);
                        let b = render(scene, seed, w, h, 160);
                        assert_ne!(a, b, "{} should animate at {}x{}", SCENES[scene].name, w, h);
                    }
                }
            }
        }
    }

    #[test]
    fn tiny_and_odd_screens_never_crash() {
        for scene in 0..SCENE_COUNT {
            for (w, h) in [(1u16, 1u16), (3, 2), (9, 4), (30, 10), (41, 13), (57, 70), (300, 9)] {
                for t in [0u64, 999, 7777] {
                    render(scene, 5, w, h, t);
                }
            }
        }
    }

    #[test]
    fn the_banner_stays_clear() {
        for scene in 0..SCENE_COUNT {
            for seed in [3u64, 99, 2024] {
                for (w, h) in SIZES {
                    let k = keep(w);
                    for t in [0u64, 37, 250, 1111] {
                        let b = render(scene, seed, w, h, t);
                        for y in k.y..(k.y + k.height).min(h) {
                            for x in k.x..k.x + k.width {
                                let s = b.get(x, y).symbol();
                                assert_eq!(s, " ", "{} drew {:?} at {},{} on {}x{}", SCENES[scene].name, s, x, y, w, h);
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_scenes_are_all_there_and_named_once() {
        assert!(SCENE_COUNT >= 16);
        for (i, a) in SCENES.iter().enumerate() {
            assert!(SCENES.iter().skip(i + 1).all(|b| b.name != a.name), "{} twice", a.name);
        }
        for name in ["forge", "mine", "magma", "harbour", "necro", "festival", "gorge", "flood"] {
            assert!(SCENES.iter().any(|s| s.name == name), "{} is missing", name);
        }
    }

    /// Each new scene's turn of events shows up over a couple of minutes:
    /// the thing it is about is on screen at some frames and not others,
    /// or always there for the scenes that never stop.
    #[test]
    fn the_new_scenes_tell_their_stories() {
        let idx = |name: &str| SCENES.iter().position(|s| s.name == name).unwrap();
        // (scene, glyph, comes and goes)
        let cases = [
            ("forge", '■', false),
            ("mine", '♦', true),
            ("magma", '●', true),
            ("harbour", 'o', false),
            ("necro", '☻', false),
            ("festival", '*', true),
            ("gorge", '↑', false),
            ("flood", '║', true),
        ];
        for (name, glyph, comes_and_goes) in cases {
            for seed in [4u64, 8, 15] {
                let seen: Vec<bool> = (0..3000u64).step_by(40).map(|t| text(&render(idx(name), seed, 120, 40, t)).contains(glyph)).collect();
                assert!(seen.iter().any(|&s| s), "{} never shows {:?} (seed {})", name, glyph, seed);
                if comes_and_goes {
                    assert!(seen.iter().any(|&s| !s), "{} always shows {:?} (seed {})", name, glyph, seed);
                }
            }
        }
    }

    /// A frame of any scene stays well inside a 30 fps budget at 200x60
    /// (best of a few tries, so a busy machine doesn't trip it; looser for
    /// unoptimised builds).
    #[test]
    fn frames_are_quick() {
        let limit = if cfg!(debug_assertions) { 150.0 } else { 10.0 };
        let area = Rect::new(0, 0, 200, 60);
        let mut b = Buffer::empty(area);
        for scene in 0..SCENE_COUNT {
            let best = (0..5u64)
                .map(|k| {
                    let t0 = std::time::Instant::now();
                    for t in 0..6u64 {
                        draw_pick(&mut b, area, 300 + k * 97 + t, keep(200), Pick { scene, seed: 7 + k });
                    }
                    t0.elapsed().as_secs_f64() * 1000.0 / 6.0
                })
                .fold(f64::MAX, f64::min);
            assert!(best < limit, "{} takes {:.2} ms a frame", SCENES[scene].name, best);
        }
    }

    #[test]
    fn the_fortress_keeps_its_dwarves_and_magma() {
        for seed in [1u64, 5, 9] {
            let s = text(&render(0, seed, 80, 24, 100));
            assert!(s.contains('☺') && s.contains('≈'));
        }
    }

    #[test]
    fn the_picker_never_repeats_itself() {
        for seed in 0..500u64 {
            for last in 0..SCENE_COUNT {
                let s = choose(seed, Some(last));
                assert!(s < SCENE_COUNT && s != last);
            }
            assert!(choose(seed, None) < SCENE_COUNT);
            assert!(choose(seed, Some(99)) < SCENE_COUNT);
        }
        // And over many launches every scene turns up.
        let mut seen = [false; SCENE_COUNT];
        let mut last = None;
        for seed in 0..1000u64 {
            let s = choose(splitmix(seed), last);
            seen[s] = true;
            last = Some(s);
        }
        assert!(seen.iter().all(|&s| s));
        // Nor does a long stay ever dissolve into the same scene again.
        let mut seen = [false; SCENE_COUNT];
        for seed in 0..40u64 {
            let p = Pick { scene: (seed % SCENE_COUNT as u64) as usize, seed: splitmix(seed) };
            let mut cur = p;
            for k in 1..30u64 {
                let n = showing(p, k * HOLD + 5).0;
                assert_ne!(n.scene, cur.scene);
                seen[n.scene] = true;
                cur = n;
            }
        }
        assert!(seen.iter().all(|&s| s));
    }

    #[test]
    fn the_seed_varies_each_scene() {
        for scene in 0..SCENE_COUNT {
            let frames: Vec<Buffer> = (0..6u64).map(|k| render(scene, splitmix(k + 10), 120, 40, 50)).collect();
            assert!(frames.iter().skip(1).any(|b| *b != frames[0]), "{} looks the same every launch", SCENES[scene].name);
        }
    }

    #[test]
    fn a_long_stay_moves_on_to_another_scene() {
        let p = Pick { scene: 2, seed: 42 };
        let (a, none) = showing(p, 10);
        assert_eq!(a, p);
        assert!(none.is_none());
        let (_, fading) = showing(p, HOLD - 10);
        let (n, f) = fading.expect("a dissolve near the end of the hold");
        assert!(n.scene != p.scene && f > 0.5);
        assert_eq!(showing(p, HOLD + 5).0, n);
        let area = Rect::new(0, 0, 80, 24);
        let mut b = Buffer::empty(area);
        draw_pick(&mut b, area, HOLD - 30, keep(80), p);
    }

    #[test]
    fn remembering_the_last_scene() {
        let dir = std::env::temp_dir().join(format!("dwarfclone-title-{}", std::process::id()));
        assert_eq!(recall(&dir), None);
        remember(&dir, 5);
        assert_eq!(recall(&dir), Some(5));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn taglines_are_there_and_fit() {
        for s in SCENES.iter() {
            assert!(!s.taglines.is_empty());
            for l in s.taglines {
                assert!(l.chars().count() <= 54, "{:?} is too long", l);
            }
        }
        assert!(!tagline(0).is_empty() && !tip(0).is_empty());
    }

    /// Pictures of every scene with the title laid over them roughly as
    /// the title screen does, in target/title/:
    /// `cargo test --release picture_the_scenes -- --ignored` (SIZE=WxH, FRAME=n).
    #[test]
    #[ignore]
    fn picture_the_scenes() {
        let (w, h) = std::env::var("SIZE")
            .ok()
            .and_then(|s| {
                let (a, b) = s.split_once('x')?;
                Some((a.parse().ok()?, b.parse().ok()?))
            })
            .unwrap_or((120u16, 40u16));
        let t: u64 = std::env::var("FRAME").ok().and_then(|s| s.parse().ok()).unwrap_or(333);
        // Contact sheets too, eight scenes to a page, when they fit in one buffer.
        let only = std::env::var("SCENE").ok();
        let pages = (SCENE_COUNT + 7) / 8;
        let sheet = only.is_none() && (w as u32 * 2 + 1) * (h as u32 * 4 + 3) <= u16::MAX as u32;
        let mut sheets: Vec<Buffer> = (0..if sheet { 3 * pages } else { 0 }).map(|_| Buffer::empty(Rect::new(0, 0, w * 2 + 1, h * 4 + 3))).collect();
        for scene in 0..SCENE_COUNT {
            if only.as_deref().map_or(false, |o| !o.split(',').any(|n| n == SCENES[scene].name)) {
                continue;
            }
            for (k, seed) in [11u64, 12, 13].into_iter().enumerate() {
                let mut b = render(scene, seed, w, h, t);
                let gold = Color::Rgb(250, 200, 80);
                let name = "D W A R F    C L O N E";
                let x0 = w.saturating_sub(name.len() as u16) / 2;
                for row in 1..5u16.min(h) {
                    b.set_string(x0, row, name, ratatui::style::Style::default().fg(gold));
                }
                let sky = sky_height(h);
                let (mw, mh) = (66u16.min(w), 12u16);
                let top = (sky + 1).min(h.saturating_sub(mh));
                let r = Rect::new((w - mw) / 2, top, mw, mh.min(h)).intersection(b.area);
                for y in r.y..r.y + r.height {
                    for x in r.x..r.x + r.width {
                        b.get_mut(x, y).set_char(' ').set_bg(Color::Rgb(12, 12, 20));
                    }
                }
                let path = std::path::PathBuf::from(format!("target/title/{}-{}-{}x{}-t{}.png", SCENES[scene].name, seed, w, h, t));
                snapshot::save(&b, &path);
                if !sheet {
                    continue;
                }
                let at = scene % 8;
                let (ox, oy) = ((at as u16 % 2) * (w + 1), (at as u16 / 2) * (h + 1));
                for y in 0..h {
                    for x in 0..w {
                        *sheets[k * pages + scene / 8].get_mut(ox + x, oy + y) = b.get(x, y).clone();
                    }
                }
            }
        }
        for (i, sheet) in sheets.iter().enumerate() {
            let (k, page) = (i / pages, i % pages);
            snapshot::save(sheet, &std::path::PathBuf::from(format!("target/title/sheet-{}-p{}-{}x{}-t{}.png", k, page, w, h, t)));
        }
    }

    /// How long a frame of each scene takes at 200x60, plain and mid-dissolve.
    #[test]
    #[ignore]
    fn scene_frame_times() {
        let area = Rect::new(0, 0, 200, 60);
        for scene in 0..SCENE_COUNT {
            let mut b = Buffer::empty(area);
            let t0 = std::time::Instant::now();
            for t in 0..30u64 {
                draw_pick(&mut b, area, 100 + t, keep(200), Pick { scene, seed: 7 });
            }
            let plain = t0.elapsed().as_secs_f64() * 1000.0 / 30.0;
            let t0 = std::time::Instant::now();
            for t in 0..30u64 {
                draw_pick(&mut b, area, HOLD - 40 + t, keep(200), Pick { scene, seed: 7 });
            }
            let fading = t0.elapsed().as_secs_f64() * 1000.0 / 30.0;
            println!("{:<9} {:>6.2} ms  {:>6.2} ms dissolving", SCENES[scene].name, plain, fading);
        }
    }

    /// `cargo test --release show_the_scenes -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn show_the_scenes() {
        // SCENE picks one by number or by name; SIZE=WxH as for the pictures.
        let only = std::env::var("SCENE").ok();
        let t: u64 = std::env::var("FRAME").ok().and_then(|s| s.parse().ok()).unwrap_or(333);
        let (w, h) = std::env::var("SIZE")
            .ok()
            .and_then(|s| {
                let (a, b) = s.split_once('x')?;
                Some((a.parse().ok()?, b.parse().ok()?))
            })
            .unwrap_or((120u16, 40u16));
        for scene in 0..SCENE_COUNT {
            if only.as_deref().map_or(false, |o| !o.split(',').any(|n| n == SCENES[scene].name || n.parse() == Ok(scene))) {
                continue;
            }
            for seed in [11u64, 12] {
                println!("=== {} (seed {}) ===", SCENES[scene].name, seed);
                println!("{}", text(&render(scene, seed, w, h, t)));
            }
        }
    }
}
