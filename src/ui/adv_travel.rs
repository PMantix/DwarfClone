//! Adventurer mode's travel view (G03 A3; `app::adventure::key_travel`
//! has the keys): the world map round the hero, the road they walked
//! traced over the land in gold and the places near them named, dimmed by
//! night; a sidebar with the date, the hour and the sky, the land underfoot
//! and how dangerous it is, and the hero's body; and under the map the
//! travel log, the journey told as it went.

use super::adventure::{body_block, draw_menu};
use super::{backdrop, header, key_line, plain, popup, rgb, wrap_words, BLACK, KEY, MUTED, TEXT};
use crate::app::adventure::AdvScreen;
use crate::game::adv::travel::{cheb, land_name};
use crate::game::adv::{AdvEnd, Adventure, View};
use crate::game::{clock, Weather, DAY, MONTH, MONTHS, YEAR};
use crate::overworld::{Overworld, SiteKind};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use std::collections::HashSet;

/// The sidebar's width.
const SIDE_W: u16 = 40;
/// Rows of the travel log under the map (with its border).
const LOG_ROWS: u16 = 10;
/// How far from the hero places are named on the map.
const LABEL_R: i32 = 14;
/// The most of the road drawn.
const ROAD_MAX: usize = 400;
const GOLD: Color = Color::Rgb(235, 190, 80);
const RED: Color = Color::Rgb(235, 70, 60);

pub(super) fn draw(f: &mut Frame, area: Rect, s: &AdvScreen) {
    backdrop(f, area);
    let a = &s.adv;
    if a.ow().w <= 1 {
        return;
    }
    let side = if area.width >= 80 + SIDE_W { SIDE_W } else { 0 };
    let fold: u16 = if side == 0 { 2 } else { 0 };
    let log_h = LOG_ROWS.min(area.height / 3).max(3);
    let left_w = area.width - side;
    let map_h = area.height.saturating_sub(log_h + fold);
    draw_map(f, Rect::new(area.x, area.y, left_w, map_h), a);
    draw_log(f, Rect::new(area.x, area.y + map_h + fold, left_w, log_h), s);
    if side > 0 {
        draw_side(f, Rect::new(area.x + left_w, area.y, side, area.height), s);
    } else {
        draw_strip(f, Rect::new(area.x, area.y + map_h, left_w, fold), s);
    }
    if let Some(e) = &s.meet {
        let w = 64.min(area.width.saturating_sub(4)).max(20);
        let mut v: Vec<Line<'static>> = wrap_words(&e.text, w as usize - 4).into_iter().map(|l| plain(format!(" {}", l), Color::Rgb(255, 150, 110))).collect();
        v.push(Line::default());
        v.push(key_line("Enter", "stand and fight", false));
        v.push(key_line("Esc", if s.sneak { "try to slip away (keeping to cover helps)" } else { "try to slip away" }, false));
        let h = v.len() as u16 + 2;
        popup(f, area, w, h, if e.ambush { " Ambush! " } else { " On the road " }, v);
    }
    match &s.view {
        View::Menu { sel } => draw_menu(f, area, *sel),
        View::Dead => draw_end(f, area, a),
        _ => {}
    }
    if s.help {
        draw_help(f, area);
    }
}

/// The world round the hero, one character a cell: the land as the world
/// map draws it, darker by night; the road walked in gold; places near by
/// named; the hero bright in the middle.
fn draw_map(f: &mut Frame, area: Rect, a: &Adventure) {
    let ow = a.ow();
    let now = a.now();
    let title = format!(" {} · {} ", cap(&land_name(ow, a.at.0, a.at.1)), cap(clock::time_words(now)));
    let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(MUTED)).title(Span::styled(title, Style::default().fg(KEY)));
    let r = block.inner(area);
    f.render_widget(block, area);
    let (vw, vh) = (r.width as i32, r.height as i32);
    if vw <= 0 || vh <= 0 {
        return;
    }
    let (hx, hy) = a.at;
    let x0 = if vw >= ow.w { -(vw - ow.w) / 2 } else { (hx - vw / 2).clamp(0, ow.w - vw) };
    let y0 = if vh >= ow.h { -(vh - ow.h) / 2 } else { (hy - vh / 2).clamp(0, ow.h - vh) };
    // Night darkens the land (the hero and the names stay bright).
    let light = 0.45 + 0.55 * clock::light(now, true) as f32 / 255.0;
    let road: Vec<(i32, i32)> = a.road();
    let road: HashSet<(i32, i32)> = road.iter().rev().take(ROAD_MAX).copied().collect();
    let screen = |x: i32, y: i32| -> Option<(u16, u16)> {
        let (sx, sy) = (x - x0, y - y0);
        (sx >= 0 && sy >= 0 && sx < vw && sy < vh).then(|| ((r.x as i32 + sx) as u16, (r.y as i32 + sy) as u16))
    };
    let buf = f.buffer_mut();
    for sy in 0..vh {
        for sx in 0..vw {
            let (x, y) = (x0 + sx, y0 + sy);
            if x < 0 || y < 0 || x >= ow.w || y >= ow.h {
                continue;
            }
            let (mut ch, fg, bg) = ow.map_look(x, y);
            let mut fg = shade(fg, light);
            let mut bg = shade(bg, light);
            if road.contains(&(x, y)) {
                bg = Color::Rgb(70, 52, 12);
                if ow.cell(x, y).site == 0 && ow.fort_at(x, y).is_none() {
                    ch = '·';
                    fg = GOLD;
                }
            }
            buf.get_mut((r.x as i32 + sx) as u16, (r.y as i32 + sy) as u16).set_char(ch).set_fg(fg).set_bg(bg);
        }
    }
    // The places near the hero, named beside their marks, nearest first.
    let mut taken: HashSet<(u16, u16)> = HashSet::new();
    if let Some(p) = screen(hx, hy) {
        taken.insert(p);
        taken.insert((p.0 + 1, p.1));
        taken.insert((p.0.saturating_sub(1), p.1));
    }
    for (x, y, name, col) in places(ow, a.at) {
        let Some((px, py)) = screen(x, y) else { continue };
        let text = format!(" {} ", name);
        let n = text.chars().count() as u16;
        let right = px + 1;
        let left = px.saturating_sub(n);
        let fits = |from: u16| from >= r.x && from + n <= r.x + r.width && (from..from + n).all(|c| !taken.contains(&(c, py)));
        let from = if fits(right) {
            right
        } else if fits(left) {
            left
        } else {
            continue;
        };
        for c in from.saturating_sub(1)..from + n + 1 {
            taken.insert((c, py));
        }
        let buf = f.buffer_mut();
        for (i, ch) in text.chars().enumerate() {
            buf.get_mut(from + i as u16, py).set_char(ch).set_fg(col).set_bg(Color::Rgb(16, 16, 22));
        }
        taken.insert((px, py));
    }
    // The hero, blinking between gold and white.
    if let Some((px, py)) = screen(hx, hy) {
        let blink = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() / 600) % 2 == 0;
        let (fg, bg) = if blink { (BLACK, Color::Rgb(255, 215, 60)) } else { (BLACK, Color::Rgb(255, 255, 255)) };
        f.buffer_mut().get_mut(px, py).set_char('@').set_fg(fg).set_bg(bg).set_style(Style::default().add_modifier(Modifier::BOLD));
    }
}

/// The places to name on the map near `at`: fallen forts, sites and the
/// lairs of living beasts, nearest first, each with its colour.
fn places(ow: &Overworld, at: (i32, i32)) -> Vec<(i32, i32, String, Color)> {
    let mut v: Vec<(i32, i32, i32, String, Color)> = Vec::new();
    for f in ow.forts.iter() {
        if cheb(f.site, at) <= LABEL_R {
            v.push((cheb(f.site, at), f.site.0, f.site.1, f.name.clone(), Color::Rgb(230, 70, 80)));
        }
    }
    for (i, s) in ow.sites.iter().enumerate() {
        let d = cheb((s.x, s.y), at);
        if d > LABEL_R || ow.fort_at(s.x, s.y).is_some() {
            continue;
        }
        let name = if s.kind == SiteKind::Lair {
            match ow.figures.iter().find(|f| f.lair == Some(i as u32) && f.alive()) {
                Some(f) => format!("{}'s lair", first_word(&f.name)),
                None => continue,
            }
        } else if s.ruined {
            format!("{} (ruin)", s.name)
        } else {
            s.name.clone()
        };
        let col = match ow.site_look(s).1 {
            Color::Rgb(r, g, b) => Color::Rgb(r.max(120), g.max(120), b.max(120)),
            c => c,
        };
        v.push((d, s.x, s.y, name, col));
    }
    v.sort_by_key(|p| p.0);
    v.into_iter().take(12).map(|(_, x, y, n, c)| (x, y, n, c)).collect()
}

fn first_word(s: &str) -> &str {
    s.split(' ').next().unwrap_or(s)
}

/// The date: "3rd Midsun, year 205 · day 12 on the road".
fn date_words(a: &Adventure, tick: u64) -> String {
    let month = ((tick % YEAR) / MONTH) as usize;
    let day = (tick % MONTH) / DAY + 1;
    format!("{} {}, year {}", ordinal(day), MONTHS[month], a.start_year as u64 + tick / YEAR)
}

fn ordinal(d: u64) -> String {
    let suf = match (d % 10, d % 100) {
        (1, n) if n != 11 => "st",
        (2, n) if n != 12 => "nd",
        (3, n) if n != 13 => "rd",
        _ => "th",
    };
    format!("{}{}", d, suf)
}

/// The sky in a few words and its colour.
fn sky(w: Weather, night: bool) -> (&'static str, Color) {
    match w {
        Weather::Clear if night => ("Clear and cold", Color::Rgb(160, 170, 220)),
        Weather::Clear => ("Fair", Color::Rgb(250, 230, 140)),
        Weather::Rain => ("Rain", Color::Rgb(110, 160, 230)),
        Weather::Snow => ("Snow", Color::Rgb(220, 230, 250)),
        Weather::Storm => ("Thunderstorm", Color::Rgb(170, 150, 240)),
        Weather::Blizzard => ("Blizzard", Color::Rgb(235, 240, 255)),
        Weather::BloodRain => ("Blood rain", Color::Rgb(220, 40, 50)),
    }
}

/// A crossing time in days, as said: "half a day", "a day and a half".
fn days_words(d: f64) -> String {
    let q = (d * 4.0).round() as i32;
    match q {
        0..=1 => "a few hours".into(),
        2 => "half a day".into(),
        3 => "three quarters of a day".into(),
        4 => "a day".into(),
        5 => "a day and a bit".into(),
        6 => "a day and a half".into(),
        7 => "most of two days".into(),
        8 => "two days".into(),
        n => format!("{} days", (n + 2) / 4),
    }
}

/// How dangerous the land is, by the chance of meeting something.
fn danger(p: f64) -> (&'static str, Color) {
    if p < 0.07 {
        ("quiet", Color::Rgb(120, 200, 120))
    } else if p < 0.12 {
        ("watchful", Color::Rgb(220, 210, 110))
    } else if p < 0.2 {
        ("dangerous", Color::Rgb(240, 150, 80))
    } else {
        ("deadly", RED)
    }
}

/// The date and the sky, the land underfoot, the hero, the road, the keys.
fn draw_side(f: &mut Frame, r: Rect, s: &AdvScreen) {
    let a = &s.adv;
    let ow = a.ow();
    let now = a.now();
    let night = clock::is_night(now);
    let d = a.hero_dwarf();
    let mut v: Vec<Line<'static>> = Vec::new();
    v.push(Line::from(Span::styled(d.called(), Style::default().fg(KEY).add_modifier(Modifier::BOLD))));
    v.push(plain(format!("{} {}", cap(crate::game::adv::sitegen::lore::race_adj(a.hero.race)), if s.sneak { "· keeping to cover" } else { "· on the road" }), MUTED));
    v.push(plain(date_words(a, now), TEXT));
    let mut when = cap(clock::time_words(now));
    if night {
        when = format!("{}, {}", when, clock::moon_words(now));
    }
    v.push(plain(when, if night { Color::Rgb(150, 160, 210) } else { Color::Rgb(250, 220, 150) }));
    let (w, wc) = sky(a.weather, night);
    v.push(Line::from(vec![Span::styled("Sky: ", Style::default().fg(MUTED)), Span::styled(w, Style::default().fg(wc))]));
    v.push(Line::default());
    v.push(header("Here"));
    let (x, y) = a.at;
    let c = ow.cell(x, y);
    let (g, gc) = c.biome.glyph();
    v.push(Line::from(vec![Span::styled(format!("{} ", g), Style::default().fg(gc)), Span::styled(cap(&land_name(ow, x, y)), Style::default().fg(TEXT))]));
    if let Some(rv) = ow.river_at(x, y) {
        v.push(plain(format!("{} runs here", rv.name), Color::Rgb(90, 200, 240)));
    }
    if c.evil {
        v.push(plain("An evil taint lies on the land.".into(), Color::Rgb(200, 120, 255)));
    }
    if let Some(i) = ow.fort_at(x, y) {
        v.push(plain(format!("The ruin of {} (Enter)", ow.forts[i].name), Color::Rgb(230, 70, 80)));
    } else if let Some(site) = ow.site_at(x, y) {
        let state = if site.ruined { "ruined " } else { "" };
        v.push(plain(format!("{}, a {}{} (Enter)", site.name, state, site.kind.name()), Color::Rgb(255, 230, 120)));
    } else {
        v.push(plain("Enter: walk into the land here".into(), MUTED));
    }
    if let Some(days) = a.crossing_days(x, y) {
        v.push(plain(format!("Crossing it takes {}", days_words(days * a.pace(s.sneak))), MUTED));
    }
    let (dw, dc) = danger(a.encounter_chance(a.at));
    v.push(Line::from(vec![Span::styled("The land is ", Style::default().fg(MUTED)), Span::styled(dw, Style::default().fg(dc))]));
    v.push(Line::default());
    v.push(header("Body"));
    v.extend(body_block(d, 10));
    if !a.company.is_empty() {
        v.push(Line::default());
        v.push(header("Company"));
        for c in &a.company {
            v.push(plain(c.dwarf.name.clone(), if c.dwarf.alive { TEXT } else { MUTED }));
        }
    }
    v.push(Line::default());
    let days = now.saturating_sub(clock::DAWN_TICK) / DAY;
    v.push(plain(format!("{} days out, {} cells walked", days, a.stats.cells), MUTED));
    if let Some(fl) = &s.flash {
        v.push(Line::default());
        for l in wrap_words(fl, r.width.saturating_sub(3) as usize) {
            v.push(plain(l, Color::Rgb(255, 150, 90)));
        }
    }
    let inner_h = r.height.saturating_sub(2) as usize;
    let keys = vec![
        plain("move: arrows · numpad · 7 9 1 3".into(), MUTED),
        plain("Enter go in  Z camp  s cover".into(), MUTED),
        plain("? keys  Esc menu".into(), MUTED),
    ];
    if v.len() + keys.len() < inner_h {
        v.resize(inner_h - keys.len(), Line::default());
    }
    v.extend(keys);
    let p = Paragraph::new(v).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(MUTED)));
    f.render_widget(p, r);
}

/// The sidebar folded into two lines (narrow screens).
fn draw_strip(f: &mut Frame, r: Rect, s: &AdvScreen) {
    if r.height == 0 {
        return;
    }
    let a = &s.adv;
    let now = a.now();
    let (w, wc) = sky(a.weather, clock::is_night(now));
    let l1 = Line::from(vec![
        Span::styled(a.hero_dwarf().called(), Style::default().fg(KEY).add_modifier(Modifier::BOLD)),
        Span::styled(format!("  {} · {} · ", date_words(a, now), cap(clock::time_words(now))), Style::default().fg(MUTED)),
        Span::styled(w, Style::default().fg(wc)),
    ]);
    let msg = s.flash.clone().unwrap_or_else(|| "Enter go in · Z camp · s cover · ? keys".into());
    f.render_widget(Paragraph::new(vec![l1, plain(msg, MUTED)]), r);
}

/// The travel log, newest last: each stretch of road with its day and
/// hour, and what was seen and felt on it.
fn draw_log(f: &mut Frame, r: Rect, s: &AdvScreen) {
    let block = Block::default().borders(Borders::TOP).border_style(Style::default().fg(MUTED)).title(Span::styled(" The road so far ", Style::default().fg(MUTED)));
    let inner = block.inner(r);
    f.render_widget(block, r);
    let rows = inner.height as usize;
    if rows == 0 {
        return;
    }
    let stamp_w = 16usize;
    let text_w = (inner.width as usize).saturating_sub(stamp_w + 1).max(10);
    let mut lines: Vec<Line<'static>> = Vec::new();
    for e in s.adv.log.iter().rev() {
        let stamp = if e.at.is_some() || lines.is_empty() { format!("Day {}, {}", e.tick.saturating_sub(clock::DAWN_TICK) / DAY + 1, short_time(e.tick)) } else { String::new() };
        let wrapped = wrap_words(&e.text, text_w);
        let mut block: Vec<Line<'static>> = Vec::new();
        for (i, l) in wrapped.into_iter().enumerate() {
            let st = if i == 0 { format!("{:<w$}", stamp, w = stamp_w) } else { " ".repeat(stamp_w) };
            block.push(Line::from(vec![Span::styled(st, Style::default().fg(MUTED)), Span::styled(format!(" {}", l), Style::default().fg(rgb(e.color)))]));
        }
        for l in block.into_iter().rev() {
            lines.push(l);
        }
        if lines.len() >= rows {
            break;
        }
    }
    lines.truncate(rows);
    lines.reverse();
    f.render_widget(Paragraph::new(lines), inner);
}

fn short_time(tick: u64) -> &'static str {
    match clock::hour(tick) {
        0..=4 => "night",
        5..=6 => "dawn",
        7..=10 => "morning",
        11..=13 => "midday",
        14..=17 => "afternoon",
        18..=19 => "dusk",
        _ => "night",
    }
}

/// The end on the road.
fn draw_end(f: &mut Frame, area: Rect, a: &Adventure) {
    let d = a.hero_dwarf();
    let cause = match &a.over {
        Some(AdvEnd::Died { cause, .. }) => cause.clone(),
        _ => "is gone".to_string(),
    };
    let mut v: Vec<Line<'static>> = Vec::new();
    for l in wrap_words(&format!("{} {}.", d.name, cause), 52) {
        v.push(plain(format!(" {}", l), RED));
    }
    v.push(plain(format!(" {}", date_words(a, a.now())), MUTED));
    v.push(Line::default());
    v.push(plain(" Esc: to the title".into(), MUTED));
    let h = v.len() as u16 + 2;
    popup(f, area, 58, h, " The end of the road ", v);
}

/// The travel view's keys.
fn draw_help(f: &mut Frame, area: Rect) {
    let rows = [
        ("1-9", "walk a cell (numpad; 7 9 1 3 and Home/End/PgUp/PgDn diagonal)"),
        ("arrows", "walk a cell"),
        ("Enter >", "go into the place here, or the wild land"),
        ("Z", "make camp until dawn (heals; the night may bring foes)"),
        ("s", "keep to cover: slower, fewer encounters, easier to slip away"),
        ("Esc", "menu"),
        ("", ""),
        ("T", "(in a place) take the road, from its edge or the open wild"),
    ];
    let mut v: Vec<Line<'static>> = rows.iter().map(|(k, d)| key_line(k, d, false)).collect();
    v.push(Line::default());
    v.push(plain("  any key: back".into(), MUTED));
    let h = v.len() as u16 + 2;
    popup(f, area, 76, h, " The road ", v);
}

/// A colour darkened for the hour.
fn shade(c: Color, f: f32) -> Color {
    match c {
        Color::Rgb(r, g, b) => Color::Rgb((r as f32 * f) as u8, (g as f32 * f) as u8, (b as f32 * f) as u8),
        other => other,
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}
