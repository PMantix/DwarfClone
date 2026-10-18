//! Expeditions: send a squad off the map to explore a ruin, raid an enemy
//! or hunt a beast in its lair. They come back days later with loot,
//! wounds and a story, or they don't come back.

use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Goal {
    Ruin,
    Raid,
    /// The historical figure to slay.
    Beast(u32),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Destination {
    pub goal: Goal,
    pub site: u32,
    /// "the ruins of Ashfall", "the lair of Emberwing the dragon".
    pub name: String,
    /// World map cells away.
    pub dist: i32,
    /// 1 (easy) to 8 (deadly).
    pub danger: i32,
}

impl Destination {
    /// Days away, there and back with a day at the far end.
    pub fn days(&self) -> u64 {
        2 * ((self.dist as u64 + 3) / 4).max(1) + 1
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Expedition {
    pub squad: String,
    pub members: Vec<Dwarf>,
    pub dest: Destination,
    pub back: u64,
    /// When they reach the far end, where they fight up to three times
    /// (0 in an old save: the fighting is done when they're due back).
    #[serde(default)]
    pub arrive: u64,
    /// The fights so far: won or lost. Two won is a triumph, two lost a defeat.
    #[serde(default)]
    pub rounds: Vec<bool>,
    /// Members who fell; they're carried home.
    #[serde(default)]
    pub fallen: Vec<u32>,
    /// Their nerve failed after a lost fight and they started home early.
    #[serde(default)]
    pub turned_back: bool,
    /// Who leads them (the best tactician).
    #[serde(default)]
    pub leader: Option<u32>,
    /// Drink and food taken from the stores for the road.
    #[serde(default)]
    pub rations: u32,
    /// The rations held both drink and food, a ration a head at the least.
    #[serde(default)]
    pub provisioned: bool,
}

impl Expedition {
    fn won(&self) -> usize {
        self.rounds.iter().filter(|&&w| w).count()
    }

    /// Two fights won or lost, or turned back: no more fighting.
    pub fn decided(&self) -> bool {
        self.won() >= 2 || self.rounds.len() - self.won() >= 2 || self.turned_back
    }

    /// When the next fight comes: a third of a day apart at the far end.
    fn next_fight(&self) -> u64 {
        if self.arrive == 0 {
            self.back
        } else {
            self.arrive + self.rounds.len() as u64 * DAY / 3
        }
    }

    /// Those still on their feet.
    fn standing(&self) -> Vec<&Dwarf> {
        self.members.iter().filter(|d| !self.fallen.contains(&d.id)).collect()
    }

    /// Enough on their backs to keep them fed: some drink, some food, and a
    /// ration a head at the least. An expedition from an old save, sent
    /// before rations were packed, counts as fed.
    fn fed(&self) -> bool {
        self.arrive == 0 || self.provisioned
    }

    /// The leader's fighting skill, while the leader still stands.
    fn leader_level(&self) -> u32 {
        self.standing().iter().find(|d| Some(d.id) == self.leader).map_or(0, |d| crate::dwarf::skill_level(d.fight_xp))
    }
}

/// Whether `drink` and `food` rations feed `n` dwarves on the road: some of
/// each, and one a head at the least.
fn provisioned(drink: usize, food: usize, n: usize) -> bool {
    drink > 0 && food > 0 && drink + food >= n
}

/// One dwarf's fighting strength away from home.
fn strength_of(d: &Dwarf) -> i32 {
    crate::dwarf::skill_level(d.fight_xp) as i32 + 1 + d.held.is_some() as i32 + d.armor.len() as i32 / 2
}

/// The chance a squad of `strength` beats a place of `danger`, before
/// rations and leadership.
fn win_chance(strength: i32, danger: i32) -> f64 {
    strength as f64 / (strength as f64 + danger as f64 * 3.0)
}

/// The chance of winning two fights of three, each won with chance `p`,
/// when after a lost fight the squad stays with chance `stay`.
fn triumph_chance(p: f64, stay: f64) -> f64 {
    // Won-won, won-lost-won, lost-won-won; the last two need them to stay.
    p * p * (1.0 + 2.0 * (1.0 - p) * stay)
}

/// The best tactician among them: fighting skill, and diligence to read the ground.
fn tactician(members: &[&Dwarf]) -> Option<u32> {
    members.iter().max_by_key(|d| (crate::dwarf::skill_level(d.fight_xp) as i32 + d.personality.diligence / 25, d.id)).map(|d| d.id)
}

/// The chance that a squad's nerve fails after a lost fight: anxious dwarves
/// and a leader they think little of turn back sooner, and each fallen
/// comrade makes it likelier.
fn nerve_fails(standing: &[&Dwarf], leader: Option<u32>, nerve: f64, fallen: usize) -> f64 {
    let n = standing.len().max(1) as f64;
    let anxiety = standing.iter().map(|d| d.personality.anxiety as f64).sum::<f64>() / n;
    let others: Vec<f64> = leader.map_or(Vec::new(), |l| standing.iter().filter(|d| d.id != l).map(|d| d.regard(l) as f64).collect());
    let regard = if others.is_empty() { 0.0 } else { others.iter().sum::<f64>() / others.len() as f64 };
    (nerve + anxiety / 100.0 - regard / 200.0 + 0.1 * fallen as f64).clamp(0.0, 1.0)
}

/// A wound bound up in the field: it bleeds half as fast.
fn field_dressing(body: &mut crate::body::Body) {
    body.bleeding = (body.bleeding + 1) / 2;
}

/// "one", "two" ... "twelve", then digits.
fn number_word(n: usize) -> String {
    const W: [&str; 13] = ["no", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve"];
    W.get(n).map_or_else(|| n.to_string(), |w| w.to_string())
}

/// "all of them have", "half of them have", "two of them have"...
fn share(k: usize, n: usize) -> String {
    if k >= n {
        "all of them have".into()
    } else if k * 2 == n {
        "half of them have".into()
    } else if k == 1 {
        "one of them has".into()
    } else {
        format!("{} of them have", k)
    }
}

impl Game {
    /// Places a squad could go, nearest first.
    pub fn destinations(&self) -> Vec<Destination> {
        let (sx, sy) = self.site;
        let ow = &self.overworld;
        let mut v = Vec::new();
        for s in &ow.sites {
            let dist = (s.x - sx).abs().max((s.y - sy).abs());
            if dist == 0 || dist > 30 {
                continue;
            }
            if s.kind == crate::overworld::SiteKind::Lair {
                if let Some(f) = ow.figures.iter().find(|f| f.lair == Some(s.id) && f.alive()) {
                    let danger = if f.kind == "dragon" { 8 } else { 7 };
                    v.push(Destination { goal: Goal::Beast(f.id), site: s.id, name: format!("the lair of {} the {}", f.name, f.kind), dist, danger });
                }
            } else if s.ruined {
                v.push(Destination { goal: Goal::Ruin, site: s.id, name: format!("the ruins of {}", s.name), dist, danger: 2 + dist / 12 });
            } else if let Some(c) = s.civ {
                let war = ow.civs.get(c as usize).map_or(false, |civ| civ.alive && civ.race == Race::Goblin)
                    || self.civ.map_or(false, |ours| ow.civ(ours).enemies.contains(&c) || ow.civ(c).enemies.contains(&ours));
                if war && Some(c) != self.civ {
                    v.push(Destination { goal: Goal::Raid, site: s.id, name: s.name.clone(), dist, danger: (3 + s.pop as i32 / 150).min(7) });
                }
            }
        }
        v.sort_by_key(|d| (d.dist, d.site));
        // The nearest few of each kind.
        let mut out = Vec::new();
        for (kind, keep) in [(0, 5), (1, 4), (2, 4)] {
            out.extend(
                v.iter()
                    .filter(|d| match d.goal {
                        Goal::Ruin => kind == 0,
                        Goal::Raid => kind == 1,
                        Goal::Beast(_) => kind == 2,
                    })
                    .take(keep)
                    .cloned(),
            );
        }
        out.sort_by_key(|d| (d.dist, d.site));
        out
    }

    /// A squad's fighting strength for an expedition.
    pub fn squad_strength(&self, ids: &[u32]) -> i32 {
        self.dwarves.iter().filter(|d| ids.contains(&d.id) && d.alive && !d.is_child()).map(strength_of).sum()
    }

    /// The chance that squad `ids` comes home from `dest` in triumph, as the
    /// expedition picker shows it.
    pub fn odds(&self, ids: &[u32], dest: &Destination) -> f64 {
        let squad: Vec<&Dwarf> = self.dwarves.iter().filter(|d| ids.contains(&d.id) && d.alive && !d.is_child()).collect();
        let stay = 1.0 - nerve_fails(&squad, tactician(&squad), self.knob("exp.nerve", 0.5), 0);
        triumph_chance(self.round_odds(ids, dest), stay)
    }

    /// The chance that squad `ids` wins one fight at `dest`.
    pub fn round_odds(&self, ids: &[u32], dest: &Destination) -> f64 {
        let squad: Vec<&Dwarf> = self.dwarves.iter().filter(|d| ids.contains(&d.id) && d.alive && !d.is_child()).collect();
        let lead = tactician(&squad).and_then(|l| squad.iter().find(|d| d.id == l)).map_or(0, |d| crate::dwarf::skill_level(d.fight_xp));
        let (drink, food) = self.ration_stock();
        let fed = provisioned(drink.min(squad.len()), food.min(squad.len()), squad.len());
        self.fight_chance(self.squad_strength(ids), dest.danger, fed, lead)
    }

    /// One fight's odds: strength against danger, worse on an empty stomach
    /// (knob exp.hungry), better with a good tactician (knob exp.lead per
    /// level of the leader's fighting skill).
    fn fight_chance(&self, strength: i32, danger: i32, fed: bool, leader_level: u32) -> f64 {
        let mut p = win_chance(strength, danger);
        if !fed {
            p *= self.knob("exp.hungry", 0.85);
        }
        p += self.knob("exp.lead", 0.03) * leader_level as f64;
        p.clamp(0.1, 0.95)
    }

    /// Free drink and food in the stores (meals first): what rations could be had.
    fn ration_stock(&self) -> (usize, usize) {
        let drink = self.items.values().filter(|it| it.kind == ItemKind::Drink && it.free()).count();
        let food = self.items.values().filter(|it| it.is_food() && it.free()).count();
        (drink, food)
    }

    /// Take rations for `n` dwarves from the stores: a drink and a meal each
    /// where there are meals, other food where there aren't.
    fn take_rations(&mut self, n: usize) -> (usize, usize) {
        let mut take: Vec<u32> = self.items.values().filter(|it| it.kind == ItemKind::Drink && it.free()).map(|it| it.id).take(n).collect();
        let drink = take.len();
        let mut food: Vec<(bool, u32)> = self.items.values().filter(|it| it.is_food() && it.free()).map(|it| (it.kind != ItemKind::Meal, it.id)).collect();
        food.sort();
        take.extend(food.into_iter().take(n).map(|f| f.1));
        for id in &take {
            self.items.remove(id);
        }
        (drink, take.len() - drink)
    }

    /// Armed for a fight away from home: (a metal weapon in hand, some armour on).
    /// A stand-in for the readiness check soldiers' kit will get.
    fn kitted(&self, d: &Dwarf) -> (bool, bool) {
        let weapon = d.held.and_then(|id| self.items.get(&id)).map_or(false, |it| {
            it.kind != ItemKind::Crossbow && it.kind.weapon_damage().is_some() && matches!(it.mat, Mat::Copper | Mat::Bronze | Mat::Iron | Mat::Steel)
        });
        (weapon, !d.armor.is_empty())
    }

    /// Why the overseer keeps squad `ids` home even though it could win
    /// (None: nothing stops it). The squad must be armed and armoured, the
    /// fort big enough to spare them, the last siege a season gone and the
    /// wounded few.
    ///
    /// The first of the pair is a fixed phrase found in the text, the kind of
    /// veto: the overseer says each kind once a season, though the numbers
    /// in the text change from day to day.
    pub(super) fn venture_veto(&self, o: &super::overseer::Overseer, ids: &[u32]) -> Option<(&'static str, String)> {
        let squad: Vec<&Dwarf> = self.dwarves.iter().filter(|d| ids.contains(&d.id) && d.alive).collect();
        let n = squad.len();
        let unarmed = squad.iter().filter(|d| !self.kitted(d).0).count();
        let bare = squad.iter().filter(|d| !self.kitted(d).1).count();
        if unarmed > 0 || bare > 0 {
            return Some(if bare >= unarmed {
                ("no armour", format!("{} no armour", share(bare, n)))
            } else {
                ("no metal weapon", format!("{} no metal weapon", share(unarmed, n)))
            });
        }
        let adults = self.dwarves.iter().filter(|d| d.alive && !d.is_child()).count();
        if (adults as f64) < self.knob("exp.min_adults", 12.0) {
            return Some(("grown dwarves strong", format!("the fort is only {} grown dwarves strong", adults)));
        }
        if o.last_siege > 0 && self.tick < o.last_siege + SEASON {
            return Some(("goblins were at the gate", format!("the goblins were at the gate only {} days ago", (self.tick - o.last_siege) / DAY)));
        }
        let wounded = self.dwarves.iter().filter(|d| d.alive && d.body.needs_rest()).count();
        if wounded >= 2 {
            return Some(("lie wounded", format!("{} dwarves lie wounded", wounded)));
        }
        None
    }

    /// Send squad `sq` off to `dest`.
    pub fn send_expedition(&mut self, sq: usize, dest: Destination) -> Result<String, String> {
        if self.expedition.is_some() {
            return Err("An expedition is already away.".into());
        }
        let Some(squad) = self.squads.get(sq) else { return Err("No such squad.".into()) };
        let name = squad.name.clone();
        let ids = squad.members.clone();
        let mut members = Vec::new();
        let mut i = 0;
        while i < self.dwarves.len() {
            let d = &self.dwarves[i];
            if ids.contains(&d.id) && d.alive && !d.insane && !d.is_child() {
                let mut d = self.dwarves.remove(i);
                self.release_task(&mut d);
                if let Some(it) = d.carrying.take() {
                    self.drop_item(it, d.pos);
                }
                d.path.clear();
                members.push(d);
            } else {
                i += 1;
            }
        }
        if members.is_empty() {
            return Err(format!("{} has nobody to send.", name));
        }
        let days = dest.days();
        let (drink, food) = self.take_rations(members.len());
        let rations = (drink + food) as u32;
        let provisioned = provisioned(drink, food, members.len());
        let packs = if rations == 0 { "They go without rations.".to_string() } else { format!("They carry {} rations.", rations) };
        let text = format!("{} set out for {}. They should be back in about {} days. {}", name, dest.name, days, packs);
        self.msg(&text, (200, 220, 255));
        let fort = self.fortress_name.clone();
        self.legend(format!("{} of {} set out for {}.", name, fort, dest.name));
        let leader = tactician(&members.iter().collect::<Vec<_>>());
        let arrive = self.tick + (days - 1) / 2 * DAY;
        self.count("exp_sent", 1);
        self.expedition = Some(Expedition { squad: name, members, dest, back: self.tick + days * DAY, arrive, rounds: Vec::new(), fallen: Vec::new(), turned_back: false, leader, rations, provisioned });
        Ok(text)
    }

    /// Now and then: a fight at the far end, or home again.
    pub(super) fn expedition_tick(&mut self) {
        let Some(mut ex) = self.expedition.take() else { return };
        while !ex.decided() && self.tick >= ex.next_fight() {
            self.expedition_fight(&mut ex);
        }
        if self.tick < ex.back {
            self.expedition = Some(ex);
            return;
        }
        self.expedition_returns(ex);
    }

    /// One fight at the far end. A lost one hurts one or two of them for
    /// real (bound up in the field, they come home bleeding), may cost a life,
    /// and tests their nerve: if it fails they turn for home.
    fn expedition_fight(&mut self, ex: &mut Expedition) {
        let standing: Vec<usize> = (0..ex.members.len()).filter(|&i| !ex.fallen.contains(&ex.members[i].id)).collect();
        if standing.is_empty() {
            ex.turned_back = true;
            return;
        }
        let strength: i32 = standing.iter().map(|&i| strength_of(&ex.members[i])).sum();
        let won = self.rng.chance(self.fight_chance(strength, ex.dest.danger, ex.fed(), ex.leader_level()));
        ex.rounds.push(won);
        if won {
            return;
        }
        for _ in 0..self.rng.range(1, 3) {
            let i = *self.rng.pick(&standing);
            let dmg = self.rng.range(8, 15);
            let body = &mut ex.members[i].body;
            let part = match body.pick_part(&mut self.rng) {
                crate::body::Part::Head => crate::body::Part::UpperBody,
                p => p,
            };
            if body.wound(part, dmg, true) == crate::body::Wound::Fatal || body.dead {
                // Dead where they fell: counted among the fallen, past any cap.
                let id = ex.members[i].id;
                if !ex.fallen.contains(&id) {
                    ex.fallen.push(id);
                    self.count("exp_fallen", 1);
                }
                continue;
            }
            field_dressing(body);
        }
        let standing: Vec<usize> = standing.into_iter().filter(|&i| !ex.fallen.contains(&ex.members[i].id)).collect();
        if standing.is_empty() {
            ex.turned_back = true;
            return;
        }
        // Never more than half of them fall, except to a beast in its lair.
        let cap = if matches!(ex.dest.goal, Goal::Beast(_)) { ex.members.len() } else { ex.members.len() / 2 };
        if ex.fallen.len() < cap && self.rng.chance(ex.dest.danger as f64 * self.knob("exp.death", 0.04)) {
            // The leader is the last to fall.
            let pool: Vec<u32> = standing.iter().map(|&i| ex.members[i].id).filter(|&id| Some(id) != ex.leader || standing.len() == 1).collect();
            let id = *self.rng.pick(&pool);
            ex.fallen.push(id);
            self.count("exp_fallen", 1);
        }
        if !ex.decided() {
            let fail = nerve_fails(&ex.standing(), ex.leader, self.knob("exp.nerve", 0.5), ex.fallen.len());
            if self.rng.chance(fail) {
                ex.turned_back = true;
                ex.back = self.tick + (ex.back.saturating_sub(self.tick)) / 2;
            }
        }
    }

    fn expedition_returns(&mut self, ex: Expedition) {
        let won = ex.won() >= 2 && !ex.turned_back;
        let fed = ex.fed();
        let Expedition { squad, members, dest, fallen, turned_back, leader, .. } = ex;
        let entry = self.edge_entry().unwrap_or(self.embark);
        let fort = self.fortress_name.clone();
        let tick = self.tick;
        let mut lost = Vec::new();
        let mut home: Vec<Dwarf> = Vec::new();
        for mut d in members {
            d.pos = self.spot_near(entry, 2);
            d.path.clear();
            if fallen.contains(&d.id) || d.body.dead {
                lost.push(d.name.clone());
                self.kill(&mut d, &format!("fell on the expedition to {}", dest.name));
                self.dwarves.push(d);
                continue;
            }
            if self.rng.chance(0.4) {
                let part = d.body.pick_part(&mut self.rng);
                d.body.wound(part, 3, false);
            }
            d.fight_xp += 150 + dest.danger as u32 * 40;
            if Some(d.id) == leader {
                d.fight_xp += 100;
            }
            if !fed {
                d.hunger = d.hunger.max(HUNGRY);
            }
            d.satisfy(crate::dwarf::NeedKind::Fight, 1000);
            if won {
                d.think(&format!("came home in triumph from {}", dest.name), -40, tick);
            } else {
                d.think(&format!("came home beaten from {}", dest.name), 30, tick);
            }
            home.push(d);
        }
        if home.is_empty() {
            self.msg(&format!("None of {} came back from {}.", squad, dest.name), (255, 90, 90));
            self.legend(format!("{} of {} never returned from {}.", squad, fort, dest.name));
            return;
        }
        let gone = if lost.is_empty() { String::new() } else { format!(" {} did not come back.", lost.len()) };
        if turned_back {
            // Home beaten, but home: the wounded carried between them.
            let wounded = home.iter().filter(|d| d.body.bleeding > 0 || d.body.needs_rest()).count();
            let carrying = if wounded == 0 { String::new() } else { format!(", carrying {} wounded", number_word(wounded)) };
            let lead = leader.and_then(|l| home.iter().find(|d| d.id == l)).map(|d| d.called());
            let called = lead.map_or(String::new(), |n| format!(" {} called the retreat.", n));
            let text = format!("{} turned back from {}{}.{}{}", squad, dest.name, carrying, called, gone);
            self.count("exp_turned_back", 1);
            self.msg(&text, (255, 170, 90));
            self.legend(format!("{} of {} turned back from {}{}.", squad, fort, dest.name, carrying));
            self.spotlight("Turned back", &text, None, (255, 170, 90));
            self.dwarves.extend(home);
            return;
        }
        if !won {
            self.count("exp_beaten", 1);
            let led = leader.and_then(|l| home.iter().find(|d| d.id == l)).map_or(String::new(), |d| format!(" Led by {}.", d.called()));
            self.msg(&format!("{} have returned from {}, beaten.{}{}", squad, dest.name, gone, led), (255, 150, 90));
            self.legend(format!("{} of {} were driven back from {}.", squad, fort, dest.name));
            self.dwarves.extend(home);
            return;
        }
        // Victory: loot, and a name for whoever led them.
        self.count("exp_triumph", 1);
        let lead = (0..home.len())
            .find(|&i| Some(home[i].id) == leader)
            .unwrap_or_else(|| (0..home.len()).max_by_key(|&i| (home[i].fight_xp, home[i].id)).unwrap());
        let mut leader = home.remove(lead);
        let year = self.start_year + (self.tick / YEAR) as i32;
        let (what, loot): (String, Vec<(ItemKind, Mat)>) = match dest.goal {
            Goal::Ruin => {
                self.earn_epithet(&mut leader, "Delver", super::heroes::RANK_FEAT + 1, &format!("explored {}", dest.name));
                ("explored".into(), vec![(ItemKind::Craft, Mat::Gold), (ItemKind::Gem, Mat::Amethyst), (ItemKind::Bar, Mat::Gold), (ItemKind::Craft, Mat::Marble)])
            }
            Goal::Raid => {
                self.earn_epithet(&mut leader, "Bold", super::heroes::RANK_FEAT + 1, &format!("led the raid on {}", dest.name));
                ("raided".into(), vec![(ItemKind::Bar, Mat::Iron), (ItemKind::Bar, Mat::Bronze), (ItemKind::Craft, Mat::Gold), (ItemKind::Bar, Mat::Steel)])
            }
            Goal::Beast(f) => {
                let (name, kind) = self.overworld.figures.get(f as usize).map_or(("the beast".to_string(), "beast".to_string()), |x| (x.name.clone(), x.kind.clone()));
                self.overworld.kill_figure(f, year, format!("slain in its lair by {} of {}", squad, fort));
                let title = if kind == "dragon" { "Dragonslayer".to_string() } else { format!("{}bane", super::heroes::cap_word(&kind)) };
                self.earn_epithet(&mut leader, &title, super::heroes::RANK_BEAST + 2, &format!("slew {} the {} in its lair", name, kind));
                ("slew the beast of".into(), vec![(ItemKind::Craft, Mat::Gold), (ItemKind::Bar, Mat::Gold), (ItemKind::Gem, Mat::Amethyst)])
            }
        };
        let n = self.rng.range(3, 7) + dest.danger / 2;
        for _ in 0..n {
            let (kind, mat) = *self.rng.pick(&loot);
            let p = self.spot_near(entry, 2);
            let id = self.spawn_item(kind, mat, p);
            if let Some(it) = self.items.get_mut(&id) {
                it.quality = self.rng.range(2, 5) as u8;
            }
        }
        if dest.goal == Goal::Ruin && self.rng.chance(0.35) {
            let id = self.spawn_item(ItemKind::Craft, Mat::Gold, entry);
            let name = crate::dwarf::artifact_name(&mut self.rng);
            if let Some(it) = self.items.get_mut(&id) {
                it.quality = crate::items::ARTIFACT;
                it.label = Some(name.clone());
            }
            self.msg(&format!("Among the loot: {}, an artifact of a forgotten age!", name), (255, 215, 60));
            self.empower_artifact(id, None);
        }
        let leader_name = leader.called();
        home.push(leader);
        self.milestone("expedition", &format!("The first expedition home in triumph, from {}", dest.name));
        self.dwarves.extend(home);
        let text = format!("{} have returned in triumph from {} with {} treasures!{} Led by {}, who read the ground well.", squad, dest.name, n, gone, leader_name);
        self.msg(&text, (255, 215, 60));
        self.legend(format!("{} of {}, led by {}, {} {}.", squad, fort, leader_name, what, dest.name));
        self.spotlight("Home in triumph", &text, None, (255, 215, 60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_expedition_goes_away_and_comes_home_with_loot() {
        let mut g = Game::new(3);
        g.creatures.retain(|_, c| c.tame);
        let sq = g.new_squad();
        let ids: Vec<u32> = g.dwarves.iter().filter(|d| !d.is_child()).take(3).map(|d| d.id).collect();
        for &id in &ids {
            g.set_squad_member(sq, id, true);
        }
        for d in g.dwarves.iter_mut().filter(|d| ids.contains(&d.id)) {
            d.fight_xp = 3000;
        }
        let before = g.dwarves.len();
        // What a ruin yields (rations taken along leave the stores, so count the loot itself).
        let loot = |g: &Game| g.items.values().filter(|it| matches!((it.kind, it.mat), (ItemKind::Craft, Mat::Gold) | (ItemKind::Gem, Mat::Amethyst) | (ItemKind::Bar, Mat::Gold) | (ItemKind::Craft, Mat::Marble))).count();
        let items = loot(&g);
        let dest = Destination { goal: Goal::Ruin, site: 0, name: "the ruins of Testhold".into(), dist: 4, danger: 1 };
        assert!(g.send_expedition(sq, dest.clone()).is_ok());
        assert_eq!(g.dwarves.len(), before - 3, "they've left the map");
        assert!(g.send_expedition(sq, dest).is_err(), "one at a time");
        g.expedition_tick();
        assert!(g.expedition.is_some(), "not back yet");
        g.tick = g.expedition.as_ref().unwrap().back;
        g.expedition_tick();
        assert!(g.expedition.is_none());
        assert_eq!(g.dwarves.len(), before, "home again (alive or carried)");
        let text: Vec<&str> = g.log.iter().map(|l| l.text.as_str()).collect();
        assert!(text.iter().any(|t| t.contains("returned") || t.contains("turned back")), "{:?}", text);
        if text.iter().any(|t| t.contains("triumph")) {
            assert!(loot(&g) > items, "loot");
            assert!(g.dwarves.iter().any(|d| d.epithet.as_deref() == Some("Delver")));
        }
    }

    /// A fort a year old with `adults` grown dwarves: squad 0 of `n` veterans
    /// (in iron and a helm when `armed`) and a second squad of two at home.
    fn fort_with_squad(n: usize, adults: usize, armed: bool) -> (Game, Vec<u32>) {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        while g.dwarves.iter().filter(|d| !d.is_child()).count() < adults {
            let mut d = g.dwarves[0].clone();
            d.id = g.new_id();
            d.relations.clear();
            d.held = None;
            d.armor.clear();
            d.carrying = None;
            g.dwarves.push(d);
        }
        while g.dwarves.iter().filter(|d| !d.is_child()).count() > adults {
            let i = g.dwarves.iter().rposition(|d| !d.is_child()).unwrap();
            g.dwarves.remove(i);
        }
        g.tick = YEAR;
        g.alert = false;
        let (a, b) = (g.new_squad(), g.new_squad());
        let ids: Vec<u32> = g.dwarves.iter().filter(|d| !d.is_child()).map(|d| d.id).take(n + 2).collect();
        for (i, &id) in ids.iter().enumerate() {
            g.set_squad_member(if i < n { a } else { b }, id, true);
        }
        let squad: Vec<u32> = ids[..n].to_vec();
        for &id in &squad {
            let at = g.embark;
            let (w, h) = if armed { (Some(g.spawn_item(ItemKind::Sword, Mat::Iron, at)), Some(g.spawn_item(ItemKind::Helm, Mat::Iron, at))) } else { (None, None) };
            for it in [w, h].into_iter().flatten() {
                g.items.get_mut(&it).unwrap().carried_by = Some(id);
            }
            let d = g.dwarves.iter_mut().find(|d| d.id == id).unwrap();
            d.fight_xp = 3000;
            d.held = w;
            d.armor = h.into_iter().collect();
            d.body = crate::body::Body::new(1.0);
        }
        (g, squad)
    }

    fn venture(g: &mut Game, o: &mut super::super::overseer::Overseer) -> bool {
        g.ov_venture(o, 0.3, true)
    }

    #[test]
    fn no_expedition_from_a_weak_fort() {
        use super::super::overseer::{Overseer, Style};
        let said = |o: &Overseer| o.notes.last().map(|n| n.1.clone()).unwrap_or_default();
        // Ten grown dwarves and a squad with neither armour nor a weapon.
        let (mut g, _) = fort_with_squad(3, 10, false);
        let mut o = Overseer::new(Style::Steady);
        assert!(!venture(&mut g, &mut o) && g.expedition.is_none());
        assert_eq!(said(&o), "Kept the squad home: all of them have no armour.");
        // Said once, not every day.
        let notes = o.notes.len();
        g.tick += DAY;
        assert!(!venture(&mut g, &mut o));
        assert_eq!(o.notes.len(), notes);
        // Armed, but the fort is too small to spare them.
        let (mut g, _) = fort_with_squad(3, 10, true);
        assert!(!venture(&mut g, &mut o));
        assert!(said(&o).contains("only 10 grown dwarves"), "{}", said(&o));
        // Armed and twelve strong, but the goblins were here last week.
        let (mut g, _) = fort_with_squad(3, 12, true);
        o.last_siege = g.tick - 7 * DAY;
        assert!(!venture(&mut g, &mut o));
        assert!(said(&o).contains("the goblins were at the gate only 7 days ago"), "{}", said(&o));
        // Said once a season, though the days since change.
        let notes = o.notes.len();
        g.tick += DAY;
        assert!(!venture(&mut g, &mut o));
        assert_eq!(o.notes.len(), notes, "{}", said(&o));
        // Two dwarves resting with wounds.
        o.last_siege = g.tick - SEASON - DAY;
        for d in g.dwarves.iter_mut().filter(|d| !d.soldier).take(2) {
            d.body.wound(crate::body::Part::LeftLeg, 15, false);
        }
        assert!(!venture(&mut g, &mut o));
        assert!(said(&o).contains("2 dwarves lie wounded"), "{}", said(&o));
        // A strong, armed squad from a fort that can spare it goes.
        let (mut g, _) = fort_with_squad(3, 12, true);
        assert!(venture(&mut g, &mut o), "{}", said(&o));
        assert!(g.expedition.is_some() && said(&o).starts_with("Sent the squad to"));
        // A cautious overseer never sends it after a beast.
        let (mut g, _) = fort_with_squad(3, 12, true);
        o.traits = vec![crate::game::personality::Trait::Cautious];
        if venture(&mut g, &mut o) {
            assert!(!matches!(g.expedition.as_ref().unwrap().dest.goal, Goal::Beast(_)));
        }
    }

    /// Squad 0 of a weak fort sent to `dest`, its rng reseeded.
    fn weak_squad_away(seed: u64, dest: &Destination, nerve: f64) -> (Game, u64) {
        let (mut g, squad) = fort_with_squad(3, 12, false);
        g.rng = crate::rng::Rng::new(seed);
        g.knobs.set.insert("exp.nerve".into(), nerve);
        for d in g.dwarves.iter_mut().filter(|d| squad.contains(&d.id)) {
            d.fight_xp = 400; // strength 2 each, 6 in all
            d.personality.anxiety = 50;
        }
        assert_eq!(g.squad_strength(&squad), 6);
        g.send_expedition(0, dest.clone()).unwrap();
        let back = g.expedition.as_ref().unwrap().back;
        (g, back)
    }

    #[test]
    fn a_losing_squad_comes_home_early_with_its_wounded() {
        let dest = Destination { goal: Goal::Raid, site: 0, name: "Testgrot".into(), dist: 8, danger: 7 };
        let (mut seen, mut bleeding) = (0, 0);
        for seed in 0..20 {
            let (mut g, back) = weak_squad_away(seed, &dest, 1.0);
            // To the first fight.
            g.tick = g.expedition.as_ref().unwrap().next_fight();
            g.expedition_tick();
            let ex = g.expedition.as_ref().unwrap();
            if ex.rounds != [false] {
                continue; // won the first fight: not the case under test
            }
            seen += 1;
            assert!(ex.turned_back, "a failing nerve turns them back");
            assert!(ex.back < back, "home before they were due");
            let early = ex.back;
            let before = g.dwarves.len();
            g.tick = early;
            g.expedition_tick();
            assert!(g.expedition.is_none(), "home by {}", early);
            let came: Vec<&Dwarf> = g.dwarves[before..].iter().filter(|d| d.alive).collect();
            assert!(came.len() * 2 >= 3, "at least half came home");
            let fell = came.len() < 3;
            // The one or two hurt come home bleeding (unless the one hurt is the one who fell).
            if came.iter().any(|d| d.body.bleeding > 0) {
                bleeding += 1;
            } else {
                assert!(fell, "with their wounded, still bleeding");
            }
            let text: Vec<&str> = g.log.iter().map(|l| l.text.as_str()).collect();
            assert!(text.iter().any(|t| t.contains("turned back from Testgrot") && (fell || t.contains("carrying"))), "{:?}", text);
            assert!(g.spotlights.iter().any(|s| s.title == "Turned back"));
            assert_eq!(g.annals.counters.get("exp_turned_back"), Some(&1));
        }
        assert!(seen >= 10, "the first fight is mostly lost at these odds ({})", seen);
        assert!(bleeding * 2 > seen, "mostly they bring their wounded home ({} of {})", bleeding, seen);
    }

    #[test]
    fn rations_go_with_them_and_the_tactician_leads() {
        let dest = Destination { goal: Goal::Ruin, site: 0, name: "the ruins of Testhold".into(), dist: 4, danger: 1 };
        let setup = |stores: bool| {
            let (mut g, squad) = fort_with_squad(3, 12, true);
            g.items.retain(|_, it| it.kind != ItemKind::Drink && !it.is_food());
            if stores {
                let at = g.embark;
                for kind in [ItemKind::Drink, ItemKind::Drink, ItemKind::Drink, ItemKind::Drink, ItemKind::Meal, ItemKind::Meal, ItemKind::Meat, ItemKind::Meat] {
                    g.spawn_item(kind, if kind == ItemKind::Drink { Mat::Stoutcap } else { Mat::Flesh }, at);
                }
            }
            // Equals with the sword; the most diligent reads the ground best.
            for (d, diligence) in g.dwarves.iter_mut().filter(|d| squad.contains(&d.id)).zip([0, 45, -20]) {
                d.personality.diligence = diligence;
                d.personality.anxiety = -50;
                d.fight_xp = 2000;
            }
            g.knobs.set.insert("exp.nerve".into(), -2.0);
            (g, squad)
        };
        let (hungry, squad) = setup(false);
        let (mut g, _) = setup(true);
        assert!(g.round_odds(&squad, &dest) > hungry.round_odds(&squad, &dest), "rations make better odds");
        let leader = squad[1];
        assert!(g.send_expedition(0, dest.clone()).is_ok());
        let ex = g.expedition.as_ref().unwrap();
        assert_eq!(ex.leader, Some(leader));
        assert_eq!(ex.rations, 6, "a drink and a meal each");
        let left: Vec<ItemKind> = g.items.values().filter(|it| it.kind == ItemKind::Drink || it.is_food()).map(|it| it.kind).collect();
        assert_eq!(left, vec![ItemKind::Drink, ItemKind::Meat], "meals are packed before raw meat");
        assert!(g.log.iter().any(|l| l.text.contains("They carry 6 rations.")));
        let name = ex.members.iter().find(|d| d.id == leader).unwrap().name.split(' ').next().unwrap().to_string();
        g.tick = ex.back;
        g.expedition_tick();
        let text: Vec<&str> = g.log.iter().map(|l| l.text.as_str()).collect();
        let led = format!("Led by {}", name);
        assert!(text.iter().any(|t| t.contains("in triumph") && t.contains(&led) && t.ends_with("who read the ground well.")), "{:?}", text);
        let xp = |id: u32| g.dwarves.iter().find(|d| d.id == id).unwrap().fight_xp;
        assert_eq!(xp(leader), xp(squad[0]) + 100, "the leader learns the most");
        assert!(g.dwarves.iter().filter(|d| squad.contains(&d.id)).all(|d| d.hunger < HUNGRY), "they ate on the road");
        // Without rations they come home hungry.
        let (mut g, _) = setup(false);
        g.send_expedition(0, dest.clone()).unwrap();
        assert!(g.log.iter().any(|l| l.text.contains("They go without rations.")));
        g.tick = g.expedition.as_ref().unwrap().back;
        g.expedition_tick();
        assert!(g.dwarves.iter().filter(|d| squad.contains(&d.id)).all(|d| d.hunger >= HUNGRY));
        // Drink alone, however much, does not feed them.
        let (mut g, _) = setup(false);
        let at = g.embark;
        for _ in 0..6 {
            g.spawn_item(ItemKind::Drink, Mat::Stoutcap, at);
        }
        assert_eq!(g.round_odds(&squad, &dest), hungry.round_odds(&squad, &dest), "drink alone is no meal");
        g.send_expedition(0, dest.clone()).unwrap();
        assert!(g.log.iter().any(|l| l.text.contains("They carry 3 rations.")));
        assert!(!g.expedition.as_ref().unwrap().fed());
        // An expedition from an old save, sent before rations were packed, is fed.
        let ex = g.expedition.as_mut().unwrap();
        ex.arrive = 0;
        assert!(ex.fed());
        g.tick = ex.back;
        g.expedition_tick();
        assert!(g.dwarves.iter().filter(|d| squad.contains(&d.id)).all(|d| d.hunger < HUNGRY));
    }

    #[test]
    fn one_killed_in_the_fight_falls_there() {
        let dest = Destination { goal: Goal::Raid, site: 0, name: "Testgrot".into(), dist: 8, danger: 8 };
        let up = crate::body::Part::UpperBody.idx();
        let mut killed = 0;
        for seed in 0..40 {
            let (mut g, _) = weak_squad_away(seed, &dest, -2.0);
            // All of them already near death: the next wound to the body kills.
            let ex = g.expedition.as_mut().unwrap();
            for d in ex.members.iter_mut() {
                d.body.hp[up] = -d.body.max[up] / 2 + 1;
            }
            g.tick = g.expedition.as_ref().unwrap().next_fight();
            g.expedition_tick();
            let ex = g.expedition.as_ref().unwrap();
            let dead: Vec<u32> = ex.members.iter().filter(|d| d.body.dead).map(|d| d.id).collect();
            if dead.is_empty() {
                continue;
            }
            killed += 1;
            assert!(dead.iter().all(|id| ex.fallen.contains(id)), "seed {}: the dead are among the fallen", seed);
            assert_eq!(g.annals.counters.get("exp_fallen").copied(), Some(ex.fallen.len() as u32));
        }
        assert!(killed > 0, "a wound to the near-dead kills now and then");
    }

    #[test]
    fn no_squad_is_wiped_out_away_from_a_lair() {
        let dest = Destination { goal: Goal::Raid, site: 0, name: "Testgrot".into(), dist: 8, danger: 8 };
        for seed in 0..40 {
            // Nerve that never fails: they fight to the end.
            let (mut g, back) = weak_squad_away(seed, &dest, -2.0);
            let before = g.dwarves.len();
            g.tick = back;
            g.expedition_tick();
            assert!(g.expedition.is_none());
            let dead = g.dwarves[before..].iter().filter(|d| !d.alive).count();
            assert!(dead <= 1, "seed {}: {} of 3 fell", seed, dead);
        }
    }

    #[test]
    fn destinations_include_ruins_lairs_and_enemies() {
        let g = Game::new(81);
        let v = g.destinations();
        assert!(!v.is_empty(), "nowhere to go");
        assert!(v.windows(2).all(|w| w[0].dist <= w[1].dist));
        assert!(v.iter().all(|d| d.days() >= 3));
    }
}
