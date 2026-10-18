//! The gorge: a long stone bridge on tall piers across a chasm, a
//! waterfall thundering down one wall into the mist, swifts wheeling far
//! below the deck, and a column of dwarves marching over under their
//! banners, spears up. The seed picks the hour, the banners, the size of
//! the column and which way it marches.

use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the bridge has stood nine hundred years; mind the gap",
    "march on, and do not look down",
    "the river cut this road; the dwarves only paved it",
    "every banner a hold, every hold a promise",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let bd = (cv.sky - 1).max(2).min(h - 1); // the bridge deck
    let tf = t as f32;
    let salt = cv.salt();
    let hour = cv.roll(1) % 4; // dawn, day, dusk, night
    let (top, bot) = [((70, 60, 120), (240, 170, 130)), ((70, 130, 210), (180, 215, 240)), ((40, 26, 80), (230, 120, 80)), ((6, 8, 24), (30, 34, 64))][hour as usize];
    let night = hour == 3;
    let banner = [(200, 40, 40), (40, 80, 180), (220, 180, 50), (40, 140, 70), (140, 60, 160)][(cv.roll(2) % 5) as usize];
    let big = w >= 110 && bd >= 16;

    // Sky and the far mountains seen down the gorge.
    cv.sky_fill(bd, top, bot);
    if night {
        cv.stars(bd - 2, 31);
        cv.disc((w as f32 * 0.8) as i32, (bd as f32 * 0.25) as i32 + 1, (bd as f32 * 0.08).clamp(0.8, 2.5), (240, 236, 210));
    }
    for x in 0..w {
        let m = peaks(x, w, seed, 20, 4);
        let s = bd - 1 - (m * (bd as f32 * 0.55)) as i32;
        let far = lerp(bot, (60, 64, 90), 0.45);
        for y in s.max(0)..bd {
            let snow = y - s < 1 + (m * 2.0) as i32 && m > 0.5;
            cv.put(x, y, if y == s { '▲' } else { ' ' }, rgb(lerp(far, (255, 255, 255), 0.5)), Some(rgb(if snow { lerp(far, (240, 240, 250), 0.6) } else { far })));
        }
    }

    // The two sides of the gorge, and the land above each rim.
    let l = ((w as f32 * 0.14) as i32).max(3);
    let r = w - l;
    let rimh = (bd as f32 * 0.45).max(1.0);
    let land = |x: i32| -> i32 {
        if x < l {
            bd - (rimh * (1.0 - x as f32 / l as f32).powf(0.6)) as i32
        } else if x >= r {
            bd - (rimh * ((x - r) as f32 / l as f32).powf(0.6)) as i32
        } else {
            bd + 1
        }
    };
    let wall_l = |y: i32| -> i32 { l + ((y - bd) as f32 * 0.35) as i32 + (hash(y, salt) % 2) as i32 };
    let wall_r = |y: i32| -> i32 { r - ((y - bd) as f32 * 0.35) as i32 - (hash(y, salt + 1) % 2) as i32 };
    let rock = if night { (34, 32, 40) } else { lerp(bot, (70, 60, 56), 0.7) };
    let rock_dark = lerp(rock, (0, 0, 0), 0.35);
    for x in 0..w {
        let s = land(x);
        if x < l || x >= r {
            for y in s.max(0)..bd + 1 {
                let ch = if y == s { '▄' } else if hash(x + salt, y) % 6 == 0 { '░' } else { ' ' };
                let fg = if y == s { if hour == 3 { (40, 70, 50) } else { (70, 130, 60) } } else { lerp(rock, (0, 0, 0), 0.2) };
                cv.put(x, y, ch, rgb(fg), Some(rgb(rock)));
            }
            if hash(x + salt, 77) % 4 == 0 {
                cv.glyph(x, s - 1, '♠', rgb(if night { (20, 40, 30) } else { (30, 90, 50) }));
            }
        }
    }
    // Down into the chasm: the walls closing in, mist, the river.
    let river_y = h - 2;
    for y in (bd + 1).max(0)..h {
        let (wl, wr) = (wall_l(y), wall_r(y));
        let depth = ((y - bd) as f32 / (h - bd).max(1) as f32).min(1.0);
        let air = lerp(lerp(bot, (200, 210, 220), 0.3), lerp(rock_dark, (0, 0, 0), 0.4), depth);
        for x in 0..w {
            if x < wl || x > wr {
                let face = (x - wl).abs() < 2 || (x - wr).abs() < 2;
                let ch = if hash(x * 3 + salt, y * 5) % 7 == 0 { '▒' } else { ' ' };
                let c = lerp(if face { rock } else { rock_dark }, (0, 0, 0), depth * 0.5);
                cv.put(x, y, ch, rgb(lerp(c, (0, 0, 0), 0.25)), Some(rgb(c)));
            } else if y >= river_y {
                let ch = if (x + y + (t / 3) as i32) % 4 == 0 { '~' } else { '≈' };
                cv.put(x, y, ch, rgb((200, 220, 240)), Some(rgb(lerp((30, 70, 110), (0, 0, 0), 0.2))));
            } else {
                cv.fill(x, y, rgb(air));
            }
        }
    }

    // The waterfall down the far wall.
    let fw = (w / 45).clamp(1, 5);
    let lip = land(r + 1).max(1);
    let wf = r + 1;
    for x in wf..wf + fw {
        for y in lip..h {
            let hs = hash(x * 7, y - (t / 2) as i32);
            let col = lerp((150, 190, 230), (240, 248, 255), (hs % 50) as f32 / 50.0);
            cv.put(x, y, if hs % 3 == 0 { '║' } else { '│' }, rgb(col), Some(rgb(lerp(col, (60, 110, 160), 0.5))));
        }
    }
    let mist_at = (wf + fw / 2, river_y);
    let mr = (fw as f32 * 1.5 + 3.0).min(10.0) + if big { 3.0 } else { 0.0 };
    cv.halo(mist_at.0, mist_at.1, mr, (230, 236, 245), 0.55);
    // A rainbow in the spray when the sun is up.
    if hour == 1 || hour == 0 {
        let bands = [(255, 90, 90), (255, 200, 90), (130, 230, 130), (110, 150, 255)];
        let r0 = mr * 1.1;
        let reach = (r0 + 4.0).ceil() as i32;
        for y in (mist_at.1 - reach).max(bd + 1)..mist_at.1 {
            let (wl, wr) = (wall_l(y), wall_r(y));
            for x in (mist_at.0 - reach * 2)..=(mist_at.0 + reach * 2) {
                if x <= wl || x >= wr {
                    continue;
                }
                let (dx, dy) = ((x - mist_at.0) as f32 * 0.5, (y - mist_at.1) as f32);
                let band = ((dx * dx + dy * dy).sqrt() - r0).floor();
                if (0.0..4.0).contains(&band) {
                    cv.glow(x, y, bands[band as usize], 0.3);
                }
            }
        }
    }
    for k in 0..8 {
        let age = ((t / 3) as i32 + k * 5) % 16;
        let x = mist_at.0 + ((k as f32 * 1.7 + age as f32 * 0.3).sin() * mr) as i32;
        cv.glyph(x, mist_at.1 - age / 3, '░', rgb((235, 240, 250)));
    }

    // Swifts wheeling deep in the gorge.
    for k in 0..(if big { 6 } else { 3 }) {
        let a = tf * 0.03 + k as f32 * 1.9;
        let cx = (l + r) / 2 + ((k as f32 * 3.1).sin() * (r - l) as f32 * 0.2) as i32;
        let cy = bd + 3 + (h - bd) / 3;
        let x = cx + (a.cos() * (r - l) as f32 * 0.18) as i32;
        let y = cy + (a.sin() * 2.0) as i32;
        cv.glyph(x, y, if (t / 4 + k as u64) % 2 == 0 { 'v' } else { '^' }, rgb((30, 30, 36)));
    }

    // The bridge: deck, arches and tall piers.
    let stone = if night { (70, 66, 72) } else { lerp(bot, (150, 140, 128), 0.75) };
    let shade = lerp(stone, (0, 0, 0), 0.3);
    let span = (w / 7).clamp(8, 26);
    let ah = if big { 3 } else { 2 };
    for x in (l - 2)..=(r + 1) {
        cv.put(x, bd, '▀', rgb(lerp(stone, (255, 255, 255), 0.25)), Some(rgb(stone)));
        let into = (x - l).rem_euclid(span);
        let pier = into <= if big { 2 } else { 1 } && x > l + 1 && x < r - 1;
        if pier {
            for y in bd + 1..river_y {
                let ch = if (y + x) % 3 == 0 { '▒' } else { ' ' };
                cv.put(x, y, ch, rgb(lerp(shade, (0, 0, 0), 0.2)), Some(rgb(lerp(shade, (0, 0, 0), ((y - bd) as f32 / (h - bd).max(1) as f32) * 0.4))));
            }
            if (x - l).rem_euclid(span) == 0 && bd >= 2 {
                cv.glyph(x, bd - 1, '▄', rgb(stone));
            }
        } else {
            let u = into as f32 / span as f32 * 2.0 - 1.0;
            let dip = (ah as f32 * (1.0 - (1.0 - u * u).max(0.0).sqrt())).round() as i32;
            for y in bd + 1..=bd + dip.max(0) {
                cv.put(x, y, ' ', Color::Black, Some(rgb(shade)));
            }
        }
    }

    // The column on the march.
    let n = ((r - l) / 4).clamp(5, 36) + (cv.roll(3) % 6) as i32;
    let road = (w + n * 2 + 20) as u64;
    let head = ((t / 4 + cv.roll(4) as u64) % road) as i32 - n * 2 + w / 3;
    let armour = [(180, 190, 205), (200, 170, 110), (160, 200, 230)][(cv.roll(5) % 3) as usize];
    for k in 0..n {
        let x = head - k * 2;
        if x < 0 || x >= w {
            continue;
        }
        let y = if x >= l - 2 && x <= r + 1 { bd - 1 } else { land(x) - 1 };
        let step = ((t / 8) as i32 + k) % 2;
        let bearer = k % 6 == 1;
        cv.glyph(x, y, '☺', rgb(if bearer { banner } else { armour }));
        if bearer {
            cv.glyph(x + 1, y - 1, '│', rgb((140, 100, 60)));
            cv.glyph(x + 1, y - 2, '│', rgb((140, 100, 60)));
            let flap = (t / 5 + k as u64) % 2 == 0;
            cv.glyph(x, y - 2, if flap { '◄' } else { '◀' }, rgb(banner));
        } else if k % 2 == 0 {
            cv.glyph(x + step, y - 1, '↑', rgb((190, 190, 200)));
        }
        if night && k % 4 == 0 {
            cv.glyph(x - 1, y - 1, '¡', rgb((255, 200, 90)));
            cv.halo(x - 1, y - 1, 2.0, (255, 170, 70), 0.4);
        }
    }
}
