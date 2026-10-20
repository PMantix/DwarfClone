//! A dwarf's sheet and the unit list (O06: UI-05, UI-13, UI-14).
//!
//! The sheet (`v` on a dwarf, Enter in the unit list) has tabs: Now (what
//! they are doing, their needs, gear and labors), Thoughts (each with what
//! it did to them and how long ago), Life (what the fort remembers of them,
//! season by season, from `Game.memory`; `l` opens the full Life page with
//! the portrait) and Health.
//!
//! The unit list (`u`) has four pages (citizens, pets and livestock,
//! visitors, the dead and missing; Tab) and sorts by stress, happiness, job
//! or name (`s`), with a stress gauge on every dwarf, so the one about to
//! break stands out.

use super::{add_target, header, health, plain, rgb, task_desc, unit_view, wrap_words, Target, KEY, MUTED, TEXT};
use crate::app::{App, Mode};
use crate::dwarf::{Dwarf, LABORS};
use crate::game::{Game, SEASON, SEASONS, YEAR};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

/// The sheet's tabs.
pub const TABS: [&str; 4] = ["Now", "Thoughts", "Life", "Health"];
pub const TAB_NOW: usize = 0;
pub const TAB_THOUGHTS: usize = 1;
pub const TAB_LIFE: usize = 2;
pub const TAB_HEALTH: usize = 3;

/// "Spring 203": the season and year of a tick.
pub fn season_of(g: &Game, tick: u64) -> String {
    format!("{} {}", SEASONS[((tick % YEAR) / SEASON) as usize], g.start_year as u64 + tick / YEAR)
}

/// How long ago, in words.
fn ago(now: u64, then: u64) -> String {
    let days = now.saturating_sub(then) / crate::game::DAY;
    match days {
        0 => "today".into(),
        1 => "yesterday".into(),
        n if n < 60 => format!("{} days ago", n),
        n => format!("{} months ago", n / 28),
    }
}

/// The row of tab names, the open one lit.
fn tab_bar(tab: usize) -> Line<'static> {
    let mut spans = Vec::new();
    for (i, t) in TABS.iter().enumerate() {
        let style = if i == tab { Style::default().fg(Color::Rgb(0, 0, 0)).bg(KEY).add_modifier(Modifier::BOLD) } else { Style::default().fg(MUTED) };
        spans.push(Span::styled(format!(" {} ", t), style));
    }
    Line::from(spans)
}

/// A stress gauge: how close a dwarf is to breaking, coloured by mood.
pub fn stress_bar(d: &Dwarf, n: usize) -> Span<'static> {
    let fill = ((d.stress.clamp(-200, 400) + 200) as usize * n + 300) / 600;
    let fill = fill.min(n);
    let (_, c) = d.mood();
    Span::styled(format!("{}{}", "█".repeat(fill), "░".repeat(n - fill)), Style::default().fg(rgb(c)))
}

/// The Thoughts tab: how they feel and why, newest first.
fn thought_lines(g: &Game, d: &Dwarf, w: usize) -> Vec<Line<'static>> {
    let (mood, mc) = d.mood();
    let mut v = vec![
        Line::from(vec![Span::styled("Stress  ", Style::default().fg(TEXT)), stress_bar(d, 16), Span::styled(format!(" {}", mood), Style::default().fg(rgb(mc)))]),
        Line::default(),
    ];
    if d.thoughts.is_empty() {
        v.push(plain("Nothing much on their mind.".into(), MUTED));
    }
    for t in d.thoughts.iter().rev() {
        let col = if t.stress < 0 { Color::Rgb(120, 220, 120) } else { Color::Rgb(230, 140, 100) };
        for (i, l) in wrap_words(&format!("{} {}", if t.stress < 0 { "+" } else { "-" }, t.text), w).into_iter().enumerate() {
            v.push(plain(if i == 0 { l } else { format!("  {}", l) }, col));
        }
        let felt = match t.stress {
            s if s <= -30 => "lifted them greatly",
            s if s < 0 => "cheered them",
            0 => "left them cold",
            s if s < 30 => "upset them",
            _ => "weighed on them heavily",
        };
        v.push(plain(format!("  {}, {}", felt, ago(g.tick, t.tick)), MUTED));
    }
    v
}

/// The Life tab: what the fort remembers of them, under the season it
/// happened.
pub fn life_lines(g: &Game, d: &Dwarf, w: usize) -> Vec<Line<'static>> {
    let mut v = Vec::new();
    let mut season = String::new();
    for m in g.memory_of(d.id) {
        let s = season_of(g, m.tick);
        if s != season {
            if !season.is_empty() {
                v.push(Line::default());
            }
            v.push(header(&s));
            season = s;
        }
        let col = if m.weight >= 6 { Color::Rgb(255, 215, 60) } else { TEXT };
        for (i, l) in wrap_words(&m.text, w.saturating_sub(2).max(10)).into_iter().enumerate() {
            v.push(plain(format!("{} {}", if i == 0 { "·" } else { " " }, l), col));
        }
    }
    if v.is_empty() {
        v.push(plain("The fort remembers nothing of them yet.".into(), MUTED));
    }
    v.push(Line::default());
    v.push(plain("l: their whole story, with a portrait".into(), KEY));
    v
}

/// The Health tab: wounds, bleeding, and the medical history.
fn health_lines(d: &Dwarf) -> Vec<Line<'static>> {
    let mut v = Vec::new();
    if d.body.bleeding > 0 {
        let from = d.body.worst_bleed().map_or(String::new(), |p| format!(" from the {}", p.name()));
        v.push(plain(format!("Bleeding{} (blood {}%)", from, d.body.blood * 100 / d.body.max_blood.max(1)), Color::Rgb(255, 60, 60)));
    }
    if d.body.infected() {
        v.push(plain("An infected wound".into(), Color::Rgb(220, 160, 60)));
    } else if d.body.dirty {
        v.push(plain("A dirty wound (needs cleaning)".into(), Color::Rgb(200, 180, 90)));
    }
    let hurt = health::part_lines(d, "", false);
    if hurt.is_empty() && v.is_empty() {
        v.push(plain(if d.alive { "Hale and whole.".into() } else { "Dead.".into() }, Color::Rgb(120, 220, 120)));
    }
    v.extend(hurt);
    if !d.med.is_empty() {
        v.push(Line::default());
        v.push(header("Medical history"));
        v.extend(health::history_lines(d, 20, ""));
    }
    v
}

/// A dwarf's sheet, open at `tab`, `w` columns wide: the tab bar, then the
/// tab.
pub fn sheet_lines(g: &Game, d: &Dwarf, sel: usize, tab: usize, w: usize) -> Vec<Line<'static>> {
    let mut v = vec![tab_bar(tab), Line::default()];
    match tab {
        TAB_THOUGHTS | TAB_LIFE | TAB_HEALTH => {
            v.push(Line::from(Span::styled(d.called(), Style::default().fg(rgb(d.color)).add_modifier(Modifier::BOLD))));
            v.push(Line::default());
            v.extend(match tab {
                TAB_THOUGHTS => thought_lines(g, d, w),
                TAB_LIFE => life_lines(g, d, w),
                _ => health_lines(d),
            });
        }
        _ => {
            let mut now = unit_view(g, d, sel);
            now.pop();
            v.extend(now);
        }
    }
    v.push(Line::default());
    v.push(plain("Tab/←→ pages · l life · c center · Esc".into(), MUTED));
    v
}

/// The selected labor's row on the Now page.
const PICKED: Color = Color::Rgb(50, 60, 120);

/// The sheet as a side pane beside the live map (or over the whole screen):
/// it is redrawn every frame, so it changes as the dwarf goes about their
/// day.
pub(super) fn draw(f: &mut Frame, area: Rect, app: &App, id: u32, sel: usize, tab: usize) {
    let g = &app.game;
    let Some(d) = find_dwarf(g, id) else { return };
    f.render_widget(Clear, area);
    let following = app.follow == Some(id);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(MUTED))
        .style(Style::default().bg(Color::Rgb(12, 12, 20)))
        .title(Span::styled(format!(" {}{} ", d.name, if following { " (following)" } else { "" }), Style::default().fg(KEY)));
    let inner = block.inner(area);
    f.render_widget(block, area);
    add_target(area, Target::Block);
    let w = inner.width.saturating_sub(2) as usize;
    let mut v = sheet_lines(g, d, sel, tab, w);
    v.insert(v.len() - 1, plain(if following { "f: stop following".into() } else { "f: follow them on the map".into() }, MUTED));
    let h = inner.height as usize;
    // The Now page scrolls to keep the labor cursor in sight; the others
    // scroll with the arrows.
    let scroll = if tab == TAB_NOW {
        v.iter().position(|l| l.spans.iter().any(|s| s.style.bg == Some(PICKED))).map_or(0, |i| (i + 3).saturating_sub(h))
    } else {
        sel
    };
    let scroll = scroll.min(v.len().saturating_sub(h));
    let shown: Vec<Line> = v.into_iter().skip(scroll).take(h).collect();
    f.render_widget(Paragraph::new(shown), Rect::new(inner.x + 1, inner.y, inner.width.saturating_sub(1), inner.height));
}

/// The unit list's pages.
pub const CATS: [&str; 4] = ["Citizens", "Pets and livestock", "Visitors", "Dead and missing"];
/// What the unit list sorts by (`s` cycles).
pub const SORTS: [&str; 4] = ["stress", "happiness", "job", "name"];

/// One row of the unit list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    Dwarf(u32),
    /// Away on the expedition.
    Away(u32),
    Creature(u32),
}

/// The rows on page `cat`, sorted by `sort`.
pub fn rows(g: &Game, cat: u8, sort: u8) -> Vec<Row> {
    // (stress, job, name, row)
    let mut v: Vec<(i32, String, String, Row)> = Vec::new();
    let dwarf = |d: &Dwarf, row: Row| (d.stress, if d.alive { task_desc(g, d) } else { String::new() }, d.name.clone(), row);
    let beast = |c: &crate::creature::Creature| (0, String::new(), c.title(), Row::Creature(c.id));
    match cat {
        0 => v.extend(g.dwarves.iter().filter(|d| d.alive).map(|d| dwarf(d, Row::Dwarf(d.id)))),
        1 => v.extend(g.creatures.values().filter(|c| c.alive && c.tame).map(beast)),
        2 => v.extend(g.creatures.values().filter(|c| c.alive && !c.tame && !c.hostile() && !c.siege && c.species.person()).map(beast)),
        _ => {
            v.extend(g.dwarves.iter().filter(|d| !d.alive).map(|d| dwarf(d, Row::Dwarf(d.id))));
            if let Some(ex) = &g.expedition {
                v.extend(ex.members.iter().map(|d| dwarf(d, Row::Away(d.id))));
            }
        }
    }
    match sort {
        0 => v.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.2.cmp(&b.2))),
        1 => v.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.2.cmp(&b.2))),
        2 => v.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.2.cmp(&b.2))),
        _ => v.sort_by(|a, b| a.2.cmp(&b.2)),
    }
    v.into_iter().map(|r| r.3).collect()
}

fn find_dwarf(g: &Game, id: u32) -> Option<&Dwarf> {
    g.dwarves.iter().find(|d| d.id == id).or_else(|| g.expedition.as_ref().and_then(|ex| ex.members.iter().find(|d| d.id == id)))
}

/// The unit list, for the side panel.
pub fn list_lines(g: &Game, sel: usize, cat: u8, sort: u8) -> Vec<Line<'static>> {
    let rows = rows(g, cat, sort);
    let mut v = vec![
        Line::from(vec![
            Span::styled("◄ ", Style::default().fg(KEY)),
            Span::styled(format!("{} ({})", CATS[cat as usize % CATS.len()], rows.len()), Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
            Span::styled(" ►", Style::default().fg(KEY)),
        ]),
        Line::from(vec![Span::styled("sorted by ", Style::default().fg(MUTED)), Span::styled(SORTS[sort as usize % SORTS.len()], Style::default().fg(KEY))]),
        Line::default(),
    ];
    if rows.is_empty() {
        v.push(plain("Nobody here.".into(), MUTED));
    }
    let sel = sel.min(rows.len().saturating_sub(1));
    for (i, r) in rows.iter().enumerate() {
        let style = if i == sel { Style::default().bg(Color::Rgb(50, 60, 120)) } else { Style::default() };
        match *r {
            Row::Dwarf(id) | Row::Away(id) => {
                let Some(d) = find_dwarf(g, id) else { continue };
                v.push(Line::from(vec![
                    Span::styled("☺ ", style.fg(if d.alive { rgb(d.color) } else { MUTED })),
                    Span::styled(format!("{:<19.19}", d.name), style.fg(if d.alive { TEXT } else { MUTED })),
                    if d.alive { stress_bar(d, 5) } else { Span::styled("     ", style) },
                    Span::styled(format!(" {:.8}", d.profession()), style.fg(MUTED)),
                ]));
                if i == sel {
                    let (mood, mc) = d.mood();
                    v.push(match r {
                        Row::Away(_) => plain(format!("   away: {}", g.expedition.as_ref().map_or(String::new(), |ex| ex.dest.name.clone())), Color::Rgb(150, 210, 255)),
                        _ if !d.alive => plain("   Dead".into(), MUTED),
                        _ => Line::from(vec![Span::styled(format!("   {} · ", task_desc(g, d)), Style::default().fg(TEXT)), Span::styled(mood, Style::default().fg(rgb(mc)))]),
                    });
                }
            }
            Row::Creature(id) => {
                let Some(c) = g.creatures.get(&id) else { continue };
                v.push(Line::from(vec![
                    Span::styled("  ", style),
                    Span::styled(format!("{:<19.19}", c.title()), style.fg(TEXT)),
                    Span::styled(format!(" {}", c.name()), style.fg(MUTED)),
                ]));
                if i == sel {
                    let owner = c.owner.and_then(|o| find_dwarf(g, o)).map_or(String::new(), |d| format!(", kept by {}", d.name.split(' ').next().unwrap_or("")));
                    let what = if !c.tame { "a guest" } else if c.adult { "tame" } else { "young" };
                    v.push(plain(format!("   {}{}{}", what, owner, if c.pregnant > 0 { ", with young" } else { "" }), MUTED));
                }
            }
        }
    }
    v.push(Line::default());
    v.push(plain("↑↓ select · Enter view · c center".into(), MUTED));
    v.push(plain("Tab/←→ page · s sort".into(), MUTED));
    v
}

impl App {
    /// The keys on the unit list.
    pub fn key_units(&mut self, k: KeyEvent, sel: usize, cat: u8, sort: u8) {
        let rows = rows(&self.game, cat, sort);
        let n = CATS.len() as u8;
        let sel = sel.min(rows.len().saturating_sub(1));
        let pos = |g: &Game, r: Row| match r {
            Row::Dwarf(id) => g.dwarves.iter().find(|d| d.id == id).map(|d| d.pos),
            Row::Creature(id) => g.creatures.get(&id).map(|c| c.pos),
            Row::Away(_) => None,
        };
        match k.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Up => self.mode = Mode::Units { sel: sel.saturating_sub(1), cat, sort },
            KeyCode::Down => self.mode = Mode::Units { sel: (sel + 1).min(rows.len().saturating_sub(1)), cat, sort },
            KeyCode::Tab | KeyCode::Right => self.mode = Mode::Units { sel: 0, cat: (cat + 1) % n, sort },
            KeyCode::BackTab | KeyCode::Left => self.mode = Mode::Units { sel: 0, cat: (cat + n - 1) % n, sort },
            KeyCode::Char('s') => self.mode = Mode::Units { sel: 0, cat, sort: (sort + 1) % SORTS.len() as u8 },
            KeyCode::Enter => match rows.get(sel) {
                Some(Row::Dwarf(id)) => self.mode = Mode::Unit { id: *id, sel: 0, tab: 0 },
                Some(Row::Away(_)) => self.flash = Some("Away on the expedition.".into()),
                Some(&r) => {
                    if let Some(p) = pos(&self.game, r) {
                        self.cursor = p;
                        self.mode = Mode::Look { sel: 0 };
                    }
                }
                None => {}
            },
            KeyCode::Char('c') => {
                if let Some(p) = rows.get(sel).and_then(|&r| pos(&self.game, r)) {
                    self.cursor = p;
                    self.mode = Mode::Normal;
                }
            }
            _ => {}
        }
    }

    /// The keys on a dwarf's sheet.
    pub fn key_unit(&mut self, k: KeyEvent, id: u32, sel: usize, tab: usize) {
        let n = TABS.len();
        match k.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Tab | KeyCode::Right => self.mode = Mode::Unit { id, sel: 0, tab: (tab + 1) % n },
            KeyCode::BackTab | KeyCode::Left => self.mode = Mode::Unit { id, sel: 0, tab: (tab + n - 1) % n },
            KeyCode::Char('c') => {
                if let Some(d) = self.game.dwarves.iter().find(|d| d.id == id) {
                    self.cursor = d.pos;
                }
                self.mode = Mode::Normal;
            }
            KeyCode::Up => self.mode = Mode::Unit { id, sel: sel.saturating_sub(1), tab },
            KeyCode::Down => {
                let max = if tab == TAB_NOW { LABORS.len() - 1 } else { 500 };
                self.mode = Mode::Unit { id, sel: (sel + 1).min(max), tab };
            }
            KeyCode::Enter if tab == TAB_NOW => {
                if let Some(lost) = self.game.toggle_labor(id, LABORS[sel.min(LABORS.len() - 1)]) {
                    self.flash = Some(format!("{} dropped: one tool labor per dwarf.", lost.name()));
                }
            }
            KeyCode::Char('s') => {
                let on = self.game.toggle_specialist(id);
                self.flash = Some(if on { "Specialist: won't haul.".into() } else { "Will haul again.".into() });
            }
            KeyCode::Char('m') => self.game.toggle_soldier(id),
            KeyCode::Char('l') => self.mode = Mode::Life { id, scroll: 0 },
            KeyCode::Char('f') => {
                self.follow = if self.follow == Some(id) { None } else { Some(id) };
                self.follow_unit();
            }
            _ => {}
        }
    }

    /// Keep the map on the dwarf whose sheet is open, while `f` is on.
    pub fn follow_unit(&mut self) {
        let Some(id) = self.follow else { return };
        match self.mode {
            Mode::Unit { id: open, .. } if open == id => {
                if let Some(d) = self.game.dwarves.iter().find(|d| d.id == id && d.alive) {
                    self.cursor = d.pos;
                    self.free_view = false;
                }
            }
            _ => self.follow = None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::memory::{Memory, MemoryKind};

    fn text(lines: &[Line]) -> Vec<String> {
        lines.iter().map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect()).collect()
    }

    #[test]
    fn a_dwarfs_sheet_tells_their_life() {
        let mut g = Game::new(5);
        g.memory = Default::default();
        let (a, b) = (g.dwarves[0].id, g.dwarves[1].id);
        let mut add = |tick: u64, kind: MemoryKind, who: Vec<u32>, text: &str, weight: u8| {
            let id = g.memory.events.len() as u32;
            g.memory.events.push(Memory { id, tick, kind, who, at: None, text: text.into(), weight });
        };
        add(SEASON / 2, MemoryKind::Arrived, vec![a], "Urist came to the fort.", 2);
        add(YEAR + SEASON + 3, MemoryKind::Married, vec![a, b], "Urist and Kib were wed.", 4);
        add(YEAR + 2 * SEASON + 5, MemoryKind::Epithet, vec![a], "Urist slew the troll and was known after as the Bold.", 6);
        add(YEAR + 2 * SEASON + 6, MemoryKind::Arrived, vec![b], "Kib came to the fort.", 2);
        let y = g.start_year;
        let d = g.dwarves[0].clone();
        let lines = text(&sheet_lines(&g, &d, 0, TAB_LIFE, 60));
        let at = |s: &str| lines.iter().position(|l| l.contains(s)).unwrap_or_else(|| panic!("no {:?} in {:#?}", s, lines));
        assert!(lines[0].contains("Life"));
        assert!(at(&format!("Spring {}", y)) < at("came to the fort"));
        assert!(at("came to the fort") < at(&format!("Summer {}", y + 1)));
        assert!(at(&format!("Summer {}", y + 1)) < at("were wed"));
        assert!(at("were wed") < at(&format!("Autumn {}", y + 1)));
        assert!(at(&format!("Autumn {}", y + 1)) < at("the Bold"));
        assert!(!lines.iter().any(|l| l.contains("Kib came")), "only their own memories");
        // The tabs cycle, and the thoughts tab says what each did to them.
        let mut app = App::with_game_for_tests(g);
        app.mode = Mode::Unit { id: a, sel: 3, tab: TAB_NOW };
        app.key_unit(KeyEvent::from(KeyCode::Tab), a, 3, TAB_NOW);
        assert_eq!(app.mode, Mode::Unit { id: a, sel: 0, tab: TAB_THOUGHTS });
        app.key_unit(KeyEvent::from(KeyCode::BackTab), a, 0, TAB_NOW);
        assert_eq!(app.mode, Mode::Unit { id: a, sel: 0, tab: TAB_HEALTH });
        let tick = app.game.tick;
        app.game.dwarves[0].think("saw a friend die", 60, tick);
        let d = app.game.dwarves[0].clone();
        let lines = text(&sheet_lines(&app.game, &d, 0, TAB_THOUGHTS, 60));
        assert!(lines.iter().any(|l| l.contains("saw a friend die")) && lines.iter().any(|l| l.contains("weighed on them heavily, today")), "{:#?}", lines);
    }

    fn dump(app: &mut App, w: u16, h: u16) -> String {
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
        term.draw(|f| super::super::draw(f, app)).unwrap();
        let buf = term.backend().buffer().clone();
        let mut s = String::new();
        for y in 0..h {
            for x in 0..w {
                s.push_str(buf.get(x, y).symbol());
            }
            s.push('\n');
        }
        s
    }

    #[test]
    fn the_unit_list_sorts_by_stress_and_lists_the_dead() {
        let mut g = Game::new(5);
        for (i, d) in g.dwarves.iter_mut().enumerate() {
            d.stress = i as i32 * 10;
        }
        g.dwarves[2].stress = 500;
        g.dwarves[0].alive = false;
        let (worst, dead) = (g.dwarves[2].id, g.dwarves[0].id);
        let list = rows(&g, 0, 0);
        assert_eq!(list[0], Row::Dwarf(worst), "the most stressed first");
        assert!(!list.contains(&Row::Dwarf(dead)));
        let stress = |r: &Row| match r {
            Row::Dwarf(id) => g.dwarves.iter().find(|d| d.id == *id).unwrap().stress,
            _ => panic!(),
        };
        assert!(list.windows(2).all(|w| stress(&w[0]) >= stress(&w[1])));
        let happy = rows(&g, 0, 1);
        assert_eq!(happy.last(), Some(&Row::Dwarf(worst)), "happiness puts them last");
        let names: Vec<String> = rows(&g, 0, 3).iter().map(|r| match r {
            Row::Dwarf(id) => g.dwarves.iter().find(|d| d.id == *id).unwrap().name.clone(),
            _ => panic!(),
        }).collect();
        assert!(names.windows(2).all(|w| w[0] <= w[1]));
        assert_eq!(rows(&g, 3, 0), vec![Row::Dwarf(dead)], "the dead have their own page");
        // A tame goat is livestock, a merchant a visitor.
        let p = g.dwarves[1].pos;
        let mut goat = crate::creature::Creature::new(9001, crate::creature::Species::Goat, p, 0);
        goat.tame = true;
        g.creatures.insert(9001, goat);
        g.creatures.insert(9002, crate::creature::Creature::new(9002, crate::creature::Species::Merchant, p, 0));
        let (pets, guests) = (rows(&g, 1, 0), rows(&g, 2, 0));
        assert!(pets.contains(&Row::Creature(9001)) && !pets.contains(&Row::Creature(9002)));
        assert!(guests.contains(&Row::Creature(9002)) && !guests.contains(&Row::Creature(9001)));
        // On screen: the page, the gauge, the keys.
        let mut app = App::with_game_for_tests(g);
        app.mode = Mode::Units { sel: 0, cat: 0, sort: 0 };
        let s = dump(&mut app, 130, 50);
        assert!(s.contains("Citizens") && s.contains("sorted by stress") && s.contains("█"), "{}", s);
        app.key_units(KeyEvent::from(KeyCode::Char('s')), 0, 0, 0);
        assert_eq!(app.mode, Mode::Units { sel: 0, cat: 0, sort: 1 });
        app.key_units(KeyEvent::from(KeyCode::BackTab), 0, 0, 1);
        assert_eq!(app.mode, Mode::Units { sel: 0, cat: 3, sort: 1 });
        app.key_units(KeyEvent::from(KeyCode::Enter), 0, 3, 1);
        assert_eq!(app.mode, Mode::Unit { id: dead, sel: 0, tab: 0 }, "the dead have sheets too");
    }

    #[test]
    fn the_unit_sheet_opens_beside_a_live_map() {
        let mut app = App::with_game_for_tests(Game::new(5));
        let d = app.game.dwarves[1].clone();
        app.mode = Mode::Unit { id: d.id, sel: 0, tab: TAB_LIFE };
        assert!(app.dockable() && app.docked);
        let s = dump(&mut app, 160, 50);
        let row = |s: &str, y: usize| s.lines().nth(y).unwrap_or("").to_string();
        // The pane takes the right of the screen; the map stays on the left.
        let title = (0..50).map(|y| row(&s, y)).find(|r| r.contains(&d.name)).unwrap_or_else(|| panic!("{}", s));
        let col = |r: &str| r[..r.find(&d.name).unwrap()].chars().count();
        assert!(col(&title) > 60, "{}", s);
        assert!(app.map_rect.width >= 40, "the map is still drawn: {:?}", app.map_rect);
        assert!(s.contains("Thoughts") && s.contains("Health"));
        // `f` keeps the map on them as they move.
        app.key_unit(KeyEvent::from(KeyCode::Char('f')), d.id, 0, TAB_LIFE);
        assert_eq!(app.follow, Some(d.id));
        let to = crate::geom::Pos { x: d.pos.x + 1, ..d.pos };
        app.game.dwarves[1].pos = to;
        app.follow_unit();
        assert_eq!(app.cursor, to);
        assert!(dump(&mut app, 160, 50).contains("following"));
        // Closing the sheet stops it.
        app.key_unit(KeyEvent::from(KeyCode::Esc), d.id, 0, TAB_LIFE);
        app.follow_unit();
        assert_eq!(app.follow, None);
        // Full screen when undocked.
        app.mode = Mode::Unit { id: d.id, sel: 0, tab: TAB_NOW };
        app.docked = false;
        let s = dump(&mut app, 160, 50);
        let title = (0..50).map(|y| row(&s, y)).find(|r| r.contains(&d.name)).unwrap();
        assert!(col(&title) < 10, "{}", s);
    }

    #[test]
    fn joining_a_squad_goes_on_the_life_page() {
        let mut g = Game::new(5);
        let id = g.dwarves.iter().find(|d| !d.is_child()).unwrap().id;
        let s = g.new_squad();
        g.set_squad_member(s, id, true);
        let name = g.squads[s].name.clone();
        assert!(g.memory_of(id).any(|m| m.kind == MemoryKind::Soldier && m.text.contains(&name)));
        let d = g.dwarves.iter().find(|d| d.id == id).unwrap().clone();
        assert!(text(&life_lines(&g, &d, 60)).iter().any(|l| l.contains("joined")));
    }
}
