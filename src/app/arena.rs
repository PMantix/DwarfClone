//! The arena's screens (WISHLIST G01): set up a matchup, watch the bout,
//! read how it went, rerun it many times for win rates, and keep the
//! matchups worth keeping in `saves/`. The fighting itself is
//! `game::arena`; the drawing is `ui::arena`.

use super::{App, Mode, SPEEDS};
use crate::game::arena::{self, Battle, Matchup, Tally, SETUP_ROWS};
use crossterm::event::{KeyCode, KeyEvent};

/// Which arena screen is up.
#[derive(Clone, Debug, PartialEq)]
pub enum View {
    Setup,
    /// The bout on the map (with the summary over it once it's over).
    Battle,
    /// The bout's whole combat log.
    Report { scroll: usize },
    /// Bouts run many times over, and the table of every rerun so far.
    Results,
    /// Saved matchups and the presets to pick from.
    Load { sel: usize },
}

/// How many bouts a rerun runs (cycled with `n`).
pub const RERUNS: [u32; 4] = [10, 50, 100, 500];

/// Bouts being rerun in the background of the results screen, a few each frame.
pub struct Batch {
    pub matchup: Matchup,
    pub n: u32,
    pub tally: Tally,
}

pub struct ArenaScreen {
    pub matchup: Matchup,
    /// The setup cursor: column (0 west, 1 east) and row (`arena::ROW_*`).
    pub side: usize,
    pub row: usize,
    pub view: View,
    pub battle: Option<Battle>,
    /// Index into `SPEEDS`.
    pub speed: usize,
    pub paused: bool,
    /// The fighter the camera follows (None: it goes where the blood is).
    pub follow: Option<usize>,
    /// Bouts watched so far (each rerun is the next seed).
    pub bouts: u64,
    /// Index into `RERUNS`.
    pub reruns: usize,
    pub batch: Option<Batch>,
    /// Every rerun so far: (matchup name, what came of it).
    pub history: Vec<(String, Tally)>,
    /// Who you think will win the bout you watch, and how your picks have gone (right, made).
    pub pick: Option<usize>,
    pub record: (u32, u32),
    /// The summary scrolled down this far.
    pub scroll: usize,
    /// The next preset `p` brings up.
    pub preset: usize,
    /// Ticks the summary has been up after the end (it waits a moment).
    pub ended: u32,
    pub flash: Option<String>,
}

impl ArenaScreen {
    pub fn new() -> ArenaScreen {
        ArenaScreen {
            matchup: Matchup::default(),
            side: 0,
            row: 0,
            view: View::Setup,
            battle: None,
            speed: 2,
            paused: false,
            follow: None,
            bouts: 0,
            reruns: 2,
            batch: None,
            history: Vec::new(),
            pick: None,
            record: (0, 0),
            scroll: 0,
            preset: 1,
            ended: 0,
            flash: None,
        }
    }

    /// Start watching the matchup's next bout.
    pub fn fight(&mut self) {
        let seed = self.matchup.seed.wrapping_add(self.bouts);
        self.bouts += 1;
        self.battle = Some(Battle::new(&self.matchup, seed));
        self.view = View::Battle;
        self.paused = false;
        self.follow = None;
        self.scroll = 0;
        self.ended = 0;
    }

    /// Rerun the matchup `RERUNS[reruns]` times, headless.
    pub fn rerun(&mut self) {
        self.batch = Some(Batch { matchup: self.matchup.clone(), n: RERUNS[self.reruns], tally: Tally::default() });
        self.view = View::Results;
    }

    /// Run bouts of the batch for about `budget` (all of them with None).
    pub fn work(&mut self, budget: Option<std::time::Duration>) {
        let t0 = std::time::Instant::now();
        let Some(b) = self.batch.as_mut() else { return };
        while b.tally.runs < b.n {
            let mut bout = Battle::new(&b.matchup, b.matchup.seed.wrapping_add(b.tally.runs as u64));
            bout.run();
            b.tally.add(&bout);
            if budget.map_or(false, |d| t0.elapsed() >= d) {
                return;
            }
        }
        let b = self.batch.take().unwrap();
        self.history.push((b.matchup.name.clone(), b.tally));
    }

    /// The bout on screen moves on a frame's worth of ticks.
    pub fn tick(&mut self) {
        let Some(b) = self.battle.as_mut() else { return };
        if b.outcome.is_some() {
            self.ended += 1;
            return;
        }
        if self.paused {
            return;
        }
        for _ in 0..SPEEDS[self.speed] {
            b.step();
            if b.outcome.is_some() {
                break;
            }
        }
        if b.outcome.is_some() {
            self.settle_pick();
            return;
        }
        // A followed fighter who falls: the camera finds the next on their side.
        if let Some(f) = self.follow {
            if !b.alive(f) {
                self.follow = (0..b.warriors.len()).find(|&i| b.warriors[i].side == b.warriors[f].side && b.standing(i));
            }
        }
    }

    /// The bout is over: was your pick right?
    fn settle_pick(&mut self) {
        let (Some(p), Some(b)) = (self.pick, self.battle.as_ref()) else { return };
        if let Some(o) = &b.outcome {
            self.record.1 += 1;
            if o.winner == Some(p) {
                self.record.0 += 1;
            }
        }
    }

    /// Follow the next fighter still alive after the one followed now.
    fn follow_next(&mut self, dir: i32) {
        let Some(b) = self.battle.as_ref() else { return };
        let n = b.warriors.len() as i32;
        let from = self.follow.map_or(if dir > 0 { -1 } else { n }, |f| f as i32);
        for k in 1..=n {
            let i = (from + dir * k).rem_euclid(n) as usize;
            if b.alive(i) {
                self.follow = Some(i);
                return;
            }
        }
        self.follow = None;
    }
}

impl App {
    /// From the title screen.
    pub fn open_arena(&mut self) {
        if self.arena.is_none() {
            self.arena = Some(Box::new(ArenaScreen::new()));
        }
        if let Some(a) = self.arena.as_mut() {
            a.view = View::Setup;
        }
        self.mode = Mode::Arena;
    }

    /// Each frame while the arena is up.
    pub fn arena_frame(&mut self) {
        let Some(a) = self.arena.as_mut() else { return };
        a.tick();
        if a.batch.is_some() {
            a.work(Some(std::time::Duration::from_millis(25)));
        }
    }

    pub fn key_arena(&mut self, k: KeyEvent) {
        let dir = self.save_dir.clone();
        let Some(a) = self.arena.as_mut() else {
            self.mode = Mode::Title { sel: 4 };
            return;
        };
        a.flash = None;
        match a.view.clone() {
            View::Setup => match k.code {
                KeyCode::Esc => self.mode = Mode::Title { sel: 4 },
                KeyCode::Up => a.row = (a.row + SETUP_ROWS - 1) % SETUP_ROWS,
                KeyCode::Down => a.row = (a.row + 1) % SETUP_ROWS,
                KeyCode::Tab => a.side = 1 - a.side.min(1),
                KeyCode::Left | KeyCode::Char('-') => a.matchup.adjust(a.side, a.row, -1),
                KeyCode::Right | KeyCode::Char('+') | KeyCode::Char('=') => a.matchup.adjust(a.side, a.row, 1),
                KeyCode::Enter => a.fight(),
                KeyCode::Char('r') => a.rerun(),
                KeyCode::Char('n') => a.reruns = (a.reruns + 1) % RERUNS.len(),
                KeyCode::Char('b') => {
                    a.pick = match a.pick {
                        None => Some(0),
                        Some(0) => Some(1),
                        _ => None,
                    }
                }
                KeyCode::Char('p') => {
                    let all = arena::presets();
                    a.matchup = all[a.preset % all.len()].clone();
                    a.preset += 1;
                    a.bouts = 0;
                }
                KeyCode::Char('?') => {
                    let seed = crate::app::random_seed();
                    a.matchup = arena::surprise(seed);
                    a.bouts = 0;
                }
                KeyCode::Char('s') => {
                    a.flash = Some(match arena::save_matchup(&dir, &a.matchup) {
                        Ok(p) => format!("Saved to {}.", p.display()),
                        Err(e) => format!("Couldn't save: {}", e),
                    });
                }
                KeyCode::Char('l') => a.view = View::Load { sel: 0 },
                _ => {}
            },
            View::Battle => {
                let over = a.battle.as_ref().map_or(true, |b| b.outcome.is_some());
                match k.code {
                    KeyCode::Esc => a.view = View::Setup,
                    KeyCode::Char(' ') => a.paused = !a.paused,
                    KeyCode::Char('+') | KeyCode::Char('=') => a.speed = (a.speed + 1).min(SPEEDS.len() - 1),
                    KeyCode::Char('-') => a.speed = a.speed.saturating_sub(1),
                    KeyCode::Char('.') => {
                        if let Some(b) = a.battle.as_mut() {
                            b.step();
                            if b.outcome.is_some() {
                                a.settle_pick();
                            }
                        }
                    }
                    KeyCode::Tab | KeyCode::Right => a.follow_next(1),
                    KeyCode::BackTab | KeyCode::Left => a.follow_next(-1),
                    KeyCode::Char('f') => a.follow = None,
                    KeyCode::Char('v') => a.view = View::Report { scroll: 0 },
                    KeyCode::Char('e') if !over => {
                        if let Some(b) = a.battle.as_mut() {
                            b.run();
                        }
                        a.settle_pick();
                    }
                    KeyCode::Up if over => a.scroll = a.scroll.saturating_sub(1),
                    KeyCode::Down if over => a.scroll += 1,
                    KeyCode::Char('r') if over => a.fight(),
                    KeyCode::Char('n') if over => a.rerun(),
                    KeyCode::Enter if over => a.view = View::Setup,
                    _ => {}
                }
            }
            View::Report { scroll } => match k.code {
                KeyCode::Up => a.view = View::Report { scroll: scroll + 1 },
                KeyCode::Down => a.view = View::Report { scroll: scroll.saturating_sub(1) },
                KeyCode::PageUp => a.view = View::Report { scroll: scroll + 15 },
                KeyCode::PageDown => a.view = View::Report { scroll: scroll.saturating_sub(15) },
                _ => a.view = View::Battle,
            },
            View::Results => match k.code {
                KeyCode::Char('r') if a.batch.is_none() => a.rerun(),
                KeyCode::Char('n') => a.reruns = (a.reruns + 1) % RERUNS.len(),
                KeyCode::Esc | KeyCode::Enter => {
                    a.batch = None;
                    a.view = View::Setup;
                }
                _ => {}
            },
            View::Load { sel } => {
                let list = load_list(&dir);
                let n = list.len().max(1);
                match k.code {
                    KeyCode::Up => a.view = View::Load { sel: (sel + n - 1) % n },
                    KeyCode::Down => a.view = View::Load { sel: (sel + 1) % n },
                    KeyCode::Enter => {
                        if let Some((m, _)) = list.into_iter().nth(sel) {
                            a.flash = Some(format!("Loaded \"{}\".", m.name));
                            a.matchup = m;
                            a.bouts = 0;
                        }
                        a.view = View::Setup;
                    }
                    _ => a.view = View::Setup,
                }
            }
        }
    }
}

/// What the load screen offers: saved matchups (marked true), then the presets.
pub fn load_list(dir: &std::path::Path) -> Vec<(Matchup, bool)> {
    arena::saved_matchups(dir).into_iter().map(|m| (m, true)).chain(arena::presets().into_iter().map(|m| (m, false))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn key(app: &mut App, c: KeyCode) {
        app.on_key(KeyEvent::new(c, KeyModifiers::NONE));
    }

    #[test]
    fn the_title_screen_opens_the_arena_and_a_matchup_is_set_up_and_fought() {
        let mut app = App::new(1);
        app.save_dir = std::env::temp_dir().join(format!("dwarfclone-arena-app-{}", std::process::id()));
        app.mode = Mode::Title { sel: 4 };
        key(&mut app, KeyCode::Enter);
        assert_eq!(app.mode, Mode::Arena);
        // Fewer goblins, a bridge to fight on.
        key(&mut app, KeyCode::Tab);
        key(&mut app, KeyCode::Down);
        key(&mut app, KeyCode::Left);
        for _ in 0..7 {
            key(&mut app, KeyCode::Down);
        }
        key(&mut app, KeyCode::Right);
        key(&mut app, KeyCode::Right);
        key(&mut app, KeyCode::Right);
        let a = app.arena.as_ref().unwrap();
        assert_eq!(a.matchup.sides[1].count, 5);
        assert_eq!(a.matchup.field, arena::Field::Bridge);
        key(&mut app, KeyCode::Char('b'));
        key(&mut app, KeyCode::Enter);
        assert_eq!(app.arena.as_ref().unwrap().view, View::Battle);
        for _ in 0..2000 {
            app.frame();
            if app.arena.as_ref().unwrap().battle.as_ref().unwrap().outcome.is_some() {
                break;
            }
        }
        let a = app.arena.as_ref().unwrap();
        assert!(a.battle.as_ref().unwrap().outcome.is_some(), "the bout never ended");
        assert_eq!(a.record.1, 1, "the pick wasn't settled");
        key(&mut app, KeyCode::Enter);
        assert_eq!(app.arena.as_ref().unwrap().view, View::Setup);
        key(&mut app, KeyCode::Esc);
        assert!(matches!(app.mode, Mode::Title { .. }));
    }
}
