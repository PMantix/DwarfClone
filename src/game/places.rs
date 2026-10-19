//! Named places (WLD-02, ENG-16): one store for every name a part of the
//! fort carries, kept in `Game.stories.places` beside the battlefields
//! `stories.rs` names. A place is the ground within `r` of `at`, or a box
//! from `at` to `b`; it is renamed only for a heavier deed, and never once
//! the player has named it.
//!
//! (The overseer's room titles, "the Great Dining Hall", are worked out
//! afresh each day in `architect.rs: place_name`; a name given here
//! outlasts them.)

// Wave 0 interfaces for the wave-1 packages: some are not called yet.
#![allow(dead_code)]

use super::stories::Place;
use super::*;
use serde::{Deserialize, Serialize};

/// What sort of place.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaceKind {
    Hall,
    Gate,
    #[default]
    Battlefield,
    Tomb,
    Deep,
    Road,
    Work,
}

fn in_box(a: Pos, b: Pos, p: Pos) -> bool {
    p.x >= a.x.min(b.x) && p.x <= a.x.max(b.x) && p.y >= a.y.min(b.y) && p.y <= a.y.max(b.y) && p.z >= a.z.min(b.z) && p.z <= a.z.max(b.z)
}

fn covers(pl: &Place, p: Pos) -> bool {
    match pl.b {
        Some(b) => in_box(pl.at, b, p),
        None => pl.at.z == p.z && (pl.at.x - p.x).abs() <= pl.r && (pl.at.y - p.y).abs() <= pl.r,
    }
}

fn area(pl: &Place) -> i64 {
    match pl.b {
        Some(b) => ((pl.at.x - b.x).abs() as i64 + 1) * ((pl.at.y - b.y).abs() as i64 + 1) * ((pl.at.z - b.z).abs() as i64 + 1),
        None => (2 * pl.r as i64 + 1).pow(2),
    }
}

impl Game {
    /// The named place covering `p`: the smallest, if several do.
    pub fn place_at(&self, p: Pos) -> Option<&Place> {
        self.stories.places.iter().filter(|pl| covers(pl, p)).min_by_key(|pl| area(pl))
    }

    /// Name the place from `a` to `b` (or rename it, if `weight` beats the
    /// old name's and the player didn't name it); returns its index in
    /// `stories.places`. `why` is the deed it is named for ("where …").
    /// A name the player gives (`by_player`) always takes.
    #[allow(clippy::too_many_arguments)]
    pub fn name_place(&mut self, kind: PlaceKind, a: Pos, b: Pos, name: String, why: String, weight: u8, by_player: bool) -> usize {
        let year = self.start_year + (self.tick / YEAR) as i32;
        let same = |pl: &Place| pl.b.map_or(false, |pb| (pl.at, pb) == (a, b) || (pl.at, pb) == (b, a));
        if let Some(i) = self.stories.places.iter().position(same) {
            let pl = &mut self.stories.places[i];
            if by_player || (!pl.by_player && weight > pl.weight) {
                *pl = Place { name, at: a, r: 0, deed: why, year, kind, b: Some(b), weight, by_player };
            }
            return i;
        }
        self.stories.places.push(Place { name, at: a, r: 0, deed: why, year, kind, b: Some(b), weight, by_player });
        self.stories.places.len() - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_heavier_deed_renames_a_place() {
        let mut g = Game::new(5);
        g.stories.places.clear();
        let (a, b) = (Pos::new(10, 10, 5), Pos::new(16, 14, 5));
        let i = g.name_place(PlaceKind::Hall, a, b, "the Hall of Vows".into(), "where Urist wed".into(), 2, false);
        assert_eq!(g.place_at(Pos::new(12, 12, 5)).map(|p| p.name.as_str()), Some("the Hall of Vows"));
        assert!(g.place_at(Pos::new(12, 12, 6)).is_none() && g.place_at(Pos::new(17, 12, 5)).is_none());
        // A lighter deed doesn't rename it; a heavier one does.
        assert_eq!(g.name_place(PlaceKind::Hall, a, b, "the Quiet Hall".into(), String::new(), 1, false), i);
        assert_eq!(g.stories.places[i].name, "the Hall of Vows");
        g.name_place(PlaceKind::Hall, b, a, "the Hall of the Unbeaten".into(), "where the goblin lord fell".into(), 7, false);
        assert_eq!(g.stories.places[i].name, "the Hall of the Unbeaten");
        assert_eq!(g.stories.places.len(), 1);
        // The player's name sticks.
        g.name_place(PlaceKind::Hall, a, b, "Mead Hall".into(), String::new(), 0, true);
        g.name_place(PlaceKind::Hall, a, b, "the Weeping Hall".into(), String::new(), 9, false);
        assert_eq!(g.stories.places[i].name, "Mead Hall");
        // The smallest place wins, and the look panel tells it.
        g.name_place(PlaceKind::Tomb, Pos::new(12, 12, 5), Pos::new(13, 13, 5), "the Quiet Tomb".into(), "where the fallen lie".into(), 3, false);
        assert_eq!(g.place_at(Pos::new(12, 12, 5)).map(|p| p.name.as_str()), Some("the Quiet Tomb"));
        assert!(g.story_place_at(Pos::new(15, 14, 5)).map_or(false, |s| s.starts_with("Mead Hall")));
        // A battle's place (a ring round a spot) isn't taken for a box.
        assert!(g.story_place(Pos::new(12, 12, 5)).is_none());
    }
}
