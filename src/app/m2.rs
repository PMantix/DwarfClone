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

/// What the player is naming (O11): a zone or a building.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameTarget {
    Zone(u32),
    Building(u32),
}

/// The name being typed for a hall or a building (O11, `N` in look or a
/// building's query). While it is open every key goes to it.
#[derive(Clone, Debug, PartialEq)]
pub struct Naming {
    pub target: NameTarget,
    pub text: String,
}

impl App {
    /// `N` in look or query: start naming the building picked in look (or
    /// queried), else the zone under the cursor, else its building.
    pub(super) fn start_naming(&mut self, bld: Option<u32>) {
        let g = &self.game;
        let target = match bld {
            Some(b) => Some(NameTarget::Building(b)),
            None => g.zone_at(self.cursor).map(|z| NameTarget::Zone(z.id)).or_else(|| {
                let b = g.world.get(self.cursor).map_or(0, |t| t.building);
                (b != 0).then_some(NameTarget::Building(b))
            }),
        };
        let Some(target) = target else {
            self.flash = Some("Nothing here to name: stand on a zone or a building.".into());
            return;
        };
        let text = match target {
            NameTarget::Zone(id) => g.zones.get(&id).and_then(|z| z.name.clone()),
            NameTarget::Building(id) => g.buildings.get(&id).and_then(|b| b.name.clone()),
        }
        .unwrap_or_default();
        self.naming = Some(Naming { target, text });
        self.naming_prompt();
    }

    fn naming_prompt(&mut self) {
        let Some(n) = &self.naming else { return };
        let what = match n.target {
            NameTarget::Zone(id) => self.game.zones.get(&id).map_or("zone".to_string(), |z| z.kind.name().to_lowercase()),
            NameTarget::Building(id) => self.game.buildings.get(&id).map_or("building".to_string(), |b| b.kind.name().to_lowercase()),
        };
        self.flash = Some(format!("Name this {}: {}_   (Enter: done, Esc: cancel)", what, n.text));
    }

    /// `B` at a zone box's second corner: each enclosed room in the box its
    /// own zone (O11).
    pub(super) fn zone_bulk(&mut self, tool: super::ZoneTool, corner: Option<crate::geom::Pos>) {
        let (Some(a), Some(kind)) = (corner, tool.kind()) else {
            self.flash = Some("Mark the box's first corner, then B: each room in it gets its own zone.".into());
            return;
        };
        let (zones, tiles) = self.game.add_zone_rooms(a, self.cursor, kind);
        self.flash = Some(match zones {
            0 => "Zones need clear, revealed floor not already zoned.".into(),
            _ => format!("{} {} zones, {} tiles in all.", zones, kind.name(), tiles),
        });
        self.mode = Mode::Zone { tool, corner: None };
    }

    /// Typing a name: letters, Backspace, Enter to give it, Esc to leave it.
    pub(super) fn key_naming(&mut self, k: KeyEvent) {
        let Some(mut n) = self.naming.take() else { return };
        match k.code {
            KeyCode::Esc => {
                self.flash = Some("Left as it was.".into());
                return;
            }
            KeyCode::Enter => {
                let ok = match n.target {
                    NameTarget::Zone(id) => self.game.name_zone(id, &n.text),
                    NameTarget::Building(id) => self.game.name_building(id, &n.text),
                };
                let name = n.text.trim();
                self.flash = Some(match (ok, name.is_empty()) {
                    (false, _) => "It is gone.".into(),
                    (true, true) => "Its name is forgotten.".into(),
                    (true, false) => format!("It shall be known as {}.", name),
                });
                return;
            }
            KeyCode::Backspace => {
                n.text.pop();
            }
            KeyCode::Char(c) if !c.is_control() && n.text.chars().count() < 40 => n.text.push(c),
            _ => {}
        }
        self.naming = Some(n);
        self.naming_prompt();
    }

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

    /// Picking the overseer's traits by hand (O01).
    fn key_overseer_traits(&mut self, k: KeyEvent, _sel: usize) {
        if k.code == KeyCode::Esc {
            self.mode = Mode::Autoplay { sel: 0 };
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
        for m in [Mode::OverseerTraits { sel: 0 }, Mode::Journal { year: 0 }, Mode::Family { id: 1, sel: 0 }, Mode::Announce { sel: 0 }] {
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
}
