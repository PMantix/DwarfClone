//! The trap hall: a long corridor up to the fortress gate with a weapon
//! trap set every few strides. The goblins come in a column, and each
//! trap takes the first of them to step on it in a burst of spikes and
//! gore, while crossbows rattle from the arrow slits overhead. The last of
//! them thinks better of it. Then a dwarf comes out with a barrow to tidy
//! up. The seed picks the stone, the traps and how many come.

use super::b_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the hallway is the defence",
    "watch your step. no, really",
    "every trap was built by someone who cared",
    "the goblins were told there would be loot",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t) = (cv.w(), cv.h(), cv.t);
    let g = (cv.sky - 1).max(3).min(h - 1);
    let th = ((g as f32 * 0.35) as i32).clamp(2, 5);
    let ty = (g - th).max(0);
    let salt = cv.salt();
    let stone = [(78, 72, 70), (70, 64, 80), (84, 74, 60)][(cv.roll(1) % 3) as usize];
    let air = lerp(stone, (0, 0, 0), 0.64);
    bedrock(cv, 0, h, stone, salt);

    // The corridor, smoothed, from the gate to the dark at the far end.
    let gx = 3 + (cv.roll(2) % 3) as i32;
    hollow(cv, gx, ty, w, g, air);
    floor(cv, gx, w, g, lerp(stone, (170, 165, 160), 0.35));
    for y in ty..g {
        cv.put(gx - 1, y, '█', rgb((120, 90, 50)), Some(rgb((60, 44, 24))));
    }
    cv.glyph(gx - 1, ty - 1, '▄', rgb((90, 70, 40)));
    for (k, x) in (0..).map(|k| gx + 5 + k * 15).take_while(|&x| x < w - 2).enumerate() {
        torch(cv, x, ty, k as i32, 3.0);
    }
    // The far end is dark; the goblins come out of it.
    for x in (w - w / 8).max(gx)..w {
        let f = (x - (w - w / 8)) as f32 / (w / 8).max(1) as f32;
        for y in ty..g {
            let bg = cv.bg_at(x, y);
            cv.fill(x, y, mix(bg, (0, 0, 0), f * 0.8));
        }
    }

    // The traps, right to left the way the goblins meet them.
    let gap = 7 + (cv.roll(3) % 5) as i32;
    let traps: Vec<i32> = (0..).map(|k| w - w / 6 - k * gap).take_while(|&x| x > gx + 6).take(6).collect();
    let n = traps.len().max(1) as i32;
    let ngob = n + 1 + (cv.roll(4) % 2) as i32;

    // The beat: the column comes, the traps fire, the tidying.
    let at = beat(cv, 1400, 5) as i32;
    let (come, speed, spacing) = (80, 3, 14);
    let gob = (100, 190, 80);
    let blood = (150, 10, 16);
    // Where goblin k is, and when (if ever) its trap gets it.
    let trap_of = |k: i32| -> Option<i32> { traps.get(k as usize).copied() };
    let start_x = w + 1;
    let pos = |k: i32| -> i32 { start_x - (at - come - k * spacing) / speed };
    let death = |k: i32| -> Option<i32> { trap_of(k).map(|tx| come + k * spacing + (start_x - tx) * speed) };
    let tidy = come + ngob * spacing + (start_x - gx) * speed / 2 + 150;

    for (j, &tx) in traps.iter().enumerate() {
        let k = j as i32;
        let fired = death(k).map_or(false, |d| at >= d);
        let since = death(k).map_or(0, |d| at - d);
        // Gore, splashed about the trap once it fires.
        if fired && at < tidy + 200 {
            for dy in 0..th {
                for dx in -3..=3 {
                    let hs = hash(tx + dx + salt, dy + k * 7);
                    let near = 1.0 - (dx.abs() as f32 / 4.0) - dy as f32 / (th as f32 * 2.0);
                    if near > 0.0 && hs % 3 != 0 {
                        cv.glow(tx + dx, g - 1 - dy, blood, near * 0.7);
                    }
                    if hs % 11 == 0 && dy == 0 {
                        cv.glyph(tx + dx, g - 1, if hs % 2 == 0 { ',' } else { '~' }, rgb((200, 30, 30)));
                    }
                }
            }
            cv.tint(tx, g, rgb(blood));
        }
        // The spikes, shooting up and sinking back.
        if fired && since < 30 {
            let up = if since < 6 { since / 2 + 1 } else { (30 - since) / 8 + 1 };
            for dy in 0..up.min(th) {
                for dx in -1..=1 {
                    cv.glyph(tx + dx, g - 1 - dy, if dy + 1 == up.min(th) { '▲' } else { '│' }, rgb((220, 220, 230)));
                }
            }
            if since < 12 {
                for s in 0..6 {
                    let a = s as f32 * 1.05 + k as f32;
                    let r = since as f32 * 0.4;
                    cv.glyph(tx + (a.cos() * r * 2.0) as i32, g - 2 - (a.sin().abs() * r) as i32, '•', rgb((220, 20, 30)));
                }
            }
        } else {
            cv.glyph(tx, g - 1, '^', rgb(lerp(stone, (200, 200, 200), 0.4)));
        }
        // What's left of the goblin.
        if fired && since >= 6 && at < tidy + (j as i32) * 20 {
            cv.glyph(tx + 1, g - 1, '%', rgb((150, 160, 90)));
        }
    }

    // The column of goblins, and the one that runs.
    for k in 0..ngob {
        if at < come + k * spacing {
            continue;
        }
        let x = pos(k);
        if let Some(d) = death(k) {
            if at >= d {
                continue;
            }
        } else {
            // No trap left for this one: it stops, sees, and flees.
            let stop = traps.last().map_or(gx + 10, |&l| l + 4 + (k - n) * 2);
            let reach_at = come + k * spacing + (start_x - stop) * speed;
            if at >= reach_at {
                let back = stop + (at - reach_at - 40).max(0) / 2;
                if back < w {
                    cv.glyph(back, g - 1, 'g', rgb(gob));
                    if at - reach_at < 60 {
                        cv.glyph(back, g - 2, '!', rgb((255, 240, 160)));
                    }
                }
                continue;
            }
        }
        if x < w {
            cv.glyph(x, g - 1, 'g', rgb(gob));
            if k == 0 {
                cv.glyph(x, g - 2, '¶', rgb((170, 40, 40)));
            } else if (t / 5 + k as u64) % 2 == 0 {
                cv.glyph(x - 1, g - 1, '/', rgb((160, 160, 170)));
            }
        }
    }

    // Arrow slits overhead, crossbows behind them.
    if ty >= 3 {
        let gy = ty - 2;
        hollow(cv, gx, gy, w * 3 / 4, gy + 1, air);
        let battle = at > come && at < tidy - 100;
        let mut x = gx + 4;
        let mut k = 0;
        while x < w * 3 / 4 - 1 {
            cv.put(x, ty - 1, '╬', rgb(lerp(stone, (200, 200, 200), 0.3)), Some(rgb(air)));
            cv.glyph(x, gy, '☺', rgb(beard(cv.roll(6) + k)));
            if battle && (t / 4 + k as u64 * 5) % 13 == 0 {
                cv.glyph(x, ty, '↓', rgb((210, 200, 180)));
            }
            x += 8;
            k += 1;
        }
    }

    // The tidying: a dwarf with a barrow, collecting.
    if at >= tidy && at < tidy + 400 {
        let x = gx + (at - tidy) / 3;
        if x < w {
            cv.glyph(x, g - 1, '☺', rgb(beard(cv.roll(7))));
            cv.sprite(x + 1, g - 1, &["▐▌"], rgb((120, 86, 50)));
            if x > traps.last().copied().unwrap_or(w) {
                cv.glyph(x + 1, g - 2, '%', rgb((150, 160, 90)));
            }
        }
    } else if at < come {
        // Before: the gate guard yawns.
        cv.glyph(gx, g - 1, '☺', rgb(beard(cv.roll(7))));
        if (t / 50) % 3 == 0 {
            cv.glyph(gx + 1, g - 2, 'z', rgb((180, 180, 200)));
        }
    } else {
        cv.glyph(gx, g - 1, '☺', rgb(beard(cv.roll(7))));
    }

    // Below, the mechanisms: chains and levers running the traps.
    let my = h - 4;
    if my - g > 6 {
        hollow(cv, 2, my, w - 2, h - 1, air);
        floor(cv, 2, w - 2, h - 1, lerp(stone, (150, 140, 130), 0.3));
        for &tx in &traps {
            for y in g + 1..my {
                cv.glyph(tx, y, if (y + (t / 6) as i32) % 2 == 0 { '§' } else { '│' }, rgb((110, 110, 120)));
            }
            let spin = ['|', '/', '─', '\\'][((t / 4 + tx as u64) % 4) as usize];
            cv.glyph(tx, my, spin, rgb((180, 170, 140)));
            cv.glyph(tx - 1, my, '[', rgb((120, 110, 100)));
            cv.glyph(tx + 1, my, ']', rgb((120, 110, 100)));
        }
        let lx = (w / 6).max(3);
        cv.glyph(lx, h - 2, if (t / 60) % 2 == 0 { '╘' } else { '╛' }, rgb((180, 170, 140)));
        cv.glyph(lx + 1, h - 2, '☺', rgb(beard(cv.roll(8))));
        torch(cv, w / 2, my, 9, 3.0);
    }
}
