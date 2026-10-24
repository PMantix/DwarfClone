//! Quests (A8): work given by a named person of a town, tied to the
//! world's own history. Ask "Is there work?" and the elder of a dwarf hold
//! may ask you to avenge the fort their kin lost, a hunter to slay the
//! beast of legend that lairs in the hills, a smith to bring home the axe
//! left in a fallen fort's deep hall, a guard to clear the ruin bandits
//! have made their nest. Every quest is a place in the world, marked on
//! the travel map, and a chapter of the hero's saga when it is sworn.
//!
//! What can be asked of the hero is read from what the speaker's town
//! knows (fallen forts and what holds them, the lairs of living beasts,
//! ruins), and which one a speaker asks is a hash of their name, never
//! the dice: ask again, hear the same. A giver offers one quest; the hero
//! holds three at most.
//!
//! `check_quests` runs with `notice`, after every act: a slain figure, a
//! cleared ruin or a relic in hand wins a quest, and walking back into the
//! giver's town pays it (coins, standing with their people, and a deed
//! the legends tell). A hero who dies with oaths unkept takes them into
//! the legends too (`oaths_at_death`).

use super::talk::{days_words, Person, KNOWN_FORT};
use super::travel::{cheb, way_to};
use super::{Adventure, Quest, QuestKind, QuestState, SiteKey, HERO_ID};
use crate::game::stories;
use crate::items::ARTIFACT;
use crate::overworld::{Occupant, Race, SiteKind};

/// Most quests the hero holds at once.
pub const MAX_OPEN: usize = 3;
/// What a quest done is worth with the giver's people.
const STANDING: i32 = 3;
/// A quest done is told in the legends.
const QUEST_WEIGHT: i32 = 3;

/// A quest the speaker could give, before it is sworn.
#[derive(Clone)]
struct Offer {
    kind: QuestKind,
    target: String,
    key: SiteKey,
    cell: (i32, i32),
    place: String,
    figure: Option<u32>,
    /// What killed a fort, for the plea.
    killer: String,
    weight: i32,
    /// How hard: sets the reward.
    danger: u32,
}

impl Quest {
    /// What it asks, in a few words: "Slay Ashmaw the dragon".
    pub fn ask(&self) -> String {
        match self.kind {
            QuestKind::Slay => format!("Slay {}", self.target),
            QuestKind::Recover => format!("Bring {} home from {}", self.target, self.place),
            QuestKind::Clear => format!("Clear the ruin of {}", self.place),
            QuestKind::Avenge if self.target != self.place => format!("Avenge {}: slay {}", self.place, self.target),
            QuestKind::Avenge => format!("Avenge {}", self.place),
        }
    }

    /// The giver's first name.
    pub fn giver_first(&self) -> &str {
        self.giver.split([' ', ',']).next().unwrap_or(&self.giver)
    }
}

// --- The givers' words, by kind and people (dwarf, human, elf, goblin).
// `{place}` `{target}` `{killer}` `{reward}`. ---

const AVENGE: [&[&str]; 4] = [
    &["{place} was our kin's hold. {killer} put out its hearths, and its dead lie unburied. Avenge them, and you'll have {reward} coins and our people's thanks.", "My brother died in {place}. {killer} did it. I'm too old to carry an axe there. You're not. {reward} coins, and my blessing.", "Nobody has answered for {place}. {killer} still has its halls. Make it answer, and {reward} coins are yours."],
    &["Our cousins of {place} were butchered by {killer}. Avenge them, and we'll pay {reward} coins.", "{place} fell to {killer}, and nobody has lifted a hand since. Will you? {reward} coins."],
    &["The bearded ones of {place} were our friends, once. {killer} ended them. End {killer}, for {reward} coins.", "{place} cries out under {killer}. Answer it, and the grove gives you {reward} coins."],
    &["{place} was ours! {killer} took it. Kill them all, and I'll give you {reward} coins. Maybe.", "Get {place} back from {killer}. {reward} coins. Don't ask where I got them."],
];

const RECOVER: [&[&str]; 4] = [
    &["{target} lies in the dark of {place}, where it fell. Bring it home to us. {reward} coins, and our people will sing of it.", "My grandmother's hands made {target}. It's still down in {place}. Bring it back and I'll pay {reward} coins.", "{target} should not lie in {place} with the dead. Bring it home: {reward} coins."],
    &["There's a treasure called {target} in the ruin of {place}. Fetch it to me and I'll pay {reward} coins, no questions.", "A scholar would pay well for {target}, from {place}. I'll pay better: {reward} coins."],
    &["{target} sleeps in {place}. It should not sleep among the dead. {reward} coins to bring it to us.", "Bring {target} out of {place}, and the grove will owe you {reward} coins."],
    &["{target}. In {place}. Shiny. Bring it to me, {reward} coins. Deal?", "Dead dwarves don't need {target}. I do. It's in {place}. {reward} coins."],
];

const SLAY: [&[&str]; 4] = [
    &["{target} lairs at {place}. It has taken more of ours than I'll name. Kill it, and {reward} coins are yours.", "Our hunters won't go near {place} since {target} came. Put it down: {reward} coins.", "Bring me word that {target} is dead, and I'll pay {reward} coins and drink to you."],
    &["{target} has made {place} its lair. Sheep, children... Kill it, friend. {reward} coins, the whole village chipped in.", "There's a bounty on {target}, at {place}: {reward} coins. Nobody's lived to collect it."],
    &["{target} is a wound in the world, at {place}. Close it. The grove gives {reward} coins.", "{target} must not wake again. It lairs at {place}. {reward} coins, and our songs."],
    &["{target} ate my cousin at {place}. I didn't like him much, but still. {reward} coins if you kill it.", "Kill {target} at {place}. Then we raid there. {reward} coins for you."],
];

const CLEAR: [&[&str]; 4] = [
    &["Something has made {place} its nest. Clear it out, and {reward} coins are yours.", "The old stones of {place} shelter filth now. Clear them, for {reward} coins."],
    &["Bandits or worse are using {place}. Clear it out and you'll have {reward} coins.", "The ruin at {place} isn't empty any more, and our carts go past it. Clear it: {reward} coins."],
    &["{place} is fouled. Cleanse it, and the grove gives {reward} coins.", "Things lurk in {place}. Drive them out, for {reward} coins."],
    &["Somebody's squatting in {place}. Our spot. Get them out: {reward} coins.", "Clear out {place} for us. {reward} coins. Easy work for a killer like you."],
];

/// "Our thanks": the giver pays.
const PAID: [&[&str]; 4] = [
    &["It's done? Then here: {reward} coins, and you'll drink free in this hall.", "By my beard. You did it. {reward} coins, and our thanks for as long as there's stone."],
    &["You did it! Here, {reward} coins, as promised. They'll sing of this at the inn.", "I never thought I'd see the day. {reward} coins, friend, and my thanks."],
    &["The grove is grateful. Take these {reward} coins, and walk softly.", "It is done, then. {reward} coins, and our songs will carry your name."],
    &["Hah! You did it! {reward} coins. Don't spend them all on ale. Or do.", "Good work. Here's {reward} coins. I only bit one."],
];

/// The hero asks again while the quest is open.
const WAITING: [&[&str]; 4] = [
    &["Well? {ask}. It still waits.", "Not done yet? Then why are you drinking my ale?"],
    &["Any luck? {ask}, remember.", "Still here? The road's that way, friend."],
    &["The grove waits. {ask}.", "Not yet, I see. Be patient, and be brave."],
    &["Not done? Then go do it.", "Talk's cheap. {ask}."],
];

fn race_idx(r: Race) -> usize {
    match r {
        Race::Dwarf => 0,
        Race::Human => 1,
        Race::Elf => 2,
        Race::Goblin => 3,
    }
}

fn fill(t: &str, with: &[(&str, &str)]) -> String {
    let mut s = t.to_string();
    for (k, v) in with {
        s = s.replace(&format!("{{{}}}", k), v);
    }
    s
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// A speaker's hash: the same person always asks the same.
fn h(p: &Person, salt: u64) -> u64 {
    stories::hash(&p.name, salt ^ 0x9E57_0A8)
}

/// Whether a speaker has work to give: the elder of a place always, its
/// guards and some of its folk; travellers never (they don't live here).
pub fn gives_work(p: &Person) -> bool {
    p.home.is_some() && (p.role.starts_with("elder") || p.role.starts_with("guard") || h(p, 1) % 3 == 0)
}

/// What an item is called in the legends (its label, else its kind).
fn called(it: &crate::items::Item) -> String {
    it.label.clone().unwrap_or_else(|| it.name())
}

impl Adventure {
    /// What `p` could ask of the hero from where they live: the weightiest
    /// first. Places the hero is already sworn to aren't asked twice.
    fn offers(&self, p: &Person) -> Vec<Offer> {
        let ow = self.ow();
        let at = self.at;
        let r = self.knob("quest.radius", 14.0) as i32;
        let near = |d: i32| 100 * (r + 1) / (d + 2);
        // What each people asks of strangers most.
        let likes = |k: QuestKind| -> i32 {
            match (p.race, k) {
                (Race::Dwarf, QuestKind::Avenge) | (Race::Dwarf, QuestKind::Recover) | (Race::Elf, QuestKind::Slay) | (Race::Human, QuestKind::Slay) | (Race::Goblin, QuestKind::Clear) => 3,
                (Race::Human, QuestKind::Clear) | (Race::Elf, QuestKind::Clear) | (Race::Goblin, QuestKind::Recover) => 2,
                _ => 1,
            }
        };
        let sworn: Vec<SiteKey> = self.quests.iter().filter(|q| matches!(q.state, QuestState::Open | QuestState::Won)).filter_map(|q| q.key).collect();
        let mut v: Vec<Offer> = Vec::new();
        for (i, f) in ow.forts.iter().enumerate() {
            if f.fell.is_none() || matches!(f.occupant, Some(Occupant::Empty)) {
                continue;
            }
            let d = cheb(f.site, at);
            let key = SiteKey::Fort(i as u32);
            if d > r || sworn.contains(&key) {
                continue;
            }
            let beast = match &f.occupant {
                Some(Occupant::Beast { figure }) => ow.figures.get(*figure as usize).filter(|b| b.alive()),
                _ => None,
            };
            let killer = match (beast, &f.occupant, &f.killer) {
                (Some(b), _, _) => format!("{} the {}", b.name, b.kind),
                (_, Some(Occupant::Horror { label }), _) => label.clone(),
                (_, _, Some(k)) => format!("the {}s", k.trim_end_matches('s')),
                _ => "something nobody saw".to_string(),
            };
            let base = Offer { kind: QuestKind::Clear, target: f.name.clone(), key, cell: f.site, place: f.name.clone(), figure: None, killer, weight: 0, danger: 3 };
            // Kin avenge their own.
            if p.civ.is_some() && p.civ == f.civ {
                let target = beast.map_or(f.name.clone(), |b| format!("{} the {}", b.name, b.kind));
                v.push(Offer { kind: QuestKind::Avenge, target, figure: beast.map(|b| b.id), weight: near(d) * 4 * likes(QuestKind::Avenge), danger: 4, ..base });
                continue;
            }
            let taken: Vec<&String> = f.robbed.iter().flat_map(|rb| rb.took.iter()).collect();
            if let Some(a) = f.artifacts.iter().find(|a| !taken.contains(a)) {
                v.push(Offer { kind: QuestKind::Recover, target: a.clone(), weight: near(d) * 3 * likes(QuestKind::Recover), danger: 4, ..base });
            } else {
                v.push(Offer { weight: near(d) * likes(QuestKind::Clear), ..base });
            }
        }
        for fid in ow.beasts_near(at.0, at.1, r) {
            let b = &ow.figures[fid as usize];
            let Some(lair) = b.lair.and_then(|l| ow.sites.get(l as usize)) else { continue };
            let key = SiteKey::Site(lair.id);
            if sworn.contains(&key) {
                continue;
            }
            let d = cheb((lair.x, lair.y), at);
            let target = format!("{} the {}", b.name, b.kind);
            v.push(Offer { kind: QuestKind::Slay, target: target.clone(), key, cell: (lair.x, lair.y), place: lair.name.clone(), figure: Some(fid), killer: target, weight: near(d) * (2 + b.kills.min(4) as i32) * likes(QuestKind::Slay) / 2, danger: 5 + b.kills.min(5) });
        }
        for s in ow.sites.iter().filter(|s| s.ruined && s.kind != SiteKind::Lair) {
            let d = cheb((s.x, s.y), at);
            let key = SiteKey::Site(s.id);
            if d > r || d == 0 || ow.fort_at(s.x, s.y).is_some() || sworn.contains(&key) || self.memos.get(&key).map_or(false, |m| m.cleared) {
                continue;
            }
            v.push(Offer { kind: QuestKind::Clear, target: s.name.clone(), key, cell: (s.x, s.y), place: s.name.clone(), figure: None, killer: String::new(), weight: near(d) * 2 * likes(QuestKind::Clear), danger: 2 });
        }
        v.sort_by(|a, b| b.weight.cmp(&a.weight).then(a.target.cmp(&b.target)));
        v
    }

    /// The quest `p` would give (one of the three weightiest, by their
    /// hash; or the one about `want`), with their words. Not yet sworn.
    fn quest_of(&self, p: &Person, want: Option<SiteKey>) -> Option<Quest> {
        let offers = self.offers(p);
        let n = offers.len().min(3);
        let o = match want.and_then(|k| offers.iter().find(|o| o.key == k)) {
            Some(o) => o.clone(),
            None if n > 0 => offers[(h(p, 2) % n as u64) as usize].clone(),
            None => return None,
        };
        let reward = 10 + 8 * o.danger;
        let t = match o.kind {
            QuestKind::Avenge => &AVENGE,
            QuestKind::Recover => &RECOVER,
            QuestKind::Slay => &SLAY,
            QuestKind::Clear => &CLEAR,
        };
        let plea = fill(stories::pick(h(p, 3), 3, t[race_idx(p.race)]), &[("place", &o.place), ("target", &o.target), ("killer", &o.killer), ("reward", &reward.to_string())]);
        let plea = cap(&plea);
        Some(Quest {
            id: self.quests.iter().map(|q| q.id + 1).max().unwrap_or(1),
            kind: o.kind,
            giver: p.name.clone(),
            at_site: p.home,
            target: o.target,
            reward,
            state: QuestState::Open,
            key: Some(o.key),
            cell: o.cell,
            place: o.place,
            figure: o.figure,
            civ: p.civ,
            plea,
            year: self.year(),
        })
    }

    /// "Is there work?" asked of `who` (A7's talk menu): the quest they
    /// give, in their words, sworn at once and written in the quest log;
    /// or how the one they gave goes; or a reward paid. None: no work,
    /// and talk answers for them.
    pub fn quest_talk(&mut self, who: &Person) -> Option<Vec<String>> {
        let ri = race_idx(who.race);
        // One they gave already.
        if let Some(i) = self.quests.iter().rposition(|q| q.giver == who.name && q.at_site == who.home) {
            let q = self.quests[i].clone();
            return Some(match q.state {
                QuestState::Open => {
                    let ask = q.ask();
                    let mut v = vec![fill(stories::pick(h(who, 4), 4, WAITING[ri]), &[("ask", &ask)])];
                    if q.cell != self.at {
                        v.push(format!("{} is {} {}.", q.place, days_words(cheb(q.cell, self.at)), way_to(self.at, q.cell)));
                    }
                    v
                }
                QuestState::Won => {
                    // (Paid here and now, in their own town.)
                    self.check_quests();
                    vec!["Our thanks, always.".to_string()]
                }
                _ => vec!["You've done right by us. There's nothing more I'd ask.".to_string()],
            });
        }
        if !gives_work(who) {
            return None;
        }
        if self.quests.iter().filter(|q| q.state == QuestState::Open).count() >= MAX_OPEN {
            return Some(vec!["You've three oaths on your back already. Keep those first, then come back.".to_string()]);
        }
        let q = self.quest_of(who, None)?;
        let lines = vec![q.plea.clone(), "Then it's sworn. Go well, and come back to me.".to_string()];
        self.take_quest(q);
        Some(lines)
    }

    /// A quest sworn: in the log, its place marked on the map, and a
    /// chapter of the hero's saga.
    fn take_quest(&mut self, q: Quest) {
        let mark = match q.key {
            Some(SiteKey::Site(id)) => Some(id),
            Some(SiteKey::Fort(i)) => Some(KNOWN_FORT | i),
            _ => None,
        };
        if let Some(m) = mark {
            self.hero.known.insert(m);
        }
        let home = self.site_called(q.at_site);
        let sworn = match q.kind {
            QuestKind::Avenge => format!("swore to {} of {} to avenge the fall of {}", q.giver, home, q.place),
            QuestKind::Recover => format!("swore to {} of {} to bring {} home from {}", q.giver, home, q.target, q.place),
            QuestKind::Slay => format!("swore to {} of {} to slay {}", q.giver, home, q.target),
            QuestKind::Clear => format!("swore to {} of {} to clear the ruin of {}", q.giver, home, q.place),
        };
        self.say(&format!("You have sworn: {}. (j: your quests)", q.ask()), (255, 215, 90));
        self.quests.push(q);
        self.deed(&sworn, 0);
    }

    fn site_called(&self, site: Option<u32>) -> String {
        site.and_then(|s| self.ow().sites.iter().find(|x| x.id == s)).map_or("the road".to_string(), |s| s.name.clone())
    }

    /// The reward for quest `i`, paid by its giver (the hero is in their
    /// town): coins, standing with their people, and a deed for the
    /// legends. The giver's thanks go in the log.
    fn pay(&mut self, i: usize, race: Race) {
        let Some(q) = self.quests.get_mut(i).filter(|q| q.state == QuestState::Won) else { return };
        q.state = QuestState::Done;
        let q = q.clone();
        self.hero.coins += q.reward;
        self.stats.gold += q.reward;
        if let Some(c) = q.civ {
            *self.fame.by_civ.entry(c).or_default() += STANDING;
        }
        let home = self.site_called(q.at_site);
        let did = match q.kind {
            QuestKind::Avenge => format!("avenged the fall of {}, as {} of {} had asked", q.place, q.giver, home),
            QuestKind::Recover => format!("brought {} home from {} to {} of {}", q.target, q.place, q.giver, home),
            QuestKind::Slay => format!("brought {} of {} word that {} was dead", q.giver, home, q.target),
            QuestKind::Clear => format!("cleared the ruin of {} for {} of {}", q.place, q.giver, home),
        };
        self.deed(&did, QUEST_WEIGHT);
        let words = fill(stories::pick(stories::hash(&q.giver, 5), 5, PAID[race_idx(race)]), &[("reward", &q.reward.to_string())]);
        self.say(&format!("{}: \"{}\"", q.giver_first(), words), (255, 215, 90));
    }

    /// Check open quests (after every act, and on arrival): a figure slain,
    /// a place cleared, a relic in hand wins one; back in the giver's town
    /// a won quest is paid (and a relic handed over).
    pub fn check_quests(&mut self) {
        if self.quests.is_empty() || self.over.is_some() {
            return;
        }
        let key = self.visit.as_ref().map(|v| v.key);
        for i in 0..self.quests.len() {
            let q = self.quests[i].clone();
            if q.state == QuestState::Open && self.quest_won(&q, key) {
                self.quests[i].state = QuestState::Won;
                let home = self.site_called(q.at_site);
                let done = match q.kind {
                    QuestKind::Recover => format!("{} is yours. Bring it to {} in {}.", q.target, q.giver_first(), home),
                    QuestKind::Clear => format!("Nothing holds {} any more. {} in {} will want to hear of it.", q.place, q.giver_first(), home),
                    _ if q.figure.is_some() => format!("{} is dead! {} in {} will want to hear of it.", cap(&q.target), q.giver_first(), home),
                    _ => format!("{} is avenged. {} in {} will want to hear of it.", q.place, q.giver_first(), home),
                };
                self.say(&done, (255, 215, 90));
            }
            // Home to the giver: the reward (a relic is handed over).
            let at_giver = q.at_site.is_some() && key == q.at_site.map(SiteKey::Site);
            if at_giver && self.quests[i].state == QuestState::Won {
                if q.kind == QuestKind::Recover && !self.hand_over(&q.target) {
                    // (Dropped on the way: it is still to be brought.)
                    self.quests[i].state = QuestState::Open;
                    continue;
                }
                let race = self.ow().sites.iter().find(|s| Some(s.id) == q.at_site).and_then(|s| s.civ).and_then(|c| self.ow().civs.get(c as usize)).map_or(Race::Human, |c| c.race);
                self.pay(i, race);
            }
        }
    }

    /// Whether quest `q` is won, as the world stands (`key`: where the
    /// hero is).
    fn quest_won(&self, q: &Quest, key: Option<SiteKey>) -> bool {
        if let Some(f) = q.figure {
            return self.ow().figures.get(f as usize).map_or(false, |f| !f.alive());
        }
        if q.kind == QuestKind::Recover {
            return self.carries(&q.target);
        }
        // Cleared: inside it, nothing that held it is left; or the site's
        // memory says so; or a fort's mark does.
        if let (Some(k), Some(v)) = (q.key, &self.visit) {
            if key == Some(k) {
                let held = v.placed.iter().filter(|(_, k)| k.starts_with('@')).any(|(id, _)| v.g.creatures.get(id).map_or(false, |c| c.alive && (c.hostile() || c.siege)));
                return !held;
            }
        }
        match q.key {
            Some(SiteKey::Fort(i)) => matches!(self.ow().forts.get(i as usize).and_then(|f| f.occupant.as_ref()), Some(Occupant::Empty)),
            Some(k) => self.memos.get(&k).map_or(false, |m| m.cleared),
            None => false,
        }
    }

    /// Whether the hero carries a thing by this name.
    fn carries(&self, name: &str) -> bool {
        match &self.visit {
            Some(v) => v.g.items.values().any(|it| it.carried_by == Some(HERO_ID) && called(it) == name),
            None => self.hero.pack.iter().any(|it| called(it) == name),
        }
    }

    /// Give the relic to its giver: out of the hero's hands for good.
    fn hand_over(&mut self, name: &str) -> bool {
        let Some(v) = self.visit.as_mut() else { return false };
        let Some(id) = v.g.items.values().filter(|it| it.carried_by == Some(HERO_ID) && called(it) == name).map(|it| it.id).min() else { return false };
        let d = &mut v.g.dwarves[v.hero];
        if d.held == Some(id) {
            d.held = None;
        }
        d.armor.retain(|&a| a != id);
        d.clothes.retain(|&a| a != id);
        if v.g.items.remove(&id).map_or(false, |it| it.quality >= ARTIFACT) {
            // (Counted as taken already: given away, it isn't taken again.)
            self.fame.seen.brought.push(name.to_string());
        }
        true
    }

    /// The hero died with oaths unkept: the legends say so. The lines.
    pub fn oaths_at_death(&mut self) -> Vec<String> {
        let called = self.hero_dwarf().called();
        let open: Vec<Quest> = self.quests.iter().filter(|q| matches!(q.state, QuestState::Open | QuestState::Won)).cloned().collect();
        let mut lines = Vec::new();
        for q in open {
            let line = match (q.state, q.kind) {
                (QuestState::Won, _) => format!("{} never came back to tell {} that it was done.", called, q.giver),
                (_, QuestKind::Avenge) => format!("{} died with {} still unavenged, as {} had asked.", called, q.place, q.giver),
                (_, QuestKind::Slay) => format!("{} died sworn to slay {}, and {} lives on.", called, q.target, q.target),
                (_, QuestKind::Recover) => format!("{} died before bringing {} home to {}.", called, q.target, q.giver),
                (_, QuestKind::Clear) => format!("{} died sworn to clear {}, and it is not cleared.", called, q.place),
            };
            if let Some(q) = self.quests.iter_mut().find(|x| x.id == q.id) {
                q.state = QuestState::Failed;
            }
            lines.push(line);
        }
        let year = self.year();
        for l in &lines {
            self.ow_mut().log(year, l.clone());
        }
        lines
    }

    /// The quest log, for `j`, newest first: (state, what and for whom,
    /// the way there or what is left to do).
    pub fn quest_lines(&self) -> Vec<(QuestState, String, String)> {
        self.quests
            .iter()
            .rev()
            .map(|q| {
                let home = self.site_called(q.at_site);
                let way = match q.state {
                    QuestState::Open if q.cell == self.at => format!("{}: you are there.", q.place),
                    QuestState::Open => format!("{}: {} {}.", q.place, days_words(cheb(q.cell, self.at)), way_to(self.at, q.cell)),
                    QuestState::Won => {
                        let to = q.at_site.and_then(|s| self.ow().sites.iter().find(|x| x.id == s)).map_or(self.at, |s| (s.x, s.y));
                        let way = if to == self.at { "here".to_string() } else { format!("{} {}", days_words(cheb(to, self.at)), way_to(self.at, to)) };
                        format!("Done! Back to {} in {} for the reward: {}.", q.giver_first(), home, way)
                    }
                    QuestState::Done => format!("Done, and paid: {} coins.", q.reward),
                    QuestState::Failed => "Never done.".to_string(),
                };
                (q.state, format!("{}, for {} of {} ({} coins)", q.ask(), q.giver, home, q.reward), way)
            })
            .collect()
    }

    /// For the bot (A18): ask the folk here for work, as a player would,
    /// until one asks for the deed at `want` (else take what the first
    /// offers). The line for its story.
    pub fn seek_work(&mut self, want: SiteKey) -> Option<String> {
        let v = self.visit.as_ref()?;
        let mut ids: Vec<u32> = v.g.creatures.values().filter(|c| c.alive && (c.visitor == super::sitegen::town::FOLK || c.visitor == super::sitegen::town::GUARD) && !c.hostile() && !c.siege).map(|c| c.id).collect();
        ids.sort();
        let people: Vec<Person> = ids.into_iter().filter_map(|id| self.person(id)).filter(|p| gives_work(p) && !self.greet(p).closed).collect();
        let p = people.iter().find(|p| self.offers(p).iter().any(|o| o.key == want)).or(people.first())?.clone();
        let q = self.quest_of(&p, Some(want))?;
        let line = format!("{}, {}, asks: \"{}\"", p.name, p.role, q.plea);
        self.take_quest(q);
        Some(line)
    }
}

#[cfg(test)]
mod tests {
    use super::super::sitegen::town::tests::{visit, world};
    use super::super::sitegen::town::FOLK;
    use super::super::talk::Topic;
    use super::*;
    use crate::overworld::FortMark;

    /// A hero of a town's own civ, in that town.
    fn town(seed: u64) -> Adventure {
        let ow = world();
        let s = ow
            .sites
            .iter()
            .find(|s| !s.ruined && matches!(s.kind, SiteKind::Town | SiteKind::Hamlet | SiteKind::Fortress) && s.civ.and_then(|c| ow.civs.get(c as usize)).map_or(false, |c| c.race != Race::Goblin))
            .unwrap()
            .clone();
        let race = ow.civs[s.civ.unwrap() as usize].race;
        visit(s.id, race, s.civ, seed)
    }

    /// The town's elder.
    fn elder(a: &Adventure) -> Person {
        let v = a.visit.as_ref().unwrap();
        let mut ids: Vec<u32> = v.g.creatures.values().filter(|c| c.visitor == FOLK && c.alive).map(|c| c.id).collect();
        ids.sort();
        ids.into_iter().filter_map(|id| a.person(id)).find(|p| p.role.starts_with("elder")).expect("an elder")
    }

    /// A fort of the town's own people, fallen 3 cells off, with a relic.
    fn fallen_kin(a: &mut Adventure) -> u32 {
        let (x, y) = a.at;
        let civ = a.hero.civ;
        let ow = a.ow_mut();
        let fx = if x + 3 < ow.w { x + 3 } else { x - 3 };
        ow.forts.push(FortMark {
            name: "Copperhold".into(),
            site: (fx, y),
            civ,
            founded: 250,
            fell: Some(253),
            killer: Some("goblin".into()),
            artifacts: vec!["Ironmoon, a steel axe".into()],
            save_file: "copperhold.json".into(),
            peak_pop: 30,
            journal: Vec::new(),
            occupant: None,
            robbed: Vec::new(),
        });
        (a.ow().forts.len() - 1) as u32
    }

    #[test]
    fn a_town_offers_to_slay_the_beast_of_the_hills() {
        let mut a = town(41);
        let p = elder(&a);
        // Only a beast of legend to ask about: its lair two cells off, no
        // fort or ruin near.
        let (x, y) = a.at;
        a.ow_mut().forts.clear();
        for s in a.ow_mut().sites.iter_mut().filter(|s| s.ruined && s.kind != SiteKind::Lair) {
            s.ruined = false;
        }
        let lair = a.ow().sites.iter().find(|s| s.kind == SiteKind::Lair).cloned().expect("a lair");
        {
            let ow = a.ow_mut();
            for f in ow.figures.iter_mut() {
                if f.lair.is_some() {
                    f.lair = None;
                }
            }
            let l = ow.sites.iter_mut().find(|s| s.id == lair.id).unwrap();
            l.x = (x + 2).min(ow.w - 1);
            l.y = y;
            let f = &mut ow.figures[0];
            f.lair = Some(lair.id);
            f.died = None;
        }
        let lair = a.ow().sites.iter().find(|s| s.id == lair.id).cloned().unwrap();
        let mut chat = a.greet(&p);
        a.ask(&mut chat, Topic::Work);
        let q = a.quests.last().cloned().expect("the elder gives work");
        assert_eq!(q.kind, QuestKind::Slay);
        assert_eq!(q.figure, Some(0));
        assert_eq!(q.giver, p.name);
        assert!(q.reward > 0 && q.plea.contains(&q.target), "{:?}", q);
        assert!(chat.lines.iter().any(|(_, l)| l == &q.plea), "the plea is said: {:?}", chat.lines);
        assert!(a.hero.known.contains(&lair.id), "its lair marked on the map");
        assert!(a.fame.saga.as_ref().unwrap().chapters.iter().any(|c| c.2.contains("swore to")), "a chapter of the saga");
        // In the log, with the way there.
        let log = a.quest_lines();
        assert!(log[0].1.starts_with("Slay ") && log[0].2.contains(&lair.name) && log[0].2.contains("east"), "{:?}", log);
        // Asked again: the same giver doesn't give two.
        let n = a.quests.len();
        a.ask(&mut chat, Topic::Work);
        assert_eq!(a.quests.len(), n);
        // The beast dies: won, and (in the giver's town already) paid.
        let year = a.year();
        a.ow_mut().figures[0].died = Some(year);
        let coins = a.hero.coins;
        a.check_quests();
        assert_eq!(a.quests.last().unwrap().state, QuestState::Done);
        assert_eq!(a.hero.coins, coins + q.reward);
        assert!(a.fame.deeds.iter().any(|d| d.text.contains("was dead") && d.weight >= QUEST_WEIGHT));
        assert!(a.ow().events.iter().any(|e| e.text.contains(&q.target) && e.text.contains("word")), "the legends tell it");
    }

    #[test]
    fn the_relic_comes_home_and_the_giver_pays() {
        let mut a = town(42);
        let fort = fallen_kin(&mut a);
        // Not kin: a stranger's fort, so it's the relic they want.
        a.ow_mut().forts[fort as usize].civ = None;
        let p = elder(&a);
        assert!(a.offers(&p).iter().any(|o| o.key == SiteKey::Fort(fort) && o.kind == QuestKind::Recover));
        let line = a.seek_work(SiteKey::Fort(fort)).expect("work about the fort");
        assert!(line.contains("Ironmoon"), "{}", line);
        let qi = a.quests.len() - 1;
        assert_eq!(a.quests[qi].kind, QuestKind::Recover);
        // The relic in hand (as if from the fort).
        let v = a.visit.as_mut().unwrap();
        let p0 = v.g.dwarves[v.hero].pos;
        let axe = v.g.spawn_item(crate::items::ItemKind::Axe, crate::material::Mat::Steel, p0);
        let it = v.g.items.get_mut(&axe).unwrap();
        it.label = Some("Ironmoon, a steel axe".into());
        it.quality = ARTIFACT;
        it.carried_by = Some(HERO_ID);
        // Away from the giver's town: won, not paid.
        let key = v.key;
        v.key = SiteKey::Wild(0, 0);
        a.check_quests();
        assert_eq!(a.quests[qi].state, QuestState::Won);
        assert!(a.quest_lines()[0].2.contains("Back to"), "{:?}", a.quest_lines());
        // Home: the relic handed over, and paid.
        a.visit.as_mut().unwrap().key = key;
        let coins = a.hero.coins;
        a.check_quests();
        assert_eq!(a.quests[qi].state, QuestState::Done);
        assert!(a.hero.coins > coins);
        assert!(!a.carries("Ironmoon, a steel axe"), "given to the giver");
        assert!(a.fame.deeds.iter().any(|d| d.text.contains("brought Ironmoon")));
    }

    #[test]
    fn avenge_the_fort() {
        let mut a = town(43);
        let fort = fallen_kin(&mut a);
        let p = elder(&a);
        assert!(a.offers(&p).iter().any(|o| o.kind == QuestKind::Avenge && o.key == SiteKey::Fort(fort)), "kin are asked to avenge their own");
        a.seek_work(SiteKey::Fort(fort)).unwrap();
        let qi = a.quests.len() - 1;
        assert_eq!(a.quests[qi].kind, QuestKind::Avenge);
        assert!(a.quests[qi].plea.contains("Copperhold") && a.quests[qi].plea.to_lowercase().contains("goblin"), "{}", a.quests[qi].plea);
        assert!(a.hero.known.contains(&(KNOWN_FORT | fort)));
        a.check_quests();
        assert_eq!(a.quests[qi].state, QuestState::Open, "not yet");
        // Its killers driven out (as the fort's mark says once they are).
        a.ow_mut().forts[fort as usize].occupant = Some(Occupant::Empty);
        a.check_quests();
        assert_eq!(a.quests[qi].state, QuestState::Done, "won, and paid in the giver's town");
        assert!(a.ow().events.iter().any(|e| e.text.contains("avenged the fall of Copperhold")));
        // A hero who dies sworn takes the oath into the legends.
        let mut b = town(44);
        let f2 = fallen_kin(&mut b);
        b.seek_work(SiteKey::Fort(f2)).unwrap();
        let lines = b.oaths_at_death();
        assert!(lines.iter().any(|l| l.contains("unavenged")), "{:?}", lines);
        assert_eq!(b.quests.last().unwrap().state, QuestState::Failed);
        assert!(b.ow().events.iter().any(|e| e.text.contains("unavenged")));
    }
}
