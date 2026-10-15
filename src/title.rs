//! The title screen's living backdrop: a mountain under a night sky that
//! turns through the seasons, the fortress dug into it with its dwarves at
//! work, and the magma sea glowing far below.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use std::sync::OnceLock;
use std::time::Instant;

/// Frames (about 30 a second) since the title first showed.
pub fn now() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    (START.get_or_init(Instant::now).elapsed().as_millis() / 33) as u64
}

/// Frames per season of the backdrop's little year.
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

fn hash(x: i32, y: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^ (h >> 12)
}

fn mix(a: (u8, u8, u8), b: (u8, u8, u8), f: f32) -> Color {
    let f = f.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * f) as u8;
    Color::Rgb(l(a.0, b.0), l(a.1, b.1), l(a.2, b.2))
}

/// Back and forth between `a` and `b`, one step every `every` frames.
fn pingpong(t: u64, every: u64, a: i32, b: i32) -> i32 {
    let span = (b - a).max(1) as u64;
    let k = (t / every.max(1)) % (2 * span);
    a + if k < span { k as i32 } else { (2 * span - k) as i32 }
}

struct Canvas<'a> {
    buf: &'a mut Buffer,
    area: Rect,
}

impl Canvas<'_> {
    fn put(&mut self, x: i32, y: i32, ch: char, fg: Color, bg: Option<Color>) {
        if x < 0 || y < 0 || x >= self.area.width as i32 || y >= self.area.height as i32 {
            return;
        }
        let c = self.buf.get_mut(self.area.x + x as u16, self.area.y + y as u16);
        c.set_char(ch).set_fg(fg);
        if let Some(bg) = bg {
            c.set_bg(bg);
        }
    }
}

/// Paint the backdrop over all of `area` at frame `t`, leaving the sky
/// in `keep` (relative to `area`) clear of stars and clouds for the title.
/// Returns the height of the sky, so the caller can lay the menu out
/// beneath it.
pub fn draw(buf: &mut Buffer, area: Rect, t: u64, keep: Rect) -> u16 {
    let (w, h) = (area.width as i32, area.height as i32);
    let mut cv = Canvas { buf, area };
    let kept = |x: i32, y: i32| {
        x >= keep.x as i32 && x < (keep.x + keep.width) as i32 && y >= keep.y as i32 && y < (keep.y + keep.height) as i32
    };
    let sky_h = (h * 2 / 5).max(9).min((h - 6).max(1));
    let magma_top = h - 3;
    let season = (t / SEASON_FRAMES) % 4;

    // The mountain's skyline: peaks rise towards the middle-right.
    let rise = (sky_h - 7).max(1) as f32;
    let surf: Vec<i32> = (0..w)
        .map(|x| {
            let fx = x as f32 / w.max(1) as f32;
            let ridge = (fx * 3.1 + 0.4).sin() * 0.45 + (fx * 9.0).sin() * 0.22 + (fx * 23.0 + 1.3).sin() * 0.1;
            let m = ((ridge + 0.5) / 1.25).clamp(0.0, 1.0);
            sky_h - 1 - (m * rise) as i32
        })
        .collect();

    // Sky, stars and moon.
    let rock_bg = Color::Rgb(26, 22, 20);
    for y in 0..h {
        for x in 0..w {
            if y < surf[x as usize] {
                let f = y as f32 / sky_h.max(1) as f32;
                let bg = mix((6, 8, 26), (44, 28, 58), f);
                let hs = hash(x * 7, y * 13);
                let (ch, fg) = if hs % 53 == 0 && y < sky_h - 3 && !kept(x, y) {
                    let twinkle = (t / 6 + hs as u64 / 53) % 23 == 0;
                    (if twinkle { '+' } else { '·' }, if twinkle { Color::Rgb(255, 255, 230) } else { Color::Rgb(150, 150, 190) })
                } else {
                    (' ', Color::Black)
                };
                cv.put(x, y, ch, fg, Some(bg));
            } else if y >= magma_top {
                // The magma sea, breathing.
                let wave = ((x as f32 * 0.35 + t as f32 * 0.08).sin() + (x as f32 * 0.11 - t as f32 * 0.05).sin()) * 0.25 + 0.5;
                let bubble = hash(x, (t / 9) as i32 + y * 31) % 97 == 0;
                let (ch, fg) = if bubble {
                    ('°', Color::Rgb(255, 240, 120))
                } else {
                    (if (x + (t / 12) as i32) % 3 == 0 { '~' } else { '≈' }, mix((200, 40, 0), (255, 190, 50), wave))
                };
                cv.put(x, y, ch, fg, Some(mix((70, 10, 0), (130, 30, 0), wave)));
            } else {
                // Rock, with veins and gems that catch the light.
                let hs = hash(x * 3 + 1, y * 5 + 2);
                let deep = (y - sky_h).max(0) as f32 / (magma_top - sky_h).max(1) as f32;
                let heat = if y >= magma_top - 2 { 0.25 } else { 0.0 };
                let bg = mix((26, 22, 20), (60, 18, 8), deep * deep * 0.6 + heat);
                let (ch, fg) = match hs % 41 {
                    0 => ('☼', if (t / 5 + hs as u64) % 37 == 0 { Color::Rgb(255, 255, 255) } else { Color::Rgb(90, 200, 220) }),
                    1 | 2 => ('░', Color::Rgb(70, 60, 55)),
                    3 => ('£', Color::Rgb(150, 110, 60)),
                    _ => (' ', Color::Black),
                };
                cv.put(x, y, ch, fg, Some(bg));
            }
        }
    }
    let moon_x = w * 5 / 6;
    if surf.get(moon_x as usize).map_or(false, |&s| s > 3) && !kept(moon_x, 2) {
        cv.put(moon_x, 2, '●', Color::Rgb(245, 240, 210), None);
    }

    // Drifting clouds.
    for k in 0..3i32 {
        let len = 6 + (hash(k, 9) % 6) as i32;
        let y = 1 + (hash(k, 3) % 4) as i32;
        let x0 = ((hash(k, 5) as u64 + t / (14 + k as u64 * 5)) % (w as u64 + len as u64 + 10)) as i32 - len - 5;
        for i in 0..len {
            let x = x0 + i;
            if x >= 0 && x < w && y < surf[x as usize] && !kept(x, y) {
                let edge = i == 0 || i == len - 1;
                cv.put(x, y, if edge { '▄' } else { '█' }, Color::Rgb(55, 50, 80), None);
            }
        }
    }

    // The mountainside: a snowy crest, trees on the lower slopes in the
    // colours of the season.
    let tree_col = |x: i32| -> Color {
        let hs = hash(x, 77);
        match season {
            0 if hs % 3 == 0 => Color::Rgb(255, 175, 205),
            0 => Color::Rgb(150, 215, 110),
            1 => Color::Rgb(40, 170, 50),
            2 => [Color::Rgb(235, 205, 60), Color::Rgb(235, 135, 40), Color::Rgb(205, 60, 40)][(hs % 3) as usize],
            _ => Color::Rgb(125, 95, 70),
        }
    };
    for x in 0..w {
        let s = surf[x as usize];
        let high = s < sky_h - 1 - (rise * 0.55) as i32;
        let snowy = high || season == 3;
        cv.put(x, s, '▲', if snowy { Color::Rgb(235, 240, 250) } else { Color::Rgb(120, 110, 100) }, Some(rock_bg));
        if !high && hash(x, 41) % 3 == 0 && s > 0 {
            let ch = if hash(x, 43) % 4 == 0 { '♠' } else { '♣' };
            let col = if ch == '♠' { Color::Rgb(30, 120, 60) } else { tree_col(x) };
            cv.put(x, s - 1, ch, col, None);
        }
    }

    // What the season brings down from the sky.
    for k in 0..(w / 3).max(4) {
        let speed = 3 + (k % 3) as u64;
        let x = ((hash(k, 1) as u64 + t / (speed * 3)) % w.max(1) as u64) as i32;
        let y = ((hash(k, 2) as u64 + t / speed) % sky_h.max(1) as u64) as i32;
        if y >= surf[x as usize] || kept(x, y) {
            continue;
        }
        let (ch, fg) = match season {
            0 => ('·', Color::Rgb(255, 190, 215)),
            1 if (t / 8 + k as u64) % 5 == 0 => ('·', Color::Rgb(230, 255, 120)),
            1 => continue,
            2 => ([',', '\'', '`'][(k % 3) as usize], [Color::Rgb(235, 135, 40), Color::Rgb(205, 60, 40), Color::Rgb(235, 205, 60)][(k % 3) as usize]),
            _ => ('*', Color::Rgb(240, 240, 255)),
        };
        cv.put(x, y, ch, fg, None);
    }

    // The fortress: two halls joined by a stair, a forge with its chimney,
    // a miner at the face.
    let floor_bg = Color::Rgb(10, 9, 9);
    let t1 = sky_h + 2;
    let t2 = (sky_h + (magma_top - sky_h) * 3 / 5).max(t1 + 3);
    if t2 >= magma_top - 1 || w < 30 {
        return sky_h as u16;
    }
    let (a1, b1) = (3, w - 4);
    let (a2, b2) = (6, w - 9);
    for (row, a, b) in [(t1, a1, b1), (t2, a2, b2)] {
        for x in a..=b {
            cv.put(x, row, '.', Color::Rgb(70, 64, 58), Some(floor_bg));
        }
    }
    let sx = w * 3 / 4;
    let top = surf[sx as usize];
    for y in top..=t2 {
        let ch = if y == top { '>' } else if y == t2 { '<' } else { 'X' };
        cv.put(sx, y, ch, Color::Rgb(160, 150, 140), Some(floor_bg));
    }
    // The forge glows; smoke climbs its chimney and out over the slope.
    let fx = a1 + 3;
    for x in a1..a1 + 6 {
        let hs = hash(x, (t / 3) as i32);
        let fire = [Color::Rgb(255, 220, 80), Color::Rgb(255, 140, 30), Color::Rgb(220, 60, 20)][(hs % 3) as usize];
        cv.put(x, t1 - 1, if hs % 2 == 0 { '≈' } else { '~' }, fire, Some(Color::Rgb(60, 15, 0)));
    }
    let chimney_top = surf[fx as usize];
    for y in chimney_top + 1..t1 - 1 {
        cv.put(fx, y, '│', Color::Rgb(90, 80, 75), Some(floor_bg));
    }
    for k in 0..6i32 {
        let age = ((t / 7) as i32 + k * 4) % 24;
        let y = chimney_top - 1 - age / 3;
        let x = fx + (age / 5) + ((hash(k, age) % 3) as i32 - 1) * (age / 12);
        if y >= 0 && y < surf.get(x.max(0) as usize).copied().unwrap_or(0) {
            let fade = 1.0 - age as f32 / 24.0;
            cv.put(x, y, if age < 10 { '░' } else { '·' }, mix((40, 36, 50), (150, 145, 150), fade), None);
        }
    }
    if (t / 4) % 5 == 0 {
        cv.put(a1 + 7, t1 - 1, '*', Color::Rgb(255, 255, 160), None);
    }
    // A dining table or two on the lower hall.
    for x in [a2 + 3, a2 + 6, a2 + 9] {
        cv.put(x, t2, '╥', Color::Rgb(170, 120, 70), Some(floor_bg));
    }
    // The miner hacks at the face; chips fly.
    cv.put(b2 + 1, t2, '█', Color::Rgb(110, 100, 95), Some(rock_bg));
    cv.put(b2, t2, '☺', Color::Rgb(255, 215, 60), Some(floor_bg));
    if (t / 5) % 3 == 0 {
        cv.put(b2 + 1, t2 - 1, '*', Color::Rgb(255, 255, 200), None);
    }

    // Dwarves going about their business.
    let walkers = [
        (t1, a1 + 7, b1, 5u64, 0u64, (120, 200, 255)),
        (t1, a1 + 9, b1 - 4, 7, 40, (255, 150, 120)),
        (t1, a1 + 6, sx, 6, 90, (160, 255, 150)),
        (t2, a2 + 1, b2 - 1, 6, 15, (230, 180, 255)),
        (t2, a2 + 1, sx, 9, 60, (255, 230, 140)),
    ];
    for (row, a, b, every, off, col) in walkers {
        let x = pingpong(t + off * every, every, a, b);
        cv.put(x, row, '☺', Color::Rgb(col.0, col.1, col.2), None);
    }
    let climber = pingpong(t + 33, 10, top + 1, t2 - 1);
    cv.put(sx, climber, '☺', Color::Rgb(200, 220, 255), None);

    // Something waits in a pocket of the deep caverns.
    let (cx, cy) = (w / 7, t2 + 2);
    if cy < magma_top - 1 {
        for dx in -3..=3 {
            cv.put(cx + dx, cy, ' ', Color::Black, Some(Color::Rgb(4, 3, 6)));
        }
        let blink = (t / 30) % 13;
        if blink >= 4 && blink != 9 {
            cv.put(cx - 1 + ((t / 120) % 3) as i32, cy, '•', Color::Rgb(255, 40, 40), None);
            cv.put(cx + 1 + ((t / 120) % 3) as i32, cy, '•', Color::Rgb(255, 40, 40), None);
        }
    }
    sky_h as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_backdrop_moves_and_fits_any_screen() {
        for (w, h) in [(20u16, 8u16), (80, 24), (200, 60)] {
            let area = Rect::new(0, 0, w, h);
            let mut a = Buffer::empty(area);
            let mut b = Buffer::empty(area);
            let sky = draw(&mut a, area, 100, Rect::default());
            draw(&mut b, area, 160, Rect::default());
            assert!(sky < h);
            if w >= 80 {
                assert_ne!(a, b, "{}x{} should animate", w, h);
                let text: String = a.content.iter().map(|c| c.symbol().to_string()).collect();
                assert!(text.contains('☺') && text.contains('≈'), "dwarves and magma on {}x{}", w, h);
            }
        }
    }
}
