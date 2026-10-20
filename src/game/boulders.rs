//! Digging out boulders (O10, DIG-34): great stones bedded in rocky ground
//! (`TileKind::Boulder`, placed by terrain.rs) stand in the way like trees.
//! A miner digs one out for a boulder to carry off; short of stone, the
//! overseer breaks out the ones by the wagon first.

use super::*;

impl Game {
    /// Designate up to `max` bedded boulders near the wagon, nearest first;
    /// how many were newly marked.
    pub(super) fn quarry_boulders(&mut self, max: usize) -> usize {
        let e = self.embark;
        let mut near: Vec<(i32, Pos)> = Vec::new();
        for y in (e.y - 20).max(0)..=(e.y + 20).min(self.world.h - 1) {
            for x in (e.x - 20).max(0)..=(e.x + 20).min(self.world.w - 1) {
                let z = self.world.surface_z(x, y);
                for p in [Pos::new(x, y, z), Pos::new(x, y, z + 1)] {
                    if self.world.get(p).map_or(false, |t| t.kind == TileKind::Boulder && t.desig == Desig::None) {
                        near.push(((x - e.x).abs() + (y - e.y).abs(), p));
                    }
                }
            }
        }
        near.sort();
        let mut n = 0;
        for (_, p) in near.into_iter().take(max) {
            n += self.designate(p, p, Desig::Dig).min(1);
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boulders_can_be_quarried() {
        let mut g = Game::new(5);
        let e = g.embark;
        // One bedded beside the wagon.
        let p = (2..8).map(|dx| e.offset(dx, 3, 0)).find(|&p| g.world.walkable(p) && g.world.t(p).building == 0).expect("open ground by the wagon");
        let t = g.world.t_mut(p);
        t.kind = TileKind::Boulder;
        t.mat = Mat::Granite;
        assert!(!g.world.walkable(p), "a boulder is in the way");
        assert_eq!(g.quarry_boulders(6), 1);
        assert_eq!(g.world.t(p).desig, Desig::Dig);
        assert_eq!(g.quarry_boulders(6), 0, "marked once");
        let before = g.items.values().filter(|it| it.kind == ItemKind::Boulder && it.mat == Mat::Granite).count();
        g.complete_dig(p);
        assert_eq!(g.world.t(p).kind, TileKind::Floor);
        assert!(g.world.walkable(p));
        let after = g.items.values().filter(|it| it.kind == ItemKind::Boulder && it.mat == Mat::Granite).count();
        assert_eq!(after, before + 1, "it leaves a boulder to carry off");
    }
}
