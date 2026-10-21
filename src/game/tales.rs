//! Legends that drift and lore that is lost (O09, WLD-09 and WLD-18).
//!
//! *Drift.* A legend retold year after year grows: five goblins become a
//! dozen, the deed passes to a greater hero's name, it picks up a flourish
//! ("and the mountain shook"), it turns into a tale ("They say ..."), or it
//! fades from memory altogether. Each people tells it its own way: the
//! changes are hashed from the legend, the year and the teller, so the same
//! deed is a dwarven boast in one hall and an elven lament in a grove. The
//! truth (`Event.text`) never changes; our own people's telling is kept in
//! `Event.told`, and anyone else's is worked out when asked (`tale_for`).
//!
//! *Forgotten lore.* At embark the fortress knows its own people's past, the
//! country round about and living memory; the rest of history is forgotten
//! (`Overworld.known`). It comes back through ruins (an expedition copies
//! their engravings, a sealed hall's walls are read), through bards' songs,
//! and it is kept from fading by carving it in stone: an engraved legend
//! never fades. The legends viewer shows the gaps.
//!
//! Nothing here draws on the game's random numbers.

use super::*;
use crate::overworld::{Overworld, Race};
use serde::{Deserialize, Serialize};

/// In a year, the chance (in a hundred) that an old tale changes.
const DRIFT: u64 = 20;
/// A tale must be this many years old before it drifts.
const SETTLE: i32 = 2;
/// At embark, the years of living memory.
const MEMORY: i32 = 30;
/// At embark, how far (in world cells) the country round about reaches.
const COUNTRY: i32 = 20;

/// What the year did to the fortress's lore, for its chronicle.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Lore {
    /// Tales that grew this year (event indices).
    #[serde(default)]
    pub grown: Vec<usize>,
    /// Tales that faded from memory this year.
    #[serde(default)]
    pub faded: Vec<usize>,
    /// Lore found again: a line each.
    #[serde(default)]
    pub found: Vec<String>,
    /// The age that began at the turn of the year.
    #[serde(default)]
    pub age: Option<super::ages::Age>,
}

/// What one year of retelling did to a tale.
#[derive(Clone, Debug, PartialEq)]
pub enum Step {
    Same,
    Grew(String),
    Faded,
}

const OPEN_DWARF: [&str; 3] = ["They say", "The old songs tell that", "Grandmothers swear that"];
const OPEN_HUMAN: [&str; 3] = ["It is told that", "The minstrels sing that", "In the inns they say"];
const OPEN_ELF: [&str; 3] = ["The trees remember that", "The singers of the grove say", "It is sung under the leaves that"];
const OPEN_GOBLIN: [&str; 3] = ["Around the pits they boast that", "The goblins jeer that", "In the dark towers they say"];
const FLOURISH: [&str; 8] = [
    ", and the mountain shook",
    ", with a single blow",
    ", though wounded thrice",
    ", singing all the while",
    ", and the sky went dark",
    ", and the stones wept",
    ", by the light of a falling star",
    ", and was never the same after",
];
/// The ladder numbers climb as a tale is retold.
const LADDER: [&str; 15] = ["two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "a dozen", "a score", "fifty", "a hundred", "a thousand", "ten thousand"];

fn openers(r: Race) -> &'static [&'static str] {
    match r {
        Race::Dwarf => &OPEN_DWARF,
        Race::Human => &OPEN_HUMAN,
        Race::Elf => &OPEN_ELF,
        Race::Goblin => &OPEN_GOBLIN,
    }
}

fn lower_first(s: &str) -> String {
    // "The goblins ..." becomes "the goblins ..."; a name keeps its capital.
    if let Some(rest) = s.strip_prefix("The ") {
        format!("the {}", rest)
    } else {
        s.to_string()
    }
}

/// Make the first number in `text` bigger by three to five rungs:
/// "3 goblins" → "six goblins", "five" → "eight", "nine" or "ten".
pub fn grow_number(text: &str, h: u64) -> Option<String> {
    let words: Vec<&str> = text.split(' ').collect();
    for (i, w) in words.iter().enumerate() {
        let bare = w.trim_end_matches(|c: char| !c.is_alphanumeric());
        let tail = &w[bare.len()..];
        let up = 3 + (h % 3) as usize;
        let bigger = if let Ok(n) = bare.parse::<u32>() {
            if n < 2 {
                continue;
            }
            let at = LADDER.iter().position(|l| *l == number_word(n)).unwrap_or(match n {
                0..=11 => 8,
                12..=19 => 9,
                20..=49 => 10,
                50..=99 => 11,
                100..=999 => 12,
                _ => 13,
            });
            LADDER[(at + up).min(LADDER.len() - 1)].to_string()
        } else if let Some(at) = LADDER.iter().position(|l| !l.contains(' ') && *l == bare) {
            LADDER[(at + up).min(LADDER.len() - 1)].to_string()
        } else if bare == "dozen" || bare == "score" || bare == "hundred" || bare == "thousand" {
            // "a dozen" climbs as one: the "a" before it goes with it.
            if i == 0 || words[i - 1] != "a" {
                continue;
            }
            let at = LADDER.iter().position(|l| *l == format!("a {}", bare)).unwrap_or(9);
            if at + 1 >= LADDER.len() {
                return None;
            }
            let mut v: Vec<String> = words.iter().map(|w| w.to_string()).collect();
            v[i - 1] = String::new();
            v[i] = format!("{}{}", LADDER[(at + up).min(LADDER.len() - 1)], tail);
            return Some(v.into_iter().filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" "));
        } else {
            continue;
        };
        let mut v: Vec<String> = words.iter().map(|w| w.to_string()).collect();
        v[i] = format!("{}{}", bigger, tail);
        return Some(v.join(" "));
    }
    None
}

fn number_word(n: u32) -> &'static str {
    match n {
        2 => "two",
        3 => "three",
        4 => "four",
        5 => "five",
        6 => "six",
        7 => "seven",
        8 => "eight",
        9 => "nine",
        10 => "ten",
        _ => "",
    }
}

/// The name a legend starts with ("Urist Kadol slew ..."), if it does.
fn leading_name(text: &str) -> Option<String> {
    let mut name: Vec<&str> = Vec::new();
    for w in text.split(' ') {
        let bare = w.trim_end_matches(|c: char| !c.is_alphanumeric());
        if bare.chars().next().map_or(false, |c| c.is_uppercase()) && bare.len() == w.len() {
            name.push(w);
        } else {
            break;
        }
    }
    if name.len() < 2 || ["The", "A", "An", "In", "They", "It", "Around"].contains(&name[0]) {
        return None;
    }
    Some(name.join(" "))
}

/// One year of retelling `now` (the tale as it stands; `truth` is the
/// event's text): unchanged most years, else grown or faded.
pub fn step(truth: &str, now: &str, salt: u64, year: i32, race: Race, hero: Option<&str>) -> Step {
    let h = stories::hash(truth, salt ^ (year as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
    if h % 100 >= DRIFT {
        return Step::Same;
    }
    let roll = (h >> 8) % 100;
    let opened = openers(race).iter().any(|o| now.starts_with(o)) || now.starts_with("A bard sang");
    let flourished = FLOURISH.iter().any(|f| now.contains(f));
    let open = || format!("{} {}", stories::pick(h, 5, openers(race)), lower_first(now));
    let flourish = || {
        let body = now.trim_end_matches('.');
        format!("{}{}.", body, stories::pick(h, 6, &FLOURISH))
    };
    let chosen = if roll < 30 {
        grow_number(now, h >> 16)
    } else if roll < 45 {
        match (hero, leading_name(now)) {
            (Some(hero), Some(name)) if !now.contains(hero) && !hero.contains(&name) => Some(now.replacen(&name, hero, 1)),
            _ => None,
        }
    } else if roll < 65 {
        (!opened).then(open)
    } else if roll < 85 {
        (!flourished).then(flourish)
    } else {
        return Step::Faded;
    };
    let told = chosen.or_else(|| (!flourished).then(flourish)).or_else(|| (!opened).then(open));
    match told {
        Some(t) if t != now => Step::Grew(t),
        _ => Step::Same,
    }
}

/// The hero a people puts into its tales: its greatest named figure.
#[allow(dead_code)]
fn civ_hero(ow: &Overworld, civ: u32) -> Option<String> {
    ow.figures.iter().filter(|f| f.civ == Some(civ) && (f.kills > 0 || f.title.is_some())).max_by_key(|f| (f.kills, f.id)).map(|f| f.name.clone())
}

/// Event `i` as people `teller` (a civ id; None for our own fortress's
/// people, as `Event.told` keeps it) tells it in `year`, or None if they
/// have let it fade. For the adventurer's talk (A7): a speaker's rumours of
/// a place come through here, so each people tells the old deeds its own
/// way and every year a little bigger.
#[allow(dead_code)] // A7 talk reads it next round.
pub fn tale_for(ow: &Overworld, i: usize, teller: Option<u32>, year: i32) -> Option<String> {
    let e = ow.events.get(i)?;
    let Some(civ) = teller else {
        return ow.is_known(i).then(|| e.told.clone().unwrap_or_else(|| e.text.clone()));
    };
    let race = ow.civs.get(civ as usize).map_or(Race::Dwarf, |c| c.race);
    let hero = civ_hero(ow, civ);
    let mut now = e.text.clone();
    for y in (e.year + SETTLE).max(year - 300)..=year {
        match step(&e.text, &now, civ as u64 + 1, y, race, hero.as_deref()) {
            Step::Same => {}
            Step::Grew(t) => now = t,
            Step::Faded => return None,
        }
    }
    Some(now)
}

/// The tales people `teller` tells in `year` that name `name`, newest
/// first, as (event index, the tale): what an adventurer hears asking
/// after a place or a figure (A7).
#[allow(dead_code)] // A7 talk reads it next round.
pub fn tales_about(ow: &Overworld, name: &str, teller: Option<u32>, year: i32) -> Vec<(usize, String)> {
    if name.is_empty() {
        return Vec::new();
    }
    (0..ow.events.len()).rev().filter(|&i| ow.events[i].text.contains(name)).filter_map(|i| tale_for(ow, i, teller, year).map(|t| (i, t))).collect()
}

impl Game {
    /// The year as the legends count it.
    fn lore_year(&self) -> i32 {
        self.start_year + (self.tick / YEAR) as i32
    }

    /// Our people's race, for how our tales are told.
    fn our_race(&self) -> Race {
        self.civ.and_then(|c| self.overworld.civs.get(c as usize)).map_or(Race::Dwarf, |c| c.race)
    }

    /// The fortress's greatest hero, whose name creeps into its tales.
    fn fort_hero(&self) -> Option<String> {
        self.dwarves.iter().filter(|d| d.epithet.is_some()).max_by_key(|d| (d.epithet_rank, d.kills, d.id)).map(|d| d.called())
    }

    /// The legends carved somewhere in the fortress (1-based, as tiles
    /// keep them): stone keeps them from fading.
    fn carved(&self) -> HashSet<u32> {
        self.world.tiles.iter().filter(|t| t.smooth >= super::detail::ENGRAVED && t.engraving != 0).map(|t| t.engraving & !super::detail::GEM_SET).collect()
    }

    /// At embark: the fortress knows its own people's past, the country
    /// round about and living memory; the rest is forgotten.
    pub(super) fn forget_lore(&mut self) {
        let ow = &self.overworld;
        let (sx, sy) = self.site;
        let mut names: Vec<String> = Vec::new();
        if let Some(c) = self.civ.and_then(|c| ow.civs.get(c as usize)) {
            names.push(c.name.clone());
            names.extend(ow.figures.iter().filter(|f| f.civ == Some(c.id)).map(|f| f.name.clone()));
        }
        for s in &ow.sites {
            if s.civ.is_some() && s.civ == self.civ || (s.x - sx).abs().max((s.y - sy).abs()) <= COUNTRY {
                names.push(s.name.clone());
            }
        }
        names.retain(|n| !n.is_empty());
        let recent = ow.year - MEMORY;
        let known: Vec<bool> = ow.events.iter().map(|e| e.year >= recent || names.iter().any(|n| e.text.contains(n.as_str()))).collect();
        self.overworld.known = known;
    }

    /// The share of the world's history the fortress has forgotten.
    pub fn lore_lost(&self) -> f64 {
        let n = self.overworld.events.len();
        if n == 0 || self.overworld.known.is_empty() {
            return 0.0;
        }
        (0..n).filter(|&i| !self.overworld.is_known(i)).count() as f64 / n as f64
    }

    /// What lost lore adds to a ruin expedition's worth to the overseer:
    /// one when more than half of history is forgotten, two for a pious
    /// or greedy one (old gods, old gold).
    pub(super) fn ruin_lore_worth(&self, o: &Overseer) -> i32 {
        if self.lore_lost() <= 0.5 {
            return 0;
        }
        use super::personality::Trait;
        1 + (o.has(Trait::Pious) || o.has(Trait::Greedy)) as i32
    }

    /// Remember event `i` again; whether it had been forgotten.
    fn recall(&mut self, i: usize) -> bool {
        if self.overworld.is_known(i) {
            return false;
        }
        self.overworld.known[i] = true;
        self.count("o09.lore_found", 1);
        true
    }

    /// A scene to engrave: the legend picked, or if the fortress has
    /// forgotten it, the nearest one it still knows.
    pub(super) fn known_scene(&self, scene: u32) -> u32 {
        let Some(i) = scene.checked_sub(1).map(|i| i as usize) else { return scene };
        let n = self.overworld.events.len();
        if self.overworld.is_known(i) {
            return scene;
        }
        (0..n).filter(|&j| self.overworld.is_known(j)).min_by_key(|&j| ((j as i64 - i as i64).abs(), j)).map_or(scene, |j| j as u32 + 1)
    }

    /// The turn of the year, before the chronicle is written: the world may
    /// call a new age, and the fortress's old tales are retold.
    pub(super) fn lore_new_year(&mut self) {
        if self.tick < YEAR / 2 {
            return;
        }
        self.lore.age = self.ages_new_year();
        self.retell_tales();
    }

    /// Each of the fortress's own tales older than two years may grow in
    /// the telling, or fade (unless it is carved in stone).
    fn retell_tales(&mut self) {
        let year = self.lore_year();
        let fort = self.fortress_name.clone();
        let race = self.our_race();
        let hero = self.fort_hero();
        let salt = self.civ.map_or(0, |c| c as u64 + 1);
        let carved = self.carved();
        for i in 0..self.overworld.events.len() {
            let e = &self.overworld.events[i];
            if e.year > year - SETTLE || !e.text.contains(&fort) || !self.overworld.is_known(i) {
                continue;
            }
            let now = e.told.clone().unwrap_or_else(|| e.text.clone());
            match step(&e.text, &now, salt, year, race, hero.as_deref()) {
                Step::Same => {}
                Step::Grew(t) => {
                    let e = &mut self.overworld.events[i];
                    e.told = Some(t);
                    e.fame = e.fame.saturating_add(1);
                    self.lore.grown.push(i);
                    self.count("o09.tales_grown", 1);
                }
                Step::Faded if carved.contains(&(i as u32 + 1)) => {}
                Step::Faded => {
                    if self.overworld.known.is_empty() {
                        self.overworld.known = vec![true; self.overworld.events.len()];
                    }
                    self.overworld.known[i] = false;
                    self.lore.faded.push(i);
                    self.count("o09.tales_faded", 1);
                }
            }
        }
    }

    /// A bard sang legend `i` in the tavern: if the fortress had forgotten
    /// it, now it remembers.
    pub(super) fn lore_sung(&mut self, i: usize) {
        if self.recall(i) {
            let text = self.overworld.events[i].text.clone();
            self.msg(&format!("Nobody here remembered that song: {}", text), (200, 180, 255));
            self.lore.found.push(format!("A bard sang an old song nobody here remembered: {}", text));
        }
    }

    #[cfg(test)]
    pub fn lore_sung_for_test(&mut self, i: usize) {
        self.lore_sung(i);
    }

    /// Now and then a bard makes a tale up: a deed of one of ours that
    /// never happened, sung as if it had. Whether one did.
    pub(super) fn bard_invents(&mut self, c: &Creature) -> bool {
        let h = super::names::hash(&[self.tick, c.id as u64, 0xBA4D]);
        let year = self.lore_year();
        if h % 5 != 0 || self.overworld.events.iter().rev().take_while(|e| e.year == year).any(|e| e.bard) {
            return false;
        }
        let alive: Vec<&Dwarf> = self.dwarves.iter().filter(|d| d.alive && !d.is_child()).collect();
        if alive.is_empty() {
            return false;
        }
        let d = alive[(h >> 8) as usize % alive.len()];
        let (who, id) = (d.called(), d.id);
        const FEATS: [&str; 6] = [
            "wrestled a cave bear bare-handed and won",
            "drank a whole barrel of stoutcap ale and dug a tunnel straight after",
            "outran a river in flood",
            "split a boulder with a single blow of the fist",
            "sang a dragon to sleep",
            "talked a goblin warband into going home",
        ];
        let feat = stories::pick(h, 7, &FEATS);
        let fort = self.fortress_name.clone();
        let text = format!("A bard sang that {} of {} once {}.", who, fort, feat);
        self.legend(text);
        if let Some(e) = self.overworld.events.last_mut() {
            e.bard = true;
            e.fame = 1;
        }
        self.msg(&format!("{} the bard sings a new song in the tavern: how {} once {}. Nobody can remember it happening.", c.title(), who, feat), (200, 180, 255));
        let tick = self.tick;
        if let Some(d) = self.dwarves.iter_mut().find(|d| d.id == id) {
            d.think("was sung of for a deed they never did", -10, tick);
        }
        self.count("o09.bard_tales", 1);
        true
    }

    /// An expedition home from the ruins of `site`: the delvers copied its
    /// engravings, and the fortress remembers five to ten forgotten tales of
    /// the place and its people.
    pub(super) fn lore_from_ruin(&mut self, site: u32) {
        let Some(s) = self.overworld.sites.iter().find(|s| s.id == site) else { return };
        let mut names = vec![s.name.clone()];
        if let Some(c) = s.civ.and_then(|c| self.overworld.civs.get(c as usize)) {
            names.push(c.name.clone());
        }
        let sname = s.name.clone();
        let lost: Vec<usize> = (0..self.overworld.events.len()).filter(|&i| !self.overworld.is_known(i) && names.iter().any(|n| self.overworld.events[i].text.contains(n.as_str()))).collect();
        let n = 5 + (super::names::hash(&[site as u64, self.tick]) % 6) as usize;
        let found: Vec<usize> = lost.into_iter().take(n).collect();
        if found.is_empty() {
            return;
        }
        for &i in &found {
            self.recall(i);
        }
        let oldest = self.overworld.events[found[0]].year;
        let line = format!(
            "The delvers copied the engravings of {}: {} forgotten tale{} of the place came home, the oldest from {}.",
            sname,
            found.len(),
            if found.len() == 1 { "" } else { "s" },
            super::adv::sitegen::lore::in_year(oldest)
        );
        self.msg(&line, (220, 200, 255));
        self.lore.found.push(line);
    }

    /// The miners broke into the sealed hall: its walls tell of a forgotten
    /// age, and the fortress reads them.
    pub(super) fn lore_from_vault(&mut self) {
        let Some(v) = self.vault.clone() else { return };
        let mut found: Vec<usize> = Vec::new();
        for x in v.min.x - 1..=v.max.x + 1 {
            for y in v.min.y - 1..=v.max.y + 1 {
                let p = Pos::new(x, y, v.min.z);
                let Some(t) = self.world.get(p) else { continue };
                if t.smooth < super::detail::ENGRAVED {
                    continue;
                }
                if let Some(i) = (t.engraving & !super::detail::GEM_SET).checked_sub(1).map(|i| i as usize) {
                    if !found.contains(&i) && self.recall(i) {
                        found.push(i);
                    }
                }
            }
        }
        if found.is_empty() {
            return;
        }
        let line = format!("The carvings of {} were read: {} tales of a forgotten age are remembered.", v.name, found.len());
        self.msg(&line, (220, 200, 255));
        self.lore.found.push(line);
    }

    /// The chronicle's lines on the world's lore: a new age, the tales
    /// that grew or faded, and what was found again.
    pub(super) fn lines_lore(&mut self) -> Vec<String> {
        let lore = std::mem::take(&mut self.lore);
        let mut v = Vec::new();
        if let Some(a) = &lore.age {
            let why = if a.why.is_empty() { String::new() } else { format!(" {}", a.why) };
            v.push(format!("As the year turned, the world began to speak of {}.{}", a.name, why));
        }
        for &i in lore.grown.iter().rev().take(2) {
            let Some(e) = self.overworld.events.get(i) else { continue };
            if let Some(t) = &e.told {
                v.push(format!("The tale of the year {} grows in the telling: \"{}\"", e.year, t));
            }
        }
        if !lore.faded.is_empty() {
            let n = lore.faded.len();
            let one = &self.overworld.events[lore.faded[0]];
            v.push(if n == 1 {
                format!("Nobody tells any more how {}; it was never carved in stone.", lower_first(one.text.trim_end_matches('.')))
            } else {
                format!("{} old tales of the fortress faded from memory, none of them carved in stone.", n)
            });
        }
        v.extend(lore.found.into_iter().take(3));
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_climb_the_ladder() {
        assert_eq!(grow_number("Urist slew 3 goblins.", 0).as_deref(), Some("Urist slew six goblins."));
        assert_eq!(grow_number("five goblins fell.", 0).as_deref(), Some("eight goblins fell."));
        assert_eq!(grow_number("a dozen goblins fell.", 0).as_deref(), Some("a hundred goblins fell."));
        assert_eq!(grow_number("Nobody fell.", 0), None);
    }

    #[test]
    fn an_old_tale_grows_in_the_telling() {
        let mut g = Game::new(81);
        let fort = g.fortress_name.clone();
        let truth = format!("Urist Kadol of {} slew 5 goblins at the gate.", fort);
        g.legend(truth.clone());
        let i = g.overworld.events.len() - 1;
        // Ten years on, retold every year.
        let mut grew = Vec::new();
        for y in 1..=12 {
            g.tick = y * YEAR;
            g.retell_tales();
            if let Some(t) = &g.overworld.events[i].told {
                if grew.last() != Some(t) {
                    grew.push(t.clone());
                }
            }
            if !g.overworld.is_known(i) {
                break;
            }
        }
        assert_eq!(g.overworld.events[i].text, truth, "the truth stays as it was");
        assert!(!grew.is_empty() || !g.overworld.is_known(i), "in twelve years the tale changed");
        if let Some(t) = g.overworld.events[i].told.clone() {
            assert_ne!(t, truth);
            assert!(g.overworld.events[i].fame >= 1);
            // The chronicle tells it.
            g.lore.grown.push(i);
            assert!(g.lines_lore().iter().any(|l| l.contains(&t)));
        }
        // Another people tells the same deed its own way.
        let year = g.lore_year() + 40;
        let tellings: Vec<Option<String>> = (0..g.overworld.civs.len() as u32).map(|c| tale_for(&g.overworld, i, Some(c), year)).collect();
        assert!(tellings.iter().any(|t| t.as_deref() != Some(truth.as_str())), "forty years on, someone tells it differently");
        // Telling is steady: asked twice, the same tale.
        assert_eq!(tellings[0], tale_for(&g.overworld, i, Some(0), year));
    }

    #[test]
    fn a_carved_tale_does_not_fade() {
        let mut g = Game::new(81);
        let fort = g.fortress_name.clone();
        g.legend(format!("The miners of {} found a vein of gold.", fort));
        let i = g.overworld.events.len() - 1;
        // Carve it on a tile.
        let t = g.world.t_mut(g.embark);
        t.smooth = super::super::detail::ENGRAVED;
        t.engraving = i as u32 + 1;
        for y in 1..=200 {
            g.tick = y * YEAR;
            g.retell_tales();
        }
        assert!(g.overworld.is_known(i), "the stone remembers");
    }

    #[test]
    fn a_ruin_expedition_brings_lore_home() {
        let mut g = Game::new(81);
        let lost = g.lore_lost();
        assert!(lost > 0.0, "history before living memory is forgotten at embark");
        let Some(s) = g.overworld.sites.iter().find(|s| (0..g.overworld.events.len()).any(|i| !g.overworld.is_known(i) && g.overworld.events[i].text.contains(&s.name))).map(|s| s.id) else {
            return;
        };
        g.lore_from_ruin(s);
        assert!(g.lore_lost() < lost);
        assert!(g.lore.found.iter().any(|l| l.contains("forgotten tale")));
    }
}
