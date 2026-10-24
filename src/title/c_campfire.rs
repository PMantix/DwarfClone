//! The road at dusk: a lone adventurer camped beside the way under a
//! crooked tree, a small fire crackling and sending sparks up into the
//! first stars, the road winding off over the hills to wherever tomorrow
//! goes. The moon climbs, something with shining eyes watches from the
//! dark, and now and then a rider passes with a lantern. The seed picks
//! the hour, the hills, the traveller's cloak and companion, and the moon.

use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "every road is long enough to tell a story",
    "the fire keeps the wolves honest",
    "one more league tomorrow, and one after that",
    "far from any hall, close to every star",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let tf = t as f32;
    let sky = cv.sky.min(h - 1).max(1);
    let hz = (sky - 3).max(1).min(h - 1); // the far hills' feet
    let hour = cv.roll(1) % 3; // dusk, late dusk, night
    let (top, bot) = [((44, 36, 96), (245, 150, 96)), ((24, 20, 62), (170, 84, 96)), ((8, 10, 30), (42, 44, 86))][hour as usize];
    cv.sky_fill(hz, top, bot);
    cv.stars((hz * 2 / 3).max(0), [131, 53, 29][hour as usize]);
    // The last of the sunset along the hills.
    let glow = [(255, 190, 110), (230, 110, 90), (90, 70, 110)][hour as usize];
    for y in (hz - 4).max(0)..hz {
        for x in 0..w {
            let f = (1.0 - (hz - y) as f32 / 4.0) * (1.0 - (x as f32 / w.max(1) as f32 - 0.3).abs()).max(0.0);
            cv.glow(x, y, glow, f * 0.45);
        }
    }

    // The moon, climbing slowly, in whatever phase the seed says.
    let rise = (tf / 1500.0).min(1.0);
    let (mx, my) = ((w as f32 * 0.82) as i32, (hz as f32 * 0.45) as i32 - (rise * hz as f32 * 0.2) as i32);
    let mr = (hz as f32 * 0.1).max(1.0).min(2.5);
    let moon = (240, 232, 200);
    cv.halo(mx, my, mr * 3.0, moon, 0.25);
    cv.disc(mx, my, mr, moon);
    let shade = (cv.roll(2) % 5) as i32 - 2;
    if shade != 0 {
        let behind = lerp(top, bot, my as f32 / hz.max(1) as f32);
        cv.disc(mx + shade * 2, my, mr, behind);
    }

    // Mountains far off, then two lines of hills, darker as they near.
    let peaks_c = lerp(bot, (60, 50, 90), 0.45);
    let far = lerp(bot, (34, 30, 54), 0.68);
    let near = lerp(bot, (16, 18, 26), 0.85);
    for x in 0..w {
        let m = hz - 1 - (peaks(x, w, seed, 30, 4) * hz as f32 * 0.5) as i32;
        for y in m.max(0)..hz {
            let snow = y - m < 1 && hz - m > hz / 3;
            cv.fill(x, y, rgb(if snow { lerp(peaks_c, (230, 220, 230), 0.35) } else { peaks_c }));
        }
        let a = hz - 1 - (ridge(x, w, seed, 10) * hz as f32 * 0.28) as i32;
        for y in a.max(0)..hz {
            cv.fill(x, y, rgb(far));
        }
        let b = hz - (ridge(x, w, seed, 20) * hz as f32 * 0.14) as i32;
        for y in b.max(0)..hz {
            cv.fill(x, y, rgb(near));
        }
    }

    // The near land, and the road winding away over it.
    let ground = [(38, 42, 28), (26, 30, 24), (14, 18, 18)][hour as usize];
    let dust = lerp((120, 100, 76), ground, 0.25 + hour as f32 * 0.2);
    let p0 = (cv.roll(3) % 628) as f32 / 100.0;
    let road = |y: i32| -> (i32, i32) {
        let d = (y - hz) as f32 / (h - hz).max(1) as f32;
        let cx = w as f32 * (0.52 + 0.28 * d) + (d * 6.0 + p0).sin() * w as f32 * 0.07 * (0.3 + d);
        (cx as i32, (d * w as f32 * 0.05) as i32)
    };
    let salt = cv.salt();
    for y in hz.max(0)..h {
        let (rc, rw) = road(y);
        let d = (y - hz) as f32 / (h - hz).max(1) as f32;
        for x in 0..w {
            if (x - rc).abs() <= rw {
                let ch = if hash(x + salt, y) % 7 == 0 { '·' } else { ' ' };
                cv.put(x, y, ch, rgb(lerp(dust, (200, 180, 150), 0.3)), Some(rgb(dust)));
            } else {
                let hs = hash(x * 3 + salt, y * 7);
                let ch = match hs % 9 {
                    0 => '"',
                    1 => ',',
                    2 => '\'',
                    _ => ' ',
                };
                let grass = lerp((70, 96, 50), ground, 0.3 + hour as f32 * 0.25);
                cv.put(x, y, ch, rgb(grass), Some(rgb(lerp(ground, (8, 10, 10), d * 0.5))));
            }
        }
    }

    // The camp, above where the menu sits: the crooked tree, the fire.
    let fx = ((w as f32 * 0.17) as i32).max(4);
    let fy = (sky - 2).max(hz + 1).min(h - 1);
    let tx = fx - 7;
    // The crooked tree, black against the sunset.
    let bark = (24, 18, 18);
    let tall = (sky / 2).max(4);
    for k in 0..tall {
        let y = fy - k;
        let lean = (k * k) / (tall * 2).max(1);
        cv.put(tx + lean, y, ' ', Color::Black, Some(rgb(bark)));
        if k == tall / 2 {
            for j in 1..4 {
                cv.put(tx + lean - j, y - j / 2, if j == 3 { '╲' } else { ' ' }, rgb(bark), if j == 3 { None } else { Some(rgb(bark)) });
            }
        }
    }
    let (cx_, cy_) = (tx + (tall * tall) / (tall * 2).max(1), fy - tall);
    for dy in -2..=1i32 {
        for dx in -6..=6i32 {
            let (u, v) = (dx as f32 / 6.0, dy as f32 / 2.2);
            let n = (hash(dx + salt, dy) % 100) as f32 / 100.0;
            if u * u + v * v <= 0.75 + n * 0.4 {
                cv.put(cx_ + dx, cy_ + dy, if hash(dx, dy + salt) % 3 == 0 { '♣' } else { ' ' }, rgb((26, 34, 26)), Some(rgb((12, 14, 14))));
            }
        }
    }
    // The tent.
    let canvas = (150, 120, 80);
    let tent = fx + 8;
    for (dy, half) in [(0, 2), (1, 1), (2, 0)] {
        for dx in -half..=half {
            let ch = if dy == 2 { '▲' } else if dx == 0 && dy == 0 { '▌' } else { ' ' };
            if dy == 2 {
                cv.glyph(tent + dx, fy - dy, ch, rgb(canvas));
            } else {
                cv.put(tent + dx, fy - dy, ch, rgb((30, 20, 14)), Some(rgb(lerp(canvas, (0, 0, 0), if dx > 0 { 0.35 } else { 0.0 }))));
            }
        }
    }
    // The fire's light, flickering on everything near.
    let flick = ((tf * 0.7).sin() * 0.5 + (tf * 1.9).sin() * 0.3 + (hash((t / 3) as i32, 5) % 100) as f32 / 300.0) * 0.6;
    cv.halo(fx, fy, 6.0 + flick * 2.0, (255, 140, 60), 0.42 + flick * 0.1);
    let poke = (t / 90) % 5 == 0;
    cv.put(fx - 1, fy, '▄', rgb((90, 60, 36)), None);
    cv.put(fx + 1, fy, '▄', rgb((90, 60, 36)), None);
    let flame = [(255, 240, 150), (255, 190, 70), (240, 110, 40), (200, 60, 30)];
    for (dy, wide) in [(0, 0), (1, 1), (2, 0)] {
        for dx in -wide..=wide {
            let hs = hash(dx + dy * 7, (t / 2) as i32 + dy);
            if dy == 2 && hs % 3 == 0 && !poke {
                continue;
            }
            let ch = ['▲', '♠', '*', '▲'][(hs % 4) as usize];
            cv.glyph(fx + dx, fy - dy, ch, rgb(flame[((hs / 4) % 2 + dy as u32) as usize % 4]));
        }
    }
    // Sparks and smoke going up.
    for k in 0..10 {
        let life = 30 + (hash(k, 3) % 30) as u64;
        let age = (t + hash(k, 4) as u64) % life;
        let y = fy - 2 - (age as i32 * (sky / 3).max(4)) / life as i32;
        let x = fx + ((age as f32 * 0.25 + k as f32).sin() * 1.5) as i32;
        let f = age as f32 / life as f32;
        cv.glyph(x, y, if f < 0.5 { '·' } else { '\'' }, rgb(lerp((255, 220, 120), (150, 50, 30), f)));
    }
    for k in 0..4 {
        let age = ((t / 4) as i32 + k * 5) % 20;
        let y = fy - 4 - age / 2;
        if y >= 0 {
            cv.glyph(fx + age / 3 + (k % 2), y, '░', mix((70, 66, 70), lerp(top, bot, 0.5), age as f32 / 20.0));
        }
    }

    // The traveller, their pack and blade, and a companion.
    let cloak = [(60, 110, 70), (120, 50, 50), (60, 80, 140), (140, 110, 60), (90, 70, 110)][(cv.roll(4) % 5) as usize];
    let ax = fx + 3;
    cv.glyph(ax, fy, '☺', rgb(cloak));
    cv.glyph(ax + 1, fy, '■', rgb((110, 80, 50)));
    cv.glyph(fx + 2, fy + 1, '▬', rgb(lerp(cloak, (0, 0, 0), 0.3)));
    cv.glyph(ax + 2, fy - 1, '┼', rgb((190, 190, 200)));
    cv.glyph(ax + 2, fy, '│', rgb((170, 170, 185)));
    if poke {
        cv.glyph(ax - 1, fy, '\\', rgb((120, 90, 60)));
    }
    match cv.roll(5) % 3 {
        0 => {
            cv.glyph(fx - 2, fy + 1, 'd', rgb((150, 120, 80)));
            cv.glyph(fx - 3, fy + 1, if (t / 10) % 2 == 0 { '~' } else { '-' }, rgb((150, 120, 80)));
        }
        1 => {
            cv.glyph(tx + 2, fy, 'h', rgb((130, 90, 60)));
            cv.glyph(tx + 3, fy, if (t / 14) % 3 == 0 { '~' } else { '\'' }, rgb((90, 60, 40)));
        }
        _ => {}
    }

    // Something in the dark beyond the firelight, watching.
    let watch = (t + cv.roll(6) as u64 % 500) % 500;
    if watch < 160 && (t / 13) % 9 != 0 {
        let ex = fx + 14 + (cv.roll(7) % 8) as i32;
        cv.glyph(ex, fy - 1 + (watch > 80) as i32, '"', rgb((240, 200, 60)));
    }

    // A rider passing on the road with a lantern.
    let ride = (t + cv.roll(8) as u64 % 900) % 900;
    if ride < 360 {
        let y = hz + (ride as i32 * (h - hz)) / 360;
        let (rc, _) = road(y);
        cv.halo(rc, y, 2.0, (255, 210, 120), 0.4);
        cv.glyph(rc, y, 'h', rgb((150, 110, 80)));
        cv.glyph(rc, y - 1, '☺', rgb((200, 200, 220)));
    }

    // Fireflies over the grass.
    for k in 0..(4 + w / 20) {
        if (t / 9 + hash(k, 8) as u64) % 5 == 0 {
            continue;
        }
        let x = ((hash(k, 9) as u64 + t / 11) % w.max(1) as u64) as i32;
        let y = hz + 1 + ((hash(k, 10) as i32 + ((tf * 0.05 + k as f32).sin() * 2.0) as i32).rem_euclid((sky - hz).max(1)));
        cv.glyph(x, y, '·', rgb((220, 255, 120)));
    }
}
