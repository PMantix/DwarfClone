//! A tantrum spiral in the dining hall. Supper, ale, someone's sock in
//! the soup, and one dwarf snaps: curses, a chair through the air, and the
//! one it hits snaps too, and the next. Tables go over, mugs fly, until
//! the militia comes stamping in and it all goes quiet and sulky. Below,
//! the cells where the worst of them cool off. The seed picks who starts
//! it and how far it spreads.

use super::b_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "it started with a sock in the soup",
    "one bad mood is a tantrum. three is a spiral",
    "the militia commander has had a long day",
    "ale first. then everything else",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t) = (cv.w(), cv.h(), cv.t);
    let g = (cv.sky - 1).max(3).min(h - 1);
    let salt = cv.salt();
    let stone = [(90, 76, 60), (76, 72, 80), (84, 70, 70)][(cv.roll(1) % 3) as usize];
    let air = lerp(stone, (40, 24, 10), 0.6);
    bedrock(cv, 0, h, stone, salt);
    hollow(cv, 0, 1, w, g, air);
    floor(cv, 0, w, g, lerp(stone, (170, 150, 120), 0.3));
    for (k, x) in (0..).map(|k| 6 + k * 16).take_while(|&x| x < w - 1).enumerate() {
        torch(cv, x, 2.max(g - 6), k as i32, 4.0);
    }

    // The tables, and who sits at them.
    let wood = (130, 96, 60);
    let n = (w / 22).max(1);
    let mut seats: Vec<(i32, i32)> = Vec::new(); // (x, table)
    let mut tables: Vec<(i32, i32)> = Vec::new();
    for k in 0..n {
        let tl = (w / n / 2).clamp(5, 12);
        let tx = 3 + k * (w - 6) / n.max(1) + (hash(k, salt) % 3) as i32;
        tables.push((tx, tl));
        let mut x = tx;
        while x < tx + tl {
            seats.push((x, k));
            x += 3;
        }
    }
    let ns = seats.len() as i32;

    // The beat: supper; it starts; it spreads; the militia; the quiet.
    let at = beat(cv, 1400, 3) as i32;
    let spread = 2 + (cv.roll(4) as i32 % 4).min(ns - 1).max(0); // how many lose it
    let first = (cv.roll(5) % ns.max(1) as u32) as i32;
    // Who loses it, in order, and when.
    let victim = |i: i32| -> i32 { (first + i * 2 + i * i) % ns.max(1) };
    let onset = |i: i32| -> i32 { 240 + i * 90 - i * i * 6 };
    let militia = onset(spread) + 160;
    let calm = militia + 120;
    let angry_from = |s: i32| -> Option<i32> { (0..spread).find(|&i| victim(i) == s).map(onset) };
    let swing = (t / 6) % 2 == 0;

    // Tables: on their legs, or over once the trouble reaches them.
    for (k, &(tx, tl)) in tables.iter().enumerate() {
        let over = (0..spread).any(|i| at >= onset(i) + 30 && seats[victim(i) as usize].1 == k as i32);
        if over && at < calm + 200 {
            for x in tx..tx + tl {
                cv.glyph(x, g - 1, '▄', rgb(lerp(wood, (0, 0, 0), 0.2)));
            }
            cv.glyph(tx + tl, g - 1, '╨', rgb(wood));
        } else {
            for x in tx..tx + tl {
                cv.glyph(x, g - 2, '═', rgb(wood));
            }
            cv.glyph(tx, g - 1, '║', rgb(wood));
            cv.glyph(tx + tl - 1, g - 1, '║', rgb(wood));
            for x in (tx + 1..tx + tl - 1).step_by(3) {
                cv.glyph(x, g - 3, if hash(x, 4) % 2 == 0 { '°' } else { '☼' }, rgb((200, 170, 100)));
            }
        }
    }

    // The diners.
    for (s, &(sx, _)) in seats.iter().enumerate() {
        let s = s as i32;
        let b = beard(hash(s, salt) % 6);
        match angry_from(s) {
            Some(on) if at >= on && at < calm => {
                // Raging: hopping about, cursing, red in the face.
                let jig = ((t / 5 + s as u64) % 5) as i32 - 2;
                let x = sx + jig;
                let y = g - 1 - ((t / 4 + s as u64) % 3 == 0) as i32;
                cv.halo(x, y, 1.5, (255, 40, 30), 0.3);
                cv.glyph(x, y, '☻', rgb((255, 80, 60)));
                let cuss = ['#', '$', '%', '!', '&', '#', '@'][((t / 6 + s as u64) % 7) as usize];
                cv.glyph(x, y - 1, cuss, rgb((255, 230, 120)));
                if swing {
                    cv.glyph(x + 1, y, '*', rgb((255, 240, 200)));
                }
            }
            Some(on) if at >= calm && at < calm + 300 => {
                // Sulking now, under the militia's eye.
                let _ = on;
                cv.glyph(sx, g - 1, '☺', rgb((120, 120, 150)));
                if (t / 30 + s as u64) % 5 == 0 {
                    cv.glyph(sx, g - 2, '.', rgb((150, 150, 170)));
                }
            }
            _ => {
                // Eating, or ducking.
                let trouble = at >= onset(0) && at < calm;
                let x = if trouble { sx + ((s % 2) * 2 - 1) } else { sx };
                cv.glyph(x, g - 1, '☺', rgb(b));
                if !trouble && (t / 25 + s as u64) % 6 == 0 {
                    cv.glyph(x, g - 2, '♪', rgb((240, 220, 160)));
                } else if trouble && (t / 15 + s as u64) % 4 == 0 {
                    cv.glyph(x, g - 2, '!', rgb((240, 220, 160)));
                }
            }
        }
    }
    // Things flying: a chair or a mug from each angry one to the next.
    for i in 1..spread {
        let on = onset(i);
        if at >= on - 30 && at < on {
            let (a, b) = (seats[victim(i - 1) as usize].0, seats[victim(i) as usize].0);
            let s = (at - (on - 30)) as f32 / 30.0;
            let x = a as f32 + (b - a) as f32 * s;
            let y = g as f32 - 2.0 - (s * (1.0 - s) * 4.0 * ((g - 2) as f32 * 0.5).min(5.0));
            cv.glyph(x as i32, y as i32, if i % 2 == 0 { 'π' } else { '°' }, rgb((220, 190, 140)));
        }
        if at >= on && at < on + 8 {
            let x = seats[victim(i) as usize].0;
            cv.glyph(x, g - 2, '*', rgb((255, 255, 255)));
        }
    }
    // The militia, stamping in from the door.
    if at >= militia - 80 && at < calm + 300 {
        let walk = ((at - (militia - 80)) / 2).min(w / 2);
        for k in 0..3 {
            let x = walk - k * 3;
            if x >= 0 {
                cv.glyph(x, g - 1, '☺', rgb((150, 180, 220)));
                cv.glyph(x + 1, g - 1, if swing { '/' } else { '│' }, rgb((200, 200, 210)));
            }
        }
    }

    // Below: the cells, and whoever is in them.
    let lb = below(cv);
    if h - lb >= 4 {
        let cy = (lb + (h - lb) / 2).min(h - 2);
        hollow(cv, 2, cy - 2, w - 2, cy + 1, lerp(stone, (0, 0, 0), 0.75));
        floor(cv, 2, w - 2, cy + 1, lerp(stone, (120, 110, 100), 0.3));
        let nc = ((w - 4) / 10).max(1);
        for k in 0..nc {
            let x = 3 + k * 10;
            for y in cy - 2..=cy {
                cv.glyph(x, y, '║', rgb((140, 140, 150)));
            }
            if hash(k, salt + 3) % 2 == 0 {
                let px = x + 2 + pingpong(t + k as u64 * 37, 11, 0, 5);
                cv.glyph(px, cy, '☺', rgb((180, 130, 120)));
                if (t / 40 + k as u64) % 5 == 0 {
                    cv.glyph(px, cy - 1, '%', rgb((230, 200, 120)));
                }
            }
        }
        torch(cv, w / 2, cy - 2, 7, 3.0);
    }
}
