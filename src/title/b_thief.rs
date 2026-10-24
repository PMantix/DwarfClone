//! The vault: heaps of coin, chests, gems, one lamp, and the guard asleep
//! in his chair by the door. Through a crack in the back wall comes a
//! kobold, low and quick, to the nearest glittering thing; it grabs, it
//! turns, and the guard wakes. Sometimes it gets away down its hole. Below,
//! the mint pounding out more coin to be stolen. The seed picks the
//! treasure, the thief's luck and where the crack is.

use super::b_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "skulkers steal the shiniest thing lying about",
    "lock it up. then lock that up",
    "the guard was only resting his eyes",
    "a fortress is a very large purse",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t) = (cv.w(), cv.h(), cv.t);
    let g = (cv.sky - 1).max(3).min(h - 1);
    let th = ((g as f32 * 0.45) as i32).clamp(2, 7);
    let ty = (g - th).max(0);
    let salt = cv.salt();
    let stone = [(76, 70, 64), (70, 70, 80), (82, 66, 58)][(cv.roll(1) % 3) as usize];
    let gold = (250, 205, 80);
    let air = lerp(stone, (0, 0, 0), 0.75);
    bedrock(cv, 0, h, stone, salt);

    // The vault, from its door to the cracked back wall.
    let dx0 = 4 + (cv.roll(2) % 4) as i32;
    let back = (w - 3 - (cv.roll(3) % 6) as i32).max(dx0 + 12);
    hollow(cv, dx0, ty, back, g, air);
    floor(cv, dx0, back + 1, g, lerp(stone, (170, 160, 150), 0.3));
    for x in dx0..back {
        cv.glow(x, g - 1, gold, 0.1);
    }
    // The door, and the guard's chair beyond it.
    for y in ty..g {
        cv.put(dx0 - 1, y, '▓', rgb((130, 130, 140)), Some(rgb((60, 60, 70))));
    }
    cv.glyph(dx0 - 1, g - 2, '•', rgb(gold));
    hollow(cv, 0, ty, dx0 - 1, g, lerp(air, (40, 24, 10), 0.3));
    floor(cv, 0, dx0 - 1, g, lerp(stone, (150, 140, 130), 0.3));
    torch(cv, dx0 - 3, ty, 1, 3.0);
    // The crack in the back wall, and the kobold's hole beyond it.
    for y in ty + th / 2..g {
        cv.put(back, y, ' ', Color::Black, Some(rgb((6, 4, 4))));
    }
    for y in g..(g + 4).min(h) {
        cv.put(back + (y - g) / 2, y, ' ', Color::Black, Some(rgb((6, 4, 4))));
    }

    // The hoard: heaps of coin, chests, gems, a lamp over it all.
    let mut heaps: Vec<i32> = Vec::new();
    let mut x = dx0 + 3;
    while x < back - 4 {
        let hw = 2 + (hash(x, salt) % 3) as i32;
        let hh = (hw - 1).min(th - 1);
        for dy in 0..hh {
            for xx in x - hw + dy..=x + hw - dy {
                let hs = hash(xx + salt, dy + 9);
                let glint = (t / 4 + hs as u64) % 43 == 0;
                let ch = if glint { '*' } else { ['$', '•', '$', '°'][(hs % 4) as usize] };
                cv.put(xx, g - 1 - dy, ch, rgb(if glint { (255, 255, 230) } else { lerp(gold, (120, 80, 20), (hs % 3) as f32 * 0.2) }), Some(rgb(lerp(gold, (60, 40, 10), 0.7))));
            }
        }
        heaps.push(x);
        x += hw * 2 + 3;
        if hash(x, salt + 1) % 2 == 0 && x < back - 4 {
            cv.sprite(x - 1, g - 1, &["■■"], rgb((150, 100, 50)));
            cv.glyph(x - 1, g - 2, '♦', rgb([(120, 220, 255), (255, 120, 140), (140, 255, 170)][(hash(x, 5) % 3) as usize]));
            x += 4;
        }
    }
    let lamp = (dx0 + back) / 2;
    torch(cv, lamp, ty, 2, 4.0);

    // The beat: quiet; the thief creeps in; grabs; the guard wakes; away or caught.
    let at = beat(cv, 1200, 4) as i32;
    let target = *heaps.last().unwrap_or(&(back - 4));
    let (enter, grab, wake) = (260, 260 + (back - target) * 5, 260 + (back - target) * 5 + 40);
    let lucky = cv.roll(5) % 2 == 0;
    let gx = dx0 - 3; // the guard's chair
    // Laden, an unlucky thief is slow; the guard runs it down.
    let caught_at = wake + (target + 2 - gx).max(1) * 8 / 3;
    let prize = (230, 230, 240);
    let kob = (150, 120, 90);
    let thief = if at < enter {
        None
    } else if at < grab {
        Some((back - (at - enter) / 5, false))
    } else if at < wake {
        Some((target + 1, true))
    } else if lucky {
        let x = target + 1 + (at - wake) / 2;
        if x <= back { Some((x, true)) } else { None }
    } else if at < caught_at {
        Some(((target + 1 + (at - wake) / 8).min(back - 1), true))
    } else {
        None
    };
    if let Some((kx, has)) = thief {
        let sneaking = !has;
        let ky = g - 1;
        cv.glyph(kx, ky, 'k', rgb(if sneaking { lerp(kob, air, 0.3) } else { kob }));
        if has {
            cv.glyph(kx, ky - 1, '°', rgb(prize));
        } else if (t / 12) % 4 == 0 {
            cv.glyph(kx - 1, ky, '·', rgb((255, 230, 120)));
        }
    }
    // The guard: dozing, then up and running.
    let b = beard(cv.roll(6));
    if at < wake {
        cv.glyph(gx, g - 1, '☺', rgb(b));
        cv.glyph(gx + 1, g - 1, 'h', rgb((140, 100, 60)));
        if (t / 25) % 4 != 0 {
            let zz = ((t / 25) % 4) as i32;
            cv.glyph(gx + zz / 2, g - 2 - zz / 2, if zz < 2 { 'z' } else { 'Z' }, rgb((170, 170, 200)));
        }
    } else {
        let s = at - wake;
        let reach = if lucky { back - 2 } else { (target + 1 + s / 8).min(back - 1) - 1 };
        let x = (gx + s / 2).min(reach);
        cv.glyph(x, g - 1, '☺', rgb(b));
        if s < 80 || (!lucky && at >= caught_at && at < caught_at + 120) {
            cv.glyph(x, g - 2, '!', rgb((255, 240, 160)));
        }
        if !lucky && at >= caught_at && at < caught_at + 200 {
            cv.glyph(x + 1, g - 2, '°', rgb(prize));
        }
        if lucky && x >= back - 2 && (t / 10) % 2 == 0 {
            cv.glyph(x, g - 2, '#', rgb((255, 120, 100)));
        }
    }

    // Below: the mint, a press pounding out coin down a chute.
    let lb = below(cv);
    if h - lb >= 4 {
        let my = (lb + (h - lb) / 2).min(h - 2);
        hollow(cv, 2, my - 3, w - 2, my + 1, lerp(stone, (20, 12, 4), 0.7));
        floor(cv, 2, w - 2, my + 1, lerp(stone, (150, 140, 120), 0.3));
        let n = ((w - 4) / 20).max(1);
        for k in 0..n {
            let px = 6 + k * 20;
            let down = ((t / 3 + k as u64 * 7) % 12) < 3;
            cv.glyph(px, my - 3, '║', rgb((150, 150, 160)));
            cv.glyph(px, my - 2, if down { '║' } else { '▀' }, rgb((180, 180, 190)));
            if down {
                cv.glyph(px, my - 1, '▀', rgb((180, 180, 190)));
                cv.glyph(px + 1, my, '*', rgb((255, 255, 220)));
            }
            cv.glyph(px, my, '▄', rgb((120, 120, 130)));
            for c in 0..6 {
                let cx = px + 2 + ((t / 2) as i32 + c * 3) % 16;
                cv.glyph(cx, my, '$', rgb(gold));
            }
            cv.glyph(px - 1, my, '☺', rgb(beard(cv.roll(7) + k as u32)));
        }
        torch(cv, w / 2, my - 3, 6, 3.0);
    }
}
