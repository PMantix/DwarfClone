//! Wind on the ridge: a long bare crest under racing clouds, a few old
//! trees bent flat by a hundred years of the same wind, their crowns
//! streaming, the grass running in silver waves, leaves and seeds flying,
//! a hawk hanging still in the gale, and a dwarf leaning into it, one hand
//! on her hat. The gusts come and go. The seed picks the wind's side, the
//! season and the trees.

use super::a_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the wind has been blowing here since the world began",
    "trees that bow, and never break",
    "hold on to your beard",
    "up here even the stone is leaning",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let g = (cv.sky - 1).max(1);
    let tf = t as f32;
    let salt = cv.salt();
    let autumn = cv.roll(1) % 2 == 0;
    let (top, bot) = if autumn { ((70, 90, 140), (210, 190, 170)) } else { ((50, 110, 200), (180, 215, 240)) };
    let gust = ((tf * 0.021 + (cv.roll(2) % 100) as f32).sin() * 0.5 + 0.5).powf(1.5);

    cv.sky_fill(g, top, bot);
    // Clouds racing over, faster in the gusts.
    for k in 0..(w / 18).max(3) {
        let len = 4 + (hash(k + salt, 19) % 10) as i32;
        let y = 1 + (hash(k + salt, 13) % (g as u32 * 2 / 3).max(1)) as i32;
        let x = ((hash(k + salt, 15) as u64 + t * (2 + (k % 3) as u64) / 5) % (w as u64 + len as u64 + 10)) as i32 - len - 5;
        // Wisps torn off the back.
        cloud(cv, x, y, len, lerp(bot, (255, 255, 255), 0.4), lerp(bot, (255, 255, 255), 0.75));
        for i in 1..4 {
            cv.glyph(x - i * 2, y + 1, '─', rgb(lerp(bot, (255, 255, 255), 0.5)));
        }
    }

    // The ridge: a long crest climbing from one side.
    let rise = (g - 1).max(1) as f32;
    let surf: Vec<i32> = (0..w)
        .map(|x| {
            let u = x as f32 / w.max(1) as f32;
            g - 1 - (((u * 1.4).min(1.0) * 0.65 + ridge(x, w, seed, 220) * 0.15) * rise) as i32
        })
        .collect();
    for x in 0..w {
        let s = surf[x as usize];
        for y in s.max(0)..h {
            let d = (y - s) as f32;
            let wave = (x as f32 * 0.18 + y as f32 * 0.6 - tf * (0.15 + gust * 0.2)).sin();
            let base = if autumn { lerp((170, 150, 80), (120, 100, 60), (d / 10.0).min(1.0)) } else { lerp((110, 160, 70), (70, 120, 50), (d / 10.0).min(1.0)) };
            let c = lerp(base, (230, 230, 200), ((wave - 0.4).max(0.0) * 0.6) * (0.4 + gust));
            let ch = if wave > 0.6 { '\'' } else if wave > -0.2 { '"' } else { ' ' };
            let ch = if ch == '\'' && gust > 0.6 { '`' } else { ch };
            cv.put(x, y, if y == s { '▄' } else { ch }, rgb(lerp(c, (255, 255, 230), 0.3)), Some(rgb(c)));
        }
    }

    // Wind-bent trees along the crest, crowns streaming downwind.
    let leaf = if autumn { [(220, 140, 40), (190, 70, 40), (230, 190, 60)] } else { [(40, 110, 50), (60, 130, 60), (30, 90, 40)] };
    let trees = 3 + (cv.roll(3) % 3) as i32;
    for k in 0..trees {
        let x0 = (w as f32 * (0.08 + k as f32 * 0.85 / trees as f32)) as i32 + (hash(k + salt, 3) % 4) as i32;
        if x0 >= w {
            continue;
        }
        let base = surf[x0 as usize] - 1;
        let tall = 3 + (hash(k + salt, 4) % 3) as i32;
        let sway = (gust * 2.0 + (tf * 0.3 + k as f32).sin() * gust).round() as i32;
        // The trunk leans over, then the crown streams flat.
        let mut x = x0;
        for i in 0..tall {
            cv.glyph(x, base - i, if i < tall / 2 { '│' } else { '╱' }, rgb((90, 66, 46)));
            if i >= tall / 2 {
                x += 1;
            }
        }
        let cy = base - tall;
        let len = 5 + tall + sway;
        for i in 0..len {
            let thick = if i < len / 2 { 1 } else { 0 };
            for dy in -thick..=1 {
                let hs = hash(x + i, cy + dy + salt);
                let ch = if hs % 4 == 0 { '♠' } else { '♣' };
                let c = leaf[(hs % 3) as usize];
                let (px, py) = (x + i - 1, cy + dy + (i * dy.abs()) / len.max(1));
                cv.glow(px, py, lerp(c, (0, 0, 0), 0.45), 0.85);
                cv.glyph(px, py, ch, rgb(lerp(c, (255, 255, 200), 0.15)));
            }
        }
    }

    // A dwarf leaning into it on the crest, hat in hand.
    let dx = (w as f32 * 0.86) as i32 + pingpong(t, 30, 0, 2);
    if dx < w {
        let y = surf[dx as usize] - 1;
        dwarf(cv, dx, y, (120, 100, 180));
        cv.glyph(dx - 1, y, '\\', rgb((150, 120, 90)));
        let hat = gust > 0.75;
        if hat {
            let fly = ((tf * 0.5) as i32) % 12;
            cv.glyph(dx + 1 + fly, y - 2 - fly / 4, '▀', rgb((90, 60, 40)));
        } else {
            cv.glyph(dx, y - 1, '▀', rgb((90, 60, 40)));
        }
    }

    // A hawk hanging still in the wind, slipping and holding.
    let hx = (w as f32 * 0.4) as i32 + ((tf * 0.05).sin() * 3.0) as i32;
    let hy = (g as f32 * 0.35) as i32 + ((tf * 0.11).sin() * 1.0) as i32;
    cv.glyph(hx, hy, if gust > 0.5 { '~' } else { 'v' }, rgb((60, 40, 30)));

    // Leaves and seeds flying.
    let n = ((w / 5) as f32 * (0.3 + gust)) as i32 + 3;
    for k in 0..n {
        let speed = 1 + (k % 2) as u64;
        let x = ((hash(k + salt, 1) as u64 + t * 2 / speed) % (w as u64 + 4)) as i32 - 2;
        let y = (hash(k + salt, 2) % h.max(1) as u32) as i32 + ((x as f32 * 0.25 + k as f32).sin() * 2.0) as i32;
        cv.glyph(x, y, if k % 3 == 0 { ',' } else { '·' }, rgb(leaf[(k % 3) as usize]));
    }
}
