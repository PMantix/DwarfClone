//! A waterfall at dawn: the sun coming up behind the ridge, its first
//! light creeping down a tall cliff where a river leaps off the edge in
//! two drops, mist rising off the plunge pool and drifting in banks
//! through the pines, and an early riser filling buckets at the water's
//! edge. The seed picks the dawn's colours, the cliff's place and the
//! height of the falls.

use super::a_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the mountain pours out its heart every morning",
    "fresh water, falling forever",
    "first light on the falls; the day begins",
    "listen: the river is older than the gods",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let g = (cv.sky - 1).max(1);
    let tf = t as f32;
    let salt = cv.salt();
    let (top, mid, low) = [((40, 40, 100), (200, 120, 150), (255, 190, 120)), ((30, 50, 110), (150, 140, 190), (255, 210, 160)), ((50, 36, 90), (230, 110, 100), (255, 200, 110))][(cv.roll(1) % 3) as usize];
    // The sun climbs over the long hold of a scene.
    let climb = ((t + cv.roll(2) as u64 % 1500) % 4500) as f32 / 4500.0;
    let sun_y = g as f32 * (0.85 - climb * 0.6);
    let sx = (w as f32 * 0.7) as i32;

    for y in 0..g.min(h) {
        let f = y as f32 / g.max(1) as f32;
        let c = if f < 0.5 { lerp(top, mid, f * 2.0) } else { lerp(mid, low, (f - 0.5) * 2.0) };
        for x in 0..w {
            cv.fill(x, y, rgb(c));
        }
    }
    cv.halo(sx, sun_y as i32, (g as f32 * 0.5).max(2.0), (255, 220, 160), 0.55);
    cv.disc(sx, sun_y as i32, (g as f32 * 0.08).clamp(0.7, 2.4), (255, 240, 200));
    // A few last stars, fading.
    if climb < 0.4 {
        for y in 0..(g / 3) {
            for x in 0..w {
                if hash(x * 7 + 3, y * 13 + 1) % 53 == 0 {
                    cv.glyph(x, y, '·', rgb(lerp((220, 220, 240), top, climb * 2.5)));
                }
            }
        }
    }

    // The ridge behind, dark against the dawn, its top catching the light.
    let back: Vec<i32> = (0..w).map(|x| g - 1 - (ridge(x, w, seed, 80) * (g as f32 * 0.4)) as i32).collect();
    for x in 0..w {
        let s = back[x as usize];
        for y in s.max(0)..g {
            let rim = y == s;
            cv.put(x, y, if rim { '▄' } else { ' ' }, rgb(lerp(low, (255, 255, 255), 0.3)), Some(rgb(lerp((60, 50, 80), (40, 36, 60), (y - s) as f32 / 4.0))));
        }
    }

    // The cliff the river falls from: a tall block off to one side.
    let cx = (w as f32 * (0.1 + (cv.roll(3) % 10) as f32 / 100.0)) as i32;
    let cw = (w / 6).max(5);
    let lip = (g as f32 * (0.15 + (cv.roll(4) % 20) as f32 / 100.0)) as i32;
    let ledge = lip + (g - lip) / 2; // the falls drop in two steps
    let lit_to = lip + ((g - lip) as f32 * climb * 1.4) as i32; // first light creeps down
    let cliff = |x: i32, y: i32| -> bool {
        let edge = cx - cw / 2 + ((y as f32 * 0.7).sin() * 1.2) as i32;
        let edge2 = cx + cw / 2 + ((y as f32 * 0.5 + 2.0).sin() * 1.2) as i32;
        y >= lip && y < g && x >= edge && x <= edge2
    };
    for y in lip.max(0)..g {
        for x in (cx - cw)..=(cx + cw) {
            if cliff(x, y) {
                let ch = if hash(x + salt, y * 3) % 5 == 0 { '▒' } else { ' ' };
                let c = lerp((70, 62, 70), (46, 42, 52), (hash(x, y) % 10) as f32 / 20.0);
                let c = if y < lit_to { lerp(c, (230, 150, 100), 0.35) } else { c };
                cv.put(x, y, ch, rgb(lerp(c, (0, 0, 0), 0.3)), Some(rgb(c)));
            }
        }
        if y == lip {
            for x in (cx - cw / 2)..=(cx + cw / 2) {
                if hash(x + salt, 5) % 2 == 0 {
                    cv.glyph(x, y - 1, '♠', rgb((40, 70, 50)));
                }
            }
        }
    }

    // The water: a broad upper drop, a ledge, a narrower lower drop.
    let fw = (cw / 3).max(1);
    let pool = g + 1;
    for (y0, y1, x0, wd) in [(lip, ledge, cx - fw / 2, fw), (ledge, pool, cx - fw / 2 + 1, (fw - 1).max(1))] {
        for y in y0..y1 {
            for x in x0..x0 + wd {
                let hs = hash(x * 7, y - (t / 2) as i32);
                let c = lerp((140, 180, 220), (250, 250, 255), (hs % 50) as f32 / 50.0);
                let c = if y < lit_to { lerp(c, (255, 220, 180), 0.3) } else { c };
                cv.put(x, y, if hs % 3 == 0 { '║' } else { '│' }, rgb(c), Some(rgb(lerp(c, (60, 100, 150), 0.5))));
            }
        }
        // White water where it lands.
        for k in 0..3 {
            let age = ((t / 3) as i32 + k * 3) % 6;
            cv.glyph(x0 - 1 + (k * 2) % (wd + 2), y1 - age / 3, '°', rgb((250, 250, 255)));
        }
    }

    // The valley floor: pines and the pool, the river running off.
    for y in g.max(0)..h {
        for x in 0..w {
            if cliff(x, y) {
                continue;
            }
            let hs = hash(x * 5 + salt, y * 3 + 2);
            let water = y >= pool - 1 && y <= pool + 2 && (x - cx).abs() < cw + (y - pool) * 3;
            let river = y > pool + 2 && (x - (cx + (y - pool) * 2)).abs() < 2 + (y - pool) / 2;
            if water || river {
                let f = (x as f32 * 0.4 - tf * 0.15 + y as f32 * 2.0).sin();
                cv.put(x, y, if f > 0.8 { '~' } else { ' ' }, rgb((200, 220, 240)), Some(rgb(lerp((50, 80, 120), low, 0.15))));
            } else {
                let (ch, fg) = if hs % 3 == 0 { ('♠', (30, 70, 50)) } else if hs % 5 == 0 { ('♣', (40, 84, 54)) } else { (' ', (0, 0, 0)) };
                cv.put(x, y, ch, rgb(fg), Some(rgb(lerp((26, 44, 36), (40, 56, 40), (y - g) as f32 / (h - g).max(1) as f32))));
            }
        }
    }
    cv.halo(cx, pool, (cw as f32 * 0.5).max(2.0), (230, 236, 245), 0.55);

    // Mist banks drifting through the valley and the trees on the ridge.
    for k in 0..4 {
        let row = g - 2 + k * ((h - g) / 4).max(1);
        let len = (w / 4).max(6);
        let x0 = drift(t, k + salt, 6 + k as u64 * 2, w + len) - len / 2;
        for i in 0..len {
            let f = (i as f32 / len as f32 * std::f32::consts::PI).sin();
            cv.glow(x0 + i, row, (225, 220, 230), f * 0.45);
            cv.glow(x0 + i + 2, row + 1, (225, 220, 230), f * 0.25);
        }
    }

    // Someone down early, filling buckets at the pool.
    let bx = cx + cw + 2 + pingpong(t, 20, 0, 6);
    if bx < w {
        dwarf(cv, bx, pool - 2, (170, 120, 200));
        let full = pingpong(t, 20, 0, 6) < 2;
        cv.glyph(bx + 1, pool - 2, if full { 'Ü' } else { 'u' }, rgb((150, 120, 80)));
    }
    // A heron lifting off.
    let hx = drift(t, salt, 3, w + 20) - 10;
    let hy = (g as f32 * 0.6) as i32 - ((hx as f32 * 0.05).sin() * 2.0) as i32;
    cv.glyph(hx, hy, if (t / 6) % 2 == 0 { '^' } else { '~' }, rgb((40, 36, 50)));
}
