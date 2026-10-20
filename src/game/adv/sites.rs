//! Entering and leaving sites. Entering builds (or, for a fallen fort,
//! loads) a `Game` for the site (`sitegen`), moves the overworld into it
//! (never copied: it can be megabytes) and carries the hero in; leaving
//! carries the hero out, takes the overworld back and runs the world's
//! clock on to the visit's.
//!
//! W0 builds every site as a flat field (`sitegen::wild::flat`); A4 makes
//! the land of the cell (`land`) and what stands on it.

use super::items::{carry_in, carry_out};
use super::{sitegen, Adventure, SiteKey, Visit};
use crate::game::Game;
use crate::geom::Pos;
use crate::overworld::Overworld;
use crate::world::World;

impl Adventure {
    /// Go into a site (leaving any site the hero is in first).
    pub fn enter(&mut self, key: SiteKey) -> Result<(), String> {
        self.enter_from(key, None)
    }

    /// Go into a site, arriving from the edge in direction `from` (A3: the
    /// neighbouring cell walked in from), or at its arrival point.
    pub fn enter_from(&mut self, key: SiteKey, from: Option<(i32, i32)>) -> Result<(), String> {
        if self.visit.is_some() {
            self.leave();
        }
        let seed = site_seed(self.seed, key);
        let (world, arrive) = sitegen::build(self, key, seed)?;
        let ow = std::mem::replace(&mut self.ow, Overworld::empty());
        let cell = match key {
            SiteKey::Wild(x, y) => (x, y),
            SiteKey::Site(id) => ow.sites.iter().find(|s| s.id == id).map_or(self.at, |s| (s.x, s.y)),
            SiteKey::Fort(i) => ow.forts.get(i as usize).map_or(self.at, |f| f.site),
        };
        let name = site_name(&ow, key);
        let mut g = Game::from_world(world, seed, ow, cell, arrive);
        g.fortress_name = name;
        g.start_year = self.start_year;
        g.tick = self.clock;
        g.weather = self.weather;
        let at = match from {
            Some((dx, dy)) => edge_spot(&g.world, arrive, dx, dy),
            None => arrive,
        };
        let hero = carry_in(&mut g, &mut self.hero, at);
        for c in self.company.iter_mut() {
            let p = g.spot_near(at, 2);
            carry_in(&mut g, c, p);
        }
        self.at = cell;
        self.visit = Some(Box::new(Visit { key, g, hero }));
        Ok(())
    }

    /// Come out of the site: the hero (and companions) come out, the world
    /// comes back, and the clock runs on. The key of the site left.
    pub fn leave(&mut self) -> Option<SiteKey> {
        let mut v = self.visit.take()?;
        // Companions first (they come after the hero in `dwarves`).
        let mut company = std::mem::take(&mut self.company);
        for c in company.iter_mut().rev() {
            if let Some(i) = v.g.dwarves.iter().rposition(|d| d.id == c.dwarf.id) {
                carry_out(&mut v.g, i, c);
            }
        }
        self.company = company;
        let hero = v.g.dwarves.iter().position(|d| d.id == super::HERO_ID).unwrap_or(v.hero);
        carry_out(&mut v.g, hero, &mut self.hero);
        self.clock = v.g.tick;
        self.weather = v.g.weather;
        self.ow = std::mem::replace(&mut v.g.overworld, Overworld::empty());
        let year = self.start_year + (self.clock / crate::game::YEAR) as i32;
        self.ow.year = self.ow.year.max(year);
        self.memos.entry(v.key).or_default().visited_tick = self.clock;
        Some(v.key)
    }
}

/// A site's own seed: the same site always gives the same land.
pub fn site_seed(seed: u64, key: SiteKey) -> u64 {
    let k = match key {
        SiteKey::Wild(x, y) => 1 ^ ((x as u64) << 8) ^ ((y as u64) << 28),
        SiteKey::Site(id) => 2 ^ ((id as u64) << 8),
        SiteKey::Fort(i) => 3 ^ ((i as u64) << 8),
    };
    crate::game::stories::hash(&format!("site|{}", k), seed)
}

/// What the site is called in the texts ("the wilds of the Grey Fells").
pub fn site_name(ow: &Overworld, key: SiteKey) -> String {
    match key {
        SiteKey::Site(id) => ow.sites.iter().find(|s| s.id == id).map_or_else(|| "a nameless place".to_string(), |s| s.name.clone()),
        SiteKey::Fort(i) => ow.forts.get(i as usize).map_or_else(|| "a fallen fort".to_string(), |f| f.name.clone()),
        SiteKey::Wild(x, y) => {
            if ow.w > 1 && x >= 0 && y >= 0 && x < ow.w && y < ow.h {
                let r = ow.region_at(x, y);
                match r {
                    Some(r) => format!("the wilds of {}", r.name),
                    None => "the wilds".to_string(),
                }
            } else {
                "the wilds".to_string()
            }
        }
    }
}

/// Where one walking in from direction (dx, dy) comes onto the map: the
/// edge on that side, level with the arrival point.
fn edge_spot(w: &World, arrive: Pos, dx: i32, dy: i32) -> Pos {
    let x = if dx < 0 { 0 } else if dx > 0 { w.w - 1 } else { arrive.x };
    let y = if dy < 0 { 0 } else if dy > 0 { w.h - 1 } else { arrive.y };
    let p = Pos::new(x, y, w.surface_z(x, y));
    if w.walkable(p) {
        p
    } else {
        arrive
    }
}

/// The land of a world cell as a fort embarked there would get it (A4).
pub fn land(ow: &Overworld, seed: u64, x: i32, y: i32) -> World {
    let params = ow.embark_params(seed, x, y, crate::world::MapSize::ALL[0]);
    crate::world::generate(seed ^ ((x as u64) << 20) ^ ((y as u64) << 40), &params)
}
