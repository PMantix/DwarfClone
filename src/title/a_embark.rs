//! The first wagon arriving at a new embark: untouched wild country, a
//! lone wagon rolling in along no road at all, seven dwarves piling out
//! to unload barrels and crates, one striking the first blow into the
//! hillside with sparks flying, and the wild things watching from the
//! trees. Then the light changes and another wagon comes. The seed picks
//! the season and the lay of the land.

use super::a_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "seven dwarves, one wagon, and a whole mountain",
    "strike the earth!",
    "no walls yet, no beds, no beer: get digging",
    "every great fortress started with a wagon",
];

/// Frames from the wagon showing up to the next one.
const CYCLE: u64 = 1500;

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let g = (cv.sky - 1).max(1);
    let salt = cv.salt();
    let season = cv.roll(1) % 3; // spring, summer, autumn
    let (top, bot) = [((90, 150, 220), (210, 230, 240)), ((60, 130, 220), (180, 220, 250)), ((100, 110, 170), (240, 200, 150))][season as usize];
    let leaves: [Rgb; 3] = [[(110, 190, 80), (240, 180, 210), (90, 170, 70)], [(50, 140, 50), (40, 120, 50), (70, 150, 60)], [(220, 150, 40), (200, 80, 40), (230, 200, 70)]][season as usize];
    let c = (t + cv.roll(2) as u64) % CYCLE;

    cv.sky_fill(g, top, bot);
    clouds(cv, (w / 30).max(2), (1, (g / 3).max(2)), 10, lerp(bot, (255, 255, 255), 0.6), (255, 255, 255), salt);

    // The hill the fortress will be dug into, on one side; wild land elsewhere.
    let hx = (w as f32 * 0.72) as i32;
    let hw = (w as f32 * 0.25).max(8.0);
    let rise = (g - 1).max(1) as f32;
    let surf: Vec<i32> = (0..w)
        .map(|x| {
            let u = ((x - hx) as f32 / hw).abs();
            let hill = (1.0 - u * u).max(0.0) * 0.85 + peaks(x, w, seed, 170, 2) * 0.3 * (1.0 - u.min(1.0));
            g - 1 - (hill.max(ridge(x, w, seed, 175) * 0.2) * rise) as i32
        })
        .collect();
    for x in 0..w {
        let s = surf[x as usize];
        for y in s.max(0)..g {
            let d = (y - s) as f32;
            let rocky = d > 2.0 && hash(x + salt, y) % 4 == 0;
            let bg = if rocky { (110, 104, 96) } else { lerp((100, 140, 70), (80, 110, 56), (d / 6.0).min(1.0)) };
            let ch = if y == s { '▄' } else if rocky { '░' } else { ' ' };
            cv.put(x, y, ch, rgb(if y == s { leaves[0] } else { (90, 86, 80) }), Some(rgb(lerp(bg, bot, 0.1))));
        }
    }
    // The wild plain, with trees.
    for y in g.max(0)..h {
        for x in 0..w {
            let hs = hash(x * 3 + salt, y * 7);
            let (ch, fg) = match hs % 10 {
                0 => ('♣', leaves[(hs / 10 % 3) as usize]),
                1 => ('♠', (40, 100, 50)),
                2 | 3 => ('"', (120, 170, 80)),
                _ => (' ', (0, 0, 0)),
            };
            cv.put(x, y, ch, rgb(fg), Some(rgb(lerp((92, 136, 60), (70, 110, 46), (hs % 9) as f32 / 12.0))));
        }
    }
    for x in 0..w {
        if hash(x + salt, 3) % 5 == 0 && (x - hx).abs() as f32 > hw * 0.6 {
            cv.glyph(x, surf[x as usize] - 1, if x % 2 == 0 { '♣' } else { '♠' }, rgb(leaves[(x % 3) as usize]));
        }
    }

    // The wagon rolls in from the far side and stops below the hill.
    let stop = hx - (hw * 0.6) as i32;
    let roll_in = (c / 3) as i32;
    let wx = (-8 + roll_in).min(stop - 6);
    let parked = wx == stop - 6;
    let y = g - 1;
    if g >= 3 {
        cv.sprite(wx, y - 2, &["▄███▄", "█████", "○   ○"], rgb((150, 104, 60)));
        cv.sprite(wx, y - 2, &["▄▀▀▀▄"], rgb((230, 222, 200)));
        let wheel = if parked || (t / 4) % 2 == 0 { '○' } else { 'o' };
        cv.glyph(wx, y, wheel, rgb((90, 70, 50)));
        cv.glyph(wx + 4, y, wheel, rgb((90, 70, 50)));
        cv.glyph(wx + 6, y, 'Ö', rgb((150, 110, 70)));
        cv.glyph(wx + 7, y, 'Ö', rgb((140, 100, 60)));
    }

    // Once parked, the seven pile out and get to work.
    if parked {
        let since = c.saturating_sub(((stop + 2).max(0) * 3) as u64);
        let out = (since / 20).min(7) as i32;
        let face = hx - (hw * 0.25) as i32;
        for k in 0..out {
            let coat = COATS[((k + salt) % 8) as usize];
            let walk = ((since - k as u64 * 20) / 4) as i32;
            match k {
                // The miner heads for the hill and strikes the earth.
                0 => {
                    let x = (wx + 3 + walk).min(face - 1);
                    let sy = surf[x.clamp(0, w - 1) as usize] - 1;
                    dwarf(cv, x, sy.max(y - 1).min(y), coat);
                    if x == face - 1 {
                        let swing = (t / 5) % 4;
                        cv.glyph(x + 1, y - if swing < 2 { 1 } else { 0 }, if swing < 2 { '/' } else { '─' }, rgb((190, 190, 200)));
                        if swing == 2 {
                            for s in 0..3 {
                                cv.glyph(x + 2 + s % 2, y - 1 - s / 2, '*', rgb((255, 230, 120)));
                            }
                        }
                        // The first dark mouth of the fortress, growing.
                        let dug = ((since / 120) as i32).min(3);
                        for d in 0..dug {
                            cv.fill(face + d, y, rgb((30, 24, 20)));
                        }
                    }
                }
                // Unloaders, carrying barrels and crates to a growing pile.
                1..=4 => {
                    let span = 6 + k;
                    let p = pingpong(since / 3 + k as u64 * 7, 1, 0, span);
                    let x = wx - 1 - p;
                    dwarf(cv, x, y, coat);
                    let carrying = (since / 3 + k as u64 * 7) % (2 * span as u64) < span as u64;
                    if carrying {
                        cv.glyph(x, y - 1, if k % 2 == 0 { '■' } else { 'ö' }, rgb((160, 110, 60)));
                    }
                }
                // Others look the place over.
                _ => {
                    let x = wx + 9 + k + pingpong(t + k as u64 * 40, 15, 0, 4);
                    dwarf(cv, x, y, coat);
                }
            }
        }
        let pile = ((since / 60) as i32).min(5);
        for p in 0..pile {
            cv.glyph(wx - 12 - p, y, if p % 2 == 0 { 'ö' } else { '■' }, rgb((150, 100, 55)));
        }
    }

    // A deer at the forest edge, and a few birds.
    let dx = (w as f32 * 0.06) as i32 + pingpong(t, 30, 0, 3);
    cv.glyph(dx, g - 1, 'D', rgb((170, 120, 70)));
    for k in 0..3 {
        let x = drift(t, k + salt, 3, w + 10) - 5;
        cv.glyph(x, 2 + k * 2, if (t / 5 + k as u64) % 2 == 0 { 'v' } else { '^' }, rgb((50, 50, 60)));
    }
}
