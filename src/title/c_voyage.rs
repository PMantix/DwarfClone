//! A night crossing: a three-masted ship riding the long swell under a sky
//! thick with stars and the river of the galaxy, the moon laying a road
//! of light over the water, lanterns lit at her stern and her wake
//! glowing green. Stars fall now and then; far off a whale comes up to
//! blow. The seed picks the sky, the moon, the sails and the course.

use super::*;
use super::c_kit::*;

pub(super) const TAGLINES: &[&str] = &[
    "steer by the stars; they have never lied",
    "the map ends here, so the voyage begins",
    "a thousand leagues of black water, and a song",
    "somewhere past the edge, an undiscovered coast",
];

// Facing right. S sail, F pennant, M mast, r rail, H hull, w lit window,
// L stern lantern, b bowsprit, others as they are.
const SMALL: [&str; 6] = ["  F   F    ", " SSS SSS   ", " SSS SSS   ", "  M   M  b ", "LrrrrrrrrrQ", " HHwHHwHHH "];
const BIG: [&str; 9] = [
    "     F     F     F       ",
    "    SSS   SSS   SSS      ",
    "   SSSSS SSSSS SSSSS     ",
    "   SSSSS SSSSS SSSSS     ",
    "    SSS   SSS   SSS      ",
    "     M     M     M   bb  ",
    "LrrrrrrrrrrrrrrrrrrrrrrQ ",
    " HHHwHHHHwHHHHwHHHHwHHH  ",
    "   ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀   ",
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t) = (cv.w(), cv.h(), cv.t);
    let tf = t as f32;
    let sky = cv.sky.min(h - 1).max(1);
    let hz = ((sky * 3 / 4).max(1)).min(h - 1); // the horizon
    let salt = cv.salt();
    let tint = cv.roll(1) % 3;
    let (top, bot) = [((4, 6, 20), (20, 28, 60)), ((10, 4, 24), (40, 22, 60)), ((2, 10, 22), (14, 40, 60))][tint as usize];
    cv.sky_fill(hz, top, bot);

    // The galaxy's river across the sky, and the stars.
    let (b0, slope) = (hz as f32 * (0.2 + (cv.roll(2) % 50) as f32 / 100.0), (if cv.roll(3) % 2 == 0 { 0.12 } else { -0.12 }) * (hz as f32 / w.max(1) as f32) * 4.0);
    for y in 0..hz.min(h) {
        for x in 0..w {
            let d = ((y as f32 - (b0 + slope * x as f32)) / (hz as f32 * 0.22).max(1.0)).abs();
            let n = fbm(x as f32 * 0.08, y as f32 * 0.16, salt, 3);
            let band = ((1.0 - d).max(0.0) * (0.4 + n)).min(1.0);
            if band > 0.05 {
                cv.glow(x, y, (90, 80, 130), band * 0.35);
            }
            let hs = hash(x * 7 + salt, y * 11);
            if hs % (if band > 0.3 { 5 } else { 23 }) == 0 {
                let twinkle = (t / 6 + hs as u64 / 31) % 29 == 0;
                let col = [(150, 150, 190), (190, 180, 150), (160, 190, 220)][(hs % 3) as usize];
                cv.glyph(x, y, if twinkle { '+' } else { '·' }, rgb(if twinkle { (255, 255, 235) } else { col }));
            }
        }
    }
    // Falling stars.
    let fall = (t + cv.roll(4) as u64 % 160) % 160;
    if fall < 14 {
        let k = ((t + cv.roll(4) as u64 % 160) / 160) as i32;
        let (x0, y0) = ((hash(k, 1) % w.max(1) as u32) as i32, (hash(k, 2) % (hz.max(2) as u32 / 2 + 1)) as i32);
        for j in 0..5 {
            let s = fall as i32 - j;
            if s >= 0 {
                cv.glyph(x0 + s * 2, y0 + s / 2, if j == 0 { '*' } else { '·' }, mix((255, 255, 230), top, j as f32 / 5.0));
            }
        }
    }

    // The moon and the road it lays on the water.
    let moon = (236, 232, 210);
    let (mx, my) = ((w as f32 * (0.18 + (cv.roll(5) % 20) as f32 / 100.0)) as i32, (hz as f32 * 0.35) as i32);
    let mr = (hz as f32 * 0.1).max(0.8).min(2.5);
    cv.halo(mx, my, mr * 4.0, moon, 0.3);
    cv.disc(mx, my, mr, moon);

    // The sea.
    let (sea_hi, sea_lo) = (lerp(bot, (30, 50, 80), 0.5), (4, 10, 24));
    for y in hz.max(0)..h {
        let d = ((y - hz) as f32 / (h - hz).max(1) as f32).min(1.0);
        let base = lerp(sea_hi, sea_lo, d.sqrt());
        for x in 0..w {
            let (fx, fy) = (x as f32, y as f32);
            let ph = (fx * 0.2 / (1.0 + d * 2.0) + fy * 1.3 - tf * 0.06 + (fx * 0.04).sin() * 2.0).sin();
            let (mut ch, mut fg) = (' ', lerp(base, (200, 210, 240), 0.35));
            if ph > 0.8 {
                ch = '~';
            }
            let spread = 1 + (y - hz) * 2 / 3;
            if (x - mx).abs() < spread && hash(x, y + (t / 4) as i32) % 3 == 0 {
                ch = if hash(x, y) % 2 == 0 { '─' } else { '═' };
                fg = lerp(moon, base, 0.25 + d * 0.4);
            }
            cv.put(x, y, ch, rgb(fg), Some(rgb(base)));
        }
    }

    // A whale far out, coming up to blow.
    let blow = (t + cv.roll(6) as u64 % 600) % 600;
    if blow < 90 && w >= 40 {
        let wx = (w as f32 * 0.8) as i32 - ((t + cv.roll(6) as u64 % 600) / 600 % 3) as i32 * 7;
        let wy = hz + 1;
        let back = (20, 26, 40);
        let up = if blow < 70 { 1 } else { 0 };
        for i in 0..4 {
            cv.glyph(wx + i, wy, if up == 1 && (i == 1 || i == 2) { '█' } else { '▄' }, rgb(back));
        }
        if (10..60).contains(&blow) {
            let s = ((blow - 10) as i32).min(8);
            for j in 0..s.min(4) {
                cv.glyph(wx + 1, wy - 1 - j, '│', rgb((200, 215, 240)));
            }
            if s >= 4 {
                cv.glyph(wx, wy - 4, '·', rgb((200, 215, 240)));
                cv.glyph(wx + 2, wy - 4, '·', rgb((200, 215, 240)));
            }
        }
    }

    // The ship, rising and falling on the swell.
    let big = w >= 110 && sky >= 16;
    let rows: &[&str] = if big { &BIG } else { &SMALL };
    let sw = rows[0].chars().count() as i32;
    let travel = (w + sw + 16) as u64;
    let shipx = ((t / 12 + cv.roll(7) as u64) % travel) as i32 - sw - 8;
    let base = (hz + 1 + (h - hz) / 4).min(sky - 1).max(hz).min(h - 1);
    let bob = ((tf * 0.07).sin() * 0.9).round() as i32;
    let top_y = base - rows.len() as i32 + 1 + bob;
    let sail = [(232, 226, 205), (200, 180, 150), (180, 70, 60), (150, 160, 190)][(cv.roll(8) % 4) as usize];
    let sail = lerp(sail, (40, 50, 80), 0.35);
    let wood = (70, 46, 30);
    let mut lantern = None;
    for (r, row) in rows.iter().enumerate() {
        for (c, ch) in row.chars().enumerate() {
            let (x, y) = (shipx + c as i32, top_y + r as i32);
            match ch {
                'S' => {
                    let fold = if (x + y * 2 + (t / 15) as i32) % 7 == 0 { '░' } else { ' ' };
                    cv.put(x, y, fold, rgb(lerp(sail, (0, 0, 0), 0.25)), Some(rgb(sail)));
                }
                'F' => cv.glyph(x, y, if (t / 6 + c as u64) % 2 == 0 { '►' } else { '▶' }, rgb((200, 60, 50))),
                'M' => cv.glyph(x, y, '│', rgb(wood)),
                'b' => cv.glyph(x, y, '═', rgb(wood)),
                'r' => cv.glyph(x, y, '▄', rgb(wood)),
                'Q' => cv.glyph(x, y, '▀', rgb(wood)),
                'H' => cv.fill(x, y, rgb(lerp(wood, (0, 0, 0), 0.35))),
                'w' => cv.put(x, y, '▪', rgb((255, 210, 110)), Some(rgb(lerp(wood, (0, 0, 0), 0.35)))),
                'L' => lantern = Some((x, y)),
                ' ' => {}
                c => cv.glyph(x, y, c, rgb(lerp(wood, (0, 0, 0), 0.2))),
            }
        }
    }
    if let Some((lx, ly)) = lantern {
        cv.halo(lx, ly, 2.0, (255, 200, 100), 0.45);
        cv.glyph(lx, ly - 1, '☼', rgb((255, 220, 130)));
        // Its light on the water below.
        for k in 1..4 {
            if (t / 5 + k as u64) % 3 != 0 {
                cv.glyph(lx + (k % 2), ly + k, '─', rgb((200, 150, 80)));
            }
        }
    }
    // The wake, glowing where the hull stirs the water.
    let wy = top_y + rows.len() as i32 - if big { 2 } else { 1 };
    for k in 1..(sw / 2 + 8) {
        let x = shipx - k;
        if (x + (t / 3) as i32) % 3 != 0 {
            let f = k as f32 / (sw / 2 + 8) as f32;
            cv.glyph(x, wy + (k / 7).min(2), if k % 4 == 0 { '·' } else { '~' }, mix((140, 255, 200), (30, 60, 80), f));
        }
    }
    cv.glyph(shipx + sw - 1, wy, '≈', rgb((210, 230, 240)));
}
