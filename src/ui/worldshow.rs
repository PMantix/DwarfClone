//! The world-creation screen: the map as `genshow::Show` plays its making
//! back, and beside it the stages, then history told age by age, with a
//! banner over the map while a great event holds the show.

use super::*;
use crate::genshow::{chronicle, epic_color, Stage};

pub(super) fn draw_worldgen(f: &mut Frame, app: &App) {
    let area = f.size();
    let Some(g) = &app.worldgen else { return };
    let ow = &g.ow;
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(10)])
        .split(area);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(SIDEBAR_W + 12)])
        .split(rows[1]);
    let show = app.genshow.as_ref();
    let t = app.show_t;
    let (stage, _) = show.map_or((Stage::Ocean, 0.0), |s| s.stage_at(t));
    let year = show.and_then(|s| s.year_at(t));
    let era = show.and_then(|s| s.era_at(t));
    let finished = app.worldgen_finished();
    let speed = show.map_or(1.0, |s| s.speed);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" ☼ DwarfClone ", Style::default().fg(KEY).add_modifier(Modifier::BOLD)),
            Span::styled(format!("│ Creating {} │ seed {}", ow.name, g.params.seed), Style::default().fg(TEXT)),
            Span::styled(if speed != 1.0 { format!(" │ ×{}", speed) } else { String::new() }, Style::default().fg(KEY)),
        ]))
        .style(Style::default().bg(Color::Rgb(20, 20, 30))),
        rows[0],
    );
    let title = match year {
        Some(y) => format!(" Year {} ", y),
        None => format!(" {} ", stage.short()),
    };
    let caption = match (era, stage) {
        (Some((_, e)), Stage::History) => e.name.clone(),
        _ => stage.caption().to_string(),
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(MUTED))
        .title(Span::styled(title, Style::default().fg(KEY)))
        .title_bottom(Span::styled(format!(" {} ", caption), Style::default().fg(TEXT)));
    let inner = block.inner(cols[0]);
    f.render_widget(block, cols[0]);
    if let Some(s) = show {
        let fr = s.frame(t, g);
        let w = ow.w;
        paint_cells(f, inner, (ow.w, ow.h), ow.h, None, &|x, y| fr.site[(y * w + x) as usize], &|x, y| fr.looks[(y * w + x) as usize]);
    }
    // A great event: its banner over the map, fading in and out.
    if let Some((e, q)) = show.and_then(|s| s.epic_at(t, g)).filter(|_| inner.width >= 24 && inner.height >= 8) {
        let col = epic_color(e.kind);
        let alpha = ((q / 0.12).min(1.0).min((1.0 - q) / 0.12)).clamp(0.0, 1.0) as f32;
        let fg = blend(Color::Rgb(20, 20, 20), col, alpha);
        let txt = blend(Color::Rgb(20, 20, 20), TEXT, alpha);
        let width = inner.width.min(70);
        let body = e.text.splitn(2, ": ").nth(1).unwrap_or(&e.text).to_string();
        let lines = vec![
            Line::from(Span::styled(format!("★ {} ★", e.kind.title().to_uppercase()), Style::default().fg(fg).add_modifier(Modifier::BOLD))).alignment(ratatui::layout::Alignment::Center),
            Line::from(Span::styled(e.name.clone(), Style::default().fg(fg).add_modifier(Modifier::BOLD))).alignment(ratatui::layout::Alignment::Center),
            Line::default(),
            Line::from(Span::styled(body, Style::default().fg(txt))),
        ];
        let h = (4 + (e.text.len() as u16 / width.saturating_sub(4).max(1)) + 3).min(inner.height);
        let r = Rect { x: inner.x + (inner.width - width) / 2, y: inner.y + inner.height.saturating_sub(h) / 5, width, height: h };
        f.render_widget(Clear, r);
        f.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: true }).style(Style::default().bg(Color::Rgb(15, 10, 10))).block(
                Block::default().borders(Borders::ALL).border_style(Style::default().fg(fg)),
            ),
            r,
        );
    }

    // The stages, then what history has made so far.
    let width = 30usize;
    let frac = show.map_or(0.0, |s| (t / s.end()).clamp(0.0, 1.0)) as f32;
    let filled = ((frac * width as f32).round() as usize).min(width);
    let mut v = vec![
        plain(stage.caption().to_string(), TEXT),
        Line::from(vec![
            Span::styled("█".repeat(filled), Style::default().fg(Color::Rgb(255, 200, 80))),
            Span::styled("░".repeat(width - filled), Style::default().fg(Color::Rgb(60, 55, 50))),
            Span::styled(format!(" {:>3}%", (frac * 100.0) as u32), Style::default().fg(MUTED)),
        ]),
        Line::default(),
    ];
    let all = &Stage::ALL[..Stage::ALL.len() - 1];
    if stage < Stage::History {
        for s in all {
            let (mark, col) = if *s < stage { ("✓ ", MUTED) } else if *s == stage { ("▶ ", KEY) } else { ("  ", Color::Rgb(90, 90, 100)) };
            v.push(plain(format!("{}{}", mark, s.short()), col));
        }
    } else {
        v.push(plain(format!("✓ The land is made; {}", stage.short().to_lowercase()), MUTED));
    }
    v.push(Line::default());
    let y = year.unwrap_or(0);
    let (towns, ruins) = match (show, year) {
        (Some(s), Some(y)) => s.tally(g, y),
        _ => (0, 0),
    };
    let peoples = match (show, year) {
        (Some(s), Some(y)) => s.peoples(g, y),
        _ => 0,
    };
    v.push(plain(format!("Year {:>4} of {:<4}  Peoples {:>3}", y, g.params.history, peoples), TEXT));
    v.push(plain(format!("Settlements {:>4}   Ruins   {:>3}", towns, ruins), TEXT));
    if let (Some(s), Some(y)) = (show, year) {
        let none = Vec::new();
        let clashes = g.trace.as_ref().map_or(&none, |t| &t.clashes);
        // The ages gone by, in a line each.
        let (now, _) = era.map_or((usize::MAX, None), |(k, e)| (k, Some(e)));
        v.push(Line::default());
        v.push(header("The ages"));
        for (k, e) in s.eras().iter().enumerate() {
            if k > now {
                break;
            }
            let on = k == now;
            v.push(Line::from(vec![
                Span::styled(format!("{:>4}  ", e.from), Style::default().fg(MUTED)),
                Span::styled(e.name.clone(), if on { Style::default().fg(KEY).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::Rgb(150, 150, 160)) }),
            ]));
        }
        if let Some((_, e)) = era {
            v.push(Line::default());
            v.push(plain(e.theme.clone(), Color::Rgb(230, 215, 170)));
            let tl = chronicle::tally(ow, clashes, e.from, y);
            let line = tl.line();
            if !line.is_empty() {
                v.push(plain(line, MUTED));
            }
            // The great events of the age so far, and its great deeds.
            v.push(Line::default());
            v.push(header("Great deeds"));
            for p in ow.epics.iter().filter(|p| p.year >= e.from && p.year <= y) {
                v.push(Line::from(vec![
                    Span::styled(format!("{:>4} ★ ", p.year), Style::default().fg(epic_color(p.kind))),
                    Span::styled(p.name.clone(), Style::default().fg(epic_color(p.kind)).add_modifier(Modifier::BOLD)),
                ]));
            }
            let (great, lesser) = chronicle::deeds(ow, e.from, e.to, y);
            // As many of the latest as fit (about two lines each).
            let fit = (cols[1].height as usize).saturating_sub(v.len() + 5) / 2;
            let skip = great.len().saturating_sub(fit.max(1));
            for (yr, text) in &great[skip..] {
                v.push(Line::from(vec![
                    Span::styled(format!("{:>4} ", yr), Style::default().fg(MUTED)),
                    Span::styled(text.clone(), Style::default().fg(Color::Rgb(190, 190, 200))),
                ]));
            }
            if lesser + skip > 0 {
                v.push(plain(format!("     …and {} lesser deeds", lesser + skip), MUTED));
            }
        }
    }
    let keys = if finished { " Enter accept · r another world · Esc settings " } else { " +/- speed · Tab next · Space skip · Esc " };
    f.render_widget(
        Paragraph::new(v)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(MUTED))
                    .title(Span::styled(if finished { " The world is ready " } else { " Generating " }, Style::default().fg(KEY)))
                    .title_bottom(Span::styled(keys, Style::default().fg(KEY))),
            ),
        cols[1],
    );
}
