//! The lighthouse in the storm: a tall striped tower on its headland, the
//! lamp turning and the beam sweeping through the rain, the sea heaving
//! and hurling itself at the rocks in great bursts of spray, lightning
//! splitting the clouds, and a small boat fighting its way home by the
//! light. The seed picks the tower's paint, the storm and the boat.

use super::*;
use super::c_kit::*;

pub(super) const TAGLINES: &[&str] = &[
    "keep the lamp lit, whatever comes in off the sea",
    "every wreck on this coast is a story told twice",
    "the keeper counts the boats out and counts them in",
    "a light in the dark is the oldest promise there is",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let tf = t as f32;
    let sky = cv.sky.min(h - 1).max(1);
    let sea = (sky - 2).max(1).min(h - 1); // the mean sea level
    let salt = cv.salt();
    let flash_n = (t + cv.roll(1) as u64 % 210) / 210;
    let flash = (t + cv.roll(1) as u64 % 210) % 210 < 4;
    let paint = [(200, 40, 40), (30, 40, 60), (40, 90, 60), (200, 150, 40)][(cv.roll(2) % 4) as usize];

    // Clouds rolling over in the dark.
    for y in 0..(sea + 3).min(h) {
        for x in 0..w {
            let n = fbm(x as f32 * 0.05 - tf * 0.01, y as f32 * 0.11 + tf * 0.002, salt, 4);
            let mut c = lerp((14, 16, 24), (64, 70, 86), ((n - 0.3) * 1.8).max(0.0));
            c = lerp(c, (30, 34, 44), y as f32 / sea.max(1) as f32 * 0.5);
            if flash {
                c = lerp(c, (210, 215, 240), 0.55);
            }
            cv.fill(x, y, rgb(c));
        }
    }

    // The headland, and the tower on it.
    let cw = ((w as f32 * 0.24) as i32).max(6);
    let rock_top = |x: i32| -> i32 { sea - 1 - ((1.0 - x as f32 / cw as f32).max(0.0).powf(0.7) * sky as f32 * 0.2) as i32 + (hash(x, 9) % 2) as i32 };
    for x in 0..cw + 2 {
        let s = rock_top(x);
        for y in s.max(0)..h {
            let ch = if hash(x + salt, y) % 5 == 0 { '▒' } else { ' ' };
            cv.put(x, y, ch, rgb((40, 40, 46)), Some(rgb(if flash { (90, 90, 100) } else { (28, 28, 34) })));
        }
    }
    let big = w >= 110 && sky >= 16;
    let tw = if big { 2 } else { 1 }; // half-width
    let tx = (cw as f32 * 0.4) as i32;
    let base = rock_top(tx).min(rock_top(tx - tw)).min(rock_top(tx + tw));
    let tall = ((sky as f32 * 0.62) as i32).min(base - 3).max(2);
    for k in 0..tall {
        let y = base - 1 - k;
        let band = (k / 2) % 2 == 0;
        let col = if band { (230, 228, 220) } else { paint };
        let shade = lerp(col, (0, 0, 0), 0.45);
        for dx in -tw..=tw {
            cv.fill(tx + dx, y, rgb(if dx == tw { shade } else { col }));
        }
        if k % 4 == 2 {
            cv.put(tx, y, '▪', rgb((255, 220, 120)), Some(rgb(col)));
        }
    }
    let gal = base - 1 - tall;
    for dx in -tw - 1..=tw + 1 {
        cv.glyph(tx + dx, gal, '▄', rgb((60, 60, 70)));
    }
    let lamp_y = gal - 1;
    for dx in -tw..=tw {
        cv.put(tx + dx, lamp_y, '░', rgb((255, 240, 180)), Some(rgb((255, 220, 120))));
        cv.glyph(tx + dx, lamp_y - 1, '▄', rgb(paint));
    }

    // The beam, sweeping round; it spills over the whole headland as it
    // swings towards us.
    let a = tf * 0.045;
    let (ca, sa) = (a.cos(), a.sin());
    let reach = w as f32 * 0.85;
    if sa > -0.3 {
        let len = (reach * ca.abs()).max(3.0) as i32;
        let dir = if ca >= 0.0 { 1 } else { -1 };
        for i in 1..len {
            let x = tx + i * dir;
            let fall = 1.0 - i as f32 / len as f32;
            let spread = i / 10;
            for dy in -spread..=spread {
                let f = 0.55 * fall * (1.0 - dy.abs() as f32 / (spread + 1) as f32);
                cv.glow(x, lamp_y + dy, (255, 245, 190), f);
            }
        }
    }
    let toward = (1.0 - ca.abs()) * (sa > 0.0) as i32 as f32;
    cv.halo(tx, lamp_y, 2.0 + toward * 6.0, (255, 240, 170), 0.5 + toward * 0.4);

    // The heaving sea.
    let surf = |x: i32| -> i32 { sea + ((x as f32 * 0.13 - tf * 0.09).sin() * 1.2 + (x as f32 * 0.05 + tf * 0.04).sin()).round() as i32 };
    for x in 0..w {
        let s = surf(x);
        for y in s.max(0)..h {
            if x < cw && y >= rock_top(x) {
                continue;
            }
            let d = ((y - s) as f32 / (h - s).max(1) as f32).min(1.0);
            let base = lerp((40, 60, 76), (6, 12, 22), d.sqrt());
            let base = if flash { lerp(base, (150, 160, 190), 0.4) } else { base };
            let crest = y == s;
            let ph = (x as f32 * 0.31 + y as f32 * 1.7 - tf * 0.12).sin();
            let (ch, fg) = if crest {
                (if hash(x, (t / 3) as i32) % 2 == 0 { '≈' } else { '~' }, (220, 230, 240))
            } else if ph > 0.85 {
                ('~', (150, 170, 190))
            } else {
                (' ', (0, 0, 0))
            };
            cv.put(x, y, ch, rgb(fg), Some(rgb(base)));
        }
    }

    // Breakers bursting on the rocks.
    for b in 0..3 {
        let period = 70 + b as u64 * 23;
        let age = (t + hash(b, 3) as u64) % period;
        if age > 30 {
            continue;
        }
        let bx = cw + 1 - b * 3;
        let by = rock_top(bx).max(sea - 1);
        for k in 0..14 {
            let vx = (hash(k, b * 7) % 9) as f32 / 4.0 - 1.5;
            let vy = 0.35 + (hash(k, b * 7 + 1) % 10) as f32 / 18.0;
            let a = age as f32;
            let (x, y) = (bx + (vx * a * 0.35) as i32, by - (vy * a - 0.018 * a * a) as i32);
            if y <= by {
                cv.glyph(x, y, if k % 3 == 0 { '░' } else { '·' }, rgb((230, 238, 245)));
            }
        }
    }

    // The boat, pitching on its way in.
    let trip = 1400u64;
    let pt = (t + cv.roll(3) as u64 % trip) % trip;
    let bx = w - 4 - ((pt as f32 / trip as f32) * (w - cw - 6).max(1) as f32) as i32;
    let by = surf(bx) - 1;
    let tilt = (tf * 0.1).sin();
    cv.glyph(bx - 1, by + 1, if tilt > 0.0 { '▄' } else { '▀' }, rgb((90, 60, 40)));
    cv.put(bx, by + 1, ' ', Color::Black, Some(rgb((90, 60, 40))));
    cv.glyph(bx + 1, by + 1, if tilt > 0.0 { '▀' } else { '▄' }, rgb((90, 60, 40)));
    cv.glyph(bx, by, '│', rgb((120, 100, 80)));
    cv.glyph(bx + 1, by, '◄', rgb((200, 190, 160)));
    cv.glyph(bx - 1, by, '☺', rgb((240, 200, 120)));

    // Rain driving in, and the lightning.
    for k in 0..(w * h / 30).max(4) {
        let y = ((hash(k, 2) as u64 + t * 2) % h.max(1) as u64) as i32;
        let x = ((hash(k, 1) as u64 + t) % w.max(1) as u64) as i32 - y;
        cv.glyph(x.rem_euclid(w.max(1)), y, '/', rgb((110, 125, 150)));
    }
    if flash {
        let mut x = cw + (hash(flash_n as i32, seed as i32) % (w - cw).max(1) as u32) as i32;
        for y in 0..sea {
            cv.glyph(x, y, if hash(x, y) % 2 == 0 { '/' } else { '\\' }, rgb((250, 250, 255)));
            x += if hash(y, flash_n as i32) % 2 == 0 { 1 } else { -1 };
        }
    }
}
