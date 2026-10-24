//! A cavern strung with webs: great wheels of silk from wall to roof,
//! glinting as something walks them, a wrapped bundle turning slowly on
//! its line, and the giant cave spider itself letting down on a thread
//! towards a dwarf who has come too far with a torch. Below, the egg sacs.
//! The seed places the webs and picks the spider's colours.

use super::b_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the silk is worth a fortune. the spider disagrees",
    "something has been wrapped up for later",
    "mind the threads. they're mind you",
    "eight legs, eight eyes, no patience",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t) = (cv.w(), cv.h(), cv.t);
    let g = (cv.sky - 1).max(3).min(h - 1);
    let tf = t as f32;
    let salt = cv.salt();
    let air = [(12, 12, 18), (16, 12, 14), (10, 14, 14)][(cv.roll(1) % 3) as usize];
    let silk = [(200, 205, 220), (220, 210, 180), (190, 220, 210)][(cv.roll(2) % 3) as usize];
    let (body, mark) = [((40, 34, 40), (200, 40, 40)), ((70, 60, 50), (240, 200, 80)), ((30, 40, 60), (120, 220, 255))][(cv.roll(3) % 3) as usize];

    // The cavern.
    bedrock(cv, 0, h, (54, 50, 56), salt);
    let roof = |x: i32| 1 + (ridge(x, w, cv.seed, 30) * 2.0) as i32;
    let roofs: Vec<i32> = (0..w).map(roof).collect();
    for x in 0..w {
        for y in roofs[x as usize]..g {
            let f = (y as f32 / g.max(1) as f32) * 0.3;
            cv.fill(x, y, mix(air, (30, 30, 36), f));
        }
        cv.put(x, g, '▀', rgb((70, 64, 70)), Some(rgb((34, 30, 34))));
    }

    // The webs: spokes out from a hub, rings of silk across them.
    let nweb = (w / 50).clamp(1, 3) + 1;
    for k in 0..nweb {
        let cx = ((k as f32 + 0.5) / nweb as f32 * w as f32) as i32 + (roll(cv.seed, 10 + k as u32) % 9) as i32 - 4;
        let cy = (g / 2 + (roll(cv.seed, 20 + k as u32) % 3) as i32 - 1).max(2);
        let r = ((g - 2) as f32 * 0.5).max(2.0);
        let spokes = 6 + (roll(cv.seed, 30 + k as u32) % 4) as i32;
        let dim = lerp(silk, air, 0.6);
        for s in 0..spokes {
            let a = s as f32 / spokes as f32 * std::f32::consts::TAU + k as f32;
            let end = (cx + (a.cos() * r * 2.0) as i32, cy + (a.sin() * r) as i32);
            for (x, y) in line((cx, cy), end) {
                if y >= roofs[x.clamp(0, w - 1) as usize] && y < g {
                    let ch = if (a.sin() * a.cos()).abs() < 0.2 { if a.cos().abs() > 0.7 { '─' } else { '│' } } else if a.sin() * a.cos() > 0.0 { '\\' } else { '/' };
                    cv.glyph(x, y, ch, rgb(dim));
                }
            }
        }
        let rings = (r as i32 / 2).clamp(2, 4);
        for ring in 1..=rings {
            let rr = r * ring as f32 / rings as f32;
            let n = (rr * 8.0) as i32;
            for i in 0..n {
                let a = i as f32 / n as f32 * std::f32::consts::TAU;
                let (x, y) = (cx + (a.cos() * rr * 2.0) as i32, cy + (a.sin() * rr) as i32);
                if y >= roofs[x.clamp(0, w - 1) as usize] && y < g {
                    let glint = (t / 3 + (i * 7 + ring * 13 + k * 5) as u64) % 61 == 0;
                    cv.glyph(x, y, '·', rgb(if glint { (255, 255, 255) } else { silk }));
                }
            }
        }
    }
    // Loose strands hanging from the roof.
    for x in 0..w {
        if hash(x + salt, 77) % 9 == 0 {
            let len = (hash(x, 78) % (g as u32 / 2).max(1)) as i32;
            for y in roofs[x as usize]..roofs[x as usize] + len {
                cv.glyph(x, y, '│', rgb(lerp(silk, air, 0.6)));
            }
        }
    }

    // A wrapped bundle, swinging on its line. Something in it kicks.
    let bxh = (w as f32 * (0.2 + (cv.roll(4) % 10) as f32 / 100.0)) as i32;
    let blen = ((g - 3) as f32 * 0.55).max(2.0);
    let sw = (tf * 0.03).sin() * 0.35;
    let (ex, ey) = (bxh + (sw.sin() * blen * 2.0) as i32, roofs[bxh.clamp(0, w - 1) as usize] + (sw.cos() * blen) as i32);
    for (x, y) in line((bxh, roofs[bxh.clamp(0, w - 1) as usize]), (ex, ey)) {
        cv.glyph(x, y, '·', rgb(silk));
    }
    let kick = (t / 40) % 7 == 0 && (t / 4) % 2 == 0;
    cv.put(ex, ey + 1, '▓', rgb(silk), Some(rgb(lerp(silk, air, 0.5))));
    cv.put(ex, ey + 2, if kick { '▒' } else { '▓' }, rgb(silk), Some(rgb(lerp(silk, air, 0.5))));
    cv.put(ex + if kick { 1 } else { 0 }, ey + 3, '▀', rgb(silk), None);

    // The spider, letting itself down on a thread and climbing back.
    let sx = (w as f32 * (0.62 + (cv.roll(5) % 15) as f32 / 100.0)) as i32;
    let top = roofs[sx.clamp(0, w - 1) as usize] + 1;
    let at = beat(cv, 700, 6);
    let lowest = (g - 3).max(top);
    let drop = if at < 200 { 0.0 } else if at < 320 { span(at, 200, 320) } else if at < 520 { 1.0 } else { 1.0 - span(at, 520, 680) };
    let sy = top + ((lowest - top) as f32 * drop) as i32;
    for y in roofs[sx.clamp(0, w - 1) as usize]..sy {
        cv.glyph(sx, y, '│', rgb(silk));
    }
    let walk = (t / 5) % 2 == 0;
    let legs_up = if walk { "╲╲ ╱╱" } else { "╲ │ ╱" };
    let legs_dn = if walk { "╱╱ ╲╲" } else { "╱ │ ╲" };
    cv.sprite(sx - 2, sy - 1, &[legs_up], rgb(lerp(body, (120, 110, 110), 0.4)));
    cv.sprite(sx - 2, sy + 1, &[legs_dn], rgb(lerp(body, (120, 110, 110), 0.4)));
    cv.glyph(sx - 2, sy, '─', rgb(lerp(body, (120, 110, 110), 0.4)));
    cv.glyph(sx + 2, sy, '─', rgb(lerp(body, (120, 110, 110), 0.4)));
    cv.put(sx - 1, sy, '(', rgb(lerp(body, (150, 140, 140), 0.5)), Some(rgb(body)));
    cv.put(sx, sy, '@', rgb(mark), Some(rgb(body)));
    cv.put(sx + 1, sy, ')', rgb(lerp(body, (150, 140, 140), 0.5)), Some(rgb(body)));
    cv.halo(sx, sy + 1, 1.2, (255, 40, 40), 0.25);

    // The dwarf with the torch, who looks up at the wrong moment.
    let dx = (sx - 6 - (cv.roll(7) % 6) as i32).max(1);
    let fear = drop > 0.6;
    let back = if fear { ((at.saturating_sub(320)) / 3).min(8) as i32 } else { 0 };
    let dxx = dx - back;
    torch(cv, dxx + 1, g - 2, 1, 4.0);
    cv.glyph(dxx, g - 1, '☺', rgb(beard(cv.roll(8))));
    if fear {
        cv.glyph(dxx, g - 2, '!', rgb((255, 240, 160)));
    }

    // Below: the egg sacs, and the hatchlings when they come.
    let lb = below(cv);
    if h - lb >= 4 {
        let n = (w / 14).max(2);
        for k in 0..n {
            let x = 4 + k * (w - 8) / n.max(1) + (hash(k, salt) % 5) as i32;
            let y = lb + 1 + (hash(k, salt + 1) % (h - lb - 2).max(1) as u32) as i32;
            cv.halo(x, y, 1.5, lerp(silk, (0, 0, 0), 0.6), 0.3);
            cv.put(x, y, 'O', rgb(silk), Some(rgb(lerp(silk, (0, 0, 0), 0.6))));
            for (x2, y2) in line((x, y - 1), (x + (k % 3) - 1, (y - 4).max(lb))) {
                cv.glyph(x2, y2, '│', rgb(lerp(silk, (0, 0, 0), 0.4)));
            }
            // Hatchlings skittering off.
            if (t / 120 + k as u64) % 4 == 0 {
                let age = ((t % 120) / 3) as i32;
                cv.glyph(x + age / 2 * if k % 2 == 0 { 1 } else { -1 }, y + (age / 7) % 2, '*', rgb(lerp(body, (200, 200, 200), 0.4)));
            }
        }
    }
}
