//! A caravan crossing a high pass in a snowstorm: two great shoulders of
//! mountain with a saddle between, a track marked by cairns and poles
//! climbing over it, and a line of roped dwarves and laden mules leaning
//! into the wind, lanterns swinging, while gusts of snow blot them out
//! and give them back. The seed picks the light, the pass and the beasts.

use super::a_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the pass is open, the gods alone know for how long",
    "follow the poles; the snow lies about the rest",
    "rope up, heads down, one more cairn",
    "the cold is a toll every trader pays",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let g = (cv.sky - 1).max(1);
    let tf = t as f32;
    let salt = cv.salt();
    let dusk = cv.roll(1) % 2 == 0;
    let (top, bot) = if dusk { ((50, 56, 80), (150, 140, 160)) } else { ((120, 130, 150), (200, 205, 214)) };
    let beast = [('M', (180, 160, 130)), ('h', (150, 120, 90)), ('Ö', (200, 190, 175))][(cv.roll(2) % 3) as usize];
    // How hard it is blowing: gusts come and go.
    let gust = ((tf * 0.013 + (cv.roll(3) % 100) as f32).sin() * 0.5 + 0.5).powi(2);

    cv.sky_fill(g, top, bot);

    // Two shoulders of mountain and the saddle between them.
    let saddle = (w as f32 * (0.45 + (cv.roll(4) % 15) as f32 / 100.0)) as i32;
    let low = g - (g as f32 * 0.35) as i32; // the top of the pass
    let rise = g as f32 * 0.95;
    let surf: Vec<i32> = (0..w)
        .map(|x| {
            let u = ((x - saddle) as f32 / (w as f32 * 0.5).max(1.0)).abs();
            let shoulder = (u * 1.6).min(1.0) * (0.6 + 0.4 * peaks(x, w, seed, 120, 3));
            (g - (shoulder * rise) as i32).min(low + (u * 4.0) as i32).max(0)
        })
        .collect();
    for x in 0..w {
        let s = surf[x as usize];
        for y in s..g {
            let d = (y - s) as f32;
            let rock = d > 1.0 && (x as f32 * 0.31 + y as f32 * 0.9).sin() + (x as f32 * 0.13 - y as f32 * 0.5 + 2.0).sin() > 1.25 + (hash(x + salt, y) % 10) as f32 * 0.03;
            let c = if rock { lerp((90, 90, 100), bot, 0.3) } else { lerp((236, 238, 244), (196, 202, 216), (d / 6.0).min(1.0)) };
            cv.put(x, y, if y == s { '▲' } else if rock && hash(x, y) % 3 == 0 { '▒' } else { ' ' }, rgb(if rock { (70, 70, 80) } else { (250, 250, 255) }), Some(rgb(lerp(c, bot, 0.15))));
        }
    }
    // The snowfield below.
    for y in g.max(0)..h {
        for x in 0..w {
            let drift_ = ((x as f32 * 0.09 + y as f32 * 0.8).sin() * 0.5 + 0.5) * 0.25;
            let ch = if hash(x * 3 + salt, y) % 31 == 0 { '·' } else { ' ' };
            cv.put(x, y, ch, rgb((170, 176, 196)), Some(rgb(lerp((224, 228, 238), (180, 188, 206), drift_ + (y - g) as f32 / (h - g).max(1) as f32 * 0.3))));
        }
    }

    // The track: up from the near left over the saddle and down the far side.
    let up = (w / 12).max(1);
    let points = [(-2, g), (up, g - 1), (saddle - (saddle - up) / 2, (low + g) / 2), (saddle, low - 1), (saddle + (w - saddle) / 2, (low + g) / 2), (w + 2, g - 1)];
    let mut road: Vec<(i32, i32)> = Vec::new();
    for pair in points.windows(2) {
        for c in line(pair[0], pair[1]) {
            if road.last() != Some(&c) {
                road.push(c);
            }
        }
    }
    for (i, &(x, y)) in road.iter().enumerate() {
        cv.put(x, y, if i % 3 == 0 { '·' } else { ' ' }, rgb((150, 150, 170)), Some(rgb((206, 210, 222))));
        // Marker poles every so often, and a cairn at the top.
        if i % 9 == 4 {
            cv.glyph(x, y - 1, '│', rgb((60, 50, 40)));
            cv.glyph(x, y - 2, '▪', rgb((200, 60, 50)));
        }
    }
    cv.sprite(saddle - 1, low - 3, &[" ▄ ", "▄█▄"], rgb((110, 106, 110)));

    // The caravan, strung out along the track.
    let mut train: Vec<(char, Rgb)> = vec![('☺', (210, 80, 70)), ('¡', (255, 210, 110))];
    for k in 0..(3 + cv.roll(5) % 3) {
        train.push((beast.0, beast.1));
        train.push(('▄', [(160, 120, 80), (120, 140, 170), (170, 80, 70)][(k % 3) as usize]));
        train.push(('☺', COATS[(k as usize * 3 + salt as usize) % 8]));
        train.push((' ', (0, 0, 0)));
    }
    train.push(('☺', (90, 140, 200)));
    let len = road.len() as u64;
    let cycle = len + train.len() as u64 * 2 + 40;
    let head = ((t + seed % 991) / 6) % cycle.max(1);
    for (j, &(ch, col)) in train.iter().enumerate() {
        let idx = head as i64 - j as i64;
        if ch == ' ' || idx < 0 || idx >= len as i64 {
            continue;
        }
        let (x, y) = road[idx as usize];
        let y = if ch == '▄' || ch == '¡' { y - 1 } else { y };
        let x = if ch == '▄' { x + 1 } else { x };
        cv.glyph(x, y, ch, rgb(col));
        if ch == '¡' {
            let swing = ((tf * 0.2).sin() * 0.3 + 0.7) * (1.0 - gust * 0.6);
            cv.halo(x, y, 2.5, (255, 190, 90), swing * 0.5);
        }
    }

    // Whiteout: banks of blown snow sweeping across with the gusts.
    for k in 0..5 {
        let row = (hash(k + salt, 3) % h.max(1) as u32) as i32;
        let len = (w / 3).max(6);
        let x0 = drift(t, k + salt, 1, w + len) - len / 2;
        let tall = 2 + (h / 12);
        for dy in -tall..=tall {
            for i in 0..len {
                let f = (i as f32 / len as f32 * std::f32::consts::PI).sin() * (1.0 - (dy as f32 / (tall + 1) as f32).abs());
                cv.glow(x0 + i, row + dy, (236, 238, 244), f * gust * 0.7);
            }
        }
    }
    // Snow driven sideways.
    let flakes = ((w * h) as f32 / 10.0 * (0.4 + gust)) as i32;
    for k in 0..flakes.max(6) {
        let speed = 1 + (k % 3) as u64;
        let x = ((hash(k + salt, 1) as u64 + t * 2 / speed) % (w.max(1) as u64)) as i32;
        let y = ((hash(k + salt, 2) as u64 + t / (speed * 2)) % h.max(1) as u64) as i32 + ((x as f32 * 0.2 + k as f32).sin() * 1.2) as i32;
        cv.glyph(x, y, if k % 4 == 0 { '*' } else if gust > 0.5 { '─' } else { '·' }, rgb((246, 248, 255)));
    }
}
