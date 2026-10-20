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
        Mode::Journal { year, scroll } => draw_journal(f, area, app, year, scroll),
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

/// The journal's page width inside its border (it narrows on a small screen).
const JOURNAL_W: u16 = 86;

/// Wrap `text` to `width` columns at the spaces.
fn wrap_words(text: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let n = line.chars().count();
        if n > 0 && n + 1 + word.chars().count() > width {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() || out.is_empty() {
        out.push(line);
    }
    out
}

/// A journal page's body, laid out to `width` columns: each entry with its
/// day in the margin (once a day), the dead marked with a cross, its
/// secrets in red ink, its jottings faded, and the year's last word.
fn journal_body(app: &App, page: usize, width: usize) -> Vec<Line<'static>> {
    use crate::game::journal::EntryKind;
    use ratatui::layout::Alignment;
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::Span;
    let g = &app.game;
    let Some(p) = g.overseer.as_ref().and_then(|o| o.journal.get(page)) else { return Vec::new() };
    let ink = Style::default().fg(Color::Rgb(40, 32, 70));
    let faded = Style::default().fg(Color::Rgb(128, 108, 88));
    let margin = Style::default().fg(Color::Rgb(150, 75, 45));
    let red = Style::default().fg(Color::Rgb(160, 30, 25)).add_modifier(Modifier::ITALIC);
    let violet = Style::default().fg(Color::Rgb(100, 40, 130)).add_modifier(Modifier::BOLD);
    let pride = Style::default().fg(Color::Rgb(30, 80, 50));
    let fear = Style::default().fg(Color::Rgb(90, 45, 40));
    const M: usize = 12;
    let text_w = width.saturating_sub(M).max(10);
    let mut v: Vec<Line<'static>> = Vec::new();
    let mut last_day = String::new();
    for e in &p.entries {
        let date = g.short_date(e.at);
        let day: String = date.trim().rsplitn(2, ' ').nth(1).unwrap_or("").to_string();
        if e.kind == EntryKind::Season {
            let season = e.text.split_whitespace().next().unwrap_or("").to_string();
            v.push(Line::default());
            v.push(Line::from(Span::styled(format!("~ {} ~", season), margin.add_modifier(Modifier::BOLD))).alignment(Alignment::Center));
            for l in wrap_words(&e.text, width.saturating_sub(4)) {
                v.push(Line::from(Span::styled(format!("  {}", l), ink.add_modifier(Modifier::ITALIC))));
            }
            v.push(Line::default());
            last_day.clear();
            continue;
        }
        let (mark, style) = match e.kind {
            EntryKind::Jot => ("", faded),
            EntryKind::Dead => ("\u{2020} ", ink.add_modifier(Modifier::BOLD)),
            EntryKind::Secret => ("", red),
            EntryKind::Changed => ("", violet),
            EntryKind::Pride => ("", pride),
            EntryKind::Fear => ("", fear),
            _ => ("", ink),
        };
        let shown = if day != last_day { format!("{:>w$}  ", day, w = M - 2) } else { " ".repeat(M) };
        last_day = day;
        for (i, l) in wrap_words(&format!("{}{}", mark, e.text), text_w).into_iter().enumerate() {
            let m = if i == 0 { shown.clone() } else { " ".repeat(M) };
            v.push(Line::from(vec![Span::styled(m, margin), Span::styled(l, style)]));
        }
    }
    let last = g.overseer.as_ref().map_or(0, |o| o.journal.len()) == page + 1;
    if !p.lines.is_empty() {
        v.push(Line::default());
        v.push(Line::from(Span::styled("\u{2767}", margin)).alignment(Alignment::Center));
        for l in &p.lines {
            for w in wrap_words(l, width.saturating_sub(4)) {
                v.push(Line::from(Span::styled(format!("  {}", w), ink.add_modifier(Modifier::ITALIC))));
            }
        }
    } else if last && g.alive_count() == 0 {
        v.push(Line::default());
        v.push(Line::from(Span::styled("The rest of the pages are blank.", faded.add_modifier(Modifier::ITALIC))).alignment(Alignment::Center));
    } else if last {
        v.push(Line::default());
        v.push(Line::from(Span::styled("the ink is still wet", faded.add_modifier(Modifier::ITALIC))).alignment(Alignment::Center));
    }
    v
}

/// How many rows a journal page's body takes (for the keys' scrolling).
pub(crate) fn journal_rows(app: &App, page: usize) -> usize {
    journal_body(app, page, JOURNAL_W as usize - 4).len()
}

/// The overseer's journal (O07): one page for each year of each reign, in
/// its own hand, on parchment like the chronicle's but its own.
fn draw_journal(f: &mut Frame, area: Rect, app: &App, page: usize, scroll: usize) {
    use ratatui::layout::Alignment;
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::Span;
    use ratatui::widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph};
    let g = &app.game;
    let pages = g.overseer.as_ref().map_or(0, |o| o.journal.len());
    if pages == 0 {
        let v = vec![
            Line::default(),
            plain("  The overseer has not begun a journal yet.".into(), MUTED),
            plain("  It starts one when it takes charge of the fortress (A).".into(), MUTED),
            Line::default(),
            plain("  Esc back".into(), MUTED),
        ];
        popup(f, area, 64, 7, " The overseer's journal ", v);
        return;
    }
    let page = page.min(pages - 1);
    let p = &g.overseer.as_ref().unwrap().journal[page];
    let paper = Color::Rgb(224, 208, 168);
    let brown = Color::Rgb(150, 75, 45);
    let w = JOURNAL_W.min(area.width);
    let h = area.height;
    let r = super::centered(area, w, h);
    let inner = w.saturating_sub(4) as usize;
    let traits: Vec<&str> = p.traits.iter().map(|t| t.name()).collect();
    let year = g.start_year as u64 + p.year_index();
    let head = vec![
        Line::from(Span::styled(format!("The Journal of {}", p.by), Style::default().fg(Color::Rgb(115, 30, 25)).add_modifier(Modifier::BOLD))).alignment(Alignment::Center),
        Line::from(Span::styled(
            format!("{}, the {} overseer of {}", if traits.is_empty() { "overseer".to_string() } else { traits.join(" and ") }, crate::game::reign::ordinal(p.nth.max(1)), g.fortress_name),
            Style::default().fg(Color::Rgb(120, 90, 60)),
        ))
        .alignment(Alignment::Center),
        Line::from(Span::styled(format!("~ the year {} ~", year), Style::default().fg(brown))).alignment(Alignment::Center),
        Line::default(),
    ];
    let body = journal_body(app, page, inner);
    let room = (h as usize).saturating_sub(2 + head.len() + 2).max(1);
    let top = scroll.min(body.len().saturating_sub(room));
    let mut v = head;
    v.extend(body.iter().skip(top).take(room).cloned());
    while v.len() < (h as usize).saturating_sub(3) {
        v.push(Line::default());
    }
    let more = if top + room < body.len() { "\u{2193} more" } else if top > 0 { "\u{2191} above" } else { "" };
    let nav = format!(
        "{}  page {} of {}  {}    \u{2191}\u{2193} PgUp PgDn read \u{b7} Esc close  {}",
        if page > 0 { "\u{25c0}" } else { " " },
        page + 1,
        pages,
        if page + 1 < pages { "\u{25b6}" } else { " " },
        more
    );
    v.push(Line::from(Span::styled(nav, Style::default().fg(Color::Rgb(120, 90, 60)))).alignment(Alignment::Center));
    f.render_widget(Clear, r);
    f.render_widget(
        Paragraph::new(v).style(Style::default().bg(paper)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Thick)
                .border_style(Style::default().fg(Color::Rgb(95, 55, 30)).bg(paper))
                .padding(Padding::horizontal(1)),
        ),
        r,
    );
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
