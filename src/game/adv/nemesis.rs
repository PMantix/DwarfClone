//! The sworn enemy (A12): a named foe who survives a fight with the hero,
//! swears on them, and follows them across the map.
//!
//! Cut down some of a band and walk away from the rest (a patrol on the
//! road, a ruin's bandits, a fallen fort's goblins) and one who lived may
//! swear vengeance: a figure of the legends from then on, named, with the
//! grudge said in the saga and the legends. On the world map it moves a
//! cell or two a day toward the hero, its band growing each season, and
//! when it reaches the hero's cell on the road it is the encounter
//! (`nemesis_meet`, one line in `roll_encounter_with`). Their first
//! meeting is the Turn of the hero's saga. Drive it off and it comes back;
//! kill it and the feud closes in the legends; die to it and *it* takes
//! the name and the legend ("Snaga Dwarfbane, who slew Urist at last").
//!
//! The watch (`nemesis_check`, with `notice` after every act) needs no
//! hook in combat: it remembers who was out for the hero's blood at the
//! last look in a site, and how many the hero had slain there.

use super::super::*;
use super::travel::{EncKind, Encounter, DIRS};
use super::{sitegen, Adventure, SiteKey, HERO_ID};
use crate::creature::Species;
use crate::game::sagas::Beat;
use crate::overworld::{Figure, Race};
use serde::{Deserialize, Serialize};

/// The most a sworn enemy brings with them.
const BAND_MAX: u8 = 6;

/// What the watch saw at the last look in a site.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Watch {
    #[serde(default)]
    pub key: Option<SiteKey>,
    /// The hero's kills on coming in, and at the last look.
    #[serde(default)]
    pub kills0: u32,
    #[serde(default)]
    pub kills: u32,
    /// People out for the hero's blood at the last look: (creature id,
    /// their label, species).
    #[serde(default)]
    pub foes: Vec<(u32, String, Option<Species>)>,
    /// The sworn enemy was here, alive, at the last look.
    #[serde(default)]
    pub enemy_here: bool,
}

/// A person who would swear: a people, not a beast.
fn people(sp: Species) -> Option<(Race, &'static str)> {
    match sp {
        Species::Goblin => Some((Race::Goblin, "goblin")),
        Species::Human => Some((Race::Human, "human")),
        Species::Elf => Some((Race::Elf, "elf")),
        Species::Raider => Some((Race::Human, "bandit")),
        _ => None,
    }
}

impl Adventure {
    /// After every act (with `notice`): a band left with blood on it may
    /// swear; the enemy met is named on the field, driven off, or slain.
    pub fn nemesis_check(&mut self) {
        if self.over.is_some() {
            return;
        }
        let key = self.visit.as_ref().map(|v| v.key);
        let kills = self.hero_dwarf().kills;
        if key != self.watch.key {
            let w = std::mem::take(&mut self.watch);
            if w.key.is_some() {
                self.left_the_field(&w);
            }
            self.watch = Watch { key, kills0: kills, kills, foes: Vec::new(), enemy_here: false };
        }
        let Some(v) = self.visit.as_mut() else { return };
        self.watch.kills = kills;
        // The enemy on the field: only the first of their band bears the name.
        let fid = self.nemesis.as_ref().filter(|n| !n.done).and_then(|n| n.figure);
        let mut here = false;
        if let Some(f) = fid {
            let mut ids: Vec<u32> = v.g.creatures.values().filter(|c| c.figure == Some(f)).map(|c| c.id).collect();
            ids.sort();
            for id in ids.iter().skip(1) {
                if let Some(c) = v.g.creatures.get_mut(id) {
                    c.figure = None;
                    c.label = None;
                }
            }
            here = ids.first().map_or(false, |id| v.g.creatures.get(id).map_or(false, |c| c.alive));
        }
        let mut foes: Vec<(u32, String, Option<Species>)> = v
            .g
            .creatures
            .values()
            .filter(|c| c.alive && !c.tame && (c.siege || c.hostile()) && c.figure.is_none() && people(c.species).is_some())
            .filter(|c| c.target == Some(HERO_ID) || c.siege)
            .map(|c| (c.id, c.label.clone().unwrap_or_default(), Some(c.species)))
            .collect();
        foes.sort_by_key(|f| f.0);
        self.watch.foes = foes;
        let first_sight = here && !self.watch.enemy_here;
        self.watch.enemy_here = here;
        if first_sight {
            self.enemy_met();
        }
        // Slain.
        if let Some(f) = fid {
            if self.ow().figures.get(f as usize).map_or(false, |x| !x.alive()) {
                self.enemy_slain();
            }
        }
    }

    /// Out of a site: if the hero shed blood there and some of those who
    /// wanted theirs still live, one may swear; an enemy who lived through
    /// a meeting goes off to gather more.
    fn left_the_field(&mut self, w: &Watch) {
        if let Some(n) = self.nemesis.as_mut().filter(|n| !n.done) {
            if w.enemy_here {
                n.band = (n.band + 1).min(BAND_MAX);
                n.next = self.clock + DAY * 2;
                let name = n.name.clone();
                self.say(&format!("{} lives, and will come again with more.", name), (255, 120, 90));
            }
            return;
        }
        if self.nemesis.is_some() || w.kills <= w.kills0 {
            return;
        }
        let Some(foe) = w.foes.first().cloned() else { return };
        let chance = self.knob("nemesis.chance", 0.5);
        if !self.rng().chance(chance) {
            return;
        }
        let place = w.key.map_or_else(String::new, |k| super::sites::site_name(self.ow(), k));
        self.swear(&foe, &place);
    }

    /// One who lived swears on the hero: a figure of the legends now.
    fn swear(&mut self, foe: &(u32, String, Option<Species>), place: &str) {
        let Some((race, kind)) = foe.2.and_then(people) else { return };
        let name = match foe.1.split(", ").next().filter(|n| !n.is_empty() && n.chars().next().map_or(false, |c| c.is_uppercase()) && !n.starts_with("The ")) {
            Some(n) => n.to_string(),
            None => {
                let mut rng = crate::rng::Rng::new(names::hash(&[self.seed, foe.0 as u64, self.clock]));
                crate::overworld::person_name(&mut rng, race)
            }
        };
        let year = self.year();
        let ow = self.ow_mut();
        let id = ow.figures.len() as u32;
        ow.figures.push(Figure { id, name: name.clone(), kind: kind.to_string(), civ: None, born: year - 20, died: None, cause: None, title: None, kills: 0, lair: None });
        // (Its death is the feud's end, told here, not as an old legend.)
        self.fame.seen.dead.insert(id);
        let hero = self.hero_dwarf().called();
        let grudge = format!("swore vengeance on {} for the blood shed in {}", hero, place);
        self.nemesis = Some(super::Nemesis {
            figure: Some(id),
            name: name.clone(),
            at: self.at,
            race: Some(race),
            kind: kind.to_string(),
            grudge: grudge.clone(),
            band: 2,
            since: self.clock,
            moved: self.clock,
            next: self.clock + DAY,
            met: 0,
            done: false,
        });
        self.say(&format!("{} the {} saw what you did in {}, and lived. {} swears to find you.", name, kind, place, name), (255, 90, 70));
        self.deed(&format!("made a sworn enemy of {} the {}, who lived through the blood shed in {}", name, kind, place), 0);
        self.ow_mut().log(year, format!("{} the {} {}.", name, kind, grudge));
    }

    /// The enemy moves on the world map toward where the hero is: a cell
    /// or two a day (`nemesis.pace`), its band growing each season.
    pub fn nemesis_day(&mut self) {
        let pace = self.knob("nemesis.pace", 1.5);
        let (clock, to) = (self.clock, self.at);
        let Some(n) = self.nemesis.as_mut().filter(|n| !n.done) else { return };
        let days = clock.saturating_sub(n.moved) as f64 / DAY as f64;
        let steps = (days * pace) as i32;
        if steps <= 0 {
            return;
        }
        n.moved += (steps as f64 / pace * DAY as f64) as u64;
        for _ in 0..steps {
            if n.at == to {
                break;
            }
            n.at = (n.at.0 + (to.0 - n.at.0).signum(), n.at.1 + (to.1 - n.at.1).signum());
        }
        let seasons = (clock.saturating_sub(n.since) / SEASON) as u8;
        n.band = n.band.max((2 + seasons).min(BAND_MAX));
    }

    /// The enemy has caught up with the hero on the road: the encounter
    /// (one line in `roll_encounter_with`, before the dice for the rest).
    pub fn nemesis_meet(&mut self) -> Option<Encounter> {
        self.nemesis_day();
        let (clock, at) = (self.clock, self.at);
        let n = self.nemesis.as_mut().filter(|n| !n.done && n.at == at && clock >= n.next)?;
        n.next = clock + DAY * 2;
        let race = n.race.unwrap_or(Race::Goblin);
        let sp = if n.kind == "bandit" { Species::Raider } else { sitegen::soldier(race) };
        let dir = DIRS[(names::hash(&[clock, at.0 as u64, at.1 as u64]) % 8) as usize];
        let way = super::travel::dir_words(dir.0, dir.1);
        let text = if n.met == 0 {
            format!("{} the {} has found you! Sworn on you since {}, they come from the {} with {} at their back.", n.name, n.kind, n.grudge.rsplit(" in ").next().unwrap_or("that day"), way, super::travel::number(n.band))
        } else {
            format!("{} again! Your sworn enemy comes from the {}, and has brought {} this time.", n.name, way, super::travel::number(n.band))
        };
        let kind = if n.kind == "bandit" { EncKind::Bandits } else { EncKind::Patrol };
        Some(Encounter { kind, text, figure: n.figure, foes: vec![(sp, 1), (sp, n.band)], dir, came: None, ambush: false, race: n.race })
    }

    /// Face to face: the first meeting is the Turn of the hero's saga.
    fn enemy_met(&mut self) {
        let Some(n) = self.nemesis.as_mut() else { return };
        n.met = n.met.saturating_add(1);
        let (first, name, kind) = (n.met == 1, n.name.clone(), n.kind.clone());
        let hero = self.hero_dwarf().name.split(' ').next().unwrap_or("").to_string();
        if first {
            self.chapter(Beat::Turn, format!("{} the {}, sworn on {}, found them on the road at last.", name, kind, hero), 2);
        }
    }

    /// The sworn enemy is dead: the feud ends in the legends.
    fn enemy_slain(&mut self) {
        let Some(n) = self.nemesis.as_mut().filter(|n| !n.done) else { return };
        n.done = true;
        let (name, kind) = (n.name.clone(), n.kind.clone());
        self.deed(&format!("slew {} the {}, the sworn enemy who had hunted them", name, kind), 3);
    }

    /// The hero is dead: if their sworn enemy was there, it takes the name
    /// and the legend.
    pub fn nemesis_at_death(&mut self) {
        let here = self.watch.enemy_here || self.visit.as_ref().map_or(false, |v| self.nemesis.as_ref().and_then(|n| n.figure).map_or(false, |f| v.g.creatures.values().any(|c| c.alive && c.figure == Some(f))));
        let hero = self.hero_dwarf().called();
        let bane = format!("{}bane", match self.hero.race {
            Race::Dwarf => "Dwarf",
            Race::Human => "Man",
            Race::Elf => "Elf",
            Race::Goblin => "Goblin",
        });
        let year = self.year();
        let Some(n) = self.nemesis.as_mut().filter(|n| !n.done) else { return };
        if !here {
            return;
        }
        n.done = true;
        let (name, kind, fid) = (n.name.clone(), n.kind.clone(), n.figure);
        if let Some(f) = fid.and_then(|f| self.ow_mut().figures.get_mut(f as usize)) {
            f.title = Some(bane.clone());
            f.kills += 1;
        }
        self.ow_mut().log(year, format!("{} {}, the {} who had sworn on {}, slew them at last.", name, bane, kind, hero));
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::test_hero;
    use super::super::travel::TravelOutcome;
    use super::*;

    fn on_road(seed: u64) -> Adventure {
        let ow = super::super::sitegen::town::tests::world();
        let mut a = Adventure::new(ow, seed, test_hero(seed), None);
        a.knobs.set.insert("travel.encounters".into(), 0.0);
        a
    }

    #[test]
    fn the_enemy_follows_and_finds_you() {
        let mut a = on_road(61);
        // A fight in the wild: the hero cuts one goblin down, two live.
        let (x, y) = a.at;
        a.enter(SiteKey::Wild(x, y)).unwrap();
        a.notice();
        let v = a.visit.as_mut().unwrap();
        let p = v.g.dwarves[v.hero].pos;
        for k in 0..2 {
            let at = v.g.spot_near(p.offset(6, k, 0), 1);
            let id = v.g.spawn_creature(Species::Goblin, at, DAY);
            if let Some(c) = v.g.creatures.get_mut(&id) {
                c.siege = true;
                c.target = Some(HERO_ID);
            }
        }
        a.notice();
        let h = a.visit.as_ref().unwrap().hero;
        a.visit.as_mut().unwrap().g.dwarves[h].kills += 1;
        a.notice();
        a.knobs.set.insert("nemesis.chance".into(), 1.0);
        a.leave();
        a.notice();
        let n = a.nemesis.clone().expect("one who lived swore");
        assert_eq!(n.kind, "goblin");
        assert!(a.ow().figures.iter().any(|f| Some(f.id) == n.figure && f.name == n.name && f.alive()), "a figure of the legends");
        assert!(a.ow().events.iter().any(|e| e.text.contains(&n.name) && e.text.contains("vengeance")), "the legends tell the grudge");
        assert!(a.fame.saga.as_ref().unwrap().chapters.iter().any(|c| c.2.contains("sworn enemy")));
        // Sent far off, the enemy walks back toward the hero, who waits.
        let start = n.at;
        let ow_w = a.ow().w;
        a.nemesis.as_mut().unwrap().at = ((start.0 + 6).min(ow_w - 1), start.1);
        let mut met = None;
        for _ in 0..30 {
            a.clock += DAY / 2;
            if let Some(e) = a.roll_encounter() {
                met = Some(e);
                break;
            }
        }
        let e = met.expect("within days it arrives as an encounter");
        assert_eq!(e.figure, n.figure);
        assert!(e.text.contains(&n.name), "{}", e.text);
        // The meeting: only the first of the band bears the name, and it is
        // the saga's turn.
        a.meet(&e).unwrap();
        a.notice();
        let v = a.visit.as_ref().unwrap();
        assert_eq!(v.g.creatures.values().filter(|c| c.figure == n.figure).count(), 1);
        assert!(a.fame.saga.as_ref().unwrap().chapters.iter().any(|c| c.1 == Beat::Turn && c.2.contains(&n.name)));
        // The hero slays them: the feud closes in the legends.
        let fid = n.figure.unwrap();
        let year = a.year();
        a.ow_mut().kill_figure(fid, year, "slain".into());
        a.notice();
        assert!(a.nemesis.as_ref().unwrap().done);
        assert!(a.fame.deeds.iter().any(|d| d.text.contains("sworn enemy who had hunted them")));
        let _ = TravelOutcome::Arrived;
    }

    #[test]
    fn the_enemy_takes_the_name_of_the_hero_it_slays() {
        let mut a = on_road(62);
        a.nemesis = Some(super::super::Nemesis { figure: None, name: "Snaga".into(), at: a.at, race: Some(Race::Goblin), kind: "goblin".into(), grudge: "swore vengeance".into(), band: 2, since: 0, moved: a.clock, next: 0, met: 0, done: false });
        let year = a.year();
        let id = a.ow().figures.len() as u32;
        a.ow_mut().figures.push(Figure { id, name: "Snaga".into(), kind: "goblin".into(), civ: None, born: year - 20, died: None, cause: None, title: None, kills: 0, lair: None });
        a.nemesis.as_mut().unwrap().figure = Some(id);
        a.watch.enemy_here = true;
        a.hero_died("was slain by a goblin");
        let f = &a.ow().figures[id as usize];
        assert_eq!(f.title.as_deref(), Some("Dwarfbane"));
        assert!(a.ow().events.iter().any(|e| e.text.contains("Snaga Dwarfbane")));
    }
}
