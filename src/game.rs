use crate::buildings::{Building, BuildingKind, Order, Recipe};
use crate::dwarf::{self, Dwarf, Labor, Task, LABORS};
use crate::geom::{box_iter, Pos, DIRS4, DIRS8};
use crate::items::{Item, ItemKind, StockFilter, CONTAINER_CAP};
use crate::material::Mat;
use crate::path;
use crate::rng::Rng;
use crate::world::{self, Desig, Mover, TileKind, World};
use serde::{Deserialize, Serialize};
use crate::creature::Creature;
use crate::world::{DOOR_LOCKED, DOOR_OPEN};
use std::collections::{BTreeMap, HashMap, HashSet};

mod combat;
#[cfg(test)]
mod industry_tests;
mod psyche;
#[cfg(test)]
mod psyche_tests;
pub use psyche::Mandate;

pub const DAY: u64 = 1200;
pub const MONTH: u64 = DAY * 28;
pub const SEASON: u64 = MONTH * 3;
pub const YEAR: u64 = MONTH * 12;

pub const THIRSTY: u32 = 4800;
pub const HUNGRY: u32 = 9000;
pub const TIRED: u32 = 2200;
const THIRST_DEATH: u32 = 16000;
const HUNGER_DEATH: u32 = 32000;
/// Growth points to ripen; a plot gains one point every 10 ticks (~a month).
pub const RIPE: u16 = 3200;
const MOVE_DELAY: u32 = 3;
const SEARCH_LIMIT: usize = 40_000;

pub const MONTHS: [&str; 12] = [
    "Earlybloom", "Midbloom", "Latebloom", "Earlysun", "Midsun", "Latesun", "Earlyfall", "Midfall",
    "Latefall", "Earlyfrost", "Midfrost", "Latefrost",
];
pub const SEASONS: [&str; 4] = ["Spring", "Summer", "Autumn", "Winter"];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub text: String,
    pub color: (u8, u8, u8),
    pub tick: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZoneKind {
    Stockpile(StockFilter),
    Farm,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Zone {
    pub id: u32,
    pub kind: ZoneKind,
    pub tiles: Vec<Pos>,
}

enum Travel {
    Arrived,
    Moving,
    Failed,
}

#[derive(Serialize, Deserialize)]
pub struct Game {
    pub world: World,
    pub dwarves: Vec<Dwarf>,
    pub items: BTreeMap<u32, Item>,
    pub buildings: BTreeMap<u32, Building>,
    pub zones: BTreeMap<u32, Zone>,
    pub tick: u64,
    pub next_id: u32,
    pub rng: Rng,
    pub log: Vec<LogEntry>,
    pub fortress_name: String,
    pub embark: Pos,
    /// Designated tiles awaiting work.
    pub designations: HashSet<Pos>,
    /// Designations and farm tiles a dwarf has taken on.
    pub claimed: HashSet<Pos>,
    /// Stockpile tiles a hauler is bringing something to.
    pub reserved: HashSet<Pos>,
    warns: BTreeMap<String, u64>,
    pub creatures: BTreeMap<u32, Creature>,
    /// Where idle soldiers gather.
    pub station: Option<Pos>,
    /// Soldiers hunt down hostiles when set.
    pub alert: bool,
    /// Idle soldiers hunt game animals for the butcher.
    #[serde(default)]
    pub hunt: bool,
    siege_size: u32,
    siege_routed: bool,
    #[serde(default)]
    pub mandate: Option<Mandate>,
    #[serde(skip)]
    pub pause_request: bool,
    /// Bumped whenever designations or dug terrain change.
    #[serde(skip)]
    dig_version: u64,
    /// Last failed dig search per kind (mine, chop): (version, tick).
    #[serde(skip)]
    dig_fail: [Option<(u64, u64)>; 2],
    /// Cells holding water, rebuilt from the map when missing (e.g. after loading).
    #[serde(skip)]
    water_cells: Option<Vec<usize>>,
}

impl Game {
    pub fn new(seed: u64) -> Game {
        let world = world::generate(seed);
        let mut rng = Rng::new(seed.wrapping_mul(31).wrapping_add(7));
        let embark = find_embark(&world);
        let mut g = Game {
            world,
            dwarves: Vec::new(),
            items: BTreeMap::new(),
            buildings: BTreeMap::new(),
            zones: BTreeMap::new(),
            tick: 0,
            next_id: 1,
            fortress_name: dwarf::fortress_name(&mut rng),
            rng,
            log: Vec::new(),
            embark,
            designations: HashSet::new(),
            claimed: HashSet::new(),
            reserved: HashSet::new(),
            warns: BTreeMap::new(),
            pause_request: false,
            water_cells: None,
            creatures: BTreeMap::new(),
            station: None,
            alert: true,
            hunt: false,
            siege_size: 0,
            siege_routed: false,
            mandate: None,
            dig_version: 0,
            dig_fail: [None, None],
        };
        let crews: [&[Labor]; 7] = [
            &[Labor::Mining, Labor::Crafting],
            &[Labor::Mining, Labor::Smelting],
            &[Labor::Woodcutting, Labor::Carpentry],
            &[Labor::Carpentry, Labor::Butchery],
            &[Labor::Masonry, Labor::Smithing],
            &[Labor::Farming, Labor::Cooking],
            &[Labor::Brewing, Labor::Farming],
        ];
        for crew in crews {
            let pos = g.random_spot_near(embark, 3);
            let id = g.new_id();
            let mut d = dwarf::new_dwarf(id, pos, &mut g.rng, crew);
            // Miners start with a pick, woodcutters with an axe.
            let tool = match crew[0] {
                Labor::Mining => Some(ItemKind::Pick),
                Labor::Woodcutting => Some(ItemKind::Axe),
                _ => None,
            };
            if let Some(k) = tool {
                let iid = g.spawn_item(k, Mat::Copper, pos);
                g.items.get_mut(&iid).unwrap().carried_by = Some(id);
                d.held = Some(iid);
            }
            g.dwarves.push(d);
        }
        g.populate_caverns();
        g.appoint_leader();
        let supplies = [
            (ItemKind::Mushroom, Mat::Stoutcap, 30),
            (ItemKind::Drink, Mat::Stoutcap, 40),
            (ItemKind::Log, Mat::Wood, 6),
            (ItemKind::Axe, Mat::Copper, 1),
            (ItemKind::Pick, Mat::Copper, 1),
            (ItemKind::Seed, Mat::Stoutcap, 15),
            (ItemKind::Barrel, Mat::Wood, 2),
        ];
        for (kind, mat, n) in supplies {
            for _ in 0..n {
                let pos = g.random_spot_near(embark, 2);
                g.spawn_item(kind, mat, pos);
            }
        }
        let name = g.fortress_name.clone();
        g.msg(
            &format!("Seven dwarves have arrived to found the outpost of {}. Strike the earth!", name),
            (255, 255, 120),
        );
        g
    }

    fn new_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn msg(&mut self, text: &str, color: (u8, u8, u8)) {
        self.log.push(LogEntry { text: text.to_string(), color, tick: self.tick });
        if self.log.len() > 500 {
            self.log.remove(0);
        }
    }

    /// Announce at most once per day for a given key.
    fn warn(&mut self, key: &str, text: &str, color: (u8, u8, u8)) {
        let last = self.warns.get(key).copied();
        if last.map_or(true, |t| self.tick >= t + DAY) {
            self.warns.insert(key.to_string(), self.tick);
            self.msg(text, color);
        }
    }

    fn random_spot_near(&mut self, c: Pos, r: i32) -> Pos {
        for _ in 0..200 {
            let x = c.x + self.rng.range(-r, r + 1);
            let y = c.y + self.rng.range(-r, r + 1);
            if x < 0 || y < 0 || x >= self.world.w || y >= self.world.h {
                continue;
            }
            let p = Pos::new(x, y, self.world.surface_z(x, y));
            if self.world.walkable(p) && self.world.t(p).water == 0 {
                return p;
            }
        }
        c
    }

    pub fn spawn_item(&mut self, kind: ItemKind, mat: Mat, pos: Pos) -> u32 {
        let id = self.new_id();
        let pos = self.world.settle(pos);
        self.items.insert(
            id,
            Item { id, kind, mat, pos, carried_by: None, claimed_by: None, label: None, container: None, quality: 0 },
        );
        id
    }

    pub fn date_string(&self) -> String {
        let year = self.tick / YEAR + 1;
        let month = ((self.tick % YEAR) / MONTH) as usize;
        let day = (self.tick % MONTH) / DAY + 1;
        format!("{} {}, Year {} ({})", day_ordinal(day), MONTHS[month], year, SEASONS[month / 3])
    }

    pub fn alive_count(&self) -> usize {
        self.dwarves.iter().filter(|d| d.alive).count()
    }

    // ------------------------------------------------------------------
    // Player commands
    // ------------------------------------------------------------------

    pub fn desig_valid(&self, p: Pos, d: Desig) -> bool {
        let t = match self.world.get(p) {
            Some(t) => *t,
            None => return false,
        };
        let wall = t.kind == TileKind::Wall && t.mat.diggable();
        match d {
            Desig::None => true,
            Desig::Dig | Desig::StairUp | Desig::Ramp => wall,
            Desig::StairDown => {
                wall || matches!(t.kind, TileKind::Floor | TileKind::Ramp | TileKind::StairUp | TileKind::Shrub)
            }
            Desig::StairUD => {
                wall || matches!(
                    t.kind,
                    TileKind::Floor | TileKind::Ramp | TileKind::StairUp | TileKind::StairDown | TileKind::Shrub
                )
            }
            Desig::Channel => {
                p.z > 0 && t.mat.diggable() && !matches!(t.kind, TileKind::Open | TileKind::Tree)
            }
            Desig::Chop => t.kind == TileKind::Tree,
        }
    }

    /// Designate (or with `Desig::None`, clear) every tile in a box. Returns how many changed.
    pub fn designate(&mut self, a: Pos, b: Pos, d: Desig) -> usize {
        self.dig_version += 1;
        let mut n = 0;
        for p in box_iter(a, b) {
            if !self.world.in_bounds(p) {
                continue;
            }
            let t = *self.world.t(p);
            if d == Desig::None {
                if t.desig != Desig::None {
                    self.world.t_mut(p).desig = Desig::None;
                    self.designations.remove(&p);
                    n += 1;
                }
                continue;
            }
            // Hidden tiles can be designated blind, like the real thing.
            let hidden_ok = !t.revealed && d != Desig::Chop;
            if hidden_ok || self.desig_valid(p, d) {
                self.world.t_mut(p).desig = d;
                self.designations.insert(p);
                n += 1;
            }
        }
        n
    }

    pub fn add_zone(&mut self, a: Pos, b: Pos, kind: ZoneKind) -> usize {
        let mut tiles = Vec::new();
        for p in box_iter(a, b) {
            let Some(t) = self.world.get(p) else { continue };
            if !t.revealed || t.zone != 0 || t.building != 0 || !matches!(t.kind, TileKind::Floor | TileKind::Shrub) {
                continue;
            }
            if kind == ZoneKind::Farm && !t.mat.is_soil() {
                continue;
            }
            tiles.push(p);
        }
        if tiles.is_empty() {
            return 0;
        }
        let id = self.new_id();
        for &p in &tiles {
            let t = self.world.t_mut(p);
            t.zone = id;
            t.farm = 0;
            if kind == ZoneKind::Farm {
                t.kind = TileKind::Floor;
            }
        }
        let n = tiles.len();
        self.zones.insert(id, Zone { id, kind, tiles });
        n
    }

    pub fn remove_zones(&mut self, a: Pos, b: Pos) -> usize {
        let mut n = 0;
        for p in box_iter(a, b) {
            let Some(t) = self.world.get(p) else { continue };
            let zid = t.zone;
            if zid == 0 {
                continue;
            }
            let t = self.world.t_mut(p);
            t.zone = 0;
            t.farm = 0;
            if let Some(z) = self.zones.get_mut(&zid) {
                z.tiles.retain(|&q| q != p);
                if z.tiles.is_empty() {
                    self.zones.remove(&zid);
                }
            }
            n += 1;
        }
        n
    }

    pub fn can_place(&self, kind: BuildingKind, top_left: Pos) -> bool {
        let s = kind.size();
        for dy in 0..s {
            for dx in 0..s {
                let p = top_left.offset(dx, dy, 0);
                let Some(t) = self.world.get(p) else { return false };
                if !t.revealed || t.building != 0 || t.water > 0 || !matches!(t.kind, TileKind::Floor | TileKind::Shrub) {
                    return false;
                }
            }
        }
        true
    }

    pub fn place_building(&mut self, kind: BuildingKind, top_left: Pos) -> bool {
        if !self.can_place(kind, top_left) {
            return false;
        }
        let id = self.new_id();
        let b = Building {
            id,
            kind,
            pos: top_left,
            built: false,
            mat: Mat::Air,
            orders: Vec::new(),
            worker: None,
            occupant: None,
            retry_tick: 0,
            locked: false,
            owner: None,
            quality: 0,
            room_value: 0,
        };
        for p in b.tiles() {
            let t = self.world.t_mut(p);
            t.building = id;
            if t.kind == TileKind::Shrub {
                t.kind = TileKind::Floor;
            }
        }
        self.buildings.insert(id, b);
        true
    }

    pub fn remove_building(&mut self, id: u32) {
        let Some(b) = self.buildings.get(&id).cloned() else { return };
        for i in 0..self.dwarves.len() {
            let uses = match &self.dwarves[i].task {
                Task::Construct { bld, .. } | Task::Craft { bld, .. } => *bld == id,
                Task::Sleep { bed: Some(bed), .. } | Task::Rest { bed: Some(bed), .. } => *bed == id,
                _ => false,
            };
            if uses {
                let mut d = std::mem::take(&mut self.dwarves[i]);
                self.release_task(&mut d);
                self.dwarves[i] = d;
            }
        }
        for p in b.tiles() {
            let t = self.world.t_mut(p);
            t.building = 0;
            t.door = world::DOOR_NONE;
        }
        self.buildings.remove(&id);
        if b.built {
            let kind = match b.kind {
                BuildingKind::Carpenter | BuildingKind::Butcher => ItemKind::Log,
                BuildingKind::Bed => ItemKind::Bed,
                BuildingKind::Table => ItemKind::Table,
                BuildingKind::Chair => ItemKind::Chair,
                BuildingKind::Door => ItemKind::Door,
                _ => ItemKind::Boulder,
            };
            let id = self.spawn_item(kind, b.mat, b.center());
            self.items.get_mut(&id).unwrap().quality = b.quality;
        }
    }

    pub fn add_order(&mut self, bld: u32, recipe: Recipe) {
        if let Some(b) = self.buildings.get_mut(&bld) {
            b.orders.push(Order { recipe, repeat: false });
        }
    }

    pub fn toggle_lock(&mut self, bld: u32) {
        let Some(b) = self.buildings.get_mut(&bld) else { return };
        if b.kind != BuildingKind::Door || !b.built {
            return;
        }
        b.locked = !b.locked;
        let state = if b.locked { DOOR_LOCKED } else { DOOR_OPEN };
        let pos = b.pos;
        self.world.t_mut(pos).door = state;
    }

    pub fn toggle_soldier(&mut self, dwarf_id: u32) {
        if let Some(i) = self.dwarves.iter().position(|d| d.id == dwarf_id) {
            let mut d = std::mem::take(&mut self.dwarves[i]);
            d.soldier = !d.soldier;
            if !d.task.is_need() {
                self.release_task(&mut d);
            }
            self.dwarves[i] = d;
        }
    }

    pub fn toggle_labor(&mut self, dwarf_id: u32, labor: Labor) {
        if let Some(i) = self.dwarves.iter().position(|d| d.id == dwarf_id) {
            let on = !self.dwarves[i].labors[labor.idx()];
            self.dwarves[i].labors[labor.idx()] = on;
            if !on {
                let mut d = std::mem::take(&mut self.dwarves[i]);
                if !d.task.is_need() {
                    self.release_task(&mut d);
                }
                self.dwarves[i] = d;
            }
        }
    }

    // ------------------------------------------------------------------
    // Simulation
    // ------------------------------------------------------------------

    pub fn step(&mut self) {
        self.tick += 1;
        if self.tick % 3 == 0 {
            self.fluids();
        }
        if self.tick % 10 == 0 {
            self.grow_farms();
        }
        for i in 0..self.dwarves.len() {
            if self.dwarves[i].alive {
                let mut d = std::mem::take(&mut self.dwarves[i]);
                self.update_dwarf(&mut d);
                self.dwarves[i] = d;
            }
        }
        self.update_creatures();
        if self.tick % DAY == DAY / 2 {
            self.wildlife();
        }
        if self.tick % DAY == 0 {
            self.assign_rooms();
            self.nobles_daily();
            self.check_mandate();
        }
        if self.tick % SEASON == 0 {
            let season = (self.tick / SEASON) % 4;
            let s = SEASONS[season as usize];
            self.msg(&format!("{} has arrived.", s), (200, 200, 255));
            if season == 1 || season == 2 {
                self.migrants();
            }
            self.maybe_siege();
            self.issue_mandate();
            self.maybe_strange_mood();
        }
    }

    fn fluids(&mut self) {
        let mut cells = match self.water_cells.take() {
            Some(c) => c,
            None => self
                .world
                .tiles
                .iter()
                .enumerate()
                .filter(|(_, t)| t.water > 0)
                .map(|(i, _)| i)
                .collect(),
        };
        let mut wet: HashSet<usize> = HashSet::with_capacity(cells.len() + 16);
        for i in (1..cells.len()).rev() {
            let j = self.rng.range(0, i as i32 + 1) as usize;
            cells.swap(i, j);
        }
        for i in cells {
            let p = self.world.pos_of(i);
            let t = self.world.tiles[i];
            if t.water == 0 {
                continue;
            }
            // Fall through anything without a floor.
            if matches!(t.kind, TileKind::Open | TileKind::StairDown | TileKind::StairUD) {
                let b = p.down();
                if self.world.in_bounds(b) && !self.world.solid(b) {
                    let room = 7 - self.world.t(b).water;
                    if room > 0 {
                        let amt = room.min(t.water);
                        self.world.t_mut(b).water += amt;
                        self.world.tiles[i].water -= amt;
                        wet.insert(self.world.idx(b));
                        if self.world.tiles[i].water > 0 {
                            wet.insert(i);
                        }
                        continue;
                    }
                }
            }
            // Level out sideways.
            let (dx, dy) = DIRS4[self.rng.range(0, 4) as usize];
            let q = p.offset(dx, dy, 0);
            if !self.world.solid(q) {
                let qw = self.world.t(q).water;
                let here = self.world.tiles[i].water;
                if qw + 1 < here || (here == 1 && qw == 0 && self.rng.chance(0.1)) {
                    self.world.t_mut(q).water += 1;
                    self.world.tiles[i].water -= 1;
                    wet.insert(self.world.idx(q));
                }
            }
            let here = self.world.tiles[i].water;
            if here == 1 {
                let rate = if self.world.outside(p) { 0.01 } else { 0.001 };
                if self.rng.chance(rate) {
                    self.world.tiles[i].water = 0;
                }
            }
            if self.world.tiles[i].water > 0 {
                wet.insert(i);
            }
        }
        let mut next: Vec<usize> = wet.into_iter().collect();
        next.sort_unstable();
        self.water_cells = Some(next);
    }

    fn grow_farms(&mut self) {
        let tiles: Vec<Pos> = self
            .zones
            .values()
            .filter(|z| z.kind == ZoneKind::Farm)
            .flat_map(|z| z.tiles.iter().copied())
            .collect();
        for p in tiles {
            let t = self.world.t_mut(p);
            if t.farm > 0 && t.farm < RIPE {
                t.farm = (t.farm + 1).min(RIPE);
            }
        }
    }

    fn migrants(&mut self) {
        let n = self.rng.range(3, 7);
        let w = self.world.w;
        let h = self.world.h;
        let mut entry = None;
        for _ in 0..200 {
            let (x, y) = match self.rng.range(0, 4) {
                0 => (0, self.rng.range(0, h)),
                1 => (w - 1, self.rng.range(0, h)),
                2 => (self.rng.range(0, w), 0),
                _ => (self.rng.range(0, w), h - 1),
            };
            let p = Pos::new(x, y, self.world.surface_z(x, y));
            if self.world.walkable(p) && self.world.t(p).water == 0 {
                entry = Some(p);
                break;
            }
        }
        let entry = entry.unwrap_or(self.embark);
        let pool = [
            Labor::Mining,
            Labor::Woodcutting,
            Labor::Carpentry,
            Labor::Masonry,
            Labor::Farming,
            Labor::Brewing,
            Labor::Cooking,
            Labor::Butchery,
            Labor::Smelting,
            Labor::Smithing,
            Labor::Crafting,
        ];
        for _ in 0..n {
            let id = self.new_id();
            let spec = *self.rng.pick(&pool);
            let specs: Vec<Labor> = if self.rng.chance(0.3) { vec![] } else { vec![spec] };
            let pos = self.random_spot_near(entry, 2);
            let d = dwarf::new_dwarf(id, pos, &mut self.rng, &specs);
            self.dwarves.push(d);
        }
        self.msg(&format!("Some migrants have arrived. ({} dwarves)", n), (120, 255, 120));
        self.pause_request = true;
    }

    fn update_dwarf(&mut self, d: &mut Dwarf) {
        d.thirst += 1;
        d.hunger += 1;
        if !d.asleep {
            d.fatigue += 1;
        }
        if d.thirst > THIRST_DEATH {
            self.kill(d, "has died of thirst");
            return;
        }
        if d.hunger > HUNGER_DEATH {
            self.kill(d, "has starved to death");
            return;
        }

        // Falling and drowning.
        if !self.world.walkable(d.pos) && !self.world.solid(d.pos) {
            let t = self.world.t(d.pos);
            if !t.kind.has_floor() && !self.world.solid(d.pos.down()) {
                d.pos = d.pos.down();
                d.path.clear();
                self.carry_follow(d);
                return;
            }
        }
        if self.tick % 10 == 0 {
            d.body.tick();
            if d.body.dead {
                self.kill(d, "has bled to death");
                return;
            }
        }
        if self.tick % 200 == 0 {
            let in_bed = d.asleep
                && matches!(d.task, Task::Rest { bed: Some(_), .. } | Task::Sleep { bed: Some(_), .. });
            d.body.heal(if in_bed { 3 } else { 1 });
        }
        if d.body.unconscious() {
            if d.task != Task::Idle {
                self.release_task(d);
            }
            self.carry_follow(d);
            return;
        }
        d.attack_cd = d.attack_cd.saturating_sub(1);
        if self.tick % 5 == 0 {
            self.combat_awareness(d);
        }

        if self.world.t(d.pos).water >= 6 {
            d.drown += 1;
            if d.drown > 400 {
                self.kill(d, "has drowned");
                return;
            }
        } else {
            d.drown = 0;
        }

        if self.tick % 100 == 0 {
            if d.thirst > THIRSTY * 2 || d.hunger > HUNGRY * 2 {
                d.stress += 4;
            }
            d.stress -= d.stress.signum();
        }

        self.psyche_tick(d);
        if d.insane || !d.alive {
            if d.task == Task::Idle && self.rng.chance(0.02) {
                self.start_wander(d);
            }
            if let Task::Wander { target } = d.task {
                if !matches!(self.travel(d, target), Travel::Moving) {
                    d.task = Task::Idle;
                }
            }
            return;
        }
        if d.tantrum > 0 {
            d.tantrum -= 1;
        } else if d.stress >= d.tantrum_threshold() {
            self.release_task(d);
            d.tantrum = 1500;
            // Venting helps a little; a truly miserable dwarf keeps sliding.
            d.stress -= 120;
            let name = d.name.clone();
            self.msg(&format!("{} is throwing a tantrum!", name), (255, 90, 90));
        }

        // Pressing needs interrupt ordinary work.
        if !d.task.is_need() && d.task != Task::Idle {
            if d.thirst > THIRSTY + 2000 || d.hunger > HUNGRY + 4000 || d.fatigue > TIRED + 1500 {
                self.release_task(d);
            }
        }

        if d.task == Task::Idle {
            if d.idle_cd > 0 {
                d.idle_cd -= 1;
            } else {
                self.choose_task(d);
                if d.task == Task::Idle {
                    d.idle_cd = 20 + self.rng.range(0, 20) as u32;
                    if self.rng.chance(0.25) {
                        self.start_wander(d);
                    }
                }
            }
        }
        if d.task != Task::Idle {
            self.do_task(d);
        }
        self.carry_follow(d);
    }

    fn carry_follow(&mut self, d: &Dwarf) {
        for id in d.carrying.iter().chain(d.held.iter()).chain(d.armor.iter()) {
            if let Some(it) = self.items.get_mut(id) {
                it.pos = d.pos;
            }
        }
    }

    fn kill(&mut self, d: &mut Dwarf, cause: &str) {
        self.release_task(d);
        d.alive = false;
        let pos = d.pos;
        for id in d.held.take().into_iter().chain(d.armor.drain(..)) {
            self.drop_item(id, pos);
        }
        let id = self.spawn_item(ItemKind::Corpse, Mat::Flesh, pos);
        if let Some(it) = self.items.get_mut(&id) {
            it.label = Some(d.name.clone());
        }
        let name = d.name.clone();
        self.msg(&format!("{} {}.", name, cause), (255, 60, 60));
        let tick = self.tick;
        let dead = d.id;
        for o in self.dwarves.iter_mut().filter(|o| o.alive) {
            let regard = o.regard(dead);
            if regard >= 30 {
                o.think(&format!("lost their dear friend {}", name), 90, tick);
            } else if regard <= -40 {
                o.think(&format!("felt satisfied at the death of {}", name), -10, tick);
            } else {
                o.think(&format!("grieved over the death of {}", name), 25, tick);
            }
        }
        self.end_mood(d);
        d.noble = None;
        self.pause_request = true;
    }

    /// Abandon the current task, dropping anything carried and freeing every claim.
    fn release_task(&mut self, d: &mut Dwarf) {
        match d.task.clone() {
            Task::Dig { target, .. } => {
                self.claimed.remove(&target);
            }
            Task::Haul { item, dest, .. } => {
                self.unclaim(item);
                self.reserved.remove(&dest);
            }
            Task::Construct { bld, item } => {
                self.unclaim(item);
                if let Some(b) = self.buildings.get_mut(&bld) {
                    b.worker = None;
                }
            }
            Task::Craft { bld, items, .. } => {
                for item in items {
                    self.unclaim(item);
                }
                if let Some(b) = self.buildings.get_mut(&bld) {
                    b.worker = None;
                }
            }
            Task::Plant { pos, seed } => {
                self.claimed.remove(&pos);
                self.unclaim(seed);
            }
            Task::Harvest { pos } => {
                self.claimed.remove(&pos);
            }
            Task::Eat { item } | Task::DrinkItem { item } | Task::Equip { item } => self.unclaim(item),
            Task::Dine { item, seat } => {
                self.unclaim(item);
                if let Some(b) = self.buildings.get_mut(&seat) {
                    b.occupant = None;
                }
            }
            Task::Sleep { bed: Some(bed), .. } | Task::Rest { bed: Some(bed), .. } => {
                if let Some(b) = self.buildings.get_mut(&bed) {
                    b.occupant = None;
                }
            }
            _ => {}
        }
        if let Some(id) = d.carrying.take() {
            let pos = self.world.settle(d.pos);
            // A moody dwarf keeps hold of the materials they've claimed.
            let keep = d.mood.as_ref().map_or(false, |m| m.items.contains(&id));
            if let Some(it) = self.items.get_mut(&id) {
                it.carried_by = None;
                if !keep {
                    it.claimed_by = None;
                }
                it.pos = pos;
            }
        }
        d.task = Task::Idle;
        d.path.clear();
        d.working = false;
        d.asleep = false;
    }

    fn unclaim(&mut self, item: u32) {
        if let Some(it) = self.items.get_mut(&item) {
            it.claimed_by = None;
        }
    }

    fn travel(&self, d: &mut Dwarf, dest: Pos) -> Travel {
        if d.pos == dest {
            return Travel::Arrived;
        }
        if d.move_cd > 0 {
            d.move_cd -= 1;
            return Travel::Moving;
        }
        if d.path.first() != Some(&dest) {
            match path::astar(&self.world, d.pos, dest, SEARCH_LIMIT, Mover::Dwarf) {
                Some(mut p) => {
                    p.reverse();
                    d.path = p;
                }
                None => return Travel::Failed,
            }
        }
        let Some(next) = d.path.pop() else { return Travel::Failed };
        let mut nb = Vec::new();
        self.world.neighbors(d.pos, &mut nb, Mover::Dwarf);
        if !nb.iter().any(|&(q, _)| q == next) {
            // The world changed under us; replan next tick.
            d.path.clear();
            return Travel::Moving;
        }
        d.pos = next;
        d.move_cd = MOVE_DELAY + if d.carrying.is_some() { 1 } else { 0 } + d.body.limp();
        if d.pos == dest {
            Travel::Arrived
        } else {
            Travel::Moving
        }
    }

    /// Walk to an item and pick it up. Some(true) = holding it.
    fn fetch(&mut self, d: &mut Dwarf, item: u32) -> Option<bool> {
        if d.carrying == Some(item) {
            return Some(true);
        }
        let ipos = match self.items.get(&item) {
            Some(it) if it.carried_by.is_none() => it.pos,
            _ => return None,
        };
        match self.travel(d, ipos) {
            Travel::Arrived => {
                if let Some(it) = self.items.get_mut(&item) {
                    it.carried_by = Some(d.id);
                    it.container = None;
                }
                d.carrying = Some(item);
                Some(true)
            }
            Travel::Moving => Some(false),
            Travel::Failed => None,
        }
    }

    fn work_time(base: u32, level: u32) -> u32 {
        (base * 10 / (10 + level * 2)).max(5)
    }

    /// Count down work at the current spot. Returns true when finished.
    fn labor(d: &mut Dwarf, base: u32, level: u32) -> bool {
        if !d.working {
            d.working = true;
            let diligent = (100 - d.personality.diligence / 3) as u32;
            d.work = Self::work_time(base * diligent / 100, level);
        }
        if d.work > 0 {
            d.work -= 1;
        }
        if d.work == 0 {
            d.working = false;
            true
        } else {
            false
        }
    }

    fn start_wander(&mut self, d: &mut Dwarf) {
        for _ in 0..10 {
            let q = d.pos.offset(self.rng.range(-5, 6), self.rng.range(-5, 6), 0);
            if self.world.walkable(q) && q != d.pos {
                d.task = Task::Wander { target: q };
                return;
            }
        }
    }

    // ---------------------------- job selection ----------------------------

    fn choose_task(&mut self, d: &mut Dwarf) {
        if d.thirst > THIRSTY && self.find_drink(d) {
            return;
        }
        if d.hunger > HUNGRY && self.find_food(d) {
            return;
        }
        if d.body.needs_rest() {
            self.start_bed(d, true);
            return;
        }
        if d.fatigue > TIRED {
            self.start_bed(d, false);
            return;
        }
        if d.tantrum > 0 {
            self.start_wander(d);
            return;
        }
        if d.mood.is_some() {
            d.task = Task::Mood;
            return;
        }
        if d.soldier {
            self.choose_military(d);
            return;
        }
        if d.has(Labor::Building) && self.find_construct(d) {
            return;
        }
        if self.find_craft(d) {
            return;
        }
        if d.has(Labor::Mining) && self.find_dig(d, false) {
            return;
        }
        if d.has(Labor::Woodcutting) && self.find_dig(d, true) {
            return;
        }
        if d.has(Labor::Farming) && self.find_farm(d) {
            return;
        }
        if d.has(Labor::Hauling) {
            self.find_haul(d);
        }
    }

    fn loose_items<F: Fn(&Item) -> bool>(&self, f: F) -> HashMap<Pos, u32> {
        let mut at = HashMap::new();
        for it in self.items.values() {
            if it.carried_by.is_none() && it.claimed_by.is_none() && f(it) {
                at.entry(it.pos).or_insert(it.id);
            }
        }
        at
    }

    fn find_item<F: Fn(&Item) -> bool>(&self, from: Pos, f: F) -> Option<(u32, Vec<Pos>)> {
        let at = self.loose_items(f);
        if at.is_empty() {
            return None;
        }
        let (p, path) = path::search(&self.world, from, SEARCH_LIMIT, Mover::Dwarf, |p| at.contains_key(&p))?;
        Some((at[&p], path))
    }

    fn claim_item(&mut self, item: u32, by: u32) {
        if let Some(it) = self.items.get_mut(&item) {
            it.claimed_by = Some(by);
        }
    }

    fn set_path(d: &mut Dwarf, mut path: Vec<Pos>) {
        path.reverse();
        d.path = path;
    }

    fn find_drink(&mut self, d: &mut Dwarf) -> bool {
        if let Some((item, path)) = self.find_item(d.pos, |it| it.kind == ItemKind::Drink) {
            self.claim_item(item, d.id);
            d.task = Task::DrinkItem { item };
            Self::set_path(d, path);
            return true;
        }
        let w = &self.world;
        let water_near = |p: Pos| {
            DIRS8.iter().any(|&(dx, dy)| {
                let q = p.offset(dx, dy, 0);
                let below = q.down();
                w.get(q).map_or(false, |t| t.water > 0)
                    || (w.kind(q) == Some(TileKind::Open) && w.get(below).map_or(false, |t| t.water > 0))
            })
        };
        if let Some((stand, path)) = path::search(w, d.pos, SEARCH_LIMIT, Mover::Dwarf, |p| water_near(p)) {
            d.task = Task::DrinkWater { stand };
            Self::set_path(d, path);
            return true;
        }
        self.warn("nodrink", "Your dwarves have nothing to drink!", (255, 120, 60));
        false
    }

    fn find_food(&mut self, d: &mut Dwarf) -> bool {
        let meal = self.find_item(d.pos, |it| it.kind == ItemKind::Meal);
        if let Some((item, path)) = meal.or_else(|| self.find_item(d.pos, |it| it.kind.is_food())) {
            self.claim_item(item, d.id);
            let seats = self.free_seats();
            let ipos = self.items[&item].pos;
            let seat = if seats.is_empty() {
                None
            } else {
                path::search(&self.world, ipos, SEARCH_LIMIT, Mover::Dwarf, |p| seats.contains_key(&p)).map(|(p, _)| seats[&p])
            };
            match seat {
                Some(seat) => {
                    self.buildings.get_mut(&seat).unwrap().occupant = Some(d.id);
                    d.task = Task::Dine { item, seat };
                }
                None => d.task = Task::Eat { item },
            }
            Self::set_path(d, path);
            return true;
        }
        self.warn("nofood", "Your dwarves have nothing to eat!", (255, 120, 60));
        false
    }

    fn choose_military(&mut self, d: &mut Dwarf) {
        if self.equip_soldier(d) {
            return;
        }
        if self.alert {
            let at: HashMap<Pos, u32> = self
                .creatures
                .values()
                .filter(|c| (c.hostile() || self.hunt) && !c.leaving && self.creature_active(c))
                .map(|c| (c.pos, c.id))
                .collect();
            if !at.is_empty() {
                let found = path::search(&self.world, d.pos, SEARCH_LIMIT, Mover::Dwarf, |q| {
                    at.contains_key(&q) || DIRS8.iter().any(|&(dx, dy)| at.contains_key(&q.offset(dx, dy, 0)))
                });
                if let Some((p, _)) = found {
                    let target = at.get(&p).copied().or_else(|| {
                        DIRS8.iter().find_map(|&(dx, dy)| at.get(&p.offset(dx, dy, 0)).copied())
                    });
                    if let Some(target) = target {
                        d.task = Task::Attack { target };
                        return;
                    }
                }
            }
        }
        if let Some(st) = self.station {
            if (st.x - d.pos.x).abs() > 3 || (st.y - d.pos.y).abs() > 3 || st.z != d.pos.z {
                d.task = Task::Station { pos: st };
                return;
            }
        }
        d.task = Task::Train;
    }

    fn start_bed(&mut self, d: &mut Dwarf, rest: bool) {
        let task = |bed, stand| if rest { Task::Rest { bed, stand } } else { Task::Sleep { bed, stand } };
        let own = self
            .buildings
            .values()
            .find(|b| b.kind == BuildingKind::Bed && b.built && b.occupant.is_none() && b.owner == Some(d.id))
            .map(|b| (b.pos, b.id));
        let beds: HashMap<Pos, u32> = match own {
            Some((p, id)) => [(p, id)].into_iter().collect(),
            None => self
                .buildings
                .values()
                .filter(|b| b.kind == BuildingKind::Bed && b.built && b.occupant.is_none() && b.owner.is_none())
                .map(|b| (b.pos, b.id))
                .collect(),
        };
        if !beds.is_empty() {
            if let Some((p, path)) = path::search(&self.world, d.pos, SEARCH_LIMIT, Mover::Dwarf, |p| beds.contains_key(&p)) {
                let bed = beds[&p];
                if let Some(b) = self.buildings.get_mut(&bed) {
                    b.occupant = Some(d.id);
                }
                d.task = task(Some(bed), p);
                Self::set_path(d, path);
                return;
            }
        }
        d.task = task(None, d.pos);
    }

    fn find_construct(&mut self, d: &mut Dwarf) -> bool {
        let pending: Vec<u32> = self
            .buildings
            .values()
            .filter(|b| !b.built && b.worker.is_none() && b.retry_tick <= self.tick)
            .map(|b| b.id)
            .collect();
        for bid in pending {
            let kind = self.buildings[&bid].kind;
            if let Some((item, path)) = self.find_item(d.pos, |it| kind.needs(it)) {
                self.claim_item(item, d.id);
                self.buildings.get_mut(&bid).unwrap().worker = Some(d.id);
                d.task = Task::Construct { bld: bid, item };
                Self::set_path(d, path);
                return true;
            }
            self.buildings.get_mut(&bid).unwrap().retry_tick = self.tick + 300;
            self.warn(
                &format!("need{}", bid),
                &format!("{} construction needs {}.", kind.name(), kind.needs_desc()),
                (230, 200, 90),
            );
        }
        false
    }

    fn find_craft(&mut self, d: &mut Dwarf) -> bool {
        let shops: Vec<u32> = self
            .buildings
            .values()
            .filter(|b| b.built && b.worker.is_none() && !b.orders.is_empty() && b.retry_tick <= self.tick)
            .filter(|b| d.has(b.orders[0].recipe.labor()))
            .map(|b| b.id)
            .collect();
        for bid in shops {
            let recipe = self.buildings[&bid].orders[0].recipe;
            let site = self.buildings[&bid].center();
            let mut chosen: Vec<u32> = Vec::new();
            for input in recipe.inputs() {
                match self.find_item(site, |it| input.accepts(it) && !chosen.contains(&it.id)) {
                    Some((id, _)) => chosen.push(id),
                    None => break,
                }
            }
            if chosen.len() == recipe.inputs().len() {
                for &id in &chosen {
                    self.claim_item(id, d.id);
                }
                self.buildings.get_mut(&bid).unwrap().worker = Some(d.id);
                d.task = Task::Craft { bld: bid, items: chosen, delivered: 0 };
                return true;
            }
            let shop_name = self.buildings[&bid].kind.name();
            self.buildings.get_mut(&bid).unwrap().retry_tick = self.tick + 300;
            self.warn(
                &format!("craft{}", bid),
                &format!("{}: \"{}\" needs {}.", shop_name, recipe.name(), recipe.input_desc()),
                (230, 200, 90),
            );
        }
        false
    }

    /// Cells a dwarf standing at `p` can work on.
    fn work_targets(&self, p: Pos) -> Vec<Pos> {
        let mut v: Vec<Pos> = DIRS8.iter().map(|&(dx, dy)| p.offset(dx, dy, 0)).collect();
        match self.world.kind(p) {
            Some(TileKind::StairDown) => v.push(p.down()),
            Some(TileKind::StairUp) => v.push(p.up()),
            Some(TileKind::StairUD) => {
                v.push(p.down());
                v.push(p.up());
            }
            Some(TileKind::Ramp) => v.extend(DIRS8.iter().map(|&(dx, dy)| p.offset(dx, dy, 1))),
            _ => {}
        }
        v
    }

    fn find_dig(&mut self, d: &mut Dwarf, chop: bool) -> bool {
        let wanted = |ds: Desig| if chop { ds == Desig::Chop } else { ds != Desig::Chop && ds != Desig::None };
        let open: HashSet<Pos> = self
            .designations
            .iter()
            .copied()
            .filter(|p| !self.claimed.contains(p) && wanted(self.world.t(*p).desig))
            .collect();
        if open.is_empty() {
            return false;
        }
        let kind = chop as usize;
        if let Some((v, t)) = self.dig_fail[kind] {
            if v == self.dig_version && self.tick < t + 600 {
                return false;
            }
        }
        if let Some(started) = self.need_tool(d, if chop { ItemKind::Axe } else { ItemKind::Pick }) {
            return started;
        }
        let mut target = None;
        let found = path::search(&self.world, d.pos, SEARCH_LIMIT, Mover::Dwarf, |p| {
            for q in self.work_targets(p) {
                if open.contains(&q) && self.desig_valid(q, self.world.t(q).desig) {
                    target = Some(q);
                    return true;
                }
            }
            false
        });
        if let (Some((stand, path)), Some(target)) = (found, target) {
            self.claimed.insert(target);
            d.task = Task::Dig { target, stand };
            Self::set_path(d, path);
            return true;
        }
        self.dig_fail[kind] = Some((self.dig_version, self.tick));
        false
    }

    fn find_farm(&mut self, d: &mut Dwarf) -> bool {
        let mut plots: HashMap<Pos, bool> = HashMap::new(); // pos -> is harvest
        for z in self.zones.values().filter(|z| z.kind == ZoneKind::Farm) {
            for &p in &z.tiles {
                if self.claimed.contains(&p) {
                    continue;
                }
                let f = self.world.t(p).farm;
                if f == 0 {
                    plots.insert(p, false);
                } else if f >= RIPE {
                    plots.insert(p, true);
                }
            }
        }
        if plots.is_empty() {
            return false;
        }
        // Planting needs seed; without any, only ripe plots are worth visiting.
        let seed = self.find_item(d.pos, |it| it.kind == ItemKind::Seed).map(|(id, _)| id);
        if seed.is_none() {
            plots.retain(|_, harvest| *harvest);
            if plots.is_empty() {
                self.warn("noseed", "Farmers have no stoutcap spawn to plant.", (230, 200, 90));
                return false;
            }
        }
        if let Some((p, path)) = path::search(&self.world, d.pos, SEARCH_LIMIT, Mover::Dwarf, |p| plots.contains_key(&p)) {
            self.claimed.insert(p);
            if plots[&p] {
                d.task = Task::Harvest { pos: p };
                Self::set_path(d, path);
            } else {
                let seed = seed.unwrap();
                self.claim_item(seed, d.id);
                d.task = Task::Plant { pos: p, seed };
            }
            return true;
        }
        false
    }

    fn stockpile_accepts(&self, p: Pos, kind: ItemKind) -> bool {
        let zid = self.world.t(p).zone;
        match self.zones.get(&zid).map(|z| z.kind) {
            Some(ZoneKind::Stockpile(f)) => stock_ok(f, kind),
            _ => false,
        }
    }

    pub fn contents(&self, container: u32) -> usize {
        self.items.values().filter(|it| it.container == Some(container)).count()
    }

    fn find_haul(&mut self, d: &mut Dwarf) -> bool {
        let occupied: HashSet<Pos> = self
            .items
            .values()
            .filter(|it| it.carried_by.is_none())
            .map(|it| it.pos)
            .collect();
        // Containers sitting in stockpiles, with how many more items they take.
        let mut fill: HashMap<u32, usize> = HashMap::new();
        for it in self.items.values() {
            if let Some(c) = it.container {
                *fill.entry(c).or_default() += 1;
            }
        }
        let mut boxes: HashMap<Pos, (u32, ItemKind)> = HashMap::new();
        for it in self.items.values() {
            if it.kind.is_container()
                && it.carried_by.is_none()
                && it.container.is_none()
                && self.world.t(it.pos).zone != 0
                && fill.get(&it.id).copied().unwrap_or(0) < CONTAINER_CAP
            {
                boxes.insert(it.pos, (it.id, it.kind));
            }
        }
        let mut free: Vec<(Pos, StockFilter)> = Vec::new();
        for z in self.zones.values() {
            if let ZoneKind::Stockpile(f) = z.kind {
                for &p in &z.tiles {
                    if !occupied.contains(&p) && !self.reserved.contains(&p) && self.world.walkable(p) {
                        free.push((p, f));
                    }
                }
            }
        }
        let filter_at = |p: Pos| match self.zones.get(&self.world.t(p).zone).map(|z| z.kind) {
            Some(ZoneKind::Stockpile(f)) => Some(f),
            _ => None,
        };
        let box_for = |kind: ItemKind| {
            boxes
                .iter()
                .filter(move |(p, (_, ck))| kind.stored_in() == Some(*ck) && filter_at(**p).map_or(false, |f| f.accepts(kind)))
                .map(|(p, (id, _))| (*p, *id))
        };
        if free.is_empty() && boxes.is_empty() {
            return false;
        }
        let haulable = |it: &Item| {
            if it.container.is_some() || (it.kind.is_container() && fill.contains_key(&it.id)) {
                return false;
            }
            let stored = self.stockpile_accepts(it.pos, it.kind);
            // Loose goods already in their stockpile still move into a container if one has room.
            let could_box = it.kind.stored_in().is_some() && box_for(it.kind).next().is_some();
            (!stored && (free.iter().any(|(_, f)| stock_ok(*f, it.kind)) || could_box)) || (stored && could_box)
        };
        let Some((item, path)) = self.find_item(d.pos, haulable) else { return false };
        let (ipos, kind) = {
            let it = &self.items[&item];
            (it.pos, it.kind)
        };
        let into: HashMap<Pos, u32> = box_for(kind).collect();
        let dests: HashSet<Pos> = free.iter().filter(|(_, f)| stock_ok(*f, kind)).map(|(p, _)| *p).collect();
        let found = path::search(&self.world, ipos, SEARCH_LIMIT, Mover::Dwarf, |p| {
            into.contains_key(&p) || (!self.stockpile_accepts(ipos, kind) && dests.contains(&p))
        });
        let Some((dest, _)) = found else { return false };
        let target = into.get(&dest).copied();
        self.claim_item(item, d.id);
        if target.is_none() {
            self.reserved.insert(dest);
        }
        d.task = Task::Haul { item, dest, into: target };
        Self::set_path(d, path);
        true
    }

    // ---------------------------- task execution ----------------------------

    fn do_task(&mut self, d: &mut Dwarf) {
        match d.task.clone() {
            Task::Idle => {}
            Task::Wander { target } => {
                if !matches!(self.travel(d, target), Travel::Moving) {
                    d.task = Task::Idle;
                }
            }
            Task::Dig { target, stand } => {
                let desig = self.world.t(target).desig;
                if desig == Desig::None || !self.desig_valid(target, desig) {
                    if desig != Desig::None {
                        self.world.t_mut(target).desig = Desig::None;
                        self.designations.remove(&target);
                    }
                    self.release_task(d);
                    return;
                }
                match self.travel(d, stand) {
                    Travel::Moving => {}
                    Travel::Failed => {
                        self.release_task(d);
                        d.idle_cd = 40;
                    }
                    Travel::Arrived => {
                        let labor = if desig == Desig::Chop { Labor::Woodcutting } else { Labor::Mining };
                        let base = if desig == Desig::Chop { 80 } else { self.world.t(target).mat.dig_ticks() };
                        if Self::labor(d, base, d.skill(labor)) {
                            self.complete_dig(target);
                            d.gain_xp(labor, 25);
                            d.task = Task::Idle;
                        }
                    }
                }
            }
            Task::Haul { item, dest, into } => match self.fetch(d, item) {
                None => self.release_task(d),
                Some(false) => {}
                Some(true) => match self.travel(d, dest) {
                    Travel::Moving => {}
                    Travel::Failed => self.release_task(d),
                    Travel::Arrived => {
                        d.carrying = None;
                        let boxed = into.filter(|c| {
                            self.items.get(c).map_or(false, |b| b.pos == dest && b.carried_by.is_none())
                                && self.contents(*c) < CONTAINER_CAP
                        });
                        if let Some(it) = self.items.get_mut(&item) {
                            it.carried_by = None;
                            it.claimed_by = None;
                            it.pos = dest;
                            it.container = boxed;
                        }
                        self.reserved.remove(&dest);
                        d.gain_xp(Labor::Hauling, 5);
                        d.task = Task::Idle;
                    }
                },
            },
            Task::Construct { bld, item } => {
                let Some(site) = self.buildings.get(&bld).map(|b| b.center()) else {
                    self.release_task(d);
                    return;
                };
                match self.fetch(d, item) {
                    None => self.release_task(d),
                    Some(false) => {}
                    Some(true) => match self.travel(d, site) {
                        Travel::Moving => {}
                        Travel::Failed => {
                            self.release_task(d);
                            if let Some(b) = self.buildings.get_mut(&bld) {
                                b.retry_tick = self.tick + 600;
                            }
                        }
                        Travel::Arrived => {
                            if Self::labor(d, 120, d.skill(Labor::Building)) {
                                let (mat, quality) = self.items.remove(&item).map_or((Mat::Wood, 0), |it| (it.mat, it.quality));
                                d.carrying = None;
                                d.gain_xp(Labor::Building, 20);
                                d.task = Task::Idle;
                                if let Some(b) = self.buildings.get_mut(&bld) {
                                    b.built = true;
                                    b.mat = mat;
                                    b.quality = quality;
                                    b.worker = None;
                                    if b.kind == BuildingKind::Door {
                                        let pos = b.pos;
                                        self.world.t_mut(pos).door = DOOR_OPEN;
                                    }
                                }
                                let name = self.buildings[&bld].name();
                                self.msg(&format!("{} has been completed.", name), (160, 220, 255));
                            }
                        }
                    },
                }
            }
            Task::Craft { bld, items, delivered } => {
                let Some(site) = self.buildings.get(&bld).map(|b| b.center()) else {
                    self.release_task(d);
                    return;
                };
                if delivered < items.len() {
                    let item = items[delivered];
                    match self.fetch(d, item) {
                        None => self.release_task(d),
                        Some(false) => {}
                        Some(true) => match self.travel(d, site) {
                            Travel::Moving => {}
                            Travel::Failed => self.release_task(d),
                            Travel::Arrived => {
                                // Set it down at the workshop (still claimed) and go for the next.
                                d.carrying = None;
                                if let Some(it) = self.items.get_mut(&item) {
                                    it.carried_by = None;
                                    it.pos = site;
                                }
                                d.task = Task::Craft { bld, items, delivered: delivered + 1 };
                            }
                        },
                    }
                    return;
                }
                match self.travel(d, site) {
                    Travel::Moving => return,
                    Travel::Failed => {
                        self.release_task(d);
                        return;
                    }
                    Travel::Arrived => {}
                }
                let Some(order) = self.buildings[&bld].orders.first().copied() else {
                    self.release_task(d);
                    return;
                };
                let recipe = order.recipe;
                let level = d.skill(recipe.labor());
                if Self::labor(d, recipe.work(), level) {
                    let used: Vec<Item> = items.iter().filter_map(|id| self.items.remove(id)).collect();
                    d.gain_xp(recipe.labor(), 30);
                    d.task = Task::Idle;
                    let mut made: Vec<String> = Vec::new();
                    let quality = self.roll_quality(level);
                    for (kind, mat, label) in recipe.produce(&used) {
                        let id = self.spawn_item(kind, mat, site);
                        let it = self.items.get_mut(&id).unwrap();
                        it.label = label;
                        if kind.base_value() >= 10 && kind != ItemKind::Bar || kind == ItemKind::Block {
                            it.quality = quality;
                        }
                        made.push(it.name());
                    }
                    if d.likes_mat == used.first().map_or(Mat::Air, |it| it.mat) {
                        d.think("enjoyed working with a favorite material", -6, self.tick);
                    }
                    if let Some(b) = self.buildings.get_mut(&bld) {
                        b.worker = None;
                        if !order.repeat {
                            b.orders.remove(0);
                        }
                    }
                    let n = match made.len() {
                        0 => "nothing".to_string(),
                        1 => made[0].clone(),
                        k => format!("{} x{}", made[0], k),
                    };
                    let who = d.name.clone();
                    self.msg(&format!("{} has made {}.", who, n), (190, 190, 190));
                }
            }
            Task::Plant { pos, .. } | Task::Harvest { pos } => {
                let harvest = matches!(d.task, Task::Harvest { .. });
                let f = self.world.t(pos).farm;
                let still_valid = self.world.t(pos).zone != 0 && if harvest { f >= RIPE } else { f == 0 };
                if !still_valid {
                    self.release_task(d);
                    return;
                }
                if let Task::Plant { seed, .. } = d.task {
                    match self.fetch(d, seed) {
                        None => {
                            self.release_task(d);
                            return;
                        }
                        Some(false) => return,
                        Some(true) => {}
                    }
                }
                match self.travel(d, pos) {
                    Travel::Moving => {}
                    Travel::Failed => self.release_task(d),
                    Travel::Arrived => {
                        if Self::labor(d, 40, d.skill(Labor::Farming)) {
                            self.claimed.remove(&pos);
                            d.gain_xp(Labor::Farming, 15);
                            d.task = Task::Idle;
                            if harvest {
                                self.world.t_mut(pos).farm = 0;
                                let n = 1 + (d.skill(Labor::Farming) >= 3) as u32 + self.rng.chance(0.5) as u32;
                                for _ in 0..n {
                                    self.spawn_item(ItemKind::Mushroom, Mat::Stoutcap, pos);
                                }
                            } else {
                                if let Some(seed) = d.carrying.take() {
                                    self.items.remove(&seed);
                                }
                                self.world.t_mut(pos).farm = 1;
                            }
                        }
                    }
                }
            }
            Task::Eat { item } => match self.fetch(d, item) {
                None => self.release_task(d),
                Some(false) => {}
                Some(true) => {
                    if Self::labor(d, 40, 0) {
                        let kind = self.items.remove(&item).map_or(ItemKind::Mushroom, |it| it.kind);
                        d.carrying = None;
                        d.hunger = 0;
                        d.task = Task::Idle;
                        self.eat_thoughts(d, kind, None);
                    }
                }
            },
            Task::DrinkItem { item } => match self.fetch(d, item) {
                None => self.release_task(d),
                Some(false) => {}
                Some(true) => {
                    if Self::labor(d, 25, 0) {
                        self.items.remove(&item);
                        d.carrying = None;
                        d.thirst = 0;
                        d.task = Task::Idle;
                        d.think("enjoyed a drink of stoutcap ale", -15, self.tick);
                        // An emptied barrel is left behind now and then.
                        if self.rng.chance(0.25) {
                            self.spawn_item(ItemKind::Barrel, Mat::Wood, d.pos);
                        }
                    }
                }
            },
            Task::DrinkWater { stand } => match self.travel(d, stand) {
                Travel::Moving => {}
                Travel::Failed => self.release_task(d),
                Travel::Arrived => {
                    if Self::labor(d, 25, 0) {
                        d.thirst = 0;
                        d.task = Task::Idle;
                        d.think("had to drink water; there was no ale", 12, self.tick);
                    }
                }
            },
            Task::Sleep { bed, stand } => {
                if !d.asleep {
                    match self.travel(d, stand) {
                        Travel::Moving => return,
                        Travel::Failed => {
                            self.release_task(d);
                            d.task = Task::Sleep { bed: None, stand: d.pos };
                            return;
                        }
                        Travel::Arrived => d.asleep = true,
                    }
                }
                let rate = if bed.is_some() { 6 } else { 4 };
                d.fatigue = d.fatigue.saturating_sub(rate);
                if d.fatigue == 0 || d.thirst > THIRSTY + 3000 || d.hunger > HUNGRY + 6000 {
                    let rested = d.fatigue == 0;
                    self.release_task(d);
                    if rested {
                        self.sleep_thought(d, bed);
                    }
                }
            }
            Task::Rest { stand, .. } => {
                if !d.asleep {
                    match self.travel(d, stand) {
                        Travel::Moving => return,
                        Travel::Failed => {
                            self.release_task(d);
                            d.task = Task::Rest { bed: None, stand: d.pos };
                            return;
                        }
                        Travel::Arrived => d.asleep = true,
                    }
                }
                d.fatigue = d.fatigue.saturating_sub(2);
                if self.tick % 50 == 0 {
                    d.body.heal(1);
                }
                if d.body.healed() || d.thirst > THIRSTY + 1000 || d.hunger > HUNGRY + 2000 {
                    self.release_task(d);
                }
            }
            Task::Equip { item } => self.do_equip(d, item),
            Task::Mood => self.do_mood(d),
            Task::Dine { item, seat } => {
                let Some(spos) = self.buildings.get(&seat).filter(|b| b.built).map(|b| b.pos) else {
                    self.release_task(d);
                    return;
                };
                match self.fetch(d, item) {
                    None => self.release_task(d),
                    Some(false) => {}
                    Some(true) => match self.travel(d, spos) {
                        Travel::Moving => {}
                        Travel::Failed => {
                            if let Some(b) = self.buildings.get_mut(&seat) {
                                b.occupant = None;
                            }
                            d.task = Task::Eat { item };
                        }
                        Travel::Arrived => {
                            if Self::labor(d, 40, 0) {
                                let kind = self.items.remove(&item).map_or(ItemKind::Mushroom, |it| it.kind);
                                d.carrying = None;
                                d.hunger = 0;
                                d.task = Task::Idle;
                                self.eat_thoughts(d, kind, Some(seat));
                            }
                        }
                    },
                }
            }
            Task::Attack { target } => self.do_attack(d, target),
            Task::Flee => self.do_flee(d),
            Task::Station { pos } => match self.travel(d, pos) {
                Travel::Moving => {}
                Travel::Failed => {
                    d.task = Task::Idle;
                    d.idle_cd = 60;
                }
                Travel::Arrived => d.task = Task::Idle,
            },
            Task::Train => {
                if Self::labor(d, 300, 0) {
                    d.task = Task::Idle;
                } else if d.work % 2 == 0 {
                    d.fight_xp += 1;
                }
            }
        }
    }

    fn complete_dig(&mut self, q: Pos) {
        self.dig_version += 1;
        let t = *self.world.t(q);
        let was_wall = t.kind == TileKind::Wall;
        let mut drop_at = q;
        match t.desig {
            Desig::Dig => self.world.t_mut(q).kind = TileKind::Floor,
            Desig::StairUp => self.world.t_mut(q).kind = TileKind::StairUp,
            Desig::StairDown => {
                self.world.t_mut(q).kind =
                    if t.kind == TileKind::StairUp { TileKind::StairUD } else { TileKind::StairDown };
            }
            Desig::StairUD => self.world.t_mut(q).kind = TileKind::StairUD,
            Desig::Ramp => {
                self.world.t_mut(q).kind = TileKind::Ramp;
                let up = q.up();
                if self.world.kind(up).map_or(false, |k| k.has_floor()) && self.world.t(up).building == 0 {
                    self.world.t_mut(up).kind = TileKind::Open;
                    self.drop_items_at(up);
                }
            }
            Desig::Channel => {
                self.world.t_mut(q).kind = TileKind::Open;
                let below = q.down();
                if self.world.kind(below) == Some(TileKind::Wall) && self.world.t(below).mat.diggable() {
                    self.world.t_mut(below).kind = TileKind::Ramp;
                    drop_at = below;
                }
                self.world.reveal_around(below);
                self.drop_items_at(q);
            }
            Desig::Chop => {
                let outside = self.world.outside(q);
                let tt = self.world.t_mut(q);
                tt.kind = TileKind::Floor;
                tt.mat = if outside { Mat::Grass } else { Mat::Loam };
                let n = 1 + self.rng.chance(0.4) as i32;
                for _ in 0..n {
                    self.spawn_item(ItemKind::Log, Mat::Wood, q);
                }
            }
            Desig::None => {}
        }
        let tt = self.world.t_mut(q);
        tt.desig = Desig::None;
        if tt.kind == TileKind::Open {
            tt.farm = 0;
        }
        self.designations.remove(&q);
        self.claimed.remove(&q);
        if was_wall && t.desig != Desig::Chop {
            let m = t.mat;
            let drop = m.is_ore() || m.is_gem() || (m.is_building_stone() && self.rng.chance(0.3));
            if drop {
                self.spawn_item(ItemKind::Boulder, m, drop_at);
            }
            if m.is_gem() {
                self.msg(&format!("You have struck {}!", m.name()), (220, 140, 255));
            } else if m.is_ore() && self.rng.chance(0.15) {
                self.msg(&format!("You have struck {}!", m.name()), (255, 210, 90));
            }
        }
        if self.world.reveal_around(q) && !self.world.outside(q) {
            self.warn(
                "cavern",
                "You have discovered an expansive cavern deep underground.",
                (120, 200, 255),
            );
            self.pause_request = true;
        }
    }

    fn drop_items_at(&mut self, p: Pos) {
        let dest = self.world.settle(p);
        for it in self.items.values_mut() {
            if it.pos == p && it.carried_by.is_none() {
                it.pos = dest;
            }
        }
    }
}

/// Whether a stockpile with this filter wants the item, counting containers
/// as belonging wherever their contents do.
fn stock_ok(f: StockFilter, kind: ItemKind) -> bool {
    f.accepts(kind)
        || match kind {
            ItemKind::Barrel => f.accepts(ItemKind::Mushroom),
            ItemKind::Bin => f.accepts(ItemKind::Bar) || f.accepts(ItemKind::Craft) || f.accepts(ItemKind::Pick),
            _ => false,
        }
}

fn day_ordinal(d: u64) -> String {
    let suf = match (d % 10, d % 100) {
        (1, n) if n != 11 => "st",
        (2, n) if n != 12 => "nd",
        (3, n) if n != 13 => "rd",
        _ => "th",
    };
    format!("{}{}", d, suf)
}

/// Pick a dry, open spot near the middle of the map for the wagon.
fn find_embark(w: &World) -> Pos {
    let (cx, cy) = (w.w / 2, w.h / 2);
    let mut best = None;
    let mut best_score = i32::MIN;
    for y in 4..w.h - 4 {
        for x in 4..w.w - 4 {
            let p = Pos::new(x, y, w.surface_z(x, y));
            if !w.walkable(p) || w.t(p).water > 0 {
                continue;
            }
            let mut score = 0;
            for dy in -2..=2 {
                for dx in -2..=2 {
                    let q = Pos::new(x + dx, y + dy, p.z);
                    if w.walkable(q) && w.t(q).water == 0 && w.t(q).kind != TileKind::Ramp {
                        score += 4;
                    }
                }
            }
            score -= (x - cx).abs() + (y - cy).abs();
            if score > best_score {
                best_score = score;
                best = Some(p);
            }
        }
    }
    best.unwrap_or(Pos::new(cx, cy, w.surface_z(cx, cy)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(g: &Game, k: ItemKind) -> usize {
        g.items.values().filter(|it| it.kind == k).count()
    }

    fn run(g: &mut Game, ticks: u64) {
        for _ in 0..ticks {
            g.step();
        }
    }

    #[test]
    fn embark_is_sane() {
        for seed in 1..6 {
            let g = Game::new(seed);
            assert_eq!(g.alive_count(), 7);
            for d in &g.dwarves {
                assert!(g.world.walkable(d.pos), "seed {} dwarf on unwalkable {:?}", seed, d.pos);
            }
        }
    }

    #[test]
    fn fortress_works() {
        let mut g = Game::new(42);
        let e = g.embark;
        // Chop every tree within 12 tiles.
        g.designate(e.offset(-12, -12, -2), e.offset(12, 12, 2), Desig::Chop);
        let trees = g.designations.len();
        // A staircase down 6 levels next to the wagon, then a room at the bottom.
        let s = (1..10)
            .map(|i| e.offset(i, 0, 0))
            .find(|&p| g.world.t(p).kind == TileKind::Floor)
            .expect("no flat tile");
        g.designate(s, s.offset(0, 0, -6), Desig::StairUD);
        let bottom = s.offset(0, 0, -6);
        g.designate(bottom.offset(1, -2, 0), bottom.offset(5, 2, 0), Desig::Dig);
        // Stockpiles and a farm on the surface.
        assert!(g.add_zone(e.offset(-6, 3, -1), e.offset(-1, 7, 1), ZoneKind::Stockpile(StockFilter::All)) > 0);
        let farm_tiles = g.add_zone(e.offset(-8, -8, -1), e.offset(-2, -3, 1), ZoneKind::Farm);
        assert!(farm_tiles > 0, "no farmable soil near the wagon");

        run(&mut g, 6 * DAY);
        let logs = count(&g, ItemKind::Log);
        assert!(trees == 0 || logs > 6, "no trees chopped ({} designated, {} logs)", trees, logs);
        assert!(g.world.t(bottom).kind == TileKind::StairUD, "stairs not dug: {:?}", g.world.t(bottom).kind);
        let room_dug = box_iter(bottom.offset(1, -2, 0), bottom.offset(5, 2, 0))
            .filter(|&p| g.world.t(p).kind == TileKind::Floor)
            .count();
        assert!(room_dug > 10, "room not dug ({} tiles)", room_dug);

        // Build a carpenter's workshop and queue beds.
        let site = (0..40)
            .map(|i| e.offset(-8 + i % 10, -10 + i / 10 * 4, 0))
            .find(|&p| g.can_place(BuildingKind::Carpenter, p))
            .expect("no site for workshop");
        assert!(g.place_building(BuildingKind::Carpenter, site));
        run(&mut g, 2 * DAY);
        let shop = *g.buildings.keys().next().unwrap();
        assert!(g.buildings[&shop].built, "workshop never built");
        g.add_order(shop, Recipe::WoodBed);
        g.add_order(shop, Recipe::WoodBed);
        run(&mut g, 2 * DAY);
        assert!(count(&g, ItemKind::Bed) >= 2, "beds not made");

        // Farm should be planted and eventually produce.
        run(&mut g, 6 * DAY);
        let planted = g
            .zones
            .values()
            .filter(|z| z.kind == ZoneKind::Farm)
            .flat_map(|z| z.tiles.iter())
            .filter(|&&p| g.world.t(p).farm > 0)
            .count();
        assert!(planted > 0 || count(&g, ItemKind::Mushroom) > 0, "farm untouched");
        // Stockpile used.
        let stocked = g.items.values().filter(|it| g.stockpile_accepts(it.pos, it.kind)).count();
        assert!(stocked > 5, "hauling not working ({} stocked)", stocked);
        assert_eq!(g.alive_count(), 7, "dwarves died: {:?}", g.log.iter().rev().take(5).map(|l| &l.text).collect::<Vec<_>>());
    }

    #[test]
    fn long_run_survives_with_supplies() {
        let mut g = Game::new(7);
        run(&mut g, 20 * DAY);
        assert_eq!(g.alive_count(), 7);
        // Stress shouldn't explode without cause.
        assert!(g.dwarves.iter().all(|d| d.tantrum == 0 || d.stress < 600));
    }

    #[test]
    fn water_flows_into_channel() {
        let mut g = Game::new(3);
        // Find a lake cell and a dry neighbour column at the same surface height.
        let w = &g.world;
        let mut spot = None;
        'outer: for y in 1..w.h - 1 {
            for x in 1..w.w - 1 {
                let p = Pos::new(x, y, w.surface_z(x, y));
                if w.t(p).water > 0 || !w.walkable(p) {
                    continue;
                }
                for (dx, dy) in DIRS4 {
                    let q = Pos::new(x + dx, y + dy, p.z);
                    if w.get(q).map_or(false, |t| t.water == 7) {
                        spot = Some(p);
                        break 'outer;
                    }
                }
            }
        }
        let Some(p) = spot else { return };
        g.world.t_mut(p).kind = TileKind::Open;
        g.world.t_mut(p.down()).kind = TileKind::Floor;
        for _ in 0..300 {
            g.fluids();
        }
        assert!(g.world.t(p.down()).water > 0 || g.world.t(p).water > 0, "water didn't flow");
    }

    #[test]
    fn save_roundtrip() {
        let mut g = Game::new(9);
        run(&mut g, 500);
        let s = serde_json::to_string(&g).unwrap();
        let g2: Game = serde_json::from_str(&s).unwrap();
        assert_eq!(g2.tick, g.tick);
        assert_eq!(g2.items.len(), g.items.len());
        assert_eq!(g2.dwarves.len(), g.dwarves.len());
    }
}

#[cfg(test)]
mod bench {
    use super::*;
    #[test]
    #[ignore]
    fn one_year_timing() {
        let mut g = Game::new(5);
        let e = g.embark;
        g.designate(e.offset(-15, -15, -1), e.offset(15, 15, 1), Desig::Chop);
        g.add_zone(e.offset(-8, 3, -1), e.offset(-1, 9, 1), ZoneKind::Stockpile(StockFilter::All));
        g.add_zone(e.offset(-8, -8, -1), e.offset(-2, -3, 1), ZoneKind::Farm);
        let t = std::time::Instant::now();
        for _ in 0..YEAR {
            g.step();
        }
        eprintln!("1 year: {:?}, alive {}/{}, items {}", t.elapsed(), g.alive_count(), g.dwarves.len(), g.items.len());
        for l in g.log.iter().rev().take(12) {
            eprintln!("  {}", l.text);
        }
    }
}
