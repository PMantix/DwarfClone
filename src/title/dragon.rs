//! A dragon crosses the sky over the peaks, swooping low to breathe fire
//! on the forest; the trees it lights burn, smoke, and smoulder. The seed
//! picks the hour, the dragon's hide and which way it flies.

use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "something old has woken in the high places",
    "the sky burns, and the forest with it",
    "wise dwarves dig deep and close the doors",
    "a hoard draws a wyrm as honey draws flies",
];

// Facing right; flipped for flights the other way.
const WINGS_UP: [&str; 3] = ["    ▄▀▀▀▄     ", "≈═▄▄██████▄▄▓►", "     ╜  ╜     "];
const WINGS_DOWN: [&str; 3] = ["              ", "≈═▄▄██████▄▄▓►", "    ▀▄▄▄▀ ╜   "];
// A bigger beast for big screens; the body is the third row.
const BIG_UP: [&str; 5] = [
    "          ▄▄▀▀▀▀▄▄      ",
    "       ▄▀▀  ▄▄▀▀▀▀▀▄    ",
    "≈≈═══▄▄▄███████████▄▄▓▓►",
    "         ▀█▀     ▀█▀    ",
    "          ╜       ╜     ",
];
const BIG_DOWN: [&str; 5] = [
    "                        ",
    "                        ",
    "≈≈═══▄▄▄███████████▄▄▓▓►",
    "       ▀▀▄▄▄█▀▀▀▀▀▀█▀   ",
    "            ▀▀▄▄▀  ╜    ",
];

fn flip(row: &str) -> String {
    row.chars()
        .rev()
        .map(|c| match c {
            '►' => '◄',
            '╜' => '╙',
            c => c,
        })
        .collect()
}

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, sky, seed) = (cv.w(), cv.h(), cv.t, cv.sky, cv.seed);
    let g = sky - 1; // the forest's edge
    let hour = cv.roll(1) % 3;
    let (top, bot) = match hour {
        0 => ((20, 16, 54), (150, 70, 70)),
        1 => ((6, 8, 26), (36, 26, 56)),
        _ => ((50, 80, 130), (120, 140, 170)),
    };
    cv.sky_fill(g, top, bot);
    if hour == 1 {
        cv.stars(g - 3, 41);
    }
    let hide = [(200, 45, 35), (60, 160, 70), (225, 185, 60), (150, 175, 225)][(cv.roll(2) % 4) as usize];
    let dir = if cv.roll(3) % 2 == 0 { 1 } else { -1 };

    // The peaks behind the forest.
    let rise = ((sky - 2).max(1) * 3 / 4).max(1) as f32;
    let rock = lerp(bot, (30, 28, 34), 0.7);
    for x in 0..w {
        let m = peaks(x, w, seed, 30, 3 + (w / 60).max(1) as u32);
        let s = g - 1 - (m * rise) as i32;
        for y in s.max(0)..g {
            let snow = (y - s) < 1 + (m * 2.0) as i32 && m > 0.5;
            let ch = if y == s { '▲' } else if snow { '▀' } else { ' ' };
            let fg = if snow { (235, 240, 250) } else { (110, 104, 112) };
            cv.put(x, y, ch, rgb(fg), Some(rgb(rock)));
        }
    }

    // The forest, with a river winding through it.
    let salt = cv.salt();
    let forest = |x: i32, y: i32| -> (char, Rgb, Rgb) {
        let hs = hash(x * 3 + 7 + salt, y * 5 + 1);
        let glade = ((x as f32 * 0.07 + salt as f32).sin() + (y as f32 * 0.21 - x as f32 * 0.03).sin()) > 1.1;
        let ch = match hs % if glade { 30 } else { 11 } {
            0 | 1 => '♠',
            2..=5 => '♣',
            6 | 7 => '"',
            _ => ' ',
        };
        let col = [(30, 120, 55), (40, 140, 60), (25, 100, 50), (70, 150, 60)][(hs % 4) as usize];
        (ch, col, if glade { (26, 44, 20) } else { (12, 26, 14) })
    };
    let roff = (cv.roll(4) % (w.max(4) as u32 / 3)) as i32;
    let river = |y: i32| w / 3 + roff + ((y as f32 * 0.35).sin() * 3.0) as i32;
    for y in g.max(0)..h {
        let rx = river(y);
        for x in 0..w {
            if y > g && (x - rx).abs() <= 1 {
                let ch = if (x + y + (t / 8) as i32) % 4 == 0 { '~' } else { '≈' };
                cv.put(x, y, ch, rgb((90, 130, 200)), Some(rgb((20, 40, 80))));
            } else {
                let (ch, col, bg) = forest(x, y);
                cv.put(x, y, ch, rgb(col), Some(rgb(bg)));
            }
        }
    }

    // The flight: across and gone, then back again later.
    let period = (w as u64 + 40) * 2 + 300;
    let local = (t + (cv.roll(5) % 400) as u64) % period;
    let ymin = 1;
    let big = w >= 110 && sky >= 14;
    let (wings_up, wings_down, body): (&[&str], &[&str], i32) = if big { (&BIG_UP, &BIG_DOWN, 2) } else { (&WINGS_UP, &WINGS_DOWN, 1) };
    let width = wings_up[body as usize].chars().count() as i32;
    let ymax = (g - 3 - wings_up.len() as i32 + body).max(ymin + body);
    let ymin = ymin + body;
    let head_at = |tt: u64| -> i32 {
        if dir > 0 {
            -width - 6 + (tt / 2) as i32
        } else {
            w + 6 + width - (tt / 2) as i32
        }
    };
    let alt = |hx: i32| -> i32 {
        let swoop = (hx as f32 / (w as f32 * 0.35).max(8.0) * std::f32::consts::PI + seed as f32 % 3.0).cos() * 0.5 + 0.5;
        ymin + ((ymax - ymin) as f32 * swoop).round() as i32
    };
    let breathing = |tt: u64, hx: i32| -> bool { alt(hx) >= ymax - 1 && (tt / 18) % 5 != 0 };
    let ground_hit = |hx: i32| -> i32 { hx + dir * (2 + (g - 1 - alt(hx)) * 3 / 2) };

    // Where the fire has fallen this flight, and when.
    let mut lit: Vec<Option<u64>> = vec![None; w.max(0) as usize];
    let mut tt = 0;
    while tt <= local {
        let hx = head_at(tt);
        if breathing(tt, hx) {
            let gx = ground_hit(hx);
            for x in gx - 1..=gx + 1 {
                if x >= 0 && x < w && lit[x as usize].is_none() {
                    lit[x as usize] = Some(tt);
                }
            }
        }
        tt += 2;
    }
    for x in 0..w {
        let Some(at) = lit[x as usize] else { continue };
        let age = local - at;
        let depth = 1 + (hash(x, 61) % 3) as i32;
        for y in g..(g + depth).min(h) {
            if age < 260 {
                let hs = hash(x + y * 31, (t / 3) as i32);
                let fire = [(255, 230, 90), (255, 150, 40), (230, 70, 25)][(hs % 3) as usize];
                cv.put(x, y, if hs % 2 == 0 { '▲' } else { '♠' }, rgb(fire), Some(rgb((70, 20, 5))));
            } else {
                let ember = (t / 7 + x as u64) % 19 == 0;
                cv.put(x, y, if ember { '·' } else { '♣' }, if ember { rgb((255, 120, 40)) } else { rgb((55, 48, 44)) }, Some(rgb((16, 12, 10))));
            }
        }
        // Smoke climbing from the burning and the burnt.
        if age > 20 {
            let rows = if age < 260 { 6 } else { 3 };
            for k in 0..rows {
                let a = ((t / 6) as i32 + k * 3 + x) % (rows * 3);
                let y = g - 1 - a / 2;
                let sx = x + (a / 4) * -dir;
                if y >= 0 && hash(x, k) % 3 == 0 {
                    let fade = 1.0 - a as f32 / (rows * 3) as f32;
                    cv.glyph(sx, y, if a < 4 { '░' } else { '·' }, mix((60, 56, 62), (150, 140, 140), fade));
                }
            }
        }
    }

    // The dragon itself.
    let hx = head_at(local);
    let y = alt(hx);
    let up = (t / 6) % 2 == 0;
    let rows = if up { wings_up } else { wings_down };
    let x0 = if dir > 0 { hx - width + 1 } else { hx };
    let flipped: Vec<String> = rows.iter().map(|r| if dir > 0 { r.to_string() } else { flip(r) }).collect();
    let refs: Vec<&str> = flipped.iter().map(|s| s.as_str()).collect();
    cv.sprite(x0, y - body, &refs, rgb(hide));
    let eye = if dir > 0 { hx - 1 } else { hx + 1 };
    cv.glyph(eye, y, '▓', rgb(lerp(hide, (255, 255, 200), 0.5)));

    // Fire pouring from its jaws to the trees.
    if breathing(local, hx) {
        let (mx, my) = (hx + dir, y);
        let gx = ground_hit(hx);
        let path = line((mx, my), (gx, g));
        for (i, (px, py)) in path.iter().enumerate() {
            let spread = (i as i32) / 3;
            for dx in -spread..=spread {
                let hs = hash(px + dx, *py + (t / 2) as i32);
                if hs % 3 == 0 && dx != 0 {
                    continue;
                }
                let col = [(255, 240, 120), (255, 170, 50), (240, 80, 20)][((i as u32 / 2 + hs) % 3) as usize];
                let ch = ['▒', '░', '▓'][(hs % 3) as usize];
                cv.glyph(px + dx, *py, ch, rgb(col));
            }
        }
    }

    // Birds fleeing the other way.
    for k in 0..(w / 25).max(1) {
        let bx = ((hash(k, 4) as i64 - (t / 3) as i64 * dir as i64).rem_euclid(w.max(1) as i64 + 20)) as i32 - 10;
        let by = 2 + (hash(k, 6) % (g.max(3) as u32 / 2 + 1)) as i32;
        cv.glyph(bx, by, if (t / 5 + k as u64) % 2 == 0 { 'v' } else { '^' }, rgb(if hour == 1 { (120, 120, 150) } else { (24, 22, 30) }));
    }
}
