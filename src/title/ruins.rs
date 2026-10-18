//! A fallen fortress: broken walls and towers under a great moon, graves
//! before them, mist creeping over the ground and the restless dead
//! drifting through it all. Below, the old halls lie open to the sky, and
//! something still glitters in one of them. The seed sets the moon, the
//! ruin's outline, how many ghosts walk and the halls' plan.

use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "losing is fun, and the ghosts agree",
    "here lies a fortress; it was a good one",
    "the walls fell, but the names are carved deep",
    "someone will come back for the hoard",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, sky, seed) = (cv.w(), cv.h(), cv.t, cv.sky, cv.seed);
    let g = sky - 1;
    cv.sky_fill(g, (4, 4, 14), (30, 32, 52));
    cv.stars(g - 2, 59);

    // The moon, big and low, with a phase.
    let r = (sky / 4).clamp(1, 5);
    let right = cv.roll(1) % 2 == 0;
    let mx = if right { w - w / 8 - r } else { w / 8 + r };
    let my = (r + 1).min(g - r - 1).max(r);
    let phase = [0.0, 0.5, -0.5, 1.2][(cv.roll(2) % 4) as usize] * r as f32;
    for y in my - r * 2..=my + r * 2 {
        for x in mx - r * 4..=mx + r * 4 {
            let (dx, dy) = ((x - mx) as f32 / 2.0, (y - my) as f32);
            let d = (dx * dx + dy * dy).sqrt();
            if y < 0 || y >= g {
                continue;
            }
            if d <= r as f32 + 0.3 {
                let shade = (dx - phase) * (dx - phase) + dy * dy < (r * r) as f32 && phase != 0.0;
                let crater = hash(x * 3, y * 5) % 5 == 0;
                let bg = if shade { (40, 42, 60) } else { (232, 228, 196) };
                cv.put(x, y, if crater && !shade { '°' } else { ' ' }, rgb((190, 186, 160)), Some(rgb(bg)));
            } else if d <= r as f32 * 2.0 {
                let a = 1.0 - (d - r as f32) / r as f32;
                let under = cv.bg_at(x, y);
                cv.tint(x, y, rgb(lerp(under, (70, 72, 96), a * 0.6)));
            }
        }
    }
    // Thin cloud passing over it.
    for k in 0..2i32 {
        let len = 8 + (hash(k, 3) % 8) as i32;
        let y = (my + k * 2 - 1).clamp(0, (g - 2).max(0));
        let x0 = ((hash(k, 7) as u64 + t / (20 + k as u64 * 9)) % (w as u64 + len as u64 + 10)) as i32 - len - 5;
        for i in 0..len {
            cv.glyph(x0 + i, y, '▀', rgb((70, 72, 92)));
        }
    }

    // The ruined walls along the skyline.
    let tall = (sky - 3).max(2);
    let mut x = 0;
    let mut seg = 0;
    let stone = |x: i32, y: i32, lit: f32| mix((38, 36, 44), (82, 80, 92), lit * (0.7 + (hash(x, y) % 30) as f32 / 100.0));
    while x < w {
        let len = 3 + (hash(seg, 11 + seed as i32) % 9) as i32;
        let kind = hash(seg, 13 + seed as i32) % 5;
        let height = match kind {
            0 => 0,
            1 | 2 => 1 + (hash(seg, 17) % (tall as u32 / 2).max(1)) as i32,
            _ => tall / 2 + (hash(seg, 19) % (tall as u32 / 2 + 1)) as i32,
        };
        for i in 0..len {
            let cx = x + i;
            let jag = (hash(cx, 23) % 3) as i32;
            let top = g - height + jag;
            let lit = if (cx < mx) == right { 0.4 } else { 0.8 };
            for y in top.max(0)..g {
                let window = kind >= 3 && i % 4 == 2 && y > top + 1 && y < top + 3;
                if window {
                    continue;
                }
                let ch = if y == top { if jag == 2 { '▄' } else { '▓' } } else if hash(cx * 3, y) % 6 == 0 { '░' } else { ' ' };
                cv.put(cx, y, ch, rgb((60, 58, 66)), Some(stone(cx, y, lit)));
            }
        }
        x += len;
        seg += 1;
    }

    // Ground before the walls, graves, a dead tree.
    for y in g.max(0)..h {
        let deep = (y - g) as f32 / (h - g).max(1) as f32;
        for x in 0..w {
            let hs = hash(x * 7 + 5, y * 11);
            let (ch, fg) = match hs % 15 {
                0 => ('"', (60, 80, 60)),
                1 => ('.', (70, 70, 74)),
                2 => ('░', (52, 52, 58)),
                _ => (' ', (0, 0, 0)),
            };
            cv.put(x, y, ch, rgb(fg), Some(mix((20, 26, 22), (14, 16, 18), deep)));
        }
    }
    for k in 0..(w / 10).max(1) {
        let gx = 2 + (hash(k, 29 + seed as i32) % w.max(1) as u32) as i32;
        let ch = if hash(k, 31) % 2 == 0 { '∩' } else { '┼' };
        cv.glyph(gx, g, ch, rgb((130, 128, 136)));
    }
    let tx = if right { 3 + w / 20 } else { w - 7 - w / 20 };
    cv.sprite(tx, g - 3, &["╲│╱", " │╱", " │ "], rgb((90, 80, 72)));

    // The old halls below, open to the sky.
    if h - g > 8 {
        let rooms = 3 + (cv.roll(3) % 4) as i32;
        let mut glint = None;
        for k in 0..rooms {
            let rw = 8 + (hash(k, 41 + seed as i32) % 14) as i32;
            let rh = 4 + (hash(k, 43 + seed as i32) % 5) as i32;
            let rx = (hash(k, 47 + seed as i32) % (w - rw).max(1) as u32) as i32;
            let ry = g + 2 + (hash(k, 53 + seed as i32) % (h - g - rh - 2).max(1) as u32) as i32;
            for y in ry..=ry + rh {
                for x in rx..=rx + rw {
                    let edge = y == ry || y == ry + rh || x == rx || x == rx + rw;
                    if edge && hash(x, y + k) % 5 != 0 {
                        cv.put(x, y, '▓', rgb((96, 94, 104)), Some(rgb((44, 42, 50))));
                    } else if !edge && hash(x * 13, y * 7) % 23 == 0 {
                        cv.glyph(x, y, '%', rgb((200, 196, 180)));
                    }
                }
            }
            if glint.is_none() && hash(k, 59) % 2 == 0 {
                glint = Some((rx + rw / 2, ry + rh / 2));
            }
        }
        if let Some((x, y)) = glint {
            let shine = (t as f32 * 0.08).sin() * 0.5 + 0.5;
            cv.glyph(x, y, '☼', mix((150, 110, 30), (255, 240, 150), shine));
        }
    }

    // Mist rolling over the ground.
    for y in (g - 2).max(0)..h {
        for x in 0..w {
            let n = (x as f32 * 0.13 + t as f32 * 0.03 + y as f32 * 0.7).sin() + (x as f32 * 0.05 - t as f32 * 0.02 + y as f32 * 1.3).sin();
            let a = ((n - 0.6) * 0.25).clamp(0.0, 0.35);
            if a > 0.0 {
                let under = cv.bg_at(x, y);
                cv.tint(x, y, rgb(lerp(under, (110, 120, 130), a)));
            }
        }
    }

    // The restless dead, fading in and out.
    let ghosts = 2 + (cv.roll(4) % 4) as i32;
    for i in 0..ghosts {
        let fi = i as f32;
        let alpha = ((t as f32 * 0.02 + fi * 2.1).sin() * 0.5 + 0.5).powf(0.5);
        if alpha < 0.2 {
            continue;
        }
        let low = i % 3 == 2 && h - g > 8;
        let base = (hash(i, 61 + seed as i32) % w.max(1) as u32) as f32;
        let x = (base + (t as f32 * 0.012 + fi).sin() * w as f32 * 0.25).round() as i32;
        let y = if low {
            g + 3 + (((t as f32 * 0.01 + fi).sin() * 0.5 + 0.5) * (h - g - 6).max(0) as f32 * 0.5) as i32 + (hash(i, 67) % 4) as i32
        } else {
            g - 2 - (hash(i, 71) % (sky as u32 / 3).max(1)) as i32 + ((t as f32 * 0.05 + fi).sin() * 1.2).round() as i32
        };
        let heading = (t as f32 * 0.012 + fi).cos();
        let back = if heading > 0.0 { -1 } else { 1 };
        let col = lerp((60, 72, 96), (215, 240, 255), alpha);
        cv.glyph(x + back, y, '░', rgb(lerp((40, 50, 70), col, 0.5)));
        cv.glyph(x + back * 2, y, '·', rgb(lerp((40, 50, 70), col, 0.3)));
        cv.glyph(x, y, '☺', rgb(col));
    }

    // Bats.
    for k in 0..(w / 30 + 1) {
        let bx = ((hash(k, 73) as u64 + t / 2) % (w as u64 + 10)) as i32 - 5;
        let by = 1 + ((hash(k, 79) as f32 + t as f32 * 0.15).sin() * 1.5 + (sky as f32 / 3.0)) as i32;
        cv.glyph(bx, by, if (t / 3 + k as u64) % 2 == 0 { 'v' } else { '^' }, rgb((120, 110, 130)));
    }
}
