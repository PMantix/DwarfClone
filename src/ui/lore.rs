//! The legends viewer's History tab (O09): the world's events under the
//! names of their ages, the tales as our people tell them now beside the
//! truth, and, after embark, the gaps where the fortress has forgotten.

use super::{KEY, MUTED, TEXT};
use crate::game::ages::ages_of;
use crate::overworld::Overworld;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

const AGE: Color = Color::Rgb(230, 200, 120);
const TOLD: Color = Color::Rgb(220, 170, 120);
const BARD: Color = Color::Rgb(200, 160, 255);
const GAP: Color = Color::Rgb(95, 95, 110);

/// The header of the legends viewer: the world and the age it is in.
pub fn legends_title(ow: &Overworld) -> String {
    match crate::game::ages::current_age(ow) {
        Some(a) => format!(" Legends of {} · {} ", ow.name, a),
        None => format!(" Legends of {} ", ow.name),
    }
}

/// The History tab. `all`: everything (the world map's legends, before
/// any fortress forgot anything); otherwise forgotten stretches are gaps.
pub fn history_lines(ow: &Overworld, all: bool) -> Vec<Line<'static>> {
    let ages = ages_of(ow);
    let mut next_age = 0;
    let mut v: Vec<Line<'static>> = Vec::new();
    let mut gap: Option<(usize, i32, i32)> = None;
    let flush = |v: &mut Vec<Line<'static>>, gap: &mut Option<(usize, i32, i32)>| {
        if let Some((n, a, b)) = gap.take() {
            let years = if a == b { format!("the year {}", a) } else { format!("the years {}–{}", a, b) };
            v.push(Line::from(Span::styled(
                format!("      … {} tale{} of {}, forgotten …", n, if n == 1 { "" } else { "s" }, years),
                Style::default().fg(GAP).add_modifier(Modifier::ITALIC),
            )));
        }
    };
    for (i, e) in ow.events.iter().enumerate() {
        while next_age < ages.len() && ages[next_age].from <= e.year {
            flush(&mut v, &mut gap);
            let a = &ages[next_age];
            if !v.is_empty() {
                v.push(Line::default());
            }
            v.push(Line::from(Span::styled(format!("═══ {} ═══", super::cap_first(&a.name)), Style::default().fg(AGE).add_modifier(Modifier::BOLD))));
            if !a.why.is_empty() {
                v.push(Line::from(Span::styled(format!("      began: {}", a.why), Style::default().fg(MUTED))));
            }
            next_age += 1;
        }
        if !all && !ow.is_known(i) {
            gap = Some(match gap {
                Some((n, a, _)) => (n + 1, a, e.year),
                None => (1, e.year, e.year),
            });
            continue;
        }
        flush(&mut v, &mut gap);
        v.push(Line::from(vec![
            Span::styled(format!("{:>4}  ", e.year), Style::default().fg(MUTED)),
            Span::styled(e.text.clone(), Style::default().fg(if e.bard { BARD } else { TEXT })),
            Span::styled(if e.bard { "  (sung by a bard)".to_string() } else { String::new() }, Style::default().fg(KEY)),
        ]));
        if let Some(t) = &e.told {
            v.push(Line::from(vec![
                Span::styled("      as told now: ".to_string(), Style::default().fg(MUTED)),
                Span::styled(t.clone(), Style::default().fg(TOLD).add_modifier(Modifier::ITALIC)),
            ]));
        }
    }
    flush(&mut v, &mut gap);
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;

    fn text(lines: &[Line]) -> String {
        lines.iter().map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect::<String>()).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn forgotten_legends_show_as_gaps_until_found() {
        let mut g = Game::new(81);
        let ow = &g.overworld;
        let lost = (0..ow.events.len()).find(|&i| !ow.is_known(i)).expect("something forgotten at embark");
        let truth = ow.events[lost].text.clone();
        let after = text(&history_lines(ow, false));
        assert!(after.contains("forgotten …"), "a gap");
        assert!(!after.lines().any(|l| l.ends_with(&truth)), "the forgotten tale is hidden");
        // The world map's legends show everything.
        assert!(text(&history_lines(ow, true)).contains(&truth));
        // Found again (a bard sang it): it shows.
        g.lore_sung_for_test(lost);
        assert!(text(&history_lines(&g.overworld, false)).contains(&truth));
        // The ages head their stretches of history, and the title names the age.
        assert!(after.contains("═══ The Age of"));
        assert!(legends_title(&g.overworld).contains("Age of"));
    }

    #[test]
    fn a_retold_tale_shows_both_ways() {
        let mut g = Game::new(81);
        g.overworld.log(1, "Urist slew 3 goblins.".into());
        g.overworld.events.last_mut().unwrap().told = Some("They say Urist slew a dozen goblins.".into());
        let t = text(&history_lines(&g.overworld, false));
        assert!(t.contains("Urist slew 3 goblins."));
        assert!(t.contains("as told now: They say Urist slew a dozen goblins."));
    }
}
