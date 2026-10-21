//! Quests: slay, recover, clear, avenge (A8).

use super::Adventure;

impl Adventure {
    /// Check open quests after an act or on leaving a site (A8).
    pub fn check_quests(&mut self) {}

    /// "Is there work?" asked of `who` (A7's talk menu): the quest they
    /// offer, in their words. None: no work, and talk answers for them.
    pub fn quest_talk(&mut self, _who: &super::talk::Person) -> Option<Vec<String>> {
        None
    }
}
