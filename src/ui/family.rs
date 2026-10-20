//! A dwarf's family tree (O06, UI-15): `F` on the Life tab of a dwarf's
//! sheet, or on the Life page. A row per generation, grandparents at the
//! top and grandchildren at the bottom, each relative a card with what
//! they are to the dwarf, a ♥ or ✗ for a friendship or a grudge between
//! them, and for the dead a † and where they lie, or that they lie
//! unburied. Below the tree, the one picked: the living as they are, the
//! dead as they died (`Game::death_scene`).
//!
//! The arrows move about the tree, Enter opens the relative's sheet, `t`
//! redraws the tree around them and `c` goes to them on the map.

use super::{add_target, plain, rgb, task_desc, wrap_words, Target, KEY, MUTED, TEXT};
use crate::app::{App, Mode};
use crate::game::familytree::{Kin, Mark, Who};
use crate::game::Game;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

/// A card's width, and the gap between cards.
const CARD_W: usize = 24;
const GAP: usize = 2;

const FRIEND: Color = Color::Rgb(120, 220, 120);
const GRUDGE: Color = Color::Rgb(240, 90, 80);
const DEAD: Color = Color::Rgb(150, 150, 160);
const UNBURIED: Color = Color::Rgb(210, 140, 110);
const GHOST: Color = Color::Rgb(170, 200, 255);
const PICKED: Color = Color::Rgb(50, 60, 120);

/// Everyone on the tree in reading order, each with the row they are on.
pub fn flat(g: &Game, id: u32) -> Vec<(usize, Kin)> {
    g.family_tree(id).into_iter().enumerate().flat_map(|(r, gen)| gen.kin.into_iter().map(move |k| (r, k))).collect()
}

fn fit(s: &str, w: usize) -> String {
    let n = s.chars().count();
    if n <= w {
        format!("{:<w$}", s, w = w)
    } else {
        format!("{}…", s.chars().take(w - 1).collect::<String>())
    }
}

/// A card's three lines.
fn card(g: &Game, k: &Kin, me: bool) -> [(String, Style); 3] {
    let st = |c: Color| Style::default().fg(c);
    let (mark, mc) = match k.mark {
        Mark::Friend => ("♥ ", FRIEND),
        Mark::Grudge => ("✗ ", GRUDGE),
        Mark::None => ("  ", TEXT),
    };
    match k.who {
        Who::Dwarf(id) => {
            let Some(d) = g.kin_dwarf(id) else { return Default::default() };
            let name_c = if me { KEY } else if d.alive { rgb(d.color) } else { DEAD };
            let mut name = st(name_c);
            if me {
                name = name.add_modifier(Modifier::BOLD);
            }
            let rel = if me { "".to_string() } else { format!("{} · ", k.rel) };
            let line2 = if d.alive { format!("{}{}", rel, d.profession().to_lowercase()) } else { format!("{}† {}", rel, d.died.map_or(String::new(), |t| (g.start_year as u64 + t / crate::game::YEAR).to_string())) };
            let (line3, c3) = match g.resting_place(d) {
                Some(r) if d.ghost => (r, GHOST),
                Some(r) if r.contains("unburied") => (r, UNBURIED),
                Some(r) => (r, DEAD),
                None if g.expedition.as_ref().map_or(false, |ex| ex.members.iter().any(|m| m.id == id)) => ("away on the expedition".into(), Color::Rgb(150, 210, 255)),
                None => (d.age.map_or("alive".into(), |a| format!("alive, {}", a)), MUTED),
            };
            [(format!("{}{}", mark, d.called()), name.fg(if k.mark == Mark::None || me { name_c } else { mc })), (line2, st(if d.alive { TEXT } else { DEAD })), (line3, st(c3))]
        }
        Who::Beast(id) => {
            let Some(c) = g.creatures.get(&id) else { return Default::default() };
            let state = if c.pregnant > 0 {
                "with young"
            } else if !c.adult {
                "young"
            } else {
                "tame"
            };
            let (gl, col) = (c.title(), rgb(c.species.info().color));
            [(format!("  {}", gl), st(col)), (format!("{} · {}", k.rel, c.name()), st(TEXT)), (state.into(), st(MUTED))]
        }
    }
}

/// What the picked relative is like: the living as they are, the dead as
/// they died.
fn detail(g: &Game, root: u32, k: &Kin, w: usize) -> Vec<Line<'static>> {
    let mut v = Vec::new();
    let mut text = |s: String, c: Color| {
        for l in wrap_words(&s, w) {
            v.push(plain(l, c));
        }
    };
    match k.who {
        Who::Dwarf(id) => {
            let Some(d) = g.kin_dwarf(id) else { return v };
            if d.alive {
                let age = d.age.map_or(String::new(), |a| format!(", {}", a));
                text(format!("{}{}: {}{}. {}.", d.called(), if id == root { String::new() } else { format!(", {}", k.rel) }, d.profession().to_lowercase(), age, task_desc(g, d)), TEXT);
            } else {
                text(g.death_scene(d), TEXT);
                if let Some(r) = g.resting_place(d) {
                    let she = match d.female {
                        Some(true) => "She",
                        Some(false) => "He",
                        None => "They",
                    };
                    text(format!("{} {}.", she, r), if r.contains("unburied") { UNBURIED } else { DEAD });
                }
            }
            if id != root {
                let (Some(a), Some(b)) = (g.kin_dwarf(root), Some(d)) else { return v };
                let feel = |r: i32| match r {
                    r if r >= 80 => "thinks the world of",
                    r if r >= crate::game::familytree::FRIEND => "is fond of",
                    r if r <= -70 => "hates",
                    r if r <= crate::game::familytree::GRUDGE => "bears a grudge against",
                    r if r < 0 => "is cool toward",
                    _ => "has no strong feelings for",
                };
                let first = |n: &str| n.split(' ').next().unwrap_or("").to_string();
                let c = match k.mark {
                    Mark::Friend => FRIEND,
                    Mark::Grudge => GRUDGE,
                    Mark::None => MUTED,
                };
                text(format!("{} {} {}; {} {} {}.", first(&a.name), feel(a.regard(id)), first(&b.name), first(&b.name), feel(b.regard(root)), first(&a.name)), c);
            }
        }
        Who::Beast(id) => {
            if let Some(c) = g.creatures.get(&id) {
                let pasture = c.pasture.and_then(|z| g.zones.get(&z)).map_or(String::new(), |z| format!(", kept in {}", z.name.clone().unwrap_or_else(|| "a pasture".into())));
                text(format!("{}, {}{}.", c.title(), k.rel, pasture), TEXT);
            }
        }
    }
    v
}

/// The tree's lines at `w` columns, and the line the picked card starts on.
pub fn tree_lines(g: &Game, id: u32, sel: usize, w: usize) -> (Vec<Line<'static>>, usize) {
    let tree = g.family_tree(id);
    let per = ((w + GAP) / (CARD_W + GAP)).max(1);
    let mut v: Vec<Line<'static>> = Vec::new();
    let mut at = 0;
    let mut n = 0;
    for gen in &tree {
        v.push(Line::from(Span::styled(gen.title.to_string(), Style::default().fg(Color::Rgb(140, 200, 255)).add_modifier(Modifier::BOLD))));
        for chunk in gen.kin.chunks(per) {
            let cards: Vec<[(String, Style); 3]> = chunk.iter().map(|k| card(g, k, k.who == Who::Dwarf(id))).collect();
            for row in 0..3 {
                let mut spans = Vec::new();
                for (i, c) in cards.iter().enumerate() {
                    let picked = n + i == sel;
                    if picked && row == 0 {
                        at = v.len();
                    }
                    let style = if picked { c[row].1.bg(PICKED) } else { c[row].1 };
                    spans.push(Span::styled(fit(&c[row].0, CARD_W), style));
                    spans.push(Span::raw(" ".repeat(GAP)));
                }
                v.push(Line::from(spans));
            }
            n += chunk.len();
        }
        v.push(Line::default());
    }
    (v, at)
}

pub(super) fn draw(f: &mut Frame, area: Rect, app: &App, id: u32, sel: usize) {
    let g = &app.game;
    let Some(d) = g.kin_dwarf(id) else { return };
    let w = (area.width.saturating_sub(4)).min(4 * (CARD_W + GAP) as u16 + 2).max(30);
    let r = Rect::new(area.x + (area.width.saturating_sub(w + 4)) / 2, area.y, (w + 4).min(area.width), area.height);
    f.render_widget(Clear, r);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(MUTED))
        .style(Style::default().bg(Color::Rgb(12, 12, 20)))
        .title(Span::styled(format!(" The family of {} ", d.called()), Style::default().fg(KEY)));
    let inner = block.inner(r);
    f.render_widget(block, r);
    add_target(r, Target::Block);
    let w = inner.width.saturating_sub(2) as usize;
    let all = flat(g, id);
    let sel = sel.min(all.len().saturating_sub(1));
    let (tree, at) = tree_lines(g, id, sel, w);
    let mut foot: Vec<Line<'static>> = Vec::new();
    if let Some((_, k)) = all.get(sel) {
        foot.extend(detail(g, id, k, w));
    }
    foot.push(Line::default());
    foot.push(Line::from(vec![
        Span::styled("♥ ", Style::default().fg(FRIEND)),
        Span::styled("friends  ", Style::default().fg(MUTED)),
        Span::styled("✗ ", Style::default().fg(GRUDGE)),
        Span::styled("a grudge  ", Style::default().fg(MUTED)),
        Span::styled("† ", Style::default().fg(DEAD)),
        Span::styled("dead", Style::default().fg(MUTED)),
    ]));
    foot.push(plain("←→↑↓ move · Enter their sheet · t their tree · c go to · Esc back".into(), MUTED));
    let h = inner.height as usize;
    let tree_h = h.saturating_sub(foot.len() + 1);
    // Scroll the tree to keep the picked card in sight.
    let scroll = (at + 4).saturating_sub(tree_h).min(tree.len().saturating_sub(tree_h));
    let mut shown: Vec<Line> = tree.into_iter().skip(scroll).take(tree_h).collect();
    while shown.len() < tree_h {
        shown.push(Line::default());
    }
    shown.push(Line::from(Span::styled("─".repeat(w), Style::default().fg(Color::Rgb(50, 50, 70)))));
    shown.extend(foot);
    f.render_widget(Paragraph::new(shown), Rect::new(inner.x + 1, inner.y, inner.width.saturating_sub(1), inner.height));
}

impl App {
    /// `F` on a dwarf's sheet or Life page: their family tree.
    pub fn open_family(&mut self, id: u32) {
        let at = flat(&self.game, id).iter().position(|(_, k)| k.who == Who::Dwarf(id)).unwrap_or(0);
        self.mode = Mode::Family { id, sel: at };
    }

    /// The keys on the family tree.
    pub fn key_family_tree(&mut self, k: KeyEvent, id: u32, sel: usize) {
        let all = flat(&self.game, id);
        if all.is_empty() {
            self.mode = Mode::Normal;
            return;
        }
        let sel = sel.min(all.len() - 1);
        let row = all[sel].0;
        // Up and down keep to about the same column.
        let col = sel - all.iter().position(|x| x.0 == row).unwrap_or(sel);
        let step = |to: usize| {
            let start = all.iter().position(|x| x.0 == to).unwrap_or(0);
            let len = all.iter().filter(|x| x.0 == to).count();
            start + col.min(len.saturating_sub(1))
        };
        let rows = all.last().map_or(0, |x| x.0);
        match k.code {
            KeyCode::Esc => self.mode = Mode::Unit { id, sel: 0, tab: super::units::TAB_LIFE },
            KeyCode::Left => self.mode = Mode::Family { id, sel: sel.saturating_sub(1) },
            KeyCode::Right | KeyCode::Tab => self.mode = Mode::Family { id, sel: (sel + 1).min(all.len() - 1) },
            KeyCode::Up if row > 0 => self.mode = Mode::Family { id, sel: step(row - 1) },
            KeyCode::Down if row < rows => self.mode = Mode::Family { id, sel: step(row + 1) },
            KeyCode::Enter => match all[sel].1.who {
                Who::Dwarf(o) => self.mode = Mode::Unit { id: o, sel: 0, tab: super::units::TAB_LIFE },
                Who::Beast(c) => {
                    if let Some(p) = self.game.creatures.get(&c).map(|c| c.pos) {
                        self.cursor = p;
                        self.mode = Mode::Look { sel: 0 };
                    }
                }
            },
            KeyCode::Char('t') => {
                if let Who::Dwarf(o) = all[sel].1.who {
                    self.open_family(o);
                }
            }
            KeyCode::Char('c') => {
                let p = match all[sel].1.who {
                    Who::Dwarf(o) => self.game.dwarves.iter().find(|d| d.id == o && (d.alive || d.ghost)).map(|d| d.pos),
                    Who::Beast(c) => self.game.creatures.get(&c).map(|c| c.pos),
                };
                match p {
                    Some(p) => {
                        self.cursor = p;
                        self.free_view = false;
                        self.mode = Mode::Normal;
                    }
                    None => self.flash = Some("Not to be found in the fortress.".into()),
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn a_family_tree_shows_parents_spouse_and_children() {
        let mut g = Game::new(5);
        let ids: Vec<u32> = g.dwarves.iter().take(5).map(|d| d.id).collect();
        let (ma, pa, me, wife, kid) = (ids[0], ids[1], ids[2], ids[3], ids[4]);
        for d in g.dwarves.iter_mut() {
            d.relations.clear();
            d.spouse = None;
            d.parents = None;
        }
        let at = |g: &mut Game, id: u32| g.dwarves.iter().position(|d| d.id == id).unwrap();
        let i = at(&mut g, me);
        g.dwarves[i].parents = Some((ma, pa));
        g.dwarves[i].female = Some(false);
        g.dwarves[i].spouse = Some(wife);
        g.dwarves[i].adjust_regard(pa, -70);
        let i = at(&mut g, wife);
        g.dwarves[i].spouse = Some(me);
        g.dwarves[i].female = Some(true);
        g.dwarves[i].adjust_regard(me, 90);
        let i = at(&mut g, kid);
        g.dwarves[i].parents = Some((wife, me));
        g.dwarves[i].female = Some(false);
        let i = at(&mut g, ma);
        g.dwarves[i].female = Some(true);
        // The mother died and lies unburied.
        let mut d = std::mem::take(&mut g.dwarves[i]);
        g.kill_for_test(&mut d, "was struck down");
        g.dwarves[i] = d;
        let names: Vec<String> = ids.iter().map(|&id| g.dwarves.iter().find(|d| d.id == id).unwrap().called()).collect();
        let mut app = App::with_game_for_tests(g);
        // `F` on the Life tab opens it, at the dwarf themself.
        app.mode = Mode::Unit { id: me, sel: 0, tab: super::super::units::TAB_LIFE };
        app.on_key(KeyEvent::from(KeyCode::Char('F')));
        let Mode::Family { id, sel } = app.mode.clone() else { panic!("{:?}", app.mode) };
        assert_eq!(id, me);
        assert_eq!(flat(&app.game, me)[sel].1.who, Who::Dwarf(me));
        let s = dump(&mut app, 160, 50);
        println!("{}", s);
        for t in ["Parents", "Spouses and siblings", "Children", "mother", "father", "wife", "son", "lies unburied", "♥", "✗"] {
            assert!(s.contains(t), "no {:?} in\n{}", t, s);
        }
        for n in &names {
            let short: String = n.chars().take(CARD_W - 3).collect();
            assert!(s.contains(&short), "no {} in\n{}", n, s);
        }
        // Up to the parents, Enter opens the mother's sheet.
        app.on_key(KeyEvent::from(KeyCode::Up));
        let Mode::Family { sel, .. } = app.mode.clone() else { panic!() };
        assert_eq!(flat(&app.game, me)[sel].0, 0, "on the parents' row");
        let mother = flat(&app.game, me).iter().position(|(_, k)| k.who == Who::Dwarf(ma)).unwrap();
        app.mode = Mode::Family { id: me, sel: mother };
        let s = dump(&mut app, 160, 50);
        assert!(s.contains("was struck down"), "the death scene below the tree:\n{}", s);
        app.on_key(KeyEvent::from(KeyCode::Enter));
        assert_eq!(app.mode, Mode::Unit { id: ma, sel: 0, tab: super::super::units::TAB_LIFE });
        // `t` redraws the tree around the son: his grandparents.
        app.open_family(me);
        let Mode::Family { sel, .. } = app.mode.clone() else { panic!() };
        let son = flat(&app.game, me).iter().position(|(_, k)| k.who == Who::Dwarf(kid)).unwrap();
        app.key_family_tree(KeyEvent::from(KeyCode::Char('t')), me, son);
        let Mode::Family { id, .. } = app.mode.clone() else { panic!() };
        assert_eq!(id, kid);
        assert!(app.game.family_tree(kid)[0].title == "Grandparents");
        let _ = sel;
        // Esc goes back to the sheet; the Life page opens it too.
        app.on_key(KeyEvent::from(KeyCode::Esc));
        assert_eq!(app.mode, Mode::Unit { id: kid, sel: 0, tab: super::super::units::TAB_LIFE });
        app.mode = Mode::Life { id: me, scroll: 0 };
        app.on_key(KeyEvent::from(KeyCode::Char('F')));
        assert!(matches!(app.mode, Mode::Family { id, .. } if id == me));
        // It draws at a small size too.
        let s = dump(&mut app, 80, 24);
        assert!(s.contains("The family of"), "{}", s);
    }
}
