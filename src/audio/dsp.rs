//! The synth's building blocks: noise, oscillators, envelopes, filters,
//! delay lines and a small reverb. Everything works a sample at a time on
//! `f32`, allocates only when made, and is deterministic for a given seed.

use std::f32::consts::PI;

pub const TAU: f32 = 2.0 * PI;

/// A small, fast, seeded random source (xorshift).
#[derive(Clone, Debug)]
pub struct Noise {
    s: u32,
}

impl Noise {
    pub fn new(seed: u32) -> Noise {
        Noise { s: seed.wrapping_mul(0x9E37_79B9) | 1 }
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.s;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.s = x;
        x
    }

    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// White noise in [-1, 1).
    pub fn white(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }

    /// In [lo, hi).
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }

    /// True with probability `p`.
    pub fn chance(&mut self, p: f32) -> bool {
        self.unit() < p
    }

    /// An index below `n` (n > 0).
    pub fn pick(&mut self, n: usize) -> usize {
        (self.next_u32() as usize) % n.max(1)
    }
}

/// Brown (red) noise: integrated white noise, leaky so it stays bounded.
#[derive(Clone, Debug)]
pub struct Brown {
    n: Noise,
    y: f32,
}

impl Brown {
    pub fn new(seed: u32) -> Brown {
        Brown { n: Noise::new(seed), y: 0.0 }
    }

    pub fn next(&mut self) -> f32 {
        self.y = (self.y + 0.02 * self.n.white()) * 0.995;
        (self.y * 3.5).clamp(-1.0, 1.0)
    }
}

/// MIDI note number to frequency.
pub fn midi_hz(m: f32) -> f32 {
    440.0 * 2f32.powf((m - 69.0) / 12.0)
}

/// A phase accumulator; the waveforms read its phase.
#[derive(Clone, Debug)]
pub struct Osc {
    pub phase: f32,
    sr: f32,
}

impl Osc {
    pub fn new(sr: f32) -> Osc {
        Osc { phase: 0.0, sr }
    }

    fn step(&mut self, hz: f32) -> f32 {
        let p = self.phase;
        self.phase += hz / self.sr;
        if self.phase >= 1.0 {
            self.phase -= self.phase.floor();
        }
        p
    }

    pub fn sine(&mut self, hz: f32) -> f32 {
        (self.step(hz) * TAU).sin()
    }

    pub fn tri(&mut self, hz: f32) -> f32 {
        let p = self.step(hz);
        4.0 * (p - 0.5).abs() - 1.0
    }

    /// A saw softened with polyBLEP so it doesn't alias harshly.
    pub fn saw(&mut self, hz: f32) -> f32 {
        let dt = (hz / self.sr).min(0.5);
        let p = self.step(hz);
        let mut v = 2.0 * p - 1.0;
        if p < dt {
            let t = p / dt;
            v -= t + t - t * t - 1.0;
        } else if p > 1.0 - dt {
            let t = (p - 1.0) / dt;
            v -= t * t + t + t + 1.0;
        }
        v
    }
}

/// Attack, decay to a sustain level, release once `release()` is called.
/// Times in seconds; the decay and release are exponential.
#[derive(Clone, Debug)]
pub struct Adsr {
    sr: f32,
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    pub level: f32,
    stage: u8,
}

impl Adsr {
    pub fn new(sr: f32, attack: f32, decay: f32, sustain: f32, release: f32) -> Adsr {
        Adsr { sr, attack: attack.max(0.0005), decay: decay.max(0.001), sustain, release: release.max(0.001), level: 0.0, stage: 0 }
    }

    /// A struck sound: instant rise, then it rings away (no sustain).
    pub fn perc(sr: f32, attack: f32, decay: f32) -> Adsr {
        Adsr::new(sr, attack, decay, 0.0, decay)
    }

    pub fn release(&mut self) {
        if self.stage < 3 {
            self.stage = 3;
        }
    }

    pub fn next(&mut self) -> f32 {
        match self.stage {
            0 => {
                self.level += 1.0 / (self.attack * self.sr);
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = 1;
                }
            }
            1 => {
                let k = 1.0 - (-1.0 / (self.decay * self.sr)).exp();
                self.level += (self.sustain - self.level) * k * 4.0;
                if (self.level - self.sustain).abs() < 0.0005 {
                    self.level = self.sustain;
                    self.stage = 2;
                }
            }
            2 => {}
            _ => {
                let k = 1.0 - (-1.0 / (self.release * self.sr)).exp();
                self.level -= self.level * k * 4.0;
            }
        }
        self.level
    }

    /// Faded away to nothing (after the release, or a percussive decay).
    pub fn done(&self) -> bool {
        (self.stage == 3 || (self.stage == 2 && self.sustain == 0.0)) && self.level < 0.0005
    }
}

/// A one-pole low-pass.
#[derive(Clone, Debug, Default)]
pub struct OnePole {
    y: f32,
    a: f32,
}

impl OnePole {
    pub fn new(sr: f32, hz: f32) -> OnePole {
        let mut f = OnePole { y: 0.0, a: 0.0 };
        f.set(sr, hz);
        f
    }

    pub fn set(&mut self, sr: f32, hz: f32) {
        self.a = 1.0 - (-TAU * hz.clamp(1.0, sr * 0.45) / sr).exp();
    }

    pub fn lp(&mut self, x: f32) -> f32 {
        self.y += self.a * (x - self.y);
        self.y
    }
}

/// A state-variable filter (the trapezoidal kind): stable while swept.
#[derive(Clone, Debug)]
pub struct Svf {
    ic1: f32,
    ic2: f32,
    g: f32,
    k: f32,
    sr: f32,
}

pub struct SvfOut {
    pub lp: f32,
    pub bp: f32,
    pub hp: f32,
}

impl Svf {
    pub fn new(sr: f32, hz: f32, q: f32) -> Svf {
        let mut f = Svf { ic1: 0.0, ic2: 0.0, g: 0.0, k: 1.0, sr };
        f.set(hz, q);
        f
    }

    pub fn set(&mut self, hz: f32, q: f32) {
        self.g = (PI * hz.clamp(10.0, self.sr * 0.45) / self.sr).tan();
        self.k = 1.0 / q.max(0.1);
    }

    pub fn run(&mut self, x: f32) -> SvfOut {
        let (g, k) = (self.g, self.k);
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        let v3 = x - self.ic2;
        let v1 = a1 * self.ic1 + a2 * v3;
        let v2 = self.ic2 + a2 * self.ic1 + a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        SvfOut { lp: v2, bp: v1, hp: x - k * v1 - v2 }
    }

    pub fn lp(&mut self, x: f32) -> f32 {
        self.run(x).lp
    }

    pub fn bp(&mut self, x: f32) -> f32 {
        self.run(x).bp
    }
}

/// A delay line read back with linear interpolation.
#[derive(Clone, Debug)]
pub struct Delay {
    buf: Vec<f32>,
    w: usize,
}

impl Delay {
    pub fn new(len: usize) -> Delay {
        Delay { buf: vec![0.0; len.max(2)], w: 0 }
    }

    pub fn write(&mut self, x: f32) {
        self.buf[self.w] = x;
        self.w = (self.w + 1) % self.buf.len();
    }

    /// The sample written `d` samples ago (1 = the last one).
    pub fn read(&self, d: f32) -> f32 {
        let n = self.buf.len();
        let d = d.clamp(1.0, (n - 1) as f32);
        let i = d.floor() as usize;
        let f = d - i as f32;
        let a = self.buf[(self.w + n - i) % n];
        let b = self.buf[(self.w + n - i - 1) % n];
        a + (b - a) * f
    }
}

/// Freeverb's damped comb.
#[derive(Clone, Debug)]
struct Comb {
    buf: Vec<f32>,
    i: usize,
    store: f32,
}

impl Comb {
    fn run(&mut self, x: f32, feedback: f32, damp: f32) -> f32 {
        let out = self.buf[self.i];
        self.store = out * (1.0 - damp) + self.store * damp;
        self.buf[self.i] = x + self.store * feedback;
        self.i = (self.i + 1) % self.buf.len();
        out
    }
}

#[derive(Clone, Debug)]
struct Allpass {
    buf: Vec<f32>,
    i: usize,
}

impl Allpass {
    fn run(&mut self, x: f32) -> f32 {
        let b = self.buf[self.i];
        let out = b - x;
        self.buf[self.i] = x + b * 0.5;
        self.i = (self.i + 1) % self.buf.len();
        out
    }
}

/// A small stereo reverb in the Schroeder/Freeverb style: four damped
/// combs and two allpasses a side, the right side a little longer.
#[derive(Clone, Debug)]
pub struct Reverb {
    combs: [Vec<Comb>; 2],
    aps: [Vec<Allpass>; 2],
    pub feedback: f32,
    pub damp: f32,
}

impl Reverb {
    pub fn new(sr: f32) -> Reverb {
        let k = sr / 44100.0;
        let make = |spread: usize| {
            let combs = [1116, 1277, 1422, 1557].iter().map(|&n| Comb { buf: vec![0.0; ((n + spread) as f32 * k) as usize + 1], i: 0, store: 0.0 }).collect();
            let aps = [556, 341].iter().map(|&n| Allpass { buf: vec![0.0; ((n + spread) as f32 * k) as usize + 1], i: 0 }).collect();
            (combs, aps)
        };
        let (cl, al) = make(0);
        let (cr, ar) = make(23);
        Reverb { combs: [cl, cr], aps: [al, ar], feedback: 0.82, damp: 0.35 }
    }

    pub fn run(&mut self, x: f32) -> (f32, f32) {
        let mut out = [0.0f32; 2];
        for (side, o) in out.iter_mut().enumerate() {
            let mut s = 0.0;
            for c in self.combs[side].iter_mut() {
                s += c.run(x, self.feedback, self.damp);
            }
            s *= 0.25;
            for a in self.aps[side].iter_mut() {
                s = a.run(s);
            }
            *o = s;
        }
        (out[0], out[1])
    }
}

/// A value that glides toward a target (for levels and volumes, so changes
/// never click).
#[derive(Clone, Copy, Debug)]
pub struct Smooth {
    pub value: f32,
    pub target: f32,
    k: f32,
}

impl Smooth {
    /// `secs`: roughly how long a change takes.
    pub fn new(sr: f32, secs: f32, v: f32) -> Smooth {
        Smooth { value: v, target: v, k: 1.0 - (-1.0 / (secs.max(0.001) * sr)).exp() }
    }

    pub fn next(&mut self) -> f32 {
        self.value += (self.target - self.value) * self.k;
        self.value
    }
}

/// A slowly wandering value in [0, 1]: random targets glided between.
#[derive(Clone, Debug)]
pub struct Wander {
    n: Noise,
    v: f32,
    target: f32,
    left: u32,
    rate: f32,
    every: u32,
}

impl Wander {
    /// Picks a new target every `secs` (roughly) and glides there.
    pub fn new(sr: f32, secs: f32, seed: u32) -> Wander {
        let every = (secs * sr) as u32;
        Wander { n: Noise::new(seed), v: 0.5, target: 0.5, left: 0, rate: 1.0 / every.max(1) as f32 * 3.0, every: every.max(1) }
    }

    pub fn next(&mut self) -> f32 {
        if self.left == 0 {
            self.target = self.n.unit();
            self.left = self.every / 2 + self.n.next_u32() % self.every;
        }
        self.left -= 1;
        self.v += (self.target - self.v) * self.rate;
        self.v
    }
}
