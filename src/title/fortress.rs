//! A mountain under a night sky that turns through the seasons, the
//! fortress dug into it with its dwarves at work, and the magma sea glowing
//! far below. The seed shapes the mountain, sets the season it starts in,
//! how hard the weather falls and who is about.

use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "a fortress in the deep places of the world",
    "strike the earth, and keep on striking",
    "every great hall began as one dwarf with a pick",
    "the mountain gives, and the mountain keeps",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, sky_h, seed) = (cv.w(), cv.h(), cv.t, cv.sky, cv.seed);
    let magma_top = h - 3;
    let season = (t / SEASON_FRAMES + cv.roll(1) as u64) % 4;

    // The mountain's skyline, shaped by the seed.
    let rise = (sky_h - 7).max(1) as f32;
    let surf: Vec<i32> = (0..w).map(|x| sky_h - 1 - (ridge(x, w, seed, 10) * rise) as i32).collect();

    // Sky, stars, rock and magma.
    let rock_bg = rgb((26, 22, 20));
    for y in 0..h {
        for x in 0..w {
            if y < surf[x as usize] {
                let f = y as f32 / sky_h.max(1) as f32;
                cv.fill(x, y, mix((6, 8, 26), (44, 28, 58), f));
                let hs = hash(x * 7, y * 13);
                if hs % 53 == 0 && y < sky_h - 3 {
                    let twinkle = (t / 6 + hs as u64 / 53) % 23 == 0;
                    let (ch, fg) = if twinkle { ('+', (255, 255, 230)) } else { ('·', (150, 150, 190)) };
                    cv.glyph(x, y, ch, rgb(fg));
                }
            } else if y >= magma_top {
                // The magma sea, breathing.
                let wave = ((x as f32 * 0.35 + t as f32 * 0.08).sin() + (x as f32 * 0.11 - t as f32 * 0.05).sin()) * 0.25 + 0.5;
                let bubble = hash(x, (t / 9) as i32 + y * 31) % 97 == 0;
                let (ch, fg) = if bubble {
                    ('°', rgb((255, 240, 120)))
                } else {
                    (if (x + (t / 12) as i32) % 3 == 0 { '~' } else { '≈' }, mix((200, 40, 0), (255, 190, 50), wave))
                };
                cv.put(x, y, ch, fg, Some(mix((70, 10, 0), (130, 30, 0), wave)));
            } else {
                // Rock, with veins and gems that catch the light.
                let hs = hash(x * 3 + 1, y * 5 + 2);
                let deep = (y - sky_h).max(0) as f32 / (magma_top - sky_h).max(1) as f32;
                let heat = if y >= magma_top - 2 { 0.25 } else { 0.0 };
                let bg = mix((26, 22, 20), (60, 18, 8), deep * deep * 0.6 + heat);
                let (ch, fg) = match hs % 41 {
                    0 => ('☼', if (t / 5 + hs as u64) % 37 == 0 { rgb((255, 255, 255)) } else { rgb((90, 200, 220)) }),
                    1 | 2 => ('░', rgb((70, 60, 55))),
                    3 => ('£', rgb((150, 110, 60))),
                    _ => (' ', Color::Black),
                };
                cv.put(x, y, ch, fg, Some(bg));
            }
        }
    }
    let moon_x = w * (3 + (cv.roll(2) % 3) as i32) / 7;
    let moon_y = 1 + (cv.roll(3) % 2) as i32;
    if surf.get(moon_x.max(0) as usize).map_or(false, |&s| s > moon_y + 1) {
        cv.glyph(moon_x, moon_y, '●', rgb((245, 240, 210)));
    }

    // Drifting clouds.
    let clouds = 2 + (cv.roll(4) % 3) as i32;
    for k in 0..clouds {
        let len = 6 + (hash(k, 9) % 6) as i32;
        let y = 1 + (hash(k, 3) % 4) as i32;
        let x0 = ((hash(k, 5) as u64 + t / (14 + k as u64 * 5)) % (w as u64 + len as u64 + 10)) as i32 - len - 5;
        for i in 0..len {
            let x = x0 + i;
            if x >= 0 && x < w && y < surf[x as usize] {
                let edge = i == 0 || i == len - 1;
                cv.glyph(x, y, if edge { '▄' } else { '█' }, rgb((55, 50, 80)));
            }
        }
    }

    // The mountainside: a snowy crest, trees on the lower slopes in the
    // colours of the season.
    let tree_col = |x: i32| -> Color {
        let hs = hash(x, 77);
        match season {
            0 if hs % 3 == 0 => rgb((255, 175, 205)),
            0 => rgb((150, 215, 110)),
            1 => rgb((40, 170, 50)),
            2 => [rgb((235, 205, 60)), rgb((235, 135, 40)), rgb((205, 60, 40))][(hs % 3) as usize],
            _ => rgb((125, 95, 70)),
        }
    };
    let woods = 2 + (cv.roll(5) % 3);
    for x in 0..w {
        let s = surf[x as usize];
        let high = s < sky_h - 1 - (rise * 0.55) as i32;
        let snowy = high || season == 3;
        cv.put(x, s, '▲', if snowy { rgb((235, 240, 250)) } else { rgb((120, 110, 100)) }, Some(rock_bg));
        if !high && hash(x, 41) % woods == 0 && s > 0 {
            let ch = if hash(x, 43) % 4 == 0 { '♠' } else { '♣' };
            let col = if ch == '♠' { rgb((30, 120, 60)) } else { tree_col(x) };
            cv.glyph(x, s - 1, ch, col);
        }
    }

    // What the season brings down from the sky, lightly or hard.
    let fall = [w / 8, w / 3, w / 2][(cv.roll(6) % 3) as usize].max(4);
    for k in 0..fall {
        let speed = 3 + (k % 3) as u64;
        let x = ((hash(k, 1) as u64 + t / (speed * 3)) % w.max(1) as u64) as i32;
        let y = ((hash(k, 2) as u64 + t / speed) % sky_h.max(1) as u64) as i32;
        if y >= surf[x as usize] {
            continue;
        }
        let (ch, fg) = match season {
            0 => ('·', rgb((255, 190, 215))),
            1 if (t / 8 + k as u64) % 5 == 0 => ('·', rgb((230, 255, 120))),
            1 => continue,
            2 => ([',', '\'', '`'][(k % 3) as usize], [rgb((235, 135, 40)), rgb((205, 60, 40)), rgb((235, 205, 60))][(k % 3) as usize]),
            _ => ('*', rgb((240, 240, 255))),
        };
        cv.glyph(x, y, ch, fg);
    }

    // The fortress: two halls joined by a stair, a forge with its chimney,
    // a miner at the face.
    let floor_bg = rgb((10, 9, 9));
    let t1 = sky_h + 2;
    let t2 = (sky_h + (magma_top - sky_h) * 3 / 5).max(t1 + 3);
    if t2 >= magma_top - 1 || w < 30 {
        return;
    }
    let (a1, b1) = (3, w - 4);
    let (a2, b2) = (6, w - 9);
    for (row, a, b) in [(t1, a1, b1), (t2, a2, b2)] {
        for x in a..=b {
            cv.put(x, row, '.', rgb((70, 64, 58)), Some(floor_bg));
        }
    }
    let sx = w * 3 / 4;
    let top = surf[sx as usize];
    for y in top..=t2 {
        let ch = if y == top { '>' } else if y == t2 { '<' } else { 'X' };
        cv.put(sx, y, ch, rgb((160, 150, 140)), Some(floor_bg));
    }
    // The forge glows; smoke climbs its chimney and out over the slope.
    let fx = a1 + 3;
    for x in a1..a1 + 6 {
        let hs = hash(x, (t / 3) as i32);
        let fire = [rgb((255, 220, 80)), rgb((255, 140, 30)), rgb((220, 60, 20))][(hs % 3) as usize];
        cv.put(x, t1 - 1, if hs % 2 == 0 { '≈' } else { '~' }, fire, Some(rgb((60, 15, 0))));
    }
    let chimney_top = surf[fx as usize];
    for y in chimney_top + 1..t1 - 1 {
        cv.put(fx, y, '│', rgb((90, 80, 75)), Some(floor_bg));
    }
    for k in 0..6i32 {
        let age = ((t / 7) as i32 + k * 4) % 24;
        let y = chimney_top - 1 - age / 3;
        let x = fx + (age / 5) + ((hash(k, age) % 3) as i32 - 1) * (age / 12);
        if y >= 0 && y < surf.get(x.max(0) as usize).copied().unwrap_or(0) {
            let fade = 1.0 - age as f32 / 24.0;
            cv.glyph(x, y, if age < 10 { '░' } else { '·' }, mix((40, 36, 50), (150, 145, 150), fade));
        }
    }
    if (t / 4) % 5 == 0 {
        cv.glyph(a1 + 7, t1 - 1, '*', rgb((255, 255, 160)));
    }
    // A dining table or two on the lower hall.
    for x in [a2 + 3, a2 + 6, a2 + 9] {
        cv.put(x, t2, '╥', rgb((170, 120, 70)), Some(floor_bg));
    }
    // The miner hacks at the face; chips fly.
    cv.put(b2 + 1, t2, '█', rgb((110, 100, 95)), Some(rock_bg));
    cv.put(b2, t2, '☺', rgb((255, 215, 60)), Some(floor_bg));
    if (t / 5) % 3 == 0 {
        cv.glyph(b2 + 1, t2 - 1, '*', rgb((255, 255, 200)));
    }

    // Dwarves going about their business; who's up and about varies.
    let walkers = [
        (t1, a1 + 7, b1, 5u64, 0u64, (120, 200, 255)),
        (t1, a1 + 9, b1 - 4, 7, 40, (255, 150, 120)),
        (t1, a1 + 6, sx, 6, 90, (160, 255, 150)),
        (t2, a2 + 1, b2 - 1, 6, 15, (230, 180, 255)),
        (t2, a2 + 1, sx, 9, 60, (255, 230, 140)),
        (t1, sx + 1, b1, 8, 25, (255, 255, 255)),
        (t2, a2 + 4, b2 - 6, 11, 70, (140, 220, 220)),
    ];
    let skip = cv.roll(7);
    for (i, (row, a, b, every, off, col)) in walkers.into_iter().enumerate() {
        if i >= 2 && (skip >> i) & 1 == 1 {
            continue;
        }
        let x = pingpong(t + off * every, every, a, b);
        cv.glyph(x, row, '☺', rgb(col));
    }
    let climber = pingpong(t + 33, 10, top + 1, t2 - 1);
    cv.glyph(sx, climber, '☺', rgb((200, 220, 255)));
    // Some nights a cat patrols the upper hall.
    if cv.roll(8) % 3 == 0 {
        cv.glyph(pingpong(t + 500, 4, a1 + 8, b1 - 1), t1, 'c', rgb((230, 160, 80)));
    }

    // Something waits in a pocket of the deep caverns.
    let (cx, cy) = (w / 7, t2 + 2);
    if cy < magma_top - 1 {
        for dx in -3..=3 {
            cv.fill(cx + dx, cy, rgb((4, 3, 6)));
        }
        let eye = [(255, 40, 40), (120, 255, 80), (240, 240, 255)][(cv.roll(9) % 3) as usize];
        let blink = (t / 30) % 13;
        if blink >= 4 && blink != 9 {
            cv.glyph(cx - 1 + ((t / 120) % 3) as i32, cy, '•', rgb(eye));
            cv.glyph(cx + 1 + ((t / 120) % 3) as i32, cy, '•', rgb(eye));
        }
    }
}
