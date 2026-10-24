//! The comet: a great star with a burning tail hung over the world, its
//! light shimmering down the length of it, shards of it falling as
//! shooting stars, while below on a dark hill the folk of a village come
//! out with lanterns to stare and point, and the old standing stones
//! catch its glow. The seed picks the comet's colour, its path and the
//! lie of the land.

use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the seers agree it means something; not what",
    "a new star with a tail of fire: an age is ending",
    "it came last when the first king fell",
    "everyone looks up; nobody watches the road",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let tf = t as f32;
    let sky = cv.sky.min(h - 1).max(1);
    let g = (sky - 1).max(1);
    let salt = cv.salt();
    let (core, tail) = [((230, 245, 255), (120, 180, 255)), ((220, 255, 230), (90, 230, 160)), ((255, 230, 210), (240, 90, 70)), ((255, 250, 220), (230, 190, 110))][(cv.roll(1) % 4) as usize];
    let (top, bot) = ((4, 6, 18), (24, 26, 54));
    cv.sky_fill(h, top, bot);
    cv.stars(g, 23);

    // The comet: its head drifting very slowly, the tail streaming away
    // up and back across the sky.
    let drift = (tf / 700.0).sin() * 3.0;
    let hx = w as f32 * (0.78 + (cv.roll(2) % 10) as f32 / 100.0) - drift;
    let hy = (g as f32 * 0.45).max(1.0);
    let (tx, ty) = (w as f32 * 0.08, -(g as f32) * 0.3); // where the tail points
    let (ax, ay) = ((tx - hx) * 0.5, ty - hy); // (in "round" units, cells being tall)
    let alen = (ax * ax + ay * ay).sqrt().max(1.0);
    let (ux, uy) = (ax / alen, ay / alen);
    for y in 0..g.min(h) {
        for x in 0..w {
            let (dx, dy) = ((x as f32 - hx) * 0.5, y as f32 - hy);
            let along = dx * ux + dy * uy;
            let across = (dx * -uy + dy * ux).abs();
            if along < -1.0 || along > alen {
                continue;
            }
            let u = (along / alen).max(0.0);
            let width = 0.6 + u * g as f32 * 0.35;
            let fall = (1.0 - u).powf(1.3) * (-(across / width).powi(2)).exp();
            let shimmer = 0.75 + 0.25 * (u * 40.0 - tf * 0.15 + across).sin();
            let f = (fall * shimmer).min(1.0);
            if f > 0.03 {
                cv.glow(x, y, lerp(tail, core, (1.0 - u * 2.0).max(0.0)), f * 0.8);
                if f > 0.25 && hash(x + (t / 3) as i32, y + salt) % 5 == 0 {
                    cv.glyph(x, y, if hash(x, y) % 2 == 0 { '·' } else { '\'' }, rgb(lerp(core, tail, u)));
                }
            }
        }
    }
    let (hxi, hyi) = (hx as i32, hy as i32);
    cv.halo(hxi, hyi, 3.5, core, 0.7);
    cv.disc(hxi, hyi, 0.6, (255, 255, 255));
    cv.glyph(hxi, hyi, '*', rgb((255, 255, 255)));

    // Pieces of it falling.
    for k in 0..3u64 {
        let period = 97 + k * 41;
        let age = (t + k * 37) % period;
        if age < 16 {
            let n = ((t + k * 37) / period) as i32;
            let (x0, y0) = ((hash(n, k as i32) % w.max(1) as u32) as i32, (hash(k as i32, n) % (g.max(2) as u32 / 2 + 1)) as i32);
            for j in 0..5 {
                let s = age as i32 - j;
                if s >= 0 {
                    cv.glyph(x0 - s * 2, y0 + s, if j == 0 { '*' } else { '·' }, mix(core, top, j as f32 / 5.0));
                }
            }
        }
    }

    // The dark land: hills, a village with lit windows, standing stones.
    let land = (12, 14, 22);
    let surf = |x: i32| -> i32 { g - (ridge(x, w, seed, 6) * sky as f32 * 0.22) as i32 };
    for x in 0..w {
        let s = surf(x);
        for y in s.max(0)..h {
            let lit = 0.12 * (1.0 - ((x as f32 - hx).abs() / w as f32));
            let ch = if hash(x + salt, y) % 9 == 0 { '"' } else { ' ' };
            cv.put(x, y, ch, rgb((30, 40, 40)), Some(rgb(lerp(land, tail, lit))));
        }
        if hash(x, salt + 4) % 7 == 0 {
            cv.glyph(x, s - 1, '♠', rgb((14, 20, 24)));
        }
    }
    // The village, off to one side.
    let vx = (w as f32 * 0.12) as i32;
    for k in 0..(3 + w / 40) {
        let x = vx + k * 4 + (hash(k, salt) % 2) as i32;
        let base = surf(x);
        cv.put(x, base - 1, '▲', rgb((30, 26, 34)), Some(rgb(land)));
        cv.put(x + 1, base - 1, '▀', rgb((30, 26, 34)), Some(rgb((22, 20, 28))));
        let on = (t / 13 + hash(k, 3) as u64) % 17 != 0;
        cv.put(x, base, if on { '▪' } else { ' ' }, rgb((255, 200, 100)), Some(rgb((22, 20, 28))));
        cv.put(x + 1, base, ' ', Color::Black, Some(rgb((22, 20, 28))));
        if on {
            cv.halo(x, base, 1.2, (255, 190, 90), 0.2);
        }
    }
    // The watchers on the hill, with lanterns, pointing up.
    let wx = (w as f32 * 0.3) as i32 + (cv.roll(3) % 6) as i32;
    for k in 0..(2 + cv.roll(4) % 3) as i32 {
        let x = wx + k * 2;
        let y = surf(x) - 1;
        cv.glyph(x, y, '☺', rgb([(200, 190, 170), (170, 190, 220), (220, 170, 170)][(k % 3) as usize]));
        let point = (t / 40 + k as u64) % 3 != 0;
        cv.glyph(x + 1, y - 1, if point { '/' } else { '│' }, rgb((180, 170, 160)));
        if k % 2 == 0 {
            cv.halo(x - 1, y, 1.5, (255, 200, 110), 0.35);
            cv.glyph(x - 1, y, '¡', rgb((255, 210, 120)));
        }
    }
    // The ring of standing stones, lit on the comet's side.
    let sx = (w as f32 * 0.62) as i32;
    for k in 0..5 {
        let x = sx + k * 3;
        let y = surf(x) - 1;
        cv.put(x, y, ' ', Color::Black, Some(rgb((60, 62, 74))));
        cv.put(x, y - 1, '▄', rgb((60, 62, 74)), None);
        cv.glow(x, y, tail, 0.15 + 0.1 * (tf * 0.05 + k as f32).sin().max(0.0));
    }
    // An owl's eyes in a tree, blinking.
    if (t / 11) % 13 != 0 {
        let ox = (w as f32 * 0.5) as i32 + (cv.roll(5) % 20) as i32;
        cv.glyph(ox, surf(ox) - 1, '"', rgb((230, 200, 80)));
    }
}
