//! The font screen (Ctrl+F in the window, or F on the title and sound
//! screens): the font, the cell size, how wide and heavy letters are
//! drawn and how big the adventure map's tiles are, each changed live
//! over whatever the game is showing, so the game itself is the preview.

use super::backend::{MAX_CELL, MAX_TILE, MIN_CELL};
use super::display::{Display, Weight, Width};
use super::fontlist::{self, FontFile};
use super::glyphs::MONO_NAMES;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use std::path::Path;

pub const ROWS: [&str; 6] = ["Font", "Size", "Width", "Weight", "Map tiles", "Back"];
const BACK: usize = ROWS.len() - 1;

const GOLD: Color = Color::Rgb(255, 210, 90);
const TEXT: Color = Color::Rgb(210, 210, 210);
const MUTED: Color = Color::Rgb(130, 130, 140);
const WARN: Color = Color::Rgb(240, 150, 110);

pub struct FontScreen {
    pub sel: usize,
    /// The fonts there are to choose from.
    pub list: Vec<FontFile>,
    /// Which of them is showing (None for a font from elsewhere: `--font`).
    pub at: Option<usize>,
    /// Something to tell the player (a font that wouldn't load).
    pub note: Option<String>,
}

/// What a key did to the screen.
#[derive(Debug, PartialEq)]
pub enum Out {
    Stay,
    Close,
}

impl FontScreen {
    /// Open the screen, looking through the machine's fonts; `current` is the font showing.
    pub fn open(current: &Path) -> FontScreen {
        FontScreen::with(fontlist::scan(), current)
    }

    pub fn with(list: Vec<FontFile>, current: &Path) -> FontScreen {
        let at = list.iter().position(|f| f.path == current);
        FontScreen { sel: 0, list, at, note: None }
    }

    /// A key, changing `d` (the window applies what changed); `cell` and
    /// `tile` are the sizes showing now.
    pub fn key(&mut self, k: &KeyEvent, d: &mut Display, cell: usize, tile: usize) -> Out {
        let n = ROWS.len();
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let step: i32 = match k.code {
            KeyCode::Left | KeyCode::Char('-') if !ctrl => -1,
            KeyCode::Right | KeyCode::Char('+' | '=') if !ctrl => 1,
            KeyCode::Enter | KeyCode::Char(' ') if self.sel != BACK => 1,
            _ => 0,
        };
        match k.code {
            KeyCode::Esc | KeyCode::Char('q') => return Out::Close,
            KeyCode::Char('f' | 'F') if ctrl || self.sel == BACK => return Out::Close,
            KeyCode::Enter if self.sel == BACK => return Out::Close,
            KeyCode::Up => self.sel = (self.sel + n - 1) % n,
            KeyCode::Down | KeyCode::Tab => self.sel = (self.sel + 1) % n,
            _ if step != 0 => self.change(step, d, cell, tile),
            _ => {}
        }
        Out::Stay
    }

    fn change(&mut self, step: i32, d: &mut Display, cell: usize, tile: usize) {
        let cycle = |i: usize, n: usize| ((i as i32 + step).rem_euclid(n as i32)) as usize;
        match self.sel {
            0 => {
                if self.list.is_empty() {
                    return;
                }
                let next = match self.at {
                    Some(i) => cycle(i, self.list.len()),
                    None if step > 0 => 0,
                    None => self.list.len() - 1,
                };
                self.at = Some(next);
                self.note = None;
                d.font = Some(self.list[next].path.to_string_lossy().into_owned());
            }
            1 => d.cell = Some((cell as i32 + 2 * step).clamp(MIN_CELL as i32, MAX_CELL as i32) as usize),
            2 => d.width = Width::ALL[cycle(Width::ALL.iter().position(|w| *w == d.width).unwrap_or(1), 3)],
            3 => d.weight = Weight::ALL[cycle(Weight::ALL.iter().position(|w| *w == d.weight).unwrap_or(0), 3)],
            4 => d.tile = Some((tile as i32 + 4 * step).clamp(cell as i32 + 2, MAX_TILE as i32) as usize),
            _ => {}
        }
    }

    /// Draw the screen over the game; `font` names the font showing.
    pub fn draw(&self, f: &mut Frame, d: &Display, cell: usize, tile: usize, font: &str) {
        let area = f.size();
        let row = |i: usize, value: String, detail: String| -> Line {
            let on = i == self.sel;
            let st = if on { Style::default().fg(Color::Black).bg(GOLD).add_modifier(Modifier::BOLD) } else { Style::default().fg(TEXT) };
            let value = if on && i != BACK { format!("◂ {} ▸", value) } else { format!("  {}  ", value) };
            Line::from(vec![
                Span::styled(if on { " ☼ " } else { "   " }, Style::default().fg(GOLD)),
                Span::styled(format!("{:<10}", ROWS[i]), st),
                Span::styled(format!(" {:<26}", value), Style::default().fg(if on { GOLD } else { TEXT })),
                Span::styled(detail, Style::default().fg(MUTED)),
            ])
        };
        let choices = |names: &[&str], now: &str| names.iter().map(|n| if *n == now { n.to_uppercase() } else { n.to_string() }).collect::<Vec<_>>().join(" · ");
        let which = match self.at {
            Some(i) => {
                let f = &self.list[i];
                let whose = if f.mine {
                    "yours, "
                } else if f.usual {
                    "usual, "
                } else {
                    ""
                };
                format!("{}{} of {}", whose, i + 1, self.list.len())
            }
            None => "from --font".to_string(),
        };
        let widths: Vec<&str> = Width::ALL.iter().map(|w| w.name()).collect();
        let weights: Vec<&str> = Weight::ALL.iter().map(|w| w.name()).collect();
        let mut v = vec![
            Line::default(),
            row(0, font.chars().take(22).collect(), which),
            row(1, format!("{} px", cell), format!("{}-{}, also Ctrl +/-", MIN_CELL, MAX_CELL)),
            row(2, d.width.name().to_string(), choices(&widths, d.width.name())),
            row(3, d.weight.name().to_string(), choices(&weights, d.weight.name())),
            row(4, format!("{} px", tile), "the adventure map's tiles".to_string()),
            row(BACK, String::new(), String::new()),
            Line::default(),
            Line::from(Span::styled("   The quick dwarf hauls 12 gold bars; Urist cries \"Ale!\"", Style::default().fg(TEXT))),
            Line::from(Span::styled("   ABCDEFGHIJKLM nopqrstuvwxyz 0123456789 ☼ ♥ ☺ ♪ ⚒", Style::default().fg(GOLD))),
            Line::default(),
        ];
        let mut notes: Vec<(String, Color)> = Vec::new();
        if let Some(n) = &self.note {
            notes.push((n.clone(), WARN));
        }
        if self.list.is_empty() {
            notes.push(("No monospace fonts found on this PC.".to_string(), WARN));
        } else if !self.list.iter().any(|f| f.usual) {
            notes.push((format!("None of the usual fonts ({}) is on this PC; these are the monospace fonts it has.", MONO_NAMES.join(", ")), WARN));
        }
        if let Some(dir) = fontlist::my_dir() {
            notes.push((format!("Your own .ttf fonts go in {}", dir.display()), MUTED));
        }
        let w = 76.min(area.width);
        for (text, color) in notes {
            // (Long notes wrapped by hand: the box is sized by its lines.)
            // (A long path is cut wherever it must be.)
            let room = (w as usize).saturating_sub(6).max(10);
            let mut line = String::new();
            let words = text.split(' ').flat_map(|word| {
                let cs: Vec<char> = word.chars().collect();
                cs.chunks(room).map(|c| c.iter().collect::<String>()).collect::<Vec<_>>()
            });
            for word in words {
                if !line.is_empty() && line.chars().count() + word.chars().count() > room {
                    v.push(Line::from(Span::styled(format!("   {}", line.trim_end()), Style::default().fg(color))));
                    line.clear();
                }
                line.push_str(&word);
                line.push(' ');
            }
            v.push(Line::from(Span::styled(format!("   {}", line.trim_end()), Style::default().fg(color))));
        }
        v.push(Line::from(Span::styled("   ↑↓ choose  ←→ change  Esc back  (saved for next time)", Style::default().fg(MUTED))));
        let h = (v.len() as u16 + 2).min(area.height);
        let r = Rect::new(area.x + (area.width - w) / 2, area.y + area.height.saturating_sub(h) / 2, w, h);
        f.render_widget(Clear, r);
        f.render_widget(
            Paragraph::new(v).style(Style::default().bg(Color::Rgb(12, 12, 20))).block(
                Block::default().title(" Font ").borders(Borders::ALL).border_style(Style::default().fg(Color::Rgb(160, 130, 70))),
            ),
            r,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn font(name: &str, usual: bool) -> FontFile {
        FontFile { name: name.into(), path: PathBuf::from(format!("C:\\f\\{}.ttf", name)), usual, mine: false }
    }

    fn key(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::NONE)
    }

    #[test]
    fn keys_change_the_settings() {
        let list = vec![font("A", true), font("B", false)];
        let mut s = FontScreen::with(list, Path::new("C:\\f\\A.ttf"));
        assert_eq!(s.at, Some(0));
        let mut d = Display::default();
        s.key(&key(KeyCode::Right), &mut d, 16, 26);
        assert_eq!(d.font.as_deref(), Some("C:\\f\\B.ttf"));
        s.key(&key(KeyCode::Right), &mut d, 16, 26);
        assert_eq!(d.font.as_deref(), Some("C:\\f\\A.ttf"), "round to the first again");
        s.key(&key(KeyCode::Down), &mut d, 16, 26);
        s.key(&key(KeyCode::Left), &mut d, 16, 26);
        assert_eq!(d.cell, Some(14));
        s.key(&key(KeyCode::Down), &mut d, 14, 26);
        s.key(&key(KeyCode::Right), &mut d, 14, 26);
        assert_eq!(d.width, Width::Wide);
        s.key(&key(KeyCode::Down), &mut d, 14, 26);
        s.key(&key(KeyCode::Left), &mut d, 14, 26);
        assert_eq!(d.weight, Weight::Bold);
        s.key(&key(KeyCode::Down), &mut d, 14, 26);
        s.key(&key(KeyCode::Right), &mut d, 14, 26);
        assert_eq!(d.tile, Some(30));
        assert_eq!(s.key(&key(KeyCode::Esc), &mut d, 14, 30), Out::Close);
        // Ctrl+F closes it too, from any row.
        assert_eq!(s.key(&KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL), &mut d, 14, 30), Out::Close);
    }

    #[test]
    fn it_draws_and_says_when_the_usual_fonts_are_missing() {
        let s = FontScreen::with(vec![font("Odd Mono", false)], Path::new("x.ttf"));
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(90, 30)).unwrap();
        term.draw(|f| s.draw(f, &Display::default(), 16, 26, "Odd Mono")).unwrap();
        let text: String = term.backend().buffer().content().iter().map(|c| c.symbol()).collect();
        assert!(text.contains("Odd Mono"));
        assert!(text.contains("None of the usual fonts"), "{}", text);
        assert!(text.contains("from --font"));
    }
}
