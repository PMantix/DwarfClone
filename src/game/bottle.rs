//! The bottle (C08): drink is the fort's fuel, and dwarves get drunk, get
//! dry, and get into trouble either way.
//!
//! - *Drunkenness as a level* (HLT-04): each mug of ale raises a dwarf's
//!   drink level, less for a seasoned drinker, and it wears off. Merry,
//!   rash (quick to brawl), stumbling (they trip), sick, out cold (asleep
//!   on the floor), and past that poisoned.
//! - *Drink withdrawal* (HLT-09): a dwarf who needs their ale and has had
//!   none for six days gets the shakes; after ten, sick with want.
//! - *Drinking yourself to death* (HLT-10): the level-headed stop at merry
//!   when the ale is free (a festival); the rash don't stop at all.
//! - *Tripping* (HLT-24): a stumbling drunk on the move trips now and then.
//! - *Drowning sorrows* (MND-08): a miserable drinker (a widow, say) goes
//!   on a spree, mug after mug, and picks a fight with whoever they hold a
//!   grudge against.

use super::feuds::BrawlCause;
use super::*;
use crate::dwarf::NeedKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One dwarf's drinking.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DrinkState {
    /// How much ale is in them.
    pub bac: u16,
    /// Their last mug of ale.
    pub last_drink: u64,
    /// Times passed out cold.
    pub outs: u8,
    /// Drinking to forget until this tick.
    pub spree_until: u64,
    pub last_spree: u64,
    /// Felt merry this bout already.
    pub merry: bool,
    /// Last picked (or skipped) a drunken fight.
    pub last_fight: u64,
    /// Has the shakes (counted once a dry spell).
    #[serde(default)]
    pub shaky: bool,
}

/// Everyone's drinking (`Game.society.bar`), by dwarf id.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Bar {
    #[serde(default)]
    pub drinkers: BTreeMap<u32, DrinkState>,
}

/// How drunk a dwarf is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tipsy {
    Sober,
    Merry,
    Rash,
    Stumbling,
    Sick,
    Out,
    Poisoned,
}

impl Tipsy {
    pub fn of(bac: u16) -> Tipsy {
        match bac {
            0..=99 => Tipsy::Sober,
            100..=299 => Tipsy::Merry,
            300..=499 => Tipsy::Rash,
            500..=699 => Tipsy::Stumbling,
            700..=849 => Tipsy::Sick,
            850..=1000 => Tipsy::Out,
            _ => Tipsy::Poisoned,
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            Tipsy::Sober => "",
            Tipsy::Merry => "Merry",
            Tipsy::Rash => "Rash with drink",
            Tipsy::Stumbling => "Stumbling drunk",
            Tipsy::Sick => "Sick with drink",
            Tipsy::Out => "Out cold",
            Tipsy::Poisoned => "Drink-poisoned",
        }
    }
}

/// How much the Drink need matters to `d` (0 if it doesn't).
fn thirst_for_ale(d: &Dwarf) -> u8 {
    d.need(NeedKind::Drink).map_or(0, |n| n.strength)
}

/// A seasoned drinker feels each mug less.
fn tolerance(d: &Dwarf) -> u16 {
    (20 * thirst_for_ale(d) as u16 + d.age.unwrap_or(40) as u16 / 10).min(90)
}

/// Stops at merry when the ale is free.
fn level_headed(d: &Dwarf) -> bool {
    d.personality.anxiety < 0 && d.personality.diligence > 0
}

impl Game {
    /// How drunk `id` is.
    pub fn tipsy(&self, id: u32) -> Tipsy {
        self.society.bar.drinkers.get(&id).map_or(Tipsy::Sober, |s| Tipsy::of(s.bac))
    }

    /// A mug of ale goes down (`Task::DrinkItem`, a festival's mugs).
    pub(super) fn imbibe(&mut self, d: &mut Dwarf) {
        let tick = self.tick;
        let tol = tolerance(d);
        let s = self.society.bar.drinkers.entry(d.id).or_default();
        let dry = s.shaky;
        s.bac = (s.bac + 140 - tol).min(1400);
        s.last_drink = tick;
        s.shaky = false;
        let spree = tick < s.spree_until;
        if dry {
            d.think("steadied their hands with a drink at last", -10, tick);
        }
        if spree {
            d.think("drowned their sorrows", -25, tick);
        }
    }

    /// At a festival: whether `d` takes another free mug.
    pub(super) fn will_drink(&self, d: &Dwarf) -> bool {
        let t = self.tipsy(d.id);
        if level_headed(d) {
            t < Tipsy::Merry
        } else if d.personality.temper > 20 {
            true
        } else {
            t < Tipsy::Stumbling
        }
    }

    /// Every ten ticks for each dwarf (`society_dwarf`): the ale wears off,
    /// and while it's in them it does what ale does.
    pub(super) fn bottle_dwarf(&mut self, d: &mut Dwarf) {
        let tick = self.tick;
        let Some(s) = self.society.bar.drinkers.get_mut(&d.id) else { return };
        if s.bac == 0 {
            return;
        }
        s.bac = s.bac.saturating_sub(3);
        let t = Tipsy::of(s.bac);
        if t == Tipsy::Sober {
            s.merry = false;
            return;
        }
        if !s.merry {
            s.merry = true;
            d.think("got merry on ale", -5, tick);
        }
        let s = s.clone();
        // Past out cold: poisoned, asleep or not.
        if t == Tipsy::Poisoned && tick % 100 == d.id as u64 % 100 {
            let part = d.body.pick_part(&mut self.rng);
            d.body.wound(part, 1, false);
            d.think("was sick to death with drink", 20, tick);
            if self.rng.chance(0.1) {
                self.count("soc.drank_to_death", 1);
                self.kill(d, "drank themselves to death");
                return;
            }
        }
        if d.asleep || !d.alive {
            return;
        }
        match t {
            Tipsy::Out | Tipsy::Poisoned => {
                // Down where they stand, until it's slept off.
                self.release_task(d);
                d.task = Task::Sleep { bed: None, stand: d.pos };
                d.asleep = true;
                d.fatigue = d.fatigue.max(1200);
                let outs = {
                    let e = self.society.bar.drinkers.get_mut(&d.id).unwrap();
                    e.outs = e.outs.saturating_add(1);
                    e.outs
                };
                let text = format!("{} passes out cold, drunk.", d.called());
                self.msg_at(&text, (200, 170, 110), Some(d.pos), logkind::LogKind::Other);
                if outs == 3 {
                    self.gossip(format!("{} has passed out drunk three times now. They call them a barrel with a beard.", d.called()));
                }
                self.count("soc.drunk_out", 1);
            }
            Tipsy::Sick if self.rng.chance(0.04) => {
                d.think("was sick from too much ale", 5, tick);
                d.work = d.work.saturating_sub(20);
            }
            Tipsy::Stumbling | Tipsy::Sick if !d.path.is_empty() && self.rng.chance(0.03) => {
                let part = d.body.pick_part(&mut self.rng);
                d.body.wound(part, 1, false);
                d.move_cd += 10;
                d.think("tripped over their own feet, drunk", 3, tick);
                if self.rng.chance(0.2) {
                    self.gossip(format!("{} went sprawling, drunk as a cave toad.", d.called()));
                }
                self.count("soc.tripped", 1);
            }
            _ => {}
        }
        // Rash with drink, and someone they can't stand close by.
        if t >= Tipsy::Rash && t <= Tipsy::Sick && tick >= s.last_fight + DAY / 4 && !self.raided() && self.in_brawl(d.id).is_none() {
            self.society.bar.drinkers.get_mut(&d.id).unwrap().last_fight = tick;
            let foe = self
                .dwarves
                .iter()
                .filter(|o| o.alive && !o.asleep && !o.is_child() && o.id != d.id && o.pos.z == d.pos.z && (o.pos.x - d.pos.x).abs() <= 5 && (o.pos.y - d.pos.y).abs() <= 5 && d.regard(o.id) <= -20)
                .min_by_key(|o| (d.regard(o.id), o.id))
                .map(|o| o.id);
            if let Some(foe) = foe {
                if self.rng.chance(0.3) && self.start_brawl(d, foe, BrawlCause::Drunk).is_some() {
                    self.count("soc.drunk_brawl", 1);
                }
            }
        }
    }

    /// Daily: the dry get the shakes, and the miserable reach for the bottle.
    pub(super) fn bottle_daily(&mut self) {
        let tick = self.tick;
        let ale = self.items.values().any(|it| it.kind == ItemKind::Drink && it.free());
        for i in 0..self.dwarves.len() {
            let d = &self.dwarves[i];
            if !d.alive || d.is_child() || d.insane {
                continue;
            }
            let (id, want, stress) = (d.id, thirst_for_ale(d), d.stress);
            let s = self.society.bar.drinkers.entry(id).or_insert_with(|| DrinkState { last_drink: tick, ..Default::default() });
            let dry = tick.saturating_sub(s.last_drink) / DAY;
            // Drowning sorrows: miserable, fond of the bottle, and the ale is there.
            let mourning = self.society.grief.mourners.iter().find(|m| m.who == id && tick < m.until).map(|m| m.name.clone());
            let spree = ale && (stress > 250 || mourning.is_some() && stress > 120) && want >= 2 && (s.last_spree == 0 || tick >= s.last_spree + SEASON);
            if spree {
                s.last_spree = tick;
                s.spree_until = tick + DAY / 2;
                let called = self.dwarves[i].called();
                let why = mourning.map_or(String::new(), |n| format!(" over {}", n.split(' ').next().unwrap_or(&n)));
                self.gossip(format!("{} has taken to the bottle{}.", called, why));
                self.count("soc.spree", 1);
                continue;
            }
            // (Thirst comes every four days or so: two dry turns is want.)
            if want < 2 || dry < 6 {
                continue;
            }
            let first = !s.shaky;
            s.shaky = true;
            if first {
                self.count("soc.withdrawal", 1);
            }
            let d = &mut self.dwarves[i];
            if dry >= 10 {
                d.think("was sick with want of drink", 6, tick);
            } else {
                d.think("had the shakes for want of a drink", 3, tick);
            }
        }
        self.society.bar.drinkers.retain(|id, _| self.dwarves.iter().any(|d| d.id == *id && d.alive));
    }

    /// The leisure hook: on a spree, another mug.
    pub(super) fn bottle_leisure(&mut self, d: &mut Dwarf) -> bool {
        let tick = self.tick;
        let Some(s) = self.society.bar.drinkers.get(&d.id) else { return false };
        if tick >= s.spree_until || self.raided() {
            return false;
        }
        let stop = if level_headed(d) { Tipsy::Merry } else { Tipsy::Out };
        if Tipsy::of(s.bac) >= stop || !self.items.values().any(|it| it.kind == ItemKind::Drink && it.free()) {
            return false;
        }
        self.find_drink(d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drinker(g: &mut Game, i: usize) -> Dwarf {
        let mut d = std::mem::take(&mut g.dwarves[i]);
        d.personality.anxiety = 10;
        d.personality.temper = 30;
        d.age = Some(40);
        d
    }

    #[test]
    fn drink_by_drink_a_dwarf_gets_merry_then_rash_then_out() {
        let mut g = Game::new(51);
        g.creatures.retain(|_, c| c.tame);
        let mut d = drinker(&mut g, 0);
        let mut seen = Vec::new();
        for _ in 0..12 {
            g.imbibe(&mut d);
            seen.push(g.tipsy(d.id));
            g.bottle_dwarf(&mut d);
            if d.asleep {
                break;
            }
        }
        assert!(seen.contains(&Tipsy::Merry) && seen.contains(&Tipsy::Rash), "{:?}", seen);
        assert!(d.asleep && matches!(d.task, Task::Sleep { .. }), "never passed out: {:?}", seen);
        assert!(d.thoughts.iter().any(|t| t.text == "got merry on ale"));
        assert!(g.log.iter().any(|l| l.text.contains("passes out cold")));
        // It wears off.
        for _ in 0..400 {
            g.bottle_dwarf(&mut d);
        }
        assert_eq!(g.tipsy(d.id), Tipsy::Sober);
        g.dwarves[0] = d;
    }

    #[test]
    fn the_level_headed_stop_at_merry_and_the_rash_dont() {
        let mut g = Game::new(52);
        let mut calm = drinker(&mut g, 0);
        calm.personality.anxiety = -20;
        calm.personality.diligence = 20;
        let mut rash = drinker(&mut g, 1);
        rash.personality.temper = 40;
        for _ in 0..12 {
            if g.will_drink(&calm) {
                g.imbibe(&mut calm);
            }
            if g.will_drink(&rash) {
                g.imbibe(&mut rash);
            }
        }
        assert_eq!(g.tipsy(calm.id), Tipsy::Merry);
        assert_eq!(g.tipsy(rash.id), Tipsy::Poisoned);
        g.dwarves[0] = calm;
        g.dwarves[1] = rash;
    }

    #[test]
    fn a_dry_fort_gets_the_shakes() {
        let mut g = Game::new(53);
        for d in g.dwarves.iter_mut() {
            d.needs.retain(|n| n.kind != NeedKind::Drink);
            d.needs.push(crate::dwarf::Need { kind: NeedKind::Drink, strength: 3, level: 0, low: false });
        }
        g.items.retain(|_, it| it.kind != ItemKind::Drink);
        g.bottle_daily();
        g.tick += 7 * DAY;
        g.bottle_daily();
        assert!(g.dwarves.iter().filter(|d| !d.is_child()).all(|d| d.thoughts.iter().any(|t| t.text.contains("the shakes"))));
        assert!(g.annals.counters.get("soc.withdrawal").copied().unwrap_or(0) > 0);
        // One drink clears it.
        let mut d = std::mem::take(&mut g.dwarves[0]);
        g.imbibe(&mut d);
        assert!(d.thoughts.iter().any(|t| t.text.contains("steadied their hands")));
        g.dwarves[0] = d;
    }

    /// The "done when", the drink half: a miserable widow takes to the
    /// bottle, and drunk, picks a fight with her rival.
    #[test]
    fn a_widow_drinks_and_picks_a_fight() {
        let mut g = Game::new(54);
        g.creatures.retain(|_, c| c.tame);
        let (a, b) = (g.dwarves[0].id, g.dwarves[1].id);
        for i in 0..2 {
            let d = &mut g.dwarves[i];
            d.needs.retain(|n| n.kind != NeedKind::Drink);
            d.needs.push(crate::dwarf::Need { kind: NeedKind::Drink, strength: 3, level: 0, low: false });
        }
        g.dwarves[0].stress = 400;
        g.dwarves[0].adjust_regard(b, -60);
        let p = g.dwarves[0].pos;
        g.dwarves[1].pos = p.offset(1, 0, 0);
        assert!(g.items.values().any(|it| it.kind == ItemKind::Drink && it.free()));
        g.bottle_daily();
        assert!(g.gossip.iter().any(|(_, t)| t.contains("taken to the bottle")));
        let mut d = std::mem::take(&mut g.dwarves[0]);
        d.task = Task::Idle;
        assert!(g.bottle_leisure(&mut d), "no drink on the spree");
        assert!(matches!(d.task, Task::DrinkItem { .. }));
        for _ in 0..3 {
            g.imbibe(&mut d);
        }
        assert!(d.thoughts.iter().any(|t| t.text == "drowned their sorrows"));
        let mut fought = false;
        for k in 0..200 {
            g.tick += DAY / 4;
            g.society.bar.drinkers.get_mut(&a).unwrap().bac = 350;
            d.asleep = false;
            g.bottle_dwarf(&mut d);
            if g.in_brawl(a).is_some() {
                fought = true;
                break;
            }
            assert!(k < 199);
        }
        g.dwarves[0] = d;
        assert!(fought);
        assert!(g.in_brawl(b).is_some());
        assert_eq!(g.annals.counters.get("soc.drunk_brawl"), Some(&1));
    }

    /// The "done when": a harvest festival with ale aplenty leaves some
    /// merry or worse, and two rash rivals in the mugs come to blows.
    #[test]
    fn a_festival_leaves_drunks_and_one_brawl() {
        let mut g = Game::new(55);
        g.creatures.retain(|_, c| c.tame);
        let p = g.dwarves[0].pos;
        for _ in 0..40 {
            g.spawn_item(ItemKind::Drink, Mat::Stoutcap, p);
        }
        g.start_festival(super::super::festivals::FestivalKind::Harvest);
        assert!(g.festival.is_some());
        let (a, b) = (g.dwarves[0].id, g.dwarves[1].id);
        g.dwarves[0].adjust_regard(b, -50);
        g.dwarves[1].pos = p.offset(1, 0, 0);
        let mut brawled = false;
        for _ in 0..4 {
            for i in 0..4 {
                let mut d = std::mem::take(&mut g.dwarves[i]);
                d.personality.temper = 30;
                d.personality.anxiety = 10;
                g.festival_drink(&mut d, super::super::festivals::FestivalKind::Harvest);
                g.tick += 1;
                g.dwarves[i] = d;
            }
        }
        let drunk = g.dwarves.iter().filter(|d| g.tipsy(d.id) >= Tipsy::Merry).count();
        assert!(drunk >= 2, "only {} merry", drunk);
        for _ in 0..100 {
            g.tick += DAY / 4;
            g.society.bar.drinkers.get_mut(&a).unwrap().bac = 400;
            let mut d = std::mem::take(&mut g.dwarves[0]);
            d.asleep = false;
            g.bottle_dwarf(&mut d);
            g.dwarves[0] = d;
            if g.in_brawl(a).is_some() {
                brawled = true;
                break;
            }
        }
        assert!(brawled && g.in_brawl(b).is_some());
    }
}
