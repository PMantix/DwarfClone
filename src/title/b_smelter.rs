//! The smelter: a brick furnace roaring as the bellows work, ore tipped
//! into its mouth, and then the pour, a white-hot ribbon of metal running
//! down the trough into the moulds, where the bars cool from white to
//! orange to red to plain honest metal and are stacked with the rest.
//! Below, the ore bins and the coal. The seed picks the metal, the brick
//! and the size of the stack.

use super::b_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "rock goes in, wealth comes out",
    "mind your boots near the trough",
    "the furnace is hungry again",
    "every bar a little sun, cooling",
];

/// Metals once cool: (name for the ore's fleck, cold colour).
const METALS: [(Rgb, Rgb); 5] = [
    ((200, 120, 70), (150, 150, 160)),  // iron
    ((235, 195, 80), (240, 200, 80)),   // gold
    ((200, 110, 70), (210, 120, 70)),   // copper
    ((205, 205, 220), (210, 215, 225)), // silver
    ((120, 160, 120), (160, 170, 150)), // tin-ish bronze
];

/// The colour of metal `f` of the way from pour-hot (0) to cold (1).
fn cooling(f: f32, cold: Rgb) -> Rgb {
    if f < 0.25 {
        lerp((255, 250, 210), (255, 190, 70), f / 0.25)
    } else if f < 0.55 {
        lerp((255, 190, 70), (210, 60, 30), (f - 0.25) / 0.3)
    } else {
        lerp((210, 60, 30), cold, (f - 0.55) / 0.45)
    }
}

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t) = (cv.w(), cv.h(), cv.t);
    let g = (cv.sky - 1).max(3).min(h - 1);
    let tf = t as f32;
    let salt = cv.salt();
    let (ore, cold) = METALS[(cv.roll(1) % 5) as usize];
    let brick = [(120, 56, 40), (110, 84, 64), (90, 80, 84)][(cv.roll(2) % 3) as usize];
    let air = (26, 18, 16);

    // The beat: feed the fire, pour, cool, stack.
    let at = beat(cv, 1000, 3);
    let (pour_at, pour_end, stack_at) = (320u64, 420u64, 880u64);
    let heat = 0.55 + 0.45 * span(at, 0, pour_at) * (1.0 - span(at, pour_end, pour_end + 200));
    let f = flicker(t, 1);

    // The hall, glowing near the furnace.
    let fx = 2 + (cv.roll(4) % 3) as i32; // furnace's left edge
    let fw = (w / 10).clamp(5, 11);
    let mouth = (fx + fw / 2, g - 2);
    for y in 0..g {
        for x in 0..w {
            let (dx, dy) = ((x - mouth.0) as f32 * 0.5, (y - mouth.1) as f32);
            let d = (dx * dx + dy * dy).sqrt();
            let glow = (1.0 - d / (w as f32 * 0.25).max(8.0)).max(0.0) * heat * f;
            cv.fill(x, y, mix(air, (120, 50, 20), glow));
        }
    }
    floor(cv, 0, w, g, (90, 80, 70));

    // The furnace: a brick stack up to the roof, smoke out of the top.
    let ftop = 1;
    for y in ftop..g {
        let taper = ((g - y) as f32 / (g - ftop).max(1) as f32 * fw as f32 * 0.2) as i32;
        let (l, r) = (fx + (fw / 3 - taper / 2).max(0) * (y < g - 4) as i32, fx + fw - 1 - (fw / 3 - taper / 2).max(0) * (y < g - 4) as i32);
        for x in l..=r {
            let mortar = (y % 2 == 0 && x % 3 == 0) || (y % 2 == 1 && x % 3 == 1);
            let hot = (1.0 - (y - mouth.1).abs() as f32 / 5.0).max(0.0) * heat;
            cv.put(x, y, if mortar { '▪' } else { ' ' }, rgb(lerp(brick, (0, 0, 0), 0.4)), Some(rgb(lerp(brick, (255, 120, 40), hot * 0.5))));
        }
    }
    for k in 0..6 {
        let age = ((t / 4) as i32 + k * 3) % 14;
        let sx = fx + fw / 2 + ((age as f32 * 0.6 + k as f32).sin() * 1.5) as i32 + age / 3;
        if ftop - 1 - age / 4 >= 0 || age < 3 {
            cv.glyph(sx, (ftop - age / 4).max(0), '░', rgb(lerp((120, 110, 110), air, age as f32 / 14.0)));
        }
    }
    // Its mouth, roaring.
    for x in mouth.0 - 1..=mouth.0 + 1 {
        let c = lerp((255, 120, 30), (255, 240, 170), f * heat);
        cv.put(x, mouth.1, if (t / 3 + x as u64) % 3 == 0 { '▲' } else { '♠' }, rgb(c), Some(rgb((160, 40, 10))));
    }
    cv.halo(mouth.0, mouth.1, 4.0, (255, 140, 50), 0.5 * heat * f);
    // The bellows, worked by a dwarf.
    let pump = (tf * 0.25).sin() > 0.0 && at < pour_end;
    cv.glyph(fx - 1, g - 1, if pump { '▄' } else { '█' }, rgb((120, 86, 50)));
    cv.glyph(fx - 2, g - 1, '☺', rgb(beard(cv.roll(5))));
    // Sparks up the chimney's face.
    for k in 0..4 {
        let age = ((t / 2) as i32 + k * 5) % 12;
        cv.glyph(mouth.0 + (hash(k, (t / 24) as i32) % 3) as i32 - 1, mouth.1 - 1 - age / 2, '·', rgb((255, 200, 90)));
    }

    // The crucible by the mouth; it tips for the pour.
    let cx = fx + fw + 1;
    let cy = g - 3;
    let tipped = at >= pour_at && at < pour_end + 20;
    let melt = cooling(0.0, cold);
    if tipped {
        cv.sprite(cx, cy, &["▀▄", " ▀▄"], rgb((80, 74, 70)));
        cv.glyph(cx + 1, cy, '▄', rgb(melt));
    } else {
        cv.sprite(cx, cy, &["▐▀▌", "▐▄▌"], rgb((80, 74, 70)));
        cv.glyph(cx + 1, cy, '▄', rgb(if at < pour_at { lerp((200, 60, 20), melt, span(at, 100, pour_at)) } else { (60, 50, 50) }));
    }

    // The trough and its moulds, filling as the metal runs.
    let t0 = cx + 3;
    let t1 = (w * 4 / 5).max(t0 + 4).min(w - 2);
    let run = span(at, pour_at, pour_end); // how far the front has got
    let front = t0 + (run * (t1 - t0) as f32) as i32;
    let spout = (cx + 3, cy + 1);
    if tipped && at < pour_end {
        for y in spout.1..g - 1 {
            cv.glyph(spout.0, y, '│', rgb(melt));
            cv.glow(spout.0, y, (255, 200, 90), 0.5);
        }
    }
    for x in t0..t1 {
        let mould = (x - t0) % 4 == 2;
        let filled = (at >= pour_at && x <= front) || (at < pour_at && false);
        if !filled || at >= stack_at {
            cv.put(x, g - 1, if mould { '▄' } else { '_' }, rgb((90, 84, 80)), None);
            continue;
        }
        // Cooling from where the metal reached first.
        let age = at.saturating_sub(pour_at + ((x - t0) as f32 / (t1 - t0).max(1) as f32 * (pour_end - pour_at) as f32) as u64);
        let c = cooling(age as f32 / 420.0, cold);
        if mould {
            cv.put(x, g - 1, '▬', rgb(c), None);
        } else {
            cv.put(x, g - 1, '─', rgb(lerp(c, (90, 84, 80), 0.3)), None);
        }
        if age < 250 {
            // The heat shimmer over the hot metal.
            let wob = ((x as f32 * 0.9 + tf * 0.3).sin() * 0.5 + 0.5) * (1.0 - age as f32 / 250.0);
            for k in 1..4 {
                cv.glow(x, g - 1 - k, (255, 150, 60), wob * 0.35 / k as f32);
            }
            if (hash(x, (t / 4) as i32) % 23) == 0 {
                cv.glyph(x, g - 2 - (hash(x, 3) % 2) as i32, '°', rgb((240, 230, 220)));
            }
        }
    }

    // The stack of bars, growing; a dwarf carrying the new ones over.
    let sx = (t1 + 2).min(w - 3);
    let stacked = 3 + (cv.roll(6) % 8) as i32 + if at >= stack_at { 2 } else { 0 };
    for k in 0..stacked {
        let (row, col) = (k / 3, k % 3);
        let x = sx - 1 + col - (row % 2);
        if x < w {
            cv.glyph(x, g - 1 - row, '▬', rgb(lerp(cold, (255, 255, 255), if (t / 7 + k as u64) % 29 == 0 { 0.6 } else { 0.0 })));
        }
    }
    if at >= stack_at || at < 120 {
        let carry = if at >= stack_at { span(at, stack_at, 1000) } else { 1.0 };
        let x = t0 + ((sx - 2 - t0) as f32 * carry) as i32;
        cv.glyph(x, g - 1, '☺', rgb(beard(cv.roll(7))));
        cv.glyph(x, g - 2, '▬', rgb(cold));
    } else {
        // Otherwise feeding the furnace, barrow by barrow.
        let x = pingpong(t, 4, cx + 2, (t1 - 4).max(cx + 3));
        cv.glyph(x, g - 1, '☺', rgb(beard(cv.roll(7))));
        cv.sprite(x - 2, g - 1, &["▐▌"], rgb((120, 86, 50)));
        cv.glyph(x - 2, g - 2, '▲', rgb(ore));
    }
    // Lamps.
    torch(cv, w * 3 / 5, g - 5, 2, 3.0);
    torch(cv, w - 3, g - 5, 4, 3.0);

    // The ore bins and coal below.
    let by = h - 4;
    if by - g > 6 {
        bedrock(cv, g + 1, h, (64, 56, 52), salt);
        hollow(cv, 1, by, w - 1, h - 1, (20, 16, 14));
        floor(cv, 1, w - 1, h - 1, (90, 80, 70));
        let n = ((w - 2) / 8).max(1);
        for k in 0..n {
            let x = 3 + k * 8;
            let (ch, c) = if k % 3 == 1 { ('■', (40, 38, 40)) } else { ('▲', ore) };
            cv.sprite(x, h - 3, &[" ▄ ", "▄█▄"], rgb(lerp(c, (0, 0, 0), 0.3)));
            if hash(k, salt) % 2 == 0 {
                cv.glyph(x + 1, h - 3, ch, rgb(c));
            }
        }
        let dx = pingpong(t + 300, 5, 2, w - 4);
        cv.glyph(dx, h - 2, '☺', rgb(beard(cv.roll(8))));
        torch(cv, w / 2, by, 5, 3.0);
    }
}
