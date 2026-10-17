//! What the fortress is short of, as the announcements tell it ("…waiting
//! to be built: it needs a mechanism", "Your dwarves have nothing to drink!"),
//! kept as plain facts so the overseer can notice and see to them. How soon
//! it notices, and what it bothers with, depends on its style and on
//! whether it has a manager.

use super::overseer::{Overseer, RoomUse, Style};
use super::*;
use crate::buildings::{Input, Recipe};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lack {
    Stone,
    Wood,
    Blocks,
    Mechanism,
    Cage,
    Bucket,
    Ore,
    Fuel,
    Bars,
    Drink,
    Food,
    Brewable,
    Seeds,
    Thread,
    Cloth,
    Leather,
    Pick,
    Axe,
    Manager,
    /// A work order no workshop can do: this kind is missing.
    Workshop(BuildingKind),
}

impl Lack {
    /// What a building part or a construction asks for, from its words.
    pub fn from_desc(desc: &str) -> Option<Lack> {
        let has = |w: &str| desc.contains(w);
        Some(if has("mechanism") {
            Lack::Mechanism
        } else if has("cage") {
            Lack::Cage
        } else if has("bucket") {
            Lack::Bucket
        } else if has("block") {
            Lack::Blocks
        } else if has("boulder") || has("stone") || has("rock") {
            Lack::Stone
        } else if has("log") || has("wood") {
            Lack::Wood
        } else if has("bar") || has("metal") {
            Lack::Bars
        } else {
            return None;
        })
    }

    pub fn from_input(i: Input) -> Option<Lack> {
        Some(match i {
            Input::Log => Lack::Wood,
            Input::Stone | Input::Flux => Lack::Stone,
            Input::Brewable | Input::Grain => Lack::Brewable,
            Input::Fiber => Lack::Thread,
            Input::Ore => Lack::Ore,
            Input::Fuel => Lack::Fuel,
            Input::Bar | Input::IronBar => Lack::Bars,
            Input::Thread => Lack::Thread,
            Input::Cloth => Lack::Cloth,
            Input::Hide | Input::Leather => Lack::Leather,
            _ => return None,
        })
    }

    fn what(self) -> String {
        match self {
            Lack::Stone => "stone".into(),
            Lack::Wood => "wood".into(),
            Lack::Blocks => "stone blocks".into(),
            Lack::Mechanism => "mechanisms".into(),
            Lack::Cage => "cages".into(),
            Lack::Bucket => "buckets".into(),
            Lack::Ore => "ore".into(),
            Lack::Fuel => "fuel".into(),
            Lack::Bars => "metal bars".into(),
            Lack::Drink => "drink".into(),
            Lack::Food => "food".into(),
            Lack::Brewable => "plants to brew".into(),
            Lack::Seeds => "seeds".into(),
            Lack::Thread => "thread".into(),
            Lack::Cloth => "cloth".into(),
            Lack::Leather => "leather".into(),
            Lack::Pick => "picks".into(),
            Lack::Axe => "axes".into(),
            Lack::Manager => "a manager".into(),
            Lack::Workshop(k) => format!("a {}", k.name().to_lowercase()),
        }
    }
}

/// How long a trouble is remembered after it was last announced.
const MEMORY: u64 = DAY * 3;

impl Game {
    /// Something is short: remember it (an announcement said so).
    pub(super) fn trouble(&mut self, lack: Lack) {
        let tick = self.tick;
        match self.troubles.iter_mut().find(|t| t.0 == lack) {
            Some(t) => t.1 = tick,
            None => self.troubles.push((lack, tick)),
        }
    }

    /// How sharp an overseer is about the fortress's wants: by its style,
    /// sharper with a manager to keep track.
    fn attention(&self, o: &Overseer, lack: Lack) -> f64 {
        let base: f64 = match o.style {
            Style::Steady => 0.85,
            Style::Builder => 0.75,
            Style::Delver => 0.7,
            Style::Warlord => 0.6,
            Style::Reckless => 0.3,
        };
        // What each cares about most (and least).
        let care = match (o.style, lack) {
            (Style::Builder, Lack::Stone | Lack::Blocks | Lack::Mechanism | Lack::Workshop(_)) => 1.25,
            (Style::Delver, Lack::Ore | Lack::Stone | Lack::Pick | Lack::Fuel | Lack::Bars) => 1.25,
            (Style::Warlord, Lack::Bars | Lack::Mechanism | Lack::Cage | Lack::Ore) => 1.3,
            (Style::Warlord, Lack::Thread | Lack::Cloth | Lack::Leather) => 0.6,
            (Style::Reckless, Lack::Mechanism | Lack::Cage | Lack::Bucket) => 0.3,
            (_, Lack::Drink | Lack::Food) => 1.4,
            _ => 1.0,
        };
        let manager = if self.has_post(super::positions::Position::Manager) { 0.1 } else { 0.0 };
        ((base + manager) * care).min(0.98)
    }

    /// Once a day: notice what's short and do something about it.
    pub(super) fn ov_troubles(&mut self, o: &mut Overseer) {
        let tick = self.tick;
        self.troubles.retain(|t| tick < t.1 + MEMORY);
        o.fixes.retain(|f| tick < f.1 + DAY * 3);
        let open: Vec<Lack> = self.troubles.iter().map(|t| t.0).filter(|l| !o.fixes.iter().any(|f| f.0 == *l)).collect();
        for lack in open {
            if !self.rng.chance(self.attention(o, lack)) {
                continue;
            }
            if let Some(what) = self.fix(o, lack) {
                o.fixes.push((lack, tick));
                Self::note_ov(o, tick, format!("Saw the fortress short of {}: {}", lack.what(), what));
            }
        }
        // Standing orders it raised itself, kept up until they run out.
        o.boosts.retain(|b| tick < b.2);
    }

    /// Keep more of something in stock for a while.
    fn boost(o: &mut Overseer, recipe: Recipe, keep: u32, tick: u64) {
        o.boosts.retain(|b| b.0 != recipe);
        o.boosts.push((recipe, keep, tick + DAY * 20));
    }

    /// Make sure someone does `labor` (`n` dwarves, the idlest first).
    fn hands_for(&mut self, labor: Labor, n: usize) -> bool {
        let have = self.dwarves.iter().filter(|d| d.alive && !d.insane && !d.is_child() && d.has(labor)).count();
        if have >= n {
            return false;
        }
        let mut cands: Vec<(usize, u32, u32)> = self
            .dwarves
            .iter()
            // (Not one who'd have to give up the rival tool: a woodcutter isn't made a miner.)
            .filter(|d| d.alive && !d.insane && !d.is_child() && !d.soldier && !d.has(labor) && labor.exclusive_with().map_or(true, |r| !d.has(r)))
            .map(|d| (crate::dwarf::LABORS.iter().filter(|l| d.has(**l)).count(), u32::MAX - d.skills[labor.idx()], d.id))
            .collect();
        cands.sort();
        for (_, _, id) in cands.into_iter().take(n - have) {
            self.set_labor(id, labor, true);
        }
        true
    }

    /// Do something about `lack`; what was done, in the overseer's words.
    fn fix(&mut self, o: &mut Overseer, lack: Lack) -> Option<String> {
        let tick = self.tick;
        let pop = self.alive_count() as u32;
        match lack {
            Lack::Stone => {
                // Quarry out a storeroom: stone now, room for it after.
                let miners = self.hands_for(Labor::Mining, 2);
                if !o.wants.contains(&RoomUse::Stockpile) {
                    o.wants.insert(0, RoomUse::Stockpile);
                }
                Some(if miners { "set miners to quarry out a storeroom.".into() } else { "had a storeroom quarried out.".into() })
            }
            Lack::Wood => {
                self.hands_for(Labor::Woodcutting, 1);
                let e = self.embark;
                let k = o.done.iter().filter(|d| d.starts_with("chop")).count() as i32;
                let r = 22 + k * 8;
                self.designate(e.offset(-r, -r, -2), e.offset(r, r, 2), Desig::Chop);
                o.done.push(format!("chop{}", k));
                Some("sent woodcutters further out.".into())
            }
            Lack::Blocks => {
                Self::boost(o, Recipe::Blocks, 10 + pop / 2, tick);
                self.hands_for(Labor::Masonry, 1);
                Some("had the masons cut blocks.".into())
            }
            Lack::Mechanism => {
                Self::boost(o, Recipe::Mechanism, 6, tick);
                self.hands_for(Labor::Mechanics, 1);
                Some("ordered mechanisms made.".into())
            }
            Lack::Cage => {
                Self::boost(o, Recipe::WoodCage, 4, tick);
                Some("ordered cages built.".into())
            }
            Lack::Bucket => {
                Self::boost(o, Recipe::Bucket, 3, tick);
                Some("ordered buckets.".into())
            }
            Lack::Ore | Lack::Bars => {
                self.hands_for(Labor::Mining, 2);
                self.ov_mine(o, 30, 30);
                Self::boost(o, Recipe::SmeltOre, 8, tick);
                Some("sent miners after ore.".into())
            }
            Lack::Fuel => {
                Self::boost(o, Recipe::Charcoal, 8, tick);
                self.hands_for(Labor::Woodcutting, 1);
                Some("ordered charcoal burned.".into())
            }
            Lack::Drink | Lack::Brewable => {
                Self::boost(o, Recipe::Brew, 30 + pop * 3, tick);
                self.hands_for(Labor::Brewing, 2);
                self.hands_for(Labor::Farming, 3 + pop as usize / 6);
                Some("put more hands to farming and brewing.".into())
            }
            Lack::Food => {
                Self::boost(o, Recipe::Cook, 15 + pop * 2, tick);
                self.hands_for(Labor::Cooking, 1);
                self.hands_for(Labor::Farming, 3 + pop as usize / 6);
                self.hands_for(Labor::Fishing, 2);
                self.hunt = o.style != Style::Reckless;
                Some("set more to farming, fishing and the kitchen.".into())
            }
            Lack::Seeds => {
                self.hands_for(Labor::Farming, 2);
                Some("set farmers to gather what grows wild.".into())
            }
            Lack::Thread | Lack::Cloth => {
                Self::boost(o, Recipe::SpinThread, 6, tick);
                Self::boost(o, Recipe::Weave, 4, tick);
                Some("ordered thread spun and cloth woven.".into())
            }
            Lack::Leather => {
                Self::boost(o, Recipe::Tan, 4, tick);
                Some("ordered hides tanned.".into())
            }
            Lack::Pick | Lack::Axe => {
                Self::boost(o, if lack == Lack::Pick { Recipe::ForgePick } else { Recipe::ForgeAxe }, 2, tick);
                Some("ordered tools forged.".into())
            }
            Lack::Manager => {
                let m = self.candidates(super::positions::Position::Manager).first().copied()?;
                self.appoint(super::positions::Position::Manager, Some(m));
                Some("appointed a manager.".into())
            }
            Lack::Workshop(_) => {
                if !o.wants.contains(&RoomUse::Workshops) {
                    o.wants.insert(0, RoomUse::Workshops);
                }
                Some("had a workshop hall dug for it.".into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_overseer_sees_to_what_the_fortress_lacks() {
        let mut g = Game::new(5);
        g.set_overseer(Some(Style::Steady));
        for _ in 0..DAY * 2 {
            g.step();
        }
        // Nobody mines any more, and the masons have no stone.
        let ids: Vec<u32> = g.dwarves.iter().map(|d| d.id).collect();
        for id in ids {
            g.set_labor(id, Labor::Mining, false);
        }
        // (Whatever it noticed in the first days is forgotten.)
        let o = g.overseer.as_mut().unwrap();
        o.notes.clear();
        o.fixes.clear();
        let mut days = 0;
        while days < 6 && !g.overseer.as_ref().unwrap().notes.iter().any(|n| n.1.contains("short of stone:")) {
            g.trouble(Lack::Stone);
            for _ in 0..DAY {
                g.step();
            }
            days += 1;
            eprintln!("day {} miners {} notes {:?}", days, g.dwarves.iter().filter(|d| d.has(Labor::Mining)).count(), g.overseer.as_ref().unwrap().notes.iter().rev().take(3).map(|n| n.1.clone()).collect::<Vec<_>>());
        }
        let o = g.overseer.as_ref().unwrap();
        assert!(o.notes.iter().any(|n| n.1.contains("short of stone:")), "the overseer never noticed: {:?}", o.notes.iter().rev().take(5).collect::<Vec<_>>());
        let miners = g.dwarves.iter().filter(|d| d.alive && d.has(Labor::Mining)).count();
        if miners < 2 {
            for d in g.dwarves.iter() {
                eprintln!("{} alive {} soldier {} insane {} child {} labors {:?}", d.name, d.alive, d.soldier, d.insane, d.is_child(), crate::dwarf::LABORS.iter().filter(|l| d.has(**l)).map(|l| l.short()).collect::<Vec<_>>());
            }
        }
        assert!(miners >= 2, "nobody was set to mine");
        // And a shortage it has seen to isn't fussed over again at once.
        assert!(o.fixes.iter().any(|f| f.0 == Lack::Stone));
    }

    #[test]
    fn announcements_say_what_is_short() {
        assert_eq!(Lack::from_desc("a mechanism"), Some(Lack::Mechanism));
        assert_eq!(Lack::from_desc("stone blocks or a boulder"), Some(Lack::Blocks));
        assert_eq!(Lack::from_desc("a stone boulder"), Some(Lack::Stone));
        assert_eq!(Lack::from_desc("a log or blocks"), Some(Lack::Blocks));
        assert_eq!(Lack::from_input(Input::Brewable), Some(Lack::Brewable));
    }
}
