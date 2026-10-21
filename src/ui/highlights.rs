//! The Highlights screen (`h`): the reel of the fort's big moments
//! (`game/reel.rs`) played back, each a few seconds of the ground where it
//! happened and who moved there, with its caption. The overseer's moment of
//! each year has a star.
//!
//! While a screen is up, every frame is filmed off the screen exactly as the
//! map draws it (`film_drawn`): blood, smoke, fire, whatever the map shows,
//! the reel shows, without knowing what any of it is. `p` plays the fort's
//! best moments one after another, full size on the map where they
//! happened; `e` exports the fort (`ui/export.rs`).

use super::{add_target, dim, key_line, plain, rgb, wrap_words, Target, KEY, MUTED, TEXT};
use crate::app::{App, Mode};
use crate::game::reel::{Clip, Look, CLIP_H, CLIP_W, FRAMES};
use crate::geom::Pos;
use ratatui::backend::TestBackend;
use ratatui::Terminal;
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
    /// The best moments are playing one after another on the map (`p`).
    static SHOW: Cell<bool> = const { Cell::new(false) };
    /// What the last export said.
    static EXPORTED: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// The moments worth a place in the show: the weighty ones and the
/// overseer's picks, in the order they happened (all of them if none is).
pub fn best(clips: &[Clip]) -> Vec<usize> {
    let v: Vec<usize> = (0..clips.len()).filter(|&i| clips[i].score >= 55 || clips[i].star).collect();
    if v.is_empty() {
        (0..clips.len()).collect()
    } else {
        v
    }
}

/// How long a clip of `n` frames plays before the show moves on.
fn length(n: usize) -> f64 {
    n.max(1) as f64 / FPS + HOLD
}

/// Start (or stop) the show of the best moments.
pub fn set_show(on: bool) {
    SHOW.with(|s| s.set(on));
    rewind();
}

/// Open the show on the weightiest of the best moments (the screenshot's
/// `--reel`). False if the reel is empty.
pub fn open_show(app: &mut App) -> bool {
    let clips = &app.game.reel.clips;
    let Some(i) = best(clips).into_iter().max_by_key(|&i| (clips[i].score, clips[i].tick)) else { return false };
    app.mode = Mode::Highlights { sel: i };
    set_show(true);
    true
}

/// Any ratatui colour as RGB (the named ones as the terminal's usual).
pub fn to_rgb(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black | Color::Reset => (0, 0, 0),
        Color::Red => (170, 0, 0),
        Color::Green => (0, 170, 0),
        Color::Yellow => (170, 85, 0),
        Color::Blue => (0, 0, 170),
        Color::Magenta => (170, 0, 170),
        Color::Cyan => (0, 170, 170),
        Color::Gray => (170, 170, 170),
        Color::DarkGray => (85, 85, 85),
        Color::LightRed => (255, 85, 85),
        Color::LightGreen => (85, 255, 85),
        Color::LightYellow => (255, 255, 85),
        Color::LightBlue => (85, 85, 255),
        Color::LightMagenta => (255, 85, 255),
        Color::LightCyan => (85, 255, 255),
        Color::White => (255, 255, 255),
        Color::Indexed(_) => (170, 170, 170),
    }
}

/// A `w` x `h` window of level `corner.z` from `corner`, drawn off the
/// screen by the map's own drawing (no cursor, plan or tool on it), as
/// tiles row by row. The app's view is put back as it was.
pub fn capture(app: &mut App, corner: Pos, w: i32, h: i32) -> Vec<Look> {
    let saved = (app.cursor, app.view_x, app.view_y, app.free_view, app.pan_from, app.ghost, app.mode.clone(), app.map_rect, app.frame_dt);
    app.cursor = Pos::new(corner.x + w / 2, corner.y + h / 2, corner.z);
    app.view_x = corner.x;
    app.view_y = corner.y;
    app.free_view = true;
    app.pan_from = (app.cursor.x, app.cursor.y);
    app.ghost = false;
    app.mode = Mode::Normal;
    app.frame_dt = 0.0;
    let mut out = Vec::with_capacity((w * h) as usize);
    if let Ok(mut term) = Terminal::new(TestBackend::new(w as u16 + 2, h as u16 + 2)) {
        if term
            .draw(|f| {
                let r = f.size();
                super::draw_map(f, app, r)
            })
            .is_ok()
        {
            let buf = term.backend().buffer();
            for y in 0..h as u16 {
                for x in 0..w as u16 {
                    let c = buf.get(x + 1, y + 1);
                    out.push((c.symbol().chars().next().unwrap_or(' '), to_rgb(c.fg), to_rgb(c.bg)));
                }
            }
        }
    }
    (app.cursor, app.view_x, app.view_y, app.free_view, app.pan_from, app.ghost, app.mode, app.map_rect, app.frame_dt) = saved;
    out.resize((w * h) as usize, (' ', (0, 0, 0), (0, 0, 0)));
    out
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

pub(super) fn draw(f: &mut Frame, area: Rect, app: &mut App, sel: usize) {
    if SHOW.with(|s| s.get()) && !app.game.reel.clips.is_empty() {
        return draw_show(f, area, app, sel);
    }
    let app = &*app;
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
    v.push(plain(" ↑↓ pick · Enter play again · p the best of it, on the map · c go there · e export the fort · Esc back".into(), MUTED));
    if let Some(e) = EXPORTED.with(|e| e.borrow().clone()) {
        v.push(plain(format!(" {}", e), Color::Rgb(150, 220, 150)));
    }
    let text = Rect::new(right.x, screen.y + vh, right.width, right.height - vh);
    f.render_widget(Paragraph::new(v), text);
}

/// The show (`p`): the best moments one after another, each played full
/// size on the map where it happened, the live fort dimmed around it, with
/// a title above and the caption below. It moves on by itself and loops.
fn draw_show(f: &mut Frame, area: Rect, app: &mut App, sel: usize) {
    let order = best(&app.game.reel.clips);
    let mut sel = if order.contains(&sel) { sel } else { order.iter().copied().find(|&i| i >= sel).unwrap_or(order[0]) };
    let mut n = app.game.reel.clips[sel].frames.len();
    let mut t = PLAYING.with(|p| {
        let (s, t) = p.get();
        let t = if s == sel { t + app.frame_dt } else { 0.0 };
        p.set((sel, t));
        t
    });
    // Done: on to the next (and round again after the last).
    if t >= length(n) && order.len() > 1 {
        let k = order.iter().position(|&i| i == sel).unwrap_or(0);
        sel = order[(k + 1) % order.len()];
        n = app.game.reel.clips[sel].frames.len();
        t = 0.0;
        PLAYING.with(|p| p.set((sel, 0.0)));
        app.mode = Mode::Highlights { sel };
    }
    let g = &app.game;
    let c = &g.reel.clips[sel];
    let frame = frame_at(n, t);
    let cells = c.frame_cells(frame);
    f.render_widget(Clear, area);
    add_target(area, Target::Block);
    // The map, centred on the moment.
    let (w, h) = (area.width as i32, area.height as i32);
    let ox = c.at.x - w / 2;
    let oy = c.at.y - h / 2;
    let buf = f.buffer_mut();
    for sy in 0..h {
        for sx in 0..w {
            let p = Pos::new(ox + sx, oy + sy, c.corner.z);
            let (x, y) = (p.x - c.corner.x, p.y - c.corner.y);
            let cell = buf.get_mut(area.x + sx as u16, area.y + sy as u16);
            if x >= 0 && y >= 0 && x < CLIP_W && y < CLIP_H {
                let (ch, ink, paper) = cells[(y * CLIP_W + x) as usize];
                cell.set_char(ch).set_fg(rgb(ink)).set_bg(rgb(paper));
            } else if g.world.in_bounds(p) {
                // The fort as it is now, dim: where the moment sits.
                let (ch, fg, bg) = crate::ui::tile_look(&g.world, p);
                cell.set_char(ch).set_fg(dim(fg, 0.3)).set_bg(dim(bg, 0.3));
            } else {
                cell.set_char(' ').set_fg(Color::Rgb(0, 0, 0)).set_bg(Color::Rgb(0, 0, 0));
            }
        }
    }
    // The title above, the caption and the reel's progress below.
    let k = order.iter().position(|&i| i == sel).unwrap_or(0);
    let gold = Color::Rgb(255, 215, 60);
    let band = Style::default().bg(Color::Rgb(8, 8, 14));
    let head = format!(" {}{} · {} ", if c.star { "★ " } else { "" }, c.title, g.short_date(c.tick).trim());
    let hw = (head.chars().count() as u16).min(area.width);
    let top = Rect::new(area.x + (area.width - hw) / 2, area.y + 1.min(area.height.saturating_sub(1)), hw, 1);
    f.render_widget(Paragraph::new(Span::styled(head, band.fg(if c.star { gold } else { KEY }).add_modifier(Modifier::BOLD))), top);
    let tw = area.width.saturating_sub(8).clamp(10.min(area.width), 100);
    let mut v: Vec<Line> = wrap_words(&c.caption, tw.saturating_sub(4) as usize).into_iter().take(3).map(|l| plain(format!(" {} ", l), TEXT)).collect();
    let dots: String = order.iter().map(|&i| if i == sel { '●' } else { '·' }).take(40).collect();
    v.push(Line::from(vec![
        Span::styled(format!(" {} ", dots), Style::default().fg(KEY)),
        Span::styled(format!(" {}/{} · {} of {} · ←→ skip · p the list · Esc back ", frame + 1, n, k + 1, order.len()), Style::default().fg(MUTED)),
    ]));
    // (Clear of the bottom row, where the news ticker runs.)
    let th = (v.len() as u16 + 2).min(area.height.saturating_sub(1));
    let r = Rect::new(area.x + (area.width - tw) / 2, area.y + area.height.saturating_sub(th + 1), tw, th);
    f.render_widget(Clear, r);
    f.render_widget(Paragraph::new(v).style(band).block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(if c.star { gold } else { MUTED }))), r);
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
    /// After each step of the game while a screen is up: a clip that has
    /// just started takes its ground from the map as drawn, and each new
    /// frame of a clip so filmed is the map as drawn, kept as what changed.
    /// (A clip filmed in a batch, with no screen, keeps the reel's own
    /// marks of who moved.) Draws nothing and changes nothing in the fort.
    pub fn film_drawn(&mut self) {
        let todo: Vec<usize> = (0..self.game.reel.clips.len())
            .filter(|&i| {
                let c = &self.game.reel.clips[i];
                if c.drawn == 0 {
                    c.frames.len() == 1 && c.left == FRAMES
                } else {
                    c.frames.len() > c.drawn as usize
                }
            })
            .collect();
        for i in todo {
            let corner = self.game.reel.clips[i].corner;
            let cells = capture(self, corner, CLIP_W, CLIP_H);
            let c = &mut self.game.reel.clips[i];
            if c.drawn == 0 {
                c.set_ground(&cells);
            }
            let last = c.frames.len() - 1;
            c.frames[last] = c.diff_frame(&cells);
            c.drawn = c.frames.len() as u8;
        }
    }

    /// The keys on the Highlights screen.
    pub fn key_highlights(&mut self, k: KeyEvent, sel: usize) {
        let n = self.game.reel.clips.len();
        let sel = sel.min(n.saturating_sub(1));
        let back = if self.game.fallen { Mode::GameOver } else { Mode::Normal };
        // The show: arrows skip, p (or Enter) goes back to the list, Esc out.
        if SHOW.with(|s| s.get()) {
            let order = best(&self.game.reel.clips);
            let at = order.iter().position(|&i| i == sel).unwrap_or(0);
            match k.code {
                KeyCode::Right | KeyCode::Down if !order.is_empty() => self.mode = Mode::Highlights { sel: order[(at + 1) % order.len()] },
                KeyCode::Left | KeyCode::Up if !order.is_empty() => self.mode = Mode::Highlights { sel: order[(at + order.len() - 1) % order.len()] },
                KeyCode::Char('p') | KeyCode::Enter => set_show(false),
                KeyCode::Esc | KeyCode::Char('h') => {
                    set_show(false);
                    self.mode = back;
                }
                _ => {}
            }
            return;
        }
        match k.code {
            KeyCode::Esc | KeyCode::Char('h') => self.mode = back,
            KeyCode::Char('p') if n > 0 => {
                set_show(true);
                let order = best(&self.game.reel.clips);
                let start = if order.contains(&sel) { sel } else { order[0] };
                self.mode = Mode::Highlights { sel: start };
            }
            KeyCode::Char('e') => {
                let said = match super::export::export_fort(self, None) {
                    Ok(e) => e.summary(),
                    Err(e) => format!("Couldn't export the fort: {}", e),
                };
                self.flash = Some(said.clone());
                EXPORTED.with(|e| *e.borrow_mut() = Some(said));
            }
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

    /// With a screen up, a clip is the map as drawn: a dig designation
    /// (which the reel itself knows nothing of) is in its ground, and each
    /// new frame is filmed from the map too.
    #[test]
    fn a_clip_is_filmed_as_the_map_draws_it() {
        use crate::game::reel::{code, FRAME_TICKS};
        let mut app = App::with_game_for_tests(Game::new(5));
        app.mode = Mode::Normal;
        let at = app.game.dwarves[0].pos;
        let mark = Pos::new(at.x + 2, at.y, at.z);
        app.game.world.t_mut(mark).desig = crate::world::Desig::Dig;
        app.game.film_for_test("An artifact", "A thing of beauty.", Some(at));
        let view = (app.view_x, app.view_y, app.cursor);
        app.film_drawn();
        assert_eq!((app.view_x, app.view_y, app.cursor), view, "the view is left as it was");
        let c = &app.game.reel.clips[0];
        assert_eq!(c.drawn, 1);
        let i = ((mark.y - c.corner.y) * CLIP_W + mark.x - c.corner.x) as usize;
        assert_eq!(c.paper.chars().nth(i), Some(code((110, 85, 0))), "the designation is in the ground");
        let j = ((at.y - c.corner.y) * CLIP_W + at.x - c.corner.x) as usize;
        assert!(matches!(c.ground.chars().nth(j), Some('☺' | '☻')), "and the dwarf, where they stood");
        for _ in 0..FRAME_TICKS * 3 {
            app.game.step();
            app.film_drawn();
        }
        let c = &app.game.reel.clips[0];
        assert!(c.frames.len() >= 3 && c.drawn as usize == c.frames.len(), "{} frames, {} drawn", c.frames.len(), c.drawn);
    }

    /// `p` plays the best moments on the map, one after another.
    #[test]
    fn the_show_plays_the_best_moments_in_turn() {
        let mut app = App::with_game_for_tests(Game::new(5));
        app.mode = Mode::Normal;
        let at = app.game.dwarves[0].pos;
        app.game.film_for_test("An artifact", "Urist has created a thing of beauty!", Some(at));
        app.game.tick += 3 * crate::game::DAY;
        app.game.film_for_test("Traders", "Merchants.", Some(at));
        app.game.tick += 3 * crate::game::DAY;
        app.game.film_for_test("A ghost", "Kadol walks again.", Some(at));
        assert_eq!(best(&app.game.reel.clips), vec![0, 2], "the traders are left out");
        app.mode = Mode::Highlights { sel: 0 };
        app.on_key(KeyEvent::from(KeyCode::Char('p')));
        app.frame_dt = 0.0;
        let s = dump(&mut app, 120, 40);
        assert!(s.contains("An artifact") && s.contains("1 of 2") && s.contains("thing of beauty"), "{}", s);
        assert!(s.contains('☺') || s.contains('☻'));
        // Past the clip's end, the next.
        app.frame_dt = 10.0;
        let s = dump(&mut app, 120, 40);
        assert!(s.contains("A ghost") && s.contains("2 of 2"), "{}", s);
        assert_eq!(app.mode, Mode::Highlights { sel: 2 });
        // p goes back to the list, Esc out.
        app.frame_dt = 0.0;
        app.on_key(KeyEvent::from(KeyCode::Char('p')));
        let s = dump(&mut app, 120, 40);
        assert!(s.contains("Highlights of"));
        app.on_key(KeyEvent::from(KeyCode::Esc));
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn an_empty_reel_says_so() {
        let mut app = App::with_game_for_tests(Game::new(5));
        app.mode = Mode::Highlights { sel: usize::MAX };
        let s = dump(&mut app, 80, 24);
        assert!(s.contains("Nothing worth remembering"));
    }
}
