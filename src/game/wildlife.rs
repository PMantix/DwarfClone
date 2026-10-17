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
        let mut pack = 0;
        for _ in 0..n {
            let p = self.spot_near(entry, 2);
            let id = self.spawn_creature(species, p, stay);
            // Those that came together hunt together.
            if n > 1 {
                if pack == 0 {
                    pack = id;
                }
                if let Some(c) = self.creatures.get_mut(&id) {
                    c.pack = pack;
                }
            }
        }
        let info = species.info();
        if info.aggro > 0 {
            let a = if info.name.starts_with(|c: char| "aeiou".contains(c)) { "An" } else { "A" };
            let what = if n > 1 { format!("A pack of {}", info.plural) } else { format!("{} {}", a, info.name) };
            self.msg(&format!("{} has been spotted nearby.", what), (255, 170, 80));
        }
    }

    /// A wild hostile with no target picks one. Beasts with some sense
    /// size up their prey and their own numbers first; the rest go for the
    /// nearest dwarf they can get to.
    pub(super) fn predator_think(&mut self, c: &mut Creature) {
        if c.target.is_some() {
            return;
        }
        if !has_sense(c) {
            self.nearest_prey(c);
            return;
        }
        let info = c.species.info();
        let pack = self.pack_strength(c) as f64;
        let nerve = self.knob("wild.nerve", 1.5);
        // Everyone in sight, the weakest (then the nearest) first.
        let mut prey: Vec<(f64, f64, u32, Pos)> = self
            .dwarves
            .iter()
            .filter(|d| d.alive && near(d.pos, c.pos, info.aggro + 6))
            .map(|d| {
                let (odds, escorts) = self.prey_odds(d);
                (odds, escorts, d.id, d.pos)
            })
            .collect();
        prey.sort_by(|a, b| b.0.total_cmp(&a.0).then(dist2(a.3, c.pos).cmp(&dist2(b.3, c.pos))));
        let limit = ((info.aggro + 6).pow(2) * 8).clamp(400, 4000) as usize;
        let Some(&(odds, escorts, id, pos)) = prey.iter().find(|p| self.can_reach(c, p.3, limit)) else {
            if !prey.is_empty() {
                c.think_cd = 90;
            }
            return;
        };
        // It goes for one that's close, alone, or weak enough for its pack.
        let close = near(pos, c.pos, info.aggro * 2);
        if close && (escorts == 0.0 || odds * pack >= nerve) {
            c.target = Some(id);
        }
    }

    /// The nearest dwarf it can actually get to (a beast in a sealed cavern
    /// doesn't wear itself out hunting dwarves through rock).
    fn nearest_prey(&mut self, c: &mut Creature) {
        let r = c.species.info().aggro;
        let mut prey: Vec<(u32, Pos)> = self.dwarves.iter().filter(|d| d.alive && near(d.pos, c.pos, r)).map(|d| (d.id, d.pos)).collect();
        prey.sort_by_key(|(_, p)| dist2(*p, c.pos));
        let limit = (r * r * 8).clamp(400, 4000) as usize;
        c.target = prey.iter().find(|(_, p)| self.can_reach(c, *p, limit)).map(|(id, _)| *id);
        if c.target.is_none() && !prey.is_empty() {
            c.think_cd = 90;
        }
    }

    /// How a predator sizes up a dwarf: (odds, escorts). Weak prey is
    /// bleeding, limping or a child, and not holding a weapon; escorts are
    /// the others within 4 (a soldier counts as three).
    pub(super) fn prey_odds(&self, d: &Dwarf) -> (f64, f64) {
        let mut weak = 1.0;
        if d.body.bleeding > 0 || d.body.limp() > 0 {
            weak += 0.5;
        }
        if d.is_child() {
            weak += 0.5;
        }
        if d.held.and_then(|id| self.items.get(&id)).map_or(false, |it| it.kind.weapon_damage().is_some()) {
            weak -= 0.5;
        }
        let escorts: f64 = self
            .dwarves
            .iter()
            .filter(|o| o.alive && o.id != d.id && !o.is_child() && near(o.pos, d.pos, 4))
            .map(|o| if o.soldier { 3.0 } else { 1.0 })
            .sum();
        (weak / (1.0 + escorts), escorts)
    }

    /// Its pack about it: itself and the mates within 8.
    pub(super) fn pack_strength(&self, c: &Creature) -> u32 {
        if c.pack == 0 {
            return 1;
        }
        1 + self.creatures.values().filter(|o| o.pack == c.pack && o.id != c.id && o.alive && !o.leaving && near(o.pos, c.pos, 8)).count() as u32
    }
}

/// Beasts of the land, which weigh their chances: not the dead, the
/// legendary, cave things or people.
pub(super) fn has_sense(c: &Creature) -> bool {
    !c.siege
        && !c.tame
        && !c.legendary()
        && !c.species.person()
        && !c.species.dwells_underground()
        && !matches!(c.species, Species::Husk | Species::Skulker)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::ItemKind;
    use crate::material::Mat;

    /// A fort with no beasts about, and level ground a little way off:
    /// three soldiers with spears at its west end, a woodcutter with an
    /// axe 15 tiles east.
    fn meadow(seed: u64) -> (Game, Pos, [u32; 3], u32) {
        let mut g = Game::new(seed);
        g.creatures.retain(|_, c| c.tame);
        let a = g.flat_patch(16, 3, 4).offset(0, 1, 0);
        for i in 0..16 {
            for j in -1..=1 {
                g.world.t_mut(a.offset(i, j, 0)).kind = TileKind::Floor;
            }
        }
        let mut squad = [0; 3];
        for i in 0..3 {
            let spear = g.spawn_item(ItemKind::Spear, Mat::Iron, a);
            let d = &mut g.dwarves[i];
            d.soldier = true;
            d.held = Some(spear);
            d.pos = a.offset(0, i as i32 - 1, 0);
            squad[i] = d.id;
        }
        let axe = g.spawn_item(ItemKind::Axe, Mat::Iron, a);
        let d = &mut g.dwarves[3];
        d.held = Some(axe);
        d.pos = a.offset(15, 0, 0);
        let woodcutter = d.id;
        // Nobody else about.
        g.dwarves.truncate(4);
        (g, a, squad, woodcutter)
    }

    /// Run only the creatures, the dwarves held where they stand.
    fn hold(g: &mut Game, ticks: u64, mut each: impl FnMut(&Game)) {
        let spots: Vec<Pos> = g.dwarves.iter().map(|d| d.pos).collect();
        for _ in 0..ticks {
            for (d, p) in g.dwarves.iter_mut().zip(&spots) {
                d.pos = *p;
            }
            g.update_creatures();
            g.tick += 1;
            each(g);
        }
    }

    #[test]
    fn wolves_pass_a_squad_and_take_the_straggler() {
        for seed in [1, 2, 4] {
            let (mut g, a, squad, woodcutter) = meadow(seed);
            let wolves = [g.spawn_creature(Species::Wolf, a.offset(7, 0, 0), u64::MAX), g.spawn_creature(Species::Wolf, a.offset(7, 1, 0), u64::MAX)];
            for w in wolves {
                g.creatures.get_mut(&w).unwrap().pack = wolves[0];
            }
            let mut hunted = false;
            hold(&mut g, 300, |g| {
                hunted |= g.creatures.values().any(|c| c.target == Some(woodcutter));
            });
            for id in squad {
                let d = g.dwarves.iter().find(|d| d.id == id).unwrap();
                assert!(d.attacker.is_none(), "seed {}: a wolf went for a soldier", seed);
            }
            let w = g.dwarves.iter().find(|d| d.id == woodcutter).unwrap();
            assert!(hunted || w.attacker.is_some(), "seed {}: the woodcutter was left alone", seed);
        }
    }

    #[test]
    fn a_pack_sizes_up_its_prey() {
        let (g, _, squad, woodcutter) = meadow(1);
        let soldier = g.dwarves.iter().find(|d| d.id == squad[1]).unwrap();
        let (odds, escorts) = g.prey_odds(soldier);
        assert_eq!(escorts, 6.0, "two soldiers beside them");
        assert!(odds < 0.1);
        let w = g.dwarves.iter().find(|d| d.id == woodcutter).unwrap();
        assert_eq!(g.prey_odds(w), (0.5, 0.0), "alone, but holding an axe");
    }
}
