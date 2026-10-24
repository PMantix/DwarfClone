//! Ambush in the tunnels. A lone hauler pushes a barrow along a dark
//! passage, the only light the torch on its handle. Eyes open in the side
//! cracks, red, in pairs, keeping pace. Then the goblins come out of the
//! dark all at once, steel flashing in the torchlight, and it's a fight
//! until the lanterns of the militia come round the bend behind and the
//! goblins melt back into the black. Below, their campfire in the deep.
//! The seed picks the stone, the gang and how it ends.

use super::b_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "never haul alone past the old diggings",
    "eyes in the dark, keeping pace",
    "the torch is a weapon if you swing it",
    "the deep tunnels belong to someone else",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t) = (cv.w(), cv.h(), cv.t);
    let g = (cv.sky - 1).max(3).min(h - 1);
    let th = ((g as f32 * 0.4) as i32).clamp(2, 6);
    let ty = (g - th).max(0);
    let salt = cv.salt();
    let stone = [(62, 56, 52), (54, 56, 62), (64, 58, 48)][(cv.roll(1) % 3) as usize];
    let gob = (110, 190, 80);
    let eyes = (255, 50, 40);

    // The beat: hauling; eyes; the rush; the fight; the rescue.
    let at = beat(cv, 1300, 3) as i32;
    let (eyes_at, rush, rescue, gone) = (300, 520, 800, 950);
    let walk_end = (w * 3 / 5).max(10);
    let hx = if at < rush { 4 + at * (walk_end - 4) / rush } else { walk_end }; // the hauler
    let tx = hx + 2; // the torch on the barrow
    let help = if at >= rescue - 80 { -4 + (at - (rescue - 80)) / 2 } else { -99 }; // the militia
    let help = help.min(hx - 6);

    // Rock all round, nearly black; the passage, lit only near the torch.
    let lights: Vec<(i32, i32, f32)> = {
        let mut v = vec![(tx, g - 2, 11.0 * flicker(t, 1).max(0.7))];
        if help > -10 {
            v.push((help, g - 2, 9.0));
        }
        v
    };
    let lit = |x: i32, y: i32| -> f32 {
        lights.iter().map(|&(lx, ly, r)| {
            let (dx, dy) = ((x - lx) as f32 * 0.5, (y - ly) as f32);
            (1.0 - (dx * dx + dy * dy).sqrt() / r).max(0.0)
        }).fold(0.0, f32::max)
    };
    let dark = lerp(stone, (0, 0, 0), 0.9);
    for y in 0..h {
        for x in 0..w {
            let l = if y <= g + 3 { lit(x, y) } else { 0.0 };
            let open = y >= ty && y < g;
            let base = if open { lerp(stone, (0, 0, 0), 0.7) } else { stone };
            let c = lerp(dark, lerp(base, (255, 160, 70), 0.15), l);
            let hs = hash(x + salt, y * 3);
            let ch = if !open && hs % 9 == 0 { '░' } else if y == g { '▀' } else { ' ' };
            cv.put(x, y, ch, rgb(lerp(c, (0, 0, 0), 0.3)), Some(rgb(c)));
        }
    }
    // Cracks in the walls where things wait.
    let ncr = (w / 14).max(2);
    let cracks: Vec<i32> = (0..ncr).map(|k| hx.max(walk_end) - 30 + (k * 9) as i32 + (hash(k, salt) % 5) as i32).collect();

    // The eyes, keeping pace; then the rush.
    let ng = 3 + (cv.roll(4) % 3) as i32;
    for k in 0..ng {
        let side = (hash(k, salt + 1) % 2) as i32; // above or level
        let ex = walk_end + 6 + k * 4 - (k % 2) * 13;
        let ey = if side == 0 { ty - 1 } else { g - 1 };
        if at >= eyes_at && at < rush {
            let blink = (t / 7 + k as u64 * 5) % 23 == 0;
            if !blink && (at - eyes_at) > k * 25 {
                cv.glyph(ex, ey, '•', rgb(eyes));
                cv.glyph(ex + 1, ey, '•', rgb(eyes));
            }
        } else if at >= rush && at < gone {
            // Out of the dark and on to the hauler, or fleeing.
            let s = at - rush;
            let (x, y) = if at < rescue {
                let target = hx + 3 + k * 2 - (k % 2) * 7;
                ((ex + (target - ex) * s.min(20) / 20), g - 1)
            } else {
                (hx + 3 + k * 2 - (k % 2) * 7 + (at - rescue) / 2 * if k % 2 == 0 { 1 } else { 2 }, g - 1)
            };
            if x < w {
                cv.glyph(x, y, 'g', rgb(lerp(gob, (0, 0, 0), 1.0 - lit(x, y).max(0.35))));
                if at < rescue && (t / 5 + k as u64) % 2 == 0 {
                    let side = if x > hx { -1 } else { 1 };
                    cv.glyph(x + side, y - 1, if (t / 3) % 2 == 0 { '/' } else { '─' }, rgb((220, 220, 230)));
                }
            }
        }
    }
    let _ = cracks;
    // Clashes: sparks where steel meets.
    if at >= rush + 20 && at < rescue {
        for k in 0..3 {
            if (t / 3 + k) % 4 == 0 {
                let x = hx + 1 + (hash(k as i32, (t / 6) as i32) % 5) as i32 - 2;
                cv.glyph(x, g - 2 - (k % 2) as i32, '*', rgb((255, 240, 160)));
            }
        }
    }

    // The hauler, the barrow, the torch.
    let b = beard(cv.roll(5));
    let fighting = at >= rush && at < rescue;
    cv.glyph(hx, g - 1, '☺', rgb(b));
    if fighting {
        cv.glyph(hx, g - 2, '‼', rgb((255, 240, 160)));
        cv.glyph(hx + 1, g - 2, if (t / 4) % 2 == 0 { '¡' } else { '/' }, rgb((255, 200, 100)));
        cv.sprite(hx - 3, g - 1, &["▐▌"], rgb((120, 86, 50)));
    } else {
        cv.sprite(hx + 1, g - 1, &["▐█▌"], rgb((120, 86, 50)));
        cv.glyph(hx + 2, g - 2, '¡', rgb(lerp((255, 160, 60), (255, 240, 160), flicker(t, 2))));
        if at >= eyes_at && at < rush && (t / 30) % 3 == 0 {
            cv.glyph(hx, g - 2, '?', rgb((240, 220, 160)));
        }
    }
    // The militia with their lanterns.
    if help > -10 {
        for k in 0..3 {
            let x = help - k * 3;
            cv.glyph(x, g - 1, '☺', rgb((150, 180, 220)));
            cv.glyph(x + 1, g - 1, if (t / 5 + k as u64) % 2 == 0 { '/' } else { '│' }, rgb((200, 200, 210)));
        }
        cv.glyph(help, g - 2, '¡', rgb((255, 220, 140)));
    }

    // Below: their fire, deep in the old diggings, and eyes round it.
    let lb = below(cv);
    if h - lb >= 4 {
        let fy = (lb + (h - lb) * 2 / 3).min(h - 2);
        let fx = (w as f32 * (0.7 + (cv.roll(6) % 15) as f32 / 100.0)) as i32;
        let f = flicker(t, 5);
        cv.halo(fx, fy, 5.0 * f, (255, 110, 40), 0.55 * f);
        cv.glyph(fx, fy, if (t / 4) % 2 == 0 { '▲' } else { '♠' }, rgb(lerp((255, 90, 30), (255, 220, 120), f)));
        for k in 0..4 {
            let a = k as f32 * 1.6 + 0.5;
            let (x, y) = (fx + (a.cos() * 6.0) as i32, fy + (a.sin() * 1.5) as i32);
            if (t / 9 + k as u64 * 7) % 29 != 0 {
                cv.glyph(x, y, '•', rgb(eyes));
                cv.glyph(x + 1, y, '•', rgb(eyes));
            }
        }
        for k in 0..5 {
            let age = ((t / 4) as i32 + k * 4) % 16;
            cv.glyph(fx + ((age as f32 * 0.5 + k as f32).sin() * 1.5) as i32, fy - 1 - age / 2, '·', rgb(lerp((255, 180, 80), (40, 30, 30), age as f32 / 16.0)));
        }
    }
}
