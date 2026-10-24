//! A cave-in: miners driving a tunnel under a roof that has started to
//! talk. A crack runs up through the rock, grit trickles down, someone
//! shouts, and the roof comes in with a roar of boulders and a choking
//! cloud of dust. When it clears there is a dwarf on the wrong side of the
//! rubble, tapping, and the others dig like fury to reach them. The seed
//! picks the stone, where the roof gives, and who is caught.

use super::b_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "the roof was talking; nobody was listening",
    "shore it up, they said. tomorrow, we said",
    "tap twice if you can hear us",
    "every mountain settles in the end",
];

/// Stone, with the dust it makes.
const STONES: [(Rgb, Rgb); 4] = [
    ((86, 70, 58), (170, 150, 120)),  // sandstone-ish
    ((70, 70, 80), (160, 160, 165)),  // granite
    ((92, 60, 52), (180, 130, 110)),  // red marl
    ((58, 62, 54), (140, 150, 120)),  // greenstone
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t) = (cv.w(), cv.h(), cv.t);
    let g = (cv.sky - 1).max(2).min(h - 1); // the tunnel floor
    let th = ((g as f32 * 0.4) as i32).clamp(2, 7); // tunnel height
    let ty = (g - th).max(0); // first row of air
    let (stone, dust) = STONES[(cv.roll(1) % 4) as usize];
    let salt = cv.salt();
    let air = lerp(stone, (0, 0, 0), 0.66);

    // The rock, and the tunnel through it.
    bedrock(cv, 0, h, stone, salt);
    // A seam of coal or ore running across the face of it all.
    let seam = (cv.roll(2) % 3) as usize;
    let seam_col = [(30, 28, 30), (180, 120, 70), (200, 200, 215)][seam];
    let sy = (ty / 2).max(0);
    for x in 0..w {
        let y = sy + ((x as f32 * 0.07 + (salt % 50) as f32).sin() * 1.3) as i32;
        if hash(x + salt, 5) % 3 != 0 {
            cv.put(x, y, if seam == 0 { '▓' } else { '•' }, rgb(seam_col), None);
        }
    }

    // The beat: digging, the roof talking, the fall, the dust, the rescue.
    let period = 1300u64;
    let at = beat(cv, period, 3);
    let (crack_at, fall_at, settle_at, dig_at, free_at) = (260u64, 400u64, 450u64, 640u64, 1150u64);
    let cracking = span(at, crack_at, fall_at);
    let fallen = span(at, fall_at, settle_at); // how much is down
    let cleared = span(at, dig_at, free_at); // how much is dug out again
    let collapsed = at >= fall_at;

    // Where the roof gives way, and how wide.
    let hw = (w / 12).clamp(2, 9);
    let cx = (w as f32 * (0.34 + (cv.roll(4) % 20) as f32 / 100.0)) as i32;
    let entry = 1;
    let face = w - 3 - (cv.roll(5) % (w.max(20) as u32 / 10)) as i32;

    // The tunnel's air, warmed by lanterns, and a hole above the fall.
    let cavity = |x: i32| -> i32 {
        if !collapsed || (x - cx).abs() > hw {
            0
        } else {
            let u = (x - cx) as f32 / hw as f32;
            ((1.0 - u * u) * (th as f32 * 0.7 + 1.0)) as i32
        }
    };
    for x in entry..face.min(w) {
        let lamp = ((x % 14) as f32 - 7.0).abs() / 7.0;
        let bg = lerp(air, (80, 54, 30), (1.0 - lamp) * 0.35);
        for y in (ty - cavity(x)).max(0)..g {
            cv.fill(x, y, rgb(bg));
        }
    }
    floor(cv, 0, face + 1, g, lerp(stone, (150, 140, 120), 0.3));
    // Timber sets along the tunnel; the one under the fall is gone.
    let mut px = entry + 5 + (cv.roll(6) % 4) as i32;
    while px < face - 2 {
        let broken = collapsed && (px - cx).abs() <= hw;
        if !broken {
            cv.glyph(px, ty, '╥', rgb((150, 105, 60)));
            for y in ty + 1..g {
                cv.glyph(px, y, '║', rgb((125, 88, 50)));
            }
        } else if cleared >= 1.0 {
            // New timber, pale and fresh.
            cv.glyph(px, ty, '╥', rgb((210, 170, 110)));
            for y in ty + 1..g {
                cv.glyph(px, y, '║', rgb((190, 150, 95)));
            }
        }
        px += 9;
    }
    // The way in: a stair down from above.
    for y in 0..g {
        cv.put(entry - 1, y, '║', rgb((110, 80, 50)), Some(rgb(air)));
        cv.put(entry, y, if y % 2 == 0 { '═' } else { ' ' }, rgb((110, 80, 50)), Some(rgb(air)));
    }
    let lamps: Vec<i32> = (0..).map(|k| entry + 4 + k * 14).take_while(|&x| x < face - 1).collect();
    for (k, &lx) in lamps.iter().enumerate() {
        if !(collapsed && (lx - cx).abs() <= hw && cleared < 1.0) {
            torch(cv, lx, ty.max(0), k as i32, 3.0);
        }
    }

    // The crack climbing through the roof before it goes.
    if cracking > 0.0 && !collapsed {
        let reach = (cracking * (ty as f32 + 1.0)) as i32;
        let mut x = cx;
        for k in 0..reach {
            let y = ty - 1 - k;
            x += [-1, 0, 1][(hash(k + salt, 9) % 3) as usize];
            cv.glyph(x, y, if hash(k, 3) % 2 == 0 { '╱' } else { '╲' }, rgb((20, 16, 14)));
        }
        // Grit trickling down.
        for k in 0..((cracking * 6.0) as i32 + 1) {
            let fx = cx + (hash(k, salt) % (2 * hw.max(1) as u32)) as i32 - hw;
            let fy = ty + ((t / 2 + k as u64 * 3) % th.max(1) as u64) as i32;
            cv.glyph(fx, fy, '·', rgb(dust));
        }
    }

    // The rubble heap, filling the tunnel, dug away from the near side.
    let dug_to = cx - hw - 1 + (cleared * (2 * hw + 3) as f32) as i32;
    if collapsed {
        let fill = fallen;
        for x in cx - hw - 1..=cx + hw + 1 {
            if cleared > 0.0 && x < dug_to {
                continue;
            }
            let u = (x - cx) as f32 / (hw + 1) as f32;
            let top = ((1.0 - u * u).max(0.0) * (th + cavity(x)) as f32 * fill).round() as i32;
            for k in 0..top {
                let y = g - 1 - k;
                let hs = hash(x * 5 + salt, y * 3);
                let ch = ['▓', '▒', '●', '▓', '░', '•'][(hs % 6) as usize];
                cv.put(x, y, ch, rgb(lerp(stone, dust, 0.35)), Some(rgb(lerp(stone, (0, 0, 0), 0.3))));
            }
        }
    }
    // Boulders tumbling down as it goes.
    if at >= fall_at && at < settle_at + 20 {
        let f = (at - fall_at) as f32;
        for k in 0..(hw * 2) {
            let start = (hash(k, salt + 1) % 30) as f32;
            if f < start {
                continue;
            }
            let x = cx - hw + (hash(k, salt + 2) % (2 * hw as u32 + 1)) as i32;
            let y = ty - 2 + ((f - start) * 0.35) as i32;
            if y < g - 1 {
                cv.glyph(x, y, if k % 3 == 0 { '●' } else { '•' }, rgb(lerp(stone, dust, 0.5)));
            }
        }
    }

    // The dust cloud, rolling out along the tunnel and settling.
    if collapsed && at < dig_at + 300 {
        let since = (at - fall_at) as f32;
        let r = (2.0 + since * 0.25).min(w as f32 * 0.3);
        let dens = (1.0 - since / 540.0).max(0.0);
        for y in (ty - 2).max(0)..g {
            for x in (cx - (r * 2.0) as i32).max(0)..(cx + (r * 2.0) as i32).min(w) {
                let dx = (x - cx) as f32 / (r * 2.0);
                let dy = (y - (ty + g) / 2) as f32 / (r * 0.5).max(th as f32);
                let d = (dx * dx + dy * dy).sqrt();
                if d >= 1.0 {
                    continue;
                }
                let swirl = (x as f32 * 0.4 + y as f32 * 0.9 - t as f32 * 0.07).sin() * 0.5 + 0.5;
                let f = dens * (1.0 - d) * (0.6 + swirl * 0.4);
                cv.glow(x, y, dust, f * 0.85);
                if f > 0.3 && hash(x + (t / 6) as i32, y) % 4 == 0 {
                    cv.glyph(x, y, if f > 0.55 { '▒' } else { '░' }, rgb(lerp(dust, (255, 255, 255), 0.2)));
                }
            }
        }
    }

    // The miners.
    let swing = (t / 6) % 2 == 0;
    let picks = rgb((170, 170, 180));
    let caught = cv.roll(7) % 4 != 0; // someone on the far side?
    let b0 = beard(cv.roll(8));
    let b1 = beard(cv.roll(8) + 1);
    let b2 = beard(cv.roll(8) + 2);
    // At the face, digging on (or caught behind the fall, tapping).
    let far = cx + hw + 2;
    if !collapsed || !caught || cleared >= 1.0 {
        let fx = if collapsed && caught { far + 1 + ((at - free_at.min(at)) / 4) as i32 } else { face - 1 };
        let fx = fx.min(face - 1);
        cv.glyph(fx, g - 1, '☺', rgb(b0));
        cv.glyph(fx + 1, g - 2, if swing { '/' } else { '─' }, picks);
        if cleared >= 1.0 && caught {
            cv.glyph(fx, g - 2, '♥', rgb((255, 110, 120)));
        }
    } else if collapsed {
        let x = (face - 2).min(far + 2).max(far);
        cv.glyph(x, g - 1, '☺', rgb(b0));
        if (at / 25) % 3 == 0 && at >= settle_at + 60 {
            cv.glyph(x - 1, g - 2, '♪', rgb((255, 240, 160)));
        }
    }
    // Near the fall: looks up, shouts, runs, then digs.
    let near0 = cx - hw - 3;
    if at < crack_at {
        let wx = pingpong(t, 5, entry + 3, (near0 + hw).max(entry + 4));
        cv.sprite(wx, g - 1, &["▐█▌"], rgb((120, 86, 50)));
        cv.glyph(wx + 1, g - 2, '▲', rgb(lerp(stone, dust, 0.4)));
        cv.glyph(wx - 1, g - 1, '☺', rgb(b1));
    } else if !collapsed {
        cv.glyph(cx - 1, g - 1, '☺', rgb(b1));
        if cracking > 0.4 {
            cv.glyph(cx - 1, g - 2, '!', rgb((255, 240, 160)));
        }
    } else if at < dig_at {
        let run = ((at - fall_at) / 2) as i32;
        let x = (cx - hw - 2 - run).max(entry + 2);
        cv.glyph(x, g - 1, '☺', rgb(b1));
        if at < settle_at + 40 {
            cv.glyph(x, g - 2, '‼', rgb((255, 240, 160)));
        }
    } else {
        let x = (dug_to - 1).min(cx + hw);
        cv.glyph(x, g - 1, '☺', rgb(b1));
        cv.glyph(x + 1, g - 2, if swing { '/' } else { '─' }, picks);
        if cleared < 1.0 {
            let hx = pingpong(t, 4, entry + 3, (x - 3).max(entry + 4));
            cv.glyph(hx, g - 1, '☺', rgb(b2));
            cv.sprite(hx + 1, g - 1, &["▐▌"], rgb((120, 86, 50)));
        }
    }

    // Down below, an old gallery: water seeping, a pump turning.
    let b = below(cv);
    let low = ((b + h) / 2 + 2).min(h - 2);
    if h - b >= 5 {
        let ly = (low - 4).max(b);
        let deep = lerp(stone, (0, 0, 0), 0.85);
        hollow(cv, 2, ly, w - 2, low, deep);
        floor(cv, 2, w - 2, low, lerp(stone, (120, 110, 100), 0.3));
        for x in 2..w - 2 {
            if (x + (t / 9) as i32) % 31 < 6 {
                cv.put(x, low - 1, if (x + (t / 5) as i32) % 3 == 0 { '~' } else { '≈' }, rgb((90, 130, 180)), Some(rgb((16, 30, 50))));
            }
        }
        for (k, lx) in [w / 6, w / 2, w * 5 / 6].into_iter().enumerate() {
            torch(cv, lx, ly, k as i32 + 7, 2.5);
        }
        let px = w / 3;
        let spoke = ['|', '/', '─', '\\'][((t / 5) % 4) as usize];
        cv.glyph(px, ly + 1, spoke, rgb((170, 140, 90)));
        cv.glyph(px - 1, low - 1, '☺', rgb(beard(cv.roll(9))));
        // Drips from the roof.
        for k in 0..4 {
            let dx = (hash(k, salt + 3) % (w.max(8) as u32 - 4)) as i32 + 2;
            let dy = ly + ((t / 3 + k as u64 * 7) % 4) as i32;
            cv.glyph(dx, dy, '\'', rgb((120, 170, 220)));
        }
    }
}
