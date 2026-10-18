//! Stories with names (WLD-01): a siege, a massacre or a great hunt is
//! given a name once it's over ("the Battle of Copper Gate", "the Wolf
//! Winter", "the Night of Red Knives"), from where and how it happened, and
//! goes into the chronicle and the legends as one named event.
//!
//! Names come from a hash of the fort and the moment, never the game's
//! RNG, so naming things changes nothing else that happens.

use super::*;
use crate::geom::Pos;
use serde::{Deserialize, Serialize};

/// What sort of named event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpisodeKind {
    /// A siege that was broken or beaten off.
    Battle,
    /// A siege or a beast that killed many and lost few.
    Massacre,
    /// The overseer's squad came home from a great hunt.
    Hunt,
}

/// One named event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Episode {
    /// "the Battle of Copper Gate".
    pub name: String,
    pub kind: EpisodeKind,
    /// When it ended.
    pub tick: u64,
    /// Where it broke.
    pub at: Pos,
    /// Our dead, as they're known.
    #[serde(default)]
    pub dead: Vec<String>,
    /// Who was there and lived, to remember it.
    #[serde(default)]
    pub veterans: Vec<u32>,
    /// The siege it names (index into `annals.sieges`), if one.
    #[serde(default)]
    pub siege: Option<usize>,
}

/// A death or a kill, remembered a while for naming what happened.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mark {
    pub tick: u64,
    pub at: Pos,
    /// The dwarf who died, or who struck the killing blow.
    pub dwarf: u32,
    pub name: String,
    pub died: bool,
    /// What killed them, or what they killed.
    #[serde(default)]
    pub foe: Option<String>,
    #[serde(default)]
    pub siege: Option<usize>,
}

/// The fort's named stories (`Game.stories`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Stories {
    #[serde(default)]
    pub episodes: Vec<Episode>,
    /// Deaths and kills of the last month.
    #[serde(default)]
    pub marks: Vec<Mark>,
}

/// How long a mark is kept.
const MARK_DAYS: u64 = 30;

/// A steady hash of words and a number (FNV-1a, then mixed).
pub fn hash(text: &str, salt: u64) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for b in text.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h ^= h >> 31;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^ (h >> 29)
}

/// One of `list`, chosen by `h` (and a salt, so one hash picks several words).
pub fn pick<'a>(h: u64, salt: u64, list: &[&'a str]) -> &'a str {
    list[(hash("", h ^ salt.wrapping_mul(0x2545_F491_4F6C_DD1D)) % list.len() as u64) as usize]
}

const ADJ: [&str; 20] = [
    "Copper", "Iron", "Red", "Grey", "Broken", "High", "Cold", "Black", "Ashen", "Long", "Old", "Bitter", "Raven", "Thorn",
    "Silent", "Bloody", "Tin", "Hollow", "Crooked", "Last",
];
const VICTORY: [&str; 4] = ["the Battle of {P}", "the Stand at {P}", "the Breaking at {P}", "the Defence of {P}"];
const BLOODY: [&str; 3] = ["the Bitter Battle of {P}", "the Red {S} at {P}", "the Dear-Bought Battle of {P}"];
const MASSACRE: [&str; 5] = ["the Night of Red {W}", "the Slaughter at {P}", "the {S} of Weeping", "the Red {S}", "the Sorrow at {P}"];
const BEASTLY: [&str; 3] = ["the {K} {S}", "the Night of the {Ks}", "the Coming of the {Ks}"];
const HUNT: [&str; 3] = ["the Great {K} Hunt", "the {K} {S}", "the Hunt at {P}"];
const LEGEND_HUNT: [&str; 2] = ["the Hunt of {N}", "the Slaying of {N}"];
const WEAPONS: [&str; 4] = ["Knives", "Spears", "Axes", "Blades"];
const ORDINALS: [&str; 5] = ["Second", "Third", "Fourth", "Fifth", "Sixth"];

fn cap(s: &str) -> String {
    super::heroes::cap_word(s)
}

fn list(names: &[String]) -> String {
    match names.len() {
        0 => String::new(),
        1 => names[0].clone(),
        n => format!("{} and {}", names[..n - 1].join(", "), names[n - 1]),
    }
}

fn near(a: Pos, b: Pos, r: i32) -> bool {
    a.z == b.z && (a.x - b.x).abs() <= r && (a.y - b.y).abs() <= r
}

impl Game {
    /// A dwarf died (`annals::note_death`): remember where.
    pub(super) fn story_death(&mut self, d: &Dwarf, siege: Option<usize>) {
        let foe = self.annals.wounded_by.iter().find(|w| w.0 == d.id).map(|w| w.1.clone());
        self.stories.marks.push(Mark { tick: self.tick, at: d.pos, dwarf: d.id, name: d.called(), died: true, foe, siege });
    }

    /// A dwarf struck the killing blow (`heroes::credit_kill`).
    pub(super) fn story_kill(&mut self, d: &Dwarf, c: &Creature) {
        let siege = if c.siege { self.annals.sieges.iter().rposition(|s| s.ended.is_none()) } else { None };
        self.stories.marks.push(Mark { tick: self.tick, at: c.pos, dwarf: d.id, name: d.called(), died: false, foe: Some(super::annals::kind_of(c)), siege });
    }

    fn season_word(&self) -> &'static str {
        SEASONS[((self.tick / SEASON) % 4) as usize]
    }

    /// Where the fighting was thickest among `marks`, if anywhere.
    fn thickest(marks: &[&Mark]) -> Option<Pos> {
        marks.iter().max_by_key(|m| (marks.iter().filter(|o| near(o.at, m.at, 6)).count(), u64::MAX - m.tick)).map(|m| m.at)
    }

    /// What kind of place `p` is: "Gate", "Bridge", "Hall", "Ford"...
    pub(super) fn place_noun(&self, p: Pos) -> &'static str {
        let w = &self.world;
        if !w.in_bounds(p) {
            return "Field";
        }
        let close = |r: i32, k: BuildingKind| self.buildings.values().any(|b| b.kind == k && b.built && near(b.pos, p, r));
        if close(2, BuildingKind::Bridge) {
            return "Bridge";
        }
        if let Some(work) = self.work_at(p) {
            if work.contains("gate") || work.contains("Gate") {
                return "Gate";
            }
            if work.contains("Tower") {
                return "Tower";
            }
            return "Wall";
        }
        let gate = self.overseer.as_ref().map_or(false, |o| o.top.into_iter().chain(o.base).any(|g| near(g, p, 4)));
        if gate || close(2, BuildingKind::Floodgate) {
            return "Gate";
        }
        if close(1, BuildingKind::Door) && !w.outside(p) {
            return "Door";
        }
        let around = || (-2..=2).flat_map(|dy| (-2..=2).map(move |dx| p.offset(dx, dy, 0))).filter(|q| w.in_bounds(*q));
        if w.outside(p) {
            if around().any(|q| w.t(q).water > 0 && !w.t(q).frozen) || around().any(|q| w.in_bounds(q.offset(0, 0, -1)) && w.t(q.offset(0, 0, -1)).water > 0) {
                return "Ford";
            }
            if around().filter(|q| w.t(*q).kind == TileKind::Tree).count() >= 3 {
                return "Wood";
            }
            return "Field";
        }
        if self.zone_at(p).is_some() || self.place_name(p).is_some() {
            return "Hall";
        }
        // A narrow way underground.
        let open = around().filter(|q| !w.t(*q).kind.solid()).count();
        if open <= 12 {
            "Tunnel"
        } else {
            "Deeps"
        }
    }

    /// What to call the place at `p` in a name: "Copper Gate", or the hall's
    /// own name ("the Great Dining Hall").
    pub(super) fn place_words(&self, p: Pos, h: u64) -> String {
        let noun = self.place_noun(p);
        if noun == "Hall" {
            if let Some(n) = self.place_name(p).filter(|n| n.starts_with("the ")) {
                return n;
            }
        }
        format!("{} {}", pick(h, 1, &ADJ), noun)
    }

    /// Fill a name template.
    fn fill(&self, t: &str, h: u64, place: &str, kind: &str, plural: &str, who: &str) -> String {
        t.replace("{P}", place)
            .replace("{S}", self.season_word())
            .replace("{W}", pick(h, 2, &WEAPONS))
            .replace("{Ks}", &cap(plural))
            .replace("{K}", &cap(kind))
            .replace("{N}", who)
    }

    /// The name, made unique: a second battle at the same place is "the
    /// Second Battle of Copper Gate".
    fn unique(&self, name: String) -> String {
        let n = self.stories.episodes.iter().filter(|e| e.name == name || e.name.ends_with(&name[4.min(name.len())..])).count();
        if n == 0 || !name.starts_with("the ") {
            return name;
        }
        format!("the {} {}", ORDINALS[(n - 1).min(ORDINALS.len() - 1)], &name[4..])
    }

    /// Tell a named event: announcement, card, chronicle and legend.
    fn tell_episode(&mut self, e: Episode, summary: String, color: (u8, u8, u8)) {
        let title = cap(&e.name);
        let at = e.at;
        self.msg(&format!("It will be remembered as {}.", e.name), color);
        self.spotlight(&title, &summary, Some(at), color);
        self.legend(format!("{}: {}", title, summary));
        self.year_log.stories.push(format!("{}: {}", title, summary));
        if e.kind != EpisodeKind::Hunt || self.year_log.battle.is_none() {
            self.year_log.battle = Some(e.name.clone());
        }
        if let Some(mut o) = self.overseer.take() {
            Self::note_ov(&mut o, self.tick, format!("Named it {}.", e.name));
            self.overseer = Some(o);
        }
        self.count("stories.named", 1);
        self.stories.episodes.push(e);
    }

    fn veterans_near(&self, at: Pos, r: i32) -> Vec<u32> {
        self.dwarves.iter().filter(|d| d.alive && (d.soldier || near(d.pos, at, r))).map(|d| d.id).collect()
    }

    /// A siege has ended (`on_siege_end`): name it, if it was a fight
    /// worth the name.
    pub(super) fn name_siege(&mut self, i: usize) {
        let Some(s) = self.annals.sieges.get(i).cloned() else { return };
        if self.stories.episodes.iter().any(|e| e.siege == Some(i)) {
            return;
        }
        let blood = s.dwarves_lost + s.foes_slain + s.foes_caught;
        if (blood as f64) < self.knob("story.battle_blood", 2.0) {
            return;
        }
        let marks: Vec<&Mark> = self.stories.marks.iter().filter(|m| m.siege == Some(i)).collect();
        let at = Self::thickest(&marks).or_else(|| self.fresh_battlefield()).or_else(|| self.overseer.as_ref().and_then(|o| o.top)).unwrap_or(self.embark);
        let dead: Vec<String> = marks.iter().filter(|m| m.died).map(|m| m.name.clone()).collect();
        let h = hash(&self.fortress_name, s.tick ^ (i as u64) << 20);
        let place = self.place_words(at, h);
        let fell = s.outcome == "the fortress fell";
        let massacre = fell || (s.dwarves_lost >= 3 && s.dwarves_lost >= 2 * s.foes_slain.max(1));
        let (kind, pool): (EpisodeKind, &[&str]) = if massacre {
            (EpisodeKind::Massacre, &MASSACRE)
        } else if s.dwarves_lost >= 2 && s.dwarves_lost >= s.foes_slain {
            (EpisodeKind::Battle, &BLOODY)
        } else {
            (EpisodeKind::Battle, &VICTORY)
        };
        let t = pick(h, 3, pool);
        let name = self.unique(self.fill(t, h, &place, "", "", ""));
        let fort = self.fortress_name.clone();
        let mut summary = format!("{} {} under {} came against {}", s.size, s.foe, s.leader, fort);
        summary.push_str(&match s.outcome.as_str() {
            "routed" => " and were routed".to_string(),
            "withdrew" => " and were beaten off".to_string(),
            o => format!(", and {}", o),
        });
        if s.foes_slain > 0 {
            summary.push_str(&format!("; {} of them were slain", s.foes_slain));
        }
        summary.push_str(&match dead.len() {
            0 if s.dwarves_lost == 0 => ", and not one dwarf fell.".to_string(),
            0 => format!(". We lost {}.", s.dwarves_lost),
            _ => format!(". We lost {}.", list(&dead)),
        });
        let e = Episode { name, kind, tick: self.tick, at, dead, veterans: self.veterans_near(at, 12), siege: Some(i) };
        let color = if massacre { (255, 110, 90) } else { (255, 215, 60) };
        self.tell_episode(e, summary, color);
    }

    /// The overseer's squad is home from a hunt (`hunts::end_hunt`): a great
    /// one gets a name. `beast`: the pack's kind ("wolf", "wolves") or the
    /// legend's title.
    pub(super) fn name_hunt(&mut self, squad: &str, kind: &str, plural: &str, kills: u32, legend: bool) {
        let great = legend || kills as f64 >= self.knob("story.hunt_kills", 3.0);
        if !great || kills == 0 {
            return;
        }
        let at = self.fresh_battlefield().or_else(|| self.overseer.as_ref().and_then(|o| o.top)).unwrap_or(self.embark);
        let h = hash(&self.fortress_name, self.tick ^ 0x4855_4e54);
        let place = self.place_words(at, h);
        let t = if legend { pick(h, 3, &LEGEND_HUNT) } else if self.season_word() == "Winter" { "the {K} Winter" } else { pick(h, 3, &HUNT) };
        let who = kind.split(" the ").next().unwrap_or(kind);
        let name = self.unique(self.fill(t, h, &place, kind, plural, who));
        let summary = if legend {
            format!("{} went out after {} and came home with its head.", squad, kind)
        } else {
            format!("{} hunted down {} {} that prowled about {}.", squad, kills, plural, self.fortress_name)
        };
        let e = Episode { name, kind: EpisodeKind::Hunt, tick: self.tick, at, dead: Vec::new(), veterans: self.veterans_near(at, 12), siege: None };
        self.tell_episode(e, summary, (255, 200, 90));
    }

    /// Daily: forget old marks, and name a massacre by beasts or worse
    /// outside any siege (three or more of ours dead within days, to the
    /// same kind of killer).
    pub(super) fn stories_daily(&mut self) {
        let tick = self.tick;
        self.stories.marks.retain(|m| tick < m.tick + MARK_DAYS * DAY || m.siege.map_or(false, |i| self.annals.sieges.get(i).map_or(false, |s| s.ended.is_none())));
        let window = self.knob("story.massacre_days", 3.0) as u64 * DAY;
        let since = self.stories.episodes.iter().filter(|e| e.kind == EpisodeKind::Massacre && e.siege.is_none()).map(|e| e.tick).max().unwrap_or(0);
        let recent: Vec<&Mark> = self.stories.marks.iter().filter(|m| m.died && m.siege.is_none() && m.foe.is_some() && m.tick > since && tick < m.tick + window).collect();
        let mut by: Vec<(String, usize)> = Vec::new();
        for m in &recent {
            let f = m.foe.clone().unwrap();
            match by.iter_mut().find(|b| b.0 == f) {
                Some(b) => b.1 += 1,
                None => by.push((f, 1)),
            }
        }
        let Some((foe, n)) = by.into_iter().max_by_key(|b| b.1) else { return };
        if (n as f64) < self.knob("story.massacre_dead", 3.0) {
            return;
        }
        let marks: Vec<&Mark> = recent.iter().copied().filter(|m| m.foe.as_deref() == Some(foe.as_str())).collect();
        let dead: Vec<String> = marks.iter().map(|m| m.name.clone()).collect();
        let at = Self::thickest(&marks).unwrap_or(self.embark);
        let h = hash(&self.fortress_name, tick ^ 0x4d41_5353);
        let place = self.place_words(at, h);
        let plural = crate::creature::Species::ALL.iter().map(|s| s.info()).find(|i| i.name == foe).map_or(format!("{}s", foe), |i| i.plural.to_string());
        let pool: &[&str] = if self.season_word() == "Winter" && pick(h, 4, &["a", "b"]) == "a" { &["the {K} Winter"] } else { &BEASTLY };
        let name = self.unique(self.fill(pick(h, 3, pool), h, &place, &foe, &plural, ""));
        let summary = format!("The {} fell upon {} and killed {}.", plural, self.fortress_name, list(&dead));
        let e = Episode { name, kind: EpisodeKind::Massacre, tick, at, dead, veterans: self.veterans_near(at, 12), siege: None };
        self.tell_episode(e, summary, (255, 110, 90));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::Species;

    fn quiet(seed: u64) -> Game {
        let mut g = Game::new(seed);
        g.creatures.retain(|_, c| c.tame);
        g
    }

    /// A goblin siege laid and then fought out: `lost` of ours killed and
    /// `slain` goblins killed by dwarf 0, the rest routed.
    pub(crate) fn fought_siege(g: &mut Game, lost: usize, slain: usize) {
        let n = g.neighbors.iter().find(|n| g.at_war_with(n)).cloned().or_else(|| g.neighbors.first().cloned()).expect("a neighbour");
        let mut foe = n.clone();
        foe.race = crate::overworld::Race::Goblin;
        g.lay_siege_of(&foe, 5, None);
        let gobs: Vec<u32> = g.creatures.values().filter(|c| c.siege).map(|c| c.id).collect();
        for k in 0..lost {
            let gob = g.creatures[&gobs[0]].clone();
            let mut d = std::mem::take(&mut g.dwarves[k + 1]);
            let before = d.body.clone();
            d.body.wound(crate::body::Part::LeftLeg, 9, true);
            g.note_wound(&mut d, &gob, &before);
            g.kill(&mut d, "was struck down");
            g.dwarves[k + 1] = d;
        }
        for (k, cid) in gobs.iter().enumerate() {
            // The slain fall to dwarf 0; the rest slink off.
            let c = g.creatures.remove(cid).unwrap();
            if k < slain {
                let mut d = std::mem::take(&mut g.dwarves[0]);
                g.credit_kill(&mut d, &c);
                g.dwarves[0] = d;
                g.creature_died(c);
            }
        }
        g.annals_tick();
        assert!(g.annals.sieges.last().unwrap().ended.is_some());
    }

    #[test]
    fn names_are_steady_and_vary() {
        assert_eq!(hash("Ironhold", 7), hash("Ironhold", 7));
        assert_ne!(hash("Ironhold", 7), hash("Ironhold", 8));
        let words: std::collections::HashSet<&str> = (0..40).map(|s| pick(hash("x", s), 1, &ADJ)).collect();
        assert!(words.len() >= 8, "{:?}", words);
    }

    #[test]
    fn a_broken_siege_gets_a_name_in_the_chronicle_and_legends() {
        let mut g = quiet(5);
        fought_siege(&mut g, 1, 4);
        let e = g.stories.episodes.last().expect("the siege should have a name").clone();
        assert_eq!(e.kind, EpisodeKind::Battle);
        assert!(e.name.starts_with("the "), "{}", e.name);
        assert_eq!(e.siege, Some(0));
        assert_eq!(e.dead.len(), 1);
        let title = cap(&e.name);
        assert!(g.log.iter().any(|l| l.text.contains(&e.name)), "no announcement");
        assert!(g.spotlights.iter().any(|s| s.title == title), "no card");
        assert!(g.overworld.events.iter().any(|ev| ev.text.starts_with(&title)), "not in the legends");
        assert_eq!(g.year_log.battle.as_deref(), Some(e.name.as_str()));
        assert_eq!(g.year_log.title(), format!("The Year of {}", e.name));
        g.tick = YEAR;
        g.write_chronicle_for_test();
        let c = g.chronicles.last().unwrap();
        assert!(c.lines.iter().any(|l| l.starts_with(&title)), "{:?}", c.lines);
    }

    #[test]
    fn a_massacre_is_named_as_one() {
        let mut g = quiet(7);
        fought_siege(&mut g, 4, 0);
        let e = g.stories.episodes.last().expect("named").clone();
        assert_eq!(e.kind, EpisodeKind::Massacre);
        assert_eq!(e.dead.len(), 4);
        // A second siege at the same spot gets a name of its own.
        fought_siege(&mut g, 0, 5);
        let names: Vec<&str> = g.stories.episodes.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names.len(), 2);
        assert_ne!(names[0], names[1]);
    }

    #[test]
    fn a_skirmish_with_no_blood_is_not_named() {
        let mut g = quiet(5);
        let n = g.neighbors.first().cloned().unwrap();
        g.lay_siege_of(&n, 3, None);
        let ids: Vec<u32> = g.creatures.values().filter(|c| c.siege).map(|c| c.id).collect();
        for id in ids {
            g.creatures.remove(&id);
        }
        g.annals_tick();
        assert!(g.annals.sieges[0].ended.is_some());
        assert!(g.stories.episodes.is_empty());
    }

    #[test]
    fn wolves_that_kill_three_make_a_named_massacre() {
        let mut g = quiet(9);
        g.tick = 3 * SEASON + 5 * DAY;
        let at = g.dwarves[0].pos;
        let id = g.spawn_creature(Species::Wolf, at, DAY);
        let wolf = g.creatures[&id].clone();
        for k in 0..3 {
            let mut d = std::mem::take(&mut g.dwarves[k]);
            let before = d.body.clone();
            d.body.wound(crate::body::Part::LeftArm, 9, true);
            g.note_wound(&mut d, &wolf, &before);
            g.kill(&mut d, "has bled to death");
            g.dwarves[k] = d;
        }
        g.stories_daily();
        let e = g.stories.episodes.last().expect("the wolves' work should be named").clone();
        assert_eq!(e.kind, EpisodeKind::Massacre);
        assert!(e.name.contains("Wol"), "{}", e.name);
        assert_eq!(e.dead.len(), 3);
        // Once only.
        g.stories_daily();
        assert_eq!(g.stories.episodes.len(), 1);
    }

    #[test]
    fn a_great_hunt_is_named() {
        let mut g = quiet(11);
        g.tick = 3 * SEASON + DAY;
        g.name_hunt("The Axes of Dawn", "wolf", "wolves", 1, false);
        assert!(g.stories.episodes.is_empty(), "one wolf is no great hunt");
        g.name_hunt("The Axes of Dawn", "wolf", "wolves", 4, false);
        let e = g.stories.episodes.last().expect("named").clone();
        assert_eq!(e.kind, EpisodeKind::Hunt);
        assert_eq!(e.name, "the Wolf Winter");
        g.name_hunt("The Axes of Dawn", "Emberwing the dragon", "", 1, true);
        let e = g.stories.episodes.last().unwrap();
        assert!(e.name.ends_with("of Emberwing"), "{}", e.name);
    }

    #[test]
    fn places_have_nouns() {
        let mut g = quiet(5);
        let p = g.open_patch(5, 5, 6).offset(2, 2, 0);
        assert_eq!(g.place_noun(p), "Field");
        assert!(g.place_words(p, 3).ends_with(" Field"));
    }
}
