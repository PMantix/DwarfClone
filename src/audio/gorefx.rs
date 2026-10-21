//! The sounds of a fight's gore, and of the wilds an adventurer walks
//! through: a blade going in, a bone giving, a cry when a limb comes off,
//! a body falling, a tooth skittering over stone, a bolt striking home, an
//! arrow going by, the death rattle, retching; footsteps on the road; and
//! the scattered noises of the wild and of sites (a creaking beam, an owl,
//! frogs). Each is a `Voice`, synthesized here like the rest.

use super::dsp::*;
use super::voices::{Burst, Modal, Voice};

/// Seconds to samples.
fn secs(sr: f32, s: f32) -> u32 {
    (s * sr).max(1.0) as u32
}

/// A fundamental for a cry's `pitch` (0 the deepest beast .. 255 the
/// shrillest): 60 Hz to about 950 Hz, an octave every 64.
pub fn cry_hz(pitch: u8) -> f32 {
    60.0 * 2f32.powf(pitch as f32 / 64.0)
}

// ------------------------------------------------------------ flesh and bone

/// A blade going into flesh: a wet slap, a falling squelch, a low thump
/// under it.
pub struct Squelch {
    noise: Noise,
    f: Svf,
    hz: f32,
    fall: f32,
    env: Adsr,
    lp: OnePole,
    thump: Osc,
    tenv: Adsr,
}

impl Squelch {
    pub fn new(sr: f32, seed: u32) -> Squelch {
        let mut n = Noise::new(seed);
        let hz = n.range(800.0, 1100.0);
        Squelch {
            f: Svf::new(sr, hz, 3.0),
            hz,
            fall: (-1.0 / (0.06 * sr)).exp(),
            env: Adsr::new(sr, 0.004, n.range(0.1, 0.16), 0.0, 0.05),
            lp: OnePole::new(sr, 2500.0),
            thump: Osc::new(sr),
            tenv: Adsr::perc(sr, 0.002, 0.09),
            noise: n,
        }
    }
}

impl Voice for Squelch {
    fn next(&mut self) -> f32 {
        self.hz = (self.hz * self.fall).max(240.0);
        self.f.set(self.hz, 3.5);
        let x = self.lp.lp(self.noise.white());
        // The wet part gurgles: the noise is gated a little at random.
        let wet = self.f.bp(x) * (0.6 + 0.4 * self.noise.unit());
        let body = self.thump.sine(72.0) * self.tenv.next();
        ((wet * 1.6 * self.env.next() + body * 0.7) * 0.8).clamp(-1.0, 1.0)
    }

    fn done(&self) -> bool {
        self.env.done() && self.tenv.done()
    }
}

/// A bone giving way: a dull knock, and a quick crackle of splinters.
pub struct Crunch {
    knock: Modal,
    splinters: Vec<(u32, Burst)>,
    age: u32,
}

impl Crunch {
    /// `skull`: the deeper, wetter crack of a head.
    pub fn new(sr: f32, skull: bool, seed: u32) -> Crunch {
        let mut n = Noise::new(seed);
        let hz = if skull { n.range(260.0, 330.0) } else { n.range(420.0, 560.0) };
        let knock = Modal::new(sr, hz, &[(1.0, 0.6, 0.06), (1.83, 0.35, 0.04), (3.1, 0.2, 0.025)], (0.9, 0.02, 1400.0), seed);
        let count = 6 + n.pick(5);
        let span = if skull { 0.09 } else { 0.06 };
        let splinters = (0..count)
            .map(|i| {
                let at = secs(sr, span * (i as f32 / count as f32).powf(0.7) + n.range(0.0, 0.006));
                (at, Burst::new(sr, n.range(2200.0, 5200.0), 1.4, n.range(0.002, 0.006), n.next_u32()))
            })
            .collect();
        Crunch { knock, splinters, age: 0 }
    }
}

impl Voice for Crunch {
    fn next(&mut self) -> f32 {
        self.age += 1;
        let mut s = self.knock.next();
        for (at, b) in self.splinters.iter_mut() {
            if self.age >= *at {
                s += b.next() * 0.55;
            }
        }
        s.clamp(-1.0, 1.0)
    }

    fn done(&self) -> bool {
        self.knock.done() && self.splinters.iter().all(|(at, b)| self.age >= *at && b.done())
    }
}

/// A cry of pain: a voice on `hz` through two vowel formants, rising and
/// then falling away, rough at the start.
pub struct Cry {
    saw: Osc,
    vib: Osc,
    hz: f32,
    noise: Noise,
    f1: Svf,
    f2: Svf,
    form: (f32, f32),
    env: Adsr,
    held: u32,
    age: u32,
    sr: f32,
    lp: OnePole,
    rough: f32,
}

impl Cry {
    pub fn new(sr: f32, hz: f32, seed: u32) -> Cry {
        let mut n = Noise::new(seed);
        let hz = hz.clamp(40.0, 1200.0);
        // Bigger throats, lower formants.
        let size = (hz / 220.0).clamp(0.45, 1.6).sqrt();
        let vowel = [(760.0, 1250.0), (650.0, 1700.0), (560.0, 1000.0)][n.pick(3)];
        Cry {
            saw: Osc::new(sr),
            vib: Osc::new(sr),
            hz: hz * n.range(0.94, 1.06),
            f1: Svf::new(sr, vowel.0 * size, 4.0),
            f2: Svf::new(sr, vowel.1 * size, 5.0),
            form: (vowel.0 * size, vowel.1 * size),
            env: Adsr::new(sr, 0.03, 0.2, 0.75, 0.28),
            held: secs(sr, n.range(0.35, 0.6)),
            age: 0,
            sr,
            lp: OnePole::new(sr, 4000.0),
            rough: n.range(0.25, 0.5),
            noise: n,
        }
    }
}

impl Voice for Cry {
    fn next(&mut self) -> f32 {
        if self.age == self.held {
            self.env.release();
        }
        self.age += 1;
        let t = self.age as f32 / self.sr;
        // Up a little in the first moment, then sliding down and away.
        let contour = 1.0 + 0.18 * (t * 12.0).min(1.0) - 0.35 * ((t - 0.1).max(0.0) * 1.4).min(1.0);
        let shake = 1.0 + 0.025 * self.vib.sine(6.5) * (t * 3.0).min(1.0);
        let jitter = 1.0 + self.rough * 0.04 * self.noise.white();
        let src = self.saw.saw(self.hz * contour * shake * jitter) + self.noise.white() * 0.35;
        // The mouth opens and closes over the cry.
        let open = 0.85 + 0.15 * (t * 4.0).min(1.0);
        self.f1.set(self.form.0 * open, 4.0);
        self.f2.set(self.form.1 * open, 5.0);
        let v = self.f1.bp(src) + 0.6 * self.f2.bp(src);
        (self.lp.lp(v) * 1.3 * self.env.next()).clamp(-1.0, 1.0)
    }

    fn done(&self) -> bool {
        self.age > self.held && self.env.done()
    }
}

/// A body hitting the floor: a heavy low thump, a smaller bounce, the
/// rustle and clink of what they wore.
pub struct Fall {
    osc: Osc,
    osc2: Osc,
    hz: f32,
    fall: f32,
    brown: Brown,
    lp: OnePole,
    env: Adsr,
    bounce: Adsr,
    bounce_at: u32,
    rustle: Burst,
    age: u32,
}

impl Fall {
    pub fn new(sr: f32, seed: u32) -> Fall {
        let mut n = Noise::new(seed);
        Fall {
            osc: Osc::new(sr),
            osc2: Osc::new(sr),
            hz: n.range(62.0, 80.0),
            fall: (-1.0 / (0.12 * sr)).exp(),
            brown: Brown::new(seed ^ 0x5F),
            lp: OnePole::new(sr, 380.0),
            env: Adsr::perc(sr, 0.004, 0.28),
            bounce: Adsr::perc(sr, 0.003, 0.12),
            bounce_at: secs(sr, n.range(0.08, 0.12)),
            rustle: Burst::new(sr, n.range(1800.0, 2600.0), 0.8, 0.09, n.next_u32()),
            age: 0,
        }
    }
}

impl Voice for Fall {
    fn next(&mut self) -> f32 {
        self.age += 1;
        self.hz = (self.hz * self.fall).max(38.0);
        let thump = (self.osc.sine(self.hz) + self.lp.lp(self.brown.next()) * 1.5) * self.env.next();
        let b = if self.age >= self.bounce_at { self.bounce.next() * 0.45 } else { 0.0 };
        let bounce = self.osc2.sine(self.hz * 1.3) * b;
        (thump + bounce + self.rustle.next() * 0.12).clamp(-1.0, 1.0) * 0.9
    }

    fn done(&self) -> bool {
        self.env.done() && self.age > self.bounce_at && self.bounce.done() && self.rustle.done()
    }
}

/// A tooth skittering across stone: little hard ticks, quicker and softer
/// as it comes to rest.
pub struct Skitter {
    ticks: Vec<(u32, Modal, f32)>,
    age: u32,
}

impl Skitter {
    pub fn new(sr: f32, seed: u32) -> Skitter {
        let mut n = Noise::new(seed);
        let count = 4 + n.pick(3);
        let mut t = 0.0;
        let mut gap = n.range(0.09, 0.13);
        let ticks = (0..count)
            .map(|i| {
                let at = secs(sr, t);
                t += gap;
                gap *= n.range(0.6, 0.75);
                let hz = n.range(2800.0, 4200.0);
                let m = Modal::new(sr, hz, &[(1.0, 0.5, 0.025), (2.4, 0.25, 0.015)], (0.6, 0.004, 5000.0), n.next_u32());
                (at, m, 0.75f32.powi(i as i32))
            })
            .collect();
        Skitter { ticks, age: 0 }
    }
}

impl Voice for Skitter {
    fn next(&mut self) -> f32 {
        self.age += 1;
        let mut s = 0.0;
        for (at, m, g) in self.ticks.iter_mut() {
            if self.age >= *at {
                s += m.next() * *g;
            }
        }
        s.clamp(-1.0, 1.0)
    }

    fn done(&self) -> bool {
        self.ticks.iter().all(|(at, m, _)| self.age >= *at && m.done())
    }
}

/// A bolt striking home: a hard short knock and the shaft's buzz.
pub fn thunk(sr: f32, seed: u32) -> Modal {
    let mut n = Noise::new(seed);
    let hz = n.range(190.0, 240.0);
    Modal::new(sr, hz, &[(1.0, 0.7, 0.07), (2.2, 0.3, 0.04), (4.7, 0.25, 0.12), (4.75, 0.2, 0.12)], (1.0, 0.012, 1800.0), seed)
}

/// An arrow going by: a band of air sweeping down in pitch as it passes.
pub struct Whistle {
    noise: Noise,
    f: Svf,
    osc: Osc,
    from: f32,
    to: f32,
    len: u32,
    age: u32,
}

impl Whistle {
    pub fn new(sr: f32, seed: u32) -> Whistle {
        let mut n = Noise::new(seed);
        let from = n.range(2600.0, 3200.0);
        Whistle { f: Svf::new(sr, from, 9.0), osc: Osc::new(sr), from, to: from * n.range(0.55, 0.68), len: secs(sr, n.range(0.28, 0.4)), age: 0, noise: n }
    }
}

impl Voice for Whistle {
    fn next(&mut self) -> f32 {
        self.age += 1;
        let x = (self.age as f32 / self.len as f32).min(1.0);
        // Doppler: it falls fastest as it goes past, in the middle.
        let k = 0.5 - 0.5 * ((x - 0.5) * 6.0).tanh() / 3f32.tanh();
        let hz = self.to + (self.from - self.to) * k;
        self.f.set(hz, 9.0);
        let swell = (x * std::f32::consts::PI).sin().max(0.0).powf(1.5);
        ((self.f.bp(self.noise.white()) * 1.4 + self.osc.sine(hz) * 0.05) * swell).clamp(-1.0, 1.0)
    }

    fn done(&self) -> bool {
        self.age >= self.len
    }
}

/// The death rattle: a last breath out through a wet throat, gurgling
/// slower as it goes.
pub struct Rattle {
    noise: Noise,
    f1: Svf,
    f2: Svf,
    saw: Osc,
    pulse: Osc,
    rate: f32,
    env: Adsr,
    held: u32,
    age: u32,
    lp: OnePole,
}

impl Rattle {
    pub fn new(sr: f32, seed: u32) -> Rattle {
        let mut n = Noise::new(seed);
        Rattle {
            f1: Svf::new(sr, n.range(320.0, 420.0), 3.0),
            f2: Svf::new(sr, n.range(800.0, 1000.0), 4.0),
            saw: Osc::new(sr),
            pulse: Osc::new(sr),
            rate: n.range(13.0, 17.0),
            env: Adsr::new(sr, 0.15, 0.4, 0.7, 0.5),
            held: secs(sr, n.range(0.6, 0.9)),
            age: 0,
            lp: OnePole::new(sr, 1800.0),
            noise: n,
        }
    }
}

impl Voice for Rattle {
    fn next(&mut self) -> f32 {
        if self.age == self.held {
            self.env.release();
        }
        self.age += 1;
        let e = self.env.next();
        // The gurgle slows as the breath runs out.
        self.rate = (self.rate * 0.99998).max(6.0);
        let gate = (0.5 + 0.5 * self.pulse.sine(self.rate)).powi(3);
        let src = self.noise.white() * 0.9 + self.saw.saw(70.0) * 0.25;
        let v = self.f1.bp(src) + 0.5 * self.f2.bp(src);
        (self.lp.lp(v) * gate * e * 1.4).clamp(-1.0, 1.0)
    }

    fn done(&self) -> bool {
        self.age > self.held && self.env.done()
    }
}

/// One heave of retching: a rough, low, forced "uh" through the throat.
pub struct Heave {
    noise: Noise,
    saw: Osc,
    hz: f32,
    f1: Svf,
    f2: Svf,
    env: Adsr,
    held: u32,
    age: u32,
    lp: OnePole,
}

impl Heave {
    pub fn new(sr: f32, hz: f32, seed: u32) -> Heave {
        let mut n = Noise::new(seed);
        Heave {
            saw: Osc::new(sr),
            hz: hz * n.range(0.9, 1.1),
            f1: Svf::new(sr, n.range(420.0, 520.0), 3.0),
            f2: Svf::new(sr, n.range(900.0, 1100.0), 4.0),
            env: Adsr::new(sr, 0.05, 0.15, 0.6, 0.12),
            held: secs(sr, n.range(0.22, 0.32)),
            age: 0,
            lp: OnePole::new(sr, 2200.0),
            noise: n,
        }
    }
}

impl Voice for Heave {
    fn next(&mut self) -> f32 {
        if self.age == self.held {
            self.env.release();
        }
        self.age += 1;
        let rough = 1.0 + 0.12 * self.noise.white();
        let src = self.saw.saw(self.hz * rough) * 0.5 + self.noise.white() * 0.7;
        let v = self.f1.bp(src) + 0.5 * self.f2.bp(src);
        (self.lp.lp(v) * 1.5 * self.env.next()).clamp(-1.0, 1.0)
    }

    fn done(&self) -> bool {
        self.age > self.held && self.env.done()
    }
}

/// What comes up, landing: a short wet splash.
pub fn splash(sr: f32, seed: u32) -> Burst {
    Burst::new(sr, 700.0, 0.7, 0.16, seed)
}

/// A footfall on a road: a soft crunch of grit.
pub fn footfall(sr: f32, seed: u32) -> Burst {
    let mut n = Noise::new(seed);
    Burst::new(sr, n.range(500.0, 900.0), 0.7, n.range(0.04, 0.07), seed)
}

// ------------------------------------------------------------ the wilds

/// A beam or a door creaking: wood sticking and slipping, a ragged
/// buzz through the timber's ring.
pub struct Creak {
    noise: Noise,
    rate: f32,
    phase: f32,
    sr: f32,
    f1: Svf,
    f2: Svf,
    env: Adsr,
    held: u32,
    age: u32,
    drift: f32,
}

impl Creak {
    pub fn new(sr: f32, seed: u32) -> Creak {
        let mut n = Noise::new(seed);
        Creak {
            rate: n.range(35.0, 80.0),
            phase: 0.0,
            sr,
            f1: Svf::new(sr, n.range(600.0, 900.0), 6.0),
            f2: Svf::new(sr, n.range(1400.0, 1900.0), 6.0),
            env: Adsr::new(sr, 0.08, 0.3, 0.7, 0.2),
            held: secs(sr, n.range(0.35, 0.8)),
            age: 0,
            drift: n.range(-0.8, 0.8),
            noise: n,
        }
    }
}

impl Voice for Creak {
    fn next(&mut self) -> f32 {
        if self.age == self.held {
            self.env.release();
        }
        self.age += 1;
        let t = self.age as f32 / self.sr;
        let rate = self.rate * (1.0 + self.drift * t) * (1.0 + 0.15 * self.noise.white());
        self.phase += rate.max(10.0) / self.sr;
        let click = if self.phase >= 1.0 {
            self.phase -= 1.0;
            1.0
        } else {
            0.0
        };
        let v = self.f1.bp(click) + 0.6 * self.f2.bp(click);
        (v * 3.0 * self.env.next()).clamp(-1.0, 1.0)
    }

    fn done(&self) -> bool {
        self.age > self.held && self.env.done()
    }
}

/// An owl's call: two soft low hoots, the second longer.
pub struct Hoot {
    osc: Osc,
    hz: f32,
    notes: [(u32, u32); 2],
    age: u32,
}

impl Hoot {
    pub fn new(sr: f32, seed: u32) -> Hoot {
        let mut n = Noise::new(seed);
        let first = secs(sr, n.range(0.18, 0.25));
        let gap = secs(sr, n.range(0.25, 0.4));
        let second = secs(sr, n.range(0.4, 0.6));
        Hoot { osc: Osc::new(sr), hz: n.range(330.0, 420.0), notes: [(0, first), (first + gap, second)], age: 0 }
    }
}

impl Voice for Hoot {
    fn next(&mut self) -> f32 {
        self.age += 1;
        for (k, &(at, len)) in self.notes.iter().enumerate() {
            if self.age >= at && self.age < at + len {
                let x = (self.age - at) as f32 / len as f32;
                let env = (x * std::f32::consts::PI).sin().powf(0.7);
                // The second hoot sinks a little.
                let hz = self.hz * (1.0 - 0.06 * x * k as f32);
                return self.osc.sine(hz) * env * 0.6;
            }
        }
        0.0
    }

    fn done(&self) -> bool {
        let (at, len) = self.notes[1];
        self.age >= at + len
    }
}

/// A frog's croak: a few quick buzzing pulses.
pub struct Croak {
    saw: Osc,
    hz: f32,
    f: Svf,
    pulses: u32,
    pulse: u32,
    gap: u32,
    t: u32,
}

impl Croak {
    pub fn new(sr: f32, seed: u32) -> Croak {
        let mut n = Noise::new(seed);
        let hz = n.range(110.0, 220.0);
        Croak { saw: Osc::new(sr), hz, f: Svf::new(sr, hz * 5.0, 4.0), pulses: 2 + n.pick(3) as u32, pulse: secs(sr, n.range(0.05, 0.08)), gap: secs(sr, n.range(0.04, 0.07)), t: 0 }
    }
}

impl Voice for Croak {
    fn next(&mut self) -> f32 {
        self.t += 1;
        let cycle = self.pulse + self.gap;
        let i = self.t % cycle;
        if self.t >= self.pulses * cycle || i >= self.pulse {
            return 0.0;
        }
        let x = i as f32 / self.pulse as f32;
        let env = (x * std::f32::consts::PI).sin();
        (self.f.bp(self.saw.saw(self.hz)) * env * 0.8).clamp(-1.0, 1.0)
    }

    fn done(&self) -> bool {
        self.t >= self.pulses * (self.pulse + self.gap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run a voice to its end (or `max` seconds): its samples.
    pub fn run(v: &mut dyn Voice, sr: f32, max: f32) -> Vec<f32> {
        let mut out = Vec::new();
        while !v.done() && out.len() < (max * sr) as usize {
            out.push(v.next());
        }
        out
    }

    /// Every new voice makes a finite, bounded sound that ends.
    #[test]
    fn every_new_voice_sounds_ends_and_stays_in_range() {
        let sr = 22050.0;
        let voices: Vec<(&str, Box<dyn Voice>)> = vec![
            ("squelch", Box::new(Squelch::new(sr, 3))),
            ("crunch", Box::new(Crunch::new(sr, false, 3))),
            ("skull", Box::new(Crunch::new(sr, true, 3))),
            ("cry low", Box::new(Cry::new(sr, cry_hz(10), 3))),
            ("cry high", Box::new(Cry::new(sr, cry_hz(240), 3))),
            ("fall", Box::new(Fall::new(sr, 3))),
            ("skitter", Box::new(Skitter::new(sr, 3))),
            ("thunk", Box::new(thunk(sr, 3))),
            ("whistle", Box::new(Whistle::new(sr, 3))),
            ("rattle", Box::new(Rattle::new(sr, 3))),
            ("heave", Box::new(Heave::new(sr, 120.0, 3))),
            ("splash", Box::new(splash(sr, 3))),
            ("footfall", Box::new(footfall(sr, 3))),
            ("creak", Box::new(Creak::new(sr, 3))),
            ("hoot", Box::new(Hoot::new(sr, 3))),
            ("croak", Box::new(Croak::new(sr, 3))),
        ];
        for (name, mut v) in voices {
            let buf = run(v.as_mut(), sr, 5.0);
            assert!(v.done(), "{} never finished", name);
            assert!(buf.len() < (3.0 * sr) as usize, "{} lasts {} s", name, buf.len() as f32 / sr);
            assert!(buf.iter().all(|x| x.is_finite() && x.abs() <= 1.0), "{} out of range", name);
            let peak = buf.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(peak > 0.01, "{} is silent (peak {})", name, peak);
        }
    }

    /// Cries follow their pitch: a beast's is lower than a goblin's.
    #[test]
    fn cries_follow_the_throat() {
        assert!(cry_hz(0) < 70.0 && cry_hz(128) > 200.0 && cry_hz(255) < 1000.0);
        let sr = 22050.0;
        let crossings = |hz: f32| {
            let buf = run(&mut Cry::new(sr, hz, 9), sr, 0.3);
            buf.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count()
        };
        assert!(crossings(cry_hz(30)) < crossings(cry_hz(200)), "a deep cry crosses zero less often");
    }
}
