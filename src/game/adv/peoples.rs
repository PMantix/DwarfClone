//! How the world's peoples see the hero (decision 1, plan "Peoples"): the
//! stance of a site's folk to a hero of a race and civ. Sites (A4) give
//! occupants their faction by it, encounters (A3) pick patrols by it,
//! prices (A15) and rumours (A7) read it.

use crate::overworld::{Overworld, Race, Site};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stance {
    /// The hero's own civ.
    Home,
    Friendly,
    /// Served, but watched; told little.
    Wary,
    /// Guards come for them.
    Hostile,
}

/// How a site's folk take a hero of `race` from civ `civ`.
pub fn stance(ow: &Overworld, race: Race, civ: Option<u32>, site: &Site) -> Stance {
    let Some(sc) = site.civ else { return Stance::Wary };
    if Some(sc) == civ {
        return Stance::Home;
    }
    let theirs = ow.civs.get(sc as usize);
    let at_war = |a: u32, b: u32| ow.civs.get(a as usize).map_or(false, |c| c.enemies.contains(&b));
    if let Some(c) = civ {
        if at_war(sc, c) || at_war(c, sc) {
            return Stance::Hostile;
        }
    }
    match (race, theirs.map(|c| c.race)) {
        (Race::Goblin, Some(Race::Goblin)) => Stance::Friendly,
        (Race::Goblin, _) | (_, Some(Race::Goblin)) => Stance::Hostile,
        (Race::Dwarf | Race::Human, Some(Race::Elf)) => Stance::Wary,
        _ => Stance::Friendly,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goblins_are_unwelcome_and_home_is_home() {
        let ow = crate::game::Game::new(5).overworld;
        let living: Vec<&Site> = ow.sites.iter().filter(|s| !s.ruined && s.civ.is_some()).collect();
        let s = living[0];
        let race = ow.civs[s.civ.unwrap() as usize].race;
        assert_eq!(stance(&ow, race, s.civ, s), Stance::Home);
        if race != Race::Goblin {
            assert_eq!(stance(&ow, Race::Goblin, None, s), Stance::Hostile);
        }
    }
}
