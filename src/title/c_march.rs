//! An army on the march across the open plain: rank on rank of spears
//! under fluttering standards, a captain riding at the head, drummers
//! beating the step, the baggage carts rolling behind and a long cloud of
//! dust hanging in their wake, while the wind runs through the grass in
//! waves. The seed picks the hour, the colours, the arms and how many.

use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "ten thousand boots, one road, one drum",
    "the war will be decided by who brought more bread",
    "banners up; somewhere ahead, the enemy",
    "the kingdom marches, and the plain remembers",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let tf = t as f32;
    let sky = cv.sky.min(h - 1).max(1);
    let hz = (sky / 2).max(1).min(h - 1); // the far edge of the plain
    let hour = cv.roll(1) % 3; // morning, noon, evening
    let (top, bot) = [((90, 140, 210), (230, 210, 180)), ((60, 120, 210), (170, 205, 240)), ((70, 50, 110), (245, 150, 90))][hour as usize];
    cv.sky_fill(hz, top, bot);
    let sun = [(255, 240, 200), (255, 255, 235), (255, 180, 100)][hour as usize];
    let (sx, sy) = ((w as f32 * [0.85, 0.7, 0.12][hour as usize]) as i32, (hz as f32 * [0.5, 0.25, 0.7][hour as usize]) as i32);
    cv.halo(sx, sy, 5.0, sun, 0.35);
    cv.disc(sx, sy, (hz as f32 * 0.1).max(0.8).min(2.0), sun);
    // Clouds sailing.
    for k in 0..(2 + w / 40) {
        let len = 6 + (hash(k, 19) % 9) as i32;
        let y = 1 + (hash(k, 13) % (hz.max(3) as u32 - 2).max(1)) as i32;
        let x0 = ((hash(k, 15) as u64 + t / (14 + k as u64 * 5)) % (w as u64 + len as u64 + 10)) as i32 - len - 5;
        for i in 0..len {
            cv.glyph(x0 + i, y, if i == 0 || i == len - 1 { '▄' } else { '█' }, rgb(lerp(bot, (255, 255, 255), 0.6)));
        }
    }
    // Hills on the horizon.
    let far = lerp(bot, (70, 90, 100), 0.55);
    for x in 0..w {
        let s = hz - (ridge(x, w, seed, 5) * hz as f32 * 0.3) as i32;
        for y in s.max(0)..hz {
            cv.fill(x, y, rgb(far));
        }
    }

    // The plain, the wind running through the grass in waves.
    let grass = [(110, 150, 60), (120, 160, 70), (150, 130, 60)][hour as usize];
    let salt = cv.salt();
    let (road_y, ranks) = {
        let r = 2 + (cv.roll(2) % 2) as i32;
        let r = r.min(((sky - hz - 2) / 2).max(1));
        ((sky - 1 - r * 2).max(hz + 1), r)
    };
    for y in hz.max(0)..h {
        let d = (y - hz) as f32 / (h - hz).max(1) as f32;
        for x in 0..w {
            let gust = (x as f32 * 0.09 - tf * 0.12 + y as f32 * 0.3).sin();
            let base = lerp(lerp(grass, far, (1.0 - d) * 0.4), (40, 60, 20), d * 0.4);
            let bg = if gust > 0.7 { lerp(base, (230, 230, 160), 0.18) } else { base };
            let on_road = y >= road_y - 1 && y <= road_y + ranks * 2 - 1;
            let hs = hash(x + salt, y * 5);
            let (ch, fg, bg) = if on_road {
                (if hs % 6 == 0 { '·' } else { ' ' }, (140, 120, 90), lerp((150, 130, 96), bg, 0.3))
            } else {
                (if hs % 3 == 0 { if gust > 0.4 { '\'' } else { '"' } } else { ' ' }, lerp(base, (230, 240, 150), 0.35), bg)
            };
            cv.put(x, y, ch, rgb(fg), Some(rgb(bg)));
        }
    }

    // The host: carts at the back, then ranks of spears, the standards,
    // the drummers, and the captain riding ahead.
    let steel = [(200, 205, 215), (190, 140, 70), (70, 70, 80), (220, 220, 190)][(cv.roll(3) % 4) as usize];
    let banners = [(180, 30, 40), (30, 60, 160), (220, 180, 40), (30, 120, 60), (110, 40, 140)];
    let (b1, b2) = (banners[(cv.roll(4) % 5) as usize], banners[(cv.roll(5) % 5) as usize]);
    let files = ((w as f32 * 0.35) as i32).max(8);
    let length = files + 16;
    let head = ((t / 5 + cv.roll(6) as u64) % (w + length + 10) as u64) as i32 - 5;
    let step = (t / 5) % 2 == 0;
    for r in 0..ranks {
        let y = road_y + r * 2 + 1;
        for f in 0..files {
            let x = head - 6 - f * 1 - r;
            if f % 9 == 4 {
                // A standard, rippling.
                for i in 0..3 {
                    let ripple = (i as f32 * 1.3 - tf * 0.3 + f as f32).sin();
                    let ch = if ripple > 0.4 { '▀' } else if ripple < -0.4 { '▄' } else { '█' };
                    cv.glyph(x - i, y - 2, ch, rgb(if (f / 9) % 2 == 0 { b1 } else { b2 }));
                }
                cv.glyph(x, y - 1, '│', rgb((120, 90, 60)));
            } else {
                let tip = if (t / 3 + (f * 7 + r * 3) as u64) % 53 == 0 { '*' } else { '↑' };
                cv.glyph(x, y - 1, tip, rgb(if tip == '*' { (255, 255, 240) } else { steel }));
            }
            let bob = (f + r) % 2 == 0;
            cv.glyph(x, y, if bob == step { '☺' } else { '☻' }, rgb(steel));
        }
    }
    // Drummers at the front of the column.
    let dy = road_y + 1;
    for k in 0..2 {
        let x = head - 4 + k;
        cv.glyph(x, dy + k * 2, '☺', rgb(b1));
        if (t / 8 + k as u64) % 2 == 0 {
            cv.glyph(x, dy + k * 2 - 1, '♪', rgb((255, 240, 180)));
        }
    }
    // The captain on horseback, ahead, a pennon on the lance.
    let cx = head;
    cv.glyph(cx, road_y + 1, 'H', rgb((110, 70, 40)));
    cv.glyph(cx + 1, road_y + 1, '\'', rgb((110, 70, 40)));
    cv.glyph(cx, road_y, '☺', rgb(b2));
    cv.glyph(cx + 1, road_y - 1, '/', rgb((150, 120, 80)));
    cv.glyph(cx + 2, road_y - 2, if (t / 4) % 2 == 0 { '►' } else { '▶' }, rgb(b1));
    // The carts behind, and the dust they raise.
    let tail = head - 8 - files - ranks;
    for k in 0..2 {
        let x = tail - k * 6;
        let y = road_y + 1 + (k % ranks.max(1)) * 2;
        cv.glyph(x, y, 'Ö', rgb((140, 110, 80)));
        cv.glyph(x - 1, y, '█', rgb((220, 210, 180)));
        cv.glyph(x - 2, y, '█', rgb((220, 210, 180)));
        cv.glyph(x - 1, y + 1, 'o', rgb((90, 70, 50)));
    }
    for k in 0..(files / 2 + 10) {
        let age = ((t / 3) as i32 + k * 7) % 30;
        let x = tail - 4 - (hash(k, 3) % (files as u32 / 2 + 6)) as i32 + age / 3;
        let y = road_y + ranks * 2 - 1 - age / 6 - (hash(k, 4) % 2) as i32;
        let f = age as f32 / 30.0;
        cv.glyph(x, y, if f < 0.5 { '░' } else { '·' }, mix((200, 180, 140), lerp(grass, bot, 0.5), f));
    }

    // Crows following the host.
    for k in 0..(1 + w / 50) {
        let x = head - files / 2 + ((tf * 0.04 + k as f32 * 2.0).cos() * files as f32 * 0.4) as i32;
        let y = (road_y - 5 - k % 2).max(1) + ((tf * 0.06 + k as f32).sin() * 1.2) as i32;
        cv.glyph(x, y, if (t / 6 + k as u64) % 2 == 0 { 'v' } else { '^' }, rgb((30, 30, 36)));
    }
}
