//! The spring thaw: snow going grey and thin on the peaks, meltwater
//! threading down every gully, icicles dripping from the hold's lintel,
//! the river below running high and carrying the last of the ice away,
//! and the first flowers out on the bare patches. The seed picks the
//! hour, the flowers, and how far the thaw has got.

use super::a_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the mountain wakes up wet and hungry",
    "every gully a stream, every stream a song",
    "the ice goes out, the traders come in",
    "first flowers on the slope, first mud in the hall",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let g = (cv.sky - 1).max(1);
    let tf = t as f32;
    let salt = cv.salt();
    let hour = cv.roll(1) % 3; // morning, noon, afternoon
    let (top, bot) = [((90, 140, 210), (230, 210, 200)), ((70, 140, 220), (190, 225, 245)), ((80, 110, 190), (245, 200, 160))][hour as usize];
    let thaw = 0.3 + (cv.roll(2) % 50) as f32 / 100.0; // how much snow has gone
    let bloom = [(190, 120, 230), (250, 220, 80), (250, 250, 250), (120, 150, 250)];

    cv.sky_fill(g, top, bot);
    let (sx, sy) = ((w as f32 * 0.8) as i32, (g as f32 * (0.2 + hour as f32 * 0.1)) as i32 + 1);
    cv.halo(sx, sy, (g as f32 * 0.3).max(2.0), (255, 245, 210), 0.45);
    cv.disc(sx, sy, (g as f32 * 0.07).clamp(0.6, 2.2), (255, 250, 230));
    clouds(cv, (w / 30).max(2), (1, (g / 3).max(2)), 9, (240, 244, 250), (255, 255, 255), salt);

    // The peaks, their snow grey and shrinking.
    let rise = (g - 1).max(1) as f32;
    let surf: Vec<i32> = (0..w).map(|x| g - 1 - (peaks(x, w, seed, 30, 4).max(ridge(x, w, seed, 36) * 0.45) * rise * 0.92) as i32).collect();
    let snowline = |x: i32| g - (rise * (0.35 + thaw * 0.45)) as i32 + ((x as f32 * 0.45).sin() * 1.5) as i32;
    for x in 0..w {
        let s = surf[x as usize];
        for y in s.max(0)..g {
            let d = (y - s) as f32 / 5.0;
            let snowy = y < snowline(x) && hash(x + salt, y * 3) % 5 != 0;
            let (ch, fg, bg) = if snowy {
                (if y == s { '▲' } else { ' ' }, (240, 244, 250), lerp((222, 228, 238), (180, 186, 200), d))
            } else {
                let grass = y > snowline(x) + 2;
                let rock = if grass { lerp((96, 116, 70), (70, 86, 52), d) } else { lerp((112, 104, 100), (82, 76, 76), d) };
                (if y == s { '▲' } else if hash(x * 3 + salt, y) % 9 == 0 { '░' } else { ' ' }, lerp(rock, (220, 220, 220), 0.3), rock)
            };
            cv.put(x, y, ch, rgb(fg), Some(rgb(bg)));
        }
    }

    // Meltwater down the gullies: sparkling threads that run downhill.
    let streams = (w / 12).max(2);
    for k in 0..streams {
        let x0 = ((k as f32 + 0.5) / streams as f32 * w as f32) as i32 + (hash(k + salt, 5) % 5) as i32 - 2;
        let start = snowline(x0).max(surf[x0.clamp(0, w - 1) as usize] + 1);
        for y in start..g {
            let x = x0 + ((y as f32 * 0.6 + k as f32 * 1.3).sin() * 1.4) as i32;
            if x < 0 || x >= w || y < surf[x as usize] {
                continue;
            }
            let run = (y as i64 - (t / 2) as i64 + k as i64 * 3).rem_euclid(5);
            let ch = if run == 0 { '·' } else if (x + y) % 2 == 0 { '│' } else { '╎' };
            cv.put(x, y, ch, rgb(if run == 0 { (255, 255, 255) } else { (170, 210, 240) }), Some(rgb((70, 120, 170))));
        }
    }

    // The meadow: wet grass, old snow in the hollows, first flowers.
    let river_top = g + ((h - g) as f32 * 0.45) as i32;
    let river_bot = (river_top + ((h - g) / 4).max(2)).min(h);
    for y in g..river_top.min(h) {
        for x in 0..w {
            let hs = hash(x + salt, y * 7 + 3);
            let patch = ((x as f32 * 0.13 + y as f32 * 0.7).sin() + (x as f32 * 0.05 - y as f32 * 0.3 + 1.0).sin()) * 0.5 > 0.55 - thaw * 0.3;
            if patch {
                let ch = if hs % 6 == 0 { '░' } else { ' ' };
                cv.put(x, y, ch, rgb((200, 206, 214)), Some(rgb((214, 220, 230))));
            } else {
                let bg = lerp((84, 110, 56), (64, 88, 44), (hs % 10) as f32 / 20.0);
                let (ch, fg) = match hs % 11 {
                    0 => ('*', bloom[(hs / 11 % 4) as usize]),
                    1 | 2 => ('"', (130, 170, 80)),
                    3 => (',', (110, 90, 60)),
                    _ => (' ', (0, 0, 0)),
                };
                // Flowers nod in the breeze.
                let ch = if ch == '*' && (t / 20 + hs as u64) % 7 == 0 { '·' } else { ch };
                cv.put(x, y, ch, rgb(fg), Some(rgb(bg)));
            }
        }
    }

    // The river running high, ice floes riding it.
    for y in river_top.max(0)..river_bot {
        for x in 0..w {
            let f = (x as f32 * 0.35 - tf * 0.25 + y as f32 * 1.7).sin();
            let ch = if f > 0.7 { '≈' } else if f < -0.85 { '~' } else { ' ' };
            cv.put(x, y, ch, rgb((180, 210, 235)), Some(rgb(lerp((60, 110, 160), (40, 86, 140), (y - river_top) as f32 / 3.0))));
        }
    }
    for k in 0..(w / 10).max(2) {
        let len = 2 + (hash(k + salt, 41) % 4) as i32;
        let y = river_top + (hash(k + salt, 43) % (river_bot - river_top).max(1) as u32) as i32;
        let x = drift(t, k + salt, 3 + (k % 2) as u64, w + len + 4) - len;
        for i in 0..len {
            cv.glyph(x + i, y, if i == 0 || i == len - 1 { '▄' } else { '█' }, rgb((236, 242, 250)));
        }
    }
    // The far bank, muddy, beyond it more meadow.
    for y in river_bot..h {
        for x in 0..w {
            let hs = hash(x * 3 + salt, y * 11);
            let ch = if hs % 13 == 0 { '"' } else if hs % 17 == 0 { '*' } else { ' ' };
            let fg = if ch == '*' { bloom[(hs % 4) as usize] } else { (120, 160, 70) };
            cv.put(x, y, ch, rgb(fg), Some(rgb(if y == river_bot { (96, 80, 58) } else { (76, 100, 50) })));
        }
    }

    // The hold's door on the slope, icicles on the lintel, dripping.
    let hx = (w as f32 * 0.18) as i32 + (cv.roll(3) % 5) as i32;
    if g >= 4 && hx < w {
        for y in g - 2..g {
            cv.fill(hx, y, rgb((40, 30, 24)));
        }
        for x in hx - 2..=hx + 2 {
            cv.glyph(x, g - 3, '▄', rgb((130, 120, 110)));
        }
        for (i, x) in (hx - 2..=hx + 2).enumerate() {
            if i % 2 == 0 {
                cv.glyph(x, g - 2, '▼', rgb((220, 236, 250)));
                let fall = ((t / 3) as i32 + i as i32 * 5) % 9;
                if fall < 3 {
                    cv.glyph(x, g - 1 + fall / 2, '·', rgb((160, 200, 240)));
                }
            }
        }
        // A dwarf with a broom sweeping out the slush.
        let bx = hx + 3 + pingpong(t, 14, 0, 4);
        dwarf(cv, bx, g - 1, (200, 150, 110));
        cv.glyph(bx + 1, g - 1, if (t / 6) % 2 == 0 { '/' } else { '|' }, rgb((170, 140, 90)));
    }

    // Birds coming home.
    for k in 0..(w / 30).max(2) {
        let x = drift(t, k + salt, 2, w + 10) - 5;
        let y = 2 + (hash(k, 77) % (g / 2).max(1) as u32) as i32 + ((tf * 0.1 + k as f32).sin() * 0.8) as i32;
        cv.glyph(x, y, if (t / 5 + k as u64) % 2 == 0 { 'v' } else { '^' }, rgb((50, 50, 60)));
    }
}
