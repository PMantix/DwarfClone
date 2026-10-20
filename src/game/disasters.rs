//! Disasters: halls dug too wide without a pillar bring the ceiling down.

use super::*;

/// A freshly dug tile this far from every wall, pillar or undug rock brings
/// the ceiling down; one less and the ceiling creaks in warning.
pub const COLLAPSE_SPAN: i32 = 5;

impl Game {
    /// Every tile within `r` of `q` on its level has been dug out: nothing
    /// holds the ceiling up. Natural caverns don't count; they've stood for
    /// ages.
    fn unsupported(&self, q: Pos, r: i32) -> bool {
        for dy in -r..=r {
            for dx in -r..=r {
                match self.world.get(q.offset(dx, dy, 0)) {
                    Some(t) if t.dug && !t.kind.solid() => {}
                    _ => return false,
                }
            }
        }
        true
    }

    /// After digging out `q`: warn of, or bring down, an unsupported ceiling.
    pub(super) fn check_ceiling(&mut self, q: Pos) {
        if self.world.outside(q) || !self.unsupported(q, COLLAPSE_SPAN - 1) {
            return;
        }
        if self.unsupported(q, COLLAPSE_SPAN) {
            self.cave_in(q);
        } else {
            self.warn(
                "creak",
                "The ceiling creaks over a wide hall. Leave a pillar of rock or build a wall before you dig further.",
                (240, 180, 90),
            );
        }
    }

    /// The rock overhead comes down around `q`: rubble, and anyone beneath
    /// is crushed.
    pub(super) fn cave_in(&mut self, q: Pos) {
        let tick = self.tick;
        let mut fell = 0;
        for dy in -2..=2 {
            for dx in -2..=2 {
                let p = q.offset(dx, dy, 0);
                let up = p.up();
                let Some(t) = self.world.get(up).copied() else { continue };
                if t.kind != TileKind::Wall || t.built_from.is_some() || t.mat == Mat::Bedrock {
                    continue;
                }
                fell += 1;
                let tt = self.world.t_mut(up);
                tt.kind = TileKind::Open;
                tt.revealed = true;
                if self.rng.chance(0.6) && t.mat.is_building_stone() {
                    self.spawn_item(ItemKind::Boulder, t.mat, p);
                }
                for i in 0..self.dwarves.len() {
                    if self.dwarves[i].alive && self.dwarves[i].pos == p {
                        let part = self.dwarves[i].body.pick_part(&mut self.rng);
                        let d = &mut self.dwarves[i];
                        d.body.wound(part, 7, false);
                        d.think("was caught in a cave-in", 30, tick);
                    }
                }
                let hit: Vec<u32> = self.creatures.values().filter(|c| c.alive && c.pos == p).map(|c| c.id).collect();
                for id in hit {
                    let part = self.creatures[&id].body.pick_part(&mut self.rng);
                    self.creatures.get_mut(&id).unwrap().body.wound(part, 7, false);
                }
            }
        }
        if fell == 0 {
            return;
        }
        self.dig_version += 1;
        self.water_cells = None;
        self.year_log.cave_in = true;
        let tick_msg = "Cave-in! The ceiling of a wide hall has come down. Halls need a pillar every ten tiles.";
        self.msg_at(tick_msg, (255, 120, 60), Some(q), super::logkind::LogKind::Other);
        let fort = self.fortress_name.clone();
        self.legend(format!("The ceiling of a great hall in {} fell in.", fort));
        self.spotlight("Cave-in!", tick_msg, Some(q), (255, 120, 60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Carve an open, dug-out square of `r` around `c`, leaving the rock above.
    fn hall(g: &mut Game, c: Pos, r: i32) {
        for dy in -r..=r {
            for dx in -r..=r {
                let p = c.offset(dx, dy, 0);
                let t = g.world.t_mut(p);
                t.kind = TileKind::Floor;
                t.dug = true;
                t.water = 0;
                let up = g.world.t_mut(p.up());
                up.kind = TileKind::Wall;
                up.mat = Mat::Granite;
                up.built_from = None;
            }
        }
    }

    #[test]
    fn a_hall_dug_too_wide_caves_in_and_pillars_hold_it_up() {
        let mut g = Game::new(81);
        let e = g.embark;
        let c = Pos::new(e.x, e.y, g.world.surface_z(e.x, e.y) - 8);
        // Nine wide: it creaks but holds.
        hall(&mut g, c, COLLAPSE_SPAN - 1);
        g.check_ceiling(c);
        assert!(g.log.iter().any(|l| l.text.contains("creaks")));
        assert_eq!(g.world.t(c.up()).kind, TileKind::Wall);
        // A pillar in the middle of an eleven-wide hall holds it.
        hall(&mut g, c, COLLAPSE_SPAN);
        g.world.t_mut(c.offset(1, 0, 0)).kind = TileKind::Wall;
        g.check_ceiling(c);
        assert_eq!(g.world.t(c.up()).kind, TileKind::Wall, "the pillar should hold it");
        // Without it, down it comes on whoever's underneath.
        g.world.t_mut(c.offset(1, 0, 0)).kind = TileKind::Floor;
        g.dwarves[0].pos = c;
        let before = g.dwarves[0].body.clone();
        g.check_ceiling(c);
        assert_eq!(g.world.t(c.up()).kind, TileKind::Open, "the ceiling should fall");
        assert!(g.log.iter().any(|l| l.text.starts_with("Cave-in!")));
        assert_ne!(g.dwarves[0].body.hp, before.hp, "the dwarf beneath should be hurt");
        assert!(g.items.values().any(|it| it.kind == ItemKind::Boulder && (it.pos.x - c.x).abs() <= 2));
        // Natural caverns stand however wide they are.
        let c2 = c.offset(0, 0, -3);
        hall(&mut g, c2, COLLAPSE_SPAN);
        for dy in -COLLAPSE_SPAN..=COLLAPSE_SPAN {
            for dx in -COLLAPSE_SPAN..=COLLAPSE_SPAN {
                g.world.t_mut(c2.offset(dx, dy, 0)).dug = false;
            }
        }
        g.check_ceiling(c2);
        assert_eq!(g.world.t(c2.up()).kind, TileKind::Wall);
    }
}
