//! The dark fortress: black spires on a crag under a bleeding sky, clouds
//! churning red over them, a great brazier burning on the tallest, slit
//! windows glowing, fissures in the plain breathing fire, and a war band
//! filing out of the gate behind its torches to the beat of the drums.
//! The seed picks the sky, the brazier's fire, the spires and the band.

use super::*;
use super::c_kit::*;

pub(super) const TAGLINES: &[&str] = &[
    "the goblins have a word for mercy; it means weak",
    "the drums carry for leagues on a still night",
    "every spire was raised by stolen hands",
    "march on it, and pray they are all asleep",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t) = (cv.w(), cv.h(), cv.t);
    let tf = t as f32;
    let sky = cv.sky.min(h - 1).max(1);
    let g = (sky - 1).max(1); // the plain before the crag
    let salt = cv.salt();
    let (top, bot) = [((40, 6, 10), (200, 50, 30)), ((30, 6, 20), (170, 30, 60)), ((26, 10, 8), (220, 90, 30))][(cv.roll(1) % 3) as usize];
    let fire = [(255, 150, 40), (120, 255, 90), (180, 120, 255)][(cv.roll(2) % 3) as usize];
    let beat = (t % 24) < 3; // the drums
    let pulse = if beat { 0.15 } else { 0.0 };

    // The sky, its clouds churning, a dim sun swollen on the horizon.
    for y in 0..g.min(h) {
        let base = lerp(top, bot, (y as f32 / g.max(1) as f32).powf(1.3));
        for x in 0..w {
            let n = fbm(x as f32 * 0.05 + tf * 0.008, y as f32 * 0.12 - tf * 0.003, salt, 4);
            let c = if n > 0.52 { lerp(base, (16, 6, 8), ((n - 0.52) * 3.0).min(0.85)) } else { lerp(base, (255, 120, 60), (0.52 - n) * 0.4) };
            cv.fill(x, y, rgb(c));
        }
    }
    let (sx, sy) = ((w as f32 * 0.2) as i32, g - 2);
    cv.halo(sx, sy, 6.0, (255, 80, 40), 0.3);
    cv.disc(sx, sy, (g as f32 * 0.14).max(1.0).min(3.0), (200, 40, 30));

    // The crag and its spires.
    let cx = (w as f32 * 0.74) as i32;
    let half = ((w as f32 * 0.17) as i32).max(6);
    let crag = |x: i32| -> i32 {
        let d = ((x - cx) as f32 / half as f32).abs();
        if d >= 1.0 {
            g
        } else {
            g - ((1.0 - d).powf(0.6) * sky as f32 * 0.25) as i32 + (hash(x, 3) % 2) as i32
        }
    };
    let black = (18, 10, 14);
    for x in 0..w {
        let s = crag(x);
        for y in s.max(0)..=g {
            let ch = if hash(x + salt, y) % 6 == 0 { '▒' } else { ' ' };
            cv.put(x, y, ch, rgb((40, 24, 26)), Some(rgb((26, 16, 18))));
        }
    }
    let n = 3 + (cv.roll(3) % 3) as i32;
    let mut tallest = (cx, g);
    for k in 0..n {
        let u = (k as f32 + 0.5) / n as f32;
        let x = cx - half / 2 + (u * half as f32) as i32;
        let base = crag(x);
        let mid = (k - n / 2).abs();
        let tall = ((sky as f32 * (0.62 - mid as f32 * 0.12)) as i32 + (hash(k, salt) % 3) as i32).max(3);
        let wd = 1 + (tall / 6).min(2);
        let tip = base - tall;
        if tip < tallest.1 {
            tallest = (x, tip);
        }
        for y in tip.max(0)..base {
            let f = (y - tip) as f32 / tall as f32;
            let hw = (f * wd as f32).round() as i32;
            for dx in -hw..=hw {
                cv.fill(x + dx, y, rgb(if dx == hw && hw > 0 { (34, 18, 22) } else { black }));
            }
            // Barbs off the sides.
            if (y - tip) % 4 == 3 && hw > 0 {
                cv.glyph(x - hw - 1, y, '◄', rgb(black));
                cv.glyph(x + hw + 1, y, '►', rgb(black));
            }
            if (y - tip) % 3 == 2 && hw > 0 && hash(x, y) % 2 == 0 {
                let on = (t / 7 + hash(y, x) as u64) % 11 != 0;
                cv.put(x, y, '▪', rgb(if on { lerp(fire, (255, 60, 30), 0.5) } else { (80, 20, 20) }), Some(rgb(black)));
            }
        }
        cv.glyph(x, tip - 1, '▲', rgb(black));
    }
    // The brazier on the tallest spire.
    let (bx, by) = (tallest.0, tallest.1 - 2);
    let flick = (tf * 0.6).sin() * 0.5 + 0.5;
    cv.halo(bx, by, 4.0 + flick + pulse * 8.0, fire, 0.5);
    for dx in -1..=1 {
        let hs = hash(dx, (t / 2) as i32);
        cv.glyph(bx + dx, by - (hs % 2) as i32, ['▲', '♠', '*'][(hs % 3) as usize], rgb(lerp(fire, (255, 255, 200), (hs % 3) as f32 * 0.25)));
    }
    for k in 0..6 {
        let age = ((t / 4) as i32 + k * 5) % 30;
        let y = by - 2 - age / 3;
        if y >= 0 {
            cv.glyph(bx + age / 4 + k % 2, y, if age < 12 { '░' } else { '·' }, mix((50, 30, 30), top, age as f32 / 30.0));
        }
    }

    // The plain: scorched earth and fissures breathing fire.
    for y in (g + 1).max(0)..h {
        for x in 0..w {
            let hs = hash(x + salt, y * 7);
            let crack = ((x as f32 * 0.11 + y as f32 * 0.5 + salt as f32).sin() * (y as f32 * 0.3 - x as f32 * 0.05).cos()).abs() < 0.035 && hash(x / 3, y) % 3 != 0;
            if crack {
                let heat = 0.5 + 0.5 * (tf * 0.1 + x as f32 * 0.3).sin();
                cv.put(x, y, if hs % 3 == 0 { '≈' } else { '~' }, mix((255, 200, 80), (255, 80, 20), heat), Some(mix((120, 30, 10), (200, 60, 10), heat)));
            } else {
                let ch = if hs % 7 == 0 { '·' } else if hs % 11 == 0 { ',' } else { ' ' };
                cv.put(x, y, ch, rgb((70, 44, 40)), Some(rgb((38, 22, 22))));
            }
        }
    }

    // The war band filing out of the gate behind their torches.
    let gate = (cx, g);
    cv.fill(gate.0, g, rgb((6, 2, 4)));
    cv.fill(gate.0, g - 1, rgb((6, 2, 4)));
    let size = 10 + (cv.roll(4) % 10) as i32;
    let lead = ((t / 6 + cv.roll(5) as u64) % (w as u64 + size as u64 * 2 + 20)) as i32;
    for k in 0..size {
        let x = gate.0 - lead + k * 2;
        if x > gate.0 {
            continue;
        }
        let y = g;
        let torch = k % 3 == 0;
        cv.glyph(x, y, 'g', rgb((110, 150, 70)));
        if torch {
            cv.halo(x, y - 1, 1.6, (255, 150, 60), 0.4);
            cv.glyph(x, y - 1, if (t / 3 + k as u64) % 2 == 0 { '*' } else { '¡' }, rgb((255, 180, 70)));
        } else {
            cv.glyph(x, y - 1, '↑', rgb((140, 130, 130)));
        }
    }

    // Bats about the spires, and the drums' beat in the fires.
    for k in 0..(3 + w / 40) {
        let a = tf * (0.05 + k as f32 * 0.007) + k as f32 * 1.9;
        let x = cx + (a.cos() * half as f32 * 0.9) as i32;
        let y = (tallest.1 + 3 + (a.sin() * 3.0) as i32 + (k % 3)).max(0);
        cv.glyph(x, y, if (t / 3 + k as u64) % 2 == 0 { 'v' } else { 'w' }, rgb((10, 6, 8)));
    }
    if beat {
        for y in (g + 1).max(0)..h.min(g + 3) {
            for x in 0..w {
                cv.glow(x, y, (255, 90, 30), 0.12);
            }
        }
    }
}
