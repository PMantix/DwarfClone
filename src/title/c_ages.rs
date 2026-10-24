//! The turning of the age: the sun and the moon wheel round each other
//! over the world in a slow dance, day and night chasing across the sky
//! as first one and then the other climbs, trailing sparks of gold and
//! silver, while on the hill below an old ring of stones throws shadows
//! that swing round with them. The seed picks the land, the stones and
//! the pace of the dance.

use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the age turns; the stones keep count",
    "sun and moon, partners since the world was young",
    "each dawn is a little older than the last",
    "the dance goes on whether anyone watches or not",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let tf = t as f32;
    let sky = cv.sky.min(h - 1).max(1);
    let g = (sky - 1).max(1);
    let salt = cv.salt();
    let pace = 0.012 + (cv.roll(1) % 5) as f32 * 0.001;
    let a = tf * pace + (cv.roll(2) % 628) as f32 / 100.0;
    // The pair's middle glides along, and they spin about it.
    let mid_x = w as f32 * 0.5 + (tf * pace * 0.21).sin() * w as f32 * 0.22;
    let mid_y = g as f32 * 0.55;
    let (rx, ry) = (w as f32 * 0.16, g as f32 * 0.3);
    let sun = (mid_x + a.cos() * rx, mid_y - a.sin() * ry);
    let moon = (mid_x - a.cos() * rx, mid_y + a.sin() * ry);
    let day = (a.sin() * 0.5 + 0.5).powf(0.8); // the sun high, the moon low

    // The sky, day or night as the dance goes.
    let (top, bot) = (lerp((6, 8, 24), (60, 120, 210), day), lerp((30, 30, 60), (180, 210, 240), day));
    let dusk = (1.0 - (a.sin()).abs()).powi(3);
    for y in 0..g.min(h) {
        let v = y as f32 / g.max(1) as f32;
        let c = lerp(lerp(top, bot, v), (240, 130, 80), dusk * v * v * 0.8);
        for x in 0..w {
            cv.fill(x, y, rgb(c));
        }
    }
    if day < 0.45 {
        cv.stars(g - 1, (12.0 + day * 60.0) as u32);
    }
    // The track they follow, marked like a dial.
    for k in 0..24 {
        let b = k as f32 / 24.0 * std::f32::consts::TAU;
        let (x, y) = (mid_x + b.cos() * rx, mid_y - b.sin() * ry);
        cv.glyph(x as i32, y as i32, if k % 6 == 0 { '+' } else { '·' }, mix(lerp(bot, (255, 255, 255), 0.3), (255, 230, 150), 0.3));
    }
    // Trails of sparks behind each.
    for j in 1..10 {
        let b = a - j as f32 * 0.07;
        let f = j as f32 / 10.0;
        let (x, y) = (mid_x + b.cos() * rx, mid_y - b.sin() * ry);
        cv.glyph(x as i32, y as i32, if j < 4 { '*' } else { '·' }, mix((255, 210, 100), bot, f));
        let (x, y) = (mid_x - b.cos() * rx, mid_y + b.sin() * ry);
        cv.glyph(x as i32, y as i32, if j < 4 { '*' } else { '·' }, mix((210, 220, 255), bot, f));
    }
    let r = (g as f32 * 0.13).max(0.8).min(3.0);
    let (sxi, syi) = (sun.0 as i32, sun.1 as i32);
    let (mxi, myi) = (moon.0 as i32, moon.1 as i32);
    cv.halo(sxi, syi, r * 3.5, (255, 220, 140), 0.5);
    cv.disc(sxi, syi, r, (255, 236, 170));
    cv.halo(mxi, myi, r * 3.0, (200, 210, 240), 0.3);
    cv.disc(mxi, myi, r * 0.9, (220, 226, 240));
    // The moon's face: a few dark seas.
    for k in 0..3 {
        let (dx, dy) = ((hash(k, 3) % 3) as i32 - 1, (hash(k, 4) % 2) as i32 - (k == 2) as i32);
        if r >= 1.5 {
            cv.tint(mxi + dx, myi + dy, rgb((180, 186, 206)));
        }
    }

    // The land, lit by whichever is up.
    let light = lerp((40, 50, 80), (255, 240, 200), day);
    let far = lerp(lerp((20, 24, 40), (110, 130, 150), day), bot, 0.3);
    for x in 0..w {
        let s = g - (ridge(x, w, seed, 4) * sky as f32 * 0.3) as i32;
        for y in s.max(0)..g {
            cv.fill(x, y, rgb(far));
        }
    }
    let hill = |x: i32| -> i32 { g + 1 - (ridge(x, w, seed, 14) * sky as f32 * 0.14) as i32 };
    let grass = lerp((14, 24, 20), (80, 130, 60), day);
    for x in 0..w {
        let s = hill(x);
        for y in s.max(0)..h {
            let hs = hash(x + salt, y);
            let ch = if hs % 6 == 0 { '"' } else { ' ' };
            cv.put(x, y, ch, rgb(lerp(grass, light, 0.25)), Some(rgb(grass)));
        }
    }
    // The ring of stones, and the shadows they throw away from the sun
    // (or from the moon by night).
    let ring_x = (w as f32 * (0.14 + (cv.roll(3) % 8) as f32 / 100.0)) as i32;
    let lamp = if day > 0.5 { sun.0 } else { moon.0 };
    let n = 5 + (cv.roll(4) % 3) as i32;
    for k in 0..n {
        let x = ring_x + k * 3;
        let base = hill(x) - 1;
        let tall = 2 + (hash(k, salt) % 2) as i32;
        let stone = lerp(lerp((40, 40, 50), (150, 150, 140), day), light, 0.1);
        let dir = if lamp < x as f32 { 1 } else { -1 };
        let len = 1 + (((x as f32 - lamp).abs() / w as f32) * 6.0) as i32;
        for i in 1..=len {
            cv.tint(x + i * dir, base + 1, rgb(lerp(grass, (0, 0, 0), 0.45)));
        }
        for j in 0..tall {
            cv.put(x, base - j, if j == tall - 1 { '▄' } else { ' ' }, rgb(stone), if j == tall - 1 { None } else { Some(rgb(stone)) });
        }
        cv.tint(x, base, rgb(stone));
    }
    // A figure keeps watch among the stones, robed, with a lantern by night.
    let fx = ring_x + n * 3 / 2;
    let fy = hill(fx) - 1;
    cv.glyph(fx, fy, '☺', rgb(lerp((150, 150, 170), (230, 230, 240), day)));
    if day < 0.4 {
        cv.halo(fx + 1, fy, 1.5, (255, 200, 120), 0.4);
        cv.glyph(fx + 1, fy, '¡', rgb((255, 210, 130)));
    }
}
