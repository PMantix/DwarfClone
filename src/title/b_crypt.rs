//! The crypt under the fortress: a vaulted hall of stone coffins and
//! guttering candles. Night after night one lid grinds aside, and what
//! the fortress buried as its old mayor sits up, pale, eyes like coals,
//! and goes walking with the bats while the candles shrink from it. By
//! morning the lid is back. Below, the ossuary, and mist. The seed picks
//! the stone, the candles and which coffin it is.

use super::b_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the old mayor never seems to age",
    "someone keeps moving the lid",
    "the cats will not go into the crypt",
    "sleep with your door barred tonight",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t) = (cv.w(), cv.h(), cv.t);
    let g = (cv.sky - 1).max(3).min(h - 1);
    let tf = t as f32;
    let salt = cv.salt();
    let stone = [(70, 66, 74), (80, 74, 66), (60, 64, 70)][(cv.roll(1) % 3) as usize];
    let flame = [(255, 190, 90), (190, 150, 255), (140, 230, 160)][(cv.roll(2) % 3) as usize];
    let air = lerp(stone, (0, 0, 0), 0.82);
    bedrock(cv, 0, h, stone, salt);

    // The beat: still; the lid moves; it rises and walks; it returns.
    let at = beat(cv, 1600, 3);
    let (lid_at, up_at, walk_at, back_at, down_at) = (500u64, 560u64, 640u64, 1150u64, 1260u64);
    let lid_open = if at < lid_at { 0.0 } else if at < down_at { span(at, lid_at, up_at) } else { 1.0 - span(at, down_at, down_at + 60) };
    let awake = at >= up_at && at < down_at;

    // The vault: the hall, and arches between the pillars.
    let top = 1;
    hollow(cv, 0, top, w, g, air);
    floor(cv, 0, w, g, lerp(stone, (150, 140, 150), 0.25));
    let bay = 14 + (cv.roll(4) % 5) as i32;
    let mut px = 2 + (cv.roll(5) % 6) as i32;
    let mut pillars = Vec::new();
    while px < w {
        pillars.push(px);
        px += bay;
    }
    for &px in &pillars {
        for y in top..g {
            cv.put(px, y, '█', rgb(lerp(stone, (140, 130, 140), 0.3)), Some(rgb(stone)));
        }
        cv.glyph(px, top, '╦', rgb(lerp(stone, (180, 170, 180), 0.3)));
    }
    for w2 in pillars.windows(2) {
        let (a, b) = (w2[0], w2[1]);
        let mid = (a + b) as f32 / 2.0;
        let half = (b - a) as f32 / 2.0;
        for x in a + 1..b {
            let u = (x as f32 - mid) / half;
            let y = top + ((1.0 - (1.0 - u * u).max(0.0).sqrt()) * ((g - top) as f32 * 0.45).min(5.0)) as i32;
            for yy in top..=y.min(g - 1) {
                cv.fill(x, yy, rgb(lerp(stone, (0, 0, 0), 0.2)));
            }
            cv.glyph(x, y.min(g - 1), '▀', rgb(lerp(stone, (0, 0, 0), 0.2)));
        }
    }

    // The coffins, one to a bay, and its candles.
    let theirs = (cv.roll(6) % pillars.len().max(1) as u32) as usize;
    let mut home = (w / 3, g - 1);
    for (k, w2) in pillars.windows(2).enumerate() {
        let (a, b) = (w2[0], w2[1]);
        let cw = ((b - a) / 2).clamp(3, 7);
        let cx = (a + b) / 2 - cw / 2;
        for x in cx..cx + cw {
            cv.put(x, g - 1, '█', rgb(lerp(stone, (120, 110, 120), 0.4)), Some(rgb(stone)));
        }
        let lid = lerp(stone, (170, 160, 170), 0.4);
        let shift = if k == theirs { (lid_open * (cw as f32 * 0.6)) as i32 } else { 0 };
        for x in cx..cx + cw {
            cv.glyph(x + shift, g - 2, '▄', rgb(lid));
        }
        if k == theirs {
            home = (cx + cw / 2, g - 1);
            if lid_open > 0.0 {
                cv.glyph(cx, g - 2, '▄', rgb((20, 10, 12)));
            }
        }
        // Candles, shrinking back as it passes.
        for (j, x) in [cx - 1, cx + cw].into_iter().enumerate() {
            let near = if awake { 1.0 - (((x - home.0) as f32).abs() / 20.0).min(1.0) } else { 0.0 };
            let f = flicker(t, k as i32 * 2 + j as i32) * (1.0 - near * 0.8);
            cv.halo(x, g - 2, 3.0 * f, flame, 0.45 * f);
            cv.glyph(x, g - 2, '¡', rgb(lerp(lerp(flame, (0, 0, 0), 0.6), flame, f)));
        }
    }

    // It: sitting up, walking out among the bays, back to bed.
    if awake || (at >= lid_at + 30 && at < down_at + 30) {
        let pale = (220, 214, 230);
        let eyes = (255, 30, 30);
        let path = (w as f32 * 0.25).max(6.0);
        let (vx, vy) = if at < walk_at {
            (home.0, g - 2 - (span(at, up_at, walk_at) * 1.0) as i32)
        } else if at < back_at {
            let s = span(at, walk_at, back_at);
            let d = (s * std::f32::consts::PI).sin() * path;
            (home.0 + d as i32, g - 1)
        } else {
            (home.0, g - 2)
        };
        if at >= up_at {
            cv.halo(vx, vy, 3.0, (120, 0, 20), 0.35);
            cv.glyph(vx, vy, 'V', rgb(pale));
            cv.glyph(vx, vy - 1, '"', rgb(eyes));
            if at >= walk_at && at < back_at {
                cv.glyph(vx - 1, vy, '/', rgb((90, 20, 30)));
                cv.glyph(vx + 1, vy, '\\', rgb((90, 20, 30)));
            }
        } else {
            cv.glyph(home.0, g - 2, '"', rgb(eyes));
        }
        // Bats wheeling about it.
        let n = 5 + (cv.roll(7) % 5) as i32;
        for k in 0..n {
            let a = tf * 0.07 + k as f32 * 1.3;
            let r = 3.0 + (k % 3) as f32 * 1.5;
            let (bx, by) = (vx + (a.cos() * r * 2.0) as i32, vy - 2 + (a.sin() * r * 0.6) as i32);
            if by > top && by < g {
                cv.glyph(bx, by, if (t / 4 + k as u64) % 2 == 0 { 'v' } else { '^' }, rgb((90, 70, 90)));
            }
        }
    }

    // Below: the ossuary, skulls in their niches, mist creeping.
    let lb = below(cv);
    if h - lb >= 4 {
        let oy = lb + (h - lb) / 4;
        let oh = ((h - lb) / 2).clamp(3, 10);
        hollow(cv, 1, oy, w - 1, oy + oh, air);
        for y in oy..oy + oh - 1 {
            for x in 1..w - 1 {
                if (x + y * 2) % 4 == 0 && hash(x + salt, y) % 5 != 0 {
                    cv.glyph(x, y, if (x / 4 + y) % 2 == 0 { '☻' } else { '═' }, rgb(lerp((210, 200, 170), air, 0.35)));
                }
            }
        }
        floor(cv, 1, w - 1, oy + oh, lerp(stone, (120, 110, 120), 0.3));
        for x in 1..w - 1 {
            let m = ((x as f32 * 0.15 - tf * 0.04).sin() * 0.5 + 0.5) * ((x as f32 * 0.05 + tf * 0.02).sin() * 0.5 + 0.5);
            cv.glow(x, oy + oh - 1, (150, 150, 170), m * 0.5);
            if m > 0.55 {
                cv.glyph(x, oy + oh - 1, '░', rgb((160, 160, 180)));
            }
        }
    }
}
