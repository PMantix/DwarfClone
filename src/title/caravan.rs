//! A merchant caravan winds up a switchback road through the woods to the
//! fortress gate in the mountainside. The seed sets the hour, whether the
//! woods are in autumn or high summer, the wagons' canvas and the beasts
//! that pull them.

use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the traders come when the leaves turn",
    "wagons on the mountain road, bells on the oxen",
    "a fair price, a long road, a full larder",
    "news from the wide world, and cloth, and wine",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, sky, seed) = (cv.w(), cv.h(), cv.t, cv.sky, cv.seed);
    let g = sky - 1; // the road along the foot of the mountain
    let hour = cv.roll(1) % 3;
    let autumn = cv.roll(2) % 3 != 0;
    let (top, bot) = match hour {
        0 => ((40, 46, 100), (220, 150, 110)),
        1 => ((44, 74, 128), (130, 150, 180)),
        _ => ((26, 18, 58), (190, 90, 60)),
    };
    cv.sky_fill(g, top, bot);

    // A few clouds and birds.
    for k in 0..3i32 {
        let len = 5 + (hash(k, 19) % 7) as i32;
        let y = 1 + (hash(k, 13) % 3) as i32;
        let x0 = ((hash(k, 15) as u64 + t / (18 + k as u64 * 6)) % (w as u64 + len as u64 + 10)) as i32 - len - 5;
        for i in 0..len {
            cv.glyph(x0 + i, y, if i == 0 || i == len - 1 { '▄' } else { '█' }, rgb(lerp(top, (240, 230, 230), 0.45)));
        }
    }

    // The mountain on the right, with the gate cut into its foot.
    let mx = w * 3 / 5;
    let rise = (sky - 2).max(1) as f32;
    let surf = |x: i32| -> i32 {
        if x < mx {
            let hill = ridge(x, w, seed, 40) * 2.5;
            g - 1 - hill as i32
        } else {
            let f = ((x - mx) as f32 / (w - mx).max(1) as f32 * 1.6).min(1.0);
            let m = f * (0.55 + 0.45 * peaks(x, w, seed, 44, 3));
            g - 1 - (m * rise) as i32
        }
    };
    for x in 0..w {
        let s = surf(x);
        let mountain = x >= mx;
        for y in s.max(0)..g {
            let bg = if mountain { mix((58, 52, 50), (40, 36, 36), (y - s) as f32 / 6.0) } else { rgb((30, 44, 24)) };
            let ch = if y == s && mountain {
                '▲'
            } else if !mountain && hash(x, y + 50) % 2 == 0 {
                '♣'
            } else {
                ' '
            };
            let fg = if mountain && s < g - (rise * 0.6) as i32 { (235, 240, 250) } else { (110, 100, 96) };
            let fg = if mountain { fg } else { leaf(x * 7 + y, autumn) };
            cv.put(x, y, ch, rgb(fg), Some(bg));
        }
    }
    let gx = (mx + (w - mx) / 3).min(w - 2);
    let lamp = (t / 5 + 3) % 7 != 0;
    for y in (g - 2).max(0)..=g {
        for x in gx - 1..=gx + 1 {
            cv.fill(x, y, rgb((10, 8, 6)));
        }
    }
    for x in gx - 2..=gx + 2 {
        cv.put(x, g - 3, '▄', rgb((120, 112, 104)), None);
    }
    cv.glyph(gx - 2, g - 1, '☼', rgb(if lamp { (255, 210, 90) } else { (200, 140, 60) }));
    cv.glyph(gx + 2, g - 1, '☼', rgb(if lamp { (255, 210, 90) } else { (200, 140, 60) }));

    // The woods below.
    let salt = cv.salt();
    for y in g.max(0)..h {
        for x in 0..w {
            let hs = hash(x * 5 + 2 + salt, y * 3 + 9);
            let (ch, fg) = match hs % 7 {
                0 | 1 => ('♣', leaf(x * 3 + y * 7, autumn)),
                2 => ('♠', (36, 104, 56)),
                3 => (if autumn { ',' } else { '"' }, if autumn { (170, 90, 40) } else { (70, 120, 50) }),
                _ => (' ', (0, 0, 0)),
            };
            cv.put(x, y, ch, rgb(fg), Some(rgb((24, 34, 18))));
        }
    }

    // The road: up from the bottom of the screen in switchbacks, then
    // along the mountain's foot to the gate.
    let mut points = vec![(-2, h - 1)];
    let mut y = h - 1;
    let mut right = true;
    let step = ((h - g) / 4).max(3);
    while y - step > g + 1 {
        y -= step;
        let jog = (hash(y, salt) % (w.max(10) as u32 / 10)) as i32;
        points.push((if right { w * 3 / 5 - jog } else { w / 10 + jog }, y));
        right = !right;
    }
    points.push((w / 12, g));
    points.push((gx, g));
    let mut road: Vec<(i32, i32)> = Vec::new();
    for pair in points.windows(2) {
        for c in line(pair[0], pair[1]) {
            if road.last() != Some(&c) {
                road.push(c);
            }
        }
    }
    for &(x, y) in &road {
        let ch = if hash(x, y) % 4 == 0 { '·' } else { ' ' };
        cv.put(x, y, ch, rgb((130, 108, 78)), Some(rgb((78, 62, 42))));
    }

    // The caravan along it.
    let canvas = [(225, 215, 190), (190, 60, 50), (70, 110, 180), (200, 170, 70)][(cv.roll(3) % 4) as usize];
    let beast = ['Ö', 'h', 'M'][(cv.roll(4) % 3) as usize];
    let beast_col = [(150, 110, 70), (170, 130, 90), (200, 190, 170)][(cv.roll(4) % 3) as usize];
    let mut train: Vec<(char, Rgb)> = vec![('☺', (200, 220, 255)), (' ', (0, 0, 0))];
    let wagons = 2 + cv.roll(5) % 3;
    for k in 0..wagons {
        train.push((beast, beast_col));
        train.push((beast, beast_col));
        train.push(('█', canvas));
        train.push(('█', canvas));
        train.push((' ', (0, 0, 0)));
        if k % 2 == 0 {
            train.push(('☺', [(240, 200, 120), (180, 240, 160), (240, 160, 200)][(k % 3) as usize]));
            train.push((' ', (0, 0, 0)));
        }
    }
    train.push(('☺', (200, 220, 255)));
    let len = road.len() as u64;
    let cycle = len + train.len() as u64 + 60;
    let head = ((t + (seed % 997)) / 4) % cycle.max(1);
    for (j, &(ch, col)) in train.iter().enumerate() {
        let idx = head as i64 - j as i64;
        if ch == ' ' || idx < 0 || idx >= len as i64 {
            continue;
        }
        let (x, y) = road[idx as usize];
        // The wheels jolt, the beasts plod.
        let col = if ch == '█' && (t / 6 + j as u64) % 4 == 0 { lerp(col, (0, 0, 0), 0.2) } else { col };
        cv.glyph(x, y, ch, rgb(col));
    }

    // Leaves (or in summer, seeds and fireflies) on the wind.
    for k in 0..(w / 4).max(3) {
        let speed = 4 + (k % 3) as u64;
        let y = ((hash(k, 2) as u64 + t / speed) % h.max(1) as u64) as i32;
        let x = ((hash(k, 1) as u64 + t / (speed - 1)) % w.max(1) as u64) as i32 + ((y as f32 * 0.6 + k as f32).sin() * 1.5) as i32;
        let (ch, col) = if autumn {
            ([',', '\'', '`'][(k % 3) as usize], leaf(k * 11, true))
        } else if hour == 2 && (t / 7 + k as u64) % 4 == 0 {
            ('·', (230, 255, 120))
        } else {
            continue;
        };
        cv.glyph(x, y, ch, rgb(col));
    }
}

/// The colour of a tree's leaves.
fn leaf(k: i32, autumn: bool) -> Rgb {
    let hs = hash(k, 91);
    if autumn {
        [(235, 205, 60), (235, 135, 40), (205, 60, 40), (160, 120, 40), (60, 130, 50)][(hs % 5) as usize]
    } else {
        [(40, 150, 50), (60, 170, 60), (90, 160, 60)][(hs % 3) as usize]
    }
}
