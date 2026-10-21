//! Making a character (A1): the hero's people, calling, skills, kit, name
//! and home, chosen on the maker screen (`ui/adv_create.rs`, keys in
//! `app/adventure.rs: key_create`). Everything here is pure: the same
//! choices make the same hero. Names, looks and the backstory are drawn
//! from hashes (`names::hash`), never from a game's dice.
//!
//! The world's peoples are offered where they have a living site of their
//! own kind (plan, "Peoples"): dwarves in a fortress or hamlet, humans in a
//! town or hamlet, elves in a forest retreat, goblins in a dark fortress.
//! Home is a site whose folk take the hero as their own (`Stance::Home`).

use super::peoples::{stance, Stance};
use super::{Adventure, Calling, Hero, SiteKey, HERO_ID};
use crate::dwarf::{new_dwarf, Labor};
use crate::game::loadout::{level_cost, MAX_LEVEL};
use crate::game::names;
use crate::geom::Pos;
use crate::items::{Item, ItemKind};
use crate::material::Mat;
use crate::overworld::{Overworld, Race, Site, SiteKind};
use crate::rng::Rng;

pub const CALLINGS: [Calling; 4] = [Calling::Warrior, Calling::Hunter, Calling::Wanderer, Calling::Peasant];
const RACES: [Race; 4] = [Race::Dwarf, Race::Human, Race::Elf, Race::Goblin];

impl Calling {
    pub fn name(self) -> &'static str {
        match self {
            Calling::Warrior => "warrior",
            Calling::Hunter => "hunter",
            Calling::Wanderer => "wanderer",
            Calling::Peasant => "peasant",
        }
    }

    /// One line on what the calling means.
    pub fn blurb(self) -> &'static str {
        match self {
            Calling::Warrior => "Trained to arms, armoured and shielded.",
            Calling::Hunter => "A crossbow and a keen eye; lightly clad.",
            Calling::Wanderer => "A little of everything, a staff and a purse.",
            Calling::Peasant => "Nothing but a pick. Ruin is likelier, glory greater.",
        }
    }

    /// Skill points the calling may spend (`loadout::level_cost` prices).
    pub fn budget(self) -> i32 {
        match self {
            Calling::Peasant => 15,
            _ => 45,
        }
    }
}

/// The people's word for one of them.
pub fn race_noun(r: Race) -> &'static str {
    match r {
        Race::Dwarf => "dwarf",
        Race::Human => "human",
        Race::Elf => "elf",
        Race::Goblin => "goblin",
    }
}

/// The kinds of site a people calls home.
pub fn home_kinds(r: Race) -> &'static [SiteKind] {
    match r {
        Race::Dwarf => &[SiteKind::Fortress, SiteKind::Hamlet],
        Race::Human => &[SiteKind::Town, SiteKind::Hamlet],
        Race::Elf => &[SiteKind::Retreat],
        Race::Goblin => &[SiteKind::DarkFortress],
    }
}

/// The labours a people leans to (goblins to none: they take, not make).
pub fn leanings(r: Race) -> &'static [Labor] {
    match r {
        Race::Dwarf => &[Labor::Mining, Labor::Smithing, Labor::Masonry],
        Race::Human => &[Labor::Butchery, Labor::Leatherworking, Labor::Crafting],
        Race::Elf => &[Labor::Medicine, Labor::Fishing, Labor::Crafting],
        Race::Goblin => &[],
    }
}

/// A skill the maker sets: fighting, or a labour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Skill {
    Fighting,
    Labor(Labor),
}

impl Skill {
    pub fn name(self) -> &'static str {
        match self {
            Skill::Fighting => "Fighting",
            Skill::Labor(Labor::Butchery) => "Hunting",
            Skill::Labor(Labor::Medicine) => "Herbalism",
            Skill::Labor(l) => l.name(),
        }
    }
}

/// The sites a hero of `race` may set out from: living sites of a living
/// civ of that people, of a kind they live in, whose folk take the hero as
/// their own. Nearest a fallen fort first when the world has one (that is
/// where the stories are), else in a steady order from the world's seed.
pub fn homes(ow: &Overworld, race: Race) -> Vec<u32> {
    let mut v: Vec<&Site> = ow
        .sites
        .iter()
        .filter(|s| !s.ruined && home_kinds(race).contains(&s.kind))
        .filter(|s| s.civ.and_then(|c| ow.civs.get(c as usize)).map_or(false, |c| c.alive && c.race == race))
        .filter(|s| stance(ow, race, s.civ, s) == Stance::Home)
        .collect();
    let fallen: Vec<(i32, i32)> = ow.forts.iter().filter(|f| f.fell.is_some()).map(|f| f.site).collect();
    v.sort_by_key(|s| {
        let near = fallen.iter().map(|&(x, y)| (s.x - x).abs().max((s.y - y).abs())).min();
        (near.unwrap_or(0), names::hash(&[ow.seed, s.id as u64]))
    });
    v.into_iter().map(|s| s.id).collect()
}

/// The peoples a hero may be in this world: those with a home to start from.
pub fn peoples(ow: &Overworld) -> Vec<Race> {
    RACES.iter().copied().filter(|&r| !homes(ow, r).is_empty()).collect()
}

/// What a calling carries, as the hero's people make it: (kind, material,
/// label, how it is borne) and coins. An elf's metal becomes wood; a dwarf
/// warrior takes an axe, mail and a helm, a human a spear or sword, a goblin an
/// iron spear.
pub fn kit(race: Race, calling: Calling, pick: u64) -> (Vec<(ItemKind, Mat, Option<&'static str>, Bear)>, u32) {
    use Bear::*;
    use ItemKind as K;
    let metal = match race {
        Race::Elf => Mat::Wood,
        Race::Goblin => Mat::Iron,
        _ => Mat::Copper,
    };
    let weapon = match race {
        Race::Dwarf => K::Axe,
        Race::Human if pick % 2 == 0 => K::Sword,
        _ => K::Spear,
    };
    let mut v = match calling {
        Calling::Warrior => vec![
            (weapon, if race == Race::Dwarf { Mat::Copper } else { metal }, None, Held),
            (K::Breastplate, if race == Race::Dwarf { Mat::Copper } else { Mat::Leather }, None, Worn),
            (K::Shield, Mat::Wood, None, Worn),
            // A cap: a bare head is where a fight ends (A16).
            (K::Helm, if race == Race::Elf { Mat::Leather } else { metal }, None, Worn),
        ],
        Calling::Hunter => vec![
            (K::Crossbow, Mat::Wood, None, Held),
            (K::Bolt, Mat::Wood, None, Carried),
            (K::Bolt, Mat::Wood, None, Carried),
            (K::Pick, metal, Some("skinning pick"), Carried),
        ],
        Calling::Wanderer => vec![(K::Hammer, Mat::Wood, Some("walking staff"), Held), (K::Shirt, Mat::Threadreed, Some("travelling cloak"), Clothed)],
        Calling::Peasant => vec![(K::Pick, metal, None, Held)],
    };
    // Every people goes clothed but goblins, who go in leather.
    if race == Race::Goblin && calling != Calling::Warrior {
        v.push((K::Breastplate, Mat::Leather, Some("hide jerkin"), Worn));
    }
    // A dwarf never leaves home without a pick.
    if race == Race::Dwarf && !v.iter().any(|t| t.0 == K::Pick) {
        v.push((K::Pick, Mat::Copper, None, Carried));
    }
    let coins = match calling {
        Calling::Wanderer => 30,
        Calling::Peasant => 5,
        _ => 10,
    };
    (v, coins)
}

/// How a piece of kit is borne.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bear {
    Held,
    Worn,
    Clothed,
    Carried,
}

/// The maker's choices.
#[derive(Clone, Debug)]
pub struct Maker {
    pub seed: u64,
    /// Peoples offered, and the one chosen.
    pub races: Vec<Race>,
    pub race: usize,
    pub calling: usize,
    /// The skills shown and their levels (0 none, 1 novice…).
    pub skills: Vec<(Skill, u32)>,
    /// Which roll of looks, personality and name.
    pub roll: u64,
    pub name: String,
    /// The name is being typed.
    pub naming: bool,
    /// Home sites offered for the race, and the one chosen.
    pub homes: Vec<u32>,
    pub home: usize,
    /// The highlighted row (`ROWS`, then the skills).
    pub sel: usize,
}

/// The rows above the skills.
pub const ROWS: [&str; 4] = ["People", "Calling", "Name", "Home"];

impl Maker {
    pub fn new(ow: &Overworld, seed: u64) -> Maker {
        let races = peoples(ow);
        let mut m = Maker { seed, races, race: 0, calling: 0, skills: Vec::new(), roll: 0, name: String::new(), naming: false, homes: Vec::new(), home: 0, sel: 0 };
        m.set_race(ow, 0);
        m
    }

    pub fn race(&self) -> Race {
        self.races.get(self.race).copied().unwrap_or(Race::Dwarf)
    }

    pub fn calling(&self) -> Calling {
        CALLINGS[self.calling % CALLINGS.len()]
    }

    pub fn home_id(&self) -> Option<u32> {
        self.homes.get(self.home).copied()
    }

    /// Rows in all: the four, the skills, and "Set out".
    pub fn rows(&self) -> usize {
        ROWS.len() + self.skills.len() + 1
    }

    /// Choose a people: their homes, their name for the hero, their skills.
    pub fn set_race(&mut self, ow: &Overworld, i: usize) {
        if self.races.is_empty() {
            return;
        }
        self.race = i % self.races.len();
        self.homes = homes(ow, self.race());
        self.home = 0;
        self.name = self.rolled_name();
        self.set_calling(self.calling);
    }

    /// Choose a calling: its skills as it teaches them.
    pub fn set_calling(&mut self, i: usize) {
        self.calling = i % CALLINGS.len();
        let lean = leanings(self.race());
        let c = self.calling();
        self.skills = std::iter::once(Skill::Fighting).chain(lean.iter().map(|&l| Skill::Labor(l))).map(|s| (s, 0)).collect();
        for (i, (s, lv)) in self.skills.iter_mut().enumerate() {
            *lv = match (c, s) {
                (Calling::Warrior, Skill::Fighting) => 2,
                (Calling::Hunter, Skill::Fighting) => 1,
                (Calling::Hunter, _) if i == 1 => 1,
                (Calling::Wanderer, _) => 1,
                _ => 0,
            };
        }
        self.sel = self.sel.min(self.rows() - 1);
    }

    /// Points the skills cost.
    pub fn spent(&self) -> i32 {
        self.skills.iter().map(|s| level_cost(s.1)).sum()
    }

    pub fn points_left(&self) -> i32 {
        self.calling().budget() - self.spent()
    }

    /// Raise (`+1`) or lower (`-1`) skill `i`, within the budget. False if it can't.
    pub fn adjust(&mut self, i: usize, by: i32) -> bool {
        let Some(&(_, lv)) = self.skills.get(i) else { return false };
        let to = lv as i32 + by;
        if to < 0 || to > MAX_LEVEL as i32 {
            return false;
        }
        let cost = level_cost(to as u32) - level_cost(lv);
        if cost > self.points_left() {
            return false;
        }
        self.skills[i].1 = to as u32;
        true
    }

    /// Another face, another temper, another name.
    pub fn reroll(&mut self) {
        self.roll += 1;
        self.name = self.rolled_name();
    }

    fn roll_seed(&self) -> u64 {
        names::hash(&[self.seed, self.roll, self.race() as u64, 0xC4EA7E])
    }

    /// The name the hero's people would give them on this roll.
    pub fn rolled_name(&self) -> String {
        let mut rng = Rng::new(self.roll_seed() ^ 0x4A3E);
        crate::overworld::person_name(&mut rng, self.race())
    }

    /// The hero as chosen, with their kit in the pack.
    pub fn hero(&self, ow: &Overworld) -> Hero {
        let race = self.race();
        let mut rng = Rng::new(self.roll_seed());
        let mut d = new_dwarf(HERO_ID, Pos::new(0, 0, 0), &mut rng, &[]);
        d.name = if self.name.trim().is_empty() { self.rolled_name() } else { self.name.trim().to_string() };
        // Grown, by their people's measure.
        d.age = Some(match race {
            Race::Dwarf => 30 + (self.roll_seed() % 60) as u32,
            Race::Human => 17 + (self.roll_seed() % 25) as u32,
            Race::Elf => 40 + (self.roll_seed() % 120) as u32,
            Race::Goblin => 14 + (self.roll_seed() % 20) as u32,
        });
        for &(s, lv) in &self.skills {
            match s {
                Skill::Fighting => d.fight_xp = lv * 400,
                Skill::Labor(l) => d.skills[l.idx()] = lv * 400,
            }
        }
        let mut h = Hero::new(d, race);
        let (things, coins) = kit(race, self.calling(), self.roll_seed() >> 7);
        for (i, (kind, mat, label, bear)) in things.into_iter().enumerate() {
            let id = i as u32 + 1;
            h.pack.push(item(id, kind, mat, label));
            match bear {
                Bear::Held => h.dwarf.held = Some(id),
                Bear::Worn => h.dwarf.armor.push(id),
                Bear::Clothed => h.dwarf.clothes.push(id),
                Bear::Carried => {}
            }
        }
        h.coins = coins;
        h.calling = self.calling();
        h.home = self.home_id();
        h.civ = self.home_id().and_then(|id| ow.sites.iter().find(|s| s.id == id)).and_then(|s| s.civ);
        h
    }

    /// The line of the hero's past the world's legends give them.
    pub fn backstory(&self, ow: &Overworld) -> String {
        let h = self.hero(ow);
        backstory(ow, &h, ow.year + 1)
    }
}

fn item(id: u32, kind: ItemKind, mat: Mat, label: Option<&str>) -> Item {
    Item {
        id,
        kind,
        mat,
        pos: Pos::new(0, 0, 0),
        carried_by: Some(HERO_ID),
        claimed_by: None,
        label: label.map(|s| s.to_string()),
        container: None,
        quality: 0,
        to_depot: false,
        dump: false,
        wear: 0,
        forbidden: false,
        kills: 0,
        maker: None,
        decor: None,
    }
}

/// How much a legend stirs: the dragon's burning over the founding of a hamlet.
fn drama(text: &str) -> i32 {
    [
        ("laid waste", 6),
        ("razed", 5),
        ("conquered", 5),
        ("fell defending", 5),
        ("slew", 4),
        ("broke upon the gates", 4),
        ("vanished from the world", 4),
        ("driven off", 3),
        ("repulsed", 3),
        ("declared war", 3),
        ("made peace", 2),
        ("created", 2),
        ("became", 1),
    ]
    .iter()
    .filter(|(k, _)| text.contains(k))
    .map(|(_, w)| *w)
    .max()
    .unwrap_or(0)
}

/// A legend as words to follow "the year" or "when": "Ashmaw the dragon
/// laid waste to Kogsak", or "the Sack of Kogsak" for a named battle.
fn clause(text: &str) -> (String, bool) {
    let t = text.trim_end_matches('.');
    let low = |s: &str| if let Some(r) = s.strip_prefix("The ") { format!("the {}", r) } else { s.to_string() };
    match t.split_once(": ") {
        Some((label, _)) => (low(label), true),
        None => (low(t), false),
    }
}

/// One line of the hero's past, from the world's legends: the most
/// stirring thing that happened in their lifetime, nearest home first.
/// "Born in the year Ashmaw the dragon laid waste to Kogsak."
pub fn backstory(ow: &Overworld, h: &Hero, now_year: i32) -> String {
    let age = h.dwarf.age.unwrap_or(30) as i32;
    let born = now_year - age;
    let home = h.home.and_then(|id| ow.sites.iter().find(|s| s.id == id));
    let civ = h.civ.and_then(|c| ow.civs.get(c as usize));
    let score = |text: &str| {
        let d = drama(text);
        if d == 0 {
            return 0;
        }
        d * 10 + home.map_or(0, |s| if text.contains(&s.name) { 25 } else { 0 }) + civ.map_or(0, |c| if text.contains(&c.name) { 8 } else { 0 })
    };
    let pick = |lo: i32, hi: i32| {
        ow.events
            .iter()
            .filter(|e| e.year >= lo && e.year <= hi && score(&e.text) > 0)
            .max_by_key(|e| (score(&e.text), names::hash(&[ow.seed, h.dwarf.name.len() as u64, e.year as u64, e.text.len() as u64])))
    };
    // Born under it, or lived through it.
    if let Some(e) = pick(born - 1, born + 1).filter(|e| score(&e.text) >= 40) {
        let (c, named) = clause(&e.text);
        return if named { format!("Born in the year of {}.", c) } else { format!("Born in the year {}.", c) };
    }
    if let Some(e) = pick(born.max(0), now_year) {
        let (c, named) = clause(&e.text);
        let then = (e.year - born).max(0);
        let who = match then {
            0 => "A babe in arms".to_string(),
            1..=12 => format!("A child of {}", then),
            _ => format!("{} years old", then),
        };
        return if named { format!("{} at {}.", who, c) } else { format!("{} when {}.", who, c) };
    }
    if let Some(e) = pick(i32::MIN, now_year) {
        let (c, _) = clause(&e.text);
        return format!("Raised on the old tale of how {}.", c.trim_start_matches("the "));
    }
    match home {
        Some(s) => format!("Born in {} in a quiet year, and restless ever since.", s.name),
        None => "Born in a quiet year, and restless ever since.".to_string(),
    }
}

/// The world, with no hero yet: the maker's starting point (the title's
/// "Adventure" makes the world, then this; `unmade` knows it).
pub fn unmade_in(ow: Overworld, seed: u64) -> Adventure {
    Adventure::new(ow, seed, Hero::new(Default::default(), Race::Dwarf), None)
}

/// An adventure still waiting for its hero (the maker shows).
pub fn unmade(a: &Adventure) -> bool {
    a.visit.is_none() && a.over.is_none() && a.hero.home.is_none() && a.hero.dwarf.name.is_empty()
}

/// Set out: the adventure begins at dawn in the hero's home.
pub fn begin(ow: Overworld, seed: u64, hero: Hero) -> Adventure {
    let home = hero.home;
    let story = backstory(&ow, &hero, ow.year + 1);
    let mut a = Adventure::new(ow, seed, hero, home);
    let key = match home {
        Some(id) => SiteKey::Site(id),
        None => SiteKey::Wild(a.at.0, a.at.1),
    };
    if a.enter(key).is_ok() {
        let d = a.hero_dwarf();
        let place = a.visit.as_ref().map_or(String::new(), |v| v.g.fortress_name.clone());
        let line = format!("{} the {} {} sets out from {} at dawn.", d.name, race_noun(a.hero.race), a.hero.calling.name(), place);
        a.say(&story, (190, 170, 220));
        a.say(&line, (255, 215, 90));
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Game;

    fn world() -> Overworld {
        Game::new(5).overworld
    }

    fn maker_for(ow: &Overworld, race: Race) -> Option<Maker> {
        let mut m = Maker::new(ow, 5);
        let i = m.races.iter().position(|&r| r == race)?;
        m.set_race(ow, i);
        Some(m)
    }

    #[test]
    fn every_calling_makes_a_hero_with_its_kit() {
        let ow = world();
        let mut m = Maker::new(&ow, 5);
        for (i, c) in CALLINGS.iter().enumerate() {
            m.set_calling(i);
            let h = m.hero(&ow);
            assert_eq!(h.calling, *c);
            assert_eq!(h.dwarf.id, HERO_ID);
            assert!(!h.dwarf.name.is_empty());
            assert!(m.points_left() >= 0, "{:?} overspends", c);
            let kinds: Vec<ItemKind> = h.pack.iter().map(|it| it.kind).collect();
            match c {
                Calling::Warrior => {
                    assert!(h.dwarf.fight_level() >= 2, "a warrior is adequate at arms");
                    assert!(kinds.contains(&ItemKind::Shield));
                    assert!(h.dwarf.held.is_some());
                    assert_eq!(h.dwarf.armor.len(), 3);
                    assert!(kinds.contains(&ItemKind::Helm), "a warrior sets out with a helm");
                }
                Calling::Hunter => {
                    assert!(kinds.contains(&ItemKind::Crossbow) && kinds.iter().filter(|k| **k == ItemKind::Bolt).count() == 2);
                }
                Calling::Wanderer => {
                    assert_eq!(h.coins, 30);
                    assert!(h.pack.iter().any(|it| it.label.as_deref() == Some("walking staff")));
                }
                Calling::Peasant => {
                    assert_eq!(h.dwarf.fight_xp, 0);
                    assert_eq!(h.coins, 5);
                    assert!(kinds.contains(&ItemKind::Pick));
                }
            }
            // Held and worn ids point into the pack.
            for id in h.dwarf.held.iter().chain(&h.dwarf.armor).chain(&h.dwarf.clothes) {
                assert!(h.pack.iter().any(|it| it.id == *id));
            }
        }
        // The budget holds: a peasant can't buy a master's arm.
        m.set_calling(3);
        let mut n = 0;
        while m.adjust(0, 1) {
            n += 1;
        }
        assert!(n >= 1 && n < 5, "{}", n);
        assert!(m.points_left() >= 0);
        // Points come back when a skill is lowered.
        let left = m.points_left();
        assert!(m.adjust(0, -1));
        assert!(m.points_left() > left);
    }

    #[test]
    fn each_people_starts_at_a_site_of_its_own() {
        let ow = world();
        let m = Maker::new(&ow, 5);
        assert!(m.races.len() >= 2, "{:?}", m.races);
        for race in RACES {
            let living = ow.civs.iter().any(|c| c.alive && c.race == race && ow.sites.iter().any(|s| s.civ == Some(c.id) && !s.ruined && home_kinds(race).contains(&s.kind)));
            let Some(m) = maker_for(&ow, race) else {
                assert!(!living, "{:?} has a home but isn't offered", race);
                continue;
            };
            assert!(living);
            let h = m.hero(&ow);
            assert_eq!(h.race, race);
            let site = ow.sites.iter().find(|s| Some(s.id) == h.home).expect("a home");
            assert!(home_kinds(race).contains(&site.kind), "{:?} in a {:?}", race, site.kind);
            assert_eq!(ow.civs[h.civ.unwrap() as usize].race, race);
            assert_eq!(stance(&ow, race, h.civ, site), Stance::Home);
            if race == Race::Elf {
                assert!(h.pack.iter().all(|it| !matches!(it.mat, Mat::Copper | Mat::Bronze | Mat::Iron | Mat::Steel)), "no metal for an elf");
            }
            if race == Race::Goblin {
                assert_eq!(site.kind, SiteKind::DarkFortress);
            }
        }
        // Every kit, by every people, keeps an elf from metal.
        for c in CALLINGS {
            assert!(kit(Race::Elf, c, 1).0.iter().all(|t| !matches!(t.1, Mat::Copper | Mat::Iron)));
            assert!(kit(Race::Goblin, Calling::Warrior, 1).0.iter().any(|t| t.0 == ItemKind::Spear && t.1 == Mat::Iron));
        }
    }

    #[test]
    fn the_hero_starts_at_home_in_spring() {
        let ow = world();
        let m = Maker::new(&ow, 5);
        let h = m.hero(&ow);
        let (home, civ, year) = (h.home.unwrap(), h.civ, ow.year);
        let a = begin(ow, 5, h);
        assert_eq!(a.start_year, year + 1);
        assert_eq!(a.year(), year + 1);
        assert_eq!(a.hour(), 6, "dawn");
        assert_eq!(a.clock, crate::game::clock::DAWN_TICK);
        let v = a.visit.as_ref().expect("in town");
        assert_eq!(v.key, SiteKey::Site(home));
        assert_eq!(a.hero.civ, civ);
        assert_eq!(v.g.dwarves[v.hero].id, HERO_ID);
        assert!(!unmade(&a));
        // The past is told in the log.
        let log: Vec<&str> = v.g.log.iter().map(|l| l.text.as_str()).collect();
        assert!(log.iter().any(|l| l.contains("sets out from")), "{:?}", log);
    }

    #[test]
    fn a_backstory_comes_from_the_legends() {
        let ow = world();
        let m = Maker::new(&ow, 5);
        let mut seen = std::collections::BTreeSet::new();
        let mut m2 = m.clone();
        for _ in 0..12 {
            let s = m2.backstory(&ow);
            assert!(s.ends_with('.') && s.len() > 20, "{}", s);
            seen.insert(s);
            m2.reroll();
        }
        assert!(seen.len() >= 2, "rerolls change the past too: {:?}", seen);
        // Same choices, same hero.
        assert_eq!(m.hero(&ow).dwarf.name, m.clone().hero(&ow).dwarf.name);
        assert_eq!(m.backstory(&ow), m.clone().backstory(&ow));
        // The words: a named battle and a deed.
        assert_eq!(clause("The Sack of Kogsak: the goblins of the Pit razed Kogsak."), ("the Sack of Kogsak".to_string(), true));
        assert_eq!(clause("Ashmaw the dragon laid waste to Kogsak.").0, "Ashmaw the dragon laid waste to Kogsak");
    }
}
