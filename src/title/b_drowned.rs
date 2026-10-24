//! A tunnel cut too close to an underground lake. The wall weeps, then
//! bursts; water roars in and climbs while the miners scramble for the
//! stair, until the floodgate drops and the pumps grind it back down,
//! leaving mud and one bewildered fish. Below it all, the old halls that
//! drowned long ago, where bubbles still rise past the statues. The seed
//! picks the stone, the water's colour and where the wall gives.

use super::b_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the wall was wet. that should have been a clue",
    "everyone up the stairs. leave the barrows",
    "the lake was here first",
    "below us, the halls our grandparents drowned",
];

/// Water: the bright surface, the body, the deep.
const WATERS: [(Rgb, Rgb, Rgb); 3] = [
    ((130, 190, 240), (24, 60, 110), (6, 16, 40)),
    ((120, 220, 200), (16, 70, 80), (4, 22, 30)),
    ((170, 170, 230), (40, 44, 100), (10, 10, 36)),
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t) = (cv.w(), cv.h(), cv.t);
    let g = (cv.sky - 1).max(2).min(h - 1);
    let th = ((g as f32 * 0.42) as i32).clamp(2, 7);
    let ty = (g - th).max(0);
    let stone = [(74, 66, 62), (62, 66, 74), (80, 70, 56)][(cv.roll(1) % 3) as usize];
    let (surf, body, deep) = WATERS[(cv.roll(2) % 3) as usize];
    let salt = cv.salt();
    let tf = t as f32;
    let air = lerp(stone, (0, 0, 0), 0.8);
    bedrock(cv, 0, h, stone, salt);

    // The beat: a weeping wall, the burst, the rise, the gate, the pumps.
    let at = beat(cv, 1500, 3);
    let (burst, full, gate, drained) = (220u64, 600u64, 760u64, 1100u64);
    let level = if at < burst {
        0.0
    } else if at < gate {
        span(at, burst, full)
    } else {
        1.0 - span(at, gate + 40, drained)
    };
    let lv = (level * (th as f32 + 0.5)) as i32; // rows of water
    let wy = g - lv; // the surface row

    // The tunnel, from the breach to the stair.
    let bx = 2 + (cv.roll(4) % 4) as i32; // the breach
    let sx = (w - 4 - (cv.roll(5) % (w.max(30) as u32 / 15)) as i32).max(bx + 6); // the stair
    hollow(cv, bx, ty, sx + 2, g, air);
    floor(cv, bx, sx + 2, g, lerp(stone, (140, 130, 120), 0.3));
    for (k, lx) in (0..).map(|k| bx + 6 + k * 13).take_while(|&x| x < sx - 2).enumerate() {
        torch(cv, lx, ty, k as i32, 3.0);
    }
    // The stair up and out, and the floodgate in front of it.
    for y in 0..g {
        cv.put(sx + 1, y, if (y + g) % 2 == 0 { '═' } else { '╪' }, rgb((120, 90, 60)), Some(rgb(air)));
    }
    let gx = sx - 1;
    let gate_down = if at >= gate && at < drained + 120 { span(at, gate, gate + 30) } else { 0.0 };
    for y in ty..ty + (gate_down * th as f32) as i32 {
        cv.put(gx, y, '█', rgb((110, 112, 120)), Some(rgb((40, 40, 46))));
    }
    if gate_down == 0.0 {
        cv.glyph(gx, ty, '▄', rgb((110, 112, 120)));
    }

    // The breach: weeping, then gushing.
    let lake = lerp(body, (0, 0, 0), 0.2);
    for y in ty..g {
        cv.put(bx - 1, y, ' ', Color::Black, Some(rgb(lake)));
    }
    if at < burst {
        cv.glyph(bx, ty + 1, '░', rgb(lerp(stone, body, 0.5)));
        for k in 0..2u64 {
            let y = ty + 1 + ((t / 4 + k * 5) % (th.max(2) as u64 - 1)) as i32;
            cv.glyph(bx, y, '\'', rgb(surf));
        }
    } else if at < gate {
        let force = 1.0 - span(at, full - 100, gate) * 0.7;
        let len = ((at - burst) as f32 * 0.6).min(w as f32 * 0.4) * force;
        let jy = ty + th / 2;
        for i in 0..len as i32 {
            let y = jy + ((i as f32 * i as f32) * 0.004) as i32;
            let ch = if (i + (t / 2) as i32) % 3 == 0 { '~' } else { '≈' };
            cv.put(bx + i, y.min(g - 1), ch, rgb(surf), Some(rgb(body)));
        }
        for k in 0..6 {
            let a = (hash(k, (t / 3) as i32) % 100) as i32;
            cv.glyph(bx + (len as i32 * a / 100), jy - 1 - (a % 2), '°', rgb((230, 240, 255)));
        }
    }

    // The water in the tunnel.
    if lv > 0 {
        let end = if gate_down > 0.0 { gx } else { sx + 2 };
        for y in wy.max(ty)..g {
            for x in bx..end {
                let wave = ((x as f32 * 0.4 - tf * 0.12).sin() + (x as f32 * 0.13 + tf * 0.05).sin()) * 0.25 + 0.5;
                if y == wy {
                    let ch = if (x + (t / 6) as i32) % 4 == 0 { '~' } else { '≈' };
                    cv.put(x, y, ch, mix(surf, (255, 255, 255), wave * 0.3), Some(mix(body, surf, wave * 0.25)));
                } else {
                    cv.put(x, y, ' ', Color::Black, Some(mix(body, deep, (y - wy) as f32 / th.max(1) as f32)));
                }
            }
        }
        // Bubbles rising through it.
        for k in 0..(w / 8).max(3) {
            let x = bx + (hash(k, salt) % (gx - bx).max(1) as u32) as i32;
            let rise = ((t / 4 + hash(k, 3) as u64) % (lv.max(1) as u64 + 1)) as i32;
            let y = g - 1 - rise;
            if y > wy {
                cv.glyph(x, y, if k % 3 == 0 { 'o' } else { '°' }, rgb(lerp(surf, (255, 255, 255), 0.4)));
            }
        }
    }
    // Mud once it's down, and a stranded fish flapping in it.
    if at >= drained || (at >= gate && level < 0.15) {
        for x in bx..gx {
            if hash(x, salt + 4) % 3 == 0 {
                cv.glyph(x, g - 1, '≈', rgb(lerp(stone, body, 0.4)));
            }
        }
        let fx = bx + (gx - bx) / 3 + (cv.roll(6) % 6) as i32;
        let flop = (t / 7) % 3;
        cv.glyph(fx, g - 1 - (flop == 1) as i32, if flop == 2 { '>' } else { '<' }, rgb((200, 210, 220)));
    }
    // The pumps by the stair, cranked hard while it drains.
    let pumping = at >= gate + 30 && at < drained + 60;
    let crank = if pumping { ['|', '/', '─', '\\'][((t / 3) % 4) as usize] } else { '|' };
    cv.glyph(sx + 2, ty - 1, crank, rgb((180, 150, 100)));
    cv.glyph(sx + 3, ty - 1, '§', rgb((150, 150, 160)));
    if pumping {
        cv.glyph(sx + 2, ty - 2, '☺', rgb(beard(cv.roll(7))));
        for k in 0..3u64 {
            let y = ty - 3 - ((t / 2 + k * 3) % 5) as i32;
            cv.glyph(sx + 4, y, if k % 2 == 0 { '≈' } else { '~' }, rgb(surf));
        }
    }

    // The miners: at work, then up the stair, then back to brick the wall.
    let b0 = beard(cv.roll(8));
    let b1 = beard(cv.roll(8) + 3);
    let swing = (t / 6) % 2 == 0;
    let mx = bx + 3 + (gx - bx) / 3;
    if at < burst + 10 {
        cv.glyph(mx, g - 1, '☺', rgb(b0));
        cv.glyph(mx + 1, g - 2, if swing { '/' } else { '─' }, rgb((170, 170, 180)));
        let hx = pingpong(t, 4, mx + 4, (gx - 3).max(mx + 5));
        cv.glyph(hx, g - 1, '☺', rgb(b1));
        if at + 60 > burst && (t / 10) % 2 == 0 {
            cv.glyph(mx, g - 2, '?', rgb((255, 240, 160)));
        }
    } else if at < gate {
        // Fleeing: wading, swimming, then up the stair.
        let run = ((at - burst - 10) / 2) as i32;
        let x0 = (mx + run).min(sx + 1);
        let y = if x0 > sx { (g - 1 - (mx + run - sx) / 2).max(0) } else { (g - 1).min(wy - 1).max(ty) };
        cv.glyph(x0, y, '☺', rgb(b0));
        if x0 <= sx && lv > 0 {
            cv.glyph(x0, y - 1, '‼', rgb((255, 240, 160)));
        }
        cv.glyph(sx + 1, (g - 1 - run / 2).max(0), '☺', rgb(b1));
    } else if at >= drained {
        // The masons brick up the breach.
        let built = span(at, drained + 60, 1450);
        for y in g - (built * th as f32) as i32..g {
            cv.put(bx, y, '▓', rgb((150, 140, 130)), Some(rgb(lerp(stone, (0, 0, 0), 0.3))));
        }
        let hx = pingpong(t, 3, bx + 3, (bx + 9).min(gx - 1));
        cv.glyph(hx, g - 1, '☺', rgb(b0));
        cv.glyph(bx + 1, g - 1, '☺', rgb(b1));
        cv.glyph(bx + 2, g - 2, if swing { '¬' } else { '─' }, rgb((170, 170, 180)));
    }

    // Down below: the halls that drowned long ago.
    let top = g + 3;
    if h - top >= 3 {
        for y in top..h {
            let f = (y - top) as f32 / (h - top).max(1) as f32;
            for x in 0..w {
                let wave = ((x as f32 * 0.21 + y as f32 * 0.7 - tf * 0.03).sin() * 0.5 + 0.5) * 0.25;
                let bg = mix(lerp(body, deep, 0.4), deep, f - wave);
                let ch = if y == top && (x + (t / 9) as i32) % 5 == 0 { '~' } else if y == top { '≈' } else { ' ' };
                cv.put(x, y, ch, rgb(lerp(surf, body, 0.4)), Some(bg));
            }
        }
        // Old pillars and statues standing in the murk.
        let sp = (w / 7).max(6);
        let mut x = sp / 2 + (cv.roll(9) % 4) as i32;
        let mut k = 0;
        let ph = (h - top - 3).clamp(1, 8);
        while x < w {
            if k % 3 == 1 {
                cv.glyph(x, h - 2, 'Ω', rgb(lerp(stone, surf, 0.3)));
                cv.glyph(x, h - 3, '☺', rgb(lerp(stone, surf, 0.35)));
            } else {
                for y in h - 1 - ph..h - 1 {
                    cv.glyph(x, y, '║', rgb(lerp(stone, body, 0.45)));
                }
                cv.glyph(x, h - 2 - ph, '╦', rgb(lerp(stone, body, 0.45)));
            }
            x += sp + (hash(x, salt) % 5) as i32;
            k += 1;
        }
        floor(cv, 0, w, h - 1, lerp(stone, deep, 0.5));
        // Bubbles forever rising from the vents.
        for k in 0..(w / 5).max(4) {
            let x = (hash(k, salt + 7) % w.max(1) as u32) as i32;
            let speed = 3 + (k % 3) as u64;
            let rows = (h - top - 1).max(1) as u64;
            let y = h - 2 - ((t / speed + hash(k, 1) as u64) % rows) as i32;
            let wob = ((tf * 0.1 + k as f32).sin() * 0.8) as i32;
            cv.glyph(x + wob, y, if k % 4 == 0 { 'o' } else { '°' }, rgb(lerp(surf, (255, 255, 255), 0.3)));
        }
        // A pale cave fish nosing about.
        let off = t + cv.roll(10) as u64 % 500;
        let run = (w - 5).max(1) as u64;
        let fx = pingpong(off, 5, 2, w - 3);
        let right = (off / 5) % (2 * run) < run;
        cv.glyph(fx, top + (h - top) / 2, if right { '►' } else { '◄' }, rgb((220, 220, 200)));
    } else {
        // No room below: a bubble or two by the breach.
        cv.glyph(bx, (g - 1).max(0), '°', rgb(surf));
    }
}
