//! Seasons and weather: ice in winter, rain and snow.

use super::*;
use crate::overworld::Biome;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Weather {
    #[default]
    Clear,
    Rain,
    Snow,
}

impl Weather {
    pub fn name(self) -> &'static str {
        match self {
            Weather::Clear => "Clear",
            Weather::Rain => "Rain",
            Weather::Snow => "Snow",
        }
    }
}

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

    /// Each morning: what the sky does today, by climate and season.
    pub(super) fn weather_daily(&mut self) {
        let season = ((self.tick / SEASON) % 4) as usize;
        let wet = match self.biome {
            Biome::Desert => 0.03,
            Biome::Swamp => 0.45,
            Biome::Ocean | Biome::Forest => 0.3,
            Biome::Mountain | Biome::Tundra => 0.25,
            _ => 0.2,
        };
        let cold = self.freezes() && (season == 3 || (season == 2 && self.biome == Biome::Tundra));
        let before = self.weather;
        self.weather = if !self.rng.chance(wet) {
            Weather::Clear
        } else if cold {
            Weather::Snow
        } else {
            Weather::Rain
        };
        if self.weather != before {
            let text = match self.weather {
                Weather::Rain => "It has started to rain.",
                Weather::Snow => "Snow is falling.",
                Weather::Clear => "The skies have cleared.",
            };
            self.msg(text, (170, 190, 220));
        }
        if self.weather == Weather::Snow {
            // Fresh snow lies on open ground.
            for y in 0..self.world.h {
                for x in 0..self.world.w {
                    let p = Pos::new(x, y, self.world.surface_z(x, y));
                    if let Some(t) = self.world.get(p) {
                        if t.kind.has_floor() && t.water == 0 && t.building == 0 {
                            self.world.t_mut(p).snow = true;
                        }
                    }
                }
            }
            self.snow_lying = true;
        } else if self.snow_lying && season != 3 {
            for t in self.world.tiles.iter_mut() {
                t.snow = false;
            }
            self.snow_lying = false;
        }
    }

    /// Now and then while it rains or snows, dwarves out under the sky mind it.
    pub(super) fn weather_thoughts(&mut self) {
        let text = match self.weather {
            Weather::Clear => return,
            Weather::Rain => "was caught in the rain",
            Weather::Snow => "was out in the snow",
        };
        let tick = self.tick;
        for i in 0..self.dwarves.len() {
            let d = &self.dwarves[i];
            if !d.alive || !self.world.outside(d.pos) || d.thoughts.iter().any(|t| t.text == text && tick < t.tick + DAY) {
                continue;
            }
            self.dwarves[i].think(text, 2, tick);
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

    #[test]
    fn it_rains_in_summer_snows_in_winter_and_dwarves_outside_mind_it() {
        let mut g = Game::new(81);
        g.biome = Biome::Swamp;
        g.tick = SEASON + 5;
        let mut seen = HashSet::new();
        for _ in 0..40 {
            g.weather_daily();
            seen.insert(g.weather);
        }
        assert!(seen.contains(&Weather::Rain) && !seen.contains(&Weather::Snow), "summer: {:?}", seen);
        g.biome = Biome::Tundra;
        g.tick = 3 * SEASON + 5;
        while g.weather != Weather::Snow {
            g.weather_daily();
        }
        let e = g.embark;
        assert!(g.world.t(Pos::new(e.x + 3, e.y + 3, g.world.surface_z(e.x + 3, e.y + 3))).snow || g.world.tiles.iter().any(|t| t.snow));
        let id = g.dwarves[0].id;
        assert!(g.world.outside(g.dwarves[0].pos));
        g.weather_thoughts();
        g.weather_thoughts();
        let d = g.dwarves.iter().find(|d| d.id == id).unwrap();
        assert_eq!(d.thoughts.iter().filter(|t| t.text == "was out in the snow").count(), 1, "once a day");
        // Spring melts the snow on the first clear day.
        g.tick = 4 * SEASON + 5;
        g.biome = Biome::Desert;
        while g.weather != Weather::Clear {
            g.weather_daily();
        }
        g.weather_daily();
        assert!(!g.world.tiles.iter().any(|t| t.snow));
    }
}
