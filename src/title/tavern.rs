//! The tavern at night: a roaring hearth, dwarves raising tankards, a bard
//! sending tunes up into the rafters, a cat doing its rounds, and the
//! weather at the window. The seed sets the weather, the bard's
//! instrument, the cat's coat and how full the house is.

use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "ale first, then everything else",
    "a song for the lost, a round for the living",
    "the hearth is warm and the barrels are full",
    "tell the one about the elf and the anvil again",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, sky, seed) = (cv.w(), cv.h(), cv.t, cv.sky, cv.seed);
    let g = sky - 1; // the floorboards
    let hearth = (1, (w / 9).clamp(7, 15).min(w / 3).max(5)); // stone surround, left and right edges
    let body = (g / 2).clamp(4, 9); // rows of stonework above the floor
    let firebox = (body - 2).max(2);
    let flicker = ((t as f32 * 0.31).sin() * 0.5 + (t as f32 * 0.73).sin() * 0.3) * 0.12 + 0.88;
    let fire_at = ((hearth.0 + hearth.1) / 2, g - 2);
    let light = |x: i32, y: i32| -> f32 {
        let (dx, dy) = ((x - fire_at.0) as f32 * 0.5, (y - fire_at.1) as f32);
        ((1.0 - (dx * dx + dy * dy).sqrt() / (w as f32 / 3.0).max(12.0)).max(0.0) * flicker).min(1.0)
    };

    // Back wall of planks, rafters above, warm where the fire reaches.
    for y in 0..g.min(h) {
        for x in 0..w {
            let l = light(x, y);
            let rafter = y == 0 || (y == 2 && g > 6);
            let base = if rafter { (24, 14, 8) } else { (44, 28, 16) };
            let bg = mix(base, (130, 76, 34), l * 0.75);
            let ch = if rafter {
                '═'
            } else if (x + (y / 3) * 3) % 7 == 0 {
                '│'
            } else {
                ' '
            };
            cv.put(x, y, ch, mix((30, 18, 10), (90, 56, 30), l), Some(bg));
        }
    }

    // Lanterns hung from the rafters.
    let mut lx = w / 5;
    while lx < w {
        cv.glyph(lx, 1, '│', rgb((70, 60, 50)));
        let on = (t / 7 + lx as u64) % 13 != 0;
        cv.glyph(lx, 2, '☼', rgb(if on { (255, 205, 100) } else { (210, 150, 70) }));
        lx += (w / 5).max(8);
    }

    // The floor, and the room seen from above below it.
    for y in g.max(0)..h {
        for x in 0..w {
            let l = light(x, y);
            let seam = y == g || (x + (y * 7) % 13) % 13 == 0;
            let bg = mix((52, 32, 16), (130, 80, 38), l * 0.6);
            let ch = if y == g { '═' } else if seam { '│' } else { ' ' };
            cv.put(x, y, ch, mix((34, 20, 10), (90, 58, 30), l), Some(bg));
        }
    }

    // The hearth.
    let stone = rgb((88, 82, 78));
    for y in 0..g {
        for x in hearth.0..=hearth.1 {
            let inner = x > hearth.0 + 1 && x < hearth.1 - 1;
            if inner && y >= g - firebox {
                continue;
            }
            let chimney = x > hearth.0 + 2 && x < hearth.1 - 2;
            if y < g - body - 1 && !chimney {
                continue;
            }
            let ch = if (x + (y % 2) * 2) % 4 == 0 { '│' } else { '▒' };
            cv.put(x, y, ch, rgb((104, 98, 92)), Some(stone));
        }
    }
    for y in (g - firebox).max(0)..g {
        for x in hearth.0 + 2..hearth.1 - 1 {
            let hs = hash(x * 13 + y, (t / 3) as i32);
            let hot = (y - (g - firebox)) as f32 / firebox as f32;
            let tall = (g - y) as f32 / firebox as f32;
            let lick = ((x as f32 * 1.3 + t as f32 * 0.4).sin() * 0.5 + 0.5) * 0.6 + 0.3;
            if tall > lick + 0.25 {
                cv.put(x, y, ' ', Color::Black, Some(rgb((40, 12, 4))));
                continue;
            }
            let col = [(255, 235, 110), (255, 160, 40), (230, 70, 20)][((hs % 3) as f32 * (1.0 - hot * 0.5)) as usize % 3];
            let ch = if y == g - 1 { if hs % 2 == 0 { '═' } else { '▲' } } else if hs % 3 == 0 { ' ' } else { '▲' };
            let fg = if y == g - 1 && ch == '═' { (120, 70, 30) } else { col };
            cv.put(x, y, ch, rgb(fg), Some(mix((60, 16, 4), (120, 40, 8), flicker - 0.7)));
        }
    }
    // Sparks out of the fire.
    for k in 0..3 {
        let age = ((t / 3) as i32 + k * 5) % 12;
        let sx = fire_at.0 + 1 + age / 3 + (hash(k, (t / 36) as i32) % 3) as i32;
        let sy = g - firebox - age / 4;
        if age < 9 {
            cv.glyph(sx, sy, '·', rgb((255, 200, 90)));
        }
    }

    // The window, and the night outside.
    let weather = cv.roll(1) % 3;
    let (wx0, wx1) = ((w - 12).max(hearth.1 + 3), (w - 4).max(hearth.1 + 6));
    let (wy0, wy1) = (2.min(g - 3).max(1), (g - 3).max(2).min(2 + (g / 2)));
    for y in wy0..=wy1 {
        for x in wx0..=wx1 {
            let frame = y == wy0 || y == wy1 || x == wx0 || x == wx1 || x == (wx0 + wx1) / 2;
            if frame {
                cv.put(x, y, ' ', Color::Black, Some(rgb((70, 44, 22))));
            } else {
                let hs = hash(x * 17, y * 3);
                cv.fill(x, y, rgb((8, 10, 28)));
                if weather == 0 && hs % 6 == 0 {
                    cv.glyph(x, y, '·', rgb((170, 170, 210)));
                }
            }
        }
    }
    if weather == 0 && wx1 - wx0 > 3 {
        cv.glyph(wx1 - 2, wy0 + 1, '●', rgb((240, 235, 205)));
    }
    if weather > 0 && wx1 > wx0 + 1 && wy1 > wy0 + 1 {
        for k in 0..8 {
            let inner_w = (wx1 - wx0 - 1).max(1);
            let inner_h = (wy1 - wy0 - 1).max(1);
            let y = wy0 + 1 + ((hash(k, 3) as u64 + t / if weather == 1 { 6 } else { 2 }) % inner_h as u64) as i32;
            let x = wx0 + 1 + ((hash(k, 4) as u64 + t / 9) % inner_w as u64) as i32;
            if x != (wx0 + wx1) / 2 {
                let (ch, col) = if weather == 1 { ('*', (235, 235, 250)) } else { ('/', (110, 130, 180)) };
                cv.glyph(x, y, ch, rgb(col));
            }
        }
    }

    // Barrels under the window.
    let bx = wx0;
    for (dx, dy) in [(0, 1), (1, 1), (2, 1), (3, 1), (1, 2), (2, 2)] {
        cv.glyph(bx + dx, g - dy, 'Θ', rgb((150, 100, 50)));
    }

    // The bard by the barrels; the tune floats up.
    let bard = (bx - 3).max(hearth.1 + 2);
    let tune = [('♪', '♫'), ('♫', '♪'), ('♪', '♪')][(cv.roll(2) % 3) as usize];
    cv.glyph(bard, g - 1, '☻', rgb((220, 160, 255)));
    let strum = (t / 5) % 2 == 0;
    cv.glyph(bard - 1, g - 1, if strum { '/' } else { '─' }, rgb((180, 130, 70)));
    for k in 0..5 {
        let age = ((t / 4) as i32 + k * 7) % 35;
        let y = g - 2 - age / 3;
        let x = bard - age / 4 + ((age as f32 * 0.6).sin() * 1.5) as i32;
        if y >= 0 {
            let ch = if k % 2 == 0 { tune.0 } else { tune.1 };
            let fade = age as f32 / 35.0;
            cv.glyph(x, y, ch, mix((255, 230, 160), (120, 80, 50), fade));
        }
    }

    // The drinkers, at tables along the floor, raising their tankards.
    let room = (hearth.1 + 3, bard - 3);
    // A shelf of bottles and kegs on the back wall above them.
    let shelf = g - (body - 1).max(3);
    // Hangings on the wall above, when there's wall enough for them.
    if shelf - 3 > 6 {
        let n = ((room.1 - room.0) / 22).max(1);
        for k in 0..n {
            let hx = room.0 + 6 + k * (room.1 - room.0 - 8) / n.max(1) + (hash(k, seed as i32) % 4) as i32;
            let col = [(120, 24, 30), (30, 50, 120), (30, 90, 50), (90, 40, 110)][((roll(seed, 40 + k as u32)) % 4) as usize];
            let (top, bottom) = (4, shelf - 3);
            for y in top..=bottom {
                for x in hx - 2..=hx + 2 {
                    let edge = x == hx - 2 || x == hx + 2;
                    let ch = if y == bottom { '▼' } else if edge { '│' } else { ' ' };
                    cv.put(x, y, ch, rgb(lerp(col, (230, 190, 90), 0.5)), Some(rgb(col)));
                }
            }
            let emblem = ['☼', '♦', '▲', '♣'][(roll(seed, 50 + k as u32) % 4) as usize];
            cv.glyph(hx, (top + bottom) / 2, emblem, rgb((240, 200, 90)));
        }
    }
    if shelf > 2 {
        for x in room.0..room.1 {
            cv.glyph(x, shelf, '▀', rgb((110, 70, 36)));
            let hs = hash(x, 97 + seed as i32);
            let bottle = [(90, 160, 90), (170, 60, 50), (200, 170, 80), (120, 110, 190), (220, 220, 220)][(hs % 5) as usize];
            if hs % 3 != 0 {
                cv.glyph(x, shelf - 1, if hs % 7 == 0 { 'Θ' } else { '¡' }, rgb(bottle));
            }
        }
    }
    let crowd = 2 + cv.roll(3) % 3 + (w / 50) as u32;
    let tables = ((room.1 - room.0) / 9).clamp(0, crowd as i32);
    let coats = [(120, 200, 255), (255, 150, 120), (160, 255, 150), (255, 230, 140), (230, 180, 255), (200, 200, 200)];
    for k in 0..tables {
        let cx = room.0 + 4 + k * ((room.1 - room.0 - 4) / tables.max(1));
        cv.glyph(cx, g - 1, '╥', rgb((170, 120, 70)));
        let cheer = (t + k as u64 * 50) % 220 < 30;
        for (i, side) in [-1, 1].iter().enumerate() {
            let dx = side * (1 + (hash(k, i as i32) % 2) as i32);
            let x = cx + dx;
            let col = coats[(hash(k, 40 + i as i32).wrapping_add(seed as u32) % 6) as usize];
            cv.glyph(x, g - 1, '☺', rgb(col));
            let sip = (t / 40 + k as u64 + i as u64) % 5 == 0;
            if cheer || sip {
                cv.glyph(x, g - 2, 'Ü', rgb((235, 190, 70)));
                if cheer && (t / 6) % 2 == 0 {
                    cv.glyph(x + side, g - 3, '!', rgb((255, 240, 180)));
                }
            }
        }
    }

    // The cat on its rounds, or curled by the fire.
    let coat = [(230, 150, 70), (150, 150, 160), (60, 55, 60), (240, 240, 235)][(cv.roll(4) % 4) as usize];
    let round = (t + seed % 2000) % 2000;
    if round < 1200 {
        let x = pingpong(round, 5, hearth.1 + 1, bard - 2);
        cv.glyph(x, g, 'c', rgb(coat));
    } else {
        cv.glyph(hearth.1 + 1, g, 'c', rgb(coat));
        if (t / 30) % 2 == 0 {
            cv.glyph(hearth.1 + 2, g - 1, 'z', rgb((180, 180, 200)));
        }
    }

    // Tables in the room below, with company.
    if h - g > 8 {
        let mut ty = g + 3;
        let mut row = 0;
        while ty < h - 2 {
            let mut tx = 4 + (row % 2) * 6;
            while tx < w - 8 {
                if hash(tx, ty + seed as i32) % 3 != 0 {
                    for dx in 0..4 {
                        cv.put(tx + dx, ty, '═', rgb((160, 110, 60)), Some(rgb((96, 62, 30))));
                    }
                    for (i, (sx, sy)) in [(-1, 0), (4, 0), (1, -1), (2, 1)].iter().enumerate() {
                        if hash(tx + i as i32, ty) % 3 == 0 {
                            continue;
                        }
                        let col = coats[(hash(tx * 7 + i as i32, ty) % 6) as usize];
                        cv.glyph(tx + sx, ty + sy, '☺', rgb(col));
                    }
                    let mug = (t / 30 + tx as u64 + ty as u64) % 6 == 0;
                    cv.glyph(tx + 1 + (hash(tx, ty) % 2) as i32, ty, if mug { ' ' } else { 'Ü' }, rgb((235, 190, 70)));
                }
                tx += 13;
            }
            ty += 5;
            row += 1;
        }
    }
}
