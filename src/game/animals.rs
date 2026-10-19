//! Tame animals: pets, livestock, pastures, breeding, war dogs, taming and slaughter.

use super::{Game, ZoneKind, DAY, MONTH, SEASON, SEARCH_LIMIT};
use crate::creature::{Creature, Species};
use crate::dwarf::{Dwarf, Labor, Task};
use crate::geom::{Pos, DIRS8};
use crate::items::ItemKind;
use crate::path;
use crate::world::Mover;

/// No more young once a species has this many tame animals.
const HERD_CAP: usize = 24;

fn near(a: Pos, b: Pos, r: i32) -> bool {
    a.z == b.z && (a.x - b.x).abs() <= r && (a.y - b.y).abs() <= r
}

impl Game {
    /// A tame animal of `species` at `pos`.
    pub fn spawn_tame(&mut self, species: Species, pos: Pos, female: bool, adult: bool) -> u32 {
        let id = self.spawn_creature(species, pos, u64::MAX);
        let tick = self.tick;
        if let Some(c) = self.creatures.get_mut(&id) {
            c.tame = true;
            c.female = female;
            c.adult = adult;
            c.born = tick;
        }
        id
    }

    /// Release a caged creature as a tame animal; the cage stays behind, empty.
    pub fn tame_from_cage(&mut self, cage: u32) -> Result<String, String> {
        let Some(it) = self.items.get(&cage).filter(|it| it.kind == ItemKind::Cage && it.carried_by.is_none()) else {
            return Err("That's not a cage on the ground.".into());
        };
        let Some(label) = it.label.clone() else { return Err("The cage is empty.".into()) };
        let Some(species) = Species::from_name(&label) else {
            return Err(format!("A {} can't be tamed.", label));
        };
        if !species.tameable() {
            return Err(format!("A {} won't be tamed.", label));
        }
        let pos = it.pos;
        if let Some(it) = self.items.get_mut(&cage) {
            it.label = None;
        }
        let female = self.rng.chance(0.5);
        self.spawn_tame(species, pos, female, true);
        let msg = format!("The {} has been tamed.", label);
        self.msg(&msg, (160, 230, 160));
        Ok(msg)
    }

    /// Next pasture for a tame animal (cycling through pastures, then none).
    pub fn cycle_pasture(&mut self, cid: u32) -> Option<String> {
        let pastures: Vec<u32> = self.zones.values().filter(|z| z.kind == ZoneKind::Pasture).map(|z| z.id).collect();
        let c = self.creatures.get_mut(&cid).filter(|c| c.tame)?;
        let next = match c.pasture.and_then(|p| pastures.iter().position(|&z| z == p)) {
            None => pastures.first().copied(),
            Some(i) => pastures.get(i + 1).copied(),
        };
        c.pasture = next;
        c.path.clear();
        Some(next.map_or("no pasture".to_string(), |z| format!("pasture {}", z)))
    }

    /// Put every tame grazer without a pasture into this one. Returns how many.
    pub fn assign_grazers(&mut self, zid: u32) -> usize {
        let mut n = 0;
        let grazer = |s: Species| s == Species::Goat || s.beast().map_or(false, |b| b.temper == crate::bestiary::Temper::Grazer);
        for c in self.creatures.values_mut().filter(|c| c.tame && grazer(c.species) && c.pasture.is_none()) {
            c.pasture = Some(zid);
            n += 1;
        }
        n
    }

    /// Next soldier for a dog to guard (cycling through soldiers, then none).
    pub fn cycle_dog_owner(&mut self, cid: u32) -> Option<String> {
        let soldiers: Vec<(u32, String)> = self.dwarves.iter().filter(|d| d.alive && d.soldier).map(|d| (d.id, d.name.clone())).collect();
        let c = self.creatures.get_mut(&cid).filter(|c| c.tame && c.species == Species::Dog)?;
        let next = match c.owner.and_then(|o| soldiers.iter().position(|(id, _)| *id == o)) {
            None => soldiers.first().cloned(),
            Some(i) => soldiers.get(i + 1).cloned(),
        };
        c.owner = next.as_ref().map(|(id, _)| *id);
        Some(next.map_or("nobody".to_string(), |(_, n)| n))
    }

    pub fn toggle_slaughter(&mut self, cid: u32) -> Option<bool> {
        let c = self.creatures.get_mut(&cid).filter(|c| c.tame)?;
        c.slaughter = !c.slaughter;
        Some(c.slaughter)
    }

    /// Embark animals: a pair of dogs, a pair of goats and a cat.
    pub(super) fn bring_animals(&mut self, near_pos: Pos, which: &[(Species, bool)]) {
        for &(species, female) in which {
            let p = self.random_spot_near(near_pos, 3);
            self.spawn_tame(species, p, female, true);
        }
    }

    /// A tame animal's turn. Returns false if it's gone.
    pub(super) fn tame_tick(&mut self, c: &mut Creature) -> bool {
        // Eggs and wool from the bestiary's kept animals (beasts.rs).
        self.kept_products(c);
        // Growing up and giving birth.
        if !c.adult && self.tick >= c.born + SEASON {
            c.adult = true;
        }
        if c.pregnant > 0 && self.tick >= c.pregnant {
            c.pregnant = 0;
            let n = self.rng.range(1, c.species.litter() + 1);
            for _ in 0..n {
                let female = self.rng.chance(0.5);
                let id = self.spawn_tame(c.species, c.pos, female, false);
                if let Some(young) = self.creatures.get_mut(&id) {
                    young.pasture = c.pasture;
                }
            }
            let what = if n == 1 { format!("a {}", c.species.young()) } else { format!("{} {}s", n, c.species.young()) };
            self.msg(&format!("A {} has given birth to {}.", c.name(), what), (160, 230, 160));
        }
        // Checked every tenth tick (those always get a full turn), about once a day on average.
        if c.female && c.adult && c.pregnant == 0 && self.tick % 10 == 0 && self.rng.chance(10.0 / DAY as f64) {
            let herd = self.creatures.values().filter(|o| o.tame && o.alive && o.species == c.species).count() + 1;
            let mate = self.creatures.values().any(|o| {
                o.tame && o.alive && o.adult && !o.female && o.species == c.species && near(o.pos, c.pos, 12)
            });
            if mate && herd < HERD_CAP && self.rng.chance(0.35) {
                c.pregnant = self.tick + MONTH;
            }
        }

        c.move_cd = c.move_cd.saturating_sub(1);
        c.attack_cd = c.attack_cd.saturating_sub(1);
        let info = c.species.info();

        // War dogs go for hostiles near them, and otherwise keep at their soldier's heel.
        if c.species == Species::Dog && c.adult {
            let foe = self
                .creatures
                .values()
                .filter(|o| o.hostile() && self.creature_active(o) && near(o.pos, c.pos, 6))
                .min_by_key(|o| (o.pos.x - c.pos.x).abs() + (o.pos.y - c.pos.y).abs())
                .map(|o| (o.id, o.pos));
            if let Some((fid, fpos)) = foe.filter(|_| c.owner.is_some()) {
                if super::combat::adjacent(fpos, c.pos) {
                    if c.attack_cd == 0 {
                        c.attack_cd = info.attack_delay;
                        self.beast_strikes_beast(c, fid);
                    }
                } else if c.move_cd == 0 {
                    if let Some(p) = path::astar(&self.world, c.pos, fpos, 2000, Mover::Dwarf) {
                        c.path = p.into_iter().rev().collect();
                    }
                    self.creature_step(c, info.move_delay);
                }
                return true;
            }
            if let Some(owner) = c.owner {
                match self.dwarves.iter().find(|d| d.id == owner && d.alive && d.soldier).map(|d| d.pos) {
                    Some(opos) => {
                        if !near(opos, c.pos, 2) && c.move_cd == 0 && c.path.is_empty() {
                            if let Some(p) = path::astar(&self.world, c.pos, opos, 4000, Mover::Dwarf) {
                                c.path = p.into_iter().rev().collect();
                            }
                        }
                        self.creature_step(c, info.move_delay);
                        return true;
                    }
                    None => c.owner = None,
                }
            }
        }

        // Pets keep near their dwarves.
        if c.pet_of.is_some() && self.pet_follow(c) {
            return true;
        }
        // Pastured animals stay in their pasture.
        if let Some(zid) = c.pasture {
            let Some(tiles) = self.zones.get(&zid).filter(|z| z.kind == ZoneKind::Pasture).map(|z| z.tiles.clone()) else {
                c.pasture = None;
                return true;
            };
            let inside = tiles.contains(&c.pos);
            if c.path.is_empty() && c.move_cd == 0 && inside && !self.rng.chance(0.2) {
                // Grazing: stand a while.
                c.move_cd = 10;
                return true;
            }
            if c.path.is_empty() && c.move_cd == 0 {
                let target = tiles[self.rng.range(0, tiles.len() as i32) as usize];
                if let Some(p) = path::astar(&self.world, c.pos, target, 6000, Mover::Dwarf) {
                    c.path = p.into_iter().rev().collect();
                } else if !inside {
                    c.pasture = None;
                }
            }
            self.creature_step(c, info.move_delay + 2);
            return true;
        }

        // Everyone else potters about, resting a while between steps.
        if c.path.is_empty() && c.move_cd == 0 {
            if self.rng.chance(0.3) {
                let d = DIRS8[self.rng.range(0, 8) as usize];
                let q = c.pos.offset(d.0, d.1, 0);
                if self.world.passable(q, Mover::Dwarf) {
                    c.path = vec![q];
                }
            } else {
                c.move_cd = 10;
                return true;
            }
        }
        self.creature_step(c, info.move_delay + 2);
        true
    }

    /// One animal bites or kicks another (a war dog at a goblin, say).
    fn beast_strikes_beast(&mut self, c: &Creature, target: u32) {
        let Some(t) = self.creatures.get(&target) else { return };
        let att = self.creature_fighter(c);
        let def = self.creature_fighter(t);
        let mut body = t.body.clone();
        let (tpos, tsp, owner, seen) = (t.pos, t.species, super::gore::owner_of_creature(t), super::melee::a_or_name(t));
        let blow = self.strike(&att, &def, &mut body);
        if let Some(m) = blow.msg.as_ref().filter(|_| blow.notable) {
            self.msg(&format!("The war dog: {}", m.trim_start_matches("The ").trim_start_matches("the ")), (235, 225, 130));
        }
        self.aftermath(&blow, c.pos, tpos, &owner, &seen, Some(tsp), false);
        let dead = body.dead;
        if let Some(t) = self.creatures.get_mut(&target) {
            t.body = body;
        }
        if dead {
            if let Some(t) = self.creatures.remove(&target) {
                self.creature_died(t);
            }
        }
    }

    /// A butcher goes to an animal marked for slaughter.
    pub(super) fn find_slaughter(&mut self, d: &mut Dwarf) -> bool {
        let marked: Vec<(u32, Pos)> = self
            .creatures
            .values()
            .filter(|c| c.tame && c.slaughter && c.alive && !self.slaughtering.contains(&c.id))
            .map(|c| (c.id, c.pos))
            .collect();
        if marked.is_empty() {
            return false;
        }
        let goal = |q: Pos| marked.iter().any(|&(_, p)| near(p, q, 1));
        let Some((at, path)) = path::search(&self.world, d.pos, SEARCH_LIMIT, Mover::Dwarf, goal) else { return false };
        let (cid, _) = *marked.iter().find(|&&(_, p)| near(p, at, 1)).unwrap();
        self.slaughtering.insert(cid);
        d.task = Task::Slaughter { creature: cid };
        Self::set_path(d, path);
        true
    }

    pub(super) fn do_slaughter(&mut self, d: &mut Dwarf, cid: u32) {
        let Some(cpos) = self.creatures.get(&cid).filter(|c| c.alive).map(|c| c.pos) else {
            self.release_task(d);
            return;
        };
        if !near(cpos, d.pos, 1) {
            if let super::Travel::Failed = self.travel(d, cpos) {
                self.release_task(d);
            }
            return;
        }
        if Self::labor(d, 40, d.skill(Labor::Butchery)) {
            self.slaughtering.remove(&cid);
            d.task = Task::Idle;
            d.gain_xp(Labor::Butchery, 10);
            if let Some(c) = self.creatures.remove(&cid) {
                let name = c.name();
                self.msg(&format!("{} has slaughtered a {}.", d.name, name), (200, 170, 140));
                self.creature_died(c);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::Mat;
    use crate::world::TileKind;

    #[test]
    fn a_pastured_pair_breeds_within_a_season() {
        let mut g = Game::new(91);
        let e = g.embark;
        // A pasture of open floor beside the wagon.
        let _ = e;
        let a = g.flat_patch(6, 6, 3);
        assert!(g.add_zone(a, a.offset(5, 5, 0), ZoneKind::Pasture) >= 20, "no room for a pasture");
        // The corner tile itself may not be in the zone (a tree, say); the zone is the newest.
        let zid = *g.zones.keys().max().unwrap();
        let goats: Vec<u32> = g.creatures.values().filter(|c| c.tame && c.species == Species::Goat).map(|c| c.id).collect();
        assert_eq!(goats.len(), 2, "embark should bring a pair of goats");
        for id in &goats {
            g.creatures.get_mut(id).unwrap().pasture = Some(zid);
        }
        let count = |g: &Game| g.creatures.values().filter(|c| c.tame && c.alive && c.species == Species::Goat).count();
        for _ in 0..SEASON {
            g.step();
        }
        assert!(count(&g) > 2, "no kids after a season ({} goats)", count(&g));
        let tiles = g.zones[&zid].tiles.clone();
        let strays = g.creatures.values().filter(|c| c.species == Species::Goat && c.adult && !tiles.contains(&c.pos)).count();
        assert!(strays == 0, "{} adult goats wandered out of the pasture", strays);
    }

    #[test]
    fn caged_wolves_can_be_tamed_and_slaughtered() {
        let mut g = Game::new(92);
        let e = g.embark;
        let p = (1..10).map(|i| e.offset(i, 2, 0)).find(|p| g.world.t(*p).kind == TileKind::Floor).unwrap();
        let cage = g.spawn_item(ItemKind::Cage, Mat::Wood, p);
        g.items.get_mut(&cage).unwrap().label = Some("goblin".into());
        assert!(g.tame_from_cage(cage).is_err(), "goblins aren't pets");
        g.items.get_mut(&cage).unwrap().label = Some("wolf".into());
        assert!(g.tame_from_cage(cage).is_ok());
        let wolf = g.creatures.values().find(|c| c.species == Species::Wolf && c.tame).map(|c| c.id).expect("no tame wolf");
        assert!(!g.creatures[&wolf].hostile(), "a tame wolf is no threat");
        assert_eq!(g.toggle_slaughter(wolf), Some(true));
        let carcasses = g.items.values().filter(|it| it.kind == ItemKind::Carcass).count();
        for _ in 0..DAY {
            g.step();
            if !g.creatures.contains_key(&wolf) {
                break;
            }
        }
        assert!(!g.creatures.contains_key(&wolf), "nobody slaughtered the wolf");
        assert!(g.items.values().filter(|it| it.kind == ItemKind::Carcass).count() > carcasses);
    }

    #[test]
    fn war_dogs_keep_to_their_soldier() {
        let mut g = Game::new(93);
        let soldier = g.dwarves[2].id;
        g.toggle_soldier(soldier);
        let dog = g.creatures.values().find(|c| c.tame && c.species == Species::Dog && c.adult).map(|c| c.id).unwrap();
        assert_eq!(g.cycle_dog_owner(dog).as_deref(), Some(g.dwarves[2].name.as_str()));
        // Station the soldier away from the wagon and see whether the dog comes along.
        let e = g.embark;
        let st = (6..20).map(|i| e.offset(i, 0, 0)).map(|p| Pos::new(p.x, p.y, g.world.surface_z(p.x, p.y))).find(|p| g.world.walkable(*p)).unwrap();
        g.station = Some(st);
        for _ in 0..DAY / 2 {
            g.step();
        }
        let spos = g.dwarves[2].pos;
        let dpos = g.creatures[&dog].pos;
        assert!(near(spos, dpos, 3), "dog at {:?}, soldier at {:?}", dpos, spos);
    }
}
