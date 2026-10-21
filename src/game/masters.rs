//! Workshop masters and apprentices (O08, ECO-13). A workshop may have a
//! master (`Building.master`) and a least skill to work there
//! (`Building.min_skill`). The master's own work comes out a little better;
//! anyone else working there while the master is near is an apprentice:
//! they learn half as fast again, and their work is lifted a third of the
//! way toward the master's. The overseer names the most skilled dwarf of each workshop's
//! trade its master, weekly.

use super::*;

/// How near the master must be to teach.
const TEACH_RANGE: i32 = 10;

impl Game {
    /// The labor a workshop's work mostly needs.
    pub fn shop_labor(&self, b: &Building) -> Option<Labor> {
        b.orders.first().map(|o| o.recipe.labor()).or_else(|| b.kind.recipes().first().map(|r| r.labor()))
    }

    /// Whether `d` may take the job at the front of `b`'s queue: skilled
    /// enough, or nobody who is could (a workshop never stalls for want of
    /// a master's hand).
    pub(super) fn skilled_enough(&self, b: &Building, d: &Dwarf, labor: Labor) -> bool {
        if b.min_skill == 0 || d.skill(labor) >= b.min_skill {
            return true;
        }
        !self.dwarves.iter().any(|o| {
            o.alive && o.id != d.id && !o.is_child() && o.has(labor) && o.skill(labor) >= b.min_skill && (b.workers.is_empty() || b.workers.contains(&o.id))
        })
    }

    /// The workshop's master, alive and within reach of `at`.
    fn master_near(&self, bld: u32, worker: u32, at: Pos) -> Option<&Dwarf> {
        let m = self.buildings.get(&bld)?.master.filter(|&m| m != worker)?;
        self.dwarves.iter().find(|d| d.id == m && d.alive && (d.pos.x - at.x).abs().max((d.pos.y - at.y).abs()) <= TEACH_RANGE && (d.pos.z - at.z).abs() <= 1)
    }

    /// Experience for a job at `bld`: half as much again beside its master.
    pub(super) fn apprentice_xp(&self, d: &Dwarf, bld: u32, xp: u32) -> u32 {
        match self.master_near(bld, d.id, d.pos) {
            Some(_) => xp * 3 / 2,
            None => xp,
        }
    }

    /// The skill a job at `bld` is made with: a little more for the master
    /// in their own workshop, lifted a third of the way to the master's
    /// beside them.
    pub(super) fn guided_level(&self, d: &Dwarf, bld: u32, labor: Labor, level: u32) -> u32 {
        let Some(b) = self.buildings.get(&bld) else { return level };
        if b.master == Some(d.id) {
            return level + 1;
        }
        match self.master_near(bld, d.id, d.pos) {
            Some(m) => level + m.skill(labor).saturating_sub(level) / 3,
            None => level,
        }
    }

    /// Name `who` master of workshop `bld` (None: no master).
    pub fn set_master(&mut self, bld: u32, who: Option<u32>) {
        if let Some(b) = self.buildings.get_mut(&bld) {
            b.master = who;
        }
    }

    /// The candidates for master of `bld`, most skilled first.
    pub fn master_candidates(&self, bld: u32) -> Vec<u32> {
        let Some(b) = self.buildings.get(&bld) else { return Vec::new() };
        let Some(labor) = self.shop_labor(b) else { return Vec::new() };
        let mut v: Vec<&Dwarf> = self.dwarves.iter().filter(|d| d.alive && !d.is_child() && d.has(labor)).collect();
        v.sort_by(|a, b| b.skill(labor).cmp(&a.skill(labor)).then(a.id.cmp(&b.id)));
        v.into_iter().map(|d| d.id).collect()
    }

    /// The next master of `bld` in the list after the present one (the `M`
    /// key: round the candidates, then nobody).
    pub fn cycle_master(&mut self, bld: u32) -> Option<u32> {
        let c = self.master_candidates(bld);
        let now = self.buildings.get(&bld).and_then(|b| b.master);
        let next = match now.and_then(|m| c.iter().position(|&x| x == m)) {
            Some(i) => c.get(i + 1).copied(),
            None => c.first().copied(),
        };
        self.set_master(bld, next);
        next
    }

    /// The query screen's line: "Master: Kib Ustuth (smithing 7)".
    pub fn master_line(&self, bld: u32) -> Option<String> {
        let b = self.buildings.get(&bld)?;
        let labor = self.shop_labor(b)?;
        let m = b.master.and_then(|m| self.dwarves.iter().find(|d| d.id == m && d.alive));
        let who = match m {
            Some(d) => format!("Master: {} ({} at {})", d.name, crate::dwarf::skill_rank(d.skills[labor.idx()]), labor.name().to_lowercase()),
            None => "Master: none".into(),
        };
        Some(if b.min_skill > 0 { format!("{} · least skill {}", who, b.min_skill) } else { who })
    }

    /// Weekly, for the overseer: the most skilled dwarf of each workshop's
    /// trade is its master (a dead master's place goes to the next best).
    pub(super) fn ov_masters(&mut self, o: &mut Overseer) {
        let _ = o;
        if self.tick % (7 * DAY) >= DAY {
            return;
        }
        let shops: Vec<u32> = self.buildings.values().filter(|b| b.built && b.kind.is_workshop() && !b.kind.recipes().is_empty()).map(|b| b.id).collect();
        // One dwarf masters one workshop.
        let mut taken: Vec<u32> = Vec::new();
        for bid in shops {
            let Some(labor) = self.shop_labor(&self.buildings[&bid]) else { continue };
            let now = self.buildings[&bid].master.filter(|m| self.dwarves.iter().any(|d| d.id == *m && d.alive));
            let best = self
                .master_candidates(bid)
                .into_iter()
                .filter(|id| !taken.contains(id))
                .find(|&id| self.dwarves.iter().any(|d| d.id == id && !d.soldier && d.skill(labor) >= 3));
            let keep = now.filter(|&m| {
                let skill = |id: u32| self.dwarves.iter().find(|d| d.id == id).map_or(0, |d| d.skill(labor));
                !taken.contains(&m) && best.map_or(true, |b| skill(b) < skill(m) + 2)
            });
            let pick = keep.or(best);
            if let Some(m) = pick {
                taken.push(m);
            }
            if pick != self.buildings[&bid].master {
                self.set_master(bid, pick);
                if let Some(m) = pick {
                    let name = self.dwarves.iter().find(|d| d.id == m).map_or(String::new(), |d| d.name.clone());
                    let shop = self.buildings[&bid].name.clone().unwrap_or_else(|| self.buildings[&bid].kind.name().to_string());
                    let at = self.buildings[&bid].center();
                    let text = format!("{} is named master of the {}; the others there will learn at their side.", name, shop);
                    self.msg_at(&text, (200, 190, 140), Some(at), logkind::LogKind::Job);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buildings::{BuildingKind, Recipe};

    /// A built mason's workshop by the embark, with granite to work.
    fn mason(seed: u64) -> (Game, u32) {
        let mut g = Game::new(seed);
        g.creatures.retain(|_, c| c.tame);
        let site = g.ov_spot(BuildingKind::Mason, g.embark.offset(3, 0, 0), 20).expect("room for a workshop");
        assert!(g.place_building(BuildingKind::Mason, site));
        let id = g.world.t(site).building;
        g.buildings.get_mut(&id).unwrap().built = true;
        (g, id)
    }

    #[test]
    fn an_apprentice_learns_faster_by_a_master() {
        let (mut g, id) = mason(77);
        let at = g.buildings[&id].center();
        let (pupil, master) = (g.dwarves[1].id, g.dwarves[2].id);
        g.dwarves[2].skills[Labor::Masonry.idx()] = 5000;
        // Alone, then with the master beside them.
        g.dwarves[2].pos = at.offset(60, 60, 0);
        let alone = g.apprentice_xp(&g.dwarves[1], id, 30);
        g.set_master(id, Some(master));
        let far = g.apprentice_xp(&g.dwarves[1], id, 30);
        g.dwarves[2].pos = at.offset(2, 1, 0);
        g.dwarves[1].pos = at;
        let near = g.apprentice_xp(&g.dwarves[1], id, 30);
        assert_eq!((alone, far, near), (30, 30, 45));
        assert!(g.guided_level(&g.dwarves[1], id, Labor::Masonry, 0) > 0, "work lifted toward the master's");
        assert_eq!(g.guided_level(&g.dwarves[2], id, Labor::Masonry, 7), 8, "the master's own work a little better");
        // The whole job: a table made beside the master teaches more.
        for d in g.dwarves.iter_mut() {
            d.labors[Labor::Masonry.idx()] = d.id == pupil;
        }
        let before = g.dwarves[1].skills[Labor::Masonry.idx()];
        g.spawn_item(ItemKind::Boulder, Mat::Granite, at);
        g.add_order(id, Recipe::StoneTable);
        for _ in 0..2 * DAY {
            // The master stays at the bench.
            g.dwarves[2].pos = g.dwarves[1].pos.offset(1, 0, 0);
            g.dwarves[2].task = Task::Idle;
            g.step();
            if g.items.values().any(|it| it.kind == ItemKind::Table && it.maker == Some(pupil)) {
                break;
            }
        }
        assert!(g.items.values().any(|it| it.kind == ItemKind::Table && it.maker == Some(pupil)), "the pupil made the table");
        assert_eq!(g.dwarves[1].skills[Labor::Masonry.idx()] - before, 45, "half as much again");
    }

    #[test]
    fn the_least_skill_never_stalls_a_workshop() {
        let (mut g, id) = mason(77);
        g.buildings.get_mut(&id).unwrap().min_skill = 5;
        let b = g.buildings[&id].clone();
        for d in g.dwarves.iter_mut() {
            d.skills[Labor::Masonry.idx()] = 0;
            d.labors[Labor::Masonry.idx()] = true;
        }
        assert!(g.skilled_enough(&b, &g.dwarves[0], Labor::Masonry), "nobody is skilled enough, so anyone may");
        g.dwarves[1].skills[Labor::Masonry.idx()] = 100_000;
        assert!(g.dwarves[1].skill(Labor::Masonry) >= 5);
        assert!(!g.skilled_enough(&b, &g.dwarves[0], Labor::Masonry), "now the novice waits for the skilled");
        assert!(g.skilled_enough(&b, &g.dwarves[1], Labor::Masonry));
    }

    #[test]
    fn the_overseer_names_masters() {
        let (mut g, id) = mason(77);
        g.set_overseer(Some(super::super::overseer::Style::Steady));
        for d in g.dwarves.iter_mut() {
            d.labors[Labor::Masonry.idx()] = true;
            d.skills[Labor::Masonry.idx()] = 0;
            d.soldier = false;
        }
        let best = g.dwarves[3].id;
        g.dwarves[3].skills[Labor::Masonry.idx()] = 100_000;
        let mut o = g.overseer.take().unwrap();
        g.tick = 14 * DAY;
        g.ov_masters(&mut o);
        assert_eq!(g.buildings[&id].master, Some(best));
        assert!(g.master_line(id).unwrap().contains(&g.dwarves[3].name));
        // The master dies: the next best takes the workshop at the next week.
        g.dwarves[3].alive = false;
        g.dwarves[4].skills[Labor::Masonry.idx()] = 50_000;
        g.tick = 21 * DAY;
        g.ov_masters(&mut o);
        assert_eq!(g.buildings[&id].master, Some(g.dwarves[4].id));
    }
}
