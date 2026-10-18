//! What towers are for: a soldier keeps watch from a watchtower in peace
//! (and a watched tower sees an army coming sooner), marksdwarves hold the
//! gate towers' tops in an alert and shoot down from them over the
//! battlements, the towers' doors are barred when the enemy comes close,
//! and the curtain's drawbridge rises with the gate's own. The towers
//! themselves are laid out and built in `fortify.rs`.

use super::fortify::WorkKind;
use super::military::{UniformWeapon, RANGE};
use super::overseer::Overseer;
use super::*;

/// How many levels down a bolt can be shot from a tower.
pub const MAX_DROP: i32 = 6;

impl Game {
    /// A straight shot from `a` to `b`: on one level as `clear_shot` has it,
    /// or down from a height, over whatever stands lower in between and
    /// through battlements, onto a target under the open sky.
    pub fn sight(&self, a: Pos, b: Pos) -> bool {
        if a.z == b.z {
            return self.clear_shot(a, b);
        }
        let drop = a.z - b.z;
        if !(1..=MAX_DROP).contains(&drop) {
            return false;
        }
        // Nothing solid on the shooter's own level all the way out...
        let over = Pos::new(b.x, b.y, a.z);
        if !self.clear_shot(a, over) || self.world.solid(over) {
            return false;
        }
        // ...and nothing but air between there and the target.
        (b.z + 1..a.z).all(|z| self.world.kind(Pos::new(b.x, b.y, z)) == Some(TileKind::Open))
    }

    /// Whether `b` is in crossbow range from `a`: a little further from a height.
    pub fn in_range(a: Pos, b: Pos) -> bool {
        let reach = RANGE + (a.z - b.z).clamp(0, MAX_DROP);
        (a.x - b.x).abs().max((a.y - b.y).abs()) <= reach
    }

    /// In range and in sight.
    pub fn can_shoot(&self, a: Pos, b: Pos) -> bool {
        Self::in_range(a, b) && self.sight(a, b)
    }

    /// Finished towers with a battlement top whose door is hung: the ones
    /// soldiers can hold. Gate towers first.
    fn manned_towers(&self) -> Vec<usize> {
        let mut v: Vec<usize> = (0..self.works.list.len())
            .filter(|&i| {
                let w = &self.works.list[i];
                matches!(w.kind, WorkKind::GateTower | WorkKind::Watchtower)
                    && w.done()
                    && w.door.map_or(false, |p| self.buildings.values().any(|b| b.kind == BuildingKind::Door && b.pos == p && b.built))
                    && self.world.walkable(w.top())
            })
            .collect();
        v.sort_by_key(|&i| (self.works.list[i].kind != WorkKind::GateTower, i));
        v
    }

    /// The spot on a tower's top platform on its lookout side.
    fn lookout(&self, i: usize) -> Pos {
        let w = &self.works.list[i];
        let p = w.top().offset(w.outward.0.signum(), w.outward.1.signum(), 0);
        if self.world.walkable(p) {
            p
        } else {
            w.top()
        }
    }

    /// Where a squad of marksdwarves holds in an alert, if a tower can take
    /// them: the top of a gate tower (or a watchtower), over the approach.
    pub(super) fn tower_station(&self, squad: usize) -> Option<Pos> {
        if squad == 0 || self.squads.get(squad).map_or(true, |s| s.weapon != UniformWeapon::Crossbow) {
            return None;
        }
        let towers = self.manned_towers();
        if towers.is_empty() {
            return None;
        }
        let archers = (1..self.squads.len()).filter(|&s| self.squads[s].weapon == UniformWeapon::Crossbow).position(|s| s == squad).unwrap_or(0);
        Some(self.lookout(towers[archers % towers.len()]))
    }

    /// Where a soldier keeps watch in peacetime, if they are the watchman:
    /// the top of the first watchtower (else a gate tower).
    pub(super) fn watch_post(&self, dwarf: u32) -> Option<Pos> {
        if self.works.watchman != Some(dwarf) || self.alert {
            return None;
        }
        let towers = self.manned_towers();
        let i = towers.iter().copied().find(|&i| self.works.list[i].kind == WorkKind::Watchtower).or(towers.first().copied())?;
        Some(self.lookout(i))
    }

    /// Daily: choose the watchman, a soldier of the last squad (a
    /// marksdwarf if there is one), once there is a tower to watch from.
    pub(super) fn pick_watchman(&mut self, o: &mut Overseer) {
        if self.manned_towers().is_empty() {
            self.works.watchman = None;
            return;
        }
        let ok = |g: &Game, id: u32| g.dwarves.iter().any(|d| d.id == id && d.alive && d.soldier && !d.insane);
        if self.works.watchman.map_or(false, |id| ok(self, id)) {
            return;
        }
        let pick = self
            .squads
            .iter()
            .rev()
            .flat_map(|s| s.members.iter().copied())
            .filter(|&id| ok(self, id))
            .max_by_key(|&id| self.dwarves.iter().find(|d| d.id == id).map_or(false, |d| self.holds_crossbow(d)));
        if let Some(id) = pick {
            let first = self.works.watchman.is_none();
            self.works.watchman = Some(id);
            let name = self.dwarves.iter().find(|d| d.id == id).map_or(String::new(), |d| d.called());
            let tower = self.watch_post(id).and_then(|p| self.work_at(p)).unwrap_or_else(|| "the tower".into());
            if first {
                Self::note_ov(o, self.tick, format!("Set {} to keep watch from {}.", name, tower));
            }
        }
    }

    /// The watch: a soldier on a watchtower's top now (or lately) sees an
    /// army coming half a day sooner. Returns the extra warning in ticks and
    /// the tower that saw them.
    pub(super) fn watch_warning(&self) -> (u64, Option<String>) {
        let tops: Vec<(Pos, String)> = self.works.list.iter().filter(|w| w.kind == WorkKind::Watchtower && w.done()).map(|w| (w.top(), w.name.clone())).collect();
        for (top, name) in &tops {
            let on = self.dwarves.iter().any(|d| d.alive && !d.asleep && d.pos.z == top.z && (d.pos.x - top.x).abs() <= 1 && (d.pos.y - top.y).abs() <= 1);
            if on || (self.works.watched_at > 0 && self.tick < self.works.watched_at + DAY / 2) {
                return (DAY / 2, Some(name.clone()));
            }
        }
        (0, None)
    }

    /// Every ten ticks from the alert check: note who is on watch, bar the
    /// towers' doors when the enemy is close (once those meant to be up
    /// there are), and raise or lower the curtain's drawbridge with the
    /// gate's.
    pub(super) fn ov_towers_alert(&mut self, o: &mut Overseer) {
        if self.works.list.is_empty() {
            return;
        }
        if self.tick % 100 < 10 {
            for w in self.works.list.iter().filter(|w| w.kind == WorkKind::Watchtower && w.done()) {
                let top = w.top();
                if self.dwarves.iter().any(|d| d.alive && d.pos.z == top.z && (d.pos.x - top.x).abs() <= 1 && (d.pos.y - top.y).abs() <= 1) {
                    self.works.watched_at = self.tick;
                }
            }
        }
        let foes: Vec<Pos> = self.creatures.values().filter(|c| c.alive && c.hostile() && !c.sneaking && (c.siege || c.legendary())).map(|c| c.pos).collect();
        // The doors.
        let posted: Vec<Pos> = self.squads.iter().filter(|s| s.order == SquadOrder::Station).filter_map(|s| s.station).collect();
        let mut toggle = Vec::new();
        for w in self.works.list.iter().filter(|w| w.kind.is_tower()) {
            let Some(dp) = w.door else { continue };
            let Some(b) = self.buildings.values().find(|b| b.kind == BuildingKind::Door && b.pos == dp && b.built) else { continue };
            let near = |r: i32| foes.iter().any(|f| (f.x - dp.x).abs() <= r && (f.y - dp.y).abs() <= r && (f.z - dp.z).abs() <= 2);
            // Those posted up there and not there yet.
            let top = w.top();
            let ours: Vec<u32> = self.squads.iter().filter(|s| s.order == SquadOrder::Station && s.station.map_or(false, |st| st.z == top.z && (st.x - top.x).abs() <= 2 && (st.y - top.y).abs() <= 2)).flat_map(|s| s.members.iter().copied()).collect();
            let outside = self.dwarves.iter().any(|d| ours.contains(&d.id) && d.alive && !d.body.unconscious() && !w.covers(d.pos));
            let want = near(10) && (!outside || near(3));
            if b.locked != want {
                toggle.push(b.id);
            }
        }
        let _ = posted;
        for id in toggle {
            self.toggle_lock(id);
        }
        // The curtain's drawbridge goes with the gate's.
        if o.did("curtainbridge") {
            let base = o.base.unwrap();
            let gate_up = self.buildings.values().any(|b| b.kind == BuildingKind::Bridge && b.raised && b.pos.z == base.z && b.pos.x == base.x + 2 && (b.pos.y - base.y).abs() <= 1);
            let lp = self.works.lever;
            let lever = self.buildings.values().find(|b| b.kind == BuildingKind::Lever && Some(b.pos) == lp && b.built && !b.links.is_empty()).map(|b| (b.id, b.pos, b.pull));
            let up = self.buildings.values().any(|b| b.kind == BuildingKind::Bridge && b.raised && self.works.bridges.contains(&b.pos));
            if let Some((id, lp, pull)) = lever {
                if up != gate_up {
                    let by = self.dwarves.iter().any(|d| d.alive && d.pos.z == lp.z && (d.pos.x - lp.x).abs() <= 1 && (d.pos.y - lp.y).abs() <= 1);
                    if by {
                        self.pull_lever(id);
                    } else if !pull {
                        self.request_pull(id);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::fortify::{tower_steps, Crown, Work};
    use super::*;
    use crate::creature::Species;

    /// A finished watchtower beside the wagon, and its top.
    fn a_tower(g: &mut Game) -> usize {
        let e = g.embark;
        let c = (6..40)
            .flat_map(|r| [(r, 0), (-r, 0), (0, r), (0, -r), (r, r), (-r, -r)])
            .find_map(|(dx, dy)| {
                g.tower_site(e.x + dx, e.y + dy, 3, e).filter(|c| {
                    // Level open ground in front of it to shoot over.
                    c.y >= 12 && (3..=9).all(|k| g.world.surface_z(c.x, c.y - k) == c.z && g.world.walkable(c.offset(0, -k, 0)))
                })
            })
            .expect("no tower site");
        let steps = tower_steps(c, 3, (0, 2), &[], Crown::Battlements);
        g.works.list.push(Work {
            kind: WorkKind::Watchtower,
            name: "the Test Watchtower".into(),
            at: c,
            height: 3,
            door: Some(c.offset(0, 2, 0)),
            outward: (0, -1),
            steps,
            step: 0,
            hold: usize::MAX,
            mark: 0,
            since: 0,
            begun: 0,
            finished: 0,
            skipped: 0,
        });
        g.raise_works_now();
        g.works.list.len() - 1
    }

    #[test]
    fn a_marksdwarf_shoots_down_from_a_tower() {
        let mut g = Game::new(71);
        let i = a_tower(&mut g);
        let w = g.works.list[i].clone();
        let top = w.top().offset(0, -1, 0);
        assert!(g.world.walkable(top), "the top isn't a platform");
        // Up the stair from the foot to the top, through the door.
        let outside = w.at.offset(0, 4, 0);
        assert!(path::astar(&g.world, outside, top, 20_000, Mover::Dwarf).is_some(), "no way up the tower");
        // A goblin on the ground in front of the tower, a few tiles out.
        let gp = w.at.offset(0, -6, 0);
        assert!(g.world.walkable(gp));
        assert!(g.sight(top, gp) && g.can_shoot(top, gp), "no shot from the top");
        assert!(!g.sight(gp, top) || gp.z == top.z, "shots go down, not up");
        // A goblin right under a solid roof can't be seen.
        let gob = g.spawn_creature(Species::Goblin, gp, u64::MAX);
        g.creatures.get_mut(&gob).unwrap().siege = true;
        let d0 = g.dwarves[0].id;
        let bow = g.spawn_item(ItemKind::Crossbow, crate::material::Mat::Iron, top);
        {
            let d = &mut g.dwarves[0];
            d.pos = top;
            d.soldier = true;
            d.held = Some(bow);
            d.ammo = 20;
        }
        g.items.get_mut(&bow).unwrap().carried_by = Some(d0);
        let hp0: i32 = g.creatures[&gob].body.hp.iter().sum();
        let mut shot = false;
        for _ in 0..200 {
            let mut d = std::mem::take(&mut g.dwarves[0]);
            d.attack_cd = d.attack_cd.saturating_sub(1);
            g.archer_awareness(&mut d);
            g.dwarves[0] = d;
            if g.dwarves[0].ammo < 20 {
                shot = true;
            }
            if !g.creatures.contains_key(&gob) {
                break;
            }
        }
        assert!(shot, "the marksdwarf on the tower never shot");
        let hurt = g.creatures.get(&gob).map_or(true, |c| c.body.hp.iter().sum::<i32>() < hp0);
        assert!(hurt, "twenty bolts and not a scratch");
    }

    #[test]
    fn a_manned_watchtower_warns_of_an_army_sooner() {
        let warning = |manned: bool| {
            let mut g = Game::new(72);
            let i = a_tower(&mut g);
            if manned {
                let top = g.works.list[i].top();
                g.dwarves[0].pos = top;
            }
            // An enemy that comes for certain, past the first two seasons.
            g.tick = SEASON * 3;
            g.siege_warned = true;
            for _ in 0..50 {
                if g.incoming.is_some() {
                    break;
                }
                g.siege_warned = true;
                g.maybe_siege();
            }
            let (at, ..) = g.incoming.clone().expect("no army came");
            let seen = g.log.iter().any(|l| l.text.contains("Test Watchtower"));
            (at - g.tick, seen)
        };
        let (plain, seen0) = warning(false);
        let (watched, seen1) = warning(true);
        assert!(!seen0 && seen1, "the watch should be named only when it saw them");
        assert_eq!(watched, plain + DAY / 2, "the watch saw them no sooner");
    }
}
