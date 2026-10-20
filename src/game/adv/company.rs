//! Companions (A9): hired, they follow, fight, flee and may quit.
//! `Game::company_tick` (turn.rs) is their hook in `adv_step`.

use super::Adventure;

impl Adventure {
    /// Companions after a day on the road (A9).
    pub fn company_day(&mut self) {}
}
