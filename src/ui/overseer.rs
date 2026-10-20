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

/// Announcement control (O05, UI-07): a row per kind of announcement,
/// with whether it pauses, moves the camera and shows.
fn draw_announce(f: &mut Frame, area: Rect, app: &App, sel: usize) {
    use crate::game::logkind::LogKind;
    use ratatui::style::{Color, Style};
    use ratatui::text::Span;
    let on = |b: bool, c: Color| Span::styled(format!("{:^8}", if b { "yes" } else { "-" }), Style::default().fg(if b { c } else { MUTED }));
    let mut v: Vec<Line<'static>> = vec![
        plain(format!("  {:<22}{:^8}{:^8}{:^8}", "Announcements of", "Pause", "Camera", "Shown"), super::KEY),
        Line::default(),
    ];
    for (i, &k) in LogKind::ALL.iter().enumerate() {
        let r = app.announce.rule(k);
        let st = if i == sel { Style::default().fg(super::BLACK).bg(super::KEY) } else { Style::default().fg(super::TEXT) };
        v.push(Line::from(vec![
            Span::styled(format!("{} {:<21}", if i == sel { ">" } else { " " }, k.name()), st),
            on(r.pause, Color::Rgb(255, 140, 90)),
            on(r.recenter, Color::Rgb(140, 200, 255)),
            on(!r.hide, Color::Rgb(140, 230, 140)),
        ]));
    }
    v.push(Line::default());
    v.push(plain("  Hidden ones stay in the reports (r).".into(), MUTED));
    v.push(plain("  Up/Down choose · p pause · c camera · h hide/show · Esc back".into(), MUTED));
    let h = v.len() as u16 + 2;
    popup(f, area, 70, h, " Announcements ", v);
}

/// The announcements panel's lines (O05): the newest `n` not hidden by
/// announcement control, oldest first. What happened out of sight is
/// told as a sound (`sound_reports.rs`), the newest marked for Enter.
pub(super) fn announcement_lines(app: &App, n: usize) -> Vec<Line<'static>> {
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::Span;
    let g = &app.game;
    let ear = app.earshot();
    let mut marked = false;
    let mut v: Vec<Line<'static>> = g
        .log
        .iter()
        .rev()
        .take(600)
        .filter(|e| !app.announce.rule(e.kind()).hide)
        .take(n)
        .map(|e| match g.heard(e, ear) {
            Some(s) => {
                let (r, gr, b) = e.color;
                let mut spans = vec![Span::styled(format!("~ {}", s), Style::default().fg(Color::Rgb(r / 2 + 70, gr / 2 + 70, b / 2 + 70)).add_modifier(Modifier::ITALIC))];
                if !marked && g.tick <= e.tick + crate::game::DAY * 2 {
                    marked = true;
                    spans.push(Span::styled(" (Enter)", Style::default().fg(MUTED)));
                }
                Line::from(spans)
            }
            None => plain(e.text.clone(), super::rgb(e.color)),
        })
        .collect();
    v.reverse();
    v
}
