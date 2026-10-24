//! The dragon is dead. On a scorched hilltop at sunrise the hero stands on
//! the great wyrm's shoulder with the blade held high, smoke still curling
//! off its scales, its wing torn and sprawled, crows wheeling over it, and
//! the hoard spilling glittering out of the cave behind. The seed picks
//! the dragon's hide, the hero's cloak, the dawn and which way it fell.

use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the songs will get the details wrong",
    "it took the hoard a thousand years to gather",
    "first the wyrm, then the paperwork",
    "a legend, made in a single bloody morning",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let tf = t as f32;
    let sky = cv.sky.min(h - 1).max(1);
    let gy = (sky - 2).max(1).min(h - 1); // the hilltop the wyrm lies on
    let hz = (gy - 2).max(1);
    let dawn = cv.roll(1) % 3;
    let (top, bot) = [((60, 60, 120), (255, 170, 110)), ((40, 50, 110), (250, 200, 150)), ((70, 40, 80), (255, 120, 80))][dawn as usize];
    let hide = [(170, 40, 34), (40, 120, 70), (60, 70, 90), (180, 140, 40), (110, 50, 120)][(cv.roll(2) % 5) as usize];
    let cloak = [(40, 80, 170), (200, 200, 210), (60, 130, 60), (170, 30, 40)][(cv.roll(3) % 4) as usize];

    // The sky, the climbing sun and its rays.
    cv.sky_fill(hz, top, bot);
    let sun = (255, 236, 180);
    let (sx, sy) = ((w as f32 * 0.72) as i32, hz - 1 - ((tf / 900.0).min(1.0) * hz as f32 * 0.25) as i32);
    let turn = tf * 0.004;
    for y in 0..hz.min(h) {
        for x in 0..w {
            let (dx, dy) = ((x - sx) as f32 * 0.5, (y - sy) as f32);
            let d = (dx * dx + dy * dy).sqrt();
            let a = dy.atan2(dx);
            let ray = (a * 9.0 + turn).sin();
            if ray > 0.55 {
                let f = 0.22 * (ray - 0.55) / 0.45 * (1.0 - d / (w as f32 * 0.4).max(4.0)).max(0.0);
                cv.glow(x, y, sun, f);
            }
        }
    }
    let sr = (hz as f32 * 0.14).max(0.8).min(3.0);
    cv.halo(sx, sy, sr * 3.5, sun, 0.4);
    cv.disc(sx, sy, sr, sun);

    // Far hills under the dawn, and the scorched hilltop.
    let far = lerp(bot, (70, 50, 70), 0.6);
    for x in 0..w {
        let s = hz - (ridge(x, w, seed, 12) * (hz as f32 * 0.3)) as i32;
        for y in s.max(0)..=gy {
            cv.fill(x, y, rgb(far));
        }
    }
    let salt = cv.salt();
    for y in (gy + 1).max(0)..h {
        for x in 0..w {
            let hs = hash(x + salt, y * 3);
            let burnt = ((x as f32 * 0.08 + salt as f32).sin() + (y as f32 * 0.4).sin()) > -0.4;
            let (ch, fg, bg) = if burnt {
                let ember = hs % 23 == 0 && (t / 7 + hs as u64) % 5 != 0;
                if ember {
                    ('·', (255, 120 + (hs % 80) as u8, 40), (34, 24, 22))
                } else {
                    (if hs % 5 == 0 { '░' } else { ' ' }, (60, 54, 52), (34, 26, 24))
                }
            } else {
                (if hs % 4 == 0 { '"' } else { ' ' }, (90, 120, 50), (46, 60, 30))
            };
            cv.put(x, y, ch, rgb(fg), Some(rgb(bg)));
        }
    }

    // The cave and the spilled hoard on the far side.
    let cx = (w as f32 * 0.86) as i32;
    let rock = (66, 58, 62);
    for x in (cx - 6)..w {
        let top_ = gy - 6 + ((x - cx).abs() / 2).min(5) - (hash(x, 3) % 2) as i32;
        for y in top_.max(0)..=gy {
            cv.fill(x, y, rgb(lerp(rock, (0, 0, 0), (y - top_) as f32 * 0.05)));
        }
    }
    for y in (gy - 2)..=gy {
        for x in (cx - 1)..=(cx + 1) {
            cv.fill(x, y, rgb((10, 8, 10)));
        }
    }
    for k in 0..(10 + w / 12) {
        let x = cx - 1 - (hash(k, 7) % 12) as i32;
        let y = gy + (hash(k, 8) % 3) as i32 - (x > cx - 4) as i32;
        let glint = (t / 4 + hash(k, 9) as u64) % 17 == 0;
        let ch = if glint { '*' } else { ['☼', '·', '•', '♦'][(hash(k, 10) % 4) as usize] };
        let col = if glint { (255, 255, 230) } else { [(250, 200, 60), (230, 170, 40), (200, 220, 255), (220, 60, 80)][(hash(k, 11) % 4) as usize] };
        cv.glyph(x, y, ch, rgb(col));
    }

    // The dead wyrm: tail, body, neck and head along the hilltop.
    let x0 = (w as f32 * 0.08) as i32;
    let len = ((w as f32 * 0.5) as i32).min(64).max(12);
    let thick = |i: i32| -> i32 {
        let u = i as f32 / len as f32;
        if u < 0.35 {
            1 + (u / 0.35 * 2.0) as i32
        } else if u < 0.75 {
            3 + ((u - 0.35) * 10.0).sin().max(0.0).round() as i32
        } else {
            2
        }
    };
    let belly = lerp(hide, (230, 210, 160), 0.5);
    for i in 0..len {
        let x = x0 + i;
        let th = thick(i);
        for k in 0..th {
            let y = gy - k;
            let ch = if hash(x, y + salt) % 4 == 0 { '▒' } else { ' ' };
            let bg = if k == 0 && i as f32 > len as f32 * 0.3 { belly } else { hide };
            cv.put(x, y, ch, rgb(lerp(hide, (0, 0, 0), 0.3)), Some(rgb(bg)));
        }
        let u = i as f32 / len as f32;
        if u < 0.75 && i % 2 == 0 {
            cv.glyph(x, gy - th, '▲', rgb(lerp(hide, (0, 0, 0), 0.35)));
        }
    }
    // The head: an eye that will not open again, the jaw slack.
    let hx = x0 + len;
    for x in hx..hx + 3 {
        cv.fill(x, gy - 1, rgb(hide));
        cv.put(x, gy, '▼', rgb((240, 235, 220)), Some(rgb((90, 20, 20))));
    }
    cv.put(hx + 1, gy - 1, 'x', rgb((20, 10, 10)), Some(rgb(hide)));
    cv.glyph(hx + 3, gy - 1, '▄', rgb(hide));
    cv.glyph(hx + 3, gy, '~', rgb((200, 40, 50)));
    // The torn wing, spread back over the tail.
    let shx = x0 + (len as f32 * 0.55) as i32;
    let shy = gy - thick(shx - x0);
    let span = (gy as f32 * 0.6).max(3.0);
    let bones: Vec<(i32, i32)> = (0..5)
        .map(|k| {
            let (ang, l) = (1.95 + k as f32 * 0.28, span * (1.0 - k as f32 * 0.06));
            (shx + (ang.cos() * l * 2.0) as i32, (shy - (ang.sin() * l) as i32).min(gy - 2))
        })
        .collect();
    let membrane = lerp(hide, (20, 10, 10), 0.45);
    for pair in bones.windows(2) {
        let (a, b, c) = ((shx, shy), pair[0], pair[1]);
        let (xmin, xmax) = (a.0.min(b.0).min(c.0), a.0.max(b.0).max(c.0));
        let (ymin, ymax) = (a.1.min(b.1).min(c.1), a.1.max(b.1).max(c.1));
        let side = |p: (i32, i32), q: (i32, i32), r: (i32, i32)| (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0);
        for y in ymin..=ymax {
            for x in xmin..=xmax {
                let (d1, d2, d3) = (side(a, b, (x, y)), side(b, c, (x, y)), side(c, a, (x, y)));
                let inside = (d1 >= 0 && d2 >= 0 && d3 >= 0) || (d1 <= 0 && d2 <= 0 && d3 <= 0);
                if inside && hash(x + salt, y) % 4 != 0 && y < gy - 1 {
                    let torn = hash(x, y + salt) % 3 == 0;
                    cv.put(x, y, if torn { '░' } else { ' ' }, rgb(lerp(membrane, bot, 0.5)), Some(rgb(membrane)));
                }
            }
        }
    }
    let bone = lerp(hide, (0, 0, 0), 0.25);
    for &b in &bones {
        let (dx, dy) = ((b.0 - shx) as f32 * 0.5, (b.1 - shy) as f32);
        let ch = if dy.abs() > dx.abs() * 2.0 { '│' } else if dx.abs() > dy.abs() * 2.0 { '─' } else if (dx < 0.0) == (dy < 0.0) { '╲' } else { '╱' };
        for (x, y) in line((shx, shy), b) {
            cv.glyph(x, y, ch, rgb(bone));
        }
        cv.glyph(b.0, b.1, '▲', rgb(bone));
    }

    // Smoke still curling off the scales.
    for k in 0..(len / 6).max(2) {
        let bx = x0 + 3 + k * 6 + (hash(k, 20) % 3) as i32;
        let top_ = gy - thick(bx - x0) - 1;
        for j in 0..5 {
            let age = ((t / 5) as i32 + j * 4 + k * 3) % 20;
            let y = top_ - age / 3;
            let x = bx + age / 5 + ((age as f32 * 0.5 + k as f32).sin()) as i32;
            if hash(k, j) % 3 != 0 {
                cv.glyph(x, y, if age < 8 { '░' } else { '·' }, mix((90, 84, 86), lerp(top, bot, 0.6), age as f32 / 20.0));
            }
        }
    }

    // The hero on its shoulder, blade raised whenever the cheering starts.
    let hero_x = shx + 2;
    let hero_y = gy - thick(hero_x - x0);
    cv.glyph(hero_x, hero_y, '☺', rgb((250, 225, 190)));
    let flap = (t / 7) % 3;
    cv.glyph(hero_x - 1, hero_y, ['▐', '▌', '▐'][flap as usize], rgb(cloak));
    let raised = (t / 60) % 3 != 2;
    if raised {
        cv.glyph(hero_x + 1, hero_y - 1, '│', rgb((220, 225, 235)));
        cv.glyph(hero_x + 1, hero_y, '┴', rgb((180, 150, 80)));
        if (t / 4) % 9 == 0 {
            cv.glyph(hero_x + 1, hero_y - 2, '*', rgb((255, 255, 240)));
            cv.halo(hero_x + 1, hero_y - 2, 1.5, (255, 255, 230), 0.5);
        }
    } else {
        cv.glyph(hero_x + 1, hero_y, '/', rgb((220, 225, 235)));
    }

    // Crows wheeling above.
    let (ccx, ccy) = (x0 + len / 2, (gy - 8).max(1));
    for k in 0..(3 + cv.roll(4) % 3) {
        let a = tf * 0.03 + k as f32 * 2.1;
        let x = ccx + (a.cos() * (len as f32 * 0.35)) as i32;
        let y = ccy + (a.sin() * 2.0) as i32;
        cv.glyph(x, y, if (t / 5 + k as u64) % 2 == 0 { 'v' } else { '^' }, rgb((24, 20, 26)));
    }
}
