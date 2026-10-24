//! The quest log (`j`, G03 A8) and the company (`c`, A9), as popups over
//! the local view or the travel map. The words are `game/adv/quests.rs`
//! and `company.rs`; the keys `app::adventure::{key_quests, key_company}`.

use super::{plain, popup, wrap_words, BLACK, KEY, MUTED, TEXT};
use crate::app::adventure::AdvScreen;
use crate::game::adv::company::loyalty_word;
use crate::game::adv::QuestState;
use crate::dwarf::Task;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::Frame;

const GOLD: Color = Color::Rgb(255, 215, 90);
const GREEN: Color = Color::Rgb(150, 220, 140);
const RED: Color = Color::Rgb(235, 90, 80);

/// The quest log: what was sworn, to whom, the way there, and what is done.
pub(super) fn draw_quests(f: &mut Frame, area: Rect, s: &AdvScreen) {
    let w = 84.min(area.width.saturating_sub(2)).max(30);
    let text_w = w.saturating_sub(6) as usize;
    let mut v: Vec<Line<'static>> = Vec::new();
    let lines = s.adv.quest_lines();
    if lines.is_empty() {
        v.push(plain(" You are sworn to nothing yet.".into(), MUTED));
        v.push(plain(" Ask the folk of a town \"Is there work?\" (k to talk).".into(), MUTED));
    }
    for (state, what, way) in lines {
        let (mark, c) = match state {
            QuestState::Open => ("◆", GOLD),
            QuestState::Won => ("✓", GREEN),
            QuestState::Done => ("✓", MUTED),
            QuestState::Failed => ("✗", RED),
        };
        let text = if state == QuestState::Done { MUTED } else { TEXT };
        for (i, l) in wrap_words(&what, text_w).into_iter().enumerate() {
            let head = if i == 0 { format!(" {} ", mark) } else { "   ".to_string() };
            v.push(Line::from(vec![Span::styled(head, Style::default().fg(c)), Span::styled(l, Style::default().fg(text).add_modifier(if state == QuestState::Open { Modifier::BOLD } else { Modifier::empty() }))]));
        }
        for l in wrap_words(&way, text_w) {
            v.push(plain(format!("   {}", l), if state == QuestState::Won { GREEN } else { MUTED }));
        }
        v.push(Line::default());
    }
    if s.adv.hero.coins > 0 {
        v.push(plain(format!(" {} coins in the purse", s.adv.hero.coins), Color::Rgb(240, 200, 90)));
    }
    v.push(plain(" j or Esc: back".into(), MUTED));
    let h = (v.len() as u16 + 2).min(area.height.saturating_sub(2)).max(8);
    popup(f, area, w, h, " Quests ", v);
}

/// The company: each companion's body, loyalty and order, and the orders.
pub(super) fn draw_company(f: &mut Frame, area: Rect, s: &AdvScreen, sel: usize) {
    let a = &s.adv;
    let w = 72.min(area.width.saturating_sub(2)).max(30);
    let mut v: Vec<Line<'static>> = Vec::new();
    let band = a.companions();
    if band.is_empty() {
        v.push(plain(" Nobody walks with you.".into(), MUTED));
        v.push(plain(" Ask the folk of a town \"Will you come with me?\" (k).".into(), MUTED));
    }
    for (n, (i, d)) in band.iter().enumerate() {
        let c = &a.company[*i];
        let home = c.home.and_then(|h| a.ow().sites.iter().find(|s| s.id == h)).map_or("the road".to_string(), |s| s.name.clone());
        let st = if n == sel { Style::default().fg(BLACK).bg(KEY) } else { Style::default().fg(TEXT).add_modifier(Modifier::BOLD) };
        v.push(Line::from(vec![Span::styled(" ☺ ", Style::default().fg(c.race.color())), Span::styled(d.name.clone(), st), Span::styled(format!(" of {}", home), Style::default().fg(MUTED))]));
        let b = &d.body;
        let (health, hc) = if !d.alive {
            ("dead".to_string(), RED)
        } else if b.unconscious() {
            ("senseless".to_string(), RED)
        } else if b.badly_hurt() {
            ("badly hurt".to_string(), Color::Rgb(240, 150, 110))
        } else if b.healed() {
            ("unhurt".to_string(), GREEN)
        } else {
            ("wounded".to_string(), Color::Rgb(230, 200, 120))
        };
        let order = match d.task {
            Task::Station { .. } => "waiting",
            Task::Attack { .. } => "attacking",
            _ => "following",
        };
        let weapon = d.held.and_then(|id| a.visit.as_ref().and_then(|v| v.g.items.get(&id)).map(|it| it.name()).or_else(|| c.pack.iter().find(|it| it.id == id).map(|it| it.name()))).unwrap_or_else(|| "bare hands".to_string());
        v.push(Line::from(vec![
            Span::raw("   "),
            Span::styled(health, Style::default().fg(hc)),
            Span::styled(format!(" · {} · {} · ", weapon, order), Style::default().fg(MUTED)),
            Span::styled(loyalty_word(c.loyalty), Style::default().fg(if c.loyalty <= 3 { Color::Rgb(240, 150, 110) } else { GREEN })),
            Span::styled(if d.kills > 0 { format!(" · {} slain", d.kills) } else { String::new() }, Style::default().fg(MUTED)),
        ]));
    }
    if !a.band.lost.is_empty() {
        v.push(Line::default());
        v.push(plain(format!(" Lost: {}", a.band.lost.join(", ")), RED));
    }
    v.push(Line::default());
    if a.visit.is_some() {
        v.push(plain(" ↑↓ choose · f follow · w wait here · a attack · x send home".into(), MUTED));
    } else {
        v.push(plain(" ↑↓ choose · x send home · (orders are for a site)".into(), MUTED));
    }
    v.push(plain(" c or Esc: back".into(), MUTED));
    let h = (v.len() as u16 + 2).min(area.height.saturating_sub(2)).max(8);
    popup(f, area, w, h, " Company ", v);
}
