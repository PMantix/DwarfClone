//! The atlas (M4 B01, `W` in play): the world map as it is now, with the
//! fort, the ruins history has made since the embark, and the armies on the
//! march toward the fort; beside it, who is coming and the year's news.

use super::*;
use crate::game::worldturn::PartyKind;

/// The fort's mark on the atlas.
pub const FORT_GLYPH: char = '☼';
/// An army on the march.
pub const ARMY_GLYPH: char = '!';

/// What the atlas puts over the world map: the fort, and each party where
/// it is now (glyph, foreground, background).
pub fn overlay(g: &Game) -> Vec<((i32, i32), (char, Color, Color))> {
    let mut v = Vec::new();
    for p in &g.worldturn.parties {
        let at = p.at(g.tick);
        let look = match p.kind {
            PartyKind::Army => (ARMY_GLYPH, g.party_race(p).color(), Color::Rgb(110, 0, 0)),
        };
        v.push((at, look));
    }
    v.push((g.site, (FORT_GLYPH, Color::Rgb(255, 230, 120), Color::Rgb(60, 40, 0))));
    v
}

/// The side panel's lines: armies on the march, then the news.
pub fn panel_lines(g: &Game) -> Vec<Line<'static>> {
    let mut v = vec![Line::from(Span::styled("On the march", Style::default().fg(KEY)))];
    if g.worldturn.parties.is_empty() {
        v.push(Line::from(Span::styled("  No army is marching on the fort.", Style::default().fg(MUTED))));
    }
    for p in &g.worldturn.parties {
        let civ = g.overworld.civs.get(p.civ as usize).map_or(String::new(), |c| c.name.clone());
        let from = p.from.and_then(|f| g.overworld.sites.get(f as usize)).map_or(String::new(), |s| format!(" from {}", s.name));
        let days = p.days_out(g.tick);
        let when = if days <= 1 { "a day away".to_string() } else { format!("{} days away", days) };
        v.push(Line::from(vec![
            Span::styled(format!(" {} ", ARMY_GLYPH), Style::default().fg(g.party_race(p).color()).bg(Color::Rgb(110, 0, 0))),
            Span::styled(format!(" {} of the {} under {}{}, {}", p.size, civ, p.leader, from, when), Style::default().fg(TEXT)),
        ]));
    }
    v.push(Line::default());
    v.push(Line::from(Span::styled("News from the world", Style::default().fg(KEY))));
    if g.worldturn.news.is_empty() {
        v.push(Line::from(Span::styled("  No news yet: it comes each spring.", Style::default().fg(MUTED))));
    }
    for (y, text) in g.worldturn.news.iter().rev().take(12) {
        v.push(Line::from(Span::styled(format!(" {}: {}", y, text), Style::default().fg(TEXT))));
    }
    v
}

pub fn draw_atlas(f: &mut Frame, area: Rect, app: &App, cx: i32, cy: i32) {
    let g = &app.game;
    let ow = &g.overworld;
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(KEY))
        .style(Style::default().bg(Color::Rgb(15, 15, 25)))
        .title(Span::styled(format!(" Atlas of {} · year {} ", ow.name, g.start_year + (g.tick / crate::game::YEAR) as i32), Style::default().fg(KEY)))
        .title_bottom(Span::styled(" arrows move · f the fort · Esc back ", Style::default().fg(MUTED)));
    let inner = block.inner(area);
    f.render_widget(block, area);
    let side = (inner.width / 3).clamp(24, 52).min(inner.width.saturating_sub(10));
    let parts = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Min(10), Constraint::Length(side)]).split(inner);
    let marks = overlay(g);
    let look = |x: i32, y: i32| marks.iter().rev().find(|(p, _)| *p == (x, y)).map(|(_, l)| *l).unwrap_or_else(|| ow.map_look(x, y));
    let mark = |x: i32, y: i32| ow.cell(x, y).site > 0 || marks.iter().any(|(p, _)| *p == (x, y));
    paint_cells(f, parts[0], (ow.w, ow.h), ow.h, Some((cx, cy)), &mark, &look);
    let mut lines = panel_lines(g);
    let (cx, cy) = (cx.clamp(0, ow.w - 1), cy.clamp(0, ow.h - 1));
    let here = match ow.site_at(cx, cy) {
        Some(s) => format!("{} ({}{})", s.name, s.kind.name(), if s.ruined { ", in ruins" } else { "" }),
        None if (cx, cy) == g.site => g.fortress_name.clone(),
        None => ow.cell(cx, cy).biome.name().to_string(),
    };
    lines.insert(0, Line::from(Span::styled(here, Style::default().fg(Color::Rgb(220, 200, 0)))));
    lines.insert(1, Line::default());
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), parts[1]);
}
