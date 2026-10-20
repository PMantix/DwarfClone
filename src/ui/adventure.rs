//! Adventurer mode's screens (G03; `app::adventure` has the keys). W0: the
//! local map centred on the hero, a sidebar and the visit's log. A2 builds
//! the full local view (sight, look, popups, the 80-column fold), and each
//! other view gets its own file (`adv_create`, `adv_travel`, `adv_talk`,
//! `adv_end`).

use super::looks::tile_look;
use super::{backdrop, dim, plain, rgb, BLACK, KEY, MUTED, TEXT};
use crate::app::adventure::AdvScreen;
use crate::app::App;
use crate::game::adv::{Adventure, View};
use crate::game::clock;
use crate::geom::Pos;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

/// The sidebar's width, as the fort's.
const SIDE_W: u16 = 38;
/// Rows of the log under the map.
const LOG_ROWS: u16 = 6;

pub(super) fn draw(f: &mut Frame, app: &App) {
    let area = f.size();
    let Some(s) = app.adv.as_deref() else {
        backdrop(f, area);
        return;
    };
    if s.adv.visit.is_none() {
        backdrop(f, area);
        let p = Paragraph::new(vec![Line::default(), plain("The adventure begins…".into(), KEY)]).alignment(ratatui::layout::Alignment::Center);
        f.render_widget(p, area);
        return;
    }
    let side = if area.width >= 80 + SIDE_W { SIDE_W } else { 0 };
    let log_h = LOG_ROWS.min(area.height / 3);
    let map = Rect::new(area.x, area.y, area.width - side, area.height - log_h);
    let log = Rect::new(area.x, area.y + map.height, area.width - side, log_h);
    draw_local(f, map, &s.adv);
    draw_log(f, log, s);
    if side > 0 {
        draw_side(f, Rect::new(area.x + area.width - side, area.y, side, area.height), s);
    }
}

/// The site's map around the hero, on the hero's level.
fn draw_local(f: &mut Frame, r: Rect, a: &Adventure) {
    let Some(v) = a.visit.as_ref() else { return };
    let g = &v.g;
    let hero = &g.dwarves[v.hero];
    let me = hero.pos;
    let w = &g.world;
    let (vw, vh) = (r.width as i32, r.height as i32);
    let ox = if vw >= w.w { -(vw - w.w) / 2 } else { (me.x - vw / 2).clamp(0, w.w - vw) };
    let oy = if vh >= w.h { -(vh - w.h) / 2 } else { (me.y - vh / 2).clamp(0, w.h - vh) };
    let z = me.z;
    let night = clock::light(g.tick, true) < 128 && w.outside(me);
    let sight = g.sight_radius(v.hero);
    let mut units: std::collections::HashMap<(i32, i32), (char, Color)> = Default::default();
    for it in g.items.values().filter(|it| it.carried_by.is_none() && it.container.is_none() && it.pos.z == z) {
        units.entry((it.pos.x, it.pos.y)).or_insert_with(|| it.glyph());
    }
    for c in g.creatures.values().filter(|c| c.alive && !c.sneaking && c.pos.z == z) {
        let info = c.species.info();
        units.insert((c.pos.x, c.pos.y), (info.glyph, rgb(info.color)));
    }
    for d in g.dwarves.iter().filter(|d| d.alive && d.pos.z == z) {
        units.insert((d.pos.x, d.pos.y), ('☺', rgb((200, 200, 200))));
    }
    let race = a.hero.race.color();
    let buf = f.buffer_mut();
    for sy in 0..vh {
        for sx in 0..vw {
            let (x, y) = (ox + sx, oy + sy);
            let cell = buf.get_mut(r.x + sx as u16, r.y + sy as u16);
            let p = Pos::new(x, y, z);
            if !w.in_bounds(p) {
                cell.set_char(' ').set_bg(BLACK);
                continue;
            }
            let (mut ch, mut fg, mut bg) = tile_look(w, p);
            if let Some(&(c, col)) = units.get(&(x, y)) {
                ch = c;
                fg = col;
            }
            if p == me {
                ch = '☺';
                fg = if hero.alive { race } else { Color::Rgb(120, 30, 30) };
            }
            // Out of sight under the night sky: dim.
            let far = (x - me.x).pow(2) + (y - me.y).pow(2) > sight * sight;
            if night && far {
                fg = dim(fg, 0.35);
                bg = dim(bg, 0.35);
            }
            cell.set_char(ch).set_fg(fg).set_bg(bg);
        }
    }
}

/// The hero, their body, what they hold, and the time.
fn draw_side(f: &mut Frame, r: Rect, s: &AdvScreen) {
    let a = &s.adv;
    let d = a.hero_dwarf();
    let mut v: Vec<Line> = Vec::new();
    v.push(plain(d.called(), KEY));
    v.push(plain(format!("A {} {:?}", race_word(a), a.hero.calling).to_lowercase(), MUTED));
    v.push(Line::default());
    if let Some(vis) = a.visit.as_ref() {
        v.push(plain(vis.g.fortress_name.clone(), TEXT));
        v.push(plain(vis.g.date_string(), MUTED));
    }
    let now = a.now();
    v.push(plain(format!("{}, {}", cap(clock::time_words(now)), clock::moon_words(now)), MUTED));
    v.push(Line::default());
    let blood = d.body.blood * 100 / d.body.max_blood.max(1);
    let bc = if blood > 70 { Color::Rgb(120, 200, 120) } else if blood > 40 { Color::Rgb(230, 200, 80) } else { Color::Rgb(230, 70, 60) };
    v.push(Line::from(vec![Span::styled("Blood ", Style::default().fg(MUTED)), Span::styled(format!("{}%", blood), Style::default().fg(bc))]));
    if d.body.bleeding > 0 {
        v.push(plain("Bleeding".into(), Color::Rgb(230, 60, 60)));
    }
    if !d.alive {
        v.push(plain("Dead".into(), Color::Rgb(230, 60, 60)));
    }
    if let Some(vis) = a.visit.as_ref() {
        if let Some(it) = d.held.and_then(|id| vis.g.items.get(&id)) {
            v.push(plain(format!("Holds {}", it.name()), TEXT));
        }
    }
    v.push(Line::default());
    v.push(plain("arrows/numpad move · . wait".into(), MUTED));
    v.push(plain("Esc title".into(), MUTED));
    let p = Paragraph::new(v).wrap(Wrap { trim: true }).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(MUTED)));
    f.render_widget(p, r);
}

/// The visit's newest announcements, the flash first.
fn draw_log(f: &mut Frame, r: Rect, s: &AdvScreen) {
    let mut v: Vec<Line> = Vec::new();
    if let Some(fl) = &s.flash {
        v.push(plain(fl.clone(), Color::Rgb(240, 150, 110)));
    }
    if s.view == View::Dead {
        v.push(plain("You are dead. Esc returns to the title.".into(), Color::Rgb(230, 60, 60)));
    }
    let log = s.adv.visit.as_ref().map_or(&s.adv.log, |v| &v.g.log);
    let room = (r.height as usize).saturating_sub(v.len());
    let start = log.len().saturating_sub(room);
    for e in &log[start..] {
        v.push(plain(e.text.clone(), rgb(e.color)));
    }
    f.render_widget(Paragraph::new(v), r);
}

fn race_word(a: &Adventure) -> &'static str {
    match a.hero.race {
        crate::overworld::Race::Dwarf => "dwarf",
        crate::overworld::Race::Human => "human",
        crate::overworld::Race::Elf => "elf",
        crate::overworld::Race::Goblin => "goblin",
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}
