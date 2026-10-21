//! The reckless levy. A Reckless overseer keeps only a few under arms and
//! builds no gate, so when an army or a beast out of legend came, those
//! few died at the stair and the rest of the fort was cut down at its work
//! (in the release batch no Reckless fort lived three years). It still
//! makes no plans: but when the scouts cry an army, or a beast comes into
//! the halls, it hands out whatever is in the stores and sends every hand
//! it can spare at it, as many as the enemy brings, and sends them back to
//! work once it's over. Numbers, not walls: costly, and some forts still
//! fall when the numbers aren't there.

use super::military::SquadOrder;
use super::overseer::{Overseer, Style};
use super::*;

impl Game {
    /// What the Reckless levy wants under arms right now (`ov_army`): for
    /// an army, as many as are coming (knob reckless.levy dwarves a foe),
    /// for a beast in the halls a band of them (knob reckless.levy_beast);
    /// never all the fort (a few keep the stores, knob reckless.keep).
    pub(super) fn reckless_levy(&self, o: &Overseer, adults: usize) -> usize {
        let army = self.incoming.as_ref().map_or(0, |i| i.2 as usize) + self.creatures.values().filter(|c| c.alive && c.siege && !c.legendary() && c.hostile() && !c.leaving).count();
        let mut want = (army as f64 * self.knob("reckless.levy", 1.0)).ceil() as usize;
        if let (Some(top), Some(base)) = (o.top, o.base) {
            if self.beast_in_halls(top, base).is_some() {
                want = want.max(self.knob("reckless.levy_beast", 6.0) as usize);
            }
        }
        want.min(adults.saturating_sub(self.knob("reckless.keep", 3.0) as usize))
    }

    /// Every ten ticks (`overseer_hooks.rs`): the levy goes after a beast
    /// in the halls, and goes home when the danger is past.
    pub(super) fn ov_levy(&mut self, o: &mut Overseer) {
        if o.style != Style::Reckless || self.squads.is_empty() {
            return;
        }
        let (Some(top), Some(base)) = (o.top, o.base) else { return };
        if let Some(beast) = self.beast_in_halls(top, base) {
            if !self.squads.iter().any(|s| s.order == SquadOrder::Kill(beast)) {
                let s = (0..self.squads.len()).max_by_key(|&s| self.squads[s].members.len()).unwrap_or(0);
                if self.squads[s].order != SquadOrder::Kill(beast) {
                    self.squads[s].order = SquadOrder::Kill(beast);
                    let name = self.creatures.get(&beast).map_or("beast".to_string(), |c| c.name().to_string());
                    let text = format!("The overseer {} raises the levy: every dwarf who can be spared is to go at the {}!", o.name, name);
                    self.msg(&text, (255, 190, 90));
                    self.count("levy.beasts", 1);
                }
            }
            return;
        }
        // The danger past: back to work, all but the few it always keeps.
        if self.alert || self.incoming.is_some() || self.tick % 150 != 3 {
            return;
        }
        let adults = self.dwarves.iter().filter(|d| d.alive && !d.is_child()).count();
        let keep = self.reckless_standing(adults);
        let mut soldiers: Vec<(u32, u32)> = self.squads.iter().flat_map(|s| s.members.iter().copied()).filter_map(|id| self.dwarves.iter().find(|d| d.id == id && d.alive)).map(|d| (d.fight_xp, d.id)).collect();
        if soldiers.len() <= keep {
            return;
        }
        // The best fighters stay.
        soldiers.sort_by(|a, b| b.cmp(a));
        let home: Vec<u32> = soldiers[keep..].iter().map(|s| s.1).collect();
        for id in &home {
            if let Some(s) = self.squads.iter().position(|s| s.members.contains(id)) {
                self.set_squad_member(s, *id, false);
            }
        }
        let text = o.voiced(&format!("Sent {} of the levy back to their work.", home.len()));
        Self::note_ov(o, self.tick, text);
        self.count("levy.home", 1);
    }

    /// A beast out of legend (a horror up from the caverns, a titan, a
    /// dragon) near the stair or the halls and able to get at them.
    fn beast_in_halls(&self, top: Pos, base: Pos) -> Option<u32> {
        let r = self.knob("reckless.levy_range", 30.0) as i32;
        let near = |p: Pos| {
            let by = |q: Pos| (p.x - q.x).abs() <= r && (p.y - q.y).abs() <= r;
            (by(top) || by(base)) && p.z >= base.z - 3 && p.z <= top.z + 2
        };
        self.creatures
            .values()
            .filter(|c| c.alive && c.legendary() && c.hostile() && !c.leaving && self.creature_active(c) && near(c.pos) && self.can_reach(c, base, 8000))
            .min_by_key(|c| (crate::game::combat::dist2(c.pos, base), c.id))
            .map(|c| c.id)
    }

    /// The few a Reckless fort keeps under arms between dangers: a handful
    /// once there are hands enough (knobs reckless.squad, reckless.squad_from),
    /// and once it has been attacked a share of the adults (reckless.share).
    pub(super) fn reckless_standing(&self, adults: usize) -> usize {
        let mut want = 0;
        if adults >= self.knob("reckless.squad_from", 8.0) as usize {
            want = self.knob("reckless.squad", 3.0) as usize;
        }
        if self.siege_size > 0 {
            want = want.max(2).max((adults as f64 * self.knob("reckless.share", 0.2)).ceil() as usize);
        }
        want
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::overseer::trial_game;

    /// Scouts cry an army of ten: the Reckless overseer puts as many under
    /// arms as are coming (leaving a few at home); once it's gone, all but
    /// its usual few go back to work.
    #[test]
    fn the_reckless_raise_a_levy_and_send_it_home() {
        let mut g = (7000..7010u64).map(|s| trial_game(s, Style::Reckless, 100)).find(|g| g.dwarves.iter().filter(|d| d.alive && !d.is_child()).count() >= 12 && !g.neighbors.is_empty()).expect("a Reckless fort of a dozen by day 100");
        g.creatures.retain(|_, c| c.tame);
        let adults = g.dwarves.iter().filter(|d| d.alive && !d.is_child()).count();
        let soldiers = |g: &Game| g.dwarves.iter().filter(|d| d.alive && d.soldier).count();
        let standing = g.reckless_standing(adults);
        let foe = g.neighbors[0].clone();
        // (Far enough off that it doesn't arrive while we watch.)
        g.incoming = Some((g.tick + 20 * DAY, foe, 10, "Snagbub".into()));
        let start = g.tick;
        while g.tick < start + 400 {
            g.step();
        }
        // (Nobles and those holding a post aren't levied.)
        // (As many as can be spared: those holding a post or a title stay.)
        let free = g.dwarves.iter().filter(|d| d.alive && !d.is_child() && !d.insane && d.noble.is_none() && g.position_of(d.id).is_none()).count();
        assert!(soldiers(&g) >= (standing * 2).max(5).min(free), "only {} under arms against ten ({} usually, {} free)", soldiers(&g), standing, free);
        assert!(soldiers(&g) <= adults - 3, "nobody left at home");
        // The army thinks better of it.
        g.incoming = None;
        let start = g.tick;
        while g.tick < start + DAY && soldiers(&g) > standing {
            g.step();
        }
        assert!(soldiers(&g) <= standing.max(2), "the levy never went home: {} under arms", soldiers(&g));
        assert!(g.annals.counters.get("levy.home").is_some());
    }
}
