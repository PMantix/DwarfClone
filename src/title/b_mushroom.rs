//! The underground farm through one season: muddy plots under the rock,
//! farmers sowing, the sprouts coming up pale, the plump helmets swelling
//! until the whole cave glows with them and the spores drift, then the
//! harvest, baskets heaped, and the plots bare again. Below, the cellar
//! where it all ends up as something to drink. The seed picks the crops
//! and the glow.

use super::b_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "plump helmets: food, drink and lantern in one",
    "no sun, no rain, no problem",
    "the farmers sing to the mushrooms. it helps",
    "sow in spring, brew in autumn",
];

/// Crops: (ripe glyph, ripe colour, glow).
const CROPS: [(char, Rgb, Rgb); 4] = [
    ('♠', (190, 110, 255), (110, 50, 170)), // plump helmets
    ('"', (230, 210, 120), (110, 90, 30)),  // cave wheat
    (',', (255, 150, 170), (120, 50, 70)),  // pig tails
    ('♣', (120, 230, 200), (30, 110, 90)),  // sweet pods
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t) = (cv.w(), cv.h(), cv.t);
    let g = (cv.sky - 1).max(3).min(h - 1);
    let tf = t as f32;
    let salt = cv.salt();
    let stone = [(70, 62, 56), (60, 60, 70), (72, 66, 58)][(cv.roll(1) % 3) as usize];
    let air = lerp(stone, (0, 0, 0), 0.8);
    bedrock(cv, 0, h, stone, salt);

    // The beat: sow, grow, ripen and glow, harvest, lie bare.
    let at = beat(cv, 1700, 3);
    let (sown, ripe, harvest, bare) = (240u64, 900u64, 1320u64, 1560u64);
    let grow = span(at, sown, ripe);
    let ripe_now = at >= ripe && at < harvest;
    let shine = if ripe_now { span(at, ripe, ripe + 120) } else { 0.0 };

    // The cave, warmed a little by the crop's glow.
    let main = CROPS[0].2;
    for y in 1..g {
        for x in 0..w {
            let f = shine * (1.0 - (g - y) as f32 / g as f32) * 0.5;
            cv.fill(x, y, mix(air, main, f));
        }
    }
    // Stalactites and a dripping roof.
    for x in 0..w {
        let hang = (hash(x + salt, 21) % 3) as i32;
        for y in 1..1 + hang {
            cv.fill(x, y, rgb(lerp(stone, (0, 0, 0), 0.3)));
        }
        if hang > 0 {
            cv.glyph(x, 1 + hang, '▼', rgb(lerp(stone, (0, 0, 0), 0.1)));
        }
    }

    // The plots, with a channel of water between.
    let np = (w / 24).max(1) + 1;
    let pw = ((w - 4) / np - 3).max(3);
    let order = cv.roll(4) as usize;
    let mut plots = Vec::new();
    for k in 0..np {
        let x0 = 2 + k * (pw + 3);
        let crop = if k == 0 { 0 } else { (order + k as usize) % 4 };
        plots.push((x0, crop));
        for x in x0..(x0 + pw).min(w) {
            cv.put(x, g, '▀', rgb((110, 80, 50)), Some(rgb((60, 40, 24))));
        }
        let channel = x0 + pw + 1;
        if channel < w - 1 {
            let ch = if (channel + (t / 8) as i32) % 2 == 0 { '~' } else { '≈' };
            cv.put(channel, g, ch, rgb((110, 160, 220)), Some(rgb((20, 40, 70))));
        }
    }
    for &(x0, crop) in &plots {
        let (ch, col, glow) = CROPS[crop];
        for x in x0..(x0 + pw).min(w) {
            let hs = hash(x + salt, crop as i32);
            let me = (hs % 100) as f32 / 100.0 * 0.25; // each plant its own pace
            if at < sown {
                // Sown so far, left to right.
                let sown_to = x0 + (span(at, 60, sown) * pw as f32) as i32;
                if x < sown_to && hs % 2 == 0 {
                    cv.glyph(x, g - 1, '.', rgb((120, 100, 70)));
                }
            } else if at < harvest || {
                let cut = x0 + (span(at, harvest, bare - 40) * pw as f32) as i32;
                x >= cut
            } {
                let gr = (grow - me).max(0.0);
                if ripe_now || gr >= 0.95 {
                    let pulse = ((tf * 0.06 + x as f32 * 0.5).sin() * 0.5 + 0.5) * 0.3 + 0.7;
                    if crop == 0 || hs % 3 != 0 {
                        cv.glow(x, g - 1, glow, 0.7 * pulse * shine.max(0.3));
                        cv.glyph(x, g - 1, ch, rgb(lerp(col, (255, 255, 255), if (t / 5 + hs as u64) % 53 == 0 { 0.5 } else { 0.0 })));
                    }
                } else if gr > 0.5 {
                    cv.glyph(x, g - 1, if crop == 0 { 'τ' } else { '\'' }, rgb(lerp((150, 170, 120), col, gr - 0.5)));
                } else if gr > 0.05 {
                    cv.glyph(x, g - 1, '.', rgb((150, 180, 120)));
                }
            }
        }
    }
    // Spores rising off the ripe crop.
    if shine > 0.0 {
        for k in 0..(w / 4).max(3) {
            let x = (hash(k, salt + 2) % w.max(1) as u32) as i32 + ((tf * 0.05 + k as f32).sin() * 2.0) as i32;
            let span_y = (g - 2).max(1) as u64;
            let y = g - 2 - ((t / (5 + (k % 4) as u64) + hash(k, 1) as u64) % span_y) as i32;
            cv.glyph(x, y, '·', rgb(CROPS[0].1));
        }
    }

    // The farmers: sowing, tending, harvesting with baskets.
    let nf = 2 + (cv.roll(5) % 2) as i32;
    for k in 0..nf {
        let b = beard(cv.roll(6) + k as u32);
        let x = if at < sown {
            2 + (span(at, 60, sown) * (w - 6) as f32) as i32 - k * 4
        } else if at >= harvest && at < bare {
            2 + (span(at, harvest, bare - 40) * (w - 6) as f32) as i32 - k * 4
        } else {
            pingpong(t + k as u64 * 211, 7, 2, w - 4)
        };
        cv.glyph(x, g - 1, '☺', rgb(b));
        if at >= harvest && at < bare {
            cv.glyph(x + 1, g - 1, '▄', rgb((170, 130, 80)));
            cv.glyph(x + 1, g - 2, CROPS[0].0, rgb(CROPS[0].1));
        } else if at < harvest && (t / 30 + k as u64) % 5 == 0 {
            cv.glyph(x, g - 2, '♪', rgb((240, 220, 160)));
        }
    }

    // Below: the cellar, barrels and the still.
    let lb = below(cv);
    if h - lb >= 4 {
        let cy = (lb + (h - lb) / 2).min(h - 2);
        hollow(cv, 2, cy - 2, w - 2, cy + 1, lerp(stone, (20, 10, 4), 0.7));
        floor(cv, 2, w - 2, cy + 1, lerp(stone, (150, 120, 90), 0.3));
        for x in (4..w - 4).step_by(3) {
            if hash(x, salt + 7) % 4 != 0 {
                cv.glyph(x, cy, '0', rgb((150, 105, 60)));
                if hash(x, salt + 8) % 3 == 0 {
                    cv.glyph(x, cy - 1, '0', rgb((140, 95, 55)));
                }
            }
        }
        let sx = w / 2;
        cv.glyph(sx, cy, 'Ö', rgb((190, 130, 80)));
        let f = flicker(t, 4);
        cv.halo(sx, cy, 2.0, (255, 140, 60), 0.4 * f);
        let bub = (t / 6) % 4;
        cv.glyph(sx, cy - 1 - bub as i32 / 2, if bub < 2 { 'o' } else { '°' }, rgb((220, 200, 160)));
        torch(cv, w / 4, cy - 2, 5, 3.0);
        torch(cv, w * 3 / 4, cy - 2, 6, 3.0);
    }
}
