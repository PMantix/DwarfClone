//! The gods of the world: each civilization worships a few, each god has a
//! sphere or two (fire, fortune, the deep, death...), a thing it forbids,
//! and a line of myth. Made once the history is written, from a random
//! stream of its own so the world and its history come out the same.

use crate::overworld::{Overworld, Race};
use crate::rng::Rng;
use serde::{Deserialize, Serialize};

/// What a god is god of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Sphere {
    Fire,
    Fortune,
    Deep,
    Death,
    Craft,
    War,
    Love,
    Harvest,
    Stone,
    Water,
    Night,
    Beasts,
    Song,
}

impl Sphere {
    pub fn name(self) -> &'static str {
        match self {
            Sphere::Fire => "fire",
            Sphere::Fortune => "fortune",
            Sphere::Deep => "the deep",
            Sphere::Death => "death",
            Sphere::Craft => "craft",
            Sphere::War => "war",
            Sphere::Love => "love",
            Sphere::Harvest => "the harvest",
            Sphere::Stone => "stone",
            Sphere::Water => "water",
            Sphere::Night => "night",
            Sphere::Beasts => "beasts",
            Sphere::Song => "song",
        }
    }

    /// Titles a god of this sphere goes by.
    fn titles(self) -> &'static [&'static str] {
        match self {
            Sphere::Fire => &["the Ember", "the Flame-Crowned", "Forgeheart"],
            Sphere::Fortune => &["the Lucky", "the Coin-Turner", "of the Thrown Dice"],
            Sphere::Deep => &["the Delver", "of the Lightless Halls", "the Root of Mountains"],
            Sphere::Death => &["the Pale", "the Last Door", "Keeper of Tombs"],
            Sphere::Craft => &["the Maker", "the Hammer", "of the Steady Hand"],
            Sphere::War => &["the Axe", "the Shieldbreaker", "the Red"],
            Sphere::Love => &["the Kind", "the Hearth-Mother", "of Joined Hands"],
            Sphere::Harvest => &["the Sower", "of the Full Barrel", "the Green"],
            Sphere::Stone => &["the Unmoved", "the Mountain", "of the Grey Seam"],
            Sphere::Water => &["the Drowned", "of the Cold Springs", "the Tide"],
            Sphere::Night => &["the Veiled", "of the Long Dark", "Moonwatcher"],
            Sphere::Beasts => &["the Antlered", "of the Wild Hunt", "the Pack-Mother"],
            Sphere::Song => &["the Singer", "of the Sweet Pipes", "the Echo"],
        }
    }

    /// What a god of this sphere did in the oldest days ({} is the name).
    fn myth(self) -> &'static [&'static str] {
        match self {
            Sphere::Fire => &[
                "{} struck the first spark from the anvil of the world, and the mountains keep its embers still.",
                "{} breathed on the cold stone until it ran red, and taught the first smith to catch it.",
            ],
            Sphere::Fortune => &[
                "{} won the world in a game of bones from a brother who never forgave the loss.",
                "{} hides a gem in every hundredth stone, so that miners will never stop digging.",
            ],
            Sphere::Deep => &[
                "{} dug the caverns with bare hands, looking for the bottom of the world.",
                "{} sleeps under the magma sea, and the earth shakes when {} turns over.",
            ],
            Sphere::Death => &[
                "{} keeps the door at the end of every tunnel, and counts all who pass.",
                "{} carved the first tomb, and lay down in it to show the dead how to rest.",
            ],
            Sphere::Craft => &[
                "{} hammered the first dwarves out of iron and set them on their feet.",
                "{} made the first mug, and then the first ale to fill it.",
            ],
            Sphere::War => &[
                "{} broke a giant's shield with a single blow and hung the pieces in the sky.",
                "{} has never lost a battle, and never forgiven one fled from.",
            ],
            Sphere::Love => &[
                "{} wove the first two hearts together with a thread of gold.",
                "{} weeps at every wedding, and the tears are found as gems.",
            ],
            Sphere::Harvest => &[
                "{} sowed the first seed in a crack of the rock, and the stoutcap grew there.",
                "{} fills the barrels in the dark of winter, if the barrels are kept clean.",
            ],
            Sphere::Stone => &[
                "{} lay down at the making of the world and became the mountains.",
                "{} has not moved since the first age, and holds up the roofs of all halls.",
            ],
            Sphere::Water => &[
                "{} wept the rivers out of the mountains, and the fish are the tears that lived.",
                "{} drowned the first flood in a cup, and drank it.",
            ],
            Sphere::Night => &[
                "{} stole half of every day and hid it under the mountains.",
                "{} lit the stars so the lost would find their way home.",
            ],
            Sphere::Beasts => &[
                "{} ran with the first wolves and named every creature that could be caught.",
                "{} sleeps in the oldest tree of the forest, and wakes when it is felled.",
            ],
            Sphere::Song => &[
                "{} sang the world awake, and it has been humming ever since.",
                "{} taught the first bard the tune that makes stone weep.",
            ],
        }
    }

    /// What a god of this sphere is likeliest to forbid.
    fn taboo(self) -> Taboo {
        match self {
            Sphere::Water | Sphere::Beasts | Sphere::Night => Taboo::EatingFish,
            Sphere::Harvest | Sphere::Love | Sphere::Song | Sphere::Death => Taboo::FellingTrees,
            _ => Taboo::BloodInTemple,
        }
    }
}

/// Something a god forbids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Taboo {
    /// Cutting down a living tree.
    FellingTrees,
    /// Blood shed on the floor of a temple.
    BloodInTemple,
    /// Eating fish.
    EatingFish,
}

impl Taboo {
    pub fn name(self) -> &'static str {
        match self {
            Taboo::FellingTrees => "felling living trees",
            Taboo::BloodInTemple => "spilling blood in the temple",
            Taboo::EatingFish => "eating fish",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct God {
    pub id: u32,
    /// "Ral".
    pub name: String,
    /// "the Ember".
    pub title: String,
    pub spheres: Vec<Sphere>,
    pub female: bool,
    pub taboo: Taboo,
    /// The civilizations that worship it.
    pub civs: Vec<u32>,
    /// A line of the oldest stories.
    pub myth: String,
}

impl God {
    /// "Ral the Ember".
    pub fn called(&self) -> String {
        format!("{} {}", self.name, self.title)
    }

    /// "fire and craft".
    pub fn spheres_text(&self) -> String {
        let n: Vec<&str> = self.spheres.iter().map(|s| s.name()).collect();
        match n.len() {
            0 => "nothing".into(),
            1 => n[0].to_string(),
            k => format!("{} and {}", n[..k - 1].join(", "), n[k - 1]),
        }
    }

    /// "goddess of fire and craft".
    pub fn describe(&self) -> String {
        format!("{} of {}", if self.female { "goddess" } else { "god" }, self.spheres_text())
    }

    pub fn has(&self, s: Sphere) -> bool {
        self.spheres.contains(&s)
    }
}

/// The spheres each people leans to, likeliest first.
fn leanings(r: Race) -> &'static [Sphere] {
    match r {
        Race::Dwarf => &[Sphere::Craft, Sphere::Stone, Sphere::Deep, Sphere::Fire, Sphere::Fortune, Sphere::War, Sphere::Death, Sphere::Harvest, Sphere::Love],
        Race::Human => &[Sphere::Harvest, Sphere::War, Sphere::Love, Sphere::Fortune, Sphere::Water, Sphere::Death, Sphere::Song],
        Race::Elf => &[Sphere::Beasts, Sphere::Song, Sphere::Love, Sphere::Water, Sphere::Night, Sphere::Harvest],
        Race::Goblin => &[Sphere::War, Sphere::Death, Sphere::Night, Sphere::Fire],
    }
}

/// The gods of every civilization in the world.
pub fn pantheon(ow: &Overworld) -> Vec<God> {
    let mut rng = Rng::new(ow.seed ^ 0x6D79_7468_6F73);
    let mut gods: Vec<God> = Vec::new();
    for civ in &ow.civs {
        // A people a villain made kneels to the dark god its maker served
        // (on a stream of its own, so the other peoples' gods come out the
        // same whether or not it was made).
        if ow.made_of(civ.id).is_some() {
            gods.extend(made_god(ow, civ.id, gods.len() as u32));
            continue;
        }
        let lean = leanings(civ.race);
        let n = match civ.race {
            Race::Dwarf => 3 + rng.chance(0.5) as usize,
            Race::Goblin => 1 + rng.chance(0.5) as usize,
            _ => 2 + rng.chance(0.4) as usize,
        };
        let mut used: Vec<Sphere> = Vec::new();
        for _ in 0..n {
            // The first spheres of a people's leanings come up most.
            let pick = |rng: &mut Rng, used: &[Sphere]| -> Option<Sphere> {
                let free: Vec<Sphere> = lean.iter().copied().filter(|s| !used.contains(s)).collect();
                if free.is_empty() {
                    return None;
                }
                let a = rng.range(0, free.len() as i32) as usize;
                let b = rng.range(0, free.len() as i32) as usize;
                Some(free[a.min(b)])
            };
            let Some(first) = pick(&mut rng, &used) else { break };
            used.push(first);
            let mut spheres = vec![first];
            if rng.chance(0.6) {
                if let Some(second) = pick(&mut rng, &used) {
                    used.push(second);
                    spheres.push(second);
                }
            }
            let name = crate::overworld::person_name(&mut rng, civ.race).split(' ').next().unwrap_or("Nameless").to_string();
            let title = rng.pick(first.titles()).to_string();
            let taboo = if rng.chance(0.75) { first.taboo() } else { *rng.pick(&[Taboo::FellingTrees, Taboo::BloodInTemple, Taboo::EatingFish]) };
            let myth = rng.pick(first.myth()).replace("{}", &name);
            gods.push(God { id: gods.len() as u32, name, title, spheres, female: rng.chance(0.5), taboo, civs: vec![civ.id], myth });
        }
    }
    gods
}

/// The dark god of a people a villain made: god of night or death (and
/// perhaps war or fire), who whispered to their maker in the tower.
pub fn made_god(ow: &Overworld, civ: u32, id: u32) -> Option<God> {
    let m = ow.made_of(civ)?;
    let c = ow.civs.get(civ as usize)?;
    let mut rng = Rng::new(ow.seed ^ 0x6D61_6465_676F_64 ^ (civ as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let first = *rng.pick(&[Sphere::Night, Sphere::Death]);
    let mut spheres = vec![first];
    if rng.chance(0.6) {
        spheres.push(*rng.pick(&[Sphere::War, Sphere::Fire]));
    }
    let name = crate::overworld::person_name(&mut rng, c.race).split(' ').next().unwrap_or("Nameless").to_string();
    let title = rng.pick(first.titles()).to_string();
    let maker = ow.figures.get(m.maker as usize).and_then(|f| f.name.split(' ').next()).unwrap_or("their maker");
    let myth = format!("{} whispered to {} in the dark of the tower, and the {} were made in {}'s image.", name, maker, m.plural, name);
    Some(God { id, name, title, spheres, female: rng.chance(0.5), taboo: first.taboo(), civs: vec![civ], myth })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_people_has_gods_with_spheres_and_a_taboo() {
        let ow = Overworld::generate(5003);
        assert!(!ow.gods.is_empty(), "no gods");
        for c in &ow.civs {
            let theirs: Vec<&God> = ow.gods.iter().filter(|g| g.civs.contains(&c.id)).collect();
            assert!(!theirs.is_empty(), "{} has no gods", c.name);
            if c.race == Race::Dwarf {
                assert!(theirs.len() >= 3);
            }
        }
        for g in &ow.gods {
            assert!(!g.spheres.is_empty() && g.spheres.len() <= 2);
            assert!(g.myth.contains(&g.name), "{}", g.myth);
            assert!(g.describe().starts_with("god"), "{}", g.describe());
        }
        // The same world makes the same gods, and the history is untouched.
        let again = Overworld::generate(5003);
        assert_eq!(again.gods, ow.gods);
        println!("{}", ow.gods.iter().map(|g| format!("{}, {} (forbids {}): {}", g.called(), g.describe(), g.taboo.name(), g.myth)).collect::<Vec<_>>().join("\n"));
    }
}
