//! A meteor shower over the fortress: a clear moonless night, the milky
//! river of stars arching over, shooting stars streaking down by the
//! dozen with now and then a great fireball that lights the snow, the
//! fortress dark in its mountain with a few windows lit, and dwarves out
//! on the battlements pointing. The seed picks the radiant, the colours
//! and the fort.

use super::a_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the sky is falling, and it is beautiful",
    "make a wish; then go find the iron",
    "somewhere out there, a star landed on a goblin",
    "even the miners came up to look",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let g = (cv.sky - 1).max(1);
    let salt = cv.salt();
    let tint = [(255, 255, 255), (200, 230, 255), (255, 230, 190)][(cv.roll(1) % 3) as usize];
    let p = (cv.roll(2) % 628) as f32 / 100.0;
    // Where the meteors seem to come from.
    let (rx, ry) = ((w as f32 * (0.2 + (cv.roll(3) % 60) as f32 / 100.0)) as i32, -(g / 3) - 2);

    // The night, with the milky river slanting across it.
    for y in 0..g.min(h) {
        let base = lerp((2, 4, 16), (12, 16, 38), y as f32 / g.max(1) as f32);
        for x in 0..w {
            let band = (y as f32 - (g as f32 * 0.7 - x as f32 * 0.25 + (x as f32 * 0.05 + p).sin() * 3.0)).abs();
            let m = (1.0 - band / (g as f32 * 0.25).max(2.0)).max(0.0);
            let c = lerp(base, (60, 60, 90), m * m * 0.7);
            cv.fill(x, y, rgb(c));
            let hs = hash(x * 7 + 3, y * 13 + 1);
            if hs % (if m > 0.3 { 7 } else { 23 }) == 0 {
                let tw = (t / 6 + hs as u64 / 53) % 19 == 0;
                cv.glyph(x, y, if tw { '+' } else { '·' }, rgb(if tw { (255, 255, 230) } else { lerp((110, 110, 150), (200, 200, 230), m) }));
            }
        }
    }

    // A fireball every so often lights everything.
    let fb_every = 300 + (cv.roll(4) % 200) as u64;
    let fb_age = (t + cv.roll(5) as u64) % fb_every;
    let fb_round = ((t + cv.roll(5) as u64) / fb_every) as i32;
    let fb_life = 40u64;
    let lit = if fb_age < fb_life { 1.0 - fb_age as f32 / fb_life as f32 } else { 0.0 };

    // Mountains with the fort cut into them, snow on the slopes.
    let rise = (g - 1).max(1) as f32 * 0.8;
    let surf: Vec<i32> = (0..w).map(|x| g - 1 - (peaks(x, w, seed, 200, 4).max(ridge(x, w, seed, 205) * 0.4) * rise) as i32).collect();
    for x in 0..w {
        let s = surf[x as usize];
        for y in s.max(0)..g {
            let snow = y - s < 2;
            let c = if snow { (70, 76, 100) } else { (22, 22, 34) };
            let c = lerp(c, (200, 190, 170), lit * if snow { 0.5 } else { 0.25 });
            cv.put(x, y, if y == s { '▲' } else { ' ' }, rgb(lerp(c, (200, 210, 240), 0.4)), Some(rgb(c)));
        }
    }
    for y in g.max(0)..h {
        for x in 0..w {
            let hs = hash(x * 3 + salt, y * 5);
            let c = lerp(lerp((50, 56, 80), (30, 34, 56), (y - g) as f32 / (h - g).max(1) as f32), (210, 200, 180), lit * 0.35);
            cv.put(x, y, if hs % 23 == 0 { '♠' } else if hs % 11 == 0 { '·' } else { ' ' }, rgb(lerp(c, (10, 20, 20), 0.6)), Some(rgb(c)));
        }
    }

    // The fortress: a gate, walls with crenels, a tower, lit windows.
    let fx = (w as f32 * 0.2) as i32 + (cv.roll(6) % 6) as i32;
    if g >= 6 {
        let top = g - 4;
        for x in fx - 6..=fx + 6 {
            for y in top..g {
                cv.put(x, y, ' ', Color::Black, Some(rgb(lerp((40, 38, 46), (180, 170, 150), lit * 0.3))));
            }
            if (x - fx) % 2 == 0 {
                cv.glyph(x, top - 1, '▄', rgb((60, 58, 66)));
            }
        }
        for y in (top - 4).max(0)..top {
            for x in fx + 3..=fx + 5 {
                cv.put(x, y, ' ', Color::Black, Some(rgb((46, 44, 52))));
            }
        }
        cv.glyph(fx + 4, top - 5, '▲', rgb((60, 58, 66)));
        for y in g - 2..g {
            cv.fill(fx, y, rgb((255, 190, 90)));
        }
        for (k, (dx, dy)) in [(-4, 2), (-2, 2), (3, 2), (5, 1), (4, -2)].iter().enumerate() {
            let on = (t / 17 + k as u64 * 5) % 23 != 0;
            cv.glyph(fx + dx, top + dy, '■', rgb(if on { (255, 210, 110) } else { (120, 90, 50) }));
        }
        // Watchers on the wall, one pointing at the sky.
        for k in 0..4 {
            let x = fx - 5 + k * 3;
            cv.glyph(x, top - 2, '☺', rgb(COATS[((k + salt) % 8) as usize]));
        }
        let point = (t / 15) % 3 != 0;
        cv.glyph(fx - 4, top - 3, if point { '/' } else { '│' }, rgb((220, 200, 180)));
    }

    // A watchman walking the foot of the wall with his lantern.
    let wx = fx + 8 + pingpong(t, 6, 0, (w / 6).max(4));
    dwarf(cv, wx, g - 1, (160, 170, 200));
    cv.glyph(wx + 1, g - 1, '¡', rgb((255, 210, 110)));
    cv.halo(wx + 1, g - 1, 2.0, (255, 190, 90), 0.45);

    // The shower: meteors streaking away from the radiant.
    let n = (w / 8).max(6);
    for k in 0..n {
        let life = 30 + (hash(k + salt, 1) % 30) as u64;
        let gap = life + 10 + (hash(k + salt, 2) % 60) as u64;
        let all = t + hash(k + salt, 3) as u64 % gap;
        let a = all % gap;
        if a >= life {
            continue;
        }
        let round = (all / gap) as i32;
        let hs = hash(k + salt, round * 13 + 5);
        // A start somewhere in the sky, heading away from the radiant.
        let (sx, sy) = ((hs % w.max(1) as u32) as i32, ((hs >> 10) % (g as u32 * 2 / 3).max(1)) as i32);
        let (dx, dy) = ((sx - rx) as f32, (sy - ry) as f32 * 2.0);
        let d = (dx * dx + dy * dy).sqrt().max(1.0);
        let (ux, uy) = (dx / d, dy / d * 0.5);
        let speed = 1.2 + (hs % 10) as f32 * 0.12;
        let along = a as f32 * speed;
        let fade = 1.0 - a as f32 / life as f32;
        let ch = if ux.abs() > uy.abs() * 3.0 { '─' } else if (ux > 0.0) == (uy > 0.0) { '╲' } else { '╱' };
        for i in 0..8 {
            let back = along - i as f32 * 1.1;
            if back < 0.0 {
                break;
            }
            let (x, y) = ((sx as f32 + ux * back).round() as i32, (sy as f32 + uy * back).round() as i32);
            if y >= g {
                continue;
            }
            let f = fade * (1.0 - i as f32 / 8.0);
            cv.glyph(x, y, if i == 0 { '*' } else { ch }, rgb(lerp((20, 24, 50), tint, f)));
        }
    }
    // The fireball itself.
    if lit > 0.0 {
        let hs = hash(fb_round + salt, 77);
        let (sx, sy) = ((hs % w.max(1) as u32) as i32, 1 + ((hs >> 9) % (g as u32 / 3).max(1)) as i32);
        let dir = if hs % 2 == 0 { 1 } else { -1 };
        let along = fb_age as i32;
        let (x, y) = (sx + dir * along, sy + along / 3);
        cv.halo(x, y, 4.0, (255, 200, 140), lit * 0.8);
        for i in 0..12 {
            let (tx, ty) = (x - dir * i, y - i / 3);
            cv.glyph(tx, ty, if i == 0 { '☼' } else if i < 4 { '▬' } else { '·' }, rgb(lerp((255, 240, 200), (200, 80, 40), i as f32 / 12.0)));
        }
    }
}
