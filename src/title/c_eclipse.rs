//! The eclipse: the moon creeps across the sun at midday, the light going
//! strange and thin, the sky darkening to night with a sunset all round
//! the horizon, stars coming out, the corona flaring white about the black
//! disc, a diamond of light at the edge as the sun comes back. On the hill
//! a crowd lights torches against the dark while the birds go to roost.
//! The seed picks the land, the corona's streamers and where the crowd is.

use super::*;
use super::c_kit::*;

pub(super) const TAGLINES: &[&str] = &[
    "the sun goes out at noon, and every dog howls",
    "the old priests knew the day, and told no one",
    "a black sun, and a crown of white fire",
    "light the torches; it will come back. it will.",
];

/// Frames from one eclipse to the next.
const CYCLE: u64 = 1500;

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t, seed) = (cv.w(), cv.h(), cv.t, cv.seed);
    let tf = t as f32;
    let sky = cv.sky.min(h - 1).max(1);
    let g = (sky - 1).max(1);
    let salt = cv.salt();
    let p = phase(t, CYCLE, cv.roll(1) % CYCLE as u32);
    let r = (sky as f32 * 0.17).max(1.0).min(4.0);
    let off = (p - 0.5) * 6.0 * r; // the moon's offset from the sun, in round units
    let cover = (1.0 - off.abs() / (2.0 * r)).max(0.0);
    let dark = ease(0.55, 0.97, cover);
    let total = off.abs() < 0.12 * r;

    // The sky, going from noon to night and back, with a ring of sunset.
    let (day_t, day_b) = ((60, 120, 210), (170, 205, 240));
    let (ngt_t, ngt_b) = ((4, 6, 18), (30, 28, 50));
    for y in 0..g.min(h) {
        let v = y as f32 / g.max(1) as f32;
        let mut c = lerp(lerp(day_t, day_b, v), lerp(ngt_t, ngt_b, v), dark);
        c = lerp(c, (240, 130, 70), dark * v.powi(3) * 0.7);
        for x in 0..w {
            cv.fill(x, y, rgb(c));
        }
    }
    if dark > 0.5 {
        let every = (60.0 - dark * 45.0) as u32;
        cv.stars(g - 2, every.max(7));
    }

    // The sun, its corona, the moon over it.
    let (sx, sy) = ((w as f32 * 0.78) as i32, (g as f32 * 0.42) as i32);
    let sun = (255, 246, 214);
    cv.halo(sx, sy, r * 4.0, sun, 0.5 * (1.0 - dark));
    if dark > 0.75 {
        let cr = ((dark - 0.75) * 4.0).min(1.0);
        let k = (cv.roll(2) % 3) as f32 + 2.0;
        for y in sy - (r * 3.0) as i32..=sy + (r * 3.0) as i32 {
            for x in sx - (r * 6.0) as i32..=sx + (r * 6.0) as i32 {
                let (dx, dy) = ((x - sx) as f32 * 0.5, (y - sy) as f32);
                let d = (dx * dx + dy * dy).sqrt() / r;
                if d > 0.9 && d < 3.0 {
                    let a = dy.atan2(dx);
                    let streak = (a * k + 0.7).sin().abs().powf(3.0) * 0.7 + 0.3;
                    let f = cr * streak * ((3.0 - d) / 2.1).powi(2) * (0.9 + 0.1 * (tf * 0.2 + a * 7.0).sin());
                    cv.glow(x, y, (235, 240, 255), f.min(0.9));
                }
            }
        }
    }
    cv.disc(sx, sy, r, lerp(sun, (255, 200, 140), dark * 0.5));
    let mxx = sx + (off * 2.0).round() as i32;
    cv.disc(mxx, sy, r * 1.02, lerp((150, 170, 200), (8, 8, 14), (cover * 3.0).min(1.0)));
    // The diamond ring as the sun slips out from behind.
    if (0.82..0.98).contains(&cover) && !total {
        let side = if off > 0.0 { -1.0 } else { 1.0 };
        let (dx, dy) = (sx + (side * r * 2.0) as i32, sy);
        cv.halo(dx, dy, 2.5, (255, 255, 240), 0.8);
        cv.glyph(dx, dy, '*', rgb((255, 255, 255)));
    }

    // The land, dimming with the light.
    let land = |c: Rgb| lerp(c, lerp(c, (6, 8, 16), 0.85), dark);
    let far = lerp(day_b, (80, 100, 120), 0.5);
    for x in 0..w {
        let s = g - (peaks(x, w, seed, 8, 3) * sky as f32 * 0.35) as i32;
        for y in s.max(0)..g {
            cv.fill(x, y, rgb(land(far)));
        }
    }
    let hill = |x: i32| -> i32 { g - (ridge(x, w, seed, 18) * sky as f32 * 0.15) as i32 };
    for x in 0..w {
        let s = hill(x);
        for y in s.max(0)..h {
            let hs = hash(x + salt, y);
            let ch = if hs % 5 == 0 { '"' } else if hs % 31 == 0 { '♣' } else { ' ' };
            cv.put(x, y, ch, rgb(land((110, 160, 70))), Some(rgb(land((70, 116, 50)))));
        }
    }
    // Trees the birds go home to.
    let tx = (w as f32 * 0.9) as i32;
    for k in 0..3 {
        let x = tx + k * 2;
        let y = hill(x) - 1;
        cv.glyph(x, y, '♣', rgb(land((40, 100, 50))));
        cv.glyph(x, y - 1, '♣', rgb(land((50, 120, 60))));
    }
    for k in 0..(2 + w / 40) {
        let home = (dark * 1.2).min(1.0);
        let x0 = (hash(k, 4) % w.max(1) as u32) as i32;
        let y0 = 2 + (hash(k, 5) % (g.max(4) as u32 / 2)) as i32;
        let (x, y) = (x0 + ((tx - x0) as f32 * home) as i32, y0 + ((hill(tx) - 2 - y0) as f32 * home) as i32);
        if home < 0.98 {
            let wobble = ((tf * 0.3 + k as f32).sin() * 1.0) as i32;
            cv.glyph(x + wobble, y, if (t / 4 + k as u64) % 2 == 0 { 'v' } else { '^' }, rgb((30, 30, 40)));
        }
    }
    // The crowd on the hill: torches lit as the dark comes.
    let cx = (w as f32 * 0.15) as i32 + (cv.roll(3) % 8) as i32;
    for k in 0..(5 + cv.roll(4) % 4) as i32 {
        let x = cx + k * 2 - (k % 2);
        let y = hill(x) - 1 + (k % 2);
        cv.glyph(x, y, '☺', rgb(land([(200, 80, 70), (80, 110, 190), (210, 190, 90), (120, 170, 90)][(k % 4) as usize])));
        if k % 2 == 0 && dark > 0.45 + k as f32 * 0.04 {
            let fl = 0.3 + 0.1 * (tf * 0.7 + k as f32).sin();
            cv.halo(x + 1, y - 1, 2.0, (255, 170, 70), fl);
            cv.glyph(x + 1, y - 1, if (t / 3 + k as u64) % 2 == 0 { '*' } else { '¡' }, rgb((255, 200, 90)));
        } else if total || dark > 0.9 {
            cv.glyph(x, y - 1, '!', rgb((230, 230, 240)));
        }
    }
}
