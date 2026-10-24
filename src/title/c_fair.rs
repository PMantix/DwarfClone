//! Fair day in a market town: timbered houses leaning over the square,
//! bunting swinging between the gables, striped stalls heaped with
//! apples, cloth and trinkets, the crowd milling from one to the next, a
//! juggler keeping his balls in the air, and children's kites tugging
//! high over the roofs. The seed picks the awnings, the wares, the
//! houses and the weather.

use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "three coppers for the pie, one for the gossip",
    "the whole valley comes to town on fair day",
    "watch your purse, and watch the juggler",
    "all the world's goods, and some of its rogues",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let tf = t as f32;
    let sky = cv.sky.min(h - 1).max(1);
    let g = (sky - 1).max(1); // the square's front row
    let salt = cv.salt();
    let breezy = cv.roll(1) % 2 == 0;
    let (top, bot) = if breezy { ((70, 140, 220), (190, 225, 245)) } else { ((110, 150, 200), (240, 220, 180)) };
    cv.sky_fill(g, top, bot);
    for k in 0..(2 + w / 40) {
        let len = 6 + (hash(k, 9) % 8) as i32;
        let y = 1 + (hash(k, 8) % (g.max(4) as u32 / 3).max(1)) as i32;
        let x0 = ((hash(k, 7) as u64 + t / (12 + k as u64 * 4)) % (w as u64 + len as u64 + 10)) as i32 - len - 5;
        for i in 0..len {
            cv.glyph(x0 + i, y, if i == 0 || i == len - 1 { '▄' } else { '█' }, rgb((250, 250, 250)));
        }
    }

    // The houses round the square, timbered and gabled.
    let houses_base = g - 3; // their ground floor is hidden by the stalls
    let mut gables: Vec<(i32, i32)> = Vec::new();
    let mut x = -((hash(salt, 1) % 4) as i32);
    let mut k = 0;
    while x < w {
        let wd = 5 + (hash(k, salt) % 4) as i32;
        let floors = 2 + (hash(k, 2) % 2) as i32 + (sky / 14).min(2);
        let top_y = houses_base - floors * 2;
        let plaster = [(230, 220, 190), (220, 200, 170), (200, 210, 200), (235, 205, 180)][(hash(k, 3) % 4) as usize];
        let roof = [(160, 60, 40), (120, 70, 50), (80, 80, 100), (150, 90, 50)][(hash(k, 4) % 4) as usize];
        for y in top_y..=g {
            for dx in 0..wd {
                let beam = dx == 0 || dx == wd - 1 || (y - top_y) % 2 == 0;
                let window = !beam && dx % 2 == 0 && (y - top_y) % 2 == 1 && y < houses_base;
                let (ch, fg) = if window { ('▪', (60, 80, 110)) } else if beam { ('▒', (80, 56, 40)) } else { (' ', (0, 0, 0)) };
                cv.put(x + dx, y, ch, rgb(fg), Some(rgb(if beam { lerp(plaster, (80, 56, 40), 0.55) } else { plaster })));
            }
        }
        // The gable, stepped up to a point.
        let peak = wd / 2;
        for dx in 0..wd {
            let rise = peak - (dx - peak).abs();
            for r in 0..=rise {
                cv.fill(x + dx, top_y - 1 - r, rgb(roof));
            }
            cv.glyph(x + dx, top_y - 2 - rise, '▄', rgb(roof));
        }
        gables.push((x + peak, top_y - 2 - peak));
        x += wd;
        k += 1;
    }

    // Bunting between the gables, swinging.
    let flags = [(220, 50, 50), (250, 210, 60), (50, 110, 210), (60, 170, 80), (240, 240, 240)];
    for (j, pair) in gables.windows(2).enumerate() {
        let ((ax, ay), (bx, by)) = (pair[0], pair[1]);
        let len = (bx - ax).max(1);
        let swing = (tf * 0.08 + j as f32).sin() * 0.6;
        for i in 0..=len {
            let u = i as f32 / len as f32;
            let y = ay as f32 + (by - ay) as f32 * u + (u * std::f32::consts::PI).sin() * (1.5 + swing);
            if i % 2 == 1 {
                cv.glyph(ax + i, y.round() as i32, '▼', rgb(flags[((i as usize / 2) + j) % 5]));
            }
        }
    }

    // The stalls along the front, striped awnings and heaped wares.
    let awnings = [((220, 50, 50), (245, 240, 230)), ((40, 90, 190), (245, 240, 230)), ((60, 150, 70), (250, 220, 90)), ((150, 60, 150), (250, 200, 120))];
    let wares = [('o', (220, 40, 30)), ('o', (120, 200, 60)), ('♦', (100, 220, 240)), ('§', (220, 160, 220)), ('•', (240, 190, 60)), ('ô', (190, 140, 90))];
    let mut sx = 1 + (hash(salt, 5) % 4) as i32;
    let mut s = 0;
    while sx < w - 4 {
        let wd = 5 + (hash(s, salt + 1) % 3) as i32;
        let (a, b) = awnings[(hash(s, salt + 2) % 4) as usize];
        for dx in 0..wd {
            let stripe = if dx % 2 == 0 { a } else { b };
            cv.put(sx + dx, g - 3, '▄', rgb(stripe), Some(rgb(lerp(stripe, (0, 0, 0), 0.15))));
            cv.put(sx + dx, g - 2, ' ', Color::Black, Some(rgb((50, 36, 28))));
            let (ch, col) = wares[(hash(s * 7 + dx, salt) % 6) as usize];
            cv.put(sx + dx, g - 1, ch, rgb(col), Some(rgb((120, 84, 50))));
        }
        cv.glyph(sx + wd / 2, g - 2, '☺', rgb((240, 210, 170)));
        sx += wd + 3 + (hash(s, 9) % 4) as i32;
        s += 1;
    }

    // The cobbled square and the crowd.
    for y in g.max(0)..h {
        for x in 0..w {
            let hs = hash(x + salt, y * 3);
            cv.put(x, y, if hs % 4 == 0 { '·' } else { ' ' }, rgb((150, 140, 130)), Some(rgb(if (x + y) % 2 == 0 { (112, 104, 98) } else { (104, 96, 92) })));
        }
    }
    let folk = [(200, 60, 60), (60, 90, 180), (220, 190, 70), (70, 150, 80), (160, 100, 170), (230, 230, 230), (170, 120, 70)];
    for k in 0..(6 + w / 6) {
        let speed = 4 + (hash(k, 11) % 5) as u64;
        let span = (w + 10) as u64;
        let x0 = hash(k, 12) as u64 % span;
        let dir = if hash(k, 13) % 2 == 0 { 1 } else { -1 };
        // They wander, stop at a stall a while, and wander on.
        let walked = t / speed;
        let paused = (walked / 40) % 3 == 2;
        let pos = (walked - (walked / 120) * 40 - if paused { walked % 40 } else { 0 }) as i64;
        let x = ((x0 as i64 + pos * dir).rem_euclid(span as i64)) as i32 - 5;
        let y = g + (hash(k, 14) % 2) as i32;
        cv.glyph(x, y, '☺', rgb(folk[(hash(k, 15) % 7) as usize]));
    }

    // The juggler, and the balls going up and round.
    let jx = (w as f32 * 0.12) as i32 + 1;
    cv.glyph(jx, g, '☺', rgb((240, 60, 140)));
    let ball = [(255, 80, 80), (80, 200, 255), (255, 230, 80)];
    for k in 0..3 {
        let a = tf * 0.18 + k as f32 * 2.094;
        let x = jx + (a.cos() * 1.5).round() as i32;
        let y = g - 2 - (a.sin().max(-0.2) * 2.0).round() as i32;
        cv.glyph(x, y, '•', rgb(ball[k as usize]));
    }

    // Kites tugging high above the roofs, their tails waving.
    for k in 0..(1 + w / 60).min(3) {
        let ax = (w as f32 * (0.15 + 0.35 * k as f32 + (hash(k, seed as i32) % 10) as f32 / 100.0)) as i32;
        let x = ax + ((tf * 0.03 + k as f32 * 2.0).sin() * 3.0) as i32;
        let y = 2 + k % 2 + ((tf * 0.05 + k as f32).sin() * 1.0).round() as i32;
        let col = flags[(k as usize * 2 + 1) % 5];
        cv.glyph(x, y, '♦', rgb(col));
        for i in 1..5 {
            let wob = ((tf * 0.2 - i as f32 * 0.8 + k as f32).sin() * 1.0).round() as i32;
            cv.glyph(x - i / 2 + wob, y + i, if i % 2 == 0 { '~' } else { '·' }, rgb(flags[(k as usize + i as usize) % 5]));
        }
    }

    // Pigeons going up off the square and settling again.
    let flap = (t + cv.roll(2) as u64 % 300) % 300;
    for k in 0..4 {
        let px = (w as f32 * 0.85) as i32 + k * 2;
        let y = if flap < 60 { g - (flap as i32 * (k + 2)) / 20 } else { g };
        cv.glyph(px + if flap < 60 { flap as i32 / 6 } else { 0 }, y, if flap < 60 && (t / 3) % 2 == 0 { 'v' } else { ',' }, rgb((140, 140, 150)));
    }
}
