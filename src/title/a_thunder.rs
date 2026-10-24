//! A summer thunderstorm rolling in over the peaks: an anvil of cloud
//! sliding across the sky, its shadow and its grey curtains of rain
//! sweeping over the sunlit valley, lightning forking down onto the
//! summits and lighting everything at once, and dwarves running the hay in
//! before it breaks. The seed picks the hour, the storm's way in and how
//! often it strikes.

use super::a_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the mountain is louder than the forge today",
    "count the seconds; then run",
    "the peaks drink lightning and ask for more",
    "summer rain, and every gutter in the hold sings",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let g = (cv.sky - 1).max(1);
    let tf = t as f32;
    let salt = cv.salt();
    let evening = cv.roll(1) % 2 == 0;
    let (top, bot) = if evening { ((80, 90, 150), (250, 190, 130)) } else { ((70, 140, 220), (200, 225, 245)) };
    let p = (cv.roll(2) % 628) as f32 / 100.0;
    // Where the storm's front has got to: it rolls across, then starts over.
    let span = (w * 2 + 40) as u64;
    let front = ((t / 4 + cv.roll(3) as u64) % span) as i32 - 20;
    let gap = 70 + (cv.roll(4) % 60) as u64; // frames between strikes
    let since = (t + cv.roll(5) as u64) % gap;
    let strike = (t + cv.roll(5) as u64) / gap;
    let flash = since < 4 && since != 2;
    let cloud_base = (g as f32 * 0.45) as i32;
    let under = |x: i32| x < front;

    // Sky, with the cloud bank over everything the front has passed.
    for y in 0..g.min(h) {
        let base = lerp(top, bot, y as f32 / g.max(1) as f32);
        for x in 0..w {
            let edge = front as f32 - x as f32;
            let lump = ((x as f32 * 0.21 + p).sin() + (y as f32 * 0.9 + x as f32 * 0.07 - tf * 0.02).sin()) * 1.5;
            let depth = edge + lump - (y - cloud_base).max(0) as f32 * 3.0;
            let mut c = base;
            if depth > 0.0 {
                let dark = (depth / 12.0).min(1.0);
                let bulge = ((x as f32 * 0.4 + y as f32 * 1.3 + p).sin() * 0.5 + 0.5) * 0.25;
                c = lerp(lerp((150, 150, 165), (46, 48, 62), dark), (200, 200, 210), bulge * (1.0 - dark));
            }
            if flash {
                c = lerp(c, (230, 230, 255), 0.55);
            }
            cv.fill(x, y, rgb(c));
        }
    }
    if front < w {
        let (sx, sy) = ((w as f32 * 0.85) as i32, (g as f32 * 0.3) as i32);
        if !under(sx) {
            cv.halo(sx, sy, (g as f32 * 0.3).max(2.0), (255, 240, 200), 0.4);
            cv.disc(sx, sy, (g as f32 * 0.07).clamp(0.6, 2.0), (255, 250, 230));
        }
    }

    // The peaks.
    let rise = (g - 1).max(1) as f32 * 0.85;
    let surf: Vec<i32> = (0..w).map(|x| g - 1 - (peaks(x, w, seed, 70, 5) * rise) as i32).collect();
    for x in 0..w {
        let s = surf[x as usize];
        let dim = if under(x) { 0.45 } else { 0.0 };
        for y in s.max(0)..g {
            let snow = y - s < 2 && s < g - (rise * 0.6) as i32;
            let c = if snow { (230, 234, 242) } else { lerp((120, 110, 110), (80, 74, 76), (y - s) as f32 / 6.0) };
            let c = lerp(c, (30, 32, 44), if flash { 0.0 } else { dim });
            cv.put(x, y, if y == s { '▲' } else { ' ' }, rgb(lerp(c, (255, 255, 255), 0.3)), Some(rgb(c)));
        }
    }

    // The valley: meadow, haycocks, the hold's gate, a lane.
    for y in g.max(0)..h {
        for x in 0..w {
            let hs = hash(x + salt, y * 5);
            let lit = if under(x) && !flash { 0.5 } else { 0.0 };
            let bg = lerp(lerp((90, 140, 60), (60, 104, 44), (hs % 8) as f32 / 10.0), (24, 34, 30), lit);
            let ch = if hs % 7 == 0 { '"' } else if hs % 23 == 0 { '♣' } else { ' ' };
            cv.put(x, y, ch, rgb(lerp((140, 190, 90), (40, 60, 40), lit)), Some(rgb(bg)));
        }
    }

    // Rain curtains hanging from the cloud, leaning with the wind.
    let drops = (w * h / 14).max(8);
    for k in 0..drops {
        let speed = 1 + (k % 2) as u64;
        let y = cloud_base + ((hash(k + salt, 2) as u64 + t / speed) % (h - cloud_base).max(1) as u64) as i32;
        let x = (hash(k + salt, 3) % w.max(1) as u32) as i32 - (y - cloud_base) / 3;
        if x < 0 || x >= front - 6 {
            continue;
        }
        cv.glyph(x, y, '/', rgb(if flash { (230, 230, 255) } else { (150, 160, 190) }));
    }
    for y in g..h {
        for x in 0..front.min(w) {
            if hash(x * 3 + (t / 3) as i32, y * 7) % 37 == 0 {
                cv.glyph(x, y, '°', rgb((170, 180, 210)));
            }
        }
    }

    // Lightning: a fork from the cloud down onto a peak under it.
    if since < 6 && front > 4 {
        let target = (hash(strike as i32 + salt, 9) % front.min(w).max(1) as u32) as i32;
        let ground = surf[target.clamp(0, w - 1) as usize];
        let mut x = target + (hash(strike as i32, 4) % 7) as i32 - 3;
        let col = if since < 3 { (255, 255, 240) } else { (180, 180, 255) };
        for y in cloud_base..ground {
            let jog = (hash(strike as i32 * 31 + y, salt) % 3) as i32 - 1;
            let ch = match jog {
                -1 => '╱',
                1 => '╲',
                _ => '│',
            };
            x += jog;
            cv.glyph(x, y, ch, rgb(col));
            if y % 4 == 1 && hash(y, strike as i32) % 2 == 0 {
                cv.glyph(x + 1, y + 1, '╲', rgb(lerp(col, (120, 120, 200), 0.5)));
            }
        }
        cv.halo(x, ground, 3.0, (220, 220, 255), 0.5);
    }

    // The hold's gate, and dwarves hurrying the hay in ahead of the rain.
    let gx = (w as f32 * 0.62) as i32;
    if g >= 4 {
        let s = surf[gx.clamp(0, w - 1) as usize];
        if s < g - 3 {
            for y in g - 2..g {
                for x in gx - 1..=gx + 1 {
                    cv.fill(x, y, rgb((20, 16, 14)));
                }
            }
            cv.halo(gx, g - 1, 2.0, (255, 190, 90), if under(gx) { 0.5 } else { 0.15 });
        }
    }
    let run = (w / 18).max(2);
    for k in 0..run {
        let x = ((t / 3) as i32 + k * 11) % (gx - 1).max(5);
        dwarf(cv, x, g - 1, COATS[(k % 8) as usize]);
        if k % 2 == 0 {
            cv.glyph(x + 1, g - 2, '▲', rgb((220, 190, 90)));
        }
    }
}
