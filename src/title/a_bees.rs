//! The beekeeper's yard in high summer: a row of straw skeps on a bench
//! at the meadow's edge, a lime tree in flower, bees streaming out to the
//! clover and back in looping trails, and the beekeeper in her veil
//! puffing smoke over a hive while she lifts the honey. The seed picks
//! the flowers, the number of hives and the time of day.

use super::a_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "mead does not make itself; ask the bees",
    "ten thousand workers and not one tantrum",
    "a little smoke, a little patience, a lot of honey",
    "the whole meadow hums",
];

const SKEP: [&str; 3] = [" ▄█▄ ", "▐███▌", "▐█▀█▌"];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let g = (cv.sky - 1).max(1);
    let tf = t as f32;
    let salt = cv.salt();
    let late = cv.roll(1) % 3 == 0;
    let (top, bot) = if late { ((90, 110, 190), (250, 200, 140)) } else { ((70, 140, 220), (200, 230, 250)) };
    let bloom = [[(250, 250, 250), (240, 140, 200), (250, 220, 70)], [(170, 120, 240), (250, 250, 250), (250, 200, 90)], [(250, 100, 90), (250, 230, 90), (130, 160, 250)]][(cv.roll(2) % 3) as usize];

    cv.sky_fill(g, top, bot);
    let (sx, sy) = ((w as f32 * 0.85) as i32, (g as f32 * if late { 0.6 } else { 0.25 }) as i32 + 1);
    cv.halo(sx, sy, (g as f32 * 0.35).max(2.0), (255, 240, 190), 0.4);
    cv.disc(sx, sy, (g as f32 * 0.08).clamp(0.6, 2.4), (255, 250, 220));
    clouds(cv, (w / 40).max(1), (1, (g / 4).max(2)), 14, lerp(bot, (255, 255, 255), 0.6), (255, 255, 255), salt);

    // Low green hills, hedgerows.
    for x in 0..w {
        let s = g - 1 - (ridge(x, w, seed, 160) * g as f32 * 0.3) as i32;
        for y in s.max(0)..g {
            let ch = if y == s && hash(x + salt, 1) % 2 == 0 { '♣' } else { ' ' };
            cv.put(x, y, ch, rgb((60, 110, 50)), Some(rgb(lerp((120, 170, 90), bot, 0.35))));
        }
    }

    // The meadow, thick with clover and flowers swaying.
    for y in g.max(0)..h {
        for x in 0..w {
            let hs = hash(x * 3 + salt, y * 5 + 1);
            let sway = ((x as f32 * 0.2 + tf * 0.06 + y as f32 * 0.5).sin() > 0.6) as u32;
            let (ch, fg) = match (hs + sway) % 9 {
                0 => ('*', bloom[(hs / 9 % 3) as usize]),
                1 => ('♣', (70, 150, 60)),
                2 | 3 => ('"', (120, 190, 80)),
                4 => ('·', bloom[(hs / 7 % 3) as usize]),
                _ => (' ', (0, 0, 0)),
            };
            cv.put(x, y, ch, rgb(fg), Some(rgb(lerp((90, 150, 60), (70, 124, 46), (hs % 10) as f32 / 12.0))));
        }
    }

    // The lime tree in flower on one side.
    let tx = (w as f32 * 0.12) as i32;
    let crown = (g / 3).max(2);
    let cy = g - 1 - crown;
    for y in (cy - crown).max(0)..g {
        for x in tx - crown * 2..=tx + crown * 2 {
            let (dx, dy) = ((x - tx) as f32 / (crown * 2) as f32, (y - cy) as f32 / crown as f32);
            let hs = hash(x, y + salt);
            if dx * dx + dy * dy <= 1.0 + (hs % 3) as f32 * 0.1 {
                let ch = if hs % 5 == 0 { '·' } else if hs % 3 == 0 { '♣' } else { ' ' };
                cv.put(x, y, ch, rgb(if ch == '·' { (250, 250, 180) } else { (40, 100, 40) }), Some(rgb(if dy < -0.5 { (80, 140, 60) } else { (60, 120, 50) })));
            } else if y > cy && x == tx {
                cv.put(x, y, ' ', Color::Black, Some(rgb((90, 66, 44))));
            }
        }
    }

    // The bench of skeps.
    let hives = 3 + (cv.roll(3) % 3) as i32;
    let bench = (w as f32 * 0.22) as i32 + 2;
    let mut doors = Vec::new();
    for k in 0..hives {
        let x = bench + k * 6;
        if g >= 4 {
            cv.sprite(x, g - 4, &SKEP, rgb((210, 170, 80)));
            for i in 0..5 {
                if (i + k) % 2 == 0 {
                    cv.glyph(x + i, g - 3, '▬', rgb((170, 130, 60)));
                }
            }
            cv.put(x + 2, g - 2, '●', rgb((40, 30, 20)), None);
            cv.sprite(x, g - 1, &["╥───╥"], rgb((120, 90, 60)));
        }
        doors.push((x + 2, g - 2));
    }

    // The beekeeper, veiled, at the last hive with her smoker.
    let kx = bench + hives * 6 + 1;
    dwarf(cv, kx, g - 1, (240, 240, 230));
    cv.glyph(kx, g - 2, '∩', rgb((230, 230, 220)));
    cv.glyph(kx - 1, g - 1, '¥', rgb((140, 120, 100)));
    for k in 0..6 {
        let age = ((t / 4) as i32 + k * 3) % 18;
        let x = kx - 2 - age / 3;
        let y = g - 2 - age / 4;
        cv.glyph(x, y, if age < 6 { '░' } else { '·' }, mix((220, 220, 220), bot, age as f32 / 18.0));
    }
    // A jar of honey filling on the bench.
    let fill = pingpong(t, 40, 0, 3);
    cv.glyph(kx + 2, g - 1, ['u', 'u', 'ü', 'Ü'][fill as usize], rgb((240, 180, 40)));

    // The bees: out to the flowers and home again on looping paths.
    let n = (w / 2).max(12);
    for k in 0..n {
        let hs = hash(k + salt, 33);
        let (dx, dy) = doors[(hs % doors.len() as u32) as usize];
        let fx = (hs >> 4) % w.max(1) as u32;
        let fy = g + 1 + ((hs >> 12) % (h - g).max(1) as u32) as i32;
        let trip = 160 + (hs % 80) as u64;
        let ph = ((t + hs as u64) % trip) as f32 / trip as f32;
        let out = if ph < 0.5 { ph * 2.0 } else { 2.0 - ph * 2.0 };
        let loop_ = (ph * std::f32::consts::TAU * 3.0 + k as f32).sin();
        let x = dx as f32 + (fx as f32 - dx as f32) * out + loop_ * 2.0;
        let y = dy as f32 + (fy as f32 - dy as f32) * out + (ph * std::f32::consts::TAU * 2.0 + k as f32).cos() * 1.2 - out * (1.0 - out) * 6.0;
        let buzz = (t / 2 + k as u64) % 2;
        cv.glyph(x as i32 + buzz as i32, y as i32, '·', rgb(if (k + t as i32 / 3) % 3 == 0 { (30, 24, 10) } else { (250, 210, 40) }));
    }
}
