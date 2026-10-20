//! Names for things (CRF-08): halls, battles, weapons, places, works and
//! automatons, coined from a seed. A name is a hash of its seed, never a
//! draw on the game's random numbers, so naming something changes nothing
//! else that happens. (Battles and the places they broke are named in
//! `stories.rs` with the same hashing; this is the general coiner.)

// Wave 0 interfaces for the wave-1 packages: some are not called yet.
#![allow(dead_code)]

use super::stories;

/// What sort of thing is being named.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameTheme {
    Hall,
    Battle,
    Weapon,
    Place,
    Work,
    Automaton,
}

/// A steady hash of several numbers (for a seed: the fort, the tick, a
/// dwarf's id...).
pub fn hash(parts: &[u64]) -> u64 {
    parts.iter().enumerate().fold(0x51_7cc1_b727_220a, |h, (i, &p)| stories::hash("", h ^ p.wrapping_add(i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)))
}

const ADJ: [&str; 24] = [
    "Amber", "Ashen", "Bright", "Brass", "Deep", "Dusk", "Ember", "Frost", "Gilded", "Glimmer", "Granite", "Hollow", "Iron", "Lantern", "Long",
    "Moss", "Quiet", "Red", "Silver", "Slate", "Steadfast", "Thunder", "Umber", "Wending",
];
const NOUN: [&str; 24] = [
    "Anvil", "Arch", "Beard", "Bell", "Candle", "Cavern", "Crown", "Drum", "Ember", "Forge", "Hammer", "Hearth", "Horn", "Lantern", "Oath",
    "Pillar", "Root", "Seam", "Shield", "Spark", "Stair", "Stone", "Torch", "Vault",
];
const HALL: [&str; 6] = ["the Hall of the {A} {N}", "the {A} {N} Hall", "the {N} Hall of {A} Stone", "the Hall of {A} {N}s", "the {A} Gallery of the {N}", "the {A} {N}-Hall"];
const BATTLE: [&str; 4] = ["the Battle of the {A} {N}", "the {A} Stand at the {N}", "the Fight at the {A} {N}", "the {A} Day of the {N}"];
const WEAPON: [&str; 6] = ["{A}{n}", "{A} {N}-Biter", "the {A} Fang of the {N}", "the {A} {N}", "{N}fall the {A}", "Old {A} {N}"];
const PLACE: [&str; 5] = ["{A} {N}", "the {A} Reach of the {N}", "the {A} {N}'s Rest", "the {A} Deeps by the {N}", "the {A} {N}watch"];
const WORK: [&str; 5] = ["the {A} {N}", "the {N} of {A} Stone", "the Great {A} {N}", "the {A} Wonder of the {N}", "the {A} {N}work"];
const AUTOMATON: [&str; 5] = ["{A} {N}", "the {A} Sentinel of the {N}", "Old {A} {N}", "the {A} {N}-Warden", "{A}-Step the {N}"];

/// A name of the given sort from a seed: the same seed, the same name.
pub fn coin(seed: u64, theme: NameTheme) -> String {
    let forms: &[&str] = match theme {
        NameTheme::Hall => &HALL,
        NameTheme::Battle => &BATTLE,
        NameTheme::Weapon => &WEAPON,
        NameTheme::Place => &PLACE,
        NameTheme::Work => &WORK,
        NameTheme::Automaton => &AUTOMATON,
    };
    let h = hash(&[seed, theme as u64]);
    let a = stories::pick(h, 1, &ADJ);
    let n = stories::pick(h, 2, &NOUN);
    stories::pick(h, 3, forms).replace("{A}", a).replace("{N}", n).replace("{n}", &n.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_stable_and_varied() {
        for t in [NameTheme::Hall, NameTheme::Battle, NameTheme::Weapon, NameTheme::Place, NameTheme::Work, NameTheme::Automaton] {
            assert_eq!(coin(77, t), coin(77, t), "the same seed gives the same name");
            let names: std::collections::HashSet<String> = (0..200u64).map(|s| coin(hash(&[s, 9]), t)).collect();
            assert!(names.len() >= 150, "{:?}: only {} names in 200", t, names.len());
            assert!(names.iter().all(|n| !n.contains('{')), "{:?}", names);
        }
        assert_ne!(hash(&[1, 2]), hash(&[2, 1]));
    }
}
