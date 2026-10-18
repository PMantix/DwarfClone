//! Personal needs: socializing, drinking, crafting, fighting, learning,
//! praying and making merry. Unmet needs distract a dwarf, which costs focus.

use super::*;
use crate::dwarf::{random_needs, NeedKind, NEED_BADLY, NEED_MIN, NEED_UNMET};

/// A dwarf in one of these zones while at leisure is meeting the need.
fn zone_need(kind: ZoneKind) -> Option<NeedKind> {
    match kind {
        ZoneKind::Temple => Some(NeedKind::Pray),
        ZoneKind::Tavern => Some(NeedKind::Party),
        ZoneKind::Laboratory => Some(NeedKind::Learn),
        _ => None,
    }
}

impl Game {
    /// Every 50 ticks per dwarf: needs fade, and leisure in a temple or tavern
    /// meets them. Unmet needs mostly cost focus; going badly unmet, or being
    /// met again at last, leaves a thought.
    pub(super) fn needs_tick(&mut self, d: &mut Dwarf) {
        if d.insane {
            return;
        }
        if d.needs.is_empty() {
            // Older saves: give everyone needs the first time round.
            d.needs = random_needs(&d.personality, &mut self.rng);
        }
        self.fill_family(d);
        let leisure = !d.asleep
            && matches!(d.task, Task::Idle | Task::Wander { .. } | Task::Eat { .. } | Task::Dine { .. } | Task::DrinkItem { .. });
        if leisure {
            if let Some(k) = self.zone_kind_at(d.pos).and_then(zone_need) {
                d.satisfy(k, 120);
            }
        }
        let tick = self.tick;
        let mut thoughts: Vec<(String, i32)> = Vec::new();
        for n in d.needs.iter_mut() {
            n.level = (n.level - 2 * n.strength as i32).max(NEED_MIN);
            let s = n.strength as i32;
            if !n.low && n.level <= NEED_BADLY {
                n.low = true;
                thoughts.push((format!("is distracted by being unable to {}", n.kind.verb()), 2 * s));
            } else if n.low && n.level > NEED_UNMET {
                n.low = false;
                thoughts.push((format!("was able to {} at last", n.kind.verb()), -4 * s));
            }
        }
        for (t, s) in thoughts {
            d.think(&t, s, tick);
        }
    }

    /// The leisure zone a dwarf most wants to visit, if its need is below
    /// `below` and such a zone exists.
    fn wanted_zone(&self, d: &Dwarf, below: i32) -> Option<ZoneKind> {
        [ZoneKind::Temple, ZoneKind::Tavern, ZoneKind::Laboratory]
            .into_iter()
            .filter_map(|z| {
                let n = d.need(zone_need(z)?)?;
                (n.level < below).then_some((n.level, z))
            })
            .filter(|(_, z)| self.zones.values().any(|q| q.kind == *z))
            .min_by_key(|(l, _)| *l)
            .map(|(_, z)| z)
    }

    /// A dwarf whose need to pray or carouse has gone unmet takes time off
    /// work to see to it. True while on such a break.
    pub(super) fn on_break(&mut self, d: &mut Dwarf) -> bool {
        if self.tick < d.break_until {
            return true;
        }
        if (d.soldier && self.alert) || d.tantrum > 0 || d.mood.is_some() || self.burrow_mask(d) != 0 {
            return false;
        }
        if self.wanted_zone(d, NEED_UNMET).is_none() {
            return false;
        }
        d.break_until = self.tick + DAY / 6;
        true
    }

    /// Head for (or stay in) a temple or tavern if a need calls for it.
    pub(super) fn seek_need_zone(&mut self, d: &mut Dwarf) -> bool {
        let Some(kind) = self.wanted_zone(d, 150) else {
            d.break_until = 0;
            return false;
        };
        let here = self.zone_kind_at(d.pos) == Some(kind);
        if here && !self.rng.chance(0.15) {
            return true;
        }
        let tiles: Vec<Pos> = self
            .zones
            .values()
            .filter(|z| z.kind == kind)
            .flat_map(|z| z.tiles.iter().copied())
            .filter(|&p| self.world.walkable(p))
            .collect();
        if tiles.is_empty() {
            return false;
        }
        let q = tiles[self.rng.range(0, tiles.len() as i32) as usize];
        if q != d.pos {
            d.task = Task::Wander { target: q };
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dwarf::Need;

    fn only_need(d: &mut Dwarf, kind: NeedKind) {
        d.needs = vec![Need { kind, strength: 3, level: 200, low: false }];
    }

    fn place_near(g: &mut Game, kind: BuildingKind) -> u32 {
        let e = g.embark;
        for r in 2..25 {
            for dy in -r..=r {
                for dx in -r..=r {
                    let p = e.offset(dx, dy, 0);
                    if g.can_place(kind, p) && g.place_building(kind, p) {
                        let id = g.world.t(p).building;
                        g.buildings.get_mut(&id).unwrap().built = true;
                        return id;
                    }
                }
            }
        }
        panic!("no room for {:?}", kind);
    }

    #[test]
    fn a_dwarf_kept_from_crafting_loses_focus_until_they_craft() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        for d in g.dwarves.iter_mut() {
            d.labors[Labor::Crafting.idx()] = false;
        }
        only_need(&mut g.dwarves[0], NeedKind::Craft);
        g.dwarves[0].labors[Labor::Crafting.idx()] = true;
        let id = g.dwarves[0].id;
        let focus = |g: &Game| g.dwarves.iter().find(|d| d.id == id).unwrap().focus();
        let before = focus(&g);
        for _ in 0..10 * DAY {
            g.step();
        }
        let starved = focus(&g);
        assert!(starved < before - 20, "focus only fell from {} to {}", before, starved);
        assert!(g.dwarves[0].needs[0].low, "the craft need never went badly unmet");

        let shop = place_near(&mut g, BuildingKind::Craftsdwarf);
        let e = g.embark;
        for _ in 0..4 {
            g.spawn_item(ItemKind::Boulder, Mat::Granite, e);
            g.add_order(shop, Recipe::StoneCraft);
        }
        for _ in 0..3 * DAY {
            g.step();
        }
        let after = focus(&g);
        assert!(after > starved + 20, "crafting only brought focus from {} to {}", starved, after);
    }

    #[test]
    fn focus_changes_work_time() {
        let mut d = Dwarf::default();
        only_need(&mut d, NeedKind::Craft);
        d.needs[0].level = NEED_MIN;
        Game::labor(&mut d, 400, 0);
        let slow = d.work;
        d.working = false;
        d.needs[0].level = 400;
        Game::labor(&mut d, 400, 0);
        assert!(slow > d.work * 3 / 2, "distracted {} vs focused {}", slow, d.work);
    }

    #[test]
    fn a_dwarf_who_needs_to_pray_takes_a_break_at_the_temple() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let e = g.embark;
        assert!(g.add_zone(e.offset(-8, -8, -1), e.offset(-3, -3, 1), ZoneKind::Temple) > 0);
        for d in g.dwarves.iter_mut() {
            only_need(d, NeedKind::Craft);
        }
        only_need(&mut g.dwarves[0], NeedKind::Pray);
        g.dwarves[0].needs[0].level = NEED_BADLY;
        let id = g.dwarves[0].id;
        let mut prayed = false;
        for _ in 0..2 * DAY {
            g.step();
            let d = g.dwarves.iter().find(|d| d.id == id).unwrap();
            prayed |= g.zone_kind_at(d.pos) == Some(ZoneKind::Temple);
        }
        let d = g.dwarves.iter().find(|d| d.id == id).unwrap();
        assert!(prayed, "never went to the temple");
        assert!(d.need(NeedKind::Pray).unwrap().level > NEED_UNMET, "prayer need still {}", d.need(NeedKind::Pray).unwrap().level);
    }
}
