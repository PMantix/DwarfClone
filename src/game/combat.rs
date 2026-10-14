//! Creatures, fighting, wildlife and sieges.

use super::*;
use crate::body::{Body, Part, Wound};
use crate::creature::{Creature, Species};
use crate::items::Slot;
use crate::world::{DOOR_LOCKED, DOOR_NONE};

pub(super) struct Fighter {
    pub name: String,
    pub skill: u32,
    pub dmg: f32,
    pub verb: &'static str,
    pub edged: bool,
    pub helm: i32,
    pub mail: i32,
    pub shield: bool,
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

pub(super) fn adjacent(a: Pos, b: Pos) -> bool {
    a.z == b.z && (a.x - b.x).abs() <= 1 && (a.y - b.y).abs() <= 1
}

fn near(a: Pos, b: Pos, r: i32) -> bool {
    (a.z - b.z).abs() <= 1 && (a.x - b.x).abs() <= r && (a.y - b.y).abs() <= r
}

fn dist2(a: Pos, b: Pos) -> i32 {
    (a.x - b.x).pow(2) + (a.y - b.y).pow(2) + 4 * (a.z - b.z).pow(2)
}

impl Game {
    fn apply_gear(&self, f: &mut Fighter, held: Option<u32>, armor: &[u32], can_hold: bool) {
        if can_hold {
            if let Some(it) = held.and_then(|id| self.items.get(&id)) {
                if let Some(w) = it.kind.weapon_damage() {
                    let q = if it.quality >= crate::items::ARTIFACT { 1.5 } else { 1.0 + it.quality as f32 * 0.06 };
                    f.dmg = w * it.mat.weapon_mult() * q;
                    f.verb = it.kind.weapon_verb();
                    f.edged = it.kind.edged();
                }
            }
        }
        for it in armor.iter().filter_map(|id| self.items.get(id)) {
            let bonus = (it.quality as i32 + 1) / 2;
            match it.kind.armor_slot() {
                Some(Slot::Head) => f.helm = it.mat.armor_value() + bonus,
                Some(Slot::Body) => f.mail = it.mat.armor_value() + bonus,
                Some(Slot::Shield) => f.shield = true,
                None => {}
            }
        }
    }

    pub(super) fn dwarf_fighter(&self, d: &Dwarf) -> Fighter {
        let mut f = Fighter {
            name: d.name.clone(),
            skill: d.fight_level(),
            dmg: 3.0,
            verb: "punches",
            edged: false,
            helm: 0,
            mail: 0,
            shield: false,
        };
        self.apply_gear(&mut f, d.held, &d.armor, d.body.can_hold());
        f
    }

    pub(super) fn creature_fighter(&self, c: &Creature) -> Fighter {
        let info = c.species.info();
        let mut f = Fighter {
            name: format!("the {}", info.name),
            skill: info.skill,
            dmg: info.damage,
            verb: info.verb,
            edged: info.edged,
            helm: 0,
            mail: 0,
            shield: false,
        };
        self.apply_gear(&mut f, c.weapon, &c.armor, c.body.can_hold());
        f
    }

    /// Resolve one blow. Returns a log line for anything worth reporting.
    pub(super) fn strike(&mut self, att: &Fighter, def: &Fighter, body: &mut Body) -> Option<String> {
        let chance = (0.55 + 0.05 * (att.skill as f64 - def.skill as f64)).clamp(0.15, 0.95);
        if !self.rng.chance(chance) {
            return None;
        }
        if def.shield && self.rng.chance(0.25) {
            return Some(format!("{} blocks {}'s attack with a shield.", cap(&def.name), att.name));
        }
        let part = body.pick_part(&mut self.rng);
        let roll = 0.6 + self.rng.f64() as f32 * 0.8;
        let armor = match part {
            Part::Head => def.helm,
            Part::UpperBody | Part::LowerBody => def.mail,
            _ => def.mail / 2,
        };
        let dmg = (att.dmg * roll + att.skill as f32 * 0.5).round() as i32 - armor;
        if dmg <= 0 {
            return Some(format!(
                "{} {} {} in the {}, but the armor deflects the blow.",
                cap(&att.name),
                att.verb,
                def.name,
                part.name()
            ));
        }
        let suffix = match body.wound(part, dmg, att.edged) {
            Wound::Bruise => "",
            Wound::Gash => if att.edged { ", tearing the flesh" } else { ", bruising the muscle" },
            Wound::Mangle => ", mangling it!",
            Wound::Sever => ", severing it!",
            Wound::Fatal => " — a fatal blow!",
        };
        Some(format!("{} {} {} in the {}{}", cap(&att.name), att.verb, def.name, part.name(), suffix))
    }

    pub(super) fn creature_active(&self, c: &Creature) -> bool {
        c.alive && !c.body.unconscious() && (!c.species.dwells_underground() || self.world.t(c.pos).revealed)
    }

    /// Active hostile creatures close enough to worry about.
    pub(super) fn threats_near(&self, p: Pos, r: i32) -> Vec<(u32, Pos)> {
        self.creatures
            .values()
            .filter(|c| c.hostile() && self.creature_active(c) && near(c.pos, p, r))
            .map(|c| (c.id, c.pos))
            .collect()
    }

    pub fn hostile_count(&self) -> usize {
        self.creatures.values().filter(|c| c.hostile() && self.creature_active(c)).count()
    }

    pub(super) fn drop_item(&mut self, id: u32, pos: Pos) {
        let pos = self.world.settle(pos);
        if let Some(it) = self.items.get_mut(&id) {
            it.carried_by = None;
            it.claimed_by = None;
            it.pos = pos;
        }
    }

    // ------------------------------------------------------------------
    // Dwarves in combat
    // ------------------------------------------------------------------

    pub(super) fn dwarf_attack(&mut self, d: &mut Dwarf, cid: u32) {
        let Some(c) = self.creatures.get(&cid) else { return };
        if !c.alive {
            return;
        }
        let att = self.dwarf_fighter(d);
        let def = self.creature_fighter(c);
        let mut body = c.body.clone();
        let msg = self.strike(&att, &def, &mut body);
        d.attack_cd = 9 + d.body.limp();
        d.fight_xp += 8;
        d.satisfy(crate::dwarf::NeedKind::Fight, 60);
        if let Some(m) = msg {
            self.msg(&m, (235, 225, 130));
        }
        let dead = body.dead;
        if let Some(c) = self.creatures.get_mut(&cid) {
            c.body = body;
            c.target = c.target.or(Some(d.id));
        }
        if dead {
            if let Some(c) = self.creatures.remove(&cid) {
                self.creature_died(c);
            }
        }
    }

    /// Called every few ticks: soldiers engage, civilians flee or fight back.
    pub(super) fn combat_awareness(&mut self, d: &mut Dwarf) {
        let threats = self.threats_near(d.pos, 7);
        if d.attack_cd == 0 {
            if let Some(&(cid, _)) = threats.iter().find(|(_, p)| adjacent(*p, d.pos)) {
                let struck_recently = d.attacker.map_or(false, |(_, t)| self.tick < t + 40);
                if d.soldier || struck_recently || d.tantrum > 0 {
                    self.dwarf_attack(d, cid);
                    return;
                }
            }
        }
        if d.soldier {
            if !threats.is_empty() && !matches!(d.task, Task::Attack { .. }) && self.alert {
                let nearest = threats.iter().min_by_key(|(_, p)| dist2(*p, d.pos)).unwrap().0;
                if !d.task.is_need() || d.task == Task::Flee {
                    self.release_task(d);
                }
                if d.task == Task::Idle {
                    d.task = Task::Attack { target: nearest };
                }
            }
            return;
        }
        if !threats.is_empty() {
            if d.task != Task::Flee {
                self.release_task(d);
                d.task = Task::Flee;
                let name = d.name.clone();
                let what = self.creatures.get(&threats[0].0).map_or("something", |c| c.name());
                self.warn(&format!("flee{}", d.id), &format!("{} flees from the {}!", name, what), (255, 170, 80));
            }
        } else if d.task == Task::Flee && self.threats_near(d.pos, 11).is_empty() {
            d.task = Task::Idle;
        }
    }

    pub(super) fn do_flee(&mut self, d: &mut Dwarf) {
        if d.move_cd > 0 {
            d.move_cd -= 1;
            return;
        }
        let threats = self.threats_near(d.pos, 12);
        if threats.is_empty() {
            d.task = Task::Idle;
            return;
        }
        let score = |p: Pos| threats.iter().map(|(_, t)| dist2(*t, p)).min().unwrap_or(0);
        let mut nb = Vec::new();
        self.world.neighbors(d.pos, &mut nb, Mover::Dwarf);
        let here = score(d.pos);
        if let Some(&(best, _)) = nb.iter().max_by_key(|(q, _)| score(*q)) {
            if score(best) > here {
                d.pos = best;
                d.path.clear();
                d.move_cd = MOVE_DELAY + d.body.limp();
            }
        }
    }

    pub(super) fn do_attack(&mut self, d: &mut Dwarf, target: u32) {
        let Some(cpos) = self.creatures.get(&target).filter(|c| c.alive).map(|c| c.pos) else {
            d.task = Task::Idle;
            return;
        };
        if adjacent(cpos, d.pos) {
            if d.attack_cd == 0 {
                self.dwarf_attack(d, target);
            }
            return;
        }
        // Marksdwarves shoot from where they stand.
        if self.try_shoot(d, target) {
            return;
        }
        if let Travel::Failed = self.travel(d, cpos) {
            d.task = Task::Idle;
            d.idle_cd = 30;
        }
    }

    /// Soldiers pick up the best weapon and any missing armor before anything else.
    pub(super) fn equip_soldier(&mut self, d: &mut Dwarf) -> bool {
        // The squad's uniform says which weapon; within that, the best material.
        let (weapon, armor) = self.uniform_of(d.id);
        let score = |it: &Item| {
            if !weapon.accepts(it.kind) {
                return 0.0;
            }
            it.kind.weapon_damage().map_or(0.0, |w| w * it.mat.weapon_mult()) + 0.1
        };
        let current = d.held.and_then(|id| self.items.get(&id)).map_or(0.0, score);
        if let Some((item, path)) = self.find_item(d.pos, |it| score(it) > current + 0.5 || (current == 0.0 && score(it) > 0.0)) {
            self.claim_item(item, d.id);
            d.task = Task::Equip { item };
            Self::set_path(d, path);
            return true;
        }
        if self.wants_bolts(d) {
            if let Some((item, path)) = self.find_item(d.pos, super::military::is_bolts) {
                self.claim_item(item, d.id);
                d.task = Task::Equip { item };
                Self::set_path(d, path);
                return true;
            }
        }
        if !armor {
            return false;
        }
        let worn: Vec<Slot> = d
            .armor
            .iter()
            .filter_map(|id| self.items.get(id).and_then(|it| it.kind.armor_slot()))
            .collect();
        if let Some((item, path)) =
            self.find_item(d.pos, |it| it.kind.armor_slot().map_or(false, |s| !worn.contains(&s)))
        {
            self.claim_item(item, d.id);
            d.task = Task::Equip { item };
            Self::set_path(d, path);
            return true;
        }
        false
    }

    /// Miners need a pick and woodcutters an axe. Returns false if the dwarf
    /// already holds one; otherwise starts fetching one (or warns there's none).
    pub(super) fn need_tool(&mut self, d: &mut Dwarf, tool: ItemKind) -> Option<bool> {
        if d.held.and_then(|id| self.items.get(&id)).map_or(false, |it| it.kind == tool) {
            return None;
        }
        if let Some((item, path)) = self.find_item(d.pos, |it| it.kind == tool) {
            self.claim_item(item, d.id);
            d.task = Task::Equip { item };
            Self::set_path(d, path);
            return Some(true);
        }
        let what = if tool == ItemKind::Pick { "pick" } else { "axe" };
        self.warn(&format!("tool{}", what), &format!("A dwarf needs a {} but none is available.", what), (230, 200, 90));
        Some(false)
    }

    pub(super) fn do_equip(&mut self, d: &mut Dwarf, item: u32) {
        let Some(ipos) = self.items.get(&item).filter(|it| it.carried_by.is_none()).map(|it| it.pos) else {
            self.release_task(d);
            return;
        };
        match self.travel(d, ipos) {
            Travel::Moving => {}
            Travel::Failed => self.release_task(d),
            Travel::Arrived => {
                let kind = self.items[&item].kind;
                if kind == ItemKind::Bolt {
                    // Bolts go into the quiver.
                    self.items.remove(&item);
                    d.ammo += 10;
                    d.task = Task::Idle;
                    return;
                }
                if let Some(slot) = kind.clothing_slot() {
                    let old: Vec<u32> = d
                        .clothes
                        .iter()
                        .copied()
                        .filter(|id| self.items.get(id).and_then(|it| it.kind.clothing_slot()) == Some(slot))
                        .collect();
                    for o in old {
                        d.clothes.retain(|&c| c != o);
                        self.drop_item(o, d.pos);
                    }
                    d.clothes.push(item);
                    d.think("was pleased to put on fresh clothes", -2, self.tick);
                } else if let Some(slot) = kind.armor_slot() {
                    let old: Vec<u32> = d
                        .armor
                        .iter()
                        .copied()
                        .filter(|id| self.items.get(id).and_then(|it| it.kind.armor_slot()) == Some(slot))
                        .collect();
                    for o in old {
                        d.armor.retain(|&a| a != o);
                        self.drop_item(o, d.pos);
                    }
                    d.armor.push(item);
                } else {
                    if let Some(old) = d.held.take() {
                        self.drop_item(old, d.pos);
                    }
                    d.held = Some(item);
                }
                if let Some(it) = self.items.get_mut(&item) {
                    it.carried_by = Some(d.id);
                    it.claimed_by = None;
                }
                d.task = Task::Idle;
            }
        }
    }

    // ------------------------------------------------------------------
    // Creatures
    // ------------------------------------------------------------------

    pub(super) fn spawn_creature(&mut self, species: Species, pos: Pos, stay: u64) -> u32 {
        let id = self.new_id();
        let leave = self.tick.saturating_add(stay);
        self.creatures.insert(id, Creature::new(id, species, pos, leave));
        id
    }

    pub(super) fn edge_entry(&mut self) -> Option<Pos> {
        let (w, h) = (self.world.w, self.world.h);
        for _ in 0..200 {
            let (x, y) = match self.rng.range(0, 4) {
                0 => (0, self.rng.range(0, h)),
                1 => (w - 1, self.rng.range(0, h)),
                2 => (self.rng.range(0, w), 0),
                _ => (self.rng.range(0, w), h - 1),
            };
            let p = Pos::new(x, y, self.world.surface_z(x, y));
            if self.world.walkable(p) && self.world.t(p).water == 0 {
                return Some(p);
            }
        }
        None
    }

    fn spot_near(&mut self, c: Pos, r: i32) -> Pos {
        for _ in 0..50 {
            let q = c.offset(self.rng.range(-r, r + 1), self.rng.range(-r, r + 1), 0);
            if self.world.walkable(q) {
                return q;
            }
        }
        c
    }

    pub(super) fn populate_caverns(&mut self) {
        let mut floors = Vec::new();
        let bands = self.world.caverns.clone();
        for z in bands.iter().flat_map(|&(lo, hi)| lo..=hi) {
            for y in 0..self.world.h {
                for x in 0..self.world.w {
                    let p = Pos::new(x, y, z);
                    let t = self.world.t(p);
                    if !t.revealed && t.kind == TileKind::Floor && t.water == 0 {
                        floors.push(p);
                    }
                }
            }
        }
        if floors.is_empty() {
            return;
        }
        for _ in 0..2 {
            let p = *self.rng.pick(&floors);
            self.spawn_creature(Species::GiantSpider, p, u64::MAX);
        }
        for _ in 0..2 {
            let c = *self.rng.pick(&floors);
            for _ in 0..3 {
                let p = self.spot_near(c, 2);
                self.spawn_creature(Species::Troglodyte, p, u64::MAX);
            }
        }
    }

    pub(super) fn wildlife(&mut self) {
        if !self.rng.chance(0.18) {
            return;
        }
        let Some(entry) = self.edge_entry() else { return };
        let roll = self.rng.f64();
        let tame = 1.0 - self.savagery.clamp(0.05, 0.9) * 0.7;
        let (species, n) = if roll < tame || self.tick < SEASON {
            (Species::Deer, self.rng.range(2, 5))
        } else if roll < 0.95 || self.tick < YEAR {
            (Species::Wolf, self.rng.range(2, 4))
        } else {
            (Species::Bear, 1)
        };
        let stay = DAY * self.rng.range(2, 5) as u64;
        for _ in 0..n {
            let p = self.spot_near(entry, 2);
            self.spawn_creature(species, p, stay);
        }
        let info = species.info();
        if info.aggro > 0 {
            let what = if n > 1 { format!("A pack of {}", info.plural) } else { format!("A {}", info.name) };
            self.msg(&format!("{} has been spotted nearby.", what), (255, 170, 80));
        }
    }

    pub(super) fn maybe_siege(&mut self) {
        let pop = self.alive_count() as f64;
        let goblins = self.neighbors.iter().filter(|n| n.race == Race::Goblin).map(|n| n.dist).min();
        let Some(dist) = goblins else { return };
        let near = (1.0 - dist as f64 / 35.0).max(0.2);
        if self.tick < SEASON * 2 || !self.rng.chance((0.3 + pop * 0.02) * near + self.wealth() as f64 / 200_000.0) {
            return;
        }
        let Some(entry) = self.edge_entry() else { return };
        let first = self.siege_size == 0;
        let n = (3 + self.alive_count() / 4).min(if first { 5 } else { 20 });
        let stay = DAY * 5;
        for _ in 0..n {
            let p = self.spot_near(entry, 3);
            let id = self.spawn_creature(Species::Goblin, p, stay);
            let mat = *self.rng.pick(&[Mat::Copper, Mat::Copper, Mat::Bronze, Mat::Iron]);
            let wk = *self.rng.pick(&[ItemKind::Spear, ItemKind::Sword, ItemKind::Axe, ItemKind::Hammer]);
            let mut gear = vec![wk];
            if self.rng.chance(0.5) {
                gear.push(ItemKind::Helm);
            }
            if self.rng.chance(0.3) {
                gear.push(ItemKind::Breastplate);
            }
            if self.rng.chance(0.25) {
                gear.push(ItemKind::Shield);
            }
            for k in gear {
                let iid = self.spawn_item(k, mat, p);
                if let Some(it) = self.items.get_mut(&iid) {
                    it.carried_by = Some(id);
                }
                let c = self.creatures.get_mut(&id).unwrap();
                if k.weapon_damage().is_some() {
                    c.weapon = Some(iid);
                } else {
                    c.armor.push(iid);
                }
                c.siege = true;
            }
        }
        self.siege_size = n as u32;
        self.siege_routed = false;
        self.msg(
            &format!("A vile force of darkness has arrived! ({} goblins)", n),
            (255, 70, 70),
        );
        let name = self.fortress_name.clone();
        self.legend(format!("Goblins laid siege to {}.", name));
        self.pause_request = true;
    }

    pub(super) fn update_creatures(&mut self) {
        let ids: Vec<u32> = self.creatures.keys().copied().collect();
        let body_tick = self.tick % 10 == 0;
        for id in ids {
            // Tame animals mostly wait out a step; do that in place, it's most of their ticks.
            if !body_tick {
                if let Some(c) = self.creatures.get_mut(&id) {
                    if c.tame && c.move_cd > 1 {
                        c.move_cd -= 1;
                        c.attack_cd = c.attack_cd.saturating_sub(1);
                        continue;
                    }
                }
            }
            let Some(mut c) = self.creatures.remove(&id) else { continue };
            if self.creature_tick(&mut c) {
                self.creatures.insert(id, c);
            }
        }
    }

    pub(super) fn creature_died(&mut self, mut c: Creature) {
        c.alive = false;
        // Squads sent after it are done.
        let mut done = Vec::new();
        for s in self.squads.iter_mut().filter(|s| s.order == super::SquadOrder::Kill(c.id)) {
            s.order = super::SquadOrder::Train;
            done.push(s.name.clone());
        }
        for name in done {
            self.msg(&format!("{} has carried out its kill order.", name), (160, 230, 160));
        }
        for id in c.weapon.take().into_iter().chain(c.armor.drain(..)) {
            self.drop_item(id, c.pos);
        }
        let info = c.species.info();
        if c.species == Species::Merchant {
            self.merchant_died(&c);
        }
        let kind = if matches!(c.species, Species::Goblin | Species::Merchant) { ItemKind::Corpse } else { ItemKind::Carcass };
        let id = self.spawn_item(kind, Mat::Flesh, c.pos);
        if let Some(it) = self.items.get_mut(&id) {
            it.label = Some(info.name.to_string());
        }
        if c.hostile() {
            self.msg(&format!("The {} has been slain.", info.name), (160, 230, 160));
        }
        if c.siege && !self.siege_routed {
            let left = self.creatures.values().filter(|o| o.siege && o.alive).count() as u32;
            if left * 2 < self.siege_size {
                self.siege_routed = true;
                for o in self.creatures.values_mut().filter(|o| o.siege) {
                    o.leaving = true;
                    o.target = None;
                    o.path.clear();
                }
                self.msg("The goblins are fleeing!", (120, 255, 120));
            }
        }
    }

    /// Returns false when the creature should be removed (died or left the map).
    fn creature_tick(&mut self, c: &mut Creature) -> bool {
        let info = c.species.info();
        if self.world.get(c.pos).map_or(false, |t| t.magma > 0) {
            self.msg(&format!("The {} is consumed by magma.", info.name), (255, 120, 40));
            self.creature_died(c.clone());
            return false;
        }
        if self.tick % 10 == 0 {
            c.body.tick();
            if c.body.dead {
                self.creature_died(c.clone());
                return false;
            }
            if self.tick % 200 == 0 {
                c.body.heal(1);
            }
        }
        if !self.creature_active(c) {
            return true;
        }
        if c.tame {
            return self.tame_tick(c);
        }
        // Animals lose their nerve when hurt; goblins only when close to death.
        let breaking = if c.siege { c.body.blood * 10 < c.body.max_blood * 4 } else { c.body.badly_hurt() };
        if c.hostile() && !c.leaving && breaking && !c.species.dwells_underground() {
            c.leaving = true;
            c.target = None;
            c.path.clear();
            self.msg(&format!("The wounded {} flees!", info.name), (160, 230, 160));
        }
        c.attack_cd = c.attack_cd.saturating_sub(1);
        c.move_cd = c.move_cd.saturating_sub(1);

        if !c.leaving && self.tick >= c.leave_tick {
            c.leaving = true;
            c.target = None;
            c.path.clear();
        }
        if c.leaving {
            let p = c.pos;
            if (p.x == 0 || p.y == 0 || p.x == self.world.w - 1 || p.y == self.world.h - 1) && self.world.outside(p) {
                for id in c.weapon.iter().chain(c.armor.iter()) {
                    self.items.remove(id);
                }
                return false;
            }
            if c.path.is_empty() && c.think_cd == 0 {
                c.think_cd = 50;
                let (w, h) = (self.world.w, self.world.h);
                let wo = &self.world;
                if let Some((_, path)) = path::search(wo, c.pos, SEARCH_LIMIT, info.mover, |q| {
                    (q.x == 0 || q.y == 0 || q.x == w - 1 || q.y == h - 1) && wo.outside(q)
                }) {
                    c.path = path.into_iter().rev().collect();
                }
            }
            c.think_cd = c.think_cd.saturating_sub(1);
            self.creature_step(c, info.move_delay);
            return true;
        }

        if c.think_cd == 0 {
            c.think_cd = 15;
            self.creature_think(c);
        } else {
            c.think_cd -= 1;
        }

        if let Some(tid) = c.target {
            let Some(di) = self.dwarves.iter().position(|d| d.id == tid && d.alive) else {
                c.target = None;
                return true;
            };
            let dpos = self.dwarves[di].pos;
            if !c.siege && !near(dpos, c.pos, info.aggro * 2 + 2) {
                c.target = None;
                c.path.clear();
                return true;
            }
            if adjacent(c.pos, dpos) {
                if c.attack_cd == 0 {
                    c.attack_cd = info.attack_delay + c.body.limp();
                    self.creature_attack(c, di);
                }
                return true;
            }
            if c.move_cd == 0 && c.path.first() != Some(&dpos) {
                match path::astar(&self.world, c.pos, dpos, 8000, info.mover) {
                    Some(p) => c.path = p.into_iter().rev().collect(),
                    None => {
                        c.target = None;
                        c.path.clear();
                        return true;
                    }
                }
            }
            self.creature_step(c, info.move_delay);
        } else if let Some(door) = c.bash {
            if self.world.t(door).door != DOOR_LOCKED {
                c.bash = None;
            } else if adjacent(c.pos, door) {
                c.bash_work += 1;
                if c.bash_work > 250 {
                    c.bash = None;
                    c.bash_work = 0;
                    self.break_door(door, info.name);
                }
            } else {
                self.creature_step(c, info.move_delay);
            }
        } else {
            if c.path.is_empty() && c.species != Species::Merchant && self.rng.chance(0.03) {
                let q = c.pos.offset(self.rng.range(-6, 7), self.rng.range(-6, 7), 0);
                if self.world.passable(q, info.mover) {
                    if let Some(p) = path::astar(&self.world, c.pos, q, 600, info.mover) {
                        c.path = p.into_iter().rev().collect();
                    }
                }
            }
            self.creature_step(c, info.move_delay + 1);
        }
        true
    }

    pub(super) fn creature_step(&mut self, c: &mut Creature, delay: u32) {
        if c.move_cd > 0 {
            return;
        }
        let Some(next) = c.path.pop() else { return };
        let mover = c.mover();
        let mut nb = Vec::new();
        self.world.neighbors(c.pos, &mut nb, mover);
        if nb.iter().any(|&(q, _)| q == next) {
            c.pos = next;
            c.move_cd = delay + c.body.limp();
        } else {
            c.path.clear();
        }
    }

    fn creature_think(&mut self, c: &mut Creature) {
        let info = c.species.info();
        let dwarves: Vec<(u32, Pos)> = self.dwarves.iter().filter(|d| d.alive).map(|d| (d.id, d.pos)).collect();
        if c.species == Species::Merchant {
            self.merchant_think(c);
            return;
        }
        if info.aggro == 0 {
            // Skittish animals bolt from anyone nearby.
            if dwarves.iter().any(|(_, p)| near(*p, c.pos, 4)) {
                let score = |q: Pos| dwarves.iter().map(|(_, p)| dist2(*p, q)).min().unwrap_or(0);
                let mut nb = Vec::new();
                self.world.neighbors(c.pos, &mut nb, info.mover);
                if let Some(&(best, _)) = nb.iter().max_by_key(|(q, _)| score(*q)) {
                    c.path = vec![best];
                }
            }
            return;
        }
        if c.siege {
            if c.target.map_or(false, |t| dwarves.iter().any(|(id, _)| *id == t)) {
                return;
            }
            if dwarves.is_empty() {
                c.think_cd = 200;
                return;
            }
            let at: HashMap<Pos, u32> = dwarves.iter().map(|&(id, p)| (p, id)).collect();
            if let Some((p, path)) = path::search(&self.world, c.pos, SEARCH_LIMIT, info.mover, |q| at.contains_key(&q)) {
                c.target = Some(at[&p]);
                c.path = path.into_iter().rev().collect();
                c.bash = None;
                return;
            }
            // Nobody reachable: go batter down a locked door.
            if c.bash.is_none() {
                let w = &self.world;
                let locked = |q: Pos| {
                    DIRS8.iter().find_map(|&(dx, dy)| {
                        let d = q.offset(dx, dy, 0);
                        (w.get(d).map_or(false, |t| t.door == DOOR_LOCKED)).then_some(d)
                    })
                };
                if let Some((p, path)) = path::search(w, c.pos, SEARCH_LIMIT, info.mover, |q| locked(q).is_some()) {
                    c.bash = locked(p);
                    c.path = path.into_iter().rev().collect();
                } else {
                    c.think_cd = 120;
                }
            }
            return;
        }
        if c.target.is_none() {
            let r = info.aggro;
            c.target = dwarves
                .iter()
                .filter(|(_, p)| near(*p, c.pos, r))
                .min_by_key(|(_, p)| dist2(*p, c.pos))
                .map(|(id, _)| *id);
        }
    }

    fn creature_attack(&mut self, c: &Creature, di: usize) {
        let mut d = std::mem::take(&mut self.dwarves[di]);
        let att = self.creature_fighter(c);
        let def = self.dwarf_fighter(&d);
        let msg = self.strike(&att, &def, &mut d.body);
        d.attacker = Some((c.id, self.tick));
        if let Some(m) = msg {
            self.msg(&m, (255, 120, 100));
        }
        if d.asleep {
            self.release_task(&mut d);
        }
        if d.body.dead {
            let cause = format!("has been struck down by {}", att.name);
            self.kill(&mut d, &cause);
        }
        self.dwarves[di] = d;
    }

    fn break_door(&mut self, pos: Pos, by: &str) {
        let bid = self.world.t(pos).building;
        if bid != 0 {
            for p in self.buildings.get(&bid).map(|b| b.tiles()).unwrap_or_default() {
                let t = self.world.t_mut(p);
                t.building = 0;
                t.door = DOOR_NONE;
            }
            self.buildings.remove(&bid);
        }
        self.world.t_mut(pos).door = DOOR_NONE;
        self.msg(&format!("A {} has battered down a door!", by), (255, 90, 90));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(g: &mut Game, ticks: u64) {
        for _ in 0..ticks {
            g.step();
        }
    }

    #[test]
    fn miners_start_with_picks() {
        let g = Game::new(11);
        let picks = g
            .dwarves
            .iter()
            .filter(|d| d.held.and_then(|id| g.items.get(&id)).map_or(false, |it| it.kind == ItemKind::Pick))
            .count();
        assert_eq!(picks, 2);
    }

    #[test]
    fn siege_is_fought_and_resolves() {
        let mut g = Game::new(21);
        g.creatures.retain(|_, c| c.species.dwells_underground());
        // Arm and draft four dwarves.
        let e = g.embark;
        let ids: Vec<u32> = g.dwarves.iter().take(4).map(|d| d.id).collect();
        for _ in 0..4 {
            g.spawn_item(ItemKind::Spear, Mat::Iron, e);
            g.spawn_item(ItemKind::Helm, Mat::Iron, e);
        }
        for id in &ids {
            g.toggle_soldier(*id);
        }
        run(&mut g, 600);
        let armed = g.dwarves.iter().filter(|d| d.soldier && d.held.is_some()).count();
        assert!(armed >= 3, "soldiers didn't arm ({})", armed);
        g.tick = SEASON * 2 + 1;
        // Force a siege.
        let mut tries = 0;
        while g.siege_size == 0 && tries < 50 {
            g.maybe_siege();
            tries += 1;
        }
        let n = g.siege_size;
        assert!(n >= 3);
        let mut hits = 0;
        for _ in 0..(DAY * 8) {
            g.step();
            hits = hits.max(g.log.iter().filter(|l| l.text.contains(" in the ")).count());
        }
        let goblins = g.creatures.values().filter(|c| c.species == Species::Goblin).count();
        assert_eq!(goblins, 0, "siege never ended");
        assert!(hits > 0, "no blows were ever struck");
        eprintln!(
            "siege of {}: {} dwarves alive of {}, goblin corpses {}",
            n,
            g.alive_count(),
            g.dwarves.len(),
            g.items.values().filter(|it| it.kind == ItemKind::Corpse && it.label.as_deref() == Some("goblin")).count()
        );
    }

    #[test]
    fn locked_door_blocks_everyone_and_animals_cannot_open_doors() {
        let mut g = Game::new(5);
        let e = g.embark;
        let w = &g.world;
        let door = (1..15)
            .map(|i| e.offset(i, 0, 0))
            .find(|&p| w.t(p).kind == TileKind::Floor && w.t(p).building == 0)
            .unwrap();
        g.world.t_mut(door).door = world::DOOR_OPEN;
        assert!(g.world.passable(door, Mover::Dwarf));
        assert!(g.world.passable(door, Mover::Goblin));
        assert!(!g.world.passable(door, Mover::Beast));
        g.world.t_mut(door).door = DOOR_LOCKED;
        assert!(!g.world.passable(door, Mover::Dwarf));
        assert!(!g.world.passable(door, Mover::Goblin));
    }

    #[test]
    fn year_with_wildlife_and_caverns_runs() {
        let survived = [33, 34, 35].iter().filter(|&&seed| year(seed)).count();
        assert!(survived >= 2, "only {} of 3 fortresses survived a year", survived);
    }

    fn year(seed: u64) -> bool {
        let mut g = Game::new(seed);
        // A small militia with spears and the means to make more beds.
        for i in 0..3 {
            g.spawn_item(ItemKind::Spear, Mat::Copper, g.embark);
            let id = g.dwarves[2 + i].id;
            g.toggle_soldier(id);
        }
        let e = g.embark;
        g.add_zone(e.offset(-8, 3, -1), e.offset(-1, 9, 1), ZoneKind::Stockpile(StockFilter::All));
        let mut farm = 0;
        for (dx, dy) in [(-10, -10), (2, -10), (-10, 10), (2, 10), (-20, 0), (12, 0)] {
            if farm >= 30 {
                break;
            }
            farm += g.add_zone(e.offset(dx, dy, -2), e.offset(dx + 8, dy + 6, 2), ZoneKind::Farm);
        }
        // Dig straight down into the caverns to wake whatever lives there.
        let s = (1..10).map(|i| e.offset(i, 0, 0)).find(|&p| g.world.t(p).kind == TileKind::Floor).unwrap();
        g.designate(s, Pos::new(s.x, s.y, 5), Desig::StairUD);
        run(&mut g, YEAR);
        let slain = g.log.iter().filter(|l| l.text.contains("has been slain")).count();
        eprintln!(
            "seed {} year (farm {}): {}/{} alive, {} creatures, {} kills, cavern found: {}",
            seed,
            farm,
            g.alive_count(),
            g.dwarves.len(),
            g.creatures.len(),
            slain,
            g.creatures.values().any(|c| c.species.dwells_underground() && g.world.t(c.pos).revealed)
        );
        let deaths: Vec<&str> = g.log.iter().filter(|l| l.color == (255, 60, 60)).map(|l| l.text.as_str()).collect();
        for l in deaths.iter().take(6) {
            eprintln!("  {}", l);
        }
        g.alive_count() > 0
    }
}

#[cfg(test)]
mod profile {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    #[ignore]
    fn profile_year() {
        let mut g = Game::new(33);
        let e = g.embark;
        g.add_zone(e.offset(-8, 3, -1), e.offset(-1, 9, 1), ZoneKind::Stockpile(StockFilter::All));
        let mut farm = 0;
        for (dx, dy) in [(-10, -10), (2, -10), (-10, 10), (2, 10), (-20, 0), (12, 0)] {
            if farm < 30 {
                farm += g.add_zone(e.offset(dx, dy, -2), e.offset(dx + 8, dy + 6, 2), ZoneKind::Farm);
            }
        }
        for i in 0..3 {
            g.spawn_item(ItemKind::Spear, Mat::Copper, e);
            let id = g.dwarves[2 + i].id;
            g.toggle_soldier(id);
        }
        let t = Instant::now();
        for _ in 0..YEAR {
            g.step();
        }
        let _ = Duration::ZERO;
        eprintln!("year in {:?}; alive {}/{}", t.elapsed(), g.alive_count(), g.dwarves.len());
        let count = |pat: &str| g.log.iter().filter(|l| l.text.contains(pat)).count();
        let stress: Vec<i32> = g.dwarves.iter().filter(|d| d.alive).map(|d| d.stress).collect();
        eprintln!(
            "tantrums {} insane {} friends {} grudges {} moods {} artifacts {} mandates ok {} failed {} stress {:?}",
            count("tantrum"), g.dwarves.iter().filter(|d| d.insane).count(), count("become friends"), count("grudge"),
            count("fey mood"), count("has created"), count("fulfilled"), count("went unmet"), stress
        );
    }
}
