//! Deep winter: an aurora rippling over snowed-in hills, a hold dug into
//! the slope with its windows lit and its chimney smoking, a dwarf
//! shovelling out the door, and a frozen lake below where someone fishes
//! through the ice. The seed picks the aurora's colours, the snowfall and
//! which side the hold is on.

use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the long night, and the lights that dance in it",
    "snowed in, stocked up, singing",
    "winter keeps the goblins home",
    "a warm hearth under a cold mountain",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, sky, seed) = (cv.w(), cv.h(), cv.t, cv.sky, cv.seed);
    let g = sky - 1;
    let (lo, hi) = [((40, 220, 120), (60, 160, 230)), ((50, 230, 140), (200, 70, 200)), ((60, 200, 220), (140, 80, 240))][(cv.roll(1) % 3) as usize];
    let night_top = (4, 6, 20);
    let night_bot = (16, 22, 46);
    let p = (cv.roll(2) % 628) as f32 / 100.0;
    let tf = t as f32;

    // The sky with its aurora curtains.
    for y in 0..g.min(h) {
        for x in 0..w {
            let base = lerp(night_top, night_bot, y as f32 / g.max(1) as f32);
            let fx = x as f32;
            let c = g as f32 * 0.22 + (fx * 0.07 + tf * 0.015 + p).sin() * g as f32 * 0.12 + (fx * 0.19 - tf * 0.011).sin() * g as f32 * 0.05;
            let th = (g as f32 * 0.35).max(2.0);
            let dy = y as f32 - c;
            let v = if dy >= 0.0 && dy < th { 1.0 - dy / th } else if dy < 0.0 { (1.0 + dy).max(0.0) } else { 0.0 };
            let i = 0.3 + 0.7 * ((fx * 0.05 - tf * 0.02 + p * 2.0).sin() * 0.5 + 0.5).powi(2);
            let col = lerp(lo, hi, (dy / th).clamp(0.0, 1.0));
            let a = v * i;
            cv.fill(x, y, rgb(lerp(base, col, a * 0.7)));
            if a > 0.35 && hash(x, (t / 5) as i32 + y * 3) % 4 == 0 {
                cv.glyph(x, y, '│', rgb(lerp(col, (255, 255, 255), 0.3)));
            } else if a < 0.1 && hash(x * 7 + 3, y * 13 + 1) % 37 == 0 {
                let twinkle = (t / 6 + x as u64) % 29 == 0;
                cv.glyph(x, y, if twinkle { '+' } else { '·' }, rgb(if twinkle { (255, 255, 230) } else { (150, 150, 190) }));
            }
        }
    }

    // Where the hold is dug: a big hill towards one side.
    let left = cv.roll(4) % 2 == 0;
    let off = (cv.roll(6) % (w.max(12) as u32 / 12)) as i32;
    let hx = if left { w / 6 + off } else { w - 1 - w / 6 - off };
    let salt = cv.salt();

    // Far mountains, blue in the night.
    let far = (sky - 2).max(1) as f32 * 0.8;
    for x in 0..w {
        let m = peaks(x, w, seed, 55, 4);
        let s = g - 1 - (m * far) as i32;
        for y in s.max(0)..g.min(h) {
            let snow = y - s < 1 + (m * 2.5) as i32 && m > 0.35;
            let bg = if snow { (150, 162, 196) } else { (44, 52, 84) };
            cv.put(x, y, if y == s { '▲' } else { ' ' }, rgb((215, 222, 240)), Some(rgb(bg)));
        }
    }

    // Snowy hills.
    let rise = (sky - 5).max(1) as f32;
    let bump = |x: i32| (1.0 - (x - hx).abs() as f32 / (w as f32 / 10.0 + 4.0)).max(0.0);
    let surf: Vec<i32> = (0..w).map(|x| g - ((ridge(x, w, seed, 50) * 0.6).max(bump(x) * 0.95) * rise) as i32).collect();
    for x in 0..w {
        let s = surf[x as usize];
        for y in s.max(0)..g.min(h) {
            let shade = (y - s) as f32 / 4.0;
            cv.put(x, y, if y == s { '▄' } else { ' ' }, rgb((235, 240, 250)), Some(mix((205, 214, 232), (150, 160, 190), shade)));
        }
    }
    // Pines, snow-capped.
    for x in 0..w {
        if hash(x + salt, 81) % 4 == 0 && (x - hx).abs() > 4 {
            let s = surf[x as usize];
            cv.glyph(x, s - 1, '♠', rgb((30, 90, 60)));
            cv.glyph(x, s - 2, '▲', rgb((225, 232, 245)));
        }
    }

    // The snowfield below, and a frozen lake.
    for y in g.max(0)..h {
        for x in 0..w {
            let ch = if hash(x * 5, y * 3) % 41 == 0 { '·' } else { ' ' };
            let (fx, fy) = (x as f32, y as f32);
            let drift = ((fx * 0.11 + fy * 0.9).sin() * 0.5 + (fx * 0.031 - fy * 0.6 + p).sin() * 0.5) * 0.5 + 0.5;
            let base = lerp((200, 210, 232), (172, 184, 214), (y - g) as f32 / (h - g).max(1) as f32);
            cv.put(x, y, ch, rgb((150, 160, 190)), Some(mix(base, (140, 152, 192), drift * 0.4)));
            if y > g + 1 && hash(x + salt, y * 17) % 61 == 0 {
                cv.glyph(x, y, '♠', rgb((26, 80, 54)));
            }
        }
    }
    if h - g > 10 {
        let (cx, cy) = (w / 2 + (cv.roll(3) % (w.max(8) as u32 / 4)) as i32 - w / 8, h - (h - g) / 3);
        let (rx, ry) = ((w / 6).max(4), ((h - g) / 5).max(2));
        for y in cy - ry..=cy + ry {
            for x in cx - rx..=cx + rx {
                let (dx, dy) = ((x - cx) as f32 / rx as f32, (y - cy) as f32 / ry as f32);
                if dx * dx + dy * dy <= 1.0 {
                    let ch = if hash(x, y) % 9 == 0 { '░' } else { ' ' };
                    cv.put(x, y, ch, rgb((200, 225, 245)), Some(rgb((130, 168, 205))));
                }
            }
        }
        cv.put(cx, cy, 'O', rgb((20, 40, 70)), Some(rgb((40, 70, 110))));
        cv.glyph(cx - 1, cy, '☺', rgb((180, 80, 60)));
        if (t / 25) % 4 == 0 {
            cv.glyph(cx, cy - 1, '!', rgb((60, 60, 90)));
        }
        // Footprints from the hold down to the ice.
        for k in 0..((cy - g).max(0)) {
            if k % 2 == 0 {
                cv.glyph(cx - 1 - k / 3, cy - 1 - k, '·', rgb((120, 130, 160)));
            }
        }
    }

    // The hold.
    let door_y = g - 1;
    if hx >= 0 && hx < w && w > 4 {
        for y in (door_y - 1)..=door_y {
            cv.fill(hx, y, rgb((255, 190, 90)));
        }
        cv.glyph(hx, door_y - 2, '∩', rgb((90, 84, 90)));
        for (k, wy) in [(-3i32, door_y - 1), (3, door_y - 1), (-2, door_y - 3), (2, door_y - 3)].iter() {
            let x = hx + k;
            if x >= 0 && x < w && *wy > surf[x as usize] {
                let on = (t / 11 + (*k + 5) as u64 * 7) % 31 != 0;
                cv.glyph(x, *wy, '■', rgb(if on { (255, 210, 110) } else { (150, 110, 60) }));
            }
        }
        // Smoke from the chimney at the top of the hill.
        let top = surf[hx as usize];
        cv.glyph(hx + 1, top - 1, '▐', rgb((90, 84, 90)));
        for k in 0..6i32 {
            let age = ((t / 8) as i32 + k * 4) % 24;
            let y = top - 2 - age / 3;
            let x = hx + 1 + age / 6;
            if y >= 0 {
                let fade = 1.0 - age as f32 / 24.0;
                cv.glyph(x, y, if age < 8 { '░' } else { '·' }, mix((40, 44, 70), (170, 170, 185), fade));
            }
        }
        // Someone digging the door out.
        let dig = (t / 12) % 4;
        let sx = hx + if left { 2 } else { -2 };
        let away = if left { 1 } else { -1 };
        cv.glyph(sx, door_y, '☺', rgb((120, 200, 255)));
        cv.glyph(sx + away, door_y - if dig < 2 { 0 } else { 1 }, '/', rgb((150, 140, 130)));
        if dig >= 2 {
            cv.glyph(sx + away * (dig as i32), door_y - 2 + (dig as i32 - 2), '*', rgb((240, 245, 255)));
        }
    }

    // Snow falling, the wind pushing it along.
    let heavy = [w / 4, w / 2, w][(cv.roll(5) % 3) as usize].max(4);
    for k in 0..heavy {
        let speed = 3 + (k % 4) as u64;
        let y = ((hash(k, 2) as u64 + t / speed) % h.max(1) as u64) as i32;
        let x = ((hash(k, 1) as u64 + t / (speed * 2)) % w.max(1) as u64) as i32 + ((y as f32 * 0.5 + k as f32).sin()) as i32;
        let ch = if k % 3 == 0 { '*' } else { '·' };
        cv.glyph(x, y, ch, rgb((240, 244, 255)));
    }
}
