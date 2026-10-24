//! A world being made: the map drawn before your eyes as the land heaves
//! up out of an empty sea into continents, mountains piling up along their
//! spines, forests and deserts spreading with the rains, rivers finding
//! their way down to the sea, and then, one by one, the first towns,
//! holds and dark fortresses. Clouds pass over. In time the sea takes it
//! all back and another world rises. The seed is the world.

use super::*;
use super::c_kit::*;

pub(super) const TAGLINES: &[&str] = &[
    "first the land, then the rivers, then the trouble",
    "a world, fresh from the forge of the ages",
    "every mountain here will be somebody's home",
    "history begins the moment the map is dry",
];

/// Frames a world takes from empty sea to drowned again.
const AGE: u64 = 2400;
/// Where the sea settles once the land is up.
const SHORE: f32 = 0.5;

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t) = (cv.w(), cv.h(), cv.t);
    if w <= 0 || h <= 0 {
        return;
    }
    let tt = t + cv.roll(1) as u64 % AGE;
    let p = (tt % AGE) as f32;
    let world = (tt / AGE) as i32;
    let salt = cv.salt().wrapping_add(world.wrapping_mul(1013));
    // The sea falls as the land rises, and comes back at the end.
    let sl = if p < 900.0 {
        0.8 - (0.8 - SHORE) * ease(0.0, 900.0, p)
    } else if p > 2200.0 {
        SHORE + (0.8 - SHORE) * ease(2200.0, AGE as f32, p)
    } else {
        SHORE
    };

    // The lie of the land, once for the whole map.
    let (wu, hu) = (w as usize, h as usize);
    let mut el = vec![0f32; wu * hu];
    for y in 0..h {
        for x in 0..w {
            let (fx, fy) = (x as f32, y as f32);
            let mut e = fbm(fx * 0.045, fy * 0.09, salt, 5) * 0.6 + noise(fx * 0.014, fy * 0.028, salt.wrapping_add(9)) * 0.45;
            let (ex, ey) = (x.min(w - 1 - x) as f32 / (w as f32 * 0.08).max(1.0), y.min(h - 1 - y) as f32 / (h as f32 * 0.1).max(1.0));
            let edge = ex.min(ey).min(1.0);
            e -= (1.0 - edge) * 0.18;
            el[y as usize * wu + x as usize] = e;
        }
    }
    let at = |x: i32, y: i32| -> f32 { if x < 0 || y < 0 || x >= w || y >= h { -1.0 } else { el[y as usize * wu + x as usize] } };

    for y in 0..h {
        let lat = ((y as f32 - h as f32 / 2.0).abs() / (h as f32 / 2.0).max(1.0)).min(1.0);
        for x in 0..w {
            let e = at(x, y);
            let (fx, fy) = (x as f32, y as f32);
            let (ch, mut fg, mut bg);
            if e < sl {
                let coast = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| at(x + dx, y + dy) >= sl);
                let depth = ((sl - e) / 0.25).min(1.0);
                bg = if coast { (70, 130, 185) } else { lerp((36, 80, 150), (10, 26, 70), depth) };
                let wave = hash(x + (t / 25) as i32, y * 3 + salt) % 29 == 0;
                ch = if wave { '~' } else { ' ' };
                fg = lerp(bg, (200, 220, 240), 0.4);
            } else {
                let hgt = e - sl;
                let m = fbm(fx * 0.06, fy * 0.12, salt.wrapping_add(77), 3);
                let hs = hash(x + salt, y);
                if hgt > 0.23 {
                    let snow = hgt > 0.29 || lat > 0.7;
                    ch = '▲';
                    fg = if snow { (240, 242, 250) } else { (160, 150, 140) };
                    bg = (104, 92, 80);
                } else if lat > 0.84 {
                    ch = if hs % 5 == 0 { '·' } else { ' ' };
                    fg = (190, 200, 215);
                    bg = (222, 228, 238);
                } else if hgt < 0.012 {
                    ch = ' ';
                    fg = (0, 0, 0);
                    bg = (214, 198, 140);
                } else if m > 0.56 {
                    ch = if hs % 3 != 0 { '♣' } else { ' ' };
                    fg = if lat > 0.55 { (40, 110, 70) } else { (50, 140, 50) };
                    bg = (34, 84, 38);
                } else if m < 0.41 && lat < 0.55 {
                    ch = if hs % 6 == 0 { '~' } else { ' ' };
                    fg = (230, 200, 130);
                    bg = (206, 172, 100);
                } else {
                    ch = if hs % 4 == 0 { '"' } else { ' ' };
                    fg = (120, 180, 80);
                    bg = if hgt > 0.14 { (100, 120, 64) } else { (78, 138, 58) };
                }
            }
            // Cloud shadows drifting over.
            let c = fbm(fx * 0.04 - t as f32 * 0.012, fy * 0.08, salt.wrapping_add(5), 3);
            if c > 0.6 {
                let f = ((c - 0.6) * 4.0).min(0.6);
                bg = lerp(bg, (235, 238, 245), f);
                fg = lerp(fg, (235, 238, 245), f);
            }
            cv.put(x, y, ch, rgb(fg), Some(rgb(bg)));
        }
    }

    // Rivers running down from the heights, longer as the age goes on.
    let grown = ((p - 700.0) / 3.0).max(0.0) as usize;
    if grown > 0 {
        for k in 0..((w * h) / 260).max(1) {
            let (mut x, mut y) = ((hash(k, salt) % w as u32) as i32, (hash(salt, k) % h as u32) as i32);
            if at(x, y) < SHORE + 0.15 {
                continue;
            }
            for _ in 0..grown.min(400) {
                let mut best = (x, y, at(x, y));
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, 1), (1, -1), (-1, -1)] {
                    let e = at(x + dx, y + dy);
                    if e > -1.0 && e < best.2 {
                        best = (x + dx, y + dy, e);
                    }
                }
                if at(x, y) >= sl && at(x, y) - sl < 0.2 {
                    cv.put(x, y, '~', rgb((150, 200, 240)), Some(rgb((50, 100, 200))));
                }
                if (best.0, best.1) == (x, y) || best.2 < sl {
                    break;
                }
                x = best.0;
                y = best.1;
            }
        }
    }

    // The first peoples settling: holds, towns, retreats, dark fortresses.
    let mut placed = 0;
    for k in 0..200 {
        if placed >= 4 + (w * h) / 500 {
            break;
        }
        let (x, y) = ((hash(k, salt.wrapping_add(3)) % w as u32) as i32, (hash(salt.wrapping_add(4), k) % h as u32) as i32);
        let e = at(x, y);
        if !(SHORE + 0.02..SHORE + 0.2).contains(&e) {
            continue;
        }
        let born = 1100.0 + placed as f32 * 45.0;
        placed += 1;
        if p < born || e < sl {
            continue;
        }
        let (ch, col) = [('☼', (255, 210, 80)), ('■', (245, 245, 245)), ('♠', (120, 250, 210)), ('Ω', (240, 60, 60))][(hash(k, 5) % 4) as usize];
        let age = p - born;
        if age < 50.0 {
            cv.halo(x, y, 1.0 + age / 12.0, col, 0.6 * (1.0 - age / 50.0));
        }
        cv.put(x, y, ch, rgb(col), Some(rgb((20, 20, 24))));
    }

    // The map's frame, and its compass.
    let ink = (200, 170, 110);
    let paper = (54, 36, 22);
    for x in 0..w {
        cv.put(x, 0, '═', rgb(ink), Some(rgb(paper)));
        cv.put(x, h - 1, '═', rgb(ink), Some(rgb(paper)));
    }
    for y in 0..h {
        cv.put(0, y, '║', rgb(ink), Some(rgb(paper)));
        cv.put(w - 1, y, '║', rgb(ink), Some(rgb(paper)));
    }
    cv.put(0, 0, '╔', rgb(ink), Some(rgb(paper)));
    cv.put(w - 1, 0, '╗', rgb(ink), Some(rgb(paper)));
    cv.put(0, h - 1, '╚', rgb(ink), Some(rgb(paper)));
    cv.put(w - 1, h - 1, '╝', rgb(ink), Some(rgb(paper)));
    if w >= 30 && h >= 12 {
        let (rx, ry) = (4, h - 5);
        let needle = if (t / 20) % 2 == 0 { (255, 230, 160) } else { ink };
        cv.glyph(rx, ry - 1, 'N', rgb(needle));
        cv.glyph(rx, ry, '┼', rgb(ink));
        cv.glyph(rx - 1, ry, '─', rgb(ink));
        cv.glyph(rx + 1, ry, '─', rgb(ink));
        cv.glyph(rx, ry + 1, '│', rgb(ink));
    }
}
