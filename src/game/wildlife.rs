//! Wild things: what comes out of the land, and how a predator picks its
//! prey.

use super::combat::{dist2, near};
use super::*;
use crate::creature::{Creature, Species};

impl Game {
    /// What lives in this land: something to graze, something that hunts
    /// in packs, and something rare and dangerous.
    pub fn fauna(&self) -> (Species, Species, Species) {
        use crate::worldgen::regions::{role, Role};
        let base = self.biome_fauna();
        let (sx, sy) = self.site;
        if sx < 0 || sy < 0 || sx >= self.overworld.w || sy >= self.overworld.h || self.overworld.cells.len() <= 1 {
            return base;
        }
        // The region's own animals where it has them, the land's usual
        // ones where it doesn't.
        let Some(r) = self.overworld.region_at(sx, sy) else { return base };
        let pick = |want: Role| r.fauna.iter().copied().find(|&s| role(s) == Some(want));
        (pick(Role::Grazer).unwrap_or(base.0), pick(Role::Hunter).unwrap_or(base.1), pick(Role::Rare).unwrap_or(base.2))
    }

    /// The usual animals of the site's biome.
    fn biome_fauna(&self) -> (Species, Species, Species) {
        match self.biome {
            Biome::Taiga => (Species::Elk, Species::Wolf, Species::Bear),
            Biome::Tundra | Biome::Glacier => (Species::Elk, Species::Wolf, Species::PolarBear),
            Biome::Mountain => (Species::Ibex, Species::Wolf, Species::Bear),
            Biome::Desert => (Species::Camel, Species::Scorpion, Species::Viper),
            Biome::Badlands => (Species::Ibex, Species::Scorpion, Species::Viper),
            Biome::Savanna => (Species::Antelope, Species::Lion, Species::Lion),
            Biome::Jungle => (Species::Deer, Species::Jaguar, Species::Viper),
            Biome::Swamp | Biome::Marsh => (Species::Deer, Species::Viper, Species::Alligator),
            Biome::Shrubland => (Species::Antelope, Species::Wolf, Species::Viper),
            _ => (Species::Deer, Species::Wolf, Species::Bear),
        }
    }

    /// Once a day: a dwarf may come upon a lair, and the wild may come
    /// calling, from a lair that has beasts at home (or, for those with
    /// none, from the map's edge). A land hunted out sends nothing.
    pub(super) fn wildlife(&mut self) {
        self.come_upon_lairs();
        // The bestiary's animals come and go by their own ways (beasts.rs).
        self.beast_visit();
        if !self.rng.chance(self.knob("wildlife.chance", 0.18)) {
            return;
        }
        self.count("wildlife.visits", 1);
        let roll = self.rng.f64();
        let tame = 1.0 - self.savagery.clamp(0.05, 0.9) * 0.7;
        let (calm, pack, rare) = self.fauna();
        let (mut species, n) = if roll < tame || self.tick < SEASON {
            (calm, self.rng.range(2, 5))
        } else if roll < 0.95 || self.tick < YEAR {
            (pack, self.rng.range(2, 4))
        } else {
            (rare, 1)
        };
        // Evil land sends its dead.
        if self.evil && species.info().aggro > 0 && self.rng.chance(0.5) {
            species = Species::Husk;
        }
        let Some(from) = self.lair_sends(species, n as u32) else {
            self.count("wildlife.none", 1);
            return;
        };
        let (entry, n, lair) = match from {
            Some((i, n)) => (self.lairs[i].pos, n as i32, Some(i as u32)),
            None => match self.edge_entry() {
                Some(p) => (p, n, None),
                None => return,
            },
        };
        let stay = DAY * self.rng.range(2, 5) as u64;
        let mut pack = 0;
        for _ in 0..n {
            let p = self.spot_near(entry, 2);
            let id = self.spawn_creature(species, p, stay);
            if let Some(c) = self.creatures.get_mut(&id) {
                c.lair = lair;
            }
            // Those that came together hunt together.
            if n > 1 {
                if pack == 0 {
                    pack = id;
                }
                if let Some(c) = self.creatures.get_mut(&id) {
                    c.pack = pack;
                }
            }
        }
        let info = species.info();
        if info.aggro > 0 {
            let a = if info.name.starts_with(|c: char| "aeiou".contains(c)) { "An" } else { "A" };
            let what = if n > 1 { format!("A pack of {}", info.plural) } else { format!("{} {}", a, info.name) };
            let from = match lair.map(|i| &self.lairs[i as usize]) {
                Some(l) if l.found => format!(", out of {}", l.name),
                _ => String::new(),
            };
            self.msg(&format!("{} has been spotted nearby{}.", what, from), (255, 170, 80));
        }
    }

    /// A wild hostile with no target picks one. Beasts with some sense
    /// size up their prey and their own numbers first, and shadow a dwarf
    /// they daren't take yet; the rest go for the nearest dwarf they can get
    /// to.
    pub(super) fn predator_think(&mut self, c: &mut Creature) {
        if c.target.is_some() {
            return;
        }
        if !has_sense(c) {
            self.nearest_prey(c);
            return;
        }
        let info = c.species.info();
        if c.wary_until > self.tick {
            // Hurt: it keeps well off and licks its wounds.
            c.stalk = None;
            self.keep_off(c, info.aggro + 8);
            return;
        }
        let pack = self.pack_strength(c) as f64;
        // In the last quarter of the day it takes more chances.
        let night = self.tick % DAY >= DAY * 3 / 4;
        let nerve = self.knob("wild.nerve", 1.5) * if night { self.knob("wild.night", 0.5) } else { 1.0 };
        // With the grazing grounds emptied, hunger makes it bolder.
        let nerve = nerve * if self.prey_scarce() { self.knob("wild.hungry", 0.7) } else { 1.0 };
        // Everyone in sight (and the one it follows, a little further), the
        // weakest (then the nearest) first.
        let mut prey: Vec<(f64, f64, u32, Pos)> = self
            .dwarves
            .iter()
            .filter(|d| d.alive && (near(d.pos, c.pos, info.aggro + 6) || c.stalk == Some(d.id) && near(d.pos, c.pos, info.aggro + 12)))
            .map(|d| {
                let (odds, escorts) = self.prey_odds(d);
                (odds, escorts, d.id, d.pos)
            })
            .collect();
        prey.sort_by(|a, b| b.0.total_cmp(&a.0).then(dist2(a.3, c.pos).cmp(&dist2(b.3, c.pos))));
        let limit = ((info.aggro + 6).pow(2) * 8).clamp(400, 4000) as usize;
        let Some(&(odds, escorts, id, pos)) = prey.iter().find(|p| self.can_reach(c, p.3, limit)) else {
            c.stalk = None;
            if !prey.is_empty() {
                c.think_cd = 90;
            }
            return;
        };
        // It goes for one weak enough for its pack: within its reach, or
        // from twice as far when they're alone (once hurt, only the lone).
        let alone = escorts == 0.0;
        let bold = odds * pack >= nerve && (alone || c.wary_until == 0);
        if bold && near(pos, c.pos, if alone { info.aggro * 2 } else { info.aggro }) {
            c.target = Some(id);
            c.stalk = None;
            c.path.clear();
            return;
        }
        if c.stalk != Some(id) {
            // The first of its pack to take up this trail tells of it.
            let followed = c.pack != 0 && self.creatures.values().any(|o| o.pack == c.pack && o.stalk == Some(id));
            if c.stalk.is_none() && !followed {
                self.count("stalks", 1);
                self.woods_fall_quiet(c.pos);
                let who = self.dwarves.iter().find(|d| d.id == id).map_or(String::new(), |d| d.name.clone());
                let what = if pack > 1.0 { format!("The {}", info.plural) } else { format!("A {}", info.name) };
                let verb = if pack > 1.0 { "shadow" } else { "shadows" };
                let wher = if self.world.outside(pos) { " at the treeline" } else { "" };
                self.msg(&format!("{} {} {}{}.", what, verb, who, wher), (255, 190, 110));
            }
            c.stalk = Some(id);
        }
        self.shadow(c, pos);
    }

    /// Silent woods (CRE-22): a beast at `at` has taken up a trail, and the
    /// birds about it go quiet. Grown dwarves under the sky within 15 notice
    /// and grow uneasy; the nearest is named. An overseer takes it as warning
    /// that something prowls, before anyone has seen it (`heed_quiet_woods`).
    pub(super) fn woods_fall_quiet(&mut self, at: Pos) {
        let r = self.knob("wild.quiet_range", 15.0) as i32;
        let mut heard: Vec<usize> = (0..self.dwarves.len())
            .filter(|&i| {
                let d = &self.dwarves[i];
                d.alive && !d.is_child() && self.world.outside(d.pos) && (d.pos.x - at.x).abs() <= r && (d.pos.y - at.y).abs() <= r && (d.pos.z - at.z).abs() <= 3
            })
            .collect();
        if heard.is_empty() {
            return;
        }
        heard.sort_by_key(|&i| (dist2(self.dwarves[i].pos, at), self.dwarves[i].id));
        let d = &self.dwarves[heard[0]];
        let calling = match d.task {
            Task::Dig { target, .. } if self.world.t(target).kind == TileKind::Tree => "woodcutter".to_string(),
            Task::Gather { .. } => "gatherer".to_string(),
            Task::Fish { .. } => "fisher".to_string(),
            _ => d.profession().to_lowercase(),
        };
        let first = d.name.split(' ').next().unwrap_or("someone").to_string();
        let near_pos = d.pos;
        self.count("quiet_woods", 1);
        self.msg(&format!("The birds have gone quiet near the {} {}.", calling, first), (200, 200, 150));
        let tick = self.tick;
        for i in heard {
            self.dwarves[i].think("felt uneasy as the birds went quiet", 3, tick);
        }
        self.heed_quiet_woods(near_pos);
    }

    /// Keep `aggro + 3 .. aggro + 6` tiles off the stalked dwarf at `prey`.
    fn shadow(&mut self, c: &mut Creature, prey: Pos) {
        let a = c.species.info().aggro;
        let ring = |q: Pos| {
            let d = (q.x - prey.x).abs().max((q.y - prey.y).abs());
            (q.z - prey.z).abs() <= 1 && d >= a + 3 && d <= a + 6
        };
        if ring(c.pos) && c.path.is_empty() || c.path.first().map_or(false, |&q| ring(q)) {
            return;
        }
        let w = &self.world;
        let open_air = w.outside(c.pos);
        if let Some((_, path)) = path::search(w, c.pos, 900, c.mover(), |q| ring(q) && (!open_air || w.outside(q))) {
            c.path = path.into_iter().rev().collect();
        }
    }

    /// Get at least `r` tiles from every dwarf about.
    fn keep_off(&mut self, c: &mut Creature, r: i32) {
        let about: Vec<Pos> = self.dwarves.iter().filter(|d| d.alive && near(d.pos, c.pos, r)).map(|d| d.pos).collect();
        if about.is_empty() || !c.path.is_empty() {
            return;
        }
        let clear = |q: Pos| about.iter().all(|p| (p.x - q.x).abs().max((p.y - q.y).abs()) >= r);
        let w = &self.world;
        let open_air = w.outside(c.pos);
        if let Some((_, path)) = path::search(w, c.pos, 900, c.mover(), |q| clear(q) && (!open_air || w.outside(q))) {
            c.path = path.into_iter().rev().collect();
        }
    }

    /// The nerve to break (every tick, from `creature_tick`): the first
    /// real wound (a quarter of a part) sends a wild beast off for a while,
    /// and while it's wary it doesn't turn on whoever hits it.
    pub(super) fn wild_nerve(&mut self, c: &mut Creature) {
        self.shun_fire(c);
        if c.leaving || !c.hostile() || !has_sense(c) {
            return;
        }
        if c.wary_until == 0 && c.body.hp.iter().zip(c.body.max.iter()).any(|(hp, max)| hp * 4 <= max * 3) {
            c.wary_until = self.tick + self.knob("wild.wary", 200.0) as u64;
            self.count("wild.wary", 1);
            self.msg(&format!("The wounded {} breaks off.", c.name()), (160, 230, 160));
        }
        if c.wary_until > self.tick && (c.target.is_some() || c.stalk.is_some() || c.foe.is_some()) {
            c.target = None;
            c.stalk = None;
            c.foe = None;
            c.path.clear();
            c.think_cd = 0;
        }
    }

    /// A wild beast has just struck the dwarf at `di` (`creature_tick`). With
    /// two or more grown dwarves on their feet within 3 of them, they stand
    /// together: at each swing it may lose its nerve (knob wild.together,
    /// 0.3 for each friend past the first) and back off, wary, as if hurt.
    pub(super) fn stand_together(&mut self, c: &mut Creature, di: usize) {
        if !has_sense(c) || c.leaving || c.target.is_none() {
            return;
        }
        let Some(d) = self.dwarves.get(di).filter(|d| d.alive) else { return };
        let (at, victim) = (d.pos, d.id);
        let mut allies: Vec<usize> = (0..self.dwarves.len())
            .filter(|&i| {
                let o = &self.dwarves[i];
                o.alive && o.id != victim && !o.is_child() && !o.body.unconscious() && near(o.pos, at, 3)
            })
            .collect();
        if allies.len() < 2 || !self.rng.chance(self.knob("wild.together", 0.3) * (allies.len() - 1) as f64) {
            return;
        }
        c.wary_until = self.tick + self.knob("wild.wary", 200.0) as u64;
        c.target = None;
        c.stalk = None;
        c.foe = None;
        c.path.clear();
        c.think_cd = 0;
        self.count("wild.driven_off", 1);
        allies.sort_by_key(|&i| dist2(self.dwarves[i].pos, at));
        allies.insert(0, di);
        let first = |g: &Game, i: usize| g.dwarves[i].name.split(' ').next().unwrap_or("someone").to_string();
        let names: Vec<String> = allies.iter().take(3).map(|&i| first(self, i)).collect();
        let who = if allies.len() > 3 { format!("{}, {}, {} and the others", names[0], names[1], names[2]) } else { format!("{}, {} and {}", names[0], names[1], names[2]) };
        let name = c.name().to_string();
        self.msg(&format!("{} stand together, and the {} backs off.", who, name), (160, 230, 160));
        let tick = self.tick;
        for i in allies {
            self.dwarves[i].think(&format!("stood together against a {}", name), -3, tick);
        }
    }

    /// Flames scare the wild (every 5 ticks, from `wild_nerve`): a beast
    /// within 2 of a burning tile or a dragon's breath runs from it, and one
    /// that has been burned (`fears_fire`) keeps 6 off (knob wild.fire_fear).
    fn shun_fire(&mut self, c: &mut Creature) {
        if c.leaving || !has_sense(c) || (self.tick + c.id as u64) % 5 != 0 {
            return;
        }
        let burning = self.fire_cells.as_ref().map_or(false, |v| !v.is_empty());
        if !burning && self.flames.is_empty() {
            return;
        }
        let r = if c.fears_fire { self.knob("wild.fire_fear", 6.0) as i32 } else { 2 };
        let tick = self.tick;
        let w = &self.world;
        let fires: Vec<Pos> = self
            .fire_cells
            .iter()
            .flatten()
            .map(|&i| w.pos_of(i))
            .chain(self.flames.iter().filter(|&&(_, t)| t + 12 > tick).map(|&(p, _)| p))
            .filter(|&p| near(p, c.pos, r + 2))
            .collect();
        if !fires.iter().any(|&p| near(p, c.pos, r)) {
            return;
        }
        let was = c.target.is_some() || c.stalk.is_some() || c.foe.is_some();
        c.target = None;
        c.stalk = None;
        c.foe = None;
        c.think_cd = 30;
        let clear = |q: Pos| fires.iter().all(|&p| !near(p, q, r + 1));
        if !c.path.first().map_or(false, |&q| clear(q)) {
            c.path.clear();
            if let Some((_, path)) = path::search(w, c.pos, 900, c.mover(), |q| clear(q) && w.t(q).fire == 0) {
                c.path = path.into_iter().rev().collect();
            }
        }
        if was {
            self.count("wild.fire_fled", 1);
            let name = c.species.info().name;
            self.warn(&format!("fire_fear:{}", name), &format!("The {} shies from the flames.", name), (255, 190, 110));
        }
    }

    /// A pack member has died (`creature_died`): the rest of the pack
    /// within 12 lose heart and run.
    pub(super) fn pack_breaks(&mut self, c: &Creature) {
        if c.pack == 0 || !has_sense(c) {
            return;
        }
        let mut n = 0;
        for o in self.creatures.values_mut().filter(|o| o.pack == c.pack && o.id != c.id && o.alive && !o.leaving && near(o.pos, c.pos, 12)) {
            o.leaving = true;
            o.target = None;
            o.stalk = None;
            o.foe = None;
            o.path.clear();
            n += 1;
        }
        if n > 0 {
            self.count("packs_broken", 1);
            self.msg(&format!("The {} scatter!", c.species.info().plural), (120, 255, 120));
        }
    }

    /// The nearest dwarf it can actually get to (a beast in a sealed cavern
    /// doesn't wear itself out hunting dwarves through rock).
    fn nearest_prey(&mut self, c: &mut Creature) {
        let r = c.species.info().aggro;
        let mut prey: Vec<(u32, Pos)> = self.dwarves.iter().filter(|d| d.alive && near(d.pos, c.pos, r)).map(|d| (d.id, d.pos)).collect();
        prey.sort_by_key(|(_, p)| dist2(*p, c.pos));
        let limit = (r * r * 8).clamp(400, 4000) as usize;
        c.target = prey.iter().find(|(_, p)| self.can_reach(c, *p, limit)).map(|(id, _)| *id);
        if c.target.is_none() && !prey.is_empty() {
            c.think_cd = 90;
        }
    }

    /// How a predator sizes up a dwarf: (odds, escorts). Weak prey is
    /// bleeding, limping or a child, and not holding a weapon; escorts are
    /// the others within 4 (a soldier counts as three).
    pub(super) fn prey_odds(&self, d: &Dwarf) -> (f64, f64) {
        let mut weak = 1.0;
        if d.body.bleeding > 0 || d.body.limp() > 0 {
            weak += 0.5;
        }
        if d.is_child() {
            weak += 0.5;
        }
        if d.held.and_then(|id| self.items.get(&id)).map_or(false, |it| it.kind.weapon_damage().is_some()) {
            weak -= 0.5;
        }
        let escorts: f64 = self
            .dwarves
            .iter()
            .filter(|o| o.alive && o.id != d.id && !o.is_child() && near(o.pos, d.pos, 4))
            .map(|o| if o.soldier { 3.0 } else { 1.0 })
            .sum();
        (weak / (1.0 + escorts), escorts)
    }

    /// Its pack about it: itself and the mates within 8.
    pub(super) fn pack_strength(&self, c: &Creature) -> u32 {
        if c.pack == 0 {
            return 1;
        }
        1 + self.creatures.values().filter(|o| o.pack == c.pack && o.id != c.id && o.alive && !o.leaving && near(o.pos, c.pos, 8)).count() as u32
    }

    /// Wild hostiles near the fort (not sieges, not legends), nearest first,
    /// stalkers included: (creature id, pos, pack id). A beast on its own is
    /// its own pack.
    pub fn prowlers(&self, near: Pos, r: i32) -> Vec<(u32, Pos, u32)> {
        let mut v: Vec<(u32, Pos, u32)> = self
            .creatures
            .values()
            .filter(|c| c.hostile() && !c.siege && !c.legendary() && !c.leaving && self.creature_active(c) && !super::beasts::keeps_to_itself(c))
            .filter(|c| (c.pos.x - near.x).abs() <= r && (c.pos.y - near.y).abs() <= r && (c.pos.z - near.z).abs() <= 6)
            .map(|c| (c.id, c.pos, if c.pack != 0 { c.pack } else { c.id }))
            .collect();
        v.sort_by_key(|&(id, p, _)| (dist2(p, near), id));
        v
    }

    /// The overseer's sense of prowlers: beasts within 30 of the fort and
    /// too few soldiers to see them off (about 4 strength each). It pulls
    /// towards Muster and a little towards Fortify.
    pub(super) fn prowler_sense(&self, o: &Overseer) -> Option<(Sense, f64, Vec<(Ambition, f64)>)> {
        let at = o.base.or(o.top).unwrap_or(self.embark);
        let n = self.prowlers(at, 30).len();
        if n == 0 {
            return None;
        }
        let strength: i32 = self.squads.iter().map(|s| self.squad_strength(&s.members)).sum();
        let need = self.knob("wild.prowler_strength", 4.0) * n as f64;
        let level = ((need - strength as f64) / need).clamp(0.0, 1.0);
        (level > 0.0).then(|| (Sense::Prowlers, level, vec![(Ambition::Muster, 0.8), (Ambition::Fortify, 0.4)]))
    }

    /// What a wild beast is up to, for its card: shadowing someone, or
    /// keeping off with a wound.
    pub fn wild_mood(&self, c: &Creature) -> Option<String> {
        if !c.alive {
            return None;
        }
        if c.leaving {
            return self.lair_of(c).filter(|l| l.found).map(|l| format!("Making for {}", l.name));
        }
        if c.wary_until > self.tick {
            return Some("Wounded and wary: keeping its distance".into());
        }
        let d = self.dwarves.iter().find(|d| Some(d.id) == c.stalk)?;
        Some(format!("Shadowing {}", d.name))
    }
}

/// Beasts of the land, which weigh their chances: not the dead, the
/// legendary, cave things or people.
pub(super) fn has_sense(c: &Creature) -> bool {
    !c.siege
        && !c.tame
        && !c.legendary()
        && !c.species.person()
        && !c.species.dwells_underground()
        && !matches!(c.species, Species::Husk | Species::Skulker)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::Part;
    use crate::items::ItemKind;
    use crate::material::Mat;

    /// A fort with no beasts about, and level ground a little way off:
    /// three soldiers with spears at its west end, a woodcutter with an
    /// axe 15 tiles east.
    fn meadow(seed: u64) -> (Game, Pos, [u32; 3], u32) {
        let mut g = Game::new(seed);
        g.creatures.retain(|_, c| c.tame);
        // Room all round, so the beasts can circle as they would in the open.
        let a = g.flat_patch(44, 31, 4).offset(14, 15, 0);
        for i in 0..16 {
            for j in -1..=1 {
                g.world.t_mut(a.offset(i, j, 0)).kind = TileKind::Floor;
            }
        }
        let mut squad = [0; 3];
        for i in 0..3 {
            let spear = g.spawn_item(ItemKind::Spear, Mat::Iron, a);
            let d = &mut g.dwarves[i];
            d.soldier = true;
            d.held = Some(spear);
            d.pos = a.offset(0, i as i32 - 1, 0);
            squad[i] = d.id;
        }
        let axe = g.spawn_item(ItemKind::Axe, Mat::Iron, a);
        let d = &mut g.dwarves[3];
        d.held = Some(axe);
        d.pos = a.offset(15, 0, 0);
        let woodcutter = d.id;
        // Nobody else about.
        g.dwarves.truncate(4);
        (g, a, squad, woodcutter)
    }

    /// Run only the creatures, the dwarves held where they stand.
    fn hold(g: &mut Game, ticks: u64, mut each: impl FnMut(&Game)) {
        let spots: Vec<Pos> = g.dwarves.iter().map(|d| d.pos).collect();
        for _ in 0..ticks {
            for (d, p) in g.dwarves.iter_mut().zip(&spots) {
                d.pos = *p;
            }
            g.update_creatures();
            g.tick += 1;
            each(g);
        }
    }

    #[test]
    fn wolves_pass_a_squad_and_take_the_straggler() {
        for seed in [2, 3, 4] {
            let (mut g, a, squad, woodcutter) = meadow(seed);
            let wolves = [g.spawn_creature(Species::Wolf, a.offset(7, 0, 0), u64::MAX), g.spawn_creature(Species::Wolf, a.offset(7, 1, 0), u64::MAX)];
            for w in wolves {
                g.creatures.get_mut(&w).unwrap().pack = wolves[0];
            }
            let mut hunted = false;
            hold(&mut g, 300, |g| {
                hunted |= g.creatures.values().any(|c| c.target == Some(woodcutter) || c.stalk == Some(woodcutter));
            });
            for id in squad {
                let d = g.dwarves.iter().find(|d| d.id == id).unwrap();
                assert!(d.attacker.is_none(), "seed {}: a wolf went for a soldier", seed);
            }
            let w = g.dwarves.iter().find(|d| d.id == woodcutter).unwrap();
            assert!(hunted || w.attacker.is_some(), "seed {}: the woodcutter was left alone (stalked or attacked)", seed);
        }
    }

    #[test]
    fn a_pack_sizes_up_its_prey() {
        let (g, _, squad, woodcutter) = meadow(2);
        let soldier = g.dwarves.iter().find(|d| d.id == squad[1]).unwrap();
        let (odds, escorts) = g.prey_odds(soldier);
        assert_eq!(escorts, 6.0, "two soldiers beside them");
        assert!(odds < 0.1);
        let w = g.dwarves.iter().find(|d| d.id == woodcutter).unwrap();
        assert_eq!(g.prey_odds(w), (0.5, 0.0), "alone, but holding an axe");
    }

    fn dwarf(g: &Game, id: u32) -> &Dwarf {
        g.dwarves.iter().find(|d| d.id == id).unwrap()
    }

    fn apart(a: Pos, b: Pos) -> i32 {
        (a.x - b.x).abs().max((a.y - b.y).abs())
    }

    #[test]
    fn a_wounded_wolf_breaks_off() {
        let (mut g, a, _, woodcutter) = meadow(2);
        let wolf = g.spawn_creature(Species::Wolf, a.offset(11, 0, 0), u64::MAX);
        // At dusk a lone wolf dares a lone dwarf, empty handed.
        g.tick = DAY - DAY / 5;
        g.dwarves[3].held = None;
        let mut went = false;
        hold(&mut g, 40, |g| went |= g.creatures[&wolf].target == Some(woodcutter));
        assert!(went, "the wolf should go for the lone woodcutter");
        // The woodcutter lays its leg open.
        let c = g.creatures.get_mut(&wolf).unwrap();
        let i = Part::LeftLeg.idx();
        c.body.hp[i] = c.body.max[i] / 2;
        let at = g.tick;
        let mut turned = false;
        hold(&mut g, 150, |g| turned |= g.tick > at + 2 && g.creatures[&wolf].target.is_some());
        let c = &g.creatures[&wolf];
        assert!(c.wary_until > at && !c.leaving, "wary, not gone");
        assert!(!turned, "it doesn't come back while it's wary");
        assert!(g.log.iter().any(|l| l.text.contains("The wounded wolf breaks off")));
        assert!(dwarf(&g, woodcutter).attacker.map_or(true, |(_, t)| t <= at + 2), "no more bites");
        assert!(apart(c.pos, dwarf(&g, woodcutter).pos) >= 6, "it keeps off");
        assert_eq!(g.annals.counters.get("wild.wary"), Some(&1));
    }

    #[test]
    fn the_pack_scatters_when_one_dies() {
        let (mut g, a, _, _) = meadow(2);
        let pack = [
            g.spawn_creature(Species::Wolf, a.offset(8, 0, 0), u64::MAX),
            g.spawn_creature(Species::Wolf, a.offset(9, 1, 0), u64::MAX),
            g.spawn_creature(Species::Wolf, a.offset(9, -1, 0), u64::MAX),
            // One off on its own, out of earshot.
            g.spawn_creature(Species::Wolf, a.offset(8 + 14, 0, 0), u64::MAX),
        ];
        for id in pack {
            g.creatures.get_mut(&id).unwrap().pack = pack[0];
        }
        let dead = g.creatures.remove(&pack[0]).unwrap();
        g.creature_died(dead);
        assert!(g.creatures[&pack[1]].leaving && g.creatures[&pack[2]].leaving, "the pack runs");
        assert!(!g.creatures[&pack[3]].leaving, "the one far off doesn't know");
        assert!(g.log.iter().any(|l| l.text.contains("The wolves scatter!")));
        assert_eq!(g.annals.counters.get("packs_broken"), Some(&1));
        // A lone beast's death breaks nothing.
        let bear = g.spawn_creature(Species::Bear, a.offset(4, 0, 0), u64::MAX);
        let dead = g.creatures.remove(&bear).unwrap();
        g.creature_died(dead);
        assert_eq!(g.annals.counters.get("packs_broken"), Some(&1));
    }

    #[test]
    fn a_stalker_waits_for_its_prey_to_be_alone() {
        let (mut g, a, squad, woodcutter) = meadow(4);
        // A friend works beside the woodcutter, both empty handed.
        g.dwarves[3].held = None;
        let friend = squad[0];
        if let Some(d) = g.dwarves.iter_mut().find(|d| d.id == friend) {
            d.soldier = false;
            d.held = None;
            d.pos = a.offset(14, 0, 0);
        }
        // The wolves come up from beyond the pair, so the pair is nearest.
        let wolves = [g.spawn_creature(Species::Wolf, a.offset(22, 0, 0), u64::MAX), g.spawn_creature(Species::Wolf, a.offset(22, 1, 0), u64::MAX)];
        for w in wolves {
            g.creatures.get_mut(&w).unwrap().pack = wolves[0];
        }
        let mut struck = false;
        hold(&mut g, 200, |g| struck |= wolves.iter().any(|w| g.creatures[w].target.is_some()));
        assert!(!struck, "two together are left alone");
        assert!(wolves.iter().all(|w| matches!(g.creatures[w].stalk, Some(id) if id == woodcutter || id == friend)), "but followed");
        assert_eq!(g.log.iter().filter(|l| l.text.contains("The wolves shadow")).count(), 1, "told once");
        assert_eq!(g.annals.counters.get("stalks"), Some(&1));
        // The friend goes back to the others, and the woodcutter is alone.
        if let Some(d) = g.dwarves.iter_mut().find(|d| d.id == friend) {
            d.pos = a.offset(0, 0, 0);
        }
        let mut went = false;
        hold(&mut g, 150, |g| went |= wolves.iter().any(|w| g.creatures[w].target == Some(woodcutter)));
        assert!(went, "they strike once the woodcutter is alone");
    }

    /// Three empty-handed dwarves side by side, a wolf at the middle one's
    /// throat: they stand together and it backs off. A lone dwarf gets no
    /// such luck.
    #[test]
    fn three_dwarves_together_drive_off_a_wolf() {
        for seed in [2, 3, 4] {
            let (mut g, a, squad, woodcutter) = meadow(seed);
            for d in g.dwarves.iter_mut() {
                d.soldier = false;
                d.held = None;
            }
            let wolf = g.spawn_creature(Species::Wolf, a.offset(1, 0, 0), u64::MAX);
            g.creatures.get_mut(&wolf).unwrap().target = Some(squad[1]);
            let mut backed = 0;
            hold(&mut g, 300, |g| {
                if backed == 0 && g.creatures[&wolf].wary_until > 0 {
                    backed = g.tick;
                }
            });
            let c = &g.creatures[&wolf];
            assert!(backed > 0, "seed {}: the wolf never backed off", seed);
            assert!(c.target.is_none() && !c.leaving, "seed {}: wary, not gone", seed);
            assert!(g.log.iter().any(|l| l.text.contains("stand together, and the wolf backs off.")), "seed {}", seed);
            assert_eq!(g.annals.counters.get("wild.driven_off"), Some(&1));
            assert!(squad.iter().all(|&id| dwarf(&g, id).alive), "seed {}: nobody died", seed);
            assert!(dwarf(&g, squad[0]).thoughts.iter().any(|t| t.text.contains("stood together")));
            // A lone dwarf isn't so lucky.
            let lone = g.spawn_creature(Species::Wolf, dwarf(&g, woodcutter).pos.offset(1, 0, 0), u64::MAX);
            g.creatures.get_mut(&lone).unwrap().target = Some(woodcutter);
            hold(&mut g, 60, |_| {});
            assert_eq!(g.creatures[&lone].wary_until, 0, "seed {}: the lone woodcutter drove it off", seed);
        }
    }

    /// Fire: a wolf beside a burning tree runs from it; one that has been
    /// burned keeps well clear.
    #[test]
    fn wild_things_fear_fire() {
        let (mut g, a, _, woodcutter) = meadow(2);
        let fire = a.offset(12, 0, 0);
        g.world.t_mut(fire).fire = 20;
        g.fire_cells = Some(vec![g.world.idx(fire)]);
        let wolf = g.spawn_creature(Species::Wolf, fire.offset(1, 1, 0), u64::MAX);
        g.creatures.get_mut(&wolf).unwrap().target = Some(woodcutter);
        hold(&mut g, 40, |_| {});
        let c = &g.creatures[&wolf];
        assert!(apart(c.pos, fire) >= 3, "it stays by the fire at {:?}", c.pos);
        assert!(g.log.iter().any(|l| l.text.contains("The wolf shies from the flames.")));
        // Burned once, it won't come within 6.
        let c = g.creatures.get_mut(&wolf).unwrap();
        c.fears_fire = true;
        c.pos = fire.offset(4, 0, 0);
        c.path.clear();
        hold(&mut g, 100, |_| {});
        assert!(apart(g.creatures[&wolf].pos, fire) >= 7, "burned, it still sits at {:?}", g.creatures[&wolf].pos);
    }

    /// A pack takes up the trail of a woodcutter and a friend working
    /// together under the sky: the birds go quiet, told once, and those
    /// about grow uneasy. An overseer keeps the work indoors.
    #[test]
    fn the_woods_go_quiet_when_something_stalks() {
        let (mut g, a, squad, woodcutter) = meadow(4);
        g.dwarves[3].held = None;
        let friend = squad[0];
        if let Some(d) = g.dwarves.iter_mut().find(|d| d.id == friend) {
            d.soldier = false;
            d.held = None;
            d.pos = a.offset(14, 0, 0);
        }
        assert!(g.world.outside(a.offset(15, 0, 0)), "the meadow is under the sky");
        let wolves = [g.spawn_creature(Species::Wolf, a.offset(22, 0, 0), u64::MAX), g.spawn_creature(Species::Wolf, a.offset(22, 1, 0), u64::MAX)];
        for w in wolves {
            g.creatures.get_mut(&w).unwrap().pack = wolves[0];
        }
        hold(&mut g, 200, |_| {});
        let quiet: Vec<&str> = g.log.iter().filter(|l| l.text.contains("gone quiet")).map(|l| l.text.as_str()).collect();
        assert_eq!(quiet.len(), 1, "{:?}", quiet);
        assert!(quiet[0].starts_with("The birds have gone quiet near the "), "{}", quiet[0]);
        assert_eq!(g.annals.counters.get("quiet_woods"), Some(&1));
        for id in [woodcutter, friend] {
            assert!(dwarf(&g, id).thoughts.iter().any(|t| t.text.contains("birds went quiet")));
        }
        // The overseer takes the hint when it's near the fort.
        g.set_overseer(Some(Style::Steady));
        assert!(!g.outdoors_banned());
        let home = g.embark;
        g.heed_quiet_woods(home);
        assert!(g.outdoors_banned());
        assert!(g.overseer.as_ref().unwrap().notes.iter().any(|n| n.1.contains("woods went quiet")));
    }

    #[test]
    fn prowlers_are_the_wild_hostiles_about_the_fort() {
        let (mut g, a, _, woodcutter) = meadow(2);
        let near_wolf = g.spawn_creature(Species::Wolf, a.offset(3, 0, 0), u64::MAX);
        let far_wolf = g.spawn_creature(Species::Wolf, a.offset(9, 0, 0), u64::MAX);
        for w in [near_wolf, far_wolf] {
            g.creatures.get_mut(&w).unwrap().pack = near_wolf;
        }
        let bear = g.spawn_creature(Species::Bear, a.offset(6, 1, 0), u64::MAX);
        g.spawn_creature(Species::Deer, a.offset(2, 0, 0), u64::MAX);
        let gob = g.spawn_creature(Species::Goblin, a.offset(1, 0, 0), u64::MAX);
        g.creatures.get_mut(&gob).unwrap().siege = true;
        let gone = g.spawn_creature(Species::Wolf, a.offset(4, 0, 0), u64::MAX);
        g.creatures.get_mut(&gone).unwrap().leaving = true;
        let p = g.prowlers(a, 30);
        let ids: Vec<u32> = p.iter().map(|x| x.0).collect();
        assert_eq!(ids, vec![near_wolf, bear, far_wolf], "nearest first; no deer, besiegers or leavers");
        assert_eq!((p[0].2, p[1].2, p[2].2), (near_wolf, bear, near_wolf), "a beast alone is its own pack");
        assert!(g.prowlers(a, 2).is_empty());
        // The overseer feels them while its soldiers are few.
        g.set_overseer(Some(Style::Steady));
        let b = g.overseer_brain().unwrap();
        let level = b.senses.iter().find(|x| x.0 == Sense::Prowlers).unwrap().1;
        assert!(level > 0.5, "prowlers sensed at {}", level);
        assert!(b.links_of(Ambition::Muster).iter().any(|l| l.0 == Sense::Prowlers));
        // And the card says what a beast is about.
        let mut c = g.creatures[&far_wolf].clone();
        c.stalk = Some(woodcutter);
        assert_eq!(g.wild_mood(&c), Some(format!("Shadowing {}", dwarf(&g, woodcutter).name)));
        c.wary_until = g.tick + 10;
        assert!(g.wild_mood(&c).unwrap().contains("wary"));
    }
}
