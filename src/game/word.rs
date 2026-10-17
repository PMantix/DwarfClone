//! Word of the fortress gets around. When a dwarf goes mad for want of
//! something the fortress can't supply, it's talked of; the merchants carry
//! the talk home when they leave, and failing that the tappers of the deep
//! roads (who knock news from hold to hold through the rock) get it there
//! by the next season. Dwarves at the mountainhome who love that thing
//! don't come. Better word goes out when the fortress has it after all.

use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Want {
    Stone,
    Wood,
    Metal,
    Gems,
    Bone,
}

impl Want {
    pub fn from_need(n: crate::dwarf::MoodNeed) -> Want {
        use crate::dwarf::MoodNeed;
        match n {
            MoodNeed::Log => Want::Wood,
            MoodNeed::Stone => Want::Stone,
            MoodNeed::Bar => Want::Metal,
            MoodNeed::Gem => Want::Gems,
            MoodNeed::Bone => Want::Bone,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Want::Stone => "good stone",
            Want::Wood => "wood",
            Want::Metal => "metal",
            Want::Gems => "gems",
            Want::Bone => "bone",
        }
    }

    /// Whether a dwarf who likes `m` would care.
    pub fn loves(self, m: Mat) -> bool {
        match self {
            Want::Stone => matches!(m, Mat::Granite | Mat::Marble | Mat::Limestone),
            Want::Wood => m == Mat::Wood,
            Want::Metal => matches!(m, Mat::Copper | Mat::Iron | Mat::Steel | Mat::Gold),
            Want::Gems => m.is_gem(),
            Want::Bone => m == Mat::Bone,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Word {
    pub want: Want,
    pub since: u64,
    /// It has reached the mountainhome.
    pub home: bool,
}

impl Game {
    /// Whether the fortress has some of it now.
    fn has_some(&self, want: Want) -> bool {
        self.items.values().any(|it| {
            it.carried_by.is_none()
                && match want {
                    Want::Stone => it.kind == ItemKind::Boulder && it.mat.is_building_stone(),
                    Want::Wood => it.kind == ItemKind::Log,
                    Want::Metal => it.kind == ItemKind::Bar,
                    Want::Gems => it.mat.is_gem(),
                    Want::Bone => it.mat == Mat::Bone,
                }
        })
    }

    /// A dwarf went mad for want of this: it's talked of.
    pub(super) fn word_of_want(&mut self, want: Want) {
        if self.word.iter().any(|w| w.want == want) {
            return;
        }
        let fort = self.fortress_name.clone();
        self.word.push(Word { want, since: self.tick, home: false });
        self.gossip(format!("All the talk is of how {} has no {}.", fort, want.name()));
    }

    /// The merchants are leaving: they'll carry the talk home.
    pub(super) fn word_with_caravan(&mut self) {
        let fort = self.fortress_name.clone();
        let news: Vec<Want> = self.word.iter().filter(|w| !w.home).map(|w| w.want).collect();
        for want in news {
            self.msg(&format!("The merchants will carry word of {}'s want of {} back with them.", fort, want.name()), (200, 180, 140));
        }
        for w in self.word.iter_mut() {
            w.home = true;
        }
    }

    /// Each season: talk that the merchants didn't carry reaches the
    /// mountainhome by the deep roads; talk of a want the fortress has met
    /// fades, and better word follows it.
    pub(super) fn word_season(&mut self) {
        let tick = self.tick;
        let fort = self.fortress_name.clone();
        let mut lines = Vec::new();
        for w in self.word.iter_mut() {
            if !w.home && tick > w.since + SEASON / 2 {
                w.home = true;
                lines.push(format!("Word of {}'s want of {} has been tapped through the deep roads to the mountainhome.", fort, w.want.name()));
            }
        }
        let met: Vec<Want> = self.word.iter().map(|w| w.want).filter(|&want| self.has_some(want)).collect();
        for want in met {
            self.word.retain(|w| w.want != want);
            lines.push(format!("Better word goes out: {} has {} after all.", fort, want.name()));
        }
        for l in lines {
            self.msg(&l, (200, 180, 140));
        }
    }

    /// Of `n` dwarves thinking of coming, how many the word kept at home
    /// (those who love what the fortress lacks), and why.
    pub(super) fn word_keeps_away(&self, likes: &[Mat]) -> (Vec<bool>, Vec<Want>) {
        let known: Vec<Want> = self.word.iter().filter(|w| w.home).map(|w| w.want).collect();
        let stay: Vec<bool> = likes.iter().map(|&m| known.iter().any(|w| w.loves(m))).collect();
        let why: Vec<Want> = known.into_iter().filter(|w| likes.iter().zip(&stay).any(|(&m, &s)| s && w.loves(m))).collect();
        (stay, why)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_of_a_want_reaches_the_mountainhome_and_keeps_its_lovers_away() {
        let mut g = Game::new(11);
        g.items.retain(|_, it| !it.mat.is_gem());
        g.word_of_want(Want::Gems);
        assert!(g.gossip.iter().any(|s| s.1.contains("no gems")), "it should be talked of");
        // Not yet home: nobody's kept away.
        let (stay, _) = g.word_keeps_away(&[Mat::Amethyst, Mat::Granite]);
        assert_eq!(stay, vec![false, false]);
        // The merchants carry it home.
        g.word_with_caravan();
        assert!(g.log.iter().any(|l| l.text.contains("carry word")));
        let (stay, why) = g.word_keeps_away(&[Mat::Amethyst, Mat::Granite]);
        assert_eq!(stay, vec![true, false], "the gem-lover stays home, the granite-lover comes");
        assert_eq!(why, vec![Want::Gems]);
        // Gems at last: better word goes out.
        g.spawn_item(ItemKind::Gem, Mat::Amethyst, g.embark);
        g.word_season();
        assert!(g.word.is_empty());
        assert!(g.log.iter().any(|l| l.text.contains("Better word")));
        // And without a caravan, the deep roads carry it in time.
        g.items.retain(|_, it| !it.mat.is_gem());
        g.word_of_want(Want::Gems);
        g.tick += SEASON;
        g.word_season();
        assert!(g.word[0].home && g.log.iter().any(|l| l.text.contains("deep roads")));
    }
}
