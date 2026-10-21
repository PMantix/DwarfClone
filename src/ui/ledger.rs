//! The ledger (O08, records of production): who made what this year or in
//! all the fort's years, what the miners dug, and who masters each
//! workshop. Opened from the work orders screen with `l`.

use super::{header, key_line, plain, KEY, MUTED, TEXT};
use crate::game::Game;
use crate::items::quality_prefix;
use ratatui::style::Color;
use ratatui::text::Line;

const GOLD: Color = Color::Rgb(230, 200, 120);

/// The ledger's lines, from `scroll` on.
pub fn ledger_lines<'a>(g: &Game, all: bool, scroll: usize) -> Vec<Line<'a>> {
    let mut v: Vec<Line> = Vec::new();
    let year = (!all).then(|| g.ledger_year());
    let name = |id: u32| g.dwarves.iter().find(|d| d.id == id).map_or_else(|| "(gone)".to_string(), |d| d.name.clone());
    let mut body: Vec<Line> = Vec::new();
    body.push(header("Makers"));
    let makers = g.ledger.makers(year);
    if makers.is_empty() {
        body.push(plain("Nothing made yet.".into(), MUTED));
    }
    for m in makers.iter().take(12) {
        let kinds: Vec<String> = m.kinds.iter().take(3).map(|(k, n)| format!("{} {}", n, k.plural())).collect();
        let best = if m.best >= 3 { format!(" · best {}", quality_prefix(m.best).trim()) } else { String::new() };
        body.push(plain(format!("{:<20.20} {:>4}", name(m.who), m.n), TEXT));
        body.push(plain(format!("   {}{}", kinds.join(", "), best), if m.best >= 5 { GOLD } else { MUTED }));
    }
    body.push(Line::default());
    body.push(header("Made"));
    for (k, n) in g.ledger.kinds(year).iter().take(12) {
        body.push(plain(format!("{:>5}  {}", n, k.plural()), TEXT));
    }
    body.push(Line::default());
    body.push(header("Dug"));
    let dug = g.ledger.dug(year);
    if dug.is_empty() {
        body.push(plain("No rock dug yet.".into(), MUTED));
    }
    for (m, n) in dug.iter().take(8) {
        let col = if m.is_ore() || m.is_gem() { GOLD } else { TEXT };
        body.push(plain(format!("{:>5}  {}", n, m.name()), col));
    }
    if let Some(&(w, n)) = g.ledger.miners(year).first() {
        body.push(plain(format!("Most dug by {} ({}).", name(w), n), MUTED));
    }
    body.push(Line::default());
    body.push(header("Masters"));
    let mut any = false;
    for b in g.buildings.values().filter(|b| b.master.is_some()) {
        if let Some(line) = g.master_line(b.id) {
            any = true;
            let shop = b.name.clone().unwrap_or_else(|| b.kind.name().to_string());
            body.push(plain(format!("{}:", shop), TEXT));
            body.push(plain(format!("   {}", line.trim_start_matches("Master: ")), MUTED));
        }
    }
    if !any {
        body.push(plain("No workshop has a master (M in a workshop).".into(), MUTED));
    }
    v.push(plain(if all { "All time".into() } else { format!("This year (year {} of the fort)", g.ledger_year() + 1) }, KEY));
    v.push(Line::default());
    let scroll = scroll.min(body.len().saturating_sub(1));
    v.extend(body.into_iter().skip(scroll));
    v.push(Line::default());
    v.push(key_line("Tab", if all { "This year" } else { "All time" }, false));
    v.push(key_line("↑/↓", "Scroll", false));
    v.push(plain("l/Esc: back to the orders".into(), MUTED));
    v
}
