//! Watch and nudge (O01, OVR-24): what the overseer means to do next (its
//! intent, in the top bar and its brain), and commands from the spotlight
//! cards: raise the bridge, everyone to the shelter, stand down, muster. It
//! obeys for half a day (`Overseer.commands` with when), though a stubborn
//! one now and then won't be told, and says so.

use super::overseer::{plan, room_name, Overseer};
use super::personality::{NoteKind, Trait};
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    /// Raise the gate's drawbridge now.
    RaiseBridge,
    /// Send the civilians to the shelter.
    Shelter,
    /// Lower the bridge, call off the alert.
    StandDown,
    /// Take up the Muster ambition.
    Muster,
    /// Call an expedition home.
    Recall,
}

impl Command {
    #[allow(dead_code)]
    pub const ALL: [Command; 5] = [Command::RaiseBridge, Command::Shelter, Command::StandDown, Command::Muster, Command::Recall];

    /// The key that gives it on a card.
    pub fn key(self) -> char {
        match self {
            Command::RaiseBridge => 'b',
            Command::Shelter => 's',
            Command::StandDown => 'd',
            Command::Muster => 'm',
            Command::Recall => 'r',
        }
    }

    /// The commands a card offers, by its title: an army at the gate, a
    /// beast near the halls, a siege that is over.
    pub fn for_card(title: &str) -> Vec<Command> {
        match title {
            "Siege!" | "An army on the march" | "Raiders sighted" | "Taken unawares" => vec![Command::RaiseBridge, Command::Shelter, Command::Muster],
            "A dragon!" | "A beast of legend!" | "From the deep" | "It comes" | "The dead walk" => vec![Command::Shelter, Command::RaiseBridge],
            "The siege is broken" => vec![Command::StandDown],
            _ => Vec::new(),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Command::RaiseBridge => "raise the bridge",
            Command::Shelter => "to the shelter",
            Command::StandDown => "stand down",
            Command::Muster => "muster",
            Command::Recall => "recall the expedition",
        }
    }
}

/// How long a command holds: half a day (a whole one for a cautious
/// overseer told to shelter, which it was itching to do anyway).
fn holds(o: &Overseer, c: Command) -> u64 {
    if c == Command::Shelter && o.has(Trait::Cautious) {
        DAY
    } else {
        DAY / 2
    }
}

impl Overseer {
    /// Whether it is under orders to `c` now: told within the time a command
    /// holds, and not told to stand down since.
    pub fn ordered(&self, c: Command, tick: u64) -> bool {
        self.commands.iter().rev().take_while(|(k, _)| *k != Command::StandDown).any(|&(k, at)| k == c && tick < at + holds(self, c))
    }

    /// Whether it was told to stand down within the time a command holds,
    /// and given no order since: the bridge comes down and the shelter is
    /// let out, whatever it would do of its own accord. (`since`: when the
    /// latest siege came; a new army calls off the stand-down.)
    pub fn stood_down(&self, tick: u64, since: u64) -> bool {
        self.commands.last().map_or(false, |&(k, at)| k == Command::StandDown && tick < at + holds(self, k) && since <= at)
    }

    /// What it means to do next, in a few words: "Next: a tavern (Revel)",
    /// "Next: the drawbridge, once 2 more mechanisms are made".
    pub fn intent(&self, g: &Game) -> String {
        let why = self.ambition.map_or(String::new(), |a| format!(" ({})", a.name()));
        if self.ordered(Command::RaiseBridge, g.tick) {
            return "Next: hold the bridge up, as you said".into();
        }
        if self.ordered(Command::Shelter, g.tick) {
            return "Next: keep everyone in the shelter, as you said".into();
        }
        if self.base.is_none() {
            return "Next: choose where to break ground".into();
        }
        if let Some(&k) = self.wants.first() {
            return format!("Next: a {}{}", room_name(k), why);
        }
        if self.style != Style::Reckless && !self.did("linked") {
            if !self.did("gate") {
                return "Next: hang a gate at the foot of the stair".into();
            }
            let mechs = g.stock_of(ItemKind::Mechanism);
            return if self.did("bridge") || mechs >= 3 {
                "Next: the drawbridge on its lever".into()
            } else {
                format!("Next: the drawbridge, once {} more mechanism{} {} made", 3 - mechs, if 3 - mechs == 1 { "" } else { "s" }, if 3 - mechs == 1 { "is" } else { "are" })
            };
        }
        if let Some(&k) = plan(self.style).get(self.planned) {
            return format!("Next: a {}{}", room_name(k), why);
        }
        match self.ambition {
            Some(a) => format!("Next: {}", a.name()),
            None => "Next: to see what the fortress needs".into(),
        }
    }
}

impl Game {
    /// When the latest siege came (0 if none has).
    pub(super) fn last_siege_tick(&self) -> u64 {
        self.annals.sieges.last().map_or(0, |s| s.tick)
    }

    /// Give the overseer a command from a card or a key; what it says back
    /// (also in its notes). It obeys for half a day, but a stubborn one
    /// won't be told one time in three.
    pub fn command_overseer(&mut self, c: Command) -> String {
        let tick = self.tick;
        let Some(mut o) = self.overseer.take() else { return "There is no overseer to command.".into() };
        let refuse = o.has(Trait::Stubborn) && c != Command::StandDown && super::names::hash(&[tick, o.commands.len() as u64, c as u64]) % 3 == 0;
        let mut muster = false;
        let reply = if refuse {
            match c {
                Command::RaiseBridge => "Grudgingly, it would not be told: the bridge stays down.".to_string(),
                Command::Shelter => "It would not be told: nobody hides while it's in charge.".to_string(),
                Command::Muster => "It would not be told: the squad has better things to do.".to_string(),
                _ => "It would not be told.".to_string(),
            }
        } else {
            match c {
                Command::RaiseBridge if !o.did("linked") => "It has no drawbridge on a lever yet: it bars the doors and hopes.".to_string(),
                Command::RaiseBridge if o.has(Trait::Bold) => "Rolling its eyes, it sent word to the lever: up goes the bridge.".to_string(),
                Command::RaiseBridge => "It sent word to the lever: up goes the bridge, and stays up half a day.".to_string(),
                Command::Shelter if self.alert_burrow.is_none() => "It has marked out no shelter yet: everyone keeps to the halls.".to_string(),
                Command::Shelter if o.has(Trait::Cautious) => "At once, it sent everyone to the shelter: it was about to anyway.".to_string(),
                Command::Shelter => "It sounded the alert: everyone to the shelter.".to_string(),
                Command::StandDown => "It stood the fortress down: back to work, and back to its own judgement.".to_string(),
                Command::Muster if o.base.is_none() => "It has no fortress yet to muster.".to_string(),
                Command::Muster => {
                    muster = true;
                    "It called the dwarves to arms.".to_string()
                }
                Command::Recall => "There is no calling an expedition back once it has gone.".to_string(),
            }
        };
        if !refuse {
            o.commands.push((c, tick));
            if o.commands.len() > 20 {
                o.commands.remove(0);
            }
        }
        Self::note_as(&mut o, tick, NoteKind::Command, reply.clone());
        // Sheltering is at once: no waiting for the alert check; and so is
        // standing down (the bridge follows at the gate's next check).
        let shelter = !refuse && ((c == Command::Shelter && self.alert_burrow.is_some() && !self.civ_alert) || (c == Command::StandDown && self.civ_alert));
        self.overseer = Some(o);
        if shelter {
            let _ = self.toggle_civ_alert();
        }
        if muster {
            self.force_ambition(super::ambitions::Ambition::Muster);
        }
        self.count("ov.commands", 1);
        reply
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_overseer_says_what_it_will_do_next() {
        let mut g = Game::new(5);
        g.set_overseer(Some(Style::Steady));
        for _ in 0..300 {
            g.step();
        }
        let o = g.overseer.as_ref().unwrap();
        let said = o.intent(&g);
        assert!(said.starts_with("Next: ") && said.len() > 10, "{}", said);
        let rooms = ["hall", "storeroom", "dormitory", "tavern", "temple", "hospital", "tomb", "barracks", "office", "jail", "bedrooms", "suite", "library", "parlour", "quarter", "gate", "drawbridge"];
        assert!(rooms.iter().any(|r| said.contains(r)), "{}", said);
        // Told to raise the bridge, that is what it means to do.
        g.command_overseer(Command::RaiseBridge);
        let o = g.overseer.as_ref().unwrap();
        assert!(o.ordered(Command::RaiseBridge, g.tick) || o.has(Trait::Stubborn));
        // Stood down, it isn't.
        g.command_overseer(Command::StandDown);
        assert!(!g.overseer.as_ref().unwrap().ordered(Command::RaiseBridge, g.tick));
    }

    /// OVR-24: standing down lowers a raised gate bridge and lets the
    /// shelter out, even with an army on its way; a new army calls it off.
    #[test]
    fn standing_down_lowers_the_bridge_and_ends_the_alert() {
        let (found, _) = super::super::overseer::fort_where(Style::Steady, 5, 30, 200, |g| g.overseer.as_ref().map_or(false, |o| o.did("linked")) && g.alert_burrow.is_some());
        let (seed, mut g) = found.expect("a fort with a linked bridge and a shelter");
        let (top, base) = { let o = g.overseer.as_ref().unwrap(); (o.top.unwrap(), o.base.unwrap()) };
        let lever = g.gate_lever(base).expect("no gate lever");
        let raised = |g: &Game| g.gate_bridges(base).iter().any(|b| g.buildings[b].raised);
        // The bridge up and the shelter called, an army on its way.
        if !raised(&g) {
            g.pull_lever(lever);
        }
        if !g.civ_alert {
            let _ = g.toggle_civ_alert();
        }
        assert!(raised(&g) && g.civ_alert, "seed {}", seed);
        // Told to stand down (a stubborn overseer can't refuse it), it does at once...
        let said = g.command_overseer(Command::StandDown);
        assert!(!g.civ_alert, "{}", said);
        // ...and the gate's next look sends word to lower the bridge, army or no.
        let mut o = g.overseer.take().unwrap();
        g.ov_gate_alert(&mut o, top, base, true);
        assert!(g.buildings[&lever].pull || !raised(&g), "seed {}: the bridge wasn't called down", seed);
        // A new army calls the stand-down off.
        assert!(o.stood_down(g.tick, g.last_siege_tick()));
        assert!(!o.stood_down(g.tick, g.tick + 1));
        assert!(!o.stood_down(g.tick + DAY, g.last_siege_tick()), "it holds half a day");
        g.overseer = Some(o);
    }

    #[test]
    fn a_stubborn_overseer_sometimes_will_not_be_told() {
        let mut g = Game::new(5);
        g.set_overseer(Some(Style::Steady));
        g.set_traits(vec![Trait::Stubborn]).unwrap();
        let mut refused = 0;
        for i in 0..30 {
            g.tick = 1000 + i * 37;
            let said = g.command_overseer(Command::RaiseBridge);
            refused += said.contains("would not be told") as usize;
            assert!(g.overseer.as_ref().unwrap().notes.iter().any(|n| n.1 == said), "{}", said);
        }
        assert!((4..=18).contains(&refused), "refused {} of 30", refused);
        // A card for a siege offers the bridge and the shelter.
        assert!(Command::for_card("Siege!").contains(&Command::RaiseBridge));
        assert!(Command::for_card("Traders").is_empty());
    }
}
