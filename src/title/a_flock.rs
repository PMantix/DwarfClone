//! The flocks going south: an autumn evening over the marshes below the
//! mountain, skein after skein of geese in long ragged vees crossing the
//! sky, calling, and a great cloud of starlings swirling over the reeds,
//! folding and unfolding, while a dwarf on the dyke watches them go. The
//! seed picks the evening, the marsh and which birds come.

use super::a_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the geese know the way; the dwarves dig their own",
    "summer packs up and flies south",
    "a thousand wings and not a single map",
    "they'll be back with the thaw",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let g = (cv.sky - 1).max(1);
    let tf = t as f32;
    let salt = cv.salt();
    let (top, bot) = [((50, 60, 120), (250, 170, 110)), ((70, 80, 140), (240, 200, 150)), ((40, 40, 90), (220, 120, 100))][(cv.roll(1) % 3) as usize];
    let murmur = cv.roll(2) % 3 != 0;

    cv.sky_fill(g, top, bot);
    let (sx, sy) = ((w as f32 * 0.7) as i32, (g as f32 * 0.85) as i32);
    cv.halo(sx, sy, (g as f32 * 0.5).max(2.0), (255, 180, 110), 0.45);
    cv.disc(sx, sy, (g as f32 * 0.1).clamp(0.7, 2.8), (255, 210, 140));
    // Long thin evening clouds.
    for k in 0..(w / 30).max(2) {
        let len = (w / 5).max(6) + (hash(k + salt, 3) % 10) as i32;
        let y = 1 + (hash(k + salt, 4) % (g as u32 / 2).max(1)) as i32;
        let x0 = drift(t, k + salt, 25, w + len) - len / 2;
        for i in 0..len {
            cv.glow(x0 + i, y, lerp(bot, (255, 230, 220), 0.4), 0.45 * (i as f32 / len as f32 * std::f32::consts::PI).sin());
        }
    }

    // The mountain, far off and blue.
    for x in 0..w {
        let m = peaks(x, w, seed, 230, 3);
        let s = g - 1 - (m * g as f32 * 0.55) as i32;
        for y in s.max(0)..g {
            cv.put(x, y, if y == s { '▲' } else { ' ' }, rgb(lerp(bot, (255, 255, 255), 0.4)), Some(rgb(lerp((70, 70, 110), bot, 0.3))));
        }
    }

    // The marsh: pools holding the sunset, reeds swaying.
    for y in g.max(0)..h {
        for x in 0..w {
            let pool = ((x as f32 * 0.09 + y as f32 * 0.5 + 1.3).sin() + (x as f32 * 0.04 - y as f32 * 0.8).sin()) > 0.6;
            let hs = hash(x * 3 + salt, y * 5);
            if pool {
                let ch = if (x + y * 3 + (t / 9) as i32) % 13 == 0 { '─' } else { ' ' };
                let c = lerp(lerp(top, bot, 0.6), (30, 40, 60), 0.35);
                cv.put(x, y, ch, rgb(lerp(c, (255, 220, 180), 0.4)), Some(rgb(c)));
            } else {
                let lean = ((x as f32 * 0.15 + tf * 0.05).sin() > 0.3) as u32;
                let ch = match hs % 8 {
                    0 => if lean == 1 { '/' } else { '│' },
                    1 => '"',
                    2 => if hs % 3 == 0 { '¥' } else { '│' },
                    _ => ' ',
                };
                cv.put(x, y, ch, rgb(lerp((140, 120, 70), bot, 0.2)), Some(rgb(lerp((70, 70, 40), (50, 50, 32), (hs % 7) as f32 / 8.0))));
            }
        }
    }
    // The dyke along the horizon and the dwarf on it.
    for x in 0..w {
        cv.put(x, g, '▀', rgb((90, 80, 56)), None);
    }
    let dx = (w as f32 * 0.12) as i32;
    dwarf(cv, dx, g - 1, (120, 160, 90));
    cv.glyph(dx + 1, g - 2, if (t / 40) % 2 == 0 { '/' } else { '│' }, rgb((220, 200, 180)));

    // Skeins of geese, vee after vee crossing the sky.
    let skeins = (w / 50).max(2) + (cv.roll(3) % 2) as i32;
    for k in 0..skeins {
        let size = 4 + (hash(k + salt, 7) % 6) as i32;
        let span = (w + size * 4 + 20) as u64;
        let lx = ((t / (3 + (k % 2) as u64) + hash(k + salt, 8) as u64) % span) as i32 - size * 2 - 10;
        let ly = 2 + (hash(k + salt, 9) % (g as u32 * 2 / 3).max(1)) as i32 + ((tf * 0.02 + k as f32).sin() * 1.5) as i32;
        for i in 0..=size {
            for arm in [-1, 1] {
                if i == 0 && arm == 1 {
                    continue;
                }
                // Ragged: each bird a little off its place, flapping in turn.
                let wob = ((tf * 0.1 + (i * arm) as f32).sin() * 0.6).round() as i32;
                let x = lx - i * 2 + wob;
                let y = ly + arm * (i * 2 / 3);
                let flap = (t / 5 + (i + k) as u64) % 2 == 0;
                cv.glyph(x, y, if flap { 'v' } else { '^' }, rgb((30, 26, 34)));
            }
        }
    }

    // The murmuration: a swirling cloud of starlings over the reeds.
    if murmur {
        let n = (w * 2).max(40);
        let ccx = w as f32 * 0.5 + (tf * 0.013).sin() * w as f32 * 0.3;
        let ccy = g as f32 * 0.5 + (tf * 0.021).cos() * g as f32 * 0.2;
        let stretch = 0.6 + (tf * 0.017).sin() * 0.4;
        for k in 0..n {
            let hs = hash(k + salt, 11);
            let a = (hs % 628) as f32 / 100.0 + tf * 0.03 * (1.0 + (hs % 5) as f32 * 0.1);
            let r = ((hs >> 10) % 100) as f32 / 100.0;
            let fold = (a * 2.0 + tf * 0.02).sin() * 0.3;
            let x = ccx + a.cos() * r * w as f32 * 0.14 * (1.0 + stretch) + fold * 6.0;
            let y = ccy + (a * 1.3).sin() * r * g as f32 * 0.22 * (1.6 - stretch);
            cv.glyph(x as i32, y as i32, '·', rgb((24, 20, 30)));
        }
    }
}
