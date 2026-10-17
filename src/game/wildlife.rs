//! Wild things: what comes out of the land, and how a predator picks its
//! prey.

use super::combat::{dist2, near};
use super::*;
use crate::creature::{Creature, Species};

impl Game {
    /// What lives in this land: something to graze, something that hunts
    /// in packs, and something rare and dangerous.
    pub fn fauna(&self) -> (Species, Species, Species) {
        match self.biome {
            Biome::Taiga => (Species::Elk, Species::Wolf, Species::Bear),
            Biome::Tundra | Biome::Glacier => (Species::Elk, Species::Wolf, Species::PolarBear),
            Biome::Mountain => (Species::Ibex, Species::Wolf, Species::Bear),
            Biome::Desert => (Species::Camel, Species::Scorpion, Species::Viper),
            Biome::Badlands => (Species::Ibex, Species::Scorpion, Species::Viper),
            Biome::Savanna => (Species::Antelope, Species::Lion, Species::Lion),
            Biome::Jungle => (Species::Deer, Species::Jaguar, Species::Viper),
            Biome::Swamp => (Species::Deer, Species::Viper, Species::Alligator),
            _ => (Species::Deer, Species::Wolf, Species::Bear),
        }
    }

    pub(super) fn wildlife(&mut self) {
        if !self.rng.chance(self.knob("wildlife.chance", 0.18)) {
            return;
        }
        let Some(entry) = self.edge_entry() else { return };
        self.count("wildlife.visits", 1);
        let roll = self.rng.f64();
        let tame = 1.0 - self.savagery.clamp(0.05, 0.9) * 0.7;
        let (calm, pack, rare) = self.fauna();
        let (mut species, n) = if roll < tame || self.tick < SEASON {
            (calm, self.rng.range(2, 5))
        } else if roll < 0.95 || self.tick < YEAR {
            (pack, self.rng.range(2, 4))
        } else {
            (rare, 1)
        };
        // Evil land sends its dead.
        if self.evil && species.info().aggro > 0 && self.rng.chance(0.5) {
            species = Species::Husk;
        }
        let stay = DAY * self.rng.range(2, 5) as u64;
        for _ in 0..n {
            let p = self.spot_near(entry, 2);
            self.spawn_creature(species, p, stay);
        }
        let info = species.info();
        if info.aggro > 0 {
            let a = if info.name.starts_with(|c: char| "aeiou".contains(c)) { "An" } else { "A" };
            let what = if n > 1 { format!("A pack of {}", info.plural) } else { format!("{} {}", a, info.name) };
            self.msg(&format!("{} has been spotted nearby.", what), (255, 170, 80));
        }
    }

    /// A wild hostile with no target picks one: the nearest dwarf it can
    /// actually get to (a beast in a sealed cavern doesn't wear itself out
    /// hunting dwarves through rock).
    pub(super) fn predator_think(&mut self, c: &mut Creature) {
        if c.target.is_some() {
            return;
        }
        let r = c.species.info().aggro;
        let mut prey: Vec<(u32, Pos)> = self.dwarves.iter().filter(|d| d.alive && near(d.pos, c.pos, r)).map(|d| (d.id, d.pos)).collect();
        prey.sort_by_key(|(_, p)| dist2(*p, c.pos));
        let limit = (r * r * 8).clamp(400, 4000) as usize;
        c.target = prey.iter().find(|(_, p)| self.can_reach(c, *p, limit)).map(|(id, _)| *id);
        if c.target.is_none() && !prey.is_empty() {
            c.think_cd = 90;
        }
    }
}
