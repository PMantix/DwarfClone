//! Adventurer mode's screens (G03). The mode is a self-contained sub-app,
//! like the arena: `App.adv` holds the adventure and its view, its keys are
//! routed here before the fort's, and `ui/adventure.rs` draws it.
//!
//! One key function per view, each in its own `impl App` block, so the
//! wave-1 packages each edit only their own: `key_create` (A1), `key_local`
//! (A2), `key_travel` (A3), `key_end` (A10), `key_talk` (A7),
//! `key_company` (A9).

use super::{App, Mode, TITLE_ADVENTURE};
use crate::game::adv::turn::{Action, Outcome};
use crate::game::adv::{self, Adventure, View};
use crate::jobs::JobDone;
use crate::overworld::Overworld;
use crossterm::event::{KeyCode, KeyEvent};

/// The adventure on screen.
pub struct AdvScreen {
    pub adv: Adventure,
    pub view: View,
    /// A line for the player (refusals, interrupts).
    pub flash: Option<String>,
}

impl AdvScreen {
    pub fn new(adv: Adventure) -> AdvScreen {
        let view = if adv.over.is_some() { View::Dead } else if adv.visit.is_some() { View::Local } else { View::Travel };
        AdvScreen { adv, view, flash: None }
    }
}

impl App {
    /// From the title screen: a world made from the seed, and a hero set out
    /// from home (A1's character maker comes in front of this).
    pub fn open_adventure(&mut self) {
        self.start_adventure_seeded(self.seed);
    }

    /// An adventure in a fresh world from `seed` (also the window's `--adventure`).
    pub fn start_adventure_seeded(&mut self, seed: u64) {
        self.run_job("Setting out", move |report| {
            let ow = Overworld::generate_with(seed, report);
            report("Setting out", 0.95);
            Ok(JobDone::Visited(Box::new(adv::quick_start(ow, seed))))
        });
    }

    /// The game-over page's `a` (after the save): an adventurer into this
    /// fallen fort's world.
    pub fn adventure_in_this_world(&mut self) {
        let ow = std::mem::replace(&mut self.game.overworld, Overworld::empty());
        let seed = self.seed ^ 0xA0;
        self.game = crate::game::Game::placeholder(Overworld::empty());
        self.run_job("Setting out", move |_| Ok(JobDone::Visited(Box::new(adv::quick_start(ow, seed)))));
    }

    /// An adventure is ready (begun, a site entered, or loaded).
    pub fn adventure_ready(&mut self, a: Box<Adventure>) {
        self.adv = Some(Box::new(AdvScreen::new(*a)));
        self.mode = Mode::Adventure;
    }

    /// Each frame while the adventure is up: nothing runs while waiting for
    /// a key (A2 runs long acts a slice at a time here).
    pub fn adv_frame(&mut self) {}

    /// Every key while the adventure is up, by view.
    pub fn key_adventure(&mut self, k: KeyEvent) {
        let Some(s) = self.adv.as_mut() else {
            self.mode = Mode::Title { sel: TITLE_ADVENTURE };
            return;
        };
        s.flash = None;
        match s.view.clone() {
            View::Create => self.key_create(k),
            View::Local => self.key_local(k),
            View::Travel => self.key_travel(k),
            View::Dead => self.key_end(k),
            View::Talk { .. } => self.key_talk(k),
            View::Company => self.key_company(k),
            _ => {
                if k.code == KeyCode::Esc {
                    if let Some(s) = self.adv.as_mut() {
                        s.view = View::Local;
                    }
                }
            }
        }
    }

    /// Leave the adventure for the title (A11 saves first).
    fn adventure_to_title(&mut self) {
        self.adv = None;
        self.mode = Mode::Title { sel: TITLE_ADVENTURE };
    }
}

/// A direction from a movement key: arrows, numpad digits (as characters,
/// in the terminal and the window alike) and Home/End/PgUp/PgDn diagonals.
pub fn step_of(k: &KeyEvent) -> Option<(i32, i32)> {
    Some(match k.code {
        KeyCode::Up | KeyCode::Char('8') => (0, -1),
        KeyCode::Down | KeyCode::Char('2') => (0, 1),
        KeyCode::Left | KeyCode::Char('4') => (-1, 0),
        KeyCode::Right | KeyCode::Char('6') => (1, 0),
        KeyCode::Home | KeyCode::Char('7') => (-1, -1),
        KeyCode::PageUp | KeyCode::Char('9') => (1, -1),
        KeyCode::End | KeyCode::Char('1') => (-1, 1),
        KeyCode::PageDown | KeyCode::Char('3') => (1, 1),
        _ => return None,
    })
}

impl App {
    /// The character maker (A1).
    fn key_create(&mut self, k: KeyEvent) {
        if k.code == KeyCode::Esc {
            self.adventure_to_title();
        }
    }
}

impl App {
    /// The local map: move, bump to fight, wait (A2 adds the rest).
    fn key_local(&mut self, k: KeyEvent) {
        if k.code == KeyCode::Esc {
            return self.adventure_to_title();
        }
        let Some(s) = self.adv.as_mut() else { return };
        let Some(v) = s.adv.visit.as_mut() else { return };
        let act = if let Some((dx, dy)) = step_of(&k) {
            Action::Move(dx, dy)
        } else {
            match k.code {
                KeyCode::Char('.') | KeyCode::Char('5') => Action::Wait(10),
                KeyCode::Char('<') => Action::Climb(1),
                KeyCode::Char('>') => Action::Climb(-1),
                _ => return,
            }
        };
        let h = v.hero;
        match v.g.act(h, act) {
            Outcome::Done { .. } => {}
            Outcome::Interrupted { why, .. } | Outcome::Refused(why) => s.flash = Some(why),
            Outcome::Died => s.view = View::Dead,
        }
    }
}

impl App {
    /// The world map (A3).
    fn key_travel(&mut self, k: KeyEvent) {
        if k.code == KeyCode::Esc {
            self.adventure_to_title();
        }
    }
}

impl App {
    /// The death page (A10).
    fn key_end(&mut self, k: KeyEvent) {
        if matches!(k.code, KeyCode::Esc | KeyCode::Char('t')) {
            self.adventure_to_title();
        }
    }
}

impl App {
    /// A conversation (A7).
    fn key_talk(&mut self, k: KeyEvent) {
        if k.code == KeyCode::Esc {
            if let Some(s) = self.adv.as_mut() {
                s.view = View::Local;
            }
        }
    }
}

impl App {
    /// Companions' orders (A9).
    fn key_company(&mut self, k: KeyEvent) {
        if k.code == KeyCode::Esc {
            if let Some(s) = self.adv.as_mut() {
                s.view = View::Local;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn key(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::NONE)
    }

    /// Wait out the loading screen.
    fn settle(app: &mut App) {
        let t = std::time::Instant::now();
        while app.mode == Mode::Loading && t.elapsed().as_secs() < 120 {
            app.frame();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    #[test]
    fn the_title_offers_an_adventure() {
        let mut app = App::new(5);
        app.save_dir = std::env::temp_dir().join("dwarfclone-no-saves-here");
        app.mode = Mode::Title { sel: 0 };
        while app.mode != (Mode::Title { sel: TITLE_ADVENTURE }) {
            app.on_key(key(KeyCode::Down));
        }
        app.on_key(key(KeyCode::Enter));
        settle(&mut app);
        assert_eq!(app.mode, Mode::Adventure);
        let s = app.adv.as_ref().unwrap();
        assert_eq!(s.view, View::Local);
        let v = s.adv.visit.as_ref().expect("in a site");
        assert_eq!(v.g.dwarves[v.hero].id, adv::HERO_ID);
        assert!(s.adv.ow().w > 1, "a real world");
        // Keys move the hero: each direction by arrow and by numpad digit.
        for (k, (dx, dy)) in [(KeyCode::Right, (1, 0)), (KeyCode::Char('4'), (-1, 0)), (KeyCode::Char('3'), (1, 1)), (KeyCode::Home, (-1, -1))] {
            let before = app.adv.as_ref().map(|s| s.adv.hero_dwarf().pos).unwrap();
            app.on_key(key(k));
            let after = app.adv.as_ref().map(|s| s.adv.hero_dwarf().pos).unwrap();
            assert_eq!((after.x - before.x, after.y - before.y), (dx, dy), "{:?}", k);
        }
        let t0 = app.adv.as_ref().unwrap().adv.now();
        app.on_key(key(KeyCode::Char('.')));
        assert_eq!(app.adv.as_ref().unwrap().adv.now(), t0 + 10);
        // Drawn, at a small terminal and a large one.
        for (w, h) in [(80, 24), (130, 40)] {
            let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
            term.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
            let text: String = term.backend().buffer().content().iter().map(|c| c.symbol().to_string()).collect();
            assert!(text.contains('☺'), "the hero is drawn at {}x{}", w, h);
        }
        app.on_key(key(KeyCode::Esc));
        assert_eq!(app.mode, Mode::Title { sel: TITLE_ADVENTURE });
    }
}
