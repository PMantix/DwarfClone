//! Dreams: sleeping dwarves dream of what they lived through, of the dead,
//! of their people's music, or of nothing much. Now and then a dream is an
//! omen: of an army at the gates (and the next season the army comes), or
//! of a shape locked in the stone (and the dreamer is taken by a fey mood).
//! A dream that comes true goes into the legends and the chronicle.
//!
//! Every roll is on the culture's own stream (`culture.rs`).

use super::*;
use crate::arts::FormKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OmenKind {
    /// An army at the gates.
    Siege,
    /// A shape in the stone, calling: a fey mood.
    Mood,
}

/// A dream that foretells.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Omen {
    pub kind: OmenKind,
    pub dreamer: u32,
    pub name: String,
    /// When it was dreamt.
    pub tick: u64,
    /// What they dreamt ("an army of shadows at the gates").
    pub text: String,
    /// When it should come to pass (a mood), 0 if it's not up to us.
    #[serde(default)]
    pub due: u64,
    #[serde(default)]
    pub fulfilled: bool,
}

const SIEGE_DREAMS: [&str; 4] = ["an army of shadows at the gates", "the great door split by a ram of black iron", "the halls filled with the beating of war drums", "blood running down the stairs from the surface"];
const MOOD_DREAMS: [&str; 3] = ["a shape locked in the stone, calling to them", "their hands working by themselves at a thing of terrible beauty", "a voice in the rock that whispered how to make it"];
const IDLE_DREAMS: [(&str, i32); 6] = [
    ("falling forever down a shaft", 3),
    ("a tunnel that never ended", 2),
    ("a feast that never ran out of ale", -3),
    ("a mountain made all of gold", -3),
    ("being lost in the forest under the open sky", 4),
    ("the smell of the forge in the morning", -2),
];

impl Game {
    /// Every now and then: some of the sleepers dream; omens come due.
    pub(super) fn dreams_tick(&mut self) {
        self.omens_come_true();
        let tick = self.tick;
        let sleepers: Vec<usize> = (0..self.dwarves.len())
            .filter(|&i| {
                let d = &self.dwarves[i];
                d.alive && d.asleep && !d.ghost && !d.insane && self.culture.dreamt.get(&d.id).map_or(true, |&t| tick >= t + DAY / 2)
            })
            .collect();
        for i in sleepers {
            if self.c_roll() < self.knob("culture.dream", 0.2) {
                self.dream(i);
            }
        }
    }

    /// Dwarf `i` dreams.
    pub(super) fn dream(&mut self, i: usize) {
        let tick = self.tick;
        let id = self.dwarves[i].id;
        self.culture.dreamt.insert(id, tick);
        self.count("culture.dreams", 1);
        // An army coming, if the fortress has foes and it's late enough for one.
        let foes = self.neighbors.iter().any(|n| self.at_war_with(n));
        let pending = |g: &Game, k: OmenKind| g.culture.omens.iter().any(|o| o.kind == k && !o.fulfilled && tick < o.tick + SEASON * 2);
        let siege_p = if self.siege_warned || self.incoming.is_some() { 0.3 } else { self.knob("culture.omen_siege", 0.01) };
        if foes && tick + SEASON / 2 >= SEASON * 2 && !pending(self, OmenKind::Siege) && self.c_roll() < siege_p {
            let text = SIEGE_DREAMS[self.c_pick(SIEGE_DREAMS.len())].to_string();
            // What's dreamt will be: the next season brings them for certain.
            self.siege_warned = true;
            self.omen(i, OmenKind::Siege, text, 0);
            return;
        }
        let moody = self.dwarves.iter().any(|d| d.alive && d.mood.is_some());
        let d = &self.dwarves[i];
        if !moody && !d.is_child() && d.mood.is_none() && !d.soldier && !pending(self, OmenKind::Mood) && self.c_roll() < self.knob("culture.omen_mood", 0.01) {
            let text = MOOD_DREAMS[self.c_pick(MOOD_DREAMS.len())].to_string();
            let due = tick + DAY * (2 + self.c_pick(6) as u64);
            self.omen(i, OmenKind::Mood, text, due);
            return;
        }
        // An ordinary dream: something they lived through, a friend now dead,
        // their people's music, or nothing much.
        let fort = self.fortress_name.clone();
        let lived = self.lived_through(id);
        let dead: Vec<String> = self.dwarves[i]
            .relations
            .iter()
            .filter(|r| r.1 > 20)
            .filter_map(|r| self.dwarves.iter().find(|o| o.id == r.0 && !o.alive))
            .map(|o| o.name.clone())
            .collect();
        let roll = self.c_roll();
        let (text, stress) = if !dead.is_empty() && roll < 0.3 {
            (format!("dreamed of {}, who is dead", dead[self.c_pick(dead.len())]), 4)
        } else if !lived.is_empty() && roll < 0.6 {
            let e = lived[self.c_pick(lived.len())];
            let (_, how) = super::works::subject_of(&self.overworld.events[e].text, &fort);
            (format!("dreamed again of how {}", how), 1)
        } else if roll < 0.75 {
            let arts = self.our_arts();
            match arts.of_kind(FormKind::Dance).first() {
                Some(dance) => (format!("dreamed of dancing the {} in a hall of gold", dance.name), -4),
                None => ("dreamed of music in a hall of gold".into(), -4),
            }
        } else {
            let (t, s) = IDLE_DREAMS[self.c_pick(IDLE_DREAMS.len())];
            (format!("dreamed of {}", t), s)
        };
        self.dwarves[i].think(&text, stress, tick);
    }

    fn omen(&mut self, i: usize, kind: OmenKind, text: String, due: u64) {
        let tick = self.tick;
        let name = self.dwarves[i].name.clone();
        let id = self.dwarves[i].id;
        self.dwarves[i].think(&format!("dreamed of {}", text), 6, tick);
        self.count("culture.omens", 1);
        self.msg(&format!("{} wakes shaking from a dream of {}.", name, text), (190, 160, 255));
        self.culture.omens.push(Omen { kind, dreamer: id, name, tick, text, due, fulfilled: false });
    }

    /// Omens that come to pass: the army the dreamer saw is sighted; the
    /// shape in the stone takes hold of them.
    pub(super) fn omens_come_true(&mut self) {
        let tick = self.tick;
        // Forget the old and unfulfilled.
        self.culture.omens.retain(|o| o.fulfilled || tick < o.tick + YEAR);
        let fort = self.fortress_name.clone();
        let seen = self.incoming.is_some();
        if seen && !self.culture.siege_seen {
            let foe = self.incoming.as_ref().map(|i| self.overworld.civ(i.1.civ).name.clone()).unwrap_or_default();
            if let Some(o) = self.culture.omens.iter_mut().find(|o| o.kind == OmenKind::Siege && !o.fulfilled && tick < o.tick + SEASON * 2) {
                o.fulfilled = true;
                let (name, text) = (o.name.clone(), o.text.clone());
                self.come_true(&name, format!("{}'s dream of {} came true when the {} marched on {}.", name, text, foe, fort));
            }
        }
        self.culture.siege_seen = seen;
        // The dreamer of the shape in the stone, when its time comes.
        let due: Vec<usize> = (0..self.culture.omens.len()).filter(|&k| { let o = &self.culture.omens[k]; o.kind == OmenKind::Mood && !o.fulfilled && o.due > 0 && tick >= o.due }).collect();
        for k in due {
            let o = self.culture.omens[k].clone();
            let Some(i) = self.dwarves.iter().position(|d| d.id == o.dreamer) else { continue };
            let d = &self.dwarves[i];
            let free = d.alive && !d.insane && d.mood.is_none() && !self.dwarves.iter().any(|x| x.alive && x.mood.is_some());
            if !free {
                // Not now: it waits a day.
                self.culture.omens[k].due = tick + DAY;
                continue;
            }
            self.culture.omens[k].fulfilled = true;
            self.start_mood(i);
            self.come_true(&o.name, format!("{} of {} was taken by a fey mood, as their dream of {} had foretold.", o.name, fort, o.text));
        }
    }

    fn come_true(&mut self, name: &str, legend: String) {
        self.count("culture.omens_true", 1);
        self.msg(&format!("{}'s dream has come true!", name), (190, 160, 255));
        self.spotlight("A dream come true", &legend, None, (190, 160, 255));
        self.c_legend(legend);
    }

    /// Omens dreamt and not yet come to pass, for the dwarf's sheet.
    pub fn omen_of(&self, dwarf: u32) -> Option<String> {
        self.culture.omens.iter().rev().find(|o| o.dreamer == dwarf).map(|o| {
            if o.fulfilled {
                format!("Dreamt of {}, and it came true", o.text)
            } else {
                format!("Has dreamt of {}", o.text)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asleep(g: &mut Game) -> usize {
        let i = g.dwarves.iter().position(|d| !d.is_child() && !d.soldier).unwrap();
        g.dwarves[i].asleep = true;
        i
    }

    #[test]
    fn dwarves_dream_of_what_they_lived_through() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let fort = g.fortress_name.clone();
        g.legend(format!("A great cave spider was slain at {}.", fort));
        g.remember();
        let i = asleep(&mut g);
        g.knobs.set.insert("culture.omen_siege".into(), 0.0);
        g.knobs.set.insert("culture.omen_mood".into(), 0.0);
        for _ in 0..40 {
            g.dream(i);
        }
        let dreams: Vec<&str> = g.dwarves[i].thoughts.iter().map(|t| t.text.as_str()).filter(|t| t.starts_with("dreamed")).collect();
        assert!(dreams.iter().any(|t| t.contains("cave spider was slain")), "{:?}", dreams);
        assert!(g.culture.omens.is_empty());
        // Only once a night.
        g.dwarves[i].thoughts.clear();
        g.dreams_tick();
        assert!(g.dwarves[i].thoughts.is_empty());
    }

    #[test]
    fn a_dream_of_war_comes_true_when_the_army_is_sighted() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        assert!(g.neighbors.iter().any(|n| g.at_war_with(n)), "this world has no foes");
        g.tick = SEASON * 2 + 10;
        let i = asleep(&mut g);
        g.knobs.set.insert("culture.omen_siege".into(), 1.0);
        g.dream(i);
        assert_eq!(g.culture.omens.len(), 1);
        assert_eq!(g.culture.omens[0].kind, OmenKind::Siege);
        assert!(g.siege_warned, "what's dreamt will be");
        assert!(g.log.iter().any(|l| l.text.contains("wakes shaking from a dream")));
        // The season turns: they come, and the dream is remembered.
        g.maybe_siege();
        assert!(g.incoming.is_some());
        g.omens_come_true();
        assert!(g.culture.omens[0].fulfilled);
        let name = g.dwarves[i].name.clone();
        assert!(g.overworld.events.last().unwrap().text.starts_with(&format!("{}'s dream of", name)));
        assert!(g.omen_of(g.dwarves[i].id).unwrap().ends_with("it came true"));
        // Only the once.
        let n = g.overworld.events.len();
        g.omens_come_true();
        assert_eq!(g.overworld.events.len(), n);
    }

    #[test]
    fn a_dream_of_the_shape_in_the_stone_brings_a_fey_mood() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let i = asleep(&mut g);
        g.knobs.set.insert("culture.omen_siege".into(), 0.0);
        g.knobs.set.insert("culture.omen_mood".into(), 1.0);
        g.dream(i);
        assert_eq!(g.culture.omens.len(), 1);
        let due = g.culture.omens[0].due;
        assert!(due > g.tick);
        g.omens_come_true();
        assert!(g.dwarves[i].mood.is_none(), "not yet");
        g.tick = due;
        g.omens_come_true();
        assert!(g.dwarves[i].mood.is_some(), "the dream came true");
        assert!(g.overworld.events.last().unwrap().text.contains("as their dream of"));
        assert!(g.spotlights.iter().any(|s| s.title == "A dream come true"));
    }
}
