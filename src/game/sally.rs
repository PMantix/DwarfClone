//! A Warlord's sally. With an army shut out at the raised bridge, a
//! Warlord who war-gamed the fight and liked the odds won't sit behind its
//! walls: it lowers the bridge and sends every squad out to break the
//! siege in the open. Mostly they do, and the fort's name grows (a broken
//! siege brings a bigger one next time, `siege_strength`). Now and then the
//! war-game flattered them: the squads are cut down before the gate, the
//! overseer calls the rest back and raises the bridge, and whether it comes
//! up before the army is through decides the fort.

use super::military::SquadOrder;
use super::overseer::{Overseer, Style};
use super::personality::Trait;
use super::*;
use serde::{Deserialize, Serialize};

/// A sally under way.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Sally {
    /// The siege it rode out against (index into `annals.sieges`).
    pub siege: usize,
    pub since: u64,
    /// Soldiers who rode out.
    pub soldiers: u32,
    /// Broken: the survivors are called back and the bridge raised.
    pub recalled: bool,
}

/// The fort's sallies (`Game.sallies`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Sallies {
    #[serde(default)]
    pub on: Option<Sally>,
    /// The last siege sallied against: one sally a siege.
    #[serde(default)]
    pub last: Option<usize>,
    /// Sallies that broke a siege: the fort's martial fame, which draws
    /// bigger armies (`siege_strength`).
    #[serde(default)]
    pub won: u32,
}

impl Game {
    /// What the fort's fame for sallies adds to an army: those it broke in
    /// the open tell of it, and the next comes the stronger (knob sally.fame
    /// for each sally won).
    pub(super) fn sally_fame(&self) -> u32 {
        (self.sallies.won as f64 * self.knob("sally.fame", 1.0)).round() as u32
    }

    /// Squads are out beyond the gate and the bridge stays down for them
    /// (read by the alert check, `ov_alert`).
    pub(super) fn sallying(&self) -> bool {
        self.sallies.on.as_ref().map_or(false, |s| !s.recalled)
    }

    /// Besiegers still on the map and in the fight, and any fleeing ones
    /// that got inside the walls while the bridge was down: once it's up
    /// again they can't get out, and would hold the fort in alarm, shut in,
    /// until it died of thirst.
    fn besiegers(&self) -> Vec<(u32, Pos)> {
        self.creatures
            .values()
            .filter(|c| c.alive && c.siege && !c.legendary() && c.hostile() && (!c.leaving || !self.world.outside(c.pos)))
            .map(|c| (c.id, c.pos))
            .collect()
    }

    /// Soldiers alive and grown.
    fn soldiers_up(&self) -> u32 {
        self.dwarves.iter().filter(|d| d.alive && d.soldier && !d.is_child()).count() as u32
    }

    /// The share of war-game bouts won that is enough to ride out on: a
    /// Warlord's pride makes a close fight look like a sure one, boldness
    /// and vanity the more so; caution wants better. And every sally that
    /// broke a siege makes the next look surer (knob sally.hubris), until
    /// it rides out against odds the arena called grim.
    fn sally_odds(&self, o: &Overseer) -> f64 {
        let mut odds = self.knob("sally.odds", 0.5) - self.sallies.won as f64 * self.knob("sally.hubris", 0.08);
        if o.has(Trait::Bold) || o.has(Trait::Vain) {
            odds -= 0.15;
        }
        if o.has(Trait::Cautious) {
            odds += 0.25;
        }
        odds
    }

    /// Every ten ticks (`overseer_hooks.rs`): ride out, see the sally
    /// through, or call it back.
    pub(super) fn ov_sally(&mut self, o: &mut Overseer) {
        if self.sallies.on.is_some() {
            self.sally_goes_on(o);
            return;
        }
        if o.style != Style::Warlord || !o.did("linked") {
            return;
        }
        let (Some(top), Some(base)) = (o.top, o.base) else { return };
        let Some(i) = self.annals.sieges.iter().rposition(|s| s.ended.is_none()) else { return };
        if self.sallies.last == Some(i) {
            return;
        }
        // Shut out: the bridge is up, and the army is at it.
        let raised = self.gate_bridges(base).iter().any(|id| self.buildings.get(id).map_or(false, |b| b.raised));
        let foes = self.besiegers();
        let at_gate = foes.iter().any(|&(_, p)| (p.x - top.x).abs() <= 12 && (p.y - top.y).abs() <= 12);
        let soldiers = self.soldiers_up();
        if !raised || !at_gate || soldiers < self.knob("sally.min_soldiers", 4.0) as u32 {
            return;
        }
        // What the war-game of this army said.
        let siege_at = self.annals.sieges[i].tick;
        let Some(w) = self.wargame.as_ref().filter(|w| w.bouts > 0 && siege_at >= w.due && siege_at <= w.due + DAY) else { return };
        let (wins, bouts) = (w.wins, w.bouts);
        if (wins as f64) < self.sally_odds(o).max(self.knob("sally.least", 0.1)) * bouts as f64 {
            return;
        }
        self.sallies.last = Some(i);
        self.sallies.on = Some(Sally { siege: i, since: self.tick, soldiers, recalled: false });
        for s in 0..self.squads.len() {
            self.sally_target(s, false);
        }
        let foe = self.annals.sieges[i].foe.clone();
        let fort = self.fortress_name.clone();
        let text = format!(
            "The overseer {} will not cower behind the bridge: it is lowered, and {} soldiers ride out of {} to meet the {} in the open! (The war-game gave them {} bouts in {}.)",
            o.name, soldiers, fort, foe, wins, bouts
        );
        self.msg(&text, (255, 190, 90));
        self.spotlight("A sally!", &text, Some(top), (255, 190, 90));
        let note = o.voiced(&format!("Lowered the bridge and sent the squads out against the {}: we won {} bouts in {}.", foe, wins, bouts));
        Self::note_ov(o, self.tick, note);
        self.count("sally.sent", 1);
    }

    /// Send squad `s` after the besieger nearest its soldiers (only those
    /// inside the walls, `inside`).
    fn sally_target(&mut self, s: usize, inside: bool) {
        let members = self.squads[s].members.clone();
        let at: Vec<Pos> = self.dwarves.iter().filter(|d| d.alive && members.contains(&d.id)).map(|d| d.pos).collect();
        if at.is_empty() {
            return;
        }
        let near = |p: Pos| at.iter().map(|q| (p.x - q.x).abs() + (p.y - q.y).abs() + 4 * (p.z - q.z).abs()).min().unwrap_or(i32::MAX);
        let Some((id, _)) = self.besiegers().into_iter().filter(|&(_, p)| !inside || !self.world.outside(p)).min_by_key(|&(_, p)| near(p)) else { return };
        if self.squads[s].order != SquadOrder::Kill(id) {
            self.squads[s].order = SquadOrder::Kill(id);
        }
    }

    /// The sally under way: keep the squads on the enemy, call them back if
    /// it's going badly, and tell how it ended.
    fn sally_goes_on(&mut self, o: &mut Overseer) {
        let Some(mut sally) = self.sallies.on.clone() else { return };
        let foes = self.besiegers();
        let ended = self.annals.sieges.get(sally.siege).map_or(true, |s| s.ended.is_some());
        let fort = self.fortress_name.clone();
        if foes.is_empty() || ended {
            self.sallies.on = None;
            if sally.recalled {
                return;
            }
            let lost = sally.soldiers.saturating_sub(self.soldiers_up());
            let routed = self.annals.sieges.get(sally.siege).map_or(false, |s| s.outcome == "routed") || foes.is_empty();
            for s in 0..self.squads.len() {
                if matches!(self.squads[s].order, SquadOrder::Kill(_)) {
                    self.squads[s].order = SquadOrder::Train;
                }
            }
            if routed {
                let text = format!("The sally from {} has broken the siege! {} soldiers rode out; {} did not come back.", fort, sally.soldiers, lost);
                self.msg(&text, (255, 210, 120));
                self.legend(format!("The squads of {} rode out against the besiegers and broke them in the open.", fort));
                let note = o.voiced("The sally broke them: let the next army know what waits at our gate.");
                Self::note_ov(o, self.tick, note);
                self.count("sally.won", 1);
                self.sallies.won += 1;
            }
            return;
        }
        let up = self.soldiers_up();
        // A proud overseer leaves them out there longer.
        let mut hold = self.knob("sally.recall", 0.5);
        if o.has(Trait::Stubborn) || o.has(Trait::Vain) {
            hold *= 0.5;
        }
        let long = self.tick > sally.since + self.knob("sally.days", 2.0) as u64 * DAY;
        if !sally.recalled && ((up as f64) < hold * sally.soldiers as f64 || long) {
            // Broken, or dragged on too long: back inside, and the bridge up.
            sally.recalled = true;
            for s in 0..self.squads.len() {
                if matches!(self.squads[s].order, SquadOrder::Kill(_)) {
                    self.squads[s].order = SquadOrder::Train;
                }
            }
            if !long {
                let text = format!(
                    "The sally from {} is broken! Only {} of {} soldiers still stand, and the overseer {} cries for the bridge to be raised.",
                    fort, up, sally.soldiers, o.name
                );
                self.msg(&text, (255, 60, 60));
                self.spotlight("The sally is broken", &text, o.top, (255, 60, 60));
                self.legend(format!("A sally from {} was cut down before its own gate.", fort));
                let note = o.voiced("The war-game lied. Raise the bridge, raise it now!");
                Self::note_ov(o, self.tick, note);
                self.count("sally.broken", 1);
            }
            self.sallies.on = Some(sally);
            return;
        }
        // (Called back, they still go after any who got inside.)
        for s in 0..self.squads.len() {
            let on = match self.squads[s].order {
                SquadOrder::Kill(id) => foes.iter().any(|f| f.0 == id && (!sally.recalled || !self.world.outside(f.1))),
                _ => false,
            };
            if !on {
                self.sally_target(s, sally.recalled);
            }
        }
        self.sallies.on = Some(sally);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Warlord with its bridge up and a confident war-game rides out:
    /// the bridge comes down, the squads go after the besiegers, and the
    /// sally ends told one way or the other, with no besieger left inside
    /// the walls. The first Warlord fort from seed 7000 on that sallies
    /// within a year and a half.
    #[test]
    fn a_warlord_sallies_and_it_is_told() {
        for seed in 7000..7008u64 {
            let mut g = Game::new(seed);
            g.set_overseer(Some(Style::Warlord));
            while g.tick < YEAR * 3 / 2 && g.alive_count() > 0 && g.sallies.on.is_none() {
                g.step();
            }
            let Some(sally) = g.sallies.on.clone() else { continue };
            let base = g.overseer.as_ref().unwrap().base.unwrap();
            assert!(sally.soldiers >= 4);
            assert!(g.squads.iter().any(|s| matches!(s.order, SquadOrder::Kill(_))), "nobody sent out: {:?}", g.squads.iter().map(|s| s.order).collect::<Vec<_>>());
            assert_eq!(g.annals.counters.get("sally.sent"), Some(&1));
            // The bridge comes down for them.
            let start = g.tick;
            let mut lowered = false;
            while g.tick < start + 3 * DAY && g.sallies.on.is_some() {
                g.step();
                lowered |= g.sallies.on.as_ref().map_or(false, |s| !s.recalled) && !g.gate_bridges(base).iter().any(|id| g.buildings[id].raised);
            }
            assert!(lowered, "the bridge stayed up for the sally");
            assert!(g.sallies.on.is_none(), "the sally never ended");
            let told = ["sally.won", "sally.broken"].iter().any(|k| g.annals.counters.contains_key(*k));
            let fell = g.alive_count() == 0;
            assert!(told || fell, "the sally ended untold: {:?}", g.annals.counters.iter().filter(|c| c.0.starts_with("sally")).collect::<Vec<_>>());
            // Nobody of theirs left inside to hold the fort shut.
            assert!(!g.creatures.values().any(|c| c.alive && c.siege && !c.legendary() && !g.world.outside(c.pos)), "a besieger is still inside the walls");
            assert_eq!(g.sallies.last, Some(sally.siege));
            return;
        }
        panic!("no Warlord fort from seed 7000 on sallied in a year and a half");
    }
}
