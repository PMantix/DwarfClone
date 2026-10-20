//! The overseer's journal (O07, OVR-06): its own account of the fort, a
//! page for each year of each reign, in the first person.
//!
//! - *Jottings*: every note it makes goes in the page as it is made
//!   (`jot`, from `note`), turned to "I" ("Uneasy, I counted heads").
//! - *Entries*: once a day (`ov_journal`) it writes up what the fort
//!   remembers since (`memory.rs`): the dead by name, each with a word of its
//!   own (folded in from its mourning note when it made one), births and
//!   weddings, sieges, artifacts, madness. At each season's turn it takes
//!   stock: how many they are, what it fears, what it is proud of.
//! - *Its secrets*: what changed it (`drift`) and the truth behind a siege it
//!   spun (`spin_siege`), which nobody else gets to read.
//! - *The year's end*: when a page closes it sums the year up: who it
//!   buried, its pride, its fears, the fort's faith in it. A page cut short
//!   by its fall ends where its hand ends.
//!
//! The pages are kept with the save (no cap: a few kilobytes a year). Which
//! words it picks come from hashes, never the game's random numbers.

use super::overseer::Overseer;
use super::personality::Trait;
use super::*;
use serde::{Deserialize, Serialize};

/// What an entry is, for how the page shows it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryKind {
    /// One of its notes, made as it went.
    #[default]
    Jot,
    /// Something it wrote up.
    Entry,
    /// A death.
    Dead,
    /// Taking stock at the turn of a season.
    Season,
    /// Something to be proud of.
    Pride,
    /// Something to fear.
    Fear,
    /// What changed it.
    Changed,
    /// What it tells nobody: the truth behind a spun siege.
    Secret,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub at: u64,
    pub text: String,
    #[serde(default)]
    pub kind: EntryKind,
}

/// A page of the journal: one year of one overseer's reign.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct JournalPage {
    /// When the page was begun.
    pub tick: u64,
    /// The year's last word, written when the page closes.
    pub lines: Vec<String>,
    /// Whose hand: its name, traits and which overseer of the fort it was.
    #[serde(default)]
    pub by: String,
    #[serde(default)]
    pub traits: Vec<Trait>,
    #[serde(default)]
    pub nth: usize,
    #[serde(default)]
    pub entries: Vec<Entry>,
    #[serde(default)]
    pub closed: bool,
    /// The fort's memory is written up to here.
    #[serde(default)]
    pub upto: u64,
    /// The last season it took stock of (`tick / SEASON + 1`; 0: none).
    #[serde(default)]
    pub season: u64,
    /// The fort's worth when it last took stock.
    #[serde(default)]
    pub wealth: u32,
}

impl JournalPage {
    /// Its year, counted from the embark (0 the first).
    pub fn year_index(&self) -> u64 {
        self.tick / YEAR
    }

    /// "Urist the Cautious".
    pub fn styled(&self) -> String {
        let first = super::stories::first_name(&self.by).to_string();
        match self.traits.first() {
            Some(t) => format!("{} the {}", first, cap(t.name())),
            None => first,
        }
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_lowercase().collect::<String>() + c.as_str())
}

/// "a, b and c".
fn list(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        n => format!("{} and {}", items[..n - 1].join(", "), items[n - 1]),
    }
}

/// One of `lines`, by a steady hash of `parts`.
fn one_of(lines: &[&str], parts: &[u64]) -> String {
    lines[(super::names::hash(parts) % lines.len() as u64) as usize].to_string()
}

/// A note in the first person: "Uneasy, it counted heads" → "Uneasy, I
/// counted heads"; "It means to..." → "I mean to...". A note with no
/// subject ("Dug out a tavern.") reads as a diary's jotting already.
pub fn first_person(text: &str) -> String {
    let at = if text.starts_with("It ") {
        Some(0)
    } else {
        text.find(", it ").filter(|&i| !text[..i].contains(['.', ':', ';'])).map(|i| i + 2)
    };
    let Some(at) = at else { return text.to_string() };
    let rest = &text[at + 3..];
    let (word, tail) = rest.split_once(' ').unwrap_or((rest, ""));
    let verb = match word {
        "is" => "am",
        "has" => "have",
        "does" => "do",
        "goes" => "go",
        w if w.len() > 3 && w.ends_with('s') && !w.ends_with("ss") && !w.ends_with("us") && ["means", "wants", "needs", "thinks", "fears", "hopes", "plans", "waits", "keeps", "sees", "knows", "likes", "trusts"].contains(&w) => &w[..w.len() - 1],
        w => w,
    };
    let rest = if tail.is_empty() { verb.to_string() } else { format!("{} {}", verb, tail) };
    let rest = rest.replace(" its ", " my ").replace("itself", "myself");
    format!("{}I {}", &text[..at], rest)
}

impl Game {
    /// The page being written: this year's, in this overseer's hand. A new
    /// year or a new overseer begins a new page.
    fn journal_page(o: &mut Overseer, tick: u64) -> &mut JournalPage {
        // A name no reign was under (before it became a dwarf of the fort, or
        // re-rolled): the same hand, the same page.
        let (name, nth) = (o.name.clone(), o.reigns.len().max(1));
        if let Some(p) = o.journal.last_mut().filter(|p| p.year_index() == tick / YEAR && p.by != name && !p.closed) {
            if !o.reigns.iter().any(|r| r.name == p.by) {
                p.by = name;
                p.nth = nth;
            }
        }
        let fresh = o.journal.last().map_or(true, |p| p.year_index() != tick / YEAR || p.by != o.name);
        if fresh {
            let (upto, season, wealth) = o.journal.last().map_or((0, 0, 0), |p| (p.upto, p.season, p.wealth));
            let page = JournalPage { tick, by: o.name.clone(), traits: o.traits.clone(), nth: o.reigns.len().max(1), upto, season, wealth, ..Default::default() };
            o.journal.push(page);
        }
        let p = o.journal.last_mut().unwrap();
        // Its traits as they are now (they may drift).
        if p.traits != o.traits && !o.traits.is_empty() {
            p.traits = o.traits.clone();
        }
        p
    }

    #[cfg(test)]
    pub fn journal_for_test(o: &mut Overseer, tick: u64, text: &str) {
        Self::journal(o, tick, EntryKind::Entry, text.to_string());
    }

    /// Write in the journal.
    pub(super) fn journal(o: &mut Overseer, tick: u64, kind: EntryKind, text: String) {
        Self::journal_page(o, tick).entries.push(Entry { at: tick, text, kind });
    }

    /// `note`: every note goes in the page too, in the first person.
    pub(super) fn jot(o: &mut Overseer, tick: u64, text: &str) {
        if o.name.is_empty() {
            return;
        }
        Self::journal(o, tick, EntryKind::Jot, first_person(text));
    }

    /// Take back the jotting just made (when a fuller entry says it).
    pub(super) fn unjot(o: &mut Overseer, text: &str) {
        let mine = first_person(text);
        if let Some(p) = o.journal.last_mut() {
            if p.entries.last().map_or(false, |e| e.kind == EntryKind::Jot && e.text == mine) {
                p.entries.pop();
            }
        }
    }

    /// Daily (`ov_daily_hooks`): write up the days since, take stock at a
    /// season's turn, and close the pages of years gone by.
    pub(super) fn ov_journal(&mut self, o: &mut Overseer) {
        if o.dwarf.is_none() || o.name.is_empty() {
            return;
        }
        let tick = self.tick;
        Self::journal_page(o, tick);
        let n = o.journal.len();
        for i in 0..n - 1 {
            if !o.journal[i].closed {
                self.close_page(o, i);
            }
        }
        self.journal_memories(o);
        let season = tick / SEASON + 1;
        if o.journal.last().map_or(false, |p| p.season < season) {
            self.journal_season(o);
        }
    }

    /// The fort's memories since the page last looked, written up.
    fn journal_memories(&mut self, o: &mut Overseer) {
        let tick = self.tick;
        let upto = o.journal.last().map_or(0, |p| p.upto);
        let mems: Vec<super::memory::Memory> = self.memory_since(upto).filter(|m| m.tick < tick).cloned().collect();
        for m in mems {
            if let Some((kind, text)) = self.write_up(o, &m) {
                Self::journal(o, tick, kind, text);
            }
        }
        if let Some(p) = o.journal.last_mut() {
            p.upto = tick;
        }
    }

    /// One memory as the overseer tells it, if it is worth the ink.
    fn write_up(&self, o: &mut Overseer, m: &super::memory::Memory) -> Option<(EntryKind, String)> {
        use super::memory::MemoryKind as K;
        let fort = &self.fortress_name;
        let h = [m.tick, m.id as u64];
        // Which of its traits speaks.
        let t = o.traits.get((super::names::hash(&h) % o.traits.len().max(1) as u64) as usize).copied();
        let me = o.name.clone();
        let mut text = m.text.trim_end_matches('.').to_string();
        if !me.is_empty() && text.starts_with(&me) {
            text = format!("I{}", &text[me.len()..]);
        }
        let fact = format!("{}.", past(&text));
        Some(match m.kind {
            K::Died => {
                let id = *m.who.first()?;
                let d = self.dwarves.iter().find(|d| d.id == id)?;
                if Some(id) == o.dwarf {
                    return None;
                }
                let first = super::stories::first_name(&d.name).to_string();
                // Its mourning note, if it made one, says it best.
                let mut said = None;
                if let Some(p) = o.journal.last_mut() {
                    if let Some(i) = p.entries.iter().rposition(|e| e.kind == EntryKind::Jot && e.at + DAY >= m.tick && e.text.contains(first.as_str())) {
                        said = Some(p.entries.remove(i).text);
                    }
                }
                let dear = o.dwarf.and_then(|me| self.dwarves.iter().find(|x| x.id == me)).map_or(false, |me| {
                    me.spouse == Some(id) || me.parents.map_or(false, |(a, b)| a == id || b == id) || d.parents.map_or(false, |(a, b)| a == me.id || b == me.id) || me.regard(id) >= 40
                });
                let coda = match (said, t) {
                    (Some(s), _) => s,
                    (None, Some(Trait::Sentimental)) => one_of(&["I sat by the body until the lamps burned low.", "I have written the name here so that I will not forget it."], &h),
                    (None, Some(Trait::Pious)) => "I asked the gods to take them kindly.".into(),
                    (None, Some(Trait::Greedy)) => "One fewer pair of hands, and the work does not wait.".into(),
                    (None, Some(Trait::Bold)) => "I hope they went down fighting.".into(),
                    (None, Some(Trait::Cautious)) => "It should not have happened. I keep counting heads.".into(),
                    (None, Some(Trait::Cheerful)) => "We drank to them tonight, though nobody felt like singing.".into(),
                    (None, Some(Trait::Vain)) => "They shall have a fine tomb. I will see to it.".into(),
                    (None, Some(Trait::Stubborn)) => "The work goes on. It has to.".into(),
                    (None, None) => String::new(),
                };
                let dear = if dear { format!(" {} was dear to me.", first) } else { String::new() };
                (EntryKind::Dead, format!("{}{} {}", fact, dear, coda).trim_end().to_string())
            }
            K::Born => {
                let coda = match t {
                    Some(Trait::Sentimental) | Some(Trait::Cheerful) => "I held the little one, and it pulled my beard.",
                    Some(Trait::Greedy) => "Another mouth to feed.",
                    Some(Trait::Pious) => "A blessing on the cradle.",
                    _ => "Another dwarf of the fort.",
                };
                (EntryKind::Entry, format!("{} {}", fact, coda))
            }
            K::Married => {
                let coda = match t {
                    Some(Trait::Cheerful) => "I danced at the wedding, badly.",
                    Some(Trait::Greedy) => "The feast cost more than I like to think about.",
                    _ => "I wished them well.",
                };
                (EntryKind::Entry, format!("{} {}", fact, coda))
            }
            K::Artifact => {
                let coda = match t {
                    Some(Trait::Vain) => "They will talk of it in the hillhalls, and of the overseer who made it possible.",
                    Some(Trait::Greedy) => "I have already reckoned what it is worth.",
                    Some(Trait::Pious) => "The gods guided their hands.",
                    _ => "Something for the fort to be proud of.",
                };
                (EntryKind::Pride, format!("{} {}", fact, coda))
            }
            K::Monument => (EntryKind::Pride, format!("{} {} will stand when we are all dust.", fact, fort)),
            K::Epithet | K::Feat if m.weight >= 5 => (EntryKind::Pride, fact),
            K::Siege => {
                let coda = match t {
                    Some(Trait::Cautious) => "I had the doors barred. I hope it is enough.",
                    Some(Trait::Bold) => "Good. Let them come.",
                    Some(Trait::Pious) => "Gods, keep us.",
                    Some(Trait::Sentimental) => "I counted the children twice.",
                    _ => "May the gate hold.",
                };
                (EntryKind::Fear, format!("{} {}", fact, coda))
            }
            K::SiegeEnd => {
                // Its note on the siege, if it made one.
                let mut said = String::new();
                if let Some(p) = o.journal.last_mut() {
                    if let Some(i) = p.entries.iter().rposition(|e| e.kind == EntryKind::Jot && e.at + DAY >= m.tick && e.at >= m.tick) {
                        if p.entries[i].text.contains("siege") || p.entries[i].text.contains("gate") || p.entries[i].text.contains(" gone") {
                            said = format!(" {}", p.entries.remove(i).text);
                        }
                    }
                }
                (EntryKind::Entry, format!("{}{}", fact, said))
            }
            K::Madness => (EntryKind::Fear, format!("{} {}", fact, if t == Some(Trait::Sentimental) { "I should have seen it coming." } else { "I did not see it coming." })),
            K::Appointed | K::Expedition | K::Named | K::Inspired | K::Mood if m.weight >= 3 => (EntryKind::Entry, fact),
            K::Overseer | K::Arrived | K::Wounded | K::Soldier | K::Recovered | K::Other => return None,
            _ if m.weight >= 6 => (EntryKind::Entry, fact),
            _ => return None,
        })
    }

    /// At a season's turn: how many they are, what it fears, what it is
    /// proud of, in its voice.
    fn journal_season(&mut self, o: &mut Overseer) {
        let tick = self.tick;
        let season = super::SEASONS[((tick % YEAR) / SEASON) as usize % 4];
        let year = self.start_year as u64 + tick / YEAR;
        let (n, w) = (self.alive_count(), self.wealth());
        let was = o.journal.last().map_or(0, |p| p.wealth);
        let opening = match season {
            "Spring" => "The snow is off the slopes.",
            "Summer" => "The days are long and the forges hot.",
            "Autumn" => "The leaves are turning on the slopes.",
            _ => "The passes will close soon.",
        };
        let since = tick.saturating_sub(SEASON);
        let mut text = format!("{} of {}. {} We are {} souls, and the fort is worth \u{263c}{}.", season, year, opening, n, w);
        let h = [tick, 0x5ea];
        if let Some(f) = self.fears(o, since).first() {
            text.push(' ');
            text.push_str(&fear_line(o.traits.first().copied(), f, &h));
        }
        let mut pride = self.prides(o, since);
        if was > 0 && w > was + was / 5 {
            pride.push(format!("the fort grown richer by \u{263c}{}", w - was));
        }
        if let Some(p) = pride.first() {
            text.push(' ');
            text.push_str(&pride_line(o.traits.get(1).or(o.traits.first()).copied(), p, &h));
        }
        Self::journal(o, tick, EntryKind::Season, text);
        if let Some(p) = o.journal.last_mut() {
            p.season = tick / SEASON + 1;
            p.wealth = w;
        }
    }

    /// What it fears now, the worst first.
    fn fears(&self, o: &Overseer, since: u64) -> Vec<String> {
        let mut v = Vec::new();
        let grumble = self.knob("faith.grumble", 40.0) as i32;
        if self.annals.sieges.iter().any(|s| s.ended.is_none()) {
            v.push("the army at our gate".to_string());
        }
        let hungry: Vec<String> = o.worries.iter().filter(|w| matches!(w.lack, super::troubles::Lack::Food | super::troubles::Lack::Drink)).map(|w| w.lack.what()).collect();
        if !hungry.is_empty() {
            v.push(format!("the stores running out of {}", list(&hungry)));
        }
        if o.faith < grumble {
            v.push("the muttering about me in the halls".to_string());
        }
        let dead = self.annals.deaths.iter().filter(|d| d.tick >= since).count();
        if dead >= 2 {
            v.push(format!("more graves: we dug {} of them", dead));
        }
        if let Some(s) = self.annals.sieges.iter().rev().find(|s| s.ended.map_or(false, |e| e >= since) && s.dwarves_lost > 0) {
            v.push(format!("the {}, who will be back", s.foe));
        }
        let short: Vec<String> = o.worries.iter().filter(|w| !matches!(w.lack, super::troubles::Lack::Food | super::troubles::Lack::Drink)).take(2).map(|w| w.lack.what()).collect();
        if !short.is_empty() {
            v.push(format!("how short we are of {}", list(&short)));
        }
        v.push(
            match o.traits.first() {
                Some(Trait::Cautious) => "whatever I have not thought of",
                Some(Trait::Bold) => "growing old in a quiet year",
                Some(Trait::Cheerful) => "the ale running dry",
                Some(Trait::Greedy) => "a bad trade",
                Some(Trait::Pious) => "the silence of the gods",
                Some(Trait::Vain) => "being forgotten",
                Some(Trait::Sentimental) => "losing another of them",
                Some(Trait::Stubborn) => "being told I was wrong",
                None => "the dark under the mountain",
            }
            .to_string(),
        );
        v
    }

    /// What it is proud of since `since`, the greatest first.
    fn prides(&self, _o: &Overseer, since: u64) -> Vec<String> {
        use super::memory::MemoryKind as K;
        let mut v = Vec::new();
        let mems: Vec<&super::memory::Memory> = self.memory_since(since).collect();
        if let Some(m) = mems.iter().find(|m| m.kind == K::Monument) {
            v.push(lower_first(m.text.trim_end_matches('.')));
        }
        for s in self.annals.sieges.iter().filter(|s| s.ended.map_or(false, |e| e >= since) && s.outcome == "routed") {
            v.push(format!("the {} routed at our gate", s.foe));
        }
        let arts = mems.iter().filter(|m| m.kind == K::Artifact).count();
        if arts > 0 {
            v.push(if arts == 1 { "an artifact made in our workshops".to_string() } else { format!("{} artifacts made in our workshops", arts) });
        }
        let births = mems.iter().filter(|m| m.kind == K::Born).count();
        if births > 0 {
            v.push(if births == 1 { "a child born".to_string() } else { format!("{} children born", births) });
        }
        if let Some(m) = mems.iter().find(|m| m.kind == K::Epithet) {
            v.push(lower_first(m.text.trim_end_matches('.')));
        }
        if mems.iter().any(|m| m.kind == K::Married) {
            v.push("a wedding in the hall".to_string());
        }
        v
    }

    /// The page's last word: the year summed up, or where the hand ends.
    fn close_page(&mut self, o: &mut Overseer, i: usize) {
        let page = o.journal[i].clone();
        let end = o.journal.get(i + 1).map_or(self.tick, |p| p.tick);
        let first = super::stories::first_name(&page.by).to_string();
        let mut lines = Vec::new();
        let dead: Vec<String> = self.annals.deaths.iter().filter(|d| d.tick >= page.tick && d.tick < end && !d.name.is_empty() && d.name != page.by).map(|d| d.name.clone()).collect();
        let reign = o.reigns.iter().find(|r| r.name == page.by && r.to.map_or(false, |t| t >= page.tick && t <= end));
        let h = [page.tick, end, 0xc105e];
        if let Some(r) = reign {
            // The hand changes.
            let how = match &r.end {
                Some(super::reign::ReignEnd::Died(c)) => format!("{} {}.", first, c),
                Some(super::reign::ReignEnd::Mad) => format!("{} went mad.", first),
                Some(super::reign::ReignEnd::Lost) => format!("{} was lost.", first),
                Some(super::reign::ReignEnd::Deposed) => format!("The fort put {} aside.", first),
                Some(super::reign::ReignEnd::Elected) => format!("{} lost the mayor's election.", first),
                None => format!("{} set the ledger down.", first),
            };
            lines.push(format!("Here the hand of {} ends. {}", page.styled(), how));
            if !dead.is_empty() {
                lines.push(format!("In these pages {} remembered {}.", first, list(&dead)));
            }
        } else {
            let year = self.start_year as u64 + page.year_index();
            lines.push(format!("So ends the year {}.", year));
            lines.push(match (dead.len(), page.traits.first()) {
                (0, _) => "Nobody died this year. I hardly dare write it down.".to_string(),
                (_, Some(Trait::Sentimental)) => format!("I buried {}. I have set their names down here, so that someone remembers.", list(&dead)),
                (_, Some(Trait::Greedy)) => format!("We lost {}: {} pairs of hands.", list(&dead), dead.len()),
                (_, Some(Trait::Stubborn)) => format!("We buried {}, and kept on.", list(&dead)),
                _ => format!("We buried {}.", list(&dead)),
            });
            let fears = self.fears(o, page.tick);
            let prides = self.prides(o, page.tick);
            if let Some(p) = prides.first() {
                let _ = p;
                lines.push(pride_line(page.traits.first().copied(), &list(&prides.iter().take(3).cloned().collect::<Vec<_>>()), &h));
            }
            if let Some(f) = fears.first() {
                lines.push(fear_line(page.traits.get(1).or(page.traits.first()).copied(), f, &h));
            }
            let mood = match o.faith {
                f if f >= 70 => "They would follow me anywhere, I think.",
                f if f >= 50 => "They trust me, for now.",
                f if f >= 25 => "They grumble when they think I cannot hear.",
                _ => "I do not think they will suffer me much longer.",
            };
            lines.push(format!("The fort's faith in me stands at {}. {}", o.faith, mood));
        }
        o.journal[i].lines = lines;
        o.journal[i].closed = true;
    }

    /// The last page of a fallen fort's journal, for the world to find.
    pub fn journal_last_page(&self) -> Vec<String> {
        let Some(p) = self.overseer.as_ref().and_then(|o| o.journal.last()) else { return Vec::new() };
        let mut v = vec![format!("The journal of {}, overseer of {}", p.styled(), self.fortress_name)];
        let written: Vec<String> = p.entries.iter().filter(|e| e.kind != EntryKind::Jot).map(|e| e.text.clone()).collect();
        v.extend(written[written.len().saturating_sub(6)..].iter().cloned());
        v
    }
}

/// "has been struck down" → "was struck down".
fn past(s: &str) -> String {
    s.replace(" has been ", " was ").replace(" has ", " ")
}

/// What it fears, as its trait would put it.
fn fear_line(t: Option<Trait>, f: &str, h: &[u64]) -> String {
    match t {
        Some(Trait::Cautious) => format!("I lie awake over {}.", f),
        Some(Trait::Bold) => format!("If anything gnaws at me, it is {}.", f),
        Some(Trait::Vain) => format!("I would not say so to them, but I worry about {}.", f),
        Some(Trait::Pious) => format!("I have prayed about {}.", f),
        Some(Trait::Greedy) => format!("What worries me is {}: it will cost us.", f),
        Some(Trait::Sentimental) => format!("I worry for them: {}.", f),
        Some(Trait::Stubborn) => format!("They tell me to worry about {}. I will decide that.", f),
        Some(Trait::Cheerful) => format!("Even I cannot laugh off {}.", f),
        None => one_of(&["What I fear is ", "I fear "], h) + f + ".",
    }
}

/// What it is proud of, as its trait would put it.
fn pride_line(t: Option<Trait>, p: &str, _h: &[u64]) -> String {
    match t {
        Some(Trait::Vain) => format!("Let it be written that under me came {}.", p),
        Some(Trait::Cheerful) => format!("And yet: {}! We drank to it.", p),
        Some(Trait::Greedy) => format!("In the ledger's favour: {}.", p),
        Some(Trait::Pious) => format!("The gods gave us {}.", p),
        Some(Trait::Sentimental) => format!("I am glad of {}.", p),
        Some(Trait::Cautious) => format!("Some good news, at least: {}.", p),
        Some(Trait::Bold) => format!("{}. Not bad.", cap(p)),
        Some(Trait::Stubborn) => format!("{}, just as I planned.", cap(p)),
        None => format!("I am proud of {}.", p),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::overseer::Style;

    #[test]
    fn notes_turn_to_the_first_person() {
        assert_eq!(first_person("Uneasy, it counted heads again."), "Uneasy, I counted heads again.");
        assert_eq!(first_person("It means to lay the fortress out as a grid."), "I mean to lay the fortress out as a grid.");
        assert_eq!(first_person("It had the tale told, with itself at the heart of it."), "I had the tale told, with myself at the heart of it.");
        assert_eq!(first_person("Dug out a tavern."), "Dug out a tavern.");
        assert_eq!(first_person("Called for more blocks, and nobody stirred to make it."), "Called for more blocks, and nobody stirred to make it.");
        assert_eq!(first_person("The massacre broke its nerve: no longer bold, it is cautious now."), "The massacre broke its nerve: no longer bold, it is cautious now.");
    }

    #[test]
    fn every_note_goes_in_the_years_page() {
        let mut g = Game::new(5);
        g.creatures.retain(|_, c| c.tame);
        g.set_overseer(Some(Style::Steady));
        g.overseer.as_mut().unwrap().traits = vec![Trait::Sentimental, Trait::Vain];
        let mut o = g.overseer.take().unwrap();
        g.tick = DAY;
        Game::note_ov(&mut o, g.tick, "Dug out a tavern.".into());
        Game::note_ov(&mut o, g.tick, "It means to dig a great hall.".into());
        let page = o.journal.last().unwrap();
        assert!(page.entries.iter().any(|e| e.text == "Dug out a tavern."), "{:?}", page.entries);
        assert!(page.entries.iter().any(|e| e.text == "I mean to dig a great hall."));
        assert_eq!(page.by, o.name);
        // A death is written up by name, with its mourning folded in.
        g.overseer = Some(o);
        let i = g.dwarves.iter().position(|d| Some(d.id) != g.overseer.as_ref().unwrap().dwarf && !d.is_child()).unwrap();
        let mut d = std::mem::take(&mut g.dwarves[i]);
        g.kill_for_test(&mut d, "has been struck down");
        let name = d.name.clone();
        g.dwarves[i] = d;
        let mut o = g.overseer.take().unwrap();
        g.tick += DAY;
        g.ov_reflect(&mut o);
        g.ov_journal(&mut o);
        let page = o.journal.last().unwrap();
        let dead: Vec<&Entry> = page.entries.iter().filter(|e| e.kind == EntryKind::Dead).collect();
        assert_eq!(dead.len(), 1, "{:?}", page.entries);
        assert!(dead[0].text.contains(&name) && dead[0].text.contains("was struck down"), "{}", dead[0].text);
        assert!(!page.entries.iter().any(|e| e.kind == EntryKind::Jot && e.text.contains(super::super::stories::first_name(&name))), "the mourning note is folded in: {:?}", page.entries);
        assert!(page.entries.iter().any(|e| e.kind == EntryKind::Season), "it takes stock: {:?}", page.entries);
        // A new year, a new page; the old one is summed up.
        g.tick = YEAR + DAY;
        Game::note_ov(&mut o, g.tick, "Dug out a well.".into());
        assert_eq!(o.journal.len(), 2);
        g.ov_journal(&mut o);
        let old = &o.journal[0];
        assert!(old.closed);
        assert!(old.lines[0].starts_with("So ends the year"), "{:?}", old.lines);
        assert!(old.lines.iter().any(|l| l.contains(&name)), "the dead are remembered: {:?}", old.lines);
        assert!(old.lines.iter().any(|l| l.contains("faith in me")));
        // Saved and loaded, the journal is the same.
        let back: Overseer = serde_json::from_str(&serde_json::to_string(&o).unwrap()).unwrap();
        assert_eq!(back.journal, o.journal);
    }

    #[test]
    fn a_page_ends_where_the_overseer_falls() {
        let mut g = Game::new(5);
        g.creatures.retain(|_, c| c.tame);
        g.set_overseer(Some(Style::Steady));
        let mut o = g.overseer.take().unwrap();
        g.tick = 2 * DAY;
        g.ov_reign(&mut o);
        g.ov_journal(&mut o);
        let first = o.name.clone();
        g.succeed(&mut o, super::super::reign::ReignEnd::Mad, None);
        g.tick += DAY;
        g.ov_journal(&mut o);
        assert_eq!(o.journal.len(), 2, "{:?}", o.journal.iter().map(|p| &p.by).collect::<Vec<_>>());
        assert_eq!(o.journal[0].by, first);
        assert!(o.journal[0].lines[0].starts_with("Here the hand of"), "{:?}", o.journal[0].lines);
        assert!(o.journal[0].lines[0].contains("went mad"));
        assert_ne!(o.journal[1].by, first);
        assert!(o.journal[1].entries.iter().any(|e| e.text.contains("took up the ledger")), "{:?}", o.journal[1].entries);
    }
}
