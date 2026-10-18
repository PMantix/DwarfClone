//! Squads: members, orders, uniforms, barracks training and crossbows.

use super::{Game, ZoneKind};
use crate::dwarf::{Dwarf, Task};
use crate::geom::Pos;
use crate::items::{Item, ItemKind};
use crate::material::Mat;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SquadOrder {
    /// Train in the barracks (or where they stand).
    Train,
    /// Hold the squad's station point.
    Station,
    /// Hunt down one creature.
    Kill(u32),
}

/// What the squad's uniform puts in their hands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UniformWeapon {
    Best,
    Axe,
    Sword,
    Spear,
    Hammer,
    Crossbow,
}

impl UniformWeapon {
    pub const ALL: [UniformWeapon; 6] =
        [UniformWeapon::Best, UniformWeapon::Axe, UniformWeapon::Sword, UniformWeapon::Spear, UniformWeapon::Hammer, UniformWeapon::Crossbow];

    pub fn name(self) -> &'static str {
        match self {
            UniformWeapon::Best => "best melee weapon",
            UniformWeapon::Axe => "battle axes",
            UniformWeapon::Sword => "short swords",
            UniformWeapon::Spear => "spears",
            UniformWeapon::Hammer => "war hammers",
            UniformWeapon::Crossbow => "crossbows",
        }
    }

    pub fn next(self) -> UniformWeapon {
        let i = Self::ALL.iter().position(|&w| w == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    /// Whether a soldier in this uniform would carry `kind`.
    pub fn accepts(self, kind: ItemKind) -> bool {
        match self {
            UniformWeapon::Best => kind.weapon_damage().is_some() && !matches!(kind, ItemKind::Crossbow | ItemKind::Pick),
            UniformWeapon::Axe => kind == ItemKind::Axe,
            UniformWeapon::Sword => kind == ItemKind::Sword,
            UniformWeapon::Spear => kind == ItemKind::Spear,
            UniformWeapon::Hammer => kind == ItemKind::Hammer,
            UniformWeapon::Crossbow => kind == ItemKind::Crossbow,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Squad {
    pub id: u32,
    pub name: String,
    pub members: Vec<u32>,
    pub station: Option<Pos>,
    pub order: SquadOrder,
    pub weapon: UniformWeapon,
    pub armor: bool,
}

const SQUAD_ADJ: [&str; 10] = ["Iron", "Granite", "Copper", "Silent", "Red", "Black", "Steel", "Grim", "Bright", "Deep"];
const SQUAD_NOUN: [&str; 10] = ["Hammers", "Shields", "Axes", "Wardens", "Spears", "Fists", "Bolts", "Anvils", "Picks", "Guard"];

/// Bolts a marksdwarf carries at most (two bundles).
pub const MAX_AMMO: u32 = 20;
/// How far a crossbow reaches.
pub const RANGE: i32 = 10;

impl Game {
    pub fn new_squad(&mut self) -> usize {
        let n = self.squads.len();
        let name = format!("The {} {}", SQUAD_ADJ[(n * 3 + self.tick as usize) % 10], SQUAD_NOUN[(n * 7 + 1) % 10]);
        let id = self.new_id();
        self.squads.push(Squad {
            id,
            name,
            members: Vec::new(),
            station: None,
            order: SquadOrder::Train,
            weapon: UniformWeapon::Best,
            armor: true,
        });
        self.squads.len() - 1
    }

    /// Disband a squad; its members go back to civilian life.
    pub fn delete_squad(&mut self, idx: usize) {
        if idx >= self.squads.len() {
            return;
        }
        let s = self.squads.remove(idx);
        for id in s.members {
            if self.dwarves.iter().any(|d| d.id == id && d.soldier) {
                self.toggle_soldier(id);
            }
        }
    }

    pub fn squad_of(&self, dwarf: u32) -> Option<&Squad> {
        self.squads.iter().find(|s| s.members.contains(&dwarf))
    }

    /// Put a dwarf in a squad (drafting them) or take them out (discharging them).
    pub fn set_squad_member(&mut self, idx: usize, dwarf: u32, on: bool) {
        if on && self.dwarves.iter().any(|d| d.id == dwarf && d.is_child()) {
            return;
        }
        for s in self.squads.iter_mut() {
            s.members.retain(|&m| m != dwarf);
        }
        let soldier = self.dwarves.iter().any(|d| d.id == dwarf && d.soldier);
        if on {
            if let Some(s) = self.squads.get_mut(idx) {
                s.members.push(dwarf);
            }
        }
        if on != soldier {
            self.toggle_soldier(dwarf);
        }
    }

    pub fn order_station(&mut self, idx: usize, at: Pos) {
        if let Some(s) = self.squads.get_mut(idx) {
            s.station = Some(at);
            s.order = SquadOrder::Station;
        }
    }

    pub fn order_train(&mut self, idx: usize) {
        if let Some(s) = self.squads.get_mut(idx) {
            s.order = SquadOrder::Train;
        }
    }

    pub fn order_kill(&mut self, idx: usize, creature: u32) -> Result<String, String> {
        let Some(c) = self.creatures.get(&creature).filter(|c| c.alive) else { return Err("Nothing to kill there.".into()) };
        let what = c.name().to_string();
        let Some(s) = self.squads.get_mut(idx) else { return Err("No such squad.".into()) };
        s.order = SquadOrder::Kill(creature);
        let msg = format!("{} will hunt down the {}.", s.name, what);
        self.msg(&msg, (240, 200, 120));
        Ok(msg)
    }

    /// The squad's uniform: (weapon, wears armor). Soldiers without a squad take the best of everything.
    pub(super) fn uniform_of(&self, dwarf: u32) -> (UniformWeapon, bool) {
        self.squad_of(dwarf).map_or((UniformWeapon::Best, true), |s| (s.weapon, s.armor))
    }

    /// Orders for a soldier with nothing more pressing. Returns true if a task was set.
    pub(super) fn squad_task(&mut self, d: &mut Dwarf) -> bool {
        let Some(idx) = self.squads.iter().position(|s| s.members.contains(&d.id)) else { return false };
        match self.squads[idx].order {
            SquadOrder::Kill(target) => {
                if self.creatures.get(&target).map_or(false, |c| c.alive) {
                    d.task = Task::Attack { target };
                    return true;
                }
                let name = self.squads[idx].name.clone();
                self.squads[idx].order = SquadOrder::Train;
                self.msg(&format!("{} has carried out its kill order.", name), (160, 230, 160));
                false
            }
            SquadOrder::Station => {
                let Some(st) = self.squads[idx].station else { return false };
                if (st.x - d.pos.x).abs() > 3 || (st.y - d.pos.y).abs() > 3 || st.z != d.pos.z {
                    d.task = Task::Station { pos: st };
                } else {
                    d.task = Task::Train;
                }
                true
            }
            SquadOrder::Train => false,
        }
    }

    /// Training goes on in the barracks when there is one.
    pub(super) fn go_train(&mut self, d: &mut Dwarf) {
        let barracks: Vec<Pos> = self
            .zones
            .values()
            .filter(|z| z.kind == ZoneKind::Barracks)
            .flat_map(|z| z.tiles.iter().copied())
            .filter(|&p| self.world.walkable(p))
            .collect();
        if !barracks.is_empty() && self.zone_kind_at(d.pos) != Some(ZoneKind::Barracks) {
            let p = barracks[self.rng.range(0, barracks.len() as i32) as usize];
            d.task = Task::Station { pos: p };
            return;
        }
        d.task = Task::Train;
    }

    /// Experience for a tick of drill: sparring partners in the barracks and
    /// archery targets for marksdwarves make it count for more.
    pub(super) fn drill_xp(&self, d: &Dwarf) -> u32 {
        let mut xp = 1;
        let in_barracks = self.zone_kind_at(d.pos) == Some(ZoneKind::Barracks);
        if in_barracks
            && self.dwarves.iter().any(|o| o.alive && o.soldier && o.id != d.id && crate::game::combat::adjacent(o.pos, d.pos))
        {
            xp += 1;
        }
        if self.holds_crossbow(d) {
            let target = self.buildings.values().any(|b| {
                b.kind == crate::buildings::BuildingKind::ArcheryTarget
                    && b.built
                    && b.pos.z == d.pos.z
                    && (b.pos.x - d.pos.x).abs().max((b.pos.y - d.pos.y).abs()) <= RANGE
                    && self.clear_shot(d.pos, b.pos)
            });
            if target {
                xp += 1;
            }
        }
        // A militia commander drills them harder: a quarter more on average,
        // half for a veteran, an eighth for a poor one.
        let every = match self.post_fit(super::Position::Commander) {
            Some(super::Fit::Good) => 2,
            Some(super::Fit::Fair) => 4,
            Some(super::Fit::Poor) => 8,
            None => 0,
        };
        if every > 0 && (self.tick + d.id as u64) % every == 0 {
            xp += 1;
        }
        xp
    }

    pub(super) fn holds_crossbow(&self, d: &Dwarf) -> bool {
        d.held.and_then(|id| self.items.get(&id)).map_or(false, |it| it.kind == ItemKind::Crossbow)
    }

    /// A straight line on one level with nothing solid in between.
    /// Fortifications don't block it: they're built to shoot through.
    pub fn clear_shot(&self, a: Pos, b: Pos) -> bool {
        if a.z != b.z {
            return false;
        }
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let steps = dx.abs().max(dy.abs());
        for i in 1..steps {
            let x = a.x + (dx * i + dx.signum() * steps / 2) / steps.max(1);
            let y = a.y + (dy * i + dy.signum() * steps / 2) / steps.max(1);
            if self.world.solid(Pos::new(x, y, a.z)) {
                return false;
            }
        }
        true
    }

    /// Shoot at a creature if loaded, in range and in sight. Returns true if a bolt flew.
    pub(super) fn try_shoot(&mut self, d: &mut Dwarf, target: u32) -> bool {
        if d.ammo == 0 || !self.holds_crossbow(d) {
            return false;
        }
        let Some(c) = self.creatures.get(&target).filter(|c| c.alive) else { return false };
        let (cpos, dpos) = (c.pos, d.pos);
        if (cpos.x - dpos.x).abs().max((cpos.y - dpos.y).abs()) > RANGE || !self.clear_shot(dpos, cpos) {
            return false;
        }
        if d.attack_cd > 0 {
            return true;
        }
        d.ammo -= 1;
        d.attack_cd = 14;
        d.fight_xp += 6;
        d.satisfy(crate::dwarf::NeedKind::Fight, 40);
        // Metal bolts bite deeper, and a metal crossbow draws harder.
        // (Gold is too soft to count: no harder draw, and its bolts score below copper.)
        let metal = |m: Option<Mat>| matches!(m, Some(Mat::Copper | Mat::Bronze | Mat::Iron | Mat::Steel));
        let bow = d.held.and_then(|id| self.items.get(&id)).map(|it| it.mat);
        let draw = if metal(bow) { self.knob("bolt.draw", 1.0) as u32 } else { 0 };
        let dmg = Self::bolt_damage(d.ammo_mat) * self.knob("bolt.mult", 1.0) as f32;
        let metal_bolt = metal(d.ammo_mat);
        if d.ammo == 0 {
            d.ammo_mat = None;
        }
        let att = super::combat::Fighter {
            name: d.called(),
            skill: d.fight_level() + draw,
            dmg,
            verb: "shoots",
            edged: true,
            helm: 0,
            mail: 0,
            shield: false,
            size: 1.0,
            teeth: true,
        };
        let def = self.creature_fighter(c);
        let mut body = c.body.clone();
        let (hp0, siege): (i32, bool) = (body.hp.iter().sum(), c.siege);
        let (cpos, csp, owner, seen) = (c.pos, c.species, super::gore::owner_of_creature(c), super::melee::a_or_name(c));
        let blow = self.strike(&att, &def, &mut body);
        if let Some(m) = blow.msg.as_ref().filter(|_| blow.notable) {
            self.msg(m, (235, 225, 130));
        }
        self.aftermath(&blow, d.pos, cpos, &owner, &seen, Some(csp), false);
        if siege {
            let hurt = body.hp.iter().sum::<i32>() < hp0;
            self.note_blow(super::annals::BY_BOLT, hurt);
        }
        if metal_bolt {
            self.count("bolts_metal", 1);
        }
        let dead = body.dead;
        if let Some(c) = self.creatures.get_mut(&target) {
            c.body = body;
            c.target = c.target.or(Some(d.id));
        }
        if dead {
            if let Some(c) = self.creatures.remove(&target) {
                self.credit_kill(d, &c);
                self.creature_died(c);
            }
        }
        true
    }

    /// A bundle of bolts to pick up, if a marksdwarf is running low.
    pub(super) fn wants_bolts(&self, d: &Dwarf) -> bool {
        (self.uniform_of(d.id).0 == UniformWeapon::Crossbow || self.holds_crossbow(d)) && d.ammo + 10 <= MAX_AMMO
    }
}

/// Bolt bundles are picked up into the quiver rather than held.
pub fn is_bolts(it: &Item) -> bool {
    it.kind == ItemKind::Bolt
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::Species;
    use crate::game::DAY;
    use crate::material::Mat;

    #[test]
    fn two_squads_hold_two_stations_and_a_kill_order_is_carried_out() {
        let mut g = Game::new(71);
        let e = g.embark;
        for _ in 0..4 {
            g.spawn_item(ItemKind::Spear, Mat::Iron, e);
        }
        let ids: Vec<u32> = g.dwarves.iter().take(4).map(|d| d.id).collect();
        let a = g.new_squad();
        let b = g.new_squad();
        g.set_squad_member(a, ids[0], true);
        g.set_squad_member(a, ids[1], true);
        g.set_squad_member(b, ids[2], true);
        g.set_squad_member(b, ids[3], true);
        assert!(g.dwarves.iter().filter(|d| d.soldier).count() == 4);
        let surface = |g: &Game, p: Pos| Pos::new(p.x, p.y, g.world.surface_z(p.x, p.y));
        // Two stations either side of the wagon, on dry ground the squads can walk to.
        let station = |g: &Game, dir: i32| {
            (6..20)
                .flat_map(|r| (-4..=4).map(move |dy| e.offset(dir * r, dy, 0)))
                .map(|p| surface(g, p))
                .find(|&p| g.world.walkable(p) && g.world.t(p).water == 0 && crate::path::astar(&g.world, e, p, 20_000, crate::world::Mover::Dwarf).is_some())
                .expect("no dry ground for a station")
        };
        let (sa, sb) = (station(&g, -1), station(&g, 1));
        g.order_station(a, sa);
        g.order_station(b, sb);
        for _ in 0..DAY / 2 {
            g.step();
        }
        let near = |p: Pos, q: Pos| (p.x - q.x).abs() <= 3 && (p.y - q.y).abs() <= 3;
        for (squad, st) in [(a, sa), (b, sb)] {
            for m in &g.squads[squad].members {
                let d = g.dwarves.iter().find(|d| d.id == *m).unwrap();
                assert!(near(d.pos, st), "{} isn't at {:?} but {:?}", d.name, st, d.pos);
            }
        }

        // A deer wanders by; squad A is told to kill it.
        let deer = g.spawn_creature(Species::Deer, surface(&g, e.offset(0, 6, 0)), u64::MAX);
        assert!(g.order_kill(a, deer).is_ok());
        for _ in 0..DAY {
            g.step();
            if !g.creatures.contains_key(&deer) {
                break;
            }
        }
        assert!(!g.creatures.get(&deer).map_or(false, |c| c.alive), "the deer survived the kill order");
        assert_eq!(g.squads[a].order, SquadOrder::Train);
    }

    #[test]
    fn marksdwarves_shoot_from_range() {
        let mut g = Game::new(1);
        let e = g.embark;
        g.alert = false;
        g.spawn_item(ItemKind::Crossbow, Mat::Wood, e);
        g.spawn_item(ItemKind::Bolt, Mat::Wood, e);
        let s = g.new_squad();
        g.squads[s].weapon = UniformWeapon::Crossbow;
        g.squads[s].armor = false;
        let id = g.dwarves[0].id;
        g.set_squad_member(s, id, true);
        for _ in 0..DAY / 2 {
            g.step();
        }
        let d = g.dwarves.iter().find(|d| d.id == id).unwrap().clone();
        assert!(g.holds_crossbow(&d), "no crossbow picked up");
        assert_eq!(d.ammo, 10, "no bolts picked up");
        // Put a wolf six tiles off in the open and let fly.
        let mut d = d;
        let spot = (2..9)
            .map(|i| d.pos.offset(i, 0, 0))
            .find(|&p| g.world.walkable(p) && g.clear_shot(d.pos, p) && (p.x - d.pos.x) >= 5)
            .expect("no clear line");
        let wolf = g.spawn_creature(Species::Wolf, spot, u64::MAX);
        let mut shots = 0;
        for _ in 0..200 {
            d.attack_cd = d.attack_cd.saturating_sub(1);
            if g.try_shoot(&mut d, wolf) && d.attack_cd == 14 {
                shots += 1;
            }
            if !g.creatures.get(&wolf).map_or(false, |c| c.alive) {
                break;
            }
        }
        assert!(shots > 0, "never shot");
        assert!(d.ammo < 10);
    }

    /// Iron bolts hit harder than wooden ones, and the quiver says which it holds.
    #[test]
    fn iron_bolts_hit_harder() {
        let mean = |mat: Mat| {
            let mut g = Game::new(73);
            let gob = g.spawn_creature(Species::Goblin, g.embark, u64::MAX);
            let def = g.creature_fighter(&g.creatures[&gob]);
            let fresh = g.creatures[&gob].body.clone();
            let att = crate::game::combat::Fighter {
                name: "a marksdwarf".into(),
                skill: 5,
                dmg: Game::bolt_damage(Some(mat)),
                verb: "shoots",
                edged: true,
                helm: 0,
                mail: 0,
                shield: false,
                size: 1.0,
                teeth: true,
            };
            let mut lost = 0;
            for _ in 0..400 {
                let mut body = fresh.clone();
                g.strike(&att, &def, &mut body);
                lost += fresh.hp.iter().sum::<i32>() - body.hp.iter().sum::<i32>();
            }
            lost as f64 / 400.0
        };
        let (wood, iron) = (mean(Mat::Wood), mean(Mat::Iron));
        eprintln!("mean damage: wood {:.2} iron {:.2}", wood, iron);
        assert!(iron > wood + 2.0, "iron {:.2} vs wood {:.2}", iron, wood);
        assert!(Game::bolt_damage(Some(Mat::Steel)) > Game::bolt_damage(Some(Mat::Bronze)));
        assert_eq!(Game::bolt_damage(None), Game::bolt_damage(Some(Mat::Wood)));

        // A quiver topped up with iron shoots iron, and is counted.
        let mut g = Game::new(72);
        let e = g.embark;
        let bow = g.spawn_item(ItemKind::Crossbow, Mat::Iron, e);
        let wood = g.spawn_item(ItemKind::Bolt, Mat::Wood, e);
        let iron = g.spawn_item(ItemKind::Bolt, Mat::Iron, e);
        let mut d = g.dwarves[0].clone();
        d.pos = e;
        g.items.get_mut(&bow).unwrap().carried_by = Some(d.id);
        d.held = Some(bow);
        for b in [wood, iron] {
            g.items.get_mut(&b).unwrap().pos = d.pos;
            g.do_equip(&mut d, b);
        }
        assert_eq!((d.ammo, d.ammo_mat), (20, Some(Mat::Iron)));
        let gob = g.spawn_creature(Species::Goblin, e.offset(1, 0, 0), u64::MAX);
        d.attack_cd = 0;
        assert!(g.try_shoot(&mut d, gob));
        assert_eq!(g.annals.counters.get("bolts_metal"), Some(&1));
        d.ammo = 1;
        d.attack_cd = 0;
        if g.creatures.contains_key(&gob) {
            assert!(g.try_shoot(&mut d, gob));
            assert_eq!((d.ammo, d.ammo_mat), (0, None), "an empty quiver forgets its bolts");
        }
    }

    /// A goblin at a marksdwarf's elbow on open ground: they step back and
    /// shoot between steps, and never swing the crossbow while they can
    /// still back away.
    #[test]
    fn a_marksdwarf_backs_off_and_shoots() {
        let mut g = Game::new(71);
        g.creatures.retain(|_, c| c.tame);
        g.dwarves.truncate(1);
        let a = g.flat_patch(11, 11, 6).offset(5, 5, 0);
        let id = g.dwarves[0].id;
        let s = g.new_squad();
        g.squads[s].weapon = UniformWeapon::Crossbow;
        g.squads[s].armor = false;
        g.set_squad_member(s, id, true);
        let bow = g.spawn_item(ItemKind::Crossbow, Mat::Iron, a);
        g.items.get_mut(&bow).unwrap().carried_by = Some(id);
        // A goblin in iron, so it lasts out the test.
        let gob = g.spawn_creature(Species::Goblin, a.offset(1, 0, 0), u64::MAX);
        let helm = g.spawn_item(ItemKind::Helm, Mat::Iron, a);
        let mail = g.spawn_item(ItemKind::Breastplate, Mat::Iron, a);
        {
            let c = g.creatures.get_mut(&gob).unwrap();
            c.siege = true;
            c.target = Some(id);
            c.armor = vec![helm, mail];
        }
        g.alert = true;
        let d = &mut g.dwarves[0];
        d.pos = a;
        d.held = Some(bow);
        d.ammo = 10;
        d.ammo_mat = Some(Mat::Wood);
        d.task = Task::Attack { target: gob };
        g.note_siege("goblins", 1, "nobody");
        for _ in 0..100 {
            g.step();
            if !g.creatures.contains_key(&gob) || !g.dwarves[0].alive {
                break;
            }
        }
        let d = &g.dwarves[0];
        let blows = g.annals.sieges[0].blows;
        let (melee, shots) = (blows[crate::game::annals::BY_SOLDIER].0, blows[crate::game::annals::BY_BOLT].0);
        let backoffs = g.annals.counters.get("archer.backoff").copied().unwrap_or(0);
        eprintln!("shots {} melee {} backoffs {} ammo left {}", shots, melee, backoffs, d.ammo);
        assert!(shots >= 3, "only {} shots in 100 ticks", shots);
        assert_eq!(10 - d.ammo, shots, "every shot costs a bolt");
        assert!(backoffs >= 3, "stepped back only {} times", backoffs);
        assert_eq!(melee, 0, "swung the crossbow with bolts in the quiver");
        assert!(g.log.iter().any(|l| l.text.contains("backs away from the goblin")), "no word of it");
    }

    /// Out of bolts with nothing close, a marksdwarf goes back for more; and
    /// the overseer posts its crossbow squad back from the gate, with a clear
    /// shot at it.
    #[test]
    fn an_empty_quiver_is_filled_and_archers_stand_back() {
        let mut g = Game::new(71);
        g.creatures.retain(|_, c| c.tame);
        // Open ground that takes in the bolts (two rows down) and the goblin
        // (nine along), so nothing hangs on what the world puts round it.
        let a = g.open_patch(10, 4, 6).offset(0, 1, 0);
        let id = g.dwarves[0].id;
        let s = g.new_squad();
        g.squads[s].weapon = UniformWeapon::Crossbow;
        g.set_squad_member(s, id, true);
        let bow = g.spawn_item(ItemKind::Crossbow, Mat::Iron, a);
        g.items.get_mut(&bow).unwrap().carried_by = Some(id);
        g.spawn_item(ItemKind::Bolt, Mat::Iron, a.offset(0, 2, 0));
        let gob = g.spawn_creature(Species::Goblin, a.offset(9, 0, 0), u64::MAX);
        let mut d = std::mem::take(&mut g.dwarves[0]);
        d.pos = a;
        d.held = Some(bow);
        d.ammo = 0;
        d.task = Task::Attack { target: gob };
        g.do_attack(&mut d, gob);
        assert!(matches!(d.task, Task::Equip { .. }), "should go for bolts, not {:?}", d.task);
        // With the goblin at their elbow, they fight with what they have.
        let mut d2 = d.clone();
        g.release_task(&mut d2);
        g.creatures.get_mut(&gob).unwrap().pos = a.offset(1, 0, 0);
        d2.task = Task::Attack { target: gob };
        g.do_attack(&mut d2, gob);
        assert!(matches!(d2.task, Task::Attack { .. }));
        g.dwarves[0] = d;

        // The station: only for a second squad on crossbows, 5-7 tiles in,
        // in sight of the gate.
        let base = a;
        let s2 = g.new_squad();
        assert_eq!(g.archer_station(s, base), None, "the first squad holds the gate itself");
        g.squads[s2].weapon = UniformWeapon::Best;
        assert_eq!(g.archer_station(s2, base), None);
        g.squads[s2].weapon = UniformWeapon::Crossbow;
        let st = g.archer_station(s2, base).expect("open ground has a spot");
        assert!((5..=7).contains(&(st.x - base.x)) && g.clear_shot(st, base.offset(1, 0, 0)));

        // The overseer hands the second squad crossbows once two of metal are about, and says so.
        let mut o = crate::game::overseer::Overseer::new(crate::game::overseer::Style::Steady);
        g.squads[s2].weapon = UniformWeapon::Best;
        assert_eq!(g.archer_uniform(&mut o, s2), None, "one metal crossbow isn't enough");
        g.spawn_item(ItemKind::Crossbow, Mat::Bronze, a);
        assert_eq!(g.archer_uniform(&mut o, s2), Some(UniformWeapon::Crossbow));
        assert_eq!(g.archer_uniform(&mut o, s), None, "the first squad keeps its uniform");
        assert!(o.notes.iter().any(|(_, n)| n.contains("metal crossbows")), "{:?}", o.notes);
    }
}
