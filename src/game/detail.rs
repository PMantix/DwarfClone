//! Stone detailing: smoothing rough rock and engraving it with scenes from
//! the world's history. Both make rooms worth more.

use super::*;

/// `Tile::smooth` at or above this is engraved, and the excess is its quality.
pub const ENGRAVED: u8 = 2;

/// The high bit of `Tile::engraving`: the engraving is set with cut gems
/// (worth twice as much). The rest is the legend it shows.
pub const GEM_SET: u32 = 1 << 31;

/// How far an engraver will look for a cut gem to set into masterful work.
const GEM_REACH: i32 = 20;

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
        // Nothing reachable last time and nothing designated since: don't
        // search the whole fortress again just yet.
        if let Some((v, t)) = self.dig_fail[2] {
            if v == self.dig_version && self.tick < t + 600 {
                return false;
            }
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
        let (Some((stand, path)), Some(target)) = (found, target) else {
            self.dig_fail[2] = Some((self.dig_version, self.tick));
            return false;
        };
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
            let gem = if q >= 4 { self.gem_for_engraving(p) } else { None };
            let t = self.world.t_mut(p);
            t.smooth = ENGRAVED + q;
            t.engraving = scene | if gem.is_some() { GEM_SET } else { 0 };
            d.gain_xp(Labor::Engraving, 30);
            if q >= 4 {
                let set = gem.map_or(String::new(), |g| format!(", set with {}s", g.name()));
                if let Some(e) = scene.checked_sub(1).and_then(|i| self.overworld.events.get(i as usize)) {
                    let text = e.text.clone();
                    self.msg(&format!("{} has engraved a masterful scene{}: {}", d.name, set, text), (220, 200, 255));
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
        // Only what the fortress still remembers (tales.rs).
        self.known_scene(i as u32 + 1)
    }

    /// What's carved at `p`, for tile info: (quality, year, text).
    pub fn engraving_at(&self, p: Pos) -> Option<(u8, i32, String)> {
        let t = self.world.get(p)?;
        if t.smooth < ENGRAVED {
            return None;
        }
        let e = (t.engraving & !GEM_SET).checked_sub(1).and_then(|i| self.overworld.events.get(i as usize));
        let mut text = e.map_or("an abstract pattern".into(), |e| e.text.clone());
        if t.engraving & GEM_SET != 0 {
            text.push_str(" Its figures are set with cut gems.");
        }
        Some((t.smooth - ENGRAVED, e.map_or(0, |e| e.year), text))
    }

    /// A rich fort's masterful engraving takes a cut gem lying near (the
    /// overseer's at wealth tier 3; a player's fort keeps its gems).
    fn gem_for_engraving(&mut self, p: Pos) -> Option<Mat> {
        if !self.overseer.as_ref().map_or(false, |o| o.interior.tier >= 3) {
            return None;
        }
        let near = |q: Pos| (q.z - p.z).abs() <= 1 && (q.x - p.x).abs().max((q.y - p.y).abs()) <= GEM_REACH;
        let id = self.items.values().filter(|it| it.kind == ItemKind::Gem && it.free() && !it.to_depot && near(it.pos)).map(|it| it.id).min()?;
        self.items.remove(&id).map(|it| it.mat)
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

    #[test]
    fn built_walls_can_be_engraved() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let e = g.embark;
        let mut spots = (3..20).map(|r| e.offset(r, 0, 0)).filter(|&p| g.world.walkable(p) && g.world.walkable(p.offset(-1, 0, 0)) && g.world.t(p).building == 0);
        let (wall, wood) = (spots.next().unwrap(), spots.next().unwrap());
        for (p, mat, from) in [(wall, Mat::Granite, ItemKind::Block), (wood, Mat::Wood, ItemKind::Log)] {
            let w = g.world.t_mut(p);
            w.kind = TileKind::Wall;
            w.mat = mat;
            w.revealed = true;
            w.built_from = Some(from);
        }
        assert!(g.desig_valid(wall, Desig::Smooth), "a wall of stone blocks can be smoothed");
        assert!(!g.desig_valid(wood, Desig::Smooth), "a wooden wall can't");
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
        assert_eq!(g.designate(wall, wall, Desig::Engrave), 1);
        for _ in 0..DAY {
            g.step();
            if g.world.t(wall).smooth >= ENGRAVED {
                break;
            }
        }
        assert!(g.engraving_at(wall).is_some(), "the built wall was never engraved");
        // A rich overseer's fort sets a gem lying near into masterful work.
        let plain = g.world.detail_value(wall);
        let gem = g.spawn_item(ItemKind::Gem, Mat::Amethyst, wall.offset(-1, 0, 0));
        assert_eq!(g.gem_for_engraving(wall), None, "a player's fort keeps its gems");
        let mut o = Overseer::new(Style::Steady);
        o.interior.tier = 3;
        g.overseer = Some(o);
        assert_eq!(g.gem_for_engraving(wall), Some(Mat::Amethyst));
        assert!(!g.items.contains_key(&gem), "the gem went into the wall");
        g.world.t_mut(wall).engraving |= GEM_SET;
        assert_eq!(g.world.detail_value(wall), plain * 2);
        let (_, _, text) = g.engraving_at(wall).unwrap();
        assert!(text.ends_with("set with cut gems."), "{}", text);
        assert!(!text.starts_with("an abstract") || g.overworld.events.is_empty());
    }
}
