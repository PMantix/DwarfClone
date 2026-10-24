//! The sky islands: great clods of earth and rock adrift above a sea of
//! cloud, bobbing gently, trees on their backs and roots trailing beneath,
//! waterfalls spilling off their edges and vanishing into the mist, a
//! tower on the largest, chains between, birds wheeling and a balloon
//! drifting by. The seed picks the hour, the islands and their towers.

use super::*;
use super::c_kit::*;

pub(super) const TAGLINES: &[&str] = &[
    "the mountains that forgot to stay down",
    "a dwarf who falls from here has time to pray",
    "the rivers up here end in rain",
    "no road goes there, which is the point",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t) = (cv.w(), cv.h(), cv.t);
    let tf = t as f32;
    let sky = cv.sky.min(h - 1).max(1);
    let salt = cv.salt();
    let hour = cv.roll(1) % 3;
    let (top, bot, cloud) = [((50, 110, 210), (170, 210, 245), (250, 250, 255)), ((70, 70, 140), (250, 180, 150), (255, 220, 210)), ((30, 50, 120), (120, 170, 220), (220, 230, 250))][hour as usize];
    let cloud_top = (sky as f32 * 0.82) as i32;
    cv.sky_fill(h, top, bot);
    let (sx, sy) = ((w as f32 * 0.08) as i32 + 4, (sky as f32 * 0.25) as i32);
    cv.halo(sx, sy, 6.0, (255, 240, 200), 0.4);
    cv.disc(sx, sy, (sky as f32 * 0.08).max(0.8).min(2.0), (255, 250, 225));

    // A far, faint layer of islands, then the near ones.
    let n_far = 2 + w / 50;
    for k in 0..n_far {
        let x = (hash(k, salt) % w.max(1) as u32) as i32;
        let y = 3 + (hash(k, salt + 1) % (cloud_top.max(6) as u32 / 2)) as i32;
        let hw = 3 + (hash(k, 3) % 4) as i32;
        let col = lerp(bot, (110, 120, 150), 0.35);
        for r in 0..3 {
            let half = hw - r * hw / 3;
            for dx in -half..=half {
                cv.fill(x + dx, y + r, rgb(col));
            }
        }
    }
    let n = 3 + (w / 60).min(3);
    let mut biggest = (0, 0, 0);
    let mut falls: Vec<(i32, i32)> = Vec::new();
    let mut tops: Vec<(i32, i32, i32)> = Vec::new();
    for k in 0..n {
        let slot = w as f32 / n as f32;
        let cx = (slot * (k as f32 + 0.5) + ((hash(k, salt + 2) % 100) as f32 / 100.0 - 0.5) * slot * 0.5) as i32;
        let size = 0.5 + (hash(k, salt + 3) % 100) as f32 / 100.0;
        let hw = ((slot * 0.32 * size) as i32).max(3);
        let depth = (hw / 2).max(2).min(sky / 2);
        let base_y = 6 + (hash(k, salt + 4) % ((cloud_top - 8 - depth).max(1) as u32)) as i32;
        let bob = ((tf * 0.025 + k as f32 * 1.7).sin() * 1.2).round() as i32;
        let ty = base_y + bob;
        if hw > biggest.2 {
            biggest = (cx, ty, hw);
        }
        tops.push((cx, ty, hw));
        // The underside, tapering to roots.
        for r in 0..=depth {
            let f = r as f32 / depth as f32;
            let half = (hw as f32 * (1.0 - f).powf(0.7)) as i32 + (hash(r, k) % 2) as i32 - 1;
            for dx in -half..=half {
                let x = cx + dx;
                let y = ty + r;
                if r == 0 {
                    cv.put(x, y, if hash(x, salt) % 3 == 0 { '"' } else { ' ' }, rgb((120, 190, 80)), Some(rgb((70, 150, 60))));
                } else {
                    let strata = [(120, 90, 60), (100, 76, 56), (140, 110, 80)][((r + (hash(x, k) % 2) as i32) % 3) as usize];
                    let ch = if hash(x * 3, y + salt) % 7 == 0 { '▒' } else { ' ' };
                    cv.put(x, y, ch, rgb(lerp(strata, (0, 0, 0), 0.25)), Some(rgb(lerp(strata, (0, 0, 0), f * 0.4))));
                }
            }
            if r == depth {
                for j in 1..3 {
                    cv.glyph(cx + (hash(k, j) % 3) as i32 - 1, ty + r + j, if j == 1 { '│' } else { '╎' }, rgb((90, 70, 50)));
                }
            }
        }
        // Trees on top.
        for dx in -hw + 1..hw {
            if hash(cx + dx, salt + k) % 3 == 0 {
                cv.glyph(cx + dx, ty - 1, if hash(dx, k) % 2 == 0 { '♣' } else { '♠' }, rgb([(40, 130, 60), (50, 150, 70), (30, 110, 70)][(hash(dx, 5) % 3) as usize]));
            }
        }
        if hash(k, 77) % 2 == 0 || k == 0 {
            falls.push((cx + hw - 1, ty));
        }
    }
    // A tower on the biggest, a pennant streaming.
    let (bx, by, bw) = biggest;
    let tx = bx - bw / 3;
    let tall = (by - 2).min(5).max(2);
    for j in 1..=tall {
        cv.put(tx, by - j, if j == tall { '▀' } else { ' ' }, rgb((150, 60, 50)), Some(rgb((210, 200, 185))));
        cv.fill(tx + 1, by - j, rgb((180, 170, 160)));
    }
    cv.put(tx, by - tall + 1, '▪', rgb((60, 70, 100)), Some(rgb((210, 200, 185))));
    cv.glyph(tx, by - tall - 1, '│', rgb((90, 70, 50)));
    cv.glyph(tx + 1, by - tall - 1, if (t / 5) % 2 == 0 { '►' } else { '▶' }, rgb((200, 50, 50)));
    // Chains between neighbours.
    for pair in tops.windows(2) {
        let ((ax, ay, aw), (cx, cy, cw)) = (pair[0], pair[1]);
        let (x0, x1) = (ax + aw, cx - cw);
        if x1 - x0 < 4 {
            continue;
        }
        for x in x0..=x1 {
            let u = (x - x0) as f32 / (x1 - x0) as f32;
            let y = ay as f32 + (cy - ay) as f32 * u + (u * std::f32::consts::PI).sin() * 2.0 + 1.0;
            if (x - x0) % 2 == 0 {
                cv.glyph(x, y.round() as i32, 'o', rgb((80, 80, 90)));
            }
        }
    }
    // Waterfalls spilling off the edges into the clouds.
    for &(fx, fy) in &falls {
        for y in fy..cloud_top + 1 {
            let ch = if (y + (t / 2) as i32) % 3 == 0 { '║' } else { '│' };
            cv.put(fx + 1, y, ch, rgb((230, 245, 255)), Some(rgb(lerp((120, 170, 230), bot, (y - fy) as f32 / (cloud_top - fy).max(1) as f32 * 0.7))));
        }
        cv.halo(fx + 1, cloud_top, 2.5, (255, 255, 255), 0.5);
    }

    // The sea of cloud below, rolling slowly.
    for y in cloud_top.max(0)..h {
        for x in 0..w {
            let n = fbm(x as f32 * 0.06 + tf * 0.01, y as f32 * 0.2, salt, 4);
            let edge = y - cloud_top;
            if edge == 0 && n < 0.5 {
                continue;
            }
            let shade = lerp(cloud, lerp(bot, (120, 130, 170), 0.4), (n - 0.4).max(0.0) * 1.6);
            let ch = if edge == 0 { '▄' } else { ' ' };
            if edge == 0 {
                cv.glyph(x, y, ch, rgb(shade));
            } else {
                cv.fill(x, y, rgb(shade));
            }
        }
    }

    // Birds, and a balloon drifting through.
    for k in 0..(2 + w / 40) {
        let a = tf * 0.04 + k as f32 * 1.3;
        let x = (hash(k, 9) % w.max(1) as u32) as i32 + (a.cos() * 6.0) as i32;
        let y = 3 + (hash(k, 10) % (sky.max(6) as u32 / 2)) as i32 + (a.sin() * 1.5) as i32;
        cv.glyph(x, y, if (t / 5 + k as u64) % 2 == 0 { 'v' } else { '^' }, rgb((40, 40, 60)));
    }
    let trip = (w + 20) as u64 * 9;
    let bxp = ((t + cv.roll(5) as u64 % trip) % trip / 9) as i32 - 10;
    let byp = (sky as f32 * 0.3) as i32 + ((tf * 0.03).sin() * 1.5) as i32;
    let env = [(220, 60, 60), (240, 200, 60)];
    for dx in -1..=1 {
        cv.glyph(bxp + dx, byp, '█', rgb(env[(dx.rem_euclid(2)) as usize]));
        cv.glyph(bxp + dx, byp - 1, '▄', rgb(env[((dx + 1).rem_euclid(2)) as usize]));
    }
    cv.glyph(bxp, byp + 1, '╨', rgb((120, 90, 60)));
    cv.glyph(bxp, byp + 2, '▀', rgb((140, 100, 60)));
}
