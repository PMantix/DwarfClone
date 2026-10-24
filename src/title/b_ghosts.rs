//! The abandoned hall: long tables still laid for a feast nobody came
//! back to, chairs knocked over, dust and cobwebs, and the ghosts of the
//! fortress drifting through it all, through the tables, through the
//! pillars, the candles shrinking from them. Now and then one of them
//! knocks a goblet over. Below, the unburied bones they rose from. The
//! seed picks the hall, the light and who haunts it.

use super::b_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "unburied dead do not rest quietly",
    "the feast is still laid. nobody is hungry",
    "they remember the fortress as it was",
    "carve them a slab and they might sleep",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t) = (cv.w(), cv.h(), cv.t);
    let g = (cv.sky - 1).max(3).min(h - 1);
    let tf = t as f32;
    let salt = cv.salt();
    let stone = [(66, 70, 80), (74, 70, 64), (60, 66, 66)][(cv.roll(1) % 3) as usize];
    let haunt = [(150, 220, 255), (170, 255, 200), (210, 190, 255)][(cv.roll(2) % 3) as usize];
    let air = lerp(stone, (0, 0, 0), 0.8);
    bedrock(cv, 0, h, stone, salt);

    // The hall, dust lying everywhere.
    hollow(cv, 0, 1, w, g, air);
    floor(cv, 0, w, g, lerp(stone, (120, 120, 120), 0.3));
    for x in 0..w {
        if hash(x + salt, 3) % 4 == 0 {
            cv.glyph(x, g - 1, '.', rgb(lerp(stone, (160, 160, 150), 0.3)));
        }
    }
    // Pillars, with cobwebs in their corners.
    let bay = 16 + (cv.roll(3) % 6) as i32;
    let mut px = 3 + (cv.roll(4) % 8) as i32;
    while px < w {
        for y in 1..g {
            cv.put(px, y, '║', rgb(lerp(stone, (150, 150, 160), 0.3)), Some(rgb(lerp(stone, (0, 0, 0), 0.3))));
        }
        cv.glyph(px + 1, 1, '╱', rgb((120, 120, 130)));
        cv.glyph(px - 1, 1, '╲', rgb((120, 120, 130)));
        if hash(px, salt) % 2 == 0 {
            cv.glyph(px + 2, 1, '·', rgb((120, 120, 130)));
            cv.glyph(px + 1, 2, '·', rgb((120, 120, 130)));
        }
        px += bay;
    }
    // Tables and their wreckage.
    let n = (w / 20).max(1);
    let mut goblets = Vec::new();
    for k in 0..n {
        let tx = 4 + k * (w - 8) / n.max(1) + (hash(k, salt) % 4) as i32;
        let tl = (w / n / 2).clamp(4, 10);
        let fallen = hash(k, salt + 1) % 3 == 0;
        let wood = (110, 84, 60);
        if fallen {
            for x in tx..tx + tl {
                cv.glyph(x, g - 1, '▄', rgb(lerp(wood, air, 0.3)));
            }
            cv.glyph(tx + tl, g - 1, '╨', rgb(lerp(wood, air, 0.3)));
        } else {
            for x in tx..tx + tl {
                cv.glyph(x, g - 2, '═', rgb(wood));
            }
            cv.glyph(tx, g - 1, '║', rgb(wood));
            cv.glyph(tx + tl - 1, g - 1, '║', rgb(wood));
            goblets.push((tx + 1 + (hash(k, 5) % (tl - 2).max(1) as u32) as i32, g - 3));
            cv.glyph(tx + tl / 2, g - 3, '°', rgb((170, 150, 110)));
            cv.glyph(tx - 1, g - 1, if hash(k, 7) % 2 == 0 { 'π' } else { '╥' }, rgb(lerp(wood, air, 0.2)));
        }
    }
    // A shaft of grey light from a crack high up.
    let lx = (w as f32 * (0.15 + (cv.roll(5) % 60) as f32 / 100.0)) as i32;
    for y in 1..g {
        for dx in 0..3 {
            let x = lx + dx + (y - 1) / 2;
            cv.glow(x, y, (140, 150, 170), 0.18);
        }
    }
    // Candles still burning on the tables, for no one.
    let candles: Vec<(i32, i32)> = (0..n).map(|k| (4 + k * (w - 8) / n.max(1) + 2, g - 3)).collect();

    // The ghosts, drifting through everything.
    let ng = 3 + (cv.roll(6) % 4) as i32;
    let mut where_: Vec<(i32, i32)> = Vec::new();
    for k in 0..ng {
        let speed = 0.004 + (hash(k, salt + 9) % 5) as f32 * 0.0015;
        let ph = (hash(k, salt + 10) % 628) as f32 / 100.0;
        let x = (w as f32 * 0.5 + (tf * speed + ph).sin() * w as f32 * 0.48) as i32;
        let y = 2 + (((tf * speed * 2.3 + ph * 1.7).sin() * 0.5 + 0.5) * (g - 4).max(1) as f32) as i32;
        where_.push((x, y));
        // The wisp trailing behind.
        let back = if (tf * speed + ph).cos() > 0.0 { -1 } else { 1 };
        for i in 1..5 {
            let f = 1.0 - i as f32 / 5.0;
            cv.glow(x + back * i, y + (i / 2), haunt, 0.25 * f);
            if i < 3 {
                cv.glyph(x + back * i, y + (i / 2), '░', rgb(lerp(haunt, air, 0.5)));
            }
        }
        cv.halo(x, y, 2.0, haunt, 0.3);
        let fade = ((tf * 0.05 + k as f32).sin() * 0.5 + 0.5) * 0.4;
        cv.glyph(x, y, '☺', rgb(lerp(haunt, air, fade)));
        // What they carried in life.
        match k % 3 {
            0 => cv.glyph(x + 1, y, '/', rgb(lerp(haunt, air, 0.4))),
            1 => cv.glyph(x, y + 1, '▲', rgb(lerp(haunt, air, 0.5))),
            _ => {}
        }
    }
    for (k, &(cx, cy)) in candles.iter().enumerate() {
        let near = where_.iter().map(|&(x, y)| ((x - cx).abs() / 2 + (y - cy).abs()) as f32).fold(99.0, f32::min);
        let f = flicker(t, k as i32) * (near / 8.0).min(1.0);
        if f > 0.15 {
            cv.halo(cx, cy, 2.5 * f, (255, 190, 110), 0.4 * f);
            cv.glyph(cx, cy, '¡', rgb(lerp((120, 80, 50), (255, 220, 150), f)));
        } else {
            cv.glyph(cx, cy, '¡', rgb((70, 60, 60)));
        }
    }
    // Now and then a goblet goes over.
    if let Some(&(gx, gy)) = goblets.get(((t / 300) % goblets.len().max(1) as u64) as usize) {
        let s = t % 300;
        if s < 40 {
            let y = gy + (s as i32 / 8).min(2);
            cv.glyph(gx + (s as i32 / 10), y, if s < 20 { '°' } else { '•' }, rgb((180, 160, 110)));
            if s < 20 {
                cv.glyph(gx, gy - 1, '!', rgb(haunt));
            }
        } else {
            cv.glyph(gx, gy, '°', rgb((170, 150, 110)));
        }
    }

    // Below: the bones nobody buried, and the cold rising off them.
    let lb = below(cv);
    if h - lb >= 4 {
        let by = (lb + (h - lb) / 2).min(h - 2);
        hollow(cv, 2, by - 2, w - 2, by + 1, air);
        floor(cv, 2, w - 2, by + 1, lerp(stone, (120, 120, 120), 0.3));
        for k in 0..(w / 9).max(2) {
            let x = 3 + (hash(k, salt + 20) % (w - 6).max(1) as u32) as i32;
            cv.glyph(x, by, if k % 2 == 0 { '%' } else { '☻' }, rgb((200, 195, 170)));
            let rise = ((t / 6 + hash(k, 21) as u64) % 10) as i32;
            if rise < 6 {
                cv.glyph(x, by - 1 - rise / 2, '░', rgb(lerp(haunt, air, 0.3 + rise as f32 * 0.1)));
            }
        }
    }
}
