//! What an expedition brings: seven founders with their main labors and
//! skills, and the wagon's supplies and animals. "Play now" takes the
//! standard loadout; "Prepare carefully" spends a budget of points on a
//! different one, in the manner of the old game's embark screen.

use crate::creature::Species;
use crate::dwarf::{Labor, LABORS};
use crate::items::ItemKind;
use crate::material::Mat;
use serde::{Deserialize, Serialize};

/// Points a careful preparation may spend.
pub const BUDGET: i32 = 400;
/// The highest skill level points can buy (Proficient).
pub const MAX_LEVEL: u32 = 5;
/// Skill levels of a standard founder: main labor, then the second.
pub const STANDARD_LEVELS: [u32; 2] = [2, 1];

/// One kind of supply: what, of what, its name, the standard count, how
/// many come at a time and what that many costs.
pub struct Supply {
    pub kind: ItemKind,
    pub mat: Mat,
    pub name: &'static str,
    pub standard: u32,
    pub step: u32,
    pub cost: i32,
}

const fn supply(kind: ItemKind, mat: Mat, name: &'static str, standard: u32, step: u32, cost: i32) -> Supply {
    Supply { kind, mat, name, standard, step, cost }
}

/// Everything the wagon can carry, in the order it is unloaded.
pub const SUPPLIES: [Supply; 14] = [
    supply(ItemKind::Plant, Mat::Stoutcap, "plump stoutcaps (food)", 30, 5, 5),
    supply(ItemKind::Drink, Mat::Stoutcap, "stoutcap wine (drink)", 40, 5, 5),
    supply(ItemKind::Log, Mat::Wood, "logs", 6, 1, 3),
    supply(ItemKind::Axe, Mat::Copper, "copper axes", 1, 1, 10),
    supply(ItemKind::Pick, Mat::Copper, "copper picks", 1, 1, 10),
    supply(ItemKind::Seed, Mat::Stoutcap, "stoutcap spawn", 15, 5, 5),
    supply(ItemKind::Seed, Mat::Hollowgrain, "hollowgrain seeds", 5, 5, 5),
    supply(ItemKind::Seed, Mat::Threadreed, "threadreed seeds", 5, 5, 5),
    supply(ItemKind::Seed, Mat::Sunberry, "sunberry seeds", 5, 5, 5),
    supply(ItemKind::Seed, Mat::Goldoat, "goldoat seeds", 5, 5, 5),
    supply(ItemKind::Barrel, Mat::Wood, "barrels", 2, 1, 3),
    supply(ItemKind::Block, Mat::Granite, "granite blocks", 0, 5, 5),
    supply(ItemKind::Cloth, Mat::Threadreed, "threadreed cloth", 0, 1, 3),
    supply(ItemKind::Thread, Mat::Threadreed, "threadreed thread", 0, 1, 2),
];

/// Animals: species, name, standard count, cost each.
pub const ANIMALS: [(Species, &str, u32, i32); 3] = [(Species::Dog, "dogs", 2, 10), (Species::Goat, "goats", 2, 12), (Species::Cat, "cats", 1, 8)];

/// The standard seven: each one's main labor and a second.
const CREWS: [[Labor; 2]; 7] = [
    [Labor::Mining, Labor::Crafting],
    [Labor::Mining, Labor::Smelting],
    [Labor::Woodcutting, Labor::Carpentry],
    [Labor::Carpentry, Labor::Butchery],
    [Labor::Masonry, Labor::Smithing],
    [Labor::Farming, Labor::Cooking],
    [Labor::Brewing, Labor::Farming],
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Founder {
    /// Main labor first.
    pub labors: [Labor; 2],
    /// Skill levels (as `dwarf::skill_level` counts them) in each; None
    /// rolls them the usual way.
    #[serde(default)]
    pub levels: Option<[u32; 2]>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Loadout {
    pub founders: Vec<Founder>,
    /// How many of each `SUPPLIES` entry, in that order.
    pub supplies: Vec<u32>,
    /// How many of each `ANIMALS` entry.
    pub animals: Vec<u32>,
}

impl Default for Loadout {
    fn default() -> Self {
        Loadout::standard()
    }
}

/// What a skill level costs: each level dearer than the last.
pub fn level_cost(level: u32) -> i32 {
    let l = level as i32;
    5 * l * (l + 1) / 2
}

/// Labors a founder can be picked for (everyone hauls and builds anyway).
pub fn choosable_labors() -> Vec<Labor> {
    LABORS.iter().copied().filter(|l| !matches!(l, Labor::Hauling | Labor::Building)).collect()
}

impl Loadout {
    /// Today's expedition: the usual seven with rolled skills, and the usual wagon.
    pub fn standard() -> Loadout {
        Loadout {
            founders: CREWS.iter().map(|&labors| Founder { labors, levels: None }).collect(),
            supplies: SUPPLIES.iter().map(|s| s.standard).collect(),
            animals: ANIMALS.iter().map(|a| a.2).collect(),
        }
    }

    /// The standard expedition with every skill fixed, to start preparing from.
    pub fn prepared() -> Loadout {
        let mut l = Loadout::standard();
        for f in &mut l.founders {
            f.levels = Some(STANDARD_LEVELS);
        }
        l
    }

    pub fn supply(&self, i: usize) -> u32 {
        self.supplies.get(i).copied().unwrap_or(0)
    }

    pub fn animal(&self, i: usize) -> u32 {
        self.animals.get(i).copied().unwrap_or(0)
    }

    pub fn founder_cost(f: &Founder) -> i32 {
        let [a, b] = f.levels.unwrap_or(STANDARD_LEVELS);
        level_cost(a) + level_cost(b)
    }

    /// Points the whole loadout costs.
    pub fn cost(&self) -> i32 {
        let founders: i32 = self.founders.iter().map(Loadout::founder_cost).sum();
        let supplies: i32 = SUPPLIES.iter().enumerate().map(|(i, s)| (self.supply(i) as i32 * s.cost + s.step as i32 - 1) / s.step as i32).sum();
        let animals: i32 = ANIMALS.iter().enumerate().map(|(i, a)| self.animal(i) as i32 * a.3).sum();
        founders + supplies + animals
    }

    pub fn points_left(&self) -> i32 {
        BUDGET - self.cost()
    }

    /// Supplies to unload, in order: (kind, material, how many).
    pub fn supply_list(&self) -> Vec<(ItemKind, Mat, u32)> {
        SUPPLIES.iter().enumerate().map(|(i, s)| (s.kind, s.mat, self.supply(i))).filter(|s| s.2 > 0).collect()
    }

    /// Animals to bring, in order, and whether each is female: pairs for
    /// breeding (cats the other way round, so a lone cat is a she).
    pub fn animal_list(&self) -> Vec<(Species, bool)> {
        let mut v = Vec::new();
        for (i, a) in ANIMALS.iter().enumerate() {
            for k in 0..self.animal(i) {
                v.push((a.0, (k % 2 == 1) != (a.0 == Species::Cat)));
            }
        }
        v
    }

    /// Raise or lower a founder's skill (`which` 0 main, 1 second) by one level.
    pub fn adjust_skill(&mut self, founder: usize, which: usize, up: bool) {
        if let Some(f) = self.founders.get_mut(founder) {
            let mut lv = f.levels.unwrap_or(STANDARD_LEVELS);
            lv[which] = if up { (lv[which] + 1).min(MAX_LEVEL) } else { lv[which].saturating_sub(1) };
            f.levels = Some(lv);
        }
    }

    /// Give a founder the next (or previous) labor in place of one of theirs.
    pub fn cycle_labor(&mut self, founder: usize, which: usize, forward: bool) {
        let all = choosable_labors();
        if let Some(f) = self.founders.get_mut(founder) {
            let other = f.labors[1 - which];
            let mut i = all.iter().position(|&l| l == f.labors[which]).unwrap_or(0);
            // Skip the founder's other labor, and the one it can't be combined with.
            for _ in 0..all.len() {
                i = if forward { (i + 1) % all.len() } else { (i + all.len() - 1) % all.len() };
                let l = all[i];
                if l != other && l.exclusive_with() != Some(other) {
                    break;
                }
            }
            f.labors[which] = all[i];
        }
    }

    /// Add or take away one step of supply `i`.
    pub fn adjust_supply(&mut self, i: usize, up: bool) {
        if i >= SUPPLIES.len() {
            return;
        }
        self.supplies.resize(SUPPLIES.len(), 0);
        let s = &SUPPLIES[i];
        let n = &mut self.supplies[i];
        *n = if up { (*n + s.step).min(s.step * 40) } else { n.saturating_sub(s.step) };
    }

    pub fn adjust_animal(&mut self, i: usize, up: bool) {
        if i >= ANIMALS.len() {
            return;
        }
        self.animals.resize(ANIMALS.len(), 0);
        let n = &mut self.animals[i];
        *n = if up { (*n + 1).min(20) } else { n.saturating_sub(1) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_standard_loadout_fits_the_budget_with_some_to_spare() {
        let l = Loadout::standard();
        assert_eq!(l.cost(), Loadout::prepared().cost());
        assert!(l.points_left() > 0 && l.points_left() < 100, "{} left", l.points_left());
        assert_eq!(l.animal_list(), vec![(Species::Dog, false), (Species::Dog, true), (Species::Goat, false), (Species::Goat, true), (Species::Cat, true)]);
        // A loadout saved without some fields reads back as the standard one.
        let back: Loadout = serde_json::from_str("{}").unwrap();
        assert_eq!(back, l);
    }

    #[test]
    fn skills_and_supplies_cost_points() {
        let mut l = Loadout::prepared();
        let before = l.points_left();
        l.adjust_skill(0, 0, true);
        assert_eq!(l.points_left(), before - (level_cost(3) - level_cost(2)));
        l.adjust_supply(0, false);
        assert_eq!(l.supply(0), 25);
        l.adjust_animal(0, true);
        l.cycle_labor(0, 1, true);
        assert_ne!(l.founders[0].labors[1], Labor::Crafting);
        // A miner never becomes a woodcutter as well.
        let mut m = Loadout::prepared();
        for _ in 0..40 {
            m.cycle_labor(0, 1, true);
            assert!(m.founders[0].labors[1] != Labor::Woodcutting && m.founders[0].labors[1] != Labor::Mining);
        }
    }
}
