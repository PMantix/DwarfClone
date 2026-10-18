//! The Highlights screen (`h`): the reel of the fort's big moments
//! (`game/reel.rs`) played back, each a few seconds of the ground where it
//! happened and who moved there, with its caption. The overseer's moment of
//! each year has a star.

use super::{add_target, key_line, plain, rgb, wrap_words, Target, KEY, MUTED, TEXT};
use crate::app::{App, Mode};
use crate::game::reel::{Clip, CLIP_H, CLIP_W};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;
use std::cell::Cell;

/// Frames a second on playback, and the pause on the last frame (seconds).
const FPS: f64 = 8.0;
const HOLD: f64 = 1.5;

thread_local! {
    /// The clip playing and how far into it (seconds).
    static PLAYING: Cell<(usize, f64)> = const { Cell::new((usize::MAX, 0.0)) };
}

/// Which frame of a clip of `n` frames shows `t` seconds in (it loops).
pub(super) fn frame_at(n: usize, t: f64) -> usize {
    if n == 0 {
        return 0;
    }
    let len = n as f64 / FPS + HOLD;
    let t = t % len;
    ((t * FPS) as usize).min(n - 1)
}

/// Start the clip again from the beginning.
fn rewind() {
    PLAYING.with(|p| p.set((usize::MAX, 0.0)));
}

pub(super) fn draw(f: &mut Frame, area: Rect, app: &App, sel: usize) {
    let g = &app.game;
    let clips = &g.reel.clips;
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(KEY))
        .style(Style::default().bg(Color::Rgb(10, 10, 16)))
        .title(Span::styled(format!(" Highlights of {} ", g.fortress_name), Style::default().fg(KEY).add_modifier(Modifier::BOLD)))
        .title_bottom(Span::styled(format!(" {} clips, {} KB ", clips.len(), clips.iter().map(|c| c.size()).sum::<usize>() / 1024), Style::default().fg(MUTED)));
    let inner = block.inner(area);
    f.render_widget(block, area);
    add_target(area, Target::Block);
    if clips.is_empty() {
        let mut v = vec![Line::default()];
        for l in wrap_words("Nothing worth remembering has happened yet. The reel films sieges and battles, beasts and monsters, heroes falling, artifacts, deaths and weddings as they happen.", inner.width.saturating_sub(2) as usize) {
            v.push(plain(format!(" {}", l), TEXT));
        }
        v.push(Line::default());
        v.push(key_line("Esc", "Back", false));
        f.render_widget(Paragraph::new(v), inner);
        return;
    }
    let sel = sel.min(clips.len() - 1);
    // The playback clock.
    let t = PLAYING.with(|p| {
        let (s, t) = p.get();
        let t = if s == sel { t + app.frame_dt } else { 0.0 };
        p.set((sel, t));
        t
    });
    // The list down the left when there's room; the screen is the rest.
    let list_w = if inner.width >= 100 { 34 } else { 0 };
    if list_w > 0 {
        let r = Rect::new(inner.x, inner.y, list_w, inner.height);
        let rows = r.height as usize;
        let first = sel.saturating_sub(rows.saturating_sub(1));
        let mut v = Vec::new();
        for (i, c) in clips.iter().enumerate().skip(first).take(rows) {
            let style = if i == sel { Style::default().bg(Color::Rgb(50, 60, 120)) } else { Style::default() };
            let star = if c.star { "★" } else { " " };
            let date = g.short_date(c.tick);
            let room = (list_w as usize).saturating_sub(date.chars().count() + 3);
            v.push(Line::from(vec![
                Span::styled(star, style.fg(Color::Rgb(255, 215, 60))),
                Span::styled(format!("{} ", date), style.fg(MUTED)),
                Span::styled(format!("{:<w$}", c.title.chars().take(room).collect::<String>(), w = room), style.fg(TEXT)),
            ]));
            add_target(Rect::new(r.x, r.y + (i - first) as u16, r.width, 1), Target::Row(i));
        }
        f.render_widget(Paragraph::new(v), r);
    }
    let right = Rect::new(inner.x + list_w + if list_w > 0 { 1 } else { 0 }, inner.y, inner.width - list_w - if list_w > 0 { 1 } else { 0 }, inner.height);
    let c = &clips[sel];
    let frame = frame_at(c.frames.len(), t);
    // Below the screen: title, caption, progress and keys.
    let text_w = right.width.saturating_sub(2) as usize;
    let caption = wrap_words(&c.caption, text_w.max(10));
    let room_for_text = 4 + caption.len().min(3) as u16;
    let vh = (CLIP_H as u16).min(right.height.saturating_sub(room_for_text).max(3));
    let vw = (CLIP_W as u16).min(right.width);
    let screen = Rect::new(right.x + (right.width - vw) / 2, right.y, vw, vh);
    draw_clip(f, screen, c, frame);
    let mut v = Vec::new();
    let gold = Color::Rgb(255, 215, 60);
    let mut head = vec![Span::styled(format!(" {} ", c.title), Style::default().fg(if c.star { gold } else { KEY }).add_modifier(Modifier::BOLD)), Span::styled(g.short_date(c.tick).trim().to_string(), Style::default().fg(MUTED))];
    if c.star {
        head.push(Span::styled("  ★ the overseer's moment of the year", Style::default().fg(gold)));
    }
    v.push(Line::from(head));
    let shown = (right.height.saturating_sub(vh + 3) as usize).min(caption.len());
    for l in caption.iter().take(shown) {
        v.push(plain(format!(" {}", l), TEXT));
    }
    let bar_w = text_w.saturating_sub(16).max(4);
    let done = if c.frames.len() <= 1 { bar_w } else { frame * bar_w / (c.frames.len() - 1) };
    v.push(Line::from(vec![
        Span::styled(" ▶ ", Style::default().fg(KEY)),
        Span::styled("━".repeat(done), Style::default().fg(KEY)),
        Span::styled("─".repeat(bar_w - done), Style::default().fg(MUTED)),
        Span::styled(format!(" {}/{} · {}/{}", frame + 1, c.frames.len(), sel + 1, clips.len()), Style::default().fg(MUTED)),
    ]));
    v.push(plain(" ↑↓ pick · Enter play again · c go there · Esc back".into(), MUTED));
    let text = Rect::new(right.x, screen.y + vh, right.width, right.height - vh);
    f.render_widget(Paragraph::new(v), text);
}

/// One frame of a clip: the ground, then who's moving, cropped about where
/// it happened to fit `r`.
fn draw_clip(f: &mut Frame, r: Rect, c: &Clip, frame: usize) {
    let (w, h) = (r.width as i32, r.height as i32);
    let ax = (c.at.x - c.corner.x - w / 2).clamp(0, (CLIP_W - w).max(0));
    let ay = (c.at.y - c.corner.y - h / 2).clamp(0, (CLIP_H - h).max(0));
    let rows = c.ground_rows();
    let buf = f.buffer_mut();
    for y in 0..h {
        for x in 0..w {
            let cell = buf.get_mut(r.x + x as u16, r.y + y as u16);
            let (g, ink, paper) = rows.get((ay + y) as usize).and_then(|row| row.get((ax + x) as usize)).copied().unwrap_or((' ', (0, 0, 0), (0, 0, 0)));
            cell.set_char(g).set_fg(rgb(ink)).set_bg(rgb(paper));
        }
    }
    for m in c.marks(frame) {
        let (x, y) = (m.x - ax, m.y - ay);
        if x >= 0 && y >= 0 && x < w && y < h {
            let cell = buf.get_mut(r.x + x as u16, r.y + y as u16);
            cell.set_char(m.glyph).set_fg(rgb(m.ink));
            if let Some(p) = m.paper {
                cell.set_bg(rgb(p));
            }
        }
    }
}

impl App {
    /// The keys on the Highlights screen.
    pub fn key_highlights(&mut self, k: KeyEvent, sel: usize) {
        let n = self.game.reel.clips.len();
        let sel = sel.min(n.saturating_sub(1));
        let back = if self.game.fallen { Mode::GameOver } else { Mode::Normal };
        match k.code {
            KeyCode::Esc | KeyCode::Char('h') => self.mode = back,
            KeyCode::Up | KeyCode::Left => self.mode = Mode::Highlights { sel: sel.saturating_sub(1) },
            KeyCode::Down | KeyCode::Right => self.mode = Mode::Highlights { sel: (sel + 1).min(n.saturating_sub(1)) },
            KeyCode::Home => self.mode = Mode::Highlights { sel: 0 },
            KeyCode::End => self.mode = Mode::Highlights { sel: n.saturating_sub(1) },
            KeyCode::Enter => rewind(),
            KeyCode::Char('c') => {
                if let Some(c) = self.game.reel.clips.get(sel) {
                    self.cursor = c.at;
                    self.free_view = false;
                    self.mode = Mode::Normal;
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::draw as draw_all;
    use super::*;
    use crate::game::Game;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn dump(app: &mut App, w: u16, h: u16) -> String {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| draw_all(f, app)).unwrap();
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
    fn playback_loops_through_the_frames() {
        assert_eq!(frame_at(25, 0.0), 0);
        assert_eq!(frame_at(25, 1.0), 8);
        assert_eq!(frame_at(25, 3.5), 24, "holds on the last frame");
        assert_eq!(frame_at(25, 25.0 / FPS + HOLD + 0.01), 0, "and loops");
        assert_eq!(frame_at(0, 3.0), 0);
    }

    #[test]
    fn the_highlights_screen_plays_a_clip_at_any_size() {
        let mut app = App::with_game_for_tests(Game::new(5));
        app.mode = Mode::Normal;
        let at = app.game.dwarves[0].pos;
        app.game.film_for_test("An artifact", "Urist has created a thing of beauty!", Some(at));
        for _ in 0..120 {
            app.game.step();
        }
        app.on_key(KeyEvent::from(KeyCode::Char('h')));
        assert!(matches!(app.mode, Mode::Highlights { .. }));
        for (w, h) in [(80, 24), (200, 60)] {
            rewind();
            let s = dump(&mut app, w, h);
            println!("{}", s);
            assert!(s.contains("An artifact"), "{}x{}", w, h);
            assert!(s.contains("thing of beauty"), "{}x{}", w, h);
            assert!(s.contains("▶"), "{}x{}", w, h);
            assert!(s.contains('☺') || s.contains('☻'), "the dwarf is on the screen at {}x{}", w, h);
        }
        // Frames advance as time passes.
        app.frame_dt = 0.5;
        let a = dump(&mut app, 200, 60);
        let b = dump(&mut app, 200, 60);
        assert!(a.contains(" 5/") && b.contains(" 9/"), "{}", b.lines().find(|l| l.contains('▶')).unwrap_or(""));
        // c goes there.
        app.on_key(KeyEvent::from(KeyCode::Char('c')));
        assert_eq!((app.mode.clone(), app.cursor), (Mode::Normal, at));
    }

    #[test]
    fn an_empty_reel_says_so() {
        let mut app = App::with_game_for_tests(Game::new(5));
        app.mode = Mode::Highlights { sel: usize::MAX };
        let s = dump(&mut app, 80, 24);
        assert!(s.contains("Nothing worth remembering"));
    }
}
