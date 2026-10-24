//! The sound screen: on or off, and the master, music, ambience and effects
//! volumes, drawn over whatever it was opened from.

use super::{popup, KEY, MUTED, TEXT};
use crate::app::sound::SOUND_ROWS;
use crate::app::App;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::Frame;

/// A bar of `n` percent, 20 cells wide.
fn bar(n: u8) -> String {
    let full = (n as usize * 20 + 50) / 100;
    format!("{}{} {:>3}%", "█".repeat(full), "░".repeat(20 - full), n)
}

pub fn draw(f: &mut Frame, app: &App, sel: usize) {
    let s = &app.audio.settings;
    let mut v: Vec<Line> = vec![Line::default()];
    for (i, row) in SOUND_ROWS.iter().enumerate() {
        let value = match i {
            0 => if s.enabled { "On".to_string() } else { "Off".to_string() },
            1 => bar(s.master),
            2 => bar(s.music),
            3 => bar(s.ambience),
            4 => bar(s.sfx),
            _ => String::new(),
        };
        let on = i == sel;
        let style = if on { Style::default().fg(Color::Rgb(0, 0, 0)).bg(KEY).add_modifier(Modifier::BOLD) } else { Style::default().fg(TEXT) };
        let dim = !s.enabled && (1..=4).contains(&i);
        let vstyle = if dim { Style::default().fg(MUTED) } else { Style::default().fg(Color::Rgb(150, 200, 255)) };
        v.push(Line::from(vec![Span::styled(format!(" {} {:<14}", if on { "►" } else { " " }, row), style), Span::raw("  "), Span::styled(value, vstyle)]));
        if i == 0 || i == 4 {
            v.push(Line::default());
        }
    }
    v.push(Line::default());
    let note = if app.audio.active() || !s.enabled {
        "All of it is made as it plays: no recordings."
    } else {
        "No sound device: the game runs silently."
    };
    v.push(Line::from(Span::styled(format!(" {}", note), Style::default().fg(MUTED))));
    let font = if super::adventure::TILE_PX.with(|t| t.get()).is_some() { "  F font" } else { "" };
    v.push(Line::from(Span::styled(format!(" ↑↓ choose  ←→ change  Enter toggles  M mutes anywhere{}  Esc back", font), Style::default().fg(MUTED))));
    popup(f, f.size(), 72, v.len() as u16 + 2, " Sound ", v);
}

#[cfg(test)]
mod tests {
    use crate::app::{App, Mode};
    use crate::game::Game;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn dump(app: &mut App, w: u16, h: u16) -> String {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| crate::ui::draw(f, app)).unwrap();
        let buf = term.backend().buffer().clone();
        let mut s = String::new();
        for y in 0..h {
            for x in 0..w {
                s.push_str(buf.get(x, y).symbol());
            }
            s.push('\n');
        }
        s
    }

    #[test]
    fn the_sound_screen_shows_over_the_fort_and_the_title() {
        let mut app = App::with_game_for_tests(Game::new(2));
        app.mode = Mode::Normal;
        app.open_sound();
        let s = dump(&mut app, 150, 46);
        println!("{}", s);
        for row in ["Sound", "Master volume", "Music", "Ambience", "Effects", "Back", "65%"] {
            assert!(s.contains(row), "{} missing", row);
        }
        assert!(s.contains("DwarfClone"), "the fort is drawn under it");
        assert_eq!(app.mode, Mode::Sound { sel: 0 }, "drawing leaves the mode as it was");
        // Muted, it says so.
        app.audio.settings.enabled = false;
        assert!(dump(&mut app, 150, 46).contains("Off"));
        // Over the title, too.
        let mut app = App::with_game_for_tests(Game::placeholder(crate::overworld::Overworld::empty()));
        app.mode = Mode::Title { sel: 0 };
        app.open_sound();
        let s = dump(&mut app, 120, 40);
        assert!(s.contains("Master volume"));
    }
}
