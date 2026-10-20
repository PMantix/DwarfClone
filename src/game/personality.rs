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

    /// The trait by its name ("cautious"), for `--traits`.
    pub fn parse(name: &str) -> Option<Trait> {
        Trait::ALL.iter().copied().find(|t| t.name() == name.trim().to_lowercase())
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
    pub fn clashes(self, other: Trait) -> bool {
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

/// What a note is about: it picks the words a trait puts at its head
/// (`Trait::voice_for`). Notes nobody tagged are `Plain`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteKind {
    Plain,
    /// What it saw the fortress short of (`troubles.rs`, already voiced).
    Saw,
    /// A room dug out, a level opened, the stair sunk.
    Dug,
    /// A room fitted out.
    Fitted,
    /// The gate, its bridge and traps, the shelter.
    Gate,
    /// A new ambition, or a step toward it.
    Ambition,
    Siege,
    Death,
    Trade,
    /// An expedition or a hunt.
    Venture,
    /// What it says back to an order from you.
    Command,
}

impl NoteKind {
    /// Kinds whose notes now and then open in the overseer's voice when
    /// written (Saw notes are voiced where they are made; Siege, Death and
    /// Command notes are written whole in its voice).
    fn voiced(self) -> bool {
        matches!(self, NoteKind::Dug | NoteKind::Fitted | NoteKind::Gate | NoteKind::Ambition | NoteKind::Venture)
    }
}

impl Trait {
    /// Ways it puts a note of this kind, at its head: each ends in "it",
    /// and the note goes on in lower case ("Proudly, it dug out a tavern.").
    pub fn voice_for(self, kind: NoteKind) -> &'static [&'static str] {
        use NoteKind::*;
        use Trait::*;
        match (self, kind) {
            (Cautious, Dug) => &["Shoring every span, it", "Testing the rock twice, it", "Mindful of the ceiling, it"],
            (Cautious, Fitted) => &["Checking every hinge, it", "With a place for everything, it", "Leaving nothing to chance, it"],
            (Cautious, Gate) => &["Sleeping easier for it, it", "Glancing at the hills, it", "Never trusting the quiet, it"],
            (Cautious, Ambition) => &["Having thought it over for days, it", "Against the day the worst comes, it", "Slowly and surely, it"],
            (Cautious, Venture) => &["Against its better judgement, it", "Praying they'd come back, it", "With a long list of warnings, it"],
            (Bold, Dug) => &["Swinging the first pick itself, it", "Too impatient to plan, it", "Straight into the rock, it"],
            (Bold, Fitted) => &["Not fussing over it, it", "Quick about it, it", "Kicking the crates open, it"],
            (Bold, Gate) => &["Thinking it hardly needed, it", "Since the others fretted, it", "Rolling its eyes, it"],
            (Bold, Ambition) => &["Thumping the table, it", "Hungry for something bigger, it", "Without a second thought, it"],
            (Bold, Venture) => &["Itching to go along, it", "With a roar, it", "Spoiling for a fight, it"],
            (Vain, Dug) => &["Picturing the visitors' faces, it", "Proud of the stonework, it", "For a hall worth boasting of, it"],
            (Vain, Fitted) => &["Choosing only the finest, it", "Fussing over every chair, it", "So no guest could sneer, it"],
            (Vain, Gate) => &["Wanting a gate fit for a king, it", "With an eye to how it looks, it", "Grandly, it"],
            (Vain, Ambition) => &["Dreaming of its name in the legends, it", "For the fortress's good name, it", "Admiring itself in a polished shield, it"],
            (Vain, Venture) => &["For the glory of it, it", "Already drafting the song, it", "To be talked of in the hillhalls, it"],
            (Pious, Dug) => &["Asking the stone's pardon, it", "With a blessing on every pick, it", "Gods willing, it"],
            (Pious, Fitted) => &["Blessing every bed, it", "With a prayer at the threshold, it", "Murmuring a blessing, it"],
            (Pious, Gate) => &["Trusting the gods and a good lever, it", "With a charm hung on the hinge, it", "Taking no sign for granted, it"],
            (Pious, Ambition) => &["Taking it as a sign, it", "After a long night of prayer, it", "As the gods would want, it"],
            (Pious, Venture) => &["Commending them to the gods, it", "With a quick prayer, it", "Burning incense for their luck, it"],
            (Greedy, Dug) => &["Hoping for ore behind the wall, it", "Counting the stone it would sell, it", "With an eye on the seams, it"],
            (Greedy, Fitted) => &["Haggling over every plank, it", "Begrudging every coin, it", "Making do where it could, it"],
            (Greedy, Gate) => &["Guarding what's ours, it", "To keep thieves from the vault, it", "Weighing the price, it"],
            (Greedy, Ambition) => &["Smelling a profit, it", "Tallying the stores, it", "With an eye on the ledger, it"],
            (Greedy, Venture) => &["Expecting loot, it", "Counting the spoils in advance, it", "For a share of whatever they found, it"],
            (Sentimental, Dug) => &["Thinking of the dwarves who'd live there, it", "Fondly, it", "Making room for the young ones, it"],
            (Sentimental, Fitted) => &["Wanting them to feel at home, it", "With a soft spot for the old ones, it", "Thinking of its dwarves, it"],
            (Sentimental, Gate) => &["Worried for them all, it", "To keep its dwarves safe, it", "Sighing, it"],
            (Sentimental, Ambition) => &["For the dwarves' sake, it", "Thinking of the ones it lost, it", "Wistful, it"],
            (Sentimental, Venture) => &["Waving them off at the gate, it", "Hugging each one, it", "With a lump in its throat, it"],
            (Stubborn, Dug) => &["Just as it planned, it", "Ignoring every complaint, it", "Its way and no other, it"],
            (Stubborn, Fitted) => &["Refusing all advice, it", "Exactly as it said it would, it", "Muttering, it"],
            (Stubborn, Gate) => &["Since it must, it", "With bad grace, it", "Grudgingly, it"],
            (Stubborn, Ambition) => &["Having made up its mind long ago, it", "Not to be talked out of it, it", "Scowling, it"],
            (Stubborn, Venture) => &["Whatever the council said, it", "Unmoved by the grumbling, it", "Once and for all, it"],
            (Cheerful, Dug) => &["Whistling as the picks rang, it", "Humming a tune, it", "With a skip in its step, it"],
            (Cheerful, Fitted) => &["Hanging a garland over the door, it", "Smiling, it", "Cheerily, it"],
            (Cheerful, Gate) => &["Never one to fret, it", "Joking about goblins, it", "With a wink, it"],
            (Cheerful, Ambition) => &["Delighted with the idea, it", "Over a round of ale, it", "Grinning ear to ear, it"],
            (Cheerful, Venture) => &["With a song for the road, it", "Packing them a hamper, it", "Cheering them off, it"],
            _ => self.voices(),
        }
    }
}

/// Whether a note starts with something it did ("Dug out...", "Set...",
/// "Hung..."), so it reads on after "Proudly, it".
fn starts_with_a_deed(text: &str) -> bool {
    let word = text.split(|c: char| !c.is_alphabetic()).next().unwrap_or("");
    word.len() > 2
        && word.chars().next().map_or(false, |c| c.is_uppercase())
        && (word.ends_with("ed") || ["Dug", "Sent", "Set", "Hung", "Began", "Sank", "Took", "Made", "Struck", "Kept", "Had", "Laid", "Built", "Cut", "Put", "Gave", "Led"].contains(&word))
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

    /// A note of this kind in its voice, always (when it has a trait and
    /// the note starts with a deed): which trait speaks, and which opening,
    /// follow the tick and how far its notes have run, never the game rng.
    pub fn voiced_in(&self, kind: NoteKind, text: &str, tick: u64) -> String {
        if self.traits.is_empty() || !starts_with_a_deed(text) {
            return text.to_string();
        }
        let h = super::names::hash(&[tick, self.notes.len() as u64, kind as u64]);
        let t = self.traits[(h % self.traits.len() as u64) as usize];
        let pool = t.voice_for(kind);
        let opening = pool[((h / 7) % pool.len() as u64) as usize];
        format!("{} {}{}", opening, text[..1].to_lowercase(), &text[1..])
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

/// One of `lines`, picked by a steady hash of `parts` (never the game rng).
fn one_of(lines: &[String], parts: &[u64]) -> String {
    lines[(super::names::hash(parts) % lines.len() as u64) as usize].clone()
}

/// "Kib Ironhand, struck down by a goblin", "Kib Ironhand, dead of thirst".
fn how_they_died(d: &super::annals::DeathNote) -> String {
    let how = if d.cause.contains(' ') { d.cause.clone() } else { format!("dead of {}", d.cause) };
    match &d.killer {
        Some(k) if !how.contains(k.as_str()) && d.cause == "struck down" => format!("{}, {} by {} {}", d.name, how, if k.starts_with(['a', 'e', 'i', 'o', 'u']) { "an" } else { "a" }, k),
        _ => format!("{}, {}", d.name, how),
    }
}

impl Game {
    /// Daily: the overseer takes in the deaths and the sieges since it last
    /// looked, and the ones its traits make it feel say so in its notes. A
    /// sentimental one mourns each dwarf by name, a bold one sneers at a
    /// foe that slunk off, a cautious one frets, a greedy one counts the cost.
    pub(super) fn ov_reflect(&mut self, o: &mut Overseer) {
        let tick = self.tick;
        // Deaths, each once (those from before it took charge pass unremarked).
        let n = self.annals.deaths.len();
        for i in o.seen_deaths.min(n)..n {
            let d = &self.annals.deaths[i];
            if d.name.is_empty() || d.tick + SEASON < tick {
                continue;
            }
            let who = how_they_died(d);
            let name = d.name.clone();
            let h = [d.tick, i as u64];
            let line = o.traits.iter().find_map(|t| match t {
                Trait::Sentimental => Some(one_of(
                    &[
                        format!("With a heavy heart, it mourned {}.", who),
                        format!("It sat a long while by the body of {}.", who),
                        format!("Wistful, it told the others stories of {} late into the night.", name),
                    ],
                    &h,
                )),
                Trait::Pious => Some(one_of(&[format!("It prayed for the soul of {}.", who), format!("It asked the gods to take {} kindly.", name)], &h)),
                Trait::Bold if d.soldier => Some(format!("It raised a tankard to {}, who fell with a weapon in hand.", name)),
                Trait::Cautious if d.outside => Some(format!("Uneasy, it counted heads again: {} should never have been outside.", name)),
                Trait::Greedy if d.soldier => Some(format!("Counting the cost, it had {}'s kit taken back to the stores.", name)),
                _ => None,
            });
            if let Some(line) = line {
                Self::note_as(o, tick, NoteKind::Death, line);
            }
        }
        o.seen_deaths = n;
        // Sieges, once they are over.
        while let Some(s) = self.annals.sieges.get(o.seen_sieges) {
            if s.ended.is_none() {
                break;
            }
            let i = o.seen_sieges;
            o.seen_sieges += 1;
            if s.ended.map_or(true, |e| e + SEASON < tick) || s.outcome == "the fortress fell" {
                continue;
            }
            let foe = s.foe.clone();
            let lost = s.dwarves_lost;
            let h = [s.tick, i as u64, 7];
            let withdrew = s.outcome == "withdrew";
            let line = o.traits.iter().find_map(|t| match t {
                Trait::Bold if withdrew => Some(one_of(
                    &[
                        format!("Laughing, it watched the {} slink off without a fight. Cowards.", foe),
                        format!("It shouted after the {} from the gate: come back when you've found your nerve!", foe),
                    ],
                    &h,
                )),
                Trait::Bold => Some(format!("It strutted the halls: the {} had come for us and got what they came for.", foe)),
                Trait::Cautious => Some(one_of(
                    &[
                        format!("Fearing the worst, it doubled the watch: the {} will be back, and more of them.", foe),
                        format!("It lay awake going over the gate again: next time the {} might get in.", foe),
                    ],
                    &h,
                )),
                Trait::Greedy => Some(format!("It tallied the siege: {} of ours lost, and the {}' gear for the smelter.", lost, foe)),
                Trait::Sentimental if lost > 0 => Some(format!("It sat long with the families of the {} who fell to the {}.", lost, foe)),
                Trait::Pious => Some(format!("It gave thanks to the gods: the {} are gone.", foe)),
                Trait::Cheerful => Some(format!("It called for a round on the house: the {} are gone!", foe)),
                Trait::Vain => Some(format!("It had the tale of the siege told in every hall, with itself at the heart of it: the {} broke on our gate.", foe)),
                Trait::Stubborn => Some(format!("It changed nothing after the {} left: the plan was sound all along.", foe)),
                _ => None,
            });
            if let Some(line) = line {
                Self::note_as(o, tick, NoteKind::Siege, line);
            }
        }
    }
}

impl Game {
    /// A note tagged with what it is about: about one in three of the kinds
    /// that take a voice opens in the overseer's ("Proud of the stonework,
    /// it dug out a tavern."); the rest go in plain, as `note_ov`.
    pub(super) fn note_as(o: &mut Overseer, tick: u64, kind: NoteKind, text: String) {
        let speak = kind.voiced() && super::names::hash(&[tick, o.notes.len() as u64, 31]) % 3 == 0;
        let text = if speak { o.voiced_in(kind, &text, tick) } else { text };
        Self::note(o, tick, text);
    }

    /// Choose the overseer's traits by hand: one or two, none twice and no
    /// pair that clashes (cautious and bold). What to tell the player.
    pub fn set_traits(&mut self, traits: Vec<Trait>) -> Result<String, String> {
        let Some(o) = self.overseer.as_mut() else { return Err("There is no overseer: pick a style first.".into()) };
        if traits.len() > 2 {
            return Err("Two traits at most: untick one first.".into());
        }
        for (i, a) in traits.iter().enumerate() {
            if traits[..i].contains(a) {
                return Err(format!("It is {} once already.", a.name()));
            }
            if let Some(b) = traits[..i].iter().find(|b| a.clashes(**b)) {
                return Err(format!("Nobody is both {} and {}.", b.name(), a.name()));
            }
        }
        o.traits = traits;
        // Its layout is picked when it first breaks ground (`ov_setup`), so
        // an overseer that hasn't dug in yet lays out by its new traits.
        Ok(format!("The overseer is now {}.", o.title()))
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
    fn every_trait_speaks_in_its_own_voice() {
        let plain = "Dug out a tavern.";
        let said = |t: Trait, tick: u64| {
            let mut o = Overseer::new(Style::Steady);
            o.traits = vec![t];
            o.voiced_in(NoteKind::Dug, plain, tick)
        };
        for t in Trait::ALL {
            let text = said(t, 100);
            assert_ne!(text, plain, "{:?}", t);
            assert!(text.ends_with("dug out a tavern."), "{}", text);
            assert!(t.voice_for(NoteKind::Dug).iter().any(|v| text.starts_with(v)), "{:?}: {}", t, text);
            // The same tick, the same words: no rng in it.
            assert_eq!(text, said(t, 100));
            // And no other trait has those words for it.
            for u in Trait::ALL.iter().filter(|&&u| u != t) {
                assert!(!u.voice_for(NoteKind::Dug).iter().any(|v| text.starts_with(v)), "{:?} and {:?} share {}", t, u, text);
            }
        }
        // A note that doesn't start with a deed is left as it is.
        let mut o = Overseer::new(Style::Steady);
        o.traits = vec![Trait::Vain];
        assert_eq!(o.voiced_in(NoteKind::Dug, "It means to lay the fortress out as a grid.", 5), "It means to lay the fortress out as a grid.");
        // Written as notes, about one in three of a voiced kind is in its voice.
        let mut voiced = 0;
        for i in 0..60 {
            Game::note_as(&mut o, i * 1000, NoteKind::Fitted, format!("Fitted out room {}.", i));
            voiced += !o.notes.last().unwrap().1.starts_with("Fitted") as usize;
        }
        assert!((10..=30).contains(&voiced), "{} of 60 voiced", voiced);
    }

    #[test]
    fn a_sentimental_overseer_mourns_by_name() {
        let mut g = Game::new(5);
        g.set_overseer(Some(Style::Steady));
        assert!(g.set_traits(vec![Trait::Sentimental, Trait::Greedy]).is_ok());
        g.tick = 3 * DAY;
        let mut d = std::mem::take(&mut g.dwarves[2]);
        g.kill_for_test(&mut d, "struck down");
        let name = d.name.clone();
        g.dwarves[2] = d;
        let mut o = g.overseer.take().unwrap();
        g.tick += 10;
        g.ov_reflect(&mut o);
        let mourned = |o: &Overseer| o.notes.iter().filter(|n| n.1.contains(&name) && !n.1.starts_with("Remembered")).count();
        assert_eq!(mourned(&o), 1, "{:?}", o.notes);
        // Once only.
        g.tick += DAY;
        g.ov_reflect(&mut o);
        assert_eq!(mourned(&o), 1);
        // A greedy, cheerful one has nothing to say of a civilian's death,
        // but a siege that slinks off gets a word from a bold one.
        let mut d = std::mem::take(&mut g.dwarves[3]);
        g.kill_for_test(&mut d, "struck down");
        let other = d.name.clone();
        g.dwarves[3] = d;
        o.traits = vec![Trait::Bold, Trait::Cheerful];
        g.annals.sieges.push(crate::game::annals::SiegeNote { tick: g.tick, foe: "goblins".into(), outcome: "withdrew".into(), ended: Some(g.tick), ..Default::default() });
        g.tick += 10;
        g.ov_reflect(&mut o);
        assert!(!o.notes.iter().any(|n| n.1.contains(&other)) || g.annals.deaths.last().map_or(false, |d| d.soldier));
        assert!(o.notes.last().unwrap().1.contains("goblins"), "{:?}", o.notes.last());
    }

    #[test]
    fn traits_can_be_chosen_but_not_clashing() {
        let mut g = Game::new(5);
        assert!(g.set_traits(vec![Trait::Vain]).is_err(), "no overseer yet");
        g.set_overseer(Some(Style::Steady));
        let before = g.overseer.as_ref().unwrap().traits.clone();
        assert!(g.set_traits(vec![Trait::Cautious, Trait::Bold]).is_err());
        assert!(g.set_traits(vec![Trait::Vain, Trait::Vain]).is_err());
        assert!(g.set_traits(vec![Trait::Vain, Trait::Pious, Trait::Greedy]).is_err());
        assert_eq!(g.overseer.as_ref().unwrap().traits, before, "a refused pick changes nothing");
        assert!(g.set_traits(vec![Trait::Vain, Trait::Pious]).is_ok());
        assert!(g.overseer.as_ref().unwrap().title().ends_with("vain and pious"));
        assert_eq!(Trait::parse(" Greedy"), Some(Trait::Greedy));
    }

    /// O01's "done when": the same seed and style, other traits, another fort.
    #[test]
    fn two_tempers_build_different_forts() {
        use crate::buildings::BuildingKind;
        use crate::game::overseer::RoomUse;
        let fort = |traits: Vec<Trait>| {
            let mut g = Game::new(5);
            g.set_overseer(Some(Style::Steady));
            g.set_traits(traits).unwrap();
            for _ in 0..SEASON * 2 {
                g.step();
            }
            let o = g.overseer.clone().unwrap();
            let traps = g.buildings.values().filter(|b| matches!(b.kind, BuildingKind::CageTrap | BuildingKind::StoneTrap | BuildingKind::WeaponTrap)).count();
            let rooms = o.rooms.iter().filter(|r| matches!(r.kind, RoomUse::Tavern | RoomUse::Temple | RoomUse::Grand)).map(|r| format!("{:?}", r.kind)).collect::<Vec<_>>();
            let ambitions: Vec<String> = o.ambitions.iter().map(|a| a.1.name().to_string()).collect();
            let architect = g.brain(&o).odds_of(Ambition::Architect);
            let print = format!("{:?} {:?} {:?} traps {} patience {}", o.layout, ambitions, rooms, traps, g.bar_patience(&o));
            (print, o.layout, traps, rooms, architect, g.bar_patience(&o))
        };
        let (a, _, traps_a, _, arch_a, wait_a) = fort(vec![Trait::Cautious, Trait::Greedy]);
        let (b, layout_b, traps_b, rooms_b, arch_b, wait_b) = fort(vec![Trait::Vain, Trait::Cheerful]);
        assert_ne!(a, b, "the same fort twice");
        // Caution shows as more traps or a gate barred sooner (a trap or two
        // either way is the luck of the year).
        assert!(traps_a >= traps_b || wait_a < wait_b, "the cautious one is no warier: {} | {}", a, b);
        let showy = matches!(layout_b, Layout::Radial | Layout::Symmetric) || rooms_b.iter().any(|r| r == "Grand") || arch_b > arch_a;
        assert!(showy, "the vain one is no showier: {} | {}", a, b);
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
