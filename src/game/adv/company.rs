//! Companions (A9): hired, they follow, fight, flee and may quit.
//! `Game::company_tick` (turn.rs) is their hook in `adv_step`.

use super::Adventure;

impl Adventure {
    /// Companions after a day on the road (A9).
    pub fn company_day(&mut self) {}

    /// "Will you come with me?" asked of `who` (A7's talk menu): their
    /// answer, and hiring them if they say yes. None: talk answers for them.
    pub fn company_talk(&mut self, _who: &super::talk::Person) -> Option<Vec<String>> {
        None
    }
}
