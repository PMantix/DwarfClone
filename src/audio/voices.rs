//! Instruments and sound effects: each is a `Voice` that makes one sound
//! a sample at a time and says when it has finished. All synthesized here:
//! plucked strings (Karplus-Strong), a soft horn, a drone, drums, bells and
//! struck metal (a few decaying partials), and the noises of the world.

use super::dsp::*;

pub trait Voice: Send {
    fn next(&mut self) -> f32;
    fn done(&self) -> bool;
    /// Let go of a held note (horns and drones); struck sounds ignore it.
    fn release(&mut self) {}
}

/// Seconds to samples.
fn secs(sr: f32, s: f32) -> u32 {
    (s * sr).max(1.0) as u32
}

// ------------------------------------------------------------ strings

/// A plucked string: a burst of noise rings round a delay line that loses a
/// little of its brightness each time round (Karplus-Strong), through a
/// small body resonance.
pub struct Pluck {
    line: Delay,
    period: f32,
    excite: u32,
    exc_lp: OnePole,
    loss: f32,
    noise: Noise,
    body: Svf,
    age: u32,
    max_age: u32,
    follow: f32,
}

impl Pluck {
    /// `bright` 0..1: how hard and near the bridge the string is plucked.
    /// `sustain`: seconds it can ring.
    pub fn new(sr: f32, hz: f32, bright: f32, sustain: f32, seed: u32) -> Pluck {
        let hz = hz.clamp(30.0, sr / 8.0);
        let period = sr / hz - 0.5;
        // Loss per trip round the string, so it falls ~60 dB in `sustain`.
        let trips = sustain * hz;
        let loss = (0.001f32).powf(1.0 / trips.max(1.0)).clamp(0.9, 0.99995);
        Pluck {
            line: Delay::new(period as usize + 4),
            period,
            excite: period as u32,
            exc_lp: OnePole::new(sr, 800.0 + 7000.0 * bright.clamp(0.0, 1.0)),
            loss,
            noise: Noise::new(seed),
            body: Svf::new(sr, (hz * 2.0).clamp(180.0, 900.0), 1.2),
            age: 0,
            max_age: secs(sr, sustain + 0.3),
            follow: 1.0,
        }
    }
}

impl Voice for Pluck {
    fn next(&mut self) -> f32 {
        let input = if self.excite > 0 {
            self.excite -= 1;
            self.exc_lp.lp(self.noise.white())
        } else {
            0.0
        };
        let a = self.line.read(self.period);
        let b = self.line.read(self.period + 1.0);
        let y = input + self.loss * 0.5 * (a + b);
        self.line.write(y);
        self.age += 1;
        self.follow = self.follow * 0.9995 + y.abs() * 0.0005;
        let out = y * 0.8 + self.body.bp(y) * 0.6;
        // Fade the last moments so a cut never clicks.
        let tail = ((self.max_age.saturating_sub(self.age)) as f32 / 2000.0).min(1.0);
        out * tail
    }

    fn done(&self) -> bool {
        self.age >= self.max_age || (self.age > 4000 && self.follow < 0.0002)
    }
}

// ------------------------------------------------------------ winds

/// A soft horn: a rounded saw and its fundamental, the filter opening as the
/// breath swells, with a slow vibrato that comes in on long notes.
pub struct Horn {
    a: Osc,
    b: Osc,
    sub: Osc,
    vib: Osc,
    hz: f32,
    filt: Svf,
    env: Adsr,
    breath: Noise,
    breath_f: Svf,
    held: u32,
    age: u32,
    sr: f32,
}

impl Horn {
    /// `dur`: seconds held before it's let go.
    pub fn new(sr: f32, hz: f32, dur: f32, seed: u32) -> Horn {
        Horn {
            a: Osc::new(sr),
            b: Osc::new(sr),
            sub: Osc::new(sr),
            vib: Osc::new(sr),
            hz,
            filt: Svf::new(sr, hz * 2.0, 0.8),
            env: Adsr::new(sr, 0.12, 0.4, 0.75, 0.45),
            breath: Noise::new(seed),
            breath_f: Svf::new(sr, hz * 3.0, 2.0),
            held: secs(sr, dur),
            age: 0,
            sr,
        }
    }
}

impl Voice for Horn {
    fn next(&mut self) -> f32 {
        if self.age == self.held {
            self.env.release();
        }
        self.age += 1;
        let t = self.age as f32 / self.sr;
        let depth = ((t - 0.4) * 1.5).clamp(0.0, 1.0) * 0.004;
        let f = self.hz * (1.0 + depth * self.vib.sine(4.8));
        let e = self.env.next();
        self.filt.set(self.hz * (1.2 + 2.6 * e), 0.7);
        let raw = 0.5 * self.a.saw(f) + 0.5 * self.b.saw(f * 1.003);
        let tone = self.filt.lp(raw) + 0.5 * self.sub.sine(f);
        let breath = self.breath_f.bp(self.breath.white()) * 0.08;
        (tone + breath) * e * 0.6
    }

    fn done(&self) -> bool {
        self.age > self.held && self.env.done()
    }

    fn release(&mut self) {
        self.env.release();
        self.held = self.held.min(self.age);
    }
}

/// The drone under a piece: root, fifth and octave, slowly breathing.
pub struct Drone {
    oscs: [Osc; 4],
    hz: f32,
    env: Adsr,
    lfo: Osc,
    lp: OnePole,
    held: u32,
    age: u32,
}

impl Drone {
    pub fn new(sr: f32, hz: f32, dur: f32) -> Drone {
        Drone {
            oscs: [Osc::new(sr), Osc::new(sr), Osc::new(sr), Osc::new(sr)],
            hz,
            env: Adsr::new(sr, 3.0, 1.0, 1.0, 4.0),
            lfo: Osc::new(sr),
            lp: OnePole::new(sr, hz * 4.0),
            held: secs(sr, dur),
            age: 0,
        }
    }
}

impl Voice for Drone {
    fn next(&mut self) -> f32 {
        if self.age == self.held {
            self.env.release();
        }
        self.age += 1;
        let h = self.hz;
        let breathe = 0.85 + 0.15 * self.lfo.sine(0.13);
        let v = self.oscs[0].tri(h) * 0.5 + self.oscs[1].tri(h * 1.0025) * 0.4 + self.oscs[2].sine(h * 1.5) * 0.3 + self.oscs[3].sine(h * 2.0) * 0.15;
        self.lp.lp(v) * self.env.next() * breathe * 0.5
    }

    fn done(&self) -> bool {
        self.age > self.held && self.env.done()
    }

    fn release(&mut self) {
        self.env.release();
        self.held = self.held.min(self.age);
    }
}

// ------------------------------------------------------------ drums

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrumKind {
    /// A big war drum: deep, long, slow.
    War,
    /// A hand drum for dances.
    Frame,
    /// A wooden rim knock for the jig's off-beats.
    Rim,
}

/// A skin drum: a sine falling in pitch, with a slap of filtered noise.
pub struct Drum {
    osc: Osc,
    hz: f32,
    drop: f32,
    drop_k: f32,
    body: Adsr,
    slap: Adsr,
    noise: Noise,
    nf: Svf,
    sub: Osc,
}

impl Drum {
    pub fn new(sr: f32, kind: DrumKind, seed: u32) -> Drum {
        let (hz, decay, slap_hz, slap_decay, drop) = match kind {
            DrumKind::War => (52.0, 1.1, 180.0, 0.12, 1.8),
            DrumKind::Frame => (110.0, 0.35, 700.0, 0.06, 1.4),
            DrumKind::Rim => (420.0, 0.06, 2200.0, 0.025, 1.1),
        };
        Drum {
            osc: Osc::new(sr),
            hz,
            drop,
            drop_k: (-1.0 / (0.035 * sr)).exp(),
            body: Adsr::perc(sr, 0.002, decay),
            slap: Adsr::perc(sr, 0.0008, slap_decay),
            noise: Noise::new(seed),
            nf: Svf::new(sr, slap_hz, 1.0),
            sub: Osc::new(sr),
        }
    }
}

impl Voice for Drum {
    fn next(&mut self) -> f32 {
        self.drop = 1.0 + (self.drop - 1.0) * self.drop_k;
        let f = self.hz * self.drop;
        let body = (self.osc.sine(f) + 0.3 * self.sub.sine(f * 1.5)) * self.body.next();
        let slap = self.nf.bp(self.noise.white()) * self.slap.next() * 1.5;
        ((body + slap) * 0.9).clamp(-1.0, 1.0)
    }

    fn done(&self) -> bool {
        self.body.done() && self.slap.done()
    }
}

/// Jingles on a frame drum or a tambourine: bright noise and a few metal
/// partials, very short.
pub struct Jingle {
    noise: Noise,
    hp: Svf,
    env: Adsr,
    oscs: [Osc; 3],
}

impl Jingle {
    pub fn new(sr: f32, seed: u32) -> Jingle {
        Jingle { noise: Noise::new(seed), hp: Svf::new(sr, 7000.0, 0.9), env: Adsr::perc(sr, 0.001, 0.12), oscs: [Osc::new(sr), Osc::new(sr), Osc::new(sr)] }
    }
}

impl Voice for Jingle {
    fn next(&mut self) -> f32 {
        let e = self.env.next();
        let metal = self.oscs[0].sine(5230.0) + self.oscs[1].sine(6890.0) * 0.7 + self.oscs[2].sine(8120.0) * 0.5;
        (self.hp.run(self.noise.white()).hp * 0.6 + metal * 0.15) * e * 0.6
    }

    fn done(&self) -> bool {
        self.env.done()
    }
}

// ------------------------------------------------------------ struck metal, bells

/// A struck thing: a few inharmonic partials, each ringing away at its own
/// rate, with a click of filtered noise for the strike.
pub struct Modal {
    /// Each partial as a decaying rotation: (re, im, cos, sin), the decay
    /// folded into the rotation, so a partial costs four multiplies.
    parts: Vec<[f32; 4]>,
    click: Adsr,
    click_amp: f32,
    noise: Noise,
    cf: Svf,
    age: u32,
    max_age: u32,
}

impl Modal {
    /// `partials`: (frequency ratio, amplitude, seconds to ring away).
    pub fn new(sr: f32, hz: f32, partials: &[(f32, f32, f32)], click: (f32, f32, f32), seed: u32) -> Modal {
        let longest = partials.iter().map(|p| p.2).fold(click.1, f32::max);
        let parts = partials
            .iter()
            .filter(|p| hz * p.0 < sr * 0.45)
            .map(|&(r, a, d)| {
                let k = (-4.0 / (d.max(0.005) * sr)).exp();
                let w = TAU * hz * r / sr;
                [0.0, a, k * w.cos(), k * w.sin()]
            })
            .collect();
        Modal {
            parts,
            click: Adsr::perc(sr, 0.0005, click.1),
            click_amp: click.0,
            noise: Noise::new(seed),
            cf: Svf::new(sr, click.2, 1.2),
            age: 0,
            max_age: secs(sr, longest * 1.2 + 0.05),
        }
    }
}

impl Voice for Modal {
    fn next(&mut self) -> f32 {
        self.age += 1;
        let mut s = 0.0;
        for p in self.parts.iter_mut() {
            let (re, im) = (p[0] * p[2] - p[1] * p[3], p[0] * p[3] + p[1] * p[2]);
            p[0] = re;
            p[1] = im;
            s += re;
        }
        let click = self.cf.bp(self.noise.white()) * self.click.next() * self.click_amp;
        // Strikes start a hair soft so they don't snap.
        let rise = (self.age as f32 / 24.0).min(1.0);
        (s * rise + click) * 0.5
    }

    fn done(&self) -> bool {
        self.age >= self.max_age || (self.click.done() && self.parts.iter().all(|p| p[0].abs() + p[1].abs() < 0.0005))
    }
}

pub fn pick_on_rock(sr: f32, seed: u32) -> Modal {
    let mut n = Noise::new(seed);
    let hz = n.range(1700.0, 2300.0);
    Modal::new(sr, hz, &[(1.0, 0.45, 0.07), (1.57, 0.3, 0.05), (2.33, 0.22, 0.04), (3.11, 0.12, 0.03), (0.09, 0.5, 0.05)], (0.9, 0.02, 3200.0), seed)
}

pub fn anvil(sr: f32, seed: u32) -> Modal {
    let mut n = Noise::new(seed);
    let hz = n.range(980.0, 1120.0);
    Modal::new(sr, hz, &[(1.0, 0.7, 0.9), (2.76, 0.35, 0.5), (5.40, 0.2, 0.3), (8.93, 0.08, 0.15), (0.21, 0.3, 0.08)], (0.7, 0.015, 4000.0), seed)
}

/// Wood or stone being worked: a dull knock.
pub fn knock(sr: f32, seed: u32) -> Modal {
    let mut n = Noise::new(seed);
    let hz = n.range(320.0, 420.0);
    Modal::new(sr, hz, &[(1.0, 0.7, 0.09), (2.31, 0.35, 0.05), (3.7, 0.12, 0.03)], (0.6, 0.03, 900.0), seed)
}

/// Blade on blade.
pub fn clash(sr: f32, seed: u32) -> Modal {
    let mut n = Noise::new(seed);
    let hz = n.range(1300.0, 1800.0);
    Modal::new(sr, hz, &[(1.0, 0.5, 0.35), (1.41, 0.45, 0.3), (2.03, 0.35, 0.25), (2.73, 0.3, 0.2), (3.51, 0.2, 0.14), (4.62, 0.1, 0.1)], (0.8, 0.05, 5000.0), seed)
}

/// A great bronze bell: hum, prime, minor third, fifth, nominal.
pub fn bell(sr: f32, hz: f32, seed: u32) -> Modal {
    Modal::new(sr, hz, &[(0.5, 0.45, 3.5), (1.0, 0.8, 2.8), (1.19, 0.35, 2.2), (1.5, 0.3, 1.8), (2.0, 0.45, 1.5), (2.52, 0.18, 0.9), (3.01, 0.12, 0.6)], (0.3, 0.01, 2500.0), seed)
}

/// A small glassy chime.
pub fn chime(sr: f32, hz: f32, seed: u32) -> Modal {
    Modal::new(sr, hz, &[(1.0, 0.8, 1.8), (2.76, 0.25, 0.7), (5.40, 0.08, 0.3)], (0.1, 0.005, 6000.0), seed)
}

/// Iron and chains: a door or a bridge.
pub fn clank(sr: f32, seed: u32) -> Modal {
    let mut n = Noise::new(seed);
    let hz = n.range(190.0, 250.0);
    Modal::new(sr, hz, &[(1.0, 0.6, 0.45), (1.63, 0.45, 0.35), (2.91, 0.35, 0.25), (4.37, 0.2, 0.15), (6.1, 0.1, 0.1)], (0.7, 0.06, 1800.0), seed)
}

/// The softest tick, for menus.
pub fn click(sr: f32, seed: u32) -> Modal {
    Modal::new(sr, 1800.0, &[(1.0, 0.3, 0.012), (2.2, 0.1, 0.008)], (0.25, 0.004, 3000.0), seed)
}

/// A drop of water falling into a pool: a short rising blip.
pub struct Drip {
    osc: Osc,
    hz: f32,
    rise: f32,
    env: Adsr,
}

impl Drip {
    pub fn new(sr: f32, hz: f32) -> Drip {
        Drip { osc: Osc::new(sr), hz, rise: (1.0 / (0.05 * sr)).exp(), env: Adsr::perc(sr, 0.001, 0.08) }
    }
}

impl Voice for Drip {
    fn next(&mut self) -> f32 {
        self.hz = (self.hz * self.rise).min(4000.0);
        self.osc.sine(self.hz) * self.env.next() * 0.6
    }

    fn done(&self) -> bool {
        self.env.done()
    }
}

/// A bubble of magma bursting: a low falling blop.
pub struct Blop {
    osc: Osc,
    hz: f32,
    fall: f32,
    env: Adsr,
}

impl Blop {
    pub fn new(sr: f32, hz: f32) -> Blop {
        Blop { osc: Osc::new(sr), hz, fall: (-1.0 / (0.15 * sr)).exp(), env: Adsr::new(sr, 0.01, 0.25, 0.0, 0.25) }
    }
}

impl Voice for Blop {
    fn next(&mut self) -> f32 {
        self.hz = (self.hz * self.fall).max(25.0);
        self.osc.sine(self.hz) * self.env.next() * 0.8
    }

    fn done(&self) -> bool {
        self.env.done()
    }
}

/// A short burst of filtered noise: a crackle in the fire, a raindrop, a
/// footfall in gravel.
pub struct Burst {
    noise: Noise,
    f: Svf,
    env: Adsr,
}

impl Burst {
    pub fn new(sr: f32, hz: f32, q: f32, decay: f32, seed: u32) -> Burst {
        Burst { noise: Noise::new(seed), f: Svf::new(sr, hz, q), env: Adsr::perc(sr, 0.0005, decay) }
    }

}

impl Voice for Burst {
    fn next(&mut self) -> f32 {
        let v = self.f.bp(self.noise.white()) * 2.0;
        (v * self.env.next()).clamp(-1.0, 1.0)
    }

    fn done(&self) -> bool {
        self.env.done()
    }
}

/// A blow landing on flesh or a shield: a dull thump.
pub struct Thud {
    osc: Osc,
    hz: f32,
    fall: f32,
    env: Adsr,
    noise: Noise,
    nf: OnePole,
    nenv: Adsr,
}

impl Thud {
    pub fn new(sr: f32, seed: u32) -> Thud {
        let mut n = Noise::new(seed);
        Thud {
            osc: Osc::new(sr),
            hz: n.range(85.0, 115.0),
            fall: (-1.0 / (0.08 * sr)).exp(),
            env: Adsr::perc(sr, 0.002, 0.2),
            noise: n,
            nf: OnePole::new(sr, 500.0),
            nenv: Adsr::perc(sr, 0.001, 0.07),
        }
    }
}

impl Voice for Thud {
    fn next(&mut self) -> f32 {
        self.hz = (self.hz * self.fall).max(45.0);
        let body = self.osc.sine(self.hz) * self.env.next();
        let n = self.nf.lp(self.noise.white()) * 2.0 * self.nenv.next();
        (body + n).clamp(-1.0, 1.0) * 0.9
    }

    fn done(&self) -> bool {
        self.env.done() && self.nenv.done()
    }
}

/// A beast's roar: a growling low tone and breath through moving formants,
/// swelling and dying away.
pub struct Roar {
    saw: Osc,
    wob: Osc,
    hz: f32,
    noise: Noise,
    f1: Svf,
    f2: Svf,
    env: Adsr,
    held: u32,
    age: u32,
    sr: f32,
    lp: OnePole,
}

impl Roar {
    pub fn new(sr: f32, seed: u32) -> Roar {
        let mut n = Noise::new(seed);
        let dur = n.range(0.9, 1.4);
        Roar {
            saw: Osc::new(sr),
            wob: Osc::new(sr),
            hz: n.range(55.0, 80.0),
            noise: n,
            f1: Svf::new(sr, 400.0, 3.0),
            f2: Svf::new(sr, 900.0, 3.0),
            env: Adsr::new(sr, 0.25, 0.4, 0.8, 0.6),
            held: secs(sr, dur),
            age: 0,
            sr,
            lp: OnePole::new(sr, 1800.0),
        }
    }
}

impl Voice for Roar {
    fn next(&mut self) -> f32 {
        if self.age == self.held {
            self.env.release();
        }
        self.age += 1;
        let t = self.age as f32 / self.sr;
        let e = self.env.next();
        // The mouth opens then closes: formants sweep up and back.
        let open = (t * 2.2).min(1.0) * (1.0 - ((t - 0.8).max(0.0) * 0.8)).max(0.3);
        self.f1.set(300.0 + 450.0 * open, 3.0);
        self.f2.set(800.0 + 700.0 * open, 3.0);
        let growl = self.saw.saw(self.hz * (1.0 + 0.08 * self.wob.sine(23.0))) * (0.8 + 0.2 * self.wob.phase);
        let breath = self.noise.white() * 0.8;
        let src = growl * 0.6 + breath;
        let v = self.f1.bp(src) + self.f2.bp(src) * 0.6;
        self.lp.lp(v * 1.6 * e).clamp(-1.0, 1.0)
    }

    fn done(&self) -> bool {
        self.age > self.held && self.env.done()
    }
}

/// A crowd cheering: a dozen voices of breathy noise through shifting
/// vowels, swelling, with hands clapping.
pub struct Cheer {
    noise: Noise,
    formants: Vec<(Svf, Wander, f32)>,
    env: Adsr,
    held: u32,
    age: u32,
    claps: Vec<Burst>,
    sr: f32,
}

impl Cheer {
    pub fn new(sr: f32, seed: u32) -> Cheer {
        let mut n = Noise::new(seed);
        let formants = (0..6).map(|i| (Svf::new(sr, 600.0, 4.0), Wander::new(sr, 0.15 + 0.05 * i as f32, seed.wrapping_add(i * 17)), n.range(500.0, 1100.0))).collect();
        Cheer { noise: n, formants, env: Adsr::new(sr, 0.35, 0.6, 0.7, 0.9), held: secs(sr, 1.6), age: 0, claps: Vec::new(), sr }
    }
}

impl Voice for Cheer {
    fn next(&mut self) -> f32 {
        if self.age == self.held {
            self.env.release();
        }
        self.age += 1;
        let e = self.env.next();
        let x = self.noise.white();
        let mut s = 0.0;
        for (f, w, base) in self.formants.iter_mut() {
            f.set(*base * (0.7 + 0.8 * w.next()), 5.0);
            s += f.bp(x);
        }
        if self.age < self.held + (0.6 * self.sr) as u32 && self.noise.chance(9.0 / self.sr) {
            let seed = self.noise.next_u32();
            self.claps.push(Burst::new(self.sr, 1400.0 + (seed % 900) as f32, 1.5, 0.03, seed));
        }
        let mut c = 0.0;
        for b in self.claps.iter_mut() {
            c += b.next();
        }
        self.claps.retain(|b| !b.done());
        (s * 0.5 * e + c * 0.35).clamp(-1.0, 1.0)
    }

    fn done(&self) -> bool {
        self.age > self.held && self.env.done() && self.claps.is_empty()
    }
}

/// Thunder: a crack, then a long rolling rumble.
pub struct Thunder {
    brown: Brown,
    noise: Noise,
    lp: Svf,
    crack: Adsr,
    crack_amp: f32,
    crack_f: Svf,
    env: Adsr,
    roll: Wander,
    held: u32,
    age: u32,
}

impl Thunder {
    /// `near` 0..1: a close strike cracks; a far one only rumbles.
    pub fn new(sr: f32, near: f32, seed: u32) -> Thunder {
        let mut n = Noise::new(seed);
        let held = secs(sr, n.range(1.5, 3.0));
        Thunder {
            brown: Brown::new(seed ^ 0x55),
            noise: n,
            lp: Svf::new(sr, 120.0 + 200.0 * near, 0.7),
            crack: Adsr::perc(sr, 0.003, 0.15 + 0.2 * near),
            crack_amp: (near - 0.3).max(0.0),
            crack_f: Svf::new(sr, 1500.0, 0.6),
            env: Adsr::new(sr, 0.08 + 0.4 * (1.0 - near), 1.0, 0.7, 2.2),
            roll: Wander::new(sr, 0.2, seed ^ 0xAA),
            held,
            age: 0,
        }
    }
}

impl Voice for Thunder {
    fn next(&mut self) -> f32 {
        if self.age == self.held {
            self.env.release();
        }
        self.age += 1;
        let rumble = self.lp.lp(self.brown.next()) * 2.5 * (0.4 + 0.9 * self.roll.next()) * self.env.next();
        let crack = self.crack_f.run(self.noise.white()).hp * self.crack.next() * self.crack_amp;
        (rumble + crack).clamp(-1.0, 1.0)
    }

    fn done(&self) -> bool {
        self.age > self.held && self.env.done()
    }
}

/// A small bird's call: a few quick whistled notes, each swooping.
pub struct Chirp {
    osc: Osc,
    notes: Vec<(f32, f32, u32)>,
    i: usize,
    t: u32,
    gap: u32,
    env: f32,
    sr: f32,
}

impl Chirp {
    pub fn new(sr: f32, seed: u32) -> Chirp {
        let mut n = Noise::new(seed);
        let base = n.range(2400.0, 4200.0);
        let count = 2 + n.pick(5);
        let notes = (0..count)
            .map(|_| {
                let a = base * n.range(0.85, 1.2);
                let b = a * n.range(0.7, 1.35);
                (a, b, secs(sr, n.range(0.04, 0.11)))
            })
            .collect();
        Chirp { osc: Osc::new(sr), notes, i: 0, t: 0, gap: secs(sr, n.range(0.03, 0.08)), env: 0.0, sr }
    }
}

impl Voice for Chirp {
    fn next(&mut self) -> f32 {
        let Some(&(a, b, len)) = self.notes.get(self.i) else { return 0.0 };
        self.t += 1;
        if self.t >= len + self.gap {
            self.t = 0;
            self.i += 1;
            return 0.0;
        }
        if self.t >= len {
            return 0.0;
        }
        let x = self.t as f32 / len as f32;
        let hz = a + (b - a) * x;
        // A rounded swell over each note.
        self.env = (x * std::f32::consts::PI).sin();
        let _ = self.sr;
        self.osc.sine(hz) * self.env * 0.5
    }

    fn done(&self) -> bool {
        self.i >= self.notes.len()
    }
}

/// A cricket's chirp: three or four quick pulses of a high tone.
pub struct Cricket {
    osc: Osc,
    hz: f32,
    pulses: u32,
    pulse_len: u32,
    t: u32,
}

impl Cricket {
    pub fn new(sr: f32, seed: u32) -> Cricket {
        let mut n = Noise::new(seed);
        Cricket { osc: Osc::new(sr), hz: n.range(4200.0, 4900.0), pulses: 3 + n.pick(2) as u32, pulse_len: secs(sr, 0.022), t: 0 }
    }
}

impl Voice for Cricket {
    fn next(&mut self) -> f32 {
        self.t += 1;
        let p = self.t / self.pulse_len;
        if p >= self.pulses * 2 {
            return 0.0;
        }
        if p % 2 == 1 {
            return 0.0;
        }
        let x = (self.t % self.pulse_len) as f32 / self.pulse_len as f32;
        self.osc.sine(self.hz) * (x * std::f32::consts::PI).sin() * 0.35
    }

    fn done(&self) -> bool {
        self.t >= self.pulses * 2 * self.pulse_len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The strongest period in a stretch of sound, by autocorrelation.
    fn pitch(sr: f32, v: &mut dyn Voice) -> f32 {
        let buf: Vec<f32> = (0..(sr * 0.4) as usize).map(|_| v.next()).collect();
        let x = &buf[(sr * 0.1) as usize..];
        let (lo, hi) = ((sr / 1000.0) as usize, (sr / 50.0) as usize);
        let best = (lo..hi)
            .max_by(|&a, &b| {
                let c = |lag: usize| x.iter().zip(&x[lag..]).map(|(p, q)| p * q).sum::<f32>();
                c(a).partial_cmp(&c(b)).unwrap()
            })
            .unwrap();
        sr / best as f32
    }

    /// The instruments play the notes they're given (I can't listen; this checks the tuning).
    #[test]
    fn strings_and_horns_are_in_tune() {
        let sr = 44100.0;
        for hz in [110.0, 220.0, 330.0] {
            let p = pitch(sr, &mut Pluck::new(sr, hz, 0.6, 3.0, 5));
            assert!((p / hz - 1.0).abs() < 0.02, "pluck at {} sounds at {}", hz, p);
            let h = pitch(sr, &mut Horn::new(sr, hz, 1.0, 5));
            assert!((h / hz - 1.0).abs() < 0.02, "horn at {} sounds at {}", hz, h);
        }
    }
}
