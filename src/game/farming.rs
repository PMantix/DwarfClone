//! Gathering wild plants from shrubs, and fishing.

use super::*;
use crate::material::CROPS;

impl Game {
    /// What a wild shrub at `p` bears this season, if anything: one of the
    /// crops that grow there now, fixed for the spot. Stoutcaps grow wild only
    /// underground, though they can be farmed anywhere.
    pub fn wild_crop(&self, p: Pos) -> Option<Mat> {
        let season = ((self.tick / SEASON) % 4) as usize;
        let outside = self.world.outside(p);
        let here: Vec<Mat> = CROPS
            .iter()
            .copied()
            .filter(|m| m.crop().map_or(false, |c| c.grows(outside, season) && c.underground.unwrap_or(true) != outside))
            .collect();
        if here.is_empty() {
            return None;
        }
        let h = (p.x * 31 + p.y * 17 + p.z * 7).unsigned_abs() as usize;
        Some(here[h % here.len()])
    }

    /// Farmers pick shrubs in gathering zones that are bearing.
    pub(super) fn find_gather(&mut self, d: &mut Dwarf) -> bool {
        let spots: HashSet<Pos> = self
            .zones
            .values()
            .filter(|z| z.kind == ZoneKind::Gathering)
            .flat_map(|z| z.tiles.iter().copied())
            .filter(|p| {
                let t = self.world.t(*p);
                t.kind == TileKind::Shrub && t.farm == 0 && !self.claimed.contains(p)
            })
            .collect();
        if spots.is_empty() {
            return false;
        }
        // Everything under the sky is bare in winter; don't search for nothing.
        if !spots.iter().any(|&p| self.wild_crop(p).is_some()) {
            return false;
        }
        let Some((p, path)) = path::search(&self.world, d.pos, SEARCH_LIMIT, Mover::Dwarf, |p| spots.contains(&p) && self.wild_crop(p).is_some())
        else {
            return false;
        };
        self.claimed.insert(p);
        d.task = Task::Gather { pos: p };
        Self::set_path(d, path);
        true
    }

    pub(super) fn do_gather(&mut self, d: &mut Dwarf, pos: Pos) {
        match self.travel(d, pos) {
            Travel::Moving => {}
            Travel::Failed => self.release_task(d),
            Travel::Arrived => {
                if Self::labor(d, 60, d.skill(Labor::Farming)) {
                    self.claimed.remove(&pos);
                    d.gain_xp(Labor::Farming, 10);
                    d.task = Task::Idle;
                    if let Some(m) = self.wild_crop(pos) {
                        let n = 1 + self.rng.chance(0.5) as u32;
                        for _ in 0..n {
                            self.spawn_item(ItemKind::Plant, m, pos);
                        }
                    }
                    // Picked bare until the season turns.
                    self.world.t_mut(pos).farm = 1;
                }
            }
        }
    }

    /// Fishers cast from beside deep water, unless the larder is already
    /// full of fish.
    pub(super) fn find_fishing(&mut self, d: &mut Dwarf) -> bool {
        let fish = self.items.values().filter(|it| it.kind == ItemKind::Fish).count();
        if fish >= 20 {
            return false;
        }
        let w = &self.world;
        let deep = |q: Pos| w.get(q).map_or(false, |t| t.water >= 4 && t.magma == 0 && !t.frozen);
        let bank = |p: Pos| {
            DIRS8.iter().any(|&(dx, dy)| {
                let q = p.offset(dx, dy, 0);
                deep(q) || (w.kind(q) == Some(TileKind::Open) && deep(q.down()))
            })
        };
        // Nothing to fish from last time and nothing dug since: don't look again yet.
        if let Some((v, t)) = self.dig_fail[3] {
            if v == self.dig_version && self.tick < t + 600 {
                return false;
            }
        }
        let Some((stand, path)) = path::search(w, d.pos, SEARCH_LIMIT, Mover::Dwarf, bank) else {
            self.dig_fail[3] = Some((self.dig_version, self.tick));
            return false;
        };
        d.task = Task::Fish { stand };
        Self::set_path(d, path);
        true
    }

    pub(super) fn do_fish(&mut self, d: &mut Dwarf, stand: Pos) {
        match self.travel(d, stand) {
            Travel::Moving => {}
            Travel::Failed => self.release_task(d),
            Travel::Arrived => {
                let skill = d.skill(Labor::Fishing);
                if Self::labor(d, 200, skill) {
                    d.task = Task::Idle;
                    d.gain_xp(Labor::Fishing, 15);
                    if self.legendary_bite(d, stand) {
                        return;
                    }
                    if self.rng.chance(0.4 + 0.04 * skill as f64) {
                        let kinds: &[&str] = if self.world.outside(stand) {
                            &["mudperch", "silverfin", "river pike"]
                        } else {
                            &["blind eel", "glowfin", "cave carp"]
                        };
                        let name = kinds[self.rng.range(0, kinds.len() as i32) as usize];
                        let id = self.spawn_item(ItemKind::Fish, Mat::Flesh, stand);
                        self.items.get_mut(&id).unwrap().label = Some(name.into());
                    }
                }
            }
        }
    }

    /// A new season: picked shrubs bear again.
    pub(super) fn regrow_shrubs(&mut self) {
        let tiles: Vec<Pos> =
            self.zones.values().filter(|z| z.kind == ZoneKind::Gathering).flat_map(|z| z.tiles.iter().copied()).collect();
        for p in tiles {
            let t = self.world.t_mut(p);
            if t.kind == TileKind::Shrub {
                t.farm = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn farmers_gather_wild_plants_once_a_season() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let e = g.embark;
        // A patch of surface shrubs next to the wagon.
        let mut shrubs = Vec::new();
        'look: for r in 3..14 {
            for dx in -r..=r {
                for dy in [-r, r] {
                    let p = e.offset(dx, dy, 0);
                    let p = Pos::new(p.x, p.y, g.world.surface_z(p.x, p.y));
                    if g.world.t(p).kind == TileKind::Floor && g.world.t(p).building == 0 && g.world.t(p).zone == 0 && g.world.t(p).water == 0 {
                        g.world.t_mut(p).kind = TileKind::Shrub;
                        shrubs.push(p);
                        if shrubs.len() == 4 {
                            break 'look;
                        }
                    }
                }
            }
        }
        assert!(shrubs.len() >= 2, "no room for shrubs");
        for &p in &shrubs {
            assert!(g.add_zone(p, p, ZoneKind::Gathering) > 0);
        }
        assert!(g.wild_crop(shrubs[0]).is_some(), "spring shrubs should bear");
        for d in g.dwarves.iter_mut() {
            d.labors[Labor::Farming.idx()] = true;
        }
        let before = g.items.values().filter(|it| it.kind == ItemKind::Plant).count();
        for _ in 0..DAY {
            g.step();
            // Fed, so nobody eats the sunberries before they're counted.
            for d in g.dwarves.iter_mut() {
                d.hunger = 0;
            }
        }
        assert!(shrubs.iter().all(|p| g.world.t(*p).farm == 1), "not every shrub was picked");
        let after = g.items.values().filter(|it| it.kind == ItemKind::Plant).count();
        assert!(after >= before + shrubs.len(), "gathered only {} plants", after - before);
        g.regrow_shrubs();
        assert!(shrubs.iter().all(|p| g.world.t(*p).farm == 0));
        // Winter: nothing bears on the surface.
        g.tick = 3 * SEASON + 10;
        assert!(g.wild_crop(shrubs[0]).is_none());
    }

    #[test]
    fn a_fisherdwarf_catches_fish_from_deep_water() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        // A pond beside the wagon.
        let e = g.embark;
        let pond = (2..20)
            .map(|r| e.offset(r, -3, 0))
            .map(|p| Pos::new(p.x, p.y, g.world.surface_z(p.x, p.y)))
            .find(|&p| g.world.walkable(p) && g.world.t(p).building == 0 && g.world.walkable(p.offset(-1, 0, 0)))
            .unwrap();
        g.world.t_mut(pond).water = 7;
        g.world.t_mut(pond).kind = TileKind::Open;
        g.world.t_mut(pond.down()).kind = TileKind::Open;
        g.world.t_mut(pond.down()).water = 7;
        g.water_cells = None;
        let id = g.dwarves[0].id;
        g.set_labor(id, Labor::Fishing, true);
        for _ in 0..DAY {
            g.step();
            for d in g.dwarves.iter_mut() {
                d.hunger = 0;
            }
            if g.items.values().any(|it| it.kind == ItemKind::Fish) {
                break;
            }
        }
        let fish = g.items.values().find(|it| it.kind == ItemKind::Fish).expect("no fish caught");
        assert!(["mudperch", "silverfin", "river pike"].contains(&fish.name().as_str()), "caught a {}", fish.name());
        assert!(fish.is_food());
    }
}
