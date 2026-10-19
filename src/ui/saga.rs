//! The illuminated saga page (`game/sagas.rs`): a framed parchment with the
//! saga's title as a banner, a drop cap drawn in block characters, its
//! chapters with their dates and a progress glyph in the margin, and a
//! closing line, in crimson for doom and gold for glory. Shown as a card
//! when a saga turns or ends (the lines type themselves in), on the Sagas
//! screen (`S`), and from the game-over screen.

use super::{add_target, wrap_words, Target, MUTED};
use crate::game::sagas::{Beat, Saga, SagaKind, WorldSaga};
use crate::game::Game;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

/// A saga ready to draw, from this fort or from the world's legends.
#[derive(Clone, Debug)]
pub struct Page {
    pub title: String,
    pub doom: bool,
    pub open: bool,
    /// "pride and fall · Anvilbeard, 203".
    pub sub: String,
    /// (date, beat, text).
    pub chapters: Vec<(String, Beat, String)>,
    pub closing: String,
    /// It ended its fort.
    pub fell: bool,
}

/// This fort's saga as a page.
pub fn page_of(g: &Game, s: &Saga) -> Page {
    let fell = g.sagas.fell_by == Some(s.id);
    let year = g.start_year + (s.chapters.first().map_or(g.tick, |c| c.tick) / crate::game::YEAR) as i32;
    Page {
        title: s.title.clone(),
        doom: s.doom || (s.open() && s.kind == SagaKind::LastStand),
        open: s.open(),
        sub: format!("a saga of {} · {}, {}", s.kind.name(), g.fortress_name, year),
        chapters: s.chapters.iter().map(|c| (g.short_date(c.tick).trim().to_string(), c.beat, c.text.clone())).collect(),
        closing: if s.open() { "The saga is not yet ended...".to_string() } else { s.closing.clone() },
        fell,
    }
}

/// A saga the world remembers as a page.
pub fn page_of_world(w: &WorldSaga) -> Page {
    Page {
        title: w.title.clone(),
        doom: w.doom,
        open: false,
        sub: format!("a saga of {} · {}, {}", w.kind.name(), w.fort, w.year),
        chapters: w.chapters.clone(),
        closing: w.closing.clone(),
        fell: w.fell,
    }
}

/// Every saga to list: this fort's told ones (newest first), then those
/// of other forts the world remembers.
pub fn pages(g: &Game) -> (Vec<Page>, usize) {
    let mut v: Vec<Page> = g.told_sagas().into_iter().map(|s| page_of(g, s)).collect();
    let ours = v.len();
    let fort = &g.fortress_name;
    v.extend(g.overworld.sagas.iter().rev().filter(|w| &w.fort != fort).map(page_of_world));
    (v, ours)
}

const PARCHMENT: Color = Color::Rgb(36, 27, 17);
const INK: Color = Color::Rgb(236, 222, 188);
const FADED: Color = Color::Rgb(168, 148, 110);

/// The colour of a saga: crimson for doom, gold for glory.
pub fn hue(doom: bool) -> Color {
    if doom {
        Color::Rgb(214, 48, 58)
    } else {
        Color::Rgb(238, 188, 64)
    }
}

/// A darker shade of it, for the vines behind the drop cap.
fn shade(doom: bool) -> Color {
    if doom {
        Color::Rgb(96, 30, 34)
    } else {
        Color::Rgb(104, 82, 34)
    }
}

/// The drop cap's letters, five by five.
fn cap_rows(c: char) -> Option<[&'static str; 5]> {
    Some(match c.to_ascii_uppercase() {
        'A' => [" ### ", "#   #", "#####", "#   #", "#   #"],
        'B' => ["#### ", "#   #", "#### ", "#   #", "#### "],
        'C' => [" ####", "#    ", "#    ", "#    ", " ####"],
        'D' => ["#### ", "#   #", "#   #", "#   #", "#### "],
        'E' => ["#####", "#    ", "#### ", "#    ", "#####"],
        'F' => ["#####", "#    ", "#### ", "#    ", "#    "],
        'G' => [" ####", "#    ", "#  ##", "#   #", " ####"],
        'H' => ["#   #", "#   #", "#####", "#   #", "#   #"],
        'I' => ["#####", "  #  ", "  #  ", "  #  ", "#####"],
        'J' => ["#####", "   # ", "   # ", "#  # ", " ##  "],
        'K' => ["#   #", "#  # ", "###  ", "#  # ", "#   #"],
        'L' => ["#    ", "#    ", "#    ", "#    ", "#####"],
        'M' => ["#   #", "## ##", "# # #", "#   #", "#   #"],
        'N' => ["#   #", "##  #", "# # #", "#  ##", "#   #"],
        'O' => [" ### ", "#   #", "#   #", "#   #", " ### "],
        'P' => ["#### ", "#   #", "#### ", "#    ", "#    "],
        'Q' => [" ### ", "#   #", "# # #", "#  # ", " ## #"],
        'R' => ["#### ", "#   #", "#### ", "#  # ", "#   #"],
        'S' => [" ####", "#    ", " ### ", "    #", "#### "],
        'T' => ["#####", "  #  ", "  #  ", "  #  ", "  #  "],
        'U' => ["#   #", "#   #", "#   #", "#   #", " ### "],
        'V' => ["#   #", "#   #", "#   #", " # # ", "  #  "],
        'W' => ["#   #", "#   #", "# # #", "## ##", "#   #"],
        'X' => ["#   #", " # # ", "  #  ", " # # ", "#   #"],
        'Y' => ["#   #", " # # ", "  #  ", "  #  ", "  #  "],
        'Z' => ["#####", "   # ", "  #  ", " #   ", "#####"],
        '0' => [" ### ", "#  ##", "# # #", "##  #", " ### "],
        '1' => ["  #  ", " ##  ", "  #  ", "  #  ", " ### "],
        '2' => [" ### ", "#   #", "  ## ", " #   ", "#####"],
        '3' => ["#### ", "    #", " ### ", "    #", "#### "],
        '4' => ["#   #", "#   #", "#####", "    #", "    #"],
        '5' => ["#####", "#    ", "#### ", "    #", "#### "],
        '6' => [" ### ", "#    ", "#### ", "#   #", " ### "],
        '7' => ["#####", "    #", "   # ", "  #  ", "  #  "],
        '8' => [" ### ", "#   #", " ### ", "#   #", " ### "],
        '9' => [" ### ", "#   #", " ####", "    #", " ### "],
        _ => return None,
    })
}

/// The drop cap in its box: seven rows of seven columns.
pub fn drop_cap(c: char, doom: bool) -> Vec<Line<'static>> {
    let (ink, vine) = (Style::default().fg(hue(doom)).bg(PARCHMENT), Style::default().fg(shade(doom)).bg(PARCHMENT));
    let frame = Style::default().fg(hue(doom)).bg(PARCHMENT);
    let rows = cap_rows(c).unwrap_or(["     ", "     ", "  #  ", "     ", "     "]);
    let mut v = vec![Line::from(Span::styled("╔═════╗", frame))];
    for (y, r) in rows.iter().enumerate() {
        let mut spans = vec![Span::styled("║", frame)];
        for (x, b) in r.chars().enumerate() {
            if b == '#' {
                spans.push(Span::styled("█", ink));
            } else {
                // A vine winding behind the letter.
                spans.push(Span::styled(if (x + y) % 3 == 0 { "♣" } else if (x * 2 + y) % 4 == 1 { "·" } else { " " }, vine));
            }
        }
        spans.push(Span::styled("║", frame));
        v.push(Line::from(spans));
    }
    v.push(Line::from(Span::styled("╚═════╝", frame)));
    v
}

/// The title as a banner: spaced capitals when they fit, else plain ones.
fn banner(title: &str, w: usize) -> String {
    let caps = title.to_uppercase();
    let spaced: String = caps.chars().map(|c| c.to_string()).collect::<Vec<_>>().join(" ");
    if spaced.chars().count() + 6 <= w {
        spaced
    } else if caps.chars().count() + 4 <= w {
        caps
    } else {
        caps.chars().take(w.saturating_sub(2)).collect()
    }
}

/// Pad `spans` (taking `used` columns) to `w` on the parchment.
fn pad(mut spans: Vec<Span<'static>>, used: usize, w: usize) -> Vec<Span<'static>> {
    if used < w {
        spans.push(Span::styled(" ".repeat(w - used), Style::default().bg(PARCHMENT)));
    }
    spans
}

fn width(spans: &[Span]) -> usize {
    spans.iter().map(|s| s.content.chars().count()).sum()
}

/// A line centred in `w` columns.
fn centred(text: &str, w: usize, st: Style) -> Vec<Span<'static>> {
    let n = text.chars().count().min(w);
    let left = (w - n) / 2;
    let t: String = text.chars().take(n).collect();
    pad(vec![Span::styled(" ".repeat(left), Style::default().bg(PARCHMENT)), Span::styled(t, st)], left + n, w)
}

/// The page's lines inside the frame, `inner` columns wide: the banner,
/// the drop cap beside the first chapter, the rest, and the closing.
/// `reveal`: how many characters of the chapters and closing have been
/// written so far (None: all).
pub fn body(p: &Page, inner: usize, reveal: Option<usize>) -> Vec<Vec<Span<'static>>> {
    let bg = Style::default().bg(PARCHMENT);
    let gold = Style::default().fg(hue(p.doom)).bg(PARCHMENT);
    let mut rows: Vec<Vec<Span<'static>>> = Vec::new();
    // The banner.
    let b = banner(&p.title, inner.saturating_sub(4));
    let rule = inner.saturating_sub(b.chars().count() + 4) / 2;
    let line = format!("{} ☼ {} ☼ {}", "═".repeat(rule.saturating_sub(1)), b, "═".repeat(rule.saturating_sub(1)));
    rows.push(centred(&line, inner, gold.add_modifier(Modifier::BOLD)));
    rows.push(centred(&p.sub, inner, Style::default().fg(FADED).bg(PARCHMENT).add_modifier(Modifier::ITALIC)));
    rows.push(pad(vec![], 0, inner));
    // The text: the first chapter's first letter is the drop cap.
    let gutter = 9usize;
    let tw = inner.saturating_sub(gutter + 1).max(8);
    let mut budget = reveal.unwrap_or(usize::MAX);
    let mut text_rows: Vec<(Option<char>, Vec<Span<'static>>)> = Vec::new();
    let mut first_letter = ' ';
    for (k, (date, beat, text)) in p.chapters.iter().enumerate() {
        let mut t = text.clone();
        if k == 0 {
            let mut cs = t.chars();
            first_letter = cs.next().unwrap_or(' ');
            t = cs.as_str().to_string();
        }
        let dated = format!("{} — {}", date, t);
        let lines = wrap_words(&dated, tw);
        for (j, l) in lines.iter().enumerate() {
            if budget == 0 {
                break;
            }
            let shown: String = l.chars().take(budget).collect();
            budget = budget.saturating_sub(l.chars().count());
            // The date in faded ink, the rest dark on the parchment.
            let spans = if j == 0 {
                let d = date.chars().count().min(shown.chars().count());
                let (a, b): (String, String) = (shown.chars().take(d).collect(), shown.chars().skip(d).collect());
                vec![Span::styled(a, Style::default().fg(FADED).bg(PARCHMENT)), Span::styled(b, Style::default().fg(INK).bg(PARCHMENT))]
            } else {
                vec![Span::styled(shown, Style::default().fg(INK).bg(PARCHMENT))]
            };
            text_rows.push((if j == 0 { Some(beat.glyph(p.doom && *beat == Beat::Ending)) } else { None }, spans));
        }
        if budget > 0 && k + 1 < p.chapters.len() {
            text_rows.push((None, vec![]));
        }
    }
    let cap = drop_cap(first_letter, p.doom);
    let n = text_rows.len().max(cap.len());
    for i in 0..n {
        let mut spans: Vec<Span<'static>> = vec![Span::styled(" ", bg)];
        if i < cap.len() {
            spans.extend(cap[i].spans.iter().cloned());
            spans.push(Span::styled(" ", bg));
        } else {
            let g = text_rows.get(i).and_then(|r| r.0);
            spans.push(Span::styled(format!("    {}   ", g.unwrap_or(' ')), gold));
        }
        if let Some((_, t)) = text_rows.get(i) {
            spans.extend(t.iter().cloned());
        }
        let used = width(&spans);
        rows.push(pad(spans, used, inner));
    }
    // The closing line.
    rows.push(pad(vec![], 0, inner));
    if budget > 0 && !p.closing.is_empty() {
        let shown: String = p.closing.chars().take(budget).collect();
        for l in wrap_words(&format!("~ {} ~", shown), inner.saturating_sub(4)) {
            rows.push(centred(&l, inner, gold.add_modifier(Modifier::ITALIC)));
        }
    }
    rows
}

/// The whole framed page in `r` (clipped to `clip`), with `footer` in
/// the bottom border.
pub fn draw_page(f: &mut Frame, r: Rect, clip: Rect, p: &Page, reveal: Option<usize>, footer: &str) {
    let r = r.intersection(clip);
    if r.width < 12 || r.height < 5 {
        // No room for the frame: the title alone.
        let r = r.intersection(f.size());
        f.render_widget(Clear, r);
        f.render_widget(Paragraph::new(Span::styled(p.title.clone(), Style::default().fg(hue(p.doom)))), r);
        return;
    }
    let w = r.width as usize;
    let inner = w - 2;
    let edge = Style::default().fg(hue(p.doom)).bg(PARCHMENT);
    let mut lines: Vec<Line<'static>> = Vec::new();
    // The top border, with diamonds near its corners.
    let mut top = String::from("╔");
    for i in 0..inner {
        top.push(if i == 1 || i + 2 == inner { '♦' } else { '═' });
    }
    top.push('╗');
    lines.push(Line::from(Span::styled(top, edge)));
    let mut rows = body(p, inner, reveal);
    let room = (r.height as usize).saturating_sub(3);
    rows.truncate(room);
    while rows.len() < room {
        rows.push(pad(vec![], 0, inner));
    }
    // The progress glyphs and the footer on the last row inside.
    let glyphs: String = p.chapters.iter().map(|c| c.1.glyph(p.doom && c.1 == Beat::Ending)).collect();
    let mut last = vec![Span::styled(format!(" {} ", glyphs), edge)];
    let used = width(&last);
    let foot: String = footer.chars().take(inner.saturating_sub(used + 1)).collect();
    let gap = inner.saturating_sub(used + foot.chars().count() + 1);
    last.push(Span::styled(" ".repeat(gap), Style::default().bg(PARCHMENT)));
    last.push(Span::styled(foot, Style::default().fg(MUTED).bg(PARCHMENT)));
    last.push(Span::styled(" ", Style::default().bg(PARCHMENT)));
    rows.push(last);
    for row in rows {
        let mut spans = vec![Span::styled("║", edge)];
        let used = width(&row);
        spans.extend(row);
        if used < inner {
            spans.push(Span::styled(" ".repeat(inner - used), Style::default().bg(PARCHMENT)));
        }
        spans.push(Span::styled("║", edge));
        lines.push(Line::from(spans));
    }
    let mut bottom = String::from("╚");
    for i in 0..inner {
        bottom.push(if i == 1 || i + 2 == inner { '♦' } else { '═' });
    }
    bottom.push('╝');
    lines.push(Line::from(Span::styled(bottom, edge)));
    f.render_widget(Clear, r);
    f.render_widget(Paragraph::new(lines).style(Style::default().bg(PARCHMENT)), r);
}

/// How many rows a page needs at `w` columns, frame and all.
pub fn rows_needed(p: &Page, w: u16) -> u16 {
    body(p, (w as usize).saturating_sub(2), None).len() as u16 + 3
}

/// Characters typed in per frame of the card's reveal.
const TYPING: usize = 5;

/// A saga's card over the map, typing itself in: `frames` since it came up.
pub fn draw_card(f: &mut Frame, map: Rect, g: &Game, id: u32, frames: u32, auto: bool) {
    let Some(s) = g.saga(id) else { return };
    let p = page_of(g, s);
    let w = map.width.saturating_sub(4).clamp(map.width.min(30), 76);
    let h = rows_needed(&p, w).min(map.height);
    let r = Rect::new(map.x + map.width.saturating_sub(w) / 2, map.y + map.height.saturating_sub(h) / 2, w, h);
    let foot = if auto { "(the overseer carries on)" } else { "Enter: go on · S: every saga" };
    draw_page(f, r, map, &p, Some(frames as usize * TYPING), foot);
}

/// The Sagas screen: the list, and the selected one's page (`open`: the
/// page alone, full size).
pub fn draw_screen(f: &mut Frame, area: Rect, g: &Game, sel: usize, open: bool) {
    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new("").style(Style::default().bg(Color::Rgb(14, 11, 8))), area);
    add_target(area, Target::Block);
    let (pages, ours) = pages(g);
    let title = Style::default().fg(Color::Rgb(238, 188, 64)).add_modifier(Modifier::BOLD);
    if pages.is_empty() {
        let lines = vec![
            Line::default(),
            Line::from(Span::styled(format!("  The Sagas of {}", g.fortress_name), title)),
            Line::default(),
            Line::from(Span::styled("  No saga has been told yet. When the fort's deeds link into a tale (a pride", Style::default().fg(INK))),
            Line::from(Span::styled("  and its fall, a grudge, a hero's rise, an omen come true) it is written here.", Style::default().fg(INK))),
            Line::default(),
            Line::from(Span::styled("  Esc: back", Style::default().fg(MUTED))),
        ];
        f.render_widget(Paragraph::new(lines), area);
        return;
    }
    let sel = sel.min(pages.len() - 1);
    let side = area.width >= 90 && !open;
    let list_w = if open { 0 } else if side { 40 } else { area.width };
    if list_w > 0 {
        let mut lines = vec![Line::from(Span::styled(format!(" The Sagas of {}", g.fortress_name), title)), Line::default()];
        let mut sel_row = 0;
        for (i, p) in pages.iter().enumerate() {
            if i == ours {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(" Sagas of other forts, from the legends", title)));
            }
            if i == sel {
                sel_row = lines.len();
            }
            let hi = if i == sel { Style::default().bg(Color::Rgb(60, 46, 26)) } else { Style::default() };
            let glyphs: String = p.chapters.iter().map(|c| c.1.glyph(p.doom && c.1 == Beat::Ending)).collect();
            let mark = if p.fell { "☠" } else if p.open { "…" } else { " " };
            let t: String = p.title.chars().take((list_w as usize).saturating_sub(glyphs.chars().count() + 6)).collect();
            lines.push(Line::from(vec![
                Span::styled(format!(" {} ", mark), hi.fg(hue(p.doom))),
                Span::styled(t, hi.fg(if p.open { INK } else { hue(p.doom) })),
                Span::styled(format!(" {}", glyphs), hi.fg(FADED)),
            ]));
        }
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(" ↑↓ choose · Enter: read · Esc: back", Style::default().fg(MUTED))));
        lines.push(Line::from(Span::styled(" ○ cause  ◘ rising  ● turn  ☼ glory  † doom", Style::default().fg(FADED))));
        let h = area.height as usize;
        let skip = sel_row.saturating_sub(h.saturating_sub(4));
        let r = Rect::new(area.x, area.y, list_w.min(area.width), area.height);
        f.render_widget(Paragraph::new(lines.into_iter().skip(skip).collect::<Vec<_>>()), r);
    }
    if open || side {
        let x = area.x + list_w;
        let w = area.width.saturating_sub(list_w).min(90);
        let pw = pages[sel].clone();
        let h = rows_needed(&pw, w).min(area.height);
        let r = Rect::new(x + (area.width - list_w).saturating_sub(w) / 2, area.y + area.height.saturating_sub(h) / 2, w, h);
        draw_page(f, r, area, &pw, None, if open { "Esc: the list" } else { "" });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, Mode};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn sample() -> Page {
        Page {
            title: "The Pride of Anvilbeard".into(),
            doom: true,
            open: false,
            sub: "a saga of pride and fall · Anvilbeard, 203".into(),
            chapters: vec![
                ("3 Granite 203".into(), Beat::Cause, "The sally from Anvilbeard broke the goblins at the gate: 5 rode out, 3 did not come back.".into()),
                ("9 Slate 203".into(), Beat::Escalation, "21 goblins came back as a sworn host, armed in their best, to avenge their dead.".into()),
                ("12 Slate 203".into(), Beat::Turn, "The sworn host were beaten off, and 4 of ours fell.".into()),
                ("17 Slate 203".into(), Beat::Ending, "Five days later, a dragon came upon Anvilbeard, bereft of soldiers, and none were left.".into()),
            ],
            closing: "The overseer's pride had been dearly bought.".into(),
            fell: true,
        }
    }

    fn dump(t: &Terminal<TestBackend>) -> String {
        let b = t.backend().buffer();
        let mut s = String::new();
        for y in 0..b.area.height {
            for x in 0..b.area.width {
                s.push_str(b.get(x, y).symbol());
            }
            s.push('\n');
        }
        s
    }

    #[test]
    fn the_page_renders_at_any_size() {
        for (w, h) in [(1, 1), (8, 3), (20, 8), (34, 14), (60, 30), (80, 24), (160, 60), (300, 90)] {
            let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
            t.draw(|f| {
                let a = f.size();
                draw_page(f, a, a, &sample(), None, "Enter: go on");
                // A card typed partway, and one placed off the edge.
                draw_page(f, Rect::new(a.width / 2, a.height / 2, 70, 40), a, &sample(), Some(17), "");
            })
            .unwrap();
        }
        let mut t = Terminal::new(TestBackend::new(80, 30)).unwrap();
        t.draw(|f| draw_page(f, f.size(), f.size(), &sample(), None, "Enter: go on")).unwrap();
        let s = dump(&t);
        assert!(s.contains("T H E   P R I D E   O F   A N V I L B E A R D"), "{}", s);
        assert!(s.contains("█████"), "the drop cap: {}", s);
        assert!(s.contains("dearly bought"), "{}", s);
        assert!(s.contains("○◘●†"), "{}", s);
    }

    #[test]
    fn the_card_types_itself_in() {
        let p = sample();
        let few = body(&p, 70, Some(10));
        let all = body(&p, 70, None);
        let text = |rows: &Vec<Vec<Span>>| rows.iter().map(|r| r.iter().map(|s| s.content.to_string()).collect::<String>()).collect::<String>();
        assert!(!text(&few).contains("dragon") && text(&all).contains("dragon"));
    }

    #[test]
    fn the_sagas_screen_and_card_draw_from_a_fort() {
        let mut app = App::new(31);
        app.game = Game::new(31);
        app.mode = Mode::Normal;
        // A great siege, told.
        app.game.saga_demo();
        let id = app.game.told_sagas()[0].id;
        for (w, h) in [(40, 12), (100, 40), (200, 60)] {
            let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
            t.draw(|f| {
                let a = f.size();
                draw_card(f, a, &app.game, id, 3, true);
                draw_card(f, a, &app.game, id, 1000, false);
                draw_screen(f, a, &app.game, 0, false);
                draw_screen(f, a, &app.game, 5, true);
            })
            .unwrap();
        }
        let mut t = Terminal::new(TestBackend::new(120, 40)).unwrap();
        t.draw(|f| draw_screen(f, f.size(), &app.game, 0, false)).unwrap();
        assert!(dump(&t).contains(&app.game.told_sagas()[0].title), "{}", dump(&t));
    }
}
