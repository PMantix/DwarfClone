//! A float grid over the map, and the noise shapes the land is drawn with.

use crate::noise::Noise;

/// One value per map column, row by row.
#[derive(Clone)]
pub struct Grid {
    pub w: i32,
    pub h: i32,
    pub v: Vec<f32>,
}

impl Grid {
    pub fn new(w: i32, h: i32, fill: f32) -> Grid {
        Grid { w, h, v: vec![fill; (w * h) as usize] }
    }

    #[inline]
    pub fn idx(&self, x: i32, y: i32) -> usize {
        (y * self.w + x) as usize
    }

    #[inline]
    pub fn inside(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h
    }

    #[inline]
    pub fn at(&self, x: i32, y: i32) -> f32 {
        self.v[self.idx(x, y)]
    }

    /// The value at (x, y), the nearest edge cell outside the grid.
    #[inline]
    pub fn at_clamped(&self, x: i32, y: i32) -> f32 {
        self.at(x.clamp(0, self.w - 1), y.clamp(0, self.h - 1))
    }

    /// Bilinear sample with its gradient, at a point inside the grid.
    pub fn sample(&self, x: f32, y: f32) -> (f32, f32, f32) {
        let (xi, yi) = (x.floor() as i32, y.floor() as i32);
        let (u, v) = (x - xi as f32, y - yi as f32);
        let a = self.at_clamped(xi, yi);
        let b = self.at_clamped(xi + 1, yi);
        let c = self.at_clamped(xi, yi + 1);
        let d = self.at_clamped(xi + 1, yi + 1);
        let gx = (b - a) * (1.0 - v) + (d - c) * v;
        let gy = (c - a) * (1.0 - u) + (d - b) * u;
        let h = a * (1.0 - u) * (1.0 - v) + b * u * (1.0 - v) + c * (1.0 - u) * v + d * u * v;
        (h, gx, gy)
    }

    /// Spread `amount` over the four cells round a point.
    pub fn splat(&mut self, x: f32, y: f32, amount: f32) {
        let (xi, yi) = (x.floor() as i32, y.floor() as i32);
        let (u, v) = (x - xi as f32, y - yi as f32);
        for (dx, dy, wt) in [(0, 0, (1.0 - u) * (1.0 - v)), (1, 0, u * (1.0 - v)), (0, 1, (1.0 - u) * v), (1, 1, u * v)] {
            if self.inside(xi + dx, yi + dy) {
                let i = self.idx(xi + dx, yi + dy);
                self.v[i] += amount * wt;
            }
        }
    }
}

/// Fractal noise centred on zero (about -0.5..0.5).
pub fn fbm(n: &Noise, x: f64, y: f64, octaves: u32) -> f64 {
    (n.fbm2(x, y, octaves) - 0.5) * 2.0
}

/// Ridged noise, 0..1: sharp crests along the noise's middle, the way
/// ranges run, with the detail of each octave strongest on the crests.
pub fn ridged(n: &Noise, x: f64, y: f64, octaves: u32) -> f64 {
    let (mut sum, mut amp, mut freq, mut norm, mut weight) = (0.0, 1.0, 1.0, 0.0, 1.0);
    for o in 0..octaves {
        let v = n.value3(x * freq + o as f64 * 31.7, y * freq - o as f64 * 11.3, 0.5 + o as f64);
        let r = 1.0 - ((v - 0.5).abs() * 2.0);
        let r = r * r * weight;
        weight = (r * 1.6).clamp(0.0, 1.0);
        sum += r * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.1;
    }
    sum / norm
}

/// A point bent by noise, so shapes drawn from it wind instead of blobbing.
pub fn warp(n: &Noise, x: f64, y: f64, scale: f64, by: f64) -> (f64, f64) {
    let qx = fbm(n, x / scale, y / scale, 3);
    let qy = fbm(n, x / scale + 37.2, y / scale - 19.6, 3);
    (x + qx * by, y + qy * by)
}

#[inline]
pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A hash of a world cell, 0..1, the same on every map that sees it.
pub fn cell_hash(x: i32, y: i32, salt: u64) -> f64 {
    let h = crate::rng::splitmix((x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F) ^ salt);
    (h >> 11) as f64 / (1u64 << 53) as f64
}
