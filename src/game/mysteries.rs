//! Mysteries: small chains of events that unfold over days, each with a
//! twist. A stone that whispers in the dark; a stranger who asks for a
//! dwarf by name.

use super::*;
use crate::creature::Species;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MysteryKind {
    /// A gem dug from the deep that whispers to whoever found it.
    WhisperingStone,
    /// A hooded stranger at the gate, asking for a dwarf by name.
    Stranger,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mystery {
    pub kind: MysteryKind,
    pub stage: u8,
    /// The dwarf at the heart of it.
    pub dwarf: u32,
    /// The stone, or the stranger.
    pub thing: u32,
    pub next: u64,
}

impl Game {
    fn started(&self, kind: MysteryKind) -> bool {
        self.mysteries.iter().any(|m| m.kind == kind) || self.mysteries_done.contains(&kind)
    }

    /// A miner strikes a gem: once in a long while, it's no ordinary stone.
    pub(super) fn maybe_whispering_stone(&mut self, miner: u32, at: Pos) {
        if self.started(MysteryKind::WhisperingStone) || !self.rng.chance(0.08) {
            return;
        }
        let id = self.spawn_item(ItemKind::Gem, Mat::Amethyst, at);
        if let Some(it) = self.items.get_mut(&id) {
            it.label = Some("the Whispering Stone".into());
            it.quality = 5;
        }
        self.mysteries.push(Mystery { kind: MysteryKind::WhisperingStone, stage: 0, dwarf: miner, thing: id, next: self.tick + DAY });
    }

    /// After the first year, a stranger may come asking for someone.
    pub(super) fn maybe_stranger(&mut self) {
        if self.tick < YEAR || self.started(MysteryKind::Stranger) || !self.rng.chance(0.15) {
            return;
        }
        let Some(entry) = self.edge_entry() else { return };
        let cands: Vec<u32> = self.dwarves.iter().filter(|d| d.alive && !d.is_child()).map(|d| d.id).collect();
        if cands.is_empty() {
            return;
        }
        let who = *self.rng.pick(&cands);
        let id = self.spawn_creature(Species::Visitor, entry, self.tick + DAY * 6);
        let c = self.creatures.get_mut(&id).unwrap();
        c.label = Some("a hooded stranger".into());
        c.desc = Some("a figure in a travel-stained cloak, face hidden".into());
        c.visitor = super::visitors::STRANGER;
        let name = self.dwarves.iter().find(|d| d.id == who).map_or(String::new(), |d| d.called());
        self.msg(&format!("A hooded stranger has come to the fortress, asking for {} by name.", name), (200, 180, 255));
        self.gossip(format!("A stranger is asking after {}. Nobody knows who they are.", name));
        self.mysteries.push(Mystery { kind: MysteryKind::Stranger, stage: 0, dwarf: who, thing: id, next: self.tick + DAY });
    }

    /// Mysteries move on, a stage at a time.
    pub(super) fn mysteries_tick(&mut self) {
        let tick = self.tick;
        let due: Vec<usize> = (0..self.mysteries.len()).filter(|&i| tick >= self.mysteries[i].next).collect();
        for i in due.into_iter().rev() {
            let m = self.mysteries[i].clone();
            let done = match m.kind {
                MysteryKind::WhisperingStone => self.stone_stage(&m),
                MysteryKind::Stranger => self.stranger_stage(&m),
            };
            if done {
                self.mysteries.remove(i);
                self.mysteries_done.push(m.kind);
            } else {
                self.mysteries[i].stage += 1;
                self.mysteries[i].next = tick + DAY * 2;
            }
        }
    }

    /// True when the story is over.
    fn stone_stage(&mut self, m: &Mystery) -> bool {
        let tick = self.tick;
        let Some(di) = self.dwarves.iter().position(|d| d.id == m.dwarf && d.alive) else { return true };
        let name = self.dwarves[di].called();
        match m.stage {
            0 => {
                self.dwarves[di].think("heard the stone whisper in the dark", 8, tick);
                self.gossip(format!("{} swears the Whispering Stone speaks at night.", name));
                false
            }
            1 => {
                self.msg(&format!("{} has not slept. The Whispering Stone speaks to them of the deep places.", name), (200, 150, 255));
                false
            }
            _ => {
                let vault = self.vault.clone().filter(|v| !v.opened);
                match vault {
                    Some(v) => {
                        // The twist: the whispers knew the way to the sealed hall.
                        for dx in -1..=(v.max.x - v.min.x + 1) {
                            for dy in -1..=(v.max.y - v.min.y + 1) {
                                let p = v.min.offset(dx, dy, 0);
                                if self.world.t(p).kind == TileKind::Wall {
                                    self.world.t_mut(p).revealed = true;
                                }
                            }
                        }
                        self.dwarves[di].think("woke knowing the way to a sealed hall", -10, tick);
                        self.msg(
                            &format!("{} wakes and draws a map in the dust: the whispers led to {}, sealed in the rock at depth {}. Its carved walls can now be seen.", name, v.name, v.min.z),
                            (255, 215, 60),
                        );
                        self.pause_request = true;
                    }
                    None => {
                        // Or they were a craftsdwarf's muse all along.
                        self.msg(&format!("The Whispering Stone falls silent, and {} rises with a strange light in their eyes.", name), (200, 150, 255));
                        if self.dwarves[di].mood.is_none() && !self.dwarves.iter().any(|d| d.mood.is_some()) {
                            self.start_mood(di);
                        }
                    }
                }
                self.gossip(format!("The Whispering Stone is silent now. {} won't speak of what it said.", name));
                true
            }
        }
    }

    fn stranger_stage(&mut self, m: &Mystery) -> bool {
        let tick = self.tick;
        let Some(di) = self.dwarves.iter().position(|d| d.id == m.dwarf && d.alive) else {
            if let Some(c) = self.creatures.get_mut(&m.thing) {
                c.leaving = true;
            }
            return true;
        };
        let name = self.dwarves[di].called();
        if m.stage == 0 {
            self.gossip(format!("The stranger sat all night in the tavern, watching {} and saying nothing.", name));
            return false;
        }
        // The hood comes off.
        let pos = self.creatures.get(&m.thing).map(|c| c.pos);
        if let Some(c) = self.creatures.get_mut(&m.thing) {
            c.visitor = super::visitors::JOINED;
        }
        let Some(pos) = pos else { return true };
        if self.rng.chance(0.3) {
            // The twist: a spy.
            self.siege_warned = true;
            self.msg(&format!("The stranger who asked for {} was a goblin spy! They slipped away in the night with word of our defences.", name), (255, 90, 90));
            self.gossip(format!("{} keeps saying they never trusted that stranger.", name));
            self.dwarves[di].think("was fooled by a spy", 15, tick);
            return true;
        }
        let kin = *self.rng.pick(&["sibling", "childhood friend", "cousin", "old comrade"]);
        let id = self.new_id();
        let mut d = dwarf::new_dwarf(id, pos, &mut self.rng, &[]);
        let theirs = self.dwarves[di].id;
        d.adjust_regard(theirs, 80);
        self.dwarves[di].adjust_regard(id, 80);
        self.dwarves[di].think(&format!("was reunited with a long-lost {}", kin), -50, tick);
        self.dress(&mut d, 300);
        let newname = d.name.clone();
        self.dwarves.push(d);
        self.year_log.arrivals += 1;
        let fort = self.fortress_name.clone();
        self.msg(&format!("The stranger throws back their hood: it is {}, {}'s long-lost {}, come home at last!", newname, name, kin), (120, 255, 120));
        self.legend(format!("{}, long lost, found {} again in {}.", newname, name, fort));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_whispering_stone_leads_to_the_sealed_hall() {
        let mut g = Game::new(81);
        let v = g.vault.clone().expect("a vault");
        let miner = g.dwarves[0].id;
        while !g.started(MysteryKind::WhisperingStone) {
            g.maybe_whispering_stone(miner, g.dwarves[0].pos);
        }
        assert!(g.items.values().any(|it| it.label.as_deref() == Some("the Whispering Stone")));
        for _ in 0..4 {
            g.tick = g.mysteries.first().map_or(g.tick, |m| m.next);
            g.mysteries_tick();
        }
        assert!(g.mysteries.is_empty(), "the story should be over");
        assert!(g.world.t(v.min.offset(-1, 0, 0)).revealed, "the hall's walls should be found");
        assert!(g.log.iter().any(|l| l.text.contains("draws a map")));
        // A mystery happens once.
        g.maybe_whispering_stone(miner, g.dwarves[0].pos);
        assert!(g.mysteries.is_empty());
    }

    #[test]
    fn a_stranger_asks_for_someone_and_the_hood_comes_off() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        g.tick = YEAR;
        while !g.started(MysteryKind::Stranger) {
            g.maybe_stranger();
        }
        assert!(g.log.iter().any(|l| l.text.contains("asking for")));
        let before = g.dwarves.len();
        for _ in 0..3 {
            g.tick = g.mysteries.first().map_or(g.tick, |m| m.next);
            g.mysteries_tick();
        }
        assert!(g.mysteries.is_empty());
        let kin = g.log.iter().any(|l| l.text.contains("long-lost"));
        let spy = g.log.iter().any(|l| l.text.contains("goblin spy"));
        assert!(kin ^ spy, "one ending or the other");
        assert_eq!(g.dwarves.len(), before + kin as usize);
    }
}
