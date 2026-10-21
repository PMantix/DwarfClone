//! Sound in the app: each frame, what the camera hears and the mood of the
//! music go to the audio thread (`crate::audio`), with the effects of what
//! just happened near it; `M` mutes; the sound screen sets the volumes.

use super::{App, Mode, DESIG_KEYS};
use crate::audio::{scene, wilds, Mood, Scene, SfxKind};
use crate::buildings::BuildingKind;
use crate::game::ConstructKind;
use crate::geom::Pos;
use crate::items::StockFilter;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Rows of the sound screen.
pub const SOUND_ROWS: [&str; 6] = ["Sound", "Master volume", "Music", "Ambience", "Effects", "Back"];

/// Frames between looks at the ambience and the mood.
const SCENE_EVERY: u64 = 10;

impl App {
    /// The screen the music follows (under the sound screen, the one it was opened from).
    fn sound_mode(&self) -> &Mode {
        match self.mode {
            Mode::Sound { .. } => &self.sound_from,
            _ => &self.mode,
        }
    }

    /// Where the camera is listening: the cursor, or the middle of a panned view.
    pub fn sound_camera(&self) -> Pos {
        if self.free_view {
            Pos::new(self.view_x + self.map_rect.width as i32 / 2, self.view_y + self.map_rect.height as i32 / 2, self.cursor.z)
        } else {
            self.cursor
        }
    }

    /// A fortress is on screen (not the menus, the world map or the arena).
    fn sound_in_fortress(&self) -> bool {
        !self.game.is_placeholder()
            && !matches!(
                self.sound_mode(),
                Mode::Title { .. } | Mode::LoadMenu { .. } | Mode::WorldSetup { .. } | Mode::WorldGen | Mode::WorldMap { .. } | Mode::Embark { .. } | Mode::Prepare { .. } | Mode::Arena | Mode::Legends { from_world: true, .. }
            )
    }

    /// What the music should be about now.
    pub fn music_mood(&self) -> Mood {
        match self.sound_mode() {
            Mode::Title { .. } | Mode::LoadMenu { .. } | Mode::WorldSetup { .. } | Mode::WorldMap { .. } | Mode::Embark { .. } | Mode::Prepare { .. } | Mode::Legends { from_world: true, .. } => Mood::Title,
            Mode::WorldGen => Mood::WorldGen,
            Mode::Arena => Mood::Battle,
            Mode::GameOver => Mood::Mourning,
            _ if self.game.is_placeholder() => Mood::Title,
            _ => scene::fortress_mood(&self.game, self.sound_camera()),
        }
    }

    /// Each frame (from the main loop, after `frame`).
    pub fn sound_frame(&mut self) {
        self.audio.advance(self.frame_dt);
        self.sound_frames += 1;
        if !self.audio.active() {
            // Nobody listening: forget what was seen, so nothing old is news later.
            self.game.sounds.clear();
            self.audio.watch = scene::Watch::default();
            self.audio_spot = None;
            return;
        }
        if matches!(self.sound_mode(), Mode::Adventure) && self.adv.is_some() {
            return self.adv_sound_frame();
        }
        let fort = self.sound_in_fortress();
        let cam = self.sound_camera();
        if self.sound_frames % SCENE_EVERY == 1 {
            let mut s = if fort { scene::ambience(&self.game, cam) } else { Scene::default() };
            s.mood = self.music_mood();
            self.audio.set_scene(s);
        }
        if !fort {
            self.game.sounds.clear();
            return;
        }
        let half = if self.map_rect.width > 0 { (self.map_rect.width as i32 / 2, self.map_rect.height as i32 / 2) } else { (30, 15) };
        let mut cues = Vec::new();
        let running = !self.paused && self.job.is_none();
        self.audio.watch.look(&self.game, cam, half, running, &mut cues);
        self.game.sounds.clear();
        for c in cues {
            self.audio.play(c.kind, c.gain, c.pan);
        }
        // A big moment's card: its stinger, once.
        let shown = self.spot.as_ref().map(|(s, _)| (s.title.clone(), s.text.clone()));
        if shown.is_some() && shown != self.audio_spot {
            if let Some((s, _)) = &self.spot {
                for k in scene::spotlight_cues(&s.title, s.color) {
                    self.audio.play(k, 0.9, 0.0);
                }
            }
        }
        self.audio_spot = shown;
    }

    /// An adventure's sound: the place or the road round the hero, its
    /// music, footsteps on the road, and the fight's noises near them.
    fn adv_sound_frame(&mut self) {
        let half = if self.map_rect.width > 0 { (self.map_rect.width as i32 / 2, self.map_rect.height as i32 / 2) } else { (30, 15) };
        let scene_now = self.sound_frames % SCENE_EVERY == 1;
        let Some(s) = self.adv.as_mut() else { return };
        let a = &mut s.adv;
        // A new place: nothing already there is news.
        let tag = wilds::place_tag(a);
        if self.audio.ears.place != Some(tag) {
            self.audio.ears.place = Some(tag);
            self.audio.watch = scene::Watch::default();
        }
        if scene_now {
            let mut sc = wilds::ambience(a);
            sc.mood = wilds::mood(a, &s.view, s.meet.is_some());
            self.audio.set_scene(sc);
        }
        if wilds::stepped(&mut self.audio.ears, a) {
            self.audio.play(SfxKind::Steps, 0.8, 0.0);
        }
        let mut cues = Vec::new();
        if let Some(v) = a.visit.as_mut() {
            if let Some(cam) = v.g.dwarves.get(v.hero).map(|d| d.pos) {
                self.audio.watch.look(&v.g, cam, half, true, &mut cues);
            }
            v.g.sounds.clear();
        }
        for c in cues {
            self.audio.play(c.kind, c.gain, c.pan);
        }
    }

    /// In this screen `M` already means something (a letter in a list).
    fn m_is_taken(&self) -> bool {
        let m = 'M';
        // (A name being typed takes every letter: a hall's, or a hero's.)
        if self.naming.is_some() || self.adv.as_ref().and_then(|a| a.maker.as_ref()).map_or(false, |m| m.naming) {
            return true;
        }
        match &self.mode {
            Mode::BuildMenu => BuildingKind::ALL.iter().any(|b| b.key() == m),
            Mode::Query { bld, adding: true } => self.game.buildings.get(bld).map_or(false, |b| b.kind.recipes().iter().any(|r| r.key() == m)),
            Mode::Designate { .. } => DESIG_KEYS.iter().any(|(c, _)| *c == m),
            Mode::Stockpile { .. } => StockFilter::ALL.iter().any(|f| f.key() == m),
            Mode::Zone { .. } => super::ZoneTool::ALL.iter().any(|t| t.key() == m),
            Mode::Construct { .. } => ConstructKind::ALL.iter().any(|c| c.key() == m),
            _ => false,
        }
    }

    /// Keys for sound that work (nearly) everywhere: `M` mutes, `O` opens
    /// the sound screen from the fortress, the help and the title; and the
    /// sound screen's own keys. True if the key was used.
    pub(super) fn key_sound_global(&mut self, k: &KeyEvent) -> bool {
        if let Mode::Sound { sel } = self.mode {
            self.key_sound(k, sel);
            return true;
        }
        let plain = !k.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        if plain && k.code == KeyCode::Char('M') && !self.m_is_taken() {
            let on = self.audio.toggle();
            let msg = if on { "Sound on (M to mute, O for volumes)." } else { "Sound off (M turns it back on)." };
            self.flash = Some(msg.into());
            if let Some(a) = self.arena.as_mut() {
                a.flash = Some(msg.into());
            }
            return true;
        }
        if plain && k.code == KeyCode::Char('O') && matches!(self.mode, Mode::Normal | Mode::Help | Mode::Title { .. }) {
            self.open_sound();
            return true;
        }
        false
    }

    pub fn open_sound(&mut self) {
        self.sound_from = match &self.mode {
            Mode::Help => Mode::Normal,
            m => m.clone(),
        };
        self.mode = Mode::Sound { sel: 0 };
    }

    fn key_sound(&mut self, k: &KeyEvent, sel: usize) {
        let n = SOUND_ROWS.len();
        let step: i32 = match k.code {
            KeyCode::Left | KeyCode::Char('-') => -5,
            KeyCode::Right | KeyCode::Char('+') | KeyCode::Char('=') => 5,
            _ => 0,
        };
        match k.code {
            KeyCode::Esc | KeyCode::Char('O') | KeyCode::Char('o') | KeyCode::Char('q') => self.mode = self.sound_from.clone(),
            KeyCode::Up => self.mode = Mode::Sound { sel: (sel + n - 1) % n },
            KeyCode::Down | KeyCode::Tab => self.mode = Mode::Sound { sel: (sel + 1) % n },
            KeyCode::Enter | KeyCode::Char(' ') if sel == 0 => {
                self.audio.toggle();
            }
            KeyCode::Enter if sel == n - 1 => self.mode = self.sound_from.clone(),
            KeyCode::Char('M') => {
                self.audio.toggle();
            }
            _ if step != 0 => {
                let s = &mut self.audio.settings;
                let v = match sel {
                    0 => {
                        s.enabled = step > 0;
                        None
                    }
                    1 => Some(&mut s.master),
                    2 => Some(&mut s.music),
                    3 => Some(&mut s.ambience),
                    4 => Some(&mut s.sfx),
                    _ => None,
                };
                if let Some(v) = v {
                    *v = (*v as i32 + step).clamp(0, 100) as u8;
                }
                self.audio.apply();
                self.audio.play(SfxKind::Click, 1.0, 0.0);
            }
            _ => {}
        }
    }

    /// After a key: a soft click when it moved through a menu.
    pub(super) fn menu_click(&mut self, before: &Mode) {
        if *before != self.mode && !(matches!(before, Mode::Normal) && matches!(self.mode, Mode::Normal)) {
            self.audio.play(SfxKind::Click, 1.0, 0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{Audio, Msg};
    use crate::game::Game;

    fn key(app: &mut App, c: KeyCode) {
        app.on_key(KeyEvent::new(c, if matches!(c, KeyCode::Char(ch) if ch.is_uppercase()) { KeyModifiers::SHIFT } else { KeyModifiers::NONE }));
    }

    fn listening(game: Game) -> App {
        let mut app = App::with_game_for_tests(game);
        app.mode = Mode::Normal;
        app.audio = Audio::listening();
        app
    }

    #[test]
    fn m_mutes_everywhere_and_o_opens_the_sound_screen() {
        let mut app = listening(Game::new(3));
        assert!(app.audio.settings.enabled);
        key(&mut app, KeyCode::Char('M'));
        assert!(!app.audio.settings.enabled);
        assert!(app.flash.as_deref().unwrap_or("").contains("Sound off"));
        key(&mut app, KeyCode::Char('M'));
        assert!(app.audio.settings.enabled);
        // In the build menu M is the millstone, not mute.
        app.mode = Mode::BuildMenu;
        key(&mut app, KeyCode::Char('M'));
        assert!(app.audio.settings.enabled);
        assert!(matches!(app.mode, Mode::Place(_)));
        // On the title screen too.
        app.mode = Mode::Title { sel: 0 };
        key(&mut app, KeyCode::Char('M'));
        assert!(!app.audio.settings.enabled);
        key(&mut app, KeyCode::Char('M'));
        // O: the sound screen, and back.
        app.mode = Mode::Normal;
        key(&mut app, KeyCode::Char('O'));
        assert_eq!(app.mode, Mode::Sound { sel: 0 });
        key(&mut app, KeyCode::Down);
        let before = app.audio.settings.master;
        key(&mut app, KeyCode::Left);
        assert_eq!(app.audio.settings.master, before - 5);
        key(&mut app, KeyCode::Esc);
        assert_eq!(app.mode, Mode::Normal);
        // G grows the map now.
        key(&mut app, KeyCode::Char('G'));
        assert!(matches!(app.mode, Mode::GrowMap { .. }));
    }

    #[test]
    fn the_title_menu_has_sound() {
        let mut app = listening(Game::new(3));
        app.mode = Mode::Title { sel: super::super::TITLE_MENU.iter().position(|s| *s == "Sound").unwrap() };
        key(&mut app, KeyCode::Enter);
        assert!(matches!(app.mode, Mode::Sound { .. }));
        assert_eq!(app.music_mood(), Mood::Title, "the title's music plays under it");
        key(&mut app, KeyCode::Esc);
        assert!(matches!(app.mode, Mode::Title { .. }));
    }

    #[test]
    fn the_music_follows_the_screen_and_the_fort() {
        let mut app = listening(Game::new(4));
        assert_eq!(app.music_mood(), Mood::Calm);
        app.game.incoming = Some((app.game.tick + 500, app.game.neighbors[0].clone(), 12, "Snodub".into()));
        assert_eq!(app.music_mood(), Mood::War);
        app.mode = Mode::Arena;
        assert_eq!(app.music_mood(), Mood::Battle);
        app.mode = Mode::WorldGen;
        assert_eq!(app.music_mood(), Mood::WorldGen);
        app.mode = Mode::Normal;
        for _ in 0..12 {
            app.sound_frame();
        }
        let scene = app.audio.last_scene().expect("a scene was sent");
        assert_eq!(scene.mood, Mood::War);
    }

    #[test]
    fn a_spotlight_card_plays_its_stinger_once() {
        let mut app = listening(Game::new(5));
        app.game.spotlight_for_test("An artifact", "Urist has made a thing.");
        app.paused = false;
        let chimes = |app: &App| app.audio.sent().iter().filter(|m| matches!(m, Msg::Sfx(s) if s.kind == SfxKind::Chime)).count();
        for _ in 0..200 {
            app.frame();
            app.sound_frame();
        }
        assert!(app.spot.is_some() || chimes(&app) > 0);
        assert_eq!(chimes(&app), 1);
    }

    /// The sound's work each frame is small beside the frame (33 ms): a
    /// fortress with the overseer at work, looked at every frame.
    #[test]
    fn listening_costs_little_frame_time() {
        let mut g = crate::game::trial_game(21, crate::game::Style::Steady, 60);
        g.sounds.clear();
        let mut app = listening(g);
        app.paused = false;
        app.cursor = app.game.dwarves.iter().find(|d| d.alive).map_or(app.cursor, |d| d.pos);
        let t = std::time::Instant::now();
        let frames = 600;
        for _ in 0..frames {
            app.sound_frame();
        }
        let per = t.elapsed().as_secs_f64() * 1000.0 / frames as f64;
        assert!(per < 1.0, "sound takes {:.3} ms a frame", per);
        // And nothing in the game changed by listening.
        let tick = app.game.tick;
        app.sound_frame();
        assert_eq!(app.game.tick, tick);
    }
}
