//! A human castle on its hill: curtain walls and round towers, a tall
//! keep, banners and long pennants streaming in the wind, the town
//! crowded at its foot with smoke curling from the chimneys, and a knight
//! riding out down the road with a pennon on the lance. The seed picks the
//! stone, the heraldry, the hour and the lie of the hill.

use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the humans build tall, as if to see the end coming",
    "a king's walls, a king's taxes, a king's war",
    "trade with the castle; never trust its heralds",
    "pennants on the towers, crows on the gallows",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let tf = t as f32;
    let sky = cv.sky.min(h - 1).max(1);
    let g = (sky - 1).max(1); // the town's street
    let hour = cv.roll(1) % 3; // day, golden hour, dusk
    let (top, bot) = [((70, 130, 210), (190, 220, 240)), ((80, 110, 180), (250, 210, 150)), ((40, 34, 90), (230, 120, 90))][hour as usize];
    cv.sky_fill(g + 1, top, bot);
    for k in 0..(2 + w / 35) {
        let len = 7 + (hash(k, 9) % 10) as i32;
        let y = 1 + (hash(k, 8) % (g.max(4) as u32 / 2)) as i32;
        let x0 = ((hash(k, 7) as u64 + t / (16 + k as u64 * 4)) % (w as u64 + len as u64 + 10)) as i32 - len - 5;
        for i in 0..len {
            cv.glyph(x0 + i, y, if i == 0 || i == len - 1 { '▄' } else { '█' }, rgb(lerp(bot, (255, 250, 245), 0.55)));
        }
    }
    let stone = [(170, 168, 160), (200, 180, 140), (225, 222, 214), (120, 118, 124)][(cv.roll(2) % 4) as usize];
    let stone = if hour == 2 { lerp(stone, (60, 50, 80), 0.4) } else { stone };
    let dark = lerp(stone, (0, 0, 0), 0.35);
    let heraldry = [((180, 30, 40), (240, 200, 60)), ((30, 60, 160), (240, 240, 240)), ((30, 110, 60), (240, 220, 120)), ((20, 20, 26), (200, 40, 40))];
    let (h1, h2) = heraldry[(cv.roll(3) % 4) as usize];

    // The hill.
    let cx = (w as f32 * 0.78) as i32;
    let hill_h = (sky as f32 * 0.22) as i32;
    let hill = |x: i32| -> i32 {
        let d = (x - cx) as f32 / (w as f32 * 0.3).max(6.0);
        g - (hill_h as f32 * (1.0 - d * d).max(0.0)) as i32 - (ridge(x, w, seed, 3) * 1.5) as i32
    };
    for x in 0..w {
        let s = hill(x);
        for y in s.max(0)..=g {
            let ch = if hash(x, y + 5) % 6 == 0 { '"' } else { ' ' };
            cv.put(x, y, ch, rgb((90, 140, 60)), Some(rgb(lerp((70, 120, 50), (40, 70, 34), (y - s) as f32 / 6.0))));
        }
    }

    // The castle on its top.
    let big = w >= 110 && sky >= 16;
    let half = if big { 12 } else { 7 };
    let wall_top = hill(cx) - if big { 4 } else { 3 };
    let foot = |x: i32| hill(x).min(g);
    for x in cx - half..=cx + half {
        fill_col(cv, x, wall_top, foot(x), stone);
        if (x - cx) % 2 == 0 {
            cv.glyph(x, wall_top - 1, '▄', rgb(stone));
        }
    }
    // The gate.
    for y in (foot(cx) - 1).max(wall_top + 1)..=foot(cx) {
        cv.fill(cx, y, rgb((30, 22, 18)));
    }
    let mut poles = Vec::new();
    let tw = if big { 2 } else { 1 };
    let tower_top = wall_top - (if big { 4 } else { 3 }).min((wall_top - 3).max(1));
    for tx in [cx - half, cx + half] {
        for x in tx - tw..=tx + tw {
            fill_col(cv, x, tower_top, foot(x), if x == tx + tw { dark } else { stone });
            if (x - tx) % 2 == 0 {
                cv.glyph(x, tower_top - 1, '▄', rgb(stone));
            }
        }
        cv.glyph(tx, tower_top + 1, '▌', rgb((20, 16, 16)));
        poles.push((tx, tower_top - 2, 3));
    }
    // The keep, behind and above.
    let kw = if big { 3 } else { 2 };
    let kx = cx + if big { 3 } else { 2 };
    let keep_top = wall_top - (if big { 8 } else { 5 }).min((wall_top - 2).max(1));
    for x in kx - kw..=kx + kw {
        fill_col(cv, x, keep_top, wall_top - 1, if x == kx + kw { dark } else { lerp(stone, (255, 255, 255), 0.08) });
        if (x - kx) % 2 == 0 {
            cv.glyph(x, keep_top - 1, '▄', rgb(stone));
        }
    }
    let lit = hour == 2;
    for k in 0..2 {
        let y = keep_top + 1 + k * 2;
        if y < wall_top - 1 {
            cv.glyph(kx - 1 + k * 2, y, '▪', rgb(if lit && (t / 11 + k as u64) % 9 != 0 { (255, 210, 110) } else { (30, 26, 30) }));
        }
    }
    poles.push((kx, keep_top - 2, if big { 6 } else { 4 }));

    // Banners and pennants streaming from their poles.
    for (i, &(px, py, len)) in poles.iter().enumerate() {
        cv.glyph(px, py, '│', rgb((90, 70, 50)));
        cv.glyph(px, py + 1, '│', rgb((90, 70, 50)));
        for j in 0..len {
            let ripple = (j as f32 * 1.1 - tf * 0.25 + i as f32).sin();
            let ch = if ripple > 0.45 { '▀' } else if ripple < -0.45 { '▄' } else { '█' };
            let col = if j == len - 1 && len > 3 { h2 } else if (j + i as i32) % 3 == 1 { h2 } else { h1 };
            cv.glyph(px + 1 + j, py, ch, rgb(col));
        }
    }

    // The town crowding the foot of the hill.
    let roofs = [(170, 70, 50), (140, 90, 60), (110, 60, 50), (90, 90, 100)];
    let mut x = 1;
    let mut k = 0;
    while x < w - 1 {
        let wd = 3 + (hash(k, seed as i32) % 3) as i32;
        let near = (x - cx).abs() < half + 3;
        if !near {
            let ht = 1 + (hash(k, 4) % 2) as i32;
            let base = g;
            let roof = roofs[(hash(k, 5) % 4) as usize];
            for dx in 0..wd {
                for y in base - ht + 1..=base {
                    cv.put(x + dx, y, if dx == wd / 2 && y == base { '▌' } else { ' ' }, rgb((50, 34, 24)), Some(rgb((214, 200, 170))));
                }
                cv.glyph(x + dx, base - ht, if dx == 0 { '▄' } else if dx == wd - 1 { '▄' } else { '█' }, rgb(roof));
            }
            if lit && hash(k, 6) % 2 == 0 {
                cv.put(x + wd - 1, base, '▪', rgb((255, 210, 110)), Some(rgb((214, 200, 170))));
            }
            if hash(k, 7) % 3 == 0 {
                let chim = x + 1;
                cv.glyph(chim, base - ht - 1, '▌', rgb((90, 70, 60)));
                for j in 0..4 {
                    let age = ((t / 6) as i32 + j * 3 + k) % 12;
                    cv.glyph(chim + age / 4, base - ht - 2 - age / 2, if age < 5 { '░' } else { '·' }, mix((200, 200, 205), bot, age as f32 / 12.0));
                }
            }
        }
        x += wd + 1;
        k += 1;
    }

    // Fields below, under the menu, and the road from the gate.
    for y in (g + 1).max(0)..h {
        for x in 0..w {
            let strip = ((x + (y * 3)) / 7 + hash(x / 7, y / 3) as i32 % 3) % 3;
            let bg = [(110, 150, 60), (160, 150, 70), (90, 120, 50)][strip.rem_euclid(3) as usize];
            let ch = if hash(x, y) % 5 == 0 { '"' } else { ' ' };
            cv.put(x, y, ch, rgb(lerp(bg, (250, 240, 170), 0.3)), Some(rgb(lerp(bg, (30, 30, 40), if hour == 2 { 0.5 } else { 0.0 }))));
        }
    }
    let mut road = Vec::new();
    let pts = [(cx, foot(cx)), (cx - half - 4, g), (2, g + 3), (-2, h - 1)];
    for pair in pts.windows(2) {
        for c in line(pair[0], pair[1]) {
            if road.last() != Some(&c) {
                road.push(c);
            }
        }
    }
    for &(x, y) in &road {
        if y > g || x < cx - half {
            cv.put(x, y, ' ', Color::Black, Some(rgb((150, 130, 100))));
        }
    }
    // The knight riding out.
    if !road.is_empty() {
        let cycle = road.len() as u64 + 80;
        let i = ((t / 3 + cv.roll(5) as u64) % cycle) as usize;
        if i < road.len() {
            let (x, y) = road[i];
            cv.glyph(x, y, 'H', rgb((220, 220, 225)));
            cv.glyph(x, y - 1, '☺', rgb((200, 205, 215)));
            cv.glyph(x - 1, y - 2, '\\', rgb((150, 120, 80)));
            cv.glyph(x - 2, y - 3, if (t / 4) % 2 == 0 { '◄' } else { '◀' }, rgb(h1));
        }
    }

    // Birds over the towers.
    for k in 0..(1 + w / 60) {
        let a = tf * 0.05 + k as f32 * 2.5;
        let x = cx + (a.cos() * half as f32 * 1.4) as i32;
        let y = (keep_top - 3 + (a.sin() * 1.5) as i32).max(0);
        cv.glyph(x, y, if (t / 5 + k as u64) % 2 == 0 { 'v' } else { '^' }, rgb((30, 30, 40)));
    }
}

/// A column of masonry from `from` down to `to`.
fn fill_col(cv: &mut Canvas, x: i32, from: i32, to: i32, col: Rgb) {
    for y in from..=to {
        let ch = if hash(x, y) % 9 == 0 { '▒' } else { ' ' };
        cv.put(x, y, ch, rgb(lerp(col, (0, 0, 0), 0.15)), Some(rgb(col)));
    }
}
