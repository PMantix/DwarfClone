//! The fortress's memory of what happened to whom (M2 W0-B): one log of
//! "who did what, where, when", kept for the life of the fort. The life
//! story and unit sheets, the overseer's journal and reign, the highlight
//! reel's weights, legends and inspirations read it; the systems that tell
//! the story write it, one `record` line each.
//!
//! Not to be confused with `sagas.rs` (story arcs told on cards) or
//! `stories.rs` (named battles and places): those are told; this is what
//! the fort remembers, told or not. Recording never draws on the game's
//! random numbers.

// Wave 0 interfaces for the wave-1 packages: some are not called yet.
#![allow(dead_code)]

use super::*;
use serde::{Deserialize, Serialize};

/// What sort of thing is remembered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryKind {
    Arrived,
    Born,
    Married,
    Died,
    Wounded,
    Mood,
    Artifact,
    Epithet,
    Appointed,
    Soldier,
    Expedition,
    Siege,
    SiegeEnd,
    Monument,
    Feat,
    Madness,
    Recovered,
    Overseer,
    Named,
    Inspired,
    Other,
}

/// One remembered thing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Memory {
    pub id: u32,
    pub tick: u64,
    pub kind: MemoryKind,
    /// The dwarves it happened to (or who did it), first the main one.
    #[serde(default)]
    pub who: Vec<u32>,
    #[serde(default)]
    pub at: Option<Pos>,
    pub text: String,
    /// 1 small .. 9 legendary.
    pub weight: u8,
}

/// Everything the fort remembers (`Game.memory`), oldest first.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Memories {
    #[serde(default)]
    pub events: Vec<Memory>,
    #[serde(default)]
    next: u32,
}

/// How many memories are kept.
pub const MAX_MEMORIES: usize = 6000;

impl Game {
    /// Remember something (weight 1 small .. 9 legendary) and return its id.
    /// Keeps at most `MAX_MEMORIES`: when full, the oldest of weight 1 or 2
    /// goes first, then the oldest of all.
    pub(super) fn record(&mut self, kind: MemoryKind, who: &[u32], at: Option<Pos>, text: String, weight: u8) -> u32 {
        let m = &mut self.memory;
        let id = m.next;
        m.next += 1;
        if m.events.len() >= MAX_MEMORIES {
            let i = m.events.iter().position(|e| e.weight <= 2).unwrap_or(0);
            m.events.remove(i);
        }
        m.events.push(Memory { id, tick: self.tick, kind, who: who.to_vec(), at, text, weight: weight.clamp(1, 9) });
        id
    }

    /// What the fort remembers of one dwarf, oldest first.
    pub fn memory_of(&self, dwarf: u32) -> impl Iterator<Item = &Memory> {
        self.memory.events.iter().filter(move |e| e.who.contains(&dwarf))
    }

    /// What the fort remembers from `tick` on, oldest first.
    pub fn memory_since(&self, tick: u64) -> impl Iterator<Item = &Memory> {
        let i = self.memory.events.partition_point(|e| e.tick < tick);
        self.memory.events[i..].iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_keeps_the_big_things() {
        let mut g = Game::new(5);
        g.memory = Memories::default();
        for i in 0..7000u32 {
            g.tick = i as u64;
            let w = (i % 9 + 1) as u8;
            g.record(MemoryKind::Other, &[i % 50], None, format!("thing {}", i), w);
        }
        assert!(g.memory.events.len() <= MAX_MEMORIES);
        let heavy = (0..7000u32).filter(|i| i % 9 + 1 >= 5).count();
        assert_eq!(g.memory.events.iter().filter(|e| e.weight >= 5).count(), heavy, "every weight-5+ memory is kept");
        assert!(g.memory_of(7).count() > 0 && g.memory_of(7).all(|e| e.who.contains(&7)));
        assert!(g.memory_since(6990).all(|e| e.tick >= 6990));
        assert_eq!(g.memory_since(6990).count(), 10);
        // Ids keep counting.
        assert_eq!(g.memory.events.last().map(|e| e.id), Some(6999));
    }
}
