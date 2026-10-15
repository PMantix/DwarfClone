//! Death and family: burial in coffins, memorial slabs, ghosts of the
//! unburied, aging and old age, marriage and children.

use super::*;

/// Days an unburied dwarf rests easy before a ghost may rise.
const GHOST_DAYS: u64 = 14;
/// Dwarves come of age at twelve.
pub const ADULT_AGE: u32 = 12;

impl Game {
    fn dead_unrested(&self, name: &str) -> Option<usize> {
        self.dwarves.iter().position(|d| !d.alive && !d.rested && d.name == name)
    }

    /// Whose corpse this is, if it's one of ours still waiting for burial.
    pub(super) fn burial_corpse(&self, it: &Item) -> bool {
        it.kind == ItemKind::Corpse && it.label.as_deref().map_or(false, |n| self.dead_unrested(n).is_some())
    }

    /// Fill in what older saves lack: age and sex.
    pub(super) fn fill_family(&mut self, d: &mut Dwarf) {
        if d.age.is_none() {
            d.age = Some(self.rng.range(20, 90) as u32);
        }
        if d.female.is_none() {
            d.female = Some(self.rng.chance(0.5));
        }
    }

    // ------------------------------------------------------------------
    // Burial
    // ------------------------------------------------------------------

    /// A hauler carries a citizen's corpse to a free coffin, tombs first.
    pub(super) fn find_burial(&mut self, d: &mut Dwarf) -> bool {
        let coffin = self
            .buildings
            .values()
            .filter(|b| b.kind == BuildingKind::Coffin && b.built && b.occupant.is_none() && b.worker.is_none())
            .min_by_key(|b| (self.zone_kind_at(b.pos) != Some(ZoneKind::Tomb), b.id))
            .map(|b| b.id);
        let Some(coffin) = coffin else { return false };
        let corpses: HashSet<u32> =
            self.items.values().filter(|it| it.free() && self.burial_corpse(it)).map(|it| it.id).collect();
        if corpses.is_empty() {
            return false;
        }
        let Some((item, path)) = self.find_item(d.pos, |it| corpses.contains(&it.id)) else { return false };
        self.claim_item(item, d.id);
        self.buildings.get_mut(&coffin).unwrap().worker = Some(d.id);
        Self::set_path(d, path);
        d.task = Task::Bury { item, coffin };
        true
    }

    pub(super) fn do_bury(&mut self, d: &mut Dwarf, item: u32, coffin: u32) {
        let Some(site) = self.buildings.get(&coffin).filter(|b| b.built && b.occupant.is_none()).map(|b| b.pos) else {
            self.release_task(d);
            return;
        };
        match self.fetch(d, item) {
            None => {
                self.release_task(d);
                return;
            }
            Some(false) => return,
            Some(true) => {}
        }
        match self.travel(d, site) {
            Travel::Moving => return,
            Travel::Failed => {
                self.release_task(d);
                return;
            }
            Travel::Arrived => {}
        }
        d.carrying = None;
        let name = self.items.remove(&item).and_then(|it| it.label).unwrap_or_default();
        d.task = Task::Idle;
        let Some(i) = self.dead_unrested(&name) else {
            if let Some(b) = self.buildings.get_mut(&coffin) {
                b.worker = None;
            }
            return;
        };
        let dead = self.dwarves[i].id;
        if let Some(b) = self.buildings.get_mut(&coffin) {
            b.worker = None;
            b.occupant = Some(dead);
        }
        let tomb = self.zone_kind_at(site) == Some(ZoneKind::Tomb);
        self.lay_to_rest(i, if tomb { "has been entombed" } else { "has been laid to rest" }, tomb);
    }

    /// Mark a dead dwarf as resting, lay any ghost and comfort the mourners.
    fn lay_to_rest(&mut self, i: usize, how: &str, tomb: bool) {
        let tick = self.tick;
        let (dead, name, ghost) = {
            let d = &mut self.dwarves[i];
            d.rested = true;
            let g = std::mem::take(&mut d.ghost);
            (d.id, d.name.clone(), g)
        };
        if ghost {
            self.msg(&format!("The ghost of {} has been put to rest.", name), (180, 200, 255));
        } else {
            self.msg(&format!("{} {}.", name, how), (180, 200, 255));
        }
        for o in self.dwarves.iter_mut().filter(|o| o.alive && (o.regard(dead) >= 30 || o.spouse == Some(dead) || o.family_of(dead))) {
            if tomb {
                o.think(&format!("was comforted that {} lies in a tomb", name), -12, tick);
            } else {
                o.think(&format!("was comforted that {} was laid to rest", name), -8, tick);
            }
        }
    }

    /// Daily: a finished memorial slab is engraved for whoever most needs it.
    fn dedicate_slabs(&mut self) {
        loop {
            let Some(slab) = self.buildings.values().find(|b| b.kind == BuildingKind::Slab && b.built && b.occupant.is_none()).map(|b| b.id) else {
                return;
            };
            let who = (0..self.dwarves.len())
                .filter(|&i| !self.dwarves[i].alive && !self.dwarves[i].rested)
                .min_by_key(|&i| (!self.dwarves[i].ghost, self.dwarves[i].died.unwrap_or(0)));
            let Some(i) = who else { return };
            self.buildings.get_mut(&slab).unwrap().occupant = Some(self.dwarves[i].id);
            self.lay_to_rest(i, "is remembered on a memorial slab", false);
        }
    }

    // ------------------------------------------------------------------
    // Ghosts
    // ------------------------------------------------------------------

    /// Where a dead dwarf's remains are, if anywhere.
    fn remains_of(&self, name: &str) -> Option<Pos> {
        self.items.values().find(|it| it.kind == ItemKind::Corpse && it.label.as_deref() == Some(name)).map(|it| it.pos)
    }

    pub(super) fn ghosts_tick(&mut self) {
        let tick = self.tick;
        if tick % DAY == DAY / 4 {
            for i in 0..self.dwarves.len() {
                let d = &self.dwarves[i];
                let restless = !d.alive && !d.rested && !d.ghost && d.died.map_or(false, |t| tick > t + GHOST_DAYS * DAY);
                if restless && self.rng.chance(0.35) {
                    let name = d.name.clone();
                    let at = self.remains_of(&name).unwrap_or(d.pos);
                    let d = &mut self.dwarves[i];
                    d.ghost = true;
                    d.pos = at;
                    self.msg(&format!("The ghost of {} has risen to haunt the fortress!", name), (180, 200, 255));
                    self.pause_request = true;
                }
            }
        }
        if tick % 10 != 0 {
            return;
        }
        let ghosts: Vec<usize> = (0..self.dwarves.len()).filter(|&i| self.dwarves[i].ghost).collect();
        for i in ghosts {
            let p = self.dwarves[i].pos;
            // Drift toward the living, or about at random.
            let near = self
                .dwarves
                .iter()
                .filter(|o| o.alive && o.pos.z == p.z && (o.pos.x - p.x).abs() + (o.pos.y - p.y).abs() <= 20)
                .min_by_key(|o| (o.pos.x - p.x).abs() + (o.pos.y - p.y).abs())
                .map(|o| o.pos);
            let (dx, dy) = match near {
                Some(q) if self.rng.chance(0.25) => ((q.x - p.x).signum(), (q.y - p.y).signum()),
                _ => DIRS8[self.rng.range(0, 8) as usize],
            };
            let q = p.offset(dx, dy, 0);
            if self.world.walkable(q) {
                self.dwarves[i].pos = q;
            }
            if tick % 50 == 0 {
                self.haunt(i);
            }
        }
    }

    fn haunt(&mut self, g: usize) {
        let tick = self.tick;
        let p = self.dwarves[g].pos;
        let text = format!("was haunted by the ghost of {}", self.dwarves[g].name);
        for o in self.dwarves.iter_mut() {
            if !o.alive || o.asleep || o.pos.z != p.z || (o.pos.x - p.x).abs() > 2 || (o.pos.y - p.y).abs() > 2 {
                continue;
            }
            if o.thoughts.iter().any(|t| t.text == text && tick < t.tick + DAY) {
                continue;
            }
            o.think(&text, 10, tick);
        }
    }

    // ------------------------------------------------------------------
    // Aging, marriage and children
    // ------------------------------------------------------------------

    /// At the turn of the year everyone grows older; children come of age and
    /// the very old may die.
    pub(super) fn new_year(&mut self) {
        for i in 0..self.dwarves.len() {
            if !self.dwarves[i].alive {
                continue;
            }
            let mut d = std::mem::take(&mut self.dwarves[i]);
            let age = d.age.unwrap_or(30) + 1;
            d.age = Some(age);
            if age == ADULT_AGE {
                d.labors[Labor::Hauling.idx()] = true;
                d.labors[Labor::Building.idx()] = true;
                let name = d.name.clone();
                self.msg(&format!("{} has come of age.", name), (150, 220, 255));
            }
            // Dwarves live to about 150; few see 180.
            if age > 130 && self.rng.chance(((age - 130) as f64 / 50.0).min(1.0)) {
                self.kill(&mut d, "has died of old age");
            }
            self.dwarves[i] = d;
        }
    }

    pub(super) fn families_daily(&mut self) {
        self.dedicate_slabs();
        let tick = self.tick;
        // Births.
        for i in 0..self.dwarves.len() {
            let d = &self.dwarves[i];
            if !d.alive || d.due.map_or(true, |t| tick < t) {
                continue;
            }
            let father = d.spouse.unwrap_or(0);
            let pos = d.pos;
            let id = self.new_id();
            let child = dwarf::new_child(id, pos, &mut self.rng, &self.dwarves[i], father);
            let (cname, girl) = (child.name.clone(), child.female == Some(true));
            let m = &mut self.dwarves[i];
            m.due = None;
            m.think("gave birth to a child", -60, tick);
            let mname = m.name.clone();
            if let Some(f) = self.dwarves.iter_mut().find(|f| f.id == father && f.alive) {
                f.think("became a parent", -40, tick);
            }
            self.dwarves.push(child);
            self.msg(&format!("{} has given birth to a {}, {}.", mname, if girl { "girl" } else { "boy" }, cname), (150, 255, 150));
            let fort = self.fortress_name.clone();
            self.legend(format!("{} was born in {} to {}.", cname, fort, mname));
            self.year_log.births += 1;
        }
        // Weddings, between close friends of age.
        let adults: Vec<usize> = (0..self.dwarves.len())
            .filter(|&i| {
                let d = &self.dwarves[i];
                d.alive && !d.insane && d.age.map_or(false, |a| a >= 18) && d.female.is_some() && !self.married(d)
            })
            .collect();
        let mut wed: Option<(usize, usize)> = None;
        'find: for (n, &i) in adults.iter().enumerate() {
            for &j in &adults[n + 1..] {
                let (a, b) = (&self.dwarves[i], &self.dwarves[j]);
                if a.female != b.female && a.regard(b.id) >= 70 && b.regard(a.id) >= 70 && !a.family_of(b.id) && !b.family_of(a.id) {
                    wed = Some((i, j));
                    break 'find;
                }
            }
        }
        if let Some((i, j)) = wed.filter(|_| self.rng.chance(0.2)) {
            let (ai, bi) = (self.dwarves[i].id, self.dwarves[j].id);
            let (an, bn) = (self.dwarves[i].name.clone(), self.dwarves[j].name.clone());
            self.dwarves[i].spouse = Some(bi);
            self.dwarves[j].spouse = Some(ai);
            self.dwarves[i].think(&format!("married {}", bn), -40, tick);
            self.dwarves[j].think(&format!("married {}", an), -40, tick);
            self.msg(&format!("{} and {} have married.", an, bn), (255, 200, 220));
            let fort = self.fortress_name.clone();
            self.legend(format!("{} and {} married in {}.", an, bn, fort));
        }
        // Pregnancies.
        for i in 0..self.dwarves.len() {
            let d = &self.dwarves[i];
            let ok = d.alive
                && d.female == Some(true)
                && d.due.is_none()
                && d.age.map_or(false, |a| (18..=110).contains(&a))
                && self.married(d)
                && self.dwarves.iter().filter(|c| c.parents.map_or(false, |p| p.0 == d.id)).count() < 4;
            if ok && self.rng.chance(1.0 / 120.0) {
                self.dwarves[i].due = Some(tick + 9 * MONTH);
            }
        }
    }

    /// Has a living spouse.
    fn married(&self, d: &Dwarf) -> bool {
        d.spouse.map_or(false, |s| self.dwarves.iter().any(|o| o.id == s && o.alive))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quiet(seed: u64) -> Game {
        let mut g = Game::new(seed);
        g.creatures.retain(|_, c| c.tame);
        g
    }

    fn kill_first(g: &mut Game) -> String {
        let mut d = std::mem::take(&mut g.dwarves[0]);
        g.kill(&mut d, "was struck down");
        let name = d.name.clone();
        g.dwarves[0] = d;
        name
    }

    fn run_days(g: &mut Game, days: u64) {
        for _ in 0..days * DAY {
            g.step();
            g.creatures.retain(|_, c| c.tame || !c.hostile());
        }
    }

    #[test]
    fn an_unburied_dwarf_rises_as_a_ghost() {
        let mut g = quiet(81);
        let name = kill_first(&mut g);
        run_days(&mut g, GHOST_DAYS + 12);
        assert!(g.dwarves[0].ghost, "no ghost of {} after {} days", name, GHOST_DAYS + 12);
        // It haunts the living.
        let text = format!("was haunted by the ghost of {}", name);
        assert!(g.dwarves.iter().any(|d| d.thoughts.iter().any(|t| t.text == text)), "the ghost scared nobody");
        // A memorial slab lays it to rest.
        let e = g.embark;
        let p = (2..20).map(|r| e.offset(r, 0, 0)).find(|&p| g.can_place(BuildingKind::Slab, p)).unwrap();
        assert!(g.place_building(BuildingKind::Slab, p));
        let id = g.world.t(p).building;
        g.buildings.get_mut(&id).unwrap().built = true;
        run_days(&mut g, 1);
        assert!(!g.dwarves[0].ghost && g.dwarves[0].rested, "the slab didn't lay the ghost");
    }

    #[test]
    fn a_buried_dwarf_stays_at_rest() {
        let mut g = quiet(81);
        let e = g.embark;
        let p = (2..20).map(|r| e.offset(r, 1, 0)).find(|&p| g.can_place(BuildingKind::Coffin, p)).unwrap();
        assert!(g.place_building(BuildingKind::Coffin, p));
        let coffin = g.world.t(p).building;
        g.buildings.get_mut(&coffin).unwrap().built = true;
        let name = kill_first(&mut g);
        run_days(&mut g, GHOST_DAYS + 12);
        assert!(g.dwarves[0].rested, "{} was never buried", name);
        assert!(!g.dwarves[0].ghost);
        assert_eq!(g.buildings[&coffin].occupant, Some(g.dwarves[0].id));
        assert!(!g.items.values().any(|it| it.kind == ItemKind::Corpse && it.label.as_deref() == Some(name.as_str())));
    }

    #[test]
    fn the_very_old_die_and_children_grow_up() {
        let mut g = quiet(81);
        g.dwarves[0].age = Some(185);
        let id = g.new_id();
        let pos = g.dwarves[1].pos;
        let mut kid = dwarf::new_child(id, pos, &mut g.rng, &g.dwarves[1], g.dwarves[2].id);
        kid.age = Some(ADULT_AGE - 1);
        assert!(kid.is_child() && !kid.has(Labor::Hauling));
        g.dwarves.push(kid);
        g.new_year();
        assert!(!g.dwarves[0].alive, "a 186-year-old should die of old age");
        let kid = g.dwarves.iter().find(|d| d.id == id).unwrap();
        assert!(!kid.is_child() && kid.has(Labor::Hauling));
    }

    #[test]
    fn close_friends_marry_and_have_children() {
        let mut g = quiet(81);
        let (a, b) = (0, 1);
        for (i, f) in [(a, true), (b, false)] {
            g.dwarves[i].female = Some(f);
            g.dwarves[i].age = Some(40);
        }
        let (ai, bi) = (g.dwarves[a].id, g.dwarves[b].id);
        g.dwarves[a].adjust_regard(bi, 90);
        g.dwarves[b].adjust_regard(ai, 90);
        for _ in 0..60 {
            g.families_daily();
        }
        assert_eq!(g.dwarves[a].spouse, Some(bi), "close friends never married");
        let mut born = false;
        for _ in 0..(2 * 12 * 28) {
            g.tick += DAY;
            g.families_daily();
            if g.dwarves.iter().any(|d| d.parents == Some((ai, bi))) {
                born = true;
                break;
            }
        }
        assert!(born, "no child in two years");
        let kid = g.dwarves.iter().find(|d| d.parents == Some((ai, bi))).unwrap();
        assert!(kid.is_child());
        assert_eq!(kid.name.split(' ').nth(1), g.dwarves[a].name.split(' ').nth(1));
    }
}
