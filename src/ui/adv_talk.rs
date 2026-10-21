//! A conversation (G03 A7; the words are `game/adv/talk.rs`): who the
//! hero is talking to, what has been said, newest at the bottom, and the
//! topics to ask, DF-like, by letter.

use super::{plain, popup, wrap_words, BLACK, KEY, MUTED, TEXT};
use crate::app::adventure::AdvScreen;
use crate::game::adv::talk::{Said, Topic};
use crate::overworld::Race;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::Frame;

const GOLD: Color = Color::Rgb(255, 215, 90);

fn race_word(r: Race) -> &'static str {
    match r {
        Race::Dwarf => "dwarf",
        Race::Human => "human",
        Race::Elf => "elf",
        Race::Goblin => "goblin",
    }
}

/// Their words, a shade of their people's colour.
fn voice_color(r: Race) -> Color {
    match r {
        Race::Dwarf => Color::Rgb(240, 215, 160),
        Race::Human => Color::Rgb(225, 225, 230),
        Race::Elf => Color::Rgb(170, 235, 185),
        Race::Goblin => Color::Rgb(235, 150, 140),
    }
}

pub(super) fn draw(f: &mut Frame, area: Rect, s: &AdvScreen, sel: usize) {
    let Some(chat) = s.chat.as_ref() else { return };
    let p = &chat.who;
    let w = 84.min(area.width.saturating_sub(2)).max(30);
    let h = area.height.saturating_sub(4).min(44).max(12);
    let text_w = w.saturating_sub(4) as usize;
    let topics = s.adv.topics(chat);
    // Header: who.
    let civ = p.civ.and_then(|c| s.adv.ow().civs.get(c as usize)).map(|c| format!(" of the {}", c.name.trim_start_matches("The ").trim_start_matches("the "))).unwrap_or_default();
    let mut v: Vec<Line<'static>> = vec![
        Line::from(vec![
            Span::styled(" ☺ ", Style::default().fg(p.race.color())),
            Span::styled(p.name.clone(), Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
            Span::styled(format!(", {}", p.role), Style::default().fg(TEXT)),
            Span::styled(format!("  · {}{}", race_word(p.race), civ), Style::default().fg(MUTED)),
        ]),
        Line::from(Span::styled(format!(" {}", "─".repeat(text_w + 1)), Style::default().fg(MUTED))),
    ];
    // What has been said, as much of the end of it as fits.
    let mut said: Vec<Line<'static>> = Vec::new();
    for (who, text) in &chat.lines {
        let (lead, body, st) = match who {
            Said::Hero => ("You: ".to_string(), text.clone(), Style::default().fg(KEY)),
            Said::Them => (format!("{}: ", p.first()), format!("\u{201c}{}\u{201d}", text), Style::default().fg(voice_color(p.race))),
            Said::Aside => (String::new(), text.clone(), Style::default().fg(MUTED).add_modifier(Modifier::ITALIC)),
            Said::Marked => ("! ".to_string(), text.clone(), Style::default().fg(GOLD).add_modifier(Modifier::BOLD)),
        };
        let pad = lead.chars().count();
        if *who == Said::Hero && !said.is_empty() {
            said.push(Line::default());
        }
        for (i, l) in wrap_words(&body, text_w.saturating_sub(pad).max(10)).into_iter().enumerate() {
            let head = if i == 0 { lead.clone() } else { " ".repeat(pad) };
            let head_st = if *who == Said::Them { Style::default().fg(p.race.color()) } else { st };
            said.push(Line::from(vec![Span::raw(" "), Span::styled(head, head_st), Span::styled(l, st)]));
        }
    }
    let menu_rows = topics.len() + 3;
    let room = (h as usize).saturating_sub(2 + v.len() + menu_rows);
    let from = said.len().saturating_sub(room);
    let shown = said.len() - from;
    v.extend(said.into_iter().skip(from));
    for _ in shown..room {
        v.push(Line::default());
    }
    // The topics.
    v.push(Line::from(Span::styled(format!(" {}", "─".repeat(text_w + 1)), Style::default().fg(MUTED))));
    for (i, t) in topics.iter().enumerate() {
        let asked = chat.asked.contains(t) && *t != Topic::Farewell;
        let st = if i == sel {
            Style::default().fg(BLACK).bg(KEY)
        } else if asked {
            Style::default().fg(MUTED)
        } else {
            Style::default().fg(TEXT)
        };
        v.push(Line::from(vec![
            Span::styled(format!(" {} ", (b'a' + i as u8) as char), Style::default().fg(KEY)),
            Span::styled(format!(" {:<44}", t.ask()), st),
        ]));
    }
    v.push(plain(format!(" a-{} ask · ↑↓ Enter · Esc walk away", (b'a' + topics.len().saturating_sub(1) as u8) as char), MUTED));
    popup(f, area, w, h, " Conversation ", v);
}
