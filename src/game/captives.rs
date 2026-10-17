//! Captives: besiegers taken alive in cage traps. Each keeps a name and the
//! civilization (and siege) it came with; caging them counts toward breaking
//! the siege, and the overseer decides what becomes of them.

use super::*;
use crate::creature::Species;
use serde::{Deserialize, Serialize};

/// Someone in one of the fortress's cages.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Captive {
    /// The cage item they're in.
    pub cage: u32,
    pub species: Species,
    pub name: String,
    /// The civilization that sent them (index into the world's civs).
    pub civ: Option<u32>,
    /// The siege they came with (index into `annals.sieges`).
    pub siege: Option<usize>,
    /// When the trap shut on them.
    pub caught: u64,
    pub fate: Fate,
    /// They led the siege.
    #[serde(default)]
    pub general: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fate {
    Held,
    /// Held, and already made to talk.
    Questioned,
    Ransomed,
    Freed,
    Executed,
    Escaped,
}

impl Fate {
    pub fn name(self) -> &'static str {
        match self {
            Fate::Held => "held",
            Fate::Questioned => "questioned",
            Fate::Ransomed => "ransomed",
            Fate::Freed => "set free",
            Fate::Executed => "put to death",
            Fate::Escaped => "escaped",
        }
    }
}

impl Captive {
    /// Still in the cage.
    pub fn caged(&self) -> bool {
        matches!(self.fate, Fate::Held | Fate::Questioned)
    }

    /// "Snaga Ushat the goblin".
    pub fn title(&self) -> String {
        format!("{} the {}", self.name, self.species.info().name)
    }
}

/// The people a species belongs to, for naming them.
fn race_of(s: Species) -> Option<Race> {
    match s {
        Species::Goblin => Some(Race::Goblin),
        Species::Human | Species::Merchant | Species::Visitor => Some(Race::Human),
        Species::Elf => Some(Race::Elf),
        Species::Raider => Some(Race::Dwarf),
        _ => None,
    }
}

impl Game {
    /// A cage trap has shut on `c` (a besieger or one of the peoples), now in
    /// the cage item `cage`: write down who they are, and see whether the
    /// siege still has the heart to go on.
    pub(super) fn take_captive(&mut self, c: &Creature, cage: u32) {
        let siege = if c.siege { self.annals.sieges.iter().rposition(|s| s.ended.is_none()) } else { None };
        let civ = siege.and_then(|i| self.annals.sieges[i].civ);
        let general = c.general.is_some();
        let name = match (&c.label, race_of(c.species)) {
            (Some(l), _) => l.clone(),
            (None, Some(r)) => crate::overworld::person_name(&mut self.rng, r),
            (None, None) => c.title(),
        };
        let captive = Captive { cage, species: c.species, name, civ, siege, caught: self.tick, fate: Fate::Held, general };
        let title = captive.title();
        self.captives.push(captive);
        self.count("captives", 1);
        if let Some(i) = siege {
            self.annals.sieges[i].foes_caught += 1;
        }
        if general {
            let text = format!("The general {} is in a cage!", title);
            self.msg(&text, (255, 215, 60));
            self.spotlight("A general caught", &text, None, (255, 215, 60));
            let fort = self.fortress_name.clone();
            self.legend(format!("{} was caught in a cage trap at {}.", title, fort));
        } else {
            self.msg(&format!("The caged one is called {}.", title), (160, 230, 160));
        }
        if c.siege && !c.legendary() {
            // Losing the general breaks them, as when the general falls.
            if general && !self.siege_routed {
                self.rout();
            }
            self.check_rout();
        }
    }

    /// The captives still in their cages.
    pub fn caged(&self) -> impl Iterator<Item = &Captive> {
        self.captives.iter().filter(|c| c.caged())
    }

    /// For the Military panel: a blank line, "In cages:", then one line per
    /// captive ("Snaga Ushat the goblin (held)"); nothing when the cages are empty.
    pub fn cage_lines(&self) -> Vec<String> {
        let caged: Vec<&Captive> = self.caged().collect();
        if caged.is_empty() {
            return Vec::new();
        }
        let mut v = vec![String::new(), "In cages:".to_string()];
        for c in caged.iter().take(6) {
            v.push(format!(" {}{} ({})", c.title(), if c.general { ", general" } else { "" }, c.fate.name()));
        }
        if caged.len() > 6 {
            v.push(format!(" and {} more", caged.len() - 6));
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buildings::BuildingKind;

    /// A siege of `n` goblins at the map's edge, and a loaded cage trap.
    pub(super) fn siege_and_trap(seed: u64, n: u32) -> (Game, u32) {
        let mut g = Game::new(seed);
        g.creatures.retain(|_, c| c.tame);
        let mut foe = g.neighbors.iter().find(|n| g.at_war_with(n)).cloned().or_else(|| g.neighbors.first().cloned()).expect("a neighbour");
        foe.race = Race::Goblin;
        g.lay_siege_of(&foe, n, None);
        let at = g.open_patch(3, 3, 6).offset(1, 1, 0);
        assert!(g.place_building(BuildingKind::CageTrap, at));
        let tid = *g.buildings.keys().max().unwrap();
        let b = g.buildings.get_mut(&tid).unwrap();
        b.built = true;
        b.load = Some((ItemKind::Cage, Mat::Wood, 0));
        (g, tid)
    }

    /// Re-arm the trap by hand (in play a mechanic brings a new cage).
    pub(super) fn rearm(g: &mut Game, tid: u32) {
        g.buildings.get_mut(&tid).unwrap().built = true;
    }

    #[test]
    fn a_siege_half_caged_routs() {
        let (mut g, tid) = siege_and_trap(21, 4);
        let civ = g.annals.sieges[0].civ;
        assert!(civ.is_some(), "the siege knows who sent it");
        let rank: Vec<u32> = g.creatures.values().filter(|c| c.siege && c.general.is_none()).map(|c| c.id).collect();
        assert_eq!(rank.len(), 3);
        g.spring_trap(tid, rank[0]);
        assert!(!g.siege_routed, "one in a cage isn't enough");
        rearm(&mut g, tid);
        g.spring_trap(tid, rank[1]);
        assert!(g.siege_routed, "half of them caged: the rest should flee");
        assert_eq!(g.captives.len(), 2);
        for c in &g.captives {
            assert!(c.name.contains(' ') && c.name != "goblin", "a captive has a name: {}", c.name);
            assert_eq!((c.civ, c.siege, c.fate), (civ, Some(0), Fate::Held));
            assert!(g.items.get(&c.cage).map_or(false, |it| it.kind == ItemKind::Cage && it.label.is_some()));
        }
        assert_eq!(g.annals.sieges[0].foes_caught, 2);
        assert_eq!(g.annals.counters.get("captives"), Some(&2));
        assert!(g.log.iter().any(|l| l.text.contains("caught")));
        assert!(g.log.iter().any(|l| l.text.contains(&g.captives[0].name)), "the log names them");
        // And the Military panel lists them.
        let lines = g.cage_lines();
        assert_eq!(lines[1], "In cages:");
        assert!(lines[2].contains(&g.captives[0].name) && lines[2].ends_with("(held)"), "{:?}", lines);
        // The siege ends with those who ran: routed, not withdrawn.
        g.creatures.retain(|_, c| !c.siege);
        g.annals_tick();
        assert_eq!(g.annals.sieges[0].outcome, "routed");
    }

    #[test]
    fn caging_the_general_breaks_the_army() {
        let (mut g, tid) = siege_and_trap(21, 6);
        let gen = g.creatures.values().find(|c| c.general.is_some()).unwrap().id;
        let name = g.creatures[&gen].label.clone().unwrap();
        g.spring_trap(tid, gen);
        assert!(g.siege_routed);
        assert!(g.captives[0].general && g.captives[0].name == name);
        assert!(g.log.iter().any(|l| l.text.contains("is in a cage")));
    }

    #[test]
    fn animals_in_cages_are_not_captives() {
        let (mut g, tid) = siege_and_trap(21, 4);
        let at = g.buildings[&tid].pos;
        let wolf = g.spawn_creature(Species::Wolf, at, u64::MAX);
        g.spring_trap(tid, wolf);
        assert!(g.captives.is_empty());
        assert!(!g.siege_routed);
    }
}
