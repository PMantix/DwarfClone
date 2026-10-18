//! Festival night under the mountain: fireworks climbing from the shore
//! and bursting over the peak, the whole lake catching them, lanterns
//! strung between poles, a bonfire, and a crowd of dwarves cheering every
//! bang. The seed picks the colours, the mountain, the lanterns and how
//! big the crowd is.

use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "a hundred years of the fortress, not one dull night",
    "the mechanics outdid themselves this time",
    "the whole mountain is singing",
    "light the fuse and stand well back",
];

const PALETTES: [[Rgb; 4]; 4] = [
    [(255, 210, 90), (255, 90, 80), (120, 220, 255), (190, 255, 140)],
    [(255, 140, 220), (180, 120, 255), (120, 200, 255), (255, 255, 255)],
    [(255, 200, 60), (255, 120, 40), (255, 240, 180), (230, 60, 60)],
    [(120, 255, 180), (90, 200, 255), (255, 255, 160), (255, 150, 230)],
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let g = (cv.sky - 1).max(1); // the shore
    let tf = t as f32;
    let pal = PALETTES[(cv.roll(1) % 4) as usize];
    let big = w >= 110 && g >= 16;
    let salt = cv.salt();

    // Night sky.
    cv.sky_fill(g, (4, 6, 20), (30, 24, 60));
    cv.stars(g - 2, 37);

    // The mountain, its gate glowing at the foot.
    let mc = (w as f32 * (0.35 + (cv.roll(2) % 30) as f32 / 100.0)) as i32;
    let mh = (g as f32 * 0.85).max(2.0);
    let mw = (w as f32 * 0.32).max(10.0);
    let surf: Vec<i32> = (0..w)
        .map(|x| {
            let u = (x - mc) as f32 / mw;
            let main = (1.0 - u.abs()).max(0.0).powf(1.3);
            let side = ridge(x, w, seed, 10) * 0.35;
            g - (main.max(side) * mh) as i32 - ((hash(x, salt) % 2) as i32)
        })
        .collect();
    for x in 0..w {
        let s = surf[x as usize];
        for y in s.max(0)..g {
            let snow = y - s < 2 && s < g - (mh * 0.55) as i32;
            let bg = if snow { (120, 124, 150) } else { (22, 20, 34) };
            cv.put(x, y, if y == s { '▲' } else { ' ' }, rgb(if snow { (210, 214, 235) } else { (44, 40, 60) }), Some(rgb(bg)));
        }
    }
    let gx = mc;
    if g >= 4 {
        for y in g - 2..g {
            for x in gx - 1..=gx + 1 {
                cv.put(x, y, ' ', Color::Black, Some(rgb((255, 190, 90))));
            }
        }
        cv.glyph(gx - 1, g - 3, '▄', rgb((140, 130, 120)));
        cv.glyph(gx, g - 3, '▄', rgb((140, 130, 120)));
        cv.glyph(gx + 1, g - 3, '▄', rgb((140, 130, 120)));
        cv.halo(gx, g - 1, 4.0, (255, 180, 80), 0.5);
        // Banners either side of the gate.
        for dx in [-3, 3] {
            cv.glyph(gx + dx, g - 3, '▐', rgb(pal[1]));
            cv.glyph(gx + dx, g - 2, '▐', rgb(pal[1]));
        }
    }

    // The lake below the shore.
    for y in g.max(0)..h {
        let d = (y - g) as f32 / (h - g).max(1) as f32;
        for x in 0..w {
            let ripple = ((x as f32 * 0.4 + y as f32 * 2.1 - tf * 0.1).sin() > 0.85) as u8;
            let bg = lerp((20, 22, 48), (6, 8, 22), d);
            cv.put(x, y, if ripple == 1 { '─' } else { ' ' }, rgb(lerp(bg, (90, 100, 150), 0.6)), Some(rgb(bg)));
        }
    }
    // The mountain in the water, wavering.
    for y in (g + 1)..h.min(2 * g) {
        let src = 2 * g - y;
        for x in 0..w {
            let wob = ((y as f32 * 1.3 + tf * 0.08).sin() * 1.2) as i32;
            let sx = (x + wob).clamp(0, w - 1);
            if surf[sx as usize] <= src {
                cv.glow(x, y, (40, 36, 60), 0.6);
            }
        }
    }
    for x in 0..w {
        cv.put(x, g, '▀', rgb((60, 52, 50)), None);
    }

    // Fireworks.
    let shells = (w / 18).clamp(3, 12);
    let top = 1;
    let low = if big { (g as f32 * 0.6) as i32 } else { (g - 2).max(2) };
    for k in 0..shells {
        let life = 110 + (hash(k, 3) % 70) as u64;
        let age_all = t + (hash(k, 5) as u64 % life);
        let a = age_all % life;
        let round = (age_all / life) as i32;
        let hs = hash(k, round * 7 + 11);
        let lx = 2 + (hs % (w - 4).max(1) as u32) as i32;
        let by = top + (hash(k, round + 13) % (low - top).max(1) as u32) as i32;
        let rise = ((g - by) * 2).max(2) as u64;
        let col = pal[(hs % 4) as usize];
        let col2 = pal[((hs / 4) % 4) as usize];
        let kind = (hs / 16) % 4; // peony, willow, ring, crackle
        let reflect = |y: i32| 2 * g - y + 1;
        if a < rise {
            let y = g - 1 - (a as i32 / 2);
            cv.glyph(lx, y, '|', rgb((255, 230, 180)));
            cv.glyph(lx, y + 1, '·', rgb((200, 120, 60)));
            continue;
        }
        let b = (a - rise) as f32;
        let span = if kind == 1 { 70.0 } else { 45.0 };
        if b > span {
            continue;
        }
        if b < 5.0 {
            cv.halo(lx, by, 6.0 + if big { 4.0 } else { 0.0 }, col, 0.45 * (1.0 - b / 5.0));
        }
        let rmax = if big { 9.0 } else { 5.0 } + (hs % 3) as f32;
        let r = rmax * (1.0 - (-b / 7.0).exp());
        let fade = (1.0 - b / span).max(0.0);
        let n = if big { 28 } else { 16 };
        for j in 0..n {
            let ang = j as f32 / n as f32 * std::f32::consts::TAU + (hs % 100) as f32 / 30.0;
            let (rr, droop) = match kind {
                1 => (r, b * b * 0.006),
                2 => (r * 0.8, b * b * 0.002),
                3 => (r * (0.4 + (hash(j, round) % 60) as f32 / 100.0), b * b * 0.003),
                _ => (r * if j % 2 == 0 { 1.0 } else { 0.75 }, b * b * 0.003),
            };
            let x = lx + (ang.cos() * rr * 2.0).round() as i32;
            let y = by + (ang.sin() * rr + droop).round() as i32;
            if y >= g {
                continue;
            }
            if kind == 3 && (t / 2 + j as u64) % 3 == 0 {
                continue;
            }
            let c = lerp(lerp(col2, col, fade), (20, 20, 40), (1.0 - fade) * (1.0 - fade));
            let ch = if b < 14.0 { '*' } else if kind == 3 { '+' } else if fade > 0.5 { '•' } else { '·' };
            cv.glyph(x, y, ch, rgb(c));
            if kind == 0 && j % 2 == 0 {
                let (ix, iy) = (lx + (ang.cos() * rr * 1.0).round() as i32, by + (ang.sin() * rr * 0.5 + droop).round() as i32);
                cv.glyph(ix, iy, '·', rgb(lerp(col2, (20, 20, 40), (1.0 - fade) * 0.8)));
            }
            // A trail behind willow sparks.
            if kind == 1 && b > 10.0 {
                cv.glyph(x, y - 1, '·', rgb(lerp(c, (20, 20, 40), 0.5)));
            }
            let ry = reflect(y);
            if ry < h && ry > g {
                let wob = ((ry as f32 * 1.3 + tf * 0.08).sin() * 1.2) as i32;
                cv.glyph(x - wob, ry, '·', rgb(lerp(c, (20, 22, 48), 0.45)));
            }
        }
        if b < 8.0 {
            let ry = reflect(by);
            cv.halo(lx, ry, 4.0, col, 0.3 * (1.0 - b / 8.0));
        }
    }

    // Lanterns strung between poles along the shore.
    let poles = (w / 22).max(2);
    let gap = w / poles;
    let colours = [pal[0], pal[1], pal[2], pal[3]];
    let ly = (g - 4).max(1);
    for p in 0..=poles {
        let px = p * gap;
        for y in ly..g {
            cv.glyph(px, y, '│', rgb((90, 70, 50)));
        }
        if p < poles {
            for i in 1..gap {
                let u = i as f32 / gap as f32;
                let sag = (u * (1.0 - u) * 4.0 * if big { 3.0 } else { 1.5 }) + (tf * 0.05 + p as f32).sin() * 0.3;
                let y = ly + sag.round() as i32;
                if i % 3 == 0 {
                    let c = colours[((i / 3 + p) % 4) as usize];
                    cv.glyph(px + i, y, '●', rgb(c));
                    cv.glow(px + i, y, c, 0.25);
                } else {
                    cv.glyph(px + i, y, '·', rgb((80, 70, 60)));
                }
            }
        }
    }

    // The bonfire, and the crowd along the shore.
    let fx = (w as f32 * 0.82) as i32;
    let flick = ((tf * 0.4).sin() * 0.5 + 0.5) * 0.3 + 0.5;
    cv.halo(fx, g - 1, 4.0 + if big { 3.0 } else { 0.0 }, (255, 150, 50), flick);
    for k in -1..=1 {
        let hs = hash(k, (t / 3) as i32);
        cv.glyph(fx + k, g - 1, '▲', rgb([(255, 230, 120), (255, 150, 40), (230, 70, 20)][(hs % 3) as usize]));
    }
    cv.glyph(fx, g - 2, if (t / 3) % 2 == 0 { '^' } else { '▲' }, rgb((255, 200, 80)));
    for k in 0..4 {
        let age = ((t / 3) as i32 + k * 4) % 12;
        cv.glyph(fx + ((age + k) % 3) - 1, g - 3 - age / 3, '·', rgb((255, 190, 90)));
    }
    let crowd = (w as f32 * (0.18 + (cv.roll(3) % 20) as f32 / 100.0)) as i32;
    let shirts = [(150, 200, 240), (240, 170, 110), (200, 230, 150), (230, 150, 210), (240, 220, 120), (200, 120, 100)];
    for k in 0..crowd {
        let x = (hash(k, salt + 1) % w.max(1) as u32) as i32;
        if (x - fx).abs() < 2 || (x - gx).abs() < 2 {
            continue;
        }
        let hs = hash(k, 9);
        let jump = (t / 7 + hs as u64) % 11 == 0;
        let arms = (t / 9 + hs as u64) % 4 < 2;
        cv.glyph(x, g - 1 - jump as i32, '☺', rgb(shirts[(hs % 6) as usize]));
        if arms && hs % 3 == 0 {
            cv.glyph(x, g - 2 - jump as i32, if hs % 2 == 0 { '\\' } else { '/' }, rgb(shirts[(hs % 6) as usize]));
        }
        if hs % 7 == 0 {
            cv.glyph(x + 1, g - 1, 'Θ', rgb((150, 100, 50)));
        }
    }

    // Boats out on the lake with lanterns, for big screens.
    if h - g > 8 {
        for k in 0..(w / 40).max(1) {
            let x = ((hash(k, 41) as u64 + t / 30) % (w as u64 + 10)) as i32 - 5;
            let y = g + 3 + (hash(k, 43) % ((h - g - 4).max(1) as u32)) as i32;
            cv.sprite(x, y, &["▀▄▄▄▀"], rgb((110, 70, 40)));
            cv.glyph(x + 2, y - 1, '☺', rgb(shirts[(k % 6) as usize]));
            cv.glyph(x + 3, y - 1, '●', rgb(pal[(k % 4) as usize]));
            cv.glow(x + 3, y - 1, pal[(k % 4) as usize], 0.4);
            cv.glyph(x + 2, y + 1, '·', rgb(lerp(pal[(k % 4) as usize], (20, 22, 48), 0.5)));
        }
    }
}
