//! What an announcement is about, and where it happened (UI-06/07
//! foundation): announcement control, sound-based reports, the director
//! and the export read `LogEntry.kind` and `LogEntry.at`. `Game::msg`
//! leaves both unset and `LogEntry::kind()` works the kind out from the
//! words; `Game::msg_at` sets them. (The reports screen's filters,
//! `ui.rs: report_kind`, are separate and unchanged.)

// Wave 0 interfaces for the wave-1 packages: some are not called yet.
#![allow(dead_code)]

use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LogKind {
    /// Not said: `LogEntry::kind()` classifies the text.
    #[default]
    Other,
    Death,
    Combat,
    Siege,
    Mood,
    Birth,
    Arrival,
    Building,
    Job,
    Trade,
    Weather,
    Overseer,
    Legend,
}

impl LogKind {
    pub const ALL: [LogKind; 13] = [
        LogKind::Other,
        LogKind::Death,
        LogKind::Combat,
        LogKind::Siege,
        LogKind::Mood,
        LogKind::Birth,
        LogKind::Arrival,
        LogKind::Building,
        LogKind::Job,
        LogKind::Trade,
        LogKind::Weather,
        LogKind::Overseer,
        LogKind::Legend,
    ];

    pub fn name(self) -> &'static str {
        match self {
            LogKind::Other => "Other",
            LogKind::Death => "Deaths",
            LogKind::Combat => "Combat",
            LogKind::Siege => "Sieges",
            LogKind::Mood => "Moods",
            LogKind::Birth => "Births and weddings",
            LogKind::Arrival => "Arrivals",
            LogKind::Building => "Buildings",
            LogKind::Job => "Jobs",
            LogKind::Trade => "Trade",
            LogKind::Weather => "Weather",
            LogKind::Overseer => "The overseer",
            LogKind::Legend => "Legends",
        }
    }

    /// The kind of an announcement from its words and colour.
    pub fn classify(text: &str, color: (u8, u8, u8)) -> LogKind {
        let has = |words: &[&str]| words.iter().any(|w| text.contains(w));
        if color == (255, 60, 60) || has(&["has died", "bled to death", "starved to death", "struck down", "has been slain", "fell on the expedition", "has drowned"]) {
            LogKind::Death
        } else if has(&["siege", "army", "besieg", "invaders", "drawbridge"]) {
            LogKind::Siege
        } else if has(&[" in the head", " in the upper body", " in the lower body", " in the left ", " in the right ", "blocks", "deflects", "misses", "attack", "flees", "goblin", "breathes fire", "cage trap", "duel"]) {
            LogKind::Combat
        } else if has(&["mood", "artifact", "berserk", "insane", "melancholy", "tantrum"]) {
            LogKind::Mood
        } else if has(&["born", "married", "wed ", "come of age"]) {
            LogKind::Birth
        } else if has(&["arrived", "migrant", "visitor", "joined", "stranger", "bard"]) {
            LogKind::Arrival
        } else if has(&["caravan", "trader", "trade", "depot", "merchant"]) {
            LogKind::Trade
        } else if has(&["has been completed", "waiting to be built", "built"]) {
            LogKind::Building
        } else if has(&["has made ", "Work order", "needs ", "It has started"]) {
            LogKind::Job
        } else if has(&["rain", "snow", "storm", "skies have", "drought", "blizzard", "thaw"]) {
            LogKind::Weather
        } else if has(&["overseer"]) {
            LogKind::Overseer
        } else if has(&["legend", "called", "known as", "chronicle"]) {
            LogKind::Legend
        } else {
            LogKind::Other
        }
    }
}

pub(crate) fn is_other(k: &LogKind) -> bool {
    *k == LogKind::Other
}

impl LogEntry {
    /// What it is about: as set by `msg_at`, else from its words.
    pub fn kind(&self) -> LogKind {
        match self.kind {
            LogKind::Other => LogKind::classify(&self.text, self.color),
            k => k,
        }
    }
}

impl Game {
    /// Announce something that happened at `at`, of a known kind.
    pub fn msg_at(&mut self, text: &str, color: (u8, u8, u8), at: Option<Pos>, kind: LogKind) {
        self.msg(text, color);
        if let Some(e) = self.log.last_mut() {
            e.at = at;
            e.kind = kind;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn announcements_know_their_kind_and_place() {
        let mut g = Game::new(5);
        g.msg("Urist McTest has been struck down.", (255, 255, 255));
        assert_eq!(g.log.last().unwrap().kind(), LogKind::Death);
        g.msg("A caravan from the mountainhomes has arrived.", (255, 255, 255));
        assert_eq!(g.log.last().unwrap().kind(), LogKind::Arrival);
        g.msg("Something quiet.", (255, 255, 255));
        assert_eq!((g.log.last().unwrap().kind(), g.log.last().unwrap().at), (LogKind::Other, None));
        let p = Pos::new(3, 4, 5);
        g.msg_at("The masons finished the wall.", (255, 255, 255), Some(p), LogKind::Building);
        let e = g.log.last().unwrap();
        assert_eq!((e.kind(), e.at), (LogKind::Building, Some(p)));
        // Saved, it comes back; and an entry saved without them loads.
        let back: LogEntry = serde_json::from_str(&serde_json::to_string(e).unwrap()).unwrap();
        assert_eq!((back.kind, back.at), (LogKind::Building, Some(p)));
        let old: LogEntry = serde_json::from_str(r#"{"text":"Hi","color":[1,2,3],"tick":4}"#).unwrap();
        assert_eq!((old.kind, old.at), (LogKind::Other, None));
    }
}
