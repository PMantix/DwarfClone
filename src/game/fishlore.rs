//! A legendary fish lives in the site's waters: it takes bait now and
//! then and breaks the line, year after year, until one fisher lands it.

use super::*;
use crate::overworld::Biome;
use serde::{Deserialize, Serialize};

const NAMES: [&str; 8] = ["Grimjaw", "Ironscale", "Silverback", "Mudking", "Hookbreaker", "Longwhisker", "Blackfin", "Stoneeye"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BigFish {
    pub name: String,
    pub kind: String,
    pub escapes: u32,
    pub caught: bool,
}

impl BigFish {
    pub fn title(&self) -> String {
        format!("{} the {}", self.name, self.kind)
    }
}

impl Game {
    /// Cast into the deep: maybe the great fish bites. True if it did,
    /// landed or not.
    pub(super) fn legendary_bite(&mut self, d: &mut Dwarf, stand: Pos) -> bool {
        if self.big_fish.as_ref().map_or(false, |f| f.caught) || !self.rng.chance(0.03) {
            return false;
        }
        if self.big_fish.is_none() {
            let kind = match self.biome {
                Biome::Ocean => "sea serpent",
                Biome::Swamp => "bog eel",
                _ => "great pike",
            };
            let name = format!("Old {}", self.rng.pick(&NAMES));
            self.big_fish = Some(BigFish { name, kind: kind.into(), escapes: 0, caught: false });
        }
        let f = self.big_fish.clone().unwrap();
        let skill = d.skill(Labor::Fishing) as f64;
        let land = (0.05 + 0.06 * skill + 0.04 * f.escapes as f64).min(0.7);
        let who = d.called();
        if !self.rng.chance(land) {
            let n = f.escapes + 1;
            self.big_fish.as_mut().unwrap().escapes = n;
            let text = format!(
                "{} took {}'s bait and broke the line! It has got away {} time{}.",
                f.title(),
                who,
                n,
                if n == 1 { "" } else { "s" }
            );
            self.msg(&text, (150, 200, 255));
            self.gossip(text);
            d.think(&format!("lost {} on the line", f.title()), 6, self.tick);
            return true;
        }
        // Landed at last.
        self.big_fish.as_mut().unwrap().caught = true;
        let id = self.spawn_item(ItemKind::Fish, Mat::Flesh, stand);
        if let Some(it) = self.items.get_mut(&id) {
            it.label = Some(f.title());
            it.quality = 5;
        }
        let fort = self.fortress_name.clone();
        let text = format!("{} has landed {}, after it broke {} lines! The whole fortress turns out to see it.", who, f.title(), f.escapes);
        self.msg(&text, (255, 215, 60));
        self.gossip(text);
        self.legend(format!("{} of {} landed {}, which had broken {} lines.", who, fort, f.title(), f.escapes));
        let tick = self.tick;
        for o in self.dwarves.iter_mut().filter(|o| o.alive) {
            o.think(&format!("marveled at {}", f.title()), -6, tick);
        }
        let last = f.kind.split(' ').last().unwrap_or("Fish");
        let epithet = format!("{}-Taker", super::heroes::cap_word(last));
        self.earn_epithet(d, &epithet, super::heroes::RANK_FEAT + 1, &format!("landed {}", f.title()));
        self.pause_request = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_great_fish_breaks_lines_until_it_is_landed() {
        let mut g = Game::new(81);
        g.biome = Biome::Forest;
        let mut d = std::mem::take(&mut g.dwarves[0]);
        let stand = d.pos;
        let mut bites = 0;
        for _ in 0..5000 {
            if g.legendary_bite(&mut d, stand) {
                bites += 1;
            }
            if g.big_fish.as_ref().map_or(false, |f| f.caught) {
                break;
            }
        }
        let f = g.big_fish.clone().expect("it never bit");
        assert!(f.caught, "never landed after {} bites", bites);
        assert_eq!(f.kind, "great pike");
        assert!(g.items.values().any(|it| it.label.as_deref() == Some(f.title().as_str()) && it.quality == 5));
        assert_eq!(d.epithet.as_deref(), Some("Pike-Taker"));
        // Once landed, it bites no more.
        assert!(!(0..500).any(|_| g.legendary_bite(&mut d, stand)));
        g.dwarves[0] = d;
    }
}
