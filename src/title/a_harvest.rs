//! Harvest home: a low gold sun over fields of ripe grain, the wind
//! running through it in waves, a line of reapers swinging their scythes
//! across the top field and leaving stubble and sheaves behind, a hay
//! wain waiting, and crows wheeling for the gleanings. The seed picks the
//! crop, the hour and how many hands are out.

use super::a_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "bring it in before the frost does",
    "plump helmets below, barley above, ale everywhere",
    "a full granary is a quiet winter",
    "swing, step, swing: the oldest dance there is",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let g = (cv.sky - 1).max(1);
    let tf = t as f32;
    let salt = cv.salt();
    let late = cv.roll(1) % 2 == 0;
    let (top, bot) = if late { ((70, 60, 120), (250, 180, 110)) } else { ((80, 130, 200), (240, 220, 170)) };
    // The crop: wheat, barley or longland grass.
    let (ripe, dry) = [((230, 190, 80), (200, 150, 50)), ((220, 200, 120), (180, 160, 80)), ((200, 160, 110), (170, 120, 80))][(cv.roll(2) % 3) as usize];

    cv.sky_fill(g, top, bot);
    let (sx, sy) = ((w as f32 * 0.25) as i32, (g as f32 * if late { 0.65 } else { 0.35 }) as i32);
    cv.halo(sx, sy, (g as f32 * 0.45).max(2.0), (255, 200, 120), 0.5);
    cv.disc(sx, sy, (g as f32 * 0.09).clamp(0.7, 2.6), (255, 230, 160));
    clouds(cv, (w / 40).max(1), (1, (g / 3).max(2)), 12, lerp(bot, (255, 255, 255), 0.5), lerp(bot, (255, 255, 255), 0.8), salt);

    // Far hills, blue with distance, and hedges between the fields.
    for x in 0..w {
        let s = g - 1 - (ridge(x, w, seed, 60) * (g as f32 * 0.35)) as i32;
        for y in s.max(0)..g {
            let c = lerp(lerp(bot, (90, 110, 120), 0.5), (70, 90, 80), (y - s) as f32 / 4.0);
            cv.put(x, y, if y == s && hash(x, 61) % 3 == 0 { '♣' } else { ' ' }, rgb(lerp(c, (40, 60, 40), 0.5)), Some(rgb(c)));
        }
    }

    // The fields below: grain bending in waves of wind.
    // Hedgerows wander between the fields; each field has its own crop.
    let bend = (cv.roll(5) % 628) as f32 / 100.0;
    for y in g.max(0)..h {
        for x in 0..w {
            let yy = (y - g) as f32 - 1.0 - ((x as f32 * 0.05 + bend).sin() * 2.0 + (x as f32 * 0.13).sin() * 0.6);
            let field = (yy / 7.0).floor() as i32;
            if yy > 0.0 && (yy.round() as i32) % 7 == 0 && hash(x + salt, field) % 11 != 0 {
                let ch = if hash(x + salt, y) % 2 == 0 { '♣' } else { '♠' };
                cv.put(x, y, ch, rgb((40, 90, 40)), Some(rgb((50, 70, 34))));
                continue;
            }
            let kind = hash(field + salt, 17) % 5;
            let (ripe, dry) = match kind {
                0 => ((150, 180, 80), (110, 140, 60)),
                1 => ((170, 120, 80), (130, 90, 60)),
                _ => (ripe, dry),
            };
            let wave = (x as f32 * 0.12 - tf * 0.09 + y as f32 * 0.35).sin() + (x as f32 * 0.31 + tf * 0.05).sin() * 0.3;
            let ch = if wave > 0.75 { '/' } else if wave > 0.1 { '"' } else if wave > -0.6 { '\'' } else { ',' };
            let lit = (wave * 0.5 + 0.5) * 0.6;
            let base = lerp(dry, ripe, lit);
            cv.put(x, y, ch, rgb(lerp(base, (255, 245, 200), lit * 0.5)), Some(rgb(lerp(base, (90, 70, 30), 0.55))));
        }
    }

    // The reapers work along the top field, from the far side; behind
    // them stubble and stooks, ahead of them standing grain.
    let cycle = (w + 60) as u64;
    let reach = ((t / 6 + cv.roll(3) as u64) % cycle) as i32 - 10;
    let hands = 2 + (cv.roll(4) % 4) as i32;
    for x in 0..reach.min(w) {
        let y = g;
        let ch = if hash(x + salt, 3) % 5 == 0 { '♠' } else { ',' };
        let fg = if ch == '♠' { ripe } else { dry };
        cv.put(x, y, ch, rgb(fg), Some(rgb(lerp(dry, (90, 70, 30), 0.65))));
        if x % 6 == 2 && x < reach - 4 {
            cv.glyph(x, y - 1, '▲', rgb(ripe));
        }
    }
    for k in 0..hands {
        let x = reach - k * 4;
        if x < 0 || x >= w {
            continue;
        }
        let swing = ((t / 4) as i32 + k * 2) % 6;
        dwarf(cv, x, g - 1, COATS[((k + salt) % 8) as usize]);
        let (dx, ch) = match swing {
            0 | 1 => (1, '─'),
            2 | 3 => (1, '\\'),
            _ => (0, '│'),
        };
        cv.glyph(x + dx, g - 1 + (ch == '\\') as i32, ch, rgb((200, 200, 210)));
        // Chaff kicked up at each stroke.
        if swing < 2 {
            cv.glyph(x + 2, g - 2, '·', rgb(ripe));
        }
    }

    // The wain, and an ox dozing in the shafts.
    let wx = (w as f32 * 0.78) as i32;
    if g >= 4 {
        cv.sprite(wx, g - 3, &["▄▓▓▓▄", "█████", "○   ○"], rgb((150, 100, 50)));
        for i in 0..5 {
            cv.glyph(wx + i, g - 3, if i % 2 == 0 { '▄' } else { '▀' }, rgb(ripe));
        }
        cv.glyph(wx - 2, g - 1, 'Ö', rgb((150, 110, 70)));
        cv.glyph(wx - 1, g - 1, '─', rgb((110, 80, 50)));
        if (t / 40) % 3 == 0 {
            cv.glyph(wx - 2, g - 2, 'z', rgb((230, 230, 240)));
        }
    }

    // Crows wheeling over the stubble, dropping in to glean.
    for k in 0..(w / 25).max(3) {
        let a = tf * 0.04 + k as f32 * 2.1;
        let cx = (reach - 8).clamp(0, w.max(1)) as f32;
        let x = (cx + a.cos() * (6.0 + k as f32 * 1.5)) as i32;
        let y = (g as f32 * 0.5 + a.sin() * (g as f32 * 0.25).max(1.0)) as i32;
        cv.glyph(x, y, if (t / 4 + k as u64) % 2 == 0 { 'v' } else { '^' }, rgb((25, 20, 30)));
    }

    // Chaff and thistledown on the wind.
    for k in 0..(w / 6).max(4) {
        let x = drift(t, k + salt, 2, w + 4) - 2;
        let y = g + (hash(k, 31) % (h - g).max(1) as u32) as i32 + ((tf * 0.07 + k as f32).sin() * 1.5) as i32;
        cv.glyph(x, y, '·', rgb((255, 240, 200)));
    }
}
