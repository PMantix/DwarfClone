//! Inspired engineers (O12; packages P25 and P26 fill this in): a dwarf's
//! inspiration becomes a blueprint, an overseer ambition and a finished
//! work bearing their name. Wave 0 gives the state's place in `Game`.

use serde::{Deserialize, Serialize};

/// One dwarf's idea for a great work.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Inspiration {
    pub dwarf: u32,
    pub tick: u64,
    /// "a clockwork guardian", "a hall of a hundred pillars".
    pub what: String,
    /// The finished work, once it stands.
    #[serde(default)]
    pub done: Option<u64>,
}

/// Every inspiration the fort has had (`Game.inspirations`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Inspirations {
    #[serde(default)]
    pub list: Vec<Inspiration>,
}
