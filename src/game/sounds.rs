//! Noises worth hearing that can't be seen from outside the simulation:
//! each blow struck in a fight. The game only notes them (a few at most,
//! dropped when nobody listens); the app plays them (`src/audio`).

use super::*;

/// Most noises kept between frames.
pub const MAX_SOUNDS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    /// A blow that landed.
    Blow,
    /// A blow blocked or turned by armour.
    Parry,
}

impl Game {
    /// Note a blow at `at`: `landed`, or met (`met`) by shield or armour.
    pub(super) fn blow_sound(&mut self, landed: bool, met: bool, at: Pos) {
        let s = if landed {
            Sound::Blow
        } else if met {
            Sound::Parry
        } else {
            return;
        };
        if self.sounds.len() < MAX_SOUNDS {
            self.sounds.push((at, s));
        }
    }
}
