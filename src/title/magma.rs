//! The magma sea far below the fortress: a cavern lit red from beneath, a
//! fall of liquid fire pouring from the roof, an old stone bridge where
//! dwarves hurry across from their gate, and something vast that rises
//! out of the glow now and then to breathe fire at them before sinking
//! back. The seed picks the beast's hide, the heat of the sea and where
//! the beast surfaces.

use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "dig too deep and the deep looks back",
    "the sea below the sea is made of fire",
    "the bridge was built by braver dwarves than these",
    "it has been waiting since before the mountains",
];

// The beast, front on. Body cells: '█' and the half blocks; '●' eyes;
// '▼' fangs; the maw row is swapped for fire when it breathes.
const BEAST: [&str; 8] = [
    "▄▄         ▄▄",
    " ▀█▄     ▄█▀ ",
    "   ▐█████▌   ",
    "  ▐█●███●█▌  ",
    "  ▐███████▌  ",
    "   ▀█▼▼▼█▀   ",
    "  ▄███████▄  ",
    " ▐█████████▌ ",
];
const MAW: usize = 5;

/// One glyph as a 2x2 block, for the bigger beast.
fn grow(c: char) -> [[char; 2]; 2] {
    match c {
        '█' => [['█', '█'], ['█', '█']],
        '▄' => [[' ', ' '], ['█', '█']],
        '▀' => [['█', '█'], [' ', ' ']],
        '▐' => [[' ', '█'], [' ', '█']],
        '▌' => [['█', ' '], ['█', ' ']],
        '●' => [['▄', '▄'], ['▀', '▀']],
        '▼' => [['█', '█'], ['▼', '▼']],
        _ => [[' ', ' '], [' ', ' ']],
    }
}

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let ml = (cv.sky - 1).max(1).min(h - 1); // the sea's surface
    let tf = t as f32;
    let salt = cv.salt();
    let hides: [(Rgb, Rgb); 3] = [((150, 24, 18), (255, 120, 40)), ((44, 38, 44), (255, 140, 30)), ((170, 116, 44), (255, 220, 120))];
    let (hide, seam) = hides[(cv.roll(1) % 3) as usize];
    let (cool, hot) = [((140, 24, 0), (255, 180, 40)), ((170, 50, 0), (255, 235, 130)), ((110, 10, 10), (240, 110, 30))][(cv.roll(2) % 3) as usize];
    let big = w >= 120 && ml >= 18;

    // The cavern air, glowing redder towards the sea; the roof above.
    let ceil = |x: i32| -> i32 { (ridge(x, w, seed, 90) * (ml as f32 * 0.22)) as i32 };
    for y in 0..ml {
        let f = (y as f32 / ml.max(1) as f32).powf(1.6);
        let air = lerp((14, 6, 8), (120, 38, 12), f);
        for x in 0..w {
            if y <= ceil(x) {
                let ch = if hash(x + salt, y) % 7 == 0 { '░' } else { ' ' };
                cv.put(x, y, ch, rgb((50, 30, 28)), Some(rgb((30, 18, 18))));
            } else {
                cv.fill(x, y, rgb(air));
            }
        }
        if y == 0 {
            continue;
        }
    }
    // Stalactites.
    for x in 0..w {
        let hs = hash(x + salt, 91);
        if hs % 5 == 0 {
            let len = 1 + (hs / 5 % 3) as i32 + if big { 2 } else { 0 };
            let c = ceil(x);
            for k in 1..=len {
                cv.put(x, c + k, if k == len { '▼' } else { '█' }, rgb((44, 26, 24)), None);
            }
        }
    }
    // Far pillars, dark against the glow.
    let npill = (w / 40).max(1) + 1;
    for k in 0..npill {
        let px = (w as f32 * (0.25 + 0.5 * (hash(k, salt) % 100) as f32 / 100.0)) as i32;
        let pw = 2 + (hash(k, 3) % 3) as i32 + if big { 2 } else { 0 };
        for y in 0..ml {
            let wob = ((y as f32 * 0.5 + k as f32).sin() * 0.8) as i32;
            for x in px - pw / 2 + wob..=px + pw / 2 + wob {
                let f = (y as f32 / ml.max(1) as f32).powf(1.6);
                cv.put(x, y, ' ', Color::Black, Some(mix((26, 12, 12), (70, 22, 10), f)));
            }
        }
    }

    // The fall of fire from the roof.
    let mfx = (w as f32 * (0.3 + (cv.roll(3) % 15) as f32 / 100.0)) as i32;
    let mfw = if big { 3 } else { 1 };
    for x in mfx..mfx + mfw {
        for y in ceil(x).max(0)..ml {
            let hs = hash(x, y - (t / 2) as i32);
            let col = lerp(cool, hot, 0.6 + (hs % 40) as f32 / 100.0);
            cv.put(x, y, if hs % 3 == 0 { '║' } else { '│' }, rgb(lerp(col, (255, 255, 200), 0.3)), Some(rgb(col)));
        }
    }

    // The sea.
    let wave = |x: i32| -> i32 { if ((x as f32 * 0.31 + tf * 0.07).sin() + (x as f32 * 0.11 - tf * 0.04).sin()) > 1.2 { 1 } else { 0 } };
    for y in (ml - 1).max(0)..h {
        for x in 0..w {
            let top = ml - wave(x);
            if y < top {
                continue;
            }
            let (fx, fy) = (x as f32, y as f32);
            let n = ((fx * 0.21 + tf * 0.05 + fy * 0.7).sin() * 0.5 + (fx * 0.07 - tf * 0.03 + fy * 0.3).sin() * 0.5) * 0.5 + 0.5;
            let crust = (fx * 0.09 + fy * 0.45 + tf * 0.006).sin() + (fx * 0.05 - fy * 0.8 - tf * 0.004).sin() + (fx * 0.37 + fy * 1.3).sin() * 0.3 > 1.4;
            let near = 1.0 - ((y - ml) as f32 / 6.0).clamp(0.0, 1.0);
            let hs = hash(x * 7 + salt, y * 3 + (t / 9) as i32);
            if crust && y > ml {
                let crack = hs % 5 == 0;
                cv.put(x, y, if crack { '~' } else { '▓' }, rgb(if crack { hot } else { (34, 20, 18) }), Some(rgb((60, 24, 14))));
                continue;
            }
            let col = lerp(cool, hot, n * 0.7 + near * 0.3);
            let crest = y == top && top < ml;
            let ch = if crest { '▄' } else if y == top || hs % 4 == 0 { '≈' } else if hs % 4 == 1 { '~' } else { ' ' };
            let fg = if crest { lerp(hot, (255, 255, 220), 0.3) } else { lerp(col, hot, 0.5) };
            cv.put(x, y, ch, rgb(fg), Some(rgb(col)));
        }
    }
    // Where the fall strikes the sea.
    cv.halo(mfx, ml, 2.5 + if big { 2.0 } else { 0.0 }, (255, 230, 120), 0.6);
    for k in 0..5 {
        let age = ((t / 2) as i32 + k * 4) % 12;
        let dx = if k % 2 == 0 { age / 2 + 1 } else { -age / 2 - 1 };
        cv.glyph(mfx + dx, ml - 1 - (age * (12 - age)) / 16, '·', rgb(hot));
    }

    // The gate in the cliff, and the bridge out from it.
    let lw = (w as f32 * 0.14) as i32 + 3;
    let ltop = (ml - (ml as f32 * 0.55) as i32).max(1);
    let bb = (ml - 2 - (ml - 8).max(0) / 3).max(1);
    for y in ltop..h.min(ml + 2) {
        let edge = lw + ((y - ltop) as f32 * 0.3) as i32 + (hash(y, salt) % 3) as i32 - 1;
        for x in 0..edge {
            let lit = (1.0 - (edge - x) as f32 / 4.0).max(0.0) * 0.5;
            let ch = if hash(x + salt, y * 5) % 9 == 0 { '░' } else { ' ' };
            cv.put(x, y, ch, rgb((70, 44, 40)), Some(mix((36, 24, 24), (110, 50, 26), lit)));
        }
    }
    let gx = lw / 2;
    if bb - 1 > ltop {
        for y in (bb - 2).max(ltop)..bb {
            cv.put(gx, y, ' ', Color::Black, Some(rgb((255, 190, 90))));
            cv.put(gx + 1, y, ' ', Color::Black, Some(rgb((255, 190, 90))));
        }
        cv.glyph(gx, bb - 3, '∩', rgb((120, 100, 90)));
        cv.glyph(gx + 1, bb - 3, '∩', rgb((120, 100, 90)));
        cv.glyph(gx - 2, bb - 1, '☺', rgb((150, 200, 240)));
        cv.glyph(gx - 3, bb - 2, '↑', rgb((190, 190, 200)));
    }
    let far = w - (w as f32 * 0.04) as i32 - 2;
    for y in bb..h.min(ml + 2) {
        for x in far..w {
            cv.put(x, y, ' ', Color::Black, Some(rgb((36, 24, 24))));
        }
    }
    let span = (w / 6).clamp(8, 30);
    for x in lw..far {
        cv.put(x, bb, '▀', rgb((90, 76, 74)), Some(rgb((50, 40, 42))));
        if (x - lw) % 3 == 0 {
            cv.glyph(x, bb - 1, '▄', rgb((70, 60, 60)));
        }
        let into = (x - lw) % span;
        if into <= 1 || into >= span - 1 {
            for y in bb + 1..ml {
                cv.put(x, y, ' ', Color::Black, Some(rgb((46, 34, 36))));
            }
        } else if big {
            let u = (into as f32 / span as f32) * 2.0 - 1.0;
            let depth = ((1.0 - u * u).sqrt() * 2.0) as i32;
            for y in bb + 1..bb + 1 + (2 - depth).max(0) {
                cv.put(x, y, ' ', Color::Black, Some(rgb((50, 40, 42))));
            }
        }
    }

    // The beast's comings and goings.
    let bw = if big { 26 } else { 13 };
    let bh = if big { 16 } else { 8 };
    let period = 640u64;
    let local = (t + cv.roll(4) as u64 % period) % period;
    let bc = if w < 110 { w - bw / 2 - 4 } else { (w as f32 * (0.62 + (cv.roll(5) % 12) as f32 / 100.0)) as i32 };
    let up = match local {
        0..=89 => local as f32 / 90.0,
        90..=399 => 1.0,
        400..=479 => 1.0 - (local - 400) as f32 / 80.0,
        _ => 0.0,
    };
    let breathing = (150..260).contains(&local) || (300..360).contains(&local);
    let x0 = bc - bw / 2;
    if up > 0.0 {
        let sway = ((tf * 0.05).sin() * if big { 2.0 } else { 1.0 }).round() as i32;
        let top = ml - (bh as f32 * up).round() as i32 + 1;
        let blink = (t / 4) % 40 == 0;
        for (r, row) in BEAST.iter().enumerate() {
            for (c, ch) in row.chars().enumerate() {
                let cells: Vec<(i32, i32, char)> = if big {
                    let g = grow(if r == MAW && breathing && ch == '▼' { '█' } else { ch });
                    vec![
                        (c as i32 * 2, r as i32 * 2, g[0][0]),
                        (c as i32 * 2 + 1, r as i32 * 2, g[0][1]),
                        (c as i32 * 2, r as i32 * 2 + 1, g[1][0]),
                        (c as i32 * 2 + 1, r as i32 * 2 + 1, g[1][1]),
                    ]
                } else {
                    vec![(c as i32, r as i32, ch)]
                };
                for (dx, dy, ch) in cells {
                    let (x, y) = (x0 + dx + sway, top + dy);
                    if ch == ' ' || y >= ml {
                        continue;
                    }
                    let eye = (r == 3) && row.chars().nth(c) == Some('●');
                    let fang = r == MAW && row.chars().nth(c) == Some('▼');
                    if eye {
                        let col = if blink { hide } else { (255, 235, 100) };
                        cv.put(x, y, ch, rgb(col), Some(rgb(lerp(hide, (0, 0, 0), 0.3))));
                    } else if fang && breathing {
                        let hs = hash(x, (t / 2) as i32 + y);
                        cv.put(x, y, '▒', rgb((255, 240, 150)), Some(rgb([(255, 170, 50), (240, 90, 20)][(hs % 2) as usize])));
                    } else if fang {
                        cv.put(x, y, if big && dy % 2 == 0 { '█' } else { '▼' }, rgb((235, 225, 200)), Some(rgb((30, 8, 6))));
                    } else if ch == '█' {
                        let crack = hash(x * 3 + salt, y * 5) % 9 == 0;
                        let pulse = ((tf * 0.08 + x as f32 * 0.3).sin() * 0.5 + 0.5) * 0.6 + 0.4;
                        if crack {
                            cv.put(x, y, '▒', mix(hide, seam, pulse), Some(rgb(hide)));
                        } else {
                            cv.put(x, y, ' ', Color::Black, Some(rgb(hide)));
                        }
                    } else {
                        cv.glyph(x, y, ch, rgb(hide));
                    }
                }
            }
        }
        // Eyes light the dark; the sea boils where it rises.
        let ey = top + if big { 6 } else { 3 };
        if !blink && ey < ml {
            cv.halo(x0 + sway + if big { 9 } else { 4 }, ey, 1.5, (255, 220, 90), 0.4);
            cv.halo(x0 + sway + if big { 17 } else { 8 }, ey, 1.5, (255, 220, 90), 0.4);
        }
        for k in 0..bw + 6 {
            let x = x0 - 3 + k;
            if (x + (t / 3) as i32) % 3 == 0 {
                cv.glyph(x, ml, '▀', rgb((255, 245, 200)));
            }
        }
        // Fire from the maw, raking towards the bridge.
        if breathing && up >= 1.0 {
            let my = top + if big { MAW as i32 * 2 + 1 } else { MAW as i32 };
            let mx = x0 + sway + bw / 2;
            let len = ((mx - lw) as f32 * 0.9) as i32;
            let into = if (150..260).contains(&local) { local - 150 } else { local - 300 };
            let reach = (into as i32 * 3).min(len);
            for i in 0..reach.max(0) {
                let x = mx - i;
                let fy = my as f32 - (i as f32 * 0.22).min((my - bb + 1).max(0) as f32) + (i as f32 * 0.4 - tf * 0.5).sin() * 0.6;
                let spread = (i / 6).min(if big { 5 } else { 2 });
                for dy in -spread..=spread {
                    let y = fy.round() as i32 + dy;
                    let hs = hash(x + dy * 17, (t / 2) as i32 + y);
                    if dy != 0 && hs % 3 == 0 {
                        continue;
                    }
                    let col = [(255, 245, 160), (255, 175, 60), (235, 85, 25)][((i as u32 / 4 + hs) % 3) as usize];
                    let ch = ['▓', '▒', '░'][((dy.unsigned_abs() + hs % 2) % 3) as usize];
                    cv.glyph(x, y, ch, rgb(col));
                }
                if i % 5 == 0 {
                    cv.halo(x, fy.round() as i32, 2.0 + spread as f32, (255, 140, 40), 0.25);
                }
            }
        }
    } else if local > 560 {
        // Bubbles gathering where it will come up.
        for k in 0..((local - 560) / 10 + 1) as i32 {
            let x = x0 + (hash(k, (t / 8) as i32) % bw as u32) as i32;
            cv.glyph(x, ml, if (t / 3 + k as u64) % 2 == 0 { 'o' } else { 'O' }, rgb((255, 230, 140)));
        }
    }

    // Dwarves hurrying over the bridge; they duck when the fire comes.
    let crossers = (w / 30).clamp(2, 6);
    let blen = (far - lw).max(1);
    for k in 0..crossers {
        let x = lw + ((t / 4 + (k as u64) * (blen as u64 / crossers as u64 + 3)) % blen as u64) as i32;
        let duck = breathing && up >= 1.0;
        let col = [(150, 200, 240), (240, 170, 110), (200, 230, 150), (230, 150, 210)][(k % 4) as usize];
        cv.glyph(x, bb - 1, if duck { '☻' } else { '☺' }, rgb(col));
        if k % 2 == 0 && !duck {
            cv.glyph(x - 1, bb - 1, 'Θ', rgb((150, 100, 50)));
        }
    }

    // Embers rising off the sea.
    let n = (w * ml / 60).max(4);
    for k in 0..n {
        let life = 40 + (hash(k, 9) % 40) as u64;
        let age = (t + hash(k, 11) as u64) % life;
        let x = (hash(k, 13) % w.max(1) as u32) as i32 + ((age as f32 * 0.15 + k as f32).sin() * 1.5) as i32;
        let y = ml - 1 - (age as i32 * ml / life as i32 / 2);
        let col = lerp(hot, (120, 30, 10), age as f32 / life as f32);
        cv.glyph(x, y, '·', rgb(col));
    }
}
