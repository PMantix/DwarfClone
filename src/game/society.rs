//! The fort as a society (Milestone 3): where grief and vengeance
//! (`grief.rs`), the bottle (`bottle.rs`) and feuds and brawls (`feuds.rs`)
//! keep their state and plug into the game. Each system keeps its logic in
//! its own file; this one holds the container saved with the game, the
//! hooks the schedule calls, the overseer's round and its senses.
//!
//! Batch measures go in `soc.*` counters (vows sworn and kept, bodies
//! found, brawls and who joined them, drunks out cold, withdrawal).

use super::ambitions::{Ambition, Sense};
use super::overseer::Overseer;
use super::*;
use serde::{Deserialize, Serialize};

/// Everything the fort's society remembers (`Game.society`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Society {
    #[serde(default)]
    pub grief: super::grief::GriefBook,
    #[serde(default)]
    pub bar: super::bottle::Bar,
    #[serde(default)]
    pub feuds: super::feuds::Feuds,
}

/// `a` and `b` are close kin: spouses, parent and child, or siblings.
pub fn close_kin(a: &Dwarf, b: &Dwarf) -> bool {
    a.spouse == Some(b.id) || b.spouse == Some(a.id) || a.family_of(b.id) || b.family_of(a.id) || (a.parents.is_some() && a.parents == b.parents)
}

impl Game {
    /// Foes are at the fort: a siege, or a great beast (`alert` alone is
    /// the soldiers' readiness, on all the time in a game without an overseer).
    pub(super) fn raided(&self) -> bool {
        self.creatures.values().any(|c| c.alive && c.hostile() && (c.siege || c.legendary()))
    }

    /// Every ten ticks (the schedule): brawls, bodies found, grave visits.
    pub(super) fn society_tick(&mut self) {
        self.brawls_tick();
        if self.tick % 50 == 17 {
            self.grief_tick();
            self.feuds_tick();
        }
    }

    /// Once a day (the schedule): vows kept or broken, feuds climbing.
    pub(super) fn society_daily(&mut self) {
        self.grief_daily();
        self.feuds_daily();
        self.bottle_daily();
    }

    /// Each dwarf, every tick, from `psyche_tick`: what drink does to them.
    pub(super) fn society_dwarf(&mut self, d: &mut Dwarf) {
        if self.tick % 10 == d.id as u64 % 10 {
            self.bottle_dwarf(d);
        }
    }

    /// An idle dwarf with somewhere to be: a grave to visit, a drink to
    /// drown their sorrows in. True if they went.
    pub(super) fn society_leisure(&mut self, d: &mut Dwarf) -> bool {
        self.grief_visit(d) || self.bottle_leisure(d)
    }

    /// The overseer's daily round (`overseer_hooks.rs`): the vowed drafted,
    /// feuding squadmates parted.
    pub(super) fn ov_society(&mut self, o: &mut Overseer) {
        self.ov_vows(o);
        self.ov_feuds(o);
    }

    /// The overseer's social senses (`extra_senses`): fresh mourning and
    /// open discord, each pulling toward a feast and the stores.
    pub(super) fn society_senses(&self, o: &Overseer) -> Vec<(Sense, f64, Vec<(Ambition, f64)>)> {
        use super::personality::Trait;
        let mourners = self.society.grief.mourners.iter().filter(|m| self.tick < m.until).count() as f64;
        let mut mourning = (mourners / 6.0).min(1.0);
        if o.has(Trait::Sentimental) {
            mourning = (mourning * 1.5).min(1.0);
        }
        let f = &self.society.feuds;
        let recent = f.brawl_log.iter().filter(|&&t| self.tick < t + SEASON).count() as f64;
        let discord = ((recent + f.factions.len() as f64 * 2.0 + f.ladder.iter().filter(|l| l.stage >= 3).count() as f64) / 6.0).min(1.0);
        vec![
            (Sense::Mourning, mourning, vec![(Ambition::Revel, 0.8), (Ambition::Provision, 0.4), (Ambition::Venture, -1.0)]),
            (Sense::Discord, discord, vec![(Ambition::Revel, 1.0), (Ambition::Fortify, 0.3)]),
        ]
    }

    /// Lines for the dwarf's sheet: drunk or dry, a vow, who they mourn,
    /// the side they took.
    pub fn society_lines(&self, d: &Dwarf) -> Vec<(String, (u8, u8, u8))> {
        let mut v = Vec::new();
        if !d.alive {
            return v;
        }
        let tick = self.tick;
        let t = self.tipsy(d.id);
        if t != super::bottle::Tipsy::Sober {
            v.push((t.word().to_string(), (220, 180, 110)));
        } else if let Some(s) = self.society.bar.drinkers.get(&d.id) {
            if d.need(crate::dwarf::NeedKind::Drink).map_or(false, |n| n.strength >= 2) && tick > s.last_drink + 6 * DAY {
                v.push(("Shaky for want of a drink".into(), (200, 170, 120)));
            }
        }
        let first = |n: &str| n.split(' ').next().unwrap_or(n).to_string();
        if let Some(vow) = self.society.grief.vows.iter().find(|v| v.who == d.id) {
            v.push((format!("Vow: to slay the {} who killed {}", super::grief::plural(&vow.foe), first(&vow.for_name)), (255, 120, 80)));
        }
        if let Some(m) = self.society.grief.mourners.iter().find(|m| m.who == d.id && tick < m.until) {
            v.push((format!("Mourning {}", first(&m.name)), (170, 170, 220)));
        }
        if let Some(f) = self.society.feuds.factions.iter().find(|f| f.sides.iter().any(|s| s.contains(&d.id))) {
            let s = if f.sides[0].contains(&d.id) { 0 } else { 1 };
            v.push((format!("Sided with {} against {}", f.names[s], f.names[1 - s]), (230, 160, 90)));
        }
        v
    }
}
