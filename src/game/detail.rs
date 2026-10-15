//! Stone detailing: smoothing rough rock and engraving it with scenes from
//! the world's history. Both make rooms worth more.

use super::*;

/// `Tile::smooth` at or above this is engraved, and the excess is its quality.
pub const ENGRAVED: u8 = 2;

impl Game {
    /// Engravers (no pick needed) look for smoothing and engraving work.
    pub(super) fn find_detail(&mut self, d: &mut Dwarf) -> bool {
        let open: HashSet<Pos> = self
            .designations
            .iter()
            .copied()
            .filter(|p| !self.claimed.contains(p) && matches!(self.world.t(*p).desig, Desig::Smooth | Desig::Engrave))
            .collect();
        if open.is_empty() {
            return false;
        }
        let mut target = None;
        let found = path::search(&self.world, d.pos, SEARCH_LIMIT, Mover::Dwarf, |p| {
            for q in std::iter::once(p).chain(DIRS8.iter().map(|&(dx, dy)| p.offset(dx, dy, 0))) {
                if open.contains(&q) && self.desig_valid(q, self.world.t(q).desig) {
                    target = Some(q);
                    return true;
                }
            }
            false
        });
        let (Some((stand, path)), Some(target)) = (found, target) else { return false };
        self.claimed.insert(target);
        d.task = Task::Dig { target, stand };
        Self::set_path(d, path);
        true
    }

    /// Smooth the stone, or carve a scene from history into it.
    pub(super) fn finish_detail(&mut self, d: &mut Dwarf, p: Pos) {
        let desig = self.world.t(p).desig;
        let skill = d.skill(Labor::Engraving);
        if desig == Desig::Engrave {
            let q = self.roll_quality(skill * d.focus().max(0) as u32 / 100);
            let scene = self.pick_scene();
            let t = self.world.t_mut(p);
            t.smooth = ENGRAVED + q;
            t.engraving = scene;
            d.gain_xp(Labor::Engraving, 30);
            if q >= 4 {
                if let Some(e) = scene.checked_sub(1).and_then(|i| self.overworld.events.get(i as usize)) {
                    let text = e.text.clone();
                    self.msg(&format!("{} has engraved a masterful scene: {}", d.name, text), (220, 200, 255));
                }
            }
        } else {
            self.world.t_mut(p).smooth = self.world.t(p).smooth.max(1);
            d.gain_xp(Labor::Engraving, 15);
        }
        let t = self.world.t_mut(p);
        t.desig = Desig::None;
        self.designations.remove(&p);
        self.claimed.remove(&p);
    }

    /// An event from the world's history (1-based index into the legends,
    /// 0 if there are none), favouring the fortress's own recent past.
    pub(super) fn pick_scene(&mut self) -> u32 {
        let n = self.overworld.events.len();
        if n == 0 {
            return 0;
        }
        let i = if self.rng.chance(0.4) { n - 1 - self.rng.range(0, n.min(30) as i32) as usize } else { self.rng.range(0, n as i32) as usize };
        i as u32 + 1
    }

    /// What's carved at `p`, for tile info: (quality, year, text).
    pub fn engraving_at(&self, p: Pos) -> Option<(u8, i32, String)> {
        let t = self.world.get(p)?;
        if t.smooth < ENGRAVED {
            return None;
        }
        let e = t.engraving.checked_sub(1).and_then(|i| self.overworld.events.get(i as usize));
        Some((t.smooth - ENGRAVED, e.map_or(0, |e| e.year), e.map_or("an abstract pattern".into(), |e| e.text.clone())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engravers_smooth_then_engrave_and_rooms_gain_value() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        // A granite wall raised beside the wagon, with open ground next to it.
        let e = g.embark;
        let wall = (3..15)
            .map(|r| e.offset(r, 0, 0))
            .find(|&p| g.world.walkable(p) && g.world.walkable(p.offset(-1, 0, 0)) && g.world.t(p).building == 0)
            .unwrap();
        let stand = wall.offset(-1, 0, 0);
        {
            let w = g.world.t_mut(wall);
            w.kind = TileKind::Wall;
            w.mat = Mat::Granite;
            w.revealed = true;
        }
        assert!(g.desig_valid(wall, Desig::Smooth));
        assert!(!g.desig_valid(wall, Desig::Engrave), "rough stone can't be engraved");
        for d in g.dwarves.iter_mut() {
            d.labors[crate::dwarf::Labor::Engraving.idx()] = true;
        }
        assert_eq!(g.designate(wall, wall, Desig::Smooth), 1);
        for _ in 0..DAY {
            g.step();
            if g.world.t(wall).smooth > 0 {
                break;
            }
        }
        assert_eq!(g.world.t(wall).smooth, 1, "never smoothed");
        assert_eq!(g.designate(wall, wall, Desig::Engrave), 1);
        for _ in 0..DAY {
            g.step();
            if g.world.t(wall).smooth >= ENGRAVED {
                break;
            }
        }
        let (_, _, text) = g.engraving_at(wall).expect("never engraved");
        assert!(!text.is_empty());
        assert!(g.world.detail_value(wall) > g.world.detail_value(stand), "an engraved wall should add value");
    }
}
