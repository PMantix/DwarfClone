//! A funeral at dusk: a winding path up a hillside to a tomb cut into the
//! rock, and a slow procession climbing it by torchlight, the bier carried
//! shoulder-high, mourners behind, a piper at the head. The tomb door
//! glows as they reach it and they pass inside, and the next sad company
//! starts up from below. The seed picks the dusk, the hill and the length
//! of the line.

use super::a_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "carved in stone, remembered in song",
    "the mountain takes back its own",
    "a good life, a long rest, a name on the wall",
    "light a torch for the ones who dug before us",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let g = (cv.sky - 1).max(1);
    let tf = t as f32;
    let salt = cv.salt();
    let (top, bot) = [((20, 18, 50), (180, 90, 90)), ((24, 24, 60), (130, 90, 130)), ((16, 20, 44), (210, 130, 80))][(cv.roll(1) % 3) as usize];

    cv.sky_fill(g, top, bot);
    cv.stars(g / 3, 41);
    // The sun just gone, a last band of light.
    for x in 0..w {
        cv.glow(x, g - 1, (255, 170, 110), 0.2);
    }

    // The hill rising on one side, the tomb near its crown.
    let hx = (w as f32 * 0.7) as i32 + (cv.roll(2) % 6) as i32;
    let hw = (w as f32 * 0.32).max(10.0);
    let rise = (g - 2).max(1) as f32;
    let surf: Vec<i32> = (0..w)
        .map(|x| {
            let u = (x - hx) as f32 / hw;
            let hill = (1.0 - u * u).max(0.0).powf(0.8);
            g - 1 - ((hill * 0.82).max(ridge(x, w, seed, 180) * 0.25) * rise) as i32
        })
        .collect();
    for x in 0..w {
        let s = surf[x as usize];
        for y in s.max(0)..g {
            let d = (y - s) as f32;
            let bg = lerp(lerp((50, 50, 60), (36, 40, 46), (d / 6.0).min(1.0)), top, 0.2);
            let ch = if y == s { '▄' } else if hash(x + salt, y) % 9 == 0 { '"' } else { ' ' };
            cv.put(x, y, ch, rgb(lerp(bg, (120, 130, 110), 0.5)), Some(rgb(bg)));
        }
        if hash(x + salt, 9) % 6 == 0 && (x - hx).abs() > 3 {
            cv.glyph(x, s - 1, '♠', rgb((20, 30, 30)));
        }
    }
    // The low ground, dark already.
    for y in g.max(0)..h {
        for x in 0..w {
            let hs = hash(x * 3 + salt, y * 5);
            let ch = if hs % 9 == 0 { '"' } else if hs % 31 == 0 { '♠' } else { ' ' };
            cv.put(x, y, ch, rgb((40, 56, 46)), Some(rgb(lerp((26, 30, 36), (16, 18, 26), (y - g) as f32 / (h - g).max(1) as f32))));
        }
    }

    // The tomb: a door with a lintel cut into the hill near its top.
    let ty = surf[hx.clamp(0, w - 1) as usize] + 2;
    let door = (hx, ty.min(g - 1));
    // The path: switchbacks up the hill from the foot to the door.
    let foot = (hx - hw as i32 - 4, g - 1);
    let mut points = vec![foot];
    let turns = ((door.1 - foot.1).abs() / 3).clamp(1, 4);
    for i in 1..turns {
        let y = foot.1 - (foot.1 - door.1) * i / turns;
        let left = i % 2 == 0;
        let half = (hw * (1.0 - i as f32 / turns as f32) * 0.8) as i32;
        let x = if left { hx - half - 2 } else { hx + half / 2 };
        let x = x.clamp(0, w - 1);
        let y = y.max(surf[x as usize]);
        points.push((x, y));
    }
    points.push(door);
    let mut path: Vec<(i32, i32)> = Vec::new();
    for pair in points.windows(2) {
        for c in line(pair[0], pair[1]) {
            if path.last() != Some(&c) {
                path.push(c);
            }
        }
    }
    for &(x, y) in &path {
        cv.put(x, y, if hash(x, y) % 3 == 0 { '·' } else { ' ' }, rgb((110, 100, 90)), Some(rgb((60, 56, 56))));
    }

    // The procession along the path, then into the tomb.
    let mut line_up: Vec<(char, Rgb)> = vec![('☺', (70, 90, 70)), ('♪', (200, 190, 150)), (' ', (0, 0, 0)), ('¡', (255, 200, 90))];
    line_up.extend([('☺', (60, 60, 70)), ('▬', (150, 110, 60)), ('▬', (150, 110, 60)), ('☺', (60, 60, 70))]);
    let mourners = 4 + (cv.roll(3) % 6) as usize;
    for k in 0..mourners {
        line_up.push((' ', (0, 0, 0)));
        line_up.push(('☺', [(70, 60, 80), (60, 60, 70), (80, 70, 60)][k % 3]));
        if k % 2 == 1 {
            line_up.push(('¡', (255, 200, 90)));
        }
    }
    let len = path.len() as u64;
    let cycle = len + line_up.len() as u64 + 90;
    let head = ((t + seed % 997) / 8) % cycle.max(1);
    let inside = head as i64 - len as i64; // how many have gone in
    for (j, &(ch, col)) in line_up.iter().enumerate() {
        let idx = head as i64 - j as i64;
        if ch == ' ' || idx < 0 || idx >= len as i64 {
            continue;
        }
        let (x, y) = path[idx as usize];
        if ch == '▬' {
            // The bier rides on the bearers' shoulders.
            cv.glyph(x, y - 1, ch, rgb(col));
            cv.glyph(x, y, '☺', rgb((60, 60, 70)));
        } else if ch == '¡' {
            let fl = (tf * 0.5 + j as f32).sin() * 0.1 + 0.45;
            cv.glyph(x, y - 1, ch, rgb((255, 210, 110)));
            cv.halo(x, y - 1, 2.5, (255, 160, 70), fl);
        } else if ch == '♪' {
            if (t / 10 + j as u64) % 3 == 0 {
                cv.glyph(x + 1, y - 1 - ((t / 10) % 2) as i32, ch, rgb(col));
            }
        } else {
            cv.glyph(x, y, ch, rgb(col));
        }
    }
    // The door, glowing while they go in.
    let glow = if inside > 0 && inside < line_up.len() as i64 + 40 { 0.7 } else { 0.15 };
    for dx in -1..=1 {
        cv.fill(door.0 + dx, door.1, rgb(lerp((24, 20, 18), (255, 180, 90), glow * 0.6)));
        cv.glyph(door.0 + dx, door.1 - 1, '▄', rgb((140, 130, 120)));
    }
    cv.glyph(door.0, door.1 - 2, '†', rgb((180, 170, 150)));
    cv.halo(door.0, door.1, 3.0, (255, 170, 80), glow * 0.6);
    // Candles left by earlier mourners.
    for k in [-3, 3] {
        let on = (t / 4 + k as u64) % 9 != 0;
        cv.glyph(door.0 + k, door.1, '¡', rgb(if on { (255, 220, 120) } else { (200, 150, 80) }));
    }

    // Crows going home to roost.
    for k in 0..(w / 40).max(2) {
        let x = drift(t, k + salt, 3, w + 10) - 5;
        let y = 2 + (hash(k, 3) % (g / 2).max(1) as u32) as i32;
        cv.glyph(x, y, if (t / 6 + k as u64) % 2 == 0 { 'v' } else { '^' }, rgb((10, 10, 16)));
    }
}
