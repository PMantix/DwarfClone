//! A strange mood. A dwarf goes quiet, then stands, eyes far away, and
//! claims a workshop. Things are fetched, odd things, a gem, a bone, a
//! bolt of cloth, a bar of metal, and the door is shut, and the workshop
//! begins to shine. Days later out comes an artifact, blazing, and a name
//! the world will remember. The seed picks the dwarf, the materials, the
//! light and the name.

use super::b_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "a dwarf in a strange mood will not be denied",
    "every artifact has a name",
    "bring the bone. no, the other bone",
    "the workshop is glowing again",
];

const FIRST: [&str; 8] = ["Ur", "Dak", "Mor", "Zol", "Thrun", "Ek", "Bal", "Ost"];
const SECOND: [&str; 8] = ["ith", "abar", "olun", "egor", "um", "ilak", "aren", "ost"];
const OF: [&str; 8] = ["the Ember", "the Long Night", "Lanterns", "the Deep Bell", "Iron Rain", "the Quiet Gate", "Seven Stones", "the First Forge"];
const THING: [&str; 6] = ["crown", "amulet", "goblet", "axe", "scepter", "ring"];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t) = (cv.w(), cv.h(), cv.t);
    let g = (cv.sky - 1).max(4).min(h - 1);
    let tf = t as f32;
    let salt = cv.salt();
    let stone = [(76, 70, 66), (66, 70, 80), (80, 72, 60)][(cv.roll(1) % 3) as usize];
    let light = [(255, 230, 140), (170, 220, 255), (230, 170, 255), (160, 255, 190)][(cv.roll(2) % 4) as usize];
    let air = lerp(stone, (0, 0, 0), 0.78);
    bedrock(cv, 0, h, stone, salt);
    hollow(cv, 0, 1, w, g, air);
    floor(cv, 0, w, g, lerp(stone, (160, 150, 140), 0.3));

    // The beat: seized; fetching; working; done; taken away to be admired.
    let at = beat(cv, 1600, 3);
    let (claim, work, done, away) = (220u64, 640u64, 1150u64, 1500u64);
    let glow = if at < claim {
        0.0
    } else if at < work {
        0.25 * span(at, claim, work)
    } else if at < done {
        0.25 + 0.6 * span(at, work, done)
    } else if at < away {
        1.0 - 0.5 * span(at, done, away)
    } else {
        0.0
    };

    // The workshop, where the light comes from.
    let wx = (w as f32 * (0.22 + (cv.roll(3) % 12) as f32 / 100.0)) as i32;
    let wy = g - 2;
    if glow > 0.0 {
        // Rays turning slowly round it.
        let reach = (w as f32 * 0.35).max(8.0);
        for r in 0..10 {
            let a = r as f32 * 0.628 + tf * 0.01;
            let pulse = ((tf * 0.08 + r as f32 * 1.7).sin() * 0.5 + 0.5) * 0.6 + 0.4;
            let end = (wx + 2 + (a.cos() * reach) as i32, wy + (a.sin() * reach * 0.4) as i32);
            for (i, (x, y)) in line((wx + 2, wy), end).into_iter().enumerate() {
                if y >= 1 && y < g {
                    let fall = 1.0 - i as f32 / reach;
                    cv.glow(x, y, light, glow * pulse * fall * 0.35);
                }
            }
        }
        cv.halo(wx + 2, wy, 3.0 + glow * 6.0, light, glow * 0.6);
    }
    let frame = rgb(lerp(stone, (200, 190, 180), 0.4));
    cv.sprite(wx, g - 3, &["╔═══╗", "║   ║", "╩═══╩"], frame);
    let inside = if at >= work && at < done { if (t / 3) % 2 == 0 { '*' } else { '+' } } else { '▒' };
    cv.glyph(wx + 2, wy, inside, rgb(if glow > 0.2 { (255, 255, 255) } else { lerp(stone, light, 0.3) }));

    // The materials, fetched one by one and set down by the door.
    let mats = [('♦', (120, 220, 255)), ('%', (230, 220, 190)), ('≈', (200, 80, 90)), ('▬', (210, 210, 220)), ('•', (240, 200, 80))];
    let want = 3 + (cv.roll(4) % 2) as usize;
    let order = cv.roll(5) as usize;
    let b = beard(cv.roll(6));
    let mut mx = wx + 6;
    for i in 0..want {
        let (ch, c) = mats[(order + i * 2) % mats.len()];
        let fetched = claim + 80 + i as u64 * ((work - claim - 100) / want as u64);
        if at >= fetched && at < work {
            cv.glyph(mx, g - 1, ch, rgb(c));
        }
        mx += 2;
    }
    // The dwarf: moody, wandering, fetching, shut in, triumphant.
    let far = (w * 3 / 4).max(wx + 12);
    if at < claim {
        let x = pingpong(t, 6, far - 10, far);
        cv.glyph(x, g - 1, '☺', rgb(b));
        if (t / 20) % 3 == 0 {
            cv.glyph(x, g - 2, '?', rgb(light));
        }
    } else if at < work {
        // Fetching: out to the far side and back with something.
        let x = pingpong(t, 2, wx + 6, far);
        cv.glyph(x, g - 1, '☺', rgb(b));
        cv.glyph(x, g - 2, '!', rgb(light));
    } else if at < done {
        // Shut in: hammering heard, sparks out of the door.
        for k in 0..4 {
            let age = ((t / 2) as i32 + k * 3) % 9;
            let a = hash(k, (t / 18) as i32) as i32 % 5 - 2;
            cv.glyph(wx + 2 + a * age / 3, wy - 1 - age / 3, '·', rgb(light));
        }
    } else if at < away {
        // Out, with the thing itself held high.
        let x = wx + 6;
        cv.glyph(x, g - 1, '☺', rgb(b));
        let hop = ((t / 6) % 2) as i32;
        cv.halo(x, g - 3 - hop, 3.0, light, 0.6);
        cv.glyph(x, g - 3 - hop, '☼', rgb((255, 255, 255)));
        if (t / 10) % 2 == 0 {
            cv.glyph(x + 1, g - 2, '!', rgb(light));
        }
    }
    // Onlookers keeping their distance.
    for k in 0..3 {
        let x = far + 3 + k * 3;
        let gawp = at >= done && at < away;
        cv.glyph(x - if gawp { 4 } else { 0 }, g - 1, '☺', rgb(beard(cv.roll(7) + k as u32)));
        if gawp && (t / 15 + k as u64) % 3 == 0 {
            cv.glyph(x - 4, g - 2, '!', rgb((255, 240, 180)));
        }
    }
    torch(cv, w - 3, g - 3, 2, 3.0);
    torch(cv, 2, g - 3, 3, 3.0);

    // Below: the announcement, and older artifacts on their plinths.
    let lb = below(cv);
    if h - lb >= 4 {
        let ay = lb + 1;
        if at >= done && at < away {
            let name = format!(
                "{}{} has made {}{}, a {} of {}",
                FIRST[(cv.roll(8) % 8) as usize],
                SECOND[(cv.roll(9) % 8) as usize],
                FIRST[(cv.roll(10) % 8) as usize],
                SECOND[(cv.roll(11) % 8) as usize],
                THING[(cv.roll(12) % 6) as usize],
                OF[(cv.roll(13) % 8) as usize],
            );
            let x0 = (w - name.chars().count() as i32) / 2;
            for (i, ch) in name.chars().enumerate() {
                let sweep = ((i as i32 - (at - done) as i32 / 2).abs() < 3) as i32 as f32;
                cv.glyph(x0 + i as i32, ay, ch, rgb(lerp(light, (255, 255, 255), sweep)));
            }
        }
        let py = (ay + 2 + (h - ay - 2) / 2).min(h - 2);
        if py > ay + 1 {
            hollow(cv, 2, py - 2, w - 2, py + 1, air);
            floor(cv, 2, w - 2, py + 1, lerp(stone, (150, 140, 130), 0.3));
            let n = ((w - 4) / 12).max(1);
            for k in 0..n {
                let x = 6 + k * 12;
                let (ch, c) = [('♦', (120, 220, 255)), ('∩', (240, 200, 80)), ('Ω', (220, 220, 230)), ('♀', (230, 160, 200))][((k as u32 + cv.roll(14)) % 4) as usize];
                cv.glyph(x, py, '▄', rgb(lerp(stone, (200, 190, 180), 0.4)));
                cv.glyph(x, py - 1, ch, rgb(c));
                if (t / 5 + k as u64 * 11) % 37 == 0 {
                    cv.glyph(x, py - 2, '·', rgb((255, 255, 255)));
                }
            }
        }
    }
}
