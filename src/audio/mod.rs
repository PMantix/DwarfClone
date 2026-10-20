//! Sound: ambience, music and effects, all synthesized at run time (no
//! sample files). The game never calls in here; the app looks at the game
//! each frame (`scene.rs`), decides the mood and what's audible around the
//! camera, and sends small messages to the audio thread (`output.rs`),
//! where `engine.rs` mixes everything. With no sound device, or built
//! without the `audio` feature, the messages go nowhere and nothing else
//! changes.

// Built silent (`--no-default-features`), the synth is only used by its tests.
#![cfg_attr(not(feature = "audio"), allow(dead_code))]

pub mod dsp;
pub mod engine;
pub mod music;
#[cfg(all(feature = "audio", not(test)))]
mod output;
pub mod scene;
pub mod voices;
#[cfg(test)]
pub mod wav;

pub use music::Mood;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// What can be heard around the camera, each 0..1, and the music's mood.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Scene {
    pub wind: f32,
    pub rain: f32,
    pub thunder: f32,
    pub birds: f32,
    pub insects: f32,
    pub drips: f32,
    pub hum: f32,
    pub fire: f32,
    pub magma: f32,
    pub water: f32,
    pub mood: Mood,
    /// 0 spring .. 3 winter.
    pub season: u8,
}

impl Scene {
    /// Close enough to `o` that sending it again would change nothing heard.
    fn near(&self, o: &Scene) -> bool {
        let a = [self.wind, self.rain, self.thunder, self.birds, self.insects, self.drips, self.hum, self.fire, self.magma, self.water];
        let b = [o.wind, o.rain, o.thunder, o.birds, o.insects, o.drips, o.hum, o.fire, o.magma, o.water];
        self.mood == o.mood && self.season == o.season && a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() < 0.03)
    }
}

/// The flavour of the chord under a big moment's card.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tone {
    Bright,
    Dark,
    War,
    Grief,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SfxKind {
    /// A pick on rock.
    Pick,
    /// A hammer on an anvil.
    Anvil,
    /// Wood or stone being worked.
    Knock,
    /// Blade on blade or armour.
    Clash,
    /// A blow landing.
    Thud,
    Roar,
    /// The alarm bell.
    Bell,
    Cheer,
    Chime,
    /// A door, a bridge, a floodgate.
    Clank,
    /// Menus.
    Click,
    Thunder,
    Stinger(Tone),
}

impl SfxKind {
    /// Least seconds between two of these.
    pub fn min_gap(self) -> f64 {
        match self {
            SfxKind::Pick => 0.3,
            SfxKind::Anvil => 0.5,
            SfxKind::Knock => 0.4,
            SfxKind::Clash => 0.25,
            SfxKind::Thud => 0.2,
            SfxKind::Roar => 8.0,
            SfxKind::Bell => 10.0,
            SfxKind::Cheer => 8.0,
            SfxKind::Chime => 3.0,
            SfxKind::Clank => 0.6,
            SfxKind::Click => 0.05,
            SfxKind::Thunder => 5.0,
            SfxKind::Stinger(_) => 2.5,
        }
    }

    fn slot(self) -> usize {
        match self {
            SfxKind::Pick => 0,
            SfxKind::Anvil => 1,
            SfxKind::Knock => 2,
            SfxKind::Clash => 3,
            SfxKind::Thud => 4,
            SfxKind::Roar => 5,
            SfxKind::Bell => 6,
            SfxKind::Cheer => 7,
            SfxKind::Chime => 8,
            SfxKind::Clank => 9,
            SfxKind::Click => 10,
            SfxKind::Thunder => 11,
            SfxKind::Stinger(_) => 12,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sfx {
    pub kind: SfxKind,
    /// 0..1, after distance.
    pub gain: f32,
    /// -1 left .. 1 right.
    pub pan: f32,
}

/// Bus volumes, 0..1.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Mix {
    pub master: f32,
    pub music: f32,
    pub ambience: f32,
    pub sfx: f32,
    pub muted: bool,
}

/// What the audio thread is told.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Msg {
    Scene(Scene),
    Sfx(Sfx),
    Mix(Mix),
}

/// Where messages go: the audio thread, or nowhere.
pub trait Backend {
    fn send(&mut self, m: Msg);
    /// A real device is playing.
    fn live(&self) -> bool {
        false
    }
    /// What was sent (the silent backend keeps the last few, for tests).
    #[cfg(test)]
    fn sent(&self) -> &[Msg] {
        &[]
    }
}

/// No device: messages are kept (a few) and dropped.
#[derive(Default)]
pub struct NullSink {
    log: Vec<Msg>,
}

impl Backend for NullSink {
    fn send(&mut self, m: Msg) {
        if self.log.len() >= 512 {
            self.log.drain(..256);
        }
        self.log.push(m);
    }

    #[cfg(test)]
    fn sent(&self) -> &[Msg] {
        &self.log
    }
}

/// The sound settings, kept next to the saves.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub enabled: bool,
    /// Percent.
    pub master: u8,
    pub music: u8,
    pub ambience: u8,
    pub sfx: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { enabled: true, master: 70, music: 45, ambience: 65, sfx: 60 }
    }
}

pub const SETTINGS_FILE: &str = "sound.cfg";

impl Settings {
    /// The saved settings, or the defaults if there are none (or they can't be read).
    pub fn load(dir: &Path) -> Settings {
        std::fs::read_to_string(dir.join(SETTINGS_FILE)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn save(&self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        std::fs::write(dir.join(SETTINGS_FILE), serde_json::to_string_pretty(self).map_err(std::io::Error::other)?)
    }

    pub fn mix(&self) -> Mix {
        // Percent to gain on a gentle curve, so the low end is usable.
        let g = |p: u8| (p.min(100) as f32 / 100.0).powf(1.5);
        Mix { master: g(self.master), music: g(self.music), ambience: g(self.ambience), sfx: g(self.sfx), muted: !self.enabled }
    }
}

/// Keeps effects sparse: a least gap per kind, and a few a second at most.
#[derive(Clone, Debug, Default)]
pub struct Limiter {
    last: [Option<f64>; 13],
    recent: Vec<f64>,
}

/// Most effects (menu clicks aside) in any one second.
pub const MAX_PER_SECOND: usize = 5;

impl Limiter {
    pub fn allow(&mut self, kind: SfxKind, now: f64) -> bool {
        let i = kind.slot();
        if self.last[i].map_or(false, |t| now - t < kind.min_gap()) {
            return false;
        }
        if kind != SfxKind::Click {
            self.recent.retain(|&t| now - t < 1.0);
            if self.recent.len() >= MAX_PER_SECOND {
                return false;
            }
            self.recent.push(now);
        }
        self.last[i] = Some(now);
        true
    }
}

/// The app's handle on sound.
pub struct Audio {
    pub settings: Settings,
    backend: Box<dyn Backend>,
    /// Where the settings live; None in tests (nothing is saved).
    dir: Option<PathBuf>,
    /// May a device be opened (only the real game, from `main`).
    can_open: bool,
    limiter: Limiter,
    /// Seconds since start (frame time).
    pub clock: f64,
    last_scene: Option<Scene>,
    last_scene_at: f64,
    /// What the app has seen of the game, to notice what's new.
    pub watch: scene::Watch,
    /// Tests: act as if a device were listening (nothing is opened).
    pretend: bool,
}

impl Audio {
    /// No device and nothing saved: tests, and until `start`.
    pub fn silent() -> Audio {
        Audio {
            settings: Settings::default(),
            backend: Box::new(NullSink::default()),
            dir: None,
            can_open: false,
            limiter: Limiter::default(),
            clock: 0.0,
            last_scene: None,
            last_scene_at: -1.0,
            watch: scene::Watch::default(),
            pretend: false,
        }
    }

    /// Tests: silent, but everything is worked out as if someone listened.
    #[cfg(test)]
    pub fn listening() -> Audio {
        Audio { pretend: true, ..Audio::silent() }
    }

    /// The real game: read the settings and, if sound is on, open the
    /// device on its own thread. Never fails; without a device it's silent.
    pub fn start(dir: &Path) -> Audio {
        let mut a = Audio::silent();
        a.dir = Some(dir.to_path_buf());
        a.settings = Settings::load(dir);
        a.can_open = cfg!(all(feature = "audio", not(test)));
        a.open_if_wanted();
        a
    }

    fn open_if_wanted(&mut self) {
        if !self.can_open || !self.settings.enabled || self.backend.live() {
            return;
        }
        #[cfg(all(feature = "audio", not(test)))]
        if let Some(b) = output::open(std::process::id()) {
            self.backend = Box::new(b);
            self.last_scene = None;
        }
        let m = self.settings.mix();
        self.backend.send(Msg::Mix(m));
    }

    /// Whether sound is on and something is listening.
    pub fn active(&self) -> bool {
        self.settings.enabled && (self.backend.live() || self.pretend)
    }

    /// For tests: the messages a silent backend was sent.
    #[cfg(test)]
    pub fn sent(&self) -> &[Msg] {
        self.backend.sent()
    }

    pub fn advance(&mut self, dt: f64) {
        self.clock += dt.clamp(0.0, 0.5);
    }

    /// Play an effect, unless one like it just played (or sound is off).
    /// Returns whether it was sent.
    pub fn play(&mut self, kind: SfxKind, gain: f32, pan: f32) -> bool {
        if !self.settings.enabled || gain <= 0.01 {
            return false;
        }
        if !self.limiter.allow(kind, self.clock) {
            return false;
        }
        self.backend.send(Msg::Sfx(Sfx { kind, gain: gain.min(1.0), pan: pan.clamp(-1.0, 1.0) }));
        true
    }

    /// Tell the mixer what's around, when it has changed (or now and then).
    pub fn set_scene(&mut self, s: Scene) {
        let stale = self.clock - self.last_scene_at > 5.0;
        if self.last_scene.map_or(true, |o| !o.near(&s)) || stale {
            self.backend.send(Msg::Scene(s));
            self.last_scene = Some(s);
            self.last_scene_at = self.clock;
        }
    }

    #[cfg(test)]
    pub fn last_scene(&self) -> Option<Scene> {
        self.last_scene
    }

    /// Sound on or off (M). Returns whether it's now on.
    pub fn toggle(&mut self) -> bool {
        self.settings.enabled = !self.settings.enabled;
        self.apply();
        self.settings.enabled
    }

    /// Settings changed: tell the mixer and save them.
    pub fn apply(&mut self) {
        self.open_if_wanted();
        self.backend.send(Msg::Mix(self.settings.mix()));
        if let Some(d) = &self.dir {
            let _ = self.settings.save(d);
        }
    }
}

/// `--sound-check`: open the sound device and play a few seconds (the
/// title's theme, a chime, the alarm bell), then say what happened.
pub fn check() {
    #[cfg(all(feature = "audio", not(test)))]
    {
        println!("Sound device: {}", output::device_name().unwrap_or_else(|| "none found".into()));
        let Some(mut dev) = output::open(1) else {
            println!("Couldn't start the sound thread.");
            return;
        };
        dev.send(Msg::Mix(Settings::default().mix()));
        dev.send(Msg::Scene(Scene { mood: Mood::Title, wind: 0.3, ..Scene::default() }));
        dev.send(Msg::Sfx(Sfx { kind: SfxKind::Chime, gain: 1.0, pan: 0.0 }));
        std::thread::sleep(std::time::Duration::from_secs(3));
        dev.send(Msg::Sfx(Sfx { kind: SfxKind::Bell, gain: 1.0, pan: 0.0 }));
        std::thread::sleep(std::time::Duration::from_secs(5));
        let frames = dev.frames.load(std::sync::atomic::Ordering::Relaxed);
        if dev.live() && frames > 0 {
            println!("Played {} frames of sound in 8 seconds.", frames);
        } else {
            println!("The device didn't play (no output, or it failed to open): the game runs silently.");
        }
    }
    #[cfg(not(all(feature = "audio", not(test))))]
    println!("This build has no sound (built without the `audio` feature).");
}

#[cfg(test)]
mod tests {
    use super::engine::Engine;
    use super::*;

    const SR: f32 = 22050.0;

    fn render(e: &mut Engine, secs: f32) -> Vec<f32> {
        let mut buf = vec![0.0; (secs * SR) as usize * 2];
        e.render(&mut buf, 2);
        buf
    }

    fn engine() -> Engine {
        let mut e = Engine::new(SR, 7);
        e.handle(Msg::Mix(Settings::default().mix()));
        e
    }

    fn check(name: &str, buf: &[f32]) -> f32 {
        assert!(buf.iter().all(|x| x.is_finite()), "{}: NaN or infinity", name);
        assert!(buf.iter().all(|x| x.abs() <= 1.0), "{}: clipped beyond [-1, 1]", name);
        let peak = buf.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(peak > 0.002, "{}: silent (peak {})", name, peak);
        peak
    }

    pub const ALL_SFX: [SfxKind; 16] = [
        SfxKind::Pick,
        SfxKind::Anvil,
        SfxKind::Knock,
        SfxKind::Clash,
        SfxKind::Thud,
        SfxKind::Roar,
        SfxKind::Bell,
        SfxKind::Cheer,
        SfxKind::Chime,
        SfxKind::Clank,
        SfxKind::Click,
        SfxKind::Thunder,
        SfxKind::Stinger(Tone::Bright),
        SfxKind::Stinger(Tone::Dark),
        SfxKind::Stinger(Tone::War),
        SfxKind::Stinger(Tone::Grief),
    ];

    #[test]
    fn every_effect_sounds_and_stays_in_range() {
        for k in ALL_SFX {
            let mut e = engine();
            e.handle(Msg::Sfx(Sfx { kind: k, gain: 1.0, pan: 0.3 }));
            let buf = render(&mut e, 4.0);
            check(&format!("{:?}", k), &buf);
            // And it ends: nothing left playing once it has rung out.
            let _ = render(&mut e, 4.0);
            assert_eq!(e.voice_count(), 0, "{:?} never finished", k);
        }
    }

    /// The mixer must run far faster than real time, so the audio thread
    /// never starves (and leaves the game's cores alone).
    #[test]
    #[ignore]
    // Timed by the wall clock, so it runs on its own (`--ignored`), like the
    // year timings: on a busy machine it fails for want of a core.
    fn the_mixer_is_cheap() {
        let sr = 48000.0;
        let mut e = Engine::new(sr, 1);
        e.handle(Msg::Mix(Settings::default().mix()));
        e.handle(Msg::Scene(Scene { wind: 1.0, rain: 1.0, thunder: 1.0, fire: 1.0, water: 1.0, magma: 1.0, hum: 1.0, drips: 1.0, birds: 1.0, insects: 1.0, mood: Mood::War, season: 2 }));
        let mut buf = vec![0.0; 480 * 2];
        let t = std::time::Instant::now();
        for i in 0..1000 {
            if i % 20 == 0 {
                for k in ALL_SFX {
                    e.handle(Msg::Sfx(Sfx { kind: k, gain: 1.0, pan: 0.0 }));
                }
            }
            e.render(&mut buf, 2);
        }
        let secs = t.elapsed().as_secs_f64();
        println!("10 s of the busiest mix in {:.3} s", secs);
        // Ten seconds of the busiest sound there is.
        assert!(secs < 2.0, "10 s of audio took {:.2} s to mix", secs);
        assert!(buf.iter().all(|x| x.is_finite() && x.abs() <= 1.0));
    }

    #[test]
    fn rendering_is_deterministic() {
        let a = {
            let mut e = engine();
            e.handle(Msg::Sfx(Sfx { kind: SfxKind::Anvil, gain: 1.0, pan: 0.0 }));
            render(&mut e, 0.5)
        };
        let b = {
            let mut e = engine();
            e.handle(Msg::Sfx(Sfx { kind: SfxKind::Anvil, gain: 1.0, pan: 0.0 }));
            render(&mut e, 0.5)
        };
        assert_eq!(a, b);
    }

    #[test]
    fn every_ambience_layer_sounds() {
        let layers: [(&str, fn(&mut Scene)); 10] = [
            ("wind", |s| s.wind = 1.0),
            ("rain", |s| s.rain = 1.0),
            ("thunder", |s| s.thunder = 1.0),
            ("birds", |s| s.birds = 1.0),
            ("insects", |s| s.insects = 1.0),
            ("drips", |s| s.drips = 1.0),
            ("hum", |s| s.hum = 1.0),
            ("fire", |s| s.fire = 1.0),
            ("magma", |s| s.magma = 1.0),
            ("water", |s| s.water = 1.0),
        ];
        for (name, set) in layers {
            let mut e = engine();
            let mut s = Scene::default();
            set(&mut s);
            e.handle(Msg::Scene(s));
            // Scattered layers (thunder, birds) may wait a while.
            let buf = render(&mut e, if name == "thunder" { 90.0 } else { 12.0 });
            check(name, &buf);
        }
        // Nothing at all: silence.
        let mut e = engine();
        e.handle(Msg::Scene(Scene::default()));
        let buf = render(&mut e, 2.0);
        assert!(buf.iter().all(|x| x.abs() < 1e-4));
    }

    #[test]
    fn every_mood_plays_music() {
        for m in Mood::ALL {
            let mut e = engine();
            e.handle(Msg::Scene(Scene { mood: m, season: 1, ..Scene::default() }));
            let buf = render(&mut e, 12.0);
            check(&format!("{:?}", m), &buf);
            assert_eq!(e.piece().map(|p| p.mood), Some(m));
        }
    }

    #[test]
    fn muted_is_silent_and_volumes_scale() {
        let loud = |master: u8, enabled: bool| {
            let mut e = Engine::new(SR, 3);
            e.handle(Msg::Mix(Settings { master, enabled, ..Settings::default() }.mix()));
            e.handle(Msg::Scene(Scene { wind: 1.0, ..Scene::default() }));
            let buf = render(&mut e, 3.0);
            (buf.iter().map(|x| x * x).sum::<f32>() / buf.len() as f32).sqrt()
        };
        assert!(loud(70, false) < 1e-4);
        assert!(loud(30, true) < loud(90, true));
    }

    #[test]
    fn a_new_mood_takes_over() {
        let mut e = engine();
        e.handle(Msg::Scene(Scene { mood: Mood::Calm, ..Scene::default() }));
        let _ = render(&mut e, 25.0);
        e.handle(Msg::Scene(Scene { mood: Mood::War, ..Scene::default() }));
        let _ = render(&mut e, 4.0);
        assert_eq!(e.piece().map(|p| p.mood), Some(Mood::War), "war drums start within seconds");
    }

    #[test]
    fn the_limiter_keeps_effects_sparse() {
        let mut l = Limiter::default();
        let mut n = 0;
        // A hundred picks in one second: a handful get through.
        for i in 0..100 {
            if l.allow(SfxKind::Pick, i as f64 * 0.01) {
                n += 1;
            }
        }
        assert!((3..=4).contains(&n), "{} picks in a second", n);
        // Many kinds at once: never more than the cap a second.
        let mut l = Limiter::default();
        let kinds = [SfxKind::Pick, SfxKind::Anvil, SfxKind::Clash, SfxKind::Thud, SfxKind::Knock, SfxKind::Clank, SfxKind::Roar, SfxKind::Chime];
        let mut n = 0;
        for i in 0..200 {
            if l.allow(kinds[i % kinds.len()], i as f64 * 0.005) {
                n += 1;
            }
        }
        assert!(n <= MAX_PER_SECOND, "{} effects in a second", n);
        // A roar doesn't repeat for seconds.
        let mut l = Limiter::default();
        assert!(l.allow(SfxKind::Roar, 0.0));
        assert!(!l.allow(SfxKind::Roar, 3.0));
        assert!(l.allow(SfxKind::Roar, 9.0));
    }

    #[test]
    fn audio_rate_limits_and_respects_mute() {
        let mut a = Audio::silent();
        assert!(a.play(SfxKind::Clash, 1.0, 0.0));
        assert!(!a.play(SfxKind::Clash, 1.0, 0.0), "too soon");
        a.advance(0.5);
        assert!(a.play(SfxKind::Clash, 1.0, 0.0));
        a.toggle();
        a.advance(1.0);
        assert!(!a.play(SfxKind::Clash, 1.0, 0.0), "muted");
        assert!(matches!(a.sent().last(), Some(Msg::Mix(m)) if m.muted));
    }

    #[test]
    fn no_device_is_opened_in_tests() {
        let dir = std::env::temp_dir().join(format!("dwarfclone-sound-{}", std::process::id()));
        let a = Audio::start(&dir);
        assert!(!a.backend.live());
        assert!(!a.active());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn settings_save_and_load() {
        let dir = std::env::temp_dir().join(format!("dwarfclone-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(Settings::load(&dir), Settings::default(), "no file: defaults");
        let s = Settings { enabled: false, master: 40, music: 10, ambience: 90, sfx: 0 };
        s.save(&dir).unwrap();
        assert_eq!(Settings::load(&dir), s);
        // A broken file is ignored.
        std::fs::write(dir.join(SETTINGS_FILE), "not json {").unwrap();
        assert_eq!(Settings::load(&dir), Settings::default());
        // A partial one fills in the rest.
        std::fs::write(dir.join(SETTINGS_FILE), "{\"music\": 5}").unwrap();
        assert_eq!(Settings::load(&dir).music, 5);
        assert_eq!(Settings::load(&dir).sfx, Settings::default().sfx);
        let _ = std::fs::remove_dir_all(&dir);
        // Defaults: on, music quieter than the ambience.
        let d = Settings::default();
        assert!(d.enabled && d.music < d.ambience && d.master < 100);
    }

    /// Render a few samples to listen to:
    /// `cargo test --release render_audio_samples -- --ignored`
    /// writes WAV files into target/audio-samples.
    #[test]
    #[ignore]
    fn render_audio_samples() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target").join("audio-samples");
        std::fs::create_dir_all(&dir).unwrap();
        let sr = 44100.0;
        let out = |name: &str, setup: &dyn Fn(&mut Engine), secs: f32| {
            let mut e = Engine::new(sr, 11);
            e.handle(Msg::Mix(Settings::default().mix()));
            setup(&mut e);
            let mut buf = vec![0.0; (secs * sr) as usize * 2];
            e.render(&mut buf, 2);
            wav::write(&dir.join(format!("{}.wav", name)), &buf, sr as u32, 2).unwrap();
            let peak = buf.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            let rms = (buf.iter().map(|x| x * x).sum::<f32>() / buf.len() as f32).sqrt();
            println!("{:<22} peak {:>6.3}  rms {:>6.4}", name, peak, rms);
        };
        let scene = |s: Scene| move |e: &mut Engine| e.handle(Msg::Scene(s));
        for (m, name) in [(Mood::Title, "music-title"), (Mood::Calm, "music-calm-spring"), (Mood::Festive, "music-festive-jig"), (Mood::War, "music-war-drums"), (Mood::Mourning, "music-mourning"), (Mood::WorldGen, "music-worldgen"), (Mood::Battle, "music-arena-battle")] {
            out(name, &scene(Scene { mood: m, season: 0, ..Scene::default() }), 60.0);
        }
        out("music-calm-winter", &scene(Scene { mood: Mood::Calm, season: 3, ..Scene::default() }), 60.0);
        out("ambience-surface-day", &scene(Scene { wind: 0.5, birds: 0.9, ..Scene::default() }), 30.0);
        out("ambience-summer-dusk", &scene(Scene { wind: 0.3, insects: 1.0, ..Scene::default() }), 30.0);
        out("ambience-storm", &scene(Scene { wind: 1.0, rain: 1.0, thunder: 1.0, ..Scene::default() }), 45.0);
        out("ambience-caverns", &scene(Scene { drips: 1.0, hum: 0.9, ..Scene::default() }), 30.0);
        out("ambience-fire", &scene(Scene { fire: 1.0, ..Scene::default() }), 15.0);
        out("ambience-magma", &scene(Scene { magma: 1.0, hum: 0.3, ..Scene::default() }), 20.0);
        out("ambience-brook", &scene(Scene { water: 1.0, birds: 0.4, wind: 0.2, ..Scene::default() }), 20.0);
        for k in ALL_SFX {
            let name = format!("sfx-{:?}", k).to_lowercase().replace(['(', ')'], "-").trim_end_matches('-').to_string();
            out(&name, &move |e: &mut Engine| e.handle(Msg::Sfx(Sfx { kind: k, gain: 1.0, pan: 0.0 })), 4.0);
        }
        // A minute in a busy fort: digging and forging over the calm music.
        out(
            "mix-busy-fort",
            &|e: &mut Engine| {
                e.handle(Msg::Scene(Scene { mood: Mood::Calm, hum: 0.3, drips: 0.3, fire: 0.2, season: 1, ..Scene::default() }));
            },
            1.0,
        );
        let mut e = Engine::new(sr, 5);
        e.handle(Msg::Mix(Settings::default().mix()));
        e.handle(Msg::Scene(Scene { mood: Mood::Calm, hum: 0.3, drips: 0.3, season: 1, ..Scene::default() }));
        let mut all = Vec::new();
        let mut n = dsp::Noise::new(9);
        for i in 0..120 {
            if i % 3 == 0 {
                e.handle(Msg::Sfx(Sfx { kind: SfxKind::Pick, gain: 0.7, pan: -0.4 }));
            }
            if i % 5 == 2 {
                e.handle(Msg::Sfx(Sfx { kind: SfxKind::Anvil, gain: 0.5, pan: 0.5 }));
            }
            if i == 60 {
                e.handle(Msg::Sfx(Sfx { kind: SfxKind::Stinger(Tone::Bright), gain: 1.0, pan: 0.0 }));
            }
            let mut buf = vec![0.0; (sr * n.range(0.4, 0.6)) as usize * 2];
            e.render(&mut buf, 2);
            all.extend(buf);
        }
        wav::write(&dir.join("mix-busy-fort.wav"), &all, sr as u32, 2).unwrap();
        println!("Wrote samples to {}", dir.display());
    }
}
