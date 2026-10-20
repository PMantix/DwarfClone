//! Last words (O05, UI-02): what a dying dwarf says, drawn from what they
//! cared about most: a spouse, their children, a grudge, the thing that
//! killed them, a named weapon, their craft; a hero boasts and the mad
//! ramble. The words come from a hash of the dwarf and the death (never
//! the game's dice), so the same death gives the same words. Those killed
//! at once (crushed, drowned, burned in magma) say nothing, unless a hero.
//!
//! They go to the announcements, the death note (`DeathNote.last_words`),
//! the hero's card, the fort's memory, and so to the overseer's eulogies
//! and the chronicle's death scenes (`lifestory.rs: eulogy`).

use super::*;
use super::stories::first_name;

/// Deaths too quick for a word.
const INSTANT: [&str; 4] = ["crushed", "drowned", "in magma", "fell on the expedition"];

const SPOUSE: [&str; 5] = [
    "Tell {s} the first round is on me.",
    "{s}... keep the hearth lit for me.",
    "Don't let {s} see me like this.",
    "Tell {s} I'd marry them again. Every time.",
    "Where's {s}? I wanted... to say...",
];
const CHILD: [&str; 4] = ["Look after {c} for me.", "Tell {c} to dig deep and stand tall.", "{c} gets my hammer. And my good boots.", "Don't tell {c} it hurt."];
const KILLER: [&str; 4] = ["Mind the {k}... it's quicker than it looks.", "Get the {k}. Promise me.", "Don't go after the {k} alone.", "The {k}... I nearly had it."];
const GRUDGE: [&str; 3] = ["Tell {g} I never forgave them.", "At least I won't have to look at {g} any more.", "Don't let {g} speak at my burial."];
const WEAPON: [&str; 3] = ["Give {w} to someone brave.", "{w} served me well. Bury it with me, or better, use it.", "Keep {w} sharp. It isn't done yet."];
const CRAFT: [&str; 4] = ["The fort will need another {p}. Teach the young ones.", "I had one more piece to make. Someone finish it.", "Who'll be the fort's {p} now?", "Keep my tools oiled."];
const HERO: [&str; 4] = ["Sing of me in the great hall, and make it loud.", "They'll remember that I {d}, won't they?", "Not a bad end for one who {d}.", "Carve it in the stone: I {d}."];
const MAD: [&str; 4] = [
    "The walls... the walls are humming the old song...",
    "Blue, all of it blue... the stone is laughing...",
    "Shh. The tunnels are listening.",
    "I see the vein now. It goes all the way down. All the way...",
];
const BITTER: [&str; 3] = ["At last. Some quiet.", "Don't mourn me. This place was a misery.", "Tell the overseer... it was never worth it."];
const PLAIN: [&str; 5] = ["Is it dark already?", "Tell them I did my share.", "Put me somewhere with a view of the stone.", "Raise a mug for me.", "I've seen enough seasons. It's all right."];

/// One of `lines` picked by `h`, with each `{x}` filled in.
fn fill(lines: &[&str], h: u64, subs: &[(&str, &str)]) -> String {
    let mut s = lines[(h % lines.len() as u64) as usize].to_string();
    for (k, v) in subs {
        s = s.replace(k, v);
    }
    s
}

impl Game {
    /// What `d` says as they die of `cause`, if anything.
    pub fn last_words(&self, d: &Dwarf, cause: &str) -> Option<String> {
        let hero = d.epithet.is_some();
        if !hero && INSTANT.iter().any(|w| cause.contains(w)) {
            return None;
        }
        if d.is_child() && d.age == Some(0) {
            return None;
        }
        let h = super::names::hash(&[d.id as u64, super::stories::hash(cause, 7), self.tick]);
        let alive = |id: u32| self.dwarves.iter().find(|o| o.id == id && o.alive && o.id != d.id);
        // Who or what killed them: the death note (already written) or the wounds.
        let killer = self
            .annals
            .deaths
            .last()
            .filter(|n| n.name == d.name && n.tick == self.tick)
            .and_then(|n| n.killer.clone())
            .or_else(|| self.annals.wounded_by.iter().find(|w| w.0 == d.id).map(|w| w.1.clone()))
            .filter(|_| cause.contains("struck") || cause.contains("bled"));
        // Each thing they care about, most first; a coin of the hash lets the
        // second speak now and then.
        let mut said: Vec<String> = Vec::new();
        if d.insane {
            said.push(fill(&MAD, h, &[]));
        }
        if let (true, Some(deed)) = (hero, &d.deed) {
            said.push(fill(&HERO, h, &[("{d}", deed)]));
        }
        if let Some(sp) = d.spouse.and_then(alive) {
            said.push(fill(&SPOUSE, h, &[("{s}", first_name(&sp.name))]));
        }
        let mut kids: Vec<&Dwarf> = self.dwarves.iter().filter(|o| o.alive && o.id != d.id && o.parents.map_or(false, |(m, f)| m == d.id || f == d.id)).collect();
        kids.sort_by_key(|o| o.id);
        if let Some(c) = kids.first() {
            said.push(fill(&CHILD, h, &[("{c}", first_name(&c.name))]));
        }
        if let Some(k) = &killer {
            said.push(fill(&KILLER, h, &[("{k}", k)]));
        }
        let foe = d.relations.iter().filter(|r| r.1 <= -40).min_by_key(|r| (r.1, r.0)).and_then(|r| alive(r.0));
        if let Some(g) = foe {
            said.push(fill(&GRUDGE, h, &[("{g}", first_name(&g.name))]));
        }
        let mut blades: Vec<&crate::items::Item> = self.items.values().filter(|i| i.pos == d.pos && i.carried_by.is_none() && i.label.is_some() && i.kind.weapon_damage().is_some()).collect();
        blades.sort_by_key(|i| i.id);
        if let Some(w) = blades.first().and_then(|i| i.label.clone()) {
            said.push(fill(&WEAPON, h, &[("{w}", &w)]));
        }
        if d.stress >= 300 {
            said.push(fill(&BITTER, h, &[]));
        }
        let prof = d.profession();
        if !matches!(prof, "Peasant" | "Child" | "Baby" | "Recruit") {
            said.push(fill(&CRAFT, h, &[("{p}", &prof.to_lowercase())]));
        }
        said.push(fill(&PLAIN, h, &[]));
        let pick = if said.len() > 1 && (h >> 20) % 4 == 0 { 1 } else { 0 };
        Some(said.swap_remove(pick))
    }

    /// Called once from `kill`, after the death is noted: the dying words
    /// are announced and kept with the death, its memory and a hero's card.
    pub(super) fn speak_last_words(&mut self, d: &Dwarf, cause: &str) {
        let Some(w) = self.last_words(d, cause) else { return };
        let quote = format!("\u{201c}{}\u{201d}", w);
        self.msg_at(&format!("{}, dying: {}", d.called(), quote), (210, 170, 170), Some(d.pos), super::logkind::LogKind::Death);
        if let Some(n) = self.annals.deaths.last_mut().filter(|n| n.name == d.name) {
            n.last_words = Some(w.clone());
        }
        if let Some(m) = self.memory.events.iter_mut().rev().take(4).find(|m| m.kind == super::memory::MemoryKind::Died && m.who.contains(&d.id)) {
            m.text = format!("{}, saying {}", m.text, quote);
        }
        let called = d.called();
        if let Some(s) = self.spotlights.iter_mut().rev().find(|s| s.title == "A hero falls" && s.text.starts_with(&called)) {
            s.text = format!("{} Their last words: {}", s.text, quote);
        }
        self.count("last_words", 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dying_speak_of_what_they_love() {
        let mut g = Game::new(5);
        let (a, b) = (g.dwarves[0].id, g.dwarves[1].id);
        g.dwarves[0].spouse = Some(b);
        g.dwarves[1].spouse = Some(a);
        g.dwarves[0].insane = false;
        g.dwarves[0].epithet = None;
        g.dwarves[0].stress = 0;
        let spouse = first_name(&g.dwarves[1].name).to_string();
        // Over many deaths the spouse is what a married dwarf speaks of most.
        let d = g.dwarves[0].clone();
        let mut of_spouse = 0;
        for t in 0..40 {
            g.tick = 1000 + t * 77;
            let w = g.last_words(&d, "has bled to death").expect("words");
            // The same death gives the same words.
            assert_eq!(Some(w.clone()), g.last_words(&d, "has bled to death"));
            if w.contains(&spouse) {
                of_spouse += 1;
            }
        }
        assert!(of_spouse >= 20, "{} of 40 mention {}", of_spouse, spouse);
        // Crushed at once: no words, unless a hero.
        assert_eq!(g.last_words(&d, "was crushed"), None);
        let mut hero = d.clone();
        hero.epithet = Some("Dragonslayer".into());
        hero.deed = Some("slew the dragon Emberwing".into());
        assert!(g.last_words(&hero, "was crushed").is_some());
        // Killed for real: the words are announced and kept with the death.
        g.tick = 5000;
        let mut d = std::mem::take(&mut g.dwarves[0]);
        g.kill(&mut d, "has bled to death");
        g.dwarves[0] = d;
        let note = g.annals.deaths.last().unwrap();
        let words = note.last_words.clone().expect("kept with the death");
        assert!(g.log.iter().any(|e| e.text.contains(&words) && e.at.is_some()));
        // The overseer's eulogy quotes them.
        assert!(g.eulogy(&g.dwarves[0]).contains(&words), "{}", g.eulogy(&g.dwarves[0]));
    }
}
