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
    /// Artifacts with powers (`relics.rs`).
    #[serde(default)]
    pub relics: Vec<super::relics::Relic>,
    /// Each god's anger at broken taboos (`omens.rs`).
    #[serde(default)]
    pub wrath: BTreeMap<u32, u32>,
    /// Dwarves under a god's curse.
    #[serde(default)]
    pub curses: Vec<super::omens::Curse>,
    /// An omen seen, and not yet come true.
    #[serde(default)]
    pub omen: Option<super::omens::Omen>,
    /// When blood was last found on a temple floor.
    #[serde(default)]
    pub blood_seen: u64,
    /// The resident wizard (`wizard.rs`).
    #[serde(default)]
    pub wizard: Option<super::wizard::Wizard>,
    /// When a wizard was last turned away.
    #[serde(default)]
    pub wizard_refused: u64,
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

    /// The year's omens, curses and wonders, for the chronicle.
    pub(super) fn magic_chronicle(&mut self) -> Vec<String> {
        std::mem::take(&mut self.magic.year)
    }

    /// Once a day: the faithful.
    pub(super) fn magic_daily(&mut self) {
        self.faith_daily();
        self.curses_daily();
        self.omens_daily();
        self.wizard_daily();
    }

    /// Every 20 ticks: the relics at work, the wizard at their spells.
    pub(super) fn magic_tick(&mut self) {
        self.relics_tick();
        self.wizard_tick();
    }

    /// Each season: word of the relics gets about.
    pub(super) fn magic_season(&mut self) {
        self.relics_season();
        self.omens_season();
        self.wizard_season();
    }
}

impl Game {
    /// For the look panel, under a tile: whose temple it is.
    pub fn magic_tile_lines(&self, p: Pos) -> Vec<String> {
        self.shrine_line(p).into_iter().collect()
    }

    /// For the look panel, under a dwarf: whom they worship.
    pub fn magic_dwarf_lines(&self, id: u32) -> Vec<String> {
        self.faith_line(id).into_iter().chain(self.curse_line(id)).chain(self.wizard_lines(id)).collect()
    }

    /// For the fortress status screen: the fort's god, the omen hanging
    /// over it, the cursed, the relics and the wizard.
    pub fn magic_status_lines(&self) -> Vec<String> {
        let mut v = Vec::new();
        if let Some(god) = self.chief_god().and_then(|g| self.god(g)) {
            let temples = self.magic.shrines.values().filter(|&&s| self.god(s).map_or(false, |x| x.name == god.name)).count();
            let t = match temples {
                0 => "no temple".to_string(),
                1 => "one temple".to_string(),
                n => format!("{} temples", n),
            };
            v.push(format!("Most here worship {} ({}); {} forbids {}.", god.called(), t, god.name, god.taboo.name()));
        }
        if let Some(o) = &self.magic.omen {
            v.push(format!("An omen hangs over the fort: {}, and {}.", o.kind.sight(&self.fortress_name), o.kind.meaning()));
        }
        for c in &self.magic.curses {
            if let (Some(d), Some(god)) = (self.dwarves.iter().find(|d| d.id == c.dwarf), self.god(c.god)) {
                v.push(format!("{} is cursed by {} for {}.", d.name, god.name, c.why.name()));
            }
        }
        let relics = self.magic.relics.iter().filter(|r| self.items.contains_key(&r.item)).count();
        if relics > 0 {
            v.push(format!("Artifacts of power in the fort: {}.", relics));
        }
        if let Some(w) = &self.magic.wizard {
            let lab = if w.lab != 0 { "in their laboratory" } else { "wanting a laboratory" };
            v.push(format!("The wizard {} {} lives here, {}; {} spells cast, {} accidents.", w.name, w.title, lab, w.spells, w.accidents));
        }
        v
    }

    /// For the look panel, under an item: its power.
    pub fn magic_item_lines(&self, id: u32) -> Vec<String> {
        self.relic_lines(id)
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
