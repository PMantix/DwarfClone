//! The Life page of a dwarf's sheet (`l` on the sheet): a small portrait
//! drawn from their looks (beard, hair, scars, lost limbs, what they wear
//! and carry) beside who they are, and their life story below, told from
//! their thoughts, family, wounds, deeds and the chronicle
//! (`game/lifestory.rs`).

use super::{add_target, plain, rgb, wrap_words, Target, KEY, MUTED, TEXT};
use crate::app::{App, Mode};
use crate::body::Part;
use crate::dwarf::Dwarf;
use crate::game::lifestory::{face_scar, looks};
use crate::game::moods::rank_of;
use crate::game::Game;
use crate::items::ItemKind;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

/// The portrait's size.
pub const PORTRAIT_W: usize = 13;
#[cfg(test)]
pub const PORTRAIT_H: usize = 9;

type Seg = (String, (u8, u8, u8));

/// A dwarf's portrait: nine rows of thirteen columns, each a run of
/// coloured pieces.
pub fn portrait(g: &Game, d: &Dwarf) -> Vec<Vec<Seg>> {
    let l = looks(d);
    let (skin, hair) = (l.skin, l.hair_color);
    let scar_red = (220, 70, 70);
    let gear = |k: ItemKind| d.armor.iter().filter_map(|id| g.items.get(id)).find(|it| it.kind == k).map(|it| crate::game::reel::rgb_of(it.mat.color()));
    let s = |t: &str, c: (u8, u8, u8)| (t.to_string(), c);
    let mut rows: Vec<Vec<Seg>> = Vec::new();
    // The crown: a helm, or the hair.
    rows.push(match gear(ItemKind::Helm) {
        Some(m) => vec![s("   [=====]   ", m)],
        None => vec![match l.hair {
            0 => s("    .---.    ", skin),
            1 => s("   ~~~~~~~   ", hair),
            2 => s("   .-'''-.   ", hair),
            _ => s("     (@)     ", hair),
        }],
    });
    // Eyes: open, shut (asleep or out cold) or crossed out (dead); a scar
    // across the left one.
    let eye = if !d.alive {
        "x"
    } else if d.asleep || d.body.unconscious() {
        "-"
    } else {
        "o"
    };
    let mut r = vec![s("  ( ", skin), s(eye, (240, 240, 240))];
    if face_scar(d) {
        r.push(s("/", scar_red));
        r.push(s("  ", skin));
    } else {
        r.push(s("   ", skin));
    }
    r.push(s(eye, (240, 240, 240)));
    r.push(s(" )  ", skin));
    rows.push(r);
    rows.push(vec![s("  (   >   )  ", skin)]);
    // The moustache, and the mouth by mood.
    let mouth = match rank_of(d) {
        0 | 1 => "v",
        2 => "-",
        3..=5 => "^",
        6 => "O",
        _ => "_",
    };
    rows.push(vec![s("    ~~", hair), s(mouth, (230, 120, 110)), s("~~    ", hair)]);
    // The beard.
    let (b1, b2) = match l.beard {
        0 => ("|||||", "    |||||    "),
        1 => ("\\/\\/\\", "    \\/ \\/    "),
        2 => ("|||||", "    // \\\\    "),
        _ => ("'''''", "             "),
    };
    rows.push(vec![s("   )", skin), s(b1, hair), s("(   ", skin)]);
    rows.push(vec![s(b2, hair)]);
    // Arms and body: a breastplate shows, a lost arm doesn't.
    let lost = |p: Part| d.body.lost[p.idx()];
    let cloth = (150, 130, 110);
    let chest = gear(ItemKind::Breastplate);
    rows.push(vec![
        s("  ", cloth),
        s(if lost(Part::LeftArm) { " " } else { "/" }, cloth),
        s("|", cloth),
        match chest {
            Some(m) => s("=====", m),
            None => s("     ", cloth),
        },
        s("|", cloth),
        s(if lost(Part::RightArm) { " " } else { "\\" }, cloth),
        s("  ", cloth),
    ]);
    // Hands: what they hold in the left, a shield in the right.
    let held = d.held.and_then(|id| g.items.get(&id)).map(|it| {
        let (c, col) = it.glyph();
        (c.to_string(), crate::game::reel::rgb_of(col))
    });
    let shield = gear(ItemKind::Shield);
    rows.push(vec![
        s(" ", cloth),
        match held {
            Some(h) if !lost(Part::LeftArm) => h,
            _ => s(" ", cloth),
        },
        s(" |     | ", cloth),
        match shield {
            Some(m) if !lost(Part::RightArm) => s("]", m),
            _ => s(" ", cloth),
        },
        s(" ", cloth),
    ]);
    // Legs, and a crutch for a lost one.
    let leg = |p: Part| {
        if !lost(p) {
            s("|", cloth)
        } else if d.body.crutch {
            s("¦", (160, 110, 60))
        } else {
            s(" ", cloth)
        }
    };
    rows.push(vec![s("    ", cloth), leg(Part::LeftLeg), s("   ", cloth), leg(Part::RightLeg), s("    ", cloth)]);
    rows
}

/// The portrait as plain text (tests).
#[cfg(test)]
pub fn portrait_text(g: &Game, d: &Dwarf) -> Vec<String> {
    portrait(g, d).iter().map(|r| r.iter().map(|s| s.0.as_str()).collect()).collect()
}

pub(super) fn draw(f: &mut Frame, area: Rect, g: &Game, id: u32, scroll: usize) {
    let Some(d) = g.dwarves.iter().find(|d| d.id == id) else { return };
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(MUTED))
        .style(Style::default().bg(Color::Rgb(12, 12, 20)))
        .title(Span::styled(format!(" The life of {} ", d.name), Style::default().fg(KEY)));
    let inner = block.inner(area);
    f.render_widget(block, area);
    add_target(area, Target::Block);
    let w = inner.width.saturating_sub(2) as usize;
    let mut v: Vec<Line> = Vec::new();
    // The portrait, and beside it who they are.
    let (mood, mc) = if d.alive { d.mood() } else { ("Dead", (200, 200, 210)) };
    let mut facts = vec![
        Line::from(Span::styled(d.called(), Style::default().fg(rgb(d.color)).add_modifier(Modifier::BOLD))),
        plain(d.profession().to_string(), MUTED),
        Line::from(Span::styled(mood, Style::default().fg(rgb(mc)))),
    ];
    if let Some(age) = d.age {
        facts.push(plain(format!("Age {}", age), MUTED));
    }
    if let Some(deed) = &d.deed {
        facts.push(plain(format!("Who {}", deed), Color::Rgb(255, 215, 60)));
    }
    let side_w = w.saturating_sub(PORTRAIT_W + 2);
    let facts: Vec<Line> = facts.into_iter().map(|l| if l.width() > side_w { plain(l.spans.iter().map(|s| s.content.as_ref()).collect::<String>().chars().take(side_w).collect(), MUTED) } else { l }).collect();
    let pic = portrait(g, d);
    for (i, row) in pic.into_iter().enumerate() {
        let mut spans = vec![Span::raw(" ")];
        spans.extend(row.into_iter().map(|(t, c)| Span::styled(t, Style::default().fg(rgb(c)))));
        if inner.width as usize >= PORTRAIT_W + 12 {
            spans.push(Span::raw("  "));
            if let Some(l) = facts.get(i) {
                spans.extend(l.spans.iter().cloned());
            }
        }
        v.push(Line::from(spans));
    }
    v.push(Line::default());
    for para in g.life_story(d) {
        for l in wrap_words(&para, w.max(10)) {
            v.push(plain(format!(" {}", l), TEXT));
        }
        v.push(Line::default());
    }
    let h = inner.height.saturating_sub(1) as usize;
    let scroll = scroll.min(v.len().saturating_sub(h));
    let shown: Vec<Line> = v.into_iter().skip(scroll).take(h).collect();
    f.render_widget(Paragraph::new(shown), Rect::new(inner.x, inner.y, inner.width, inner.height.saturating_sub(1)));
    let hint = " ↑↓ read on · l/Esc the sheet · c go to them";
    f.render_widget(Paragraph::new(Span::styled(hint, Style::default().fg(MUTED))), Rect::new(inner.x, inner.y + inner.height.saturating_sub(1), inner.width, 1));
}

impl App {
    /// The keys on the Life page.
    pub fn key_life(&mut self, k: KeyEvent, id: u32, scroll: usize) {
        match k.code {
            KeyCode::Esc | KeyCode::Char('l') => self.mode = Mode::Unit { id, sel: 0 },
            KeyCode::Up => self.mode = Mode::Life { id, scroll: scroll.saturating_sub(1) },
            KeyCode::Down => self.mode = Mode::Life { id, scroll: scroll + 1 },
            KeyCode::PageUp => self.mode = Mode::Life { id, scroll: scroll.saturating_sub(10) },
            KeyCode::PageDown => self.mode = Mode::Life { id, scroll: scroll + 10 },
            KeyCode::Char('c') => {
                if let Some(d) = self.game.dwarves.iter().find(|d| d.id == id) {
                    self.cursor = d.pos;
                    self.free_view = false;
                }
                self.mode = Mode::Normal;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::draw as draw_all;
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn dump(app: &mut App, w: u16, h: u16) -> String {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| draw_all(f, app)).unwrap();
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
    fn the_portrait_shows_scars_lost_limbs_and_gear() {
        let mut g = Game::new(5);
        let plain_pic = portrait_text(&g, &g.dwarves[0]);
        assert_eq!(plain_pic.len(), PORTRAIT_H);
        assert!(plain_pic.iter().all(|r| r.chars().count() == PORTRAIT_W), "{:?}", plain_pic);
        let d = &mut g.dwarves[0];
        d.body.lost[Part::LeftArm.idx()] = true;
        d.body.lost[Part::RightLeg.idx()] = true;
        d.med.push((10, "struck in the head".into()));
        let pic = portrait_text(&g, &g.dwarves[0]);
        println!("{}", pic.join("\n"));
        assert!(pic[1].contains('/'), "a scar over the eye");
        assert_ne!(pic[6], plain_pic[6], "a lost arm shows");
        assert_ne!(pic[8], plain_pic[8], "a lost leg shows");
        // A helm.
        let p = g.dwarves[0].pos;
        let helm = g.spawn_item(ItemKind::Helm, crate::material::Mat::Iron, p);
        g.dwarves[0].armor.push(helm);
        assert!(portrait_text(&g, &g.dwarves[0])[0].contains("[=====]"));
    }

    #[test]
    fn the_life_page_opens_from_the_sheet_at_any_size() {
        let mut app = App::with_game_for_tests(Game::new(5));
        let id = app.game.dwarves[0].id;
        app.game.dwarves[0].think("admired a fine engraving", -10, 100);
        app.mode = Mode::Unit { id, sel: 0 };
        app.on_key(KeyEvent::from(KeyCode::Char('l')));
        assert_eq!(app.mode, Mode::Life { id, scroll: 0 });
        let name = app.game.dwarves[0].name.clone();
        for (w, h) in [(80, 24), (200, 60)] {
            let s = dump(&mut app, w, h);
            println!("{}", s);
            assert!(s.contains(&format!("The life of {}", name)), "{}x{}", w, h);
            assert!(s.contains("( o   o )") || s.contains("( o/  o )"), "the portrait at {}x{}", w, h);
            assert!(s.contains("admired a fine engraving"), "the story at {}x{}", w, h);
        }
        app.on_key(KeyEvent::from(KeyCode::Esc));
        assert_eq!(app.mode, Mode::Unit { id, sel: 0 });
    }
}
