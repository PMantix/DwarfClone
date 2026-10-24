//! The bridge over the chasm: the only way in, a long span across a crack
//! in the world with no bottom anyone has seen. The raiders come over it
//! in a line, a dwarf at the gate waits until the leader is nearly across,
//! pulls the lever, and the bridge swings up on its chains. The ones on
//! it go down, a long way down. The ones still on the far side shake
//! their fists and go home. The seed picks the stone, the chasm and how
//! many raiders try it.

use super::b_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "pull the lever. no, the other lever",
    "the chasm has no bottom. we checked",
    "the bridge is up. the guests have left",
    "a door is a wall that has made friends",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t) = (cv.w(), cv.h(), cv.t);
    let g = (cv.sky - 1).max(3).min(h - 1);
    let th = ((g as f32 * 0.4) as i32).clamp(2, 6);
    let ty = (g - th).max(0);
    let salt = cv.salt();
    let stone = [(70, 64, 72), (82, 70, 60), (60, 66, 70)][(cv.roll(1) % 3) as usize];
    let air = lerp(stone, (0, 0, 0), 0.64);
    let deepc = [(140, 30, 10), (20, 30, 60), (0, 0, 0)][(cv.roll(2) % 3) as usize];
    bedrock(cv, 0, h, stone, salt);

    // The chasm, splitting the rock from top to bottom.
    let pw = (w / 7).clamp(5, 18);
    let px0 = (w * 7 / 10 + (cv.roll(3) % 5) as i32).min(w - pw - 3).max(4);
    let px1 = px0 + pw;
    for y in 0..h {
        let jag = |k: i32| (hash(y + k, salt) % 2) as i32;
        let depth = (y - g).max(0) as f32 / (h - g).max(1) as f32;
        for x in px0 + jag(1)..px1 - jag(2) {
            let bg = if y > g { lerp(air, deepc, depth * depth * 0.6) } else { lerp(air, (0, 0, 0), 0.3) };
            cv.fill(x, y, rgb(bg));
        }
        cv.glyph(px0 + jag(1) - 1, y, '▐', rgb(lerp(stone, (0, 0, 0), 0.3)));
        cv.glyph(px1 - jag(2), y, '▌', rgb(lerp(stone, (0, 0, 0), 0.3)));
    }
    // The hall on our side, the ledge on theirs.
    hollow(cv, 0, ty, px0, g, air);
    hollow(cv, px1, ty, w, g, lerp(air, (0, 0, 0), 0.3));
    floor(cv, 0, px0, g, lerp(stone, (150, 140, 130), 0.3));
    floor(cv, px1, w, g, lerp(stone, (110, 100, 90), 0.3));
    for (k, x) in (0..).map(|k| 4 + k * 13).take_while(|&x| x < px0 - 2).enumerate() {
        torch(cv, x, ty, k as i32, 3.0);
    }
    // Things glinting far down in the dark, if it has a bottom at all.
    if h - g > 8 {
        for k in 0..4 {
            let x = px0 + 1 + (hash(k, salt) % (pw - 2).max(1) as u32) as i32;
            let y = h - 1 - (hash(k, 3) % 3) as i32;
            if (t / 11 + k as u64) % 5 != 0 {
                cv.glyph(x, y, '·', rgb(lerp(deepc, (255, 200, 120), 0.5)));
            }
        }
    }

    // The beat: raiders come; the lever; the fall; the bridge comes down.
    let at = beat(cv, 1200, 4) as i32;
    let n = 4 + (cv.roll(5) % 4) as i32;
    let (come, speed, spacing) = (100, 4, 3);
    let start = w + 1;
    // The lever is pulled as the leader nears our end.
    let pull = come + (start - (px0 + 2)) * speed;
    let lower = pull + 520;
    let raise = if at < pull { 0.0 } else if at < lower { span(at as u64, pull as u64, pull as u64 + 24) } else { 1.0 - span(at as u64, lower as u64, lower as u64 + 40) };
    // The bridge, swinging up about its hinge on our side.
    let len = pw as f32;
    let ang = raise * std::f32::consts::FRAC_PI_2;
    let end = (px0 as f32 + len * ang.cos(), g as f32 - len * 0.5 * ang.sin());
    let wood = (150, 110, 60);
    if raise == 0.0 {
        for x in px0..px1 {
            cv.put(x, g, '═', rgb(wood), Some(rgb(lerp(wood, (0, 0, 0), 0.6))));
        }
    } else {
        for (x, y) in line((px0, g), (end.0.round() as i32, end.1.round() as i32)) {
            let ch = if raise > 0.8 { '║' } else if raise > 0.25 { '╱' } else { '═' };
            cv.put(x, y, ch, rgb(wood), Some(rgb(lerp(wood, (0, 0, 0), 0.6))));
        }
        // Its chains, from the wall above the gate.
        for (x, y) in line((px0 - 1, (ty - 1).max(0)), (end.0.round() as i32, end.1.round() as i32)) {
            if raise < 0.95 {
                cv.glyph(x, y, '·', rgb((150, 150, 160)));
            }
        }
    }
    // The gate and its lever, and the dwarf who pulls it.
    cv.glyph(px0 - 1, g - 1, '▓', rgb((120, 90, 50)));
    let lx = (px0 - 4).max(1);
    let pulled = at >= pull && at < lower;
    cv.glyph(lx, g - 1, if pulled { '╛' } else { '╘' }, rgb((190, 180, 150)));
    cv.glyph(lx - 1, g - 1, '☺', rgb(beard(cv.roll(6))));
    if at >= pull - 40 && at < pull + 30 {
        cv.glyph(lx - 1, g - 2, '!', rgb((255, 240, 160)));
    }
    // Crossbows behind.
    for k in 0..2 {
        let x = (lx - 4 - k * 3).max(0);
        cv.glyph(x, g - 1, '☺', rgb(beard(cv.roll(7) + k as u32)));
        cv.glyph(x + 1, g - 1, '}', rgb((150, 120, 80)));
    }

    // The raiders.
    let gob = (110, 190, 80);
    for k in 0..n {
        let born = come + k * spacing * speed;
        if at < born {
            continue;
        }
        let walk = start - (at - born) / speed;
        let on_at_pull = {
            let x = start - (pull - born).max(0) / speed;
            if pull >= born && x >= px0 && x < px1 { Some(x) } else { None }
        };
        if at >= pull {
            if let Some(x0) = on_at_pull {
                // Falling, a long way.
                let s = (at - pull) as f32;
                let y = g - 1 + (s * s * 0.012) as i32;
                let x = x0 - (s * 0.05) as i32;
                if y < h + 2 && s < 300.0 {
                    cv.glyph(x, y, 'g', rgb(gob));
                    if y < g + 6 {
                        cv.glyph(x + 1, y - 1, '!', rgb((255, 240, 160)));
                    }
                }
                continue;
            }
            if pull < born || start - (pull - born) / speed >= px1 {
                // Still on their side: halt at the edge, rage, go home.
                let queue = px1 + 1 + (k - n / 2).max(0);
                let x = walk.max(queue);
                let since = at - pull;
                let home = if since > 200 { x + (since - 200) / 2 } else { x };
                if home < w {
                    cv.glyph(home, g - 1, 'g', rgb(gob));
                    if since > 20 && since < 200 && (t / 8 + k as u64) % 3 == 0 {
                        cv.glyph(home, g - 2, '!', rgb((255, 120, 100)));
                    }
                }
                continue;
            }
        }
        if walk < w && walk >= px0 {
            let y = if walk < px1 { g - 1 } else { g - 1 };
            cv.glyph(walk, y, 'g', rgb(gob));
            if k == 0 {
                cv.glyph(walk, y - 1, '¶', rgb((170, 40, 40)));
            }
        }
    }

    // Below, the dark either side of the chasm, a few old bones in the rock.
    let b = below(cv);
    for k in 0..(w / 30).max(1) {
        let x = (hash(k + salt, 13) % w.max(1) as u32) as i32;
        let y = b + (hash(k, 17) % (h - b).max(1) as u32) as i32;
        if x + 4 < px0 || x > px1 + 1 {
            cv.sprite(x, y, &["~°═°"], rgb(lerp(stone, (220, 210, 190), 0.5)));
        }
    }
}
