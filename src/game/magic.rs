//! Strange powers (WISHLIST B14, the MYT ideas): the gods the dwarves
//! worship and their temples (`faith.rs`). Everything the package keeps
//! lives in `Game.magic`, and its random draws come from a stream of their
//! own so the rest of the fortress rolls the same dice.

use super::*;
use crate::worldgen::myth::God;
use serde::{Deserialize, Serialize};

/// The fortress's share of myth and magic.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Magic {
    /// Each dwarf's patron god and how devout they are (1 lightly, 2
    /// devoutly, 3 zealously).
    #[serde(default)]
    pub patrons: BTreeMap<u32, (u32, u8)>,
    /// Temples (zone id) and the god each is dedicated to.
    #[serde(default)]
    pub shrines: BTreeMap<u32, u32>,
    /// Lines for this year's chronicle.
    #[serde(default)]
    pub year: Vec<String>,
    /// Draws from the package's own random stream.
    #[serde(default)]
    pub draws: u64,
}

impl Game {
    /// A random stream of the package's own: each call a fresh generator,
    /// so nothing here moves the fortress's own dice.
    pub(super) fn mrng(&mut self) -> Rng {
        self.magic.draws += 1;
        Rng::new(self.overworld.seed ^ self.tick.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ self.magic.draws.wrapping_mul(0xD1B5_4A32_D192_ED03) ^ 0x6D61_6769_63)
    }

    /// A god of this world by id.
    pub fn god(&self, id: u32) -> Option<&God> {
        self.overworld.gods.get(id as usize)
    }

    /// Once a day: the faithful.
    pub(super) fn magic_daily(&mut self) {
        self.faith_daily();
    }
}

impl Game {
    /// For the look panel, under a tile: whose temple it is.
    pub fn magic_tile_lines(&self, p: Pos) -> Vec<String> {
        self.shrine_line(p).into_iter().collect()
    }

    /// For the look panel, under a dwarf: whom they worship.
    pub fn magic_dwarf_lines(&self, id: u32) -> Vec<String> {
        self.faith_line(id).into_iter().collect()
    }
}

/// The legends' Gods tab: each god, who worships them, what they forbid,
/// and their myth.
pub fn gods_lines(ow: &crate::overworld::Overworld) -> Vec<ratatui::text::Line<'static>> {
    use ratatui::style::{Color, Modifier, Style};
    use ratatui::text::{Line, Span};
    let mut v = Vec::new();
    if ow.gods.is_empty() {
        v.push(Line::from(Span::styled("No gods are known in this world.", Style::default().fg(Color::Rgb(130, 130, 140)))));
    }
    for g in &ow.gods {
        let peoples: Vec<String> = g.civs.iter().filter_map(|&c| ow.civs.get(c as usize)).map(|c| format!("the {}", c.name)).collect();
        v.push(Line::from(vec![
            Span::styled(g.called(), Style::default().fg(Color::Rgb(255, 225, 150)).add_modifier(Modifier::BOLD)),
            Span::styled(format!(", {}", g.describe()), Style::default().fg(Color::Rgb(220, 220, 225))),
        ]));
        v.push(Line::from(Span::styled(
            format!("   worshipped by {}; forbids {}", peoples.join(" and "), g.taboo.name()),
            Style::default().fg(Color::Rgb(130, 130, 140)),
        )));
        v.push(Line::from(Span::styled(format!("   {}", g.myth), Style::default().fg(Color::Rgb(190, 180, 230)))));
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_legends_tell_of_the_gods() {
        let ow = crate::overworld::Overworld::generate(81);
        let text: String = gods_lines(&ow).iter().map(|l| l.spans.iter().map(|s| s.content.to_string()).collect::<String>() + "\n").collect();
        for g in &ow.gods {
            assert!(text.contains(&g.called()) && text.contains(&g.myth) && text.contains(g.taboo.name()), "{} missing", g.called());
        }
        // An old save's world without gods finds them on the first day.
        let mut g = Game::new(81);
        g.overworld.gods.clear();
        g.magic_daily();
        assert_eq!(g.overworld.gods, ow.gods);
        assert!(!g.magic.patrons.is_empty());
    }
}
