//! Talk, news and rumours (A7). `k` near one of the folk opens a
//! conversation: a greeting that reads the hero's fame (the famous are
//! known on sight, the infamous turned away), then a DF-like menu of
//! things to ask: news of the world, rumours of places worth the road
//! (lairs, ruins, fallen forts and what was left in them), the old tales as
//! the speaker's people tell them this year (`tales::tale_for`), the way to
//! places heard of, the speaker's own story, and work and company (hooks
//! for A8 and A9: `quest_talk`, `company_talk`).
//!
//! A place heard of goes into `Hero.known` and is marked `!` on the travel
//! map. Every word is chosen by hashing the speaker and the topic through
//! `stories::hash`/`pick`, never the game's dice: ask twice, hear the same.
//! Hostile folk won't talk.

use super::peoples::{stance, Stance};
use super::sitegen::town::{FOLK, GUARD};
use super::travel::{cheb, land_name, number, way_to};
use super::turn::Outcome;
use super::{Adventure, SiteKey, HERO_ID};
use crate::creature::Species;
use crate::game::{stories, tales, Game};
use crate::overworld::{Occupant, Overworld, Race, SiteKind};

/// How near (in tiles) someone must stand to be talked to.
pub const REACH: i32 = 3;
/// What a question costs, in ticks.
const TALK_TICKS: u32 = 12;
/// `Hero.known` holds site ids as they are, and these for the rest.
pub const KNOWN_FORT: u32 = 0x8000_0000;
pub const KNOWN_REMAINS: u32 = 0x4000_0000;

/// What a rumour is about: hearing it marks the place on the travel map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum About {
    Site(u32),
    /// Index into `ow.forts`.
    Fort(usize),
    /// A figure of the legends (marked by their lair).
    Figure(u32),
    /// Index into `ow.remains`.
    Remains(usize),
}

impl About {
    /// The `Hero.known` id of the place it points to.
    pub fn known_id(self, ow: &Overworld) -> Option<u32> {
        match self {
            About::Site(id) => Some(id),
            About::Fort(i) => Some(KNOWN_FORT | i as u32),
            About::Remains(i) => Some(KNOWN_REMAINS | i as u32),
            About::Figure(f) => ow.figures.get(f as usize).and_then(|f| f.lair).and_then(|l| ow.sites.get(l as usize)).map(|s| s.id),
        }
    }
}

/// One thing heard: its words, what it is about, and how much it matters.
#[derive(Clone, Debug, PartialEq)]
pub struct Rumour {
    pub text: String,
    pub about: Option<About>,
    pub weight: i32,
}

/// A rumour before its words are chosen: its kind (forts, lairs, ruins,
/// bones, sagas, raids, the hero's own deeds), what fills the words, and
/// the way there.
struct Draft {
    kind: usize,
    args: Vec<(&'static str, String)>,
    tail: String,
    about: Option<About>,
    weight: i32,
}

/// How a speaker carries themselves (hashed from their name).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Temper {
    Warm,
    Gruff,
    Gloomy,
    Boastful,
}

/// Someone the hero can talk to, read off a creature (or a dwarf) of the
/// visit: never saved, worked out again each time.
#[derive(Clone, Debug, PartialEq)]
pub struct Person {
    pub id: u32,
    pub name: String,
    /// "smith", "guard of Copper Gate", "merchant".
    pub role: String,
    pub race: Race,
    pub civ: Option<u32>,
    /// Their home site.
    pub home: Option<u32>,
    pub temper: Temper,
    /// Merchants, bards and wanderers hear of places further off.
    pub travelled: bool,
    /// What they remember of the place (some of a town's folk carry one).
    pub memory: Option<String>,
    /// Out for blood, or afraid of the hero: either way, no talk.
    pub hostile: bool,
    pub afraid: bool,
}

impl Person {
    pub fn first(&self) -> &str {
        self.name.split(' ').next().unwrap_or(&self.name)
    }

    fn h(&self, salt: u64) -> u64 {
        stories::hash(&self.name, salt ^ self.id as u64)
    }
}

/// What can be asked, in the menu's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Topic {
    News,
    Rumours,
    Legends,
    Directions,
    Yourself,
    /// Quests (A8 fills `quest_talk`).
    Work,
    /// Companions (A9 fills `company_talk`).
    Join,
    Farewell,
}

pub const TOPICS: [Topic; 8] = [Topic::News, Topic::Rumours, Topic::Legends, Topic::Directions, Topic::Yourself, Topic::Work, Topic::Join, Topic::Farewell];

impl Topic {
    /// The hero's question.
    pub fn ask(self) -> &'static str {
        match self {
            Topic::News => "What news?",
            Topic::Rumours => "Heard any rumours? Places worth the road?",
            Topic::Legends => "Tell me a tale of the old days.",
            Topic::Directions => "Where is ...?",
            Topic::Yourself => "Tell me of yourself.",
            Topic::Work => "Is there work?",
            Topic::Join => "Will you come with me?",
            Topic::Farewell => "Farewell.",
        }
    }
}

/// Who said a line of a conversation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Said {
    Hero,
    Them,
    /// Narration: "Urist backs away."
    Aside,
    /// A place marked on the map.
    Marked,
}

/// A conversation on screen (the app holds it; never saved).
#[derive(Clone, Debug)]
pub struct Chat {
    pub who: Person,
    pub lines: Vec<(Said, String)>,
    pub asked: Vec<Topic>,
    /// They won't talk: only farewell is left.
    pub closed: bool,
}

/// One of a people's lines: two to four to a table, the speaker's hash
/// picking one.
fn voice<'a>(p: &Person, topic: u64, tables: &[&'a [&'a str]; 4]) -> &'a str {
    stories::pick(p.h(topic), topic, tables[race_idx(p.race)])
}

/// The `n`th of a speaker's lines from a table: one speaker telling of
/// two ruins doesn't say the same thing of both.
fn voice_n<'a>(p: &Person, topic: u64, tables: &[&'a [&'a str]; 4], n: usize) -> &'a str {
    let t = tables[race_idx(p.race)];
    t[((p.h(topic) as usize % t.len()) + n) % t.len()]
}

fn race_idx(r: Race) -> usize {
    match r {
        Race::Dwarf => 0,
        Race::Human => 1,
        Race::Elf => 2,
        Race::Goblin => 3,
    }
}

/// `{x}` replaced by each (key, value).
fn fill(t: &str, with: &[(&str, &str)]) -> String {
    let mut s = t.to_string();
    for (k, v) in with {
        s = s.replace(&format!("{{{}}}", k), v);
    }
    s
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn a_an(w: &str) -> String {
    let vowel = w.chars().next().map_or(false, |c| "aeiouAEIOU".contains(c));
    format!("{} {}", if vowel { "an" } else { "a" }, w)
}

// --- What each people says: original lines, short, in their own voices
// (dwarf, human, elf, goblin). ---

const HELLO: [&[&str]; 4] = [
    &["Hm. A traveller. Wipe your boots.", "Well met. Stone keep you.", "Strike the earth, stranger. What brings you?", "Sit, if you like. The ale is thin but honest."],
    &["Good day to you, traveller!", "Ho there! We don't see many new faces.", "Welcome, welcome. Mind the dogs.", "Morning. Or is it afternoon already?"],
    &["The wind said someone was coming.", "Walk softly here, wanderer.", "You smell of iron and road dust. Be welcome anyway.", "Greetings, short-lived one."],
    &["What do you want? Speak quick.", "Hah. Meat or a friend? We'll see.", "Nice boots. Keep an eye on them.", "Talk. Then go."],
];

/// The famous, known on sight: `{name}`, `{deed}`.
const FAMOUS: [&[&str]; 4] = [
    &["By my beard! {name}! You're the one who {deed}!", "{name}. Even the stones talk of you. You {deed}.", "Ha! {name} in our hall! You {deed}, they say. Drink with me."],
    &["Wait, I know you! You're {name}, the one who {deed}!", "Gods above, it's {name}! They say you {deed}!", "{name}! My children won't believe it. You {deed}!"],
    &["The trees whisper your name, {name}. You {deed}.", "So you are {name}, who {deed}. The songs made you taller.", "{name}. Even the grove has heard: you {deed}."],
    &["{name}! You {deed}. Hah! I'd drink with you.", "You're the one who {deed}. Don't look at me like that.", "{name}. We tell of you in the pits. You {deed}!"],
];

/// The infamous, turned away (goblins think no worse of a killer).
const SHUNNED: [&[&str]; 4] = [
    &["You. Murderer. Get out of my sight.", "I'll not speak with blood-handed folk. Begone.", "Our dead are not even cold. Go."],
    &["Stay back! I know what you did!", "Guards! ...No. Just leave. Please.", "I've children. Leave us be."],
    &["Your hands are red. I will not speak with you.", "The grove has heard of you. Go, before it closes on you.", "There is nothing for you here but our silence."],
    &["Bad for business, you are. Out.", "Go away.", "No."],
];

/// The well-thought-of, with no deed of legend yet.
const LIKED: [&[&str]; 4] = [
    &["You've a good name among our folk. Ask what you will.", "I've heard decent things of you. Sit."],
    &["Folk speak well of you, friend. What can I do?", "A friend of ours, I hear. Come in from the road."],
    &["Your name has been spoken kindly here.", "The grove knows you, a little. Be welcome."],
    &["You've done us favours, I hear. What do you want?", "Not an enemy, then. Good. Talk."],
];

/// Watched: told little (`Stance::Wary`).
const WARY: [&[&str]; 4] = [
    &["Hm. Your sort. Say your piece and move on.", "I'll talk. Keep your hands where I can see them."],
    &["We don't want trouble, stranger.", "You'll find no welcome here, but no harm either."],
    &["Your kind cut trees for fires. Speak, and be brief.", "I will answer a little. Then go."],
    &["Hmph. Watched, you are. Talk quick.", "Keep your distance, and we'll keep ours."],
];

const NO_NEWS: [&[&str]; 4] = [
    &["News? The seam on the east face ran out. That's all the news there is.", "Nothing worth the telling. The ale went sour, the ale came back."],
    &["Quiet times. The barley came up well, praise be.", "Nothing much. Old Bera's goat got loose again."],
    &["The seasons turn. That is news enough for us.", "A fawn was born under the old oak. Little else."],
    &["Nobody's died. Boring.", "No raids, no fights, no fun."],
];

/// News of the world: an opener, then the event.
const NEWS: [&[&str]; 4] = [
    &["A runner brought word:", "Heard it at the forge:", "Word from the halls:", "My cousin's letter says"],
    &["Word on the road is", "A merchant told me:", "Did you hear?", "They were saying at the inn:"],
    &["The wind carries news:", "The birds say", "A sister of the grove told me:", "It is whispered that"],
    &["Heard a thing, didn't I:", "Some fool said", "News, eh?", "Down the pits they say"],
];

const NO_RUMOURS: [&[&str]; 4] = [
    &["I keep to the halls. Ask a merchant.", "Rumours are for idlers. Though... no. Nothing."],
    &["Nothing I'd swear to, friend.", "Ask at the inn. I hear nothing out here."],
    &["The forest keeps its secrets this season.", "Nothing that would interest your kind."],
    &["Nothing worth stealing near here. Pity.", "Ask someone who cares."],
];

/// A fallen fort: `{place}` `{when}` `{killer}` `{treasure}`.
const FORT: [&[&str]; 4] = [
    &["{place} fell {when}{killer}. Our own kin, and their halls gone dark.", "Mark me: there's still {treasure} in the deep hall of {place}. It fell {when}{killer}.", "Nobody has gone back to {place} for the dead. Fell {when}{killer}. And {treasure} down there with them."],
    &["There was a dwarf hold called {place}. Fell {when}{killer}. They say {treasure} is still inside.", "They say {place} lies empty{killer}, with {treasure} in it for anyone brave or daft.", "Merchants won't go near {place} since it fell. They say {treasure} is still down there."],
    &["The bearded ones dug too deep at {place}. It fell {when}{killer}.", "{place} is silent now{killer}. The moss takes its gates, and {treasure} lies in the dark.", "Something waits in {place}. The birds will not nest there. They say {treasure} waits too."],
    &["{place}! Fell {when}{killer}. Who's got {treasure} now, eh?", "Easy pickings at {place}, if you don't mind the dead. {treasure}, they say.", "The dwarves left {treasure} in {place} when they died. Stupid dwarves."],
];

/// A beast of the legends in its lair: `{beast}` `{place}`.
const LAIR: [&[&str]; 4] = [
    &["{beast} lairs at {place}. Bring a good axe, and a better one after it breaks.", "Don't go near {place}. {beast} sleeps there on a bed of bones.", "Three of ours went to {place} after {beast}. One came back, and he doesn't talk."],
    &["{beast} has taken {place} for its lair. Sheep go missing.", "A hunter came back from {place} without his hair. {beast}, he said.", "There's a bounty on {beast}, at {place}. Nobody's collected."],
    &["{beast} is an old wrong in the world. It lairs at {place}.", "The forest flinches near {place}. {beast} lies there.", "We sing {beast} to sleep. Do not wake it, at {place}."],
    &["{beast} lives at {place}. Even we leave it alone.", "Kill {beast} at {place} and we'd sing about you. Badly, but we'd sing.", "{beast} ate our last chief at {place}. Good riddance, but still."],
];

/// A ruined site: `{place}` `{kind}`.
const RUIN: [&[&str]; 4] = [
    &["The ruin of {place} is older than my grandmother's grudges. Old stone, old gold maybe.", "Nobody keeps {place} now. Something might.", "{place} was a {kind} once. Good masonry, they say, under the weeds."],
    &["There's a ruin at {place}. The young ones dare each other to go. None do.", "Old {place} burned long ago. Folk say there's still coin in the cellars.", "Don't camp near {place}. My uncle did."],
    &["{place} was a {kind} once. Now it is a garden of nettles.", "Ghosts walk {place}, they say. Ghosts and worse.", "Leave {place} to the roots. Or don't. You won't listen."],
    &["{place}'s a ruin. We did that. Well, someone did.", "Picked clean, {place}. Mostly. Look for the bits we missed.", "{place}. Nice and quiet. Too quiet. You go first."],
];

/// A dead adventurer's bones: `{name}` `{land}` `{when}`.
const BONES: [&[&str]; 4] = [
    &["{name} went out a hero and never came home. Fell in {land}, {when}. Their gear's still lying there.", "We drank to {name}. Died in {land}. Someone should bring their axe home."],
    &["Poor {name}. Went to seek their fortune in {land}, found a grave instead.", "{name}'s bones are out in {land}, they say, with all their kit."],
    &["{name} fell in {land}, {when}. The grass grows through their bones.", "Another seeker, {name}, lies in {land}. You walk their road."],
    &["{name} died in {land}. All that gear, just lying there. I'd go, but I'm busy.", "{name}! Dead in {land}. Finders keepers."],
];

/// A saga the world still tells: `{title}`.
const SAGA: [&[&str]; 4] = [
    &["We still tell of {title} in the long halls.", "Have you heard {title}? Sit. It takes three ales."],
    &["The minstrels still sing {title}.", "You'll hear {title} at any inn worth its salt."],
    &["The grove remembers {title}, and weeps a little.", "{title}. We sing it in winter."],
    &["{title}! A good one. Lots of dying.", "We tell {title} to the whelps. They like the gory bits."],
];

/// The hero's own deeds, as gossip: `{name}` `{deed}`.
const OWN: [&[&str]; 4] = [
    &["There's a wanderer about, {name}, who {deed}. Wait... is that you?", "They say {name} {deed}. Strange days."],
    &["Have you heard of {name}? The one who {deed}? ...Oh. Oh!", "Everybody's talking about {name}, who {deed}."],
    &["A wanderer called {name} {deed}. The trees are restless with it.", "The singers have a new song: {name}, who {deed}."],
    &["Someone called {name} {deed}. I'd like to meet them. Or rob them.", "{name} {deed}. Hah! Good for them."],
];

/// Goblins: where to rob. `{place}`.
const RAID: [&str; 3] = ["{place} is fat and soft. Good raiding, if you're bold.", "They've a storehouse at {place} nobody guards at night. I didn't tell you.", "{place}. Sheep, ale, and slow guards."];

const NO_TALES: [&[&str]; 4] = [
    &["Tales? I cut stone, I don't tell stories.", "Ask the old ones. I've forgotten."],
    &["I'm no bard, friend.", "My grandmother knew all the old tales. She took them with her."],
    &["Some tales are not for your ears.", "The old songs sleep this year."],
    &["Tales are for the weak. ...Fine, I don't know any."],
];

/// Before the tales.
const TALE: [&[&str]; 4] = [
    &["Listen well. This is how it was:", "Our people tell it so:", "The old songs keep it right:"],
    &["The way my father told it:", "Here's one the minstrels sing:", "This is how we tell it, anyway:"],
    &["Hear it as the grove sings it:", "As the elders tell it:", "Hear, and remember:"],
    &["Here's how it went. Probably:", "We tell it like this:", "Listen, or don't:"],
];

const WORK: [&[&str]; 4] = [
    &["Work? Ask the overseer. I just haul rock.", "Not for an outsider. Not today."],
    &["Work? Not that I know of. Ask again when the season turns.", "Nothing I can pay for, friend."],
    &["The grove asks nothing of you. Today.", "Not yet. Be patient."],
    &["Work? Kill someone, rob someone. Not for me, though.", "Nothing. Go away."],
];

const JOIN: [&[&str]; 4] = [
    &["Leave my hall? For you? Maybe when they sing of you.", "I've work here. Ask someone with less sense."],
    &["Me? I've a family to feed.", "Go with you? Not on your life. Literally."],
    &["My roots are here, wanderer.", "The road is yours, not mine."],
    &["What's in it for me? Nothing. So no.", "Ha! No."],
];

const BYE: [&[&str]; 4] = [
    &["Stone keep you.", "Mind the dark.", "Don't die stupid."],
    &["Safe roads!", "Gods watch over you.", "Come back with stories!"],
    &["Walk softly.", "May the leaves hide you.", "Go well, short-lived one."],
    &["Yeah, yeah.", "Don't come back. Or do. Whatever.", "Watch your back."],
];

/// Their own story: `{first}` `{role}` `{home}`.
const SELF: [&[&str]; 4] = [
    &["{first}. {role} of {home}, and my mother before me.", "I'm {first}, {role} of {home}. Sixty years at it, and the stone still surprises me."],
    &["Name's {first}. {role} hereabouts, in {home}.", "{first}, {role} of {home}. Born here, likely die here."],
    &["I am {first}. {role} of {home}, these two hundred years.", "{first}. {role}. {home} is my grove."],
    &["{first}. {role}. {home}. Happy?", "I'm {first}, {role} of {home}. Best one too."],
];

/// A temper's touch on a line about themselves.
fn temper_line(p: &Person) -> &'static str {
    let t: &[&str] = match (p.temper, p.race) {
        (Temper::Warm, _) => &["Ask me anything, I like a chat.", "You look tired. Rest a while."],
        (Temper::Gruff, _) => &["That's all you're getting.", "Don't waste my day."],
        (Temper::Gloomy, Race::Elf) => &["The world grows smaller every age.", "Everything I loved is a song now."],
        (Temper::Gloomy, _) => &["Not that it matters. We all end in the ground.", "Bad years, these."],
        (Temper::Boastful, _) => &["Best in the land, I am. Ask anyone.", "You've heard of me, surely."],
    };
    stories::pick(p.h(77), 77, t)
}

/// "last year", "four years back", "in the year 203".
fn when_words(now: i32, year: i32) -> String {
    match now - year {
        i32::MIN..=0 => "this very year".into(),
        1 => "last year".into(),
        n @ 2..=12 => format!("{} years back", number(n as u8)),
        _ => format!("in the year {}", year),
    }
}

/// How long the road is: "half a day", "two days".
pub fn days_words(cells: i32) -> String {
    let d = (cells as f64 * 0.6).round() as i32;
    match d {
        0 if cells <= 1 => "half a day".into(),
        0 | 1 => "a day".into(),
        n => format!("{} days", number(n.min(13) as u8)),
    }
}

/// A creature that talks: people, not beasts.
fn is_person(sp: Species) -> bool {
    matches!(sp, Species::Visitor | Species::Merchant | Species::Human | Species::Elf | Species::Goblin | Species::Raider)
}

impl Game {
    /// Whom the hero could talk to here, nearest first: people within
    /// `REACH` (beasts don't count; hostile folk do, and won't answer).
    pub fn talk_choices(&self, hero: usize) -> Vec<u32> {
        let p = self.dwarves[hero].pos;
        let near = |q: crate::geom::Pos| q.z == p.z && (q.x - p.x).abs().max((q.y - p.y).abs()) <= REACH;
        let d2 = |q: crate::geom::Pos| (q.x - p.x).pow(2) + (q.y - p.y).pow(2);
        let mut v: Vec<(i32, u32)> = self.dwarves.iter().filter(|o| o.id != HERO_ID && o.alive && !o.ghost && near(o.pos)).map(|o| (d2(o.pos), o.id)).collect();
        v.extend(self.creatures.values().filter(|c| c.alive && is_person(c.species) && near(c.pos)).map(|c| (d2(c.pos), c.id)));
        v.sort();
        v.into_iter().map(|(_, id)| id).collect()
    }

    /// One question's time (`Action::Talk`): refused if they're gone, out
    /// of reach, or won't talk.
    pub(super) fn talk_act(&mut self, hero: usize, id: u32) -> Outcome {
        if !self.talk_choices(hero).contains(&id) {
            return Outcome::Refused("They are not near enough to talk to.".into());
        }
        if let Some(c) = self.creatures.get(&id) {
            if c.hostile() || c.siege {
                let name = c.label.clone().unwrap_or_else(|| format!("The {}", c.name()));
                let name = name.split(',').next().unwrap_or(&name).to_string();
                return Outcome::Refused(format!("{} answers only with a snarl and a raised weapon.", name));
            }
        }
        self.adv_run(hero, TALK_TICKS, false)
    }
}

impl Adventure {
    /// The site of the visit, if it is one of the world's.
    fn here_site(&self) -> Option<&crate::overworld::Site> {
        match self.visit.as_ref()?.key {
            SiteKey::Site(id) => self.ow().sites.iter().find(|s| s.id == id),
            _ => None,
        }
    }

    /// Who `id` is, as the hero can talk to them.
    pub fn person(&self, id: u32) -> Option<Person> {
        let v = self.visit.as_ref()?;
        let g = &v.g;
        let site = self.here_site();
        let site_race = site.and_then(|s| s.civ).and_then(|c| self.ow().civs.get(c as usize)).map(|c| c.race);
        let temper = |name: &str| [Temper::Warm, Temper::Gruff, Temper::Gloomy, Temper::Boastful][(stories::hash(name, id as u64 ^ 0x7A1C) % 4) as usize];
        if let Some(d) = g.dwarves.iter().find(|d| d.id == id && d.alive) {
            return Some(Person {
                id,
                name: d.name.clone(),
                role: "wanderer".into(),
                race: Race::Dwarf,
                civ: None,
                home: None,
                temper: temper(&d.name),
                travelled: true,
                memory: None,
                hostile: false,
                afraid: false,
            });
        }
        let c = g.creatures.get(&id).filter(|c| c.alive && is_person(c.species))?;
        let label = c.label.clone().unwrap_or_else(|| cap(c.name()));
        let (name, role) = match label.split_once(", ") {
            Some((n, r)) => (n.to_string(), r.to_string()),
            None => (label.clone(), c.name().to_string()),
        };
        let local = c.visitor == FOLK || c.visitor == GUARD;
        let race = match c.species {
            Species::Human => Race::Human,
            Species::Elf => Race::Elf,
            Species::Goblin => Race::Goblin,
            Species::Raider => Race::Dwarf,
            _ => site_race.unwrap_or(Race::Human),
        };
        let memory = c.desc.as_ref().and_then(|d| d.split_once("Remembers ")).map(|(_, m)| m.to_string());
        Some(Person {
            id,
            name,
            travelled: c.species == Species::Merchant || role == "bard" || !local,
            role,
            race,
            civ: if local { site.and_then(|s| s.civ) } else { None },
            home: if local { site.map(|s| s.id) } else { None },
            temper: temper(&label),
            memory,
            hostile: c.hostile() || c.siege,
            afraid: c.wary_until > g.tick,
        })
    }

    /// How the speaker's people take the hero.
    fn stance_of(&self, p: &Person) -> Stance {
        match p.home.and_then(|h| self.ow().sites.iter().find(|s| s.id == h)) {
            Some(s) => stance(self.ow(), self.hero.race, self.hero.civ, s),
            None => Stance::Friendly,
        }
    }

    fn standing_with(&self, p: &Person) -> i32 {
        p.civ.and_then(|c| self.fame.by_civ.get(&c)).copied().unwrap_or(0)
    }

    /// The deed the hero is known for: their weightiest, said as "slew
    /// Ashmaw the dragon".
    pub fn known_for(&self) -> Option<String> {
        let d = self.fame.deeds.iter().filter(|d| d.weight > 0).max_by_key(|d| (d.weight, d.tick))?;
        let t = d.text.split(", and was hailed").next().unwrap_or(&d.text);
        let t = t.split(", of the old legends").next().unwrap_or(t);
        Some(t.to_string())
    }

    /// Famous enough to be known on sight by `p`'s people.
    fn famous_to(&self, p: &Person) -> bool {
        self.fame.renown >= self.knob("talk.famous", 3.0) as i32 && self.standing_with(p) >= 0
    }

    /// Infamous with `p`'s people: they turn the hero away.
    fn shunned_by(&self, p: &Person) -> bool {
        // Goblins think no worse of a killer.
        p.race != Race::Goblin && (self.standing_with(p) <= -(self.knob("talk.shun_standing", 10.0) as i32) || self.fame.infamy >= self.knob("talk.shun_infamy", 9.0) as i32)
    }

    /// The conversation's start: the greeting, read from the hero's fame
    /// (or why there is none).
    pub fn greet(&self, p: &Person) -> Chat {
        let mut lines = Vec::new();
        let mut closed = false;
        let called = self.hero_dwarf().called();
        if p.hostile {
            lines.push((Said::Aside, format!("{} glares at you, weapon ready. There will be no talk.", p.first())));
            closed = true;
        } else if p.afraid {
            lines.push((Said::Aside, format!("{} backs away from you.", p.first())));
            lines.push((Said::Them, "Keep away! Keep away from me!".into()));
            closed = true;
        } else if self.shunned_by(p) {
            lines.push((Said::Them, voice(p, 3, &SHUNNED).to_string()));
            closed = true;
        } else if let (true, Some(deed)) = (self.famous_to(p), self.known_for()) {
            lines.push((Said::Them, fill(voice(p, 2, &FAMOUS), &[("name", &called), ("deed", &deed)])));
            if p.race == Race::Goblin && self.fame.infamy > 0 {
                lines.push((Said::Them, "And the killing! Lovely work.".into()));
            }
        } else if self.stance_of(p) == Stance::Wary {
            lines.push((Said::Them, voice(p, 4, &WARY).to_string()));
        } else if self.standing_with(p) >= 2 {
            lines.push((Said::Them, voice(p, 5, &LIKED).to_string()));
        } else {
            lines.push((Said::Them, voice(p, 1, &HELLO).to_string()));
        }
        Chat { who: p.clone(), lines, asked: Vec::new(), closed }
    }

    /// The topics the menu offers now.
    pub fn topics(&self, chat: &Chat) -> Vec<Topic> {
        if chat.closed {
            return vec![Topic::Farewell];
        }
        TOPICS.to_vec()
    }

    /// Ask: the hero's question and the answer go into the chat; places
    /// heard of are marked on the map.
    pub fn ask(&mut self, chat: &mut Chat, t: Topic) {
        chat.lines.push((Said::Hero, t.ask().to_string()));
        if !chat.asked.contains(&t) {
            chat.asked.push(t);
        }
        let p = chat.who.clone();
        let said: Vec<String> = match t {
            Topic::News => self.news(&p),
            Topic::Rumours => {
                let rs = self.rumours(&p, self.at);
                if rs.is_empty() {
                    vec![voice(&p, 11, &NO_RUMOURS).to_string()]
                } else {
                    for r in &rs {
                        chat.lines.push((Said::Them, r.text.clone()));
                        if let Some(name) = self.hear(r) {
                            chat.lines.push((Said::Marked, format!("{} is marked on your map (!).", cap(&name))));
                        }
                    }
                    Vec::new()
                }
            }
            Topic::Legends => self.legends(&p),
            Topic::Directions => self.directions(),
            Topic::Yourself => self.about_self(&p),
            Topic::Work => self.quest_talk(&p).unwrap_or_else(|| vec![voice(&p, 21, &WORK).to_string()]),
            Topic::Join => self.company_talk(&p).unwrap_or_else(|| vec![voice(&p, 22, &JOIN).to_string()]),
            Topic::Farewell => vec![voice(&p, 23, &BYE).to_string()],
        };
        chat.lines.extend(said.into_iter().map(|s| (Said::Them, s)));
    }

    /// A rumour heard: its place marked. The place's name if it is new.
    pub fn hear(&mut self, r: &Rumour) -> Option<String> {
        let id = r.about?.known_id(self.ow())?;
        if !self.hero.known.insert(id) {
            return None;
        }
        let name = self.known_place(id).map(|k| k.1)?;
        self.say(&format!("You have heard of {}. It is marked on your map.", name), (255, 215, 90));
        Some(name)
    }

    /// A known place: its cell and name.
    pub fn known_place(&self, id: u32) -> Option<((i32, i32), String)> {
        let ow = self.ow();
        if id & KNOWN_FORT != 0 {
            let f = ow.forts.get((id & !KNOWN_FORT) as usize)?;
            Some((f.site, f.name.clone()))
        } else if id & KNOWN_REMAINS != 0 {
            let r = ow.remains.get((id & !KNOWN_REMAINS) as usize)?;
            Some(((r.x, r.y), format!("the bones of {}", r.name)))
        } else {
            let s = ow.sites.iter().find(|s| s.id == id)?;
            Some(((s.x, s.y), s.name.clone()))
        }
    }

    /// Every place the hero has heard of, with its cell and name.
    pub fn known_places(&self) -> Vec<(u32, (i32, i32), String)> {
        self.hero.known.iter().filter_map(|&id| self.known_place(id).map(|(at, n)| (id, at, n))).collect()
    }

    /// How far a speaker's knowledge reaches, in world cells.
    fn reach(&self, p: &Person) -> i32 {
        let r = self.knob("talk.radius", 15.0) as i32;
        if p.travelled {
            (r as f64 * self.knob("talk.travelled_mult", 2.7).max(1.0)) as i32
        } else {
            r
        }
    }

    /// What a speaker at `at` has heard: places worth the road, the
    /// weightiest first (five; two if they're wary of the hero). Fallen
    /// forts, lairs of the legends' beasts, ruins, the bones of heroes,
    /// sagas still told, where goblins would rob, and, once the hero is
    /// famous, the hero's own deeds.
    pub fn rumours(&self, p: &Person, at: (i32, i32)) -> Vec<Rumour> {
        let ow = self.ow();
        let r = self.reach(p);
        let year = self.year();
        // Drafts: the words are chosen once the five are, so one speaker
        // telling of two ruins doesn't say the same of both.
        let mut v: Vec<Draft> = Vec::new();
        let near = |d: i32| 100 * (r + 1) / (d + 2);
        let way = |to: (i32, i32)| format!(" {} {}.", cap(&days_words(cheb(to, at))), way_to(at, to));
        // A people talks most of what it cares for: dwarves of forts, elves
        // of beasts, goblins of ruins to pick over, humans of lost heroes.
        let likes = |kind: usize| -> i32 {
            match (p.race, kind) {
                (Race::Dwarf, 0) | (Race::Elf, 1) | (Race::Goblin, 2) | (Race::Human, 3) => 3,
                (Race::Dwarf, 2) | (Race::Elf, 4) | (Race::Goblin, 0) | (Race::Human, 1) => 2,
                _ => 1,
            }
        };
        for (i, f) in ow.forts.iter().enumerate() {
            let Some(fell) = f.fell else { continue };
            let d = cheb(f.site, at);
            if d > r {
                continue;
            }
            let taken: Vec<&String> = f.robbed.iter().flat_map(|rb| rb.took.iter()).collect();
            let left: Vec<&String> = f.artifacts.iter().filter(|a| !taken.contains(a)).collect();
            let treasure = match (left.first(), f.robbed.last()) {
                (Some(a), _) => a.to_string(),
                (None, Some(rb)) if !f.artifacts.is_empty() => format!("nothing now, since {} robbed it,", rb.by),
                _ => "its founders' gold".to_string(),
            };
            let killer = match (&f.occupant, &f.killer) {
                (Some(Occupant::Beast { figure }), _) => ow.figures.get(*figure as usize).map_or(String::new(), |b| format!(", and {} the {} lairs in it now", b.name, b.kind)),
                (Some(Occupant::Horror { label }), _) => format!(", and {} haunts it", label),
                (_, Some(k)) => format!(", the last of them slain by {}", a_an(k)),
                _ => String::new(),
            };
            let args = vec![("place", f.name.clone()), ("when", when_words(year, fell)), ("killer", killer), ("treasure", treasure)];
            v.push(Draft { kind: 0, args, tail: way(f.site), about: Some(About::Fort(i)), weight: near(d) * (2 + left.len() as i32).min(5) * likes(0) });
        }
        for fid in ow.beasts_near(at.0, at.1, r) {
            let b = &ow.figures[fid as usize];
            let Some(lair) = b.lair.and_then(|l| ow.sites.get(l as usize)) else { continue };
            let d = cheb((lair.x, lair.y), at);
            let beast = format!("{} the {}", b.name, b.kind);
            let place = format!("{}, in {}", lair.name, land_name(ow, lair.x, lair.y));
            v.push(Draft { kind: 1, args: vec![("beast", beast), ("place", place)], tail: way((lair.x, lair.y)), about: Some(About::Figure(fid)), weight: near(d) * (3 + b.kills.min(4) as i32) * likes(1) / 2 });
        }
        for s in ow.sites.iter().filter(|s| s.ruined && s.kind != SiteKind::Lair) {
            let d = cheb((s.x, s.y), at);
            if d > r || ow.fort_at(s.x, s.y).is_some() {
                continue;
            }
            v.push(Draft { kind: 2, args: vec![("place", s.name.clone()), ("kind", s.kind.name().to_string())], tail: way((s.x, s.y)), about: Some(About::Site(s.id)), weight: near(d) * 2 * likes(2) });
        }
        for (i, b) in ow.remains.iter().enumerate() {
            let d = cheb((b.x, b.y), at);
            if d > r {
                continue;
            }
            let land = land_name(ow, b.x, b.y);
            let args = vec![("name", b.name.clone()), ("land", land), ("when", when_words(year, b.year))];
            v.push(Draft { kind: 3, args, tail: way((b.x, b.y)), about: Some(About::Remains(i)), weight: near(d) * (1 + b.gear.len().min(4) as i32) * likes(3) });
        }
        for s in ow.sagas.iter().rev().take(6) {
            v.push(Draft { kind: 4, args: vec![("title", s.title.clone())], tail: String::new(), about: None, weight: 40 * likes(4) });
        }
        if p.race == Race::Goblin {
            let raid = ow
                .sites
                .iter()
                .filter(|s| !s.ruined && s.kind != SiteKind::Lair && s.civ.and_then(|c| ow.civs.get(c as usize)).map_or(false, |c| c.race != Race::Goblin))
                .filter(|s| cheb((s.x, s.y), at) <= r && (s.x, s.y) != at)
                .min_by_key(|s| (cheb((s.x, s.y), at), s.id));
            if let Some(s) = raid {
                let d = cheb((s.x, s.y), at);
                v.push(Draft { kind: 5, args: vec![("place", s.name.clone())], tail: way((s.x, s.y)), about: Some(About::Site(s.id)), weight: near(d) * 4 });
            }
        }
        if self.fame.renown > self.knob("talk.renown_heard", 10.0) as i32 {
            if let Some(deed) = self.known_for() {
                v.push(Draft { kind: 6, args: vec![("name", self.hero_dwarf().called()), ("deed", deed)], tail: String::new(), about: None, weight: 1000 });
            }
        }
        v.sort_by(|a, b| b.weight.cmp(&a.weight).then(a.args.cmp(&b.args)));
        let n = if self.stance_of(p) == Stance::Wary { 2 } else { 5 };
        v.truncate(n);
        let mut told = [0usize; 7];
        v.into_iter()
            .map(|d| {
                let t = match d.kind {
                    5 => RAID[(p.h(130) as usize + told[5]) % RAID.len()],
                    k => voice_n(p, 30 + 20 * k as u64, [&FORT, &LAIR, &RUIN, &BONES, &SAGA, &OWN, &OWN][k], told[k]),
                };
                told[d.kind] += 1;
                let args: Vec<(&str, &str)> = d.args.iter().map(|(k, v)| (*k, v.as_str())).collect();
                Rumour { text: cap(&format!("{}{}", fill(t, &args), d.tail)), about: d.about, weight: d.weight }
            })
            .collect()
    }

    /// News of the world: the latest events near the speaker, or of their
    /// people, at most three.
    pub fn news(&self, p: &Person) -> Vec<String> {
        let ow = self.ow();
        let r = self.reach(p);
        let at = self.at;
        let last = ow.events.iter().map(|e| e.year).max().unwrap_or(0);
        let near: Vec<&str> = ow.sites.iter().filter(|s| cheb((s.x, s.y), at) <= r).map(|s| s.name.as_str()).collect();
        let civ = p.civ.and_then(|c| ow.civs.get(c as usize)).map(|c| c.name.clone());
        let mut lines = Vec::new();
        for (i, e) in ow.events.iter().enumerate().rev() {
            if e.year < last - self.knob("talk.news_years", 12.0) as i32 || lines.len() >= 3 {
                break;
            }
            let ours = civ.as_ref().map_or(false, |c| e.text.contains(c.as_str()));
            if e.bard || !(ours || near.iter().any(|n| e.text.contains(n))) {
                continue;
            }
            let open = voice(p, 200 + i as u64, &NEWS);
            lines.push(format!("{} {} That was {}.", open, lower_first(&e.text), when_words(self.year(), e.year).trim_start_matches("in ")));
        }
        if lines.is_empty() {
            lines.push(voice(p, 9, &NO_NEWS).to_string());
        }
        lines
    }

    /// The old tales, as the speaker's people tell them this year
    /// (`tales::tale_for`): of their home, of the beasts near, of their
    /// people. A dwarf and a human tell the same deed differently.
    pub fn legends(&self, p: &Person) -> Vec<String> {
        let ow = self.ow();
        let year = self.year();
        let mut names: Vec<String> = Vec::new();
        if let Some(s) = p.home.and_then(|h| ow.sites.iter().find(|s| s.id == h)) {
            names.push(s.name.clone());
        }
        for f in ow.beasts_near(self.at.0, self.at.1, self.reach(p)).into_iter().take(2) {
            names.push(ow.figures[f as usize].name.clone());
        }
        if let Some(c) = p.civ.and_then(|c| ow.civs.get(c as usize)) {
            names.push(c.name.clone());
        }
        let mut seen: Vec<usize> = Vec::new();
        let mut told: Vec<String> = Vec::new();
        for n in &names {
            let old = tales::tales_about(ow, n, p.civ, year).into_iter().filter(|(i, _)| !seen.contains(i) && ow.events[*i].year <= year - 5);
            // The oldest tale of each: the most retold.
            if let Some((i, t)) = old.last() {
                seen.push(i);
                told.push(t);
            }
            if told.len() >= 3 {
                break;
            }
        }
        if told.is_empty() {
            return vec![voice(p, 31, &NO_TALES).to_string()];
        }
        let mut out = vec![voice(p, 32, &TALE).to_string()];
        out.extend(told);
        out
    }

    /// "Where is ...?": the way to every place the hero has heard of,
    /// nearest first; with none, the nearest towns.
    pub fn directions(&self) -> Vec<String> {
        let ow = self.ow();
        let at = self.at;
        let mut places: Vec<((i32, i32), String)> = self.known_places().into_iter().map(|(_, c, n)| (c, n)).collect();
        let heard = !places.is_empty();
        if !heard {
            places = ow.sites.iter().filter(|s| !s.ruined && s.kind != SiteKind::Lair && (s.x, s.y) != at).map(|s| ((s.x, s.y), s.name.clone())).collect();
        }
        places.sort_by_key(|(c, _)| cheb(*c, at));
        places.truncate(if heard { 6 } else { 2 });
        if places.is_empty() {
            return vec!["Where? There's nothing out there but the wild.".into()];
        }
        places
            .into_iter()
            .map(|(c, n)| {
                let d = cheb(c, at);
                if d == 0 {
                    return format!("{}? You're standing in it.", cap(&n));
                }
                let mid = ((at.0 + c.0) / 2, (at.1 + c.1) / 2);
                let past = land_name(ow, mid.0, mid.1);
                let past = if d >= 3 && past != land_name(ow, at.0, at.1) { format!(", past {}", past) } else { String::new() };
                format!("{}? {} {}{}.", cap(&n), cap(&days_words(d)), way_to(at, c), past)
            })
            .collect()
    }

    /// Before setting out, the bot (A18) asks one of the home folk for
    /// rumours, as a player would: the places heard of are marked, and the
    /// first rumour is the line for its story. No time passes, no dice.
    pub fn hear_at_home(&mut self) -> Option<String> {
        let v = self.visit.as_ref()?;
        let id = v.g.creatures.values().filter(|c| c.alive && c.visitor == FOLK && !c.hostile() && !c.siege).map(|c| c.id).min()?;
        let p = self.person(id)?;
        let chat = self.greet(&p);
        if chat.closed {
            return None;
        }
        let rs = self.rumours(&p, self.at);
        for r in &rs {
            self.hear(r);
        }
        rs.first().map(|r| format!("{}, {}, says: \"{}\"", p.name, p.role, r.text))
    }

    /// "Tell me of yourself": name, role, home, a memory, a temper.
    pub fn about_self(&self, p: &Person) -> Vec<String> {
        let home = p.home.and_then(|h| self.ow().sites.iter().find(|s| s.id == h)).map_or("the road".to_string(), |s| s.name.clone());
        let role = p.role.split(" of ").next().unwrap_or(&p.role).to_string();
        let mut v = vec![fill(voice(p, 40, &SELF), &[("first", p.first()), ("role", &cap(&role)), ("home", &home)])];
        if let Some(m) = &p.memory {
            v.push(format!("I remember {}", m));
        }
        v.push(temper_line(p).to_string());
        v
    }
}

/// "The goblins razed ..." in the middle of a sentence.
fn lower_first(s: &str) -> String {
    match s.strip_prefix("The ") {
        Some(rest) => format!("the {}", rest),
        None => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::sitegen::town::tests::{visit, world};
    use super::*;
    use crate::overworld::FortMark;

    /// A hero of a town's own civ, in that town (not a goblin one).
    fn town_visit(seed: u64) -> Adventure {
        let ow = world();
        let s = ow
            .sites
            .iter()
            .find(|s| !s.ruined && matches!(s.kind, SiteKind::Town | SiteKind::Hamlet | SiteKind::Fortress) && s.civ.and_then(|c| ow.civs.get(c as usize)).map_or(false, |c| c.race != Race::Goblin))
            .unwrap()
            .clone();
        let race = ow.civs[s.civ.unwrap() as usize].race;
        visit(s.id, race, s.civ, seed)
    }

    fn a_folk(a: &Adventure) -> u32 {
        let v = a.visit.as_ref().unwrap();
        let mut ids: Vec<u32> = v.g.creatures.values().filter(|c| c.visitor == FOLK && c.alive).map(|c| c.id).collect();
        ids.sort();
        ids[0]
    }

    #[test]
    fn a_town_near_a_fallen_fort_talks_of_it() {
        let mut a = town_visit(31);
        let (x, y) = a.at;
        let ow = a.ow_mut();
        let fx = if x + 3 < ow.w { x + 3 } else { x - 3 };
        ow.forts.push(FortMark {
            name: "Copperhold".into(),
            site: (fx, y),
            civ: None,
            founded: 250,
            fell: Some(253),
            killer: Some("goblin".into()),
            artifacts: vec!["Ironmoon, a steel axe".into()],
            save_file: "copperhold.json".into(),
            peak_pop: 30,
            journal: Vec::new(),
            occupant: None,
            robbed: Vec::new(),
        });
        let fort = a.ow().forts.len() - 1;
        let p = a.person(a_folk(&a)).unwrap();
        let rs = a.rumours(&p, a.at);
        assert!(rs.len() <= 5);
        let r = rs.iter().find(|r| r.about == Some(About::Fort(fort))).unwrap_or_else(|| panic!("the fort among the first rumours: {:?}", rs));
        assert!(r.text.contains("Copperhold") && (r.text.contains("east") || r.text.contains("west")), "{}", r.text);
        // Asking marks it known, and on the map.
        let mut chat = a.greet(&p);
        assert!(!chat.closed);
        a.ask(&mut chat, Topic::Rumours);
        assert!(a.hero.known.contains(&(KNOWN_FORT | fort as u32)));
        assert!(chat.lines.iter().any(|(s, l)| *s == Said::Marked && l.contains("Copperhold")), "{:?}", chat.lines);
        assert!(a.known_places().iter().any(|(_, c, n)| *c == (fx, y) && n == "Copperhold"));
        // Hearing again marks nothing new; and the way there is told.
        a.ask(&mut chat, Topic::Rumours);
        assert_eq!(chat.lines.iter().filter(|(s, l)| *s == Said::Marked && l.contains("Copperhold")).count(), 1);
        let dirs = a.directions();
        assert!(dirs.iter().any(|l| l.starts_with("Copperhold?") && l.contains("day")), "{:?}", dirs);
    }

    #[test]
    fn the_famous_are_greeted_by_name() {
        let mut a = town_visit(32);
        let p = a.person(a_folk(&a)).unwrap();
        let plain = a.greet(&p);
        assert!(!plain.lines[0].1.contains("Ashmaw"));
        a.deed("slew Ashmaw the dragon, of the old legends", 3);
        let chat = a.greet(&p);
        let hello = &chat.lines[0].1;
        assert!(hello.contains("slew Ashmaw the dragon") && hello.contains(&a.hero_dwarf().called()), "{}", hello);
        assert!(!hello.contains("old legends"));
        // The same words every time: hashed, not rolled.
        assert_eq!(a.greet(&p).lines, chat.lines);
        // A murderer is turned away (unless the speaker is a goblin).
        a.fame.infamy = 20;
        let shunned = a.greet(&p);
        if p.race != Race::Goblin {
            assert!(shunned.closed);
            assert_eq!(a.topics(&shunned), vec![Topic::Farewell]);
        }
    }

    #[test]
    fn each_people_tells_the_old_tale_its_own_way() {
        let a = town_visit(33);
        let ow = a.ow();
        let civ_of = |r: Race| ow.civs.iter().find(|c| c.race == r).map(|c| c.id);
        let (Some(d), Some(h)) = (civ_of(Race::Dwarf), civ_of(Race::Human)) else { return };
        let year = a.year();
        let differs = (0..ow.events.len()).filter(|&i| ow.events[i].year < year - 50).any(|i| tales::tale_for(ow, i, Some(d), year) != tales::tale_for(ow, i, Some(h), year));
        assert!(differs, "a dwarf and a human tell some deed differently");
        let p = a.person(a_folk(&a)).unwrap();
        assert!(!a.legends(&p).is_empty());
    }

    #[test]
    fn hostile_folk_will_not_talk() {
        let mut a = town_visit(34);
        let id = a_folk(&a);
        let v = a.visit.as_mut().unwrap();
        let h = v.hero;
        let hp = v.g.dwarves[h].pos;
        let c = v.g.creatures.get_mut(&id).unwrap();
        c.pos = hp.offset(1, 0, 0);
        c.siege = true;
        let out = v.g.talk_act(h, id);
        assert!(matches!(out, Outcome::Refused(ref w) if w.contains("snarl")), "{:?}", out);
        let p = a.person(id).unwrap();
        assert!(a.greet(&p).closed);
        // A peaceful one, near: a question takes a moment.
        let v = a.visit.as_mut().unwrap();
        let c = v.g.creatures.get_mut(&id).unwrap();
        c.siege = false;
        c.target = None;
        c.pos = hp.offset(1, 0, 0);
        c.path.clear();
        let t0 = v.g.tick;
        assert!(v.g.talk_choices(h).contains(&id));
        assert!(matches!(v.g.talk_act(h, id), Outcome::Done { .. }));
        assert!(v.g.tick > t0);
    }

    #[test]
    fn every_topic_has_an_answer() {
        let mut a = town_visit(35);
        let p = a.person(a_folk(&a)).unwrap();
        let mut chat = a.greet(&p);
        for t in TOPICS {
            let before = chat.lines.len();
            a.ask(&mut chat, t);
            assert!(chat.lines.len() >= before + 2, "{:?} answered: {:?}", t, &chat.lines[before..]);
            assert!(chat.lines[before + 1..].iter().all(|(_, l)| !l.contains('{')), "{:?}", &chat.lines[before..]);
        }
    }
}
