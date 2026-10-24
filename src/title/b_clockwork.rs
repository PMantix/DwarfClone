//! The clockwork shop: a wall of meshed gears turning against each other,
//! a pendulum keeping time, steam sighing from the pipes, and a brass
//! automaton clanking up and down the floor while its maker chases it with
//! a wrench. Down below, the engine room: pistons pounding and a belt
//! carrying cogs away. The seed picks the metal, the gears and the
//! automaton's shape.

use super::b_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "it only needs one more gear",
    "tick, tock, and a dwarf swearing",
    "the automaton has opinions about the floor",
    "brass, steam and stubbornness",
];

/// Metals: bright, mid, shadow.
const METALS: [(Rgb, Rgb, Rgb); 3] = [
    ((250, 210, 120), (190, 145, 60), (90, 64, 30)),  // brass
    ((240, 160, 110), (180, 100, 60), (84, 44, 28)),  // copper
    ((210, 215, 225), (140, 145, 160), (60, 62, 72)), // steel
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t) = (cv.w(), cv.h(), cv.t);
    let g = (cv.sky - 1).max(2).min(h - 1);
    let tf = t as f32;
    let salt = cv.salt();
    let (hi, mid, lo) = METALS[(cv.roll(1) % 3) as usize];
    let wall = [(46, 34, 28), (34, 36, 44), (44, 30, 34)][(cv.roll(2) % 3) as usize];

    // The shop: panelled walls, warm from the lamps.
    for y in 0..g.min(h) {
        for x in 0..w {
            let panel = (x % 10 == 0) || (y % 6 == 0);
            let warm = ((x as f32 * 0.08).sin() * 0.5 + 0.5) * 0.12;
            let bg = lerp(lerp(wall, (90, 60, 30), warm), (0, 0, 0), if panel { 0.25 } else { 0.0 });
            let ch = if panel && hash(x, y + salt) % 3 == 0 { '·' } else { ' ' };
            cv.put(x, y, ch, rgb(lerp(lo, wall, 0.4)), Some(rgb(bg)));
        }
    }
    floor(cv, 0, w, g, lerp(lo, (60, 50, 40), 0.5));

    // A chain of gears along the wall, each turning against the last.
    let top = 1;
    let room = (g - top - 4).max(2) as f32; // rows the gears may use
    let mut gears: Vec<(f32, i32, f32)> = Vec::new(); // (centre x, centre y, radius)
    let mut x = 2.0 + (cv.roll(3) % 6) as f32;
    let mut k = 0u32;
    while x < w as f32 - 2.0 {
        let r = (room * (0.22 + (roll(cv.seed, 40 + k) % 30) as f32 / 100.0)).clamp(1.0, 7.0);
        let cx = x + r * 2.0;
        if cx + r * 2.0 > w as f32 + 2.0 {
            break;
        }
        let cy = top + (r + (roll(cv.seed, 60 + k) % ((room - r * 2.0).max(1.0) as u32 + 1)) as f32) as i32;
        gears.push((cx, cy, r));
        x = cx + r * 2.0 + 0.2;
        k += 1;
    }
    for (k, &(cx, cy, r)) in gears.iter().enumerate() {
        let dir = if k % 2 == 0 { 1.0 } else { -1.0 };
        let rot = dir * tf * 0.06 / r + k as f32 * 0.3;
        gear(cv, cx, cy, r, rot, (hi, mid, lo));
    }
    // Axles from the bigger gears down to the floor.
    for &(cx, cy, r) in gears.iter().filter(|g| g.2 >= 2.5) {
        for y in cy + r as i32 + 1..g {
            cv.glyph(cx as i32, y, '│', rgb(lo));
        }
    }

    // A pendulum clock keeping time.
    let pcx = (w as f32 * 0.86) as i32;
    let plen = ((g - 2) as f32 * 0.6).max(2.0);
    let swing = (tf * 0.07).sin() * 0.45;
    let (bx, by) = (pcx + (swing.sin() * plen * 2.0) as i32, 1 + (swing.cos() * plen) as i32);
    for (px, py) in line((pcx, 1), (bx, by)) {
        cv.glyph(px, py, '·', rgb(mid));
    }
    cv.glyph(bx, by, 'O', rgb(hi));
    cv.glyph(pcx, 0, '▼', rgb(mid));

    // Steam from a pipe now and then.
    let px = (w as f32 * 0.1) as i32 + 1;
    for y in 1..g {
        cv.glyph(px, y, '║', rgb(lo));
    }
    if (t / 90) % 3 != 0 {
        for k in 0..5 {
            let age = ((t / 3) as i32 + k * 4) % 18;
            let sx = px + 1 + age / 2;
            let sy = 2 + ((age as f32 * 0.7).sin() * 1.2) as i32;
            if age < 14 {
                cv.glyph(sx, sy, if age < 6 { '▒' } else { '░' }, rgb(lerp((230, 230, 235), wall, age as f32 / 14.0)));
            }
        }
    }

    // The automaton, clanking along, and its maker after it.
    let kind = cv.roll(4) % 3;
    let body: [&[&str]; 3] = [&["▄▀▄", "◘█◘", "╨ ╨"], &[" Θ ", "╞█╡", "/ \\"], &["▐▀▌", "◘▓◘", "╜ ╙"]];
    let rows = body[kind as usize];
    let ah = rows.len() as i32;
    let off = t + cv.roll(5) as u64 % 700;
    let lo_x = (w / 5).max(1);
    let hi_x = (w * 3 / 4).max(lo_x + 2);
    let ax = pingpong(off, 4, lo_x, hi_x);
    let ay = g - ah;
    let step = (t / 4) % 2 == 0;
    for (dy, r) in rows.iter().enumerate() {
        for (dx, ch) in r.chars().enumerate() {
            if ch == ' ' {
                continue;
            }
            let ch = if dy as i32 == ah - 1 && step { if ch == '╨' { '╥' } else { ch } } else { ch };
            let col = if ch == '◘' || ch == 'Θ' { hi } else { mid };
            cv.glyph(ax + dx as i32, ay + dy as i32, ch, rgb(col));
        }
    }
    // Its eye glows; a puff of steam from its head.
    cv.halo(ax + 1, ay, 1.5, (255, 200, 120), 0.3);
    if (t / 6) % 5 == 0 {
        cv.glyph(ax + 1, ay - 1, '°', rgb((220, 220, 230)));
    }
    let going_right = (off / 4) % (2 * (hi_x - lo_x) as u64) < (hi_x - lo_x) as u64;
    let mx = if going_right { ax - 3 } else { ax + 5 };
    cv.glyph(mx, g - 1, '☺', rgb(beard(cv.roll(6))));
    cv.glyph(mx + if going_right { 1 } else { -1 }, g - 2, if (t / 5) % 2 == 0 { '¬' } else { '─' }, rgb((180, 180, 190)));
    if (t / 40) % 4 == 0 {
        cv.glyph(mx, g - 2, '!', rgb((255, 240, 160)));
    }
    // Lamps on the wall.
    for (k, lx) in [w / 3, w * 2 / 3].into_iter().enumerate() {
        torch(cv, lx, g - 4, k as i32, 3.5);
    }

    // The engine room below: pistons and a belt of cogs.
    bedrock(cv, g + 1, h, lerp(wall, (60, 56, 50), 0.5), salt);
    let er = below(cv);
    if h - er >= 4 {
        let ey = er;
        hollow(cv, 1, ey, w - 1, h - 1, lerp(wall, (0, 0, 0), 0.4));
        let fy = h - 1;
        // The belt, cogs riding it.
        for x in 1..w - 1 {
            let ch = if (x + (t / 3) as i32) % 4 == 0 { '╪' } else { '═' };
            cv.put(x, fy, ch, rgb(mid), Some(rgb(lerp(lo, (0, 0, 0), 0.5))));
            if (x + (t / 3) as i32) % 9 == 0 {
                cv.glyph(x, fy - 1, '☼', rgb(hi));
            }
        }
        // Pistons pounding in turn.
        let n = ((w - 2) / 9).max(1);
        for k in 0..n {
            let px = 4 + k * 9;
            let up = ((tf * 0.15 + k as f32 * 1.3).sin() * 0.5 + 0.5) * ((fy - 2 - ey).max(0) as f32);
            let head = ey + up as i32;
            for y in ey..head {
                cv.glyph(px, y, '║', rgb(mid));
            }
            cv.glyph(px - 1, head, '▀', rgb(hi));
            cv.glyph(px, head, '█', rgb(hi));
            cv.glyph(px + 1, head, '▀', rgb(hi));
        }
        // The furnace's mouth glowing at one end.
        let fx = w - 4;
        let f = flicker(t, 3);
        cv.halo(fx, fy - 2, 3.0, (255, 120, 40), 0.6 * f);
        cv.glyph(fx, fy - 2, '▲', rgb(lerp((255, 120, 40), (255, 230, 120), f)));
    }
}

/// A gear of radius `r` rows round (`cx`, `cy`), turned `rot` radians: a
/// rim, teeth standing out of it, three spokes and a hub.
fn gear(cv: &mut Canvas, cx: f32, cy: i32, r: f32, rot: f32, (hi, mid, lo): (Rgb, Rgb, Rgb)) {
    let x0 = (cx - r * 2.0 - 3.0) as i32;
    let x1 = (cx + r * 2.0 + 3.0) as i32;
    let teeth = (r * 4.0).round().max(6.0);
    let tau = std::f32::consts::TAU;
    for y in cy - r as i32 - 2..=cy + r as i32 + 2 {
        for x in x0..=x1 {
            let (dx, dy) = ((x as f32 - cx) * 0.5, (y - cy) as f32);
            let d = (dx * dx + dy * dy).sqrt();
            let a = dy.atan2(dx) - rot;
            let lit = lerp(lo, hi, (0.55 - dy / (r * 2.5)).clamp(0.0, 1.0));
            if d > r + 0.15 && d <= r + 1.0 {
                if (a / tau * teeth).rem_euclid(1.0) < 0.45 {
                    cv.put(x, y, '█', rgb(lit), Some(rgb(lerp(mid, lo, 0.3))));
                }
            } else if d > r - 0.75 && d <= r + 0.15 {
                cv.put(x, y, '▓', rgb(lit), Some(rgb(lerp(mid, lo, 0.5))));
            } else if d <= r - 0.75 {
                let spoke = (a * 3.0 / tau).rem_euclid(1.0);
                if (spoke < 0.09 || spoke > 0.91) && d > 0.5 {
                    cv.put(x, y, '▒', rgb(mid), Some(rgb(lerp(mid, lo, 0.6))));
                } else if d > 0.5 {
                    cv.fill(x, y, rgb(lerp(lo, (0, 0, 0), 0.5)));
                }
            }
        }
    }
    cv.put(cx.round() as i32, cy, '◘', rgb(hi), Some(rgb(lo)));
}
