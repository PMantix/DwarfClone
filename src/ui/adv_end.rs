//! The end of an adventure (A10), in the manner of the fort's game-over
//! page: the hero's saga leads, on its parchment, beside their portrait
//! (eyes crossed out for the dead), how they ended, the line the world
//! keeps of them and the run's numbers; then where to go on in this world.
//! Also the saga so far (`S`), from which a hero in a friendly town may
//! settle down for good.

use super::adv_create::hero_portrait;
use super::saga::{draw_page, rows_needed, Page};
use super::{key_line, plain, rgb, wrap_words, MUTED, TEXT};
use crate::app::adventure::AdvScreen;
use crate::game::adv::fame::{end_hero, writing, Ending};
use crate::game::adv::{Adventure, View};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

const RED: Color = Color::Rgb(220, 70, 60);
const GOLD: Color = Color::Rgb(255, 215, 60);
const NIGHT: Color = Color::Rgb(12, 10, 14);

pub(super) fn draw(f: &mut Frame, area: Rect, s: &AdvScreen) {
    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new("").style(Style::default().bg(NIGHT)), area);
    match (&s.view, &s.adv.fame.end) {
        (View::Dead, Some(e)) => draw_end(f, area, &s.adv, e),
        (View::Dead, None) => draw_bare_end(f, area, &s.adv),
        _ => draw_saga(f, area, s),
    }
}

/// The saga as a page: ended, or the road so far.
fn saga_page(a: &Adventure) -> Option<Page> {
    let s = a.fame.saga.as_ref()?;
    let w = writing(s, a.fame.end.is_some());
    Some(Page { title: w.title, doom: w.doom, open: a.fame.end.is_none(), sub: w.sub, chapters: w.chapters, closing: w.closing, fell: false })
}

fn draw_end(f: &mut Frame, area: Rect, a: &Adventure, e: &Ending) {
    let hue = if e.died { RED } else { GOLD };
    // The facts: the portrait, who, how, where, and the run's numbers.
    let mut v: Vec<Line<'static>> = Vec::new();
    let head = if e.died { format!("☠ {} is dead ☠", e.called) } else { format!("☼ {} wanders no more ☼", e.called) };
    v.push(Line::from(Span::styled(head, Style::default().fg(hue).add_modifier(Modifier::BOLD))));
    v.push(plain(format!("{} of {}, {}", race_word(e), e.home, e.year), MUTED));
    v.push(Line::default());
    let hero = end_hero(e);
    for row in hero_portrait(&hero) {
        let mut spans = vec![Span::raw("  ")];
        spans.extend(row.into_iter().map(|(t, c)| Span::styled(t, Style::default().fg(rgb(c)))));
        v.push(Line::from(spans));
    }
    v.push(Line::default());
    let width = 42usize.min(area.width.saturating_sub(4) as usize).max(20);
    for l in wrap_words(&format!("{} {}.", first(&e.dwarf.name), e.cause), width) {
        v.push(plain(l, hue));
    }
    v.push(Line::default());
    let st = &a.stats;
    let mut nums = vec![format!("{} slain", e.dwarf.kills.max(st.slain))];
    if st.days > 0 {
        nums.push(format!("{} days abroad", st.days));
    }
    if st.cells > 0 {
        nums.push(format!("{} leagues walked", st.cells));
    }
    v.push(plain(nums.join(" · "), TEXT));
    v.push(plain(format!("Renown {} · infamy {} · {} deeds", a.fame.renown, a.fame.infamy, a.fame.deeds.len()), TEXT));
    if let Some(r) = &e.remains {
        if !r.gear.is_empty() {
            v.push(Line::default());
            let gear: Vec<String> = r.gear.iter().take(4).map(|g| g.2.clone()).collect();
            for l in wrap_words(&format!("Where they fell lie their bones, and {}{}.", gear.join(", "), if r.gear.len() > 4 { ", and more" } else { "" }), width) {
                v.push(plain(l, MUTED));
            }
        }
    }
    v.push(Line::default());
    v.push(plain("The legends will say:".into(), MUTED));
    for l in wrap_words(&e.legend, width) {
        v.push(Line::from(Span::styled(l, Style::default().fg(Color::Rgb(200, 185, 150)).add_modifier(Modifier::ITALIC))));
    }
    if !e.eulogy.is_empty() {
        v.push(Line::default());
        for l in wrap_words(&e.eulogy, width) {
            v.push(plain(l, MUTED));
        }
    }
    v.push(Line::default());
    v.push(key_line("n", "Found a fort in this world, which remembers them", false));
    v.push(key_line("a", "Another adventurer in this world", false));
    v.push(key_line("t", "Return to the title", false));
    // Wide: the saga beside the facts; narrow: the saga above them.
    let page = saga_page(a);
    let side = (width + 4) as u16;
    if area.width >= side + 50 {
        let left = Rect::new(area.x + 1, area.y + 1, side, area.height.saturating_sub(2));
        f.render_widget(Paragraph::new(v).block(Block::default().borders(Borders::RIGHT).border_style(Style::default().fg(MUTED))).style(Style::default().bg(NIGHT)), left);
        if let Some(p) = page {
            let rw = (area.width - side - 3).min(80);
            let h = rows_needed(&p, rw).min(area.height.saturating_sub(2));
            let r = Rect::new(area.x + side + 2 + (area.width - side - 3 - rw) / 2, area.y + 1 + (area.height.saturating_sub(2 + h)) / 2, rw, h);
            draw_page(f, r, area, &p, None, if e.died { "here ends the road" } else { "the road goes on without them" });
        }
    } else {
        let mut y = area.y;
        if let Some(p) = page {
            let h = rows_needed(&p, area.width).min(area.height / 2);
            draw_page(f, Rect::new(area.x, y, area.width, h), area, &p, None, "");
            y += h;
        }
        let r = Rect::new(area.x + 1, y, area.width.saturating_sub(2), area.height.saturating_sub(y - area.y));
        f.render_widget(Paragraph::new(v).style(Style::default().bg(NIGHT)), r);
    }
}

/// An end with nothing written (an old save): who, and the keys.
fn draw_bare_end(f: &mut Frame, area: Rect, a: &Adventure) {
    let d = a.hero_dwarf();
    let v = vec![
        Line::from(Span::styled(format!("☠ {} is dead ☠", d.called()), Style::default().fg(RED).add_modifier(Modifier::BOLD))),
        Line::default(),
        key_line("n", "Found a fort in this world", false),
        key_line("a", "Another adventurer in this world", false),
        key_line("t", "Return to the title", false),
    ];
    f.render_widget(Paragraph::new(v), Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), area.height.saturating_sub(2)));
}

/// The saga so far (`S`), and settling down.
fn draw_saga(f: &mut Frame, area: Rect, s: &AdvScreen) {
    let a = &s.adv;
    let foot = match a.can_retire() {
        Ok(()) => "R: settle here for good · any key: back",
        Err(_) => "any key: back",
    };
    let Some(p) = saga_page(a) else {
        let d = a.hero_dwarf();
        let v = vec![
            Line::from(Span::styled(d.called(), Style::default().fg(GOLD).add_modifier(Modifier::BOLD))),
            Line::default(),
            plain("No saga is sung of you yet. Do something worth the singing.".into(), TEXT),
            Line::default(),
            plain(foot.into(), MUTED),
        ];
        f.render_widget(Paragraph::new(v), Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), area.height.saturating_sub(2)));
        return;
    };
    let w = area.width.saturating_sub(4).clamp(area.width.min(30), 80);
    let h = rows_needed(&p, w).min(area.height);
    let r = Rect::new(area.x + area.width.saturating_sub(w) / 2, area.y + area.height.saturating_sub(h) / 2, w, h);
    draw_page(f, r, area, &p, None, foot);
    if let Some(fl) = &s.flash {
        let y = (r.y + r.height).min(area.y + area.height.saturating_sub(1));
        f.render_widget(Paragraph::new(plain(format!(" » {}", fl), Color::Rgb(240, 150, 110))), Rect::new(area.x, y, area.width, 1));
    }
}

fn first(name: &str) -> String {
    name.split(' ').next().unwrap_or(name).to_string()
}

fn race_word(e: &Ending) -> String {
    let r = format!("{:?}", e.race).to_lowercase();
    let mut c = r.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str()) + " adventurer"
}
