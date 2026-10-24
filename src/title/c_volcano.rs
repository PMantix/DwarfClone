//! The fire mountain: a volcanic island in a dark sea, its crater
//! fountaining liquid fire, glowing bombs arcing out and hissing into the
//! water, rivers of lava running down to the shore and bursting into
//! steam, and a great plume of ash climbing and spreading, lit red from
//! beneath and cracked by lightning. Every so often it gathers itself and
//! blows. The seed picks the hour, the mountain's shape and the wind.

use super::*;
use super::c_kit::*;

pub(super) const TAGLINES: &[&str] = &[
    "the mountain clears its throat once an age",
    "under every island, a furnace that never sleeps",
    "the sea boils where the fire meets it",
    "a dwarf would call it a fine forge, from far away",
];

/// Frames between the great blasts.
const BLAST: u64 = 420;

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let tf = t as f32;
    let sky = cv.sky.min(h - 1).max(1);
    let sea = (sky - 2).max(2).min(h - 1); // the waterline
    let salt = cv.salt();
    let night = cv.roll(1) % 2 == 0;
    let wind = if cv.roll(2) % 2 == 0 { 1.0 } else { -1.0 };
    let bp = phase(t, BLAST, cv.roll(3) % BLAST as u32);
    let fury = if bp < 0.12 { 1.0 - bp / 0.12 * 0.4 } else { 0.6 * (1.0 - (bp - 0.12) / 0.88) } + 0.25; // 0.25 .. 1.25
    let (top, bot) = if night { ((10, 6, 16), (70, 30, 30)) } else { ((70, 70, 90), (200, 140, 110)) };
    cv.sky_fill(sea, top, bot);
    if night {
        cv.stars(sea / 2, 31);
    }

    // The mountain.
    let cx = (w as f32 * 0.6) as i32;
    let peak_y = (sea as f32 - sky as f32 * 0.55).max(3.0) as i32;
    let half = ((w as f32 * 0.24) as i32).max(6);
    let crater = (half / 6).max(1);
    let surface = |x: i32| -> i32 {
        let d = (x - cx).abs();
        if d <= crater {
            peak_y + 1
        } else {
            let f = ((d - crater) as f32 / (half - crater).max(1) as f32).min(1.0);
            peak_y + ((sea - peak_y) as f32 * f.powf(0.75)) as i32 + (hash(x, 5) % 2) as i32
        }
    };

    // The ash plume, rising and leaning with the wind.
    for y in 0..peak_y.min(h) {
        let up = (peak_y - y) as f32;
        let centre = cx as f32 + wind * up * up * 0.05 * (w as f32 / 120.0).max(0.4);
        let spread = 1.5 + up * 0.9 * (0.7 + fury * 0.4);
        for x in 0..w {
            let d = ((x as f32 - centre) / spread).abs();
            if d > 1.3 {
                continue;
            }
            let n = fbm(x as f32 * 0.12 - wind * tf * 0.02, y as f32 * 0.25 + tf * 0.03, salt, 3);
            let dens = ((1.3 - d) * 0.9 + (n - 0.5)).clamp(0.0, 1.0);
            if dens > 0.15 {
                let under = (1.0 - up / peak_y.max(1) as f32).max(0.0) * fury;
                let c = lerp(lerp((60, 54, 56), (30, 26, 30), n), (220, 80, 30), under * 0.6);
                cv.glow(x, y, c, dens * 0.92);
            }
        }
    }
    // Lightning inside the plume.
    let zap_n = ((t + seed % 97) / 53) as i32;
    if (t + seed % 97) % 53 < 3 && peak_y > 4 {
        let mut x = cx + (hash(zap_n, 1) % 9) as i32 - 4 + (wind * 4.0) as i32;
        for y in (peak_y / 3).max(0)..(peak_y * 3 / 4) {
            cv.glyph(x, y, if hash(x, y) % 2 == 0 { '/' } else { '\\' }, rgb((230, 220, 255)));
            x += if hash(y, zap_n) % 2 == 0 { 1 } else { -1 };
        }
    }

    // The cone, glowing cracks and lava runs on its flanks.
    let runs: Vec<i32> = (0..3).map(|k| (hash(k, salt) % (half as u32 * 2)) as i32 - half).collect();
    for x in (cx - half - 1)..=(cx + half + 1) {
        let s = surface(x);
        for y in s.max(0)..=sea {
            let hs = hash(x + salt, y);
            let rock = lerp((40, 32, 32), (20, 16, 18), (y - s) as f32 / 6.0);
            let ch = if hs % 6 == 0 { '▒' } else { ' ' };
            cv.put(x, y, ch, rgb((56, 40, 36)), Some(rgb(rock)));
        }
    }
    for (k, &r) in runs.iter().enumerate() {
        // From the crater lip down to the sea, wandering a little.
        let mut x = cx + if r < 0 { -crater } else { crater };
        let dir = if r < 0 { -1 } else { 1 };
        let reach = ((t as f32 * 0.05 + k as f32 * 30.0) % (sea - peak_y + 12) as f32) as i32 + 4;
        for y in (peak_y + 1)..=sea {
            if y - peak_y > reach {
                break;
            }
            let goal = cx + r;
            if (x - goal).abs() > 0 && hash(y, k as i32 + salt) % 2 == 0 {
                x += dir.min(1).max(-1) * if (goal - x).signum() == dir { 1 } else { 0 };
            }
            let s = surface(x);
            let yy = y.max(s);
            let heat = 0.5 + 0.5 * (tf * 0.2 - y as f32 * 0.6 + k as f32).sin();
            cv.put(x, yy, if heat > 0.7 { '≈' } else { '~' }, mix((255, 220, 90), (255, 90, 20), heat), Some(mix((200, 60, 10), (140, 30, 10), heat)));
            if yy >= sea {
                // Steam where it meets the sea.
                for j in 0..4 {
                    let age = ((t / 3) as i32 + j * 4 + k as i32 * 5) % 16;
                    cv.glyph(x + (age / 4) * dir, sea - 1 - age / 3, if age < 8 { '░' } else { '·' }, mix((235, 235, 240), bot, age as f32 / 16.0));
                }
                break;
            }
        }
    }

    // The crater and its fountain.
    cv.halo(cx, peak_y, 4.0 + fury * 4.0, (255, 120, 40), 0.6);
    for dx in -crater..=crater {
        cv.put(cx + dx, peak_y + 1, '≈', rgb((255, 230, 120)), Some(rgb((230, 90, 20))));
    }
    let jet = (2.0 + fury * sky as f32 * 0.25) as i32;
    for j in 0..jet {
        for dx in -1..=1 {
            let hs = hash(dx + j * 3, (t / 2) as i32);
            if hs % 3 == 0 || (dx != 0 && j > jet / 2) {
                continue;
            }
            let col = [(255, 250, 180), (255, 200, 80), (250, 120, 30)][((j * 3 / jet.max(1)) as usize).min(2)];
            cv.glyph(cx + dx, peak_y - j, ['▲', '♠', '*', '▓'][(hs % 4) as usize], rgb(col));
        }
    }
    // Bombs arcing out, glowing, into the sea.
    for k in 0..(4 + (fury * 6.0) as i32) {
        let life = 40 + (hash(k, 9) % 30) as u64;
        let age = (t + hash(k, 10) as u64) % life;
        let n = ((t + hash(k, 10) as u64) / life) as i32;
        let vx = ((hash(k, n) % 100) as f32 / 100.0 - 0.5) * 1.6;
        let vy = 0.6 + (hash(n, k) % 100) as f32 / 140.0 * fury;
        let a = age as f32;
        let (x, y) = (cx as f32 + vx * a, peak_y as f32 - vy * a + 0.03 * a * a);
        let (xi, yi) = (x as i32, y as i32);
        if yi <= sea {
            cv.glyph(xi, yi, '●', mix((255, 230, 120), (180, 50, 20), a / life as f32));
        }
    }

    // The sea, red with the fire.
    let glow_x = cx as f32;
    for y in sea.max(0)..h {
        let d = ((y - sea) as f32 / (h - sea).max(1) as f32).min(1.0);
        for x in 0..w {
            if (x - cx).abs() <= half + 1 && y <= sea {
                continue;
            }
            let near = (1.0 - (x as f32 - glow_x).abs() / (w as f32 * 0.4).max(4.0)).max(0.0) * (1.0 - d) * fury;
            let base = lerp(lerp((20, 34, 54), (6, 10, 20), d), (200, 60, 20), near * 0.5);
            let ph = (x as f32 * 0.22 + y as f32 * 1.6 - tf * 0.07).sin();
            let (ch, fg) = if ph > 0.85 { ('~', lerp(base, (255, 160, 90), 0.5 + near * 0.3)) } else { (' ', base) };
            cv.put(x, y, ch, rgb(fg), Some(rgb(base)));
        }
    }
    // A ship running for it.
    let sx = ((t / 8 + cv.roll(4) as u64) % (w as u64 + 20)) as i32 - 10;
    let sx = if wind > 0.0 { sx } else { w - sx };
    if (sx - cx).abs() > half + 3 {
        cv.glyph(sx, sea + 1, '▲', rgb((200, 190, 170)));
        cv.glyph(sx, sea + 2, '▀', rgb((90, 60, 40)));
    }
}
