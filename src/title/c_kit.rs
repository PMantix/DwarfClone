//! Odds and ends shared by the world, sea and sky scenes: smooth noise for
//! coasts, clouds and smoke, and a little timing help.

use super::*;

/// Smooth value noise in 0..1 at (`x`, `y`), a different field per `salt`.
pub(super) fn noise(x: f32, y: f32, salt: i32) -> f32 {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let (ix, iy) = (x0 as i32, y0 as i32);
    let k = salt.wrapping_mul(7919);
    let v = |dx: i32, dy: i32| (hash(ix.wrapping_add(dx).wrapping_add(k), iy.wrapping_add(dy).wrapping_sub(k)) % 1024) as f32 / 1023.0;
    let a = v(0, 0) + (v(1, 0) - v(0, 0)) * sx;
    let b = v(0, 1) + (v(1, 1) - v(0, 1)) * sx;
    a + (b - a) * sy
}

/// Noise in `octaves` layers, each finer and fainter, in 0..1.
pub(super) fn fbm(x: f32, y: f32, salt: i32, octaves: u32) -> f32 {
    let (mut sum, mut amp, mut f, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for k in 0..octaves {
        sum += noise(x * f, y * f, salt.wrapping_add(k as i32 * 31)) * amp;
        norm += amp;
        amp *= 0.5;
        f *= 2.03;
    }
    sum / norm
}

/// How far (0..1) frame `t` is into a repeating cycle of `period` frames,
/// shifted by `off`.
pub(super) fn phase(t: u64, period: u64, off: u32) -> f32 {
    let p = period.max(1);
    ((t + off as u64) % p) as f32 / p as f32
}

/// A smooth step from 0 at `a` to 1 at `b`.
pub(super) fn ease(a: f32, b: f32, x: f32) -> f32 {
    let f = ((x - a) / (b - a).max(1e-6)).clamp(0.0, 1.0);
    f * f * (3.0 - 2.0 * f)
}
