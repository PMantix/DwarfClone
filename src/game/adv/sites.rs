//! Entering and leaving sites. Entering builds (or, for a fallen fort,
//! loads) a `Game` for the site (`sitegen`), moves the overworld into it
//! (never copied: it can be megabytes) and carries the hero in; leaving
//! carries the hero out, takes the overworld back and runs the world's
//! clock on to the visit's.
//!
//! The land is the cell's own (`land`), what stands on it is `sitegen`'s,
//! laid over with what the hero changed there before (`SiteMemo`), and
//! the place's past is told on arrival.

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
        // The land is as it was when the adventure began (an embark's
        // aquifer rolls on the world's year, which the adventure moves on).
        let year = self.ow.year;
        self.ow.year = self.ow.year.min(self.start_year - 1);
        let built = sitegen::build(self, key, seed);
        self.ow.year = year;
        let (world, arrive) = built?;
        let ctx = sitegen::Ctx { key, seed, race: self.hero.race, civ: self.hero.civ, memo: self.memos.get(&key).cloned().unwrap_or_default(), now: self.clock };
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
        // What stands here, and what the hero did here before.
        let made = sitegen::make(&mut g, &ctx);
        let arrive = made.arrive.unwrap_or(arrive);
        let at = match from {
            Some((dx, dy)) => edge_spot(&g.world, arrive, dx, dy),
            None => arrive,
        };
        let hero = carry_in(&mut g, &mut self.hero, at);
        for c in self.company.iter_mut() {
            let p = g.spot_near(at, 2);
            carry_in(&mut g, c, p);
        }
        arrival(&mut g, &made, at, ctx.memo.visited_tick > 0);
        self.at = cell;
        self.visit = Some(Box::new(Visit { key, g, hero, placed: made.placed }));
        Ok(())
    }

    /// Come out of the site: the hero (and companions) come out, the world
    /// comes back, and the clock runs on. The key of the site left.
    pub fn leave(&mut self) -> Option<SiteKey> {
        let mut v = self.visit.take()?;
        // What the hero changed here, before their pack leaves with them.
        let infamy = if matches!(v.key, SiteKey::Fort(_)) {
            0
        } else {
            let memo = self.memos.entry(v.key).or_default();
            sitegen::remember(&v.g, &v.placed, memo);
            turned_guards(&v.g)
        };
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
        // Blood shed among folk who had welcomed the hero is remembered.
        if infamy > 0 {
            if let SiteKey::Site(id) = v.key {
                let site = self.ow.sites.iter().find(|s| s.id == id).cloned();
                if let Some(s) = site.filter(|s| super::peoples::stance(&self.ow, self.hero.race, self.hero.civ, s) != super::peoples::Stance::Hostile) {
                    self.fame.infamy += 5;
                    if let Some(c) = s.civ {
                        *self.fame.by_civ.entry(c).or_default() -= 10;
                    }
                }
            }
        }
        Some(v.key)
    }

    /// What the legends say of the site the hero is in (or the cell they
    /// stand in): a title and its story, for the look panel and the
    /// sidebar.
    pub fn site_story(&self) -> (String, Vec<String>) {
        let ow = self.ow();
        let key = self.visit.as_ref().map_or(SiteKey::Wild(self.at.0, self.at.1), |v| v.key);
        let stance = match key {
            SiteKey::Site(id) => ow.sites.iter().find(|s| s.id == id).map_or(super::peoples::Stance::Wary, |s| super::peoples::stance(ow, self.hero.race, self.hero.civ, s)),
            _ => super::peoples::Stance::Wary,
        };
        sitegen::lore::story(ow, key, stance, &site_name(ow, key))
    }
}

/// Guards of the site who have turned on the hero.
fn turned_guards(g: &Game) -> usize {
    g.creatures.values().filter(|c| c.visitor == sitegen::town::GUARD && c.siege && c.species != crate::creature::Species::Visitor).count()
}

/// The arrival: the place's story in the log, and on a card the first time.
fn arrival(g: &mut Game, made: &sitegen::Made, at: Pos, again: bool) {
    let Some(head) = made.story.first() else { return };
    let color = if made.hostile { (255, 120, 90) } else { (255, 215, 140) };
    if again {
        g.msg(&format!("You return to {}.", lowered(&made.title)), color);
    }
    for (i, line) in made.story.iter().take(4).enumerate() {
        g.msg(line, if i == 0 { color } else { (200, 190, 170) });
    }
    if !again && !made.title.is_empty() {
        let text = made.story.iter().take(3).cloned().collect::<Vec<_>>().join(" ");
        g.spotlight(&made.title, if text.is_empty() { head } else { &text }, Some(at), color);
    }
}

/// "The lair of Ashmaw" -> "the lair of Ashmaw".
fn lowered(s: &str) -> String {
    match s.strip_prefix("The ") {
        Some(r) => format!("the {}", r),
        None => s.to_string(),
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

#[cfg(test)]
mod tests {
    use super::super::sitegen::town::tests::{visit, world};
    use super::super::tests::test_hero;
    use super::super::turn::Action;
    use super::*;
    use crate::items::ItemKind;
    use crate::overworld::Race;

    #[test]
    fn the_same_cell_gives_the_same_land() {
        let ow = world();
        let mut a = Adventure::new(ow, 21, test_hero(21), None);
        let (x, y) = a.at;
        a.enter(SiteKey::Wild(x, y)).unwrap();
        let first = a.visit.as_ref().unwrap().g.world.tiles.clone();
        let (w, h) = (a.visit.as_ref().unwrap().g.world.w, a.visit.as_ref().unwrap().g.world.h);
        assert_eq!((w, h), (64, 48), "the land of a small embark");
        a.leave();
        a.enter(SiteKey::Wild(x, y)).unwrap();
        let again = &a.visit.as_ref().unwrap().g.world;
        let diff: Vec<String> = again.tiles.iter().zip(first.iter()).enumerate().filter(|(_, (x, y))| x != y).take(5).map(|(i, (x, y))| format!("{} {:?} vs {:?}", i, x, y)).collect();
        assert!(diff.is_empty(), "the same land again: {:?}", diff);
        // And the next cell is other land.
        a.enter(SiteKey::Wild(x + 1, y)).unwrap();
        assert!(a.visit.as_ref().unwrap().g.world.tiles != first);
    }

    #[test]
    #[cfg_attr(debug_assertions, ignore)]
    fn entering_is_quick() {
        let ow = world();
        let mut a = Adventure::new(ow, 22, test_hero(22), None);
        let (x, y) = a.at;
        // The best of three (other builds may share the machine).
        let mut best = f64::MAX;
        for k in 0..3 {
            let t = std::time::Instant::now();
            a.enter(SiteKey::Wild(x + k, y)).unwrap();
            best = best.min(t.elapsed().as_secs_f64());
        }
        assert!(best < 1.2, "entering took {:.2}s", best);
    }

    #[test]
    fn a_slain_beast_stays_slain() {
        let ow = world();
        let f = ow.figures.iter().filter(|f| f.alive() && f.lair.is_some()).max_by_key(|f| f.kills).expect("a living beast").clone();
        let lair = f.lair.unwrap();
        let mut a = visit(lair, Race::Dwarf, None, 6);
        let v = a.visit.as_mut().unwrap();
        let (g, h) = (&mut v.g, v.hero);
        // Slay it where it lies.
        let id = g.creatures.values().find(|c| c.figure == Some(f.id)).map(|c| c.id).expect("the beast");
        let at = g.creatures[&id].pos;
        let c = g.creatures.remove(&id).unwrap();
        g.creature_died(c);
        // And take a piece of its hoard.
        let prize = g.items.values().find(|it| it.carried_by.is_none() && matches!(it.kind, ItemKind::Craft | ItemKind::Bar | ItemKind::Gem)).map(|it| (it.id, it.pos, it.kind, it.mat)).expect("a hoard");
        g.dwarves[h].pos = prize.1;
        assert!(matches!(g.act(h, Action::Get(vec![prize.0])), super::super::turn::Outcome::Done { .. }));
        let hoard0 = g.items.values().filter(|it| it.carried_by.is_none() && matches!(it.kind, ItemKind::Craft | ItemKind::Bar | ItemKind::Gem)).count();
        a.leave();
        assert!(a.ow.figures[f.id as usize].died.is_some(), "the legends know it is dead");
        assert!(a.memos[&SiteKey::Site(lair)].taken.len() == 1);
        // Back again: no beast, its bones where it fell, the prize still gone.
        a.clock += crate::game::SEASON * 2;
        a.enter(SiteKey::Site(lair)).unwrap();
        let g = &a.visit.as_ref().unwrap().g;
        assert!(!g.creatures.values().any(|c| c.figure == Some(f.id)), "it stays dead");
        assert!(g.items.values().any(|it| it.pos == at && it.kind == ItemKind::BodyPart && it.label.as_deref().map_or(false, |l| l.starts_with("the bones of"))), "its bones lie where it fell");
        let hoard1 = g.items.values().filter(|it| it.carried_by.is_none() && matches!(it.kind, ItemKind::Craft | ItemKind::Bar | ItemKind::Gem)).count();
        assert_eq!(hoard1, hoard0, "what was taken stays taken");
        assert!(g.log.iter().any(|e| e.text.contains("empty lair")), "{:?}", g.log.iter().map(|e| &e.text).collect::<Vec<_>>());
    }
}
