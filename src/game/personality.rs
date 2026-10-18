//! The overseer as someone: a name and two traits on top of its style.
//! The traits lean it toward some ambitions and away from others, sway its
//! layout, sharpen or blunt its eye for what the fortress lacks, and colour
//! what it writes in its notes.

use super::ambitions::Ambition;
use super::layout::Layout;
use super::overseer::Overseer;
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Trait {
    Cautious,
    Bold,
    Vain,
    Pious,
    Greedy,
    Sentimental,
    Stubborn,
    Cheerful,
}

impl Trait {
    pub const ALL: [Trait; 8] = [Trait::Cautious, Trait::Bold, Trait::Vain, Trait::Pious, Trait::Greedy, Trait::Sentimental, Trait::Stubborn, Trait::Cheerful];

    pub fn name(self) -> &'static str {
        match self {
            Trait::Cautious => "cautious",
            Trait::Bold => "bold",
            Trait::Vain => "vain",
            Trait::Pious => "pious",
            Trait::Greedy => "greedy",
            Trait::Sentimental => "sentimental",
            Trait::Stubborn => "stubborn",
            Trait::Cheerful => "cheerful",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Trait::Cautious => "walls and traps first; raises the bridge early; won't send the squad far",
            Trait::Bold => "goes looking for a fight; sends expeditions; slow to bar the gate",
            Trait::Vain => "grand halls, a monument, engraved walls; a showpiece of a layout",
            Trait::Pious => "temples and feasts",
            Trait::Greedy => "wealth, trade and the riches of the deep",
            Trait::Sentimental => "mourns the dead, buries them well, keeps its dwarves close",
            Trait::Stubborn => "sticks with what it set out to do; slow to take a hint",
            Trait::Cheerful => "taverns, festivals and full tankards",
        }
    }

    /// Pairs that can't both be true of one overseer.
    fn clashes(self, other: Trait) -> bool {
        matches!((self, other), (Trait::Cautious, Trait::Bold) | (Trait::Bold, Trait::Cautious))
    }

    /// How much it leans toward (or away from) an ambition.
    pub fn lean(self, a: Ambition) -> f64 {
        use Ambition::*;
        match (self, a) {
            (Trait::Cautious, Fortify) => 1.0,
            (Trait::Cautious, Muster) => 0.5,
            (Trait::Cautious, Venture) => -1.0,
            (Trait::Cautious, Delve) => -0.4,
            (Trait::Bold, Venture) => 1.0,
            (Trait::Bold, Muster) => 0.4,
            (Trait::Bold, Fortify) => -0.5,
            (Trait::Vain, Architect) => 1.2,
            (Trait::Vain, Prosper) => 0.3,
            (Trait::Vain, Revel) => 0.3,
            (Trait::Pious, Revel) => 0.8,
            (Trait::Pious, Architect) => 0.3,
            (Trait::Greedy, Prosper) => 1.0,
            (Trait::Greedy, Delve) => 0.6,
            (Trait::Greedy, Revel) => -0.4,
            (Trait::Sentimental, Revel) => 0.5,
            (Trait::Sentimental, Provision) => 0.3,
            (Trait::Sentimental, Venture) => -0.6,
            (Trait::Cheerful, Revel) => 1.0,
            (Trait::Cheerful, Provision) => 0.2,
            _ => 0.0,
        }
    }

    /// Layouts it favours (added to its style's own).
    fn layouts(self) -> &'static [Layout] {
        match self {
            Trait::Cautious => &[Layout::Grid, Layout::Orderly],
            Trait::Bold => &[Layout::Spokes],
            Trait::Vain => &[Layout::Radial, Layout::Symmetric, Layout::Radial],
            Trait::Pious => &[Layout::Symmetric],
            Trait::Greedy => &[Layout::Orderly],
            Trait::Sentimental => &[Layout::Radial],
            Trait::Stubborn => &[],
            Trait::Cheerful => &[Layout::Spokes, Layout::Sprawling],
        }
    }

    /// How it puts things, at the head of a note.
    fn voice(self) -> &'static str {
        match self {
            Trait::Cautious => "Uneasy, it",
            Trait::Bold => "Without a second thought, it",
            Trait::Vain => "With an eye to how it looks, it",
            Trait::Pious => "Trusting in the gods, it",
            Trait::Greedy => "Counting the cost, it",
            Trait::Sentimental => "With a heavy heart, it",
            Trait::Stubborn => "Grudgingly, it",
            Trait::Cheerful => "Whistling, it",
        }
    }
}

impl Overseer {
    pub fn has(&self, t: Trait) -> bool {
        self.traits.contains(&t)
    }

    /// "Urist Gemcutter, cautious and vain".
    pub fn title(&self) -> String {
        let traits: Vec<&str> = self.traits.iter().map(|t| t.name()).collect();
        if self.name.is_empty() {
            return traits.join(" and ");
        }
        if traits.is_empty() {
            self.name.clone()
        } else {
            format!("{}, {}", self.name, traits.join(" and "))
        }
    }

    /// The traits' lean toward an ambition, all told.
    pub fn temper(&self, a: Ambition) -> f64 {
        self.traits.iter().map(|t| t.lean(a)).sum()
    }

    /// A note in its voice: "Uneasy, it saw the fortress short of…".
    pub fn voiced(&self, text: &str) -> String {
        match self.traits.first() {
            Some(t) if text.starts_with("Saw") || text.starts_with("Gave up") => format!("{} {}{}", t.voice(), text[..1].to_lowercase(), &text[1..]),
            _ => text.to_string(),
        }
    }
}

impl Game {
    /// A name and two traits for a new overseer.
    pub fn roll_personality(&mut self, o: &mut Overseer) {
        o.name = crate::overworld::person_name(&mut self.rng, crate::overworld::Race::Dwarf);
        let first = *self.rng.pick(&Trait::ALL);
        let rest: Vec<Trait> = Trait::ALL.iter().copied().filter(|&t| t != first && !t.clashes(first)).collect();
        let second = *self.rng.pick(&rest);
        o.traits = vec![first, second];
    }

    /// The layout its style and traits lean to.
    pub(super) fn pick_layout(&mut self, o: &Overseer) -> Layout {
        let mut options: Vec<Layout> = Layout::options(o.style).to_vec();
        for t in &o.traits {
            options.extend_from_slice(t.layouts());
        }
        *self.rng.pick(&options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::overseer::Style;

    #[test]
    fn every_overseer_has_a_name_and_two_traits_that_agree() {
        let mut g = Game::new(5);
        for _ in 0..40 {
            let mut o = Overseer::new(Style::Steady);
            g.roll_personality(&mut o);
            assert!(!o.name.is_empty());
            assert_eq!(o.traits.len(), 2);
            assert_ne!(o.traits[0], o.traits[1]);
            assert!(!(o.has(Trait::Cautious) && o.has(Trait::Bold)));
        }
    }

    #[test]
    fn the_same_style_with_other_traits_leans_another_way() {
        let mut g = Game::new(5);
        g.set_overseer(Some(Style::Steady));
        for _ in 0..DAY * 20 {
            g.step();
        }
        let mut o = g.overseer.clone().unwrap();
        let odds = |g: &Game, o: &Overseer, a: Ambition| g.brain(o).odds_of(a);
        o.traits = vec![Trait::Vain, Trait::Greedy];
        let (arch_v, pros_v, fort_v) = (odds(&g, &o, Ambition::Architect), odds(&g, &o, Ambition::Prosper), odds(&g, &o, Ambition::Fortify));
        o.traits = vec![Trait::Cautious, Trait::Sentimental];
        let (arch_c, pros_c, fort_c, vent_c) = (odds(&g, &o, Ambition::Architect), odds(&g, &o, Ambition::Prosper), odds(&g, &o, Ambition::Fortify), odds(&g, &o, Ambition::Venture));
        o.traits = vec![Trait::Bold, Trait::Cheerful];
        let vent_b = odds(&g, &o, Ambition::Venture);
        assert!(arch_v > arch_c && pros_v > pros_c, "the vain and greedy one leans to grandeur and riches");
        assert!(fort_c > fort_v, "the cautious one leans to walls");
        assert!(vent_b >= vent_c, "the bold one leans to adventure");
        // And the layout it digs leans its way too.
        o.traits = vec![Trait::Vain, Trait::Pious];
        let showy = (0..200).filter(|_| matches!(g.pick_layout(&o), crate::game::layout::Layout::Radial | crate::game::layout::Layout::Symmetric)).count();
        o.traits = vec![Trait::Cautious, Trait::Greedy];
        let plain = (0..200).filter(|_| matches!(g.pick_layout(&o), crate::game::layout::Layout::Radial | crate::game::layout::Layout::Symmetric)).count();
        assert!(showy > plain, "a vain overseer digs showier halls ({} vs {})", showy, plain);
    }
}
