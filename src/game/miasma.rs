//! Rot and miasma: bodies left lying about rot. Under the open sky the smell
//! blows away; underground it gathers as miasma that drifts through the halls
//! and sickens whoever breathes it. In time only bones are left (citizens'
//! bodies wait, stinking, for burial).

use super::*;

/// Days before a body starts to stink.
pub const ROT_DAYS: u16 = 5;
/// Days before the flesh is gone, leaving bones.
pub const GONE_DAYS: u16 = 40;

impl Game {
    fn rots(it: &Item) -> bool {
        (matches!(it.kind, ItemKind::Corpse | ItemKind::Carcass) || it.kind == ItemKind::BodyPart && it.mat == Mat::Flesh) && it.carried_by.is_none() && it.container.is_none()
    }

    /// Daily: bodies rot (an item's `wear` counts its days of rot), and old
    /// ones go to bones.
    pub(super) fn rot_daily(&mut self) {
        let mut gone = Vec::new();
        for it in self.items.values_mut().filter(|it| Self::rots(it)) {
            it.wear = it.wear.saturating_add(1);
            if it.wear >= GONE_DAYS {
                gone.push(it.id);
            }
        }
        for id in gone {
            // A citizen waits for burial, however long it takes.
            if self.items.get(&id).map_or(true, |it| self.burial_corpse(it)) {
                continue;
            }
            let Some(it) = self.items.remove(&id) else { continue };
            let bones = self.spawn_item(ItemKind::Bone, Mat::Bone, it.pos);
            if let Some(b) = self.items.get_mut(&bones) {
                b.label = it.label.map(|l| format!("remains of {}", l));
            }
        }
    }

    /// Every little while: rotting bodies below ground give off miasma,
    /// which drifts, thins and is blown away in the open.
    pub(super) fn miasma_tick(&mut self) {
        let mut cells = match self.miasma_cells.take() {
            Some(c) => c,
            None => self.world.tiles.iter().enumerate().filter(|(_, t)| t.miasma > 0).map(|(i, _)| i).collect(),
        };
        // New stink from what's rotting.
        let sources: Vec<Pos> = self.items.values().filter(|it| Self::rots(it) && it.wear >= ROT_DAYS).map(|it| it.pos).collect();
        for p in sources {
            if self.world.outside(p) || !self.world.in_bounds(p) {
                continue;
            }
            let i = self.world.idx(p);
            let t = &mut self.world.tiles[i];
            if t.miasma == 0 {
                cells.push(i);
            }
            t.miasma = (t.miasma + 2).min(7);
        }
        // Drift and fade.
        let mut next = Vec::with_capacity(cells.len());
        let mut spread: Vec<Pos> = Vec::new();
        for i in cells {
            let m = self.world.tiles[i].miasma;
            if m == 0 {
                continue;
            }
            let p = self.world.pos_of(i);
            if self.world.outside(p) || self.world.tiles[i].water > 3 {
                self.world.tiles[i].miasma = 0;
                continue;
            }
            if m >= 2 && self.rng.chance(0.6) {
                let (dx, dy) = DIRS8[self.rng.range(0, 8) as usize];
                let q = p.offset(dx, dy, 0);
                if self.world.get(q).map_or(false, |t| !t.kind.solid() && t.miasma + 1 < m && t.door != world::DOOR_LOCKED) {
                    self.world.tiles[i].miasma -= 1;
                    spread.push(q);
                }
            }
            if self.rng.chance(0.04) {
                self.world.tiles[i].miasma -= 1;
            }
            if self.world.tiles[i].miasma > 0 {
                next.push(i);
            }
        }
        for q in spread {
            let i = self.world.idx(q);
            if self.world.tiles[i].miasma == 0 {
                next.push(i);
            }
            self.world.tiles[i].miasma += 1;
        }
        next.sort_unstable();
        next.dedup();
        self.miasma_cells = Some(next);
        // Whoever breathes it is sickened (once a day is plenty).
        let tick = self.tick;
        for d in self.dwarves.iter_mut().filter(|d| d.alive && !d.ghost) {
            if self.world.get(d.pos).map_or(false, |t| t.miasma >= 2) && !d.thoughts.iter().any(|t| t.text == "was sickened by miasma" && tick < t.tick + DAY) {
                d.think("was sickened by miasma", 8, tick);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bodies_rot_and_stink_below_ground_and_go_to_bones() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        // A sealed room underground with a carcass in it.
        let e = g.embark;
        let c = Pos::new(e.x, e.y, g.world.surface_z(e.x, e.y) - 6);
        for dy in -3..=3 {
            for dx in -3..=3 {
                let p = c.offset(dx, dy, 0);
                let edge = dx.abs() == 3 || dy.abs() == 3;
                let t = g.world.t_mut(p);
                t.kind = if edge { TileKind::Wall } else { TileKind::Floor };
                t.water = 0;
                t.revealed = true;
                g.world.t_mut(p.up()).kind = TileKind::Wall;
            }
        }
        let body = g.spawn_item(ItemKind::Carcass, Mat::Flesh, c);
        g.dwarves[0].pos = c.offset(1, 0, 0);
        for _ in 0..ROT_DAYS + 1 {
            g.rot_daily();
        }
        for _ in 0..60 {
            g.miasma_tick();
            g.dwarves[0].pos = c.offset(1, 0, 0);
        }
        // (It drifts by chance: give it a while longer to reach the dwarf.)
        for _ in 0..120 {
            if g.dwarves[0].thoughts.iter().any(|t| t.text == "was sickened by miasma") {
                break;
            }
            g.miasma_tick();
            g.dwarves[0].pos = c.offset(1, 0, 0);
        }
        let spread = (-2..=2).flat_map(|dy| (-2..=2).map(move |dx| (dx, dy))).filter(|&(dx, dy)| g.world.t(c.offset(dx, dy, 0)).miasma > 0).count();
        assert!(spread >= 4, "miasma should drift through the room ({} tiles)", spread);
        assert!(g.dwarves[0].thoughts.iter().any(|t| t.text == "was sickened by miasma"));
        // Out in the open, no stink gathers.
        let top = g.flat_patch(3, 3, 10).offset(1, 1, 0);
        let out = g.spawn_item(ItemKind::Carcass, Mat::Flesh, top);
        g.items.get_mut(&out).unwrap().wear = ROT_DAYS + 2;
        for _ in 0..10 {
            g.miasma_tick();
        }
        assert_eq!(g.world.t(g.items[&out].pos).miasma, 0);
        // In time, only bones.
        for _ in 0..GONE_DAYS {
            g.rot_daily();
        }
        assert!(!g.items.contains_key(&body));
        assert!(g.items.values().any(|it| it.kind == ItemKind::Bone && it.pos == c));
    }
}
