//! Milestone 2's keys (W0-B): new Normal-mode keys go in `key_m2`, which
//! `key_normal` asks first, and each new screen's keys in its own function
//! here, so the M2 packages don't crowd `app.rs`. Wave 0 gives the screens
//! as placeholders that Esc leaves.
//!
//! Who fills what: `key_overseer_traits` (P1, O01), `key_announce` and the
//! step keys (P6, O05), `key_journal` (P17, O07), `key_family` (P16, O06).

use super::{App, Mode};
use crossterm::event::{KeyCode, KeyEvent};

/// Announcement control (O05, UI-07; P6 fills it in): for each kind of
/// announcement, whether it pauses, recentres or hides.
#[derive(Clone, Debug, Default)]
#[allow(dead_code)] // wave 0: P6 fills it in
pub struct Announce {
    pub rules: Vec<(crate::game::logkind::LogKind, AnnounceRule)>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnnounceRule {
    pub pause: bool,
    pub recenter: bool,
    pub hide: bool,
}

impl App {
    /// Milestone 2's Normal-mode keys, asked before the rest: true if the
    /// key was taken. One arm per key, from the package that binds it.
    pub(super) fn key_m2(&mut self, k: KeyEvent) -> bool {
        #[allow(clippy::match_single_binding)]
        match k.code {
            _ => false,
        }
    }

    /// Keys on the M2 screens: one arm per screen.
    pub(super) fn key_m2_screen(&mut self, k: KeyEvent) {
        match self.mode {
            Mode::OverseerTraits { sel } => self.key_overseer_traits(k, sel),
            Mode::Journal { year } => self.key_journal(k, year),
            Mode::Family { id, sel } => self.key_family(k, id, sel),
            Mode::Announce { sel } => self.key_announce(k, sel),
            _ => {}
        }
    }

    /// Picking the overseer's traits by hand (O01): Enter ticks or unticks
    /// the trait under the cursor, at most two and none that clash (the
    /// game says why not); Esc goes back to the Autoplay screen. (Space is
    /// pause everywhere.)
    fn key_overseer_traits(&mut self, k: KeyEvent, sel: usize) {
        use crate::game::personality::Trait;
        let n = Trait::ALL.len();
        match k.code {
            KeyCode::Up => self.mode = Mode::OverseerTraits { sel: (sel + n - 1) % n },
            KeyCode::Down => self.mode = Mode::OverseerTraits { sel: (sel + 1) % n },
            KeyCode::Enter => {
                let Some(o) = self.game.overseer.as_ref() else { return };
                let t = Trait::ALL[sel.min(n - 1)];
                let mut traits = o.traits.clone();
                if traits.contains(&t) {
                    traits.retain(|&x| x != t);
                } else {
                    traits.push(t);
                }
                self.flash = Some(match self.game.set_traits(traits) {
                    Ok(m) | Err(m) => m,
                });
            }
            KeyCode::Esc => {
                let style = self.game.overseer.as_ref().map(|o| o.style);
                let row = crate::game::Style::ALL.iter().position(|&s| Some(s) == style).map_or(0, |i| i + 1);
                self.mode = Mode::Autoplay { sel: row };
            }
            _ => {}
        }
    }

    /// The overseer's journal (O07).
    fn key_journal(&mut self, k: KeyEvent, _year: usize) {
        if k.code == KeyCode::Esc {
            self.mode = Mode::Normal;
        }
    }

    /// A dwarf's family tree (O06).
    fn key_family(&mut self, k: KeyEvent, id: u32, _sel: usize) {
        if k.code == KeyCode::Esc {
            self.mode = Mode::Life { id, scroll: 0 };
        }
    }

    /// Announcement control (O05).
    fn key_announce(&mut self, k: KeyEvent, _sel: usize) {
        if k.code == KeyCode::Esc {
            self.mode = Mode::Reports { filter: 0, scroll: 0 };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    /// Each M2 screen is a placeholder that draws and that Esc leaves.
    #[test]
    fn the_m2_screens_open_and_close() {
        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        for m in [Mode::Journal { year: 0 }, Mode::Family { id: 1, sel: 0 }, Mode::Announce { sel: 0 }] {
            let mut app = App::with_game_for_tests(crate::game::Game::new(5));
            app.mode = m.clone();
            let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40)).unwrap();
            term.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
            let shown: String = term.backend().buffer().content().iter().map(|c| c.symbol()).collect();
            assert!(shown.contains("Coming soon"), "{:?}", m);
            app.on_key(esc);
            assert_ne!(app.mode, m);
        }
    }

    /// O01: `A`, `t`, two traits ticked, Esc: the overseer is as chosen;
    /// a third, or a clashing pair, is refused.
    #[test]
    fn pick_the_overseers_traits_by_hand() {
        use crate::game::personality::Trait;
        let key = |c: KeyCode| KeyEvent::new(c, KeyModifiers::NONE);
        let mut app = App::with_game_for_tests(crate::game::Game::new(5));
        app.game.set_overseer(Some(crate::game::Style::Steady));
        app.game.overseer.as_mut().unwrap().traits.clear();
        app.mode = Mode::Autoplay { sel: 0 };
        app.on_key(key(KeyCode::Char('t')));
        assert_eq!(app.mode, Mode::OverseerTraits { sel: 0 });
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40)).unwrap();
        // Cautious (row 0), then Bold (row 1) is refused, then Vain (row 2).
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.game.overseer.as_ref().unwrap().traits, vec![Trait::Cautious], "{:?} {:?}", app.flash, app.mode);
        app.on_key(key(KeyCode::Down));
        assert_eq!(app.mode, Mode::OverseerTraits { sel: 1 });
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.game.overseer.as_ref().unwrap().traits, vec![Trait::Cautious], "{:?}", app.flash);
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Enter));
        term.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
        let shown: String = term.backend().buffer().content().iter().map(|c| c.symbol()).collect();
        assert!(shown.contains("☑ vain"), "{}", shown);
        app.on_key(key(KeyCode::Esc));
        assert_eq!(app.game.overseer.as_ref().unwrap().traits, vec![Trait::Cautious, Trait::Vain]);
        assert!(matches!(app.mode, Mode::Autoplay { .. }));
    }
}
