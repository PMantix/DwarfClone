//! A rainbow after rain: the storm drawing off over the far hills, sun
//! breaking through behind, a great bow (and a fainter second one) over
//! the wet valley, the last drops falling, puddles ringing, the grass
//! shining, and the hold's folk coming out to look. The seed picks where
//! the bow stands and how the clouds lie.

use super::a_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the gods paint the sky; the dwarves mine the colours",
    "somewhere under that bow is a vein of gold",
    "after the rain, everything smells of stone",
    "the storm went over; the mountain stayed",
];

const BANDS: [Rgb; 6] = [(240, 70, 70), (250, 160, 60), (250, 230, 90), (100, 210, 100), (80, 140, 240), (150, 90, 210)];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let g = (cv.sky - 1).max(1);
    let tf = t as f32;
    let salt = cv.salt();
    let p = (cv.roll(1) % 628) as f32 / 100.0;
    // The storm drifts off to one side as the scene goes on.
    let gone = ((t + cv.roll(2) as u64 % 600) % 4500) as f32 / 4500.0;
    let storm_edge = w as f32 * (0.55 - gone * 0.3);

    // Sky: clean blue behind, a grey wall of cloud where the storm still is.
    for y in 0..g.min(h) {
        let base = lerp((90, 150, 220), (200, 225, 240), y as f32 / g.max(1) as f32);
        for x in 0..w {
            let lump = (y as f32 * 0.8 + p).sin() * 3.0 + (x as f32 * 0.2 + y as f32 * 0.5).sin() * 1.5;
            let d = x as f32 - storm_edge - lump;
            let c = if d > 0.0 { lerp((140, 146, 160), (80, 86, 104), (d / 20.0).min(1.0)) } else { base };
            let c = if d > -2.0 && d <= 0.0 { lerp(c, (250, 250, 255), 0.5) } else { c };
            cv.fill(x, y, rgb(c));
        }
    }
    // The sun behind the viewer's shoulder, low: its light on the clouds.
    let (sx, sy) = ((w as f32 * 0.08) as i32, (g as f32 * 0.2) as i32 + 1);
    cv.halo(sx, sy, (g as f32 * 0.3).max(2.0), (255, 245, 200), 0.5);
    cv.disc(sx, sy, (g as f32 * 0.07).clamp(0.6, 2.0), (255, 250, 230));

    // The bow: bright where it stands against the dark cloud.
    let (cx, cy) = ((w as f32 * (0.55 + (cv.roll(3) % 20) as f32 / 100.0)) as i32, g + (g / 4));
    let r0 = (g as f32 * 1.05).max(4.0);
    let width = (r0 * 0.035).max(0.45);
    let shimmer = 0.85 + (tf * 0.03).sin() * 0.15;
    for y in 0..g.min(h) {
        for x in 0..w {
            let (dx, dy) = ((x - cx) as f32 * 0.5, (y - cy) as f32);
            let r = (dx * dx + dy * dy).sqrt();
            // Brightest against the storm, faint against the clear sky.
            let dark = if x as f32 > storm_edge { 1.0 } else { 0.0 };
            let band = (r - r0) / width;
            if (0.0..6.0).contains(&band) {
                cv.glow(x, y, BANDS[5 - band as usize], (0.3 + dark * 0.35) * shimmer);
            } else {
                let band2 = (r - r0 * 1.22) / (width * 1.3);
                if (0.0..6.0).contains(&band2) {
                    cv.glow(x, y, BANDS[band2 as usize], (0.08 + dark * 0.14) * shimmer);
                }
            }
        }
    }

    // Hills, wet and shining.
    let surf: Vec<i32> = (0..w).map(|x| g - 1 - (ridge(x, w, seed, 210) * g as f32 * 0.35) as i32).collect();
    for x in 0..w {
        let s = surf[x as usize];
        let shade = if x as f32 > storm_edge { 0.35 } else { 0.0 };
        for y in s.max(0)..g {
            let c = lerp(lerp((90, 160, 70), (60, 120, 50), (y - s) as f32 / 5.0), (30, 40, 40), shade);
            cv.put(x, y, if y == s { '▄' } else if hash(x + salt, y) % 6 == 0 { '♣' } else { ' ' }, rgb(lerp(c, (180, 230, 140), 0.4)), Some(rgb(c)));
        }
    }
    // Rain still falling under the storm.
    let mut k = 0;
    while k < (w * g / 8).max(4) {
        let x = (storm_edge as i32 + 2 + (hash(k + salt, 1) % (w as u32 / 2).max(1)) as i32).min(w + 2);
        let y = ((hash(k + salt, 2) as u64 + t) % g.max(1) as u64) as i32;
        cv.glyph(x - y / 3, y, '/', rgb((170, 180, 200)));
        k += 1;
    }

    // The valley: grass shining, puddles ringing with the last drops.
    for y in g.max(0)..h {
        for x in 0..w {
            let hs = hash(x * 3 + salt, y * 5);
            let puddle = ((x as f32 * 0.17 + y as f32 * 0.9 + p).sin() + (x as f32 * 0.07 - y as f32 * 0.4).sin()) > 1.3;
            if puddle {
                // The sky (and the bow) in the water.
                let ring = hash(x, y + (t / 6) as i32) % 23 == 0;
                cv.put(x, y, if ring { 'o' } else { ' ' }, rgb((220, 230, 250)), Some(rgb((130, 170, 210))));
            } else {
                let glint = hs % 29 == 0 && (t / 4 + hs as u64) % 13 == 0;
                let ch = if glint { '·' } else if hs % 5 == 0 { '"' } else { ' ' };
                cv.put(x, y, ch, rgb(if glint { (255, 255, 255) } else { (150, 210, 110) }), Some(rgb(lerp((80, 150, 60), (60, 120, 46), (hs % 10) as f32 / 12.0))));
            }
        }
    }

    // The hold door and the folk come out to look, pointing up.
    let hx = (w as f32 * 0.16) as i32;
    if g >= 4 {
        for y in g - 2..g {
            cv.fill(hx, y, rgb((40, 30, 24)));
        }
        cv.sprite(hx - 1, g - 3, &["▄▄▄"], rgb((130, 120, 110)));
        let out = ((t / 40) % 6) as i32;
        for k in 0..out {
            let x = hx + 2 + k * 2;
            dwarf(cv, x, g - 1, COATS[((k + salt) % 8) as usize]);
            if (t / 20 + k as u64) % 3 == 0 {
                cv.glyph(x + 1, g - 2, '/', rgb((220, 200, 180)));
            }
        }
    }
    // Drops off the eaves and leaves.
    for k in 0..(w / 8).max(3) {
        let x = (hash(k + salt, 8) % w.max(1) as u32) as i32;
        let fall = ((t / 2) as i32 + k * 7) % 12;
        cv.glyph(x, g - 3 + fall / 2, '·', rgb((200, 220, 250)));
    }
    // A pair of swallows out after the midges.
    for k in 0..2 {
        let a = tf * 0.06 + k as f32 * 3.0;
        let x = (w as f32 * 0.3 + a.cos() * w as f32 * 0.15) as i32;
        let y = (g as f32 * 0.6 + (a * 2.0).sin() * 2.0) as i32;
        cv.glyph(x, y, if (t / 3 + k as u64) % 2 == 0 { 'v' } else { '^' }, rgb((30, 30, 50)));
    }
}
