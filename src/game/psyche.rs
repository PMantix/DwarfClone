//! Personalities, relationships, rooms, nobles, strange moods and madness.

use super::combat::adjacent;
use super::*;
use crate::buildings::Input;
use crate::dwarf::{artifact_name, Mood, MoodNeed, Noble};
use crate::items::{quality_mult, ARTIFACT};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mandate {
    pub noble: u32,
    pub kind: ItemKind,
    pub mat: Option<Mat>,
    pub count: u32,
    pub baseline: u32,
    pub deadline: u64,
}

impl Mandate {
    pub fn describe(&self) -> String {
        let what = Item {
            id: 0,
            kind: self.kind,
            mat: self.mat.unwrap_or(Mat::Granite),
            pos: Pos::default(),
            carried_by: None,
            claimed_by: None,
            label: None,
            container: None,
            quality: 0,
            to_depot: false,
            dump: false,
            wear: 0,
            forbidden: false,
        }
        .name();
        let what = if self.mat.is_none() {
            what.split_whitespace().last().unwrap_or("goods").to_string()
        } else {
            what
        };
        format!("{} x{}", what, self.count)
    }
}

impl MoodNeed {
    fn input(self) -> Input {
        match self {
            MoodNeed::Log => Input::Log,
            MoodNeed::Stone => Input::Stone,
            MoodNeed::Bar => Input::Bar,
            MoodNeed::Bone => Input::Bone,
            MoodNeed::Gem => Input::RoughGem,
        }
    }

    pub fn desc(self) -> &'static str {
        self.input().desc()
    }
}

fn shop_for(l: Labor) -> BuildingKind {
    match l {
        Labor::Carpentry => BuildingKind::Carpenter,
        Labor::Masonry => BuildingKind::Mason,
        Labor::Smithing => BuildingKind::Forge,
        _ => BuildingKind::Craftsdwarf,
    }
}

impl Game {
    pub fn building_value(&self, b: &Building) -> u32 {
        if !b.built {
            return 0;
        }
        let base = if b.kind == BuildingKind::Monument {
            400
        } else if b.kind.is_workshop() {
            20
        } else {
            10
        };
        base * b.mat.value() * quality_mult(b.quality)
    }

    pub fn wealth(&self) -> u32 {
        let items: u32 = self.items.values().map(|it| it.value()).sum();
        let blds: u32 = self.buildings.values().map(|b| self.building_value(b)).sum();
        items + blds
    }

    /// An enclosed underground room containing `p`, and its value.
    pub fn room_at(&self, p: Pos) -> Option<(Vec<Pos>, u32)> {
        let w = &self.world;
        if !w.roofed(p) {
            return None;
        }
        let mut seen = HashSet::new();
        let mut stack = vec![p];
        seen.insert(p);
        let mut walls: HashSet<Pos> = HashSet::new();
        while let Some(q) = stack.pop() {
            let t = w.t(q);
            if !matches!(t.kind, TileKind::Floor | TileKind::Shrub) || !w.roofed(q) {
                return None;
            }
            for (dx, dy) in DIRS4 {
                let n = q.offset(dx, dy, 0);
                let Some(nt) = w.get(n) else { return None };
                if nt.kind.encloses() {
                    walls.insert(n);
                } else if nt.door != world::DOOR_NONE {
                    // Doors bound a room.
                } else if seen.insert(n) {
                    if seen.len() > 80 {
                        return None;
                    }
                    stack.push(n);
                }
            }
        }
        let mut value = seen.len() as u32 * 2;
        let mut counted = HashSet::new();
        for q in &seen {
            let bid = w.t(*q).building;
            if bid != 0 && counted.insert(bid) {
                if let Some(b) = self.buildings.get(&bid) {
                    value += self.building_value(b);
                }
            }
        }
        value += walls.iter().map(|q| (w.t(*q).mat.value() - 1) * 3 + w.detail_value(*q)).sum::<u32>();
        value += seen.iter().map(|q| w.detail_value(*q)).sum::<u32>();
        Some((seen.into_iter().collect(), value))
    }

    /// Daily: recompute room values and hand out bedrooms, nobles first.
    pub(super) fn assign_rooms(&mut self) {
        let ids: Vec<u32> = self.buildings.keys().copied().collect();
        for id in &ids {
            let b = &self.buildings[id];
            if !b.built || !matches!(b.kind, BuildingKind::Bed | BuildingKind::Table) {
                continue;
            }
            let v = self.room_value_at(b.pos);
            self.buildings.get_mut(id).unwrap().room_value = v;
        }
        let living: HashSet<u32> = self.dwarves.iter().filter(|d| d.alive && !d.insane).map(|d| d.id).collect();
        for b in self.buildings.values_mut() {
            if b.owner.map_or(false, |o| !living.contains(&o)) {
                b.owner = None;
            }
        }
        for z in self.zones.values_mut() {
            if z.owner.map_or(false, |o| !living.contains(&o)) {
                z.owner = None;
            }
        }
        // Rooms the player assigned are not reshuffled below.
        let fixed = self.apply_zone_beds();
        let mut order: Vec<(bool, u32)> = self
            .dwarves
            .iter()
            .filter(|d| living.contains(&d.id))
            .map(|d| (d.noble.is_none(), d.id))
            .collect();
        order.sort();
        for (_, did) in order {
            if fixed.contains(&did) {
                continue;
            }
            let owned = self.buildings.values().find(|b| b.owner == Some(did)).map(|b| (b.id, b.room_value));
            let best_free = self
                .buildings
                .values()
                .filter(|b| b.kind == BuildingKind::Bed && b.built && b.owner.is_none() && self.bed_assignable(b.pos))
                .max_by_key(|b| (b.room_value, std::cmp::Reverse(b.id)))
                .map(|b| (b.id, b.room_value));
            let is_noble = self.dwarves.iter().any(|d| d.id == did && d.noble.is_some());
            match (owned, best_free) {
                (None, Some((bid, _))) => self.buildings.get_mut(&bid).unwrap().owner = Some(did),
                (Some((old, ov)), Some((bid, bv))) if is_noble && bv > ov => {
                    self.buildings.get_mut(&old).unwrap().owner = None;
                    self.buildings.get_mut(&bid).unwrap().owner = Some(did);
                }
                _ => {}
            }
        }
    }

    /// Chairs next to tables that nobody is sitting in.
    pub(super) fn free_seats(&self) -> HashMap<Pos, u32> {
        let tables: Vec<Pos> = self
            .buildings
            .values()
            .filter(|b| b.kind == BuildingKind::Table && b.built)
            .map(|b| b.pos)
            .collect();
        self.buildings
            .values()
            .filter(|b| b.kind == BuildingKind::Chair && b.built && b.occupant.is_none())
            .filter(|b| tables.iter().any(|t| adjacent(*t, b.pos)))
            .map(|b| (b.pos, b.id))
            .collect()
    }

    pub(super) fn eat_thoughts(&mut self, d: &mut Dwarf, kind: ItemKind, mat: Mat, seat: Option<u32>) {
        let tick = self.tick;
        match kind {
            ItemKind::Meal => d.think("enjoyed a fine prepared meal", -12, tick),
            ItemKind::Meat => d.think("ate some roasted meat", -4, tick),
            ItemKind::Fish => d.think("ate a grilled fish", -4, tick),
            _ => {
                let plant = mat.crop().map_or("stoutcaps", |c| c.plant);
                d.think(&format!("ate raw {}", plant), 0, tick);
                // Eating a plant leaves its seed behind, as in DF.
                if mat.crop().is_some() {
                    self.spawn_item(ItemKind::Seed, mat, d.pos);
                }
            }
        }
        if d.likes_food == Some(kind) {
            d.think("ate their favorite food", -8, tick);
        }
        match seat {
            Some(chair) => {
                let cpos = self.buildings.get(&chair).map(|b| b.pos);
                let hall = self
                    .buildings
                    .values()
                    .filter(|b| b.kind == BuildingKind::Table && cpos.map_or(false, |c| adjacent(c, b.pos)))
                    .map(|b| b.room_value)
                    .max()
                    .unwrap_or(0);
                if hall >= 80 {
                    d.think("dined in a fine dining room", -10, tick);
                } else if cpos.map_or(false, |c| self.in_dining_hall(c)) {
                    d.think("ate in the dining hall", -6, tick);
                } else {
                    d.think("ate at a table", -4, tick);
                }
                if let Some(b) = self.buildings.get_mut(&chair) {
                    b.occupant = None;
                }
            }
            None => d.think("ate without a table", 3, tick),
        }
    }

    pub(super) fn sleep_thought(&mut self, d: &mut Dwarf, bed: Option<u32>) {
        let tick = self.tick;
        let Some(b) = bed.and_then(|id| self.buildings.get(&id)) else {
            d.think("slept on the hard ground", 10, tick);
            return;
        };
        let own = b.owner == Some(d.id);
        if self.zone_kind_at(b.pos) == Some(ZoneKind::Dormitory) {
            d.think("slept in a crowded dormitory", 2, tick);
            return;
        }
        match b.room_value {
            0 => d.think("slept in a bed", -4, tick),
            v if v < 40 => d.think(if own { "slept in a modest bedroom of their own" } else { "slept in a modest bedroom" }, -7, tick),
            v if v < 100 => d.think(if own { "slept in a decent bedroom of their own" } else { "slept in a decent bedroom" }, -11, tick),
            v if v < 250 => d.think(if own { "slept in a fine bedroom of their own" } else { "slept in a fine bedroom" }, -16, tick),
            _ => d.think("slept in a splendid bedroom", -24, tick),
        }
    }

    pub(super) fn roll_quality(&mut self, level: u32) -> u8 {
        (level as i32 / 2 + self.rng.range(-1, 2)).clamp(0, 5) as u8
    }

    /// Social life, misery and tantrum mischief. Called every tick per dwarf.
    pub(super) fn psyche_tick(&mut self, d: &mut Dwarf) {
        let tick = self.tick;
        if tick % 50 == (d.id as u64 % 50) {
            if d.stress >= 650 {
                d.misery += 50;
            } else {
                d.misery = d.misery.saturating_sub(25);
            }
            if d.misery >= 2 * DAY as u32 && !d.insane {
                self.go_insane(d, "has been stricken by melancholy");
                return;
            }
        }
        // Melancholy can lift, once the grief has eased, for a dwarf with
        // someone who cares for them still alive.
        // (Time heals, not the stress gauge: a dwarf starving in the dark
        // stays stressed whatever else.)
        if d.insane && d.melancholy && tick % DAY == d.id as u64 % DAY && tick > d.mad_since + 5 * DAY {
            let loved = d.spouse.map_or(false, |s| self.dwarves.iter().any(|o| o.id == s && o.alive))
                || d.relations.iter().any(|&(o, r)| r >= 30 && self.dwarves.iter().any(|x| x.id == o && x.alive));
            if loved && self.rng.chance(0.12) {
                d.insane = false;
                d.melancholy = false;
                d.misery = 0;
                d.think("came back out of the dark", -40, tick);
                let name = d.name.clone();
                self.msg(&format!("{} has come back out of melancholy.", name), (170, 200, 255));
                self.spotlight("Out of the dark", &format!("{} has come back out of melancholy, and knows their friends again.", name), Some(d.pos), (170, 200, 255));
            }
        }
        if tick % 50 == (d.id as u64 % 50) {
            self.needs_tick(d);
            self.socialize(d);
        }
        if d.personality.sociability > 25 && tick > d.last_chat + 3 * DAY && tick % DAY == d.id as u64 % DAY {
            d.think("felt lonely", 6, tick);
        }
        if d.tantrum > 0 && tick % 100 == d.id as u64 % 100 {
            self.tantrum_mischief(d);
        }
    }

    fn socialize(&mut self, d: &mut Dwarf) {
        let calm = matches!(
            d.task,
            Task::Idle | Task::Wander { .. } | Task::Eat { .. } | Task::Dine { .. } | Task::DrinkItem { .. } | Task::Train | Task::Station { .. }
        );
        // Dwarves at the meeting hall are there to talk.
        let gap = if self.in_meeting_area(d.pos) { DAY / 6 } else { DAY / 2 };
        if !calm || d.asleep || self.tick < d.last_chat + gap {
            return;
        }
        let Some(j) = self.dwarves.iter().position(|o| {
            o.alive && !o.asleep && !o.insane && o.id != d.id && (o.pos.x - d.pos.x).abs() <= 3 && (o.pos.y - d.pos.y).abs() <= 3 && o.pos.z == d.pos.z
        }) else {
            return;
        };
        let tick = self.tick;
        let (oid, oname, osoc, ostress, otemper) = {
            let o = &self.dwarves[j];
            (o.id, o.name.clone(), o.personality.sociability, o.stress, o.personality.temper)
        };
        let soc = d.personality.sociability;
        let argue = d.stress > 150 && ostress > 150 && (d.personality.temper.max(otemper) > 20 || self.rng.chance(0.2));
        let delta = if argue {
            -15
        } else {
            let compat = 10 - (soc - osoc).abs() / 10;
            compat / 3 + self.rng.range(-3, 4) + (soc + osoc) / 30
        };
        d.adjust_regard(oid, delta);
        d.last_chat = tick;
        d.satisfy(crate::dwarf::NeedKind::Socialize, 250);
        let regard = d.regard(oid);
        if argue {
            d.think(&format!("argued with {}", oname), 8, tick);
        } else if regard >= 50 {
            d.think(&format!("chatted with their friend {}", oname), -6, tick);
        } else {
            d.think(&format!("talked with {}", oname), if soc > 20 { -4 } else { -1 }, tick);
        }
        let dname = d.name.clone();
        let o = &mut self.dwarves[j];
        o.adjust_regard(d.id, delta);
        o.last_chat = tick;
        o.satisfy(crate::dwarf::NeedKind::Socialize, 250);
        if argue {
            o.think(&format!("argued with {}", dname), 8, tick);
        } else if o.regard(d.id) >= 50 {
            o.think(&format!("chatted with their friend {}", dname), -6, tick);
        }
        if regard <= -40 && regard - delta > -40 {
            self.msg(&format!("{} has developed a grudge against {}.", dname, oname), (230, 160, 90));
        }
    }

    pub(super) fn tantrum_mischief(&mut self, d: &mut Dwarf) {
        let tick = self.tick;
        if self.rng.chance(0.4) {
            if let Some(j) = self.dwarves.iter().position(|o| o.alive && o.id != d.id && adjacent(o.pos, d.pos)) {
                let part = self.dwarves[j].body.pick_part(&mut self.rng);
                let o = &mut self.dwarves[j];
                o.body.wound(part, 4, false);
                o.adjust_regard(d.id, -25);
                o.think(&format!("was punched by {}", d.name), 12, tick);
                d.satisfy(crate::dwarf::NeedKind::Fight, 150);
                let oname = o.name.clone();
                let oid = o.id;
                d.adjust_regard(oid, -10);
                self.msg(&format!("{} punches {} in a rage!", d.name, oname), (255, 120, 90));
                self.report_crime(super::CrimeKind::Assault, d.id, Some(oid), format!("punched {}", oname));
                return;
            }
        }
        if self.rng.chance(0.3) {
            let near = self
                .buildings
                .values()
                .find(|b| b.built && !b.kind.is_workshop() && b.kind != BuildingKind::Door && (b.pos.x - d.pos.x).abs() <= 1 && (b.pos.y - d.pos.y).abs() <= 1 && b.pos.z == d.pos.z)
                .map(|b| (b.id, b.name()));
            if let Some((bid, name)) = near {
                self.remove_building(bid);
                self.msg(&format!("{} topples a {} in a fit of rage!", d.name, name.to_lowercase()), (255, 120, 90));
                self.report_crime(super::CrimeKind::Vandalism, d.id, None, format!("toppled a {}", name.to_lowercase()));
            }
        }
    }

    pub(super) fn go_insane(&mut self, d: &mut Dwarf, how: &str) {
        self.end_mood(d);
        self.release_task(d);
        d.insane = true;
        d.melancholy = how.contains("melancholy");
        d.mad_since = self.tick;
        d.soldier = false;
        d.noble = None;
        if let Some(id) = d.held.take() {
            self.drop_item(id, d.pos);
        }
        for id in std::mem::take(&mut d.armor) {
            self.drop_item(id, d.pos);
        }
        let name = d.name.clone();
        self.msg(&format!("{} {}.", name, how), (150, 150, 255));
        self.pause_request = true;
    }

    // ------------------------------------------------------------------
    // Nobles and mandates
    // ------------------------------------------------------------------

    pub(super) fn appoint_leader(&mut self) {
        if let Some(d) = self.dwarves.iter_mut().filter(|d| d.alive && !d.is_child() && d.noble.is_none()).max_by_key(|d| d.personality.sociability) {
            d.noble = Some(Noble::Leader);
        }
    }

    pub(super) fn nobles_daily(&mut self) {
        self.assign_offices();
        let alive = self.alive_count();
        let has = |g: &Game, n: Noble| g.dwarves.iter().any(|d| d.alive && !d.insane && d.noble == Some(n));
        if alive >= 20 && !has(self, Noble::Mayor) {
            let pick = self
                .dwarves
                .iter()
                .filter(|d| d.alive && !d.insane && !d.is_child() && d.noble.map_or(true, |n| n == Noble::Leader))
                .max_by_key(|d| d.personality.sociability + d.relations.iter().map(|r| r.1).sum::<i32>() / 10)
                .map(|d| d.id);
            if let Some(id) = pick {
                for d in self.dwarves.iter_mut() {
                    if d.noble == Some(Noble::Leader) {
                        d.noble = None;
                    }
                }
                let d = self.dwarves.iter_mut().find(|d| d.id == id).unwrap();
                d.noble = Some(Noble::Mayor);
                let name = d.name.clone();
                self.msg(&format!("{} has been elected mayor.", name), (255, 230, 120));
                let fort = self.fortress_name.clone();
                self.legend(format!("{} was elected mayor of {}.", name, fort));
            }
        } else if alive > 0 && !has(self, Noble::Mayor) && !has(self, Noble::Leader) {
            self.appoint_leader();
            if let Some(d) = self.dwarves.iter().find(|d| d.noble == Some(Noble::Leader) && d.alive) {
                let name = d.name.clone();
                self.msg(&format!("{} has taken charge as expedition leader.", name), (255, 230, 120));
            }
        }
        // Nobles fume over shabby quarters.
        let tick = self.tick;
        let rooms: HashMap<u32, u32> = self.buildings.values().filter_map(|b| b.owner.map(|o| (o, b.room_value))).collect();
        for d in self.dwarves.iter_mut().filter(|d| d.alive && !d.insane) {
            if let Some(n) = d.noble {
                if rooms.get(&d.id).copied().unwrap_or(0) < n.room_demand() {
                    d.think("is upset about their inadequate quarters", 15, tick);
                }
            }
        }
    }

    fn mandate_count(&self, kind: ItemKind, mat: Option<Mat>) -> u32 {
        let items = self.items.values().filter(|it| it.kind == kind && mat.map_or(true, |m| it.mat == m)).count();
        let built = self
            .buildings
            .values()
            .filter(|b| b.built && mat.map_or(true, |m| b.mat == m))
            .filter(|b| match b.kind {
                BuildingKind::Bed => kind == ItemKind::Bed,
                BuildingKind::Table => kind == ItemKind::Table,
                BuildingKind::Chair => kind == ItemKind::Chair,
                BuildingKind::Door => kind == ItemKind::Door,
                _ => false,
            })
            .count();
        (items + built) as u32
    }

    pub(super) fn issue_mandate(&mut self) {
        if self.mandate.is_some() || !self.rng.chance(0.6) {
            return;
        }
        let Some(noble) = self.dwarves.iter().find(|d| d.alive && !d.insane && d.noble.is_some()).map(|d| d.id) else {
            return;
        };
        let kind = *self.rng.pick(&[ItemKind::Craft, ItemKind::Table, ItemKind::Chair, ItemKind::Door, ItemKind::Barrel, ItemKind::Bed]);
        let mat = if kind != ItemKind::Barrel && kind != ItemKind::Bed && self.rng.chance(0.35) {
            Some(*self.rng.pick(&[Mat::Granite, Mat::Limestone, Mat::Sandstone, Mat::Marble]))
        } else {
            None
        };
        let count = self.rng.range(2, 5) as u32;
        let baseline = self.mandate_count(kind, mat);
        let m = Mandate { noble, kind, mat, count, baseline, deadline: self.tick + SEASON };
        let title = self.dwarves.iter().find(|d| d.id == noble).and_then(|d| d.noble).map_or("noble", |n| n.title());
        self.msg(&format!("The {} has mandated the production of {}.", title.to_lowercase(), m.describe()), (255, 230, 120));
        self.mandate = Some(m);
    }

    pub(super) fn check_mandate(&mut self) {
        let Some(m) = self.mandate.clone() else { return };
        let tick = self.tick;
        let have = self.mandate_count(m.kind, m.mat);
        if have >= m.baseline + m.count {
            if let Some(n) = self.dwarves.iter_mut().find(|d| d.id == m.noble) {
                n.think("was pleased that a mandate was met", -25, tick);
            }
            self.msg(&format!("The mandate for {} has been fulfilled.", m.describe()), (150, 255, 150));
            self.mandate = None;
        } else if tick > m.deadline {
            if let Some(n) = self.dwarves.iter_mut().find(|d| d.id == m.noble) {
                n.think("is furious that a mandate went unmet", 40, tick);
            }
            let others: Vec<usize> = (0..self.dwarves.len())
                .filter(|&i| self.dwarves[i].alive && self.dwarves[i].id != m.noble)
                .collect();
            if !others.is_empty() {
                // Someone gets the blame; justice is the sheriff's business.
                let i = *self.rng.pick(&others);
                let noble = m.noble;
                let d = &mut self.dwarves[i];
                d.think("was blamed for a failed mandate", 10, tick);
                d.adjust_regard(noble, -20);
                let (name, id) = (d.name.clone(), d.id);
                self.msg(&format!("The mandate for {} went unmet. {} is blamed.", m.describe(), name), (255, 120, 90));
                self.report_crime(super::CrimeKind::Disobedience, id, Some(noble), format!("failed to make {}", m.describe()));
            }
            self.mandate = None;
        }
    }

    // ------------------------------------------------------------------
    // Strange moods
    // ------------------------------------------------------------------

    pub(super) fn maybe_strange_mood(&mut self) {
        // Rare: a season in five, in a fortress of some size.
        if self.alive_count() < 10 || self.dwarves.iter().any(|d| d.mood.is_some()) || !self.rng.chance(0.2) {
            return;
        }
        let cands: Vec<usize> = (0..self.dwarves.len())
            .filter(|&i| {
                let d = &self.dwarves[i];
                d.alive && !d.insane && !d.soldier && d.noble.is_none() && !d.is_child()
            })
            .collect();
        if cands.is_empty() {
            return;
        }
        let i = *self.rng.pick(&cands);
        self.start_mood(i);
    }

    /// Dwarf `i` is taken by a fey mood.
    pub(super) fn start_mood(&mut self, i: usize) {
        let labor = {
            let d = &self.dwarves[i];
            LABORS
                .iter()
                .copied()
                .filter(|l| l.moody())
                .max_by_key(|l| d.skills[l.idx()])
                .filter(|l| d.skills[l.idx()] > 0)
        };
        let labor = labor.unwrap_or(*self.rng.pick(&[Labor::Carpentry, Labor::Masonry, Labor::Crafting]));
        let special = if self.rng.chance(0.5) { MoodNeed::Gem } else { MoodNeed::Bone };
        let needs = match labor {
            Labor::Carpentry => vec![MoodNeed::Log, MoodNeed::Log, special],
            Labor::Masonry => vec![MoodNeed::Stone, MoodNeed::Stone, special],
            Labor::Smithing => vec![MoodNeed::Bar, MoodNeed::Bar, special],
            _ => vec![MoodNeed::Stone, MoodNeed::Bone, MoodNeed::Gem],
        };
        let tick = self.tick;
        let mut d = std::mem::take(&mut self.dwarves[i]);
        if !d.task.is_need() {
            self.release_task(&mut d);
        }
        d.mood = Some(Mood { labor, shop: None, needs, items: Vec::new(), delivered: 0, started: tick, work_left: 0 });
        let name = d.name.clone();
        self.dwarves[i] = d;
        self.msg(&format!("{} is taken by a fey mood!", name), (255, 120, 255));
        self.pause_request = true;
    }

    /// Release a mood's workshop and materials.
    pub(super) fn end_mood(&mut self, d: &mut Dwarf) {
        let Some(m) = d.mood.take() else { return };
        for id in m.items {
            self.unclaim(id);
        }
        if let Some(b) = m.shop.and_then(|s| self.buildings.get_mut(&s)) {
            if b.worker == Some(d.id) {
                b.worker = None;
            }
        }
    }

    pub(super) fn do_mood(&mut self, d: &mut Dwarf) {
        let Some(mut m) = d.mood.take() else {
            d.task = Task::Idle;
            return;
        };
        let tick = self.tick;
        if m.work_left == 0 && tick > m.started + 2 * MONTH {
            d.mood = Some(m);
            self.go_insane(d, "has gone stark raving mad, unable to finish their work");
            return;
        }
        // Claim a workshop.
        let shop_ok = m.shop.and_then(|s| self.buildings.get(&s)).map_or(false, |b| b.built);
        if !shop_ok {
            let kind = shop_for(m.labor);
            let free = self
                .buildings
                .values()
                .filter(|b| b.kind == kind && b.built && b.worker.is_none())
                .min_by_key(|b| (b.pos.x - d.pos.x).abs() + (b.pos.y - d.pos.y).abs() + 3 * (b.pos.z - d.pos.z).abs())
                .map(|b| b.id);
            match free {
                Some(s) => {
                    self.buildings.get_mut(&s).unwrap().worker = Some(d.id);
                    m.shop = Some(s);
                    let name = d.name.clone();
                    self.msg(&format!("{} has claimed a {}.", name, kind.name()), (255, 120, 255));
                }
                None => {
                    self.trouble(super::troubles::Lack::MoodShop(kind));
                    self.warn(&format!("moodshop{}", d.id), &format!("{} needs a {} for their mood!", d.name, kind.name()), (255, 120, 255));
                    d.mood = Some(m);
                    d.task = Task::Idle;
                    d.idle_cd = 200;
                    return;
                }
            }
        }
        let site = self.work_spot(self.buildings[&m.shop.unwrap()].center());
        // Gather materials, one claim at a time.
        if m.items.len() < m.needs.len() {
            let need = m.needs[m.items.len()];
            // After twenty days a moody dwarf settles for more of their main material.
            let need = if matches!(need, MoodNeed::Gem | MoodNeed::Bone) && tick > m.started + 20 * DAY && m.labor != Labor::Crafting {
                m.needs[0]
            } else {
                need
            };
            let input = need.input();
            match self.find_item(d.pos, |it| input.accepts(it)) {
                Some((id, _)) => {
                    self.claim_item(id, d.id);
                    m.items.push(id);
                }
                None => {
                    use super::troubles::Lack;
                    self.trouble(match need {
                        MoodNeed::Log => Lack::Wood,
                        MoodNeed::Stone => Lack::Stone,
                        MoodNeed::Bar => Lack::Bars,
                        MoodNeed::Bone => Lack::Bone,
                        MoodNeed::Gem => Lack::Gem,
                    });
                    self.warn(&format!("moodneed{}", d.id), &format!("{} demands {}!", d.name, need.desc()), (255, 120, 255));
                    d.mood = Some(m);
                    d.task = Task::Idle;
                    d.idle_cd = 150;
                    return;
                }
            }
        }
        if m.delivered < m.items.len() {
            let item = m.items[m.delivered];
            match self.fetch(d, item) {
                None => {
                    // The material vanished; look for another.
                    m.items.remove(m.delivered);
                }
                Some(false) => {}
                Some(true) => match self.travel(d, site) {
                    Travel::Moving => {}
                    Travel::Failed => {
                        d.mood = Some(m);
                        self.release_task(d);
                        d.idle_cd = 100;
                        return;
                    }
                    Travel::Arrived => {
                        d.carrying = None;
                        if let Some(it) = self.items.get_mut(&item) {
                            it.carried_by = None;
                            it.pos = site;
                        }
                        m.delivered += 1;
                    }
                },
            }
            d.mood = Some(m);
            return;
        }
        if m.items.len() < m.needs.len() {
            d.mood = Some(m);
            return;
        }
        match self.travel(d, site) {
            Travel::Arrived => {}
            Travel::Moving => {
                d.mood = Some(m);
                return;
            }
            Travel::Failed => {
                d.mood = Some(m);
                self.release_task(d);
                return;
            }
        }
        if m.work_left == 0 {
            m.work_left = 1500;
            let name = d.name.clone();
            self.msg(&format!("{} has begun a mysterious construction!", name), (255, 120, 255));
        }
        m.work_left -= 1;
        if m.work_left > 0 {
            d.mood = Some(m);
            return;
        }
        // The artifact.
        let used: Vec<Item> = m.items.iter().filter_map(|id| self.items.remove(id)).collect();
        let mat = used.first().map_or(Mat::Granite, |it| it.mat);
        let kinds: &[ItemKind] = match m.labor {
            Labor::Carpentry => &[ItemKind::Bed, ItemKind::Table, ItemKind::Chair, ItemKind::Door],
            Labor::Masonry => &[ItemKind::Table, ItemKind::Chair, ItemKind::Door],
            Labor::Smithing => &[ItemKind::Axe, ItemKind::Sword, ItemKind::Hammer, ItemKind::Helm, ItemKind::Breastplate],
            _ => &[ItemKind::Craft],
        };
        let kind = *self.rng.pick(kinds);
        let id = self.spawn_item(kind, mat, site);
        let name = artifact_name(&mut self.rng);
        {
            let it = self.items.get_mut(&id).unwrap();
            it.quality = ARTIFACT;
            it.label = Some(name);
        }
        let desc = self.items[&id].name();
        if let Some(b) = self.buildings.get_mut(&m.shop.unwrap()) {
            b.worker = None;
        }
        let li = m.labor.idx();
        d.skills[li] = d.skills[li].max(4000);
        d.think("created an artifact!", -200, tick);
        d.satisfy(crate::dwarf::NeedKind::Craft, 1000);
        d.task = Task::Idle;
        let who = d.name.clone();
        self.msg(&format!("{} has created {}!", who, desc), (255, 215, 60));
        self.spotlight("An artifact", &format!("{} has created {}!", who, desc), Some(site), (255, 215, 60));
        self.year_log.artifact = self.items[&id].label.clone();
        self.milestone("artifact", &format!("The first artifact: {}", desc));
        self.earn_epithet(d, "Inspired", super::heroes::RANK_FEAT, &format!("made {}", desc));
        let fort = self.fortress_name.clone();
        self.legend(format!("{} created {} in {}.", who, desc, fort));
        self.pause_request = true;
    }
}
