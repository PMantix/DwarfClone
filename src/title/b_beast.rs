//! A forgotten beast asleep in the deepest cavern: a vast shape heaving
//! with slow breath, spores settling on it, a scout with a lantern
//! creeping along the far wall. Then its eyes open, all of them. It lifts
//! its head, the cavern lights with its roar, and the scout runs. Below,
//! more eyes in the dark. The seed makes the beast: its hide, its colour,
//! its eyes, its shape.

use super::b_kit::*;
use super::*;

pub(super) const TAGLINES: &[&str] = &[
    "it was here before the mountain",
    "do not wake it. it is already awake",
    "the deeper you dig, the stranger it gets",
    "a shape in the dark the size of a hall",
];

/// Hides: (glyphs, colour, darker).
const HIDES: [(&[char], Rgb, Rgb); 5] = [
    (&['▓', '▒', '▓', '░'], (90, 120, 70), (30, 44, 26)),   // scaled
    (&['≈', '~', '≈', '≈'], (120, 80, 140), (40, 24, 50)),   // furred
    (&['♦', '▲', '♦', '▼'], (110, 170, 190), (26, 50, 60)),  // crystal
    (&['░', '·', '░', '▒'], (150, 150, 90), (50, 50, 24)),   // slimy
    (&['▓', '█', '▒', '▓'], (140, 70, 50), (50, 20, 16)),    // plated
];

pub(super) fn draw(cv: &mut Canvas) {
    let (w, h, t) = (cv.w(), cv.h(), cv.t);
    let g = (cv.sky - 1).max(3).min(h - 1);
    let tf = t as f32;
    let salt = cv.salt();
    let (glyphs, hide, dark) = HIDES[(cv.roll(1) % 5) as usize];
    let eye = [(255, 60, 40), (250, 240, 120), (140, 255, 200), (230, 120, 255)][(cv.roll(2) % 4) as usize];
    let neyes = 1 + (cv.roll(3) % 5) as i32;
    let air = [(14, 10, 18), (10, 14, 16), (18, 10, 10)][(cv.roll(4) % 3) as usize];

    // The beat: asleep, stirring, awake and roaring, settling back.
    let at = beat(cv, 1500, 5);
    let (stir, wake, settle, asleep) = (880u64, 950u64, 1250u64, 1330u64);
    let rise = if at < wake { 0.0 } else if at < settle { span(at, wake, wake + 50) } else { 1.0 - span(at, settle, asleep) };
    let eyes_open = at >= stir && at < asleep && !(at < wake && (at / 6) % 3 == 0);
    let roar = if at >= wake + 40 && at < wake + 160 { 1.0 - span(at, wake + 40, wake + 160) } else { 0.0 };

    // The cavern, lit faintly by its own eyes when open.
    bedrock(cv, 0, h, lerp(air, (60, 56, 60), 0.6), salt);
    let roof = |x: i32| -> i32 { 1 + (ridge(x, w, cv.seed, 10) * 3.0) as i32 };
    let floor_at = |x: i32| -> i32 { g - (ridge(x, w, cv.seed, 20) * 1.5) as i32 };
    let roofs: Vec<i32> = (0..w).map(roof).collect();
    let floors: Vec<i32> = (0..w).map(floor_at).collect();
    for x in 0..w {
        for y in roofs[x as usize]..floors[x as usize] {
            cv.fill(x, y, rgb(air));
        }
        cv.put(x, floors[x as usize], '▀', rgb((60, 54, 60)), Some(rgb((30, 26, 30))));
        if hash(x, 41) % 3 == 0 {
            cv.glyph(x, roofs[x as usize], '▼', rgb((50, 46, 52)));
        }
    }

    // The beast: a body heaving with breath, a head that lifts.
    let a = (w as f32 * 0.22).max(6.0); // half-length in cells
    let b = ((g - 2) as f32 * 0.45).max(2.0); // half-height in rows
    let bx = w as f32 * (0.62 + (cv.roll(6) % 10) as f32 / 100.0);
    let by = g as f32 - b * 0.55;
    let breath = (tf * 0.035).sin() * 0.05 * (1.0 - rise) + rise * 0.06;
    let ph = (cv.roll(7) % 628) as f32 / 100.0;
    let head = (bx - a * 0.95, by - b * 0.2 - rise * b * 0.9);
    let hr = (b * 0.55).max(1.5);
    let x0 = (bx - a * 1.3 - hr * 2.0) as i32;
    let x1 = (bx + a * 1.3) as i32;
    let y0 = (head.1 - hr - 2.0) as i32;
    if roar > 0.0 {
        cv.halo(head.0 as i32, head.1 as i32, 6.0 + (1.0 - roar) * 14.0, eye, 0.35 * roar);
    }
    if eyes_open {
        cv.halo(head.0 as i32, head.1 as i32, 5.0, eye, 0.25);
    }
    for y in y0.max(0)..g {
        for x in x0.max(0)..x1.min(w) {
            if y >= floors[x as usize] {
                continue;
            }
            let (u, v) = ((x as f32 - bx) / a, (y as f32 - by) / b);
            let th = v.atan2(u);
            let r = 1.0 + 0.12 * (3.0 * th + ph).sin() + 0.08 * (5.0 * th - ph).sin() + breath;
            let d = (u * u + v * v).sqrt() / r;
            let (hu, hv) = ((x as f32 - head.0) * 0.5 / hr, (y as f32 - head.1) / hr);
            let hd = (hu * hu + hv * hv).sqrt();
            // The neck joining them.
            let nt = ((x as f32 - head.0) / (bx - a * 0.6 - head.0)).clamp(0.0, 1.0);
            let ny = head.1 + (by - b * 0.1 - head.1) * nt;
            let neck = x as f32 >= head.0 && (x as f32) <= bx - a * 0.5 && (y as f32 - ny).abs() < hr * 0.6;
            if d < 1.0 || hd < 1.0 || neck {
                let edge = if d < 1.0 { d } else { hd.min(0.8) };
                let hs = hash(x * 3 + salt, y * 5);
                let shade = lerp(hide, dark, (v * 0.5 + 0.5 + edge * 0.3).clamp(0.0, 1.0));
                let ch = if hs % 3 == 0 { glyphs[(hs / 3 % 4) as usize] } else { ' ' };
                cv.put(x, y, ch, rgb(lerp(shade, hide, 0.4)), Some(rgb(lerp(shade, (0, 0, 0), 0.35))));
            }
        }
    }
    // Spines or ridges along its back.
    for k in 0..(a as i32 / 2).max(2) {
        let x = (bx - a * 0.7) as i32 + k * 2 + (hash(k, salt) % 2) as i32;
        let u = (x as f32 - bx) / a;
        let top = by - b * (1.0 - u * u).max(0.0).sqrt() * (1.0 + breath) - 0.6;
        cv.glyph(x, top as i32, '▲', rgb(lerp(hide, (255, 255, 255), 0.15)));
    }
    // Its eyes.
    for k in 0..neyes {
        let ex = head.0 as i32 - (neyes - 1) + k * 2 - 1;
        let ey = head.1 as i32 - (k % 2);
        if eyes_open {
            cv.put(ex, ey, '◉', rgb(eye), Some(rgb(lerp(eye, (0, 0, 0), 0.6))));
        } else {
            cv.glyph(ex, ey, '-', rgb(lerp(hide, (0, 0, 0), 0.4)));
        }
    }
    // Snores while it sleeps, a roar when it wakes.
    if at < stir {
        let age = (t % 90) as i32;
        if age < 50 {
            cv.glyph(head.0 as i32 - 3 - age / 10, head.1 as i32 - 1 - age / 12, if age < 25 { 'z' } else { 'Z' }, rgb((140, 140, 170)));
        }
    } else if roar > 0.0 {
        let r = (1.0 - roar) * 12.0 + 2.0;
        for s in 0..16 {
            let an = s as f32 * 0.39 + 1.6;
            let (x, y) = (head.0 + an.cos() * r * 2.0, head.1 + an.sin() * r * 0.8);
            cv.glyph(x as i32, y as i32, if s % 2 == 0 { '(' } else { '·' }, rgb(lerp(eye, (255, 255, 255), 0.3)));
        }
    }

    // Spores drifting down through the dark, settling on it.
    for k in 0..(w / 4).max(4) {
        let speed = 7 + (k % 5) as u64;
        let y = ((hash(k, 2) as u64 + t / speed) % g.max(1) as u64) as i32;
        let x = (hash(k, 1) % w.max(1) as u32) as i32 + ((y as f32 * 0.3 + k as f32).sin() * 2.0) as i32;
        cv.glyph(x, y, '·', rgb((120, 140, 110)));
    }

    // The scout with the lantern, creeping, then running for it.
    let sx0 = (w / 12).max(2);
    let creep = pingpong(t, 9, sx0, (bx - a * 1.4 - hr * 2.0 - 6.0).max(sx0 as f32 + 2.0) as i32);
    let sx = if at >= stir && at < asleep + 60 { (creep - ((at - stir) / 2) as i32).max(-2) } else { creep };
    let sy = floors[sx.clamp(0, w - 1) as usize] - 1;
    let f = flicker(t, 2);
    cv.halo(sx + 1, sy, 3.0, (255, 190, 100), 0.45 * f);
    cv.glyph(sx, sy, '☺', rgb(beard(cv.roll(8))));
    cv.glyph(sx + 1, sy, '¡', rgb((255, 220, 140)));
    if at >= stir && at < settle {
        cv.glyph(sx, sy - 1, '‼', rgb((255, 240, 160)));
    }

    // Below: more eyes in the deeper dark, opening and closing.
    let lb = below(cv);
    for k in 0..((w * (h - lb).max(0)) / 300).max(0) {
        let x = (hash(k, salt + 5) % w.max(1) as u32) as i32;
        let y = lb + (hash(k, salt + 6) % (h - lb).max(1) as u32) as i32;
        let open = (t / 13 + hash(k, 9) as u64) % 47 < 9;
        if open {
            cv.halo(x, y, 1.5, lerp(eye, (0, 0, 0), 0.4), 0.3);
            cv.glyph(x, y, '•', rgb(lerp(eye, (0, 0, 0), 0.2)));
            cv.glyph(x + 2, y, '•', rgb(lerp(eye, (0, 0, 0), 0.2)));
        }
    }
}
