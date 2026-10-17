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
}

/// One dwarf's fighting strength away from home.
fn strength_of(d: &Dwarf) -> i32 {
    crate::dwarf::skill_level(d.fight_xp) as i32 + 1 + d.held.is_some() as i32 + d.armor.len() as i32 / 2
}

/// The chance a squad of `strength` beats a place of `danger`.
fn win_chance(strength: i32, danger: i32) -> f64 {
    (strength as f64 / (strength as f64 + danger as f64 * 3.0)).clamp(0.1, 0.95)
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
        self.round_odds(ids, dest)
    }

    /// The chance that squad `ids` wins one fight at `dest`.
    pub fn round_odds(&self, ids: &[u32], dest: &Destination) -> f64 {
        win_chance(self.squad_strength(ids), dest.danger)
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
    pub(super) fn venture_veto(&self, o: &super::overseer::Overseer, ids: &[u32]) -> Option<String> {
        let squad: Vec<&Dwarf> = self.dwarves.iter().filter(|d| ids.contains(&d.id) && d.alive).collect();
        let n = squad.len();
        let unarmed = squad.iter().filter(|d| !self.kitted(d).0).count();
        let bare = squad.iter().filter(|d| !self.kitted(d).1).count();
        if unarmed > 0 || bare > 0 {
            return Some(if bare >= unarmed {
                format!("{} no armour", share(bare, n))
            } else {
                format!("{} no metal weapon", share(unarmed, n))
            });
        }
        let adults = self.dwarves.iter().filter(|d| d.alive && !d.is_child()).count();
        if (adults as f64) < self.knob("exp.min_adults", 12.0) {
            return Some(format!("the fort is only {} grown dwarves strong", adults));
        }
        if o.last_siege > 0 && self.tick < o.last_siege + SEASON {
            return Some(format!("the goblins were at the gate only {} days ago", (self.tick - o.last_siege) / DAY));
        }
        let wounded = self.dwarves.iter().filter(|d| d.alive && d.body.needs_rest()).count();
        if wounded >= 2 {
            return Some(format!("{} dwarves lie wounded", wounded));
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
        let text = format!("{} set out for {}. They should be back in about {} days.", name, dest.name, days);
        self.msg(&text, (200, 220, 255));
        let fort = self.fortress_name.clone();
        self.legend(format!("{} of {} set out for {}.", name, fort, dest.name));
        self.expedition = Some(Expedition { squad: name, members, dest, back: self.tick + days * DAY });
        Ok(text)
    }

    /// Now and then: is the expedition home?
    pub(super) fn expedition_tick(&mut self) {
        if self.expedition.as_ref().map_or(true, |e| self.tick < e.back) {
            return;
        }
        let ex = self.expedition.take().unwrap();
        self.expedition_returns(ex);
    }

    fn expedition_returns(&mut self, ex: Expedition) {
        let Expedition { squad, members, dest, .. } = ex;
        let strength: i32 = members.iter().map(strength_of).sum();
        let p = win_chance(strength, dest.danger);
        let won = self.rng.chance(p);
        let death = (dest.danger as f64 * 0.05 * if won { 0.5 } else { 1.5 }).min(0.8);
        let entry = self.edge_entry().unwrap_or(self.embark);
        let fort = self.fortress_name.clone();
        let tick = self.tick;
        let mut lost = Vec::new();
        let mut home: Vec<Dwarf> = Vec::new();
        for mut d in members {
            d.pos = self.spot_near(entry, 2);
            d.path.clear();
            if self.rng.chance(death) {
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
        if !won {
            self.msg(&format!("{} have returned from {}, beaten.{}", squad, dest.name, gone), (255, 150, 90));
            self.legend(format!("{} of {} were driven back from {}.", squad, fort, dest.name));
            self.dwarves.extend(home);
            return;
        }
        // Victory: loot, and a name for whoever led them.
        let lead = (0..home.len()).max_by_key(|&i| (home[i].fight_xp, home[i].id)).unwrap();
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
        }
        let leader_name = leader.called();
        home.push(leader);
        self.milestone("expedition", &format!("The first expedition home in triumph, from {}", dest.name));
        self.dwarves.extend(home);
        self.msg(&format!("{} have returned in triumph from {}, led by {}, with {} treasures!{}", squad, dest.name, leader_name, n, gone), (255, 215, 60));
        self.legend(format!("{} of {}, led by {}, {} {}.", squad, fort, leader_name, what, dest.name));
        self.spotlight("Home in triumph", &format!("{} have returned from {}, led by {}, with {} treasures!{}", squad, dest.name, leader_name, n, gone), None, (255, 215, 60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_expedition_goes_away_and_comes_home_with_loot() {
        let mut g = Game::new(81);
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
        let items = g.items.len();
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
        assert!(text.iter().any(|t| t.contains("returned")), "{:?}", text);
        if text.iter().any(|t| t.contains("triumph")) {
            assert!(g.items.len() > items, "loot");
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

    #[test]
    fn destinations_include_ruins_lairs_and_enemies() {
        let g = Game::new(81);
        let v = g.destinations();
        assert!(!v.is_empty(), "nowhere to go");
        assert!(v.windows(2).all(|w| w[0].dist <= w[1].dist));
        assert!(v.iter().all(|d| d.days() >= 3));
    }
}
