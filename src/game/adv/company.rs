//! Companions (A9): folk of a town who walk with the hero. Ask "Will you
//! come with me?" and a hunter who has heard of you comes for nothing, a
//! cautious brewer for coin, the elder not at all. A companion is a whole
//! person (a `Dwarf` struct, as the hero is) with the gear they carried,
//! so they bleed, limp and die by the same body; they follow the hero
//! from site to site (`carry_in`/`carry_out`) and over the road.
//!
//! In a site `Game::company_tick` (one line in `adv_step`) runs them: they
//! keep near the hero, go for what attacks the band, and run when badly
//! hurt; the hero's orders (`c`) hold them in place, set them on a foe, or
//! send them home. Loyalty rises with every foe the band brings down and
//! falls with their wounds, a friend's death and the hero's murders; below
//! nothing they leave ("Kadol has had enough, and turns for home").
//!
//! A companion's death is a chapter of the hero's saga and a line in the
//! legends; a companion who outlives the hero carries the news home.

use super::super::combat::adjacent;
use super::super::*;
use super::sitegen::town::{FOLK, GUARD};
use super::talk::Person;
use super::travel::cheb;
use super::{Adventure, Calling, Hero, SiteKey, HERO_ID};
use crate::overworld::Race;
use crate::path::astar;
use serde::{Deserialize, Serialize};

/// Most companions at once.
pub const MAX_COMPANY: usize = 3;
/// How far a companion lets the hero get ahead (tiles).
const FOLLOW: i32 = 2;
/// How far a companion looks for a fight (tiles).
const ENGAGE: i32 = 7;

/// What the band has been through, for loyalty (A9).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Band {
    /// Foes brought down by the hero and the company, at the last look.
    #[serde(default)]
    pub kills: u32,
    /// The hero's infamy at the last look.
    #[serde(default)]
    pub infamy: i32,
    /// Companions badly hurt at the last look (ids).
    #[serde(default)]
    pub hurt: Vec<u32>,
    /// How many have been hired (the next one's id follows).
    #[serde(default)]
    pub hired: u32,
    /// Who walked with the hero, and who of them were lost.
    #[serde(default)]
    pub joined: Vec<String>,
    #[serde(default)]
    pub lost: Vec<String>,
    #[serde(default)]
    pub quit: Vec<String>,
}

/// A companion's order (kept in their `task` while in a site).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    Follow,
    Wait,
    /// Go for the nearest foe the hero can see.
    Attack,
    /// Leave the company, for home.
    Home,
}

/// "Steadfast", "Uneasy": a companion's loyalty in a word.
pub fn loyalty_word(l: i32) -> &'static str {
    match l {
        i32::MIN..=0 => "wavering",
        1..=3 => "uneasy",
        4..=7 => "loyal",
        _ => "steadfast",
    }
}

fn first(name: &str) -> &str {
    name.split([' ', ',']).next().unwrap_or(name)
}

/// The weapon a companion with none takes up from home.
fn spare_weapon(race: Race, role: &str) -> (ItemKind, Mat) {
    match (race, role) {
        (Race::Elf, _) => (ItemKind::Spear, Mat::Wood),
        (_, r) if r.starts_with("hunter") => (ItemKind::Spear, Mat::Copper),
        (Race::Dwarf, _) => (ItemKind::Axe, Mat::Copper),
        _ => (ItemKind::Spear, Mat::Copper),
    }
}

impl Adventure {
    /// The companions as they are now (in the visit while inside one),
    /// with their place in `company`.
    pub fn companions(&self) -> Vec<(usize, &Dwarf)> {
        self.company
            .iter()
            .enumerate()
            .filter_map(|(i, c)| match &self.visit {
                Some(v) => v.g.dwarves.iter().find(|d| d.id == c.dwarf.id).map(|d| (i, d)),
                None => Some((i, &c.dwarf)),
            })
            .collect()
    }

    /// What `p` asks to join: None, they won't (not from here, or the
    /// company is full); Some(0), they come for the hero's name alone;
    /// Some(n), for n coins.
    pub fn terms(&self, p: &Person) -> Option<u32> {
        if p.home.is_none() || p.id > HERO_ID || self.company.len() >= MAX_COMPANY || p.role.starts_with("elder") {
            return None;
        }
        let mut caution = (crate::game::stories::hash(&p.name, 0xC0A7) % 7) as i32;
        if p.role.starts_with("guard") {
            caution += 3;
        }
        let standing = p.civ.and_then(|c| self.fame.by_civ.get(&c)).copied().unwrap_or(0) + self.fame.renown / 3;
        if self.fame.infamy >= 9 && p.race != Race::Goblin {
            return None;
        }
        if standing >= caution {
            Some(0)
        } else {
            Some(8 + 4 * caution as u32)
        }
    }

    /// "Will you come with me?" asked of `who` (A7's talk menu): their
    /// answer, and hiring them if they say yes. None: talk answers for them.
    pub fn company_talk(&mut self, who: &Person) -> Option<Vec<String>> {
        if who.id > HERO_ID {
            return Some(vec!["I'm with you already. Lead on.".to_string()]);
        }
        if who.home.is_some() && self.company.len() >= MAX_COMPANY {
            return Some(vec!["You've company enough. I'd only be one more mouth.".to_string()]);
        }
        let price = self.terms(who)?;
        if price > self.hero.coins {
            return Some(vec![format!("Leave my home for a stranger? Not for nothing. {} coins, and I'll carry a spear for you.", price)]);
        }
        self.hero.coins -= price;
        if !self.hire(who) {
            return None;
        }
        Some(vec![if price == 0 {
            "With you? They tell of what you've done. Aye, I'll come. Let me get my things.".to_string()
        } else {
            format!("{} coins? Done. I'll get my things.", price)
        }])
    }

    /// `who` joins the company: out of the town's folk and into the band,
    /// with what they carried, as a whole person.
    pub fn hire(&mut self, who: &Person) -> bool {
        let id = HERO_ID + 1 + self.band.hired;
        let Some(v) = self.visit.as_mut() else { return false };
        let Some(c) = v.g.creatures.remove(&who.id) else { return false };
        self.band.hired += 1;
        let mut rng = crate::rng::Rng::new(crate::game::stories::hash(&who.name, 0xC0A8));
        let mut d = crate::dwarf::new_dwarf(id, c.pos, &mut rng, &[]);
        d.name = who.name.clone();
        d.fight_xp = if c.visitor == GUARD {
            500
        } else if who.role.starts_with("hunter") {
            300
        } else {
            150
        };
        for iid in c.weapon.iter().chain(c.armor.iter()) {
            if let Some(it) = v.g.items.get_mut(iid) {
                it.carried_by = Some(id);
            }
        }
        d.held = c.weapon;
        d.armor = c.armor.clone();
        if d.held.is_none() {
            let (kind, mat) = spare_weapon(who.race, &who.role);
            let w = v.g.spawn_item(kind, mat, c.pos);
            if let Some(it) = v.g.items.get_mut(&w) {
                it.carried_by = Some(id);
            }
            d.held = Some(w);
        }
        d.task = Task::Idle;
        v.g.dwarves.push(d);
        let mut shell = Dwarf::default();
        shell.id = id;
        let standing = who.civ.and_then(|c| self.fame.by_civ.get(&c)).copied().unwrap_or(0).max(0);
        self.company.push(Hero { dwarf: shell, pack: Vec::new(), coins: 0, home: who.home, calling: Calling::Warrior, race: who.race, civ: who.civ, known: Default::default(), loyalty: 5 + standing.min(5) });
        // (Gone from the folk, not murdered.)
        self.fame.seen.folk.retain(|(i, _)| *i != who.id);
        self.band.kills = self.band_kills();
        self.band.infamy = self.fame.infamy;
        let home = who.home.and_then(|h| self.ow().sites.iter().find(|s| s.id == h)).map_or("the road".to_string(), |s| s.name.clone());
        let role = who.role.split(" of ").next().unwrap_or(&who.role).to_string();
        self.band.joined.push(who.name.clone());
        self.say(&format!("{}, {} of {}, joins you. (c: your company)", who.name, role, home), (170, 220, 150));
        self.deed(&format!("was joined by {}, {} of {}", who.name, role, home), 0);
        true
    }

    /// For the bot (A18): ask the folk here, one by one, to come along,
    /// and take the first who will for what the purse holds. The line for
    /// its story.
    pub fn hire_for_the_road(&mut self) -> Option<String> {
        let v = self.visit.as_ref()?;
        let mut ids: Vec<u32> = v.g.creatures.values().filter(|c| c.alive && (c.visitor == FOLK || c.visitor == GUARD) && !c.hostile() && !c.siege).map(|c| c.id).collect();
        ids.sort();
        for id in ids {
            let Some(p) = self.person(id) else { continue };
            if self.greet(&p).closed {
                continue;
            }
            let Some(price) = self.terms(&p).filter(|&n| n <= self.hero.coins) else { continue };
            self.hero.coins -= price;
            if self.hire(&p) {
                let paid = if price == 0 { "for the hero's name alone".to_string() } else { format!("for {} coins", price) };
                return Some(format!("{}, {}, comes along {}.", p.name, p.role, paid));
            }
        }
        None
    }

    /// Foes brought down by the band so far (the living in it).
    fn band_kills(&self) -> u32 {
        self.hero_dwarf().kills + self.companions().iter().map(|(_, d)| d.kills).sum::<u32>()
    }

    /// Run with `notice` after every act: a companion fallen (in a site,
    /// or bled out on the road), loyalty won by victories and lost to
    /// wounds and murder, and those whose loyalty fails leave.
    pub fn company_check(&mut self) {
        if self.company.is_empty() || self.over.is_some() {
            return;
        }
        // The fallen.
        let dead: Vec<usize> = (0..self.company.len())
            .filter(|&i| {
                let id = self.company[i].dwarf.id;
                match &self.visit {
                    Some(v) => v.g.dwarves.iter().any(|d| d.id == id && !d.alive),
                    None => !self.company[i].dwarf.alive,
                }
            })
            .collect();
        for &i in dead.iter().rev() {
            self.lost(i);
        }
        // Victories shared.
        let kills = self.band_kills();
        if kills > self.band.kills {
            for c in self.company.iter_mut() {
                c.loyalty += 1;
            }
        }
        self.band.kills = kills;
        // Wounds.
        let hurt: Vec<(u32, bool)> = self.companions().iter().map(|(_, d)| (d.id, d.body.badly_hurt())).collect();
        for (id, bad) in hurt {
            let was = self.band.hurt.contains(&id);
            if bad && !was {
                self.band.hurt.push(id);
                if let Some(c) = self.company.iter_mut().find(|c| c.dwarf.id == id) {
                    c.loyalty -= 2;
                }
            } else if !bad && was {
                self.band.hurt.retain(|&x| x != id);
            }
        }
        // Murder.
        if self.fame.infamy > self.band.infamy {
            for c in self.company.iter_mut().filter(|c| c.race != Race::Goblin) {
                c.loyalty -= 4;
            }
        }
        self.band.infamy = self.fame.infamy;
        // Those who have had enough.
        while let Some(i) = self.company.iter().position(|c| c.loyalty < 0) {
            self.part(i, false);
        }
    }

    /// A companion has fallen: a chapter of the saga, a line in the
    /// legends, and the others shaken.
    fn lost(&mut self, i: usize) {
        let c = self.company.remove(i);
        let id = c.dwarf.id;
        let (name, how) = match &self.visit {
            Some(v) => {
                let d = v.g.dwarves.iter().find(|d| d.id == id);
                let name = d.map_or(String::new(), |d| d.name.clone());
                let how = match v.g.annals.deaths.iter().rev().find(|n| n.name == name).map(|n| (n.cause.clone(), n.killer.clone())) {
                    Some((cause, Some(k))) => format!("{} by {}", cause, super::sitegen::lore::a_or_an(&k)),
                    Some((cause, None)) => cause,
                    None => "fell".to_string(),
                };
                (name, how)
            }
            None => (c.dwarf.name.clone(), "bled to death on the road".to_string()),
        };
        let place = match self.visit.as_ref().map(|v| v.key) {
            Some(k) => super::sites::site_name(self.ow(), k),
            None => super::travel::land_name(self.ow(), self.at.0, self.at.1),
        };
        let home = c.home.and_then(|h| self.ow().sites.iter().find(|s| s.id == h)).map_or("the road".to_string(), |s| s.name.clone());
        // Their pack stays with their body.
        if let Some(v) = self.visit.as_mut() {
            let pos = v.g.dwarves.iter().find(|d| d.id == id).map(|d| d.pos);
            let pack: Vec<u32> = v.g.items.values().filter(|it| it.carried_by == Some(id)).map(|it| it.id).collect();
            if let Some(p) = pos {
                for it in pack {
                    v.g.drop_item(it, p);
                }
            }
        }
        for o in self.company.iter_mut() {
            o.loyalty -= 3;
        }
        self.band.hurt.retain(|&x| x != id);
        self.band.lost.push(name.clone());
        let hero = self.hero_dwarf().called();
        self.say(&format!("{} is dead. They followed you from {}.", name, home), (235, 60, 60));
        self.deed(&format!("lost {} of {}, who {} in {} at their side", name, home, how, place), 0);
        let year = self.year();
        self.ow_mut().log(year, format!("{} of {} {} in {}, walking with {}.", name, home, how, place, hero));
    }

    /// A companion leaves the company: sent home (`sent`), or their
    /// loyalty failed. They walk off with what they carry.
    pub fn part(&mut self, i: usize, sent: bool) {
        if i >= self.company.len() {
            return;
        }
        let c = self.company.remove(i);
        let id = c.dwarf.id;
        let name = match self.visit.as_mut() {
            Some(v) => match v.g.dwarves.iter().position(|d| d.id == id) {
                Some(j) => {
                    let d = v.g.dwarves.remove(j);
                    let pack: Vec<u32> = v.g.items.values().filter(|it| it.carried_by == Some(id)).map(|it| it.id).collect();
                    for it in pack {
                        v.g.items.remove(&it);
                    }
                    d.name
                }
                None => String::new(),
            },
            None => c.dwarf.name.clone(),
        };
        self.band.hurt.retain(|&x| x != id);
        let home = c.home.and_then(|h| self.ow().sites.iter().find(|s| s.id == h)).map_or("home".to_string(), |s| s.name.clone());
        if sent {
            self.say(&format!("{} clasps your arm and turns for {}.", name, home), (200, 200, 170));
        } else {
            self.band.quit.push(name.clone());
            self.say(&format!("{} has had enough, and turns for {} without a word.", name, home), (240, 150, 110));
            self.deed(&format!("was left by {}, who had had enough", name), 0);
        }
    }

    /// An order to companion `i` (in a site): follow, wait here, go for
    /// the nearest foe in sight, or go home.
    pub fn order(&mut self, i: usize, o: Order) -> Result<String, String> {
        if o == Order::Home {
            self.part(i, true);
            return Ok("Sent home.".into());
        }
        let id = self.company.get(i).map(|c| c.dwarf.id).ok_or("No such companion.")?;
        let v = self.visit.as_mut().ok_or("Orders are for a site, not the road.")?;
        let foe = {
            let h = v.hero;
            v.g.in_sight(h).into_iter().find(|c| v.g.creatures.get(c).map_or(false, |c| c.alive && (c.hostile() || c.siege) && !c.tame))
        };
        let d = v.g.dwarves.iter_mut().find(|d| d.id == id && d.alive).ok_or("They can't hear you.")?;
        let who = first(&d.name).to_string();
        match o {
            Order::Follow => {
                d.task = Task::Idle;
                Ok(format!("{} falls in behind you.", who))
            }
            Order::Wait => {
                d.task = Task::Station { pos: d.pos };
                Ok(format!("{} waits here.", who))
            }
            Order::Attack => match foe {
                Some(f) => {
                    d.task = Task::Attack { target: f };
                    Ok(format!("{} goes for it.", who))
                }
                None => Err("There is nothing in sight to set them on.".into()),
            },
            Order::Home => unreachable!(),
        }
    }

    /// The hero is dead: the companions who live carry the news home.
    pub fn company_at_death(&mut self) {
        let hero = self.hero_dwarf().called();
        let alive: Vec<(String, Option<u32>)> = self.companions().iter().filter(|(_, d)| d.alive).map(|(i, d)| (d.name.clone(), self.company[*i].home)).collect();
        let year = self.year();
        for (name, home) in alive {
            let home = home.and_then(|h| self.ow().sites.iter().find(|s| s.id == h)).map_or("home".to_string(), |s| s.name.clone());
            self.ow_mut().log(year, format!("{} carried word of the death of {} back to {}.", name, hero, home));
        }
    }

    /// Companions after a day on the road (A9): nothing yet beyond what
    /// the road's walk does to their bodies.
    pub fn company_day(&mut self) {}
}

impl Game {
    /// Companions follow and fight (one line in `adv_step`): those with
    /// ids above the hero's.
    pub(crate) fn company_tick(&mut self) {
        if !self.dwarves.iter().any(|d| d.id > HERO_ID && d.alive) {
            return;
        }
        let Some(hero) = self.dwarves.iter().find(|d| d.id == HERO_ID).map(|d| d.pos) else { return };
        for i in 0..self.dwarves.len() {
            let d = &self.dwarves[i];
            if d.id <= HERO_ID || !d.alive || d.body.unconscious() {
                continue;
            }
            let mut d = std::mem::take(&mut self.dwarves[i]);
            d.move_cd = d.move_cd.saturating_sub(1);
            self.companion_act(&mut d, hero);
            self.dwarves[i] = d;
        }
    }

    /// One companion's tick: run when badly hurt, fight what is near (or
    /// what they were set on), else keep near the hero (or wait).
    fn companion_act(&mut self, d: &mut Dwarf, hero: Pos) {
        let me = d.pos;
        let foe = |c: &crate::creature::Creature| c.alive && !c.tame && c.pet_of.is_none() && (c.hostile() || c.siege) && !(c.visitor >= FOLK && !c.siege);
        let mut foes: Vec<(i32, u32, Pos)> = self.creatures.values().filter(|c| foe(c) && c.pos.z == me.z && cheb((c.pos.x, c.pos.y), (me.x, me.y)) <= ENGAGE).map(|c| (cheb((c.pos.x, c.pos.y), (me.x, me.y)), c.id, c.pos)).collect();
        foes.sort();
        let ordered = match d.task {
            Task::Attack { target } => match self.creatures.get(&target).filter(|c| c.alive) {
                Some(c) => Some((target, c.pos)),
                None => {
                    d.task = Task::Idle;
                    None
                }
            },
            _ => None,
        };
        // Badly hurt: away from the nearest foe.
        if d.body.badly_hurt() && d.body.blood * 2 < d.body.max_blood {
            if let Some(&(_, _, fp)) = foes.first() {
                if d.move_cd == 0 {
                    let mut nb = Vec::new();
                    self.world.neighbors(me, &mut nb, Mover::Dwarf);
                    let far = |q: Pos| (q.x - fp.x).pow(2) + (q.y - fp.y).pow(2);
                    if let Some(&(q, _)) = nb.iter().filter(|(q, _)| q.z == me.z).max_by_key(|(q, _)| far(*q)) {
                        if far(q) > far(me) {
                            d.pos = q;
                            d.path.clear();
                        }
                    }
                    d.move_cd = MOVE_DELAY + d.body.limp();
                }
                return;
            }
        }
        if let Some((cid, cp)) = ordered.or(foes.first().map(|&(_, id, p)| (id, p))) {
            if adjacent(cp, me) && cp.z == me.z {
                if d.attack_cd == 0 && d.body.can_hold() {
                    self.dwarf_attack(d, cid);
                }
            } else {
                self.companion_step(d, cp);
            }
            return;
        }
        if matches!(d.task, Task::Station { .. }) {
            return;
        }
        if me.z != hero.z || cheb((me.x, me.y), (hero.x, hero.y)) > FOLLOW {
            self.companion_step(d, hero);
        }
    }

    /// A step along the way to `to`, when their legs allow (the path kept
    /// while it still leads there).
    fn companion_step(&mut self, d: &mut Dwarf, to: Pos) {
        if d.move_cd > 0 {
            return;
        }
        let stale = d.path.last().map_or(true, |&e| e.z != to.z || cheb((e.x, e.y), (to.x, to.y)) > 1) || d.path.first().map_or(true, |&n| cheb((n.x, n.y), (d.pos.x, d.pos.y)) > 1);
        if stale {
            d.path = astar(&self.world, d.pos, to, 20_000, Mover::Dwarf).unwrap_or_default();
        }
        if d.path.is_empty() {
            // No way found: a step that brings them nearer, if there is one.
            let mut nb = Vec::new();
            self.world.neighbors(d.pos, &mut nb, Mover::Dwarf);
            let far = |q: Pos| (q.x - to.x).pow(2) + (q.y - to.y).pow(2) + 4 * (q.z - to.z).pow(2);
            match nb.iter().map(|(q, _)| *q).filter(|&q| q != to).min_by_key(|&q| far(q)).filter(|&q| far(q) < far(d.pos)) {
                Some(q) => {
                    d.pos = q;
                    d.move_cd = MOVE_DELAY + d.body.limp();
                }
                None => d.move_cd = 10,
            }
            return;
        }
        let next = d.path.remove(0);
        // (Not onto the one they follow or fight.)
        if next == to {
            d.path.clear();
            return;
        }
        d.pos = next;
        d.move_cd = MOVE_DELAY + d.body.limp();
    }
}

#[cfg(test)]
mod tests {
    use super::super::sitegen::town::tests::{visit, world};
    use super::super::talk::Topic;
    use super::super::turn::Action;
    use super::*;
    use crate::creature::Species;
    use crate::overworld::SiteKind;

    /// A hero of a town's own civ, in that town, with coin to hire.
    fn town(seed: u64) -> Adventure {
        let ow = world();
        let s = ow
            .sites
            .iter()
            .find(|s| !s.ruined && matches!(s.kind, SiteKind::Town | SiteKind::Hamlet | SiteKind::Fortress) && s.civ.and_then(|c| ow.civs.get(c as usize)).map_or(false, |c| c.race != Race::Goblin))
            .unwrap()
            .clone();
        let race = ow.civs[s.civ.unwrap() as usize].race;
        let mut a = visit(s.id, race, s.civ, seed);
        a.hero.coins = 200;
        a
    }

    /// Hire the first of the folk who will come; their id in the visit.
    fn hire_one(a: &mut Adventure) -> u32 {
        let v = a.visit.as_ref().unwrap();
        let mut ids: Vec<u32> = v.g.creatures.values().filter(|c| c.visitor == FOLK && c.alive).map(|c| c.id).collect();
        ids.sort();
        for id in ids {
            let p = a.person(id).unwrap();
            if a.terms(&p).is_some() {
                let mut chat = a.greet(&p);
                a.ask(&mut chat, Topic::Join);
                assert!(a.company.len() > 0, "{:?}", chat.lines);
                return a.company.last().unwrap().dwarf.id;
            }
        }
        panic!("nobody would come");
    }

    #[test]
    fn a_hired_companion_follows_across_sites() {
        let mut a = town(51);
        let coins = a.hero.coins;
        let id = hire_one(&mut a);
        assert!(id > HERO_ID);
        assert!(a.hero.coins <= coins);
        let name = a.companions()[0].1.name.clone();
        assert!(a.companions()[0].1.held.is_some(), "armed");
        assert!(a.fame.saga.as_ref().unwrap().chapters.iter().any(|c| c.2.contains(&name)), "a chapter of the saga");
        // The hero walks off; the companion keeps up (a few moments behind).
        for _ in 0..12 {
            let v = a.visit.as_mut().unwrap();
            let h = v.hero;
            let _ = v.g.act(h, Action::Move(1, 0));
        }
        for _ in 0..8 {
            let v = a.visit.as_mut().unwrap();
            let h = v.hero;
            let _ = v.g.act(h, Action::Wait(10));
        }
        let v = a.visit.as_ref().unwrap();
        let (hp, cp) = (v.g.dwarves[v.hero].pos, v.g.dwarves.iter().find(|d| d.id == id).unwrap().pos);
        assert!(cheb((hp.x, hp.y), (cp.x, cp.y)) <= FOLLOW + 2, "kept up: {:?} {:?}", hp, cp);
        // Out to the road and into the wild: they come along, gear and all.
        let held = a.companions()[0].1.held;
        a.leave();
        assert_eq!(a.company.len(), 1);
        assert_eq!(a.company[0].dwarf.name, name);
        assert!(!a.company[0].pack.is_empty() && held.is_some());
        let (x, y) = a.at;
        a.enter(SiteKey::Wild(x, y)).unwrap();
        let v = a.visit.as_ref().unwrap();
        let d = v.g.dwarves.iter().find(|d| d.id == id).expect("in the next site too");
        assert!(d.alive && d.held.is_some());
        // Back in town, they don't stand among the folk twice.
        a.leave();
        assert_eq!(a.company[0].dwarf.name, name);
    }

    #[test]
    fn a_companion_fights_beside_and_may_quit() {
        let mut a = town(52);
        let ids: Vec<u32> = (0..3).map(|_| hire_one(&mut a)).collect();
        assert_eq!(a.company.len(), 3);
        // A goblin band in the wild: the companions go for them.
        let (x, y) = a.at;
        a.enter(SiteKey::Wild(x, y)).unwrap();
        a.notice();
        let v = a.visit.as_mut().unwrap();
        let h = v.hero;
        let hp = v.g.dwarves[h].pos;
        let mut gobs = Vec::new();
        for k in 0..3 {
            let at = v.g.spot_near(hp.offset(5, k - 1, 0), 1);
            let g = v.g.spawn_creature(Species::Goblin, at, DAY * 10);
            if let Some(c) = v.g.creatures.get_mut(&g) {
                c.siege = true;
                c.target = Some(ids[k as usize]);
            }
            gobs.push(g);
        }
        let xp = |a: &Adventure| a.companions().iter().map(|(_, d)| d.fight_xp + 100 * d.kills).sum::<u32>();
        let xp0 = xp(&a);
        let mut fought = false;
        for _ in 0..60 {
            let v = a.visit.as_mut().unwrap();
            let _ = v.g.act(h, Action::Wait(10));
            fought |= xp(&a) > xp0;
            a.notice();
            if a.over.is_some() || gobs.iter().all(|g| !a.visit.as_ref().unwrap().g.creatures.get(g).map_or(false, |c| c.alive)) {
                break;
            }
        }
        assert!(fought, "the companions struck at the goblins");
        assert!(a.over.is_none(), "(the goblins were after the companions)");
        // All but one fall (if the goblins left them standing): the saga and
        // the legends tell it, and the last, shaken, stays a while.
        let alive: Vec<u32> = a.companions().iter().filter(|(_, d)| d.alive).map(|(_, d)| d.id).collect();
        assert!(!alive.is_empty(), "one stands");
        let v = a.visit.as_mut().unwrap();
        for id in alive.iter().take(alive.len() - 1) {
            let j = v.g.dwarves.iter().position(|d| d.id == *id).unwrap();
            let mut d = std::mem::take(&mut v.g.dwarves[j]);
            v.g.kill(&mut d, "was struck down");
            v.g.dwarves[j] = d;
        }
        let last = *alive.last().unwrap();
        if let Some(c) = a.company.iter_mut().find(|c| c.dwarf.id == last) {
            c.loyalty = 10;
        }
        a.notice();
        assert_eq!(a.band.lost.len(), 2, "{:?}", a.band);
        assert_eq!(a.company.len(), 1);
        assert!(a.company[0].loyalty < 10, "shaken by the deaths");
        assert!(a.fame.saga.as_ref().unwrap().chapters.iter().any(|c| c.2.contains("lost ")), "the saga tells it");
        assert!(a.ow().events.iter().any(|e| e.text.contains("walking with")), "the legends tell it");
        // The hero murders: the last one goes.
        a.fame.infamy += 3;
        a.company[0].loyalty = 2;
        a.notice();
        assert!(a.company.is_empty());
        assert_eq!(a.band.quit.len(), 1);
        assert!(a.visit.as_ref().unwrap().g.log.iter().any(|e| e.text.contains("has had enough")));
        assert!(a.visit.as_ref().unwrap().g.dwarves.iter().all(|d| d.id != last), "gone from the site");
    }
}
