//! The necromancer's tower: a sky gone sick and green, clouds wheeling
//! round the spire, lightning reaching down to a staff held high, and on
//! the barrow field below the dead clawing their way out and shambling off
//! towards the one lit door in the hills, where a dwarf with a torch
//! watches. The seed picks the sky's poison, the barrows and the dead.

use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the dead do not stay buried here",
    "someone up there is reading the wrong books",
    "bar the door and count the torches",
    "the barrows are emptying",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let g = (cv.sky - 1).max(1); // the ground line
    let tf = t as f32;
    let salt = cv.salt();
    let (top, bot, magic) = [((6, 18, 10), (70, 120, 50), (140, 255, 120)), ((18, 8, 26), (90, 70, 110), (190, 140, 255)), ((6, 16, 22), (60, 110, 110), (120, 255, 220))][(cv.roll(1) % 3) as usize];
    let big = w >= 110 && g >= 16;

    // The tower.
    let tc = ((w as f32 * 0.09) as i32).max(3);
    let tw = (w / 16).clamp(3, 9) | 1;
    let ttop = 2.min(g - 3).max(0) + if big { 1 } else { 0 };
    let orb = (tc + tw / 2 + 1, ttop - 1);

    // Sky, wheeling about the spire.
    let period = 210u64;
    let strike = t % period < 4 && cv.roll(2) % 3 != 0;
    for y in 0..g.min(h) {
        let base = lerp(top, bot, (y as f32 / g.max(1) as f32).powf(1.3));
        for x in 0..w {
            let (dx, dy) = ((x - tc) as f32 * 0.5, (y - ttop) as f32);
            let d = (dx * dx + dy * dy).sqrt();
            let a = dy.atan2(dx);
            let v = (a * 3.0 + d * 0.22 - tf * 0.05).sin();
            let near = (1.0 - d / (w as f32 * 0.4).max(10.0)).max(0.0);
            let cloud = v * (0.4 + near * 0.6);
            let mut c = if cloud > 0.35 { lerp(base, lerp(top, (0, 0, 0), 0.3), (cloud - 0.35) * 1.2) } else if cloud < -0.6 { lerp(base, magic, 0.12 * near) } else { base };
            if strike {
                c = lerp(c, magic, 0.35);
            }
            let ch = if cloud > 0.75 && hash(x * 3, y * 7) % 4 == 0 { '░' } else { ' ' };
            cv.put(x, y, ch, rgb(lerp(c, (0, 0, 0), 0.3)), Some(rgb(c)));
        }
    }
    // A sick moon.
    if cv.roll(3) % 4 != 0 {
        let (mx, my) = ((w as f32 * 0.8) as i32, (g as f32 * 0.3) as i32 + 1);
        let r = (g as f32 * 0.11).clamp(0.8, 3.5);
        cv.halo(mx, my, r * 2.5, lerp(magic, (255, 255, 255), 0.5), 0.25);
        cv.disc(mx, my, r, lerp(magic, (230, 240, 210), 0.7));
    }

    // Far hills, and the hill with the lit door.
    for x in 0..w {
        let m = ridge(x, w, seed, 60) * 0.5;
        let s = g - 1 - (m * (g as f32 * 0.35)) as i32;
        for y in s.max(0)..g {
            cv.fill(x, y, rgb(lerp(bot, (14, 18, 16), 0.65)));
        }
    }
    let hx = w - (w as f32 * 0.08) as i32 - 3;
    let hr = (w / 14).clamp(4, 14);
    let hh = (g as f32 * 0.35).clamp(2.0, 9.0);
    for x in hx - hr..=hx + hr {
        let u = (x - hx) as f32 / hr as f32;
        let s = g - (hh * (1.0 - u * u).max(0.0).sqrt()) as i32;
        for y in s.max(0)..g {
            cv.put(x, y, if y == s { '▄' } else { ' ' }, rgb((40, 54, 40)), Some(rgb((22, 28, 22))));
        }
    }
    cv.put(hx, g - 1, ' ', Color::Black, Some(rgb((255, 180, 80))));
    cv.glyph(hx, g - 2, '∩', rgb((110, 100, 90)));
    let flick = ((tf * 0.5).sin() * 0.5 + 0.5) * 0.2 + 0.5;
    cv.halo(hx - 2, g - 2, 3.0, (255, 160, 60), flick);
    cv.glyph(hx - 2, g - 1, '☺', rgb((240, 200, 140)));
    cv.glyph(hx - 1, g - 2, '¡', rgb((255, 200, 90)));
    cv.glyph(hx - 1, g - 3, if (t / 4) % 2 == 0 { '*' } else { '·' }, rgb((255, 150, 40)));

    // The ground, misty, and the barrows.
    for y in g.max(0)..h {
        for x in 0..w {
            let mist = ((x as f32 * 0.07 + tf * 0.01).sin() * 0.5 + 0.5) * (1.0 - ((y - g) as f32 / 3.0).min(1.0));
            let bg = lerp((20, 24, 18), lerp(bot, (200, 210, 200), 0.2), mist * 0.35);
            let hs = hash(x + salt, y * 5);
            let ch = if hs % 9 == 0 { '"' } else if hs % 47 == 0 { '·' } else { ' ' };
            cv.put(x, y, ch, rgb((70, 80, 60)), Some(rgb(bg)));
        }
    }
    let nb = ((w - tc - hr * 2 - tw) / 16).clamp(1, 7);
    let span = ((hx - hr - 4) - (tc + tw + 4)).max(nb * 4);
    let br = (span / nb / 3).clamp(2, 8);
    let bh = if big { 3 } else { 2 };
    let barrows: Vec<i32> = (0..nb).map(|k| tc + tw + 4 + span * (2 * k + 1) / (2 * nb) + (hash(k, salt) % 3) as i32 - 1).collect();
    for &bx in &barrows {
        for x in bx - br..=bx + br {
            let u = (x - bx) as f32 / br as f32;
            let s = g - (bh as f32 * (1.0 - u * u).max(0.0)).round() as i32;
            for y in s..g {
                cv.put(x, y, if y == s { '▄' } else { ' ' }, rgb((44, 60, 40)), Some(rgb((26, 34, 24))));
            }
        }
        cv.put(bx, g - 1, '∩', rgb((120, 120, 110)), Some(rgb(lerp((10, 10, 10), magic, 0.3))));
        if bx + br + 2 < hx - hr {
            cv.glyph(bx + br + 1, g - 1, '┼', rgb((110, 110, 100)));
        }
    }
    // Dead trees.
    for k in 0..(w / 30).max(1) {
        let x = tc + tw + 2 + (hash(k, salt + 5) % ((hx - hr - tc - tw - 4).max(1) as u32)) as i32;
        if barrows.iter().any(|&b| (b - x).abs() <= br + 1) {
            continue;
        }
        let dark = rgb((40, 34, 40));
        cv.glyph(x, g - 1, '│', dark);
        cv.glyph(x, g - 2, 'Y', dark);
        cv.glyph(x - 1, g - 3, '\\', dark);
        cv.glyph(x + 1, g - 3, '/', dark);
    }
    // A graveyard spreading down the slope, some graves already open.
    if h - g > 5 {
        let mut row = 0;
        let mut y = g + 2;
        while y + 1 < h {
            let mut x = 3 + (row % 2) * 4 + (hash(row, salt) % 3) as i32;
            while x < w - 2 {
                let hs = hash(x + salt, y * 3);
                if hs % 4 != 0 {
                    let stone = ['∩', '┼', '∩', '†', '▲'][(hs / 4 % 5) as usize];
                    cv.glyph(x, y, stone, rgb((112, 112, 104)));
                    let open = hs % 5 == 0;
                    if !open {
                        cv.put(x, y + 1, '▀', rgb((58, 46, 34)), None);
                    } else {
                        cv.put(x, y + 1, ' ', Color::Black, Some(rgb((8, 8, 6))));
                        let life = 300 + (hs % 200) as u64;
                        let a = (t + hs as u64) % life;
                        let col = if hs % 3 == 0 { (225, 220, 200) } else { (150, 190, 140) };
                        if (40..80).contains(&a) {
                            cv.glyph(x, y + 1, '▄', rgb(col));
                            cv.halo(x, y + 1, 1.2, magic, 0.35);
                        } else if (80..240).contains(&a) {
                            cv.glyph(x + 1, y + 1, if hs % 3 == 0 { '☺' } else { '☻' }, rgb(col));
                            cv.glyph(x + 2, y + 1 - ((a / 12) % 2) as i32, '─', rgb(lerp(col, (0, 0, 0), 0.2)));
                        } else if a < 40 && (t / 3) % 2 == 0 {
                            cv.glyph(x, y + 1, '░', rgb((90, 80, 60)));
                        }
                    }
                }
                x += 7 + (hs % 3) as i32;
            }
            y += 3;
            row += 1;
        }
        // Mist creeping between the stones.
        for y in g..h {
            for x in 0..w {
                let m = ((x as f32 * 0.05 + y as f32 * 0.3 - tf * 0.012).sin() + (x as f32 * 0.021 + tf * 0.007).sin()) * 0.5;
                if m > 0.3 {
                    cv.glow(x, y, lerp(bot, (200, 210, 200), 0.3), (m - 0.3) * 0.35);
                }
            }
        }
    }

    // The tower itself.
    let stone = (34, 32, 40);
    for y in ttop..g {
        let wide = if y <= ttop + 2 { 1 } else { 0 };
        for x in tc - tw / 2 - wide..=tc + tw / 2 + wide {
            let course = (y + if (x % 2) == 0 { 0 } else { 1 }) % 2 == 0;
            let ch = if y == ttop && (x - tc) % 2 != 0 { '▄' } else if course { '▒' } else { ' ' };
            let lit = if strike { 0.3 } else { 0.0 };
            cv.put(x, y, ch, rgb((48, 46, 56)), Some(rgb(lerp(stone, magic, lit))));
        }
    }
    let wy = ttop + 2;
    if wy < g {
        cv.put(tc, wy, '▐', rgb(magic), Some(rgb(lerp(magic, (0, 0, 0), 0.3))));
    }
    for k in 0..((g - ttop) / 3) {
        let y = ttop + 4 + k * 3;
        if y < g - 1 {
            let on = (t / 13 + k as u64) % 7 != 0;
            cv.glyph(tc + if k % 2 == 0 { -1 } else { 1 }, y, '▪', rgb(if on { magic } else { (40, 60, 40) }));
        }
    }
    cv.put(tc, g - 1, '∩', rgb((90, 90, 100)), Some(rgb((10, 10, 12))));

    // The necromancer atop it, staff high.
    let robe = [(120, 60, 160), (40, 40, 50), (150, 30, 40)][(cv.roll(4) % 3) as usize];
    if ttop >= 1 {
        let raised = (t / 40) % 3 != 0;
        cv.glyph(tc, ttop - 1, '☻', rgb(robe));
        cv.glyph(orb.0, ttop - 1, '│', rgb((110, 90, 70)));
        let oy = if raised { ttop - 2 } else { ttop - 1 };
        cv.glyph(orb.0, oy, '●', rgb(lerp(magic, (255, 255, 255), 0.4)));
        cv.halo(orb.0, oy, if raised { 2.5 } else { 1.5 }, magic, 0.5);
    }
    // Lightning down to the staff.
    if strike {
        let mut x = orb.0 + 6;
        for y in 0..(ttop - 2).max(0) {
            cv.glyph(x, y, if hash(x, y + t as i32) % 2 == 0 { '/' } else { '\\' }, rgb((230, 255, 230)));
            x -= if y % 2 == 0 { 1 } else { 0 } + (hash(y, t as i32) % 2) as i32;
        }
    }

    // The dead, rising and walking.
    let nd = (w / 12).clamp(3, 16);
    let walk_end = hx - 3;
    for k in 0..nd {
        let bx = barrows[(k % nb) as usize] + (k / nb) % 3 - 1;
        let life = 420 + (hash(k + salt, 31) % 300) as u64;
        let a = (t + hash(k + salt, 33) as u64) % life;
        let bone = hash(k + salt, 35) % 3 == 0;
        let col = if bone { (225, 220, 200) } else { (150, 190, 140) };
        if a < 30 {
            if (t / 3 + k as u64) % 2 == 0 {
                cv.glyph(bx, g - 1, '░', rgb((90, 80, 60)));
            }
        } else if a < 70 {
            cv.glyph(bx, g - 1, if a < 50 { '¥' } else { '▄' }, rgb(col));
            // The staff's call reaching it.
            if ttop >= 1 {
                for (i, (px, py)) in line(orb, (bx, g - 2)).into_iter().enumerate() {
                    if (i as u64 + t / 2) % 4 == 0 {
                        cv.glyph(px, py, '·', rgb(magic));
                    }
                }
            }
            cv.halo(bx, g - 1, 1.5, magic, 0.4);
        } else {
            let step = ((a - 70) / 9) as i32;
            let x = bx + step;
            if x >= walk_end {
                continue;
            }
            let lurch = (a / 9 + k as u64) % 3 == 0;
            cv.glyph(x, g - 1, if bone { '☺' } else { '☻' }, rgb(col));
            cv.glyph(x + 1, g - 1 - lurch as i32, '─', rgb(lerp(col, (0, 0, 0), 0.2)));
        }
    }

    // Will-o'-wisps over the barrows; crows round the spire.
    for k in 0..(nb + 2) {
        let bx = barrows[(k % nb) as usize];
        let x = bx + ((tf * 0.03 + k as f32 * 1.7).sin() * (br as f32 + 2.0)) as i32;
        let y = g - 2 - bh - ((tf * 0.05 + k as f32).sin() * 1.5).abs() as i32;
        cv.glyph(x, y, '°', rgb(lerp(magic, (255, 255, 255), 0.5)));
        cv.halo(x, y, 1.2, magic, 0.3);
    }
    for k in 0..(if big { 6 } else { 3 }) {
        let a = tf * 0.04 + k as f32 * 2.1;
        let rr = 4.0 + (k % 3) as f32 * 2.0;
        let x = tc + (a.cos() * rr * 2.0) as i32;
        let y = ttop + 2 + (a.sin() * rr * 0.5) as i32;
        cv.glyph(x, y, if (t / 5 + k as u64) % 2 == 0 { 'v' } else { '^' }, rgb((20, 20, 24)));
    }
}
