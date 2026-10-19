//! Milestone 2's screens (W0-B): the overseer's traits picker (P1, O01),
//! its journal (P17, O07), a dwarf's family tree (P16, O06) and
//! announcement control (P6, O05), each a placeholder until its package
//! draws it. One function per screen.

use super::{plain, popup, MUTED};
use crate::app::{App, Mode};
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::Frame;

/// Draw whichever M2 screen is open.
pub(super) fn draw(f: &mut Frame, area: Rect, app: &App) {
    match app.mode {
        Mode::OverseerTraits { sel } => draw_overseer_traits(f, area, app, sel),
        Mode::Journal { year } => draw_journal(f, area, app, year),
        Mode::Family { id, sel } => draw_family(f, area, app, id, sel),
        Mode::Announce { sel } => draw_announce(f, area, app, sel),
        _ => {}
    }
}

fn coming_soon(f: &mut Frame, area: Rect, title: &str) {
    let v: Vec<Line<'static>> = vec![Line::default(), plain("  Coming soon.".into(), MUTED), Line::default(), plain("  Esc back".into(), MUTED)];
    popup(f, area, 40, 6, title, v);
}

fn draw_overseer_traits(f: &mut Frame, area: Rect, _app: &App, _sel: usize) {
    coming_soon(f, area, " The overseer's traits ");
}

fn draw_journal(f: &mut Frame, area: Rect, _app: &App, _year: usize) {
    coming_soon(f, area, " The overseer's journal ");
}

fn draw_family(f: &mut Frame, area: Rect, _app: &App, _id: u32, _sel: usize) {
    coming_soon(f, area, " Family ");
}

fn draw_announce(f: &mut Frame, area: Rect, _app: &App, _sel: usize) {
    coming_soon(f, area, " Announcements ");
}
