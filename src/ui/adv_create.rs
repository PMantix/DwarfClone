//! The character maker (A1): the start of a saga. On the left the choices
//! (people, calling, name, home, skills); on the right the hero as they
//! stand, a portrait in the Life page's manner, a line of their past drawn
//! from the world's legends, their kit, and where home lies on the map.

use super::life::portrait;
use super::{backdrop, cap_first, plain, rgb, wrap_words, BLACK, KEY, MUTED, TEXT};
use crate::app::adventure::AdvScreen;
use crate::dwarf::skill_rank;
use crate::game::adv::create::{race_noun, Maker, ROWS};
use crate::game::adv::Hero;
use crate::game::lifestory::looks;
use crate::game::Game;
use crate::overworld::{Overworld, Race};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

const GOLD: Color = Color::Rgb(240, 190, 70);
const LORE: Color = Color::Rgb(190, 170, 230);
const PANEL: Color = Color::Rgb(14, 12, 20);

type Seg = (String, (u8, u8, u8));

pub(super) fn draw(f: &mut Frame, area: Rect, s: &AdvScreen) {
    backdrop(f, area);
    let Some(m) = s.maker.as_deref() else { return };
    let ow = &s.adv.ow;
    let hero = m.hero(ow);
    let w = area.width.min(118);
    let h = area.height.min(40);
    let r = Rect::new(area.x + (area.width - w) / 2, area.y + (area.height - h) / 2, w, h);
    f.render_widget(Clear, r);
    let title = format!(" ✦ A saga begins in {} ✦ ", ow.name);
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(GOLD))
            .title(Span::styled(title, Style::default().fg(GOLD).add_modifier(Modifier::BOLD)))
            .style(Style::default().bg(PANEL)),
        r,
    );
    let inner = Rect::new(r.x + 1, r.y + 1, r.width.saturating_sub(2), r.height.saturating_sub(2));
    let left_w = (inner.width / 2).clamp(36, 50).min(inner.width);
    let left = Rect::new(inner.x, inner.y, left_w, inner.height);
    let right = Rect::new(inner.x + left_w + 1, inner.y, inner.width.saturating_sub(left_w + 1), inner.height);
    draw_choices(f, left, s, m, ow);
    if right.width >= 20 {
        draw_hero(f, right, m, &hero, ow);
    }
}

/// The choices, one row each, and the keys.
fn draw_choices(f: &mut Frame, r: Rect, s: &AdvScreen, m: &Maker, ow: &Overworld) {
    let race = m.race();
    let hi = |on: bool| if on { Style::default().fg(BLACK).bg(KEY) } else { Style::default().fg(TEXT) };
    let arrow = |on: bool| Span::styled(if on { " ▶ " } else { "   " }, Style::default().fg(KEY));
    let label = |t: &str| Span::styled(format!("{:<9}", t), Style::default().fg(MUTED));
    let home = m.home_id().and_then(|id| ow.sites.iter().find(|s| s.id == id));
    let mut v: Vec<Line> = vec![Line::default()];
    let values = [
        (format!("◂ {} ▸", cap_first(race_noun(race))), Some(race.color())),
        (format!("◂ {} ▸", cap_first(m.calling().name())), None),
        (if m.naming { format!("{}▏", m.name) } else { m.name.clone() }, None),
        (
            match home {
                Some(h) => format!("◂ {} ({}/{}) ▸", h.name, m.home + 1, m.homes.len()),
                None => "nowhere to set out from".into(),
            },
            None,
        ),
    ];
    for (i, (val, col)) in values.into_iter().enumerate() {
        let on = m.sel == i;
        let mut st = hi(on);
        if let (false, Some(c)) = (on, col) {
            st = st.fg(c);
        }
        v.push(Line::from(vec![arrow(on), label(ROWS[i]), Span::styled(val, st)]));
    }
    if let Some(h) = home {
        let civ = h.civ.and_then(|c| ow.civs.get(c as usize)).map_or(String::new(), |c| format!(" of the {}", c.name));
        v.push(plain(format!("            a {}{}", h.kind.name(), civ), MUTED));
    }
    v.push(Line::default());
    let left = m.points_left();
    v.push(Line::from(vec![
        Span::styled("   Skills", Style::default().fg(Color::Rgb(140, 200, 255)).add_modifier(Modifier::BOLD)),
        Span::styled("     points left ", Style::default().fg(MUTED)),
        Span::styled(format!("{}", left), Style::default().fg(if left > 0 { Color::Rgb(140, 230, 140) } else { MUTED }).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" of {}", m.calling().budget()), Style::default().fg(MUTED)),
    ]));
    for (i, (sk, lv)) in m.skills.iter().enumerate() {
        let on = m.sel == ROWS.len() + i;
        let rank = if *lv == 0 { "—".to_string() } else { skill_rank(lv * 400).to_string() };
        let pips: String = (1..=5).map(|p| if p <= *lv { '●' } else { '·' }).collect();
        v.push(Line::from(vec![
            arrow(on),
            Span::styled(format!("{:<13}", sk.name()), Style::default().fg(TEXT)),
            Span::styled(format!("{:<11}", rank), hi(on)),
            Span::styled(format!(" {}", pips), Style::default().fg(GOLD)),
        ]));
    }
    v.push(Line::default());
    let on = m.sel == m.rows() - 1;
    v.push(Line::from(vec![
        arrow(on),
        Span::styled(" Set out at dawn ", if on { Style::default().fg(BLACK).bg(GOLD).add_modifier(Modifier::BOLD) } else { Style::default().fg(GOLD) }),
    ]));
    v.push(Line::default());
    for l in wrap_words(m.calling().blurb(), r.width.saturating_sub(4) as usize) {
        v.push(plain(format!("   {}", l), MUTED));
    }
    v.push(Line::default());
    let keys = if m.naming { "type a name · Enter done" } else { "↑↓ row · ←→ [ ] change · n name · r reroll · Enter set out · Esc back" };
    for l in wrap_words(keys, r.width.saturating_sub(4) as usize) {
        v.push(plain(format!("   {}", l), KEY));
    }
    if let Some(fl) = &s.flash {
        v.push(plain(format!("   {}", fl), Color::Rgb(255, 150, 90)));
    }
    f.render_widget(Paragraph::new(v), r);
}

/// The hero as they stand: portrait, name, past, kit, and home on the map.
fn draw_hero(f: &mut Frame, r: Rect, m: &Maker, hero: &Hero, ow: &Overworld) {
    let d = &hero.dwarf;
    let race = hero.race;
    let mut lines: Vec<Line> = vec![Line::default()];
    // The portrait, framed, with who they are beside it.
    let rows = hero_portrait(hero);
    let civ = hero.civ.and_then(|c| ow.civs.get(c as usize)).map_or(String::new(), |c| format!("of the {}", c.name));
    let beside = vec![
        Line::from(Span::styled(d.name.clone(), Style::default().fg(race.color()).add_modifier(Modifier::BOLD))),
        plain(format!("{} {}", cap_first(race_noun(race)), hero.calling.name()), TEXT),
        plain(civ, MUTED),
        plain(format!("{} years of age, {}", d.age.unwrap_or(0), if d.female == Some(true) { "she" } else { "he" }), MUTED),
        Line::default(),
        plain(format!("{} coins in the purse", hero.coins), Color::Rgb(230, 200, 90)),
    ];
    lines.push(Line::from(Span::styled("  ╭─────────────╮", Style::default().fg(GOLD))));
    for (i, row) in rows.iter().enumerate() {
        let mut spans = vec![Span::styled("  │", Style::default().fg(GOLD))];
        spans.extend(row.iter().map(|(t, c)| Span::styled(t.clone(), Style::default().fg(rgb(*c)))));
        spans.push(Span::styled("│  ", Style::default().fg(GOLD)));
        if let Some(b) = beside.get(i) {
            spans.extend(b.spans.iter().cloned());
        }
        lines.push(Line::from(spans));
    }
    lines.push(Line::from(Span::styled("  ╰─────────────╯", Style::default().fg(GOLD))));
    // A line of their past.
    lines.push(Line::default());
    let width = r.width.saturating_sub(4) as usize;
    for l in wrap_words(&m.backstory(ow), width) {
        lines.push(Line::from(Span::styled(format!("  {}", l), Style::default().fg(LORE).add_modifier(Modifier::ITALIC))));
    }
    // What they carry.
    lines.push(Line::default());
    lines.push(Line::from(Span::styled("  Carries", Style::default().fg(Color::Rgb(140, 200, 255)).add_modifier(Modifier::BOLD))));
    for it in &hero.pack {
        let how = if d.held == Some(it.id) {
            "in hand"
        } else if d.armor.contains(&it.id) || d.clothes.contains(&it.id) {
            "worn"
        } else {
            "in the pack"
        };
        let (g, c) = it.glyph();
        lines.push(Line::from(vec![
            Span::styled(format!("   {} ", g), Style::default().fg(c)),
            Span::styled(it.name(), Style::default().fg(TEXT)),
            Span::styled(format!("  {}", how), Style::default().fg(MUTED)),
        ]));
    }
    let used = lines.len() as u16;
    f.render_widget(Paragraph::new(lines), r);
    // Home, on the world map.
    let map_h = r.height.saturating_sub(used + 2).min(11);
    if map_h >= 5 {
        let mr = Rect::new(r.x + 2, r.y + used + 1, r.width.saturating_sub(4).min(33), map_h);
        draw_home_map(f, mr, m, ow);
    }
}

/// The land around home, the world map's way.
fn draw_home_map(f: &mut Frame, r: Rect, m: &Maker, ow: &Overworld) {
    let Some(home) = m.home_id().and_then(|id| ow.sites.iter().find(|s| s.id == id)) else { return };
    let title = format!(" {} ", home.name);
    let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(MUTED)).title(Span::styled(title, Style::default().fg(KEY)));
    let inner = block.inner(r);
    f.render_widget(block, r);
    let (vw, vh) = (inner.width as i32, inner.height as i32);
    let (ox, oy) = (home.x - vw / 2, home.y - vh / 2);
    let buf = f.buffer_mut();
    for sy in 0..vh {
        for sx in 0..vw {
            let (x, y) = (ox + sx, oy + sy);
            let cell = buf.get_mut(inner.x + sx as u16, inner.y + sy as u16);
            if x < 0 || y < 0 || x >= ow.w || y >= ow.h {
                cell.set_char(' ').set_bg(BLACK);
                continue;
            }
            let (mut ch, mut col) = ow.cell(x, y).biome.glyph();
            if let Some(site) = ow.site_at(x, y) {
                ch = if site.ruined { '×' } else { site.kind.glyph() };
                col = site.civ.and_then(|c| ow.civs.get(c as usize)).map_or(MUTED, |c| c.race.color());
            }
            if ow.forts.iter().any(|fm| fm.site == (x, y) && fm.fell.is_some()) {
                ch = '×';
                col = Color::Rgb(220, 30, 50);
            }
            cell.set_char(ch).set_fg(col).set_bg(BLACK);
            if (x, y) == (home.x, home.y) {
                cell.set_fg(BLACK).set_bg(KEY);
            }
        }
    }
}

/// The Life page's portrait, worn by the hero's people: the beard is a
/// dwarf's, elves and goblins have the ears, goblins the green skin and the
/// teeth.
pub fn hero_portrait(h: &Hero) -> Vec<Vec<Seg>> {
    let mut g = Game::placeholder(Overworld::empty());
    for it in &h.pack {
        g.items.insert(it.id, it.clone());
    }
    let mut rows = portrait(&g, &h.dwarf);
    if h.race == Race::Dwarf || rows.len() < 6 {
        return rows;
    }
    let skin = looks(&h.dwarf).skin;
    let s = |t: &str, c: (u8, u8, u8)| (t.to_string(), c);
    // A bare chin: the mouth alone, no moustache or beard.
    let mouth = rows[3].iter().find(|seg| seg.0.trim().len() == 1).cloned().unwrap_or_else(|| s("-", (230, 120, 110)));
    rows[3] = vec![s("      ", skin), mouth, s("      ", skin)];
    rows[4] = match h.race {
        Race::Goblin => vec![s("    \\", skin), s("vvv", (240, 240, 220)), s("/    ", skin)],
        _ => vec![s("    `---'    ", skin)],
    };
    rows[5] = vec![s("      |      ", skin)];
    // Ears.
    let ears = match h.race {
        Race::Elf => Some((" <( ", " )> ")),
        Race::Goblin => Some((" >( ", " )< ")),
        _ => None,
    };
    if let Some((a, b)) = ears {
        let n = rows[1].len();
        rows[1][0].0 = a.to_string();
        rows[1][n - 1].0 = b.to_string();
    }
    if h.race == Race::Goblin {
        let green = (110, 165, 80);
        for row in rows.iter_mut().take(6) {
            for seg in row.iter_mut() {
                if seg.1 == skin {
                    seg.1 = green;
                }
            }
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, Mode, TITLE_ADVENTURE};
    use crate::game::adv::{View, HERO_ID};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::NONE)
    }

    fn settle(app: &mut App) {
        let t = std::time::Instant::now();
        while app.mode == Mode::Loading && t.elapsed().as_secs() < 120 {
            app.frame();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    fn screen(app: &mut App, w: u16, h: u16) -> String {
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
        term.draw(|f| crate::ui::draw(f, app)).unwrap();
        let b = term.backend().buffer();
        (0..h).map(|y| (0..w).map(|x| b.get(x, y).symbol().to_string()).collect::<String>() + "\n").collect()
    }

    #[test]
    fn make_a_hero_by_keys() {
        let mut app = App::new(5);
        app.save_dir = std::env::temp_dir().join("dwarfclone-no-saves-here");
        app.mode = Mode::Title { sel: TITLE_ADVENTURE };
        app.on_key(key(KeyCode::Enter));
        settle(&mut app);
        assert_eq!(app.mode, Mode::Adventure);
        assert_eq!(app.adv.as_ref().unwrap().view, View::Create);
        let races = app.adv.as_ref().unwrap().maker.as_ref().unwrap().races.clone();
        // Drawn at a small terminal and a large one: the portrait, a past.
        for (w, h) in [(80, 24), (130, 40)] {
            let text = screen(&mut app, w, h);
            assert!(text.contains("A saga begins"), "{}", text);
            assert!(text.contains("Set out"), "{}", text);
            if w > 100 {
                assert!(text.contains("╭─────────────╮"), "{}", text);
                assert!(text.contains("Born") || text.contains("years old") || text.contains("child") || text.contains("Raised") || text.contains("babe"), "{}", text);
            }
        }
        // The last people offered, a hunter, named by hand.
        app.on_key(key(KeyCode::Left));
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Right));
        app.on_key(key(KeyCode::Char('n')));
        let m = app.adv.as_ref().unwrap().maker.as_ref().unwrap();
        assert!(m.naming);
        for _ in 0..40 {
            app.on_key(key(KeyCode::Backspace));
        }
        for c in "Ardent Vel".chars() {
            app.on_key(key(KeyCode::Char(c)));
        }
        app.on_key(key(KeyCode::Enter));
        // Fighting up a level, if the points allow.
        while app.adv.as_ref().unwrap().maker.as_ref().unwrap().sel != ROWS.len() {
            app.on_key(key(KeyCode::Down));
        }
        app.on_key(key(KeyCode::Right));
        let text = screen(&mut app, 130, 40);
        assert!(text.contains("Ardent Vel"), "{}", text);
        app.on_key(key(KeyCode::Enter));
        settle(&mut app);
        let s = app.adv.as_ref().unwrap();
        assert_eq!(s.view, View::Local);
        assert!(s.maker.is_none());
        let a = &s.adv;
        assert_eq!(a.hero.race, *races.last().unwrap());
        assert_eq!(a.hero.calling, crate::game::adv::Calling::Hunter);
        let v = a.visit.as_ref().expect("in a site");
        let d = &v.g.dwarves[v.hero];
        assert_eq!((d.id, d.name.as_str()), (HERO_ID, "Ardent Vel"));
        assert!(d.fight_level() >= 1);
        let site = a.ow().sites.iter().find(|s| Some(s.id) == a.hero.home).unwrap();
        assert!(crate::game::adv::create::home_kinds(a.hero.race).contains(&site.kind));
        // Esc from the maker goes back to the title.
        let mut app = App::new(5);
        app.mode = Mode::Title { sel: TITLE_ADVENTURE };
        app.on_key(key(KeyCode::Enter));
        settle(&mut app);
        app.on_key(key(KeyCode::Esc));
        assert_eq!(app.mode, Mode::Title { sel: TITLE_ADVENTURE });
    }

    #[test]
    fn every_people_has_its_own_face() {
        let ow = Game::new(5).overworld;
        let mut m = Maker::new(&ow, 5);
        let mut faces = Vec::new();
        for r in [Race::Dwarf, Race::Human, Race::Elf, Race::Goblin] {
            m.races = vec![r];
            m.race = 0;
            let mut h = m.hero(&ow);
            h.race = r;
            let rows = hero_portrait(&h);
            for row in &rows {
                assert_eq!(row.iter().map(|s| s.0.chars().count()).sum::<usize>(), 13, "{:?} {:?}", r, row);
            }
            faces.push(rows.iter().map(|row| row.iter().map(|s| s.0.as_str()).collect::<String>()).collect::<Vec<_>>().join("\n"));
        }
        assert!(faces[0].contains("|||||") || faces[0].contains("\\/") || faces[0].contains("'''"), "a dwarf is bearded:\n{}", faces[0]);
        assert!(faces[2].contains("<(") && faces[3].contains(">(") && faces[3].contains("vvv"));
    }
}
