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
        self.dwarves
            .iter()
            .filter(|d| ids.contains(&d.id) && d.alive && !d.is_child())
            .map(|d| crate::dwarf::skill_level(d.fight_xp) as i32 + 1 + d.held.is_some() as i32 + d.armor.len() as i32 / 2)
            .sum()
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
        let strength: i32 = members
            .iter()
            .map(|d| crate::dwarf::skill_level(d.fight_xp) as i32 + 1 + d.held.is_some() as i32 + d.armor.len() as i32 / 2)
            .sum();
        let p = (strength as f64 / (strength as f64 + dest.danger as f64 * 3.0)).clamp(0.1, 0.95);
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

    #[test]
    fn destinations_include_ruins_lairs_and_enemies() {
        let g = Game::new(81);
        let v = g.destinations();
        assert!(!v.is_empty(), "nowhere to go");
        assert!(v.windows(2).all(|w| w[0].dist <= w[1].dist));
        assert!(v.iter().all(|d| d.days() >= 3));
    }
}
