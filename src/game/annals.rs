//! The fortress's records: how each dwarf died and how each siege went.
//! Kept as plain facts (cause, killer, where, during what), so the batch
//! report can tally why forts fall and the chronicle can tell it.

use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DeathNote {
    pub tick: u64,
    pub name: String,
    /// "bled to death", "struck down", "thirst", "starvation", ...
    pub cause: String,
    /// What dealt the blow, or the wounds that bled: "goblin", "troglodyte"...
    pub killer: Option<String>,
    /// During which siege (index into `sieges`), if one was on.
    pub siege: Option<usize>,
    /// Under the open sky.
    pub outside: bool,
    /// Inside the alert burrow while the alert was on.
    pub sheltered: bool,
    pub soldier: bool,
    /// Mad when they died (melancholy or a failed mood).
    #[serde(default)]
    pub mad: bool,
    /// The madness was melancholy (which can lift).
    #[serde(default)]
    pub melancholy: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SiegeNote {
    pub tick: u64,
    /// "goblins", "human soldiers"...
    pub foe: String,
    pub size: u32,
    pub leader: String,
    pub dwarves_lost: u32,
    pub foes_slain: u32,
    /// The civilian alert was called while it lasted.
    pub shelter: bool,
    /// A drawbridge was up at some point while it lasted.
    pub bridge: bool,
    pub ended: Option<u64>,
    /// "routed", "withdrew" or "the fortress fell".
    pub outcome: String,
    /// Blows struck while it lasted: [soldiers' melee, civilians' melee,
    /// bolts shot, foes' blows on soldiers, foes' blows on civilians], each
    /// (swung, wounded).
    #[serde(default)]
    pub blows: [(u32, u32); 5],
    /// Besiegers taken alive in cage traps.
    #[serde(default)]
    pub foes_caught: u32,
    /// The civilization that sent them (index into the world's civs).
    #[serde(default)]
    pub civ: Option<u32>,
}

/// Indices into `SiegeNote::blows`.
pub const BY_SOLDIER: usize = 0;
pub const BY_CIVILIAN: usize = 1;
pub const BY_BOLT: usize = 2;
pub const ON_SOLDIER: usize = 3;
pub const ON_CIVILIAN: usize = 4;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Annals {
    pub deaths: Vec<DeathNote>,
    pub sieges: Vec<SiegeNote>,
    /// Who last wounded each dwarf, and when: (dwarf, kind, tick).
    #[serde(default)]
    pub wounded_by: Vec<(u32, String, u64)>,
    /// Tallies for the batch report ("rescues", "wells"...), from `Game::count`.
    #[serde(default)]
    pub counters: BTreeMap<String, u32>,
}

/// A death's cause, from the words it was announced with.
pub fn cause_of(text: &str) -> String {
    let has = |w: &str| text.contains(w);
    let c = if has("bled") {
        "bled to death"
    } else if has("struck down") {
        "struck down"
    } else if has("thirst") {
        "thirst"
    } else if has("starved") {
        "starvation"
    } else if has("drowned") {
        "drowned"
    } else if has("magma") || has("burned") {
        "burned"
    } else if has("crushed") {
        "crushed"
    } else if has("old age") {
        "old age"
    } else if has("expedition") {
        "fell on an expedition"
    } else {
        "other"
    };
    c.to_string()
}

/// What a creature counts as in the records.
pub fn kind_of(c: &Creature) -> String {
    match c.species {
        crate::creature::Species::Megabeast => c.label.as_deref().and_then(|l| l.rsplit(' ').next()).unwrap_or("megabeast").to_string(),
        _ => c.species.info().name.to_string(),
    }
}

impl Game {
    /// A dwarf was hurt by `c`: remember it, for when they bleed out later.
    pub(super) fn note_wound(&mut self, dwarf: u32, c: &Creature) {
        let kind = kind_of(c);
        let tick = self.tick;
        let w = &mut self.annals.wounded_by;
        w.retain(|(d, _, _)| *d != dwarf);
        w.push((dwarf, kind, tick));
    }

    /// A dwarf has died: write down how.
    pub(super) fn note_death(&mut self, d: &Dwarf, text: &str) {
        let cause = cause_of(text);
        let killer = match cause.as_str() {
            "struck down" | "bled to death" => self.annals.wounded_by.iter().find(|w| w.0 == d.id && self.tick < w.2 + DAY * 10).map(|w| w.1.clone()),
            _ => None,
        };
        let siege = self.annals.sieges.iter().rposition(|s| s.ended.is_none());
        if let Some(i) = siege {
            self.annals.sieges[i].dwarves_lost += 1;
        }
        let sheltered = self.civ_alert
            && self
                .burrows
                .iter()
                .find(|b| Some(b.bit) == self.alert_burrow)
                .map_or(false, |b| self.world.get(d.pos).map_or(false, |t| t.burrows & b.mask() != 0));
        self.annals.deaths.push(DeathNote {
            tick: self.tick,
            name: d.name.clone(),
            cause,
            killer,
            siege,
            outside: self.world.get(d.pos).map_or(false, |_| self.world.outside(d.pos)),
            sheltered,
            soldier: d.soldier,
            mad: d.insane,
            melancholy: d.insane && d.melancholy,
        });
        self.annals.wounded_by.retain(|w| w.0 != d.id);
    }

    pub(super) fn note_siege(&mut self, foe: &str, size: u32, leader: &str) {
        self.annals.sieges.push(SiegeNote { tick: self.tick, foe: foe.into(), size, leader: leader.into(), ..Default::default() });
    }

    /// A blow in a siege: which kind, and whether it drew blood.
    pub(super) fn note_blow(&mut self, kind: usize, wounded: bool) {
        if let Some(s) = self.annals.sieges.iter_mut().rev().find(|s| s.ended.is_none()) {
            s.blows[kind].0 += 1;
            if wounded {
                s.blows[kind].1 += 1;
            }
        }
    }

    /// One of the besiegers died.
    pub(super) fn note_foe_slain(&mut self, c: &Creature) {
        if !c.siege || c.legendary() {
            return;
        }
        if let Some(s) = self.annals.sieges.iter_mut().rev().find(|s| s.ended.is_none()) {
            s.foes_slain += 1;
        }
    }

    /// Every little while: keep the open siege's record up to date, and
    /// close it once the last besieger is gone.
    pub(crate) fn annals_tick(&mut self) {
        let Some(i) = self.annals.sieges.iter().rposition(|s| s.ended.is_none()) else { return };
        let bridge = self.buildings.values().any(|b| b.kind == BuildingKind::Bridge && b.raised);
        let shelter = self.civ_alert;
        let left = self.creatures.values().filter(|c| c.alive && c.siege && !c.legendary() && c.species != crate::creature::Species::CaveHorror).count();
        let fallen = self.alive_count() == 0;
        let routed = self.siege_routed;
        let tick = self.tick;
        let s = &mut self.annals.sieges[i];
        s.bridge |= bridge;
        s.shelter |= shelter;
        if fallen || left == 0 {
            s.ended = Some(tick);
            s.outcome = if fallen {
                "the fortress fell"
            } else if routed {
                "routed"
            } else {
                "withdrew"
            }
            .into();
            self.on_siege_end(i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deaths_and_sieges_are_written_down() {
        let mut g = Game::new(5);
        g.creatures.retain(|_, c| c.tame);
        let n = g.neighbors.iter().find(|n| g.at_war_with(n)).cloned().or_else(|| g.neighbors.first().cloned()).expect("a neighbour");
        let mut foe = n.clone();
        foe.race = crate::overworld::Race::Goblin;
        g.lay_siege(&foe);
        assert_eq!(g.annals.sieges.len(), 1);
        assert!(g.annals.sieges[0].size >= 3 && !g.annals.sieges[0].leader.is_empty());
        // A goblin wounds a dwarf, who later bleeds out.
        let gob = g.creatures.values().find(|c| c.siege).unwrap().clone();
        let id = g.dwarves[0].id;
        g.note_wound(id, &gob);
        let mut d = std::mem::take(&mut g.dwarves[0]);
        g.kill(&mut d, "has bled to death");
        g.dwarves[0] = d;
        let note = g.annals.deaths.last().unwrap();
        assert_eq!(note.cause, "bled to death");
        assert_eq!(note.killer.as_deref(), Some("goblin"));
        assert_eq!(note.siege, Some(0));
        assert_eq!(g.annals.sieges[0].dwarves_lost, 1);
        // The besiegers die: the siege is over.
        let ids: Vec<u32> = g.creatures.values().filter(|c| c.siege).map(|c| c.id).collect();
        for cid in ids {
            let c = g.creatures.remove(&cid).unwrap();
            g.creature_died(c);
        }
        g.annals_tick();
        let s = &g.annals.sieges[0];
        assert!(s.ended.is_some(), "the siege should be over");
        assert!(s.foes_slain >= 3);
        assert_eq!(cause_of("Urist has died of thirst."), "thirst");
    }
}
