//! Path throttling (OTH-06): a creature that can find no way to anyone (a
//! besieger shut out by the raised bridge, a beast walled in) waits longer
//! each time before it searches again, instead of flooding the map every
//! couple of hundred ticks. A dig, a bridge lowered or a door unlocked bumps
//! `dig_version`, and that sends it looking again at once.

use super::*;

/// The most the wait doubles (120 ticks → 960).
const MAX_DOUBLINGS: u8 = 3;

impl Game {
    /// A search found no way: count it and return how long to wait before the
    /// next, `base` doubled for every failure in a row since the fortress last
    /// changed shape. (`--knob path.backoff=0` turns it off: always `base`.)
    pub(super) fn no_way_wait(&self, c: &mut Creature, base: u32) -> u32 {
        if self.knob("path.backoff", 1.0) <= 0.0 {
            return base;
        }
        if c.fail_ver != self.dig_version {
            c.path_fails = 0;
        }
        c.path_fails = c.path_fails.saturating_add(1).min(100);
        c.fail_ver = self.dig_version;
        base << c.path_fails.min(MAX_DOUBLINGS)
    }

    /// Shut out twice or more, with nothing dug or opened since: it doesn't
    /// run the long searches for a way in this time, only prowls. Every
    /// fourth time it looks anyway, in case a way opened that no dig shows.
    pub(super) fn still_shut_out(&self, c: &Creature) -> bool {
        self.knob("path.backoff", 1.0) > 0.0 && c.path_fails >= 2 && c.fail_ver == self.dig_version && c.path_fails % 4 != 0
    }

    /// The chance a tick that a creature with nowhere to go sets off to prowl
    /// about. Shut out, it mostly stands and waits at the walls.
    pub(super) fn prowl_chance(&self, c: &Creature) -> f64 {
        if c.path_fails > 0 && c.fail_ver == self.dig_version && self.knob("path.backoff", 1.0) > 0.0 {
            0.01
        } else if c.hostile() {
            0.05
        } else {
            0.03
        }
    }

    /// The wait after a turn spent prowling while shut out.
    pub(super) fn shut_out_wait(&self, c: &mut Creature) -> u32 {
        self.no_way_wait(c, 120)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::Species;
    use crate::world::TileKind;

    /// A goblin of a siege outside, every dwarf sealed in a pocket in the rock.
    /// Returns the path searches it ran in `ticks` ticks.
    fn searches_while_shut_out(backoff: f64, ticks: u64) -> (u64, Game) {
        let mut g = Game::new(21);
        g.knobs.set.insert("path.backoff".into(), backoff);
        g.creatures.retain(|_, c| c.tame);
        // Nothing lying about worth carrying off (a thief goes home).
        g.items.clear();
        let a = g.open_patch(3, 3, 6).offset(1, 1, 0);
        // A pocket deep in solid rock, with no way to it.
        let (w, h, d) = (g.world.w, g.world.h, g.world.d);
        let solid = |g: &Game, p: Pos| g.world.get(p).map_or(true, |t| t.kind == TileKind::Wall);
        let pocket = (1..d)
            .flat_map(|z| (2..h - 2).flat_map(move |y| (2..w - 2).map(move |x| Pos::new(x, y, z))))
            .find(|&p| p.z < a.z - 2 && (-1..=1).all(|dz| (-1..=1).all(|dy| (-1..=1).all(|dx| solid(&g, p.offset(dx, dy, dz))))))
            .expect("solid rock somewhere");
        g.world.t_mut(pocket).kind = TileKind::Floor;
        for dw in g.dwarves.iter_mut() {
            dw.pos = pocket;
            dw.path.clear();
        }
        let gob = g.spawn_creature(Species::Goblin, a, u64::MAX);
        let mut c = g.creatures.remove(&gob).unwrap();
        c.siege = true;
        c.think_cd = 0;
        let before = crate::path::CALLS.with(|n| n.get());
        for _ in 0..ticks {
            assert!(g.creature_tick(&mut c), "the goblin should still be here");
            assert!(c.target.is_none() && !c.leaving, "nobody is reachable");
            g.tick += 1;
        }
        let n = crate::path::CALLS.with(|n| n.get()) - before;
        g.creatures.insert(gob, c);
        (n, g)
    }

    #[test]
    fn a_creature_shut_out_searches_less() {
        let (today, _) = searches_while_shut_out(0.0, 1000);
        let (now, mut g) = searches_while_shut_out(1.0, 1000);
        println!("shut out for 1000 ticks: {} searches without backoff, {} with", today, now);
        assert!(today >= 8, "shut out without backoff it searched only {} times", today);
        assert!(now * 4 <= today, "with backoff {} searches, without {}", now, today);
        // A dig anywhere sends it looking again at once.
        let id = *g.creatures.keys().find(|&&k| g.creatures[&k].siege).unwrap();
        g.dig_version += 1;
        let mut c = g.creatures.remove(&id).unwrap();
        assert!(!g.still_shut_out(&c), "a changed fortress is worth a look");
        c.think_cd = 0;
        let before = crate::path::CALLS.with(|n| n.get());
        g.creature_tick(&mut c);
        let n = crate::path::CALLS.with(|n| n.get()) - before;
        assert!(n >= 2, "it searched for a way in again ({} searches)", n);
    }
}
