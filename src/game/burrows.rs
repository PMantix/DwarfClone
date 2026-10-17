//! Burrows (areas dwarves are kept to), the civilian alert, and traffic.

use super::{Game, SEARCH_LIMIT};
use crate::dwarf::{Dwarf, Task};
use crate::geom::{box_iter, Pos};
use crate::path;
use crate::world::Mover;
use serde::{Deserialize, Serialize};

/// At most eight burrows: each is one bit in `Tile::burrows`.
pub const MAX_BURROWS: usize = 8;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Burrow {
    /// Bit index in `Tile::burrows`.
    pub bit: u8,
    pub name: String,
    /// Dwarves who must stay inside it.
    pub members: Vec<u32>,
    /// Tiles painted, kept up to date when painting.
    pub tiles: u32,
}

impl Burrow {
    pub fn mask(&self) -> u8 {
        1 << self.bit
    }
}

/// Traffic levels for `Tile::traffic`.
pub const TRAFFIC_NAMES: [&str; 4] = ["Normal", "High", "Low", "Restricted"];

impl Game {
    /// Make a new burrow. Returns its index in `self.burrows`.
    pub fn new_burrow(&mut self) -> Option<usize> {
        let bit = (0..MAX_BURROWS as u8).find(|b| !self.burrows.iter().any(|x| x.bit == *b))?;
        let n = self.burrows.len() + 1;
        self.burrows.push(Burrow { bit, name: format!("Burrow {}", n), members: Vec::new(), tiles: 0 });
        Some(self.burrows.len() - 1)
    }

    pub fn delete_burrow(&mut self, idx: usize) {
        if idx >= self.burrows.len() {
            return;
        }
        let b = self.burrows.remove(idx);
        let m = !b.mask();
        for t in self.world.tiles.iter_mut() {
            t.burrows &= m;
        }
        if self.alert_burrow == Some(b.bit) {
            self.alert_burrow = None;
            self.civ_alert = false;
        }
    }

    /// Add (or with `add == false`, remove) every walkable-ish tile in the box.
    /// Add tiles to a burrow whether or not they're dug yet (the overseer
    /// takes a room into the shelter as it lays it out).
    pub(super) fn burrow_add(&mut self, idx: usize, tiles: &[Pos]) {
        let Some(mask) = self.burrows.get(idx).map(|x| x.mask()) else { return };
        let mut added = 0;
        for &p in tiles {
            if let Some(t) = self.world.get(p) {
                if t.burrows & mask == 0 {
                    self.world.t_mut(p).burrows |= mask;
                    added += 1;
                }
            }
        }
        self.burrows[idx].tiles += added;
    }

    pub fn paint_burrow(&mut self, idx: usize, a: Pos, b: Pos, add: bool) -> usize {
        let Some(mask) = self.burrows.get(idx).map(|x| x.mask()) else { return 0 };
        let mut n = 0;
        for p in box_iter(a, b) {
            let Some(t) = self.world.get(p) else { continue };
            if add && (!t.revealed || t.kind.solid()) {
                continue;
            }
            let t = self.world.t_mut(p);
            let before = t.burrows;
            if add {
                t.burrows |= mask;
            } else {
                t.burrows &= !mask;
            }
            if t.burrows != before {
                n += 1;
            }
        }
        let count = self.world.tiles.iter().filter(|t| t.burrows & mask != 0).count() as u32;
        self.burrows[idx].tiles = count;
        n
    }

    pub fn toggle_burrow_member(&mut self, idx: usize, dwarf: u32) -> bool {
        let Some(b) = self.burrows.get_mut(idx) else { return false };
        if let Some(i) = b.members.iter().position(|&m| m == dwarf) {
            b.members.remove(i);
            false
        } else {
            b.members.push(dwarf);
            true
        }
    }

    pub fn set_alert_burrow(&mut self, idx: usize) {
        self.alert_burrow = self.burrows.get(idx).map(|b| b.bit);
    }

    /// Raise or lower the civilian alert. Needs an alert burrow with tiles in it.
    pub fn toggle_civ_alert(&mut self) -> Result<bool, String> {
        if !self.civ_alert {
            let Some(b) = self.alert_burrow.and_then(|bit| self.burrows.iter().find(|b| b.bit == bit)) else {
                return Err("Choose an alert burrow first (w, then A).".into());
            };
            if b.tiles == 0 {
                return Err(format!("{} has no tiles yet.", b.name));
            }
        }
        self.civ_alert = !self.civ_alert;
        let text = if self.civ_alert { "Civilian alert! Everyone to safety." } else { "The civilian alert is over." };
        self.msg(text, (255, 200, 90));
        Ok(self.civ_alert)
    }

    /// The burrow bits a dwarf is kept to right now (0 = free to roam).
    pub fn burrow_mask(&self, d: &Dwarf) -> u8 {
        if self.civ_alert && !d.soldier {
            if let Some(bit) = self.alert_burrow {
                return 1 << bit;
            }
        }
        self.burrows.iter().filter(|b| b.members.contains(&d.id)).fold(0, |m, b| m | b.mask())
    }

    fn in_mask(&self, p: Pos, mask: u8) -> bool {
        self.world.get(p).map_or(false, |t| t.burrows & mask != 0)
    }

    /// Whether the dwarf is already walking back into its burrow.
    pub(super) fn returning_to_burrow(&self, d: &Dwarf) -> bool {
        let mask = self.burrow_mask(d);
        mask != 0 && matches!(d.task, Task::Wander { target } if self.in_mask(target, mask))
    }

    /// Before a dwarf's turn: confine its pathing to its burrow, or send it back
    /// in if it's outside. Returns the restriction to apply while it acts.
    pub(super) fn burrow_turn(&mut self, d: &mut Dwarf) -> u8 {
        let mask = self.burrow_mask(d);
        if mask == 0 || self.in_mask(d.pos, mask) {
            return mask;
        }
        if !self.returning_to_burrow(d) && d.idle_cd == 0 && !d.asleep && !d.body.unconscious() {
            let goal = |p: Pos| self.in_mask(p, mask);
            match path::search(&self.world, d.pos, SEARCH_LIMIT, Mover::Dwarf, goal) {
                Some((target, path)) => {
                    self.release_task(d);
                    d.task = Task::Wander { target };
                    Self::set_path(d, path);
                }
                // Unreachable: let them be and try again later.
                None => d.idle_cd = 300,
            }
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{ItemKind, DAY};
    use crate::world::{Desig, TileKind};

    #[test]
    fn civilian_alert_brings_everyone_inside() {
        let mut g = Game::new(44);
        let e = g.embark;
        // Send them off to work far and wide.
        g.designate(e.offset(-25, -25, -2), e.offset(25, 25, 2), Desig::Chop);
        for _ in 0..DAY / 2 {
            g.step();
        }
        let idx = g.new_burrow().unwrap();
        let painted = g.paint_burrow(idx, e.offset(-4, -4, -1), e.offset(4, 4, 1), true);
        assert!(painted > 20);
        assert!(g.toggle_civ_alert().is_err(), "alert raised without an alert burrow");
        g.set_alert_burrow(idx);
        // One soldier stays out.
        let soldier = g.dwarves[0].id;
        g.toggle_soldier(soldier);
        assert_eq!(g.toggle_civ_alert(), Ok(true));
        for _ in 0..DAY {
            g.step();
        }
        let mask = g.burrows[idx].mask();
        for d in g.dwarves.iter().filter(|d| d.alive && !d.soldier) {
            assert!(g.world.t(d.pos).burrows & mask != 0, "{} is outside the burrow at {:?} ({:?})", d.name, d.pos, d.task);
        }
        // And they stay in while it lasts.
        for _ in 0..DAY {
            g.step();
            for d in g.dwarves.iter().filter(|d| d.alive && !d.soldier) {
                assert!(g.world.t(d.pos).burrows & mask != 0, "{} left the burrow", d.name);
            }
        }
        assert_eq!(g.toggle_civ_alert(), Ok(false));
        let logs = g.items.values().filter(|it| it.kind == ItemKind::Log).count();
        for _ in 0..DAY {
            g.step();
        }
        assert!(g.items.values().filter(|it| it.kind == ItemKind::Log).count() > logs, "work didn't resume after the alert");
    }

    #[test]
    fn restricted_traffic_is_avoided_when_there_is_another_way() {
        let mut g = Game::new(45);
        // A walled one-tile corridor with a short detour beside its middle.
        let e = g.embark;
        let z = e.z;
        let row: Vec<Pos> = (0..12).map(|i| Pos::new(e.x - 6 + i, e.y, z)).collect();
        for &p in &row {
            for dy in -1..=1 {
                let q = p.offset(0, dy, 0);
                g.world.t_mut(q).kind = if dy == 0 { TileKind::Floor } else { TileKind::Wall };
                g.world.t_mut(q.down()).kind = TileKind::Wall;
            }
        }
        for dx in -1..=1 {
            g.world.t_mut(Pos::new(e.x + dx, e.y + 1, z)).kind = TileKind::Floor;
        }
        let (a, b) = (row[0], row[11]);
        let straight = path::astar(&g.world, a, b, 10_000, Mover::Dwarf).unwrap();
        assert!(straight.iter().any(|p| p.y == e.y && p.x == e.x));
        g.world.t_mut(Pos::new(e.x, e.y, z)).traffic = 3;
        let around = path::astar(&g.world, a, b, 10_000, Mover::Dwarf).unwrap();
        assert!(!around.contains(&Pos::new(e.x, e.y, z)), "walked over a restricted tile with a free way round");
    }
}
