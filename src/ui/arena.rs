//! The arena's screens: the setup, the bout on its little map with the
//! fighters' tallies and the combat log, the summary at the end, the full
//! report, the rerun results and the load list (`app::arena` has the keys).

use super::{backdrop, dim, header, key_line, plain, popup, rgb, BLACK, KEY, MUTED, TEXT};
use crate::app::arena::{load_list, ArenaScreen, View, RERUNS};
use crate::app::{App, SPEEDS};
use crate::game::arena::{self, Battle, Kin, Tally, SETUP_ROWS, SIDE_COLORS};
use crate::geom::Pos;
use crate::world::{TileKind, DOOR_NONE};
use crate::material::Mat;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

const SIDES: [&str; 2] = ["West", "East"];

pub(super) fn draw(f: &mut Frame, app: &App) {
    let area = f.size();
    let Some(a) = app.arena.as_deref() else { return };
    match &a.view {
        View::Setup => draw_setup(f, area, a),
        View::Battle => {
            if let Some(b) = &a.battle {
                draw_battle(f, area, a, b);
                if b.outcome.is_some() && a.ended >= 12 {
                    draw_summary(f, area, a, b);
                }
            }
        }
        View::Report { scroll } => {
            if let Some(b) = &a.battle {
                draw_battle(f, area, a, b);
                draw_report(f, area, b, *scroll);
            }
        }
        View::Results => draw_results(f, area, a),
        View::Load { sel } => draw_load(f, area, &app.save_dir, *sel),
    }
}

fn side_color(s: usize) -> Color {
    rgb(SIDE_COLORS[s.min(1)])
}

fn boxed<'a>(lines: Vec<Line<'a>>, title: &str) -> Paragraph<'a> {
    Paragraph::new(lines).style(Style::default().bg(Color::Rgb(12, 12, 20))).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(160, 130, 70)))
            .title(Span::styled(title.to_string(), Style::default().fg(KEY).add_modifier(Modifier::BOLD))),
    )
}

fn cut(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

// ---------------------------------------------------------------- setup

fn draw_setup(f: &mut Frame, area: Rect, a: &ArenaScreen) {
    backdrop(f, area);
    let m = &a.matchup;
    let w = area.width.min(100);
    let inner = w.saturating_sub(2) as usize;
    let col = (inner.saturating_sub(15) / 2).max(8);
    let mut v: Vec<Line> = vec![
        Line::from(Span::styled(cut(&m.name, inner), Style::default().fg(KEY).add_modifier(Modifier::BOLD))),
    ];
    for l in wrap(&m.describe(), inner).into_iter().take(2) {
        v.push(plain(l, MUTED));
    }
    v.push(Line::from(vec![
            Span::raw(format!("{:<15}", "")),
            Span::styled(format!("{:<w$}", SIDES[0], w = col), Style::default().fg(side_color(0)).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:<w$}", SIDES[1], w = col), Style::default().fg(side_color(1)).add_modifier(Modifier::BOLD)),
        ]));
    for row in 0..arena::ROW_FIELD {
        let mut spans = vec![Span::styled(format!(" {:<14}", m.row(0, row).0), Style::default().fg(TEXT))];
        for side in 0..2 {
            let (label, value, used) = m.row(side, row);
            let value = if used { value } else { "—".to_string() };
            let shown = if side == 1 && label != m.row(0, row).0 { format!("{}: {}", label.trim(), value) } else { value };
            let st = if a.row == row && a.side == side {
                Style::default().fg(BLACK).bg(KEY)
            } else if used {
                Style::default().fg(TEXT)
            } else {
                Style::default().fg(Color::Rgb(80, 80, 90))
            };
            spans.push(Span::styled(format!("{:<w$}", cut(&shown, col - 1), w = col - 1), st));
            spans.push(Span::raw(" "));
        }
        v.push(Line::from(spans));
    }
    let fst = if a.row == arena::ROW_FIELD { Style::default().fg(BLACK).bg(KEY) } else { Style::default().fg(TEXT) };
    v.push(Line::from(vec![Span::styled(format!(" {:<14}", "Field"), Style::default().fg(TEXT)), Span::styled(format!("◄ {} ►", m.field.name()), fst)]));
    let pick = match a.pick {
        Some(s) => Span::styled(format!("{} ({})", SIDES[s].to_lowercase(), a.matchup.sides[s].kin.plural()), Style::default().fg(side_color(s))),
        None => Span::styled("nobody", Style::default().fg(MUTED)),
    };
    let record = if a.record.1 > 0 { format!("  right {} of {}", a.record.0, a.record.1) } else { String::new() };
    v.push(Line::from(vec![Span::styled(format!(" {:<14}", "Your pick"), Style::default().fg(TEXT)), pick, Span::styled(record, Style::default().fg(MUTED))]));
    v.push(Line::default());
    v.push(Line::from(vec![
        Span::styled(" ↑↓←→", Style::default().fg(KEY).add_modifier(Modifier::BOLD)),
        Span::styled(" choose ", Style::default().fg(TEXT)),
        Span::styled("Tab", Style::default().fg(KEY).add_modifier(Modifier::BOLD)),
        Span::styled(" side  ", Style::default().fg(TEXT)),
        Span::styled("Enter", Style::default().fg(KEY).add_modifier(Modifier::BOLD)),
        Span::styled(" fight!  ", Style::default().fg(TEXT)),
        Span::styled("r", Style::default().fg(KEY).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" rerun ×{}  ", RERUNS[a.reruns]), Style::default().fg(TEXT)),
        Span::styled("n", Style::default().fg(KEY).add_modifier(Modifier::BOLD)),
        Span::styled(" how many", Style::default().fg(TEXT)),
    ]));
    v.push(Line::from(vec![
        Span::styled(" b", Style::default().fg(KEY).add_modifier(Modifier::BOLD)),
        Span::styled(" pick a winner  ", Style::default().fg(TEXT)),
        Span::styled("p", Style::default().fg(KEY).add_modifier(Modifier::BOLD)),
        Span::styled(" preset  ", Style::default().fg(TEXT)),
        Span::styled("?", Style::default().fg(KEY).add_modifier(Modifier::BOLD)),
        Span::styled(" surprise me  ", Style::default().fg(TEXT)),
        Span::styled("s", Style::default().fg(KEY).add_modifier(Modifier::BOLD)),
        Span::styled("/", Style::default().fg(TEXT)),
        Span::styled("l", Style::default().fg(KEY).add_modifier(Modifier::BOLD)),
        Span::styled(" save/load  ", Style::default().fg(TEXT)),
        Span::styled("Esc", Style::default().fg(KEY).add_modifier(Modifier::BOLD)),
        Span::styled(" title", Style::default().fg(TEXT)),
    ]));
    if let Some(fl) = &a.flash {
        v.push(plain(fl.clone(), Color::Rgb(240, 150, 110)));
    }
    let h = (v.len() as u16 + 2).min(area.height);
    let r = Rect::new(area.x + (area.width - w) / 2, area.y + area.height.saturating_sub(h) / 2, w, h);
    f.render_widget(Clear, r);
    f.render_widget(boxed(v, " ⚔ The Arena "), r);
    let _ = SETUP_ROWS;
}

// ---------------------------------------------------------------- the bout

fn draw_battle(f: &mut Frame, area: Rect, a: &ArenaScreen, b: &Battle) {
    let log_h = if area.height >= 40 { 10 } else { 6 };
    let rows = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(1), Constraint::Min(6), Constraint::Length(log_h)]).split(area);
    let panel_w = if area.width >= 120 { 40 } else { 28 };
    let mid = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Min(20), Constraint::Length(panel_w)]).split(rows[1]);
    // The status line.
    let state = match (&b.outcome, a.paused) {
        (Some(_), _) => "over".to_string(),
        (None, true) => "❚❚ paused".to_string(),
        (None, false) => format!("▶ ×{}", SPEEDS[a.speed]),
    };
    let status = format!(" {} · bout {} · tick {} · {} ", b.name, b.seed, b.g.tick, state);
    let keys = " Space pause  +/- speed  Tab follow  f free  v log  e end  Esc setup";
    let line = Line::from(vec![
        Span::styled(status.clone(), Style::default().fg(BLACK).bg(KEY)),
        Span::styled(cut(keys, (area.width as usize).saturating_sub(status.chars().count())), Style::default().fg(MUTED)),
    ]);
    f.render_widget(Paragraph::new(line).style(Style::default().bg(Color::Rgb(10, 10, 16))), rows[0]);
    draw_field(f, mid[0], a, b);
    f.render_widget(boxed(panel_lines(a, b, panel_w as usize - 2), " The fighters ").wrap(Wrap { trim: false }), mid[1]);
    // The combat log, newest at the bottom: every blow, misses counted.
    let n = rows[2].height.saturating_sub(2) as usize;
    let all = bout_lines(b);
    let skip = all.len().saturating_sub(n);
    let lines: Vec<Line> = all[skip..].iter().map(|(_, t, c)| plain(t.clone(), *c)).collect();
    f.render_widget(boxed(lines, " Combat log (v: all of it) "), rows[2]);
}

/// Where the camera looks: the fighter followed, else the last telling
/// blow, else the middle of the field.
fn camera(a: &ArenaScreen, b: &Battle) -> Pos {
    a.follow.map(|i| b.pos(i)).or(b.hot).unwrap_or(Pos::new(b.g.world.w / 2, b.g.world.h / 2, 1))
}

fn draw_field(f: &mut Frame, area: Rect, a: &ArenaScreen, b: &Battle) {
    let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(MUTED)).title(Span::styled(format!(" {} ", b.field.name()), Style::default().fg(KEY)));
    let inner = block.inner(area);
    f.render_widget(block, area);
    let w = &b.g.world;
    let (vw, vh) = (inner.width as i32, inner.height as i32);
    let c = camera(a, b);
    let ox = if vw >= w.w { -(vw - w.w) / 2 } else { (c.x - vw / 2).clamp(0, w.w - vw) };
    let oy = if vh >= w.h { -(vh - w.h) / 2 } else { (c.y - vh / 2).clamp(0, w.h - vh) };
    let z = 1;
    let mut units: std::collections::HashMap<(i32, i32), usize> = Default::default();
    // The standing over the fallen, the followed over everyone.
    for i in 0..b.warriors.len() {
        if !b.alive(i) {
            continue;
        }
        let p = b.pos(i);
        let k = (p.x, p.y);
        let better = units.get(&k).map_or(true, |&j| (Some(i) == a.follow) || (!b.standing(j) && b.standing(i)));
        if better {
            units.insert(k, i);
        }
    }
    let mut items: std::collections::HashMap<(i32, i32), (char, Color)> = Default::default();
    let flying = b.g.flying_items();
    for it in b.g.items.values().filter(|it| it.carried_by.is_none() && it.pos.z == z && !flying.contains(&it.id)) {
        items.entry((it.pos.x, it.pos.y)).or_insert_with(|| it.glyph());
    }
    // Blows flashing, blood and severed parts flying (gore.rs).
    let fx: std::collections::HashMap<(i32, i32), (char, (u8, u8, u8), Option<(u8, u8, u8)>)> = b.g.gore_fx(z).into_iter().map(|(p, c, f, bg)| ((p.x, p.y), (c, f, bg))).collect();
    let tup = |c: Color| match c {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (128, 128, 128),
    };
    let buf = f.buffer_mut();
    for sy in 0..vh {
        for sx in 0..vw {
            let (x, y) = (ox + sx, oy + sy);
            let cell = buf.get_mut(inner.x + sx as u16, inner.y + sy as u16);
            let p = Pos::new(x, y, z);
            if !w.in_bounds(p) {
                cell.set_char(' ').set_bg(BLACK);
                continue;
            }
            let (mut ch, mut fg, mut bg) = ground(b, p);
            if let Some(s) = b.g.gore.stains.get(&p) {
                let (c, f2, b2) = crate::game::gore::floor_look(s, ch, tup(fg), tup(bg));
                (ch, fg, bg) = (c, rgb(f2), rgb(b2));
            }
            if let Some(s) = b.g.wall_stain(p) {
                let (c, f2, b2) = crate::game::gore::wall_look(s, ch, tup(fg));
                (ch, fg, bg) = (c, rgb(f2), rgb(b2));
            }
            if let Some(&(ic, icol)) = items.get(&(x, y)) {
                ch = ic;
                fg = icol;
            }
            if let Some(&i) = units.get(&(x, y)) {
                let wr = &b.warriors[i];
                ch = wr.kin.glyph();
                fg = side_color(wr.side);
                bg = if wr.side == 0 { Color::Rgb(15, 30, 70) } else { Color::Rgb(70, 15, 15) };
                if !b.standing(i) {
                    fg = dim(fg, 0.5);
                } else if b.body(i).bleeding > 0 {
                    bg = Color::Rgb(110, 0, 0);
                }
                if Some(i) == a.follow {
                    fg = BLACK;
                    bg = KEY;
                }
            }
            if let Some(&(gc, gcol, gbg)) = fx.get(&(x, y)) {
                ch = gc;
                fg = rgb(gcol);
                if let Some(c) = gbg {
                    bg = rgb(c);
                }
            }
            cell.set_char(ch).set_fg(fg).set_bg(bg);
        }
    }
}

/// The ground at a tile: (glyph, colour, background).
fn ground(b: &Battle, p: Pos) -> (char, Color, Color) {
    let w = &b.g.world;
    let t = w.t(p);
    if t.door != DOOR_NONE {
        return ('+', Color::Rgb(190, 140, 80), Color::Rgb(40, 25, 10));
    }
    let h = ((p.x * 73 + p.y * 151) ^ (p.x * p.y)).rem_euclid(7);
    match t.kind {
        TileKind::Wall => {
            let c = match t.mat {
                Mat::Wood => (130, 90, 50),
                Mat::Basalt => (80, 80, 95),
                _ => (115, 110, 105),
            };
            ('█', rgb(c), BLACK)
        }
        TileKind::Tree => ('♣', Color::Rgb(70, 160, 60), Color::Rgb(8, 22, 8)),
        TileKind::Open => (if h == 0 { '·' } else { ' ' }, Color::Rgb(40, 40, 70), Color::Rgb(4, 4, 12)),
        _ => match t.mat {
            Mat::Grass => (if h < 2 { '"' } else { ',' }, Color::Rgb(70, 140, 60), Color::Rgb(10, 26, 10)),
            Mat::Wood => ('=', Color::Rgb(160, 115, 60), Color::Rgb(35, 22, 10)),
            _ => ('.', Color::Rgb(115, 115, 120), Color::Rgb(18, 18, 22)),
        },
    }
}

fn kit_words(b: &Battle, i: usize) -> String {
    let mut v: Vec<String> = b.held(i).into_iter().chain(b.worn(i)).filter_map(|id| b.g.items.get(&id)).map(|it| it.name()).collect();
    if v.is_empty() {
        v.push(match b.warriors[i].kin {
            Kin::Dwarf => "bare hands".to_string(),
            k => k.name_of_attack(),
        });
    }
    if b.warriors[i].ammo > 0 {
        v.push(format!("{} bolts", b.warriors[i].ammo));
    }
    v.join(", ")
}

fn panel_lines(a: &ArenaScreen, b: &Battle, w: usize) -> Vec<Line<'static>> {
    let mut v = Vec::new();
    for s in 0..2 {
        let (up, down, dead) = b.counts(s);
        v.push(Line::from(Span::styled(cut(&format!("{} — {}", SIDES[s], b.sides[s]), w), Style::default().fg(side_color(s)).add_modifier(Modifier::BOLD))));
        v.push(plain(format!(" {} up · {} down · {} dead", up, down, dead), TEXT));
    }
    if let Some(i) = b.first_blood {
        v.push(plain(cut(&format!("First blood: {}", b.warriors[i].name), w), Color::Rgb(255, 120, 100)));
    }
    if let Some(p) = a.pick {
        v.push(plain(format!("Your pick: {}", SIDES[p].to_lowercase()), side_color(p)));
    }
    v.push(Line::default());
    match a.follow {
        Some(i) => {
            let wr = &b.warriors[i];
            v.push(header(&cut(&format!("▸ {}", wr.name), w)));
            let skill = match wr.kin {
                Kin::Dwarf => crate::dwarf::skill_rank(b.g.dwarves.iter().find(|d| d.id == wr.id).map_or(0, |d| d.fight_xp)).to_string(),
                _ => format!("skill {}", b.skill(i)),
            };
            v.push(plain(cut(&format!(" {} {}, {}", SIDES[wr.side].to_lowercase(), wr.kin.name(), skill), w), side_color(wr.side)));
            for l in wrap(&kit_words(b, i), w.saturating_sub(1)) {
                v.push(plain(format!(" {}", l), MUTED));
            }
            let state = match &wr.fate {
                Some(f) => f.clone(),
                None if !b.standing(i) => "down".to_string(),
                None => b.wounds(i),
            };
            v.push(plain(cut(&format!(" {}", state), w), if b.alive(i) { TEXT } else { Color::Rgb(255, 120, 100) }));
            let body = b.body(i);
            for p in crate::body::PARTS.iter() {
                let st = body.part_status(*p);
                if st != "fine" || body.bleeds[p.idx()] > 0 {
                    let bleed = if body.bleeds[p.idx()] > 0 { format!(" ▲{}", body.bleeds[p.idx()]) } else { String::new() };
                    v.push(plain(cut(&format!("  {}: {}{}", p.name(), st, bleed), w), Color::Rgb(235, 170, 140)));
                }
            }
            v.push(plain(format!(" kills {} · blows {} · hit {}", wr.kills.len(), wr.hits, wr.struck), TEXT));
            if !wr.kills.is_empty() {
                for l in wrap(&b.kill_list(i), w.saturating_sub(2)) {
                    v.push(plain(format!("  {}", l), MUTED));
                }
            }
        }
        None => {
            v.push(plain("Tab: follow a fighter".into(), MUTED));
            // The deadliest so far.
            let mut best: Vec<usize> = (0..b.warriors.len()).filter(|&i| b.warriors[i].hits > 0).collect();
            best.sort_by_key(|&i| (std::cmp::Reverse(b.warriors[i].kills.len()), std::cmp::Reverse(b.warriors[i].hits), i));
            for &i in best.iter().take(6) {
                let wr = &b.warriors[i];
                let mark = if b.alive(i) { ' ' } else { '†' };
                v.push(plain(cut(&format!("{}{} {}k {}h", mark, wr.name, wr.kills.len(), wr.hits), w), side_color(wr.side)));
            }
        }
    }
    v
}

fn wrap(s: &str, w: usize) -> Vec<String> {
    super::wrap_words(s, w.max(8))
}

// ---------------------------------------------------------------- the end

/// Who won, how, and what it cost each fighter.
pub(super) fn summary_lines(a: &ArenaScreen, b: &Battle, w: usize) -> Vec<Line<'static>> {
    let mut v = Vec::new();
    let Some(o) = &b.outcome else { return v };
    let verdict = match o.winner {
        Some(s) => format!("{} win{} after {} ticks.", super::cap(&b.sides[s]), if b.sides[s].ends_with('s') { "" } else { "s" }, o.ticks),
        None => format!("A draw, after {} ticks.", o.ticks),
    };
    v.push(Line::from(Span::styled(verdict, Style::default().fg(o.winner.map_or(KEY, side_color)).add_modifier(Modifier::BOLD))));
    if let Some(i) = b.first_blood {
        v.push(plain(cut(&format!("First blood: {} ({})", b.warriors[i].name, SIDES[b.warriors[i].side].to_lowercase()), w), TEXT));
    }
    if let Some(i) = b.best() {
        let k = b.warriors[i].kills.len();
        v.push(plain(cut(&format!("Fighter of the bout: {} — {} kill{}, {} telling blows", b.warriors[i].name, k, if k == 1 { "" } else { "s" }, b.warriors[i].hits), w), KEY));
    }
    let fell = b.g.annals.counters.get("arena.fell").copied().unwrap_or(0);
    let mut cost = format!("Parts severed: {}", b.severed());
    if fell > 0 {
        cost.push_str(&format!(" · over the edge: {}", fell));
    }
    v.push(plain(cost, TEXT));
    if let Some(p) = a.pick {
        let right = o.winner == Some(p);
        v.push(plain(format!("Your pick, the {}: {} (right {} of {})", SIDES[p].to_lowercase(), if right { "right!" } else { "wrong" }, a.record.0, a.record.1), if right { Color::Rgb(140, 230, 140) } else { Color::Rgb(240, 150, 110) }));
    }
    let name_w = (w / 3).clamp(10, 26);
    let state_w = (w / 3).clamp(10, 34);
    for s in 0..2 {
        let (up, down, dead) = b.counts(s);
        v.push(Line::default());
        v.push(Line::from(Span::styled(cut(&format!("{} — {}: {} standing, {} down, {} dead", SIDES[s], b.sides[s], up, down, dead), w), Style::default().fg(side_color(s)).add_modifier(Modifier::BOLD))));
        for i in (0..b.warriors.len()).filter(|&i| b.warriors[i].side == s) {
            let wr = &b.warriors[i];
            let state = match &wr.fate {
                Some(f) => f.clone(),
                None if !b.standing(i) => format!("down; {}", b.wounds(i)),
                None => b.wounds(i),
            };
            let kills = if wr.kills.is_empty() { String::new() } else { format!("{} kill{}: {}", wr.kills.len(), if wr.kills.len() == 1 { "" } else { "s" }, b.kill_list(i)) };
            let line = format!(" {:<nw$} {:<sw$} {}", cut(&wr.name, name_w), cut(&state, state_w), kills, nw = name_w, sw = state_w);
            v.push(plain(cut(&line, w), if b.alive(i) { TEXT } else { MUTED }));
        }
    }
    v
}

fn draw_summary(f: &mut Frame, area: Rect, a: &ArenaScreen, b: &Battle) {
    let w = area.width.saturating_sub(4).min(110);
    let h = area.height.saturating_sub(2);
    let mut lines = summary_lines(a, b, w.saturating_sub(2) as usize);
    let room = h.saturating_sub(4) as usize;
    let scroll = a.scroll.min(lines.len().saturating_sub(room));
    lines = lines.into_iter().skip(scroll).take(room).collect();
    lines.push(key_line("r", "rerun   n: rerun many times   v: the log   ↑↓ scroll   Enter: setup", false));
    popup(f, area, w, h, " The bout is over ", lines);
}

/// The bout as told: every blow from the fight log (misses counted, not
/// repeated) with the arena's own calls (first blood, the fallen), by tick.
fn bout_lines(b: &Battle) -> Vec<(u64, String, Color)> {
    let mut v: Vec<(u64, String, Color)> = Vec::new();
    let mut told = std::collections::HashSet::new();
    for f in &b.g.gore.fights {
        for (l, (text, grade)) in f.lines.iter().zip(crate::game::Game::fight_lines(f)) {
            told.insert(l.text.clone());
            let c = match grade {
                0 => MUTED,
                1 => Color::Rgb(215, 185, 165),
                2 => Color::Rgb(255, 150, 110),
                _ => Color::Rgb(255, 90, 90),
            };
            v.push((l.tick, text, c));
        }
    }
    for l in b.g.log.iter().filter(|l| !told.contains(&l.text)) {
        v.push((l.tick, l.text.clone(), rgb(l.color)));
    }
    v.sort_by_key(|e| e.0);
    v
}

fn draw_report(f: &mut Frame, area: Rect, b: &Battle, scroll: usize) {
    let w = area.width.saturating_sub(4).min(120);
    let h = area.height.saturating_sub(2);
    let room = h.saturating_sub(3) as usize;
    let all = bout_lines(b);
    let end = all.len().saturating_sub(scroll.min(all.len().saturating_sub(room)));
    let start = end.saturating_sub(room);
    let mut lines: Vec<Line> = all[start..end].iter().map(|(t, l, c)| plain(cut(&format!("{:>5} {}", t, l), w as usize - 2), *c)).collect();
    lines.push(plain("↑↓ PgUp PgDn scroll · any other key: back".into(), MUTED));
    popup(f, area, w, h, " The bout, blow by blow ", lines);
}

// ---------------------------------------------------------------- reruns

fn tally_lines(name: &str, t: &Tally, sides: &[String; 2], w: usize) -> Vec<Line<'static>> {
    let mut v = Vec::new();
    let n = t.runs.max(1);
    let bar = |pct: u32| -> String {
        let cells = (pct as usize * 20 + 50) / 100;
        format!("{}{}", "█".repeat(cells), "·".repeat(20 - cells))
    };
    v.push(header(&cut(&format!("{} — {} bouts", name, t.runs), w)));
    for s in 0..2 {
        v.push(Line::from(vec![
            Span::styled(format!(" {:<24}", cut(&sides[s], 24)), Style::default().fg(side_color(s))),
            Span::styled(format!("{:>3}% ", t.pct(s)), Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
            Span::styled(bar(t.pct(s)), Style::default().fg(side_color(s))),
            Span::styled(format!("  {:.1} left, {:.1} dead a bout", t.alive[s] as f64 / n as f64, t.dead[s] as f64 / n as f64), Style::default().fg(MUTED)),
        ]));
    }
    v.push(plain(format!(" Draws {}% · a bout lasts {} ticks · {:.1} parts severed a bout{}", t.draws * 100 / n, t.ticks / n as u64, t.severed as f64 / n as f64, if t.fell > 0 { format!(" · {} over the edge", t.fell) } else { String::new() }), TEXT));
    if let Some((who, s, k)) = &t.champion {
        v.push(plain(cut(&format!(" Deadliest: {} ({}), {} kills in one bout", who, SIDES[*s].to_lowercase(), k), w), KEY));
    }
    v
}

fn draw_results(f: &mut Frame, area: Rect, a: &ArenaScreen) {
    backdrop(f, area);
    let w = area.width.min(110);
    let inner = w.saturating_sub(2) as usize;
    let mut v: Vec<Line> = Vec::new();
    let sides = |m: &arena::Matchup| [format!("West: {}", m.sides[0].kin.plural()), format!("East: {}", m.sides[1].kin.plural())];
    if let Some(b) = &a.batch {
        let done = b.tally.runs * 100 / b.n.max(1);
        v.push(plain(format!("Rerunning… {} of {} ({}%)", b.tally.runs, b.n, done), KEY));
        v.extend(tally_lines(&b.matchup.name, &b.tally, &sides(&b.matchup), inner));
    } else if let Some((name, t)) = a.history.last() {
        v.extend(tally_lines(name, t, &sides(&a.matchup), inner));
    }
    v.push(Line::default());
    v.push(header("Every rerun so far"));
    v.push(plain(format!(" {:<34} {:>6} {:>6} {:>6} {:>6} {:>7}", "matchup", "bouts", "west", "east", "draw", "ticks"), MUTED));
    let room = (area.height as usize).saturating_sub(v.len() + 5);
    let skip = a.history.len().saturating_sub(room);
    for (name, t) in a.history.iter().skip(skip) {
        let n = t.runs.max(1);
        v.push(plain(cut(&format!(" {:<34} {:>6} {:>5}% {:>5}% {:>5}% {:>7}", cut(name, 34), t.runs, t.pct(0), t.pct(1), t.draws * 100 / n, t.ticks / n as u64), inner), TEXT));
    }
    if a.history.is_empty() {
        v.push(plain(" (none yet)".into(), MUTED));
    }
    v.push(Line::default());
    v.push(plain(format!(" r again ×{} · n how many · Enter back to the setup", RERUNS[a.reruns]), MUTED));
    let h = (v.len() as u16 + 2).min(area.height);
    let r = Rect::new(area.x + (area.width - w) / 2, area.y + area.height.saturating_sub(h) / 2, w, h);
    f.render_widget(Clear, r);
    f.render_widget(boxed(v, " Win rates "), r);
}

fn draw_load(f: &mut Frame, area: Rect, dir: &std::path::Path, sel: usize) {
    backdrop(f, area);
    let list = load_list(dir);
    let w = area.width.min(90);
    let inner = w.saturating_sub(2) as usize;
    let mut v: Vec<Line> = Vec::new();
    let room = (area.height as usize).saturating_sub(8).max(3);
    let first = sel.saturating_sub(room - 1);
    for (i, (m, saved)) in list.iter().enumerate().skip(first).take(room) {
        let st = if i == sel { Style::default().fg(BLACK).bg(KEY) } else { Style::default().fg(TEXT) };
        v.push(Line::from(vec![
            Span::styled(format!(" {:<8}", if *saved { "saved" } else { "preset" }), Style::default().fg(MUTED)),
            Span::styled(cut(&m.name, inner.saturating_sub(10)), st),
        ]));
    }
    v.push(Line::default());
    if let Some((m, _)) = list.get(sel) {
        for l in wrap(&m.describe(), inner.saturating_sub(2)) {
            v.push(plain(format!(" {}", l), MUTED));
        }
    }
    v.push(plain(" ↑↓ choose · Enter load · Esc back".into(), MUTED));
    let h = (v.len() as u16 + 2).min(area.height);
    let r = Rect::new(area.x + (area.width - w) / 2, area.y + area.height.saturating_sub(h) / 2, w, h);
    f.render_widget(Clear, r);
    f.render_widget(boxed(v, " Matchups "), r);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Mode;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn dump(app: &mut App, w: u16, h: u16) -> String {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| super::super::draw(f, app)).unwrap();
        let buf = term.backend().buffer().clone();
        let mut s = String::new();
        for y in 0..h {
            for x in 0..w {
                s.push_str(buf.get(x, y).symbol());
            }
            s.push('\n');
        }
        if std::env::var("SHOW_ARENA").is_ok() {
            eprintln!("{}", s);
        }
        s
    }

    fn arena_app() -> App {
        let mut app = App::new(1);
        app.save_dir = std::env::temp_dir().join(format!("dwarfclone-arena-ui-{}", std::process::id()));
        app.open_arena();
        app
    }

    #[test]
    fn the_setup_and_the_bout_draw_on_small_and_big_screens() {
        for (w, h) in [(80u16, 24u16), (200, 60)] {
            let mut app = arena_app();
            assert_eq!(app.mode, Mode::Arena);
            let s = dump(&mut app, w, h);
            assert!(s.contains("The Arena"), "{}x{}:\n{}", w, h, s);
            assert!(s.contains("dwarves") && s.contains("goblins") && s.contains("open ground"), "{}x{}:\n{}", w, h, s);
            assert!(s.contains("Enter") && s.contains("surprise me"), "no keys shown at {}x{}:\n{}", w, h, s);
            // The bout, partway.
            let a = app.arena.as_mut().unwrap();
            a.fight();
            for _ in 0..60 {
                a.tick();
            }
            a.follow = Some(0);
            let s = dump(&mut app, w, h);
            assert!(s.contains("The fighters") && s.contains("Combat log"), "{}x{}:\n{}", w, h, s);
            assert!(s.contains("West") && s.contains("East"), "{}x{}:\n{}", w, h, s);
            assert!(s.contains('☻'), "no dwarf on the map at {}x{}:\n{}", w, h, s);
            // To the end: the summary.
            let a = app.arena.as_mut().unwrap();
            a.battle.as_mut().unwrap().run();
            a.ended = 20;
            let s = dump(&mut app, w, h);
            assert!(s.contains("The bout is over"), "{}x{}:\n{}", w, h, s);
            assert!(s.contains(" win") || s.contains("draw"), "{}x{}:\n{}", w, h, s);
            // The log, the reruns and the load list.
            app.arena.as_mut().unwrap().view = View::Report { scroll: 0 };
            assert!(dump(&mut app, w, h).contains("blow by blow"));
            let a = app.arena.as_mut().unwrap();
            a.reruns = 0;
            a.rerun();
            a.work(None);
            let s = dump(&mut app, w, h);
            assert!(s.contains("Win rates") && s.contains("10 bouts"), "{}x{}:\n{}", w, h, s);
            app.arena.as_mut().unwrap().view = View::Load { sel: 0 };
            assert!(dump(&mut app, w, h).contains("preset"));
        }
    }
}
