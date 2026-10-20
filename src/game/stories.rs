//! Stories with names (WLD-01): a siege, a massacre or a great hunt is
//! given a name once it's over ("the Battle of Copper Gate", "the Wolf
//! Winter", "the Night of Red Knives"), from where and how it happened, and
//! goes into the chronicle and the legends as one named event. The place
//! where it broke takes a name from the deed done there (WLD-02: "Kolvar's
//! Stand", "the Weeping Gate"), shown on look and in the legends.
//! Children are named for the fallen (SOC-01), with the reason in the
//! chronicle. Friends who marry are wed before the fort with a feast and
//! gifts, and a jealous rival may turn up (SOC-02). A broken siege ends in
//! a victory feast the overseer calls in its own voice, with the dead of
//! the siege toasted by name (SIE-03).
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

/// A wedding or a victory feast under way, for the camera.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Occasion {
    /// "the wedding of Kolvar and Ingra".
    pub what: String,
    /// Who to watch, the first still here.
    pub who: Vec<u32>,
    pub wedding: bool,
    pub until: u64,
}

/// A place named for what happened there.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Place {
    /// "Kolvar's Stand".
    pub name: String,
    pub at: Pos,
    /// How far around `at` the name holds.
    pub r: i32,
    /// "where Kolvar the Grim slew three goblins in the Battle of Copper Gate".
    pub deed: String,
    pub year: i32,
    /// What sort of place (M2: halls, gates, roads and works are named too).
    #[serde(default)]
    pub kind: super::places::PlaceKind,
    /// A box from `at` to `b` instead of the ground within `r` of `at`.
    #[serde(default)]
    pub b: Option<Pos>,
    /// How great the deed it is named for: a heavier one renames it.
    #[serde(default)]
    pub weight: u8,
    /// The player named it, and nothing renames it.
    #[serde(default)]
    pub by_player: bool,
}

/// The fort's named stories (`Game.stories`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Stories {
    #[serde(default)]
    pub episodes: Vec<Episode>,
    /// Deaths and kills of the last month.
    #[serde(default)]
    pub marks: Vec<Mark>,
    #[serde(default)]
    pub places: Vec<Place>,
    /// The fallen children have been named for, once each.
    #[serde(default)]
    pub honoured: Vec<String>,
    /// News to tell once the day's births are announced.
    #[serde(default)]
    pub pending: Vec<String>,
    #[serde(default)]
    pub occasion: Option<Occasion>,
    /// Old battlefields (`battlefields.rs`).
    #[serde(default)]
    pub fields: Vec<super::battlefields::Field>,
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

/// "Kolvar" from "Kolvar Ironfist" (or "Kolvar the Grim").
pub fn first_name(n: &str) -> &str {
    n.split(' ').next().unwrap_or(n)
}

fn number(n: usize) -> String {
    match n {
        1 => "one".into(),
        2 => "two".into(),
        3 => "three".into(),
        4 => "four".into(),
        5 => "five".into(),
        6 => "six".into(),
        n => n.to_string(),
    }
}

fn plural_of(kind: &str) -> String {
    crate::creature::Species::from_name(kind).map(|s| s.info()).map_or(format!("{}s", kind), |i| i.plural.to_string())
}

fn list(names: &[String]) -> String {
    match names.len() {
        0 => String::new(),
        1 => names[0].clone(),
        n => format!("{} and {}", names[..n - 1].join(", "), names[n - 1]),
    }
}

/// What an overseer of trait `t` says calling a victory feast.
fn feast_words(t: super::personality::Trait, h: u64, fort: &str) -> String {
    use super::personality::Trait::*;
    let pool: &[&str] = match t {
        Cautious => &["Drink, but keep one eye on the hills.", "Eat well tonight; we mend the walls tomorrow."],
        Bold => &["Let them come again, and we'll break them again!", "Ale for every blade that stood!"],
        Vain => &["Let every caravan hear how {fort} stood!", "Bring out the fine mugs. They'll sing of this."],
        Pious => &["The gods stood with us. Drink to them, and to our dead.", "A blessing on the hall, and on the fallen."],
        Greedy => &["Open the good casks. Just this once.", "Eat, drink: it cost us dear enough."],
        Sentimental => &["Drink to those who aren't here to drink with us.", "Hold each other close tonight."],
        Stubborn => &["We held. We always hold.", "They broke on {fort}, as they always will."],
        Cheerful => &["Out with the ale! Tonight we dance!", "Music! Food! We're alive!"],
    };
    pick(h, 2, pool).replace("{fort}", fort)
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
        if let Some(pl) = self.story_place(p) {
            return pl.name.clone();
        }
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
        self.name_battle_place(&e);
        // O09: where many died, the ground remembers (battlefields.rs).
        self.remember_field(&e);
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

    /// The named place at `p`, if any.
    pub fn story_place(&self, p: Pos) -> Option<&Place> {
        self.stories.places.iter().find(|pl| pl.b.is_none() && near(pl.at, p, pl.r))
    }

    /// What the look panel says of a named place: "Kolvar's Stand: where…".
    pub fn story_place_at(&self, p: Pos) -> Option<String> {
        self.place_at(p).map(|pl| format!("{}: {} ({})", cap(&pl.name), pl.deed, pl.year))
    }

    /// The place where `e` broke takes a name from the deed done there:
    /// a dwarf's stand, a hero's fall, a massacre's weeping, or else the
    /// name the battle gave it. A place already named keeps its name.
    fn name_battle_place(&mut self, e: &Episode) {
        if self.story_place(e.at).is_some() {
            return;
        }
        let at = e.at;
        let h = hash(&e.name, e.tick);
        let noun = self.place_noun(at);
        let marks: Vec<&Mark> = self.stories.marks.iter().filter(|m| near(m.at, at, 6) && m.tick + 10 * DAY >= e.tick && (e.siege.is_none() || m.siege == e.siege)).collect();
        // Who did the most killing here.
        let mut killers: Vec<(u32, String, usize, Option<String>)> = Vec::new();
        for m in marks.iter().filter(|m| !m.died) {
            match killers.iter_mut().find(|k| k.0 == m.dwarf) {
                Some(k) => k.2 += 1,
                None => killers.push((m.dwarf, m.name.clone(), 1, m.foe.clone())),
            }
        }
        let best = killers.into_iter().max_by_key(|k| (k.2, u32::MAX - k.0));
        let hero_dead = marks.iter().find(|m| m.died && self.dwarves.iter().any(|d| d.id == m.dwarf && d.epithet.is_some())).map(|m| m.name.clone());
        let (name, deed) = match (best, hero_dead) {
            (Some((_, who, n, foe)), _) if n >= 2 => {
                let foes = foe.map_or("foes".to_string(), |f| plural_of(&f));
                (format!("{} Stand", super::gore::possessive(first_name(&who))), format!("where {} slew {} {} in {}", who, number(n), foes, e.name))
            }
            (_, Some(m)) => (format!("{} Fall", super::gore::possessive(first_name(&m))), format!("where {} fell in {}", m, e.name)),
            _ if e.kind == EpisodeKind::Massacre && !e.dead.is_empty() => {
                let w = pick(h, 5, &["Weeping", "Red", "Silent", "Widows'"]);
                (format!("the {} {}", w, noun), format!("where {} died in {}", list(&e.dead), e.name))
            }
            _ => {
                // The name the battle gave it: "Copper Gate" from "the Battle of Copper Gate".
                let Some(p) = e.name.rsplit_once(" of ").or_else(|| e.name.rsplit_once(" at ")).map(|x| x.1.to_string()) else { return };
                if p.starts_with("the ") || !p.ends_with(noun) {
                    return;
                }
                (p, format!("named for {}", e.name))
            }
        };
        let year = self.start_year + (self.tick / YEAR) as i32;
        let fort = self.fortress_name.clone();
        self.legend(format!("The {} of {} has been called {} ever since, {}.", noun.to_lowercase(), fort, name, deed));
        if let Some(mut o) = self.overseer.take() {
            Self::note_ov(&mut o, self.tick, format!("Called the place {}.", name));
            self.overseer = Some(o);
        }
        self.msg(&format!("The {} is called {} now, {}.", noun.to_lowercase(), name, deed), (230, 200, 150));
        self.count("stories.places", 1);
        self.stories.places.push(Place { name, at, r: 4, deed, year, kind: super::places::PlaceKind::Battlefield, b: None, weight: 5, by_player: false });
    }

    /// A newborn (`families_daily`): the parents may name it for one of the
    /// fallen they honour, a hero or one who died in a named fight within
    /// the last three years. The reason goes in the chronicle and legends.
    pub(super) fn named_for_hero(&mut self, mut child: Dwarf) -> Dwarf {
        let tick = self.tick;
        let (mother, father) = child.parents.unwrap_or((0, 0));
        let regard = |id: u32| -> i32 { self.dwarves.iter().filter(|p| p.id == mother || p.id == father).map(|p| p.regard(id) + if p.family_of(id) { 40 } else { 0 }).max().unwrap_or(0) };
        let fort = self.fortress_name.clone();
        let mut best: Option<(i32, String, String, String)> = None;
        for d in self.dwarves.iter().filter(|d| !d.alive && d.died.map_or(false, |t| tick < t + 3 * YEAR)) {
            let called = d.called();
            let fight = self.stories.episodes.iter().rev().find(|e| e.dead.contains(&called));
            let reason = match (&d.deed, fight) {
                (Some(deed), Some(e)) => format!("who {} and fell in {}", deed, e.name),
                (Some(deed), None) => format!("who {}", deed),
                (None, Some(e)) => format!("who fell in {}", e.name),
                (None, None) if d.soldier && self.annals.deaths.iter().any(|n| n.name == d.name && n.siege.is_some()) => format!("who fell defending {}", fort),
                _ => continue,
            };
            let score = regard(d.id) + if d.epithet.is_some() { 50 } else { 0 } - 60 * self.stories.honoured.iter().filter(|h| **h == called).count() as i32;
            if best.as_ref().map_or(true, |b| score > b.0) {
                best = Some((score, called, d.name.clone(), reason));
            }
        }
        let Some((_, hero, hero_name, reason)) = best else { return child };
        let roll = (hash(&child.name, child.id as u64) % 1000) as f64 / 1000.0;
        if roll >= self.knob("story.hero_name", 0.7) {
            return child;
        }
        let family = child.name.split(' ').nth(1).unwrap_or("").to_string();
        child.name = format!("{} {}", first_name(&hero_name), family).trim().to_string();
        let parent = self.dwarves.iter().find(|p| p.id == mother).map_or("The parents".to_string(), |p| p.called());
        let what = if child.female == Some(true) { "her daughter" } else { "her son" };
        self.stories.pending.push(format!("{} named {} {} for {}, {}.", parent, what, child.name, hero, reason));
        self.year_log.stories.push(format!("{} was named for {}, {}.", child.name, hero, reason));
        self.legend(format!("{}, born in {}, was named for {}, {}.", child.name, fort, hero, reason));
        self.stories.honoured.push(hero);
        self.count("stories.named_for_heroes", 1);
        child
    }

    /// Who and what the camera should watch of a wedding or feast under
    /// way: (dwarf, a wedding, "the wedding of Kolvar and Ingra").
    pub fn occasion_sight(&self) -> Option<(u32, bool, String)> {
        let o = self.stories.occasion.as_ref().filter(|o| self.tick < o.until)?;
        let id = o.who.iter().copied().find(|id| self.dwarves.iter().any(|d| d.id == *id && d.alive))?;
        Some((id, o.wedding, o.what.clone()))
    }

    /// Two friends have married (`families_daily`, dwarves `i` and `j`): a
    /// short ceremony before the fort, gifts from their friends, a feast,
    /// and now and then a rival who wasn't asked.
    pub(super) fn wedding(&mut self, i: usize, j: usize) {
        let tick = self.tick;
        let (a, b) = (self.dwarves[i].id, self.dwarves[j].id);
        let (an, bn) = (self.dwarves[i].called(), self.dwarves[j].called());
        self.record(super::memory::MemoryKind::Married, &[a, b], Some(self.dwarves[i].pos), format!("{} and {} were wed.", an, bn), 4);
        let h = hash(&format!("{}&{}", an, bn), tick);
        let zone = |k: ZoneKind| self.zones.values().any(|z| z.kind == k);
        let venue = if zone(ZoneKind::Temple) {
            "in the temple"
        } else if zone(ZoneKind::Meeting) || zone(ZoneKind::Tavern) {
            "in the great hall"
        } else {
            "beneath the open sky"
        };
        self.msg(&format!("{} and {} are wed {}, before the whole fortress!", an, bn, venue), (255, 200, 220));
        // Gifts, from their closest friends.
        let mut givers: Vec<(i32, usize)> = (0..self.dwarves.len())
            .filter(|&k| k != i && k != j)
            .filter_map(|k| {
                let d = &self.dwarves[k];
                let r = d.regard(a).max(d.regard(b));
                (d.alive && !d.insane && !d.is_child() && r >= 30).then_some((r, k))
            })
            .collect();
        givers.sort_by_key(|&(r, k)| (-r, k));
        let mut stock: Vec<u32> = self.items.values().filter(|it| it.free() && matches!(it.kind, ItemKind::Craft | ItemKind::Gem | ItemKind::Drink)).map(|it| it.id).collect();
        let mut gave = Vec::new();
        for (n, &(_, k)) in givers.iter().take(3).enumerate() {
            let gift = if stock.is_empty() {
                pick(h, 10 + n as u64, &["a song of their own making", "a blessing on their hearth", "a promise of ale", "a lucky stone", "a carved spoon"]).to_string()
            } else {
                let id = stock.remove((hash("gift", h ^ n as u64) % stock.len() as u64) as usize);
                let name = self.items[&id].name();
                let an = if name.starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" };
                format!("{} {}", an, name)
            };
            let giver = self.dwarves[k].called();
            self.dwarves[k].think("gave a wedding gift", -5, tick);
            for x in [i, j] {
                self.dwarves[x].think(&format!("was given a wedding gift by {}", giver), -6, tick);
            }
            self.msg(&format!("{} gives the couple {}.", giver, gift), (255, 200, 220));
            gave.push(first_name(&giver).to_string());
        }
        // A rival in love, who wasn't asked.
        let rival = (0..self.dwarves.len())
            .filter(|&k| k != i && k != j)
            .filter_map(|k| {
                let d = &self.dwarves[k];
                let unwed = d.spouse.map_or(true, |s| !self.dwarves.iter().any(|o| o.id == s && o.alive));
                let r = d.regard(a).max(d.regard(b));
                (d.alive && !d.insane && !d.is_child() && unwed && r as f64 >= self.knob("story.rival_regard", 60.0)).then_some((r, k))
            })
            .max_by_key(|&(r, k)| (r, usize::MAX - k))
            .map(|(_, k)| k);
        let mut uninvited = None;
        if let Some(r) = rival.filter(|_| (hash("rival", h) % 100) < 60) {
            let (loved, other) = if self.dwarves[r].regard(a) >= self.dwarves[r].regard(b) { (i, j) } else { (j, i) };
            let (rn, ln, on) = (self.dwarves[r].called(), self.dwarves[loved].called(), self.dwarves[other].called());
            let (rid, oid) = (self.dwarves[r].id, self.dwarves[other].id);
            self.dwarves[r].think(&format!("watched {} wed another", first_name(&ln)), 30, tick);
            self.dwarves[r].adjust_regard(oid, -30);
            self.dwarves[other].adjust_regard(rid, -15);
            let busy = self.duels.iter().any(|d| [d.a, d.b].contains(&rid) || [d.a, d.b].contains(&oid));
            let text = if self.dwarves[r].personality.temper > 30 && !busy {
                let at = (tick / DAY + 1) * DAY + DAY / 10;
                self.duels.push(super::drama::Duel { a: rid, b: oid, at });
                format!("{}, who loved {}, bursts in uninvited and challenges {} to a duel at dawn!", rn, first_name(&ln), on)
            } else {
                let how = pick(h, 6, &["drinks alone at the back", "weeps through the vows", "glowers from the doorway", "leaves before the toasts"]);
                format!("{}, who loved {}, turns up uninvited and {}.", rn, first_name(&ln), how)
            };
            self.msg(&text, (255, 170, 90));
            self.gossip(text);
            uninvited = Some(first_name(&rn).to_string());
            self.count("stories.rivals", 1);
        }
        // The feast.
        let len = self.knob("story.wedding_len", (DAY / 4) as f64) as u64;
        if self.festival.is_none() {
            self.start_festival(super::festivals::FestivalKind::Wedding);
            if let Some(f) = self.festival.as_mut().filter(|f| f.kind == super::festivals::FestivalKind::Wedding) {
                f.until = tick + len;
            }
        }
        self.stories.occasion = Some(Occasion { what: format!("the wedding of {} and {}", first_name(&an), first_name(&bn)), who: vec![a, b], wedding: true, until: tick + len });
        let mut line = format!("{} and {} were wed {}", an, bn, venue);
        if !gave.is_empty() {
            line.push_str(&format!("; {} brought gifts", list(&gave)));
        }
        if let Some(r) = uninvited {
            line.push_str(&format!(", and {} came uninvited", r));
        }
        line.push('.');
        self.year_log.stories.push(line);
        self.count("stories.weddings", 1);
    }

    /// A siege has been broken (`on_siege_end`, after `name_siege`): the
    /// overseer calls a victory feast in its own voice, the whole fort
    /// cheers, and the dead of the siege are toasted by name.
    pub(super) fn victory_feast(&mut self, i: usize) {
        let Some(s) = self.annals.sieges.get(i).cloned() else { return };
        let fought = s.foes_slain + s.foes_caught > 0 || s.outcome == "routed";
        if s.outcome == "the fortress fell" || !fought || self.alive_count() < 2 {
            return;
        }
        let tick = self.tick;
        let h = hash(&self.fortress_name, s.tick ^ ((i as u64) << 24) ^ 0xfea57);
        let fort = self.fortress_name.clone();
        let battle = self.stories.episodes.iter().rev().find(|e| e.siege == Some(i)).map(|e| e.name.clone());
        let what = match &battle {
            Some(b) => format!("the victory feast after {}", b),
            None => "the victory feast".to_string(),
        };
        // Called by the overseer, in its voice; or by the fort itself.
        let caller = self.overseer.as_ref().map(|o| (o.name.clone(), o.traits.first().copied()));
        let words = match caller.as_ref().and_then(|c| c.1) {
            Some(t) => feast_words(t, h, &fort),
            None => pick(h, 1, &["Drink! The day is ours!", "Tap the casks: we held!", "Ale for everyone who stood!"]).to_string(),
        };
        let call = match &caller {
            Some((name, _)) if !name.is_empty() => format!("The siege is broken! {} calls a victory feast: \"{}\"", name, words),
            _ => format!("The siege is broken! A victory feast: \"{}\"", words),
        };
        self.msg(&call, (255, 215, 60));
        // Fort-wide cheer.
        for d in self.dwarves.iter_mut().filter(|d| d.alive && !d.insane) {
            d.think("cheered the breaking of the siege", -10, tick);
        }
        // Toasts to the dead, by name, from whoever held them dearest.
        let dead: Vec<String> = self.annals.deaths.iter().filter(|n| n.siege == Some(i)).map(|n| n.name.clone()).collect();
        let most = self.knob("story.toasts", 4.0) as usize;
        let mut toasted = Vec::new();
        for (n, name) in dead.iter().enumerate().take(most) {
            let fallen = self.dwarves.iter().find(|d| &d.name == name).map(|d| (d.id, d.called()));
            let called = fallen.as_ref().map_or(name.clone(), |f| f.1.clone());
            let toaster = fallen.and_then(|(id, _)| {
                self.dwarves
                    .iter()
                    .enumerate()
                    .filter(|(_, d)| d.alive && !d.insane && !d.is_child() && d.regard(id) > 0)
                    .max_by_key(|(k, d)| (d.regard(id), usize::MAX - k))
                    .map(|(k, _)| k)
            });
            let line = match toaster {
                Some(k) => {
                    let tn = self.dwarves[k].called();
                    self.dwarves[k].think(&format!("drank to {}", first_name(&called)), -4, tick);
                    let how = pick(h, 20 + n as u64, &["raises a mug to", "drinks to", "stands and toasts", "pours out a measure for"]);
                    format!("{} {} {}.", tn, how, called)
                }
                None => format!("The hall drinks to {}.", called),
            };
            self.msg(&line, (255, 200, 140));
            toasted.push(first_name(&called).to_string());
        }
        if dead.len() > most {
            self.msg(&format!("...and to {} more who fell.", number(dead.len() - most)), (255, 200, 140));
        }
        // The feast itself.
        let len = self.knob("story.feast_len", (DAY / 4) as f64) as u64;
        if self.festival.is_none() {
            self.start_festival(super::festivals::FestivalKind::Victory);
            if let Some(f) = self.festival.as_mut().filter(|f| f.kind == super::festivals::FestivalKind::Victory) {
                f.until = tick + len;
            }
        }
        // The camera watches the fighters, those who slew most first.
        let mut fighters: Vec<(u32, u32)> = self.dwarves.iter().filter(|d| d.alive && (d.soldier || d.kills > 0)).map(|d| (d.kills, d.id)).collect();
        fighters.sort_by_key(|&(k, id)| (u32::MAX - k, id));
        let mut who: Vec<u32> = fighters.into_iter().map(|w| w.1).collect();
        let rest: Vec<u32> = self.dwarves.iter().filter(|d| d.alive && !who.contains(&d.id)).map(|d| d.id).take(3).collect();
        who.extend(rest);
        self.stories.occasion = Some(Occasion { what: what.clone(), who, wedding: false, until: tick + len });
        // The chronicle and the legends.
        let host = caller.as_ref().map(|c| c.0.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| fort.clone());
        let mut line = format!("{} called {}", host, what);
        if !toasted.is_empty() {
            line.push_str(&format!(", and the fort drank to {}", list(&toasted)));
        }
        line.push('.');
        self.legend(line.clone());
        self.year_log.stories.push(line);
        if let Some(mut o) = self.overseer.take() {
            Self::note_ov(&mut o, tick, format!("Called {}.", what));
            self.overseer = Some(o);
        }
        self.count("stories.feasts", 1);
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
        if self.stories.occasion.as_ref().map_or(false, |o| tick >= o.until) {
            self.stories.occasion = None;
        }
        for text in std::mem::take(&mut self.stories.pending) {
            self.msg(&text, (150, 255, 150));
        }
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
        let plural = plural_of(&foe);
        let pool: &[&str] = if self.season_word() == "Winter" && pick(h, 4, &["a", "b"]) == "a" { &["the {K} Winter"] } else { &BEASTLY };
        let name = self.unique(self.fill(pick(h, 3, pool), h, &place, &foe, &plural, ""));
        let summary = format!("The {} fell upon {} and killed {}.", plural, self.fortress_name, list(&dead));
        let e = Episode { name, kind: EpisodeKind::Massacre, tick, at, dead, veterans: self.veterans_near(at, 12), siege: None };
        self.tell_episode(e, summary, (255, 110, 90));
    }
}

#[cfg(test)]
pub(crate) mod tests {
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
    fn the_place_of_a_stand_takes_the_name_of_who_stood() {
        let mut g = quiet(5);
        fought_siege(&mut g, 0, 4);
        let e = g.stories.episodes.last().unwrap().clone();
        let pl = g.stories.places.last().expect("the place should be named").clone();
        let who = first_name(&g.dwarves[0].name).to_string();
        // Dwarf 0 struck down all four where they fell: their stand.
        assert_eq!(pl.name, format!("{} Stand", super::super::gore::possessive(&who)));
        assert!(pl.deed.contains("slew four goblins") && pl.deed.contains(&e.name), "{}", pl.deed);
        let look = g.story_place_at(pl.at).expect("look shows it");
        assert!(look.starts_with(&pl.name), "{}", look);
        assert!(g.story_place_at(pl.at.offset(20, 20, 0)).is_none());
        assert!(g.overworld.events.iter().any(|ev| ev.text.contains(&pl.name) && ev.text.contains("ever since")), "not in the legends");
        // The next fight there is fought at the named place.
        assert_eq!(g.place_words(pl.at.offset(1, 1, 0), 99), pl.name);
    }

    #[test]
    fn a_massacre_leaves_a_weeping_place() {
        let mut g = quiet(7);
        fought_siege(&mut g, 4, 0);
        let pl = g.stories.places.last().expect("named").clone();
        assert!(pl.name.starts_with("the "), "{}", pl.name);
        assert!(pl.deed.contains(" died in "), "{}", pl.deed);
    }

    #[test]
    fn a_child_is_named_for_a_fallen_hero() {
        let mut g = quiet(81);
        g.knobs.set.insert("story.hero_name".into(), 1.0);
        // A hero falls.
        let mut hero = std::mem::take(&mut g.dwarves[2]);
        hero.epithet = Some("Grim".into());
        hero.epithet_rank = 2;
        hero.deed = Some("slew ten foes".into());
        g.kill(&mut hero, "was struck down");
        let (hero_first, called) = (first_name(&hero.name).to_string(), hero.called());
        g.dwarves[2] = hero;
        // A mother is due.
        let (m, f) = (g.dwarves[0].id, g.dwarves[1].id);
        g.dwarves[0].female = Some(true);
        g.dwarves[0].spouse = Some(f);
        g.dwarves[1].spouse = Some(m);
        g.dwarves[0].due = Some(g.tick);
        g.families_daily();
        g.stories_daily();
        let kid = g.dwarves.iter().find(|d| d.parents == Some((m, f))).expect("born");
        assert_eq!(first_name(&kid.name), hero_first);
        let line = format!("{} was named for {}, who slew ten foes.", kid.name, called);
        assert!(g.year_log.stories.contains(&line), "{:?}", g.year_log.stories);
        assert!(g.log.iter().any(|l| l.text.contains("for ") && l.text.contains(&called) && l.text.contains(&kid.name)), "no announcement");
        assert!(g.overworld.events.iter().any(|e| e.text.contains("was named for")));
        assert_eq!(g.stories.honoured, vec![called]);
    }

    #[test]
    fn with_nobody_fallen_a_child_keeps_its_name() {
        let mut g = quiet(81);
        g.knobs.set.insert("story.hero_name".into(), 1.0);
        let (m, f) = (g.dwarves[0].id, g.dwarves[1].id);
        g.dwarves[0].female = Some(true);
        g.dwarves[0].spouse = Some(f);
        g.dwarves[0].due = Some(g.tick);
        g.families_daily();
        assert!(g.dwarves.iter().any(|d| d.parents == Some((m, f))));
        assert!(g.year_log.stories.is_empty() && g.stories.honoured.is_empty());
    }

    /// Two friends of age, of opposite sexes, ready to wed; dwarf 2 their
    /// friend, dwarf 3 in love with the bride.
    fn sweethearts(g: &mut Game) -> (u32, u32) {
        for (i, f) in [(0, true), (1, false), (3, false)] {
            g.dwarves[i].female = Some(f);
            g.dwarves[i].age = Some(40);
            g.dwarves[i].spouse = None;
        }
        let (a, b) = (g.dwarves[0].id, g.dwarves[1].id);
        g.dwarves[0].adjust_regard(b, 90);
        g.dwarves[1].adjust_regard(a, 90);
        g.dwarves[2].adjust_regard(a, 50);
        g.dwarves[3].adjust_regard(a, 80);
        (a, b)
    }

    #[test]
    fn a_wedding_has_gifts_a_feast_and_maybe_a_rival() {
        let mut g = quiet(81);
        let (a, b) = sweethearts(&mut g);
        g.spawn_item(ItemKind::Craft, crate::material::Mat::Granite, g.embark);
        g.wedding(0, 1);
        assert!(g.log.iter().any(|l| l.text.contains("are wed")), "no ceremony");
        assert!(g.log.iter().any(|l| l.text.contains("gives the couple")), "no gifts");
        assert!(g.dwarves[0].thoughts.iter().any(|t| t.text.starts_with("was given a wedding gift")));
        let f = g.festival.as_ref().expect("a feast");
        assert_eq!(f.kind, super::super::festivals::FestivalKind::Wedding);
        assert!(g.year_log.stories.iter().any(|l| l.contains("were wed")), "{:?}", g.year_log.stories);
        let (who, wedding, what) = g.occasion_sight().expect("the camera has a subject");
        assert!(wedding && (who == a || who == b) && what.starts_with("the wedding of"));
        // The rival: turned up or not, as the hash says, the same each time.
        let came = g.log.iter().any(|l| l.text.contains("uninvited"));
        let mut g2 = quiet(81);
        sweethearts(&mut g2);
        g2.spawn_item(ItemKind::Craft, crate::material::Mat::Granite, g2.embark);
        g2.wedding(0, 1);
        assert_eq!(came, g2.log.iter().any(|l| l.text.contains("uninvited")));
        if came {
            assert!(g.dwarves[3].thoughts.iter().any(|t| t.text.starts_with("watched")));
        }
        // The feast ends, and the camera moves on.
        for _ in 0..DAY / 4 + 60 {
            g.step();
        }
        assert!(g.festival.is_none(), "the wedding feast should be over");
        assert!(g.occasion_sight().is_none());
    }

    #[test]
    fn a_rival_always_comes_somewhere() {
        // Over a few couples, some rival turns up uninvited.
        let mut seen = false;
        for seed in [81, 82, 83, 84, 85, 86] {
            let mut g = quiet(seed);
            sweethearts(&mut g);
            g.wedding(0, 1);
            seen |= g.log.iter().any(|l| l.text.contains("uninvited"));
        }
        assert!(seen, "no rival in six weddings");
    }

    #[test]
    fn friends_who_marry_hold_a_wedding() {
        let mut g = quiet(81);
        sweethearts(&mut g);
        for _ in 0..60 {
            g.families_daily();
            if g.stories.occasion.is_some() {
                break;
            }
        }
        assert!(g.stories.occasion.as_ref().map_or(false, |o| o.wedding), "no wedding");
    }

    #[test]
    fn places_have_nouns() {
        let mut g = quiet(5);
        let p = g.open_patch(5, 5, 6).offset(2, 2, 0);
        assert_eq!(g.place_noun(p), "Field");
        assert!(g.place_words(p, 3).ends_with(" Field"));
    }

    #[test]
    fn a_broken_siege_ends_in_a_victory_feast_with_toasts() {
        let mut g = quiet(31);
        g.set_overseer(Some(crate::game::Style::Steady));
        let name = g.overseer.as_ref().unwrap().name.clone();
        let fallen = g.dwarves[1].called();
        fought_siege(&mut g, 1, 4);
        let call = g.log.iter().find(|l| l.text.contains("calls a victory feast")).expect("no feast called");
        assert!(call.text.contains(&name), "{}", call.text);
        // Toasted by name.
        assert!(g.log.iter().any(|l| l.text.contains(&format!("{}.", fallen)) && !l.text.contains("It will be")), "no toast to {}", fallen);
        // The fort cheers.
        assert!(g.dwarves.iter().filter(|d| d.alive).all(|d| d.thoughts.iter().any(|t| t.text == "cheered the breaking of the siege")));
        let f = g.festival.as_ref().expect("a feast");
        assert_eq!(f.kind, super::super::festivals::FestivalKind::Victory);
        // In the chronicle, the legends and the overseer's notes, after the battle's name.
        let battle = g.stories.episodes.last().unwrap().name.clone();
        let line = g.year_log.stories.iter().find(|l| l.contains("victory feast")).expect("chronicle").clone();
        assert!(line.contains(&battle) && line.contains(first_name(&fallen)), "{}", line);
        assert!(g.overworld.events.iter().any(|e| e.text.contains("victory feast")), "not in the legends");
        assert!(g.overseer.as_ref().unwrap().notes.iter().any(|n| n.1.contains("victory feast")));
        // The camera watches a fighter.
        let (who, wedding, what) = g.occasion_sight().expect("a subject");
        assert!(!wedding && what.starts_with("the victory feast") && who == g.dwarves[0].id, "{} {}", who, what);
        assert_eq!(g.annals.counters.get("stories.feasts"), Some(&1));
    }

    #[test]
    fn no_feast_when_nobody_was_beaten() {
        let mut g = quiet(5);
        let n = g.neighbors.first().cloned().unwrap();
        g.lay_siege_of(&n, 3, None);
        let ids: Vec<u32> = g.creatures.values().filter(|c| c.siege).map(|c| c.id).collect();
        for id in ids {
            g.creatures.remove(&id);
        }
        g.annals_tick();
        assert!(!g.log.iter().any(|l| l.text.contains("victory feast")));
        assert!(g.stories.occasion.is_none());
    }

    #[test]
    fn every_temper_has_feast_words() {
        for t in crate::game::personality::Trait::ALL {
            for h in 0..6 {
                let w = feast_words(t, h, "Boatmurdered");
                assert!(!w.is_empty() && !w.contains('{'), "{}", w);
            }
        }
    }

    #[test]
    fn the_camera_watches_weddings_and_feasts() {
        use crate::director::{candidates, Kind, Who};
        let mut g = quiet(81);
        let (a, b) = sweethearts(&mut g);
        g.wedding(0, 1);
        let v = candidates(&g);
        let s = v.iter().find(|s| s.kind == Kind::Wedding).expect("the wedding is a subject");
        assert!(matches!(s.who, Who::Dwarf(id) if id == a || id == b) && s.what.starts_with("the wedding of"));
        // The plain festival isn't shown twice.
        assert!(!v.iter().any(|s| s.kind == Kind::Festival));
        let mut g = quiet(31);
        fought_siege(&mut g, 1, 4);
        let v = candidates(&g);
        let s = v.iter().find(|s| s.kind == Kind::Feast).expect("the feast is a subject");
        assert!(s.what.starts_with("the victory feast after "), "{}", s.what);
        // Once it's over, the camera moves on.
        g.tick += DAY;
        assert!(!candidates(&g).iter().any(|s| matches!(s.kind, Kind::Feast | Kind::Wedding)));
    }
}
