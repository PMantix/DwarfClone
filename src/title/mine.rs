//! The mine seen in section: a shaft from the surface down to a timbered
//! drift, a cart rattling along the rails, and miners driving the face
//! towards a hidden geode until the rock gives way and the hollow full of
//! crystals blazes in their lanterns. Then they get to work on it. The
//! seed picks the gems, the strata, the hour up top and how far they dig.

use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "strike the earth, and the earth strikes back in gems",
    "nine parts rock, one part wonder",
    "the vein runs deeper than anyone guessed",
    "a lantern, a pick, and a whisper in the stone",
];

/// Gem colours: bright facet, deep facet, the glow they throw.
const GEMS: [(Rgb, Rgb, Rgb); 5] = [
    ((230, 160, 255), (130, 60, 190), (90, 40, 140)), // amethyst
    ((140, 255, 170), (30, 150, 80), (20, 90, 60)),   // emerald
    ((255, 140, 150), (180, 30, 50), (110, 20, 40)),  // ruby
    ((150, 200, 255), (40, 80, 200), (25, 50, 130)),  // sapphire
    ((255, 240, 150), (210, 150, 40), (120, 90, 30)), // topaz
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t) = (cv.w(), cv.h(), cv.t);
    let fl = cv.sky.min(h - 1); // the drift's floor, where the rails run
    let tf = t as f32;
    let gem = cv.roll(1) % 5;
    let (bright, deep, glowc) = GEMS[gem as usize];
    let rainbow = cv.roll(2) % 4 == 0;
    let facet = |k: u32| -> Rgb {
        if rainbow {
            GEMS[(k % 5) as usize].0
        } else if k % 3 == 0 {
            deep
        } else {
            bright
        }
    };
    let hour = cv.roll(3) % 3;
    let salt = cv.salt();
    let big = w >= 110 && fl >= 16;

    // The surface: a strip of sky, grass, then rock all the way down.
    let surf = (fl / 4).clamp(1, 5).min(h - 1);
    let (sky_top, sky_bot) = [((70, 120, 190), (150, 190, 230)), ((40, 30, 80), (200, 110, 80)), ((6, 8, 24), (24, 30, 60))][hour as usize];
    cv.sky_fill(surf, sky_top, sky_bot);
    if hour == 2 {
        cv.stars(surf, 23);
    }
    // Strata: wavy bands, each with its own stone.
    let bands: [Rgb; 6] = [(92, 64, 40), (120, 96, 66), (96, 92, 96), (70, 70, 84), (58, 52, 66), (44, 40, 56)];
    let p0 = (cv.roll(4) % 628) as f32 / 100.0;
    let thick = ((h - surf) as f32 / 5.0).max(2.0);
    let waves: Vec<f32> = (0..w).map(|x| (x as f32 * 0.05 + p0).sin() * 1.6 + (x as f32 * 0.13 - p0).sin() * 0.7).collect();
    let stratum = |x: i32, y: i32| -> usize { (((y - surf) as f32 + waves[x as usize]) / thick).max(0.0) as usize };
    // Ore veins snaking through the rock.
    let ores: [Rgb; 4] = [(200, 120, 70), (235, 195, 80), (205, 205, 220), (160, 90, 70)];
    let veins: Vec<(f32, f32, Rgb)> = (0..(1 + (h - surf) / 12).min(4))
        .map(|k| {
            let vy = surf as f32 + 3.0 + (cv.roll(20 + k as u32) % ((h - surf - 4).max(1) as u32)) as f32;
            (vy, (cv.roll(30 + k as u32) % 628) as f32 / 100.0, ores[(cv.roll(40 + k as u32) % 4) as usize])
        })
        .collect();
    let vein_at: Vec<Vec<f32>> = (0..w)
        .map(|x| {
            let fx = x as f32;
            veins.iter().map(|&(vy, ph, _)| vy + (fx * 0.06 + ph).sin() * 3.0 + (fx * 0.17 + ph).sin()).collect()
        })
        .collect();
    for y in surf.max(0)..h {
        for x in 0..w {
            if y == surf {
                let ch = if hash(x + salt, 3) % 3 == 0 { '"' } else { '▀' };
                cv.put(x, y, ch, rgb((70, 150, 60)), Some(rgb((80, 58, 36))));
                continue;
            }
            let s = stratum(x, y).min(5);
            let base = bands[s];
            let hs = hash(x * 3 + salt, y * 7 + 2);
            let vein_col = vein_at[x as usize].iter().zip(veins.iter()).find(|(vy, _)| (**vy - y as f32).abs() < 0.45).map(|(_, v)| v.2);
            let in_vein = vein_col.is_some();
            let (ch, fg) = match (in_vein, hs % 61) {
                (true, k) if k % 3 == 0 => ('•', lerp(vein_col.unwrap_or((200, 160, 70)), base, 0.3)),
                (true, k) if k % 3 == 1 => ('·', lerp(vein_col.unwrap_or((200, 160, 70)), base, 0.4)),
                (_, 0) => ('*', (200, 160, 70)),
                (_, 1) => ('·', (80, 170, 110)),
                (_, 2..=5) => ('░', lerp(base, (0, 0, 0), 0.25)),
                (_, 6) => ('▒', lerp(base, (160, 160, 160), 0.15)),
                _ => (' ', base),
            };
            cv.put(x, y, ch, rgb(fg), Some(rgb(lerp(base, (0, 0, 0), ((y - surf) as f32 / h as f32) * 0.35))));
        }
    }

    // The geode, hidden in the rock towards the far side.
    let rx = (w / 9).clamp(4, 24);
    let ry = ((h - fl) / 4 + fl / 4).clamp(2, 10);
    let gx = w - rx - 2 - (cv.roll(5) % (w.max(20) as u32 / 20)) as i32;
    let gy = fl - 1;
    let shaft = (w / 12).clamp(2, 14);
    let start = shaft + 2;
    let dist = (gx - rx - start).max(1);
    let speed = (900 / dist as u64).clamp(3, 9);
    let dig = dist as u64 * speed;
    let glory = 1100u64;
    let period = dig + glory;
    let local = (t + (cv.roll(6) as u64 % period)) % period;
    let face = start + (local / speed).min(dist as u64) as i32;
    let open = local >= dig;
    let since = local.saturating_sub(dig);
    let inside = |x: i32, y: i32| -> f32 {
        let (dx, dy) = ((x - gx) as f32 / rx as f32, (y - gy) as f32 / ry as f32);
        dx * dx + dy * dy
    };
    let ty0 = (fl - 3).max(surf + 1);
    for y in (gy - ry - 1).max(surf + 1)..=(gy + ry + 1).min(h - 1) {
        for x in (gx - rx - 1).max(0)..=(gx + rx + 1).min(w - 1) {
            let d = inside(x, y);
            if d > 1.15 {
                continue;
            }
            let hs = hash(x * 5 + salt, y * 11);
            if !open {
                // Only a glint now and then hints at it.
                if d < 1.0 && hs % 23 == 0 && (t / 9 + hs as u64) % 13 == 0 {
                    cv.glyph(x, y, '·', rgb(facet(hs)));
                }
                continue;
            }
            if d > 0.72 {
                // The crystal rind.
                let ch = ['▲', '♦', '▼', '◄', '►', '♦'][(hs % 6) as usize];
                let spark = (t / 4 + hs as u64) % 37 == 0;
                let fg = if spark { (255, 255, 255) } else { facet(hs) };
                cv.put(x, y, if spark { '*' } else { ch }, rgb(fg), Some(rgb(lerp(deep, (0, 0, 0), 0.45))));
            } else {
                let pulse = ((tf * 0.05 + x as f32 * 0.2).sin() * 0.5 + 0.5) * 0.25;
                let bg = lerp(lerp(glowc, (0, 0, 0), 0.5), glowc, pulse + (1.0 - d) * 0.5);
                let ch = if hs % 9 == 0 { '·' } else { ' ' };
                cv.put(x, y, ch, rgb(bright), Some(rgb(bg)));
            }
        }
    }

    // The shaft and its headframe.
    for y in surf.max(0)..fl {
        for x in shaft - 1..=shaft + 1 {
            let edge = x != shaft;
            cv.put(x, y, if edge { '║' } else { ' ' }, rgb((120, 86, 50)), Some(rgb((16, 12, 10))));
        }
    }
    if surf >= 2 {
        cv.glyph(shaft - 1, surf - 1, '/', rgb((140, 100, 60)));
        cv.glyph(shaft + 1, surf - 1, '\\', rgb((140, 100, 60)));
        cv.glyph(shaft, surf - 1, 'Θ', rgb((170, 130, 80)));
        if surf >= 3 {
            cv.glyph(shaft, surf - 2, '▲', rgb((140, 100, 60)));
        }
    }
    let lift = pingpong(t, 4, surf + 1, (fl - 1).max(surf + 1));
    for y in surf..lift {
        cv.glyph(shaft, y, '│', rgb((90, 80, 70)));
    }
    cv.glyph(shaft, lift, '▄', rgb((150, 110, 60)));

    // The drift: dark, timbered, lantern-lit, out to the face.
    let reach = if open { gx - rx + 1 } else { face };
    for y in ty0..fl {
        for x in shaft + 2..reach.min(w) {
            let lamp = ((x - shaft) % 12) as f32;
            let warm = (1.0 - (lamp - 6.0).abs() / 7.0).max(0.0) * 0.5;
            let spill = if open { (1.0 - (gx - rx - x) as f32 / (w as f32 * 0.3).max(8.0)).max(0.0) * 0.6 } else { 0.0 };
            let bg = lerp(lerp((22, 16, 12), (90, 60, 30), warm), glowc, spill);
            cv.put(x, y, ' ', Color::Black, Some(rgb(bg)));
        }
    }
    for x in shaft - 1..reach.min(w) {
        cv.put(x, fl, '═', rgb((120, 110, 100)), Some(rgb((40, 32, 26))));
    }
    let mut px = shaft + 8;
    while px < reach - 2 {
        cv.glyph(px, ty0, '╥', rgb((140, 100, 60)));
        for y in ty0 + 1..fl {
            cv.glyph(px, y, '║', rgb((120, 86, 50)));
        }
        let on = (t / 9 + px as u64) % 23 != 0;
        cv.glyph(px + 6, ty0, '☼', rgb(if on { (255, 210, 110) } else { (200, 140, 60) }));
        px += 12;
    }

    // On big screens, an older level further down, long since worked.
    let low = fl + (h - fl) * 3 / 5;
    if h - fl > 16 && w >= 60 {
        let end = (w as f32 * 0.55) as i32;
        for y in fl + 1..low {
            for x in shaft - 1..=shaft + 1 {
                cv.put(x, y, if x != shaft { '║' } else { ' ' }, rgb((120, 86, 50)), Some(rgb((16, 12, 10))));
            }
        }
        for y in low - 2..low {
            for x in shaft + 2..end {
                let lamp = ((x - shaft) % 10) as f32;
                let warm = (1.0 - (lamp - 5.0).abs() / 6.0).max(0.0) * 0.45;
                cv.put(x, y, ' ', Color::Black, Some(rgb(lerp((20, 15, 12), (90, 60, 30), warm))));
            }
        }
        for x in shaft - 1..end {
            cv.put(x, low, '═', rgb((110, 100, 90)), Some(rgb((36, 30, 24))));
        }
        let mut lx = shaft + 7;
        while lx < end - 1 {
            let on = (t / 11 + lx as u64) % 19 != 0;
            cv.glyph(lx, low - 2, '☼', rgb(if on { (255, 210, 110) } else { (200, 140, 60) }));
            lx += 10;
        }
        let cx = pingpong(t, 4, shaft + 3, end - 6);
        cv.sprite(cx, low - 1, &["▐█▌"], rgb((120, 86, 50)));
        cv.glyph(cx + 1, low - 2, '▲', rgb((150, 140, 130)));
        cv.glyph(cx - 1, low - 1, '☺', rgb((200, 170, 220)));
        let swing = (t / 7) % 2 == 0;
        cv.glyph(end - 2, low - 1, '☺', rgb((240, 160, 120)));
        cv.glyph(end - 1, low - 2, if swing { '/' } else { '─' }, rgb((170, 170, 180)));
    }

    // The cart, going back and forth with its load.
    if reach - shaft > 14 {
        let (c0, c1) = (shaft + 3, reach - 9);
        let cx = pingpong(t, 3, c0, c1);
        let load = if open { facet(1) } else { (130, 120, 110) };
        let wood = rgb((120, 86, 50));
        cv.glyph(cx, fl - 1, '▐', wood);
        cv.glyph(cx + 1, fl - 1, '█', wood);
        cv.glyph(cx + 2, fl - 1, '▌', wood);
        cv.glyph(cx + 1, fl - 2, if open { '♦' } else { '▲' }, rgb(load));
        let back = (t / 3 / ((c1 - c0).max(1) as u64)) % 2 == 1;
        cv.glyph(if back { cx + 3 } else { cx - 1 }, fl - 1, '☺', rgb((150, 200, 240)));
    }

    // The miners at the face, or, once through, at the crystals.
    let swing = (t / 6) % 2 == 0;
    let picks = rgb((170, 170, 180));
    if !open {
        cv.glyph(face - 1, fl - 1, '☺', rgb((235, 190, 110)));
        cv.glyph(face, fl - 2, if swing { '/' } else { '─' }, picks);
        if face - 4 > shaft + 2 {
            cv.glyph(face - 3, fl - 1, '☺', rgb((210, 140, 90)));
            cv.glyph(face - 2, fl - 2, if swing { '─' } else { '\\' }, picks);
        }
        // Chips flying back off the rock.
        if !swing {
            for k in 0..3 {
                let age = ((t / 2) as i32 + k * 3) % 8;
                let y = fl - 2 - (age % 3) + if age > 4 { 1 } else { 0 };
                cv.glyph(face - 1 - age, y.min(fl - 1), '·', rgb((170, 150, 130)));
            }
        }
        cv.glow(face, fl - 1, (180, 140, 90), 0.4);
    } else {
        // The breakthrough: a burst of light, cheering, then careful work.
        let lip = gx - rx;
        if since < 40 {
            cv.halo(lip, gy, 2.0 + since as f32 * 0.3, (255, 255, 255), 0.7 * (1.0 - since as f32 / 40.0));
        }
        let cheer = since < 220;
        let hop = if cheer && (t / 5) % 2 == 0 { 1 } else { 0 };
        cv.glyph(lip - 2, fl - 1 - hop, '☺', rgb((235, 190, 110)));
        cv.glyph(lip - 4, fl - 1 - (1 - hop), '☺', rgb((210, 140, 90)));
        if cheer {
            cv.glyph(lip - 2, fl - 2 - hop, '!', rgb((255, 240, 160)));
            cv.glyph(lip - 4, fl - 2 - (1 - hop), '♪', rgb((255, 240, 160)));
        } else {
            cv.glyph(lip - 1, fl - 2, if swing { '/' } else { '─' }, picks);
            cv.glyph(lip - 3, fl - 2, '¡', rgb((255, 200, 90)));
        }
        // Motes of light drifting about the hollow.
        let n = if big { 14 } else { 6 };
        for k in 0..n {
            let a = tf * 0.02 + k as f32 * 2.4;
            let r = 0.3 + (hash(k, 7) % 60) as f32 / 100.0;
            let x = gx + (a.cos() * rx as f32 * r) as i32;
            let y = gy + ((a * 1.3).sin() * ry as f32 * r) as i32;
            cv.glyph(x, y, if (t / 5 + k as u64) % 3 == 0 { '+' } else { '·' }, rgb(facet(k as u32)));
        }
    }

    // Deep down: an underground stream, and old bones in the rock.
    if h - fl > 6 {
        let sy = h - 2;
        for x in 0..w {
            let y = sy + ((x as f32 * 0.09 + p0).sin() * 0.8).round() as i32;
            for yy in y..(y + 2).min(h) {
                let ch = if (x + yy + (t / 7) as i32) % 5 == 0 { '~' } else { '≈' };
                cv.put(x, yy, ch, rgb((90, 140, 200)), Some(rgb((18, 34, 66))));
            }
        }
        for k in 0..(w / 40).max(1) {
            let x = (hash(k + salt, 13) % w.max(1) as u32) as i32;
            let y = fl + 3 + (hash(k, 17) % ((h - fl - 6).max(1) as u32)) as i32;
            if inside(x, y) > 1.3 && inside(x + 3, y) > 1.3 {
                cv.sprite(x, y, &["~°═°"], rgb((180, 170, 150)));
            }
        }
    }
}
