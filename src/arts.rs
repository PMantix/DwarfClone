//! The arts of the world's peoples. Every civilization has instruments of
//! its own (what they're called, what they're made of, how they're played
//! and how they sound), its own dances and its own forms of poetry and
//! music. They are made from the world's seed and the civilization, on a
//! random stream of their own, so the same world always has the same arts
//! and nothing else in it changes for them. New forms devised in play
//! (`game/culture.rs`) are kept with the world, with every people that has
//! taken them up.

// The fortress takes these up in the steps that follow (game/culture.rs).
#![cfg_attr(not(test), allow(dead_code))]

use crate::overworld::{word, Overworld, Race};
use crate::rng::{splitmix, Rng};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormKind {
    Poetry,
    Music,
    Dance,
}

impl FormKind {
    pub fn name(self) -> &'static str {
        match self {
            FormKind::Poetry => "poetic form",
            FormKind::Music => "musical form",
            FormKind::Dance => "dance",
        }
    }
}

/// A kind of musical instrument a people plays.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Instrument {
    /// Its own name ("zanmor").
    pub name: String,
    /// What sort of thing it is ("drum").
    pub kind: String,
    /// What it's made of ("of hollowed stone and stretched hide").
    pub made: String,
    /// How it's played ("struck with two padded mallets").
    pub played: String,
    /// How it sounds ("a deep roll that is felt in the chest").
    pub sound: String,
}

impl Instrument {
    /// "The zanmor is a drum of hollowed stone and stretched hide, struck
    /// with two padded mallets. It sounds a deep roll that is felt in the chest."
    pub fn describe(&self) -> String {
        format!("The {} is a {} {}, {}. It sounds {}.", self.name, self.kind, self.made, self.played, self.sound)
    }
}

/// A dance, or a form of poetry or music.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Form {
    pub name: String,
    pub kind: FormKind,
    /// What it is ("couplets that answer each other, spoken low and slow").
    pub desc: String,
    /// The people it comes from.
    pub civ: u32,
    /// Devised in play: by whom, where and when ("Urist Kadol of Ironhold, in 205").
    #[serde(default)]
    pub origin: Option<String>,
}

impl Form {
    /// "the zanost, a poetic form of battle and the dead: ...".
    pub fn describe(&self) -> String {
        format!("the {}, a {} {}", self.name, self.kind.name(), self.desc)
    }
}

/// What one people plays, dances and recites.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CivArts {
    pub instruments: Vec<Instrument>,
    pub forms: Vec<Form>,
}

impl CivArts {
    pub fn of_kind(&self, k: FormKind) -> Vec<&Form> {
        self.forms.iter().filter(|f| f.kind == k).collect()
    }
}

/// The forms devised in play, and who has taken them up.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WorldArts {
    pub invented: Vec<Form>,
    /// (civilization, form name): a people that has taken up a new form.
    pub adopted: Vec<(u32, String)>,
}

impl WorldArts {
    pub fn is_empty(&self) -> bool {
        self.invented.is_empty() && self.adopted.is_empty()
    }

    /// Whether `civ` knows the invented form `name` (devised by them or taken up).
    pub fn knows(&self, civ: u32, name: &str) -> bool {
        self.invented.iter().any(|f| f.name == name && f.civ == civ) || self.adopted.iter().any(|(c, n)| *c == civ && n == name)
    }
}

// The makings of instruments, by people: (kind, what it's made of, how it's
// played, how it sounds).
type Making = (&'static str, &'static [&'static str], &'static str, &'static [&'static str]);

const DWARF_INSTRUMENTS: [Making; 5] = [
    ("drum", &["of hollowed stone and stretched hide", "made from a copper bowl capped with leather"], "struck with two padded mallets", &["a deep roll that is felt in the chest", "a slow thunder like a far-off cave-in"]),
    ("horn", &["of curled bone bound in bronze rings", "of beaten copper three strides long"], "blown from the bottom of the lungs", &["a long low bellow that carries through the halls", "a brassy call that rings off the stone"]),
    ("lyre", &["with a frame of stone and strings of gut", "of carved bone strung with copper wire"], "plucked with a pick of horn", &["a bright, hard ringing", "a cold clear tone like dripping water"]),
    ("chime-frame", &["hung with tuned slabs of slate", "hung with bars of iron and silver"], "struck with little hammers", &["peals that hang in the air", "a ringing like a mine full of bells"]),
    ("bellows-pipe", &["of a leather bag and three bone pipes"], "squeezed under the arm while the fingers stop the pipes", &["a droning skirl", "a reedy wail over a steady drone"]),
];
const HUMAN_INSTRUMENTS: [Making; 5] = [
    ("fiddle", &["of maple with four gut strings", "of pale wood with a carved head"], "bowed with a bow of horsehair", &["a sweet, sawing song", "a quick bright skirl that sets feet tapping"]),
    ("flute", &["of bored wood with six holes", "of silver, long and thin"], "blown across the end", &["a breathy, wandering tune", "a high clear whistle"]),
    ("harp", &["of bent oak with brass strings", "small enough to hold in the lap"], "plucked with the fingertips", &["a shimmer of falling notes", "a soft and stately ringing"]),
    ("tambour", &["of stretched skin on a wooden hoop hung with tin discs"], "shaken and slapped with the palm", &["a jangling beat", "a rattle and thump"]),
    ("trumpet", &["of hammered brass, coiled once", "of straight bright brass"], "blown with tight lips", &["a fanfare that wakes the dead", "a proud, piercing call"]),
];
const ELF_INSTRUMENTS: [Making; 4] = [
    ("harp", &["grown, not made, from a living vine", "of white wood strung with spider-silk"], "stroked with the backs of the fingers", &["a sound like wind in high leaves", "a cascade of soft bright notes"]),
    ("set of reed-pipes", &["of seven river reeds bound with grass"], "blown across their tops in turn", &["a breathy, birdlike trill", "a hollow and lonely song"]),
    ("glass chimes", &["of thin glass leaves hung from a branch"], "set swaying with a breath", &["a tinkling like rain on ice", "a shiver of high, clear notes"]),
    ("singing bowl", &["of polished wood with a lip as thin as a leaf"], "rubbed round its rim with a wand", &["a hum that swells until the air itself sings", "a low, endless tone"]),
];
const GOBLIN_INSTRUMENTS: [Making; 3] = [
    ("war drum", &["of hide stretched over a barrel of bones", "of a giant's skin stretched over iron hoops"], "beaten with clubs", &["a pounding that sets the teeth on edge", "a heartbeat for a hundred marching feet"]),
    ("rattle", &["made from a skull full of teeth", "of knucklebones on a cord"], "shaken hard", &["a dry, angry clatter", "a sound like bones in a storm"]),
    ("horn", &["cut from a beast's skull with the jaw wired shut", "of twisted ram's horn"], "blown with a roar", &["a shriek that echoes for miles", "a choking, grinding blare"]),
];

const POETRY_SUBJECT: [&str; 8] = [
    "of battle and the dead",
    "of the deep places and the stone",
    "of love and its losing",
    "of drink and boasting",
    "of the deeds of heroes",
    "of mourning",
    "of the turning seasons",
    "of craft and the making of things",
];
const POETRY_SHAPE: [&str; 6] = [
    "lines that all end on the same sound",
    "couplets that answer each other, the second mocking the first",
    "a single long line chanted on one breath",
    "verses each one line longer than the last",
    "a call by one voice and an answer by many",
    "three verses of riddles and a fourth that answers them",
];
const POETRY_RULE: [&str; 5] = ["spoken low and slow", "shouted", "half-whispered", "recited to the beat of a drum", "sung to a single held note"];
const MUSIC_PACE: [&str; 5] = ["slow and stately", "fast and stamping", "rising from a whisper to a roar", "in a swaying three-beat", "wandering, without a beat at all"];
const MUSIC_SHAPE: [&str; 5] = ["with a refrain everyone joins in", "each player taking a turn alone", "the voices following the instrument", "ending on a single held note", "two parts chasing each other"];
const DANCE_FORMATION: [&str; 6] = ["in two facing lines", "in a ring round the musicians", "for a pair", "for one dancer and a clapping crowd", "in a long chain through the hall", "in a square of four"];
const DANCE_STEP: [&str; 5] = ["stamping in time to the drum", "with leaps and turns", "slow and grave, hands on shoulders", "with mock blows traded in rhythm", "spinning faster and faster until someone falls"];

fn race_instruments(r: Race) -> &'static [Making] {
    match r {
        Race::Dwarf => &DWARF_INSTRUMENTS,
        Race::Human => &HUMAN_INSTRUMENTS,
        Race::Elf => &ELF_INSTRUMENTS,
        Race::Goblin => &GOBLIN_INSTRUMENTS,
    }
}

fn lower(s: String) -> String {
    s.to_lowercase()
}

/// A new name in a people's tongue, two syllables or three.
fn art_name(rng: &mut Rng, race: Race) -> String {
    let n = 2 + rng.chance(0.3) as i32;
    lower(word(rng, race, n))
}

/// A new instrument of a people's kind.
pub fn new_instrument(rng: &mut Rng, race: Race, taken: &[&str]) -> Instrument {
    let all = race_instruments(race);
    let free: Vec<&Making> = all.iter().filter(|m| !taken.contains(&m.0)).collect();
    let m = if free.is_empty() { rng.pick(all) } else { *rng.pick(&free) };
    Instrument { name: art_name(rng, race), kind: m.0.into(), made: (*rng.pick(m.1)).into(), played: m.2.into(), sound: (*rng.pick(m.3)).into() }
}

/// A new form of a kind, for a people with these instruments.
pub fn new_form(rng: &mut Rng, race: Race, civ: u32, kind: FormKind, instruments: &[Instrument]) -> Form {
    let played = |rng: &mut Rng| instruments.get(rng.range(0, instruments.len().max(1) as i32) as usize).map(|i| format!("the {} ({})", i.name, i.kind));
    let desc = match kind {
        FormKind::Poetry => format!("{}: {}, {}", rng.pick(&POETRY_SUBJECT), rng.pick(&POETRY_SHAPE), rng.pick(&POETRY_RULE)),
        FormKind::Music => {
            let on = played(rng).unwrap_or_else(|| "voices".into());
            format!("for {} and voices: {}, {}", on, rng.pick(&MUSIC_PACE), rng.pick(&MUSIC_SHAPE))
        }
        FormKind::Dance => format!("{}, {}", rng.pick(&DANCE_FORMATION), rng.pick(&DANCE_STEP)),
    };
    Form { name: art_name(rng, race), kind, desc, civ, origin: None }
}

fn name_hash(s: &str) -> u64 {
    s.bytes().fold(0xCBF2_9CE4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01B3))
}

/// The arts a people was born with: two or three instruments, two poetic
/// forms, one or two musical forms and one or two dances.
pub fn native_arts(ow: &Overworld, civ: u32) -> CivArts {
    let Some(c) = ow.civs.iter().find(|c| c.id == civ) else { return CivArts::default() };
    let mut rng = Rng::new(splitmix(ow.seed ^ 0xA275_C0DE) ^ (civ as u64).wrapping_mul(0x9E37_79B9) ^ name_hash(&c.name));
    let mut arts = CivArts::default();
    for _ in 0..2 + rng.range(0, 2) {
        let taken: Vec<&str> = arts.instruments.iter().map(|i| i.kind.as_str()).collect();
        let i = new_instrument(&mut rng, c.race, &taken);
        arts.instruments.push(i);
    }
    let counts = [(FormKind::Poetry, 2), (FormKind::Music, 1 + rng.range(0, 2)), (FormKind::Dance, 1 + rng.range(0, 2))];
    for (kind, n) in counts {
        for _ in 0..n {
            let f = new_form(&mut rng, c.race, civ, kind, &arts.instruments);
            arts.forms.push(f);
        }
    }
    arts
}

impl Overworld {
    /// A people's arts: their own, the new forms they devised, and those
    /// they've taken up from others.
    pub fn civ_arts(&self, civ: u32) -> CivArts {
        let mut a = native_arts(self, civ);
        for f in self.arts.invented.iter().filter(|f| self.arts.knows(civ, &f.name)) {
            a.forms.push(f.clone());
        }
        a
    }

    /// The lines the legends show under a civilization.
    pub fn arts_lines(&self, civ: u32) -> Vec<String> {
        let a = self.civ_arts(civ);
        let mut v = Vec::new();
        if a.instruments.is_empty() {
            return v;
        }
        let inst: Vec<String> = a.instruments.iter().map(|i| format!("the {} ({})", i.name, i.kind)).collect();
        v.push(format!("instruments: {}", inst.join(", ")));
        let forms: Vec<String> = a
            .forms
            .iter()
            .map(|f| {
                let from = if f.civ != civ { format!(", from the {}", self.civs.iter().find(|c| c.id == f.civ).map_or("far away", |c| c.name.as_str())) } else { String::new() };
                let kind = match f.kind {
                    FormKind::Poetry => "poetry",
                    FormKind::Music => "music",
                    FormKind::Dance => "dance",
                };
                format!("the {} ({}{})", f.name, kind, from)
            })
            .collect();
        v.push(format!("arts: {}", forms.join(", ")));
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_people_has_arts_of_its_own_and_the_same_world_the_same_arts() {
        let ow = Overworld::generate(81);
        assert!(ow.civs.len() >= 2);
        let mut names = Vec::new();
        for c in &ow.civs {
            let a = native_arts(&ow, c.id);
            assert!((2..=3).contains(&a.instruments.len()), "{} instruments", a.instruments.len());
            assert_eq!(a.of_kind(FormKind::Poetry).len(), 2);
            assert!(!a.of_kind(FormKind::Music).is_empty() && !a.of_kind(FormKind::Dance).is_empty());
            for i in &a.instruments {
                assert!(!i.name.is_empty() && !i.played.is_empty() && !i.sound.is_empty());
                assert!(race_instruments(c.race).iter().any(|m| m.0 == i.kind), "a {:?} {} is not their kind", c.race, i.kind);
            }
            // Two instruments of one people are different kinds.
            let mut kinds: Vec<&str> = a.instruments.iter().map(|i| i.kind.as_str()).collect();
            kinds.sort();
            kinds.dedup();
            assert_eq!(kinds.len(), a.instruments.len());
            assert_eq!(a, native_arts(&ow, c.id), "the same world, the same arts");
            names.push(a.instruments[0].name.clone());
            println!("{} ({:?}):", c.name, c.race);
            for i in &a.instruments {
                println!("  {}", i.describe());
            }
            for f in &a.forms {
                println!("  {}", f.describe());
            }
        }
        names.sort();
        names.dedup();
        assert!(names.len() * 2 > ow.civs.len(), "peoples share their instruments' names");
        assert!(ow.arts_lines(ow.civs[0].id).len() == 2);
    }

    #[test]
    fn a_new_form_is_known_where_it_was_devised_and_where_it_was_taken_up() {
        let mut ow = Overworld::generate(81);
        let (a, b) = (ow.civs[0].id, ow.civs[1].id);
        let mut rng = Rng::new(3);
        let mut f = new_form(&mut rng, Race::Dwarf, a, FormKind::Dance, &[]);
        f.origin = Some("Urist of Testhold, in 210".into());
        let name = f.name.clone();
        let before = ow.civ_arts(b).forms.len();
        ow.arts.invented.push(f);
        assert!(ow.civ_arts(a).forms.iter().any(|x| x.name == name));
        assert!(!ow.civ_arts(b).forms.iter().any(|x| x.name == name));
        ow.arts.adopted.push((b, name.clone()));
        assert_eq!(ow.civ_arts(b).forms.len(), before + 1);
        assert!(ow.arts_lines(b)[1].contains(&format!("the {} (dance, from the", name)));
        // Saved and loaded with the world.
        let back: Overworld = serde_json::from_str(&serde_json::to_string(&ow).unwrap()).unwrap();
        assert_eq!(back.arts, ow.arts);
    }
}
