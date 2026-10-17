//! Breakdowns that shift and mend (S09: MND-06, MND-31). Madness is no
//! longer a one-way door: a raving dwarf's rage can burn out into grief
//! (and grief can lift), while grief that festers turns to raving.
//!
//! `insane` and `melancholy` keep their meaning: melancholy is
//! `insane && melancholy`, raving is `insane && !melancholy`. A `Fit` is
//! the breakdown's acute form on top of that.

use super::*;
use serde::{Deserialize, Serialize};

/// The acute form a breakdown takes, if any.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fit {
    #[default]
    None,
    /// A berserk rage (not madness: it passes, one way or another).
    Berserk { until: u64 },
    /// Lying where they fell, unreachable (they are `insane` meanwhile).
    /// `coaxed` counts today's sittings by a friend.
    Catatonic {
        since: u64,
        #[serde(default)]
        coaxed: u32,
    },
}

/// What the status line says of a broken dwarf (None for a whole one).
pub fn breakdown_mood(d: &Dwarf) -> Option<(&'static str, (u8, u8, u8))> {
    match d.fit {
        Fit::Berserk { .. } => Some(("Berserk", (255, 40, 40))),
        Fit::Catatonic { .. } => Some(("Catatonic", (140, 140, 210))),
        Fit::None if d.insane && d.melancholy => Some(("Stricken by melancholy", (150, 150, 255))),
        Fit::None if d.insane => Some(("Stark raving mad", (255, 110, 200))),
        Fit::None => None,
    }
}

impl Game {
    /// Someone who cares for them is still alive: a spouse, or a friend.
    pub(super) fn is_loved(&self, d: &Dwarf) -> bool {
        let alive = |id: u32| self.dwarves.iter().any(|o| o.id == id && o.alive);
        d.spouse.map_or(false, alive) || d.relations.iter().any(|&(o, r)| r >= 30 && alive(o))
    }

    /// Daily: madness shifts. Raving can burn out into melancholy (from which
    /// the old lifting applies), and melancholy that festers turns to raving.
    pub(super) fn shift_daily(&mut self) {
        let tick = self.tick;
        for i in 0..self.dwarves.len() {
            let d = &self.dwarves[i];
            if !d.alive || !d.insane || d.fit != Fit::None || tick < d.mad_since + 10 * DAY {
                continue;
            }
            if !d.melancholy {
                let p = self.knob("breakdown.ease", 0.06) * if self.is_loved(d) { 2.0 } else { 1.0 };
                if self.rng.chance(p) {
                    let d = &mut self.dwarves[i];
                    d.melancholy = true;
                    d.mad_since = tick;
                    let name = d.name.clone();
                    self.msg(&format!("{}'s rage has burned out into grief.", name), (170, 170, 255));
                    self.count("breakdown.eased", 1);
                }
            } else if d.stress >= 650 && self.rng.chance(self.knob("breakdown.worsen", 0.03)) {
                let d = &mut self.dwarves[i];
                d.melancholy = false;
                d.mad_since = tick;
                let name = d.name.clone();
                self.msg(&format!("{}'s grief has turned to raving madness.", name), (255, 110, 200));
                self.count("breakdown.worsened", 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A dwarf gone mad: raving, or melancholic.
    fn mad(g: &mut Game, i: usize, melancholy: bool) -> u32 {
        let mut d = std::mem::take(&mut g.dwarves[i]);
        let how = if melancholy { "has been stricken by melancholy" } else { "has gone stark raving mad, unable to finish their work" };
        g.go_insane(&mut d, how);
        let id = d.id;
        g.dwarves[i] = d;
        id
    }

    #[test]
    fn raving_eases_into_grief_and_grief_lifts() {
        let mut g = Game::new(11);
        g.creatures.retain(|_, c| c.tame);
        let id = mad(&mut g, 0, false);
        let spouse = g.dwarves[1].id;
        g.dwarves[0].spouse = Some(spouse);
        g.dwarves[1].spouse = Some(id);
        assert_eq!(breakdown_mood(&g.dwarves[0]).map(|m| m.0), Some("Stark raving mad"));
        for _ in 0..20 {
            g.tick += DAY;
            g.shift_daily();
            // The old lifting, as psyche_tick does it once a day.
            let mut d = std::mem::take(&mut g.dwarves[0]);
            let t = g.tick;
            g.tick = t - t % DAY + d.id as u64 % DAY;
            g.psyche_tick(&mut d);
            g.tick = t;
            g.dwarves[0] = d;
        }
        let d = &g.dwarves[0];
        assert!(!d.insane || d.melancholy, "still raving after 20 days with a living spouse");
        assert!(g.log.iter().any(|l| l.text.contains("rage has burned out into grief")));
        assert_eq!(g.annals.counters.get("breakdown.eased"), Some(&1));
    }

    #[test]
    fn festering_grief_turns_to_raving() {
        let mut g = Game::new(11);
        g.creatures.retain(|_, c| c.tame);
        g.knobs.set.insert("breakdown.worsen".into(), 1.0);
        mad(&mut g, 0, true);
        g.dwarves[0].stress = 700;
        g.tick += 9 * DAY;
        g.shift_daily();
        assert!(g.dwarves[0].melancholy, "too soon to turn");
        g.tick += DAY;
        g.shift_daily();
        assert!(g.dwarves[0].insane && !g.dwarves[0].melancholy, "the grief should have turned to raving");
        assert_eq!(breakdown_mood(&g.dwarves[0]).map(|m| m.0), Some("Stark raving mad"));
        // A calmer one stays melancholic.
        mad(&mut g, 1, true);
        g.dwarves[1].stress = 100;
        g.tick += 20 * DAY;
        g.shift_daily();
        assert!(g.dwarves[1].melancholy);
    }
}
