//! A wagon train at the river ford: covered wagons behind their oxen
//! rolling down the bank and out through the shallows, water foaming
//! round the wheels, drovers wading alongside, then up the far bank and
//! away along the road. A heron fishes the reeds, the current runs bright
//! and fish jump. The seed picks the hour, the canvas, the beasts and how
//! many wagons.

use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the river decides when the caravan crosses",
    "wet boots, dry cargo, if the oxen are willing",
    "every road in the world crosses water somewhere",
    "the ford is shallow, the river is not",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let tf = t as f32;
    let sky = cv.sky.min(h - 1).max(1);
    let hz = (sky * 2 / 5).max(1).min(h - 1); // the far skyline
    let r0 = (sky * 3 / 5).max(hz + 1).min(h - 1); // the river's far edge
    let r1 = sky.max(r0 + 1).min(h); // its near edge
    let hour = cv.roll(1) % 3;
    let (top, bot) = [((90, 150, 220), (210, 230, 240)), ((60, 90, 170), (250, 200, 140)), ((110, 160, 210), (230, 230, 210))][hour as usize];
    cv.sky_fill(hz, top, bot);
    let far = lerp(bot, (60, 90, 80), 0.6);
    for x in 0..w {
        let s = hz - (ridge(x, w, seed, 4) * hz as f32 * 0.4) as i32;
        for y in s.max(0)..hz {
            cv.fill(x, y, rgb(far));
        }
    }

    // The far bank: meadow and trees down to the water.
    let salt = cv.salt();
    for y in hz.max(0)..r0 {
        for x in 0..w {
            let hs = hash(x * 3 + salt, y);
            let (ch, fg) = match hs % 8 {
                0 => ('♣', (40, 120, 50)),
                1 => ('♠', (30, 100, 60)),
                2 => ('"', (120, 170, 70)),
                _ => (' ', (0, 0, 0)),
            };
            cv.put(x, y, ch, rgb(fg), Some(rgb((80, 130, 56))));
        }
    }
    // The near bank, below.
    for y in r1.max(0)..h {
        for x in 0..w {
            let hs = hash(x * 5 + salt, y * 3);
            let ch = if hs % 5 == 0 { '"' } else if hs % 17 == 0 { '♣' } else { ' ' };
            cv.put(x, y, ch, rgb((110, 160, 60)), Some(rgb((70, 116, 48))));
        }
    }

    // The river running across, shallow and pale where the ford is.
    let fx0 = (w as f32 * 0.22) as i32;
    let fx1 = (w as f32 * 0.42) as i32;
    let ford_at = |y: i32| -> i32 {
        let u = (y - r0) as f32 / (r1 - r0).max(1) as f32;
        fx1 + ((fx0 - fx1) as f32 * u) as i32
    };
    for y in r0.max(0)..r1.min(h) {
        let fc = ford_at(y);
        for x in 0..w {
            let shallow = (x - fc).abs() <= 3;
            let base = if shallow { (110, 160, 170) } else { lerp((50, 100, 150), (30, 70, 120), (y - r0) as f32 / (r1 - r0).max(1) as f32) };
            let ph = (x as f32 * 0.4 - tf * 0.25 + y as f32 * 2.1).sin();
            let (ch, fg) = if ph > 0.85 { ('~', (200, 225, 240)) } else if ph < -0.9 && !shallow { ('─', (100, 150, 190)) } else { (' ', (0, 0, 0)) };
            cv.put(x, y, ch, rgb(fg), Some(rgb(base)));
        }
        // Reeds at the edges.
        for x in 0..w {
            if (y == r0 || y == r1 - 1) && hash(x, y + salt) % 4 == 0 && (x - fc).abs() > 4 {
                cv.glyph(x, y, if (t / 20 + x as u64) % 2 == 0 { '|' } else { '/' }, rgb((90, 140, 60)));
            }
        }
    }

    // The road: down the near bank, through the ford, up and away.
    let mut road: Vec<(i32, i32)> = Vec::new();
    let pts = [(-3, h - 2), (fx0 - 6, r1 + 1), (fx0, r1 - 1), (fx1, r0), (fx1 + 6, r0 - 1), (w + 2, r0 - 1)];
    for pair in pts.windows(2) {
        for c in line(pair[0], pair[1]) {
            if road.last() != Some(&c) {
                road.push(c);
            }
        }
    }
    for &(x, y) in &road {
        if y < r0 || y >= r1 {
            cv.put(x, y, if hash(x, y) % 5 == 0 { '·' } else { ' ' }, rgb((150, 130, 100)), Some(rgb((160, 136, 100))));
        }
    }

    // The heron in the reeds, now and then striking.
    let (hx, hy) = ((w as f32 * 0.7) as i32, r1 - 1);
    let strike = (t + cv.roll(2) as u64 % 260) % 260 < 12;
    cv.glyph(hx, hy - 1, if strike { '\\' } else { '│' }, rgb((200, 200, 210)));
    cv.glyph(hx, hy, '▼', rgb((180, 180, 195)));
    if strike {
        cv.glyph(hx + 1, hy, '~', rgb((240, 250, 255)));
    }
    // Fish jumping in the current.
    for k in 0..(1 + w / 50) {
        let cycle = 70 + (hash(k, 21) % 70) as u64;
        let age = (t + hash(k, 22) as u64) % cycle;
        if age < 10 {
            let x0 = (hash(k, 23 + ((t + hash(k, 22) as u64) / cycle) as i32) % w.max(1) as u32) as i32;
            let y0 = r0 + 1 + (hash(k, 24) % ((r1 - r0 - 1).max(1) as u32)) as i32;
            let a = age as i32;
            cv.glyph(x0 + a / 2, y0 - (a * (10 - a)) / 12, if a < 5 { '»' } else { '«' }, rgb((200, 210, 200)));
        }
    }

    // The wagon train. Each wagon is an arched cover over wheels behind a
    // pair of beasts; in the water the wheels go under and the water foams.
    let canvas = [(235, 225, 200), (210, 190, 150), (200, 90, 70), (120, 150, 190)][(cv.roll(3) % 4) as usize];
    let (beast, hide) = [('Ö', (150, 110, 70)), ('h', (120, 80, 50)), ('M', (190, 180, 160))][(cv.roll(4) % 3) as usize];
    let wagons = 2 + (cv.roll(5) % 3) as i64;
    let spacing = 9i64;
    let len = road.len() as i64;
    let cycle = len + wagons * spacing + 10;
    let head = ((t / 5 + cv.roll(6) as u64) as i64) % cycle.max(1);
    let in_water = |y: i32| y >= r0 && y < r1;
    for k in 0..wagons {
        let lead = head - k * spacing;
        for (j, part) in ['b', 'b', ' ', 'w', 'w', 'w'].iter().enumerate() {
            let i = lead - j as i64;
            if *part == ' ' || i < 0 || i >= len {
                continue;
            }
            let (x, y) = road[i as usize];
            let wet = in_water(y);
            match part {
                'b' => cv.glyph(x, y, beast, rgb(hide)),
                _ => {
                    let jolt = (t / 5 + j as u64) % 4 == 0;
                    cv.glyph(x, y - 1, '▄', rgb(if jolt { lerp(canvas, (0, 0, 0), 0.15) } else { canvas }));
                    if wet {
                        cv.glyph(x, y, if (t / 3 + j as u64) % 2 == 0 { '≈' } else { '~' }, rgb((240, 248, 255)));
                    } else {
                        cv.glyph(x, y, if j == 4 { '█' } else { 'o' }, rgb(if j == 4 { lerp(canvas, (0, 0, 0), 0.2) } else { (90, 66, 44) }));
                    }
                }
            }
            if wet && *part == 'b' {
                cv.glyph(x + 1, y, '~', rgb((240, 248, 255)));
            }
        }
        // The drover wading alongside.
        let i = lead - 2;
        if i >= 0 && i < len {
            let (x, y) = road[i as usize];
            cv.glyph(x, y + 1, '☺', rgb([(200, 80, 60), (70, 110, 180), (200, 180, 90)][(k % 3) as usize]));
        }
    }
}
