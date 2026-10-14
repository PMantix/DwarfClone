//! Seasons and weather: ice in winter.

use super::*;
use crate::overworld::Biome;

impl Game {
    /// Whether this site's winters freeze its surface water.
    pub fn freezes(&self) -> bool {
        !matches!(self.biome, Biome::Desert | Biome::Swamp | Biome::Ocean)
    }

    /// At the turn of a season: winter freezes the water under the open sky
    /// (where the climate is cold enough), spring thaws it.
    pub(super) fn seasonal_ice(&mut self, season: u64) {
        if season == 3 && self.freezes() {
            let mut n = 0;
            for i in 0..self.world.tiles.len() {
                if self.world.tiles[i].water > 0 && !self.world.tiles[i].frozen {
                    let p = self.world.pos_of(i);
                    if self.world.outside(p) {
                        self.world.tiles[i].frozen = true;
                        n += 1;
                    }
                }
            }
            if n > 0 {
                self.msg("The lakes have frozen over.", (200, 230, 255));
                self.dig_version += 1;
            }
        } else if season == 0 {
            let mut n = 0;
            for t in self.world.tiles.iter_mut().filter(|t| t.frozen) {
                t.frozen = false;
                n += 1;
            }
            if n > 0 {
                self.msg("The ice has thawed.", (150, 200, 255));
                self.dig_version += 1;
                self.water_cells = None;
                self.scramble_ashore();
            }
        }
    }

    /// Anyone out on the ice when it breaks scrambles to the nearest dry ground.
    fn scramble_ashore(&mut self) {
        let shore = |w: &World, p: Pos| -> Option<Pos> {
            (1..6).find_map(|r| {
                (-r..=r)
                    .flat_map(|dy| (-r..=r).map(move |dx| (dx, dy)))
                    .map(|(dx, dy)| p.offset(dx, dy, 0))
                    .chain((1..3).map(|dz| p.offset(0, 0, dz)))
                    .find(|&q| w.walkable(q) && w.t(q).water == 0)
            })
        };
        for i in 0..self.dwarves.len() {
            let p = self.dwarves[i].pos;
            if self.dwarves[i].alive && !self.world.walkable(p) {
                if let Some(q) = shore(&self.world, p) {
                    self.dwarves[i].pos = q;
                    self.dwarves[i].path.clear();
                }
            }
        }
        let ids: Vec<u32> = self.creatures.keys().copied().collect();
        for id in ids {
            let p = self.creatures[&id].pos;
            if !self.world.walkable(p) {
                if let Some(q) = shore(&self.world, p) {
                    let c = self.creatures.get_mut(&id).unwrap();
                    c.pos = q;
                    c.path.clear();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn winter_ice_can_be_walked_on_and_spring_thaws_it() {
        let mut g = Game::new(81);
        g.biome = Biome::Forest;
        let e = g.embark;
        // A little pond under the open sky.
        let pond = (2..20)
            .map(|r| e.offset(r, -4, 0))
            .map(|p| Pos::new(p.x, p.y, g.world.surface_z(p.x, p.y)))
            .find(|&p| g.world.walkable(p) && g.world.t(p).building == 0)
            .unwrap();
        g.world.t_mut(pond).kind = TileKind::Open;
        g.world.t_mut(pond).water = 7;
        g.world.t_mut(pond.down()).kind = TileKind::Floor;
        g.water_cells = None;
        assert!(!g.world.walkable(pond));
        g.seasonal_ice(3);
        assert!(g.world.t(pond).frozen);
        assert!(g.world.walkable(pond), "ice should bear a dwarf");
        assert!(g.world.blocks_water(pond));
        // A dwarf out on the ice when it thaws gets to the shore.
        g.dwarves[0].pos = pond;
        g.seasonal_ice(0);
        assert!(!g.world.t(pond).frozen);
        assert_ne!(g.dwarves[0].pos, pond, "should have scrambled ashore");
        assert!(g.world.walkable(g.dwarves[0].pos));
        // Deserts don't freeze.
        g.biome = Biome::Desert;
        g.seasonal_ice(3);
        assert!(!g.world.t(pond).frozen);
    }
}
