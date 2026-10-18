//! The great dam: a wall of dressed stone holding back a mountain lake
//! above a farming valley. Leaks, then cracks, then jets, and then the
//! whole middle gives way and the flood goes roaring down the valley,
//! over the fields and the houses, while the farmers run for the hill.
//! The seed picks the weather, the valley's crops and houses, and how
//! long the dam holds.

use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the engineer said it would hold",
    "water always finds the weak stone",
    "run for the high ground and do not look back",
    "the lake remembers where the valley used to be",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let g = (cv.sky - 1).max(2).min(h - 1); // the valley floor
    let tf = t as f32;
    let salt = cv.salt();
    let storm = cv.roll(1) % 2 == 0;
    let (top, bot) = if storm { ((26, 28, 36), (80, 84, 96)) } else { ((50, 90, 160), (220, 170, 120)) };
    let big = w >= 110 && g >= 16;

    // How far through the dam's last day we are.
    let period = 1500u64;
    let local = (t + cv.roll(2) as u64 % period) % period;
    let cracking = local.saturating_sub(500).min(300) as f32 / 300.0;
    let burst = local >= 800 && local < 1450;
    let since = local.saturating_sub(800);

    // Sky.
    let flash = storm && t % 190 < 3;
    for y in 0..g {
        let base = lerp(top, bot, y as f32 / g.max(1) as f32);
        for x in 0..w {
            let cl = ((x as f32 * 0.05 + tf * 0.006 + y as f32 * 0.4).sin() + (x as f32 * 0.13 - tf * 0.004).sin()) * 0.5;
            let mut c = if cl > 0.3 { lerp(base, if storm { (40, 42, 50) } else { (250, 230, 220) }, (cl - 0.3) * 0.9) } else { base };
            if flash {
                c = lerp(c, (220, 225, 240), 0.5);
            }
            cv.fill(x, y, rgb(c));
        }
    }

    // Mountains behind; the dam closing a notch between them.
    let d0 = ((w as f32 * 0.1) as i32).max(3);
    let dw = (w / 30).clamp(2, 7);
    let d1 = d0 + dw;
    let dt = ((g as f32 * 0.25) as i32).max(1);
    for x in 0..w {
        let m = peaks(x, w, seed, 50, 4) * 0.9;
        let s = g - 1 - (m * (g as f32 * 0.8)) as i32;
        let s = if x >= d0 - 3 && x <= d1 + 3 { s.min(dt) } else { s };
        for y in s.max(0)..g {
            let snow = y - s < 1 && m > 0.6;
            let c = if storm { (44, 46, 54) } else { lerp(bot, (70, 66, 80), 0.6) };
            cv.put(x, y, if snow { '▲' } else { ' ' }, rgb((220, 225, 235)), Some(rgb(c)));
        }
    }

    // The valley: fields in strips, a lane, the refuge hill at the far end.
    let crops = [(150, 170, 60), (90, 150, 60), (200, 180, 80), (120, 110, 50)];
    let hill_w = (w / 7).clamp(5, 26);
    let hill_h = (g as f32 * 0.4).clamp(2.0, 10.0);
    let hill_top = |x: i32| -> i32 {
        let u = (w - 1 - x) as f32 / hill_w as f32;
        if u < 1.0 {
            g - (hill_h * (1.0 - u * u).sqrt().min(1.0)) as i32
        } else {
            g
        }
    };
    for x in 0..w {
        let s = hill_top(x);
        for y in s..g {
            cv.put(x, y, if y == s { '▄' } else { ' ' }, rgb((80, 130, 60)), Some(rgb((60, 90, 44))));
        }
    }
    for y in g..h {
        for x in 0..w {
            let strip = ((y - g) / 3 + (x / 14)) as usize % 4;
            let hs = hash(x + salt, y * 3);
            let lane = (y - g - (h - g) / 2).abs() == 0 && x > d1;
            let (ch, fg, bg) = if lane {
                ('·', (150, 130, 100), (110, 90, 60))
            } else {
                let c = crops[strip];
                (if hs % 3 == 0 { '"' } else if hs % 3 == 1 { ',' } else { ' ' }, lerp(c, (255, 255, 200), 0.2), lerp(c, (40, 40, 20), 0.55))
            };
            cv.put(x, y, ch, rgb(fg), Some(rgb(bg)));
        }
    }

    // How far the flood has come.
    let run = (w - d1) as f32;
    let front = if burst { d1 + (since as f32 * run / 320.0) as i32 } else { -1 };
    let depth = if burst { ((since as f32 / 120.0).min(1.0) * (g as f32 * 0.22).max(1.0)) as i32 } else { 0 };
    let level = |x: i32| -> i32 { (g - depth).max(hill_top(x)) };

    // Houses and folk in the valley.
    let nh = (w / 20).clamp(2, 9);
    let houses: Vec<i32> = (0..nh).map(|k| d1 + 6 + ((w - d1 - hill_w - 10) * k / nh.max(1)) + (hash(k, salt) % 4) as i32).collect();
    for (k, &hx) in houses.iter().enumerate() {
        if burst && hx < front {
            continue;
        }
        let roof = [(160, 60, 40), (120, 80, 50), (90, 90, 110)][k % 3];
        cv.glyph(hx, g - 1, '█', rgb((170, 150, 120)));
        cv.glyph(hx + 1, g - 1, '▌', rgb((170, 150, 120)));
        cv.glyph(hx, g - 2, '▄', rgb(roof));
        cv.glyph(hx + 1, g - 2, '▄', rgb(roof));
        cv.glyph(hx - 1, g - 1, '▐', rgb(roof));
        if !storm || (t / 9 + k as u64) % 5 != 0 {
            cv.glyph(hx + 1, g - 1, '▪', rgb((255, 210, 120)));
        }
    }
    let nf = (w / 12).clamp(3, 14);
    let hill_x = w - hill_w / 2 - 1;
    for k in 0..nf {
        let home = d1 + 4 + (hash(k, salt + 3) % ((w - d1 - hill_w - 4).max(1) as u32)) as i32;
        let col = [(150, 200, 240), (240, 170, 110), (200, 230, 150), (230, 150, 210)][(k % 4) as usize];
        let (x, y) = if burst {
            // Running for the hill, a little ahead of the water.
            let run_x = home + (since as i32 * 3 / 2) / (5 + k % 3);
            let x = run_x.max(front + 2).min(hill_x - (k % 5));
            (x, hill_top(x) - 1)
        } else {
            (home + pingpong(t, 11 + k as u64 % 5, -3, 3), g - 1)
        };
        if burst && x < front && hill_top(x) > g - depth {
            continue;
        }
        cv.glyph(x, y, '☺', rgb(col));
        if local >= 600 && local < 800 && (t / 6 + k as u64) % 2 == 0 {
            cv.glyph(x, y - 1, '!', rgb((255, 240, 160)));
        } else if !burst && k % 3 == 0 {
            cv.glyph(x + 1, y - 1, '/', rgb((150, 120, 80)));
        }
    }

    // The reservoir behind the dam, falling once it breaks.
    let full = dt + 1;
    let wl = if burst { full + ((since as f32 / 500.0).min(1.0) * (g - depth - full - 1).max(0) as f32) as i32 } else { full };
    let water = |x: i32, y: i32, rough: bool| -> (char, Rgb, Rgb) {
        if !rough {
            let ch = if (x + y * 3 + (t / 6) as i32) % 7 == 0 { '~' } else { ' ' };
            return (ch, (170, 200, 230), (40, 70, 110));
        }
        let (fx, fy) = (x as f32, y as f32);
        let swirl = ((fx * 0.15 - tf * 0.12 + fy * 0.5).sin() + (fx * 0.05 + fy * 1.1 - tf * 0.05).sin()) * 0.5;
        let bg = lerp((70, 92, 112), (128, 112, 82), swirl * 0.5 + 0.5);
        let streak = (x - (t / 2) as i32 + y * 5).rem_euclid(11);
        let (ch, fg) = match streak {
            0 => ('~', (225, 235, 245)),
            1 => ('─', (190, 205, 220)),
            5 if swirl > 0.3 => ('≈', (235, 240, 248)),
            _ => (' ', (200, 210, 220)),
        };
        (ch, fg, bg)
    };
    for y in wl.max(0)..h {
        for x in 0..d0 {
            let (ch, fg, bg) = water(x, y, false);
            cv.put(x, y, ch, rgb(fg), Some(rgb(bg)));
        }
    }

    // The dam.
    let breach = if burst { ((since as f32 / 30.0).min(1.0) * (g - dt) as f32 * 0.8) as i32 } else { 0 };
    let bw0 = d0 + dw / 4;
    let bw1 = d1 - dw / 4;
    for y in dt..h {
        for x in d0..=d1 {
            let gone = burst && y >= dt && y < dt + breach + 1 && x >= bw0 && x <= bw1;
            if gone {
                continue;
            }
            let course = y % 2 == 0;
            let joint = (x + if course { 0 } else { 2 }) % 4 == 0;
            let ch = if y == dt { '▀' } else if joint { '│' } else { '▄' };
            cv.put(x, y, ch, rgb((96, 92, 88)), Some(rgb((132, 126, 116))));
        }
    }
    // Cracks spreading over its face, and the jets they let out.
    let nc = ((g - dt) * dw) as f32 * cracking * 0.6;
    if !burst {
        for k in 0..nc as i32 {
            let x = d0 + (hash(k, salt + 7) % (dw + 1).max(1) as u32) as i32;
            let y = dt + 1 + (hash(k, salt + 9) % (g - dt).max(1) as u32) as i32;
            cv.glyph(x, y, ['╱', '╲', '┼', '╳'][(hash(k, 3) % 4) as usize], rgb((40, 36, 34)));
        }
        let jets = (cracking * 6.0) as i32 + 1;
        for k in 0..jets {
            let y = dt + 2 + (hash(k, salt + 11) % (g - dt - 1).max(1) as u32) as i32;
            let len = 2 + (cracking * 10.0) as i32 + (k % 3);
            for i in 0..len {
                let x = d1 + 1 + i;
                let yy = y + i * i / (len * 2).max(1);
                if (i + (t / 2) as i32) % 3 != 0 {
                    cv.glyph(x, yy, '·', rgb((200, 225, 245)));
                }
            }
        }
    } else {
        // Water pouring through the breach.
        for y in wl.max(dt)..g {
            for x in bw0..=bw1 {
                let hs = hash(x, y - (t as i32) / 2);
                cv.put(x, y, if hs % 2 == 0 { '║' } else { '│' }, rgb((240, 245, 255)), Some(rgb((150, 180, 210))));
            }
        }
        // Stones flung out as it gives.
        if since < 70 {
            for k in 0..(if big { 14 } else { 8 }) {
                let a = since as f32;
                let vx = 0.3 + (hash(k, 1) % 100) as f32 / 120.0;
                let vy = 0.4 + (hash(k, 2) % 100) as f32 / 150.0;
                let x = d1 + (vx * a) as i32;
                let y = dt + 2 - (vy * a - 0.02 * a * a) as i32;
                cv.glyph(x, y, '■', rgb((120, 116, 108)));
            }
        }
    }

    // The flood itself.
    if burst {
        for x in d1 + 1..front.min(w) {
            let s = g - depth;
            if hill_top(x) <= s {
                continue;
            }
            for y in s..h {
                let (ch, fg, bg) = water(x, y, true);
                cv.put(x, y, ch, rgb(fg), Some(rgb(bg)));
            }
        }
        // The roaring white front.
        if front < w {
            for y in (level(front) - 2).max(0)..h {
                for dx in 0..3 {
                    let x = front - dx;
                    let hs = hash(x, y + (t / 2) as i32);
                    if y < level(front) && hs % 2 == 0 {
                        continue;
                    }
                    cv.put(x, y, if hs % 2 == 0 { '≈' } else { '░' }, rgb((250, 252, 255)), Some(rgb((190, 210, 230))));
                }
            }
            for k in 0..6 {
                let age = ((t / 2) as i32 + k * 3) % 9;
                cv.glyph(front + 1 + age / 2, level(front) - 2 - (age * (9 - age)) / 10, '·', rgb((235, 242, 255)));
            }
        }
        // What the water carries: planks, barrels, a roof, someone clinging on.
        for k in 0..((front.min(w) - d1) / 8).max(0) {
            let x = d1 + 3 + ((hash(k, 51) as u64 + since / (2 + k as u64 % 3)) % (front.min(w) - d1 - 3).max(1) as u64) as i32;
            if hill_top(x) <= g - depth {
                continue;
            }
            let y = g - depth + (hash(k, 53) % ((h - g + depth).max(1) as u32)) as i32;
            let bob = ((tf * 0.1 + k as f32).sin() * 0.6).round() as i32;
            let wood = rgb((140, 96, 52));
            match hash(k, 55) % 5 {
                0 => cv.sprite(x, y + bob, &["▬▬▬"], wood),
                1 => cv.glyph(x, y + bob, 'Θ', wood),
                2 => cv.sprite(x, y + bob, &["▄▄"], rgb((160, 60, 40))),
                3 => {
                    cv.sprite(x, y + bob, &["▬▬"], wood);
                    cv.glyph(x + 1, y + bob - 1, '☺', rgb((240, 200, 140)));
                }
                _ => cv.glyph(x, y + bob, '═', wood),
            }
        }
    }

    // Rain in the storm.
    if storm {
        for k in 0..(w * h / 50).max(4) {
            let y = ((hash(k, 2) as u64 + t) % h.max(1) as u64) as i32;
            let x = ((hash(k, 1) as u64 + t / 3) % w.max(1) as u64) as i32 - y / 3;
            cv.glyph(x.rem_euclid(w.max(1)), y, '/', rgb((130, 145, 175)));
        }
        if flash {
            let mut x = (hash((t / 190) as i32, 3) % w.max(1) as u32) as i32;
            for y in 0..(g / 2) {
                cv.glyph(x, y, if hash(x, y) % 2 == 0 { '/' } else { '\\' }, rgb((245, 245, 255)));
                x += if hash(y, (t / 190) as i32) % 2 == 0 { 1 } else { -1 };
            }
        }
    }
}
