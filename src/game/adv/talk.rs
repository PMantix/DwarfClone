//! Talk, news and rumours (A7).

use super::Adventure;

/// One thing heard: its words, and what it is about.
#[derive(Clone, Debug, PartialEq)]
pub struct Rumour {
    pub text: String,
    pub weight: i32,
}

impl Adventure {
    /// What a speaker at a cell has heard (A7).
    pub fn rumours(&self, _at: (i32, i32)) -> Vec<Rumour> {
        Vec::new()
    }
}
