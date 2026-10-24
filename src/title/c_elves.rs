//! Night in an elven tree-city: trunks wider than towers rising into the
//! dark canopy, round houses ringing them with every window lit, rope
//! bridges slung between, and lanterns by the hundred swaying on their
//! cords. Elves cross the bridges, fireflies drift, and the moon looks in
//! through a gap in the leaves. The seed picks the lanterns' colours, the
//! trees and where the bridges hang.

use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the elves do not cut the trees; they ask them",
    "a city with no stone in it at all",
    "lanterns in the leaves, songs on the bridges",
    "walk softly; the forest is listening",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t) = (cv.w(), cv.h(), cv.t);
    let tf = t as f32;
    let sky = cv.sky.min(h - 1).max(1);
    let salt = cv.salt();
    let palettes = [
        [(255, 210, 110), (255, 170, 80), (255, 236, 160)],
        [(120, 240, 220), (170, 255, 200), (230, 255, 250)],
        [(255, 140, 180), (250, 200, 120), (200, 160, 255)],
        [(180, 230, 120), (255, 230, 120), (150, 220, 255)],
    ];
    let lamps = palettes[(cv.roll(1) % 4) as usize];

    // Deep blue night through the woods, the moon in a gap, faint trunks
    // far off.
    let (top, bot) = ((8, 14, 30), (16, 34, 40));
    for y in 0..h {
        let c = lerp(top, bot, y as f32 / h.max(1) as f32);
        for x in 0..w {
            cv.fill(x, y, rgb(c));
        }
    }
    let (mx, my) = ((w as f32 * (0.3 + (cv.roll(2) % 40) as f32 / 100.0)) as i32, (sky as f32 * 0.42) as i32);
    cv.halo(mx, my, 6.0, (200, 220, 230), 0.25);
    cv.disc(mx, my, (sky as f32 * 0.09).max(0.8).min(2.2), (226, 234, 236));
    for k in 0..(w / 6).max(2) {
        let x = (hash(k, salt) % w.max(1) as u32) as i32;
        let wd = 1 + (hash(k, 2) % 2) as i32;
        for y in 0..h {
            for dx in 0..wd {
                cv.glow(x + dx, y, (40, 60, 64), 0.5);
            }
        }
    }

    // The canopy overhead.
    for x in 0..w {
        let depth = 2 + ((x as f32 * 0.15 + salt as f32).sin() * 1.5 + (x as f32 * 0.04).sin() * 1.5) as i32 + (hash(x, salt) % 2) as i32;
        for y in 0..depth.max(1) {
            let hs = hash(x * 3 + salt, y);
            let ch = if hs % 3 == 0 { '♣' } else { ' ' };
            cv.put(x, y, ch, rgb((40, 80, 50)), Some(rgb((12, 30, 20))));
        }
    }

    // The great trees.
    let n = (3 + w / 45).max(2);
    let mut trunks: Vec<(i32, i32)> = Vec::new(); // (centre, half-width)
    for k in 0..n {
        let slot = w as f32 / n as f32;
        let x = (slot * (k as f32 + 0.5) + ((hash(k, salt + 1) % 100) as f32 / 100.0 - 0.5) * slot * 0.4) as i32;
        let hw = if w >= 110 { 2 + (hash(k, 3) % 2) as i32 } else { 1 };
        trunks.push((x, hw));
    }
    let bark = (58, 44, 34);
    for &(x, hw) in &trunks {
        for y in 0..h {
            for dx in -hw..=hw {
                let hs = hash(x + dx + salt, y);
                let ch = if hs % 4 == 0 { '║' } else if hs % 7 == 0 { '│' } else { ' ' };
                let shade = lerp(bark, (0, 0, 0), if dx == hw { 0.4 } else if dx == -hw { 0.1 } else { 0.25 });
                cv.put(x + dx, y, ch, rgb(lerp(shade, (0, 0, 0), 0.4)), Some(rgb(shade)));
            }
        }
    }

    // Bridges slung between neighbouring trees, lanterns hanging from them.
    let mut lanterns: Vec<(i32, i32)> = Vec::new();
    let mut deck: Vec<(i32, i32)> = Vec::new();
    for (k, pair) in trunks.windows(2).enumerate() {
        let ((ax, aw), (bx, bw)) = (pair[0], pair[1]);
        let span = (sky - 6).max(2);
        let y0 = 5 + (hash(k as i32, salt + 7) % span as u32) as i32;
        let y0 = y0.min(sky - 2).max(1);
        let (x0, x1) = (ax + aw + 1, bx - bw - 1);
        let len = (x1 - x0).max(1);
        let sag = (len as f32 / 10.0).min(3.0);
        for x in x0..=x1 {
            let u = (x - x0) as f32 / len as f32;
            let y = y0 + (sag * (u * std::f32::consts::PI).sin()).round() as i32;
            cv.glyph(x, y, '─', rgb((150, 120, 80)));
            deck.push((x, y));
            if (x - x0) % 6 == 3 {
                lanterns.push((x, y + 1));
            }
        }
        // A platform ringing each tree at the bridge's height, with a house.
        for (tx, tw) in [(ax, aw), (bx, bw)] {
            for x in tx - tw - 2..=tx + tw + 2 {
                cv.glyph(x, y0, '▀', rgb((120, 90, 60)));
            }
            let roof = y0 - 3;
            for x in tx - tw - 1..=tx + tw + 1 {
                cv.put(x, y0 - 1, ' ', Color::Black, Some(rgb((90, 70, 52))));
                cv.put(x, y0 - 2, ' ', Color::Black, Some(rgb((90, 70, 52))));
                cv.glyph(x, roof, '▄', rgb((60, 110, 70)));
            }
            let flick = (t / 9 + tx as u64) % 13 != 0;
            cv.put(tx, y0 - 2, '▪', rgb(if flick { lamps[2] } else { lamps[1] }), Some(rgb((90, 70, 52))));
            cv.halo(tx, y0 - 2, 2.0, lamps[0], 0.25);
        }
    }

    // The lanterns, swaying.
    for (i, &(x, y)) in lanterns.iter().enumerate() {
        let sway = ((tf * 0.06 + i as f32 * 0.9).sin() * 1.2).round() as i32;
        let col = lamps[i % 3];
        cv.glyph(x + sway / 2, y, '│', rgb((110, 90, 60)));
        cv.halo(x + sway, y + 1, 2.2, col, 0.35);
        cv.glyph(x + sway, y + 1, '♦', rgb(col));
    }

    // Elves crossing the bridges.
    if !deck.is_empty() {
        for k in 0..(1 + deck.len() / 40) {
            let i = ((t / 7 + hash(k as i32, 9) as u64) % deck.len() as u64) as usize;
            let (x, y) = deck[i];
            cv.glyph(x, y - 1, '☺', rgb([(220, 240, 220), (240, 220, 255), (220, 230, 255)][k % 3]));
        }
    }

    // Fireflies, and glowing toadstools on the forest floor.
    for k in 0..(6 + w / 12) {
        if (t / 8 + hash(k, 2) as u64) % 6 == 0 {
            continue;
        }
        let x = ((hash(k, 5) as f32 + tf * 0.07 + (tf * 0.03 + k as f32).sin() * 4.0) as i32).rem_euclid(w.max(1));
        let y = ((hash(k, 6) % h.max(1) as u32) as i32 + ((tf * 0.05 + k as f32 * 1.7).sin() * 2.0) as i32).rem_euclid(h.max(1));
        cv.glyph(x, y, '·', rgb((210, 255, 140)));
    }
    for x in 0..w {
        if hash(x, salt + 11) % 5 == 0 {
            let glow = 0.5 + 0.5 * (tf * 0.05 + x as f32).sin();
            cv.glyph(x, h - 1, '♠', mix((60, 120, 120), (160, 255, 230), glow));
        }
    }
}
