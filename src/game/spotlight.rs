//! The big moments: a siege at the gates, a dragon, a hero struck down,
//! traders at the depot. Each pauses the game (unless the overseer is
//! playing), turns the camera to where it happened and tells the story in
//! a card over the map.

use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Spotlight {
    pub title: String,
    pub text: String,
    /// Where to look, if anywhere in particular.
    pub at: Option<Pos>,
    pub color: (u8, u8, u8),
}

impl Game {
    /// Stop and look: something that matters has happened.
    pub(super) fn spotlight(&mut self, title: &str, text: &str, at: Option<Pos>, color: (u8, u8, u8)) {
        // A flurry of the same kind (a dozen undead rising) is one moment.
        if self.spotlights.iter().any(|s| s.title == title) {
            return;
        }
        self.spotlights.push(Spotlight { title: title.into(), text: text.into(), at, color });
        self.pause_request = true;
    }

    #[cfg(test)]
    pub fn spotlight_for_test(&mut self, title: &str, text: &str) {
        self.spotlight(title, text, None, (255, 255, 255));
    }

    #[cfg(test)]
    pub fn kill_for_test(&mut self, d: &mut Dwarf, cause: &str) {
        self.kill(d, cause);
    }

    /// The next moment to show, oldest first.
    pub fn take_spotlight(&mut self) -> Option<Spotlight> {
        if self.spotlights.is_empty() {
            None
        } else {
            Some(self.spotlights.remove(0))
        }
    }
}
