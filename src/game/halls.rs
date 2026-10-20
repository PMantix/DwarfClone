//! The halls made grand (O03): as the fort settles, the overseer has the
//! plan's own halls smoothed, stretch by stretch from the stair outward,
//! and later engraved; the atrium and the hub get statues round their
//! hearts and the great monument in the middle; a great hall gets a row of
//! statues down its length. (The rooms off the halls are smoothed by the
//! architect, `architect.rs: ov_finish`.)

use super::architect::taste;
use super::blueprint::Feature;
use super::overseer::Overseer;
use super::*;

/// Hall tiles smoothed (or engraved) as one stretch.
const STRETCH: usize = 12;

impl Game {
    /// The halls of level `k`'s plan, in the order they are cut (out from
    /// the stair: the spine, the atrium or hub, then the cross halls).
    pub(super) fn plan_halls(&self, o: &Overseer, k: i32) -> Vec<Pos> {
        let Some(bp) = Self::plan_of(o, k) else { return Vec::new() };
        let mut seen = HashSet::new();
        bp.segs.iter().flat_map(|s| s.tiles.iter().copied()).filter(|p| seen.insert(*p)).collect()
    }

    /// The statue spots and the monument's place on level `k`'s plan.
    pub(super) fn plan_features(&self, o: &Overseer, k: i32) -> Vec<(Pos, Feature)> {
        Self::plan_of(o, k).map_or(Vec::new(), |bp| bp.features)
    }

    /// Where the great monument should stand: at the heart of the main
    /// level's atrium or hub if its plan has one and there is room, else
    /// somewhere near the wagon.
    pub(super) fn monument_spot(&self, o: &Overseer) -> Option<Pos> {
        let heart = self.plan_features(o, 0).into_iter().find(|f| f.1 == Feature::Monument).map(|f| f.0);
        if let Some(p) = heart.filter(|&p| self.can_place(BuildingKind::Monument, p)) {
            return Some(p);
        }
        self.ov_spot(BuildingKind::Monument, self.embark.offset(0, 6, 0), 16)
    }

    /// The share of the main level's planned halls (those dug) that are
    /// smoothed or engraved, in percent.
    pub fn smoothed_pct(&self) -> u32 {
        let Some(o) = self.overseer.as_ref() else { return 0 };
        // (Floors of stone: soil can't take a polish.)
        let dug: Vec<Pos> = self
            .plan_halls(o, 0)
            .into_iter()
            .filter(|&p| self.world.get(p).map_or(false, |t| t.dug && t.kind == TileKind::Floor && (t.smooth > 0 || t.desig != Desig::None || self.desig_valid(p, Desig::Smooth))))
            .collect();
        if dug.is_empty() {
            return 0;
        }
        (dug.iter().filter(|&&p| self.world.t(p).smooth > 0).count() * 100 / dug.len()) as u32
    }

    /// Daily, from the architect: statues and the monument on the plan's
    /// spots, and the halls smoothed and then engraved, a stretch at a time.
    pub(super) fn ov_halls(&mut self, o: &mut Overseer) {
        let t = taste(o);
        let tier = o.interior.tier;
        if t.plain || tier == 0 || o.base.is_none() || !o.layout.planned() {
            return;
        }
        self.ov_adorn(o);
        let detailing = self.designations.iter().filter(|p| matches!(self.world.t(**p).desig, Desig::Smooth | Desig::Engrave)).count();
        if detailing >= 40 {
            return;
        }
        // Engraved once established, for a grand taste; once mature, for any.
        let carve = tier >= 3 && t.grand >= 1 || tier >= 2 && t.grand >= 2;
        let mut levels: Vec<i32> = o.levels.iter().map(|l| l.level).filter(|&k| k == 0 || tier >= 3).collect();
        levels.sort();
        let floor = |g: &Game, p: Pos, smooth: u8| g.world.get(p).map_or(false, |t| t.dug && t.kind == TileKind::Floor && t.desig == Desig::None && t.smooth == smooth);
        let mut budget = 40 - detailing;
        for k in levels {
            let halls = self.plan_halls(o, k);
            // The next stretch along the halls (as far as they are dug) not
            // yet smoothed; once smooth, engraved.
            while budget >= STRETCH {
                let rough: Vec<Pos> = halls.iter().copied().filter(|&p| floor(self, p, 0) && self.desig_valid(p, Desig::Smooth)).take(STRETCH).collect();
                let (todo, d) = if !rough.is_empty() {
                    (rough, Desig::Smooth)
                } else if carve {
                    (halls.iter().copied().filter(|&p| floor(self, p, 1)).take(STRETCH).collect(), Desig::Engrave)
                } else {
                    (Vec::new(), Desig::Smooth)
                };
                if todo.is_empty() {
                    break;
                }
                budget = budget.saturating_sub(self.detail_stretch(&todo, d).max(STRETCH));
                let (key, what) = if d == Desig::Smooth {
                    ("halls smoothed", "Had the main hall smoothed: the fort can afford to look its best now.")
                } else {
                    ("halls carved", "Set the engravers to the main hall: let its walls tell the fortress's story.")
                };
                if k == 0 && !o.did(key) {
                    o.done.push(key.into());
                    Self::note_ov(o, self.tick, what.into());
                }
            }
        }
    }

    /// Mark a stretch of hall, its floor and the walls along it, for
    /// smoothing or engraving. Returns how many tiles were marked.
    fn detail_stretch(&mut self, stretch: &[Pos], d: Desig) -> usize {
        let mut tiles: Vec<Pos> = Vec::new();
        for &p in stretch {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let q = p.offset(dx, dy, 0);
                    // (Floors in the hall itself; walls beside it.)
                    let ok = q == p || self.world.get(q).map_or(false, |t| t.kind == TileKind::Wall);
                    if ok && !tiles.contains(&q) && self.desig_valid(q, d) && self.world.t(q).desig == Desig::None {
                        tiles.push(q);
                    }
                }
            }
        }
        for &q in &tiles {
            self.designate(q, q, d);
        }
        tiles.len()
    }

    /// Statues on the plan's spots as stock allows (round the atrium's or
    /// hub's heart once settled, down the great hall once established),
    /// and, once established, the great monument at the heart itself.
    fn ov_adorn(&mut self, o: &mut Overseer) {
        let tier = o.interior.tier;
        let t = taste(o);
        let mut levels: Vec<i32> = o.levels.iter().map(|l| l.level).collect();
        levels.sort();
        for k in levels {
            for (p, f) in self.plan_features(o, k) {
                let floor = |g: &Game, q: Pos| g.world.get(q).map_or(false, |t| t.dug && t.kind == TileKind::Floor && t.building == 0);
                match f {
                    Feature::Statue | Feature::HallStatue => {
                        if f == Feature::HallStatue && tier < 2 || !floor(self, p) || self.spare(BuildingKind::Statue) == 0 {
                            continue;
                        }
                        // (Not in a doorway: a door or a room's way in beside it.)
                        if DIRS4.iter().any(|&(dx, dy)| self.bld_kind_at(p.offset(dx, dy, 0)) == Some(BuildingKind::Door)) {
                            continue;
                        }
                        self.place_building(BuildingKind::Statue, p);
                    }
                    Feature::Monument => {
                        if k != 0 || tier < 2 || t.grand == 0 || o.did("monument") || !self.can_place(BuildingKind::Monument, p) {
                            continue;
                        }
                        if !box_iter(p, p.offset(2, 2, 0)).all(|q| floor(self, q)) {
                            continue;
                        }
                        if self.place_building(BuildingKind::Monument, p) {
                            o.done.push("monument".into());
                            let what = if self.plan_at(p.z).map_or(false, |l| l.layout == super::layout::Layout::Spokes) { "the hub" } else { "the atrium" };
                            Self::note_ov(o, self.tick, format!("Began a great monument at the heart of {}.", what));
                        }
                    }
                }
            }
        }
    }

    /// For the batch: how smooth the main halls are at the end.
    pub(super) fn hall_stats(&mut self) {
        if self.overseer.is_some() {
            let pct = self.smoothed_pct();
            self.count("m2.smoothed_pct", pct);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::layout::Layout;
    use crate::game::overseer::{fort_where, trial_game, RoomUse, Style};

    fn peace(g: &mut Game, days: u64) {
        for _ in 0..days * DAY {
            g.step();
            g.creatures.retain(|_, c| c.tame || !c.hostile());
            g.incoming = None;
        }
    }

    fn set_tier(g: &mut Game, tier: u8) {
        let o = g.overseer.as_mut().unwrap();
        o.interior.tier = tier;
        o.interior.tier_since = g.tick;
    }

    fn engraved(g: &Game) -> usize {
        let o = g.overseer.as_ref().unwrap();
        g.plan_halls(o, 0).iter().filter(|&&p| (-1..=1).any(|dy| (-1..=1).any(|dx| g.world.get(p.offset(dx, dy, 0)).map_or(false, |t| t.smooth >= ENGRAVED)))).count()
    }

    /// A settled fort has its main halls smoothed within two months
    /// (its one or two engravers busy), stretch by stretch, and
    /// once it has come of age, engraved.
    #[test]
    fn a_rich_fort_smooths_then_engraves_its_halls() {
        let mut g = trial_game(5003, Style::Steady, 45);
        assert!(g.overseer.as_ref().unwrap().layout.planned());
        let before = g.smoothed_pct();
        set_tier(&mut g, 1);
        peace(&mut g, 60);
        let pct = g.smoothed_pct();
        assert!(pct >= 60, "only {}% of the main halls smoothed (from {}%)", pct, before);
        let o = g.overseer.as_ref().unwrap();
        assert!(o.notes.iter().any(|n| n.1.contains("main hall smoothed")), "no note");
        assert_eq!(engraved(&g), 0, "engraved before it came of age");
        set_tier(&mut g, 3);
        peace(&mut g, 20);
        assert!(engraved(&g) >= 10, "only {} hall tiles engraved", engraved(&g));
    }

    /// The atrium (or hub) gets statues round its heart once the fort is
    /// settled, and the great monument in the middle once it's established.
    #[test]
    fn statues_stand_in_the_atrium() {
        let heart = |g: &Game| {
            let Some(o) = g.overseer.as_ref() else { return false };
            let Some(lp) = o.levels.iter().find(|l| l.level == 0) else { return false };
            matches!(lp.layout, Layout::Radial | Layout::Spokes)
                && g.plan_features(o, 0).iter().filter(|f| f.1 == Feature::Statue).any(|f| g.world.get(f.0).map_or(false, |t| t.dug && t.kind == TileKind::Floor))
        };
        let (found, passed) = fort_where(Style::Steady, 5, 12, 45, heart);
        let Some((seed, mut g)) = found else { panic!("no fort with a dug atrium or hub by day 45 (passed over {:?})", passed) };
        set_tier(&mut g, 2);
        let at = g.embark;
        for _ in 0..6 {
            g.spawn_item(ItemKind::Statue, crate::material::Mat::Granite, at);
        }
        for _ in 0..40 {
            g.spawn_item(ItemKind::Block, crate::material::Mat::Granite, at);
        }
        peace(&mut g, 10);
        let o = g.overseer.as_ref().unwrap();
        let spots: Vec<(Pos, Feature)> = g.plan_features(o, 0);
        let on = |k: BuildingKind, f: Feature| spots.iter().filter(|s| s.1 == f).filter(|s| g.buildings.values().any(|b| b.kind == k && b.pos == s.0)).count();
        let statues = spots.iter().filter(|s| s.1 == Feature::Statue).filter(|s| g.buildings.values().any(|b| b.kind == BuildingKind::Statue && b.built && b.pos == s.0)).count();
        assert!(statues >= 1, "{}: no statue set up round the heart ({} placed)", seed, on(BuildingKind::Statue, Feature::Statue));
        // The monument at the heart, if the heart is dug out and clear (an
        // early fort may have set up a workshop there).
        let (mp, _) = *spots.iter().find(|s| s.1 == Feature::Monument).unwrap();
        let heart = box_iter(mp, mp.offset(2, 2, 0)).collect::<Vec<_>>();
        if heart.iter().all(|&p| g.world.t(p).dug) && super::super::architect::taste(o).grand > 0 {
            // (Cleared, as the rough early work is once established.)
            let in_way: Vec<u32> = heart.iter().map(|&p| g.world.t(p).building).filter(|&id| id != 0 && g.buildings.get(&id).map_or(false, |b| b.kind != BuildingKind::Monument)).collect();
            for id in in_way {
                if g.buildings.contains_key(&id) {
                    g.remove_building(id);
                }
            }
            peace(&mut g, 1);
            let o = g.overseer.as_ref().unwrap();
            let on = |k: BuildingKind, f: Feature| spots.iter().filter(|s| s.1 == f).filter(|s| g.buildings.values().any(|b| b.kind == k && b.pos == s.0)).count();
            assert_eq!(on(BuildingKind::Monument, Feature::Monument), 1, "{}: no monument at the heart", seed);
            assert!(o.notes.iter().any(|n| n.1.contains("great monument at the heart")));
        }
    }

    /// Statues and figurines show things as well as deeds (CRF-21): the
    /// fortress's beasts, its foes, its works.
    #[test]
    fn statues_show_things() {
        let g = trial_game(5003, Style::Steady, 20);
        let things = g.things();
        assert!(things.iter().any(|t| t.starts_with("a ") || t.starts_with("an ")), "{:?}", things);
        let mut g = g;
        g.knobs.set.insert("culture.thing_tells".into(), 1.0);
        g.knobs.set.insert("culture.craft_tells".into(), 1.0);
        let d = g.dwarves.iter().find(|d| d.alive).unwrap().clone();
        let id = g.spawn_item(ItemKind::Craft, crate::material::Mat::Granite, d.pos);
        g.depict_made(&d, id);
        let lines = g.depiction_lines(id);
        assert!(lines.first().map_or(false, |l| things.iter().any(|t| l.contains(t.as_str()))), "{:?}", lines);
    }

    /// Done when (O03): a fort a year on has smoothed main halls, bedrooms
    /// with a cabinet and a chest, and named halls.
    #[test]
    fn a_lived_in_fort() {
        let mut g = Game::new(5);
        g.set_overseer(Some(Style::Builder));
        let at = g.embark;
        for _ in 0..40 {
            g.spawn_item(ItemKind::Gem, crate::material::Mat::Amethyst, at);
        }
        while g.tick < YEAR && g.alive_count() > 0 {
            g.step();
        }
        let o = g.overseer.as_ref().unwrap();
        let pct = g.smoothed_pct();
        assert!(pct >= 50, "only {}% of the main halls smoothed (tier {})", pct, o.interior.tier);
        let furnished = o
            .rooms
            .iter()
            .filter(|r| r.kind == RoomUse::Bedrooms && r.stage == 1)
            .flat_map(super::super::architect::cells_of)
            .filter(|c| c.tiles.iter().any(|p| g.bld_kind_at(*p) == Some(BuildingKind::Cabinet)) && c.tiles.iter().any(|p| g.bld_kind_at(*p) == Some(BuildingKind::Chest)))
            .count();
        assert!(furnished >= 1, "no bedroom with a cabinet and a chest");
        assert!(o.interior.places.iter().any(|p| p.notable) || !g.stories.places.is_empty(), "no named hall");
    }
}
