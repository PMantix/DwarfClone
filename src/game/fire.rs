//! Fire: it catches in trees, shrubs, grass and anything wooden, spreads to
//! its neighbours, burns whoever stands in it, and dies in water or rain.

use super::*;

/// How long a fresh fire burns on one tile, in fire ticks (every 10 game ticks).
const FUEL: u8 = 20;

impl Game {
    /// How readily fire takes hold at `p`, 0 if it can't: trees, shrubs and
    /// wood catch easily, grass only now and then (and not under snow).
    fn kindling(&self, p: Pos, wood: &HashSet<Pos>) -> f64 {
        let Some(t) = self.world.get(p) else { return 0.0 };
        if t.water > 0 || t.magma > 0 || t.fire > 0 {
            return 0.0;
        }
        let wooden_building = self.buildings.get(&t.building).map_or(false, |b| b.built && b.mat == Mat::Wood);
        if matches!(t.kind, TileKind::Tree | TileKind::Shrub) || wooden_building || wood.contains(&p) {
            1.0
        } else if t.kind == TileKind::Floor && t.mat == Mat::Grass && !t.snow {
            0.25
        } else {
            0.0
        }
    }

    fn wooden_items(&self) -> HashSet<Pos> {
        self.items.values().filter(|it| it.carried_by.is_none() && it.mat == Mat::Wood).map(|it| it.pos).collect()
    }

    /// Whether fire can take hold at `p`.
    pub fn flammable(&self, p: Pos) -> bool {
        self.kindling(p, &self.wooden_items()) > 0.0
    }

    /// Set `p` alight if it will burn. Returns whether it caught.
    pub fn ignite(&mut self, p: Pos) -> bool {
        if !self.flammable(p) {
            return false;
        }
        self.set_alight(p);
        true
    }

    fn set_alight(&mut self, p: Pos) {
        if self.fire_cells.as_ref().map_or(false, |c| c.is_empty()) {
            // A new outbreak, not a fire spreading.
            self.year_log.fires += 1;
            self.year_log.autumn_fire |= (self.tick / SEASON) % 4 == 2;
        }
        self.world.t_mut(p).fire = FUEL;
        if let Some(c) = self.fire_cells.as_mut() {
            c.push(self.world.idx(p));
        }
        self.warn("fire", "A fire has broken out!", (255, 120, 40));
    }

    /// Every 10 ticks: fires burn down, spread, hurt and are put out.
    pub(super) fn fires(&mut self) {
        let cells = match self.fire_cells.take() {
            Some(c) => c,
            None => self.world.tiles.iter().enumerate().filter(|(_, t)| t.fire > 0).map(|(i, _)| i).collect(),
        };
        let raining = self.weather.wet();
        let mut next: Vec<usize> = Vec::new();
        let mut spread: Vec<Pos> = Vec::new();
        for i in cells {
            let p = self.world.pos_of(i);
            let t = self.world.tiles[i];
            if t.fire == 0 {
                continue;
            }
            // Water drowns it; rain soaks it under the open sky.
            let soaked = raining && self.world.outside(p);
            if t.water > 0 || (soaked && self.rng.chance(0.3)) {
                self.world.tiles[i].fire = 0;
                continue;
            }
            self.world.tiles[i].fire -= 1;
            self.burn_tile(p);
            // Smoke rises off it, thickest in the halls.
            self.puff(p, 4);
            if self.world.in_bounds(p.up()) && !self.world.solid(p.up()) {
                self.puff(p.up(), 3);
            }
            if self.world.tiles[i].fire == 0 {
                self.burnt_out(p);
                continue;
            }
            next.push(i);
            if !soaked {
                // Sideways, and up or down a slope.
                for &(dx, dy) in DIRS8.iter() {
                    for dz in -1..=1 {
                        if self.rng.chance(if dz == 0 { 0.1 } else { 0.05 }) {
                            spread.push(p.offset(dx, dy, dz));
                        }
                    }
                }
                if self.rng.chance(0.04) {
                    spread.push(p.up());
                }
            }
        }
        self.fire_cells = Some(next);
        if spread.is_empty() {
            return;
        }
        let wood = self.wooden_items();
        for q in spread {
            let k = self.kindling(q, &wood);
            if k > 0.0 && self.rng.chance(k) {
                self.set_alight(q);
            }
        }
    }

    /// The flames on `p` burn whoever and whatever is there.
    fn burn_tile(&mut self, p: Pos) {
        let tick = self.tick;
        for i in 0..self.dwarves.len() {
            if self.dwarves[i].alive && self.dwarves[i].pos == p {
                let part = self.dwarves[i].body.pick_part(&mut self.rng);
                let d = &mut self.dwarves[i];
                d.body.wound(part, 2, false);
                d.think("was burned in a fire", 10, tick);
            }
        }
        let ids: Vec<u32> = self.creatures.values().filter(|c| c.alive && c.pos == p && !c.breathes_fire).map(|c| c.id).collect();
        for id in ids {
            let part = self.creatures[&id].body.pick_part(&mut self.rng);
            let c = self.creatures.get_mut(&id).unwrap();
            c.body.wound(part, 2, false);
            c.fears_fire = true;
        }
        let burnt: Vec<u32> = self.items.values().filter(|it| it.pos == p && it.carried_by.is_none() && !it.mat.fireproof()).map(|it| it.id).collect();
        for id in &burnt {
            self.items.remove(id);
        }
    }

    /// The fire on `p` has gone out, leaving it scorched.
    fn burnt_out(&mut self, p: Pos) {
        let bid = self.world.t(p).building;
        if bid != 0 && self.buildings.get(&bid).map_or(false, |b| b.mat == Mat::Wood) {
            let name = self.buildings[&bid].name();
            // Nothing to salvage from the ashes.
            if let Some(mut b) = self.buildings.remove(&bid) {
                b.built = false;
                self.buildings.insert(bid, b);
            }
            self.remove_building(bid);
            self.msg_at(&format!("The {} has burned down.", name.to_lowercase()), (255, 140, 60), Some(p), super::logkind::LogKind::Other);
        }
        let t = self.world.t_mut(p);
        match t.kind {
            TileKind::Tree | TileKind::Shrub => {
                t.kind = TileKind::Floor;
                t.mat = Mat::Loam;
                self.dig_version += 1;
            }
            TileKind::Floor if t.mat == Mat::Grass => t.mat = Mat::Loam,
            _ => {}
        }
    }

    /// In a storm, lightning now and then strikes a tree.
    pub(super) fn lightning(&mut self) {
        let odds = match self.weather {
            Weather::Storm => 0.15,
            Weather::Rain => 0.02,
            _ => 0.0,
        };
        if !self.rng.chance(odds) {
            return;
        }
        let (x, y) = (self.rng.range(0, self.world.w), self.rng.range(0, self.world.h));
        let p = Pos::new(x, y, self.world.surface_z(x, y));
        if self.world.kind(p) == Some(TileKind::Tree) {
            // Strike when the rain lets up a moment: it can still catch.
            if self.ignite(p) {
                self.msg_at("Lightning strikes a tree!", (255, 255, 160), Some(p), super::logkind::LogKind::Weather);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fire_spreads_through_wood_and_water_stops_it() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        g.weather = Weather::Clear;
        g.biome = crate::overworld::Biome::Desert;
        // A band of shrubs three wide, on level ground by the wagon.
        let e = g.embark;
        let e = (0..400)
            .map(|k| e.offset(k % 20 - 10, k / 20 - 10, 0))
            .find(|&o| (4..14).all(|i| (5..8).all(|j| {
                let p = o.offset(i, j, 0);
                g.world.in_bounds(p) && g.world.surface_z(p.x, p.y) == g.world.surface_z(o.x + 4, o.y + 6) && g.world.t(Pos::new(p.x, p.y, g.world.surface_z(p.x, p.y))).water == 0
            })))
            .expect("level ground near the wagon");
        let at = |g: &Game, i: i32, j: i32| {
            let p = e.offset(i, 6 + j, 0);
            Pos::new(p.x, p.y, g.world.surface_z(p.x, p.y))
        };
        let row: Vec<Pos> = (4..14).map(|i| at(&g, i, 0)).collect();
        for i in 4..14 {
            for j in -1..=1 {
                let p = at(&g, i, j);
                let t = g.world.t_mut(p);
                t.kind = TileKind::Shrub;
                t.water = 0;
                t.building = 0;
            }
        }
        let log = g.spawn_item(ItemKind::Log, Mat::Wood, row[5]);
        let rock = g.spawn_item(ItemKind::Boulder, Mat::Granite, row[5]);
        g.dwarves[0].pos = row[2];
        assert!(g.ignite(row[0]));
        assert!(!g.ignite(row[0]), "already burning");
        for _ in 0..3000 {
            g.step();
            g.dwarves[0].pos = row[2];
            g.dwarves[0].path.clear();
        }
        let burnt = row.iter().filter(|p| g.world.t(**p).kind == TileKind::Floor).count();
        assert!(burnt >= 5, "the fire only burnt {} shrubs", burnt);
        assert!(!g.items.contains_key(&log), "the log should have burned");
        assert!(g.items.contains_key(&rock), "stone doesn't burn");
        assert!(g.dwarves[0].thoughts.iter().any(|t| t.text == "was burned in a fire"));
        // Water won't burn.
        let wet = row[9];
        g.world.t_mut(wet).kind = TileKind::Shrub;
        g.world.t_mut(wet).water = 2;
        assert!(!g.ignite(wet));
    }
}
