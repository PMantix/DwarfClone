//! A dwarf's sheet and the unit list (O06: UI-05, UI-13, UI-14).
//!
//! The sheet (`v` on a dwarf, Enter in the unit list) has tabs: Now (what
//! they are doing, their needs, gear and labors), Thoughts (each with what
//! it did to them and how long ago), Life (what the fort remembers of them,
//! season by season, from `Game.memory`; `l` opens the full Life page with
//! the portrait) and Health.

use super::{header, health, plain, rgb, unit_view, wrap_words, KEY, MUTED, TEXT};
use crate::app::{App, Mode};
use crate::dwarf::{Dwarf, LABORS};
use crate::game::{Game, SEASON, SEASONS, YEAR};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

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

impl App {
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
            _ => {}
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
}
