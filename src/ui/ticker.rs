//! The event ticker: when the announcements can't be seen (a screen or
//! side pane is over them) or news comes faster than they show it, the
//! latest notable news (deaths, sieges, beasts, heroes, artifacts,
//! weddings, births) scrolls along the bottom edge of the screen, newest
//! first, with the overseer's latest note when its pane is hidden too.

use super::rgb;
use crate::app::App;
use crate::game::{Game, LogEntry, DAY};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use std::cell::Cell;

/// How far back news stays news (ticks), and how many items run at once.
pub const FRESH: u64 = DAY * 2;
pub const MAX_ITEMS: usize = 5;
/// The most announcements looked through a frame, however busy the days.
const SCAN: usize = 300;
/// Characters a second.
const SPEED: f64 = 9.0;

thread_local! {
    static CLOCK: Cell<f64> = const { Cell::new(0.0) };
}

/// Whether an announcement is news worth the ticker.
pub fn notable(e: &LogEntry) -> bool {
    let (r, g, b) = e.color;
    let t = &e.text;
    let routine = ["Spring", "Summer", "Autumn", "Winter"].iter().any(|s| t.starts_with(s)) || t.contains("cancels") || t.contains("Merchants");
    let red = r >= 200 && g <= 130 && b <= 130;
    let gold = r >= 230 && g >= 180 && b <= 130;
    let words = ["married", "born", "artifact", "siege", "Milestone", "legend", "hero", "slain", "has fallen", "migrants"];
    !routine && (red || gold || e.color == (255, 200, 220) || words.iter().any(|w| t.contains(w)))
}

/// The news to run: notable announcements of the last two days, newest
/// first, leaving out the last `shown` (still on the announcements panel).
pub fn items(g: &Game, shown: usize) -> Vec<(String, (u8, u8, u8))> {
    let since = g.tick.saturating_sub(FRESH);
    let mut v: Vec<(String, (u8, u8, u8))> = g
        .log
        .iter()
        .rev()
        .skip(shown)
        .take_while(|e| e.tick >= since)
        .take(SCAN)
        .filter(|e| notable(e))
        .take(MAX_ITEMS)
        .map(|e| (e.text.clone(), e.color))
        .collect();
    v.dedup_by(|a, b| a.0 == b.0);
    v
}

/// Draw the ticker on the bottom row of `full`, in its longest stretch free
/// of text (clear of key hints and counters), if there's news to run.
/// `panel` is how many announcements are on screen now (0 when hidden),
/// `notes_hidden` whether the overseer's notes are covered, and `reserve`
/// the columns kept free at the right for a hint.
pub(super) fn draw(f: &mut ratatui::Frame, app: &App, full: Rect, panel: usize, notes_hidden: bool, reserve: u16) {
    let g = &app.game;
    let mut news = items(g, panel);
    if notes_hidden {
        if let Some((t, text)) = g.overseer.as_ref().and_then(|o| o.notes.last()) {
            if *t + DAY >= g.tick {
                news.push((format!("The overseer: {}", text), (150, 210, 255)));
            }
        }
    }
    if news.is_empty() || full.width < 20 {
        return;
    }
    let y = full.y + full.height - 1;
    let lo = full.x + 1;
    let hi = (full.x + full.width).saturating_sub(1 + reserve);
    // Keep off whatever is already written along the bottom row (a page's
    // key hints, a counter, the pane's hint): run only in the longest
    // stretch of bare border or blank, a cell clear of any text.
    let (x0, room) = {
        let buf = f.buffer_mut();
        let free = |x: u16| matches!(buf.get(x, y).symbol(), "─" | " " | "━" | "═");
        let mut best = (lo, 0u16);
        let mut x = lo;
        while x < hi {
            if !free(x) {
                x += 1;
                continue;
            }
            let a = x;
            while x < hi && free(x) {
                x += 1;
            }
            // A cell of gap next to any text.
            let s = if a > lo { a + 1 } else { a };
            let e = if x < hi { x.saturating_sub(1) } else { x };
            if e > s && e - s > best.1 {
                best = (s, e - s);
            }
        }
        (best.0, best.1 as usize)
    };
    if room < 16 {
        return;
    }
    let t = CLOCK.with(|c| {
        c.set(c.get() + app.frame_dt);
        c.get()
    });
    // The run of characters, each with its colour, looping.
    let mut run: Vec<(char, (u8, u8, u8))> = Vec::new();
    for (text, col) in &news {
        run.extend(text.chars().map(|c| (c, *col)));
        run.extend("   ◆   ".chars().map(|c| (c, (130, 130, 140))));
    }
    let label = " NEWS ";
    let buf = f.buffer_mut();
    let bg = Color::Rgb(24, 16, 12);
    for (i, ch) in label.chars().enumerate() {
        buf.get_mut(x0 + i as u16, y).set_char(ch).set_style(Style::default().fg(Color::Rgb(0, 0, 0)).bg(Color::Rgb(255, 170, 60)).add_modifier(Modifier::BOLD));
    }
    let start = x0 + label.len() as u16;
    let w = room.saturating_sub(label.len());
    let off = (t * SPEED) as usize % run.len();
    for i in 0..w {
        let (ch, col) = run[(off + i) % run.len()];
        buf.get_mut(start + i as u16, y).set_char(ch).set_style(Style::default().fg(rgb(col)).bg(bg));
    }
}

#[cfg(test)]
pub(super) fn reset_for_test() {
    CLOCK.with(|c| c.set(0.0));
}

#[cfg(test)]
mod tests {
    use super::super::draw as draw_all;
    use super::*;
    use crate::app::Mode;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn bottom(app: &mut App, w: u16, h: u16) -> String {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| draw_all(f, app)).unwrap();
        let buf = term.backend().buffer().clone();
        (0..w).map(|x| buf.get(x, h - 1).symbol().to_string()).collect()
    }

    #[test]
    fn notable_news_is_picked_out() {
        let mut g = Game::new(5);
        g.tick = 5000;
        g.msg("Urist was struck down.", (255, 60, 60));
        g.msg("Some boring hauling.", (200, 200, 200));
        g.msg("Urist and Kadol have married.", (255, 200, 220));
        g.msg("Summer has arrived.", (200, 200, 255));
        let v: Vec<String> = items(&g, 0).into_iter().map(|x| x.0).collect();
        assert_eq!(v, vec!["Urist and Kadol have married.".to_string(), "Urist was struck down.".to_string()]);
        // What the panel still shows isn't run again.
        assert_eq!(items(&g, 2).len(), 1);
        // Old news isn't news.
        g.tick += FRESH + 1;
        assert!(items(&g, 0).is_empty());
    }

    #[test]
    fn the_ticker_runs_when_the_announcements_are_hidden_or_full() {
        let mut app = App::with_game_for_tests(Game::new(5));
        app.mode = Mode::Normal;
        app.game.tick = 5000;
        app.game.msg("A goblin siege has come to the gates!", (255, 70, 70));
        reset_for_test();
        // The panel shows it: no ticker.
        for (w, h) in [(80, 24), (200, 60)] {
            assert!(!bottom(&mut app, w, h).contains("NEWS"), "{}x{}", w, h);
        }
        // Hidden under a full-screen page: it runs.
        app.mode = Mode::Status { tab: 0, scroll: 0 };
        for (w, h) in [(80, 24), (200, 60)] {
            let s = bottom(&mut app, w, h);
            assert!(s.contains("NEWS") && s.contains("goblin siege"), "{}x{}: {}", w, h, s);
        }
        // Full: a flood of news pushes it off the panel, and it runs.
        app.mode = Mode::Normal;
        for i in 0..12 {
            app.game.msg(&format!("Hauled thing {}.", i), (200, 200, 200));
        }
        let s = bottom(&mut app, 200, 60);
        assert!(s.contains("goblin siege"), "{}", s);
        // And it scrolls.
        app.frame_dt = 1.0;
        let a = bottom(&mut app, 200, 60);
        let b = bottom(&mut app, 200, 60);
        assert_ne!(a, b);
    }

    /// All five pieces of flair at once (faces, a filmed moment, snow,
    /// the ticker, a Life page) draw at both sizes, and a frame of them
    /// stays cheap.
    #[test]
    fn all_the_flair_at_once_stays_cheap() {
        let mut app = App::with_game_for_tests(Game::new(5));
        app.paused = false;
        app.game.tick = 5000;
        app.game.weather = crate::game::Weather::Snow;
        app.cursor.z = app.game.world.surface_z(app.cursor.x, app.cursor.y);
        let at = app.game.dwarves[0].pos;
        app.game.film_for_test("An artifact", "Urist has created a thing of beauty!", Some(at));
        app.game.msg("A goblin siege has come to the gates!", (255, 70, 70));
        let id = app.game.dwarves[0].id;
        let modes = [Mode::Normal, Mode::Highlights { sel: 0 }, Mode::Life { id, scroll: 0 }, Mode::Status { tab: 0, scroll: 0 }];
        for (w, h) in [(80u16, 24u16), (200, 60)] {
            let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
            for m in &modes {
                app.mode = m.clone();
                term.draw(|f| draw_all(f, &mut app)).unwrap();
            }
            app.mode = Mode::Status { tab: 0, scroll: 0 };
            let t0 = std::time::Instant::now();
            for _ in 0..20 {
                term.draw(|f| draw_all(f, &mut app)).unwrap();
            }
            let ms = t0.elapsed().as_secs_f64() * 1000.0 / 20.0;
            assert!(ms < 50.0, "{:.1} ms a frame at {}x{}", ms, w, h);
            let buf = term.backend().buffer().clone();
            let last: String = (0..w).map(|x| buf.get(x, h - 1).symbol().to_string()).collect();
            println!("{}x{}: {:.2} ms a frame\n{}", w, h, ms, last);
            assert!(last.contains("NEWS"));
        }
        // Every page keeps its key hints (or its counter) in sight, with the
        // news running too.
        let modes = [
            (Mode::Status { tab: 0, scroll: 0 }, "Esc back"),
            (Mode::Labors { row: 0, col: 0, detail: 0 }, "Esc back"),
            (Mode::Legends { tab: 0, scroll: 0, from_world: false }, "Esc back"),
            (Mode::Highlights { sel: 0 }, " KB "),
        ];
        for (w, h) in [(80u16, 24u16), (200, 60)] {
            for (m, hint) in &modes {
                app.mode = m.clone();
                let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
                term.draw(|f| draw_all(f, &mut app)).unwrap();
                let buf = term.backend().buffer().clone();
                let rows: Vec<String> = (0..h).map(|y| (0..w).map(|x| buf.get(x, y).symbol().to_string()).collect()).collect();
                let last = &rows[h as usize - 1];
                assert!(last.contains("NEWS"), "{:?} {}x{}: {}", m, w, h, last);
                assert!(rows.iter().any(|r| r.contains(hint)), "{:?} {}x{}: {:?} lost\n{}", m, w, h, hint, rows.join("\n"));
                if last.contains("Esc close") {
                    assert!(last.contains("full screen · Esc close"), "{}", last);
                }
            }
        }
    }

    /// When a page's key hints sit on the very bottom row (as a full-height
    /// page's would), the ticker runs beside them, not over them.
    #[test]
    fn the_ticker_runs_beside_bottom_hints() {
        use ratatui::widgets::Block;
        let mut app = App::with_game_for_tests(Game::new(5));
        app.game.tick = 5000;
        app.game.msg("A goblin siege has come to the gates!", (255, 70, 70));
        reset_for_test();
        for (w, h, hint) in [(80u16, 24u16, " Tab side · ↑↓ move · Space mark · o offer · Esc leave "), (200, 60, " Tab/1-4 switch · ↑↓ PgUp/PgDn scroll · Esc back ")] {
            let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
            term.draw(|f| {
                let full = f.size();
                f.render_widget(Block::bordered().title_bottom(hint), full);
                draw(f, &app, full, 0, false, 0);
            })
            .unwrap();
            let buf = term.backend().buffer().clone();
            let last: String = (0..w).map(|x| buf.get(x, h - 1).symbol().to_string()).collect();
            assert!(last.contains(hint.trim()) && last.contains("NEWS"), "{}", last);
        }
    }

    #[test]
    #[ignore]
    // Timed by the wall clock: run on its own with `--ignored`.
    fn the_ticker_stays_cheap_on_a_busy_log() {
        let mut g = Game::new(5);
        g.tick = 5000;
        for i in 0..4000 {
            g.msg(&format!("Hauled thing {}.", i), (200, 200, 200));
        }
        g.msg("A forgotten beast has come!", (255, 60, 60));
        let t0 = std::time::Instant::now();
        for _ in 0..200 {
            assert!(items(&g, 0).len() <= MAX_ITEMS);
        }
        let ms = t0.elapsed().as_secs_f64() * 1000.0 / 200.0;
        assert!(ms < 0.5, "{:.3} ms a frame", ms);
    }
}
