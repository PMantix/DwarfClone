//! Warm and damp stone (O10, DIG-21): a miner about to cut into rock with
//! hidden water or magma close behind it feels it first (the stone sweats,
//! or it is warm to the hand) and won't dig on unless told to. A player
//! gives the standing order on the work orders screen (`w`); the overseer
//! decides by its nature: a delver goes on toward the magma it is looking
//! for, the bold and the reckless go on through the damp, and the rest
//! leave it be. Breaking through into water anyway is a flood, and news.

use super::logkind::LogKind;
use super::overseer::Style;
use super::personality::Trait;
use super::*;
use serde::{Deserialize, Serialize};

/// The fort's damp and warm stone (`Game.damp`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Damp {
    /// Designations the miners stopped short of, and whether the stone was
    /// warm (magma) rather than damp.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub held: Vec<(Pos, bool)>,
    /// The standing order: dig on through damp and warm stone.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub dig_wet_ok: bool,
    /// Tiles the overseer ordered dug in spite of it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ok: Vec<Pos>,
}

impl Damp {
    pub fn is_held(&self, p: Pos) -> bool {
        !self.held.is_empty() && self.held.iter().any(|h| h.0 == p)
    }
}

/// How far a miner feels hidden water or magma on the level being dug (one
/// less on the levels above and below).
pub const DAMP_REACH: i32 = 2;

impl Game {
    /// Hidden water or magma near `q`: `Some(true)` warm (magma), `Some(false)`
    /// damp, `None` dry. Only what nobody has seen counts: water in sight is
    /// the digger's own lookout.
    pub fn damp_near(&self, q: Pos) -> Option<bool> {
        let mut wet = false;
        for dz in -1..=1 {
            let r = if dz == 0 { DAMP_REACH } else { DAMP_REACH - 1 };
            for dy in -r..=r {
                for dx in -r..=r {
                    let Some(t) = self.world.get(q.offset(dx, dy, dz)) else { continue };
                    if t.revealed {
                        continue;
                    }
                    if t.magma > 0 {
                        return Some(true);
                    }
                    // A heavy aquifer floods what's dug beside it; a light one only seeps.
                    if (t.water > 0 && !t.frozen) || (t.aquifer && self.world.aquifer >= 2 && dz == 0 && dx.abs() <= 1 && dy.abs() <= 1) {
                        wet = true;
                    }
                }
            }
        }
        wet.then_some(false)
    }

    /// A miner `d` is about to go and dig `q`: whether the damp (or the warmth)
    /// stops them. The tile is held for the player, or left by an overseer
    /// that won't risk it; an overseer that will says so in its notes.
    pub(super) fn damp_stop(&mut self, d: &Dwarf, q: Pos) -> bool {
        if self.damp.dig_wet_ok || self.world.outside(q) || self.damp.ok.contains(&q) {
            return false;
        }
        if self.damp.is_held(q) {
            return true;
        }
        let Some(warm) = self.damp_near(q) else { return false };
        // A well or a cistern is dug to find the water.
        if self.by_waterworks(q) {
            return false;
        }
        let who = d.name.split(' ').next().unwrap_or(&d.name).to_string();
        let (word, feel) = if warm { ("warm", "The rock is warm to the hand.") } else { ("damp", "Water beads on the rock.") };
        let tick = self.tick;
        // Already told about stone near here: no need to say it twice.
        let near_told = self.damp.held.iter().any(|h| h.0.z == q.z && (h.0.x - q.x).abs() <= 4 && (h.0.y - q.y).abs() <= 4);
        // An overseer knows the signs: only a cut that would open straight
        // into the water (or magma) gives it pause; a rock wall between holds.
        if self.overseer.is_some() && !self.breach_risk(q) {
            self.count("m2.damp_near", 1);
            return false;
        }
        if let Some(o) = self.overseer.as_mut() {
            let goes_on = if warm {
                o.style == Style::Delver || (o.style == Style::Reckless && names::hash(&[q.x as u64, q.y as u64, q.z as u64, 7]) % 3 == 0)
            } else {
                o.style == Style::Reckless || o.style == Style::Delver && o.has(Trait::Bold) || (o.has(Trait::Bold) && names::hash(&[q.x as u64, q.y as u64, q.z as u64, 5]) % 2 == 0)
            };
            if goes_on {
                self.damp.ok.push(q);
                if self.damp.ok.len() > 200 {
                    self.damp.ok.remove(0);
                }
                let text = if warm {
                    "Warm stone below. The magma must be close: ordered the miners on, carefully.".to_string()
                } else {
                    "Damp stone. Without a second thought, it ordered the miners on through the damp.".to_string()
                };
                if !near_told {
                    Self::note_ov(o, tick, text);
                    self.msg_at(&format!("{} has found {} stone. The overseer orders them on.", who, word), (120, 170, 230), Some(q), LogKind::Job);
                }
                self.count("m2.damp_dug", 1);
                return false;
            }
            // Left be: the designation goes, so the plan moves on round it.
            if !near_told {
                let text = format!("{} stone where the miners were cutting. Left it be rather than let the {} in.", if warm { "Warm" } else { "Damp" }, if warm { "magma" } else { "water" });
                Self::note_ov(o, tick, text);
                self.msg_at(&format!("{} has found {} stone. {} The miners will not dig there.", who, word, feel), (120, 170, 230), Some(q), LogKind::Job);
            }
            self.world.t_mut(q).desig = Desig::None;
            self.designations.remove(&q);
            self.damp.held.push((q, warm));
            self.count("m2.damp_left", 1);
            return true;
        }
        if !near_told {
            self.msg_at(
                &format!("{} has found {} stone. {} Digging there waits (work orders, w: dig on anyway).", who, word, feel),
                (120, 170, 230),
                Some(q),
                LogKind::Job,
            );
        }
        self.damp.held.push((q, warm));
        true
    }

    /// Digging out `q` would open it straight onto hidden water or magma:
    /// beside it, above or below.
    fn breach_risk(&self, q: Pos) -> bool {
        [q.up(), q.down()]
            .into_iter()
            .chain(DIRS4.iter().map(|&(dx, dy)| q.offset(dx, dy, 0)))
            .any(|p| self.world.get(p).map_or(false, |t| !t.revealed && (t.magma > 0 || (t.water > 0 && !t.frozen) || (t.aquifer && self.world.aquifer >= 2))))
    }

    /// The standing order to dig on through damp and warm stone: turn it
    /// over and let held digging go ahead. Whether it is now on.
    pub fn toggle_dig_wet(&mut self) -> bool {
        self.damp.dig_wet_ok = !self.damp.dig_wet_ok;
        if self.damp.dig_wet_ok {
            self.damp.held.clear();
            self.dig_version += 1;
        }
        self.damp.dig_wet_ok
    }

    /// Before digging out `q`: deep water nobody has seen lies right behind
    /// it (beside it on its level, or overhead). Asked before the dig
    /// reveals it.
    pub(super) fn flood_behind(&self, q: Pos) -> bool {
        std::iter::once(q.up())
            .chain(DIRS4.iter().map(|&(dx, dy)| q.offset(dx, dy, 0)))
            .any(|p| self.world.get(p).map_or(false, |t| !t.revealed && !t.frozen && t.water >= 4))
            && !self.world.outside(q)
            && !self.by_waterworks(q)
    }

    /// After digging out `q` into hidden water: a flood, told as one.
    pub(super) fn breach_flood(&mut self, q: Pos) {
        self.count("m2.floods", 1);
        self.water_cells = None;
        if self.warns.get("flood").map_or(false, |&t| self.tick < t + DAY) {
            return;
        }
        self.warns.insert("flood".into(), self.tick);
        let place = self.place_name(q).map(|n| format!(" by {}", n)).unwrap_or_default();
        let text = format!("The miners have broken into hidden water{}! It pours through the cut and on into the fort.", place);
        self.msg_at(&format!("Flood! {}", text), (90, 160, 255), Some(q), LogKind::Other);
        self.spotlight("Flood!", &text, Some(q), (90, 160, 255));
        let fort = self.fortress_name.clone();
        self.legend(format!("The miners of {} broke into hidden water, and it flooded the halls.", fort));
        if let Some(o) = self.overseer.as_mut() {
            let tick = self.tick;
            Self::note_ov(o, tick, "The miners broke into hidden water and it came pouring in.".into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hidden_water_makes_damp_stone_and_magma_warm() {
        let mut g = Game::new(81);
        let e = g.embark;
        let c = Pos::new(e.x, e.y, g.world.surface_z(e.x, e.y) - 8);
        for dz in -2..=2 {
            for dy in -4..=4 {
                for dx in -4..=4 {
                    let t = g.world.t_mut(c.offset(dx, dy, dz));
                    t.kind = TileKind::Wall;
                    t.mat = Mat::Granite;
                    t.water = 0;
                    t.magma = 0;
                    t.aquifer = false;
                    t.revealed = false;
                }
            }
        }
        assert_eq!(g.damp_near(c), None);
        let w = c.offset(2, 0, 0);
        g.world.t_mut(w).kind = TileKind::Open;
        g.world.t_mut(w).water = 7;
        assert_eq!(g.damp_near(c), Some(false));
        // Water in plain sight isn't damp stone.
        g.world.t_mut(w).revealed = true;
        assert_eq!(g.damp_near(c), None);
        g.world.t_mut(c.offset(0, 1, -1)).magma = 7;
        assert_eq!(g.damp_near(c), Some(true));
    }

    #[test]
    fn miners_stop_at_damp_stone_unless_told() {
        let mut g = Game::new(81);
        let e = g.embark;
        let c = Pos::new(e.x, e.y, g.world.surface_z(e.x, e.y) - 8);
        // A sealed run of floor, rock at its end, and a hidden pool behind.
        for dz in -1..=1 {
            for dy in -3..=3 {
                for dx in -2..=9 {
                    let t = g.world.t_mut(c.offset(dx, dy, dz));
                    t.kind = TileKind::Wall;
                    t.mat = Mat::Granite;
                    t.water = 0;
                    t.magma = 0;
                    t.aquifer = false;
                    t.revealed = false;
                    t.desig = Desig::None;
                    t.built_from = None;
                }
            }
        }
        for dx in 0..4 {
            let t = g.world.t_mut(c.offset(dx, 0, 0));
            t.kind = TileKind::Floor;
            t.dug = true;
            t.revealed = true;
        }
        let target = c.offset(4, 0, 0);
        g.world.t_mut(target).revealed = true;
        let pool = c.offset(6, 0, 0);
        g.world.t_mut(pool).kind = TileKind::Open;
        g.world.t_mut(pool).water = 7;
        assert_eq!(g.designate(target, target, Desig::Dig), 1);
        let pick = g.spawn_item(ItemKind::Pick, Mat::Iron, c);
        let mut d = g.dwarves[0].clone();
        d.pos = c;
        d.held = Some(pick);
        assert!(!g.find_dig(&mut d, false), "the miner should stop at the damp");
        assert!(g.damp.is_held(target));
        assert!(g.log.iter().any(|l| l.text.contains("damp stone")), "the damp is reported");
        assert_eq!(g.world.t(target).desig, Desig::Dig, "the player's designation waits");
        // Asked again, still no.
        assert!(!g.find_dig(&mut d, false));
        // The standing order sends them on.
        assert!(g.toggle_dig_wet());
        assert!(g.find_dig(&mut d, false));
        assert!(matches!(d.task, Task::Dig { target: t, .. } if t == target));
    }

    #[test]
    fn breaking_into_hidden_water_is_a_flood() {
        let mut g = Game::new(81);
        let e = g.embark;
        let c = Pos::new(e.x, e.y, g.world.surface_z(e.x, e.y) - 8);
        for dy in -2..=2 {
            for dx in -2..=3 {
                let t = g.world.t_mut(c.offset(dx, dy, 0));
                t.kind = TileKind::Wall;
                t.mat = Mat::Granite;
                t.revealed = false;
                t.water = 0;
            }
        }
        let pool = c.offset(1, 0, 0);
        g.world.t_mut(pool).kind = TileKind::Open;
        g.world.t_mut(pool).water = 7;
        assert!(g.flood_behind(c));
        g.world.t_mut(c).desig = Desig::Dig;
        g.complete_dig(c);
        assert!(g.spotlights.iter().any(|s| s.title == "Flood!"));
    }
}
