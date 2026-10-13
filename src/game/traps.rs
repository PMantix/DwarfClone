//! Traps that fire on creatures, and levers that work doors and drawbridges.

use super::{Game, Travel, SEARCH_LIMIT};
use crate::buildings::BuildingKind;
use crate::creature::Species;
use crate::dwarf::{Dwarf, Labor, Task};
use crate::geom::{Pos, DIRS8};
use crate::items::ItemKind;
use crate::material::Mat;
use crate::path;
use crate::world::{Mover, BRIDGE_RAISED, DOOR_LOCKED, DOOR_NONE, DOOR_OPEN};

impl Game {
    /// Every tick: armed traps go off on whatever creature stands on them.
    /// Dwarves know where their traps are and never set them off.
    pub(super) fn traps_tick(&mut self) {
        let traps: Vec<(u32, Pos)> = self
            .buildings
            .values()
            .filter(|b| b.kind.is_trap() && b.built && b.trap_cd <= self.tick)
            .map(|b| (b.id, b.pos))
            .collect();
        for (tid, pos) in traps {
            let victim = self
                .creatures
                .values()
                .find(|c| c.alive && c.pos == pos && c.species != Species::Merchant && !c.tame)
                .map(|c| c.id);
            if let Some(cid) = victim {
                self.spring_trap(tid, cid);
            }
        }
    }

    fn spring_trap(&mut self, tid: u32, cid: u32) {
        let Some(b) = self.buildings.get(&tid) else { return };
        let (kind, pos, quality) = (b.kind, b.pos, b.quality);
        let load = b.load.unwrap_or((ItemKind::Boulder, Mat::Granite, 0));
        let name = self.creatures.get(&cid).map_or("creature", |c| c.name()).to_string();
        match kind {
            BuildingKind::CageTrap => {
                let Some(mut c) = self.creatures.remove(&cid) else { return };
                for id in c.weapon.take().into_iter().chain(c.armor.drain(..)) {
                    self.drop_item(id, pos);
                }
                let cage = self.spawn_item(ItemKind::Cage, load.1, pos);
                if let Some(it) = self.items.get_mut(&cage) {
                    it.label = Some(name.clone());
                }
                self.msg(&format!("A {} has been caught in a cage trap!", name), (160, 230, 160));
                self.rearm(tid);
            }
            BuildingKind::WeaponTrap | BuildingKind::StoneTrap => {
                let stone = kind == BuildingKind::StoneTrap;
                let (dmg, verb, edged, hits) = if stone {
                    (14.0 * load.1.weapon_mult(), "crushes", false, 1)
                } else {
                    // Better-made weapons hit harder.
                    let d = load.0.weapon_damage().unwrap_or(8.0) * load.1.weapon_mult() * (1.0 + 0.1 * load.2 as f32);
                    (d, load.0.weapon_verb(), load.0.edged(), 3)
                };
                let att = super::combat::Fighter {
                    name: if stone { "the falling stone".into() } else { "the weapon trap".into() },
                    skill: 5 + quality as u32,
                    dmg,
                    verb,
                    edged,
                    helm: 0,
                    mail: 0,
                    shield: false,
                };
                for _ in 0..hits {
                    let Some(c) = self.creatures.get(&cid) else { break };
                    if !c.alive {
                        break;
                    }
                    let def = self.creature_fighter(c);
                    let mut body = c.body.clone();
                    if let Some(m) = self.strike(&att, &def, &mut body) {
                        self.msg(&m, (235, 200, 130));
                    }
                    let dead = body.dead;
                    if let Some(c) = self.creatures.get_mut(&cid) {
                        c.body = body;
                    }
                    if dead {
                        if let Some(c) = self.creatures.remove(&cid) {
                            self.creature_died(c);
                        }
                        break;
                    }
                }
                if stone {
                    // The stone lands on the tile; the trap needs another.
                    self.spawn_item(ItemKind::Boulder, load.1, pos);
                    self.rearm(tid);
                } else if let Some(b) = self.buildings.get_mut(&tid) {
                    b.trap_cd = self.tick + 25;
                }
            }
            _ => {}
        }
    }

    /// A spent cage or stone trap needs its last part delivered again.
    fn rearm(&mut self, tid: u32) {
        if let Some(b) = self.buildings.get_mut(&tid) {
            b.built = false;
            b.parts = b.kind.parts() - 1;
            b.load = None;
            b.retry_tick = 0;
        }
    }

    // ------------------------------------------------------------------ levers

    /// Link a lever to a door or drawbridge. Uses up a mechanism from stock.
    pub fn link_lever(&mut self, lever: u32, target: u32) -> Result<String, String> {
        let Some(t) = self.buildings.get(&target) else { return Err("Nothing to link there.".into()) };
        if !matches!(t.kind, BuildingKind::Door | BuildingKind::Bridge) {
            return Err("Levers work doors and drawbridges.".into());
        }
        let tname = t.name();
        if self.buildings.get(&lever).map_or(true, |l| l.kind != BuildingKind::Lever || !l.built) {
            return Err("The lever isn't built yet.".into());
        }
        if self.buildings[&lever].links.contains(&target) {
            return Err(format!("Already linked to the {}.", tname.to_lowercase()));
        }
        let Some(mech) = self
            .items
            .values()
            .find(|it| it.kind == ItemKind::Mechanism && it.carried_by.is_none() && it.claimed_by.is_none())
            .map(|it| it.id)
        else {
            return Err("Linking needs a spare mechanism.".into());
        };
        self.items.remove(&mech);
        self.buildings.get_mut(&lever).unwrap().links.push(target);
        Ok(format!("Lever linked to the {}.", tname.to_lowercase()))
    }

    pub fn request_pull(&mut self, lever: u32) -> bool {
        match self.buildings.get_mut(&lever) {
            Some(b) if b.kind == BuildingKind::Lever && b.built && !b.links.is_empty() => {
                b.pull = true;
                true
            }
            _ => false,
        }
    }

    pub(super) fn find_lever_pull(&mut self, d: &mut Dwarf) -> bool {
        let levers: std::collections::HashMap<Pos, u32> = self
            .buildings
            .values()
            .filter(|b| b.kind == BuildingKind::Lever && b.pull && b.worker.is_none())
            .map(|b| (b.pos, b.id))
            .collect();
        if levers.is_empty() {
            return false;
        }
        let Some((p, path)) = path::search(&self.world, d.pos, SEARCH_LIMIT, Mover::Dwarf, |p| levers.contains_key(&p)) else {
            return false;
        };
        let bld = levers[&p];
        self.buildings.get_mut(&bld).unwrap().worker = Some(d.id);
        d.task = Task::PullLever { bld };
        Self::set_path(d, path);
        true
    }

    pub(super) fn do_pull_lever(&mut self, d: &mut Dwarf, bld: u32) {
        let Some(pos) = self.buildings.get(&bld).map(|b| b.pos) else {
            self.release_task(d);
            return;
        };
        match self.travel(d, pos) {
            Travel::Moving => {}
            Travel::Failed => self.release_task(d),
            Travel::Arrived => {
                if Self::labor(d, 25, d.skill(Labor::Mechanics)) {
                    d.task = Task::Idle;
                    self.pull_lever(bld);
                }
            }
        }
    }

    /// The lever is thrown: linked doors lock or unlock, bridges rise or fall.
    pub fn pull_lever(&mut self, bld: u32) {
        let Some(l) = self.buildings.get_mut(&bld) else { return };
        l.pull = false;
        l.worker = None;
        l.raised = !l.raised;
        let links = l.links.clone();
        for t in links {
            let Some(b) = self.buildings.get_mut(&t) else { continue };
            let pos = b.pos;
            match b.kind {
                BuildingKind::Door if b.built => {
                    b.locked = !b.locked;
                    self.world.t_mut(pos).door = if b.locked { DOOR_LOCKED } else { DOOR_OPEN };
                }
                BuildingKind::Bridge if b.built => {
                    b.raised = !b.raised;
                    let raised = b.raised;
                    self.world.t_mut(pos).door = if raised { BRIDGE_RAISED } else { DOOR_NONE };
                    if raised {
                        self.clear_tile(pos);
                    }
                    self.dig_version += 1;
                }
                _ => {}
            }
        }
        self.msg("A lever has been pulled.", (220, 200, 120));
    }

    /// Whoever stands on a rising bridge is thrown off to one side.
    fn clear_tile(&mut self, pos: Pos) {
        let spot = DIRS8.iter().map(|&(dx, dy)| pos.offset(dx, dy, 0)).find(|&q| self.world.walkable(q));
        let Some(to) = spot else { return };
        for d in self.dwarves.iter_mut().filter(|d| d.pos == pos) {
            d.pos = to;
            d.path.clear();
        }
        for c in self.creatures.values_mut().filter(|c| c.pos == pos) {
            c.pos = to;
            c.path.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::DAY;
    use crate::world::TileKind;

    /// A free floor tile near the wagon with nothing on it.
    fn spot(g: &Game, skip: &[Pos]) -> Pos {
        let e = g.embark;
        for r in 3..25 {
            for dy in -r..=r {
                for dx in -r..=r {
                    let p = e.offset(dx, dy, 0);
                    let p = Pos::new(p.x, p.y, g.world.surface_z(p.x, p.y));
                    let t = g.world.t(p);
                    if t.kind == TileKind::Floor && t.water == 0 && t.building == 0 && t.zone == 0 && !skip.contains(&p) && g.items.values().all(|it| it.pos != p) {
                        return p;
                    }
                }
            }
        }
        panic!("no free spot");
    }

    fn build(g: &mut Game, kind: BuildingKind, at: Pos) -> u32 {
        assert!(g.place_building(kind, at));
        let id = *g.buildings.keys().max().unwrap();
        for _ in 0..3 * DAY {
            g.step();
            if g.buildings[&id].built {
                return id;
            }
        }
        panic!("{:?} never got built (parts {})", kind, g.buildings[&id].parts);
    }

    #[test]
    fn weapon_and_cage_traps_work() {
        let mut g = Game::new(51);
        let e = g.embark;
        for _ in 0..2 {
            g.spawn_item(ItemKind::Mechanism, Mat::Granite, e);
        }
        g.spawn_item(ItemKind::Spear, Mat::Iron, e);
        g.spawn_item(ItemKind::Cage, Mat::Wood, e);
        let a = spot(&g, &[]);
        let wt = build(&mut g, BuildingKind::WeaponTrap, a);
        // Whatever weapon was closest: the wagon's spare axe or our spear.
        assert!(g.buildings[&wt].load.map_or(false, |l| l.0.weapon_damage().is_some()), "trap not loaded with a weapon");
        let b = spot(&g, &[a]);
        let ct = build(&mut g, BuildingKind::CageTrap, b);

        // A goblin walks onto the weapon trap.
        let gob = g.spawn_creature(Species::Goblin, a, u64::MAX);
        let blood = g.creatures[&gob].body.blood;
        for _ in 0..5 {
            g.traps_tick();
            g.tick += 1;
        }
        let hurt = g.creatures.get(&gob).map_or(true, |c| c.body.blood < blood || c.body.bleeding > 0 || !c.alive || c.body.badly_hurt());
        assert!(hurt, "the weapon trap never hurt the goblin");

        // A wolf steps into the cage trap and ends up in a cage.
        let wolf = g.spawn_creature(Species::Wolf, b, u64::MAX);
        g.traps_tick();
        assert!(!g.creatures.contains_key(&wolf), "wolf not caught");
        assert!(g.items.values().any(|it| it.kind == ItemKind::Cage && it.label.as_deref() == Some("wolf")));
        assert!(!g.buildings[&ct].built, "cage trap should need a new cage");
    }

    #[test]
    fn lever_raises_and_lowers_a_drawbridge() {
        let mut g = Game::new(52);
        let e = g.embark;
        for _ in 0..3 {
            g.spawn_item(ItemKind::Mechanism, Mat::Granite, e);
        }
        let a = spot(&g, &[]);
        let bridge = build(&mut g, BuildingKind::Bridge, a);
        let l = spot(&g, &[a]);
        let lever = build(&mut g, BuildingKind::Lever, l);
        assert!(g.link_lever(lever, bridge).is_ok());
        assert!(g.world.passable(a, Mover::Goblin));
        assert!(g.request_pull(lever));
        for _ in 0..DAY {
            g.step();
            if !g.buildings[&lever].pull {
                break;
            }
        }
        assert!(g.buildings[&bridge].raised, "nobody pulled the lever");
        assert!(!g.world.passable(a, Mover::Goblin) && !g.world.passable(a, Mover::Dwarf));
        g.pull_lever(lever);
        assert!(!g.buildings[&bridge].raised);
        assert!(g.world.passable(a, Mover::Goblin));
    }

    #[test]
    fn goblin_charging_down_a_trapped_corridor_gets_hurt() {
        let mut g = Game::new(53);
        let e = g.embark;
        let z = e.z;
        // A closed corridor east of the wagon: the only way out is west, over the trap.
        for x in 3..=16 {
            for dy in -1..=1 {
                let p = Pos::new(e.x + x, e.y + dy, z);
                let open = dy == 0 && x < 16;
                g.world.t_mut(p).kind = if open { TileKind::Floor } else { TileKind::Wall };
                g.world.t_mut(p).water = 0;
                g.world.t_mut(p.down()).kind = TileKind::Wall;
                g.world.t_mut(p.up()).kind = TileKind::Open;
            }
        }
        let trap_at = Pos::new(e.x + 9, e.y, z);
        assert!(g.place_building(BuildingKind::WeaponTrap, trap_at));
        let tid = *g.buildings.keys().max().unwrap();
        {
            let b = g.buildings.get_mut(&tid).unwrap();
            b.built = true;
            b.parts = 2;
            b.load = Some((ItemKind::Sword, Mat::Steel, 3));
        }
        let gob = g.spawn_creature(Species::Goblin, Pos::new(e.x + 15, e.y, z), u64::MAX);
        let start_blood = g.creatures[&gob].body.blood;
        let mut hurt = false;
        for _ in 0..DAY / 2 {
            g.step();
            match g.creatures.get(&gob) {
                None => {
                    hurt = true;
                    break;
                }
                Some(c) if !c.alive || c.body.blood < start_blood || c.body.bleeding > 0 => {
                    hurt = true;
                    break;
                }
                Some(c) if c.pos.x < trap_at.x => break,
                _ => {}
            }
        }
        assert!(hurt, "the goblin got past the weapon trap unharmed");
    }

    #[test]
    fn mechanics_make_mechanisms() {
        let mut g = Game::new(54);
        let e = g.embark;
        let site = (0..60)
            .map(|i| e.offset(-10 + i % 12, -12 + i / 12 * 4, 0))
            .find(|&p| g.can_place(BuildingKind::Mechanic, p))
            .expect("no room for a workshop");
        assert!(g.place_building(BuildingKind::Mechanic, site));
        let shop = *g.buildings.keys().max().unwrap();
        g.buildings.get_mut(&shop).unwrap().built = true;
        for _ in 0..3 {
            g.spawn_item(ItemKind::Boulder, Mat::Granite, e);
        }
        let id = g.dwarves[0].id;
        g.set_labor(id, Labor::Mechanics, true);
        g.add_order(shop, crate::buildings::Recipe::Mechanism);
        for _ in 0..DAY {
            g.step();
        }
        assert!(g.items.values().any(|it| it.kind == ItemKind::Mechanism), "no mechanism made");
    }
}
