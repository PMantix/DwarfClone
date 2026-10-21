//! Milestone 2's keys (W0-B): new Normal-mode keys go in `key_m2`, which
//! `key_normal` asks first, and each new screen's keys in its own function
//! here, so the M2 packages don't crowd `app.rs`. Wave 0 gives the screens
//! as placeholders that Esc leaves.
//!
//! Who fills what: `key_overseer_traits` (P1, O01), `key_announce` and the
//! step keys (P6, O05), `key_journal` (P17, O07), `key_family` (P16, O06).

use super::{App, Mode};
use crate::game::logkind::LogKind;
use crate::game::sound_reports::Earshot;
use crate::game::DAY;
use crossterm::event::{KeyCode, KeyEvent};
use serde::{Deserialize, Serialize};

/// Announcement control (O05, UI-07): for each kind of announcement,
/// whether it pauses, recentres or hides. An app setting, kept in
/// `saves/announce.json` rather than in the fortress's save.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Announce {
    #[serde(default)]
    pub rules: Vec<(LogKind, AnnounceRule)>,
    /// Read from the file yet (the first fortress frame reads it).
    #[serde(skip)]
    pub loaded: bool,
    /// Ticks run by the step under way, for its "stepped" line.
    #[serde(skip)]
    pub stepped: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AnnounceRule {
    pub pause: bool,
    pub recenter: bool,
    pub hide: bool,
}

/// The step keys while paused (O05, UI-32): a tick, an hour, a day.
pub const STEP_TICK: u32 = 1;
pub const STEP_HOUR: u32 = (DAY / 24) as u32;
pub const STEP_DAY: u32 = DAY as u32;
/// The most ticks a step runs in one frame, so a day's step stays smooth.
const STEP_CHUNK: u32 = 400;

impl Announce {
    const FILE: &'static str = "announce.json";

    /// What to do with announcements of `kind`. Unset kinds show, and
    /// neither pause nor move the camera, as before there was a choice.
    pub fn rule(&self, kind: LogKind) -> AnnounceRule {
        self.rules.iter().find(|r| r.0 == kind).map_or(AnnounceRule::default(), |r| r.1)
    }

    pub fn set(&mut self, kind: LogKind, rule: AnnounceRule) {
        match self.rules.iter_mut().find(|r| r.0 == kind) {
            Some(r) => r.1 = rule,
            None => self.rules.push((kind, rule)),
        }
    }

    pub fn load(dir: &std::path::Path) -> Announce {
        let a: Announce = std::fs::read_to_string(dir.join(Self::FILE)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        Announce { loaded: true, ..a }
    }

    pub fn save(&self, dir: &std::path::Path) {
        if std::fs::create_dir_all(dir).is_ok() {
            if let Ok(s) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(dir.join(Self::FILE), s);
            }
        }
    }
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
            KeyCode::Char('s') => self.key_step(STEP_TICK),
            KeyCode::Char('T') => self.key_step(STEP_HOUR),
            KeyCode::Char('D') => self.key_step(STEP_DAY),
            // O07: the overseer's journal.
            KeyCode::Char('J') => {
                self.open_journal();
                true
            }
            KeyCode::Enter => self.go_to_sound(),
            _ => false,
        }
    }

    /// What the player can see now: the map's view and level.
    pub fn earshot(&self) -> Earshot {
        Earshot { x: self.view_x, y: self.view_y, w: self.map_rect.width.max(1) as i32, h: self.map_rect.height.max(1) as i32, z: self.cursor.z }
    }

    /// A step key (O05, UI-32): pause if running, then run `ticks` more
    /// and stop again.
    fn key_step(&mut self, ticks: u32) -> bool {
        self.paused = true;
        self.step_pending = self.step_pending.saturating_add(ticks);
        true
    }

    /// Enter: the camera goes to the latest sound heard off screen (O05).
    fn go_to_sound(&mut self) -> bool {
        match self.game.last_heard(self.earshot()) {
            Some((p, s)) => {
                self.look_at(p);
                self.flash = Some(format!("{} You go to look.", s));
            }
            None => self.flash = Some("Nothing heard off screen lately.".into()),
        }
        true
    }

    /// Centre the view and cursor on `p`.
    fn look_at(&mut self, p: crate::geom::Pos) {
        if !self.game.world.in_bounds(p) {
            return;
        }
        self.free_view = false;
        self.cursor = p;
        self.view_x = p.x - self.map_rect.width as i32 / 2;
        self.view_y = p.y - self.map_rect.height as i32 / 2;
    }

    /// Each frame, while paused: run what is left of a step, a chunk at a
    /// time, and say when it is done. News that would pause the game (a
    /// death, while you play) cuts the step short.
    pub(super) fn step_frame(&mut self) {
        if !self.paused || self.step_pending == 0 {
            return;
        }
        let run = self.step_pending.min(STEP_CHUNK);
        let mut ran = 0;
        while ran < run {
            self.game.step();
            ran += 1;
            if self.game.pause_request {
                self.game.pause_request = false;
                if self.game.overseer.is_none() {
                    self.step_pending = ran;
                    break;
                }
            }
        }
        self.step_pending -= ran;
        self.announce.stepped = self.announce.stepped.saturating_add(ran);
        if self.step_pending == 0 {
            let n = std::mem::take(&mut self.announce.stepped);
            self.flash = Some(match n {
                1 => "Stepped a tick (s a tick, T an hour, D a day; Space runs).".to_string(),
                n if n == STEP_HOUR => "Stepped an hour.".to_string(),
                n if n == STEP_DAY => "Stepped a day.".to_string(),
                n => format!("Stepped {} ticks.", n),
            });
        }
    }

    /// After the frame's ticks: honour the announcement rules for the
    /// entries added since `mark` (the log's length before them).
    pub(super) fn announce_frame(&mut self, mark: usize) {
        if !self.announce.loaded {
            self.announce = Announce::load(&self.save_dir);
        }
        let log = &self.game.log;
        // The log drops its oldest 500 when it grows long.
        let from = if log.len() < mark { mark.saturating_sub(500) } else { mark };
        let (mut pause, mut go) = (false, None);
        for e in log.iter().skip(from) {
            let r = self.announce.rule(e.kind());
            pause |= r.pause;
            if r.recenter {
                go = e.at.or(go);
            }
        }
        if let Some(p) = go {
            self.look_at(p);
        }
        if pause {
            self.paused = true;
            self.step_pending = 0;
        }
    }

    /// Keys on the M2 screens: one arm per screen.
    pub(super) fn key_m2_screen(&mut self, k: KeyEvent) {
        match self.mode {
            Mode::OverseerTraits { sel } => self.key_overseer_traits(k, sel),
            Mode::Journal { year, scroll } => self.key_journal(k, year, scroll),
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

    /// `J` (or `j` on the Autoplay screen): the overseer's journal, open at
    /// the page being written, at its latest entries.
    pub(super) fn open_journal(&mut self) {
        let n = self.game.overseer.as_ref().map_or(0, |o| o.journal.len());
        if n == 0 {
            self.flash = Some(match self.game.overseer {
                None => "No overseer keeps a journal: hand the fortress over (A) first.".into(),
                Some(_) => "The overseer has not written in its journal yet.".into(),
            });
            return;
        }
        let rows = crate::ui::overseer::journal_rows(self, n - 1);
        self.mode = Mode::Journal { year: n - 1, scroll: rows.saturating_sub(20) };
    }

    /// The overseer's journal (O07): Left and Right turn the pages, Up,
    /// Down, PgUp, PgDn, Home and End read down a page; Esc closes it.
    fn key_journal(&mut self, k: KeyEvent, page: usize, scroll: usize) {
        let n = self.game.overseer.as_ref().map_or(0, |o| o.journal.len()).max(1);
        let page = page.min(n - 1);
        let max = crate::ui::overseer::journal_rows(self, page).saturating_sub(10);
        let scroll = scroll.min(max);
        self.mode = match k.code {
            KeyCode::Left => Mode::Journal { year: page.saturating_sub(1), scroll: 0 },
            KeyCode::Right => Mode::Journal { year: (page + 1).min(n - 1), scroll: 0 },
            KeyCode::Up => Mode::Journal { year: page, scroll: scroll.saturating_sub(1) },
            KeyCode::Down => Mode::Journal { year: page, scroll: (scroll + 1).min(max) },
            KeyCode::PageUp => Mode::Journal { year: page, scroll: scroll.saturating_sub(15) },
            KeyCode::PageDown => Mode::Journal { year: page, scroll: (scroll + 15).min(max) },
            KeyCode::Home => Mode::Journal { year: page, scroll: 0 },
            KeyCode::End => Mode::Journal { year: page, scroll: max },
            KeyCode::Esc | KeyCode::Char('J') | KeyCode::Char('q') => Mode::Normal,
            _ => Mode::Journal { year: page, scroll },
        };
    }

    /// A dwarf's family tree (O06): the keys are in `ui/family.rs`.
    fn key_family(&mut self, k: KeyEvent, id: u32, sel: usize) {
        self.key_family_tree(k, id, sel);
    }

    /// Announcement control (O05): a row per kind; `p` pauses, `c` moves
    /// the camera, `h` hides or shows. Saved as soon as changed.
    fn key_announce(&mut self, k: KeyEvent, sel: usize) {
        let n = LogKind::ALL.len();
        let kind = LogKind::ALL[sel.min(n - 1)];
        let was = self.announce.rule(kind);
        let mut r = was;
        match k.code {
            KeyCode::Up => self.mode = Mode::Announce { sel: (sel + n - 1) % n },
            KeyCode::Down => self.mode = Mode::Announce { sel: (sel + 1) % n },
            KeyCode::Char('p') => r.pause = !r.pause,
            KeyCode::Char('c') => r.recenter = !r.recenter,
            KeyCode::Char('h') | KeyCode::Enter => r.hide = !r.hide,
            KeyCode::Esc => self.mode = Mode::Reports { filter: 0, scroll: 0 },
            _ => {}
        }
        if r != was {
            self.announce.set(kind, r);
            self.announce.loaded = true;
            self.announce.save(&self.save_dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    /// Each M2 screen draws, and Esc leaves it.
    #[test]
    fn the_m2_screens_open_and_close() {
        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let id = crate::game::Game::new(5).dwarves[0].id;
        for m in [Mode::Family { id, sel: 0 }] {
            let mut app = App::with_game_for_tests(crate::game::Game::new(5));
            app.mode = m.clone();
            let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40)).unwrap();
            term.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
            let shown: String = term.backend().buffer().content().iter().map(|c| c.symbol()).collect();
            assert!(shown.contains("The family of"), "{:?}", m);
            app.on_key(esc);
            assert_ne!(app.mode, m);
        }
    }

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    /// A fort on screen, paused, saving its settings to a scratch folder.
    fn fort(tag: &str) -> App {
        let mut app = App::with_game_for_tests(crate::game::Game::new(5));
        app.mode = Mode::Normal;
        app.map_rect = ratatui::layout::Rect::new(1, 2, 80, 30);
        app.save_dir = std::env::temp_dir().join(format!("dwarfclone-{}-{}", tag, std::process::id()));
        app.game.spotlights.clear();
        app
    }

    fn screen(app: &mut App) -> String {
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40)).unwrap();
        term.draw(|f| crate::ui::draw(f, app)).unwrap();
        term.backend().buffer().content().iter().map(|c| c.symbol()).collect()
    }

    #[test]
    fn hidden_announcements_dont_show_and_paused_ones_pause() {
        let mut app = fort("announce");
        let row = |k: LogKind| LogKind::ALL.iter().position(|&x| x == k).unwrap();
        // From the reports screen, `a` opens the choices.
        app.mode = Mode::Reports { filter: 0, scroll: 0 };
        app.on_key(key('a'));
        assert_eq!(app.mode, Mode::Announce { sel: 0 });
        assert!(screen(&mut app).contains("Births and weddings"));
        // Hide building completions; pause on deaths.
        app.mode = Mode::Announce { sel: row(LogKind::Building) };
        app.on_key(key('h'));
        app.mode = Mode::Announce { sel: row(LogKind::Death) };
        app.on_key(key('p'));
        assert!(app.announce.rule(LogKind::Building).hide && app.announce.rule(LogKind::Death).pause);
        // Kept for the next game.
        let back = Announce::load(&app.save_dir);
        assert_eq!((back.rule(LogKind::Building), back.rule(LogKind::Death)), (app.announce.rule(LogKind::Building), app.announce.rule(LogKind::Death)));
        let _ = std::fs::remove_dir_all(&app.save_dir);
        // Hidden ones are not on the panel; the rest are.
        app.mode = Mode::Normal;
        let at = app.cursor;
        app.game.msg_at("The carpenter's workshop has been completed.", (160, 220, 255), Some(at), LogKind::Building);
        app.game.msg("Plain news of the day.", (200, 200, 200));
        let shown = screen(&mut app);
        assert!(shown.contains("Plain news of the day.") && !shown.contains("has been completed"));
        // A death pauses a running game.
        app.paused = false;
        let mark = app.game.log.len();
        app.game.msg_at("Urist McTest has bled to death.", (255, 60, 60), Some(at), LogKind::Death);
        app.announce_frame(mark);
        assert!(app.paused);
        // News of a kind that doesn't pause doesn't.
        app.paused = false;
        let mark = app.game.log.len();
        app.game.msg("A caravan from the mountainhomes has arrived.", (200, 200, 200));
        app.announce_frame(mark);
        assert!(!app.paused);
    }

    #[test]
    fn step_one_tick_and_one_hour() {
        let mut app = fort("step");
        // Hand it to the overseer so no news cuts a step short.
        app.game.set_overseer(Some(crate::game::Style::Steady));
        let t0 = app.game.tick;
        app.on_key(key('s'));
        app.frame();
        assert_eq!(app.game.tick, t0 + 1);
        assert!(app.paused);
        assert!(app.flash.as_deref().unwrap_or("").contains("Stepped a tick"), "{:?}", app.flash);
        // (No big moment's card takes the key.)
        app.spot = None;
        app.game.spotlights.clear();
        app.on_key(key('T'));
        app.frame();
        assert_eq!(app.game.tick, t0 + 1 + STEP_HOUR as u64);
        assert_eq!(app.flash.as_deref(), Some("Stepped an hour."));
        // A day runs over a few frames, then stops.
        app.spot = None;
        app.game.spotlights.clear();
        app.on_key(key('D'));
        for _ in 0..10 {
            app.frame();
        }
        assert_eq!(app.game.tick, t0 + 1 + STEP_HOUR as u64 + DAY);
        assert_eq!(app.flash.as_deref(), Some("Stepped a day."));
        assert!(app.paused);
        // Pressed while running, it pauses first.
        app.paused = false;
        app.spot = None;
        app.on_key(key('s'));
        assert!(app.paused && app.step_pending == 1);
    }

    #[test]
    fn a_far_fight_is_heard_not_seen() {
        let mut app = fort("sound");
        app.game.log.clear();
        screen(&mut app);
        let (w, h) = (app.map_rect.width as i32, app.map_rect.height as i32);
        app.view_x = 0;
        app.view_y = 0;
        app.cursor = crate::geom::Pos::new(w / 2, h / 2, app.cursor.z);
        // On another level, below: heard, not seen.
        let below = (0..app.cursor.z).rev().map(|z| crate::geom::Pos::new(10, 10, z)).find(|p| app.game.world.in_bounds(*p)).expect("a level below");
        app.game.msg_at("The goblin hacks Urist McTest in the head!", (255, 120, 100), Some(below), LogKind::Combat);
        let shown = screen(&mut app);
        assert!(!shown.contains("hacks Urist"), "told as seen");
        assert!(shown.contains("~ ") && shown.contains("from below") && shown.contains("(Enter)"), "not heard");
        // Enter goes to look; there, it is seen.
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.cursor, below);
        assert!(screen(&mut app).contains("hacks Urist"));
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

    /// O07: `J` opens the overseer's journal at the page being written,
    /// with its header and its dated entries; Left and Right turn the
    /// pages, Esc closes it; `j` opens it from the Autoplay screen too.
    #[test]
    fn the_journal_opens_beside_the_chronicle() {
        let key = |c: KeyCode| KeyEvent::new(c, KeyModifiers::NONE);
        let mut app = fort("journal");
        app.on_key(key(KeyCode::Char('J')));
        assert_eq!(app.mode, Mode::Normal, "no overseer, no journal");
        app.game.set_overseer(Some(crate::game::Style::Steady));
        for _ in 0..crate::game::DAY * 3 {
            app.game.step();
        }
        app.mode = Mode::Normal;
        app.on_key(key(KeyCode::Char('J')));
        assert!(matches!(app.mode, Mode::Journal { year: 0, .. }), "{:?} {:?}", app.mode, app.flash);
        let shown = screen(&mut app);
        let name = app.game.overseer.as_ref().unwrap().name.clone();
        assert!(shown.contains(&format!("The Journal of {}", name)), "{}", shown);
        assert!(shown.contains("first overseer of"), "{}", shown);
        assert!(shown.contains("Earlybloom"), "dated entries: {}", shown);
        // A second page (a new year) to turn to and back.
        let tick = app.game.tick;
        {
            let o = app.game.overseer.as_mut().unwrap();
            crate::game::Game::journal_for_test(o, tick + crate::game::YEAR, "A page for the new year.");
        }
        app.on_key(key(KeyCode::Char('J')));
        app.on_key(key(KeyCode::Char('J')));
        assert!(matches!(app.mode, Mode::Journal { year: 1, .. }), "{:?}", app.mode);
        assert!(screen(&mut app).contains("A page for the new year."));
        app.on_key(key(KeyCode::Left));
        assert!(matches!(app.mode, Mode::Journal { year: 0, scroll: 0 }));
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Esc));
        assert_eq!(app.mode, Mode::Normal);
        app.mode = Mode::Autoplay { sel: 1 };
        assert!(screen(&mut app).contains("j its journal"));
        app.on_key(key(KeyCode::Char('j')));
        assert!(matches!(app.mode, Mode::Journal { .. }));
    }
}
