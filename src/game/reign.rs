//! A mortal overseer (O07; packages P8 and P17 fill this in): the overseer
//! is a dwarf of the fort, trusted or doubted, and may fall and be
//! succeeded. Wave 0 gives the types its `Overseer` fields hold (`reigns`,
//! `journal`); `embody`, `ov_reign`, `succeed` and faith come with P8, the
//! journal with P17.

use serde::{Deserialize, Serialize};

/// How a reign ended.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReignEnd {
    /// Killed, with the cause.
    Died(String),
    Mad,
    Lost,
    Deposed,
    Elected,
}

/// One overseer's time in charge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reign {
    pub name: String,
    #[serde(default)]
    pub dwarf: Option<u32>,
    #[serde(default)]
    pub traits: Vec<super::personality::Trait>,
    pub from: u64,
    #[serde(default)]
    pub to: Option<u64>,
    #[serde(default)]
    pub end: Option<ReignEnd>,
}

/// A page of the overseer's journal (OVR-06), one a season.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JournalPage {
    pub tick: u64,
    pub lines: Vec<String>,
}

/// Faith in an overseer starts here (0..100).
pub fn sixty() -> i32 {
    60
}
