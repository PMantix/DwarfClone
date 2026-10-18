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
    /// Rain with thunder: lightning strikes far more often.
    Storm,
    /// Driving snow that chills anyone caught outside.
    Blizzard,
    /// Over an evil site: rain the colour of blood, and it drives dwarves to despair.
    BloodRain,
}

impl Weather {
    pub fn name(self) -> &'static str {
        match self {
            Weather::Clear => "Clear",
            Weather::Rain => "Rain",
            Weather::Snow => "Snow",
            Weather::Storm => "Thunderstorm",
            Weather::Blizzard => "Blizzard",
            Weather::BloodRain => "Blood rain",
        }
    }

    /// Water falls from the sky (puts out fires).
    pub fn wet(self) -> bool {
        matches!(self, Weather::Rain | Weather::Storm | Weather::BloodRain)
    }

    pub fn snowy(self) -> bool {
        matches!(self, Weather::Snow | Weather::Blizzard)
    }
}

impl Game {
    /// Whether this site's winters freeze its surface water.
    pub fn freezes(&self) -> bool {
        !matches!(self.biome, Biome::Desert | Biome::Swamp | Biome::Ocean | Biome::Jungle | Biome::Savanna | Biome::Badlands)
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
            Biome::Marsh => 0.4,
            Biome::Shrubland => 0.08,
            Biome::Ocean | Biome::Forest => 0.3,
            Biome::Mountain | Biome::Tundra | Biome::Taiga => 0.25,
            Biome::Jungle => 0.5,
            Biome::Savanna => 0.12,
            Biome::Badlands => 0.05,
            Biome::Glacier => 0.2,
            _ => 0.2,
        };
        let cold = self.freezes() && (season == 3 || (season == 2 && matches!(self.biome, Biome::Tundra | Biome::Glacier | Biome::Taiga)) || self.biome == Biome::Glacier && season != 1);
        // A drought may grip a summer: no rain until autumn.
        if season == 1 && self.tick % SEASON < DAY && !matches!(self.biome, Biome::Swamp | Biome::Marsh | Biome::Ocean | Biome::Jungle | Biome::Glacier) && self.rng.chance(0.15) {
            self.drought = true;
            self.msg("A drought grips the land. No rain will fall this summer.", (230, 190, 110));
            self.gossip("The old ones say they have never seen the brook so low.".into());
        }
        if season != 1 {
            self.drought = false;
        }
        // The monsoon: the warm wet lands get weeks of rain each summer.
        let monsoon_land = matches!(self.biome, Biome::Jungle | Biome::Swamp | Biome::Savanna);
        if season == 1 && self.tick % SEASON < DAY && monsoon_land && !self.drought {
            self.monsoon = true;
            self.msg("The monsoon has come. The rains will fall for weeks.", (130, 170, 230));
        }
        if season != 1 {
            self.monsoon = false;
        }
        // Weather comes in spells: today is most likely what yesterday was.
        let before = self.weather;
        let keep = match before {
            Weather::Clear => 1.0 - wet * 0.4,
            Weather::Storm | Weather::Blizzard => 0.35,
            Weather::BloodRain => 0.5,
            _ => 0.65,
        };
        let keep = if self.monsoon { if before == Weather::Clear { 0.2 } else { 0.9 } } else { keep };
        let kept = !self.drought && self.rng.chance(keep);
        if kept && self.weather.snowy() && !cold {
            self.weather = Weather::Rain;
        }
        if !kept {
        self.weather = if self.drought || (!self.monsoon && before != Weather::Clear) || (!self.monsoon && !self.rng.chance(wet)) {
            Weather::Clear
        } else if cold {
            if self.rng.chance(0.25) {
                Weather::Blizzard
            } else {
                Weather::Snow
            }
        } else if self.evil && self.rng.chance(0.3) {
            Weather::BloodRain
        } else if self.rng.chance(0.25) {
            Weather::Storm
        } else {
            Weather::Rain
        };
        }
        if self.weather != before {
            let (text, color) = match self.weather {
                Weather::Rain => ("It has started to rain.", (170, 190, 220)),
                Weather::Snow => ("Snow is falling.", (170, 190, 220)),
                Weather::Clear => ("The skies have cleared.", (170, 190, 220)),
                Weather::Storm => ("Thunder rolls over the mountain. A storm is breaking!", (200, 200, 255)),
                Weather::Blizzard => ("A blizzard howls down from the peaks!", (220, 235, 255)),
                Weather::BloodRain => ("The sky darkens, and rain the colour of blood begins to fall.", (230, 60, 60)),
            };
            self.msg(text, color);
        }
        if self.drought {
            self.dry_up();
        }
        if self.weather.snowy() {
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
            // (A glacier's snow never melts.)
            let wh = (self.world.w * self.world.h) as usize;
            let keep: Vec<bool> = (0..wh).map(|i| self.world.biomes.get(i).and_then(|&c| crate::overworld::Biome::from_code(c)) == Some(Biome::Glacier)).collect();
            for (i, t) in self.world.tiles.iter_mut().enumerate() {
                if !keep[i % wh] {
                    t.snow = false;
                }
            }
            self.snow_lying = false;
        }
    }

    /// Now and then while it rains or snows, dwarves out under the sky mind it.
    pub(super) fn weather_thoughts(&mut self) {
        let (text, stress) = match self.weather {
            Weather::Clear => return,
            Weather::Rain => ("was caught in the rain", 2),
            Weather::Snow => ("was out in the snow", 2),
            Weather::Storm => ("was caught out in a thunderstorm", 6),
            Weather::Blizzard => ("was caught in a blizzard", 12),
            Weather::BloodRain => ("was drenched in blood rain", 30),
        };
        let tick = self.tick;
        for i in 0..self.dwarves.len() {
            let d = &self.dwarves[i];
            if !d.alive || !self.world.outside(d.pos) || d.thoughts.iter().any(|t| t.text == text && tick < t.tick + DAY) {
                continue;
            }
            self.dwarves[i].think(text, stress, tick);
        }
    }

    /// In a drought, shallow water under the open sky dries away.
    fn dry_up(&mut self) {
        let mut n = 0;
        for i in 0..self.world.tiles.len() {
            let t = self.world.tiles[i];
            if t.water > 0 && t.water <= 3 && !t.frozen && self.rng.chance(0.5) {
                let p = self.world.pos_of(i);
                if self.world.outside(p) {
                    self.world.tiles[i].water -= 1;
                    n += 1;
                }
            }
        }
        if n > 0 {
            self.water_cells = None;
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
    fn weather_comes_in_spells_and_the_monsoon_brings_weeks_of_rain() {
        let mut g = Game::new(81);
        g.biome = Biome::Forest;
        g.evil = false;
        let mut changes = 0;
        let mut last = g.weather;
        for day in 0..336u64 {
            g.tick = day * DAY;
            g.weather_daily();
            if g.weather != last {
                changes += 1;
                last = g.weather;
            }
        }
        assert!(changes < 70, "the weather changed {} times in a year", changes);
        let mut g = Game::new(81);
        g.biome = Biome::Jungle;
        g.evil = false;
        g.drought = false;
        let mut wet = 0;
        for day in 84..168u64 {
            g.tick = day * DAY;
            g.weather_daily();
            if g.weather != Weather::Clear {
                wet += 1;
            }
        }
        assert!(g.monsoon || wet > 0);
        assert!(wet > 50, "the monsoon brought only {} wet days of 84", wet);
    }


    #[test]
    fn winter_ice_can_be_walked_on_and_spring_thaws_it() {
        let mut g = Game::new(81);
        g.biome = Biome::Forest;
        let e = g.embark;
        // A little pond under the open sky.
        // A spot with dry, walkable ground all round it to scramble onto.
        let pond = (2..30)
            .flat_map(|r| [e.offset(r, -4, 0), e.offset(-r, 4, 0), e.offset(4, r, 0), e.offset(-4, -r, 0)])
            .map(|p| Pos::new(p.x, p.y, g.world.surface_z(p.x, p.y)))
            .find(|&p| {
                g.world.walkable(p)
                    && g.world.t(p).building == 0
                    && g.world.t(p).water == 0
                    && [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().all(|&(dx, dy)| g.world.walkable(p.offset(dx, dy, 0)) && g.world.t(p.offset(dx, dy, 0)).water == 0)
            })
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
    fn wild_weather_storms_blizzards_droughts_and_blood_rain() {
        let mut g = Game::new(1);
        g.biome = Biome::Forest;
        g.evil = true;
        g.tick = SEASON + DAY * 3;
        let mut seen = HashSet::new();
        for _ in 0..200 {
            g.weather_daily();
            seen.insert(g.weather);
        }
        assert!(seen.contains(&Weather::BloodRain) && seen.contains(&Weather::Storm), "{:?}", seen);
        assert!(!seen.contains(&Weather::Snow) && !seen.contains(&Weather::Blizzard));
        g.weather = Weather::BloodRain;
        let id = g.dwarves[0].id;
        assert!(g.world.outside(g.dwarves[0].pos));
        g.weather_thoughts();
        assert!(g.dwarves.iter().find(|d| d.id == id).unwrap().thoughts.iter().any(|t| t.text == "was drenched in blood rain"));
        // Winter brings blizzards.
        g.biome = Biome::Tundra;
        g.tick = 3 * SEASON + DAY * 3;
        let mut seen = HashSet::new();
        for _ in 0..200 {
            g.weather_daily();
            seen.insert(g.weather);
        }
        assert!(seen.contains(&Weather::Blizzard), "{:?}", seen);
        // A drought summer: no rain, and shallow water dries.
        g.biome = Biome::Forest;
        g.evil = false;
        g.tick = SEASON;
        g.drought = false;
        while !g.drought {
            g.weather_daily();
        }
        let puddle = g.world.tiles.iter().position(|t| t.water > 0 && t.water <= 3).map(|i| g.world.pos_of(i));
        for _ in 0..30 {
            g.weather_daily();
            assert_eq!(g.weather, Weather::Clear, "no rain in a drought");
        }
        if let Some(p) = puddle.filter(|&p| g.world.outside(p)) {
            assert!(g.world.t(p).water <= 1, "the puddle should have dried");
        }
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
