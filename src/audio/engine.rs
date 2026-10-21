//! The mixer that runs on the audio thread: the ambience beds and their
//! scattered events, the music player, the sound effects, three buses, a
//! shared reverb and the master volume. `render` fills an interleaved
//! buffer; nothing in here blocks or touches the game.

use super::dsp::*;
use super::gorefx::*;
use super::music::{compose, Inst, Mood, Piece};
use super::voices::*;
use super::{Mix, Msg, Scene, Sfx, SfxKind, Tone};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bus {
    Music,
    Ambience,
    Sfx,
}

struct Active {
    v: Box<dyn Voice>,
    bus: Bus,
    gain: f32,
    l: f32,
    r: f32,
    send: f32,
    /// Not to be cut when the bus is full (the drone under a piece).
    keep: bool,
}

/// Equal-power pan: -1 left .. 1 right.
fn pan_gains(pan: f32) -> (f32, f32) {
    let a = (pan.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
    (a.cos(), a.sin())
}

/// Most voices at once on each bus; past this the oldest goes.
const CAP_MUSIC: usize = 32;
const CAP_AMBIENCE: usize = 24;
const CAP_SFX: usize = 16;
const BLOCK: usize = 64;
/// Calibration of the buses, so that at the default volumes the music is
/// quiet, the ambience a little under it, and effects stand out softly.
const MUSIC_TRIM: f32 = 3.0;
const SFX_TRIM: f32 = 3.0;

/// The continuous layers of ambience.
struct Beds {
    noise: Noise,
    brown: Brown,
    brown2: Brown,
    wind_l: Svf,
    wind_r: Svf,
    wind_wl: Wander,
    wind_wr: Wander,
    gust: Wander,
    wind_low: OnePole,
    rain_hp: Svf,
    rain_lp: OnePole,
    hum: [Osc; 3],
    hum_lp: OnePole,
    fire_lp: OnePole,
    magma_lp: Svf,
    magma_w: Wander,
    water: [(Svf, Wander); 3],
    water_hp: Svf,
    breath_lp: Svf,
    breath_lfo: Osc,
    breath_rate: f32,
    breath_w: Wander,
    gaps: [(Svf, Wander); 2],
    gap_gust: Wander,
    voices: [(Svf, Wander, f32); 5],
    voices_lp: OnePole,
    voices_w: Wander,
    babble: Wander,
}

impl Beds {
    fn new(sr: f32, seed: u32) -> Beds {
        Beds {
            noise: Noise::new(seed ^ 0x1234),
            brown: Brown::new(seed ^ 0x77),
            brown2: Brown::new(seed ^ 0x99),
            wind_l: Svf::new(sr, 500.0, 0.9),
            wind_r: Svf::new(sr, 600.0, 0.9),
            wind_wl: Wander::new(sr, 2.5, seed ^ 1),
            wind_wr: Wander::new(sr, 2.8, seed ^ 2),
            gust: Wander::new(sr, 3.5, seed ^ 3),
            wind_low: OnePole::new(sr, 140.0),
            rain_hp: Svf::new(sr, 1500.0, 0.7),
            rain_lp: OnePole::new(sr, 7000.0),
            hum: [Osc::new(sr), Osc::new(sr), Osc::new(sr)],
            hum_lp: OnePole::new(sr, 90.0),
            fire_lp: OnePole::new(sr, 350.0),
            magma_lp: Svf::new(sr, 80.0, 0.8),
            magma_w: Wander::new(sr, 1.5, seed ^ 4),
            water: [
                (Svf::new(sr, 700.0, 5.0), Wander::new(sr, 0.07, seed ^ 5)),
                (Svf::new(sr, 1100.0, 5.0), Wander::new(sr, 0.09, seed ^ 6)),
                (Svf::new(sr, 1500.0, 5.0), Wander::new(sr, 0.11, seed ^ 7)),
            ],
            water_hp: Svf::new(sr, 3000.0, 0.7),
            breath_lp: Svf::new(sr, 260.0, 1.2),
            breath_lfo: Osc::new(sr),
            breath_rate: 0.16,
            breath_w: Wander::new(sr, 6.0, seed ^ 8),
            gaps: [(Svf::new(sr, 900.0, 14.0), Wander::new(sr, 1.8, seed ^ 9)), (Svf::new(sr, 1300.0, 14.0), Wander::new(sr, 2.3, seed ^ 10))],
            gap_gust: Wander::new(sr, 2.5, seed ^ 11),
            voices: std::array::from_fn(|i| (Svf::new(sr, 600.0, 5.0), Wander::new(sr, 0.18 + 0.04 * i as f32, seed ^ (20 + i as u32)), [480.0, 650.0, 820.0, 1100.0, 1500.0][i])),
            voices_lp: OnePole::new(sr, 1400.0),
            voices_w: Wander::new(sr, 2.0, seed ^ 12),
            babble: Wander::new(sr, 0.12, seed ^ 13),
        }
    }

    /// One stereo sample of every layer whose level is up.
    fn next(&mut self, lv: &[f32; LAYERS]) -> (f32, f32) {
        let (mut l, mut r) = (0.0, 0.0);
        let x = self.noise.white();
        if lv[WIND] > 0.0005 {
            let g = 0.25 + 0.75 * self.gust.next().powi(2);
            let cl = 220.0 + 650.0 * self.wind_wl.next();
            let cr = 220.0 + 650.0 * self.wind_wr.next();
            self.wind_l.set(cl, 1.4);
            self.wind_r.set(cr, 1.4);
            let y = self.noise.white();
            let low = self.wind_low.lp(self.brown.next()) * 0.8;
            l += (self.wind_l.bp(x) * 0.9 + low) * g * lv[WIND] * 0.55;
            r += (self.wind_r.bp(y) * 0.9 + low) * g * lv[WIND] * 0.55;
        }
        if lv[RAIN] > 0.0005 {
            let h = self.rain_lp.lp(self.rain_hp.run(x).hp) * 0.22 * lv[RAIN];
            l += h;
            r += h * 0.9 + self.noise.white() * 0.01 * lv[RAIN];
        }
        if lv[HUM] > 0.0005 {
            let h = self.hum[0].sine(55.0) * 0.5 + self.hum[1].sine(55.3) * 0.4 + self.hum[2].sine(110.2) * 0.12 + self.hum_lp.lp(self.brown2.next()) * 0.6;
            l += h * 0.22 * lv[HUM];
            r += h * 0.22 * lv[HUM];
        }
        if lv[FIRE] > 0.0005 {
            let f = self.fire_lp.lp(self.brown.next()) * 0.25 * lv[FIRE];
            l += f;
            r += f;
        }
        if lv[MAGMA] > 0.0005 {
            let m = self.magma_lp.lp(self.brown2.next()) * 0.5 * (0.4 + 0.6 * self.magma_w.next()) * lv[MAGMA];
            l += m;
            r += m;
        }
        if lv[WATER] > 0.0005 {
            let mut s = 0.0;
            for (i, (f, w)) in self.water.iter_mut().enumerate() {
                f.set(400.0 + 300.0 * i as f32 + 900.0 * w.next(), 6.0);
                s += f.bp(x);
            }
            let hiss = self.water_hp.run(x).hp * 0.05;
            l += (s * 0.11 + hiss * 0.6) * lv[WATER];
            r += (s * 0.1 + hiss * 0.6) * lv[WATER];
        }
        if lv[BREATH] > 0.0005 {
            // In and out, slow; the out-breath the louder, with a rasp.
            let ph = self.breath_lfo.sine(self.breath_rate * (0.85 + 0.3 * self.breath_w.next()));
            let swell = ph.max(0.0).powf(1.5) + 0.5 * (-ph).max(0.0).powf(2.0);
            let b = self.breath_lp.bp(self.brown2.next() * 0.7 + x * 0.25) * swell * 0.9 * lv[BREATH];
            l += b;
            r += b * 0.85;
        }
        if lv[GAPS] > 0.0005 {
            let g = (0.2 + 0.8 * self.gap_gust.next()).powi(2);
            let mut v = [0.0f32; 2];
            for (i, (f, w)) in self.gaps.iter_mut().enumerate() {
                f.set(650.0 + 500.0 * i as f32 + 700.0 * w.next(), 16.0);
                v[i] = f.bp(x);
            }
            l += v[0] * 0.2 * g * lv[GAPS];
            r += v[1] * 0.2 * g * lv[GAPS];
        }
        if lv[VOICES] > 0.0005 {
            let mut s = 0.0;
            for (f, w, base) in self.voices.iter_mut() {
                f.set(*base * (0.75 + 0.5 * w.next()), 5.0);
                s += f.bp(x);
            }
            // Talk comes and goes in bursts, far off.
            let talk = 0.35 + 0.65 * self.voices_w.next() * (0.5 + self.babble.next());
            let v = self.voices_lp.lp(s) * 0.12 * talk * lv[VOICES];
            l += v;
            r += v * 0.9;
        }
        (l, r)
    }
}

const LAYERS: usize = 17;
const WIND: usize = 0;
const RAIN: usize = 1;
const THUNDER: usize = 2;
const BIRDS: usize = 3;
const INSECTS: usize = 4;
const DRIPS: usize = 5;
const HUM: usize = 6;
const FIRE: usize = 7;
const MAGMA: usize = 8;
const WATER: usize = 9;
const BREATH: usize = 10;
const GAPS: usize = 11;
const CREAKS: usize = 12;
const VOICES: usize = 13;
const HAMMER: usize = 14;
const FROGS: usize = 15;
const OWLS: usize = 16;

fn scene_levels(s: &Scene) -> [f32; LAYERS] {
    s.levels().map(|v| if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 })
}

/// The piece playing, or the silence before the next.
struct Player {
    piece: Option<Piece>,
    start: u64,
    next: usize,
    /// No new piece before this sample.
    wait_until: u64,
    mood: Mood,
    season: u8,
    tint: u8,
    count: u32,
}

pub struct Engine {
    sr: f32,
    now: u64,
    rng: Noise,
    voices: Vec<Active>,
    pending: Vec<(u64, Active)>,
    reverb: Reverb,
    beds: Beds,
    levels: [Smooth; LAYERS],
    gains: [Smooth; 4],
    mix: Mix,
    player: Player,
    seed: u32,
}

impl Engine {
    pub fn new(sr: f32, seed: u32) -> Engine {
        Engine {
            sr,
            now: 0,
            rng: Noise::new(seed),
            voices: Vec::with_capacity(96),
            pending: Vec::new(),
            reverb: Reverb::new(sr),
            beds: Beds::new(sr, seed),
            levels: [Smooth::new(sr, 2.0, 0.0); LAYERS],
            gains: [Smooth::new(sr, 0.25, 0.0); 4],
            mix: Mix::default(),
            player: Player { piece: None, start: 0, next: 0, wait_until: 0, mood: Mood::Silent, season: 0, tint: 0, count: 0 },
            seed,
        }
    }

    pub fn handle(&mut self, m: Msg) {
        match m {
            Msg::Scene(s) => self.set_scene(s),
            Msg::Sfx(s) => self.sfx(s),
            Msg::Mix(m) => {
                let first = self.mix == Mix::default();
                self.mix = m;
                let on = if m.muted { 0.0 } else { 1.0 };
                let targets = [m.master * on, m.music, m.ambience, m.sfx];
                for (g, t) in self.gains.iter_mut().zip(targets) {
                    g.target = t;
                    // The first word on volume is taken at once; changes glide.
                    if first {
                        g.value = t;
                    }
                }
            }
        }
    }

    fn set_scene(&mut self, s: Scene) {
        let lv = scene_levels(&s);
        for (sm, v) in self.levels.iter_mut().zip(lv) {
            sm.target = v;
        }
        self.player.season = s.season;
        self.player.tint = s.tint;
        if s.mood != self.player.mood {
            let old = self.player.mood;
            self.player.mood = s.mood;
            let secs = self.sr as u64;
            if s.mood.urgent() || s.mood == Mood::Silent || old.urgent() {
                // Let the piece playing go, gently.
                self.stop_music();
            }
            // When the new mood begins: at once for war, after a breath for
            // the rest; peace after war or grief comes after a while of quiet.
            let soon = match s.mood {
                Mood::Title | Mood::WorldGen => 1,
                Mood::War | Mood::Battle => 2,
                Mood::Calm if old == Mood::Silent => 3,
                Mood::Calm => 20,
                _ => 4,
            };
            if s.mood.urgent() || old.urgent() || old == Mood::Silent {
                self.player.wait_until = self.now + soon * secs;
            }
        }
    }

    fn stop_music(&mut self) {
        self.player.piece = None;
        for a in self.voices.iter_mut().filter(|a| a.bus == Bus::Music) {
            a.v.release();
        }
        self.pending.retain(|(_, a)| a.bus != Bus::Music);
    }

    fn push(&mut self, at: u64, bus: Bus, v: Box<dyn Voice>, gain: f32, pan: f32, send: f32) {
        let (l, r) = pan_gains(pan);
        let a = Active { v, bus, gain, l, r, send, keep: false };
        if at <= self.now {
            self.admit(a);
        } else if self.pending.len() < 64 {
            self.pending.push((at, a));
        }
    }

    /// Start a voice, letting the oldest on its bus go if the bus is full.
    fn admit(&mut self, a: Active) {
        let bus = a.bus;
        let cap = match bus {
            Bus::Music => CAP_MUSIC,
            Bus::Ambience => CAP_AMBIENCE,
            Bus::Sfx => CAP_SFX,
        };
        if self.voices.iter().filter(|x| x.bus == bus).count() >= cap {
            if let Some(i) = self.voices.iter().position(|x| x.bus == bus && !x.keep) {
                self.voices.remove(i);
            }
        }
        self.voices.push(a);
    }

    fn secs(&self, s: f32) -> u64 {
        (s * self.sr) as u64
    }

    fn seed(&mut self) -> u32 {
        self.rng.next_u32()
    }

    /// Start a sound effect (some are a few strikes, scheduled ahead).
    pub fn sfx(&mut self, s: Sfx) {
        let sr = self.sr;
        let g = s.gain.clamp(0.0, 1.0);
        let now = self.now;
        let seed = self.seed();
        match s.kind {
            SfxKind::Pick => self.push(now, Bus::Sfx, Box::new(pick_on_rock(sr, seed)), 0.32 * g, s.pan, 0.15),
            SfxKind::Anvil => self.push(now, Bus::Sfx, Box::new(anvil(sr, seed)), 0.22 * g, s.pan, 0.2),
            SfxKind::Knock => self.push(now, Bus::Sfx, Box::new(knock(sr, seed)), 0.3 * g, s.pan, 0.15),
            SfxKind::Clash => self.push(now, Bus::Sfx, Box::new(clash(sr, seed)), 0.26 * g, s.pan, 0.2),
            SfxKind::Thud => self.push(now, Bus::Sfx, Box::new(Thud::new(sr, seed)), 0.3 * g, s.pan, 0.1),
            SfxKind::Roar => self.push(now, Bus::Sfx, Box::new(Roar::new(sr, seed)), 0.2 * g, s.pan, 0.4),
            SfxKind::Bell => {
                // Three strokes of the alarm bell.
                for k in 0..3 {
                    let at = now + self.secs(1.1 * k as f32);
                    let seed = self.seed();
                    self.push(at, Bus::Sfx, Box::new(bell(sr, 233.0, seed)), 0.2 * g, s.pan, 0.45);
                }
            }
            SfxKind::Cheer => self.push(now, Bus::Sfx, Box::new(Cheer::new(sr, seed)), 0.15 * g, s.pan, 0.3),
            SfxKind::Chime => {
                // A rising run on small chimes, from the key of the music.
                let base = midi_hz(self.key() as f32 + 36.0);
                for (k, st) in [0.0f32, 3.0, 7.0, 10.0, 12.0].iter().enumerate() {
                    let at = now + self.secs(0.13 * k as f32);
                    let seed = self.seed();
                    self.push(at, Bus::Sfx, Box::new(chime(sr, base * 2f32.powf(st / 12.0), seed)), 0.16 * g, s.pan + 0.15 * (k as f32 - 2.0), 0.5);
                }
            }
            SfxKind::Clank => {
                self.push(now, Bus::Sfx, Box::new(clank(sr, seed)), 0.3 * g, s.pan, 0.25);
                // The chain running out.
                for k in 1..7 {
                    let at = now + self.secs(0.05 * k as f32 + 0.02);
                    let seed = self.seed();
                    self.push(at, Bus::Sfx, Box::new(Burst::new(sr, 2600.0, 3.0, 0.02, seed)), 0.15 * g * (1.0 - k as f32 / 8.0), s.pan, 0.1);
                }
            }
            SfxKind::Click => self.push(now, Bus::Sfx, Box::new(click(sr, seed)), 0.12 * g, s.pan, 0.0),
            SfxKind::Thunder => self.push(now, Bus::Ambience, Box::new(Thunder::new(sr, 0.8, seed)), 0.6 * g, s.pan, 0.3),
            SfxKind::Stinger(tone) => self.stinger(tone, g),
            SfxKind::Wet => self.push(now, Bus::Sfx, Box::new(Squelch::new(sr, seed)), 0.26 * g, s.pan, 0.1),
            SfxKind::Crunch(skull) => self.push(now, Bus::Sfx, Box::new(Crunch::new(sr, skull, seed)), 0.28 * g, s.pan, 0.12),
            SfxKind::Cry(pitch) => self.push(now, Bus::Sfx, Box::new(Cry::new(sr, cry_hz(pitch), seed)), 0.13 * g, s.pan, 0.35),
            SfxKind::Fall => self.push(now, Bus::Sfx, Box::new(Fall::new(sr, seed)), 0.3 * g, s.pan, 0.15),
            SfxKind::Skitter => self.push(now, Bus::Sfx, Box::new(Skitter::new(sr, seed)), 0.34 * g, s.pan, 0.2),
            SfxKind::Thunk => self.push(now, Bus::Sfx, Box::new(thunk(sr, seed)), 0.26 * g, s.pan, 0.12),
            SfxKind::Whistle => self.push(now, Bus::Sfx, Box::new(Whistle::new(sr, seed)), 0.12 * g, s.pan, 0.15),
            SfxKind::Rattle => {
                self.push(now, Bus::Sfx, Box::new(Rattle::new(sr, seed)), 0.16 * g, s.pan, 0.3);
                // And the body goes down as the breath goes out.
                let (at, seed) = (now + self.secs(0.55), self.seed());
                self.push(at, Bus::Sfx, Box::new(Fall::new(sr, seed)), 0.28 * g, s.pan, 0.15);
            }
            SfxKind::Retch => {
                let hz = self.rng.range(95.0, 140.0);
                for (k, t) in [0.0f32, 0.5, 1.05].iter().enumerate() {
                    let (at, seed) = (now + self.secs(*t), self.seed());
                    self.push(at, Bus::Sfx, Box::new(Heave::new(sr, hz * (1.0 + 0.08 * k as f32), seed)), 0.14 * g, s.pan, 0.2);
                }
                let (at, seed) = (now + self.secs(1.3), self.seed());
                self.push(at, Bus::Sfx, Box::new(splash(sr, seed)), 0.12 * g, s.pan, 0.15);
            }
            SfxKind::Steps => {
                // A few strides, left and right, fading on.
                for k in 0..4 {
                    let (at, seed) = (now + self.secs(0.42 * k as f32), self.seed());
                    let side = if k % 2 == 0 { -0.15 } else { 0.15 };
                    self.push(at, Bus::Sfx, Box::new(footfall(sr, seed)), 0.22 * g * (1.0 - 0.15 * k as f32), s.pan + side, 0.05);
                }
            }
        }
    }

    /// The music's key, or D when nothing plays.
    fn key(&self) -> u8 {
        self.player.piece.as_ref().map_or(50, |p| p.root % 12 + 48)
    }

    /// A soft chord on the horns when a big moment is shown.
    fn stinger(&mut self, tone: Tone, g: f32) {
        let sr = self.sr;
        let k = self.key() as f32;
        let (chord, dur, gain): (&[f32], f32, f32) = match tone {
            Tone::Bright => (&[0.0, 7.0, 16.0], 1.8, 0.18),
            Tone::Dark => (&[-12.0, -5.0, 3.0], 2.2, 0.18),
            Tone::War => (&[-12.0, -5.0, 0.0], 2.0, 0.16),
            Tone::Grief => (&[-12.0, 3.0, 7.0], 2.8, 0.16),
        };
        for (i, st) in chord.iter().enumerate() {
            let at = self.now + self.secs(0.06 * i as f32);
            let seed = self.seed();
            self.push(at, Bus::Sfx, Box::new(Horn::new(sr, midi_hz(k + st), dur, seed)), gain * g, (i as f32 - 1.0) * 0.3, 0.45);
        }
        if tone == Tone::War {
            let seed = self.seed();
            self.push(self.now, Bus::Sfx, Box::new(Drum::new(sr, DrumKind::War, seed)), 0.2 * g, 0.0, 0.3);
        }
    }

    /// Each block: start due voices, play the next notes, scatter the
    /// ambience's events.
    fn block(&mut self) {
        let now = self.now;
        if !self.pending.is_empty() {
            let mut i = 0;
            while i < self.pending.len() {
                if self.pending[i].0 <= now {
                    let (_, a) = self.pending.swap_remove(i);
                    self.admit(a);
                } else {
                    i += 1;
                }
            }
        }
        self.music();
        self.events();
    }

    fn events(&mut self) {
        let sr = self.sr;
        let dt = BLOCK as f32 / sr;
        let lv: [f32; LAYERS] = std::array::from_fn(|i| self.levels[i].value);
        let fire = |rate: f32, n: &mut Noise| rate > 0.0 && n.chance(rate * dt);
        if fire(0.22 * lv[BIRDS], &mut self.rng) {
            let (seed, pan) = (self.seed(), self.rng.range(-0.8, 0.8));
            self.push(self.now, Bus::Ambience, Box::new(Chirp::new(sr, seed)), 0.2 * lv[BIRDS].sqrt(), pan, 0.25);
        }
        if fire(2.5 * lv[INSECTS], &mut self.rng) {
            let (seed, pan) = (self.seed(), self.rng.range(-0.9, 0.9));
            self.push(self.now, Bus::Ambience, Box::new(Cricket::new(sr, seed)), 0.07 * lv[INSECTS].sqrt(), pan, 0.15);
        }
        if fire(0.45 * lv[DRIPS], &mut self.rng) {
            let (hz, pan) = (self.rng.range(700.0, 1500.0), self.rng.range(-0.7, 0.7));
            self.push(self.now, Bus::Ambience, Box::new(Drip::new(sr, hz)), 0.14 * lv[DRIPS].sqrt(), pan, 0.7);
        }
        if fire(35.0 * lv[RAIN], &mut self.rng) {
            let (seed, hz, pan) = (self.seed(), self.rng.range(2000.0, 6000.0), self.rng.range(-1.0, 1.0));
            self.push(self.now, Bus::Ambience, Box::new(Burst::new(sr, hz, 2.0, 0.012, seed)), 0.06 * lv[RAIN], pan, 0.1);
        }
        if fire(lv[THUNDER] / 20.0, &mut self.rng) {
            let (seed, near, pan) = (self.seed(), self.rng.range(0.0, 1.0), self.rng.range(-0.6, 0.6));
            self.push(self.now, Bus::Ambience, Box::new(Thunder::new(sr, near, seed)), 0.55 * lv[THUNDER], pan, 0.3);
        }
        if fire(7.0 * lv[FIRE], &mut self.rng) {
            let (seed, hz, decay, pan) = (self.seed(), self.rng.range(1200.0, 5000.0), self.rng.range(0.003, 0.014), self.rng.range(-0.5, 0.5));
            self.push(self.now, Bus::Ambience, Box::new(Burst::new(sr, hz, 0.9, decay, seed)), 0.2 * lv[FIRE], pan, 0.05);
        }
        if fire(0.12 * lv[CREAKS], &mut self.rng) {
            let (seed, pan) = (self.seed(), self.rng.range(-0.7, 0.7));
            self.push(self.now, Bus::Ambience, Box::new(Creak::new(sr, seed)), 0.12 * lv[CREAKS].sqrt(), pan, 0.4);
        }
        if fire(0.25 * lv[HAMMER], &mut self.rng) {
            // Far off: a smith's few strokes, or a mallet.
            let (seed, pan) = (self.seed(), self.rng.range(-0.8, 0.8));
            let smith = self.rng.chance(0.5);
            let strokes = 1 + self.rng.pick(3);
            for k in 0..strokes {
                let at = self.now + self.secs(0.55 * k as f32);
                let v: Box<dyn Voice> = if smith { Box::new(anvil(sr, seed.wrapping_add(k as u32))) } else { Box::new(knock(sr, seed.wrapping_add(k as u32))) };
                self.push(at, Bus::Ambience, v, 0.07 * lv[HAMMER].sqrt(), pan, 0.5);
            }
        }
        if fire(0.9 * lv[FROGS], &mut self.rng) {
            let (seed, pan) = (self.seed(), self.rng.range(-0.9, 0.9));
            self.push(self.now, Bus::Ambience, Box::new(Croak::new(sr, seed)), 0.09 * lv[FROGS].sqrt(), pan, 0.2);
        }
        if fire(0.06 * lv[OWLS], &mut self.rng) {
            let (seed, pan) = (self.seed(), self.rng.range(-0.8, 0.8));
            self.push(self.now, Bus::Ambience, Box::new(Hoot::new(sr, seed)), 0.1 * lv[OWLS].sqrt(), pan, 0.5);
        }
        if fire(0.7 * lv[MAGMA], &mut self.rng) {
            let (hz, pan) = (self.rng.range(60.0, 120.0), self.rng.range(-0.6, 0.6));
            self.push(self.now, Bus::Ambience, Box::new(Blop::new(sr, hz)), 0.3 * lv[MAGMA], pan, 0.3);
        }
    }

    fn music(&mut self) {
        let sr = self.sr;
        if self.player.piece.is_none() {
            if self.player.mood == Mood::Silent || self.now < self.player.wait_until {
                return;
            }
            // The title has its theme first; after that, variations.
            let fresh = self.rng.next_u32();
            let p = &mut self.player;
            let seed = if p.mood == Mood::Title && p.count == 0 { 1 } else { self.seed ^ p.count.wrapping_mul(7919) ^ fresh };
            p.count += 1;
            p.piece = Some(compose(p.mood, p.season % 4 + 4 * p.tint.min(2), seed));
            p.start = self.now;
            p.next = 0;
        }
        let p = &mut self.player;
        let Some(piece) = p.piece.as_ref() else { return };
        let spb = piece.secs_per_beat();
        let elapsed = (self.now - p.start) as f32 / sr;
        let mut due = Vec::new();
        while let Some(n) = piece.notes.get(p.next) {
            if n.beat * spb > elapsed {
                break;
            }
            due.push(*n);
            p.next += 1;
        }
        let over = p.next >= piece.notes.len() && elapsed >= piece.seconds();
        let mood = p.mood;
        if over {
            p.piece = None;
            let (lo, hi) = mood.gap();
            let gap = lo + (hi - lo) * self.rng.unit();
            self.player.wait_until = self.now + (gap * sr) as u64;
        }
        for n in due {
            self.note(n, spb);
        }
    }

    fn note(&mut self, n: super::music::Note, spb: f32) {
        let sr = self.sr;
        let hz = midi_hz(n.midi as f32);
        let len = n.len * spb;
        let seed = self.seed();
        let v = n.vel;
        let (voice, gain, pan, send): (Box<dyn Voice>, f32, f32, f32) = match n.inst {
            Inst::Pluck => (Box::new(Pluck::new(sr, hz, 0.35 + 0.4 * v, (len * 1.5).clamp(1.2, 4.0), seed)), 0.42 * v, -0.2, 0.35),
            Inst::Horn => (Box::new(Horn::new(sr, hz, len.max(0.3), seed)), 0.22 * v, 0.25, 0.4),
            Inst::LowHorn => (Box::new(Horn::new(sr, hz, len.max(0.3), seed)), 0.26 * v, 0.1, 0.4),
            Inst::Drone => (Box::new(Drone::new(sr, hz, len)), 0.13 * v, 0.0, 0.3),
            Inst::Bell => (Box::new(chime(sr, hz, seed)), 0.1 * v, 0.35, 0.6),
            Inst::WarDrum => (Box::new(Drum::new(sr, DrumKind::War, seed)), 0.55 * v, 0.0, 0.3),
            Inst::FrameDrum => (Box::new(Drum::new(sr, DrumKind::Frame, seed)), 0.32 * v, 0.1, 0.2),
            Inst::Rim => (Box::new(Drum::new(sr, DrumKind::Rim, seed)), 0.12 * v, -0.15, 0.15),
            Inst::Jingle => (Box::new(Jingle::new(sr, seed)), 0.14 * v, 0.2, 0.2),
        };
        self.push(self.now, Bus::Music, voice, gain, pan, send);
        if n.inst == Inst::Drone {
            if let Some(a) = self.voices.last_mut() {
                a.keep = true;
            }
        }
    }

    /// Fill an interleaved buffer of `channels` channels.
    pub fn render(&mut self, out: &mut [f32], channels: usize) {
        let ch = channels.max(1);
        let frames = out.len() / ch;
        let mut f = 0;
        while f < frames {
            if self.now % BLOCK as u64 == 0 {
                self.block();
            }
            let (l, r) = self.frame();
            let base = f * ch;
            out[base] = l;
            if ch > 1 {
                out[base + 1] = r;
                for c in 2..ch {
                    out[base + c] = 0.5 * (l + r);
                }
            }
            f += 1;
        }
    }

    /// One stereo sample.
    fn frame(&mut self) -> (f32, f32) {
        self.now += 1;
        let master = self.gains[0].next();
        let gm = self.gains[1].next() * MUSIC_TRIM;
        let ga = self.gains[2].next();
        let gs = self.gains[3].next() * SFX_TRIM;
        let lv: [f32; LAYERS] = std::array::from_fn(|i| self.levels[i].next());
        let (mut l, mut r, mut send) = (0.0f32, 0.0f32, 0.0f32);
        let (bl, br) = self.beds.next(&lv);
        l += bl * ga;
        r += br * ga;
        send += (bl + br) * 0.5 * ga * 0.15;
        let mut any_done = false;
        for a in self.voices.iter_mut() {
            let bus = match a.bus {
                Bus::Music => gm,
                Bus::Ambience => ga,
                Bus::Sfx => gs,
            };
            let s = a.v.next() * a.gain * bus;
            l += s * a.l;
            r += s * a.r;
            send += s * a.send;
            any_done |= a.v.done();
        }
        if any_done {
            self.voices.retain(|a| !a.v.done());
        }
        let (wl, wr) = self.reverb.run(send * 0.5);
        let soft = |x: f32| {
            let x = x.clamp(-3.0, 3.0);
            x * (27.0 + x * x) / (27.0 + 9.0 * x * x)
        };
        let out_l = soft((l + wl) * master);
        let out_r = soft((r + wr) * master);
        (if out_l.is_finite() { out_l } else { 0.0 }, if out_r.is_finite() { out_r } else { 0.0 })
    }

    #[cfg(test)]
    pub fn voice_count(&self) -> usize {
        self.voices.len() + self.pending.len()
    }

    #[cfg(test)]
    pub fn piece(&self) -> Option<&Piece> {
        self.player.piece.as_ref()
    }
}
