//! Sagas: the fort notices its own story arcs. Events the game already
//! tells (a sally won, a host sworn to avenge it, a duel at dawn, an omen,
//! a hero's name, the last dwarf standing) are linked as they happen into
//! named arcs with a cause, an escalation, a turn and an ending, each a
//! chapter quoting what happened and when. When an arc turns or ends it is
//! told on an illuminated card (`ui/saga.rs`); finished ones go into the
//! world's legends, where later forts in the same world can read them.
//!
//! Titles come from a hash of the fort and the moment, never the game's
//! RNG, and nothing here is read back by the simulation, so noticing a
//! saga changes nothing else that happens.

use super::stories::{first_name, hash, pick};
use super::*;
use serde::{Deserialize, Serialize};

/// What sort of arc.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SagaKind {
    /// Sallies won, a host sworn to avenge them, then ruin or a narrow escape.
    Pride,
    /// A grudge, a duel, then a death or a reconciliation.
    Grudge,
    /// A dwarf's names, then their last stand or their death.
    Hero,
    /// An omen seen, come true, and what it cost.
    Omen,
    /// A god's curse, then atonement or death.
    Curse,
    /// A beast's lair hunted down.
    Hunt,
    /// A great siege, and how it broke (or didn't).
    Siege,
    /// The last dwarf standing.
    LastStand,
}

impl SagaKind {
    pub fn name(self) -> &'static str {
        match self {
            SagaKind::Pride => "pride and fall",
            SagaKind::Grudge => "a grudge",
            SagaKind::Hero => "a hero's rise",
            SagaKind::Omen => "an omen",
            SagaKind::Curse => "a curse",
            SagaKind::Hunt => "a hunt",
            SagaKind::Siege => "a siege",
            SagaKind::LastStand => "the last dwarf",
        }
    }
}

/// Where a chapter stands in its arc.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Beat {
    Cause,
    Escalation,
    Turn,
    Ending,
}

impl Beat {
    /// The progress glyph for a chapter of this beat.
    pub fn glyph(self, doom: bool) -> char {
        match self {
            Beat::Cause => '○',
            Beat::Escalation => '◘',
            Beat::Turn => '●',
            Beat::Ending if doom => '†',
            Beat::Ending => '☼',
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Chapter {
    pub tick: u64,
    pub beat: Beat,
    pub text: String,
}

/// One arc.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Saga {
    pub id: u32,
    pub kind: SagaKind,
    /// "The Pride of Anvilbeard".
    pub title: String,
    /// What it follows: a civ, a dwarf, a pair of dwarves, a lair, a siege.
    pub key: u64,
    #[serde(default)]
    pub chapters: Vec<Chapter>,
    /// When it ended.
    #[serde(default)]
    pub ended: Option<u64>,
    /// Ended in grief (crimson) rather than glory (gold).
    #[serde(default)]
    pub doom: bool,
    /// The closing line, once it has ended.
    #[serde(default)]
    pub closing: String,
    /// The furthest beat told on a card.
    #[serde(default)]
    pub shown: Option<Beat>,
    /// When it ends of itself if nothing more happens (0: never).
    #[serde(default)]
    pub due: u64,
    /// How much it has been through: names won, foes, lives.
    #[serde(default)]
    pub weight: u32,
}

impl Saga {
    pub fn open(&self) -> bool {
        self.ended.is_none()
    }

    /// Worth telling: enough happened, and it happened to matter.
    pub fn told(&self) -> bool {
        let n = self.chapters.len();
        let has = |b: Beat| self.chapters.iter().any(|c| c.beat == b);
        match self.kind {
            SagaKind::Pride => n >= 2 && (has(Beat::Escalation) || has(Beat::Turn) || has(Beat::Ending)),
            // Two duels at least, and an end: a death or a peace.
            SagaKind::Grudge => {
                let duels = self.chapters.iter().filter(|c| c.beat == Beat::Turn).count();
                self.ended.is_some() && (duels >= 3 || (self.doom && duels >= 2))
            }
            SagaKind::Hero => n >= 2 && self.weight >= 3,
            SagaKind::Omen => n >= 3,
            SagaKind::Curse => n >= 2 && self.ended.is_some(),
            SagaKind::Hunt => n >= 2 && self.weight >= 3,
            SagaKind::Siege => n >= 3 && self.weight >= 4,
            SagaKind::LastStand => n >= 2,
        }
    }

    /// The furthest beat it has reached.
    pub fn beat(&self) -> Beat {
        self.chapters.iter().map(|c| c.beat).max().unwrap_or(Beat::Cause)
    }
}

/// A finished saga as the world remembers it (`Overworld.sagas`): dates
/// written out, since the fort that kept them may be long gone.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldSaga {
    pub title: String,
    pub kind: SagaKind,
    pub fort: String,
    pub year: i32,
    #[serde(default)]
    pub chapters: Vec<(String, Beat, String)>,
    #[serde(default)]
    pub closing: String,
    #[serde(default)]
    pub doom: bool,
    /// It was the saga that ended its fort.
    #[serde(default)]
    pub fell: bool,
}

/// The fort's sagas (`Game.sagas`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Sagas {
    #[serde(default)]
    pub list: Vec<Saga>,
    #[serde(default)]
    pub next: u32,
    /// The saga that ended the fort, once it has fallen.
    #[serde(default)]
    pub fell_by: Option<u32>,
    /// The fort's fall has been told.
    #[serde(default)]
    pub fall_told: bool,
    /// A death since the last look: has everyone gone?
    #[serde(skip)]
    pub check_fall: bool,
}

// Keys for what a saga follows.
const KEY_PRIDE: u64 = 1 << 60;
const KEY_LAST: u64 = 2 << 60;
fn pair(a: u32, b: u32) -> u64 {
    ((a.min(b) as u64) << 32) | a.max(b) as u64
}

const BANNERS: [&str; 6] = ["Hand", "Banner", "Spear", "Gate", "Oath", "Shield"];
const COLOURS: [&str; 8] = ["Red", "Iron", "Black", "Bitter", "Broken", "Ashen", "Golden", "Grey"];

/// "a" or "an" before `w`.
fn a(w: &str) -> &'static str {
    if w.starts_with(['a', 'e', 'i', 'o', 'u', 'A', 'E', 'I', 'O', 'U']) {
        "an"
    } else {
        "a"
    }
}

/// A title for a new saga of `kind`: from the fort, the moment and who
/// it's about, never the game's RNG.
pub fn title_for(kind: SagaKind, fort: &str, tick: u64, who: &str) -> String {
    let h = hash(&format!("{}|{}|{:?}", fort, who, kind), tick);
    let col = pick(h, 7, &COLOURS);
    let ban = pick(h, 8, &BANNERS);
    let pool: &[&str] = match kind {
        SagaKind::Pride => &["The Pride of {F}", "The Oath of the {C} {B}", "The Proud Gate of {F}", "The Dear-Bought Glory of {F}", "The Sallies of {F}"],
        SagaKind::Grudge => &["The Feud of {W}", "The Bad Blood of {W}", "The Quarrel at Dawn", "The {C} Grudge"],
        SagaKind::Hero => &["The Rise of {W}", "The Song of {W}", "The Deeds of {W}", "The Lay of {W}"],
        SagaKind::Omen => &["The Year of {W}", "The {C} Omen", "What {W} Foretold"],
        SagaKind::Curse => &["The Curse of {W}", "The Wrath of {W}", "The {C} Curse"],
        SagaKind::Hunt => &["The Hunt for {W}", "The Clearing of {W}", "The {C} Hunt"],
        SagaKind::Siege => &["The Siege of {F}", "The Walls of {F}", "The {C} Siege", "The Long Watch at {F}"],
        SagaKind::LastStand => &["The Last of {F}", "{W} Alone", "The Last Lamp of {F}"],
    };
    pick(h, 3, pool).replace("{F}", fort).replace("{W}", who).replace("{C}", col).replace("{B}", ban)
}

impl Game {
    // ---- the book-keeping ----

    fn saga_find(&self, kind: SagaKind, key: u64) -> Option<usize> {
        self.sagas.list.iter().rposition(|s| s.kind == kind && s.key == key && s.open())
    }

    /// A new arc of `kind` about `who`, with its first chapter.
    fn saga_open(&mut self, kind: SagaKind, key: u64, who: &str, beat: Beat, text: String) -> usize {
        let id = self.sagas.next;
        self.sagas.next += 1;
        let title = title_for(kind, &self.fortress_name, self.tick, who);
        self.sagas.list.push(Saga { id, kind, title, key, chapters: vec![Chapter { tick: self.tick, beat, text }], ended: None, doom: false, closing: String::new(), shown: None, due: 0, weight: 0 });
        self.sagas.list.len() - 1
    }

    /// Another chapter of saga `i`: on a turn, a card if it's worth telling.
    fn saga_add(&mut self, i: usize, beat: Beat, text: String) {
        let tick = self.tick;
        let s = &mut self.sagas.list[i];
        if s.chapters.len() >= 9 {
            // A long saga keeps its first chapters and its latest.
            s.chapters.remove(2);
        }
        s.chapters.push(Chapter { tick, beat, text });
        if beat == Beat::Turn {
            self.saga_card(i);
        }
    }

    /// Saga `i` ends: its last chapter, its closing line, and its card.
    /// One not worth telling is forgotten.
    fn saga_end(&mut self, i: usize, doom: bool, text: Option<String>, closing: String) {
        let tick = self.tick;
        {
            let s = &mut self.sagas.list[i];
            if let Some(t) = text {
                s.chapters.push(Chapter { tick, beat: Beat::Ending, text: t });
            }
            s.ended = Some(tick);
            s.doom = doom;
            s.closing = closing;
        }
        if !self.sagas.list[i].told() {
            self.sagas.list.remove(i);
            return;
        }
        self.count("sagas.told", 1);
        self.saga_card(i);
        let w = self.world_saga(&self.sagas.list[i].clone(), false);
        self.overworld.sagas.push(w);
    }

    /// Forget an open arc that came to nothing.
    fn saga_drop(&mut self, i: usize) {
        self.sagas.list.remove(i);
    }

    /// The saga as the world will keep it.
    pub fn world_saga(&self, s: &Saga, fell: bool) -> WorldSaga {
        WorldSaga {
            title: s.title.clone(),
            kind: s.kind,
            fort: self.fortress_name.clone(),
            year: self.start_year + (s.chapters.first().map_or(self.tick, |c| c.tick) / YEAR) as i32,
            chapters: s.chapters.iter().map(|c| (self.short_date(c.tick).trim().to_string(), c.beat, c.text.clone())).collect(),
            closing: s.closing.clone(),
            doom: s.doom,
            fell,
        }
    }

    /// Tell saga `i` on its card, once for each beat it reaches.
    fn saga_card(&mut self, i: usize) {
        let s = &self.sagas.list[i];
        let beat = s.beat();
        if !s.told() || s.shown.map_or(false, |b| b >= beat) || self.sagas.fall_told {
            return;
        }
        let (id, title, doom) = (s.id, s.title.clone(), s.doom || (s.open() && s.kind == SagaKind::LastStand));
        self.sagas.list[i].shown = Some(beat);
        let color = if doom { (220, 50, 60) } else { (240, 190, 70) };
        self.spotlights.retain(|x| x.saga != Some(id));
        self.spotlights.push(spotlight::Spotlight { title: "A saga is told".into(), text: title, at: None, color, saga: Some(id), actions: Vec::new() });
        self.pause_request = true;
    }

    /// Foreshadowing: a line in the announcements, in the saga's gold.
    fn foreshadow(&mut self, text: &str) {
        self.msg(text, (240, 200, 110));
    }

    pub fn saga(&self, id: u32) -> Option<&Saga> {
        self.sagas.list.iter().find(|s| s.id == id)
    }

    /// The sagas worth telling, newest first.
    pub fn told_sagas(&self) -> Vec<&Saga> {
        self.sagas.list.iter().rev().filter(|s| s.told()).collect()
    }

    /// The saga that ended the fort, if it has fallen and one did.
    pub fn great_saga(&self) -> Option<&Saga> {
        self.sagas.fell_by.and_then(|id| self.saga(id))
    }

    // ---- the hooks: one line each where the game tells the event ----

    /// A sally ended (`sally.rs`): won, or broken and called back.
    pub(super) fn saga_sally(&mut self, siege: usize, soldiers: u32, lost: u32, won: bool) {
        let foe = self.annals.sieges.get(siege).map_or("the enemy".to_string(), |s| s.foe.clone());
        let fort = self.fortress_name.clone();
        let text = if won {
            match lost {
                0 => format!("The sally from {} broke the {} at the gate: {} rode out, and all came back.", fort, foe, soldiers),
                _ => format!("The sally from {} broke the {} at the gate: {} rode out, {} did not come back.", fort, foe, soldiers, lost),
            }
        } else {
            format!("The sally against the {} was broken before the gate; the bridge was raised behind the few who came back.", foe)
        };
        match self.saga_find(SagaKind::Pride, KEY_PRIDE) {
            Some(i) => {
                let beat = if won { Beat::Escalation } else { Beat::Turn };
                self.sagas.list[i].weight += 1;
                self.saga_add(i, beat, text);
            }
            None => {
                let beat = if won { Beat::Cause } else { Beat::Turn };
                let i = self.saga_open(SagaKind::Pride, KEY_PRIDE, &fort, beat, text);
                self.sagas.list[i].weight += 1;
            }
        }
    }

    /// A host sworn to avenge the sallies has come (`arm_sworn_host`).
    pub(super) fn saga_oath(&mut self, siege: usize) {
        let Some(s) = self.annals.sieges.get(siege).cloned() else { return };
        let fort = self.fortress_name.clone();
        // It is the pride's saga now, not a siege of its own.
        if let Some(j) = self.saga_find(SagaKind::Siege, siege as u64) {
            self.saga_drop(j);
        }
        let i = match self.saga_find(SagaKind::Pride, KEY_PRIDE) {
            Some(i) => i,
            None => self.saga_open(SagaKind::Pride, KEY_PRIDE, &fort, Beat::Cause, format!("The squads of {} broke an army in the open, and its people swore to avenge it.", fort)),
        };
        self.sagas.list[i].key = KEY_PRIDE;
        self.sagas.list[i].due = 0;
        self.sagas.list[i].weight += 2;
        let text = format!("{} {} came back as a sworn host, armed in their best, to avenge their dead.", s.size, s.foe);
        self.saga_add(i, Beat::Escalation, text);
        self.foreshadow(&format!("The {} have sworn an oath against {}. This will be remembered.", s.foe, fort));
    }

    /// A siege has come (`note_siege`).
    pub(super) fn saga_siege_start(&mut self, siege: usize) {
        let Some(s) = self.annals.sieges.get(siege).cloned() else { return };
        let fort = self.fortress_name.clone();
        let text = format!("{} {} under {} came against {}.", s.size, s.foe, s.leader, fort);
        let i = self.saga_open(SagaKind::Siege, siege as u64, &fort, Beat::Cause, text);
        if (s.size as f64) >= self.knob("saga.great_siege", 12.0) {
            self.sagas.list[i].weight += 2;
        }
    }

    /// The leader of an attack has fallen (`combat.rs`).
    pub(super) fn saga_leader_fell(&mut self, label: &str) {
        let Some(siege) = self.annals.sieges.iter().rposition(|s| s.ended.is_none()) else { return };
        let i = self.saga_find(SagaKind::Siege, siege as u64).or_else(|| (self.sallies.host == Some(siege)).then(|| self.saga_find(SagaKind::Pride, KEY_PRIDE)).flatten());
        let Some(i) = i else { return };
        self.sagas.list[i].weight += 1;
        self.saga_add(i, Beat::Turn, format!("{}, who led the attack, was cut down.", label));
    }

    /// A siege is over (`annals_tick`, after it's been named).
    pub(super) fn saga_siege_end(&mut self, siege: usize) {
        let Some(s) = self.annals.sieges.get(siege).cloned() else { return };
        let fort = self.fortress_name.clone();
        let named = self.stories.episodes.iter().rev().find(|e| e.siege == Some(siege)).map(|e| e.name.clone());
        let fell = s.outcome == "the fortress fell";
        let called = named.as_ref().map_or(String::new(), |n| format!(" It will be remembered as {}.", n));
        let how = match s.outcome.as_str() {
            "routed" => "were routed",
            "withdrew" => "were beaten off",
            _ => "overran the fortress",
        };
        let lost = if s.dwarves_lost == 0 { "and not one of ours fell".to_string() } else { format!("and {} of ours fell", s.dwarves_lost) };
        // The sworn host: the pride's turn.
        if self.sallies.host == Some(siege) {
            if let Some(i) = self.saga_find(SagaKind::Pride, KEY_PRIDE) {
                self.sagas.list[i].weight += s.dwarves_lost;
                let text = format!("The sworn host {}, {}.{}", how, lost, called);
                if fell {
                    self.saga_end(i, true, Some(text), format!("The oath was kept, and {} is no more. The overseer's pride had been dearly bought.", fort));
                } else {
                    self.saga_add(i, Beat::Turn, text);
                    let due = self.tick + SEASON;
                    if let Some(i) = self.saga_find(SagaKind::Pride, KEY_PRIDE) {
                        self.sagas.list[i].due = due;
                    }
                }
            }
            return;
        }
        let Some(i) = self.saga_find(SagaKind::Siege, siege as u64) else { return };
        // A siege is a saga for its dead, its size, its fallen leader, or
        // a wall it broke on.
        let held = s.bridge && s.dwarves_lost <= 1;
        self.sagas.list[i].weight += if s.dwarves_lost >= 3 { 3 } else { 0 } + if held { 1 } else { 0 };
        if held {
            self.saga_add(i, Beat::Escalation, "The bridge was raised, and they broke on the walls.".to_string());
        }
        let text = format!("The {} {}: {} of them slain, {}.{}", s.foe, how, s.foes_slain, lost, called);
        let closing = if fell {
            format!("The walls of {} did not hold.", fort)
        } else if s.dwarves_lost >= 3 {
            format!("{} held, and buried its dead beneath the hall.", fort)
        } else {
            format!("The walls of {} held, and the songs of it are sung still.", fort)
        };
        self.saga_end(i, fell || s.dwarves_lost >= 3, Some(text), closing);
    }

    /// A dwarf has won a name (`earn_epithet`).
    pub(super) fn saga_epithet(&mut self, id: u32, before: &str, now: &str, deed: &str, rank: u8) {
        let text = format!("{} {}, and was hailed as {}.", before, deed, now);
        let i = match self.saga_find(SagaKind::Hero, id as u64) {
            Some(i) => {
                self.saga_add(i, Beat::Escalation, text);
                i
            }
            None => self.saga_open(SagaKind::Hero, id as u64, now, Beat::Cause, text),
        };
        self.sagas.list[i].weight += if rank >= super::heroes::RANK_BEAST { 2 } else if rank > super::heroes::RANK_FEAT { 1 } else { 0 };
    }

    /// Two rivals: one has challenged the other (`drama.rs`).
    pub(super) fn saga_challenge(&mut self, a: u32, b: u32, an: &str, bn: &str) {
        let text = format!("{} challenged {} to a duel at dawn.", an, bn);
        match self.saga_find(SagaKind::Grudge, pair(a, b)) {
            Some(i) => self.saga_add(i, Beat::Escalation, text),
            None => {
                let who = format!("{} and {}", first_name(an), first_name(bn));
                let i = self.saga_open(SagaKind::Grudge, pair(a, b), &who, Beat::Cause, format!("Bad blood between {} and {} came to insults.", an, bn));
                self.saga_add(i, Beat::Escalation, text);
            }
        }
        if let Some(i) = self.saga_find(SagaKind::Grudge, pair(a, b)) {
            self.sagas.list[i].due = self.tick + YEAR;
        }
    }

    /// The duel was fought; `peace`: they shook hands after.
    pub(super) fn saga_duel(&mut self, w: u32, l: u32, text: &str, peace: bool) {
        let Some(i) = self.saga_find(SagaKind::Grudge, pair(w, l)) else { return };
        let duels = self.sagas.list[i].chapters.iter().filter(|c| c.beat == Beat::Turn).count();
        if peace && duels == 0 {
            // Settled at the first blood: a quarrel, not a saga.
            return self.saga_drop(i);
        }
        let text = text.split(" They shook hands").next().unwrap_or(text).to_string();
        self.saga_add(i, Beat::Turn, text);
        if peace {
            self.saga_end(i, false, None, "They shook hands at last, and the grudge was laid down.".into());
        }
    }

    /// An omen was seen (`see_omen`).
    pub(super) fn saga_omen(&mut self, kind: super::omens::OmenKind) {
        let fort = self.fortress_name.clone();
        let sight = kind.sight(&fort);
        let who = match kind {
            super::omens::OmenKind::FleeingBirds => "the Fleeing Birds",
            super::omens::OmenKind::Eclipse => "the Dark Sun",
            super::omens::OmenKind::BloodMoon => "the Blood Moon",
            super::omens::OmenKind::WeepingStone => "the Weeping Stone",
            super::omens::OmenKind::ShootingStar => "the Falling Star",
            super::omens::OmenKind::Wrath => "the Cold Wind",
        };
        // An omen still waiting is forgotten for the new one.
        if let Some(i) = self.sagas.list.iter().rposition(|s| s.kind == SagaKind::Omen && s.open() && s.due == 0) {
            self.saga_drop(i);
        }
        let mut s = sight.clone();
        if let Some(c) = s.get_mut(0..1) {
            c.make_ascii_uppercase();
        }
        self.saga_open(SagaKind::Omen, kind as u64, who, Beat::Cause, format!("{}, and the dwarves muttered that {}.", s, kind.meaning()));
    }

    /// The omen came true, or came to nothing (`omens_daily`).
    pub(super) fn saga_omen_kept(&mut self, kind: super::omens::OmenKind, came: bool) {
        let Some(i) = self.saga_find(SagaKind::Omen, kind as u64) else { return };
        if !came {
            return self.saga_drop(i);
        }
        self.sagas.list[i].due = self.tick + self.knob("saga.omen_days", 20.0) as u64 * DAY;
        self.saga_add(i, Beat::Escalation, format!("The omen came true: {}.", kind.meaning()));
    }

    /// A god has cursed a dwarf (`divine_curse`).
    pub(super) fn saga_curse(&mut self, id: u32, name: &str, god: &str, why: &str) {
        if let Some(i) = self.saga_find(SagaKind::Curse, id as u64) {
            self.saga_drop(i);
        }
        let g = god.split(',').next().unwrap_or(god).to_string();
        let i = self.saga_open(SagaKind::Curse, id as u64, &g, Beat::Cause, format!("{} cursed {} for {}.", god, name, why));
        self.sagas.list[i].due = self.tick + SEASON + DAY;
    }

    /// A cursed dwarf atoned (`atone`).
    pub(super) fn saga_atoned(&mut self, id: u32, name: &str, god: &str) {
        let Some(i) = self.saga_find(SagaKind::Curse, id as u64) else { return };
        self.saga_end(i, false, Some(format!("{} knelt before {} in the temple, and atoned.", name, god)), "The curse was lifted, and the god's anger cooled.".into());
    }

    /// A squad set out to hunt (`hunts.rs`): a den (`lair`) or a legend.
    pub(super) fn saga_hunt_out(&mut self, lair: Option<usize>, beast: u32, what: &str, text: &str) {
        let key = lair.map_or(beast as u64 | (1 << 40), |l| l as u64);
        match self.saga_find(SagaKind::Hunt, key) {
            // Back to the same den: the same hunt, told once.
            Some(_) => {}
            None => {
                let i = self.saga_open(SagaKind::Hunt, key, what, Beat::Cause, text.to_string());
                if lair.is_none() {
                    self.sagas.list[i].weight += 1;
                }
            }
        }
    }

    /// A hunt is over (`end_hunt`): `kills` of the quarry, `text` as told.
    pub(super) fn saga_hunt_home(&mut self, lair: Option<u32>, beast: u32, legend: bool, kills: u32, text: Option<String>) {
        let key = lair.map_or(beast as u64 | (1 << 40), |l| l as u64);
        let Some(i) = self.saga_find(SagaKind::Hunt, key) else { return };
        let Some(text) = text.filter(|_| kills > 0) else {
            if self.sagas.list[i].chapters.len() <= 1 {
                self.saga_drop(i);
            }
            return;
        };
        self.sagas.list[i].weight += kills.min(3);
        if legend {
            self.saga_end(i, false, Some(text), "Its head hangs in the hall, and the children are told of it.".into());
            return;
        }
        // Each sortie's kills go into one tally.
        let s = &mut self.sagas.list[i];
        s.due = self.tick + YEAR;
        let tally = s.chapters.iter().filter(|c| c.beat == Beat::Turn).count();
        if tally > 0 {
            if let Some(c) = s.chapters.iter_mut().rev().find(|c| c.beat == Beat::Turn) {
                c.text = format!("{} (sortie {} of the hunt)", text.trim_end_matches('.'), tally + 1);
                c.tick = self.tick;
            }
        } else {
            self.saga_add(i, Beat::Turn, text);
        }
    }

    /// A lair has been hunted out (`lairs.rs`).
    pub(super) fn saga_lair_silent(&mut self, lair: usize, text: String) {
        let Some(i) = self.saga_find(SagaKind::Hunt, lair as u64) else { return };
        self.sagas.list[i].weight += 1;
        self.saga_end(i, false, Some(text), "The woods fell silent, and the hunters came home.".into());
    }

    /// A dwarf has died (`note_death`).
    pub(super) fn saga_death(&mut self, d: &Dwarf, cause: &str) {
        self.sagas.check_fall = true;
        let id = d.id;
        let name = d.called();
        let siege = self.annals.sieges.iter().rposition(|s| s.ended.is_none());
        let killer = self.annals.deaths.last().and_then(|n| n.killer.clone());
        let how = match &killer {
            Some(k) if !cause.contains(k.as_str()) => format!("{} {}, at the hands of {} {}", name, cause, a(k), k),
            _ => format!("{} {}", name, cause),
        };
        // A hero's end.
        if let Some(i) = self.saga_find(SagaKind::Hero, id as u64) {
            self.sagas.list[i].weight += if siege.is_some() || killer.is_some() { 1 } else { 0 };
            let closing = if siege.is_some() { format!("{} fell with the enemy at the gate, and is remembered.", first_name(&name)) } else { format!("The fortress mourned {}.", first_name(&name)) };
            self.saga_end(i, true, Some(format!("{}.", how)), closing);
        }
        // A curse that killed.
        if let Some(i) = self.saga_find(SagaKind::Curse, id as u64) {
            self.saga_end(i, true, Some(format!("{}, still under the curse.", how)), "The god's anger was not cooled.".into());
        }
        // A grudge never settled.
        while let Some(i) = self.sagas.list.iter().position(|s| s.kind == SagaKind::Grudge && s.open() && (s.key >> 32 == id as u64 || s.key & 0xFFFF_FFFF == id as u64)) {
            self.saga_add_quiet(i, Beat::Ending, format!("{}.", how));
            self.saga_end(i, true, None, "The grudge was never settled.".into());
        }
        // The first of ours to fall in a siege.
        if let Some(s) = siege {
            if let Some(i) = self.saga_find(SagaKind::Siege, s as u64) {
                if !self.sagas.list[i].chapters.iter().any(|c| c.beat == Beat::Escalation) {
                    self.saga_add(i, Beat::Escalation, format!("{} was the first of ours to fall.", name));
                }
            }
        }
        // An omen's toll.
        if let Some(i) = self.sagas.list.iter().rposition(|s| s.kind == SagaKind::Omen && s.open() && s.due > 0).filter(|_| siege.is_some() || killer.is_some()) {
            self.sagas.list[i].weight += 1;
            if self.sagas.list[i].weight == 1 {
                self.saga_add(i, Beat::Turn, format!("{}.", how));
            }
        }
    }

    fn saga_add_quiet(&mut self, i: usize, beat: Beat, text: String) {
        let tick = self.tick;
        self.sagas.list[i].chapters.push(Chapter { tick, beat, text });
    }

    /// Every day: arcs that end of themselves, and the last dwarf standing.
    pub(super) fn sagas_daily(&mut self) {
        let tick = self.tick;
        let fort = self.fortress_name.clone();
        let mut i = 0;
        while i < self.sagas.list.len() {
            let s = &self.sagas.list[i];
            if !s.open() || s.due == 0 || tick < s.due {
                i += 1;
                continue;
            }
            let n = self.sagas.list.len();
            match s.kind {
                SagaKind::Pride => {
                    let lost: u32 = self.sallies.host.and_then(|h| self.annals.sieges.get(h)).map_or(0, |s| s.dwarves_lost);
                    let (doom, closing) = if lost >= 3 {
                        (true, format!("{} endured, barely. The overseer's pride was paid for in {} lives.", fort, lost))
                    } else {
                        (false, format!("{} endured, and its gate is feared in every hall.", fort))
                    };
                    self.saga_end(i, doom, Some(format!("A season passed, and {} still stood.", fort)), closing);
                }
                SagaKind::Omen => {
                    let w = self.sagas.list[i].weight;
                    if w == 0 {
                        self.saga_drop(i);
                    } else {
                        self.saga_end(i, true, None, format!("It cost {} {} life{}, as the omen said it would.", fort, w, if w == 1 { "" } else { "s" }));
                    }
                }
                SagaKind::Curse => self.saga_drop(i),
                SagaKind::Hunt => {
                    let w = self.sagas.list[i].weight;
                    if w >= 3 {
                        self.saga_end(i, false, None, "The beasts were few after, and the hunters had their songs.".into());
                    } else {
                        self.saga_drop(i);
                    }
                }
                _ => {
                    // A grudge that cooled with no more said.
                    self.saga_drop(i);
                }
            }
            if self.sagas.list.len() >= n {
                i += 1;
            }
        }
        // The last dwarf standing.
        let alive: Vec<&Dwarf> = self.dwarves.iter().filter(|d| d.alive).collect();
        let last = self.saga_find(SagaKind::LastStand, KEY_LAST);
        if alive.len() == 1 && last.is_none() && self.dwarves.len() >= 3 {
            let name = alive[0].called();
            self.saga_open(SagaKind::LastStand, KEY_LAST, first_name(&name), Beat::Cause, format!("{} was the last of {} left alive.", name, fort));
            self.foreshadow(&format!("{} walks the halls of {} alone. This will be remembered.", name, fort));
        } else if let Some(i) = last.filter(|_| alive.len() >= 3) {
            self.saga_end(i, false, Some(format!("Others came to {}, and the halls were filled again.", fort)), format!("The lamps of {} were not put out.", fort));
        }
    }

    /// Every tick after a death: has the fort fallen?
    pub(super) fn sagas_tick(&mut self) {
        if !self.sagas.check_fall {
            return;
        }
        self.sagas.check_fall = false;
        if self.alive_count() == 0 && !self.dwarves.is_empty() {
            self.sagas_fall();
        }
    }

    /// The last dwarf is dead: every open arc ends with the fort, and the
    /// one that ended it leads its legend.
    pub fn sagas_fall(&mut self) {
        if self.sagas.fall_told {
            return;
        }
        let fort = self.fortress_name.clone();
        let tick = self.tick;
        let last = self.annals.deaths.last().cloned();
        let killer = last.as_ref().and_then(|d| d.killer.clone());
        let mut great: Option<u32> = None;
        // Arcs in the order they'd be blamed: pride, a siege, the last dwarf.
        for kind in [SagaKind::Pride, SagaKind::Siege, SagaKind::LastStand, SagaKind::Omen, SagaKind::Hunt, SagaKind::Grudge, SagaKind::Curse, SagaKind::Hero] {
            while let Some(i) = self.sagas.list.iter().position(|s| s.kind == kind && s.open()) {
                let s = &self.sagas.list[i];
                let since = s.chapters.last().map_or(0, |c| tick.saturating_sub(c.tick) / DAY);
                let what = match (&killer, kind) {
                    (Some(k), SagaKind::Pride) if since >= 1 => {
                        format!("{} day{} later, {} {} came upon {}, bereft of soldiers, and none were left.", cap_num(since), if since == 1 { "" } else { "s" }, a(k), k, fort)
                    }
                    (Some(k), _) => format!("The last of them fell to {} {}, and {} was no more.", a(k), k, fort),
                    (None, _) => match &last {
                        Some(d) => format!("{} {}, the last of {}.", d.name, d.cause, fort),
                        None => format!("{} fell silent.", fort),
                    },
                };
                let closing = match kind {
                    SagaKind::Pride => "The overseer's pride had been dearly bought.".to_string(),
                    SagaKind::LastStand => "The last lamp went out.".to_string(),
                    _ => format!("So ended {}.", fort),
                };
                let id = s.id;
                let n = self.sagas.list.len();
                self.saga_end(i, true, Some(what), closing);
                if great.is_none() && self.sagas.list.len() == n && matches!(kind, SagaKind::Pride | SagaKind::Siege | SagaKind::LastStand) {
                    great = Some(id);
                }
            }
        }
        // Else the last saga told, if it ended in grief this last season.
        if great.is_none() {
            great = self.sagas.list.iter().rev().find(|s| s.told() && s.doom && s.ended.map_or(false, |t| t + SEASON >= tick)).map(|s| s.id);
        }
        // Else the fall is a saga of its own: the last deaths, as they came.
        if great.is_none() {
            let last: Vec<annals::DeathNote> = self.annals.deaths.iter().rev().take_while(|d| d.tick + MONTH >= tick).take(4).cloned().collect();
            if let Some(first) = last.last() {
                let tell = |d: &annals::DeathNote| death_words(d);
                let i = self.saga_open(SagaKind::LastStand, KEY_LAST, &fort, Beat::Cause, tell(first));
                self.sagas.list[i].chapters[0].tick = first.tick;
                let n = last.len();
                for (k, d) in last.iter().rev().enumerate().skip(1) {
                    let beat = if k + 1 == n { Beat::Ending } else if k + 2 == n { Beat::Turn } else { Beat::Escalation };
                    self.sagas.list[i].chapters.push(Chapter { tick: d.tick, beat, text: tell(d) });
                }
                if n == 1 {
                    self.sagas.list[i].chapters.push(Chapter { tick, beat: Beat::Ending, text: format!("And then there was no one left in {}.", fort) });
                }
                let id = self.sagas.list[i].id;
                self.saga_end(i, true, None, format!("The last lamp of {} went out.", fort));
                great = self.saga(id).map(|s| s.id);
            }
        }
        self.sagas.fell_by = great;
        self.sagas.fall_told = true;
        if let Some(s) = great.and_then(|id| self.saga(id)).cloned() {
            for w in self.overworld.sagas.iter_mut().rev() {
                if w.title == s.title && w.fort == fort {
                    w.fell = true;
                    break;
                }
            }
            self.legend(format!("{} fell, and its last saga was {}: {}", fort, s.title, s.closing));
        }
    }

    /// A great siege, told from its coming to its breaking: for the
    /// screenshot tool (`--window --screenshot --saga`) and tests.
    pub fn saga_demo(&mut self) {
        self.note_siege("goblins", 24, "Snaga Ironmaw");
        let i = self.annals.sieges.len() - 1;
        self.saga_leader_fell("Snaga Ironmaw");
        let tick = self.tick;
        let s = &mut self.annals.sieges[i];
        s.bridge = true;
        s.ended = Some(tick);
        s.outcome = "routed".into();
        s.dwarves_lost = 1;
        s.foes_slain = 17;
        self.saga_siege_end(i);
    }
}

/// "Urist was struck down by a cave horror." from the annals' plain facts.
fn death_words(d: &annals::DeathNote) -> String {
    let by = d.killer.as_ref().map(|k| format!("{} {}", a(k), k));
    match (d.cause.as_str(), by) {
        ("struck down", Some(k)) => format!("{} was struck down by {}.", d.name, k),
        ("bled to death", Some(k)) => format!("{} bled to death from the wounds of {}.", d.name, k),
        ("thirst", _) | ("starvation", _) | ("old age", _) => format!("{} died of {}.", d.name, d.cause),
        ("other", Some(k)) => format!("{} was killed by {}.", d.name, k),
        ("other", None) => format!("{} died.", d.name),
        (c, Some(k)) => format!("{} {} ({}).", d.name, c, k),
        (c, None) => format!("{} {}.", d.name, c),
    }
}

fn cap_num(n: u64) -> String {
    let w = match n {
        1 => "One",
        2 => "Two",
        3 => "Three",
        4 => "Four",
        5 => "Five",
        6 => "Six",
        7 => "Seven",
        8 => "Eight",
        9 => "Nine",
        10 => "Ten",
        _ => return n.to_string(),
    };
    w.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A quiet fort with nothing going on to tell.
    fn quiet(seed: u64) -> Game {
        let mut g = Game::new(seed);
        g.creatures.retain(|_, c| c.tame);
        g
    }

    /// A siege note the sagas can read, as `note_siege` writes it.
    fn siege(g: &mut Game, foe: &str, size: u32, civ: u32) -> usize {
        g.note_siege(foe, size, "Snaga the Cruel");
        let i = g.annals.sieges.len() - 1;
        g.annals.sieges[i].civ = Some(civ);
        i
    }

    fn end_siege(g: &mut Game, i: usize, outcome: &str, lost: u32) {
        g.annals.sieges[i].ended = Some(g.tick);
        g.annals.sieges[i].outcome = outcome.into();
        g.annals.sieges[i].dwarves_lost = lost;
        g.saga_siege_end(i);
    }

    #[test]
    fn pride_and_fall_forms_from_sallies_an_oath_and_ruin() {
        let mut g = quiet(31);
        let rng_before = format!("{:?}", g.rng);
        // Two sallies won against the goblins...
        for k in 0..2 {
            g.tick += SEASON;
            let i = siege(&mut g, "goblins", 10, 3);
            g.saga_sally(i, 5, 1 + k, true);
            end_siege(&mut g, i, "routed", 0);
        }
        assert_eq!(g.told_sagas().len(), 1, "one saga, the pride: {:?}", g.sagas.list);
        // ...then a host sworn to avenge them...
        g.tick += SEASON;
        let i = siege(&mut g, "goblins", 21, 3);
        g.sallies.host = Some(i);
        g.saga_oath(i);
        assert!(g.log.iter().any(|l| l.text.contains("sworn an oath") && l.text.contains("This will be remembered")));
        assert!(g.sagas.list.iter().all(|s| s.kind != SagaKind::Siege || !s.open()), "the host's siege is the pride's");
        end_siege(&mut g, i, "routed", 4);
        let pride = g.told_sagas()[0].clone();
        assert_eq!(pride.kind, SagaKind::Pride);
        assert_eq!(pride.beat(), Beat::Turn);
        assert!(g.spotlights.iter().any(|s| s.saga == Some(pride.id)), "a card at the turn");
        // ...and five days later the last of them dies.
        g.tick += 5 * DAY;
        g.annals.deaths.push(annals::DeathNote { tick: g.tick, name: "Urist".into(), cause: "struck down".into(), killer: Some("dragon".into()), ..Default::default() });
        for d in g.dwarves.iter_mut() {
            d.alive = false;
        }
        g.sagas.check_fall = true;
        g.sagas_tick();
        let s = g.great_saga().expect("the saga that ended it").clone();
        assert_eq!(s.id, pride.id);
        assert!(s.doom && !s.open());
        let beats: Vec<Beat> = s.chapters.iter().map(|c| c.beat).collect();
        assert_eq!(beats, vec![Beat::Cause, Beat::Escalation, Beat::Escalation, Beat::Turn, Beat::Ending]);
        assert!(s.chapters.last().unwrap().text.starts_with("Five days later, a dragon came upon"), "{:?}", s.chapters.last());
        assert!(g.overworld.sagas.iter().any(|w| w.title == s.title && w.fell), "the world keeps it");
        assert!(g.overworld.events.iter().any(|e| e.text.contains(&s.title)));
        assert_eq!(format!("{:?}", g.rng), rng_before, "noticing sagas draws nothing from the game's RNG");
    }

    #[test]
    fn a_pride_that_survives_its_host_ends_after_a_season() {
        let mut g = quiet(32);
        let i = siege(&mut g, "goblins", 10, 3);
        g.saga_sally(i, 5, 0, true);
        end_siege(&mut g, i, "routed", 0);
        let h = siege(&mut g, "goblins", 14, 3);
        g.sallies.host = Some(h);
        g.saga_oath(h);
        end_siege(&mut g, h, "routed", 1);
        g.tick += SEASON + DAY;
        g.sagas_daily();
        let s = g.told_sagas()[0];
        assert!(!s.open() && !s.doom, "{:?}", s);
        assert!(s.closing.contains("endured"));
    }

    #[test]
    fn titles_are_steady_and_vary() {
        let a = title_for(SagaKind::Pride, "Anvilbeard", 1000, "Anvilbeard");
        assert_eq!(a, title_for(SagaKind::Pride, "Anvilbeard", 1000, "Anvilbeard"));
        let all: std::collections::BTreeSet<String> = (0..40).map(|t| title_for(SagaKind::Pride, "Anvilbeard", t * 977, "Anvilbeard")).collect();
        assert!(all.len() >= 3, "{:?}", all);
        assert!(all.iter().all(|t| !t.contains('{')), "{:?}", all);
        for k in [SagaKind::Grudge, SagaKind::Hero, SagaKind::Omen, SagaKind::Curse, SagaKind::Hunt, SagaKind::Siege, SagaKind::LastStand] {
            for t in 0..20 {
                let s = title_for(k, "Emberforge", t * 131, "Kolvar");
                assert!(!s.contains('{') && s.starts_with(|c: char| c.is_uppercase()), "{}", s);
            }
        }
    }

    #[test]
    fn a_small_siege_is_forgotten_and_a_great_one_told() {
        let mut g = quiet(33);
        let i = siege(&mut g, "goblins", 4, 1);
        end_siege(&mut g, i, "withdrew", 0);
        assert!(g.sagas.list.is_empty(), "{:?}", g.sagas.list);
        let i = siege(&mut g, "goblins", 18, 1);
        g.saga_leader_fell("Snaga the Cruel");
        g.annals.sieges[i].bridge = true;
        end_siege(&mut g, i, "routed", 1);
        let s = g.told_sagas();
        assert_eq!(s.len(), 1, "{:?}", g.sagas.list);
        assert!(!s[0].doom && s[0].chapters.len() >= 4);
    }

    #[test]
    fn a_grudge_ends_in_a_death_and_a_hero_in_a_fall() {
        let mut g = quiet(34);
        let (a, b) = (g.dwarves[0].id, g.dwarves[1].id);
        let (an, bn) = (g.dwarves[0].called(), g.dwarves[1].called());
        for _ in 0..2 {
            g.saga_challenge(a, b, &an, &bn);
            g.saga_duel(a, b, "At dawn, they fought. A won; B yielded, bloodied.", false);
        }
        let mut d = std::mem::take(&mut g.dwarves[1]);
        g.saga_epithet(d.id, &bn, &format!("{} the Grim", bn), "slew ten foes", 2);
        g.saga_epithet(d.id, &bn, &format!("{} the Dragonslayer", bn), "slew a dragon", 5);
        g.kill(&mut d, "was struck down");
        g.dwarves[1] = d;
        let told = g.told_sagas();
        assert!(told.iter().any(|s| s.kind == SagaKind::Grudge && s.doom && s.closing.contains("never settled")), "{:?}", g.sagas.list);
        assert!(told.iter().any(|s| s.kind == SagaKind::Hero && !s.open()), "{:?}", g.sagas.list);
    }

    #[test]
    fn sagas_round_trip_and_old_saves_load_without_them() {
        let mut g = quiet(35);
        let i = siege(&mut g, "goblins", 20, 1);
        g.saga_leader_fell("Snaga");
        end_siege(&mut g, i, "routed", 3);
        assert!(!g.sagas.list.is_empty());
        let back: Sagas = serde_json::from_str(&serde_json::to_string(&g.sagas).unwrap()).unwrap();
        assert_eq!(back.list, g.sagas.list);
        let w: Vec<WorldSaga> = serde_json::from_str(&serde_json::to_string(&g.overworld.sagas).unwrap()).unwrap();
        assert_eq!(w, g.overworld.sagas);
        // A fort and a world saved before sagas (the whole of an old fort
        // loading is `saves::a_save_from_before_m1_plays_on`).
        let old: Sagas = serde_json::from_str("{}").unwrap();
        assert!(old.list.is_empty() && old.fell_by.is_none());
        let mut v = serde_json::to_value(&g.overworld).unwrap();
        v.as_object_mut().unwrap().remove("sagas");
        let ow: crate::overworld::Overworld = serde_json::from_value(v).unwrap();
        assert!(ow.sagas.is_empty());
    }
}
