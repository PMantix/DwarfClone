//! The big moments: a siege at the gates, a dragon, a hero struck down,
//! traders at the depot. Each pauses the game (unless the overseer is
//! playing), turns the camera to where it happened and tells the story in
//! a card over the map.

use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Spotlight {
    pub title: String,
    pub text: String,
    /// Where to look, if anywhere in particular.
    pub at: Option<Pos>,
    pub color: (u8, u8, u8),
}

impl Game {
    /// Stop and look: something that matters has happened.
    pub(super) fn spotlight(&mut self, title: &str, text: &str, at: Option<Pos>, color: (u8, u8, u8)) {
        // A flurry of the same kind (a dozen undead rising) is one moment.
        if self.spotlights.iter().any(|s| s.title == title) {
            return;
        }
        self.spotlights.push(Spotlight { title: title.into(), text: text.into(), at, color });
        self.pause_request = true;
    }

    #[cfg(test)]
    pub fn spotlight_for_test(&mut self, title: &str, text: &str) {
        self.spotlight(title, text, None, (255, 255, 255));
    }

    #[cfg(test)]
    pub fn kill_for_test(&mut self, d: &mut Dwarf, cause: &str) {
        self.kill(d, cause);
    }

    /// Tests: a dry, level patch of open ground `w`×`h` near the wagon (at
    /// least `away` tiles off), that a dwarf can walk to. Its top-left corner.
    #[cfg(test)]
    pub fn flat_patch(&self, w: i32, h: i32, away: i32) -> Pos {
        self.patch_of(w, h, away, false)
    }

    /// Tests: like `flat_patch`, but every tile open ground (no trees).
    #[cfg(test)]
    pub fn open_patch(&self, w: i32, h: i32, away: i32) -> Pos {
        self.patch_of(w, h, away, true)
    }

    #[cfg(test)]
    fn patch_of(&self, w: i32, h: i32, away: i32, all_open: bool) -> Pos {
        let e = self.embark;
        for r in away..45 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let a = e.offset(dx, dy, 0);
                    if a.x < 1 || a.y < 1 || a.x + w >= self.world.w || a.y + h >= self.world.h {
                        continue;
                    }
                    let z = self.world.surface_z(a.x, a.y);
                    // Level and dry throughout; a few trees or shrubs are fine.
                    let tiles: Vec<Pos> = (0..h).flat_map(|j| (0..w).map(move |i| Pos::new(a.x + i, a.y + j, z))).collect();
                    let dry = tiles.iter().all(|p| {
                        self.world.surface_z(p.x, p.y) == z
                            && self.world.get(*p).map_or(false, |t| matches!(t.kind, TileKind::Floor | TileKind::Tree | TileKind::Shrub) && t.water == 0 && t.building == 0 && t.zone == 0)
                    });
                    let open = tiles.iter().filter(|p| self.world.t(**p).kind == TileKind::Floor).count();
                    let enough = if all_open { open == tiles.len() } else { open * 4 >= tiles.len() * 3 };
                    let level = dry && enough && self.world.t(Pos::new(a.x, a.y, z)).kind == TileKind::Floor;
                    let a = Pos::new(a.x, a.y, z);
                    if level && crate::path::astar(&self.world, e, a, 30_000, crate::world::Mover::Dwarf).is_some() {
                        return a;
                    }
                }
            }
        }
        panic!("no flat patch {}x{} near the wagon", w, h);
    }

    /// The next moment to show, oldest first.
    pub fn take_spotlight(&mut self) -> Option<Spotlight> {
        if self.spotlights.is_empty() {
            None
        } else {
            Some(self.spotlights.remove(0))
        }
    }
}
