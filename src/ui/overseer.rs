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

/// The eight traits with a box each: ☑ for the overseer's own. Two at most.
fn draw_overseer_traits(f: &mut Frame, area: Rect, app: &App, sel: usize) {
    use crate::game::personality::Trait;
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::Span;
    let Some(o) = &app.game.overseer else {
        coming_soon(f, area, " The overseer's traits ");
        return;
    };
    let mut v: Vec<Line<'static>> = vec![plain(format!(" {}: pick up to two.", o.title()), Color::Rgb(150, 210, 255)), Line::default()];
    for (i, t) in Trait::ALL.iter().enumerate() {
        let hi = if i == sel { Style::default().bg(Color::Rgb(50, 60, 120)) } else { Style::default() };
        let on = o.has(*t);
        v.push(Line::from(vec![
            Span::styled(format!(" {} {:<12}", if on { '☑' } else { '☐' }, t.name()), hi.fg(Color::Rgb(210, 210, 210)).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:<62}", t.blurb()), hi.fg(MUTED)),
        ]));
    }
    v.push(Line::default());
    v.push(plain(" ↑↓ choose · Enter tick · Esc done".into(), MUTED));
    let h = v.len() as u16 + 2;
    popup(f, area, 82, h, " The overseer's traits ", v);
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
