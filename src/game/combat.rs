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

pub(super) fn near(a: Pos, b: Pos, r: i32) -> bool {
    (a.z - b.z).abs() <= 1 && (a.x - b.x).abs() <= r && (a.y - b.y).abs() <= r
}

pub(super) fn dist2(a: Pos, b: Pos) -> i32 {
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
            name: d.called(),
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
            name: c.title(),
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

    /// Whether a creature has a way to `p`, remembered for a little while
    /// (asked every few ticks by every dwarf, and a beast sealed off in the
    /// caverns would otherwise be searched for each time).
    pub(super) fn can_reach(&self, c: &Creature, p: Pos, limit: usize) -> bool {
        let key = (c.id, c.pos.x / 3, c.pos.y / 3, c.pos.z, p.x / 3, p.y / 3, p.z);
        let mut cache = self.reach_cache.borrow_mut();
        if let Some(&(at, ok, ver)) = cache.get(&key) {
            if self.tick < at + 120 && ver == self.dig_version {
                return ok;
            }
        }
        if cache.len() > 4000 {
            let tick = self.tick;
            cache.retain(|_, v| tick < v.0 + 120);
        }
        let ok = path::astar(&self.world, c.pos, p, limit, c.mover()).is_some();
        cache.insert(key, (self.tick, ok, self.dig_version));
        ok
    }

    /// Active hostile creatures close enough to worry about: near, and with
    /// a way through (a beast behind solid rock is no threat yet).
    pub(super) fn threats_near(&self, p: Pos, r: i32) -> Vec<(u32, Pos)> {
        self.creatures
            .values()
            .filter(|c| c.hostile() && self.creature_active(c) && near(c.pos, p, r))
            .filter(|c| adjacent(c.pos, p) || self.can_reach(c, p, (r * r * 6) as usize))
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
        let (hp0, siege): (i32, bool) = (body.hp.iter().sum(), c.siege);
        let msg = self.strike(&att, &def, &mut body);
        if siege {
            let hurt = body.hp.iter().sum::<i32>() < hp0;
            self.note_blow(if d.soldier { super::annals::BY_SOLDIER } else { super::annals::BY_CIVILIAN }, hurt);
        }
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
                self.credit_kill(d, &c);
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
        // Desperate thirst or hunger outweighs fear.
        let desperate = d.thirst > THIRSTY + 1500 || d.hunger > HUNGRY + 3000;
        if desperate && d.task == Task::Flee {
            d.task = Task::Idle;
        }
        if !threats.is_empty() && !desperate {
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
        // Run for somewhere well away from them (down the stairs counts for a
        // lot), not just a step back into a corner; look again now and then.
        let far = |p: Pos| threats.iter().map(|(_, t)| (t.x - p.x).pow(2) + (t.y - p.y).pow(2) + 36 * (t.z - p.z).pow(2)).min().unwrap_or(0);
        if d.path.is_empty() || self.tick % 60 < 3 {
            let w = &self.world;
            if let Some((_, path)) = path::search(w, d.pos, 4000, Mover::Dwarf, |q| far(q) >= 16 * 16) {
                Self::set_path(d, path);
            }
        }
        if let Some(&next) = d.path.last() {
            let mut nb = Vec::new();
            self.world.neighbors(d.pos, &mut nb, Mover::Dwarf);
            // (Never towards them.)
            if nb.iter().any(|(q, _)| *q == next) && far(next) + 2 >= far(d.pos) {
                d.path.pop();
                d.pos = next;
                d.move_cd = MOVE_DELAY + d.body.limp();
                return;
            }
            d.path.clear();
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
        self.trouble(if tool == ItemKind::Pick { super::troubles::Lack::Pick } else { super::troubles::Lack::Axe });
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

    pub(super) fn spot_near(&mut self, c: Pos, r: i32) -> Pos {
        for _ in 0..50 {
            let q = c.offset(self.rng.range(-r, r + 1), self.rng.range(-r, r + 1), 0);
            if self.world.walkable(q) {
                return q;
            }
        }
        c
    }

    /// Each cavern layer gets a spider and a band of troglodytes; the
    /// deepest gets an extra spider.
    pub(super) fn populate_caverns(&mut self) {
        let mut bands = self.world.caverns.clone();
        bands.sort();
        for (i, &(lo, hi)) in bands.iter().enumerate() {
            let mut floors = Vec::new();
            for z in lo..=hi {
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
                continue;
            }
            let spiders = if i == 0 { 2 } else { 1 };
            for _ in 0..spiders {
                let p = *self.rng.pick(&floors);
                self.spawn_creature(Species::GiantSpider, p, u64::MAX);
            }
            let c = *self.rng.pick(&floors);
            for _ in 0..3 {
                let p = self.spot_near(c, 2);
                self.spawn_creature(Species::Troglodyte, p, u64::MAX);
            }
        }
    }

    /// Once the fortress has stood a year, a megabeast from a nearby lair may
    /// come for it each season, more likely the richer it is.
    pub(super) fn maybe_megabeast(&mut self) {
        if self.tick < YEAR || self.creatures.values().any(|c| c.figure.is_some()) {
            return;
        }
        let (sx, sy) = self.site;
        let Some(&f) = self.overworld.beasts_near(sx, sy, 30).first() else { return };
        let chance = (0.12 + self.wealth() as f64 / 300_000.0).min(0.5);
        if !self.rng.chance(chance) {
            return;
        }
        self.spawn_megabeast(f);
    }

    pub(super) fn spawn_megabeast(&mut self, f: u32) -> Option<u32> {
        let entry = self.edge_entry()?;
        let (name, kind) = {
            let fig = self.overworld.figures.get(f as usize)?;
            (fig.name.clone(), fig.kind.clone())
        };
        let id = self.spawn_creature(Species::Megabeast, entry, DAY * 10);
        let label = format!("{} the {}", name, kind);
        let c = self.creatures.get_mut(&id).unwrap();
        c.siege = true;
        c.figure = Some(f);
        c.label = Some(label.clone());
        c.breathes_fire = kind == "dragon";
        self.year_log.dragon |= kind == "dragon";
        self.year_log.beast = Some(kind.clone());
        let fort = self.fortress_name.clone();
        if kind == "dragon" {
            self.msg(&format!("{} descends upon {}! The sky burns with dragonfire!", label, fort), (255, 60, 60));
        } else {
            self.msg(&format!("{} has come to {}! A {} of legend is upon you!", label, fort, kind), (255, 60, 60));
        }
        self.legend(format!("{} attacked {}.", label, fort));
        let text = if kind == "dragon" {
            format!("{} descends upon {}! The sky burns with dragonfire.", label, fort)
        } else {
            format!("{}, a {} of legend, has come to {}.", label, kind, fort)
        };
        self.spotlight(if kind == "dragon" { "A dragon!" } else { "A beast of legend!" }, &text, Some(entry), (255, 60, 60));
        Some(id)
    }

    /// A gout of flame from `c` towards `to` and a little past: it scorches
    /// everyone on the way and sets whatever will burn alight.
    pub(super) fn breathe_fire(&mut self, c: &Creature, to: Pos) {
        let (dx, dy) = (to.x - c.pos.x, to.y - c.pos.y);
        let steps = dx.abs().max(dy.abs()).max(1);
        let reach = steps + 2;
        let tick = self.tick;
        self.flames.retain(|&(_, t)| t + 12 > tick);
        let mut swept = Vec::new();
        for i in 1..=reach {
            let p = Pos::new(c.pos.x + (dx * i + dx.signum() * steps / 2) / steps, c.pos.y + (dy * i + dy.signum() * steps / 2) / steps, c.pos.z);
            if self.world.solid(p) {
                break;
            }
            swept.push(p);
            // The flame fans out as it goes.
            if i > 2 {
                for &(ox, oy) in DIRS4.iter() {
                    if self.rng.chance(0.35) {
                        let q = p.offset(ox, oy, 0);
                        if !self.world.solid(q) {
                            swept.push(q);
                        }
                    }
                }
            }
        }
        for &p in &swept {
            self.flames.push((p, tick));
            self.ignite(p);
            for i in 0..self.dwarves.len() {
                if self.dwarves[i].alive && self.dwarves[i].pos == p {
                    let part = self.dwarves[i].body.pick_part(&mut self.rng);
                    let d = &mut self.dwarves[i];
                    d.body.wound(part, 4, false);
                    d.think("was scorched by dragonfire", 20, tick);
                }
            }
            let hit: Vec<u32> = self.creatures.values().filter(|o| o.alive && o.pos == p && !o.breathes_fire).map(|o| o.id).collect();
            for id in hit {
                let part = self.creatures[&id].body.pick_part(&mut self.rng);
                self.creatures.get_mut(&id).unwrap().body.wound(part, 4, false);
            }
        }
        self.msg(&format!("{} breathes fire!", c.title()), (255, 110, 30));
    }

    /// A slain dragon leaves its skull and the hoard it carried.
    fn dragon_hoard(&mut self, c: &Creature) {
        self.milestone("dragon", &format!("A dragon slain: {}", c.title()));
        let skull = self.spawn_item(ItemKind::Craft, Mat::Bone, c.pos);
        if let Some(it) = self.items.get_mut(&skull) {
            it.quality = 5;
            it.label = Some(format!("skull of {}", c.title()));
        }
        let n = self.rng.range(4, 9);
        for _ in 0..n {
            let (kind, mat) = *self.rng.pick(&[
                (ItemKind::Craft, Mat::Gold),
                (ItemKind::Bar, Mat::Gold),
                (ItemKind::Gem, Mat::Amethyst),
                (ItemKind::Craft, Mat::Bronze),
            ]);
            let p = self.spot_near(c.pos, 1);
            let id = self.spawn_item(kind, mat, p);
            if let Some(it) = self.items.get_mut(&id) {
                it.quality = self.rng.range(2, 5) as u8;
            }
        }
        self.msg(&format!("Its hoard spills out where it fell: {} treasures, and the skull of a dragon!", n), (255, 215, 60));
    }

    /// A thief now and then, once there's something worth stealing.
    pub(super) fn maybe_thief(&mut self) {
        if self.tick < SEASON || !self.rng.chance(0.25) || !self.items.values().any(|it| it.free() && it.value() >= 20) {
            return;
        }
        if let Some(entry) = self.edge_entry() {
            self.spawn_thief(entry);
        }
    }

    pub(super) fn spawn_thief(&mut self, p: Pos) -> u32 {
        let id = self.spawn_creature(Species::Skulker, p, DAY * 3);
        self.creatures.get_mut(&id).unwrap().sneaking = true;
        id
    }

    /// Unseen, a thief makes for the most valuable thing lying about, takes
    /// it and runs. Once a dwarf sees them, they're known and they bolt.
    fn skulker_think(&mut self, c: &mut Creature) {
        if c.sneaking && self.dwarves.iter().any(|d| d.alive && near(d.pos, c.pos, 5)) {
            c.sneaking = false;
            c.leaving = true;
            c.path.clear();
            let what = if c.loot.is_some() { " with stolen goods" } else { "" };
            self.msg(&format!("A skulker has been spotted{}!", what), (255, 90, 90));
            return;
        }
        if c.loot.is_some() {
            c.leaving = true;
            return;
        }
        // Standing on the prize: take it.
        let here = self.items.values().filter(|it| it.pos == c.pos && it.free()).max_by_key(|it| it.value()).map(|it| it.id);
        if let Some(id) = here.filter(|id| self.items[id].value() >= 10) {
            let it = self.items.get_mut(&id).unwrap();
            it.carried_by = Some(c.id);
            it.container = None;
            c.loot = Some(id);
            c.leaving = true;
            c.path.clear();
            return;
        }
        if !c.path.is_empty() {
            return;
        }
        let mut best: Vec<(u32, Pos)> = self.items.values().filter(|it| it.free() && it.value() >= 10).map(|it| (it.value(), it.pos)).collect();
        best.sort_by_key(|b| std::cmp::Reverse(b.0));
        let goals: HashSet<Pos> = best.iter().take(8).map(|b| b.1).collect();
        let mover = c.species.info().mover;
        if let Some((_, path)) = path::search(&self.world, c.pos, SEARCH_LIMIT, mover, |q| goals.contains(&q)) {
            c.path = path.into_iter().rev().collect();
        } else {
            c.leaving = true;
        }
    }

    /// Once the caverns are found, now and then something crawls up out of them.
    pub(super) fn maybe_cave_horror(&mut self) {
        if self.tick < SEASON * 2 || self.creatures.values().any(|c| c.species == Species::CaveHorror) || !self.rng.chance(0.3) {
            return;
        }
        let floors = self.cavern_floors();
        if !floors.iter().any(|&p| self.world.t(p).revealed) {
            return;
        }
        let far: Vec<Pos> = floors
            .iter()
            .copied()
            .filter(|p| self.dwarves.iter().filter(|d| d.alive).all(|d| (d.pos.x - p.x).abs() + (d.pos.y - p.y).abs() + 4 * (d.pos.z - p.z).abs() > 20))
            .collect();
        let pool = if far.is_empty() { floors } else { far };
        let p = *self.rng.pick(&pool);
        self.spawn_cave_horror(p);
    }

    /// Open cavern floor (dry, not under the sky).
    pub(super) fn cavern_floors(&self) -> Vec<Pos> {
        let mut v = Vec::new();
        for z in self.world.caverns.iter().flat_map(|&(lo, hi)| lo..=hi) {
            for y in 0..self.world.h {
                for x in 0..self.world.w {
                    let p = Pos::new(x, y, z);
                    let t = self.world.t(p);
                    if t.kind == TileKind::Floor && t.water == 0 && t.magma == 0 && !self.world.outside(p) {
                        v.push(p);
                    }
                }
            }
        }
        v
    }

    pub(super) fn spawn_cave_horror(&mut self, p: Pos) -> u32 {
        let id = self.make_cave_horror(p);
        let (label, desc) = {
            let c = &self.creatures[&id];
            (c.label.clone().unwrap_or_default(), c.desc.clone().unwrap_or_default())
        };
        self.msg(&format!("{} has come up from the caverns! {}", label, desc), (230, 120, 255));
        let fort = self.fortress_name.clone();
        self.legend(format!("{} emerged from the depths beneath {}.", label, fort));
        self.spotlight("From the deep", &format!("{} has come up from the caverns. {}", label, desc), Some(p), (230, 120, 255));
        id
    }

    /// A one-of-a-kind horror with its own name and look, quietly.
    pub(super) fn make_cave_horror(&mut self, p: Pos) -> u32 {
        const SYL: [&str; 14] = ["oz", "ul", "thra", "gim", "vex", "oth", "rhu", "zal", "mor", "quo", "sk", "yth", "ash", "nug"];
        const SIZE: [&str; 5] = ["towering", "hulking", "bloated", "spindly", "enormous"];
        const SHAPE: [&str; 7] = ["blob", "serpent", "spider", "beetle", "lizard", "worm", "bat"];
        const STUFF: [&str; 7] = ["crystal", "bone", "slime", "iron", "ash", "fungus", "shadow"];
        const MARK: [&str; 6] = ["many eyes", "a gaping maw", "barbed tentacles", "glowing veins", "a long lashing tail", "no eyes at all"];
        let n = self.rng.range(2, 4);
        let mut name: String = (0..n).map(|_| *self.rng.pick(&SYL)).collect();
        name = name[..1].to_uppercase() + &name[1..];
        let desc = format!(
            "A {} {} of {} with {}.",
            self.rng.pick(&SIZE),
            self.rng.pick(&SHAPE),
            self.rng.pick(&STUFF),
            self.rng.pick(&MARK)
        );
        let id = self.spawn_creature(Species::CaveHorror, p, u64::MAX);
        let label = format!("{} the cave horror", name);
        let c = self.creatures.get_mut(&id).unwrap();
        c.siege = true;
        c.label = Some(label);
        c.desc = Some(desc);
        id
    }

    /// Whether a neighbour would march on the fortress: goblins always, and
    /// any civilization at war with ours in the world's history.
    pub fn at_war_with(&self, n: &Neighbor) -> bool {
        if !self.overworld.civs.get(n.civ as usize).map_or(false, |c| c.alive) {
            return false;
        }
        n.race == Race::Goblin
            || self.civ.map_or(false, |c| self.overworld.civ(c).enemies.contains(&n.civ) || self.overworld.civ(n.civ).enemies.contains(&c))
    }

    pub(super) fn maybe_siege(&mut self) {
        let pop = self.alive_count() as f64;
        let foes: Vec<Neighbor> = self.neighbors.iter().filter(|n| self.at_war_with(n)).cloned().collect();
        let Some(dist) = foes.iter().map(|n| n.dist).min() else { return };
        let near = (1.0 - dist as f64 / 35.0).max(0.2);
        if self.tick < SEASON * 2 || !(self.siege_warned || self.rng.chance((0.3 + pop * 0.02) * near + self.wealth() as f64 / 200_000.0)) {
            return;
        }
        // A spy's report brings them for certain.
        self.siege_warned = false;
        // The nearer an enemy, the likelier it's them.
        let weights: Vec<i32> = foes.iter().map(|n| (40 - n.dist).max(5)).collect();
        let mut r = self.rng.range(0, weights.iter().sum::<i32>());
        let foe = foes
            .iter()
            .zip(&weights)
            .find(|(_, &w)| {
                r -= w;
                r < 0
            })
            .map_or(foes[0].clone(), |(n, _)| n.clone());
        if self.incoming.is_some() {
            return;
        }
        // Scouts see them coming: they arrive in about a day.
        let n = self.siege_strength();
        let leader = self.siege_leader(&foe).1;
        let arrive = self.tick + DAY / 2 + self.rng.range(0, DAY as i32) as u64;
        let civ = self.overworld.civ(foe.civ).name.clone();
        let plural = Self::army_species(foe.race).info().plural;
        let raid = n <= 3;
        let what = if raid { format!("a raiding party of {} {}", n, plural) } else { format!("an army of {} {}", n, plural) };
        let mut text = format!("Scouts report {} of the {}, led by {}, marching on {}. They will be at the gates within a day!", what, civ, leader, self.fortress_name);
        // They remember how the last one went.
        if self.annals.sieges.last().map_or(false, |s| s.outcome == "routed") {
            text.push_str(" They have not forgotten their last defeat, and come in greater numbers.");
        }
        self.msg(&text, (255, 120, 80));
        self.spotlight(if raid { "Raiders sighted" } else { "An army on the march" }, &text, None, (255, 120, 80));
        self.incoming = Some((arrive, foe, n, leader));
    }

    /// How many come: a raiding party first, then armies that grow with the
    /// fortress (its people, its riches) and with every siege it has broken.
    pub(crate) fn siege_strength(&mut self) -> u32 {
        let past = self.annals.sieges.len() as u32;
        if past == 0 {
            return self.rng.range(2, 4) as u32;
        }
        let broken = self.annals.sieges.iter().filter(|s| s.outcome == "routed").count() as u32;
        let n = 3 + self.alive_count() as u32 / 5 + self.wealth() / 25_000 + broken + self.rng.range(0, 3) as u32;
        n.min(20)
    }

    /// The army that was seen coming reaches the edge of the map.
    pub(super) fn army_arrives(&mut self) {
        let Some((at, _, _, _)) = &self.incoming else { return };
        if self.tick < *at {
            return;
        }
        let (_, foe, n, leader) = self.incoming.take().unwrap();
        self.lay_siege_of(&foe, n, Some(leader));
    }

    fn army_species(race: Race) -> Species {
        match race {
            Race::Goblin => Species::Goblin,
            Race::Human => Species::Human,
            Race::Elf => Species::Elf,
            Race::Dwarf => Species::Raider,
        }
    }

    /// Who leads them: a living figure of their civilization, or a new name.
    fn siege_leader(&mut self, foe: &Neighbor) -> (Option<u32>, String) {
        let kind = format!("{:?}", foe.race).to_lowercase();
        let year = self.start_year + (self.tick / YEAR) as i32;
        let known: Vec<u32> = self
            .overworld
            .figures
            .iter()
            .filter(|f| f.civ == Some(foe.civ) && f.died.is_none() && f.kind == kind && year - f.born >= 20)
            .map(|f| f.id)
            .collect();
        let general = if known.is_empty() { None } else { Some(*self.rng.pick(&known)) };
        let name = match general {
            Some(f) => self.overworld.figures[f as usize].name.clone(),
            None => crate::overworld::person_name(&mut self.rng, foe.race),
        };
        (general, name)
    }

    /// An army of `foe`'s people arrives at the edge of the map, led by a
    /// general out of their history.
    pub(crate) fn lay_siege(&mut self, foe: &Neighbor) {
        let n = (3 + self.alive_count() as u32 / 4).min(if self.siege_size == 0 { 5 } else { 20 });
        self.lay_siege_of(foe, n, None);
    }

    /// `n` of them, led by `leader` (or someone of theirs).
    pub(crate) fn lay_siege_of(&mut self, foe: &Neighbor, n: u32, leader: Option<String>) {
        let n = n.max(1) as usize;
        let Some(entry) = self.edge_entry() else { return };
        let (species, mats, weapons, armor): (Species, &[Mat], &[ItemKind], [f64; 3]) = match foe.race {
            Race::Goblin => (
                Species::Goblin,
                &[Mat::Copper, Mat::Copper, Mat::Bronze, Mat::Iron],
                &[ItemKind::Spear, ItemKind::Sword, ItemKind::Axe, ItemKind::Hammer],
                [0.5, 0.3, 0.25],
            ),
            Race::Human => (Species::Human, &[Mat::Bronze, Mat::Iron, Mat::Iron], &[ItemKind::Sword, ItemKind::Spear, ItemKind::Hammer], [0.8, 0.6, 0.5]),
            Race::Elf => (Species::Elf, &[Mat::Wood], &[ItemKind::Spear, ItemKind::Sword], [0.3, 0.0, 0.4]),
            Race::Dwarf => (Species::Raider, &[Mat::Bronze, Mat::Iron, Mat::Iron], &[ItemKind::Axe, ItemKind::Hammer], [0.9, 0.7, 0.4]),
        };
        let stay = DAY * 5;
        // A living figure of theirs leads, or a new one rises to it (the one
        // the scouts named, if they did).
        let (mut general, mut general_name) = self.siege_leader(foe);
        if let Some(named) = leader {
            general = self.overworld.figures.iter().find(|f| f.name == named && f.died.is_none()).map(|f| f.id);
            general_name = named;
        }
        let civ_name = self.overworld.civ(foe.civ).name.clone();
        for i in 0..n {
            let p = self.spot_near(entry, 3);
            let id = self.spawn_creature(species, p, stay);
            let leads = i == 0;
            let mat = *self.rng.pick(mats);
            let wk = *self.rng.pick(weapons);
            let mut gear = vec![wk];
            if leads || self.rng.chance(armor[0]) {
                gear.push(ItemKind::Helm);
            }
            if (leads && armor[1] > 0.0) || self.rng.chance(armor[1]) {
                gear.push(ItemKind::Breastplate);
            }
            if leads || self.rng.chance(armor[2]) {
                gear.push(ItemKind::Shield);
            }
            if leads {
                let c = self.creatures.get_mut(&id).unwrap();
                c.label = Some(general_name.clone());
                c.desc = Some(format!("general of the {}", civ_name));
                c.general = Some(general.unwrap_or(u32::MAX));
                c.body = crate::body::Body::new(species.info().scale * 1.3);
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
        self.note_siege(species.info().plural, n as u32, &general_name);
        self.siege_race = Some(foe.race);
        self.year_log.siege = Some(civ_name.clone());
        let plural = species.info().plural;
        let text = if foe.race == Race::Goblin {
            format!("A vile force of darkness has arrived! {} {}, led by {}.", n, plural, general_name)
        } else {
            format!("The {} march to war! {} {}, led by {}, have come to settle an old score.", civ_name, n, plural, general_name)
        };
        self.msg(&text, (255, 70, 70));
        let name = self.fortress_name.clone();
        self.legend(format!("{} led the {} in a siege of {}.", general_name, civ_name, name));
        self.spotlight("Siege!", &text, Some(entry), (255, 70, 70));
    }

    /// The attackers lose heart and run for the edge of the map.
    fn rout(&mut self) {
        self.siege_routed = true;
        self.milestone("siege", "The first siege broken");
        for o in self.creatures.values_mut().filter(|o| o.siege && !o.legendary()) {
            o.leaving = true;
            o.target = None;
            o.path.clear();
        }
        let who = match self.siege_race {
            Some(Race::Human) => "human soldiers",
            Some(Race::Elf) => "elves",
            Some(Race::Dwarf) => "raiders",
            _ => "goblins",
        };
        self.msg(&format!("The {} are fleeing!", who), (120, 255, 120));
        let fort = self.fortress_name.clone();
        self.spotlight("The siege is broken", &format!("The {} are fleeing from {}!", who, fort), None, (120, 255, 120));
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
        self.note_foe_slain(&c);
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
        for id in c.weapon.take().into_iter().chain(c.armor.drain(..)).chain(c.loot.take()) {
            self.drop_item(id, c.pos);
        }
        let info = c.species.info();
        if c.hostile() || c.siege {
            self.year_log.kills += 1;
        }
        if c.species == Species::Merchant {
            self.merchant_died(&c);
        }
        let kind = if c.species.person() { ItemKind::Corpse } else { ItemKind::Carcass };
        let id = self.spawn_item(kind, Mat::Flesh, c.pos);
        if let Some(it) = self.items.get_mut(&id) {
            it.label = Some(info.name.to_string());
        }
        if c.species == Species::Undead {
            // Dust, or one of our own to bury.
            self.items.remove(&id);
        }
        if c.breathes_fire {
            self.dragon_hoard(&c);
        }
        self.pet_died(&c);
        if let Some(f) = c.figure {
            let label = c.title();
            let kind = self.overworld.figures.get(f as usize).map_or("beast".to_string(), |x| x.kind.clone());
            if let Some(it) = self.items.get_mut(&id) {
                it.label = Some(kind);
            }
            let year = self.start_year + (self.tick / YEAR) as i32;
            let fort = self.fortress_name.clone();
            self.overworld.kill_figure(f, year, format!("slain at {}", fort));
            self.legend(format!("{} was slain at {}.", label, fort));
            self.msg(&format!("{} has been slain! The fortress rejoices.", label), (255, 215, 60));
            let tick = self.tick;
            for d in self.dwarves.iter_mut().filter(|d| d.alive) {
                d.think(&format!("rejoiced at the death of {}", label), -15, tick);
            }
        } else if c.species == Species::CaveHorror {
            let label = c.title();
            let fort = self.fortress_name.clone();
            self.legend(format!("{} was slain at {}.", label, fort));
            self.msg(&format!("{} has been slain!", label), (255, 215, 60));
            let tick = self.tick;
            for d in self.dwarves.iter_mut().filter(|d| d.alive) {
                d.think(&format!("rejoiced at the death of {}", label), -10, tick);
            }
        } else if c.species == Species::Undead {
            self.undead_destroyed(&c);
        } else if let Some(f) = c.general {
            // The general falls: it goes down in history, and the army breaks.
            let label = c.title();
            let fort = self.fortress_name.clone();
            if f != u32::MAX {
                let year = self.start_year + (self.tick / YEAR) as i32;
                self.overworld.kill_figure(f, year, format!("slain besieging {}", fort));
            }
            self.legend(format!("{}, {}, fell at {}.", label, c.desc.clone().unwrap_or_default(), fort));
            self.msg(&format!("{}, who led the attack, has fallen!", label), (255, 215, 60));
            self.spotlight("Their leader falls", &format!("{}, who led the attack, has fallen!", label), Some(c.pos), (255, 215, 60));
            let tick = self.tick;
            for d in self.dwarves.iter_mut().filter(|d| d.alive) {
                d.think(&format!("cheered the fall of {}", label), -8, tick);
            }
            if c.siege && !self.siege_routed {
                self.rout();
            }
        } else if c.hostile() {
            self.msg(&format!("The {} has been slain.", info.name), (160, 230, 160));
        }
        if c.siege && !c.legendary() && !self.siege_routed {
            let left = self.creatures.values().filter(|o| o.siege && !o.legendary() && o.alive).count() as u32;
            if left * 2 < self.siege_size {
                self.rout();
            }
        }
    }

    /// Returns false when the creature should be removed (died or left the map).
    fn creature_tick(&mut self, c: &mut Creature) -> bool {
        if c.visitor == super::visitors::JOINED {
            return false;
        }
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
        // Animals lose their nerve when hurt; goblins only when close to death;
        // megabeasts never.
        let breaking = if c.legendary() {
            false
        } else if c.siege {
            c.body.blood * 10 < c.body.max_blood * 4
        } else {
            c.body.badly_hurt()
        };
        if c.hostile() && !c.leaving && breaking && !c.species.dwells_underground() {
            c.leaving = true;
            c.target = None;
            c.path.clear();
            self.msg(&format!("The wounded {} flees!", info.name), (160, 230, 160));
        }
        c.attack_cd = c.attack_cd.saturating_sub(1);
        c.breath_cd = c.breath_cd.saturating_sub(1);
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
                if let Some(it) = c.loot.and_then(|id| self.items.remove(&id)) {
                    self.msg(&format!("A thief has made off with {}!", it.name()), (255, 120, 90));
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
            // A dragon breathes fire down a clear line at whoever it hunts.
            if c.breathes_fire && c.breath_cd == 0 && c.attack_cd == 0 && !adjacent(c.pos, dpos) && near(dpos, c.pos, 7) && self.clear_shot(c.pos, dpos) {
                c.breath_cd = 90;
                c.attack_cd = info.attack_delay;
                self.breathe_fire(c, dpos);
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
                        // No way there: give it a rest before trying again.
                        c.target = None;
                        c.path.clear();
                        c.think_cd = 90;
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
            if c.path.is_empty() && !matches!(c.species, Species::Merchant | Species::Visitor) && self.rng.chance(if c.hostile() { 0.05 } else { 0.03 }) {
                // Prowl about (up and down stairs too), rather than stand where it came in.
                self.creature_wander(c, if c.hostile() { 12 } else { 7 });
            }
            self.creature_step(c, info.move_delay + 1);
        }
        true
    }

    /// Set off for somewhere a few steps away that it can walk to.
    fn creature_wander(&mut self, c: &mut Creature, r: i32) {
        let start = c.pos;
        let salt = self.rng.range(0, 1 << 20);
        // Cave things keep to the caves (they hunt into the halls, but don't
        // stroll up them), and the rest keep to the open air if they're in it.
        let cave = c.species.dwells_underground();
        let open_air = self.world.outside(start);
        let w = &self.world;
        let found = path::search(w, start, 900, c.mover(), |q| {
            let d = (q.x - start.x).abs() + (q.y - start.y).abs() + (q.z - start.z).abs() * 3;
            let ground = if cave { !w.t(q).dug && w.t(q).built_from.is_none() } else { !open_air || w.outside(q) };
            ground && d >= r / 2 && d <= r && (q.x * 31 + q.y * 17 + q.z * 7 + salt).rem_euclid(5) == 0
        });
        if let Some((_, path)) = found {
            c.path = path.into_iter().rev().collect();
        }
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
        if c.species == Species::Skulker {
            self.skulker_think(c);
            return;
        }
        if c.species == Species::Visitor {
            if !c.leaving {
                self.visitor_think(c);
            }
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
            // Standing on something worth taking: take it and go.
            if c.loot.is_none() && !c.legendary() {
                let here = self.items.values().find(|it| it.pos == c.pos && it.free() && it.container.is_none() && it.value() >= 10 && it.kind != ItemKind::Corpse).map(|it| it.id);
                if let Some(id) = here {
                    let name = self.items[&id].name();
                    if let Some(it) = self.items.get_mut(&id) {
                        it.carried_by = Some(c.id);
                    }
                    c.loot = Some(id);
                    c.leaving = true;
                    c.target = None;
                    c.path.clear();
                    self.msg(&format!("The {} seizes {} and makes for the edge of the map!", c.species.info().name, name), (255, 140, 90));
                    return;
                }
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
                    // Shut out: loot what's lying about outside, or prowl
                    // round the walls until a way in opens.
                    c.think_cd = 120;
                    if c.path.is_empty() && c.loot.is_none() && !c.legendary() {
                        let w = &self.world;
                        let goods: HashMap<Pos, u32> = self
                            .items
                            .values()
                            .filter(|it| it.free() && it.container.is_none() && it.value() >= 10 && it.kind != ItemKind::Corpse && w.outside(it.pos))
                            .map(|it| (it.pos, it.id))
                            .collect();
                        if let Some((_, path)) = path::search(w, c.pos, 3000, info.mover, |q| goods.contains_key(&q)) {
                            c.path = path.into_iter().rev().collect();
                        }
                    }
                    if c.path.is_empty() {
                        self.creature_wander(c, 10);
                    }
                }
            }
            return;
        }
        self.predator_think(c);
    }

    fn creature_attack(&mut self, c: &Creature, di: usize) {
        let mut d = std::mem::take(&mut self.dwarves[di]);
        let att = self.creature_fighter(c);
        let def = self.dwarf_fighter(&d);
        let hp0: i32 = d.body.hp.iter().sum();
        let msg = self.strike(&att, &def, &mut d.body);
        d.attacker = Some((c.id, self.tick));
        self.note_wound(d.id, c);
        if c.siege {
            let hurt = d.body.hp.iter().sum::<i32>() < hp0;
            self.note_blow(if d.soldier { super::annals::ON_SOLDIER } else { super::annals::ON_CIVILIAN }, hurt);
        }
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

    /// The arena: one dwarf against one goblin, many times over, with the
    /// blows tallied. Prints a table (cargo test arena -- --ignored --nocapture).
    #[test]
    #[ignore]
    fn arena() {
        struct Side {
            fighter: Fighter,
            body: Body,
            cd: u32,
            delay: u32,
        }
        let mut g = Game::new(5);
        let e = g.embark;
        let mut gear = |g: &mut Game, kinds: &[(ItemKind, Mat)]| -> (Option<u32>, Vec<u32>) {
            let mut held = None;
            let mut armor = Vec::new();
            for &(k, m) in kinds {
                let id = g.spawn_item(k, m, e);
                if k.weapon_damage().is_some() {
                    held = Some(id);
                } else {
                    armor.push(id);
                }
            }
            (held, armor)
        };
        // Goblins as sieges arm them: a copper-to-iron weapon, a helm half the time,
        // a breastplate a third, a shield a quarter. The typical one:
        let gob_kits: Vec<Vec<(ItemKind, Mat)>> = vec![
            vec![(ItemKind::Spear, Mat::Copper)],
            vec![(ItemKind::Sword, Mat::Bronze), (ItemKind::Helm, Mat::Bronze)],
            vec![(ItemKind::Axe, Mat::Iron), (ItemKind::Helm, Mat::Iron), (ItemKind::Breastplate, Mat::Iron), (ItemKind::Shield, Mat::Iron)],
        ];
        let dwarf_kits: Vec<(&str, u32, Vec<(ItemKind, Mat)>)> = vec![
            ("civilian, bare-handed", 0, vec![]),
            ("miner with a pick", 0, vec![(ItemKind::Pick, Mat::Iron)]),
            ("recruit, copper axe", 0, vec![(ItemKind::Axe, Mat::Copper)]),
            ("soldier, iron axe+helm+mail+shield", 0, vec![(ItemKind::Axe, Mat::Iron), (ItemKind::Helm, Mat::Iron), (ItemKind::Breastplate, Mat::Iron), (ItemKind::Shield, Mat::Iron)]),
            ("veteran (skill 4), same kit", 1600, vec![(ItemKind::Axe, Mat::Iron), (ItemKind::Helm, Mat::Iron), (ItemKind::Breastplate, Mat::Iron), (ItemKind::Shield, Mat::Iron)]),
        ];
        let gid = g.spawn_creature(Species::Goblin, e, u64::MAX);
        eprintln!("{:<36} {:<22} {:>5} {:>6} {:>7} {:>7} {:>8} {:>8} {:>9}", "dwarf", "goblin", "wins", "ticks", "d hit%", "g hit%", "d dmg", "g dmg", "d bled%");
        for (dname, xp, dkit) in &dwarf_kits {
            for (gi, gkit) in gob_kits.iter().enumerate() {
                let (dh, da) = gear(&mut g, dkit);
                let (gh, ga) = gear(&mut g, gkit);
                let mut d = g.dwarves[0].clone();
                d.held = dh;
                d.armor = da;
                d.fight_xp = *xp;
                d.body = Body::new(1.0);
                let df = g.dwarf_fighter(&d);
                let mut c = g.creatures[&gid].clone();
                c.weapon = gh;
                c.armor = ga;
                let gf = g.creature_fighter(&c);
                let (mut wins, mut ticks, mut bled) = (0, 0u64, 0);
                let (mut dsw, mut dhit, mut ddmg, mut gsw, mut ghit, mut gdmg) = (0, 0, 0i64, 0, 0, 0i64);
                let n = 400;
                for _ in 0..n {
                    let mut a = Side { fighter: Fighter { ..df_clone(&df) }, body: Body::new(1.0), cd: 0, delay: 9 };
                    let mut b = Side { fighter: df_clone(&gf), body: Body::new(Species::Goblin.info().scale), cd: 4, delay: 9 };
                    let mut t = 0;
                    loop {
                        t += 1;
                        for (att, def, sw, hit, dmg) in [(&mut a as *mut Side, &mut b as *mut Side, &mut dsw, &mut dhit, &mut ddmg), (&mut b as *mut Side, &mut a as *mut Side, &mut gsw, &mut ghit, &mut gdmg)] {
                            // (Two sides of one fight, each borrowed in turn.)
                            let (att, def) = unsafe { (&mut *att, &mut *def) };
                            if att.body.dead || att.body.unconscious() {
                                continue;
                            }
                            if att.cd > 0 {
                                att.cd -= 1;
                                continue;
                            }
                            att.cd = att.delay + att.body.limp();
                            *sw += 1;
                            let before: i32 = def.body.hp.iter().sum();
                            let bleed_before = def.body.bleeding;
                            let fa = df_clone(&att.fighter);
                            let fd = df_clone(&def.fighter);
                            g.strike(&fa, &fd, &mut def.body);
                            let after: i32 = def.body.hp.iter().sum();
                            if after < before || def.body.bleeding > bleed_before {
                                *hit += 1;
                                *dmg += (before - after) as i64;
                            }
                        }
                        if t % 10 == 0 {
                            a.body.tick();
                            b.body.tick();
                        }
                        let a_down = a.body.dead || a.body.unconscious();
                        let b_down = b.body.dead || b.body.unconscious();
                        if a_down || b_down || t > 3000 {
                            if b_down && !a_down {
                                wins += 1;
                            }
                            if a_down && a.body.blood <= a.body.max_blood / 3 && a.body.hp[crate::body::Part::Head.idx()] > 0 {
                                bled += 1;
                            }
                            ticks += t;
                            break;
                        }
                    }
                }
                let gname = ["copper spear", "bronze sword+helm", "iron axe, full kit"][gi];
                eprintln!(
                    "{:<36} {:<22} {:>4}% {:>6} {:>6}% {:>6}% {:>8.1} {:>8.1} {:>8}%",
                    dname,
                    gname,
                    wins * 100 / n,
                    ticks / n as u64,
                    dhit * 100 / dsw.max(1),
                    ghit * 100 / gsw.max(1),
                    ddmg as f64 / dhit.max(1) as f64,
                    gdmg as f64 / ghit.max(1) as f64,
                    bled * 100 / (n - wins).max(1)
                );
            }
        }

        fn df_clone(f: &Fighter) -> Fighter {
            Fighter { name: f.name.clone(), skill: f.skill, dmg: f.dmg, verb: f.verb, edged: f.edged, helm: f.helm, mail: f.mail, shield: f.shield }
        }
    }

    /// Bring a siege now: wait out the scouts' warning.
    fn force_siege(g: &mut Game) {
        for _ in 0..100 {
            g.maybe_siege();
            if let Some((at, ..)) = g.incoming.clone() {
                g.tick = at;
                g.army_arrives();
                return;
            }
        }
        panic!("no siege came");
    }

    #[test]
    fn sieges_are_seen_coming_and_build_up() {
        let mut g = Game::new(21);
        g.creatures.retain(|_, c| c.tame);
        g.tick = SEASON * 2 + 1;
        let mut tries = 0;
        while g.incoming.is_none() && tries < 100 {
            g.maybe_siege();
            tries += 1;
        }
        let (at, _, n, leader) = g.incoming.clone().expect("the scouts should see an army coming");
        assert!(at > g.tick + DAY / 3, "a warning comes ahead of them");
        assert!((2..=3).contains(&n), "the first is a raiding party, not {}", n);
        assert!(g.log.iter().any(|l| l.text.contains("Scouts report") && l.text.contains(&leader)));
        assert!(g.creatures.values().all(|c| !c.siege), "nobody's here yet");
        g.tick = at;
        g.army_arrives();
        assert_eq!(g.annals.sieges.len(), 1);
        assert_eq!(g.annals.sieges[0].leader, leader, "the one the scouts named leads them");
        assert!(g.creatures.values().filter(|c| c.siege).count() as u32 == n);
        // Broken, they come back in greater numbers, and say so.
        g.annals.sieges[0].ended = Some(g.tick);
        g.annals.sieges[0].outcome = "routed".into();
        g.creatures.retain(|_, c| !c.siege);
        g.incoming = None;
        let mut tries = 0;
        while g.incoming.is_none() && tries < 100 {
            g.maybe_siege();
            tries += 1;
        }
        let (_, _, n2, _) = g.incoming.clone().unwrap();
        assert!(n2 > n, "the second should be bigger ({} after {})", n2, n);
        assert!(g.log.iter().any(|l| l.text.contains("not forgotten their last defeat")));
    }

    #[test]
    fn besiegers_make_off_with_loot() {
        let mut g = Game::new(21);
        g.creatures.retain(|_, c| c.tame);
        let a = g.open_patch(3, 3, 6).offset(1, 1, 0);
        let gob = g.spawn_creature(Species::Goblin, a, u64::MAX);
        g.creatures.get_mut(&gob).unwrap().siege = true;
        // Nobody to fight: every dwarf is off the map, as if shut in.
        let gold = g.spawn_item(ItemKind::Craft, Mat::Gold, a);
        g.items.get_mut(&gold).unwrap().quality = 3;
        let mut c = g.creatures[&gob].clone();
        let saved: Vec<Dwarf> = std::mem::take(&mut g.dwarves);
        g.creature_think(&mut c);
        g.dwarves = saved;
        assert_eq!(c.loot, Some(gold), "it should take the gold craft");
        assert!(c.leaving);
        assert!(g.log.iter().any(|l| l.text.contains("makes for the edge of the map")));
    }

    #[test]
    fn a_fleeing_dwarf_gets_well_clear() {
        let mut g = Game::new(11);
        g.creatures.retain(|_, c| c.tame);
        let a = g.open_patch(3, 3, 3).offset(1, 1, 0);
        let gob = g.spawn_creature(Species::Goblin, a.offset(1, 0, 0), u64::MAX);
        g.dwarves[0].pos = a;
        g.dwarves[0].task = Task::Flee;
        let near = |g: &Game| {
            let (d, c) = (g.dwarves[0].pos, g.creatures[&gob].pos);
            (d.x - c.x).abs().max((d.y - c.y).abs()) + 3 * (d.z - c.z).abs()
        };
        for _ in 0..300 {
            // The goblin stands where it is; the dwarf runs.
            g.creatures.get_mut(&gob).unwrap().pos = a.offset(1, 0, 0);
            g.creatures.get_mut(&gob).unwrap().target = None;
            let mut d = std::mem::take(&mut g.dwarves[0]);
            if d.task == Task::Flee {
                g.do_flee(&mut d);
            }
            g.dwarves[0] = d;
            g.tick += 1;
        }
        assert!(near(&g) >= 10, "only {} tiles away", near(&g));
    }

    #[test]
    fn each_land_has_its_own_wildlife() {
        let mut g = Game::new(11);
        let kinds = |g: &mut Game| {
            let mut seen = std::collections::BTreeSet::new();
            for _ in 0..400 {
                g.creatures.retain(|_, c| c.tame);
                g.wildlife();
                for c in g.creatures.values().filter(|c| !c.tame) {
                    seen.insert(c.species.info().name);
                }
            }
            seen
        };
        g.tick = YEAR * 2;
        g.savagery = 0.6;
        g.biome = Biome::Desert;
        g.evil = false;
        let desert = kinds(&mut g);
        assert!(desert.contains("camel") && desert.contains("giant scorpion"), "{:?}", desert);
        assert!(!desert.contains("deer") && !desert.contains("wolf"), "{:?}", desert);
        g.biome = Biome::Taiga;
        let taiga = kinds(&mut g);
        assert!(taiga.contains("elk") && taiga.contains("wolf"), "{:?}", taiga);
        g.biome = Biome::Desert;
        g.evil = true;
        let evil = kinds(&mut g);
        assert!(evil.contains("walking husk"), "{:?}", evil);
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
        force_siege(&mut g);
        let n = g.siege_size;
        assert!(n >= 2);
        let mut hits = 0;
        for _ in 0..(DAY * 8) {
            g.step();
            hits = hits.max(g.log.iter().filter(|l| l.text.contains(" in the ")).count());
        }
        let goblins = g.creatures.values().filter(|c| c.siege).count();
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
    fn enemies_from_history_march_on_the_fort_and_break_when_their_general_falls() {
        let mut g = Game::new(5);
        g.creatures.retain(|_, c| c.tame);
        let ours = g.civ.expect("the fort has a civilization");
        let foe = (0..g.overworld.civs.len() as u32).find(|&c| c != ours && g.overworld.civ(c).alive).expect("another civ");
        g.overworld.civs[foe as usize].race = Race::Human;
        g.overworld.civs[ours as usize].enemies.retain(|&c| c != foe);
        g.overworld.civs[foe as usize].enemies.retain(|&c| c != ours);
        let n = Neighbor { civ: foe, race: Race::Human, dist: 3 };
        assert!(!g.at_war_with(&n), "at peace");
        g.overworld.civs[ours as usize].enemies.push(foe);
        assert!(g.at_war_with(&n));
        g.neighbors = vec![n.clone()];
        g.tick = SEASON * 2 + 1;
        force_siege(&mut g);
        assert!(g.siege_size >= 2, "no siege came");
        assert!(g.creatures.values().all(|c| !c.siege || c.species == Species::Human));
        let general = g.creatures.values().find(|c| c.general.is_some()).expect("a general leads them").id;
        assert!(g.creatures[&general].label.is_some());
        assert!(g.log.iter().any(|l| l.text.contains("march to war")), "no announcement");
        // Cut the general down: the army flees and history remembers.
        let c = g.creatures.remove(&general).unwrap();
        g.creature_died(c);
        assert!(g.siege_routed, "the army should break");
        assert!(g.creatures.values().filter(|c| c.siege).all(|c| c.leaving));
        assert!(g.overworld.events.iter().any(|e| e.text.contains("fell at")), "not in the legends");
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

    #[test]
    fn a_megabeast_from_the_legends_attacks_and_its_death_is_recorded() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let f = g.overworld.figures.iter().find(|f| f.lair.is_some() && f.alive()).expect("no megabeasts in this world").id;
        let id = g.spawn_megabeast(f).expect("no way in");
        let label = g.creatures[&id].label.clone().unwrap();
        assert!(label.contains(&g.overworld.figures[f as usize].name));
        let mut hunted = false;
        for _ in 0..600 {
            g.step();
            match g.creatures.get(&id) {
                Some(c) => hunted |= c.target.is_some(),
                None => break,
            }
        }
        assert!(hunted, "the beast never went after anyone");
        // Slay it outright and check the world remembers.
        if let Some(mut c) = g.creatures.remove(&id) {
            c.body.dead = true;
            g.creature_died(c);
        }
        assert!(!g.overworld.figures[f as usize].alive(), "the figure should be dead in the legends");
        assert!(g.overworld.figures[f as usize].cause.as_deref().unwrap_or("").starts_with("slain at"));
        let kind = g.overworld.figures[f as usize].kind.clone();
        assert!(g.items.values().any(|it| it.kind == ItemKind::Carcass && it.label.as_deref() == Some(kind.as_str())));
        assert!(g.dwarves.iter().filter(|d| d.alive).all(|d| d.thoughts.iter().any(|t| t.text.starts_with("rejoiced"))));
        // A dead beast doesn't come again.
        assert!(!g.overworld.beasts_near(g.site.0, g.site.1, 1000).contains(&f));
    }

    #[test]
    fn a_dragon_breathes_fire_and_leaves_a_hoard() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        g.weather = super::super::Weather::Clear;
        let f = g.overworld.figures.iter().find(|f| f.lair.is_some() && f.alive()).expect("no megabeasts").id;
        g.overworld.figures[f as usize].kind = "dragon".into();
        let id = g.spawn_megabeast(f).expect("no way in");
        assert!(g.creatures[&id].breathes_fire);
        assert!(g.log.iter().any(|l| l.text.contains("dragonfire")));
        // Let it hunt: sooner or later it breathes on someone.
        for _ in 0..3000 {
            g.step();
            if g.log.iter().any(|l| l.text.ends_with("breathes fire!")) || !g.creatures.contains_key(&id) {
                break;
            }
        }
        assert!(g.log.iter().any(|l| l.text.ends_with("breathes fire!")), "the dragon never breathed fire");
        // A dwarf five tiles off across open grass, with shrubs between.
        let e = g.embark;
        let z = g.world.surface_z(e.x, e.y + 8);
        let row: Vec<Pos> = (0..8).map(|i| Pos::new(e.x + i, e.y + 8, z)).collect();
        for &p in &row {
            for (q, kind) in [(p, TileKind::Shrub), (p.down(), TileKind::Wall), (p.up(), TileKind::Open)] {
                let t = g.world.t_mut(q);
                t.kind = kind;
                t.water = 0;
                t.building = 0;
            }
        }
        for p in [row[0], row[6]] {
            g.world.t_mut(p).kind = TileKind::Floor;
        }
        let did = g.dwarves[0].id;
        g.dwarves[0].pos = row[6];
        g.dwarves[0].path.clear();
        let mut c = g.creatures.remove(&id).unwrap();
        c.pos = row[0];
        g.breathe_fire(&c, row[6]);
        assert!(!g.flames.is_empty());
        assert!(row[1..6].iter().any(|&p| g.world.t(p).fire > 0), "the shrubs should catch");
        let d = g.dwarves.iter().find(|d| d.id == did).unwrap();
        assert!(d.thoughts.iter().any(|t| t.text == "was scorched by dragonfire"));
        // Slay it: the skull and the hoard.
        c.body.dead = true;
        g.creature_died(c);
        let skull = g.items.values().find(|it| it.label.as_deref().map_or(false, |l| l.starts_with("skull of"))).expect("no skull");
        assert!(skull.value() >= 100, "a dragon skull is a treasure ({})", skull.value());
        assert!(g.items.values().filter(|it| it.mat == Mat::Gold).count() + g.items.values().filter(|it| it.kind == ItemKind::Gem).count() >= 1);
    }

    #[test]
    fn cave_horrors_come_only_from_discovered_caverns() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        g.tick = 3 * SEASON;
        let floors = g.cavern_floors();
        assert!(!floors.is_empty());
        for p in &floors {
            g.world.t_mut(*p).revealed = false;
        }
        for _ in 0..50 {
            g.maybe_cave_horror();
        }
        assert!(!g.creatures.values().any(|c| c.species == Species::CaveHorror), "nobody has found the caverns yet");
        g.world.t_mut(floors[0]).revealed = true;
        for _ in 0..50 {
            g.maybe_cave_horror();
        }
        let c = g.creatures.values().find(|c| c.species == Species::CaveHorror).expect("no horror after the caverns were found");
        assert!(c.label.as_deref().unwrap().ends_with("the cave horror"));
        assert!(c.desc.as_deref().unwrap().starts_with("A "));
        assert!(c.legendary() && c.hostile());
        assert_eq!(g.creatures.values().filter(|c| c.species == Species::CaveHorror).count(), 1, "one at a time");
        let id = c.id;
        let label = c.label.clone().unwrap();
        let mut c = g.creatures.remove(&id).unwrap();
        c.body.dead = true;
        g.creature_died(c);
        assert!(g.overworld.events.iter().any(|e| e.text.contains(&label) && e.text.contains("slain")));
    }

    #[test]
    fn an_unseen_thief_makes_off_with_the_best_thing_lying_about() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let entry = g.edge_entry().unwrap();
        let spot = g.spot_near(entry, 4);
        let gold = g.spawn_item(ItemKind::Craft, Mat::Gold, spot);
        let cheap = g.spawn_item(ItemKind::Log, Mat::Wood, spot);
        let id = g.spawn_thief(entry);
        assert!(!g.creatures[&id].hostile(), "an unseen thief isn't known to be hostile");
        for _ in 0..DAY {
            g.step();
            if !g.creatures.contains_key(&id) {
                break;
            }
        }
        assert!(!g.creatures.contains_key(&id), "the thief never left");
        assert!(!g.items.contains_key(&gold), "the gold craft should be gone");
        assert!(g.items.contains_key(&cheap), "the log wasn't worth stealing");
        assert!(g.log.iter().any(|l| l.text.starts_with("A thief has made off with")));
    }

    #[test]
    fn a_thief_spotted_by_a_dwarf_is_known_and_runs() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let p = g.dwarves[0].pos;
        let at = g.spot_near(p, 2);
        let id = g.spawn_thief(at);
        let mut c = g.creatures.remove(&id).unwrap();
        g.skulker_think(&mut c);
        assert!(!c.sneaking && c.leaving && c.hostile());
        assert!(g.log.iter().any(|l| l.text.starts_with("A skulker has been spotted")));
    }
}
