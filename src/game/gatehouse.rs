//! The gatehouse (M2 W0-B: moved verbatim out of overseer.rs, so O04 and
//! the M1 follow-ups edit this file): the gate at the foot of the stair,
//! its drawbridge on a lever, the cage traps behind it, and barring the
//! gate and raising the bridge when an army comes.

use super::overseer::{Overseer, Style};
use super::*;

impl Game {
    /// Bar the gate when enemies are close, and raise the drawbridge
    /// against an army (from `ov_alert`, every ten ticks).
    pub(super) fn ov_gate_alert(&mut self, o: &mut Overseer, top: Pos, base: Pos, coming: bool) {
        // Bar the gate when they're close (not before, so those outside can
        // still get in).
        let close = self.creatures.values().any(|c| {
            c.alive && c.hostile() && !c.sneaking && (c.siege || c.legendary()) && {
                let p = c.pos;
                let by = |q: Pos, r: i32| (p.x - q.x).abs() <= r && (p.y - q.y).abs() <= r && (p.z - q.z).abs() <= 1;
                by(top, 10) || by(base, 12)
            }
        });
        let gate: Vec<u32> = self
            .buildings
            .values()
            .filter(|b| {
                b.kind == BuildingKind::Door
                    && b.built
                    && b.locked != (close && !self.sallying())
                    && (((b.pos.x - base.x).abs() <= 1 && (b.pos.y - base.y).abs() <= 1 && b.pos.z == base.z) || b.pos == top.offset(-3, 0, 0))
            })
            .map(|b| b.id)
            .collect();
        for id in gate {
            self.toggle_lock(id);
        }
        // Raise the drawbridge against an army, once everyone's inside or
        // when they're close; not for days on end without one at the gate.
        if o.did("linked") {
            let inside = self.dwarves.iter().filter(|d| d.alive && !d.soldier).all(|d| d.pos.z <= base.z && !self.world.outside(d.pos));
            let army = |near: bool| {
                self.creatures.values().any(|c| {
                    c.alive && c.siege && !c.legendary() && c.hostile() && (!near || ((c.pos.x - top.x).abs() <= 40 && (c.pos.y - top.y).abs() <= 40))
                })
            };
            let (any, at_gate) = (army(false) || coming, army(true));
            // How long the army has been on the map: a little while for those
            // outside to run in, then the bridge goes up regardless.
            if any && o.army_since == 0 {
                o.army_since = self.tick;
            } else if !any {
                o.army_since = 0;
            }
            let waited = any && self.tick > o.army_since + self.bar_patience(o);
            let long = self.tick > o.raised_at + DAY * 4;
            // (Not while the squads are out on a sally: sally.rs. Called
            // back, they're left to hunt down any who got inside.)
            let want = any && (at_gate || inside || waited) && !(long && !at_gate && o.raised_at > 0) && !self.sallying();
            let bridges = self.gate_bridges(base);
            let raised = bridges.iter().any(|id| self.buildings.get(id).map_or(false, |b| b.raised));
            if let Some(lever) = self.gate_lever(base) {
                // Anyone standing by the lever (the squad holds station there) throws it at once.
                let lp = self.buildings[&lever].pos;
                let by = self.dwarves.iter().any(|d| d.alive && d.pos.z == lp.z && (d.pos.x - lp.x).abs() <= 1 && (d.pos.y - lp.y).abs() <= 1);
                if raised != want && by {
                    self.pull_lever(lever);
                } else if raised != want && !self.buildings[&lever].pull {
                    self.request_pull(lever);
                }
                if want && !raised {
                    o.raised_at = self.tick;
                } else if !want && !raised {
                    o.raised_at = 0;
                }
            }
        }
    }

    pub(super) fn gate_lever(&self, base: Pos) -> Option<u32> {
        self.buildings.values().find(|b| b.kind == BuildingKind::Lever && b.built && b.pos == base.offset(3, 1, 0)).map(|b| b.id)
    }

    pub(super) fn gate_bridges(&self, base: Pos) -> Vec<u32> {
        self.buildings.values().filter(|b| b.kind == BuildingKind::Bridge && b.pos.z == base.z && b.pos.x == base.x + 2 && (b.pos.y - base.y).abs() <= 1).map(|b| b.id).collect()
    }

    /// The gate at the foot of the stair, and a corridor of cage traps
    /// behind it.
    pub(super) fn ov_defences(&mut self, o: &mut Overseer) {
        if o.style == Style::Reckless {
            return;
        }
        self.ov_quarry(o);
        let base = o.base.unwrap();
        // The gate: a door on each row of the two-wide corridor. It isn't a
        // gate until both are there, and a door broken down is hung again.
        if self.dug_share(base.offset(1, 0, 0), base.offset(6, 1, 0)) >= 1.0 {
            let mut hung = 0;
            for dy in 0..2 {
                let p = base.offset(1, dy, 0);
                if self.building_at(p) == Some(BuildingKind::Door) || self.place_building(BuildingKind::Door, p) {
                    hung += 1;
                }
            }
            if hung == 2 && !o.did("gate") {
                o.done.push("gate".into());
                Self::note(o, self.tick, "Hung a gate at the foot of the stair, to bar when enemies come.".into());
            }
        }
        // Where the miners broke into a cavern, a door: beasts can't open one.
        // (Never on the gate's own tiles: a door where the bridge or lever
        // goes keeps the drawbridge from ever being built.)
        let gate_part = |p: Pos| p.z == base.z && (1..=3).contains(&(p.x - base.x)) && (0..=1).contains(&(p.y - base.y));
        for q in std::mem::take(&mut self.breaches) {
            let spots = std::iter::once(q).chain(crate::geom::DIRS4.iter().map(|&(dx, dy)| q.offset(dx, dy, 0)));
            let fit: Vec<Pos> = spots.filter(|&p| !gate_part(p) && self.world.get(p).map_or(false, |t| t.dug && t.kind == TileKind::Floor && t.building == 0)).collect();
            let hung = fit.into_iter().any(|p| self.place_building(BuildingKind::Door, p));
            if hung {
                Self::note(o, self.tick, "Hung a door where the miners broke into the caverns.".into());
            }
        }
        // A drawbridge behind the doors, on a lever: raised, nothing gets in.
        // Both halves and the lever, each put back if it's lost.
        let parts = [(BuildingKind::Bridge, base.offset(2, 0, 0)), (BuildingKind::Bridge, base.offset(2, 1, 0)), (BuildingKind::Lever, base.offset(3, 1, 0))];
        let missing = parts.iter().filter(|&&(k, p)| self.building_at(p) != Some(k)).count();
        if o.did("gate") && missing > 0 && self.stock_of(ItemKind::Mechanism) >= missing {
            let placed = parts.iter().filter(|&&(k, p)| self.building_at(p) == Some(k) || self.place_building(k, p)).count();
            if placed == parts.len() && !o.did("bridge") {
                o.done.push("bridge".into());
                Self::note(o, self.tick, "Began a drawbridge at the gate, worked by a lever inside.".into());
            }
        }
        // The lever works every half of the bridge (again, after a rebuild).
        if o.did("bridge") {
            let lever = self.gate_lever(base);
            let bridges = self.gate_bridges(base);
            let built = bridges.len() == 2 && bridges.iter().all(|id| self.buildings.get(id).map_or(false, |b| b.built));
            if let (Some(lever), true) = (lever, built) {
                let unlinked: Vec<u32> = bridges.iter().copied().filter(|id| !self.buildings[&lever].links.contains(id)).collect();
                for id in &unlinked {
                    let _ = self.link_lever(lever, *id);
                }
                if !unlinked.is_empty() && !o.did("linked") {
                    o.done.push("linked".into());
                    Self::note(o, self.tick, "Linked the lever to the drawbridge. The gate can be shut fast.".into());
                }
            }
        }
        // Traps once the bridge is begun: it needs the mechanisms more.
        if o.did("bridge") && !o.did("traps") && self.stock_of(ItemKind::Mechanism) >= 2 && self.stock_of(ItemKind::Cage) >= 2 {
            let mut n = 0;
            for dx in 4..8 {
                for dy in 0..2 {
                    if self.place_building(BuildingKind::CageTrap, base.offset(dx, dy, 0)) {
                        n += 1;
                    }
                }
            }
            if n > 0 {
                o.done.push("traps".into());
                Self::note(o, self.tick, format!("Set {} cage traps in the entrance corridor.", n));
            }
        }
    }

    /// Whether the gate still wants mechanisms and stone is short: it holds
    /// the stone for them.
    pub(super) fn gate_wants_stone(&self, o: &Overseer) -> bool {
        if o.style == Style::Reckless || o.base.is_none() {
            return false;
        }
        let want = if o.did("linked") { self.gate_parts_missing(o) } else { 3 };
        let mechs = self.stock_of(ItemKind::Mechanism);
        let loose = self.items.values().filter(|it| it.kind == ItemKind::Boulder && it.mat.is_building_stone() && it.free()).count();
        mechs < want && loose < 12
    }

    /// Parts of the gate's drawbridge and lever not standing built (a half
    /// lost to a siege, say), each needing a mechanism to put back.
    pub(super) fn gate_parts_missing(&self, o: &Overseer) -> usize {
        let Some(base) = o.base else { return 0 };
        [(BuildingKind::Bridge, base.offset(2, 0, 0)), (BuildingKind::Bridge, base.offset(2, 1, 0)), (BuildingKind::Lever, base.offset(3, 1, 0))]
            .iter()
            .filter(|&&(k, p)| !self.buildings.values().any(|b| b.kind == k && b.pos == p && b.built))
            .count()
    }

    /// Whether `p` is in the gatehouse's walls (two deep on each side of the
    /// corridor, and round the foot of the stair), which nothing may be cut
    /// from: it would open a way round the doors and the bridge.
    pub(super) fn gatehouse(base: Pos, p: Pos) -> bool {
        p.z == base.z && (-2..=16).contains(&(p.x - base.x)) && (-2..=3).contains(&(p.y - base.y))
    }
}
