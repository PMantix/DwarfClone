//! The injury readout: each hurt part coloured by how bad it is, with its
//! bleeding ("▲3") and what's been done for it, and a dwarf's medical
//! history. Used by the Health page (status tab 3) and the dwarf card.

use super::{header, plain, MUTED};
use crate::body::PARTS;
use crate::dwarf::Dwarf;
use crate::game::DAY;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// The colour for a part's state: lost red, mangled orange, badly wounded
/// yellow, bruised a dull pink.
fn part_color(status: &str) -> Color {
    match status {
        "missing" => Color::Rgb(255, 60, 60),
        "mangled" => Color::Rgb(255, 150, 50),
        "badly wounded" => Color::Rgb(235, 215, 70),
        _ => Color::Rgb(220, 160, 140),
    }
}

/// One line per hurt or bleeding part: "left leg  mangled  ▲12 bandaged",
/// in columns when `wide`, else "left leg: mangled ▲12 bandaged".
pub(super) fn part_lines(d: &Dwarf, indent: &str, wide: bool) -> Vec<Line<'static>> {
    let b = &d.body;
    PARTS
        .iter()
        .filter(|p| b.part_status(**p) != "fine" || b.bleeds[p.idx()] > 0)
        .map(|&p| {
            let i = p.idx();
            let status = b.part_status(p);
            let (name, status_text) = if wide { (format!("{:<11}", p.name()), format!("{:<14}", status)) } else { (format!("{}: ", p.name()), format!("{} ", status)) };
            let mut spans = vec![
                Span::raw(indent.to_string()),
                Span::styled(name, Style::default().fg(part_color(status))),
                Span::styled(status_text, Style::default().fg(part_color(status))),
            ];
            if b.bleeds[i] > 0 {
                spans.push(Span::styled(format!("▲{} ", b.bleeds[i]), Style::default().fg(Color::Rgb(255, 60, 60))));
            }
            let mut care = Vec::new();
            if b.bandaged[i] {
                care.push("bandaged");
            }
            if b.pressed[i] {
                care.push("pressed");
            }
            if b.splint[i] {
                care.push("splinted");
            }
            if !care.is_empty() {
                spans.push(Span::styled(care.join(", "), Style::default().fg(Color::Rgb(150, 200, 255))));
            }
            Line::from(spans)
        })
        .collect()
}

/// The last `n` lines of a dwarf's medical history, newest last, each with
/// its day.
pub(super) fn history_lines(d: &Dwarf, n: usize, indent: &str) -> Vec<Line<'static>> {
    let from = d.med.len().saturating_sub(n);
    d.med[from..].iter().map(|(t, text)| plain(format!("{}day {:<4} {}", indent, t / DAY + 1, text), MUTED)).collect()
}

/// The dwarf card's "Medical history" section (empty if there is none).
pub(super) fn card_history(d: &Dwarf) -> Vec<Line<'static>> {
    if d.med.is_empty() {
        return Vec::new();
    }
    let mut v = vec![Line::default(), header("Medical history")];
    // (Wrapped to the side panel's width.)
    let from = d.med.len().saturating_sub(5);
    for (t, text) in &d.med[from..] {
        let mut line = format!("  day {}:", t / DAY + 1);
        for word in text.split(' ') {
            if line.chars().count() + 1 + word.chars().count() > CARD_W {
                v.push(plain(std::mem::replace(&mut line, "    ".into()), MUTED));
            } else if !line.ends_with(' ') {
                line.push(' ');
            }
            line.push_str(word);
        }
        v.push(plain(line, MUTED));
    }
    v
}

/// How wide a line of the dwarf card can be.
const CARD_W: usize = 34;

#[cfg(test)]
mod tests {
    use super::super::draw;
    use crate::app::{App, Mode};
    use crate::body::Part;
    use crate::game::Game;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn dump(app: &mut App, w: u16, h: u16) -> String {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| draw(f, app)).unwrap();
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
    fn the_health_page_shows_wounds_and_history() {
        let mut app = App::with_game_for_tests(Game::new(5));
        let d = &mut app.game.dwarves[0];
        d.body.wound(Part::LeftLeg, 12, true);
        d.body.bandage(Part::LeftLeg);
        d.med.push((40, "gashed in the left leg by a goblin".into()));
        let id = d.id;
        app.mode = Mode::Status { tab: 2, scroll: 0 };
        let s = dump(&mut app, 130, 45);
        println!("{}", s);
        assert!(s.contains("left leg"), "no hurt part on the Health page");
        assert!(s.contains("▲"), "no bleeding rate on the Health page");
        assert!(s.contains("bandaged"));
        assert!(s.contains("gashed in the left leg by a goblin"), "no history on the Health page");
        // And the dwarf's own screen has the history too.
        app.mode = Mode::Unit { id, sel: 0 };
        let s = dump(&mut app, 130, 60);
        println!("{}", s);
        assert!(s.contains("Medical history"));
    }
}
