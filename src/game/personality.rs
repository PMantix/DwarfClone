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

    /// Ways it puts things, at the head of a note.
    fn voices(self) -> &'static [&'static str] {
        match self {
            Trait::Cautious => &["Uneasy, it", "Fearing the worst, it", "Taking no chances, it", "After a long look, it", "Wary as ever, it", "Not liking it, it"],
            Trait::Bold => &["Without a second thought, it", "Shrugging, it", "Not one to wait, it", "Straight off, it", "With a grin, it", "Rolling up its sleeves, it"],
            Trait::Vain => &["With an eye to how it looks, it", "Thinking of the visitors, it", "Unwilling to look shabby, it", "Smoothing its beard, it", "For the fortress's good name, it", "Mindful of appearances, it"],
            Trait::Pious => &["Trusting in the gods, it", "With a quick prayer, it", "Gods willing, it", "Murmuring a blessing, it", "Taking it as a sign, it", "Faithful as ever, it"],
            Trait::Greedy => &["Counting the cost, it", "Tallying the stores, it", "Begrudging every coin, it", "With an eye on the ledger, it", "Weighing the price, it", "Loath to waste a thing, it"],
            Trait::Sentimental => &["With a heavy heart, it", "Thinking of its dwarves, it", "Sighing, it", "Worried for them all, it", "Fondly, it", "Wistful, it"],
            Trait::Stubborn => &["Grudgingly, it", "Muttering, it", "Against its own plans, it", "With bad grace, it", "Since it must, it", "Scowling, it"],
            Trait::Cheerful => &["Whistling, it", "Humming a tune, it", "Cheerily, it", "With a wink, it", "Never one to fret, it", "Smiling, it"],
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

    /// A note in its voice now and then: "Uneasy, it saw the fortress
    /// short of…". About one such note in three is voiced, the rest plain.
    pub fn voiced(&self, text: &str) -> String {
        self.voiced_as(text, false)
    }

    /// `voiced`, always in its voice when `first` (a new worry): never
    /// the same opening twice running.
    pub fn voiced_as(&self, text: &str, first: bool) -> String {
        let Some(t) = self.traits.first() else { return text.to_string() };
        if !(text.starts_with("Saw") || text.starts_with("Gave up")) {
            return text.to_string();
        }
        let pool = t.voices();
        let opening = |n: &str| pool.iter().position(|v| n.starts_with(v));
        // Its last few notes of this kind, newest first: voiced or plain.
        let recent: Vec<Option<usize>> =
            self.notes.iter().rev().filter(|n| n.1.starts_with("Saw") || n.1.starts_with("Gave up") || opening(&n.1).is_some()).take(2).map(|n| opening(&n.1)).collect();
        if !first && recent.iter().any(|o| o.is_some()) {
            return text.to_string();
        }
        let last = self.notes.iter().rev().find_map(|n| opening(&n.1));
        // A fixed scramble of the note and how far its notes have run.
        let h = text.bytes().chain((self.notes.len() as u32).to_le_bytes()).fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3));
        let mut i = (h % pool.len() as u64) as usize;
        if Some(i) == last {
            i = (i + 1 + (h / 7 % (pool.len() as u64 - 1)) as usize) % pool.len();
        }
        format!("{} {}{}", pool[i], text[..1].to_lowercase(), &text[1..])
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
    fn its_voice_varies_and_is_not_on_every_note() {
        for t in Trait::ALL {
            let mut o = Overseer::new(Style::Steady);
            o.traits = vec![t];
            let mut openings: Vec<usize> = Vec::new();
            for i in 0..20 {
                let text = o.voiced_as(&format!("Saw the fortress short of thing {}: did something.", i), true);
                let k = t.voices().iter().position(|v| text.starts_with(v)).expect("voiced");
                assert_ne!(openings.last(), Some(&k), "{:?} said {:?} twice running", t, t.voices()[k]);
                openings.push(k);
                o.notes.push((i, text));
            }
            let mut kinds = openings.clone();
            kinds.sort();
            kinds.dedup();
            assert!(kinds.len() >= 4, "{:?}: only {:?}", t, kinds);
            // Unprompted, about one note in three is in its voice.
            o.notes.clear();
            let mut voiced = 0;
            for i in 0..30 {
                let text = o.voiced(&format!("Saw the fortress short of thing {}: did something.", i));
                voiced += !text.starts_with("Saw") as usize;
                o.notes.push((i, text));
            }
            assert_eq!(voiced, 10, "{:?}", t);
            assert_eq!(o.voiced("Dug out a room."), "Dug out a room.");
        }
    }

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
