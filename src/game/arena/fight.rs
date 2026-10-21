//! The arena's own loop: each fighter picks the nearest foe still on their
//! feet, closes in (a free tile beside them, round anyone in the way) and
//! fights with the game's own blows (`strike`, `aftermath`); marksdwarves
//! shoot, and step back to reload. Bleeding, fainting, finishing blows on
//! the fallen, severed parts and blood are the siege's. A bout ends when
//! one side has nobody left standing.

use super::super::combat::{adjacent, cap, dist2, Fighter};
use super::super::*;
use super::fields::{self, Z};
use super::{Field, Kin, Matchup, Recruit};
use crate::body::Body;
use crate::creature::Species;
use crate::world::DOOR_NONE;

/// After this many ticks (four days) a bout is called a draw.
pub const LIMIT: u64 = DAY * 4;
/// How far a crossbow reaches.
const RANGE: i32 = 12;
/// Closer than this, a reloading marksdwarf steps back.
const KEEP_OFF: i32 = 3;
/// Blows a beast needs to batter a door down.
const DOOR_BLOWS: u32 = 14;

/// One fighter: who they are, which side, and how they've done.
#[derive(Clone, Debug)]
pub struct Warrior {
    /// The dwarf's or creature's id in the arena's game.
    pub id: u32,
    pub side: usize,
    pub kin: Kin,
    pub name: String,
    /// Index into the game's dwarves (dwarves only).
    slot: usize,
    /// Who they killed.
    pub kills: Vec<String>,
    /// Blows landed that drew blood or broke something.
    pub hits: u32,
    /// Such blows taken.
    pub struck: u32,
    pub shots: u32,
    /// How they fell, if they did.
    pub fate: Option<String>,
    pub training: u32,
    pub ammo: u32,
    pub ammo_mat: Mat,
    /// The last to wound them (credited if they bleed out).
    last_hit: Option<usize>,
    target: Option<usize>,
    path: Vec<Pos>,
    goal: Option<Pos>,
    think_cd: u32,
    move_cd: u32,
    attack_cd: u32,
    bash: u32,
}

/// How a bout ended.
#[derive(Clone, Debug, PartialEq)]
pub struct Outcome {
    /// The side left standing (None: a draw).
    pub winner: Option<usize>,
    pub ticks: u64,
}

/// A bout under way (or over), in a little game of its own.
pub struct Battle {
    pub g: Game,
    pub warriors: Vec<Warrior>,
    pub field: Field,
    pub name: String,
    /// Side names as the log tells them ("the dwarves").
    pub sides: [String; 2],
    pub outcome: Option<Outcome>,
    /// Who drew first blood.
    pub first_blood: Option<usize>,
    /// Where the last telling blow landed (for the camera).
    pub hot: Option<Pos>,
    pub seed: u64,
}

impl Battle {
    /// A matchup's bout with the given seed.
    pub fn new(m: &Matchup, seed: u64) -> Battle {
        let recruits = [m.sides[0].recruits(), m.sides[1].recruits()];
        Battle::with_recruits(&m.name, m.field, recruits, seed)
    }

    /// Fighters as given, one by one (the overseer's war-game sends its own soldiers).
    pub fn with_recruits(name: &str, field: Field, recruits: [Vec<Recruit>; 2], seed: u64) -> Battle {
        let (world, starts) = fields::build(field, seed);
        let mid = Pos::new(fields::W / 2, fields::H / 2, Z);
        let mut g = Game::from_world(world, seed, Overworld::empty(), (0, 0), mid);
        g.fortress_name = name.to_string();
        g.weather = Weather::Clear;
        let kin = |s: usize| recruits[s].first().map_or(Kin::Dwarf, |r| r.kin);
        let sides = if kin(0) == kin(1) {
            [format!("the western {}", kin(0).plural()), format!("the eastern {}", kin(1).plural())]
        } else {
            [format!("the {}", kin(0).plural()), format!("the {}", kin(1).plural())]
        };
        let mut b = Battle { g, warriors: Vec::new(), field, name: name.to_string(), sides, outcome: None, first_blood: None, hot: None, seed };
        for (side, list) in recruits.iter().enumerate() {
            let mut spots = starts[side].iter();
            let mut names: Vec<String> = Vec::new();
            for r in list {
                let Some(&p) = spots.next() else { break };
                b.enlist(side, r, p, &mut names);
            }
        }
        let text = format!("{}! {} against {}, on {}.", b.name, cap(&b.sides[0]), b.sides[1], field.name());
        b.g.msg(&text, (255, 215, 90));
        b
    }

    fn enlist(&mut self, side: usize, r: &Recruit, p: Pos, names: &mut Vec<String>) {
        let g = &mut self.g;
        let gear = |g: &mut Game, owner: u32| -> (Option<u32>, Vec<u32>) {
            let held = r.weapon.map(|(k, m)| g.spawn_item(k, m, p));
            let armour: Vec<u32> = r.armour.iter().map(|&(k, m)| g.spawn_item(k, m, p)).collect();
            for id in held.iter().chain(armour.iter()) {
                if let Some(it) = g.items.get_mut(id) {
                    it.carried_by = Some(owner);
                }
            }
            (held, armour)
        };
        let (id, slot, name) = match r.kin {
            Kin::Dwarf => {
                let id = g.new_id();
                let mut d = crate::dwarf::new_dwarf(id, p, &mut g.rng, &[]);
                if let Some(n) = &r.name {
                    d.name = n.clone();
                }
                d.body = Body::new(1.0);
                d.fight_xp = r.fight_xp;
                d.soldier = true;
                let (held, armour) = gear(g, id);
                d.held = held;
                d.armor = armour;
                let name = d.called();
                g.dwarves.push(d);
                (id, g.dwarves.len() - 1, name)
            }
            Kin::Beast(s) => {
                let id = g.spawn_creature(s, p, u64::MAX);
                let (held, armour) = gear(g, id);
                let name = r.name.clone().unwrap_or_else(|| beast_name(&mut g.rng, s, names));
                let c = g.creatures.get_mut(&id).unwrap();
                c.weapon = held;
                c.armor = armour;
                c.label = Some(name.clone());
                if r.big {
                    c.body = Body::new(s.info().scale * 1.3);
                }
                (id, 0, name)
            }
        };
        names.push(name.clone());
        self.warriors.push(Warrior {
            id,
            side,
            kin: r.kin,
            name,
            slot,
            kills: Vec::new(),
            hits: 0,
            struck: 0,
            shots: 0,
            fate: None,
            training: r.training,
            ammo: r.bolts,
            ammo_mat: r.bolt_mat,
            last_hit: None,
            target: None,
            path: Vec::new(),
            goal: None,
            think_cd: (self.warriors.len() % 10) as u32,
            move_cd: 0,
            attack_cd: 0,
            bash: 0,
        });
    }

    // ------------------------------------------------------------ the fighters

    pub fn pos(&self, i: usize) -> Pos {
        let w = &self.warriors[i];
        match w.kin {
            Kin::Dwarf => self.g.dwarves[w.slot].pos,
            Kin::Beast(_) => self.g.creatures.get(&w.id).map_or(Pos::new(0, 0, Z), |c| c.pos),
        }
    }

    fn set_pos(&mut self, i: usize, p: Pos) {
        let w = &self.warriors[i];
        match w.kin {
            Kin::Dwarf => self.g.dwarves[w.slot].pos = p,
            Kin::Beast(_) => {
                if let Some(c) = self.g.creatures.get_mut(&w.id) {
                    c.pos = p;
                }
            }
        }
    }

    pub fn body(&self, i: usize) -> &Body {
        let w = &self.warriors[i];
        match w.kin {
            Kin::Dwarf => &self.g.dwarves[w.slot].body,
            Kin::Beast(_) => &self.g.creatures[&w.id].body,
        }
    }

    fn body_mut(&mut self, i: usize) -> &mut Body {
        let w = &self.warriors[i];
        match w.kin {
            Kin::Dwarf => &mut self.g.dwarves[w.slot].body,
            Kin::Beast(_) => &mut self.g.creatures.get_mut(&w.id).unwrap().body,
        }
    }

    /// What they hold.
    pub fn held(&self, i: usize) -> Option<u32> {
        let w = &self.warriors[i];
        match w.kin {
            Kin::Dwarf => self.g.dwarves[w.slot].held,
            Kin::Beast(_) => self.g.creatures[&w.id].weapon,
        }
    }

    /// What they wear.
    pub fn worn(&self, i: usize) -> Vec<u32> {
        let w = &self.warriors[i];
        match w.kin {
            Kin::Dwarf => self.g.dwarves[w.slot].armor.clone(),
            Kin::Beast(_) => self.g.creatures[&w.id].armor.clone(),
        }
    }

    fn take_held(&mut self, i: usize) -> Option<u32> {
        let w = &self.warriors[i];
        match w.kin {
            Kin::Dwarf => self.g.dwarves[w.slot].held.take(),
            Kin::Beast(_) => self.g.creatures.get_mut(&w.id).and_then(|c| c.weapon.take()),
        }
    }

    /// Their blood, for the stains (None: a dwarf's).
    fn blood(&self, i: usize) -> Option<Species> {
        match self.warriors[i].kin {
            Kin::Dwarf => None,
            Kin::Beast(s) => Some(s),
        }
    }

    /// Alive, whether or not on their feet.
    pub fn alive(&self, i: usize) -> bool {
        self.warriors[i].fate.is_none() && !self.body(i).dead
    }

    /// Alive and on their feet.
    pub fn standing(&self, i: usize) -> bool {
        self.alive(i) && !self.body(i).unconscious()
    }

    /// How many of a side are standing, down, and dead.
    pub fn counts(&self, side: usize) -> (usize, usize, usize) {
        let mut c = (0, 0, 0);
        for i in (0..self.warriors.len()).filter(|&i| self.warriors[i].side == side) {
            if self.standing(i) {
                c.0 += 1;
            } else if self.alive(i) {
                c.1 += 1;
            } else {
                c.2 += 1;
            }
        }
        c
    }

    /// Their blows as the game's combat sees them (a creature's training
    /// adds to its skill).
    pub(in crate::game) fn fighter(&self, i: usize) -> Fighter {
        self.fighter_of(i)
    }

    /// A fighter's skill as their blows use it.
    pub fn skill(&self, i: usize) -> u32 {
        self.fighter_of(i).skill
    }

    fn fighter_of(&self, i: usize) -> Fighter {
        let w = &self.warriors[i];
        match w.kin {
            Kin::Dwarf => self.g.dwarf_fighter(&self.g.dwarves[w.slot]),
            Kin::Beast(_) => {
                let mut f = self.g.creature_fighter(&self.g.creatures[&w.id]);
                f.skill += w.training;
                f
            }
        }
    }

    fn move_delay(&self, i: usize) -> u32 {
        let limp = self.body(i).limp();
        match self.warriors[i].kin {
            Kin::Dwarf => MOVE_DELAY + limp,
            Kin::Beast(s) => s.info().move_delay + limp,
        }
    }

    fn attack_delay(&self, i: usize) -> u32 {
        let limp = self.body(i).limp();
        match self.warriors[i].kin {
            // As `dwarf_attack` has it.
            Kin::Dwarf => 9 + limp,
            Kin::Beast(s) => s.info().attack_delay + limp,
        }
    }

    /// A standing fighter (other than `but`) on this tile.
    fn occupied(&self, p: Pos, but: usize) -> bool {
        (0..self.warriors.len()).any(|j| j != but && self.standing(j) && self.pos(j) == p)
    }

    fn can_shoot(&self, i: usize) -> bool {
        self.warriors[i].ammo > 0 && self.body(i).can_hold() && self.held(i).and_then(|id| self.g.items.get(&id)).map_or(false, |it| it.kind == ItemKind::Crossbow)
    }

    // ------------------------------------------------------------ the loop

    /// One tick of the bout.
    pub fn step(&mut self) {
        if self.outcome.is_some() {
            return;
        }
        self.g.tick += 1;
        let tick = self.g.tick;
        let bp = self.g.bleed_params();
        for i in 0..self.warriors.len() {
            if !self.alive(i) {
                continue;
            }
            if tick % 10 == 0 {
                // Fighting, so their wounds clot slowly.
                let body = self.body_mut(i);
                body.tick_with(&bp, true);
                let (rate, dead) = (body.bleed_rate(), body.dead);
                let (p, who, id) = (self.pos(i), self.blood(i), self.warriors[i].id);
                self.g.walk_gore(id, p, who, rate);
                if dead {
                    self.die(i, None, None);
                    continue;
                }
            }
            if self.standing(i) {
                self.act(i);
            }
        }
        self.check_end();
    }

    /// Fight it out to the end (headless).
    pub fn run(&mut self) -> &Outcome {
        while self.outcome.is_none() {
            self.step();
        }
        self.outcome.as_ref().unwrap()
    }

    fn check_end(&mut self) {
        let up = [self.counts(0).0, self.counts(1).0];
        let tick = self.g.tick;
        let winner = match up {
            [0, 0] => None,
            [_, 0] => Some(0),
            [0, _] => Some(1),
            _ if tick >= LIMIT => None,
            _ => return,
        };
        let text = match winner {
            Some(s) => {
                let (n, ..) = self.counts(s);
                format!("{} win{}! {} still on their feet.", cap(&self.sides[s]), if self.sides[s].ends_with('s') { "" } else { "s" }, n)
            }
            None if up == [0, 0] => "Nobody is left standing. A draw!".to_string(),
            None => "Neither side can finish the other. The bout is called a draw.".to_string(),
        };
        self.g.msg(&text, (255, 215, 90));
        self.outcome = Some(Outcome { winner, ticks: tick });
    }

    /// The nearest foe still standing (or, with none, the nearest down).
    fn pick_target(&self, i: usize) -> Option<usize> {
        let (side, p) = (self.warriors[i].side, self.pos(i));
        (0..self.warriors.len())
            .filter(|&j| self.warriors[j].side != side && self.alive(j))
            .min_by_key(|&j| (!self.standing(j), dist2(self.pos(j), p), j))
    }

    fn act(&mut self, i: usize) {
        {
            let w = &mut self.warriors[i];
            w.move_cd = w.move_cd.saturating_sub(1);
            w.attack_cd = w.attack_cd.saturating_sub(1);
            w.think_cd = w.think_cd.saturating_sub(1);
        }
        let stale = self.warriors[i].target.map_or(true, |t| !self.standing(t));
        if stale || self.warriors[i].think_cd == 0 {
            self.warriors[i].think_cd = 10;
            self.warriors[i].target = self.pick_target(i);
        }
        let Some(t) = self.warriors[i].target else { return };
        let (p, tp) = (self.pos(i), self.pos(t));
        // A marksdwarf with bolts shoots, and steps back to reload.
        if self.can_shoot(i) && gap(p, tp) <= RANGE && self.g.clear_shot(p, tp) {
            if self.warriors[i].attack_cd == 0 {
                self.shoot(i, t);
            } else if gap(p, tp) < KEEP_OFF {
                self.back_off(i);
            }
            return;
        }
        // A foe on their feet beside them is fought; one lying beside them
        // is finished off when nobody else is near.
        let side = self.warriors[i].side;
        let beside = |b: &Battle, up: bool| (0..b.warriors.len()).find(|&j| b.warriors[j].side != side && b.alive(j) && b.standing(j) == up && adjacent(b.pos(j), p));
        let foe = beside(self, true).or_else(|| {
            let near = (0..self.warriors.len()).any(|j| self.warriors[j].side != side && self.standing(j) && gap(self.pos(j), p) <= 3);
            if near {
                None
            } else {
                beside(self, false)
            }
        });
        if let Some(f) = foe {
            if self.warriors[i].attack_cd == 0 {
                self.warriors[i].attack_cd = self.attack_delay(i);
                self.melee(i, f);
            }
            return;
        }
        self.advance(i, tp);
    }

    /// A step nearer `to`: along a path (made afresh when the quarry has
    /// moved), round anyone standing in the way, through a door (a beast
    /// batters it down).
    fn advance(&mut self, i: usize, to: Pos) {
        if self.warriors[i].move_cd > 0 {
            return;
        }
        let p = self.pos(i);
        let w = &self.warriors[i];
        if w.path.is_empty() || w.goal.map_or(true, |g| gap(g, to) > 1) {
            let found = path::astar(&self.g.world, p, to, 6000, Mover::Goblin);
            let w = &mut self.warriors[i];
            w.path = found.map(|v| v.into_iter().rev().collect()).unwrap_or_default();
            w.goal = Some(to);
            if w.path.is_empty() {
                w.move_cd = 10;
                return;
            }
        }
        let Some(&next) = self.warriors[i].path.last() else { return };
        if !adjacent(next, p) {
            self.warriors[i].path.clear();
            return;
        }
        let mut step = next;
        if self.occupied(next, i) {
            // Round them: a free tile that still brings them nearer.
            let mut nb = Vec::new();
            self.g.world.neighbors(p, &mut nb, Mover::Goblin);
            let here = dist2(p, to);
            let round = nb.iter().map(|&(q, _)| q).filter(|&q| !self.occupied(q, i) && dist2(q, to) < here).min_by_key(|&q| (dist2(q, to), q.x, q.y));
            match round {
                Some(q) => {
                    step = q;
                    self.warriors[i].path.clear();
                }
                None => {
                    self.warriors[i].move_cd = 2;
                    return;
                }
            }
        }
        // Beasts can't open doors: they break them.
        let beast = matches!(self.warriors[i].kin, Kin::Beast(s) if s.info().mover == Mover::Beast);
        if beast && self.g.world.t(step).door != DOOR_NONE {
            let w = &mut self.warriors[i];
            w.bash += 1;
            w.move_cd = 9;
            if w.bash >= DOOR_BLOWS {
                w.bash = 0;
                self.g.world.t_mut(step).door = DOOR_NONE;
                let text = format!("{} batters the door down!", cap(&self.warriors[i].name));
                self.g.msg(&text, (255, 150, 90));
                self.hot = Some(step);
            }
            return;
        }
        if step == next {
            self.warriors[i].path.pop();
        }
        self.set_pos(i, step);
        self.warriors[i].move_cd = self.move_delay(i);
    }

    /// Step away from the foes nearest (to reload), keeping to the level.
    fn back_off(&mut self, i: usize) {
        if self.warriors[i].move_cd > 0 {
            return;
        }
        let (side, p) = (self.warriors[i].side, self.pos(i));
        let foes: Vec<Pos> = (0..self.warriors.len()).filter(|&j| self.warriors[j].side != side && self.standing(j) && gap(self.pos(j), p) <= 7).map(|j| self.pos(j)).collect();
        let score = |q: Pos| foes.iter().map(|&f| dist2(f, q)).min().unwrap_or(0);
        let mut nb = Vec::new();
        self.g.world.neighbors(p, &mut nb, Mover::Dwarf);
        let best = nb.iter().map(|&(q, _)| q).filter(|&q| q.z == p.z && !self.occupied(q, i) && !fields::chasm(&self.g.world, q)).max_by_key(|&q| (score(q), q.x, q.y));
        if let Some(q) = best.filter(|&q| score(q) > score(p)) {
            self.set_pos(i, q);
            self.warriors[i].path.clear();
            self.warriors[i].move_cd = self.move_delay(i);
            self.g.count("arena.backoff", 1);
        }
    }

    fn melee(&mut self, a: usize, d: usize) {
        let att = self.fighter(a);
        let def = self.fighter(d);
        self.blow(a, d, &att, &def);
    }

    /// A bolt, as `try_shoot` looses one.
    fn shoot(&mut self, a: usize, d: usize) {
        let base = self.fighter(a);
        let bow = self.held(a).and_then(|id| self.g.items.get(&id)).map(|it| it.mat);
        let metal = |m: Option<Mat>| matches!(m, Some(Mat::Copper | Mat::Bronze | Mat::Iron | Mat::Steel));
        let draw = if metal(bow) { self.g.knob("bolt.draw", 1.0) as u32 } else { 0 };
        let w = &mut self.warriors[a];
        w.ammo -= 1;
        w.shots += 1;
        w.attack_cd = 14;
        let dmg = Game::bolt_damage(Some(w.ammo_mat)) * self.g.knob("bolt.mult", 1.0) as f32;
        let att = Fighter { name: base.name, skill: base.skill + draw, dmg, verb: "shoots", edged: true, helm: 0, mail: 0, shield: false, size: 1.0, teeth: true, weapon: None, held: None };
        let def = self.fighter(d);
        self.g.count("arena.bolts", 1);
        if self.warriors[a].ammo == 0 {
            let text = format!("{} has shot their last bolt.", cap(&self.warriors[a].name));
            self.g.msg(&text, (200, 200, 160));
        }
        self.blow(a, d, &att, &def);
    }

    /// What one blow does, and all that follows from it.
    fn blow(&mut self, a: usize, d: usize, att: &Fighter, def: &Fighter) {
        let mut body = self.body(d).clone();
        let (hp0, bleed0): (i32, i32) = (body.hp.iter().sum(), body.bleeding);
        let blow = self.g.strike(att, def, &mut body);
        let hurt = body.hp.iter().sum::<i32>() < hp0 || body.bleeding > bleed0;
        let (from, at) = (self.pos(a), self.pos(d));
        if hurt {
            self.warriors[a].hits += 1;
            self.warriors[d].struck += 1;
            self.warriors[d].last_hit = Some(a);
            if self.first_blood.is_none() {
                self.first_blood = Some(a);
                let text = format!("First blood to {}! {} draws it.", self.sides[self.warriors[a].side], cap(&self.warriors[a].name));
                self.g.msg(&text, (255, 215, 90));
            }
        }
        if blow.notable {
            if let Some(m) = &blow.msg {
                let c = SIDE_COLORS[self.warriors[a].side];
                self.g.msg(m, c);
            }
            self.hot = Some(at);
        }
        let (owner, seen) = match self.warriors[d].kin {
            Kin::Dwarf => {
                let n = self.warriors[d].name.split(' ').next().unwrap_or("someone").to_string();
                (n.clone(), n)
            }
            Kin::Beast(_) => {
                let c = &self.g.creatures[&self.warriors[d].id];
                (super::super::gore::owner_of_creature(c), super::super::melee::a_or_name(c))
            }
        };
        let knocked = self.g.aftermath(&blow, from, at, &owner, &seen, self.blood(d), att.size >= 2.0);
        let dead = body.dead;
        *self.body_mut(d) = body;
        if let Some(q) = knocked {
            if !self.occupied(q, d) {
                self.set_pos(d, q);
            }
            let w = &mut self.warriors[d];
            w.path.clear();
            w.move_cd += 12;
            w.attack_cd += 12;
        } else if blow.sprawl && !dead {
            // Nowhere to land but the chasm.
            let over = at.offset((at.x - from.x).signum(), (at.y - from.y).signum(), 0);
            if fields::chasm(&self.g.world, over) {
                self.set_pos(d, over);
                let text = format!("{} is sent sprawling over the edge, and falls into the chasm!", cap(&self.warriors[d].name));
                self.g.msg(&text, (255, 120, 90));
                self.hot = Some(over);
                self.g.count("arena.fell", 1);
                self.die(d, Some(a), Some(format!("knocked into the chasm by {}", self.warriors[a].name)));
                return;
            }
        }
        if blow.disarmed && !dead {
            if let Some(it) = self.take_held(d) {
                self.g.drop_item(it, at);
            }
        }
        if dead {
            self.die(d, Some(a), None);
        }
    }

    /// They're dead: what they carried falls, a body and a pool of blood
    /// are left, and the kill goes to whoever struck it (or last wounded
    /// them, if they bled out).
    fn die(&mut self, i: usize, by: Option<usize>, how: Option<String>) {
        if self.warriors[i].fate.is_some() {
            return;
        }
        let p = self.pos(i);
        let fell = fields::chasm(&self.g.world, p);
        let killer = by.or(self.warriors[i].last_hit);
        let name = self.warriors[i].name.clone();
        let fate = how.unwrap_or_else(|| match (by, killer) {
            (Some(k), _) => format!("slain by {}", self.warriors[k].name),
            (None, Some(k)) => format!("bled to death (wounded by {})", self.warriors[k].name),
            (None, None) => "bled to death".to_string(),
        });
        self.body_mut(i).dead = true;
        let items: Vec<u32> = self.held(i).into_iter().chain(self.worn(i)).collect();
        let w = &self.warriors[i];
        let big = match w.kin {
            Kin::Dwarf => {
                let d = &mut self.g.dwarves[w.slot];
                d.alive = false;
                d.held = None;
                d.armor.clear();
                false
            }
            Kin::Beast(_) => {
                let c = self.g.creatures.get_mut(&w.id).unwrap();
                c.alive = false;
                c.weapon = None;
                c.armor.clear();
                c.body.max_blood >= 150
            }
        };
        if fell {
            // Gone into the dark, and all they carried with them.
            for id in items {
                self.g.items.remove(&id);
            }
        } else {
            for id in items {
                self.g.drop_item(id, p);
            }
            let who = self.blood(i);
            self.g.death_pool(p, who, big);
            self.g.mark_battlefield(p);
            let person = matches!(self.warriors[i].kin, Kin::Dwarf) || matches!(self.warriors[i].kin, Kin::Beast(s) if s.person());
            let id = self.g.spawn_item(if person { ItemKind::Corpse } else { ItemKind::Carcass }, Mat::Flesh, p);
            if let Some(it) = self.g.items.get_mut(&id) {
                it.label = Some(name.clone());
            }
            let text = match by {
                Some(k) => format!("{} has been slain by {}.", cap(&name), self.warriors[k].name),
                None => format!("{} has bled to death.", cap(&name)),
            };
            self.g.msg(&text, (255, 170, 120));
        }
        if let Some(k) = killer {
            self.warriors[k].kills.push(name);
            if let Kin::Dwarf = self.warriors[k].kin {
                let slot = self.warriors[k].slot;
                self.g.dwarves[slot].kills += 1;
            }
        }
        self.warriors[i].fate = Some(fate);
        self.g.count("arena.deaths", 1);
    }

    // ------------------------------------------------------------ the tale

    /// Parts cut off, across everyone.
    pub fn severed(&self) -> u32 {
        self.g.annals.counters.get("gore.severed").copied().unwrap_or(0)
    }

    /// The one with most kills (then most telling blows), if anyone landed one.
    pub fn best(&self) -> Option<usize> {
        (0..self.warriors.len()).filter(|&i| self.warriors[i].hits > 0).max_by_key(|&i| (self.warriors[i].kills.len(), self.warriors[i].hits, std::cmp::Reverse(i)))
    }

    /// Who fell to them, as a line ("Snaga, Uzg").
    pub fn kill_list(&self, i: usize) -> String {
        self.warriors[i].kills.join(", ")
    }

    /// Their wounds in a few words ("left arm gone, 3 wounds, bleeding").
    pub fn wounds(&self, i: usize) -> String {
        use crate::body::PARTS;
        let b = self.body(i);
        let mut v = Vec::new();
        let lost: Vec<&str> = PARTS.iter().filter(|p| b.lost[p.idx()]).map(|p| p.name()).collect();
        if !lost.is_empty() {
            v.push(format!("lost {}", lost.join(", ")));
        }
        let hurt = PARTS.iter().filter(|p| !b.lost[p.idx()] && b.hp[p.idx()] < b.max[p.idx()]).count();
        if hurt > 0 {
            v.push(format!("{} wound{}", hurt, if hurt == 1 { "" } else { "s" }));
        }
        if b.bleeding > 0 && !b.dead {
            v.push("bleeding".into());
        }
        if v.is_empty() {
            "unhurt".into()
        } else {
            v.join(", ")
        }
    }
}

/// The colours the sides are told in (west blue, east red).
pub const SIDE_COLORS: [(u8, u8, u8); 2] = [(140, 190, 255), (255, 140, 120)];

fn gap(a: Pos, b: Pos) -> i32 {
    if a.z != b.z {
        return i32::MAX / 2;
    }
    (a.x - b.x).abs().max((a.y - b.y).abs())
}

/// "Snaga Uzbad" for a person; for a beast a word to know it by ("the
/// scarred wolf"), each different on a side.
fn beast_name(rng: &mut Rng, s: Species, taken: &[String]) -> String {
    use crate::overworld::{person_name, Race};
    let race = match s {
        Species::Goblin => Some(Race::Goblin),
        Species::Elf => Some(Race::Elf),
        Species::Raider => Some(Race::Dwarf),
        Species::Human | Species::Merchant | Species::Visitor => Some(Race::Human),
        _ => None,
    };
    if let Some(r) = race {
        for _ in 0..20 {
            let n = person_name(rng, r);
            if !taken.contains(&n) {
                return n;
            }
        }
    }
    const MARKS: [&str; 16] = ["scarred", "grey", "old", "lean", "one-eyed", "black", "great", "pale", "limping", "young", "red", "hungry", "torn-eared", "huge", "quiet", "mangy"];
    let start = rng.range(0, MARKS.len() as i32) as usize;
    for k in 0..MARKS.len() {
        let n = format!("the {} {}", MARKS[(start + k) % MARKS.len()], s.info().name);
        if !taken.contains(&n) {
            return n;
        }
    }
    format!("the {} #{}", s.info().name, taken.len() + 1)
}

/// What many bouts of one matchup came to.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tally {
    pub runs: u32,
    pub wins: [u32; 2],
    pub draws: u32,
    pub ticks: u64,
    /// Alive at the end, and dead, summed over the bouts.
    pub alive: [u32; 2],
    pub dead: [u32; 2],
    pub severed: u32,
    pub fell: u32,
    /// The deadliest single fighter seen: (name, side, kills).
    pub champion: Option<(String, usize, usize)>,
}

impl Tally {
    pub fn add(&mut self, b: &Battle) {
        let Some(o) = &b.outcome else { return };
        self.runs += 1;
        match o.winner {
            Some(s) => self.wins[s] += 1,
            None => self.draws += 1,
        }
        self.ticks += o.ticks;
        for s in 0..2 {
            let (up, down, dead) = b.counts(s);
            self.alive[s] += (up + down) as u32;
            self.dead[s] += dead as u32;
        }
        self.severed += b.severed();
        self.fell += b.g.annals.counters.get("arena.fell").copied().unwrap_or(0);
        if let Some(i) = b.best() {
            let k = b.warriors[i].kills.len();
            if k > 0 && self.champion.as_ref().map_or(true, |c| k > c.2) {
                self.champion = Some((b.warriors[i].name.clone(), b.warriors[i].side, k));
            }
        }
    }

    /// A side's share of wins, in percent.
    pub fn pct(&self, side: usize) -> u32 {
        self.wins[side] * 100 / self.runs.max(1)
    }
}

/// `n` bouts of a matchup, one after another from its seed (headless).
#[cfg(test)]
pub fn run_many(m: &Matchup, n: u32) -> Tally {
    let mut t = Tally::default();
    for k in 0..n {
        let mut b = Battle::new(m, m.seed.wrapping_add(k as u64));
        b.run();
        t.add(&b);
    }
    t
}

#[cfg(test)]
mod tests {
    use super::super::{presets, SideSpec};
    pub use super::run_many;
    use super::*;

    #[test]
    fn ten_spears_against_six_goblins_is_fought_to_a_winner() {
        let m = Matchup::default();
        let mut b = Battle::new(&m, 7);
        assert_eq!(b.warriors.len(), 16);
        assert_eq!(b.counts(0), (10, 0, 0));
        let o = b.run().clone();
        assert!(o.winner.is_some(), "a draw after {} ticks", o.ticks);
        assert!(o.ticks < LIMIT);
        let w = o.winner.unwrap();
        assert_eq!(b.counts(1 - w).0, 0, "the losers still have someone standing");
        assert!(b.counts(w).0 > 0);
        // It was fought with the game's blows: blood on the ground, bodies
        // where they fell, and the log tells of it.
        assert!(!b.g.gore.stains.is_empty(), "no blood was spilled");
        let dead: usize = b.counts(0).2 + b.counts(1).2;
        assert!(dead > 0);
        assert!(b.g.items.values().filter(|it| matches!(it.kind, ItemKind::Corpse | ItemKind::Carcass)).count() >= dead);
        assert!(b.g.log.iter().any(|l| l.text.contains("First blood")));
        assert!(b.g.log.iter().any(|l| l.text.contains(" in the ")), "no blows told");
        let kills: usize = b.warriors.iter().map(|w| w.kills.len()).sum();
        assert!(kills >= dead.saturating_sub(1), "{} kills for {} dead", kills, dead);
        // Nothing of the fortress ran: nobody ate, drank or worked.
        assert!(b.g.dwarves.iter().all(|d| d.task == Task::Idle));
    }

    /// The arena test in combat.rs (one dwarf against one goblin, blow by
    /// blow) gives a soldier in iron 80% against a goblin with a bronze
    /// sword and helm, 30% against one in a full iron kit, and a recruit
    /// with a copper axe 35% against a goblin's copper spear. Fought out
    /// here on open ground, walking up to each other first, they come out
    /// the same.
    #[test]
    fn rerun_win_rates_match_the_duel_table() {
        let soldier = SideSpec::new(Kin::Dwarf, 1).armed(ItemKind::Axe, Mat::Iron).kitted(5, Mat::Iron);
        let gob = |w, m| SideSpec::new(Kin::Beast(Species::Goblin), 1).armed(w, m);
        let cases = [
            (soldier.clone(), gob(ItemKind::Sword, Mat::Bronze).kitted(1, Mat::Bronze), 80),
            (soldier.clone(), gob(ItemKind::Axe, Mat::Iron).kitted(5, Mat::Iron), 30),
            (SideSpec::new(Kin::Dwarf, 1).armed(ItemKind::Axe, Mat::Copper), gob(ItemKind::Spear, Mat::Copper), 35),
        ];
        for (a, z, want) in cases {
            let m = Matchup { name: "duel".into(), sides: [a, z], field: Field::Open, seed: 1000 };
            let t = run_many(&m, 300);
            eprintln!("{}: {}% (table {}%), draws {}, {} ticks a bout", m.describe(), t.pct(0), want, t.draws, t.ticks / t.runs as u64);
            assert_eq!(t.runs, 300);
            assert!((t.pct(0) as i32 - want).abs() <= 9, "{}: {}% against the table's {}%", m.describe(), t.pct(0), want);
            assert!(t.draws <= 15, "{} draws", t.draws);
        }
    }

    #[test]
    fn every_preset_ends_and_the_bridge_can_throw_someone_off() {
        let mut fell = 0;
        for m in presets() {
            let t = run_many(&m, 3);
            eprintln!("{}: {:?}", m.name, t);
            assert_eq!(t.runs, 3);
            assert!(t.draws < 3, "{} never finished", m.name);
            fell += t.fell;
        }
        // A bear going after a dwarf who stays in the far hall has to break
        // the door to get at her.
        let m = Matchup { name: "door".into(), sides: [SideSpec::new(Kin::Beast(Species::Bear), 1), SideSpec::new(Kin::Dwarf, 1)], field: Field::Corridor, seed: 3 };
        let mut b = Battle::new(&m, 3);
        let home = b.pos(1);
        for _ in 0..LIMIT {
            if b.outcome.is_some() {
                break;
            }
            b.set_pos(1, home);
            b.step();
        }
        assert!(b.g.log.iter().any(|l| l.text.contains("batters the door down")), "the bear never broke the door");
        assert!(b.g.world.t(Pos::new(fields::W / 2, fields::H / 2, Z)).door == DOOR_NONE);
        // Big brutes on the bridge sometimes send someone over.
        let bridge = Matchup { name: "brink".into(), sides: [SideSpec::new(Kin::Beast(Species::Megabeast), 1), SideSpec::new(Kin::Dwarf, 12)], field: Field::Bridge, seed: 50 };
        fell += run_many(&bridge, 12).fell;
        assert!(fell > 0, "nobody ever fell off the bridge");
    }

    #[test]
    fn marksdwarves_shoot_and_keep_their_distance() {
        let m = Matchup {
            name: "range".into(),
            sides: [SideSpec::new(Kin::Dwarf, 3).armed(ItemKind::Crossbow, Mat::Iron).skilled(3), SideSpec::new(Kin::Beast(Species::Goblin), 3)],
            field: Field::Yard,
            seed: 9,
        };
        let mut b = Battle::new(&m, 9);
        b.run();
        let shots: u32 = b.warriors.iter().filter(|w| w.side == 0).map(|w| w.shots).sum();
        assert!(shots >= 5, "only {} bolts shot", shots);
        assert!(b.g.log.iter().any(|l| l.text.contains("shoots")), "no bolt was told of");
        assert_eq!(b.outcome.as_ref().unwrap().winner, Some(0), "three marksdwarves lost to three bare-handed goblins");
    }
}
