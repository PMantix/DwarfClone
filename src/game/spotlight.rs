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
    /// A saga's card (`sagas.rs`), by its id.
    pub saga: Option<u32>,
    /// Commands the card offers, each on its key (O01, `commands.rs`).
    pub actions: Vec<super::commands::Command>,
}

impl Game {
    /// Stop and look: something that matters has happened.
    pub(super) fn spotlight(&mut self, title: &str, text: &str, at: Option<Pos>, color: (u8, u8, u8)) {
        // Every big moment goes on the highlight reel (reel.rs).
        self.film(title, text, at);
        // A flurry of the same kind (a dozen undead rising) is one moment.
        if self.spotlights.iter().any(|s| s.title == title) {
            return;
        }
        self.spotlights.push(Spotlight { title: title.into(), text: text.into(), at, color, saga: None, actions: Vec::new() });
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
    /// Where the land has none, one is levelled beside the wagon (with a
    /// level way to it), so a test never hangs on the lie of the land.
    #[cfg(test)]
    pub fn flat_patch(&mut self, w: i32, h: i32, away: i32) -> Pos {
        self.patch_of(w, h, away, false).unwrap_or_else(|| self.level_patch(w, h, away))
    }

    /// Tests: like `flat_patch`, but every tile open ground (no trees).
    #[cfg(test)]
    pub fn open_patch(&mut self, w: i32, h: i32, away: i32) -> Pos {
        self.patch_of(w, h, away, true).unwrap_or_else(|| self.level_patch(w, h, away))
    }

    /// Tests: make the columns of a box level, open ground at `z` (filled
    /// with soil below, cleared above).
    #[cfg(test)]
    pub fn level_ground(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, z: i32) {
        for y in y0.max(0)..=y1.min(self.world.h - 1) {
            for x in x0.max(0)..=x1.min(self.world.w - 1) {
                for k in 1..self.world.d {
                    let t = self.world.t_mut(Pos::new(x, y, k));
                    t.water = 0;
                    t.magma = 0;
                    t.frozen = false;
                    if k < z {
                        if !t.kind.solid() {
                            t.kind = TileKind::Wall;
                            t.mat = crate::material::Mat::Loam;
                        }
                    } else if k == z {
                        t.kind = TileKind::Floor;
                        t.mat = crate::material::Mat::Grass;
                        t.revealed = true;
                    } else {
                        t.kind = TileKind::Open;
                        t.mat = crate::material::Mat::Air;
                        t.revealed = true;
                    }
                }
                let i = (y * self.world.w + x) as usize;
                self.world.surface[i] = z;
            }
        }
        self.water_cells = None;
        self.dig_version += 1;
    }

    /// Level a `w`×`h` patch `away` tiles off the wagon, and a way to it.
    #[cfg(test)]
    fn level_patch(&mut self, w: i32, h: i32, away: i32) -> Pos {
        let e = self.embark;
        let fits = |x: i32, y: i32, g: &Game| x >= 1 && y >= 1 && x + w < g.world.w - 1 && y + h < g.world.h - 1;
        let tries = [(away, -h / 2), (-away - w + 1, -h / 2), (-w / 2, away), (-w / 2, -away - h + 1)];
        // Off to one side if it fits, else as near that as the map allows.
        let (dx, dy) = tries.iter().copied().find(|&(dx, dy)| fits(e.x + dx, e.y + dy, self)).unwrap_or_else(|| {
            let (gw, gh) = (self.world.w, self.world.h);
            let (x, y) = ((e.x + away).clamp(1, (gw - w - 2).max(1)), (e.y - h / 2).clamp(1, (gh - h - 2).max(1)));
            (x - e.x, y - e.y)
        });
        let a = Pos::new(e.x + dx, e.y + dy, e.z);
        self.level_ground(a.x, a.y, a.x + w - 1, a.y + h - 1, e.z);
        // The way there, three wide.
        if dx.abs() >= dy.abs() {
            let bx = if dx > 0 { a.x } else { a.x + w - 1 };
            self.level_ground(e.x.min(bx), e.y - 1, e.x.max(bx), e.y + 1, e.z);
        } else {
            let by = if dy > 0 { a.y } else { a.y + h - 1 };
            self.level_ground(e.x - 1, e.y.min(by), e.x + 1, e.y.max(by), e.z);
        }
        a
    }

    #[cfg(test)]
    fn patch_of(&self, w: i32, h: i32, away: i32, all_open: bool) -> Option<Pos> {
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
                        return Some(a);
                    }
                }
            }
        }
        None
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
