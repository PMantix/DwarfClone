//! A goblin horde marches over the plain on the fortress gate. Archers
//! loose from the battlements, arrows arc over the moat, and the drawbridge
//! comes down now and then for a sally. The seed sets the hour, the rain,
//! the colours on the tower and what walks among the goblins.

use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the drums came over the hills at dusk",
    "raise the bridge and man the walls",
    "they want our gold; let them come and ask for it",
    "every arrow fletched in winter flies in spring",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, sky, seed) = (cv.w(), cv.h(), cv.t, cv.sky, cv.seed);
    let g = sky - 1; // the plain's edge, where the horde walks
    let hour = cv.roll(1) % 3;
    let (top, bot) = match hour {
        0 => ((24, 18, 60), (170, 80, 55)),
        1 => ((6, 8, 26), (40, 26, 54)),
        _ => ((36, 38, 52), (96, 92, 100)),
    };
    let rain = cv.roll(2) % 3 == 0;
    cv.sky_fill(g, top, bot);
    if hour == 1 {
        cv.stars(g - 2, 47);
        let mx = w / 9 + (cv.roll(3) % 5) as i32;
        cv.glyph(mx, 1, '●', rgb((240, 235, 205)));
    } else if hour == 0 {
        // The sun going down behind the hills, and smoke from burned farms.
        let sx = w / 3 + (cv.roll(3) % (w.max(3) as u32 / 3)) as i32;
        for dx in -2..=2 {
            cv.fill(sx + dx, g - 2, rgb((240, 150, 70)));
        }
    }

    // Far hills.
    let hill = lerp(bot, (10, 10, 14), 0.65);
    for x in 0..w {
        let hh = 1 + (ridge(x, w, seed, 20) * (sky as f32 / 4.0).max(3.0)) as i32;
        for y in (g - hh).max(0)..g {
            cv.fill(x, y, rgb(hill));
        }
    }
    // Smoke from farms the horde has already burned.
    let fires = 1 + (w / 70) as u32 + cv.roll(7) % 2;
    for k in 0..fires {
        let fx = (w / 8) + (roll(seed, 30 + k) % (w.max(16) as u32 / 2)) as i32;
        let base = g - 1 - (ridge(fx, w, seed, 20) * (sky as f32 / 4.0).max(3.0)) as i32;
        let tall = (sky / 2).max(4);
        cv.glyph(fx, base, '▲', rgb([(255, 170, 60), (240, 90, 30)][((t / 5 + k as u64) % 2) as usize]));
        for j in 0..tall {
            let age = ((t / 6) as i32 + j * 7) % (tall * 2);
            let y = base - 1 - age / 2;
            let x = fx + age / 5 + (((age + k as i32) as f32 * 0.7).sin() * 0.8) as i32;
            if y >= 0 {
                let fade = age as f32 / (tall * 2) as f32;
                cv.glyph(x, y, if age < tall { '░' } else { '·' }, mix((90, 84, 88), lerp(bot, (40, 40, 50), 0.3), fade));
            }
        }
    }
    // Crows waiting on the outcome.
    for k in 0..(w / 40 + 1) {
        let a = t as f32 * 0.04 + k as f32 * 2.4;
        let cx = w / 3 + (hash(k, 3 + seed as i32) % (w.max(3) as u32 / 3)) as i32;
        let x = cx + (a.cos() * (6.0 + k as f32 * 2.0)) as i32;
        let y = 2 + (sky / 4) + (a.sin() * 2.0) as i32;
        cv.glyph(x, y, if (t / 4 + k as u64) % 2 == 0 { 'v' } else { '^' }, rgb((20, 18, 24)));
    }

    // The fortress wall stands on the right, the moat before it.
    let wx = w - (w / 6).max(9).min(w / 2);
    let wall_top = (g - (sky / 3).max(4)).max(0);
    let plain_end = wx - 3;

    // The plain, and the trampled road along its edge.
    for y in g.max(0)..h {
        let depth = (y - g) as f32 / (h - g).max(1) as f32;
        let bg = mix((46, 54, 28), (28, 34, 18), depth);
        for x in 0..plain_end.max(0) {
            let hs = hash(x * 5 + 1, y * 7 + 3);
            let (ch, fg) = match hs % 21 {
                0 => ('"', (100, 120, 55)),
                1 => (',', (85, 100, 48)),
                2 => ('.', (110, 95, 62)),
                _ => (' ', (0, 0, 0)),
            };
            cv.put(x, y, ch, rgb(fg), Some(bg));
        }
    }
    for x in 0..plain_end.max(0) {
        let ch = if hash(x, 5) % 3 == 0 { '·' } else { ' ' };
        cv.put(x, g, ch, rgb((125, 105, 75)), Some(rgb((72, 58, 40))));
    }

    // The moat.
    for y in g.max(0)..h {
        for x in plain_end..wx {
            let wave = ((x * 3 + y * 5) as f32 * 0.7 + t as f32 * 0.12).sin() * 0.5 + 0.5;
            let ch = if (x + y + (t / 10) as i32) % 3 == 0 { '~' } else { '≈' };
            cv.put(x, y, ch, mix((60, 90, 150), (120, 160, 210), wave), Some(mix((14, 30, 62), (24, 46, 86), wave)));
        }
    }

    // The wall and its gatehouse tower.
    let stone = |x: i32, y: i32| mix((70, 66, 62), (92, 86, 80), (hash(x * 3, y * 11) % 100) as f32 / 300.0);
    for y in wall_top.max(0)..h {
        for x in wx.max(0)..w {
            let course = (y - wall_top).rem_euclid(2) == 0;
            let seam = (x - wx + if course { 0 } else { 2 }).rem_euclid(4) == 0;
            let ch = if x == wx { '▌' } else if seam { '│' } else { ' ' };
            cv.put(x, y, ch, rgb((52, 48, 46)), Some(stone(x, y)));
        }
    }
    for x in wx..w {
        if (x - wx) % 3 != 2 {
            cv.put(x, wall_top - 1, '▄', rgb((84, 78, 72)), None);
        }
    }
    let (tx0, tx1) = (wx, (wx + 3).min(w - 1));
    for y in (wall_top - 3).max(0)..wall_top {
        for x in tx0..=tx1 {
            cv.put(x, y, if x == tx0 { '▌' } else { ' ' }, rgb((52, 48, 46)), Some(stone(x, y)));
        }
    }
    for x in tx0..=tx1 {
        if (x - tx0) % 2 == 0 {
            cv.glyph(x, wall_top - 4, '▄', rgb((84, 78, 72)));
        }
    }
    let banner = [(200, 40, 40), (60, 120, 220), (230, 190, 60), (60, 170, 90)][(cv.roll(4) % 4) as usize];
    cv.glyph(tx0 + 1, wall_top - 5, '│', rgb((150, 140, 130)));
    let flap = if (t / 8) % 2 == 0 { '▶' } else { '►' };
    cv.glyph(tx0 + 2, wall_top - 6, flap, rgb(banner));
    cv.glyph(tx0 + 1, wall_top - 6, '▀', rgb(banner));

    // The gate and its drawbridge: up, coming down, down, going up.
    let phase = (t + (cv.roll(5) % 1200) as u64) % 1200;
    let state = match phase {
        0..=699 => 0,
        700..=739 | 1160..=1199 => 1,
        _ => 2,
    };
    let dark = rgb((12, 8, 6));
    for y in (g - 2).max(0)..=g {
        for x in wx..(wx + 2).min(w) {
            cv.fill(x, y, dark);
        }
    }
    cv.glyph(wx, g - 3, '▄', rgb((60, 56, 52)));
    let plank = rgb((140, 95, 50));
    match state {
        0 => {
            for y in (g - 2)..=g {
                cv.put(wx - 1, y, '▓', plank, None);
            }
        }
        1 => {
            for i in 1..=3 {
                cv.glyph(wx - i, g + 1 - i, '\\', plank);
            }
        }
        _ => {
            for x in plain_end..wx {
                cv.put(x, g, '═', plank, Some(rgb((70, 46, 24))));
            }
            // A sally: two axedwarves stride out to the bridge's end.
            for (i, c) in [(0, (200, 220, 240)), (1, (240, 200, 120))] {
                let x = pingpong(phase - 740 + i * 40, 12, plain_end - 1, wx + 1);
                cv.glyph(x, g - 1, '☺', rgb(c));
            }
        }
    }

    // The horde: ranks marching right, falling to the arrows at the moat.
    let span = (plain_end - 1).max(1) as u64;
    let kinds = cv.roll(6);
    let salt = cv.salt();
    let goblin = |i: i32| -> (char, Rgb) {
        let hs = hash(i, 17);
        if hs % 23 == 0 && kinds & 1 == 1 {
            ('T', (150, 160, 120))
        } else if hs % 17 == 0 && kinds & 2 == 2 {
            ('O', (170, 120, 90))
        } else if hs % 9 == 0 {
            ('G', (120, 200, 70))
        } else {
            ('g', (90, 170, 60))
        }
    };
    let mut ranks = vec![(g, 7u64, salt % 53)];
    let mut r = g + 2;
    let mut k = 1;
    while r < h - 1 {
        ranks.push((r, 7 + (k % 3) as u64 * 2, k * 37 + salt % 101));
        r += 3;
        k += 1;
    }
    for (row, every, off) in ranks {
        let gap = if row == g { 2 } else { 3 + (hash(row, 3) % 3) as i32 };
        let n = (span as i32 / gap).max(1);
        for i in 0..n {
            if row != g && hash(i, row) % 4 == 0 {
                continue;
            }
            let x = ((i * gap + off) as u64 + t / every) % span;
            let x = x as i32;
            let (ch, col) = goblin(i + off);
            if x >= span as i32 - 2 {
                cv.glyph(x, row, '%', rgb((150, 60, 50)));
            } else {
                cv.glyph(x, row, ch, rgb(col));
                if row == g && (i + off) % 7 == 3 {
                    cv.glyph(x, row - 1, '¶', rgb((190, 40, 30)));
                }
            }
        }
    }

    // A war camp at the far end of the plain.
    if h - g > 10 && plain_end > 20 {
        let n = (plain_end / 4 / 7).max(1);
        for k in 0..n {
            let x = 2 + k * 7 + (hash(k, 8) % 3) as i32;
            let y = h - 3 - (hash(k, 9) % 2) as i32;
            cv.sprite(x, y, &[" ▲ ", "/█\\"], rgb((120, 95, 70)));
            let fire = [(255, 220, 80), (255, 140, 30), (220, 60, 20)][(hash(k, (t / 4) as i32) % 3) as usize];
            cv.glyph(x + 4, y + 1, '☼', rgb(fire));
        }
    }

    // Archers on the wall-walk, and their arrows.
    let night = hour == 1;
    let mut k = 0;
    let mut ax = wx + 2;
    while ax < w && wx > 10 {
        if (ax - wx) % 3 == 2 {
            let shoot = 45 + (k * 7) % 25;
            let phase = (t + k as u64 * 17) % shoot as u64;
            let col = if phase < 5 { (255, 255, 200) } else { [(200, 220, 240), (240, 200, 120), (210, 170, 230)][(k % 3) as usize] };
            cv.glyph(ax, wall_top - 1, '☺', rgb(col));
            let volley = (t + k as u64 * 17) / shoot as u64;
            let target = 2 + (hash(k, volley as i32) % (plain_end - 6).max(1) as u32) as i32;
            let s = phase as f32 / shoot as f32;
            let (x0, y0, x1, y1) = (ax as f32, (wall_top - 1) as f32, target as f32, g as f32);
            let lift = 2.0 + (k % 3) as f32 + (x0 - x1) / 30.0;
            let x = x0 + (x1 - x0) * s;
            let y = y0 + (y1 - y0) * s - lift * 4.0 * s * (1.0 - s);
            let dy = (y1 - y0) - lift * 4.0 * (1.0 - 2.0 * s);
            let slope = dy / (x1 - x0).abs().max(1.0);
            let ch = if slope < -0.4 {
                '\\'
            } else if slope > 0.4 {
                '/'
            } else {
                '─'
            };
            let col = if night { (255, 170, 70) } else { (230, 220, 190) };
            if s > 0.96 {
                cv.glyph(target, g - 1, '*', rgb((255, 240, 160)));
            } else if s > 0.05 {
                cv.glyph(x.round() as i32, y.round() as i32, ch, rgb(col));
                if night {
                    cv.glyph(x.round() as i32 + 1, (y - dy * 0.03).round() as i32, '·', rgb((200, 90, 30)));
                }
            }
            k += 1;
        }
        ax += 1;
    }

    // Rain slanting over everything.
    if rain {
        for k in 0..(w / 2).max(3) {
            let y = ((hash(k, 1) as u64 + t) % h.max(1) as u64) as i32;
            let x = ((hash(k, 2) as u64 + t / 2) % w.max(1) as u64) as i32 - y / 2;
            cv.glyph(x.rem_euclid(w.max(1)), y, '/', rgb((110, 120, 155)));
        }
    }
}
