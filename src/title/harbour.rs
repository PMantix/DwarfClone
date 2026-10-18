//! Out past the lighthouse: a dwarven longship under a striped sail,
//! shields along its rail, running before the wind while a sea serpent
//! shadows it, coils rolling through the swell until the head rears up
//! beside the hull. The seed picks the hour and weather, the sail and the
//! serpent's colours, and where the coast lies.

use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "dwarves hate the sea, and the sea knows it",
    "a long way from any mountain",
    "row harder, it has seen us",
    "the deep keeps what the deep takes",
];

// Facing right. F flag, S sail, M mast, o shield, P/Q stern and bow,
// H hull, others drawn as they are.
const SMALL: [&str; 6] = ["     F     ", "   SSSSS   ", "   SSSSS   ", "     M     ", "P▄o▄o▄o▄o▄Q", " ▀HHHHHHH▀ "];
const BIG: [&str; 10] = [
    "       F         F       ",
    "     SSSSS     SSSSS     ",
    "    SSSSSSS   SSSSSSS    ",
    "    SSSSSSS   SSSSSSS    ",
    "    SSSSSSS   SSSSSSS    ",
    "     SSSSS     SSSSS     ",
    "       M         M       ",
    "P▄o▄o▄o▄o▄o▄o▄o▄o▄o▄o▄o▄Q",
    " ▀HHHHHHHHHHHHHHHHHHHHH▀ ",
    "   ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀   ",
];
// The serpent's head, facing left (towards the ship behind it).
const HEAD_SMALL: [&str; 2] = [" ▄●▄", "<▀██"];
const HEAD_BIG: [&str; 4] = ["   ▄▄██▄ ", " ▄██●███▄", "<▄▄▄▀████", "  ▀▀▀ ███"];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let g = (cv.sky - 1).max(1);
    let hz = (g - 1 - (g - 8).max(0) / 3).clamp(1, (h - 1).max(1));
    let tf = t as f32;
    let hour = cv.roll(1) % 4; // dawn, day, dusk, storm
    let storm = hour == 3;
    let (top, bot, sea_hi, sea_lo) = match hour {
        0 => ((60, 50, 110), (250, 160, 120), (120, 120, 170), (20, 40, 80)),
        1 => ((60, 120, 200), (170, 210, 240), (60, 130, 190), (14, 50, 100)),
        2 => ((30, 20, 70), (230, 110, 70), (110, 70, 110), (16, 24, 60)),
        _ => ((20, 22, 30), (60, 64, 76), (50, 70, 90), (10, 18, 30)),
    };
    let flash = storm && (t % 170) < 3;
    let sails: [(Rgb, Rgb); 4] = [((200, 40, 40), (235, 225, 205)), ((40, 70, 160), (230, 190, 70)), ((40, 120, 70), (230, 230, 220)), ((120, 50, 140), (240, 200, 120))];
    let (s1, s2) = sails[(cv.roll(2) % 4) as usize];
    let (scale, belly) = [((40, 140, 100), (190, 210, 120)), ((40, 90, 170), (150, 200, 230)), ((150, 40, 40), (230, 170, 90))][(cv.roll(3) % 3) as usize];
    let big = w >= 110 && g >= 16;
    let p0 = (cv.roll(4) % 628) as f32 / 100.0;

    // Sky and clouds.
    for y in 0..hz.min(h) {
        let base = lerp(top, bot, y as f32 / hz.max(1) as f32);
        for x in 0..w {
            let (fx, fy) = (x as f32, y as f32);
            let cl = ((fx * 0.045 + tf * 0.004 + p0).sin() + (fx * 0.11 - tf * 0.003 + fy * 0.7 + p0 * 2.0).sin()) * 0.5;
            let thick = if storm { 0.0 } else { 0.45 };
            let cloud = if storm { (60, 62, 72) } else { lerp(bot, (255, 255, 255), 0.5) };
            let mut c = if cl > thick { lerp(base, cloud, ((cl - thick) * 2.0).min(1.0) * 0.8) } else { base };
            if flash {
                c = lerp(c, (200, 210, 240), 0.6);
            }
            cv.fill(x, y, rgb(c));
        }
    }
    // Sun or moon, away from the middle.
    let (sx, sy) = ((w as f32 * 0.86) as i32, (hz as f32 * 0.4) as i32);
    let r = (hz as f32 * 0.12).clamp(0.8, 4.0);
    if !storm {
        let disc = [(255, 220, 160), (255, 250, 220), (255, 170, 90)][hour as usize];
        cv.halo(sx, sy, r * 3.0, disc, 0.35);
        cv.disc(sx, sy, r, disc);
    }

    // The coast and its lighthouse.
    let coast = (w as f32 * 0.22) as i32;
    let lx = (w as f32 * 0.08) as i32 + 2;
    let mut lamp = (lx, hz - 2);
    for x in 0..coast {
        let m = peaks(x, coast.max(8) * 2, seed, 40, 2) * (1.0 - x as f32 / coast.max(1) as f32).sqrt();
        let land = hz - 1 - (m * (hz as f32 * 0.45)) as i32;
        let land = if (x - lx).abs() <= 2 { land.min(hz - 2) } else { land };
        for y in land.max(0)..hz {
            let shade = if storm { (24, 26, 30) } else { lerp(bot, (40, 40, 60), 0.75) };
            cv.fill(x, y, rgb(shade));
        }
        if x == lx {
            let tall = (hz as f32 * 0.35).clamp(2.0, 9.0) as i32;
            for k in 0..tall {
                let y = land - 1 - k;
                cv.put(x, y, '█', rgb(if (k / 2) % 2 == 0 { (230, 230, 225) } else { (190, 50, 40) }), None);
            }
            lamp = (x, land - 1 - tall);
        }
    }
    let night = hour != 1;
    cv.glyph(lamp.0, lamp.1, '☼', rgb((255, 240, 160)));
    if night {
        // The beam sweeping round.
        let a = (tf * 0.04).cos();
        let len = ((w as f32 * 0.3) * a.abs()) as i32;
        let dir = if a > 0.0 { 1 } else { -1 };
        for i in 1..len {
            let x = lamp.0 + i * dir;
            let f = 0.5 * (1.0 - i as f32 / len.max(1) as f32);
            for dy in -(i / 12)..=(i / 12) {
                cv.glow(x, lamp.1 + dy, (255, 240, 170), f);
            }
        }
        cv.halo(lamp.0, lamp.1, 2.0, (255, 240, 170), 0.6);
    }

    // The sea.
    for y in hz.max(0)..h {
        let d = ((y - hz) as f32 / (h - hz).max(1) as f32).min(1.0);
        let base = lerp(sea_hi, sea_lo, d.sqrt());
        for x in 0..w {
            let (fx, fy) = (x as f32, y as f32);
            let sp = 1.0 + (y - hz) as f32 * 0.05;
            let ph = (fx * 0.25 / sp + fy * 1.7 - tf * 0.08 + (fx * 0.05 + p0).sin() * 2.0).sin();
            let mut bg = base;
            let mut ch = ' ';
            let mut fg = lerp(base, (230, 240, 255), 0.4);
            if ph > 0.97 - if storm { 0.15 } else { 0.0 } {
                ch = '≈';
                fg = (225, 235, 245);
            } else if ph > 0.7 {
                ch = '~';
            } else if ph < -0.8 {
                bg = lerp(base, (0, 0, 0), 0.15);
            }
            // The glitter under the sun or moon.
            if !storm && (x - sx).abs() < 2 + (y - hz) * 2 / 3 && hash(x, y + (t / 3) as i32) % 3 == 0 {
                ch = if hash(x, y) % 2 == 0 { '─' } else { '═' };
                fg = lerp((255, 240, 200), bot, 0.3);
            }
            if flash {
                bg = lerp(bg, (160, 170, 200), 0.4);
            }
            cv.put(x, y, ch, rgb(fg), Some(rgb(bg)));
        }
    }
    // Rocks off the coast with surf breaking on them.
    for k in 0..(w / 50).max(1) {
        let x = (hash(k, seed as i32) % coast.max(4) as u32) as i32 + 1;
        let y = hz + 1 + (hash(k, 3) % ((h - hz - 2).max(1) as u32)) as i32;
        cv.put(x, y, '▲', rgb((70, 66, 70)), None);
        if (t / 6 + k as u64) % 5 < 2 {
            cv.glyph(x - 1, y, '~', rgb((240, 245, 255)));
            cv.glyph(x + 1, y, '~', rgb((240, 245, 255)));
        }
    }

    // A far sail on the horizon, and fish leaping in the swell.
    if w >= 60 {
        let fx = (w as f32 * 0.3) as i32 + ((t / 40) % (w as u64 / 2).max(1)) as i32;
        cv.glyph(fx, hz - 1, '▲', rgb(lerp(bot, (40, 40, 50), 0.6)));
        cv.glyph(fx, hz, '▀', rgb(lerp(sea_hi, (20, 20, 30), 0.6)));
    }
    for k in 0..(w * (h - hz) / 900).max(1) {
        let cycle = 90 + (hash(k, 21) % 90) as u64;
        let age = (t + hash(k, 22) as u64) % cycle;
        if age < 14 {
            let x0 = (hash(k, 23 + ((t + hash(k, 22) as u64) / cycle) as i32) % w.max(1) as u32) as i32;
            let y0 = hz + 2 + (hash(k, 24) % ((h - hz - 3).max(1) as u32)) as i32;
            let a = age as i32;
            cv.glyph(x0 + a / 2, y0 - (a * (14 - a)) / 20, if a < 7 { '»' } else { '«' }, rgb((200, 210, 220)));
            if a == 0 || a == 13 {
                cv.glyph(x0 + a / 2, y0 + 1, '≈', rgb((240, 245, 255)));
            }
        }
    }

    // The ship.
    let rows: &[&str] = if big { &BIG } else { &SMALL };
    let sw = rows[0].chars().count() as i32;
    let base = g.min(h - 1);
    let travel = (w + sw + 12) as u64;
    let shipx = ((t / 10 + cv.roll(5) as u64) % travel) as i32 - sw - 6;
    let top_y = base - rows.len() as i32 + 1;
    let roll_ = ((tf * 0.12).sin() * 0.6).round() as i32;
    let wood = (110, 70, 40);
    for (r, row) in rows.iter().enumerate() {
        for (c, ch) in row.chars().enumerate() {
            let (x, y) = (shipx + c as i32, top_y + r as i32 + if r + 3 < rows.len() { roll_.min(0) } else { 0 });
            let stripe = if (c / 2) % 2 == 0 { s1 } else { s2 };
            match ch {
                'S' => {
                    let billow = if (x + y + (t / 12) as i32) % 9 == 0 { '░' } else { ' ' };
                    cv.put(x, y, billow, rgb(lerp(stripe, (0, 0, 0), 0.2)), Some(rgb(if storm { lerp(stripe, (0, 0, 0), 0.4) } else { stripe })));
                }
                'F' => cv.glyph(x, y, if (t / 5) % 2 == 0 { '►' } else { '▶' }, rgb(s1)),
                'M' => cv.glyph(x, y, '│', rgb(wood)),
                'o' => cv.glyph(x, y, 'o', rgb(if c % 4 == 2 { s1 } else { s2 })),
                'P' => cv.glyph(x, y, '█', rgb(wood)),
                'Q' => cv.glyph(x, y, '▀', rgb(wood)),
                'H' => cv.put(x, y, ' ', Color::Black, Some(rgb(lerp(wood, (0, 0, 0), 0.2)))),
                ' ' => {}
                c => cv.glyph(x, y, c, rgb(wood)),
            }
        }
    }
    // Masts above the sail tops.
    if night && !storm {
        cv.glyph(shipx, top_y + rows.len() as i32 - 3, '☼', rgb((255, 210, 110)));
        cv.halo(shipx, top_y + rows.len() as i32 - 3, 1.5, (255, 200, 100), 0.5);
    }
    // The wake.
    for k in 1..(sw / 2 + 4) {
        let x = shipx - k;
        if (x + (t / 3) as i32) % 3 != 0 {
            cv.glyph(x, base + (k / 6).min(2), '~', rgb((235, 240, 250)));
        }
    }
    cv.glyph(shipx + sw, base, '≈', rgb((240, 245, 255)));

    // The serpent: coils rolling behind, and the head that rears beside.
    let hump_w = if big { 5 } else { 3 };
    let nh = if big { 4 } else { 3 };
    for k in 0..nh {
        let up = (tf * 0.08 - k as f32 * 1.3).sin();
        if up < 0.1 {
            continue;
        }
        let x = shipx - sw / 2 - 8 - k * (hump_w + 4) + ((tf * 0.05).sin() * 2.0) as i32;
        let y = base + 1 + k / 2;
        let tall = up > 0.6;
        for i in 0..hump_w {
            let edge = i == 0 || i == hump_w - 1;
            cv.glyph(x + i, y, if edge { '▄' } else { '█' }, rgb(scale));
            if tall && !edge {
                cv.glyph(x + i, y - 1, '▄', rgb(scale));
            }
        }
        cv.glyph(x - 1, y, '~', rgb((235, 240, 250)));
        cv.glyph(x + hump_w, y, '~', rgb((235, 240, 250)));
    }
    let period = 520u64;
    let local = (t + cv.roll(6) as u64 % period) % period;
    let rise = match local {
        0..=59 => local as f32 / 60.0,
        60..=379 => 1.0,
        380..=439 => 1.0 - (local - 380) as f32 / 60.0,
        _ => 0.0,
    };
    if rise > 0.0 {
        let head: &[&str] = if big { &HEAD_BIG } else { &HEAD_SMALL };
        let neck_h = ((rows.len() as f32 - 1.0) * rise).round() as i32;
        let nx = shipx + sw + if big { 6 } else { 4 };
        let thick = if big { 3 } else { 2 };
        let sway = (tf * 0.06).sin();
        let mut hx = nx;
        for j in 0..=neck_h {
            let y = base - j;
            let x = nx + (sway * (j as f32 / 3.0)).round() as i32;
            hx = x;
            for i in 0..thick {
                let col = if i == 0 { belly } else { scale };
                cv.put(x + i, y, ' ', Color::Black, Some(rgb(col)));
            }
            if j % 2 == 0 {
                cv.glyph(x + thick - 1, y, '▒', rgb(lerp(scale, (0, 0, 0), 0.3)));
            }
        }
        let hy = base - neck_h - head.len() as i32 + 1;
        let hw = head[0].chars().count() as i32;
        let hxx = hx + thick - hw;
        for (r, row) in head.iter().enumerate() {
            for (c, ch) in row.chars().enumerate() {
                let (x, y) = (hxx + c as i32, hy + r as i32);
                match ch {
                    '●' => cv.put(x, y, '●', rgb((255, 230, 80)), Some(rgb(scale))),
                    '<' => {
                        if (t / 4) % 3 != 0 {
                            cv.glyph(x, y, '~', rgb((220, 40, 40)));
                        }
                    }
                    '█' => cv.put(x, y, ' ', Color::Black, Some(rgb(scale))),
                    ' ' => {}
                    c => cv.glyph(x, y, c, rgb(scale)),
                }
            }
        }
        // Water streaming off it.
        if rise < 1.0 || local < 100 {
            for k in 0..8 {
                let age = ((t / 2) as i32 + k * 3) % 12;
                let dx = if k % 2 == 0 { age / 2 + thick } else { -age / 2 - 1 };
                cv.glyph(nx + dx, base - (age * (12 - age)) / 12, '·', rgb((230, 240, 255)));
            }
        }
        for i in -1..=thick {
            if (i + (t / 3) as i32) % 2 == 0 {
                cv.glyph(nx + i, base + 1, '≈', rgb((240, 245, 255)));
            }
        }
    }

    // Gulls by day; rain and lightning in the storm.
    if hour < 2 {
        for k in 0..(w / 30).max(1) {
            let x = ((hash(k, 4) as u64 + t / 4) % (w as u64 + 10)) as i32 - 5;
            let y = 1 + (hash(k, 6) % (hz.max(3) as u32 / 2 + 1)) as i32 + (((t / 7) as i32 + k) % 3 == 0) as i32;
            cv.glyph(x, y, if (t / 6 + k as u64) % 2 == 0 { 'v' } else { '^' }, rgb((40, 40, 50)));
        }
    }
    if storm {
        for k in 0..(w * h / 40).max(4) {
            let y = ((hash(k, 2) as u64 + t) % h.max(1) as u64) as i32;
            let x = ((hash(k, 1) as u64 + t / 2) % w.max(1) as u64) as i32 - y / 2;
            cv.glyph(x.rem_euclid(w.max(1)), y, '/', rgb((120, 140, 180)));
        }
        if flash {
            let bx = (hash((t / 170) as i32, 5) % w.max(1) as u32) as i32;
            let mut x = bx;
            for y in 0..hz {
                cv.glyph(x, y, if hash(x, y) % 2 == 0 { '/' } else { '\\' }, rgb((240, 245, 255)));
                x += if hash(y, (t / 170) as i32) % 2 == 0 { 1 } else { -1 };
            }
        }
    }
}
