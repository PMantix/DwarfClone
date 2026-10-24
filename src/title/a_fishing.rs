//! A lone dwarf fishing from a little boat on a mountain lake: the peaks
//! upside down in the still water, the float bobbing and now and then
//! going under, the rod bending, a fish flashing up on the line, rings
//! spreading where others rise, and loons crossing far out. The seed
//! picks the hour, the weather and where the boat lies.

use super::a_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the only quiet dwarf in the mountain",
    "no orders, no moods, no goblins: just the float",
    "the big one is always the one that got away",
    "patience is a kind of stone",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let g = (cv.sky - 1).max(1);
    let tf = t as f32;
    let salt = cv.salt();
    let hour = cv.roll(1) % 3; // morning mist, high noon, sunset
    let (top, bot) = [((120, 140, 180), (220, 220, 225)), ((60, 120, 210), (170, 210, 240)), ((50, 40, 100), (250, 150, 90))][hour as usize];
    let water = lerp(bot, (20, 50, 80), 0.6);

    cv.sky_fill(g, top, bot);
    if hour == 2 {
        let (sx, sy) = ((w as f32 * 0.75) as i32, (g as f32 * 0.7) as i32);
        cv.halo(sx, sy, (g as f32 * 0.45).max(2.0), (255, 170, 90), 0.5);
        cv.disc(sx, sy, (g as f32 * 0.09).clamp(0.7, 2.6), (255, 210, 130));
    }
    clouds(cv, (w / 45).max(1), (1, (g / 3).max(2)), 20, lerp(bot, (255, 255, 255), 0.4), lerp(bot, (255, 255, 255), 0.7), salt);

    // The mountains around the lake.
    let rise = (g - 1).max(1) as f32 * 0.8;
    let surf: Vec<i32> = (0..w).map(|x| g - 1 - (peaks(x, w, seed, 100, 4).max(ridge(x, w, seed, 105) * 0.4) * rise) as i32).collect();
    let rock = |y: i32, s: i32| -> Rgb {
        let snow = y - s < 2 && s < g - (rise * 0.55) as i32;
        if snow {
            lerp((235, 238, 245), bot, 0.2)
        } else if y - s < 3 {
            lerp((96, 92, 100), bot, 0.3)
        } else {
            lerp((40, 64, 50), bot, 0.15)
        }
    };
    for x in 0..w {
        let s = surf[x as usize];
        for y in s.max(0)..g {
            let c = rock(y, s);
            let ch = if y == s { '▲' } else if y - s > 2 && hash(x + salt, y) % 3 == 0 { '♠' } else { ' ' };
            cv.put(x, y, ch, rgb(lerp(c, if ch == '♠' { (20, 40, 30) } else { (255, 255, 255) }, 0.4)), Some(rgb(c)));
        }
    }

    // The lake, holding the mountains upside down, wavering.
    for y in g.max(0)..h {
        let d = (y - g) as f32 / (h - g).max(1) as f32;
        let src = 2 * g - y - 1;
        let wob = ((y as f32 * 1.1 + tf * 0.06).sin() * 0.8) as i32;
        for x in 0..w {
            let sx = (x + wob).clamp(0, w - 1);
            let s = surf[sx as usize];
            let base = lerp(water, lerp(water, (8, 18, 34), 0.5), d);
            let c = if src >= s && src < g { lerp(rock(src, s), base, 0.5 + d * 0.3) } else { lerp(lerp(top, bot, (src.max(0)) as f32 / g.max(1) as f32), base, 0.55 + d * 0.3) };
            let ripple = (x as f32 * 0.3 + y as f32 * 2.3 - tf * 0.05).sin() > 0.93;
            cv.put(x, y, if ripple { '─' } else { ' ' }, rgb(lerp(c, (255, 255, 255), 0.3)), Some(rgb(c)));
        }
    }
    for x in 0..w {
        cv.glyph(x, g, if hash(x, salt) % 3 == 0 { '▀' } else { '▄' }, rgb((70, 80, 60)));
    }

    // The boat, off to one side so it shows beside the menu.
    let by = g + 2 + ((h - g) / 6).min(3);
    let bx = (w as f32 * 0.1) as i32 + (cv.roll(2) % 4) as i32;
    let roll = ((tf * 0.08).sin() * 0.6).round() as i32;
    cv.sprite(bx - 2, by + 1, &["▀▄▄▄▄▀"], rgb((120, 76, 40)));
    dwarf(cv, bx, by + roll.min(0), (100, 140, 200));
    cv.glyph(bx, by - 1 + roll.min(0), '▄', rgb((170, 130, 70))); // a hat
    // The float, and what happens to it.
    let cycle = 240 + (cv.roll(3) % 120) as u64;
    let c = (t + cv.roll(4) as u64) % cycle;
    let fx = bx + 7;
    let (rod, float_y, catch) = if c < cycle - 60 {
        // Waiting: the float bobs, sometimes nibbles.
        let nib = c % 47 < 3;
        ('/', by + 1 + nib as i32, false)
    } else if c < cycle - 40 {
        // It goes under; the rod bends.
        (')', by + 2, false)
    } else {
        ('/', by, true)
    };
    let tip = (bx + 3, by - 2);
    cv.glyph(bx + 1, by - 1, '/', rgb((150, 110, 60)));
    cv.glyph(bx + 2, by - 2, if rod == ')' { '─' } else { '/' }, rgb((150, 110, 60)));
    for (i, (x, y)) in line(tip, (fx, float_y - 1)).into_iter().enumerate() {
        if i > 0 {
            cv.glyph(x, y, '·', rgb((200, 200, 200)));
        }
    }
    if catch {
        let up = ((c - (cycle - 40)) / 5) as i32;
        cv.glyph(fx - up / 2, float_y - 1 - up.min(3), '►', rgb((200, 220, 230)));
        cv.glyph(fx - up / 2 + 1, float_y - 1 - up.min(3), '°', rgb((240, 250, 255)));
    } else {
        cv.glyph(fx, float_y, 'o', rgb((230, 60, 50)));
    }
    // Rings round the float and where fish rise.
    let ring = |cv: &mut Canvas, x: i32, y: i32, age: u64| {
        let r = (age / 6) as i32 + 1;
        if r < 6 {
            let col = lerp((220, 230, 240), water, r as f32 / 6.0);
            cv.glyph(x - r * 2, y, '(', rgb(col));
            cv.glyph(x + r * 2, y, ')', rgb(col));
            if r > 1 {
                for i in (x - r * 2 + 1)..(x + r * 2) {
                    if i % 2 == 0 {
                        cv.glyph(i, y - 1 + (r % 2), '─', rgb(lerp(col, water, 0.5)));
                    }
                }
            }
        }
    };
    ring(cv, fx, by + 1, (c % 36).min(35));
    for k in 0..(w / 40).max(1) {
        let period = 150 + (hash(k + salt, 3) % 100) as u64;
        let age = (t + hash(k, 5) as u64) % period;
        let round = ((t + hash(k, 5) as u64) / period) as i32;
        let x = (hash(k + salt, round) % w.max(1) as u32) as i32;
        let y = g + 2 + (hash(k, round + 1) % (h - g - 2).max(1) as u32) as i32;
        if age < 8 {
            cv.glyph(x, y - 1, '►', rgb((190, 210, 220)));
        } else {
            ring(cv, x, y, age - 8);
        }
    }

    // Loons far out, diving and coming up again.
    for k in 0..2 {
        let x = (w as f32 * (0.6 + k as f32 * 0.2)) as i32 + pingpong(t, 25, 0, 8);
        if (t / 90 + k as u64) % 3 != 0 {
            cv.glyph(x, g + 1, '▄', rgb((20, 24, 30)));
            cv.glyph(x + 1, g + 1, '·', rgb((20, 24, 30)));
        }
    }
    // Morning mist on the water.
    if hour == 0 {
        for k in 0..3 {
            let len = (w / 3).max(5);
            let x0 = drift(t, k + salt, 8, w + len) - len / 2;
            for i in 0..len {
                let f = (i as f32 / len as f32 * std::f32::consts::PI).sin();
                cv.glow(x0 + i, g + k, (230, 232, 236), f * 0.4);
            }
        }
    }
}
