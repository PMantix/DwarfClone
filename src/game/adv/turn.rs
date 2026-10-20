//! The turn engine: the hero acts, and the world runs for what the act
//! costs in ticks. Turn-based at the player's end, the simulation's ticks
//! underneath: a step costs `MOVE_DELAY` (more with a limp), a blow the
//! hero's attack delay, and creatures move between keys at their own speeds.
//!
//! `adv_step` is the contract: one tick of exactly the systems an adventure
//! wants, one line each (as `schedule.rs` does for the fort). What isn't
//! listed doesn't run: no migrants, sieges, jobs, needs, overseer, or the
//! fall test (`sagas_tick`), which would end a fort that never was.

use super::super::combat::{adjacent, cap, near};
use super::super::*;
use super::clock;
use crate::body::Part;

/// What the hero does.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// A step (into a foe: a blow).
    Move(i32, i32),
    /// Up (+1) or down (-1) a stair or ramp.
    Climb(i32),
    /// Stand a while, so many ticks.
    Wait(u32),
    /// A blow, aimed at a part if given (A6).
    Attack { target: Target, part: Option<Part> },
    /// Shoot (A6).
    Fire { target: Target },
    /// Pick up items here.
    Get(Vec<u32>),
    Drop(Vec<u32>),
    /// Wield a weapon, or put on armour or clothes.
    Wear(u32),
    /// Take off, or put away, into the pack.
    Remove(u32),
    /// Press and bandage wounds (A6).
    Bind,
    /// Rest until healed, interrupted, or a day has passed.
    Rest,
    /// Speak to someone (A7).
    Talk(u32),
}

/// Whom an act is aimed at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Creature(u32),
    Dwarf(u32),
}

/// How an act went.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Done { ticks: u32 },
    /// A long act stopped early, and why.
    Interrupted { ticks: u32, why: String },
    /// Not done (no time passes), and why.
    Refused(String),
    /// The hero is dead.
    Died,
}

/// What a get, drop, wear or remove costs.
const HANDLE_TICKS: u32 = 20;
/// How long an unconscious hero lies before the world is asked again.
const DOWN_TICKS: u32 = 50;
/// The longest rest at a time.
const REST_TICKS: u32 = DAY as u32;

impl Game {
    /// One tick of what an adventure runs (the list is the contract: one
    /// line per system).
    #[rustfmt::skip]
    pub(crate) fn adv_step(&mut self) {
        self.tick += 1;
        let tick = self.tick;
        if tick % 3 == 0 { self.fluids(); }
        if tick % 10 == 5 && self.fire_cells.as_ref().map_or(true, |c| !c.is_empty()) { self.fires(); }
        if tick % gore::SMOKE_TICK == 8 { self.smoke_tick(); }
        if tick % gore::GORE_TICK == 45 { self.gore_tick(); }
        if tick % horrors::HORROR_TICK == 11 { self.horrors_tick(); }
        if tick % 20 == 11 { self.magic_tick(); }
        // The hero's and companions' bodies (and any dwarf folk's).
        self.adv_bodies();
        self.ghosts_tick();
        self.update_creatures();
        // A fallen fort's traps still work.
        self.traps_tick();
        // Companions follow and fight (A9).
        self.company_tick();
    }

    /// Wounds bleed and clot every 10 ticks, and heal a little every 200,
    /// as in the fort (`update_dwarf`) and the arena (`Battle::step`).
    fn adv_bodies(&mut self) {
        let tick = self.tick;
        for i in 0..self.dwarves.len() {
            if !self.dwarves[i].alive {
                continue;
            }
            self.dwarves[i].attack_cd = self.dwarves[i].attack_cd.saturating_sub(1);
            if tick % 10 != 0 {
                continue;
            }
            let bp = self.bleed_params();
            let mut d = std::mem::take(&mut self.dwarves[i]);
            d.body.tick_with(&bp, d.attack_cd > 0);
            let rate = d.body.bleed_rate();
            if rate > 0 {
                self.blood_trail(d.pos, None, rate);
            }
            if d.body.dead {
                self.kill(&mut d, "has bled to death");
            } else if tick % 200 == 0 {
                d.body.heal(1);
            }
            self.dwarves[i] = d;
        }
    }

    /// The hero (`dwarves[hero]`) does `a`; the world runs for its cost,
    /// stopping early on an interrupt if the act is a long one.
    pub(crate) fn act(&mut self, hero: usize, a: Action) -> Outcome {
        if !self.dwarves[hero].alive {
            return Outcome::Died;
        }
        // Out cold: the world runs on until they wake or die.
        if self.dwarves[hero].body.unconscious() {
            return self.adv_run(hero, DOWN_TICKS, false);
        }
        let limp = self.dwarves[hero].body.limp();
        match a {
            Action::Move(dx, dy) => {
                let from = self.dwarves[hero].pos;
                let to = from.offset(dx, dy, 0);
                if let Some(cid) = self.foe_at(to) {
                    return self.act(hero, Action::Attack { target: Target::Creature(cid), part: None });
                }
                if !self.world.in_bounds(to) {
                    return Outcome::Refused("The edge of the land: travel on from here (T).".into());
                }
                // The way on: level, or up or down a ramp beside.
                let mut nb = Vec::new();
                self.world.neighbors(from, &mut nb, Mover::Dwarf);
                let Some(&(next, _)) = nb.iter().find(|(q, _)| q.x == to.x && q.y == to.y) else {
                    return Outcome::Refused("Something is in the way.".into());
                };
                self.dwarves[hero].pos = next;
                self.adv_run(hero, MOVE_DELAY + limp, false)
            }
            Action::Climb(dz) => {
                let from = self.dwarves[hero].pos;
                let mut nb = Vec::new();
                self.world.neighbors(from, &mut nb, Mover::Dwarf);
                let Some(&(next, _)) = nb.iter().find(|(q, _)| q.x == from.x && q.y == from.y && q.z == from.z + dz.signum()) else {
                    return Outcome::Refused(if dz > 0 { "There is no way up here." } else { "There is no way down here." }.into());
                };
                self.dwarves[hero].pos = next;
                self.adv_run(hero, MOVE_DELAY + limp, false)
            }
            Action::Wait(n) => self.adv_run(hero, n.max(1), n > 10),
            Action::Attack { target: Target::Creature(cid), part: _ } => {
                let p = self.dwarves[hero].pos;
                let Some(c) = self.creatures.get(&cid).filter(|c| c.alive) else {
                    return Outcome::Refused("There is nobody there.".into());
                };
                if !adjacent(c.pos, p) {
                    return Outcome::Refused(format!("The {} is out of reach.", c.species.info().name));
                }
                let mut d = std::mem::take(&mut self.dwarves[hero]);
                self.dwarf_attack(&mut d, cid);
                let cost = d.attack_cd.max(1);
                self.dwarves[hero] = d;
                self.adv_run(hero, cost, false)
            }
            Action::Attack { target: Target::Dwarf(_), .. } => Outcome::Refused("You can't bring yourself to (A6).".into()),
            Action::Get(ids) => {
                let (p, me) = (self.dwarves[hero].pos, self.dwarves[hero].id);
                let mut got = 0;
                for id in ids {
                    if let Some(it) = self.items.get_mut(&id).filter(|it| it.pos == p && it.carried_by.is_none()) {
                        it.carried_by = Some(me);
                        it.container = None;
                        it.claimed_by = None;
                        got += 1;
                    }
                }
                if got == 0 {
                    return Outcome::Refused("There is nothing here to take.".into());
                }
                self.adv_run(hero, HANDLE_TICKS, false)
            }
            Action::Drop(ids) => {
                let (p, me) = (self.dwarves[hero].pos, self.dwarves[hero].id);
                let mut dropped = 0;
                for id in ids {
                    if self.items.get(&id).map_or(false, |it| it.carried_by == Some(me)) {
                        let d = &mut self.dwarves[hero];
                        if d.held == Some(id) {
                            d.held = None;
                        }
                        d.armor.retain(|&a| a != id);
                        d.clothes.retain(|&a| a != id);
                        self.drop_item(id, p);
                        dropped += 1;
                    }
                }
                if dropped == 0 {
                    return Outcome::Refused("You have nothing like that.".into());
                }
                self.adv_run(hero, HANDLE_TICKS, false)
            }
            Action::Wear(id) => {
                let me = self.dwarves[hero].id;
                let Some(it) = self.items.get(&id).filter(|it| it.carried_by == Some(me)) else {
                    return Outcome::Refused("You have nothing like that.".into());
                };
                let kind = it.kind;
                let d = &mut self.dwarves[hero];
                if kind.armor_slot().is_some() {
                    if !d.armor.contains(&id) {
                        d.armor.push(id);
                    }
                } else if kind.clothing_slot().is_some() {
                    if !d.clothes.contains(&id) {
                        d.clothes.push(id);
                    }
                } else {
                    d.held = Some(id);
                }
                self.adv_run(hero, HANDLE_TICKS, false)
            }
            Action::Remove(id) => {
                let d = &mut self.dwarves[hero];
                let was = d.held == Some(id) || d.armor.contains(&id) || d.clothes.contains(&id);
                if !was {
                    return Outcome::Refused("You aren't wearing that.".into());
                }
                if d.held == Some(id) {
                    d.held = None;
                }
                d.armor.retain(|&a| a != id);
                d.clothes.retain(|&a| a != id);
                self.adv_run(hero, HANDLE_TICKS, false)
            }
            Action::Rest => {
                if let Some(why) = self.alarm(hero, self.tick) {
                    return Outcome::Refused(format!("You can't rest now: {}", why));
                }
                self.adv_run(hero, REST_TICKS, true)
            }
            Action::Fire { .. } => Outcome::Refused("Shooting comes later (A6).".into()),
            Action::Bind => Outcome::Refused("Binding wounds comes later (A6).".into()),
            Action::Talk(_) => Outcome::Refused("Talk comes later (A7).".into()),
        }
    }

    /// Run the world `ticks` ticks for the hero's act; a long act stops on an
    /// alarm (a rest also stops once the body is whole).
    fn adv_run(&mut self, hero: usize, ticks: u32, long: bool) -> Outcome {
        let start = self.tick;
        let hp0: i32 = self.dwarves[hero].body.hp.iter().sum();
        for n in 1..=ticks {
            self.adv_step();
            if !self.dwarves[hero].alive {
                return Outcome::Died;
            }
            if long && n % 5 == 0 {
                if let Some(why) = self.alarm(hero, start) {
                    self.wound_news(hero, hp0, start);
                    return Outcome::Interrupted { ticks: n, why };
                }
                if ticks == REST_TICKS && self.dwarves[hero].body.healed() {
                    return Outcome::Done { ticks: n };
                }
            }
        }
        self.wound_news(hero, hp0, start);
        Outcome::Done { ticks }
    }

    /// The hero was hurt during an act: say by what (the fort's log tells
    /// only the notable blows).
    fn wound_news(&mut self, hero: usize, hp0: i32, since: u64) {
        let d = &self.dwarves[hero];
        let hp: i32 = d.body.hp.iter().sum();
        if hp >= hp0 {
            return;
        }
        let by = match d.attacker {
            Some((cid, t)) if t >= since => self.creatures.get(&cid).map_or("something".to_string(), |c| format!("the {}", c.species.info().name)),
            _ => "something".to_string(),
        };
        let first = d.name.split(' ').next().unwrap_or("").to_string();
        let how = if d.body.bleeding > 0 { ", and bleeds" } else { "" };
        self.msg(&format!("{} is wounded by {}{}.", first, by, how), (235, 120, 100));
    }

    /// What would make a long action stop: a blow taken since `since_tick`,
    /// or a hostile in sight.
    pub(crate) fn alarm(&self, hero: usize, since_tick: u64) -> Option<String> {
        let d = &self.dwarves[hero];
        if let Some((cid, t)) = d.attacker {
            if t >= since_tick && t > 0 {
                let who = self.creatures.get(&cid).map_or("something".to_string(), |c| format!("the {}", c.species.info().name));
                return Some(format!("{} strikes you!", cap(&who)));
            }
        }
        let seen = self.in_sight(hero);
        let foe = seen.iter().filter_map(|id| self.creatures.get(id)).find(|c| c.hostile() && !c.leaving)?;
        Some(format!("{} comes into view.", cap(&a_or_an(foe.species.info().name))))
    }

    /// How far the hero sees now: far by day under the sky, a few tiles at
    /// night, a torch's reach under rock.
    pub(crate) fn sight_radius(&self, hero: usize) -> i32 {
        let p = self.dwarves[hero].pos;
        let sky = self.world.in_bounds(p) && self.world.outside(p);
        if !sky {
            return 7;
        }
        if clock::light(self.tick, true) >= 128 {
            20
        } else {
            7
        }
    }

    /// Creatures the hero can see: alive, on their level, within the sight
    /// radius, with a clear line to them; nearest first.
    pub(crate) fn in_sight(&self, hero: usize) -> Vec<u32> {
        let p = self.dwarves[hero].pos;
        let r = self.sight_radius(hero);
        let mut v: Vec<(i32, u32)> = self
            .creatures
            .values()
            .filter(|c| c.alive && c.pos.z == p.z && near(c.pos, p, r) && self.clear_shot(p, c.pos))
            .map(|c| ((c.pos.x - p.x).pow(2) + (c.pos.y - p.y).pow(2), c.id))
            .collect();
        v.sort();
        v.into_iter().map(|(_, id)| id).collect()
    }

    /// The tiles the hero sees on their level: within the sight radius,
    /// with a clear line to them (a wall at the line's end is seen).
    pub(crate) fn seen_tiles(&self, hero: usize) -> Sight {
        let p = self.dwarves[hero].pos;
        let r = self.sight_radius(hero);
        let n = (2 * r + 1) as usize;
        let mut cells = vec![false; n * n];
        for dy in -r..=r {
            for dx in -r..=r {
                let q = p.offset(dx, dy, 0);
                if dx * dx + dy * dy <= r * r && self.world.in_bounds(q) && self.clear_shot(p, q) {
                    cells[(dy + r) as usize * n + (dx + r) as usize] = true;
                }
            }
        }
        Sight { at: p, r, cells }
    }

    /// What the hero sees is known from now on (`Tile.revealed`), so the
    /// map remembers it dimly once they look away.
    pub(crate) fn reveal_seen(&mut self, hero: usize) {
        let s = self.seen_tiles(hero);
        for dy in -s.r..=s.r {
            for dx in -s.r..=s.r {
                let q = s.at.offset(dx, dy, 0);
                if s.sees(q) && !self.world.t(q).revealed {
                    self.world.t_mut(q).revealed = true;
                }
            }
        }
    }

    /// A goblin raider come for the hero, a few steps off (for the window's
    /// `--adventure --fight` picture and the view's tests). Its id.
    pub(crate) fn raider_for(&mut self, hero: usize) -> u32 {
        let p = self.dwarves[hero].pos;
        let at = self.spot_near(p.offset(3, 0, 0), 2);
        let id = self.spawn_creature(crate::creature::Species::Goblin, at, DAY * 10);
        if let Some(c) = self.creatures.get_mut(&id) {
            c.siege = true;
            c.target = Some(self.dwarves[hero].id);
        }
        id
    }

    /// A living foe standing on `p`.
    fn foe_at(&self, p: Pos) -> Option<u32> {
        self.creatures.values().find(|c| c.alive && c.pos == p && !c.tame && c.pet_of.is_none()).map(|c| c.id)
    }

    /// Companions follow and fight (A9 fills it).
    pub(crate) fn company_tick(&mut self) {}
}

/// What the hero sees: a square of tiles round them on their level.
pub struct Sight {
    pub at: Pos,
    pub r: i32,
    cells: Vec<bool>,
}

impl Sight {
    /// Whether the hero sees `q`.
    pub fn sees(&self, q: Pos) -> bool {
        let (dx, dy) = (q.x - self.at.x, q.y - self.at.y);
        if q.z != self.at.z || dx.abs() > self.r || dy.abs() > self.r {
            return false;
        }
        let n = (2 * self.r + 1) as usize;
        self.cells[(dy + self.r) as usize * n + (dx + self.r) as usize]
    }
}

fn a_or_an(name: &str) -> String {
    let vowel = name.chars().next().map_or(false, |c| "aeiouAEIOU".contains(c));
    format!("{} {}", if vowel { "an" } else { "a" }, name)
}

#[cfg(test)]
mod tests {
    use super::super::tests::test_hero;
    use super::super::*;
    use super::*;
    use crate::creature::Species;
    use crate::items::ItemKind;
    use crate::material::Mat;

    /// An adventure in a flat wild field, the hero at its middle.
    fn field(seed: u64) -> Adventure {
        // One world for every bout (making it is most of a bout's time).
        static WORLD: std::sync::OnceLock<crate::overworld::Overworld> = std::sync::OnceLock::new();
        let ow = WORLD.get_or_init(|| crate::overworld::Overworld::generate(5)).clone();
        let mut a = Adventure::new(ow, seed, test_hero(seed), None);
        let (x, y) = a.at;
        a.enter(SiteKey::Wild(x, y)).unwrap();
        a
    }

    #[test]
    fn a_hero_steps_and_the_wolf_comes() {
        let mut a = field(11);
        let v = a.visit.as_mut().unwrap();
        let (g, h) = (&mut v.g, v.hero);
        let p = g.dwarves[h].pos;
        // A step east and back.
        assert_eq!(g.act(h, Action::Move(1, 0)), Outcome::Done { ticks: 3 });
        assert_eq!(g.dwarves[h].pos.x, p.x + 1);
        assert!(matches!(g.act(h, Action::Move(-1, 0)), Outcome::Done { .. }));
        assert_eq!(g.dwarves[h].pos, p);
        let wolf = g.spawn_creature(Species::Wolf, p.offset(8, 0, 0), DAY * 10);
        if let Some(c) = g.creatures.get_mut(&wolf) {
            c.target = Some(HERO_ID);
        }
        let t0 = g.tick;
        let hp0: i32 = g.dwarves[h].body.hp.iter().sum();
        let mut wounded = false;
        for _ in 0..20 {
            match g.act(h, Action::Wait(10)) {
                Outcome::Died => break,
                o => assert!(matches!(o, Outcome::Done { .. }), "{:?}", o),
            }
            let hp: i32 = g.dwarves[h].body.hp.iter().sum();
            if hp < hp0 || g.dwarves[h].body.bleeding > 0 {
                wounded = true;
                break;
            }
        }
        assert!(wounded, "the wolf came and bit within 200 ticks");
        assert!(g.tick - t0 <= 200);
        assert!(g.log.iter().any(|e| e.tick >= t0 && e.text.to_lowercase().contains("wolf")), "the log tells of it: {:?}", g.log.iter().rev().take(5).map(|e| &e.text).collect::<Vec<_>>());
        // A long wait is cut short by the wolf beside you.
        if g.dwarves[h].alive {
            assert!(matches!(g.act(h, Action::Wait(500)), Outcome::Interrupted { .. } | Outcome::Died));
        }
    }

    #[test]
    fn a_wall_hides_what_is_behind_it_until_seen() {
        let mut a = field(12);
        let v = a.visit.as_mut().unwrap();
        let (g, h) = (&mut v.g, v.hero);
        let p = g.dwarves[h].pos;
        let (wall, behind, open) = (p.offset(2, 0, 0), p.offset(4, 0, 0), p.offset(0, 4, 0));
        g.world.t_mut(wall).kind = crate::world::TileKind::Wall;
        for q in [behind, open] {
            g.world.t_mut(q).revealed = false;
        }
        let s = g.seen_tiles(h);
        assert!(s.sees(p) && s.sees(wall) && s.sees(open), "the wall itself, and open ground");
        assert!(!s.sees(behind), "not through the wall");
        assert!(!s.sees(p.offset(0, 0, 1)), "only the hero's level");
        g.reveal_seen(h);
        assert!(g.world.t(open).revealed && !g.world.t(behind).revealed, "what was seen is remembered");
    }

    /// One bout: the hero with an iron axe (and the skill of a soldier)
    /// bumps into a goblin every turn until one of them is down.
    fn bout(seed: u64) -> (bool, Adventure) {
        let mut a = field(seed);
        let v = a.visit.as_mut().unwrap();
        let (g, h) = (&mut v.g, v.hero);
        let p = g.dwarves[h].pos;
        let axe = g.spawn_item(ItemKind::Axe, Mat::Iron, p);
        g.items.get_mut(&axe).unwrap().carried_by = Some(HERO_ID);
        g.dwarves[h].held = Some(axe);
        g.dwarves[h].fight_xp = 600;
        let gob = g.spawn_creature(Species::Goblin, p.offset(1, 0, 0), DAY * 10);
        if let Some(c) = g.creatures.get_mut(&gob) {
            c.siege = true;
            c.target = Some(HERO_ID);
        }
        for _ in 0..400 {
            let foe = g.creatures.get(&gob).filter(|c| c.alive).map(|c| c.pos);
            let Some(fp) = foe else { break };
            let me = g.dwarves[h].pos;
            let (dx, dy) = ((fp.x - me.x).signum(), (fp.y - me.y).signum());
            if g.act(h, Action::Move(dx, dy)) == Outcome::Died {
                break;
            }
        }
        let won = g.dwarves[h].alive && !g.creatures.get(&gob).map_or(false, |c| c.alive);
        (won, a)
    }

    #[test]
    fn a_hero_with_an_iron_axe_kills_a_goblin_more_often_than_not() {
        let mut wins = 0;
        let mut deaths = 0;
        let mut kills = 0;
        for seed in 0..50 {
            let (won, a) = bout(1000 + seed);
            let v = a.visit.as_ref().unwrap();
            let d = &v.g.dwarves[v.hero];
            if won {
                wins += 1;
                // (A foe that bleeds out after the blow isn't credited, as in the fort.)
                kills += d.kills;
            }
            if !d.alive {
                deaths += 1;
                // Death goes through `kill`: a death note, and the act says so.
                assert!(v.g.annals.deaths.iter().any(|n| n.name == d.name), "a death note");
                let mut g2 = a;
                let v2 = g2.visit.as_mut().unwrap();
                let h = v2.hero;
                assert_eq!(v2.g.act(h, Action::Wait(1)), Outcome::Died);
            }
        }
        assert!(wins >= 30, "{} wins of 50 ({} deaths)", wins, deaths);
        assert!(kills * 2 >= wins, "most wins are the hero's own kills: {} of {}", kills, wins);
    }
}
