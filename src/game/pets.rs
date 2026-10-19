//! Pets: a cat chooses its dwarf, a dog is taken in. They follow their
//! owners about, get up to mischief, and are mourned when they die.

use super::*;
use crate::creature::{Creature, Species};
use super::combat::near;

const NAMES: [&str; 20] = [
    "Pebble", "Soot", "Mossy", "Nugget", "Bramble", "Ember", "Tuft", "Biscuit", "Flint", "Pickle", "Cinder", "Gravel", "Whisk", "Tumble", "Rusty",
    "Smudge", "Clover", "Boulder", "Sprocket", "Mallet",
];

const CAT_ANTICS: [&str; 5] = [
    "caught a rat and left it on {}'s pillow",
    "knocked {}'s mug off the table, slowly, while looking at them",
    "slept all day in {}'s spot by the forge",
    "followed {} into the mine and got stuck on a ledge",
    "brought {} a live cave beetle",
];

const DOG_ANTICS: [&str; 5] = [
    "stole a sausage from the kitchen and shared it with {}",
    "dug up a bone and buried it again in {}'s bed",
    "howled at the moon all night beside {}",
    "fetched {}'s pick, and then would not give it back",
    "chased its own tail until {} laughed",
];

/// What any other pet gets up to.
const BEAST_ANTICS: [&str; 4] = [
    "followed {} about all day",
    "would eat only from {}'s hand",
    "was found asleep in {}'s boots",
    "hid from everyone but {}",
];

impl Game {
    pub fn pet_of(&self, dwarf: u32) -> Option<&Creature> {
        self.creatures.values().find(|c| c.alive && c.pet_of == Some(dwarf))
    }

    /// Daily: cats choose their dwarves, dogs are taken in, and pets get up
    /// to things.
    pub(super) fn pets_daily(&mut self) {
        let tick = self.tick;
        let ids: Vec<u32> = self.creatures.keys().copied().collect();
        for id in ids {
            let Some(c) = self.creatures.get(&id) else { continue };
            let petlike = matches!(c.species, Species::Cat | Species::Dog) || c.species.beast().map_or(false, |b| b.has(crate::bestiary::PETLIKE));
            if !c.alive || !c.tame || !c.adult || !petlike {
                continue;
            }
            match c.pet_of {
                None if c.owner.is_none() && c.pasture.is_none() && self.rng.chance(0.12) => self.adopt(id),
                Some(owner) => {
                    let Some(oi) = self.dwarves.iter().position(|d| d.id == owner && d.alive) else {
                        let name = c.title();
                        let who = self.dwarves.iter().find(|d| d.id == owner).map_or("their dwarf".to_string(), |d| d.called());
                        self.creatures.get_mut(&id).unwrap().pet_of = None;
                        self.gossip(format!("{} waits by the door for {}, who will not come back.", name, who));
                        continue;
                    };
                    let (pname, cat) = (c.title(), c.species == Species::Cat);
                    let near_owner = near(c.pos, self.dwarves[oi].pos, 4);
                    if near_owner {
                        self.dwarves[oi].think(&format!("played with {}", pname), -3, tick);
                    }
                    if self.rng.chance(0.2) {
                        let oname = self.dwarves[oi].called();
                        let beast = matches!(self.creatures.get(&id).map(|c| c.species), Some(Species::Beast(_)));
                        let antic = if beast { *self.rng.pick(&BEAST_ANTICS) } else if cat { *self.rng.pick(&CAT_ANTICS) } else { *self.rng.pick(&DOG_ANTICS) };
                        self.gossip(format!("{} {}.", pname, antic.replace("{}", &oname)));
                    }
                }
                _ => {}
            }
        }
    }

    fn adopt(&mut self, id: u32) {
        let tick = self.tick;
        let free: Vec<usize> = (0..self.dwarves.len())
            .filter(|&i| {
                let d = &self.dwarves[i];
                d.alive && !d.insane && !d.is_child() && self.pet_of(d.id).is_none()
            })
            .collect();
        if free.is_empty() {
            return;
        }
        let i = free[self.rng.range(0, free.len() as i32) as usize];
        let name = *self.rng.pick(&NAMES);
        let c = self.creatures.get_mut(&id).unwrap();
        c.pet_of = Some(self.dwarves[i].id);
        let cat = c.species == Species::Cat;
        let label = format!("{} the {}", name, c.name());
        c.label = Some(label.clone());
        let d = &mut self.dwarves[i];
        d.think(&format!("took {} as a pet", label), -10, tick);
        let text = if cat { format!("{} has adopted {}.", label, d.called()) } else { format!("{} has taken in {} as a pet.", d.called(), label) };
        self.msg(&text, (200, 230, 160));
        self.gossip(text);
    }

    /// A pet keeps near its dwarf. True if it had somewhere to go.
    pub(super) fn pet_follow(&mut self, c: &mut Creature) -> bool {
        let Some(owner) = c.pet_of else { return false };
        let Some(opos) = self.dwarves.iter().find(|d| d.id == owner && d.alive).map(|d| d.pos) else { return false };
        // Look for the way now and then, not every step.
        if !near(opos, c.pos, 3) && c.move_cd == 0 && c.path.is_empty() && (self.tick + c.id as u64) % 40 == 0 {
            match path::astar(&self.world, c.pos, opos, 2000, Mover::Dwarf) {
                // Follow part of the way, then look again.
                Some(p) => c.path = p.into_iter().rev().collect(),
                // Out of reach: give up for a while.
                None => {
                    c.move_cd = 120;
                    return false;
                }
            }
        }
        if c.path.is_empty() {
            return false;
        }
        let delay = c.species.info().move_delay;
        self.creature_step(c, delay);
        true
    }

    /// A pet has died: its dwarf grieves.
    pub(super) fn pet_died(&mut self, c: &Creature) {
        let Some(owner) = c.pet_of else { return };
        let tick = self.tick;
        if let Some(d) = self.dwarves.iter_mut().find(|d| d.id == owner && d.alive) {
            d.think(&format!("lost their beloved pet {}", c.title()), 40, tick);
            let text = format!("{}, {}'s pet, has died.", c.title(), d.called());
            self.msg(&text, (200, 160, 160));
            self.gossip(text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cats_choose_their_dwarves_and_are_mourned() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame && c.species == Species::Cat);
        let cat = *g.creatures.keys().next().expect("a cat came on the wagon");
        g.creatures.get_mut(&cat).unwrap().adult = true;
        for _ in 0..200 {
            g.pets_daily();
            if g.creatures[&cat].pet_of.is_some() {
                break;
            }
        }
        let owner = g.creatures[&cat].pet_of.expect("the cat never chose anyone");
        assert!(g.creatures[&cat].label.as_deref().unwrap().ends_with("the cat"));
        assert!(g.pet_of(owner).is_some());
        for _ in 0..40 {
            g.pets_daily();
        }
        assert!(g.gossip.len() >= 2, "the cat should get up to something: {:?}", g.gossip);
        let c = g.creatures.remove(&cat).unwrap();
        g.creature_died(c);
        let d = g.dwarves.iter().find(|d| d.id == owner).unwrap();
        assert!(d.thoughts.iter().any(|t| t.text.starts_with("lost their beloved pet")));
    }
}
