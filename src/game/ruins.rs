//! Ancient ruins: a sealed hall of an older people lies somewhere in the
//! rock. Mine into it to find its carvings, its treasure, and sometimes
//! the thing that was sealed in with them.

use super::*;
use crate::items::ARTIFACT;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vault {
    pub name: String,
    /// The corners of its floor.
    pub min: Pos,
    pub max: Pos,
    pub opened: bool,
}

impl Vault {
    pub fn center(&self) -> Pos {
        Pos::new((self.min.x + self.max.x) / 2, (self.min.y + self.max.y) / 2, self.min.z)
    }

    #[cfg(test)]
    pub fn contains(&self, p: Pos) -> bool {
        p.z == self.min.z && p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }
}

impl Game {
    /// At embark: seal a hall somewhere in the deep rock.
    /// Bring an older save up to date: it gets a vault of its own.
    pub fn after_load(&mut self) {
        // Older maps don't know their columns' biomes: all the site's own.
        if self.world.biomes.is_empty() {
            let code = self.biome.code() | if self.evil { 0x80 } else { 0 };
            self.world.biomes = vec![code; (self.world.w * self.world.h) as usize];
        }
        if !self.vault_placed {
            self.place_vault();
        }
        if !self.necro_rolled {
            self.place_necromancer();
        }
        // Older forts have no lairs about them yet (lairs.rs): roll them
        // from the site itself, leaving the game's own dice alone.
        if self.lairs.is_empty() {
            let seed = ((self.site.0 as u64) << 40) ^ ((self.site.1 as u64) << 20) ^ ((self.embark.x as u64) << 8) ^ self.embark.y as u64;
            self.make_lairs(seed);
        }
    }

    pub(super) fn place_vault(&mut self) {
        self.vault_placed = true;
        let (w, h) = (7, 5);
        let people = {
            let dead: Vec<String> = self.overworld.civs.iter().filter(|c| !c.alive).map(|c| c.name.clone()).collect();
            if dead.is_empty() {
                "First Folk".to_string()
            } else {
                self.rng.pick(&dead).clone()
            }
        };
        for _ in 0..300 {
            let x = self.rng.range(4, self.world.w - w - 4);
            let y = self.rng.range(4, self.world.h - h - 4);
            let top = (x..x + w).flat_map(|x| (y..y + h).map(move |y| (x, y))).map(|(x, y)| self.world.surface_z(x, y)).min().unwrap_or(0);
            if top < 12 {
                continue;
            }
            let z = self.rng.range(4.max(top - 16), top - 6);
            let min = Pos::new(x, y, z);
            // Solid rock all round, above and below, and no water in it.
            let sound = (-1..=w).all(|dx| {
                (-1..=h).all(|dy| {
                    (-1..=1).all(|dz| {
                        self.world.get(min.offset(dx, dy, dz)).map_or(false, |t| {
                            t.kind == TileKind::Wall && !t.aquifer && t.mat != Mat::Bedrock && !t.revealed && t.water == 0 && t.magma == 0
                        })
                    })
                })
            });
            if !sound {
                continue;
            }
            let ancient: Vec<u32> = self
                .overworld
                .events
                .iter()
                .enumerate()
                .filter(|(_, e)| e.year < self.start_year - 50)
                .map(|(i, _)| i as u32 + 1)
                .collect();
            for dx in -1..=w {
                for dy in -1..=h {
                    let p = min.offset(dx, dy, 0);
                    let inside = dx >= 0 && dx < w && dy >= 0 && dy < h;
                    let scene = if ancient.is_empty() { 0 } else { *self.rng.pick(&ancient) };
                    let t = self.world.t_mut(p);
                    t.mat = Mat::Marble;
                    if inside {
                        t.kind = TileKind::Floor;
                        t.smooth = 1;
                    } else {
                        t.smooth = ENGRAVED + 3;
                        t.engraving = scene;
                    }
                }
            }
            self.vault = Some(Vault { name: format!("the Sealed Hall of the {}", people), min, max: min.offset(w - 1, h - 1, 0), opened: false });
            return;
        }
    }

    /// After a dig: has it broken into the vault?
    pub(super) fn check_vault(&mut self) {
        let Some(v) = self.vault.as_ref() else { return };
        if v.opened || !self.world.t(v.center()).revealed {
            return;
        }
        let v = v.clone();
        self.vault.as_mut().unwrap().opened = true;
        let fort = self.fortress_name.clone();
        self.msg(
            &format!("Your miners have broken into {}! Its walls are carved with scenes of a forgotten age, and treasure glints in the dark.", v.name),
            (255, 215, 60),
        );
        self.legend(format!("The miners of {} broke into {}, sealed since a forgotten age.", fort, v.name));
        self.year_log.ruin = Some(v.name.clone());
        self.milestone("vault", &format!("Broke into {}", v.name));
        self.pause_request = true;
        // Treasure, and one thing of legend.
        let tiles: Vec<Pos> = (v.min.x..=v.max.x).flat_map(|x| (v.min.y..=v.max.y).map(move |y| Pos::new(x, y, v.min.z))).collect();
        let n = self.rng.range(5, 10);
        for _ in 0..n {
            let (kind, mat) = *self.rng.pick(&[(ItemKind::Craft, Mat::Gold), (ItemKind::Bar, Mat::Gold), (ItemKind::Gem, Mat::Amethyst), (ItemKind::Craft, Mat::Marble)]);
            let p = *self.rng.pick(&tiles);
            let id = self.spawn_item(kind, mat, p);
            if let Some(it) = self.items.get_mut(&id) {
                it.quality = self.rng.range(3, 6) as u8;
            }
        }
        let kind = *self.rng.pick(&[ItemKind::Sword, ItemKind::Axe, ItemKind::Hammer, ItemKind::Craft, ItemKind::Helm]);
        let p = *self.rng.pick(&tiles);
        let id = self.spawn_item(kind, Mat::Steel, p);
        let name = crate::dwarf::artifact_name(&mut self.rng);
        if let Some(it) = self.items.get_mut(&id) {
            it.quality = ARTIFACT;
            it.label = Some(name);
        }
        // Something was sealed in, and it isn't dead.
        if self.rng.chance(0.75) {
            let id = self.make_cave_horror(v.center());
            let c = self.creatures.get_mut(&id).unwrap();
            let base = c.label.clone().unwrap_or_default().replace(" the cave horror", "");
            let label = format!("{}, warden of {}", base, v.name);
            c.label = Some(label.clone());
            let desc = c.desc.clone().unwrap_or_default();
            self.msg(&format!("Something stirs in the dark: {} wakes! {}", label, desc), (230, 120, 255));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sealed_hall_waits_in_the_rock() {
        let mut g = Game::new(81);
        let v = g.vault.clone().expect("no vault was placed");
        assert!(!v.opened);
        let c = v.center();
        assert_eq!(g.world.t(c).kind, TileKind::Floor);
        assert!(!g.world.t(c).revealed);
        assert!(g.world.t(v.min.offset(-1, 0, 0)).smooth >= ENGRAVED, "carved walls");
        assert_eq!(g.world.t(v.min.offset(-1, 0, 0)).kind, TileKind::Wall);
        // Dig into it from the side.
        let items = g.items.len();
        let wall = v.min.offset(-1, 0, 0);
        g.world.t_mut(wall).desig = Desig::Dig;
        g.complete_dig(wall);
        assert!(g.vault.as_ref().unwrap().opened, "breaking in should open it");
        assert!(g.items.len() >= items + 6, "treasure");
        assert!(g.items.values().any(|it| it.quality == ARTIFACT && v.contains(it.pos)));
        assert!(g.log.iter().any(|l| l.text.contains("broken into")));
        g.check_vault();
        assert_eq!(g.log.iter().filter(|l| l.text.contains("broken into")).count(), 1, "only once");
    }
}
