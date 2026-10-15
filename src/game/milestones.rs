//! Milestones: the fortress's firsts and great numbers, each greeted with
//! a banner across the screen and kept in a list.

use super::*;

/// How long a banner stays up, in ticks.
pub const BANNER_TICKS: u64 = 400;

impl Game {
    /// Mark a milestone the first time it's reached.
    pub(super) fn milestone(&mut self, key: &str, text: &str) {
        if self.milestones.iter().any(|(k, _, _)| k == key) {
            return;
        }
        self.milestones.push((key.to_string(), text.to_string(), self.tick));
        self.banner = Some((text.to_string(), self.tick));
        self.msg(&format!("☼ Milestone: {} ☼", text), (255, 215, 60));
    }

    /// Daily: the milestones that are a matter of counting.
    pub(super) fn milestones_daily(&mut self) {
        let pop = self.alive_count();
        for (n, text) in [(20, "Twenty dwarves call the fortress home"), (50, "Fifty dwarves call the fortress home"), (100, "A hundred dwarves call the fortress home")] {
            if pop >= n {
                self.milestone(&format!("pop{}", n), text);
            }
        }
        let wealth = self.wealth();
        for (w, text) in [(10_000, "The fortress is worth ☼10,000"), (50_000, "The fortress is worth ☼50,000"), (100_000, "The fortress is worth ☼100,000"), (250_000, "The fortress is worth a quarter of a million")] {
            if wealth >= w {
                self.milestone(&format!("wealth{}", w), text);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn milestones_come_once_with_a_banner() {
        let mut g = Game::new(81);
        g.milestone("test", "Something grand");
        assert_eq!(g.milestones.len(), 1);
        assert_eq!(g.banner.as_ref().map(|b| b.0.as_str()), Some("Something grand"));
        g.milestone("test", "Something grand");
        assert_eq!(g.milestones.len(), 1, "only once");
        assert_eq!(g.log.iter().filter(|l| l.text.contains("Milestone")).count(), 1);
        // The wealth of a new fortress already counts for something.
        g.milestones_daily();
        let rich = g.wealth() >= 10_000;
        assert_eq!(g.milestones.iter().any(|m| m.0 == "wealth10000"), rich);
    }
}
