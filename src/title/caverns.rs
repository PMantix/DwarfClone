//! The first cavern: glowing fungus trees, a waterfall feeding a still
//! lake where a dwarf fishes, spores drifting up through the dark, and
//! something vast walking slowly along the back wall. Below the ledge, a
//! deeper cavern glows with more of the same. The seed picks the fungus
//! light, the beast's shape and eyes, and where things stand.

use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "what grows where the sun has never been",
    "the fishing is good in the first cavern",
    "do not wake what walks beneath",
    "light without a sun, water without a sky",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, sky, seed) = (cv.w(), cv.h(), cv.t, cv.sky, cv.seed);
    let g = sky - 1; // the ledge's floor
    let low = h - 2; // the deep cavern's floor
    let (glow, dim) = [
        ((80, 225, 205), (16, 46, 54)),
        ((190, 115, 255), (40, 18, 60)),
        ((160, 255, 110), (24, 52, 18)),
        ((255, 175, 85), (60, 32, 12)),
    ][(cv.roll(1) % 4) as usize];
    let air = (7, 6, 12);

    // Fungus trees on both floors, so their light can be laid on first.
    let lake = (w * 2 / 5, w * 7 / 10);
    let fisher_x = lake.0 - 2;
    let salt = cv.salt();
    let mut trees: Vec<(i32, i32, i32)> = Vec::new(); // (x, floor, height)
    let mut x = 2 + (cv.roll(2) % 4) as i32;
    while x < w - 1 {
        let near_lake = x >= fisher_x - 2 && x <= lake.1 + 1;
        if !near_lake {
            trees.push((x, g, 2 + (hash(x + salt, 3) % (g.max(4) as u32 / 2).max(1)) as i32));
        }
        x += 5 + (hash(x + salt, 7) % 6) as i32;
    }
    let mut x = 3 + (cv.roll(3) % 5) as i32;
    while x < w - 1 && low > g + 6 {
        trees.push((x, low, 3 + (hash(x + salt, 9) % ((low - g - 4) as u32 / 2).max(1)) as i32));
        x += 4 + (hash(x + salt, 11) % 7) as i32;
    }
    let pulse = |k: i32| ((t as f32 * 0.05 + k as f32 * 1.7).sin() * 0.5 + 0.5) * 0.35 + 0.65;

    // Cavern air, lit by the fungus.
    for y in 0..h {
        for x in 0..w {
            let mut light: f32 = 0.0;
            for &(tx, fl, th) in &trees {
                if (x - tx).abs() > 14 {
                    continue;
                }
                let (dx, dy) = ((x - tx) as f32 * 0.5, (y - (fl - th)) as f32);
                let d = (dx * dx + dy * dy).sqrt();
                light = light.max((1.0 - d / 7.0) * pulse(tx));
            }
            cv.fill(x, y, mix(air, dim, light));
        }
    }

    // The beast walks along the back of the upper cavern.
    let brute = cv.roll(4) % 2 == 0;
    let eye = [(255, 50, 40), (120, 255, 90), (250, 250, 255), (255, 200, 40)][(cv.roll(5) % 4) as usize];
    let (bw, bh) = if brute { (14, 4) } else { (24, 2) };
    let bx = pingpong(t + (seed % 3000), 7, -bw - 4, w + 4);
    let going_right = ((t + (seed % 3000)) / 7) % (2 * (w + bw + 8) as u64) < (w + bw + 8) as u64;
    let feet = g - 1;
    let shadow = rgb((2, 1, 4));
    let hide = rgb((40, 30, 48));
    if brute {
        for dy in 0..bh {
            let y = feet - 1 - dy;
            let r = 1.0 - (dy as f32 / bh as f32).powi(2) * 0.6;
            let half = (bw as f32 / 2.0 * r) as i32;
            for dx in -half..=half {
                cv.put(bx + bw / 2 + dx, y, '▓', hide, Some(shadow));
            }
        }
        for k in 0..4 {
            cv.put(bx + 3 + k * 3, feet - bh - 1, '▲', rgb((60, 48, 66)), None);
        }
        let stride = (t / 10) % 2 == 0;
        for (k, lx) in [2, 5, bw - 6, bw - 3].iter().enumerate() {
            let ch = if stride == (k % 2 == 0) { '/' } else { '\\' };
            cv.put(bx + lx, feet, ch, rgb((24, 18, 28)), Some(shadow));
        }
    } else {
        for i in 0..bw {
            let y = feet - 1 - (((i as f32 * 0.5 + t as f32 * 0.08).sin() + 1.0) * 0.5) as i32;
            cv.put(bx + i, y, '▓', hide, Some(shadow));
            cv.put(bx + i, y - 1, if i % 3 == 0 { '▲' } else { '▄' }, rgb((60, 48, 66)), Some(shadow));
        }
    }
    let blink = (t / 25) % 17 == 0;
    if !blink {
        let head = if going_right { bx + bw - 2 } else { bx + 1 };
        let ey = if brute { feet - bh + 1 } else { feet - 2 };
        cv.glyph(head, ey, '•', rgb(eye));
        cv.glyph(head + 1, ey, '•', rgb(eye));
    }

    // Stalactites from the roof.
    for x in 0..w {
        let hang = (hash(x, 21) % 4) as i32;
        for y in 0..hang {
            cv.fill(x, y, rgb((28, 24, 28)));
        }
        if hang > 0 {
            cv.glyph(x, hang, '▼', rgb((60, 54, 58)));
        }
    }

    // The ledge, the deep floor, and the rock between.
    let rock = rgb((34, 28, 32));
    for y in g.max(0)..h {
        let open = y > g + 2 && y < low;
        for x in 0..w {
            if open {
                continue;
            }
            let hs = hash(x * 3, y * 7);
            let ch = if y == g || y == low {
                '▀'
            } else if hs % 29 == 0 {
                '♦'
            } else if hs % 7 == 0 {
                '░'
            } else {
                ' '
            };
            let fg = if ch == '♦' {
                if (t / 4 + hs as u64) % 41 == 0 {
                    (255, 255, 255)
                } else {
                    glow
                }
            } else {
                (74, 64, 66)
            };
            cv.put(x, y, ch, rgb(fg), Some(rock));
        }
    }

    // Spores drifting up through the dark.
    for k in 0..(w / 3).max(4) {
        let speed = 5 + (k % 4) as u64;
        let y = h - 1 - ((hash(k, 2) as u64 + t / speed) % h.max(1) as u64) as i32;
        let x = (hash(k, 1) % w.max(1) as u32) as i32 + ((y as f32 * 0.4 + k as f32).sin() * 2.0) as i32;
        if (t / 9 + k as u64) % 3 != 0 {
            cv.glyph(x, y, '·', rgb(glow));
        }
    }

    // The waterfall from the roof, a stream, and the lake.
    let wf = (w / 16 + 2).min(w - 1);
    for y in 0..g {
        let ch = if (y as u64 + t / 2) % 3 == 0 { '│' } else { '║' };
        cv.put(wf, y, ch, rgb((150, 200, 240)), Some(rgb((30, 60, 100))));
    }
    if (t / 3) % 2 == 0 {
        cv.glyph(wf - 1, g - 1, '°', rgb((200, 230, 255)));
    } else {
        cv.glyph(wf + 1, g - 1, '°', rgb((200, 230, 255)));
    }
    let water = |x: i32, y: i32| {
        let wave = ((x as f32 * 0.5 + t as f32 * 0.07).sin() + (x as f32 * 0.17 - t as f32 * 0.04 + y as f32).sin()) * 0.25 + 0.5;
        (if (x + (t / 14) as i32) % 4 == 0 { '~' } else { '≈' }, mix((60, 110, 170), lerp((120, 170, 220), glow, 0.3), wave), mix((12, 26, 52), (20, 40, 70), wave))
    };
    for x in wf..lake.0 {
        let (ch, fg, bg) = water(x, g);
        cv.put(x, g, ch, fg, Some(bg));
    }
    for y in g..(g + 2).min(h) {
        for x in lake.0..=lake.1 {
            let (ch, fg, bg) = water(x, y);
            cv.put(x, y, ch, fg, Some(bg));
        }
    }

    // The fisher on the shore, line in the water.
    cv.glyph(fisher_x, g - 1, '☺', rgb((255, 215, 120)));
    cv.glyph(fisher_x + 1, g - 2, '/', rgb((150, 110, 70)));
    cv.glyph(fisher_x + 2, g - 3, '/', rgb((150, 110, 70)));
    cv.glyph(fisher_x + 3, g - 2, '\\', rgb((110, 110, 120)));
    cv.glyph(fisher_x + 4, g - 1, '\\', rgb((110, 110, 120)));
    let bob = if (t / 20) % 3 == 0 { 'o' } else { '°' };
    cv.glyph(fisher_x + 5, g, bob, rgb((255, 90, 70)));
    // Now and then a fish leaps.
    let leap = (t + seed % 500) % 400;
    if leap < 24 && lake.1 - lake.0 > 6 {
        let fx = lake.0 + 6 + (hash((t / 400) as i32, 5) % (lake.1 - lake.0 - 5).max(1) as u32) as i32;
        let s = leap as f32 / 24.0;
        let y = g - (s * (1.0 - s) * 8.0) as i32;
        cv.glyph(fx + (s * 4.0) as i32, y, if s < 0.5 { '<' } else { '«' }, rgb((200, 220, 230)));
    }

    // The fungus trees themselves.
    for &(tx, fl, th) in &trees {
        let p = pulse(tx);
        let cap = lerp(dim, glow, p);
        for y in (fl - th + 1)..fl {
            cv.glyph(tx, y, '│', rgb((190, 180, 160)));
        }
        let wide = hash(tx, fl) % 2 == 0;
        let y = fl - th;
        if wide {
            cv.sprite(tx - 2, y, &["▄███▄"], rgb(cap));
        } else {
            cv.sprite(tx - 1, y, &["▄█▄"], rgb(cap));
        }
        if hash(tx, 31) % 3 == 0 {
            cv.glyph(tx + 2, fl - 1, '♠', rgb(lerp(cap, (0, 0, 0), 0.3)));
        }
    }

    // A deep pool and a wanderer on the lower floor.
    if low > g + 6 {
        let px = w - w / 4;
        for x in px..(px + w / 8).min(w) {
            let (ch, fg, bg) = water(x, low);
            cv.put(x, low, ch, fg, Some(bg));
        }
        let crawler = pingpong(t + 200, 9, 2, px - 2);
        cv.glyph(crawler, low - 1, 'ö', rgb((170, 150, 120)));
    }
}
