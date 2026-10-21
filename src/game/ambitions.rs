//! The overseer's ambitions: what it has its heart set on for the next
//! season or so. Each style leans towards some more than others, and the
//! fortress's troubles push it towards what's needed.

use super::personality::NoteKind;
use super::overseer::{Overseer, RoomUse, Style};
use super::*;
use crate::buildings::Recipe;
use crate::game::expeditions::Goal;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Ambition {
    /// Fields, fishing, hunting, brewing and cooking.
    Provision,
    /// Traps, gates and a walled yard.
    Fortify,
    /// More soldiers, more weapons and armour.
    Muster,
    /// Crafts, gems, ore and trade.
    Prosper,
    /// Down the shaft, into the deep places.
    Delve,
    /// New halls and levels.
    Expand,
    /// Taverns, temples, ale and fine engravings.
    Revel,
    /// Expeditions out into the world.
    Venture,
    /// A grand work: a pillared hall, a rotunda, a gallery.
    Architect,
}

impl Ambition {
    pub const ALL: [Ambition; 9] = [
        Ambition::Provision,
        Ambition::Fortify,
        Ambition::Muster,
        Ambition::Prosper,
        Ambition::Delve,
        Ambition::Expand,
        Ambition::Revel,
        Ambition::Venture,
        Ambition::Architect,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Ambition::Provision => "Provision",
            Ambition::Fortify => "Fortify",
            Ambition::Muster => "Muster",
            Ambition::Prosper => "Prosper",
            Ambition::Delve => "Delve",
            Ambition::Expand => "Expand",
            Ambition::Revel => "Revel",
            Ambition::Venture => "Venture",
            Ambition::Architect => "Architect",
        }
    }

    pub fn announce(self) -> &'static str {
        match self {
            Ambition::Provision => "The overseer frets over the larders: more fields, fishing and hunting.",
            Ambition::Fortify => "The overseer looks to the walls: traps, gates and a walled yard.",
            Ambition::Muster => "The overseer calls the fortress to arms: more soldiers, more weapons.",
            Ambition::Prosper => "The overseer dreams of riches: crafts, gems and ore for the caravans.",
            Ambition::Delve => "The overseer turns to the deep places. The shaft goes down.",
            Ambition::Expand => "The overseer wants room to grow: new halls are dug.",
            Ambition::Revel => "The overseer decides the dwarves deserve some cheer: taverns, temples and ale.",
            Ambition::Venture => "The overseer grows restless: the squad will march out to seek glory.",
            Ambition::Architect => "The overseer is seized by a vision: a grand work will be carved from the rock.",
        }
    }

    /// How much a style is drawn to each ambition.
    fn weight(self, style: Style) -> f64 {
        use Ambition::*;
        match (style, self) {
            (Style::Builder, Architect) => 4.0,
            (Style::Reckless, Architect) => 2.0,
            (Style::Warlord, Architect) => 0.5,
            (Style::Steady, Architect) => 1.5,
            (Style::Steady, Delve | Revel | Venture) => 1.0,
            (Style::Steady, _) => 2.0,
            (Style::Builder, Prosper | Expand | Revel) => 3.0,
            (Style::Builder, Provision) => 2.0,
            (Style::Builder, Venture) => 0.5,
            (Style::Builder, _) => 1.0,
            (Style::Delver, Delve) => 4.0,
            (Style::Delver, Prosper) => 3.0,
            (Style::Delver, Provision) => 2.0,
            (Style::Delver, _) => 1.0,
            (Style::Warlord, Muster | Venture) => 4.0,
            (Style::Warlord, Fortify | Provision) => 2.0,
            (Style::Warlord, _) => 1.0,
            (Style::Reckless, Fortify | Muster) => 0.0,
            (Style::Reckless, Delve | Revel) => 3.0,
            (Style::Reckless, Provision) => 1.0,
            (Style::Reckless, _) => 2.0,
        }
    }
}

/// What the overseer notices: each sense pushes the ambitions towards or
/// away from themselves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sense {
    /// The style's own leanings.
    Leaning,
    Hunger,
    Thirst,
    /// An army came this season.
    Besieged,
    /// Fewer beds than dwarves.
    Crowded,
    /// The dwarves are unhappy.
    Unrest,
    /// The fortress is rich for its size.
    Riches,
    /// A monster prowls near the halls.
    Danger,
    /// A strong squad with nothing to do.
    Wanderlust,
    /// Too early, or no squad, to go anywhere.
    Unready,
    /// Tired of what it has just been doing.
    Weary,
    /// Your nudges.
    Nudge,
    /// Its traits (cautious, vain, pious…).
    Temper,
    /// A lesson learned from a siege, still fresh.
    Lesson,
    /// Wild beasts prowl near the fort and the squad is weak.
    Prowlers,
    /// Soldiers without proper arms.
    Unarmed,
    /// Wounded dwarves and too little care.
    Wounded,
    /// An ill omen hangs over the fort (`omens.rs`).
    Omens,
    // Milestone 2 (wave 0): silent until their packages give them a level
    // and links through `overseer_hooks.rs: extra_senses`.
    /// The fort has lost faith in its overseer (O07, `reign.rs`).
    Doubt,
    /// Rich ore found and not yet mined (O10, `veins.rs`).
    Ore,
    /// A dwarf has an inspiration worth building (O12, `inspire.rs`).
    Inspired,
}

impl Sense {
    pub const ALL: [Sense; 21] = [
        Sense::Temper,
        Sense::Leaning,
        Sense::Hunger,
        Sense::Thirst,
        Sense::Besieged,
        Sense::Crowded,
        Sense::Unrest,
        Sense::Riches,
        Sense::Danger,
        Sense::Wanderlust,
        Sense::Unready,
        Sense::Weary,
        Sense::Nudge,
        Sense::Lesson,
        Sense::Prowlers,
        Sense::Unarmed,
        Sense::Wounded,
        Sense::Omens,
        Sense::Doubt,
        Sense::Ore,
        Sense::Inspired,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Sense::Leaning => "Its nature",
            Sense::Hunger => "Hunger",
            Sense::Thirst => "Thirst",
            Sense::Besieged => "Besieged",
            Sense::Crowded => "Crowded",
            Sense::Unrest => "Unrest",
            Sense::Riches => "Riches",
            Sense::Danger => "Danger",
            Sense::Wanderlust => "Wanderlust",
            Sense::Unready => "Unready",
            Sense::Weary => "Weary",
            Sense::Nudge => "Your nudge",
            Sense::Temper => "Its temper",
            Sense::Lesson => "A lesson",
            Sense::Prowlers => "Prowlers",
            Sense::Unarmed => "Unarmed",
            Sense::Wounded => "Wounded",
            Sense::Omens => "An omen",
            Sense::Doubt => "Doubt",
            Sense::Ore => "Ore",
            Sense::Inspired => "Inspired",
        }
    }
}

/// The overseer's mind at a glance: how strongly each sense fires (0..1),
/// what each sense adds to or takes from each ambition, and the resulting
/// odds of choosing it next.
#[derive(Clone, Debug)]
pub struct Brain {
    pub senses: Vec<(Sense, f64)>,
    pub links: Vec<(Ambition, Vec<(Sense, f64)>)>,
    pub odds: Vec<(Ambition, f64)>,
}

impl Brain {
    pub fn odds_of(&self, a: Ambition) -> f64 {
        self.odds.iter().find(|x| x.0 == a).map_or(0.0, |x| x.1)
    }

    pub fn links_of(&self, a: Ambition) -> &[(Sense, f64)] {
        self.links.iter().find(|x| x.0 == a).map_or(&[], |x| &x.1)
    }
}

/// A sense that rules an ambition out entirely.
const BLOCK: f64 = -9.0;

impl Game {
    /// Read the overseer's mind (for the brain screen).
    pub fn overseer_brain(&self) -> Option<Brain> {
        self.overseer.as_ref().map(|o| self.brain(o))
    }

    pub(super) fn brain(&self, o: &Overseer) -> Brain {
        let tick = self.tick;
        let pop = self.alive_count().max(1) as f64;
        let food = self.food_per_head();
        let drink = self.per_head(&[ItemKind::Drink]);
        let hunger = ((4.0 - food) / 4.0).clamp(0.0, 1.0);
        let thirst = ((3.0 - drink) / 3.0).clamp(0.0, 1.0);
        let besieged = if o.last_siege > 0 && tick < o.last_siege + SEASON { 1.0 - (tick - o.last_siege) as f64 / SEASON as f64 } else { 0.0 };
        let beds = self.buildings.values().filter(|b| b.kind == BuildingKind::Bed && b.built).count() as f64;
        let crowded = ((pop - beds) / pop).clamp(0.0, 1.0);
        let stress = self.dwarves.iter().filter(|d| d.alive).map(|d| d.stress as f64).sum::<f64>() / pop;
        let unrest = ((stress - 60.0) / 200.0).clamp(0.0, 1.0);
        let riches = ((self.wealth() as f64 / pop - 2000.0) / 6000.0).clamp(0.0, 1.0);
        let danger = if let (Some(base), true) = (o.base, true) {
            if self.creatures.values().any(|c| c.alive && c.legendary() && c.hostile() && (c.pos.x - base.x).abs() <= 30 && (c.pos.y - base.y).abs() <= 30 && (c.pos.z - base.z).abs() <= 6) {
                1.0
            } else {
                0.0
            }
        } else {
            0.0
        };
        let strength = self.squads.first().map_or(0, |s| self.squad_strength(&s.members)) as f64;
        let wanderlust = if self.expedition.is_none() && !self.alert { ((strength - 8.0) / 12.0).clamp(0.0, 1.0) } else { 0.0 };
        let unready = if self.squads.is_empty() || tick < YEAR / 2 || o.style == Style::Reckless && strength < 1.0 { 1.0 } else { 0.0 };
        // Senses the items add (overseer_hooks.rs), with their pulls.
        let extra = self.extra_senses(o);
        let mut senses = vec![
            (Sense::Temper, if o.traits.is_empty() { 0.0 } else { 1.0 }),
            (Sense::Leaning, 1.0),
            (Sense::Hunger, hunger),
            (Sense::Thirst, thirst),
            (Sense::Besieged, besieged),
            (Sense::Crowded, crowded),
            (Sense::Unrest, unrest),
            (Sense::Riches, riches),
            (Sense::Danger, danger),
            (Sense::Wanderlust, wanderlust),
            (Sense::Unready, unready),
            (Sense::Weary, if o.ambition.is_some() { 1.0 } else { 0.0 }),
            (Sense::Nudge, if o.nudges.iter().any(|&n| n != 0) { 1.0 } else { 0.0 }),
            (Sense::Lesson, 0.0),
            (Sense::Prowlers, 0.0),
            (Sense::Unarmed, 0.0),
            (Sense::Wounded, 0.0),
            (Sense::Omens, 0.0),
            (Sense::Doubt, 0.0),
            (Sense::Ore, 0.0),
            (Sense::Inspired, 0.0),
        ];
        for &(s, level, _) in &extra {
            if let Some(x) = senses.iter_mut().find(|x| x.0 == s) {
                x.1 = level;
            }
        }
        let mut links = Vec::new();
        for &a in Ambition::ALL.iter() {
            use Ambition::*;
            let mut l: Vec<(Sense, f64)> = Vec::new();
            let w = a.weight(o.style);
            l.push((Sense::Leaning, if w > 0.0 { w.ln() } else { BLOCK }));
            let mut add = |s: Sense, v: f64| {
                if v.abs() > 0.01 {
                    l.push((s, v));
                }
            };
            match a {
                Provision => {
                    add(Sense::Hunger, 2.2 * hunger);
                    add(Sense::Thirst, 2.2 * thirst);
                    add(Sense::Riches, -0.3 * riches);
                }
                Fortify => {
                    add(Sense::Besieged, 1.3 * besieged);
                    add(Sense::Danger, 0.6 * danger);
                }
                Muster => {
                    add(Sense::Besieged, 1.3 * besieged);
                    add(Sense::Danger, 0.9 * danger);
                    add(Sense::Wanderlust, -0.5 * wanderlust);
                }
                Prosper => {
                    add(Sense::Riches, -0.8 * riches);
                    add(Sense::Hunger, -0.6 * hunger);
                }
                Delve => {
                    add(Sense::Danger, -0.6 * danger);
                    add(Sense::Hunger, -0.6 * hunger);
                }
                Expand => {
                    add(Sense::Crowded, 1.6 * crowded);
                    add(Sense::Riches, 0.3 * riches);
                }
                Revel => {
                    add(Sense::Unrest, 1.4 * unrest);
                    add(Sense::Riches, 0.5 * riches);
                    add(Sense::Thirst, -0.5 * thirst);
                }
                Venture => {
                    add(Sense::Wanderlust, 1.0 * wanderlust);
                    add(Sense::Besieged, -0.8 * besieged);
                    add(Sense::Unready, BLOCK * unready);
                }
                Architect => {
                    add(Sense::Riches, 0.9 * riches);
                    add(Sense::Unrest, 0.3 * unrest);
                    add(Sense::Hunger, -0.8 * hunger);
                    add(Sense::Besieged, -0.6 * besieged);
                }
            }
            for (s, level, weights) in &extra {
                for &(b, w) in weights {
                    if b == a {
                        add(*s, w * level);
                    }
                }
            }
            if o.ambition == Some(a) {
                // (A stubborn one tires of it more slowly.)
                add(Sense::Weary, if o.has(super::personality::Trait::Stubborn) { -0.8 } else { -1.6 });
            }
            add(Sense::Temper, o.temper(a));
            add(Sense::Nudge, 0.8 * o.nudge(a) as f64);
            links.push((a, l));
        }
        let scores: Vec<(Ambition, f64)> = links.iter().map(|(a, l)| (*a, l.iter().map(|x| x.1).sum::<f64>())).collect();
        let total: f64 = scores.iter().filter(|x| x.1 > BLOCK / 2.0).map(|x| x.1.exp()).sum();
        let odds = scores.iter().map(|(a, s)| (*a, if *s > BLOCK / 2.0 && total > 0.0 { s.exp() / total } else { 0.0 })).collect();
        Brain { senses, links, odds }
    }

    /// Push the overseer towards (or away from) an ambition, -3..=3.
    pub fn nudge_ambition(&mut self, a: Ambition, by: i8) {
        if let Some(o) = self.overseer.as_mut() {
            let i = Ambition::ALL.iter().position(|&x| x == a).unwrap();
            if o.nudges.len() < Ambition::ALL.len() {
                o.nudges.resize(Ambition::ALL.len(), 0);
            }
            o.nudges[i] = (o.nudges[i] + by).clamp(-3, 3);
        }
    }

    pub fn clear_nudges(&mut self) {
        if let Some(o) = self.overseer.as_mut() {
            o.nudges.clear();
        }
    }

    /// Make the overseer take up an ambition right now.
    pub fn force_ambition(&mut self, a: Ambition) {
        let Some(mut o) = self.overseer.take() else { return };
        if o.base.is_some() {
            self.begin_ambition(&mut o, a);
            Self::note_as(&mut o, self.tick, NoteKind::Command, format!("You talked it into: {}.", a.name()));
        }
        self.overseer = Some(o);
    }

    fn per_head(&self, kinds: &[ItemKind]) -> f64 {
        let n: usize = kinds.iter().map(|&k| self.stock_of(k)).sum();
        n as f64 / self.alive_count().max(1) as f64
    }

    pub(super) fn food_per_head(&self) -> f64 {
        self.per_head(&[ItemKind::Meal, ItemKind::Plant, ItemKind::Meat, ItemKind::Fish])
    }

    /// Daily: keep at the current ambition, or settle on a new one.
    pub(super) fn ov_ambition(&mut self, o: &mut Overseer) {
        let tick = self.tick;
        let since = tick.saturating_sub(o.ambition_since);
        let finished = o.ambition.map_or(true, |a| since >= SEASON || (since >= DAY * 10 && self.ambition_met(o, a)));
        if finished {
            self.choose_ambition(o);
        }
        if let Some(a) = o.ambition {
            self.pursue(o, a);
        }
    }

    /// Whether the current ambition has got what it wanted.
    fn ambition_met(&self, o: &Overseer, a: Ambition) -> bool {
        match a {
            Ambition::Provision => self.food_per_head() >= 8.0 && self.per_head(&[ItemKind::Drink]) >= 5.0,
            Ambition::Fortify => o.did("stonetraps") && o.did("yard"),
            Ambition::Muster => {
                let soldiers = self.dwarves.iter().filter(|d| d.alive && d.soldier).count();
                soldiers >= self.muster_target(o)
            }
            Ambition::Prosper => self.wealth() >= o.ambition_mark.saturating_mul(13) / 10,
            Ambition::Delve => o.done.iter().filter(|d| d.starts_with("shaft")).count() as u32 >= o.ambition_mark + 2,
            Ambition::Expand | Ambition::Revel => o.wants.is_empty() && o.rooms.iter().all(|r| r.stage == 1),
            Ambition::Venture => o.did(&format!("ventured{}", o.ambition_since)) && self.expedition.is_none(),
            Ambition::Architect => o.did(&format!("grand{}", o.ambition_since)) && o.rooms.iter().all(|r| r.stage == 1),
        }
    }

    fn choose_ambition(&mut self, o: &mut Overseer) {
        let brain = self.brain(o);
        let mut r = self.rng.range(0, 10_000) as f64 / 10_000.0;
        let mut pick = None;
        for &(a, p) in &brain.odds {
            if p > 0.0 && r < p {
                pick = Some(a);
                break;
            }
            r -= p;
        }
        let Some(pick) = pick.or_else(|| brain.odds.iter().filter(|x| x.1 > 0.0).max_by(|a, b| a.1.total_cmp(&b.1)).map(|x| x.0)) else { return };
        self.begin_ambition(o, pick);
    }

    fn begin_ambition(&mut self, o: &mut Overseer, pick: Ambition) {
        let tick = self.tick;
        o.ambition = Some(pick);
        o.ambition_since = tick;
        o.ambition_mark = match pick {
            Ambition::Prosper => self.wealth(),
            Ambition::Delve => o.done.iter().filter(|d| d.starts_with("shaft")).count() as u32,
            _ => 0,
        };
        o.ambitions.push((tick, pick));
        self.msg(pick.announce(), (150, 210, 255));
        Self::note_as(o, tick, NoteKind::Ambition, format!("Set its heart on: {}.", pick.name()));
        // What some ambitions want dug.
        let has = |o: &Overseer, k: RoomUse| o.rooms.iter().filter(|r| r.kind == k).count();
        match pick {
            Ambition::Expand => {
                o.wants.extend([RoomUse::Dormitory, RoomUse::Stockpile, RoomUse::Dining]);
                if self.alive_count() >= 20 {
                    o.wants.push(RoomUse::Workshops);
                }
            }
            Ambition::Revel => {
                if has(o, RoomUse::Tavern) < 1 + self.alive_count() / 25 {
                    o.wants.push(RoomUse::Tavern);
                }
                if has(o, RoomUse::Temple) == 0 {
                    o.wants.push(RoomUse::Temple);
                }
            }
            Ambition::Muster if has(o, RoomUse::Barracks) == 0 => o.wants.push(RoomUse::Barracks),
            _ => {}
        }
    }

    pub(super) fn muster_target(&self, o: &Overseer) -> usize {
        let adults = self.dwarves.iter().filter(|d| d.alive && !d.is_child()).count();
        ((adults as f64 * 0.35).ceil() as usize).max(4).min(adults.saturating_sub(4)).max(if o.style == Style::Reckless { 0 } else { 2 })
    }

    /// A day's work towards the ambition.
    fn pursue(&mut self, o: &mut Overseer, a: Ambition) {
        let tick = self.tick;
        let e = self.embark;
        match a {
            Ambition::Provision => {
                if self.ov_more_fields(o) {
                    Self::note_as(o, tick, NoteKind::Ambition, "Sowed new fields.".into());
                }
                let fishers = self.dwarves.iter().filter(|d| d.alive && d.has(Labor::Fishing)).count();
                if fishers < 2 {
                    if let Some(id) = self.dwarves.iter().filter(|d| d.alive && !d.is_child() && !d.soldier && !d.has(Labor::Fishing)).max_by_key(|d| d.skills[Labor::Fishing.idx()]).map(|d| d.id) {
                        self.set_labor(id, Labor::Fishing, true);
                    }
                }
                if self.buildings.values().all(|b| b.kind != BuildingKind::Butcher) {
                    if let Some(p) = self.ov_spot(BuildingKind::Butcher, e.offset(6, 6, 0), 16) {
                        self.place_building(BuildingKind::Butcher, p);
                    }
                }
            }
            Ambition::Fortify => {
                self.set_stone_traps(o);
                self.plan_yard(o);
            }
            Ambition::Muster => {
                if self.squads.len() < 2 && self.dwarves.iter().filter(|d| d.alive && d.soldier).count() >= 6 {
                    self.new_squad();
                    let name = self.squads.last().unwrap().name.clone();
                    Self::note_as(o, tick, NoteKind::Ambition, format!("Raised a second squad, {}.", name));
                }
            }
            Ambition::Prosper => {
                self.ov_mine(o, 30, 30);
                if self.wealth() >= 30_000 && o.style != Style::Reckless && !o.did("monument") {
                    if let Some(p) = self.monument_spot(o) {
                        self.place_building(BuildingKind::Monument, p);
                        o.done.push("monument".into());
                        Self::note_as(o, tick, NoteKind::Ambition, "Began a great monument.".into());
                    }
                }
                self.ov_detail(o, true);
            }
            Ambition::Delve => self.ov_shaft(o),
            Ambition::Expand => {}
            Ambition::Revel => self.ov_detail(o, true),
            Ambition::Venture => {
                if self.ov_venture(o, 0.5, true) {
                    o.done.push(format!("ventured{}", o.ambition_since));
                }
            }
            Ambition::Architect => {
                let key = format!("grand{}", o.ambition_since);
                if !o.did(&key) {
                    o.done.push(key);
                    o.wants.insert(0, RoomUse::Grand);
                }
                self.ov_detail(o, true);
            }
        }
    }

    /// Stone-fall traps further down the corridor (boulders are plenty), once.
    pub(super) fn set_stone_traps(&mut self, o: &mut Overseer) {
        let base = o.base.unwrap();
        // (The gate's drawbridge and lever have the first mechanisms.)
        let gate_first = o.style != Style::Reckless && !o.did("linked");
        if !o.did("stonetraps") && !gate_first && self.stock_of(ItemKind::Mechanism) >= 2 && self.stock_of(ItemKind::Boulder) >= 2 {
            let mut n = 0;
            // (Past the trap hall's plates and weapon traps: gatehouse.rs.)
            for dx in 12..15 {
                for dy in 0..2 {
                    if self.place_building(BuildingKind::StoneTrap, base.offset(dx, dy, 0)) {
                        n += 1;
                    }
                }
            }
            if n > 0 {
                o.done.push("stonetraps".into());
                Self::note_as(o, self.tick, NoteKind::Gate, format!("Set {} stone-fall traps deeper in the corridor.", n));
            }
        }
    }

    /// A walled yard round the top of the stair, with a door in it, once.
    /// The door goes in the middle of a side where the ground runs level
    /// through it and on to the wagon: on rough land, walls on the three
    /// sides that can take them and a door that can't be hung (the ground
    /// falling away under it) shut the stair in, and the fort out of its
    /// own halls.
    pub(super) fn plan_yard(&mut self, o: &mut Overseer) {
        let top = o.top.unwrap();
        if o.did("yard") {
            return;
        }
        let r = 3;
        let e = self.embark;
        let gate = [(-1, 0), (0, 1), (0, -1), (1, 0)]
            .iter()
            .map(|&(ux, uy)| (top.offset(ux * r, uy * r, 0), top.offset(ux * (r + 1), uy * (r + 1), 0)))
            .find(|&(g, out)| {
                self.world.walkable(g)
                    && self.world.walkable(out)
                    && self.can_place(BuildingKind::Door, g)
                    && path::astar(&self.world, top, g, 2_000, Mover::Dwarf).is_some()
                    && path::astar(&self.world, out, e, 20_000, Mover::Dwarf).is_some()
            })
            .map(|x| x.0);
        o.done.push("yard".into());
        let Some(gate) = gate else {
            Self::note_ov(o, self.tick, "Meant to wall the yard round the stair, but the ground gives it no level way in.".into());
            return;
        };
        let mut n = 0;
        for (a, b) in [
            (top.offset(-r, -r, 0), top.offset(r, -r, 0)),
            (top.offset(-r, r, 0), top.offset(r, r, 0)),
            (top.offset(r, -r + 1, 0), top.offset(r, r - 1, 0)),
            (top.offset(-r, -r + 1, 0), top.offset(-r, r - 1, 0)),
        ] {
            for p in box_iter(a, b) {
                if p != gate {
                    n += self.plan_constructions(p, p, super::ConstructKind::Wall, super::BuildMat::Any);
                }
            }
        }
        self.place_building(BuildingKind::Door, gate);
        Self::note_as(o, self.tick, NoteKind::Gate, format!("Planned a walled yard round the stair ({} wall sections) with a gate.", n));
    }

    /// Work orders the ambition adds or raises.
    pub(super) fn ambition_orders(&self, o: &Overseer, orders: &mut Vec<(Recipe, u32)>) {
        let pop = self.alive_count() as u32;
        let raise = |orders: &mut Vec<(Recipe, u32)>, r: Recipe, keep: u32| match orders.iter_mut().find(|x| x.0 == r) {
            Some(x) => x.1 = x.1.max(keep),
            None => orders.push((r, keep)),
        };
        match o.ambition {
            Some(Ambition::Provision) => {
                raise(orders, Recipe::Brew, 40 + pop * 3);
                raise(orders, Recipe::Cook, 20 + pop * 2);
                raise(orders, Recipe::Mill, 10);
            }
            Some(Ambition::Fortify) => {
                raise(orders, Recipe::Mechanism, 10);
                raise(orders, Recipe::WoodCage, 8);
                raise(orders, Recipe::Blocks, 30);
            }
            Some(Ambition::Muster) => {
                raise(orders, Recipe::WoodCrossbow, 6);
                raise(orders, Recipe::Bolts, 16);
                raise(orders, Recipe::LeatherArmor, 4);
                raise(orders, Recipe::LeatherHelm, 4);
                raise(orders, Recipe::Tan, 4);
                raise(orders, Recipe::ForgeSpear, 3);
                raise(orders, Recipe::ForgeHelm, 4);
                raise(orders, Recipe::ForgeBreastplate, 3);
                raise(orders, Recipe::ForgeShield, 3);
                raise(orders, Recipe::SmeltOre, 10);
            }
            Some(Ambition::Prosper) => {
                raise(orders, Recipe::StoneCraft, 30);
                raise(orders, Recipe::CutGem, 10);
                raise(orders, Recipe::BoneCraft, 10);
                raise(orders, Recipe::LeatherCraft, 6);
                raise(orders, Recipe::StoneTable, 4);
                raise(orders, Recipe::StoneChair, 4);
                raise(orders, Recipe::SmeltOre, 10);
            }
            Some(Ambition::Revel) => raise(orders, Recipe::Brew, 60 + pop * 3),
            Some(Ambition::Expand) => {
                raise(orders, Recipe::WoodBed, 6);
                raise(orders, Recipe::WoodTable, 4);
                raise(orders, Recipe::WoodChair, 4);
                raise(orders, Recipe::WoodDoor, 4);
            }
            _ => {}
        }
    }

    /// Whether a delver may dig into the caverns' reach yet (after its
    /// first four months).
    pub(super) fn delving_deep(&self) -> bool {
        self.tick as f64 >= self.knob("delve.caverns_day", 120.0) * DAY as f64
    }

    /// Sink the exploring shaft another few levels, with a tunnel at the bottom.
    pub(super) fn ov_shaft(&mut self, o: &mut Overseer) {
        let base = o.base.unwrap();
        // Inside the gate, so nothing comes down the stair and straight into
        // the deep (older forts keep the shaft they dug beside the stair).
        let old = base.offset(-1, 0, -1);
        let shaft = if self.world.kind(old).map_or(false, |k| matches!(k, TileKind::StairUD | TileKind::StairDown)) { base.offset(-1, 0, 0) } else { base.offset(11, 1, 0) };
        let deep = (1..=base.z).rev().take_while(|&z| self.world.kind(Pos::new(shaft.x, shaft.y, z)).map_or(false, |k| matches!(k, TileKind::StairUD | TileKind::StairDown))).last().unwrap_or(base.z);
        let digging = self.designations.iter().filter(|p| p.x == shaft.x && p.y == shaft.y).count();
        // Not into the caverns' reach in the fort's first months: a shaft
        // that opens one in the first weeks lets the spiders up before
        // there is a soldier to meet them.
        let floor = if self.delving_deep() { 2 } else { self.world.caverns.iter().map(|c| c.1).max().unwrap_or(0) + 2 };
        if deep > 3 && deep - 1 >= floor && digging == 0 && self.designations.len() < 80 {
            let to = Pos::new(shaft.x, shaft.y, (deep - 4).max(2).max(floor));
            self.designate(Pos::new(shaft.x, shaft.y, deep), to, Desig::StairUD);
            // Tunnels west and north-south only, well clear of the halls to the east.
            self.designate(to.offset(-14, 0, 0), to, Desig::Dig);
            self.designate(to.offset(0, -12, 0), to.offset(0, 12, 0), Desig::Dig);
            let key = format!("shaft{}", to.z);
            if !o.did(&key) {
                o.done.push(key);
                Self::note_as(o, self.tick, NoteKind::Dug, format!("Sank the shaft to level {}.", to.z));
            }
        }
        self.ov_mine(o, 20, 14);
    }

    /// Smooth, then engrave, the halls folk gather in.
    pub(super) fn ov_detail(&mut self, o: &mut Overseer, engrave: bool) {
        let detailing = self.designations.iter().filter(|p| matches!(self.world.t(**p).desig, Desig::Smooth | Desig::Engrave)).count();
        if detailing >= 40 {
            return;
        }
        let halls: Vec<_> = o.rooms.iter().filter(|r| r.stage == 1 && matches!(r.kind, RoomUse::Dining | RoomUse::Tavern | RoomUse::Temple | RoomUse::Meeting)).cloned().collect();
        for r in halls {
            let smooth = format!("engrave{}", r.a.x * 1000 + r.a.y + r.a.z * 1_000_000);
            if !o.did(&smooth) {
                self.designate_prio(r.a.offset(-1, -1, 0), r.b.offset(1, 1, 0), Desig::Smooth, 6);
                o.done.push(smooth);
                return;
            }
            let carve = format!("carve{}", r.a.x * 1000 + r.a.y + r.a.z * 1_000_000);
            if engrave && !o.did(&carve) && self.tick > YEAR / 3 {
                self.designate_prio(r.a.offset(-1, -1, 0), r.b.offset(1, 1, 0), Desig::Engrave, 6);
                o.done.push(carve);
                return;
            }
        }
    }

    /// Send the first squad on the most worthwhile expedition it can likely
    /// win. True if they went.
    pub(super) fn ov_venture(&mut self, o: &mut Overseer, bold: f64, certain: bool) -> bool {
        if self.expedition.is_some() || self.alert || self.squads.is_empty() || o.style == Style::Reckless && !certain {
            return false;
        }
        let members = self.squads[0].members.clone();
        let strength = self.squad_strength(&members);
        if members.len() < 3 || strength < 8 || self.tick < YEAR / 2 {
            return false;
        }
        // Someone must hold the gate while they're away: another squad at
        // home, or the fort goes without an expedition.
        let at_home: usize = self.squads.iter().skip(1).map(|s| s.members.len()).sum();
        if at_home < 2 {
            return false;
        }
        // The cautious leave the beasts in their lairs.
        let cautious = o.has(super::personality::Trait::Cautious);
        let pick = self.destinations().into_iter().filter(|d| !(cautious && matches!(d.goal, Goal::Beast(_))) && self.round_odds(&members, d) >= bold).max_by_key(|d| {
            let worth = match d.goal {
                Goal::Beast(_) => 3,
                Goal::Raid => 2,
                Goal::Ruin => 1,
            };
            (worth, -d.dist)
        });
        let Some(d) = pick else { return false };
        if !certain && !self.rng.chance(0.2) {
            return false;
        }
        // It could win; but is the squad fit to go, and can the fort spare it?
        if let Some((kind, why)) = self.venture_veto(o, &members) {
            // Once a season for each kind of reason, whatever the numbers say.
            let text = format!("Kept the squad home: {}.", why);
            if !o.notes.iter().rev().any(|(t, n)| n.starts_with("Kept the squad home:") && n.contains(kind) && self.tick < t + SEASON) {
                Self::note_ov(o, self.tick, text);
            }
            return false;
        }
        let name = d.name.clone();
        if self.send_expedition(0, d).is_ok() {
            Self::note_ov(o, self.tick, format!("Sent the squad to {}.", name));
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_overseer_moves_from_ambition_to_ambition() {
        let mut g = Game::new(5);
        g.set_overseer(Some(Style::Builder));
        for _ in 0..(SEASON * 3) {
            g.step();
            if g.alive_count() == 0 {
                break;
            }
        }
        let o = g.overseer.as_ref().unwrap();
        assert!(o.ambitions.len() >= 2, "only {:?}", o.ambitions);
        assert!(o.ambition.is_some());
        assert!(g.log.iter().any(|l| l.text.starts_with("The overseer")), "ambitions should be announced");
        // Never the same twice running is likely, but not forced; any two in a row
        // should at least both be real ambitions.
        assert!(o.ambitions.iter().all(|a| Ambition::ALL.contains(&a.1)));
    }

    #[test]
    fn every_sense_is_on_the_brain_screen() {
        let mut g = Game::new(5);
        g.set_overseer(Some(Style::Steady));
        let b = g.overseer_brain().unwrap();
        // Each sense once, in the screen's order; the ones no item has wired yet are quiet.
        let order: Vec<Sense> = b.senses.iter().map(|s| s.0).collect();
        assert_eq!(order, Sense::ALL.to_vec());
        for s in [Sense::Lesson, Sense::Prowlers, Sense::Unarmed, Sense::Wounded] {
            assert_eq!(b.senses.iter().find(|x| x.0 == s).unwrap().1, 0.0);
            assert!(!s.name().is_empty());
        }
    }
}
