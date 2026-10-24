//! A forest at midsummer night: a clearing under the moon ringed by tall
//! dark trees, a still pond holding the moon's reflection, and hundreds of
//! fireflies drifting and blinking over the grass, while an owl watches
//! from a branch and a dwarf with a lantern walks the path home. The seed
//! picks the moon, the trees and the fireflies' colour.

use super::a_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the trees are older than the fortress, and know it",
    "a thousand little lanterns, and none of them ours",
    "walk softly; the forest is listening",
    "midsummer, and the woods are full of stars",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let g = (cv.sky - 1).max(1);
    let tf = t as f32;
    let salt = cv.salt();
    let fly = [(220, 255, 110), (255, 230, 120), (150, 255, 190)][(cv.roll(1) % 3) as usize];
    let full = cv.roll(2) % 2 == 0;

    cv.sky_fill(g, (4, 8, 22), (18, 30, 50));
    cv.stars(g - 1, 29);
    let (mx, my) = ((w as f32 * 0.68) as i32, (g as f32 * 0.25) as i32 + 1);
    let mr = (g as f32 * 0.08).clamp(0.7, 2.6);
    cv.halo(mx, my, mr * 4.0, (180, 200, 230), 0.3);
    cv.disc(mx, my, mr, (236, 236, 214));
    if !full {
        cv.disc(mx + (mr * 1.2) as i32, my - 1, mr, lerp((4, 8, 22), (18, 30, 50), my as f32 / g.max(1) as f32));
    }

    // The far treeline, then tall near trees standing on either side.
    for x in 0..w {
        let s = g - 1 - (ridge(x, w, seed, 90) * (g as f32 * 0.3)) as i32 - (hash(x, salt) % 2) as i32;
        for y in s.max(0)..g {
            cv.put(x, y, if y == s { '♠' } else { ' ' }, rgb((16, 30, 30)), Some(rgb((10, 20, 22))));
        }
    }
    // The clearing: grass, the pond with the moon in it, a path.
    let clearing = (w as f32 * 0.25) as i32;
    let pond_y = g + (h - g) / 3;
    let (px, prx) = (mx, (w / 7).max(4));
    for y in g.max(0)..h {
        for x in 0..w {
            let (dx, dy) = ((x - px) as f32 / prx as f32, (y - pond_y) as f32 / ((h - g) as f32 / 6.0).max(1.0));
            if dx * dx + dy * dy <= 1.0 {
                let wob = ((y as f32 * 2.0 + tf * 0.05).sin() * 1.0) as i32;
                let moon = (x + wob - mx).abs() <= (mr * 2.0) as i32 && (y - pond_y).abs() <= 1;
                let ch = if moon { '▬' } else if (x + y + (t / 10) as i32) % 9 == 0 { '─' } else { ' ' };
                cv.put(x, y, ch, rgb(if moon { (220, 220, 190) } else { (50, 70, 100) }), Some(rgb((10, 18, 34))));
            } else {
                let hs = hash(x + salt, y * 3);
                let ch = if hs % 4 == 0 { '"' } else if hs % 9 == 0 { ',' } else { ' ' };
                cv.put(x, y, ch, rgb((30, 60, 40)), Some(rgb(lerp((14, 30, 22), (8, 18, 14), (y - g) as f32 / (h - g).max(1) as f32))));
            }
        }
    }

    let trees = (w / 7).max(3);
    for k in 0..trees {
        let side = k % 2;
        let off = (hash(k + salt, 11) % clearing.max(1) as u32) as i32;
        let x = if side == 0 { off } else { w - 1 - off };
        let top = (hash(k + salt, 12) % (g / 2).max(1) as u32) as i32;
        let girth = 1 + (hash(k, 13) % 2) as i32;
        let bark = lerp((30, 26, 24), (14, 14, 16), off as f32 / clearing.max(1) as f32);
        for y in top..h {
            for i in 0..girth {
                let ch = if hash(x + i, y + salt) % 5 == 0 { '│' } else { ' ' };
                cv.put(x + i, y, ch, rgb((40, 36, 32)), Some(rgb(bark)));
            }
        }
        // A crown of leaves, moonlit on the top edge.
        let cr = 3 + (hash(k, 14) % 4) as i32;
        for dy in -cr / 2..=cr / 2 {
            for dx in -cr..=cr {
                let (fx, fy) = (dx as f32 / cr as f32, dy as f32 / (cr as f32 * 0.5).max(1.0));
                if fx * fx + fy * fy <= 1.0 + (hash(x + dx, top + dy) % 3) as f32 * 0.1 {
                    let lit = dy == -cr / 2 || fy < -0.6;
                    cv.put(x + dx, top + dy, if hash(x + dx, top + dy + 7) % 3 == 0 { '♣' } else { ' ' }, rgb(if lit { (60, 90, 80) } else { (24, 44, 34) }), Some(rgb(if lit { (26, 46, 44) } else { (12, 26, 22) })));
                }
            }
        }
    }

    // The owl on a branch, blinking.
    let ox = clearing / 2 + 1;
    let oy = (g / 2).max(1);
    cv.glyph(ox - 1, oy + 1, '─', rgb((50, 40, 34)));
    cv.glyph(ox, oy + 1, '─', rgb((50, 40, 34)));
    cv.glyph(ox, oy, if (t / 4) % 37 < 2 { '-' } else { 'Ö' }, rgb((200, 170, 90)));

    // A dwarf with a lantern on the path along the treeline.
    let walk = (w + 20) as u64;
    let lx = ((t / 7 + cv.roll(3) as u64) % walk) as i32 - 10;
    let ly = g - 1;
    dwarf(cv, lx, ly, (200, 170, 140));
    cv.glyph(lx + 1, ly, '¡', rgb((255, 210, 110)));
    cv.halo(lx + 1, ly, 2.5, (255, 190, 90), 0.5);

    // Fireflies: each wanders its own loop and glows on and off.
    let n = (w * h / 30).max(12);
    for k in 0..n {
        let hs = hash(k + salt, 99);
        let bx = (hs % w.max(1) as u32) as f32;
        let by = g as f32 * 0.4 + ((hs >> 8) % ((h as f32 - g as f32 * 0.4).max(1.0) as u32)) as f32;
        let sp = 0.02 + (hs % 7) as f32 * 0.004;
        let x = (bx + (tf * sp + k as f32).sin() * 4.0 + (tf * sp * 0.37).cos() * 3.0) as i32;
        let y = (by + (tf * sp * 0.8 + k as f32 * 2.0).sin() * 1.5) as i32;
        let phase = (t + (hs >> 4) as u64) % (60 + (hs % 40) as u64);
        if phase < 18 {
            let f = 1.0 - (phase as f32 - 9.0).abs() / 9.0;
            cv.glyph(x, y, if f > 0.6 { '•' } else { '·' }, rgb(lerp((60, 80, 40), fly, f)));
            cv.glow(x, y, fly, f * 0.35);
        }
    }
}
