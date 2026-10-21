//! Fighting as a person (G03 A6): the hero's aimed blows, crossbow shots,
//! binding wounds, and the gear of the dead.
//!
//! An aimed blow (`A`) goes where the hero means it to, at a cost to the
//! odds: the head for a quick kill, the legs to cripple and trip, the
//! weapon arm to disarm, and called shots at the eyes and the teeth. It is
//! `combat::strike_at`, the fort's own blow with a mark, so the wounds,
//! the words and the blood are the fort's. A shot (`f`) looses a bolt from
//! the pack: it flies on the map, and lies where it fell to be picked up
//! again (unless it broke). Binding (`b`) presses the bleeding wounds and
//! bandages them with cloth or thread from the pack, else with strips torn
//! from the hero's clothes: it takes 300 ticks, and a blow cuts it short.

use super::super::combat::{adjacent, cap, Blow, Fighter};
use super::super::*;
use super::turn::{Outcome, Target};
use crate::body::{Body, Hurt, Part, Wound, PARTS};
use crate::items::ItemKind;

/// Where a blow is aimed: a part of the body, or a called shot at the
/// eyes or the teeth (both of the head, and harder still to hit).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aim {
    Part(Part),
    Eyes,
    Teeth,
}

/// The marks offered when aiming, in the order the list shows them.
pub const AIMS: [Aim; 9] = [
    Aim::Part(Part::Head),
    Aim::Eyes,
    Aim::Teeth,
    Aim::Part(Part::UpperBody),
    Aim::Part(Part::LowerBody),
    Aim::Part(Part::RightArm),
    Aim::Part(Part::LeftArm),
    Aim::Part(Part::RightLeg),
    Aim::Part(Part::LeftLeg),
];

impl Aim {
    /// The part the blow lands on.
    pub fn part(self) -> Part {
        match self {
            Aim::Part(p) => p,
            Aim::Eyes | Aim::Teeth => Part::Head,
        }
    }

    /// The cost to the chance to hit, in percent: a small mark is harder.
    pub fn penalty(self) -> i32 {
        match self {
            Aim::Eyes => 35,
            Aim::Teeth => 25,
            Aim::Part(Part::Head) => 20,
            Aim::Part(Part::LeftArm | Part::RightArm) => 10,
            Aim::Part(Part::LeftLeg | Part::RightLeg) => 5,
            Aim::Part(_) => 0,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Aim::Eyes => "eyes",
            Aim::Teeth => "teeth",
            Aim::Part(p) => p.name(),
        }
    }

    /// What aiming there is for, in a few words.
    pub fn hope(self) -> &'static str {
        match self {
            Aim::Part(Part::Head) => "a quick kill",
            Aim::Eyes => "put out an eye",
            Aim::Teeth => "knock their teeth out",
            Aim::Part(Part::UpperBody) => "the broadest mark",
            Aim::Part(Part::LowerBody) => "the belly",
            Aim::Part(Part::RightArm) => "the weapon arm: disarm",
            Aim::Part(Part::LeftArm) => "the shield arm",
            Aim::Part(_) => "cripple, bring them down",
        }
    }
}

/// The chance a blow from `att` lands on `def` (before a shield): skill
/// against skill, less for an aimed one.
pub(in crate::game) fn hit_chance(att: &Fighter, def: &Fighter, aim: Option<Aim>) -> f64 {
    let chance = (0.55 + 0.05 * (att.skill as f64 - def.skill as f64)).clamp(0.15, 0.95);
    match aim {
        Some(a) => (chance - a.penalty() as f64 / 100.0).max(0.05),
        None => chance,
    }
}

/// How long a crossbow takes to shoot and span again.
pub const RELOAD_TICKS: u32 = 20;
/// What binding wounds costs.
pub const BIND_TICKS: u32 = 300;
/// How far a crossbow reaches (the fort's marksdwarves' range).
pub const SHOT_RANGE: i32 = super::super::military::RANGE;

impl Game {
    /// What an aimed blow does beyond where it lands: a called shot at the
    /// eyes puts one out, at the teeth knocks some loose; a blow at the
    /// weapon arm may disarm, and one at a leg bring them down.
    pub(in crate::game) fn called_shot(&mut self, aim: Aim, att: &Fighter, def: &Fighter, w: Wound, body: &mut Body, b: &mut Blow) {
        // A real blow, not a glancing knock.
        let landed = w != Wound::Bruise || att.edged;
        match aim {
            Aim::Eyes => {
                if landed && body.harm.eyes < 2 && !b.hurts.iter().any(|h| matches!(h, Hurt::Eye(_))) {
                    body.harm.eyes += 1;
                    b.hurts.push(Hurt::Eye(att.edged));
                }
            }
            Aim::Teeth => {
                if landed && def.teeth && b.teeth == 0 && body.harm.teeth < 32 {
                    let n = 1 + (self.gore_roll(0x7ee7) % if att.edged { 2 } else { 4 }) as u8;
                    b.teeth = n.min(32 - body.harm.teeth);
                    body.harm.teeth += b.teeth;
                }
            }
            Aim::Part(Part::RightArm) => {
                if !b.disarmed && !body.dead && w == Wound::Gash && self.rng.chance(self.knob("aim.disarm", 0.5)) {
                    b.disarmed = true;
                }
            }
            Aim::Part(Part::LeftLeg | Part::RightLeg) => {
                if !b.sprawl && !body.dead && matches!(w, Wound::Gash | Wound::Mangle | Wound::Sever) && self.rng.chance(self.knob("aim.trip", 0.4)) {
                    b.sprawl = true;
                }
            }
            Aim::Part(_) => {}
        }
    }

    /// The hero strikes the creature `cid` in reach, aimed or not.
    pub(crate) fn hero_blow(&mut self, hero: usize, cid: u32, aim: Option<Aim>) -> Outcome {
        let p = self.dwarves[hero].pos;
        let Some(c) = self.creatures.get(&cid).filter(|c| c.alive) else {
            return Outcome::Refused("There is nobody there.".into());
        };
        if !adjacent(c.pos, p) {
            return Outcome::Refused(format!("The {} is out of reach.", c.species.info().name));
        }
        if let Some(a) = aim.filter(|a| c.body.lost[a.part().idx()]) {
            return Outcome::Refused(format!("The {} has no {} left to strike.", c.species.info().name, a.part().name()));
        }
        let mut d = std::mem::take(&mut self.dwarves[hero]);
        self.dwarf_attack_at(&mut d, cid, aim);
        let cost = d.attack_cd.max(1);
        self.dwarves[hero] = d;
        self.adv_run(hero, cost, false)
    }

    /// The hero's crossbow, if they hold one and can draw it.
    fn crossbow(&self, hero: usize) -> Option<u32> {
        let d = &self.dwarves[hero];
        d.held.filter(|id| self.items.get(id).map_or(false, |it| it.kind == ItemKind::Crossbow)).filter(|_| d.body.can_hold())
    }

    /// The bolts in the hero's pack, the best first.
    pub fn bolts(&self, hero: usize) -> Vec<u32> {
        let me = self.dwarves[hero].id;
        let mut v: Vec<(i32, u32)> = self
            .items
            .values()
            .filter(|it| it.kind == ItemKind::Bolt && it.carried_by == Some(me))
            .map(|it| (-(Self::bolt_damage(Some(it.mat)) * 10.0) as i32, it.id))
            .collect();
        v.sort();
        v.into_iter().map(|(_, id)| id).collect()
    }

    /// The hero as a shooter with this bolt (as a marksdwarf shoots:
    /// `try_shoot`): a metal crossbow draws harder, a metal bolt bites deeper.
    fn bolt_fighter(&self, hero: usize, bolt: u32) -> Fighter {
        let d = &self.dwarves[hero];
        let metal = |m: Option<crate::material::Mat>| matches!(m, Some(crate::material::Mat::Copper | crate::material::Mat::Bronze | crate::material::Mat::Iron | crate::material::Mat::Steel));
        let bow = d.held.and_then(|id| self.items.get(&id)).map(|it| it.mat);
        let draw = if metal(bow) { self.knob("bolt.draw", 1.0) as u32 } else { 0 };
        let mat = self.items.get(&bolt).map(|it| it.mat);
        let mut f = Fighter {
            name: d.called(),
            skill: d.fight_level() + draw,
            dmg: Self::bolt_damage(mat) * self.knob("bolt.mult", 1.0) as f32,
            verb: "shoots",
            edged: true,
            helm: 0,
            mail: 0,
            shield: false,
            size: 1.0,
            teeth: true,
            weapon: None,
            held: None,
        };
        // A lost eye spoils the aim.
        super::super::gore::hurt_fighter(&mut f, &d.body);
        f
    }

    /// Creatures the hero could shoot at: in sight and in range, foes first.
    pub fn marks(&self, hero: usize) -> Vec<u32> {
        let p = self.dwarves[hero].pos;
        let mut v: Vec<(bool, u32)> = self
            .in_sight(hero)
            .into_iter()
            .filter_map(|id| self.creatures.get(&id))
            .filter(|c| (c.pos.x - p.x).abs().max((c.pos.y - p.y).abs()) <= SHOT_RANGE && self.clear_shot(p, c.pos))
            .map(|c| (!c.hostile(), c.id))
            .collect();
        // (Stable: `in_sight` gives them nearest first.)
        v.sort_by_key(|&(peaceful, _)| peaceful);
        v.into_iter().map(|(_, id)| id).collect()
    }

    /// The hero's odds, in percent, of landing a blow (or a shot) on the
    /// creature `cid`, aimed at `aim`.
    pub fn hit_odds(&self, hero: usize, cid: u32, aim: Option<Aim>, shot: bool) -> i32 {
        let Some(c) = self.creatures.get(&cid) else { return 0 };
        let att = match self.bolts(hero).first().filter(|_| shot) {
            Some(&b) => self.bolt_fighter(hero, b),
            None => self.dwarf_fighter(&self.dwarves[hero]),
        };
        let def = self.creature_fighter(c);
        let p = if c.body.unconscious() {
            self.knob("opening.hit", 0.95)
        } else if c.body.harm.out > 0 {
            hit_chance(&att, &def, aim).max(0.85)
        } else {
            hit_chance(&att, &def, aim)
        };
        (p * 100.0).round() as i32
    }

    /// The hero shoots a bolt at the creature `cid`: it flies on the map,
    /// strikes as an aimed blow would, and lies where it falls (a bolt
    /// that hits may break).
    pub(crate) fn hero_shoot(&mut self, hero: usize, cid: u32, aim: Option<Aim>) -> Outcome {
        if self.crossbow(hero).is_none() {
            return Outcome::Refused("You have no crossbow in hand.".into());
        }
        let Some(&bolt) = self.bolts(hero).first() else {
            return Outcome::Refused("You have no bolts left.".into());
        };
        let from = self.dwarves[hero].pos;
        let Some(c) = self.creatures.get(&cid).filter(|c| c.alive) else {
            return Outcome::Refused("There is nobody there.".into());
        };
        let name = c.species.info().name;
        if (c.pos.x - from.x).abs().max((c.pos.y - from.y).abs()) > SHOT_RANGE || !self.clear_shot(from, c.pos) {
            return Outcome::Refused(format!("You have no clear shot at the {}.", name));
        }
        if let Some(a) = aim.filter(|a| c.body.lost[a.part().idx()]) {
            return Outcome::Refused(format!("The {} has no {} left to shoot.", name, a.part().name()));
        }
        let att = self.bolt_fighter(hero, bolt);
        let def = self.creature_fighter(c);
        let mut body = c.body.clone();
        let (at, species, owner, seen) = (c.pos, c.species, super::super::gore::owner_of_creature(c), super::super::melee::a_or_name(c));
        // The bolt is seen to fly (drills.rs draws it; the adventure has no
        // drills to clear the old ones away).
        let tick = self.tick;
        self.drills.flights.retain(|f| f.lands() + 2 > tick);
        self.loose_bolt(from, at);
        let blow = self.strike_at(&att, &def, &mut body, aim);
        if let Some(m) = blow.msg.as_ref().filter(|_| blow.notable) {
            self.msg_at(m, (235, 225, 130), Some(at), super::super::logkind::LogKind::Combat);
        }
        let knocked = self.aftermath(&blow, from, at, &owner, &seen, Some(species), false);
        let hit = blow.msg.is_some() && blow.report.is_none();
        let dead = body.dead;
        let mut dropped = None;
        if let Some(c) = self.creatures.get_mut(&cid) {
            c.body = body;
            c.target = Some(super::HERO_ID);
            if let Some(q) = knocked {
                c.pos = q;
                c.path.clear();
                c.move_cd += 12;
                c.attack_cd += 12;
            }
            if blow.disarmed && !dead {
                dropped = c.weapon.take().map(|w| (w, c.pos));
            }
        }
        if let Some((w, p)) = dropped {
            self.drop_item(w, p);
        }
        // Where the bolt ends up: in the one it struck (and falls there,
        // unless it broke), or on past them into the ground.
        if hit && self.rng.chance(self.knob("bolt.break", 0.3)) {
            self.items.remove(&bolt);
        } else {
            let on = 1 + self.rng.range(0, 3);
            let land = if hit { at } else { self.past(from, at, on) };
            self.drop_item(bolt, land);
        }
        let mut d = std::mem::take(&mut self.dwarves[hero]);
        d.fight_xp += 6;
        if dead {
            if let Some(c) = self.creatures.remove(&cid) {
                self.credit_kill(&mut d, &c);
                self.creature_died(c);
            }
        }
        self.dwarves[hero] = d;
        self.adv_run(hero, RELOAD_TICKS, false)
    }

    /// The last open tile up to `n` tiles on past `at`, along the line from `from`.
    fn past(&self, from: Pos, at: Pos, n: i32) -> Pos {
        let (dx, dy) = ((at.x - from.x).signum(), (at.y - from.y).signum());
        let mut p = at;
        for _ in 0..n {
            let q = p.offset(dx, dy, 0);
            if !self.world.in_bounds(q) || self.world.solid(q) {
                break;
            }
            p = q;
        }
        p
    }

    /// The hero binds their wounds: a hand pressed on each that bleeds,
    /// then bandages of cloth or thread from the pack (used up), or strips
    /// torn from their clothes. 300 ticks, cut short by a blow.
    pub(crate) fn bind(&mut self, hero: usize) -> Outcome {
        let bleeding: Vec<Part> = PARTS.iter().copied().filter(|p| self.dwarves[hero].body.bleeds[p.idx()] > 0).collect();
        if bleeding.is_empty() {
            return Outcome::Refused("None of your wounds is bleeding.".into());
        }
        let me = self.dwarves[hero].id;
        let first = self.dwarves[hero].name.split(' ').next().unwrap_or("").to_string();
        let cloth = self.items.values().filter(|it| it.carried_by == Some(me) && matches!(it.kind, ItemKind::Cloth | ItemKind::Thread)).map(|it| it.id).min();
        let shirt = self.dwarves[hero].clothes.first().and_then(|id| self.items.get(id)).map(|it| it.name());
        let (with, bandage) = match (cloth, &shirt) {
            (Some(id), _) => {
                let what = self.items.get(&id).map_or("cloth".to_string(), |it| it.name());
                self.items.remove(&id);
                (format!("binds the wounds with {}", what), true)
            }
            (None, Some(s)) => (format!("tears strips from the {} to bind the wounds", s), true),
            (None, None) => ("presses a hand to the wounds, with nothing to bind them".to_string(), false),
        };
        let parts: Vec<&str> = bleeding.iter().map(|p| p.name()).collect();
        self.msg(&format!("{} {} ({}).", first, with, parts.join(", ")), (200, 220, 180));
        let start = self.tick;
        let mut spent = 0;
        while spent < BIND_TICKS {
            {
                let b = &mut self.dwarves[hero].body;
                for &p in &bleeding {
                    if b.bleeds[p.idx()] > 0 {
                        b.press(p);
                    }
                }
                // The bandages go on once the first bleeding is held back.
                if bandage && spent == 60 {
                    for &p in &bleeding {
                        b.bandage(p);
                    }
                }
            }
            match self.adv_run(hero, 10, false) {
                Outcome::Done { ticks } => spent += ticks,
                o => return o,
            }
            if let Some((cid, t)) = self.dwarves[hero].attacker {
                if t > start {
                    let who = self.creatures.get(&cid).map_or("something".to_string(), |c| format!("the {}", c.species.info().name));
                    return Outcome::Interrupted { ticks: spent, why: format!("{} strikes you as you bind your wounds!", cap(&who)) };
                }
            }
            if self.dwarves[hero].body.bleed_rate() == 0 && (!bandage || spent > 60) {
                break;
            }
        }
        let b = &self.dwarves[hero].body;
        let (text, col) = if b.bleed_rate() == 0 {
            (format!("{}'s wounds stop bleeding.", first), (160, 230, 160))
        } else {
            (format!("{}'s wounds still bleed.", first), (235, 120, 100))
        };
        self.msg(&text, col);
        Outcome::Done { ticks: spent }
    }
}

/// What lies on a tile, the dead first and their gear after (for `g`).
pub fn lying_at(g: &Game, p: Pos) -> Vec<u32> {
    let mut v: Vec<(bool, u32)> = g
        .items
        .values()
        .filter(|it| it.pos == p && it.carried_by.is_none() && it.container.is_none())
        .map(|it| (!matches!(it.kind, ItemKind::Corpse | ItemKind::Carcass), it.id))
        .collect();
    v.sort();
    v.into_iter().map(|(_, id)| id).collect()
}

/// Everything lying here worth carrying off: all but the bodies.
pub fn loot_at(g: &Game, p: Pos) -> Vec<u32> {
    lying_at(g, p).into_iter().filter(|id| g.items.get(id).map_or(false, |it| !matches!(it.kind, ItemKind::Corpse | ItemKind::Carcass))).collect()
}

/// The creature an act is aimed at, if any.
pub fn creature_of_act(a: &super::turn::Action) -> Option<u32> {
    match a {
        super::turn::Action::Attack { target: Target::Creature(id), .. } | super::turn::Action::Fire { target: Target::Creature(id), .. } => Some(*id),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::test_hero;
    use super::super::turn::Action;
    use super::super::*;
    use super::*;
    use crate::creature::Species;
    use crate::material::Mat;

    /// An adventure in a wild field with nothing else in it, the hero at
    /// its middle.
    fn field(seed: u64) -> Adventure {
        static WORLD: std::sync::OnceLock<crate::overworld::Overworld> = std::sync::OnceLock::new();
        let ow = WORLD.get_or_init(|| crate::overworld::Overworld::generate(5)).clone();
        let mut a = Adventure::new(ow, seed, test_hero(seed), None);
        let (x, y) = a.at;
        a.enter(SiteKey::Wild(x, y)).unwrap();
        a.visit.as_mut().unwrap().g.creatures.clear();
        a
    }

    /// Put an item of the kind in the hero's pack; its id.
    fn give(g: &mut Game, kind: ItemKind, mat: Mat) -> u32 {
        let p = g.dwarves.iter().find(|d| d.id == HERO_ID).unwrap().pos;
        let id = g.spawn_item(kind, mat, p);
        g.items.get_mut(&id).unwrap().carried_by = Some(HERO_ID);
        id
    }

    /// A foe come for the hero at `p`.
    fn foe(g: &mut Game, s: Species, p: Pos) -> u32 {
        let id = g.spawn_creature(s, p, DAY * 10);
        let c = g.creatures.get_mut(&id).unwrap();
        c.siege = s.person();
        c.target = Some(HERO_ID);
        id
    }

    #[test]
    fn aiming_at_the_head_hits_less_and_kills_more() {
        let mut a = field(31);
        let v = a.visit.as_mut().unwrap();
        let (g, h) = (&mut v.g, v.hero);
        let axe = give(g, ItemKind::Axe, Mat::Iron);
        g.dwarves[h].held = Some(axe);
        g.dwarves[h].fight_xp = 600;
        let gob = foe(g, Species::Goblin, g.dwarves[h].pos.offset(1, 0, 0));
        let att = g.dwarf_fighter(&g.dwarves[h]);
        let def = g.creature_fighter(&g.creatures[&gob]);
        let fresh = g.creatures[&gob].body.clone();
        // (hits, downed or dead, eyes put out, teeth knocked out) in 2,000 blows.
        let tally = |g: &mut Game, aim: Option<Aim>| {
            let (mut hits, mut downs, mut eyes, mut teeth) = (0, 0, 0, 0);
            for _ in 0..2000 {
                let mut body = fresh.clone();
                g.strike_at(&att, &def, &mut body, aim);
                if body.hp != fresh.hp {
                    hits += 1;
                }
                if body.dead || body.unconscious() {
                    downs += 1;
                }
                eyes += body.harm.eyes as i32;
                teeth += (body.harm.teeth > 0) as i32;
            }
            (hits, downs, eyes, teeth)
        };
        let free = tally(g, None);
        let head = tally(g, Some(Aim::Part(Part::Head)));
        let eyes = tally(g, Some(Aim::Eyes));
        let teeth = tally(g, Some(Aim::Teeth));
        let leg = tally(g, Some(Aim::Part(Part::LeftLeg)));
        assert!(head.0 * 100 < free.0 * 90, "the head is harder to hit: {:?} vs {:?}", head, free);
        assert!(head.1 > free.1 * 2, "but a blow there downs or kills far more often: {:?} vs {:?}", head, free);
        assert!(eyes.0 < head.0, "the eyes are harder still: {:?} vs {:?}", eyes, head);
        assert!(eyes.2 > free.2 * 5 && eyes.2 * 10 > eyes.0 * 7, "a called shot at the eyes puts them out: {:?} vs {:?}", eyes, free);
        assert!(teeth.3 > free.3 * 3 && teeth.3 * 10 > teeth.0 * 7, "and at the teeth knocks them out: {:?} vs {:?}", teeth, free);
        assert!(leg.0 > head.0 && leg.1 < head.1, "a leg is easy to hit and seldom kills: {:?}", leg);
    }

    #[test]
    fn a_blow_at_the_weapon_arm_disarms_and_at_the_legs_trips() {
        let (mut disarmed, mut tripped, mut free_trips) = (0, 0, 0);
        let mut a = field(32);
        let v = a.visit.as_mut().unwrap();
        let (g, h) = (&mut v.g, v.hero);
        let axe = give(g, ItemKind::Axe, Mat::Iron);
        g.dwarves[h].held = Some(axe);
        g.dwarves[h].fight_xp = 600;
        let gob = foe(g, Species::Goblin, g.dwarves[h].pos.offset(1, 0, 0));
        let att = g.dwarf_fighter(&g.dwarves[h]);
        let def = g.creature_fighter(&g.creatures[&gob]);
        let fresh = g.creatures[&gob].body.clone();
        for _ in 0..1000 {
            disarmed += g.strike_at(&att, &def, &mut fresh.clone(), Some(Aim::Part(Part::RightArm))).disarmed as i32;
            tripped += g.strike_at(&att, &def, &mut fresh.clone(), Some(Aim::Part(Part::RightLeg))).sprawl as i32;
            free_trips += g.strike_at(&att, &def, &mut fresh.clone(), None).sprawl as i32;
        }
        assert!(disarmed > 150, "{} disarmed in 1,000", disarmed);
        assert!(tripped > 100 && tripped > free_trips * 3, "{} tripped against {} unaimed", tripped, free_trips);
        // And a disarming blow from the hero leaves the weapon on the ground.
        let p = g.creatures[&gob].pos;
        let spear = g.spawn_item(ItemKind::Spear, Mat::Iron, p);
        g.items.get_mut(&spear).unwrap().carried_by = Some(gob);
        g.creatures.get_mut(&gob).unwrap().weapon = Some(spear);
        for _ in 0..200 {
            if g.creatures.get(&gob).map_or(true, |c| c.weapon.is_none() || !c.alive) || !g.dwarves[h].alive {
                break;
            }
            let at = g.dwarves[h].pos.offset(1, 0, 0);
            if let Some(c) = g.creatures.get_mut(&gob) {
                c.body = fresh.clone();
                c.pos = at;
            }
            g.dwarves[h].body = Body::default();
            g.hero_blow(h, gob, Some(Aim::Part(Part::RightArm)));
        }
        assert!(g.items[&spear].carried_by.is_none(), "the spear was struck from its hand");
    }

    #[test]
    fn a_bound_wound_stops_bleeding() {
        let bout = |cloth: bool| {
            let mut a = field(41);
            let v = a.visit.as_mut().unwrap();
            let (g, h) = (&mut v.g, v.hero);
            assert!(matches!(g.act(h, Action::Bind), Outcome::Refused(_)), "nothing to bind");
            let rag = if cloth { Some(give(g, ItemKind::Cloth, Mat::Threadreed)) } else { None };
            g.dwarves[h].clothes.clear();
            let bp = g.bleed_params();
            g.dwarves[h].body.sever_with(Part::LeftArm, &bp);
            assert!(g.dwarves[h].body.bleed_rate() > 0);
            let out = if cloth { g.act(h, Action::Bind) } else { g.act(h, Action::Wait(BIND_TICKS)) };
            let d = &g.dwarves[h];
            (out, d.alive, d.body.bleed_rate(), rag.map_or(false, |r| g.items.contains_key(&r)), g.log.iter().any(|e| e.text.contains("stop bleeding")))
        };
        let (out, alive, rate, rag_left, told) = bout(true);
        assert!(matches!(out, Outcome::Done { ticks } if ticks <= BIND_TICKS), "{:?}", out);
        assert!(alive && rate == 0, "the stump bound with cloth stops bleeding ({})", rate);
        assert!(!rag_left, "the cloth went into the bandage");
        assert!(told, "and the log says so");
        let (_, alive, rate, _, _) = bout(false);
        assert!(!alive || rate > 0, "unbound, the stump bleeds on");
    }

    #[test]
    fn binding_is_cut_short_by_a_blow() {
        let mut a = field(42);
        let v = a.visit.as_mut().unwrap();
        let (g, h) = (&mut v.g, v.hero);
        g.dwarves[h].body.wound(Part::LeftLeg, 8, true);
        let p = g.dwarves[h].pos;
        foe(g, Species::Wolf, p.offset(1, 0, 0));
        let out = g.act(h, Action::Bind);
        assert!(matches!(out, Outcome::Interrupted { ticks, .. } if ticks < BIND_TICKS) || out == Outcome::Died, "{:?}", out);
    }

    #[test]
    fn a_slain_goblins_gear_can_be_taken() {
        let mut a = field(51);
        let v = a.visit.as_mut().unwrap();
        let (g, h) = (&mut v.g, v.hero);
        let p = g.dwarves[h].pos;
        let gob = foe(g, Species::Goblin, p.offset(1, 0, 0));
        crate::game::adv::sitegen::arm(g, gob, ItemKind::Spear, Mat::Iron);
        crate::game::adv::sitegen::arm(g, gob, ItemKind::Breastplate, Mat::Iron);
        // It falls (as to the hero's blow).
        let c = g.creatures.remove(&gob).unwrap();
        g.creature_died(c);
        assert!(matches!(g.act(h, Action::Move(1, 0)), Outcome::Done { .. }));
        let here = lying_at(g, g.dwarves[h].pos);
        assert_eq!(here.len(), 3, "the body, its spear and its mail");
        assert_eq!(g.items[&here[0]].kind, ItemKind::Corpse, "the dead first");
        let loot = loot_at(g, g.dwarves[h].pos);
        assert_eq!(loot.len(), 2);
        assert!(matches!(g.act(h, Action::Get(loot.clone())), Outcome::Done { .. }));
        for id in &loot {
            assert_eq!(g.items[id].carried_by, Some(HERO_ID));
        }
        let spear = *loot.iter().find(|id| g.items[*id].kind == ItemKind::Spear).unwrap();
        assert!(matches!(g.act(h, Action::Wear(spear)), Outcome::Done { .. }));
        assert_eq!(g.dwarves[h].held, Some(spear), "the goblin's spear in the hero's hand");
    }

    #[test]
    fn a_bolt_flies_strikes_and_lies_where_it_fell() {
        let mut a = field(61);
        let v = a.visit.as_mut().unwrap();
        let (g, h) = (&mut v.g, v.hero);
        let p = g.dwarves[h].pos;
        let gob = foe(g, Species::Goblin, p.offset(6, 0, 0));
        let fire = Action::Fire { target: Target::Creature(gob), aim: Some(Aim::Part(Part::UpperBody)) };
        assert!(matches!(g.act(h, fire.clone()), Outcome::Refused(_)), "no crossbow");
        let bow = give(g, ItemKind::Crossbow, Mat::Wood);
        g.dwarves[h].held = Some(bow);
        assert!(matches!(g.act(h, fire.clone()), Outcome::Refused(_)), "no bolts");
        for _ in 0..6 {
            give(g, ItemKind::Bolt, Mat::Iron);
        }
        g.dwarves[h].fight_xp = 600;
        assert_eq!(g.marks(h), vec![gob]);
        let hp0: i32 = g.creatures[&gob].body.hp.iter().sum();
        let t0 = g.tick;
        let mut shots = 0;
        while !g.bolts(h).is_empty() && g.creatures.get(&gob).map_or(false, |c| c.alive) && g.dwarves[h].alive {
            // (It stands where it is, to be shot.)
            let c = g.creatures.get_mut(&gob).unwrap();
            c.pos = p.offset(6, 0, 0);
            c.target = None;
            c.move_cd = 100;
            let out = g.act(h, fire.clone());
            assert!(matches!(out, Outcome::Done { .. }), "{:?}", out);
            shots += 1;
        }
        assert!(shots >= 1 && g.bolts(h).len() + shots == 6);
        assert!(g.drills.flights.iter().any(|f| f.at >= t0 && f.from == p), "the bolts were seen to fly");
        let hurt = g.creatures.get(&gob).map_or(true, |c| c.body.hp.iter().sum::<i32>() < hp0);
        assert!(hurt, "iron bolts from a soldier hurt a goblin");
        // The bolts that didn't break lie on the ground, on or past the goblin.
        let lying: Vec<Pos> = g.items.values().filter(|it| it.kind == ItemKind::Bolt && it.carried_by.is_none()).map(|it| it.pos).collect();
        assert!(lying.iter().all(|q| q.y == p.y && q.x >= p.x + 6 && q.x <= p.x + 9), "{:?}", lying);
    }

    /// One bout: the hero bumps into the foe every turn until one is down
    /// (or they've fought 400 turns). Whether the hero won.
    fn bout(seed: u64, s: Species, armed: bool) -> bool {
        let mut a = field(seed);
        let v = a.visit.as_mut().unwrap();
        let (g, h) = (&mut v.g, v.hero);
        g.dwarves[h].held = None;
        g.dwarves[h].fight_xp = 600;
        if armed {
            let axe = give(g, ItemKind::Axe, Mat::Iron);
            g.dwarves[h].held = Some(axe);
            let mail = give(g, ItemKind::Breastplate, Mat::Iron);
            g.dwarves[h].armor = vec![mail];
        }
        let p = g.dwarves[h].pos;
        let f = foe(g, s, p.offset(1, 0, 0));
        for _ in 0..400 {
            let Some(fp) = g.creatures.get(&f).filter(|c| c.alive).map(|c| c.pos) else { break };
            let me = g.dwarves[h].pos;
            if g.act(h, Action::Move((fp.x - me.x).signum(), (fp.y - me.y).signum())) == Outcome::Died {
                break;
            }
        }
        g.dwarves[h].alive && !g.creatures.get(&f).map_or(false, |c| c.alive)
    }

    #[test]
    fn bare_hands_lose_to_a_bear_iron_wins_against_a_goblin() {
        let bear = (0..50).filter(|&s| bout(7000 + s, Species::Bear, false)).count();
        let goblin = (0..50).filter(|&s| bout(8000 + s, Species::Goblin, true)).count();
        assert!(bear <= 5, "bare hands beat a bear {} times in 50", bear);
        assert!(goblin >= 35, "an iron axe and mail beat a goblin only {} times in 50", goblin);
    }
}
