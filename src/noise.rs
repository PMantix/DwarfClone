use crate::rng::splitmix;

/// Hash-based value noise with fractal octaves. Output is roughly in 0..1.
pub struct Noise {
    seed: u64,
}

fn smooth(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

impl Noise {
    pub fn new(seed: u64) -> Self {
        Noise { seed: splitmix(seed) }
    }

    fn hash(&self, x: i64, y: i64, z: i64) -> f64 {
        let h = self.seed
            ^ (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
            ^ (z as u64).wrapping_mul(0x1656_67B1_9E37_79F9);
        (splitmix(h) >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn value3(&self, x: f64, y: f64, z: f64) -> f64 {
        let (xi, yi, zi) = (x.floor() as i64, y.floor() as i64, z.floor() as i64);
        let (tx, ty, tz) = (
            smooth(x - xi as f64),
            smooth(y - yi as f64),
            smooth(z - zi as f64),
        );
        let c = |dx: i64, dy: i64, dz: i64| self.hash(xi + dx, yi + dy, zi + dz);
        let x00 = lerp(c(0, 0, 0), c(1, 0, 0), tx);
        let x10 = lerp(c(0, 1, 0), c(1, 1, 0), tx);
        let x01 = lerp(c(0, 0, 1), c(1, 0, 1), tx);
        let x11 = lerp(c(0, 1, 1), c(1, 1, 1), tx);
        lerp(lerp(x00, x10, ty), lerp(x01, x11, ty), tz)
    }

    pub fn fbm3(&self, x: f64, y: f64, z: f64, octaves: u32) -> f64 {
        let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
        for o in 0..octaves {
            sum += amp * self.value3(x * freq + o as f64 * 17.3, y * freq, z * freq);
            norm += amp;
            amp *= 0.5;
            freq *= 2.0;
        }
        sum / norm
    }

    pub fn fbm2(&self, x: f64, y: f64, octaves: u32) -> f64 {
        self.fbm3(x, y, 0.5, octaves)
    }
}
