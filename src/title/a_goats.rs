//! Goats on the high pasture: steep green slopes under a big sky, rocks
//! and alpine flowers, the herd cropping its way along, kids skipping,
//! two billies squaring up, one bold goat hopping from crag to crag, and
//! the goatherd with her crook and her dog keeping them together while
//! the bells clank. The seed picks the slopes, the herd and the weather.

use super::a_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "milk, cheese, wool, and opinions",
    "a goat will climb anything a dwarf can dig",
    "bells on the mountain: all's well",
    "the herd goes where it likes; the herder follows",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let g = (cv.sky - 1).max(1);
    let tf = t as f32;
    let salt = cv.salt();
    let (top, bot) = [((60, 120, 210), (190, 220, 245)), ((90, 140, 200), (230, 225, 210))][(cv.roll(1) % 2) as usize];
    let flowers = [(250, 220, 80), (240, 240, 250), (200, 120, 230), (90, 140, 250), (250, 120, 120)];

    cv.sky_fill(g, top, bot);
    clouds(cv, (w / 25).max(2), (1, (g / 3).max(2)), 7, (236, 240, 248), (255, 255, 255), salt);
    // Distant snowy range.
    for x in 0..w {
        let m = peaks(x, w, seed, 140, 5);
        let s = g - 1 - (m * g as f32 * 0.7) as i32;
        for y in s.max(0)..g {
            let snow = y - s < 1 + (m * 2.0) as i32;
            cv.put(x, y, if y == s { '▲' } else { ' ' }, rgb((250, 250, 255)), Some(rgb(if snow { lerp((236, 240, 248), bot, 0.3) } else { lerp((110, 130, 160), bot, 0.4) })));
        }
    }

    // The pasture: two slopes rising to either side of the screen.
    let pasture = |x: i32| -> i32 {
        let u = (x as f32 / w.max(1) as f32 - 0.5).abs() * 2.0;
        g - (u.powf(1.6) * g as f32 * 0.55) as i32 - (ridge(x, w, seed, 150) * 2.5) as i32
    };
    let surf: Vec<i32> = (0..w).map(pasture).collect();
    for x in 0..w {
        let s = surf[x as usize];
        for y in s.max(0)..h {
            let hs = hash(x + salt, y * 7);
            let d = (y - s) as f32;
            let bg = lerp((110, 170, 70), (70, 130, 50), ((d / 8.0).min(1.0) + (hs % 10) as f32 / 30.0).min(1.0));
            let (ch, fg) = match hs % 13 {
                0 => ('*', flowers[(hs / 13 % 5) as usize]),
                1 | 2 => ('"', (150, 200, 90)),
                3 if d > 2.0 => ('▓', (130, 130, 120)),
                _ => (' ', (0, 0, 0)),
            };
            let bg = if ch == '▓' { (150, 150, 140) } else { bg };
            cv.put(x, y, if y == s { '▄' } else { ch }, rgb(if y == s { (140, 200, 90) } else { fg }), Some(rgb(bg)));
        }
    }
    // Crags for the bold goat to hop between.
    let crags: Vec<i32> = (0..4).map(|k| (w as f32 * (0.08 + k as f32 * 0.06)) as i32 + (hash(k + salt, 5) % 3) as i32).collect();
    for &cx in &crags {
        let s = surf[cx.clamp(0, w - 1) as usize];
        cv.sprite(cx - 1, s - 2, &[" ▄ ", "▐█▌"], rgb((140, 136, 130)));
    }

    // The herd, grazing along the slopes.
    let herd = (w / 9).clamp(5, 24) + (cv.roll(2) % 5) as i32;
    let coats = [(240, 240, 235), (170, 120, 80), (60, 50, 46), (220, 200, 170)];
    for k in 0..herd {
        let hs = hash(k + salt, 21);
        let home = (hs % w.max(1) as u32) as i32;
        let x = home + pingpong(t + hs as u64 % 500, 30 + (hs % 20) as u64, 0, 6) - 3;
        if x < 0 || x >= w {
            continue;
        }
        let y = surf[x as usize] - 1;
        let kid = hs % 5 == 0;
        let grazing = (t / 40 + hs as u64) % 3 != 0;
        let hop = kid && (t / 5 + hs as u64) % 9 == 0;
        cv.glyph(x, y - hop as i32, 'g', rgb(coats[(hs / 7 % 4) as usize]));
        if grazing && !kid {
            cv.glyph(x + 1, y, ',', rgb((80, 140, 50)));
        }
        // Bells.
        if !kid && (t / 9 + hs as u64) % 29 == 0 {
            cv.glyph(x, y - 1, '♪', rgb((250, 230, 140)));
        }
    }
    // Two billies butting heads, near the middle where the menu leaves room.
    let bx = (w as f32 * 0.82) as i32;
    if bx + 3 < w {
        let y = surf[bx as usize] - 1;
        let gap = pingpong(t, 6, 0, 3);
        cv.glyph(bx - gap, y, 'G', rgb((200, 190, 170)));
        cv.glyph(bx + 1 + gap, y, 'G', rgb((90, 70, 60)));
        if gap == 0 {
            cv.glyph(bx, y - 1, '*', rgb((255, 250, 200)));
        }
    }
    // The bold one, leaping crag to crag.
    let leg = (t / 30) as usize % (crags.len() * 2 - 2);
    let (from, to) = if leg < crags.len() - 1 { (leg, leg + 1) } else { let b = crags.len() * 2 - 2 - leg; (b, b - 1) };
    let f = (t % 30) as f32 / 30.0;
    let (ax, bx2) = (crags[from], crags[to]);
    let (ay, by) = (surf[ax.clamp(0, w - 1) as usize] - 3, surf[bx2.clamp(0, w - 1) as usize] - 3);
    let jump = (f.clamp(0.4, 0.8) - 0.4) / 0.4;
    let x = ax + ((bx2 - ax) as f32 * jump).round() as i32;
    let y = ay + ((by - ay) as f32 * jump).round() as i32 - ((jump * std::f32::consts::PI).sin() * 2.0).round() as i32;
    cv.glyph(x, y, 'g', rgb((250, 250, 245)));

    // The goatherd and her dog.
    let hx = (w as f32 * 0.7) as i32 + pingpong(t, 18, 0, 8);
    if hx < w {
        let y = surf[hx as usize] - 1;
        dwarf(cv, hx, y, (200, 90, 80));
        cv.glyph(hx + 1, y - 1, '¶', rgb((150, 110, 60)));
        let dx = hx - 3 - pingpong(t, 4, 0, 5);
        if dx >= 0 && dx < w {
            cv.glyph(dx, surf[dx as usize] - 1, 'd', rgb((120, 80, 50)));
        }
    }

    // An eagle turning high over it all.
    let a = tf * 0.02;
    let ex = (w as f32 * 0.5 + a.cos() * w as f32 * 0.3) as i32;
    let ey = (g as f32 * 0.25 + a.sin() * 2.0) as i32 + 1;
    cv.glyph(ex, ey, if (t / 8) % 3 == 0 { 'v' } else { '~' }, rgb((60, 44, 40)));
}
