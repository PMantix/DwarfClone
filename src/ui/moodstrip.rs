//! The mood strip in the status bar (UI-01): a tiny face for every dwarf,
//! coloured by their mood, and a skull for each who died this season. Click
//! a face to go to that dwarf; hover over it for their name and mood. When
//! the bar is too narrow for a face each, the faces are counted instead
//! ("☺9 ☹2 ☠1"), and a click on a count goes to the worst of them.

use super::{add_target, rgb, Target, MUTED, TEXT};
use crate::app::{App, Mode};
use crate::game::moods::{Face, FROWN, SKULL, SMILE, WILD};
use crate::game::Game;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

/// More dwarves than this are counted rather than drawn, however wide the bar.
const MAX_FACES: usize = 120;

/// Put the strip into the status bar's spans at `at` (where the dwarf count
/// went), as wide as the rest of the bar allows, and make it clickable.
pub(super) fn splice(g: &Game, spans: &mut Vec<Span<'static>>, at: usize, row: Rect) {
    let others: usize = spans.iter().map(|s| s.width()).sum();
    let avail = (row.width as usize).saturating_sub(others);
    let faces = g.mood_faces();
    let head = Span::styled(format!("│ {} ", g.alive_count()), Style::default().fg(TEXT));
    let mut x = row.x + spans[..at].iter().map(|s| s.width()).sum::<usize>() as u16 + head.width() as u16;
    let mut strip = vec![head];
    let wide = faces.len() <= MAX_FACES && strip[0].width() + faces.len() + 1 <= avail;
    if wide {
        for f in &faces {
            strip.push(Span::styled(f.glyph.to_string(), Style::default().fg(rgb(f.color))));
            add_target(Rect::new(x, row.y, 1, 1), Target::Dwarf(f.id));
            x += 1;
        }
        strip.push(Span::raw(" "));
    } else {
        for (glyph, group) in tally(&faces) {
            let worst = group.last().unwrap();
            let text = format!("{}{} ", glyph, group.len());
            let w = text.chars().count() as u16;
            add_target(Rect::new(x, row.y, w - 1, 1), Target::Dwarf(worst.id));
            x += w;
            strip.push(Span::styled(text, Style::default().fg(rgb(worst.color))));
        }
    }
    spans.splice(at..at, strip);
}

/// The faces counted by kind: smiles, frowns, the wild, the dead (each
/// group worst last, as the strip sorts them).
fn tally(faces: &[Face]) -> Vec<(char, Vec<&Face>)> {
    [SMILE, FROWN, WILD, SKULL]
        .into_iter()
        .map(|c| (c, faces.iter().filter(|f| f.glyph == c).collect::<Vec<_>>()))
        .filter(|(_, v)| !v.is_empty())
        .collect()
}

/// Hovering over a face: who it is and how they are, just below the bar.
pub(super) fn draw_tip(f: &mut Frame, app: &App) {
    let Some((col, row)) = app.hover else { return };
    let Some(Target::Dwarf(id)) = super::target_at(col, row) else { return };
    let g = &app.game;
    let Some(d) = g.dwarves.iter().find(|d| d.id == id) else { return };
    let (mood, mc) = if d.alive { d.mood() } else { ("Dead", (200, 200, 210)) };
    let mut lines = vec![Line::from(vec![
        Span::styled(d.name.clone(), Style::default().fg(rgb(d.color))),
        Span::styled(" · ", Style::default().fg(MUTED)),
        Span::styled(mood, Style::default().fg(rgb(mc))),
    ])];
    let why = if d.alive {
        d.thoughts.iter().rev().find(|t| t.stress != 0).map(|t| format!("Lately {}", t.text))
    } else {
        g.annals.deaths.iter().rev().find(|n| n.name == d.name).map(|n| format!("{} ({})", super::cap(&n.cause), g.short_date(n.tick).trim()))
    };
    if let Some(w) = why {
        lines.push(Line::from(Span::styled(w, Style::default().fg(TEXT))));
    }
    lines.push(Line::from(Span::styled("click: go to them", Style::default().fg(MUTED))));
    let area = f.size();
    let w = (lines.iter().map(|l| l.width()).max().unwrap_or(10) as u16 + 2).min(area.width).min(60);
    let h = lines.len() as u16 + 2;
    let x = col.min(area.width.saturating_sub(w));
    let tip = Rect::new(x, row + 1, w, h.min(area.height.saturating_sub(row + 1)));
    f.render_widget(Clear, tip);
    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(Color::Rgb(18, 18, 30))).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(MUTED))),
        tip,
    );
}

impl App {
    /// Go to a dwarf from the strip: the camera on them, their sheet open.
    pub fn jump_to_dwarf(&mut self, id: u32) {
        let Some(d) = self.game.dwarves.iter().find(|d| d.id == id) else { return };
        self.cursor = d.pos;
        self.free_view = false;
        self.mode = Mode::Unit { id, sel: 0, tab: 0 };
    }
}

#[cfg(test)]
mod tests {
    use super::super::draw;
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn top_row(app: &mut App, w: u16, h: u16) -> String {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| draw(f, app)).unwrap();
        let buf = term.backend().buffer().clone();
        (0..w).map(|x| buf.get(x, 0).symbol().to_string()).collect()
    }

    #[test]
    fn a_face_for_each_dwarf_and_a_click_goes_to_one() {
        let mut app = App::with_game_for_tests(Game::new(5));
        app.mode = Mode::Normal;
        app.game.dwarves[2].stress = 400;
        let n = app.game.alive_count();
        let row = top_row(&mut app, 200, 60);
        assert_eq!(row.matches(SMILE).count() + row.matches(FROWN).count(), n, "{}", row);
        assert_eq!(row.matches(FROWN).count(), 1);
        // The frown is the last face; click it.
        let x = row.chars().position(|c| c == FROWN).unwrap() as u16;
        assert_eq!(super::super::target_at(x, 0), Some(Target::Dwarf(app.game.dwarves[2].id)));
        let id = app.game.dwarves[2].id;
        app.on_mouse(crossterm::event::MouseEvent { kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left), column: x, row: 0, modifiers: crossterm::event::KeyModifiers::NONE });
        assert_eq!(app.mode, Mode::Unit { id, sel: 0, tab: 0 });
        assert_eq!(app.cursor, app.game.dwarves[2].pos);
    }

    #[test]
    fn a_narrow_bar_counts_the_faces() {
        let mut app = App::with_game_for_tests(Game::new(5));
        app.mode = Mode::Normal;
        // A crowd: more faces than an 80-column bar has room for.
        for i in 0..60 {
            let mut d = app.game.dwarves[2].clone();
            d.id = 5000 + i;
            app.game.dwarves.push(d);
        }
        app.game.dwarves[0].stress = 400;
        let mut d = std::mem::take(&mut app.game.dwarves[1]);
        app.game.kill_for_test(&mut d, "was struck down");
        app.game.dwarves[1] = d;
        let n = app.game.alive_count();
        let row = top_row(&mut app, 80, 24);
        assert!(row.contains(&format!("{}{}", SMILE, n - 1)), "{}", row);
        assert!(row.contains(&format!("{}1", FROWN)), "{}", row);
        assert!(row.contains(&format!("{}1", SKULL)), "{}", row);
        // Wide, the skull is drawn as a face of its own.
        let row = top_row(&mut app, 200, 60);
        assert_eq!(row.matches(SKULL).count(), 1, "{}", row);
    }

    #[test]
    fn hovering_names_the_face() {
        let mut app = App::with_game_for_tests(Game::new(5));
        app.mode = Mode::Normal;
        let row = top_row(&mut app, 200, 60);
        let x = row.chars().position(|c| c == SMILE).unwrap() as u16;
        app.hover = Some((x, 0));
        let mut term = Terminal::new(TestBackend::new(200, 60)).unwrap();
        term.draw(|f| draw(f, &mut app)).unwrap();
        let buf = term.backend().buffer().clone();
        let below: String = (0..200).map(|c| buf.get(c, 2).symbol().to_string()).collect();
        let first = app.game.mood_faces()[0].id;
        let name = app.game.dwarves.iter().find(|d| d.id == first).unwrap().name.clone();
        assert!(below.contains(&name), "{}", below);
    }
}
