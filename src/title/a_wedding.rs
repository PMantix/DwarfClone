//! A wedding feast out of doors: long tables in the meadow under strings
//! of bunting, the whole hold seated with mugs raised in toast after
//! toast, the couple garlanded at the head table, a ring of dancers
//! turning to the fiddler's tune, a pig on the spit, and petals on the
//! breeze. As the evening comes on, the lanterns are lit. The seed picks
//! the hour, the colours and the size of the party.

use super::a_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "two clans, one table, a great deal of ale",
    "may your tunnels be straight and your mugs full",
    "everybody dances; even the militia commander",
    "the bride wore steel, the groom wore flowers",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let g = (cv.sky - 1).max(1);
    let tf = t as f32;
    let salt = cv.salt();
    let evening = cv.roll(1) % 2 == 0;
    let (top, bot) = if evening { ((40, 40, 100), (240, 150, 110)) } else { ((80, 140, 220), (220, 236, 245)) };
    let pal: [Rgb; 4] = [[(230, 80, 80), (250, 220, 90), (90, 160, 230), (240, 240, 240)], [(240, 140, 200), (250, 250, 250), (140, 210, 140), (250, 200, 80)], [(200, 60, 60), (60, 120, 200), (250, 210, 60), (90, 180, 90)]][(cv.roll(2) % 3) as usize];

    cv.sky_fill(g, top, bot);
    if evening {
        cv.stars(g / 3, 47);
    } else {
        clouds(cv, (w / 35).max(1), (1, (g / 4).max(2)), 11, (240, 244, 250), (255, 255, 255), salt);
    }
    // Hills with the hold's gate in them.
    for x in 0..w {
        let s = g - 1 - (ridge(x, w, seed, 190) * g as f32 * 0.5) as i32;
        for y in s.max(0)..g {
            let ch = if y == s { '▄' } else if hash(x + salt, y) % 7 == 0 { '♣' } else { ' ' };
            let bg = lerp(lerp((90, 130, 70), (70, 100, 56), (y - s) as f32 / 6.0), bot, 0.25);
            cv.put(x, y, ch, rgb(lerp(bg, (40, 80, 40), 0.5)), Some(rgb(bg)));
        }
    }

    // The meadow.
    for y in g.max(0)..h {
        for x in 0..w {
            let hs = hash(x * 3 + salt, y * 5);
            let ch = if hs % 6 == 0 { '"' } else if hs % 17 == 0 { '*' } else { ' ' };
            let fg = if ch == '*' { pal[(hs % 4) as usize] } else { (130, 180, 80) };
            cv.put(x, y, ch, rgb(fg), Some(rgb(lerp((96, 146, 64), (76, 120, 50), (hs % 8) as f32 / 10.0))));
        }
    }

    // Poles and bunting along the back of the feast.
    let poles = (w / 20).max(2);
    let gap = (w / poles).max(1);
    let by = (g - 5).max(1);
    for p in 0..=poles {
        let px = p * gap;
        for y in by..g {
            cv.glyph(px, y, '│', rgb((110, 80, 50)));
        }
        if p < poles {
            for i in 1..gap {
                let u = i as f32 / gap as f32;
                let sag = u * (1.0 - u) * 4.0 * 1.5 + (tf * 0.07 + p as f32 + u * 3.0).sin() * 0.4;
                let y = by + sag.round() as i32;
                let c = pal[((i + p) % 4) as usize];
                cv.glyph(px + i, y, if i % 2 == 0 { '▼' } else { '·' }, rgb(c));
                if evening && i % 4 == 0 {
                    cv.glyph(px + i, y + 1, '●', rgb((255, 210, 110)));
                    cv.glow(px + i, y + 1, (255, 200, 100), 0.35);
                }
            }
        }
    }

    // The long tables, guests behind them toasting.
    let row = g - 1;
    let tl = (w as f32 * 0.04) as i32;
    let tr = w - tl;
    let head = w / 2;
    for x in tl..tr {
        let food = hash(x + salt, 5);
        let (ch, fg) = match food % 7 {
            0 => ('%', (200, 140, 80)),
            1 => ('!', (230, 200, 120)),
            2 => ('ö', (190, 170, 130)),
            _ => ('─', (150, 110, 70)),
        };
        cv.put(x, row, ch, rgb(fg), Some(rgb((120, 84, 52))));
        if (x - tl) % 2 == 0 && (x - head).abs() > 2 {
            let hs = hash(x, salt);
            let toast = (t / 12 + hs as u64 % 9) % 9 == 0;
            dwarf(cv, x, row - 1 - toast as i32, COATS[(hs % 8) as usize]);
            if toast {
                cv.glyph(x + 1, row - 2, '!', rgb((240, 200, 90)));
            }
        }
    }
    // The couple, garlanded.
    dwarf(cv, head - 1, row - 1, (250, 250, 250));
    dwarf(cv, head + 1, row - 1, (200, 60, 60));
    cv.glyph(head - 1, row - 2, '*', rgb(pal[0]));
    cv.glyph(head + 1, row - 2, '*', rgb(pal[1]));
    if (t / 30) % 4 == 0 {
        cv.glyph(head, row - 2, '♥', rgb((250, 100, 120)));
    }

    // The ring of dancers to one side, below the table, turning.
    let (rx, ry) = ((w as f32 * 0.1) as i32 + 4, g + 3);
    let n = 8;
    for k in 0..n {
        let a = tf * 0.05 + k as f32 / n as f32 * std::f32::consts::TAU;
        let x = rx + (a.cos() * 4.0).round() as i32;
        let y = ry + (a.sin() * 1.5).round() as i32;
        dwarf(cv, x, y, COATS[((k + salt) % 8) as usize]);
    }
    cv.glyph(rx, ry, '♫', rgb((250, 230, 150)));
    // The fiddler, and notes going up.
    let fx = rx + 7;
    dwarf(cv, fx, ry - 1, (170, 120, 200));
    cv.glyph(fx + 1, ry - 1, if (t / 3) % 2 == 0 { '/' } else { '─' }, rgb((180, 130, 80)));
    for k in 0..3 {
        let age = ((t / 6) as i32 + k * 4) % 12;
        cv.glyph(fx + 1 + age / 4, ry - 2 - age / 2, if k % 2 == 0 { '♪' } else { '♫' }, rgb(lerp((250, 230, 150), bot, age as f32 / 12.0)));
    }

    // The spit on the other side, the fire under it, smoke going up.
    let sx = w - (w as f32 * 0.1) as i32 - 4;
    let sy = g + 3;
    cv.sprite(sx - 1, sy - 1, &["┬─────┬"], rgb((120, 110, 100)));
    cv.glyph(sx + 2, sy - 1, if (t / 20) % 2 == 0 { '@' } else { 'Ö' }, rgb((220, 150, 120)));
    for k in 0..3 {
        let hs = hash(k, (t / 3) as i32);
        cv.glyph(sx + 1 + k, sy, '▲', rgb([(255, 220, 110), (255, 140, 40), (230, 70, 30)][(hs % 3) as usize]));
    }
    cv.halo(sx + 2, sy, 3.0, (255, 160, 60), 0.4);
    for k in 0..5 {
        let age = ((t / 5) as i32 + k * 3) % 15;
        cv.glyph(sx + 2 + age / 5, sy - 2 - age / 2, '░', mix((200, 200, 200), bot, age as f32 / 15.0));
    }

    // Petals on the breeze.
    for k in 0..(w / 5).max(6) {
        let x = drift(t, k + salt, 2, w + 4) - 2;
        let y = ((hash(k, 9) as u64 + t / 5) % h.max(1) as u64) as i32 + ((x as f32 * 0.3 + k as f32).sin() * 1.2) as i32;
        cv.glyph(x, y, if k % 2 == 0 { '·' } else { '\'' }, rgb(pal[(k % 4) as usize]));
    }
}
