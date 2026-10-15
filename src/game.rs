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
use crate::overworld::{Biome, Neighbor, Overworld, Race};
use crate::world::{DOOR_LOCKED, DOOR_OPEN};
use std::collections::{BTreeMap, HashMap, HashSet};

mod combat;
#[cfg(test)]
mod industry_tests;
mod psyche;
#[cfg(test)]
mod psyche_tests;
mod labor;
pub use labor::WorkDetail;
mod rooms;
pub use rooms::PileToggle;
mod construct;
mod burrows;
mod traps;
mod military;
mod medicine;
mod animals;
mod needs;
mod family;
pub use family::ADULT_AGE;
mod water;
mod clothing;
mod farming;
mod orders;
mod positions;
mod justice;
mod detail;
mod weather;
mod festivals;
mod heroes;
mod disasters;
mod chronicle;
mod visitors;
mod ruins;
mod expeditions;
mod drama;
mod pets;
mod fishlore;
mod necromancy;
mod milestones;
mod mysteries;
mod megaprojects;
pub use mysteries::{Mystery, MysteryKind};
pub use necromancy::Necromancer;
pub fn milestones_banner_ticks() -> u64 {
    milestones::BANNER_TICKS
}
pub use fishlore::BigFish;
pub use drama::Duel;
pub use expeditions::Expedition;
pub use expeditions::Goal as expeditions_goal;
pub use ruins::Vault;
pub use chronicle::{Chronicle, YearLog};
pub use festivals::Festival;
mod fire;
pub use weather::Weather;
pub use detail::ENGRAVED;
pub use justice::{Crime, CrimeKind};
pub use positions::{Demand, Fit, Position};
pub use orders::{all_recipes, WorkOrder};
pub use medicine::Treatment;
pub use military::{Squad, SquadOrder};
#[cfg(test)]
pub fn military_weapon_for_tests() -> military::UniformWeapon {
    military::UniformWeapon::Crossbow
}
pub use burrows::{Burrow, MAX_BURROWS, TRAFFIC_NAMES};
pub use construct::{BuildMat, ConstructKind, Construction};
mod trade;
pub use trade::Caravan;
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
    /// Where idle dwarves gather and chat.
    Meeting,
    /// One dwarf's room; beds in it are theirs.
    Bedroom,
    /// Shared beds nobody owns.
    Dormitory,
    /// Seats here are taken first at mealtimes.
    Dining,
    /// A noble's office.
    Office,
    /// Items marked for dumping are thrown here.
    Dump,
    /// Soldiers train and spar here.
    Barracks,
    /// The wounded rest here and doctors work.
    Hospital,
    /// Tame animals assigned here stay inside and breed.
    Pasture,
    /// Dwarves come here to pray.
    Temple,
    /// A meeting hall for drinking and making merry.
    Tavern,
    /// Coffins here are used first; burial in a tomb comforts mourners more.
    Tomb,
    /// Farmers pick wild shrubs here for whatever is in season.
    Gathering,
    /// Where the sheriff's prisoners serve their sentences.
    Jail,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Zone {
    pub id: u32,
    pub kind: ZoneKind,
    pub tiles: Vec<Pos>,
    /// Bedrooms and offices belong to one dwarf.
    #[serde(default)]
    pub owner: Option<u32>,
    /// The one crop a farm grows (None: whatever suits the season).
    #[serde(default)]
    pub crop: Option<Mat>,
    /// Kinds of item a stockpile refuses, though its category takes them.
    #[serde(default)]
    pub deny: Vec<ItemKind>,
    /// Materials a stockpile refuses (containers are let in regardless).
    #[serde(default)]
    pub deny_mats: Vec<Mat>,
}

impl Zone {
    /// Whether this stockpile takes `kind` made of `mat`.
    pub fn takes(&self, kind: ItemKind, mat: Mat) -> bool {
        let ZoneKind::Stockpile(f) = self.kind else { return false };
        stock_ok(f, kind) && !self.deny.contains(&kind) && (kind.is_container() || !self.deny_mats.contains(&mat))
    }
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
    /// Who the last siege came from: their race, for messages.
    #[serde(default)]
    siege_race: Option<Race>,
    #[serde(default)]
    pub mandate: Option<Mandate>,
    pub overworld: Overworld,
    /// Embark location on the world map.
    pub site: (i32, i32),
    pub biome: Biome,
    pub evil: bool,
    pub savagery: f64,
    /// The dwarven civilization this outpost belongs to.
    pub civ: Option<u32>,
    pub neighbors: Vec<Neighbor>,
    pub start_year: i32,
    #[serde(default)]
    pub caravan: Option<Caravan>,
    #[serde(default = "labor::default_details")]
    pub work_details: Vec<WorkDetail>,
    /// Planned walls, floors, stairs and removals.
    #[serde(default)]
    pub constructions: BTreeMap<u32, Construction>,
    #[serde(default)]
    pub burrows: Vec<Burrow>,
    #[serde(default)]
    pub squads: Vec<Squad>,
    /// Bit of the burrow civilians flee to when the alert is raised.
    #[serde(default)]
    pub alert_burrow: Option<u8>,
    #[serde(default)]
    pub civ_alert: bool,
    #[serde(skip)]
    pub pause_request: bool,
    /// Set at each new season; the app saves an autosave and clears it.
    #[serde(skip)]
    pub autosave_request: bool,
    /// Bumped whenever designations or dug terrain change.
    #[serde(skip)]
    dig_version: u64,
    /// Last failed dig search per kind (mine, chop): (version, tick).
    #[serde(skip)]
    dig_fail: [Option<(u64, u64)>; 2],
    /// Cells holding water, rebuilt from the map when missing (e.g. after loading).
    #[serde(skip)]
    water_cells: Option<Vec<usize>>,
    /// Magma cells that may still flow; rebuilt whenever the map changes.
    #[serde(skip)]
    magma_cells: Option<Vec<usize>>,
    #[serde(skip)]
    magma_seen: u64,
    /// Open cells beside (or dug out of) aquifer rock, and the dig version they were found at.
    #[serde(skip)]
    aquifer_leaks: Option<(u64, Vec<usize>)>,
    /// Every aquifer cell on the map (they never change).
    #[serde(skip)]
    aquifer_tiles: Option<Vec<usize>>,
    /// Fortress-wide "keep N in stock" orders.
    #[serde(default)]
    pub work_orders: Vec<WorkOrder>,
    /// Appointed administrators: (position, dwarf id).
    #[serde(default)]
    pub positions: Vec<(Position, u32)>,
    /// Titled nobles' demands for furnishings in their rooms.
    #[serde(default)]
    pub demands: Vec<Demand>,
    /// Each post holder's fit, for while they're out for their own turn.
    #[serde(skip)]
    fit_cache: Vec<(Position, Fit)>,
    /// The fortress's record of crimes, newest last.
    #[serde(default)]
    pub crimes: Vec<Crime>,
    #[serde(default)]
    pub weather: Weather,
    /// A summer without rain.
    #[serde(default)]
    pub drought: bool,
    /// Some tile has snow on it (so a thaw knows to look).
    #[serde(default)]
    snow_lying: bool,
    /// Burning cells, rebuilt from the map when missing.
    #[serde(skip)]
    fire_cells: Option<Vec<usize>>,
    /// The festival under way, if any.
    #[serde(default)]
    pub festival: Option<Festival>,
    /// The sealed hall of an older people, somewhere in the rock.
    #[serde(default)]
    pub vault: Option<Vault>,
    /// A vault has been looked for (older saves get one when loaded).
    #[serde(default)]
    vault_placed: bool,
    /// The name of the fortress's great monument, once dedicated.
    #[serde(default)]
    pub monument: Option<String>,
    /// Mysteries unfolding, and those already told.
    #[serde(default)]
    pub mysteries: Vec<Mystery>,
    #[serde(default)]
    pub mysteries_done: Vec<MysteryKind>,
    /// A spy has carried word of us home: the next siege comes for certain.
    #[serde(default)]
    siege_warned: bool,
    /// Milestones reached: (key, text, tick).
    #[serde(default)]
    pub milestones: Vec<(String, String, u64)>,
    /// The milestone banner on screen, with when it went up.
    #[serde(skip)]
    pub banner: Option<(String, u64)>,
    /// A necromancer in a tower nearby, at some sites.
    #[serde(default)]
    pub necromancer: Option<Necromancer>,
    #[serde(default)]
    necro_rolled: bool,
    /// The legendary fish in the site's waters, once it has shown itself.
    #[serde(default)]
    pub big_fish: Option<BigFish>,
    /// Who said what about whom, newest last.
    #[serde(default)]
    pub gossip: Vec<(u64, String)>,
    /// Duels waiting for dawn.
    #[serde(default)]
    pub duels: Vec<Duel>,
    /// A squad away from the map, if any.
    #[serde(default)]
    pub expedition: Option<Expedition>,
    /// What's happened so far this year, for the chronicle.
    #[serde(default)]
    pub year_log: YearLog,
    /// One page per year gone by.
    #[serde(default)]
    pub chronicles: Vec<Chronicle>,
    /// A new page has been written and should be shown.
    #[serde(skip)]
    pub chronicle_ready: bool,
    /// Tiles a dragon's breath just swept, with the tick, for drawing.
    #[serde(skip)]
    pub flames: Vec<(Pos, u64)>,
    /// After a fruitless search for hauling work, nobody looks again until this tick.
    #[serde(skip)]
    haul_retry: u64,
    /// Patients a doctor is already on the way to.
    #[serde(skip)]
    treating: HashSet<u32>,
    /// Animals a butcher is already on the way to.
    #[serde(skip)]
    slaughtering: HashSet<u32>,
}

impl Game {
    /// A fresh world with the expedition sent to a sensible default site (tests).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn new(seed: u64) -> Game {
        let ow = Overworld::generate(seed);
        let (x, y) = ow.default_site();
        Game::embark(seed, ow, x, y)
    }

    /// A stand-in game holding just a world map: the title screen and site choice
    /// run on this until the expedition embarks.
    pub fn placeholder(ow: Overworld) -> Game {
        let site = if ow.w > 1 { ow.default_site() } else { (0, 0) };
        let mut g = Game::from_world(World::empty(), 0, ow, site, Pos::new(0, 0, 0));
        g.fortress_name = String::new();
        g
    }

    /// Whether this is the stand-in from `placeholder`, not a real fortress.
    pub fn is_placeholder(&self) -> bool {
        self.world.w <= 1
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn embark(seed: u64, ow: Overworld, sx: i32, sy: i32) -> Game {
        Self::embark_with(seed, ow, sx, sy, world::MapSize::MEDIUM, &|_, _| {})
    }

    pub fn embark_with(seed: u64, ow: Overworld, sx: i32, sy: i32, size: world::MapSize, report: &dyn Fn(&str, f32)) -> Game {
        report("Surveying the site", 0.05);
        let cell = ow.cell(sx, sy).clone();
        let mut params = cell.biome.params();
        params.minerals = ow.minerals;
        params.size = size;
        params.aquifer = ow.aquifer(sx, sy);
        let world = world::generate(seed ^ ((sx as u64) << 20) ^ ((sy as u64) << 40), &params);
        report("Loading the wagon", 0.8);
        let embark = find_embark(&world);
        let mut g = Game::from_world(world, seed, ow, (sx, sy), embark);
        g.settle_expedition(embark);
        g.bring_animals(embark);
        g.place_vault();
        g.place_necromancer();
        report("Strike the earth!", 1.0);
        g
    }

    fn from_world(world: World, seed: u64, ow: Overworld, (sx, sy): (i32, i32), embark: Pos) -> Game {
        let cell = ow.cell(sx, sy).clone();
        let params = cell.biome.params();
        let civ = ow.parent_civ(sx, sy);
        let neighbors = ow.neighbors(sx, sy, 30);
        let start_year = ow.year + 1;
        let mut rng = Rng::new(seed.wrapping_mul(31).wrapping_add(7));
        Game {
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
            autosave_request: false,
            water_cells: None,
            magma_cells: None,
            magma_seen: 0,
            aquifer_leaks: None,
            aquifer_tiles: None,
            work_orders: Vec::new(),
            positions: Vec::new(),
            demands: Vec::new(),
            fit_cache: Vec::new(),
            crimes: Vec::new(),
            weather: Weather::Clear,
            drought: false,
            snow_lying: false,
            fire_cells: None,
            flames: Vec::new(),
            festival: None,
            year_log: YearLog::default(),
            vault: None,
            vault_placed: false,
            expedition: None,
            gossip: Vec::new(),
            big_fish: None,
            necromancer: None,
            milestones: Vec::new(),
            mysteries: Vec::new(),
            monument: None,
            mysteries_done: Vec::new(),
            siege_warned: false,
            banner: None,
            necro_rolled: false,
            duels: Vec::new(),
            chronicles: Vec::new(),
            chronicle_ready: false,
            creatures: BTreeMap::new(),
            station: None,
            alert: true,
            hunt: false,
            siege_size: 0,
            siege_race: None,
            siege_routed: false,
            mandate: None,
            savagery: (params.savagery + if cell.evil { 0.3 } else { 0.0 }) * ow.savagery,
            biome: cell.biome,
            evil: cell.evil,
            overworld: ow,
            site: (sx, sy),
            civ,
            neighbors,
            start_year,
            caravan: None,
            work_details: labor::default_details(),
            constructions: BTreeMap::new(),
            burrows: Vec::new(),
            squads: Vec::new(),
            alert_burrow: None,
            civ_alert: false,
            dig_version: 0,
            dig_fail: [None, None],
            haul_retry: 0,
            treating: HashSet::new(),
            slaughtering: HashSet::new(),
        }
    }

    /// The seven founders, their tools and the wagon's supplies.
    fn settle_expedition(&mut self, embark: Pos) {
        let g = self;
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
            g.dress(&mut d, 120);
            g.dwarves.push(d);
        }
        g.populate_caverns();
        g.appoint_leader();
        let supplies = [
            (ItemKind::Plant, Mat::Stoutcap, 30),
            (ItemKind::Drink, Mat::Stoutcap, 40),
            (ItemKind::Log, Mat::Wood, 6),
            (ItemKind::Axe, Mat::Copper, 1),
            (ItemKind::Pick, Mat::Copper, 1),
            (ItemKind::Seed, Mat::Stoutcap, 15),
            (ItemKind::Seed, Mat::Hollowgrain, 5),
            (ItemKind::Seed, Mat::Threadreed, 5),
            (ItemKind::Seed, Mat::Sunberry, 5),
            (ItemKind::Seed, Mat::Goldoat, 5),
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
        let civ_name = g.civ.map_or("a band of wanderers".to_string(), |c| format!("the {}", g.overworld.civ(c).name));
        let (sx, sy) = g.site;
        let place = g.overworld.region_at(sx, sy).map_or(format!("the {}", g.biome.name()), |r| r.name.clone());
        g.legend(format!("The dwarves of {} founded the outpost of {} in {}.", civ_name, name, place));
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
            Item {
                id,
                kind,
                mat,
                pos,
                carried_by: None,
                claimed_by: None,
                label: None,
                container: None,
                quality: 0,
                to_depot: false,
                dump: false,
                wear: 0,
                forbidden: false,
            },
        );
        id
    }

    pub fn date_string(&self) -> String {
        let year = self.tick / YEAR + 1;
        let month = ((self.tick % YEAR) / MONTH) as usize;
        let day = (self.tick % MONTH) / DAY + 1;
        let year = self.start_year as u64 + year - 1;
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
        // Built walls are taken down, not mined.
        let wall = t.kind == TileKind::Wall && t.mat.diggable() && t.built_from.is_none();
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
            Desig::Smooth => {
                let stone = t.mat.is_building_stone() || t.mat.is_ore() || t.mat.is_gem();
                t.revealed && t.built_from.is_none() && stone && t.smooth == 0 && matches!(t.kind, TileKind::Wall | TileKind::Floor)
            }
            Desig::Engrave => t.revealed && t.smooth == 1 && matches!(t.kind, TileKind::Wall | TileKind::Floor),
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
            let hidden_ok = !t.revealed && !matches!(d, Desig::Chop | Desig::Smooth | Desig::Engrave);
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
            // A meeting hall can take in the tables and chairs already standing in it.
            let bld_ok = t.building == 0 || kind.holds_furniture();
            if !t.revealed || t.zone != 0 || !bld_ok || !matches!(t.kind, TileKind::Floor | TileKind::Shrub) {
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
        self.zones.insert(id, Zone { id, kind, tiles, owner: None, crop: None, deny: Vec::new(), deny_mats: Vec::new() });
        n
    }

    pub fn remove_zones(&mut self, a: Pos, b: Pos) -> usize {
        self.remove_zones_where(a, b, |_| true)
    }

    /// Clear zone tiles in the box whose zone kind passes `which`.
    pub fn remove_zones_where<F: Fn(ZoneKind) -> bool>(&mut self, a: Pos, b: Pos, which: F) -> usize {
        let mut n = 0;
        for p in box_iter(a, b) {
            let Some(t) = self.world.get(p) else { continue };
            let zid = t.zone;
            if zid == 0 || !self.zones.get(&zid).map_or(false, |z| which(z.kind)) {
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
        if kind == BuildingKind::Well {
            return self.can_place_well(top_left);
        }
        if kind.uses_magma() {
            return self.can_place_magma_shop(kind, top_left);
        }
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
            parts: 0,
            load: None,
            trap_cd: 0,
            links: Vec::new(),
            pull: false,
            raised: false,
            workers: Vec::new(),
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
                BuildingKind::Coffin => ItemKind::Coffin,
                BuildingKind::Slab => ItemKind::Slab,
                BuildingKind::Well => ItemKind::Bucket,
                _ => ItemKind::Boulder,
            };
            // A well gives back its bucket, at the edge rather than down the hole.
            let mat = if b.kind == BuildingKind::Well { b.load.map_or(Mat::Wood, |l| l.1) } else { b.mat };
            let id = self.spawn_item(kind, mat, self.work_spot(b.center()));
            self.items.get_mut(&id).unwrap().quality = b.quality;
        }
    }

    pub fn add_order(&mut self, bld: u32, recipe: Recipe) {
        if let Some(b) = self.buildings.get_mut(&bld) {
            b.orders.push(Order { recipe, repeat: false });
        }
    }

    /// Add a dwarf to a workshop's profile, or take them off it.
    pub fn toggle_worker(&mut self, bld: u32, dwarf: u32) {
        let Some(b) = self.buildings.get_mut(&bld) else { return };
        if let Some(i) = b.workers.iter().position(|&w| w == dwarf) {
            b.workers.remove(i);
        } else {
            b.workers.push(dwarf);
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
        if let Some(i) = self.dwarves.iter().position(|d| d.id == dwarf_id && !(d.is_child() && !d.soldier)) {
            let mut d = std::mem::take(&mut self.dwarves[i]);
            d.soldier = !d.soldier;
            if !d.soldier {
                for s in self.squads.iter_mut() {
                    s.members.retain(|&m| m != dwarf_id);
                }
            }
            if !d.task.is_need() {
                self.release_task(&mut d);
            }
            self.dwarves[i] = d;
        }
    }

    // ------------------------------------------------------------------
    // Simulation
    // ------------------------------------------------------------------

    /// A manager sees to work orders once a day, twice if they're good at it;
    /// a bookkeeper's counts let them do it more often, by how good they are.
    pub(super) fn orders_due(&self, tick: u64) -> bool {
        let counts = match self.post_fit(Position::Bookkeeper) {
            Some(Fit::Good) => Some(150),
            Some(Fit::Fair) => Some(300),
            Some(Fit::Poor) => Some(600),
            None => None,
        };
        let good_manager = self.post_fit(Position::Manager) == Some(Fit::Good);
        tick % DAY == DAY / 4
            || counts.map_or(false, |every| tick % every == 150 % every)
            || (good_manager && tick % DAY == DAY * 3 / 4)
    }

    pub fn step(&mut self) {
        self.tick += 1;
        if self.tick % 3 == 0 {
            self.fluids();
        }
        if self.tick % 6 == 0 {
            self.magma();
        }
        if self.tick % 25 == 0 {
            self.aquifers();
        }
        if self.tick % 300 == 75 {
            self.weather_thoughts();
            self.lightning();
        }
        if self.tick % 10 == 5 && self.fire_cells.as_ref().map_or(true, |c| !c.is_empty()) {
            self.fires();
        }
        if self.tick % 50 == 0 {
            self.count_fits();
        }
        if self.orders_due(self.tick) {
            self.manage_orders();
        }
        if self.tick % DAY == DAY / 4 {
            self.festivals_daily();
        }
        if self.tick % 50 == 25 {
            self.festival_tick();
        }
        if self.tick % 100 == 60 {
            self.expedition_tick();
            self.duels_tick();
            self.mysteries_tick();
        }
        if self.tick % DAY == DAY / 2 + 7 {
            self.drama_daily();
            self.pets_daily();
            self.milestones_daily();
            self.monument_daily();
        }
        if self.tick % 10 == 0 {
            self.grow_farms();
        }
        for i in 0..self.dwarves.len() {
            if self.dwarves[i].alive {
                let mut d = std::mem::take(&mut self.dwarves[i]);
                self.world.restrict = self.burrow_turn(&mut d);
                self.update_dwarf(&mut d);
                self.world.restrict = 0;
                self.dwarves[i] = d;
            }
        }
        self.ghosts_tick();
        self.update_creatures();
        self.traps_tick();
        self.update_caravan();
        if self.tick % DAY == DAY / 2 {
            self.wildlife();
        }
        if self.tick % DAY == 0 {
            self.assign_rooms();
            self.families_daily();
            self.clothes_daily();
            self.weather_daily();
            self.nobles_daily();
            self.titles_daily();
            self.check_mandate();
        }
        if self.tick % SEASON == 0 {
            let season = (self.tick / SEASON) % 4;
            let s = SEASONS[season as usize];
            self.msg(&format!("{} has arrived.", s), (200, 200, 255));
            self.autosave_request = true;
            if season == 0 {
                self.write_chronicle();
                self.new_year();
            }
            self.seasonal_ice(season);
            if season == 1 || season == 2 {
                self.migrants();
            }
            self.regrow_shrubs();
            self.maybe_siege();
            self.maybe_megabeast();
            self.maybe_cave_horror();
            self.maybe_thief();
            self.maybe_visitor(season);
            self.maybe_undead();
            self.maybe_stranger();
            self.caravan_season(season);
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
            if t.frozen {
                wet.insert(i);
                continue;
            }
            // Fall through anything without a floor.
            if matches!(t.kind, TileKind::Open | TileKind::StairDown | TileKind::StairUD) {
                let b = p.down();
                if self.world.in_bounds(b) && !self.world.blocks_water(b) {
                    if self.world.t(b).magma > 0 {
                        self.quench(b);
                        self.world.tiles[i].water -= 1;
                        wet.insert(i);
                        continue;
                    }
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
            if !self.world.blocks_water(q) && self.world.t(q).magma > 0 {
                self.quench(q);
                self.world.tiles[i].water -= 1;
            } else if !self.world.blocks_water(q) {
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
            Labor::Leatherworking,
            Labor::Clothesmaking,
            Labor::Soapmaking,
            Labor::Fishing,
            Labor::Engraving,
        ];
        for _ in 0..n {
            let id = self.new_id();
            let spec = *self.rng.pick(&pool);
            let specs: Vec<Labor> = if self.rng.chance(0.3) { vec![] } else { vec![spec] };
            let pos = self.random_spot_near(entry, 2);
            let mut d = dwarf::new_dwarf(id, pos, &mut self.rng, &specs);
            self.dress(&mut d, 250);
            self.dwarves.push(d);
        }
        self.msg(&format!("Some migrants have arrived. ({} dwarves)", n), (120, 255, 120));
        self.year_log.arrivals += n as u32;
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

        if self.world.t(d.pos).water >= 6 && !self.world.t(d.pos).frozen {
            d.drown += 1;
            if d.drown > 400 {
                self.kill(d, "has drowned");
                return;
            }
        } else {
            d.drown = 0;
        }
        if self.world.t(d.pos).magma > 0 {
            self.kill(d, "has burned to death in magma");
            return;
        }

        if self.tick % 100 == 0 {
            if d.thirst > THIRSTY * 2 || d.hunger > HUNGRY * 2 {
                d.stress += 4;
            }
            d.stress -= d.stress.signum();
            d.stress = d.stress.max(crate::dwarf::STRESS_FLOOR);
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
            } else if self.on_break(d) {
                d.idle_cd = 20 + self.rng.range(0, 20) as u32;
                self.idle_activity(d);
            } else {
                self.choose_task(d);
                if d.task == Task::Idle {
                    d.idle_cd = 20 + self.rng.range(0, 20) as u32;
                    self.idle_activity(d);
                }
            }
        }
        // An idle stroll gives way as soon as there's real work.
        if matches!(d.task, Task::Wander { .. })
            && d.tantrum == 0
            && self.tick % 30 == d.id as u64 % 30
            && !self.returning_to_burrow(d)
            && self.tick >= d.break_until
        {
            let stroll = std::mem::take(&mut d.task);
            self.choose_task(d);
            if d.task == Task::Idle {
                d.task = stroll;
            }
        }
        if d.task != Task::Idle {
            self.do_task(d);
        }
        self.carry_follow(d);
    }

    /// Meeting-area tiles a dwarf could stand on.
    pub fn meeting_tiles(&self) -> Vec<Pos> {
        self.zones
            .values()
            .filter(|z| matches!(z.kind, ZoneKind::Meeting | ZoneKind::Tavern))
            .flat_map(|z| z.tiles.iter().copied())
            .filter(|&p| self.world.walkable(p))
            .collect()
    }

    pub fn in_meeting_area(&self, p: Pos) -> bool {
        self.zones.get(&self.world.t(p).zone).map_or(false, |z| matches!(z.kind, ZoneKind::Meeting | ZoneKind::Tavern))
    }

    /// Nothing to do: head for the meeting area, or at least out of the way.
    fn idle_activity(&mut self, d: &mut Dwarf) {
        if self.festival_activity(d) {
            return;
        }
        if self.seek_need_zone(d) {
            return;
        }
        let meet = self.meeting_tiles();
        if !meet.is_empty() {
            let inside = self.in_meeting_area(d.pos);
            if !inside || self.rng.chance(0.15) {
                let q = meet[self.rng.range(0, meet.len() as i32) as usize];
                if q != d.pos {
                    d.task = Task::Wander { target: q };
                }
            }
            return;
        }
        // Without one, don't loiter in corridors and doorways.
        if self.openness(d.pos) <= 3 || self.rng.chance(0.25) {
            self.start_wander(d);
        }
    }

    /// Number of tiles a dwarf could step to from `p`.
    fn openness(&self, p: Pos) -> usize {
        let mut nb = Vec::new();
        self.world.neighbors(p, &mut nb, Mover::Dwarf);
        nb.len()
    }

    fn carry_follow(&mut self, d: &Dwarf) {
        for id in d.carrying.iter().chain(d.held.iter()).chain(d.armor.iter()).chain(d.clothes.iter()) {
            if let Some(it) = self.items.get_mut(id) {
                it.pos = d.pos;
            }
        }
    }

    fn kill(&mut self, d: &mut Dwarf, cause: &str) {
        self.release_task(d);
        d.alive = false;
        let pos = d.pos;
        for id in d.held.take().into_iter().chain(d.armor.drain(..)).chain(d.clothes.drain(..)) {
            self.drop_item(id, pos);
        }
        let id = self.spawn_item(ItemKind::Corpse, Mat::Flesh, pos);
        if let Some(it) = self.items.get_mut(&id) {
            it.label = Some(d.name.clone());
        }
        let name = d.name.clone();
        self.msg(&format!("{} {}.", name, cause), (255, 60, 60));
        self.hero_fell(d, cause);
        self.year_log.deaths.push(d.called());
        let tick = self.tick;
        d.died = Some(tick);
        d.due = None;
        let dead = d.id;
        let (spouse, parents) = (d.spouse, d.parents);
        for o in self.dwarves.iter_mut().filter(|o| o.alive) {
            let regard = o.regard(dead);
            if spouse == Some(o.id) {
                o.think(&format!("lost their spouse {}", name), 140, tick);
            } else if parents.map_or(false, |(m, f)| m == o.id || f == o.id) {
                o.think(&format!("lost their child {}", name), 150, tick);
            } else if o.family_of(dead) {
                o.think(&format!("lost their parent {}", name), 110, tick);
            } else if regard >= 30 {
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
            Task::PullLever { bld } => {
                if let Some(b) = self.buildings.get_mut(&bld) {
                    b.worker = None;
                }
            }
            Task::Slaughter { creature } => {
                self.slaughtering.remove(&creature);
            }
            Task::Bury { item, coffin } => {
                self.unclaim(item);
                if let Some(b) = self.buildings.get_mut(&coffin) {
                    if b.worker == Some(d.id) {
                        b.worker = None;
                    }
                }
            }
            Task::Treat { patient, item, .. } => {
                self.treating.remove(&patient);
                if let Some(item) = item {
                    self.unclaim(item);
                }
            }
            Task::BuildTile { con, item, .. } => {
                if let Some(item) = item {
                    self.unclaim(item);
                }
                if let Some(c) = self.constructions.get_mut(&con) {
                    c.worker = None;
                }
            }
            Task::Harvest { pos } | Task::Gather { pos } => {
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
            let focus = d.focus().max(1) as u32;
            d.work = Self::work_time(base * diligent / focus, level);
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
        // Prefer somewhere roomy over a spot in the middle of a corridor.
        let mut fallback = None;
        for _ in 0..12 {
            let q = d.pos.offset(self.rng.range(-6, 7), self.rng.range(-6, 7), 0);
            if !self.world.walkable(q) || q == d.pos {
                continue;
            }
            if self.openness(q) >= 5 {
                d.task = Task::Wander { target: q };
                return;
            }
            fallback.get_or_insert(q);
        }
        if let Some(q) = fallback {
            d.task = Task::Wander { target: q };
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
        if self.serve_sentence(d) {
            return;
        }
        if d.want_clothes && self.find_clothes(d) {
            return;
        }
        if d.is_child() {
            // Children play; idle_activity takes them to the meeting area.
            return;
        }
        if d.soldier {
            self.choose_military(d);
            return;
        }
        if self.find_punishment(d) {
            return;
        }
        if self.find_lever_pull(d) {
            return;
        }
        if d.has(Labor::Medicine) && self.find_patient(d) {
            return;
        }
        if d.has(Labor::Butchery) && self.find_slaughter(d) {
            return;
        }
        if d.has(Labor::Hauling) && self.find_burial(d) {
            return;
        }
        if d.has(Labor::Building) && (self.find_construct(d) || self.find_build_tile(d)) {
            return;
        }
        if self.find_craft(d) {
            return;
        }
        if d.has(Labor::Engraving) && self.find_detail(d) {
            return;
        }
        if d.has(Labor::Mining) && self.find_dig(d, false) {
            return;
        }
        if d.has(Labor::Woodcutting) && self.find_dig(d, true) {
            return;
        }
        if d.has(Labor::Farming) && (self.find_farm(d) || self.find_gather(d)) {
            return;
        }
        if d.has(Labor::Fishing) && self.find_fishing(d) {
            return;
        }
        if d.has(Labor::Hauling) && !d.specialist && !self.find_depot_haul(d) && !self.find_dump_haul(d) {
            self.find_haul(d);
        }
    }

    fn loose_items<F: Fn(&Item) -> bool>(&self, f: F) -> HashMap<Pos, u32> {
        let mut at = HashMap::new();
        for it in self.items.values() {
            if it.free() && f(it) {
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
        let wells = self.working_wells();
        let w = &self.world;
        let water_near = |p: Pos| {
            DIRS8.iter().any(|&(dx, dy)| {
                let q = p.offset(dx, dy, 0);
                let below = q.down();
                // Frozen water will do: they break the ice.
                wells.contains(&q)
                    || w.get(q).map_or(false, |t| t.water > 0)
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
        if let Some((item, path)) = meal.or_else(|| self.find_item(d.pos, |it| it.is_food())) {
            self.claim_item(item, d.id);
            let seats = self.free_seats();
            let ipos = self.items[&item].pos;
            // Seats in a dining hall are taken first.
            let hall: HashMap<Pos, u32> = seats.iter().filter(|(p, _)| self.in_dining_hall(**p)).map(|(p, b)| (*p, *b)).collect();
            let pick = |s: &HashMap<Pos, u32>| {
                if s.is_empty() {
                    return None;
                }
                path::search(&self.world, ipos, SEARCH_LIMIT, Mover::Dwarf, |p| s.contains_key(&p)).map(|(p, _)| s[&p])
            };
            let seat = pick(&hall).or_else(|| pick(&seats));
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
        // A kill order comes before everything else.
        if matches!(self.squad_of(d.id).map(|s| s.order), Some(SquadOrder::Kill(_))) && self.squad_task(d) {
            return;
        }
        if self.alert {
            let at: HashMap<Pos, u32> = self
                .creatures
                .values()
                .filter(|c| {
                    (c.hostile() || (self.hunt && !c.tame && c.species != crate::creature::Species::Merchant))
                        && !c.leaving
                        && self.creature_active(c)
                })
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
        if self.squad_task(d) {
            return;
        }
        // Soldiers without a squad follow the old fortress-wide station.
        if self.squad_of(d.id).is_none() {
            if let Some(st) = self.station {
                if (st.x - d.pos.x).abs() > 3 || (st.y - d.pos.y).abs() > 3 || st.z != d.pos.z {
                    d.task = Task::Station { pos: st };
                    return;
                }
            }
        }
        self.go_train(d);
    }

    fn start_bed(&mut self, d: &mut Dwarf, rest: bool) {
        let task = |bed, stand| if rest { Task::Rest { bed, stand } } else { Task::Sleep { bed, stand } };
        let own = self
            .buildings
            .values()
            .find(|b| b.kind == BuildingKind::Bed && b.built && b.occupant.is_none() && b.owner == Some(d.id))
            .map(|b| (b.pos, b.id));
        let hospital = if rest { self.hospital_beds() } else { HashMap::new() };
        if !hospital.is_empty() {
            if let Some((p, path)) = path::search(&self.world, d.pos, SEARCH_LIMIT, Mover::Dwarf, |p| hospital.contains_key(&p)) {
                let bed = hospital[&p];
                if let Some(b) = self.buildings.get_mut(&bed) {
                    b.occupant = Some(d.id);
                }
                d.task = task(Some(bed), p);
                Self::set_path(d, path);
                return;
            }
        }
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
            let (kind, part) = (self.buildings[&bid].kind, self.buildings[&bid].parts);
            if let Some((item, path)) = self.find_item(d.pos, |it| kind.needs_part(part, it)) {
                self.claim_item(item, d.id);
                self.buildings.get_mut(&bid).unwrap().worker = Some(d.id);
                d.task = Task::Construct { bld: bid, item };
                Self::set_path(d, path);
                return true;
            }
            self.buildings.get_mut(&bid).unwrap().retry_tick = self.tick + 300;
            // One warning per kind of building, not one per building.
            let waiting = self.buildings.values().filter(|b| b.kind == kind && !b.built).count();
            self.warn(
                &format!("need{}", kind.name()),
                &format!("{} {} waiting to be built: it needs {}.", waiting, kind.name().to_lowercase(), kind.part_desc(part)),
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
            .filter(|b| d.has(b.orders[0].recipe.labor()) && (b.workers.is_empty() || b.workers.contains(&d.id)))
            .map(|b| b.id)
            .collect();
        for bid in shops {
            let recipe = self.buildings[&bid].orders[0].recipe;
            let site = self.buildings[&bid].center();
            let shop = self.buildings[&bid].kind;
            if shop.uses_magma() && !self.magma_below(site) {
                self.buildings.get_mut(&bid).unwrap().retry_tick = self.tick + 300;
                self.warn(&format!("craft{}", bid), &format!("{}: there's no magma below it.", shop.name()), (230, 200, 90));
                continue;
            }
            let inputs = recipe.inputs_at(shop);
            let mut chosen: Vec<u32> = Vec::new();
            for input in inputs.iter() {
                match self.find_item(site, |it| input.accepts(it) && !chosen.contains(&it.id)) {
                    Some((id, _)) => chosen.push(id),
                    None => break,
                }
            }
            if chosen.len() == inputs.len() {
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
                &format!("{}: \"{}\" needs {}.", shop_name, recipe.name(), recipe.input_desc_at(shop)),
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
        let wanted = |ds: Desig| {
            if chop {
                ds == Desig::Chop
            } else {
                !matches!(ds, Desig::Chop | Desig::None | Desig::Smooth | Desig::Engrave)
            }
        };
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
        let season = ((self.tick / SEASON) % 4) as usize;
        // Seed on hand, by crop.
        let mut seeds: BTreeMap<Mat, usize> = BTreeMap::new();
        for it in self.items.values().filter(|it| it.kind == ItemKind::Seed && it.free()) {
            *seeds.entry(it.mat).or_default() += 1;
        }
        // Plot -> None to harvest, or Some(crop) to plant.
        let mut plots: HashMap<Pos, Option<Mat>> = HashMap::new();
        let mut fallow = false;
        for z in self.zones.values().filter(|z| z.kind == ZoneKind::Farm) {
            let Some(&first) = z.tiles.first() else { continue };
            let outside = self.world.outside(first);
            let fits = |m: Mat| seeds.get(&m).map_or(false, |&n| n > 0) && m.crop().map_or(false, |c| c.grows(outside, season));
            // A farm set to one crop plants only that; otherwise the most plentiful seed that suits.
            let choice = match z.crop {
                Some(m) => Some(m).filter(|&m| fits(m)),
                None => seeds.iter().filter(|(m, _)| fits(**m)).max_by_key(|(_, n)| **n).map(|(m, _)| *m),
            };
            for &p in &z.tiles {
                if self.claimed.contains(&p) {
                    continue;
                }
                let f = self.world.t(p).farm;
                if f >= RIPE {
                    plots.insert(p, None);
                } else if f == 0 {
                    fallow = true;
                    if let Some(m) = choice {
                        plots.insert(p, Some(m));
                    }
                }
            }
        }
        if plots.is_empty() {
            if fallow {
                self.warn("noseed", "Farmers have no seed that grows on your farms this season.", (230, 200, 90));
            }
            return false;
        }
        if let Some((p, path)) = path::search(&self.world, d.pos, SEARCH_LIMIT, Mover::Dwarf, |p| plots.contains_key(&p)) {
            match plots[&p] {
                None => {
                    self.claimed.insert(p);
                    d.task = Task::Harvest { pos: p };
                    Self::set_path(d, path);
                }
                Some(m) => {
                    let Some((seed, _)) = self.find_item(d.pos, |it| it.kind == ItemKind::Seed && it.mat == m) else { return false };
                    self.claimed.insert(p);
                    self.claim_item(seed, d.id);
                    d.task = Task::Plant { pos: p, seed };
                }
            }
            return true;
        }
        false
    }

    /// Cycle what a farm grows: any suitable crop, then each crop in turn.
    pub fn cycle_farm_crop(&mut self, zone: u32) -> Option<String> {
        let z = self.zones.get_mut(&zone).filter(|z| z.kind == ZoneKind::Farm)?;
        let i = z.crop.and_then(|c| crate::material::CROPS.iter().position(|&m| m == c));
        z.crop = match i {
            None => Some(crate::material::CROPS[0]),
            Some(i) if i + 1 < crate::material::CROPS.len() => Some(crate::material::CROPS[i + 1]),
            Some(_) => None,
        };
        Some(match z.crop {
            Some(m) => {
                let c = m.crop().unwrap();
                format!("This farm will grow {} ({}; {}).", c.plant, c.where_when(), c.uses())
            }
            None => "This farm will grow whatever suits the season.".into(),
        })
    }

    fn stockpile_accepts(&self, p: Pos, kind: ItemKind, mat: Mat) -> bool {
        self.zones.get(&self.world.t(p).zone).map_or(false, |z| z.takes(kind, mat))
    }

    pub fn contents(&self, container: u32) -> usize {
        self.items.values().filter(|it| it.container == Some(container)).count()
    }

    fn find_haul(&mut self, d: &mut Dwarf) -> bool {
        // Idle dwarves ask constantly; a failed search holds for a moment.
        if self.tick < self.haul_retry || !self.zones.values().any(|z| matches!(z.kind, ZoneKind::Stockpile(_))) {
            return false;
        }
        let found = self.search_haul(d);
        if !found {
            self.haul_retry = self.tick + 40;
        }
        found
    }

    fn search_haul(&mut self, d: &mut Dwarf) -> bool {
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
        let mut free: Vec<(Pos, &Zone)> = Vec::new();
        for z in self.zones.values() {
            if let ZoneKind::Stockpile(_) = z.kind {
                for &p in &z.tiles {
                    if !occupied.contains(&p) && !self.reserved.contains(&p) && self.world.walkable(p) {
                        free.push((p, z));
                    }
                }
            }
        }
        // Stockpile tiles a container could be set down on, swallowing the loose goods there.
        let mut on_tile: HashMap<Pos, Vec<&Item>> = HashMap::new();
        for it in self.items.values().filter(|it| it.carried_by.is_none() && it.container.is_none()) {
            on_tile.entry(it.pos).or_default().push(it);
        }
        let absorb_ok = |p: Pos, ck: ItemKind| {
            !self.reserved.contains(&p)
                && on_tile.get(&p).map_or(false, |v| {
                    v.len() < CONTAINER_CAP && v.iter().all(|it| it.claimed_by.is_none() && !it.forbidden && it.kind.stored_in() == Some(ck))
                })
        };
        let pile_at = |p: Pos| self.zones.get(&self.world.t(p).zone).filter(|z| matches!(z.kind, ZoneKind::Stockpile(_)));
        let box_for = |kind: ItemKind, mat: Mat| {
            boxes
                .iter()
                .filter(move |(p, (_, ck))| kind.stored_in() == Some(*ck) && pile_at(**p).map_or(false, |z| z.takes(kind, mat)))
                .map(|(p, (id, _))| (*p, *id))
        };
        let spare_box = self.items.values().any(|it| it.kind.is_container() && it.carried_by.is_none() && !fill.contains_key(&it.id));
        if free.is_empty() && boxes.is_empty() && !spare_box {
            return false;
        }
        // Work out once per kind and material (not once per item) where things could go.
        let kinds: HashSet<(ItemKind, Mat)> = self.items.values().map(|it| (it.kind, it.mat)).collect();
        let has_free: HashSet<(ItemKind, Mat)> =
            kinds.iter().copied().filter(|&(k, m)| free.iter().any(|(_, z)| z.takes(k, m))).collect();
        let boxable: HashSet<(ItemKind, Mat)> =
            kinds.iter().copied().filter(|&(k, m)| k.stored_in().is_some() && box_for(k, m).next().is_some()).collect();
        let box_room: HashSet<(ItemKind, Mat)> = kinds
            .iter()
            .copied()
            .filter(|&(k, m)| k.is_container() && on_tile.keys().any(|p| absorb_ok(*p, k) && pile_at(*p).map_or(false, |z| z.takes(k, m))))
            .collect();
        let haulable = |it: &Item| {
            if it.to_depot || it.dump || it.container.is_some() || (it.kind.is_container() && fill.contains_key(&it.id)) {
                return false;
            }
            let km = (it.kind, it.mat);
            let stored = self.stockpile_accepts(it.pos, it.kind, it.mat);
            // Loose goods already in their stockpile still move into a container if one has room.
            let could_box = boxable.contains(&km);
            (!stored && (has_free.contains(&km) || could_box || box_room.contains(&km))) || (stored && could_box)
        };
        let Some((item, path)) = self.find_item(d.pos, haulable) else { return false };
        let (ipos, kind, mat) = {
            let it = &self.items[&item];
            (it.pos, it.kind, it.mat)
        };
        let into: HashMap<Pos, u32> = box_for(kind, mat).collect();
        let dests: HashSet<Pos> = free.iter().filter(|(_, z)| z.takes(kind, mat)).map(|(p, _)| *p).collect();
        let stored_now = self.stockpile_accepts(ipos, kind, mat);
        let found = path::search(&self.world, ipos, SEARCH_LIMIT, Mover::Dwarf, |p| {
            into.contains_key(&p)
                || (!stored_now && dests.contains(&p))
                || (!stored_now && kind.is_container() && absorb_ok(p, kind) && pile_at(p).map_or(false, |z| z.takes(kind, mat)))
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
            Task::BuildTile { con, item, stand } => self.do_build_tile(d, con, item, stand),
            Task::PullLever { bld } => self.do_pull_lever(d, bld),
            Task::Treat { patient, what, item } => self.do_treat(d, patient, what, item),
            Task::Bury { item, coffin } => self.do_bury(d, item, coffin),
            Task::Slaughter { creature } => self.do_slaughter(d, creature),
            Task::Punish { culprit } => self.do_punish(d, culprit),
            Task::Wander { target } => match self.travel(d, target) {
                Travel::Moving => {}
                Travel::Arrived => d.task = Task::Idle,
                Travel::Failed => {
                    // Probably an unreachable meeting area; don't hammer the pathfinder.
                    d.task = Task::Idle;
                    d.idle_cd = 150;
                }
            },
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
                    Travel::Arrived if matches!(desig, Desig::Smooth | Desig::Engrave) => {
                        let base = if desig == Desig::Smooth { 60 } else { 140 };
                        if Self::labor(d, base, d.skill(Labor::Engraving)) {
                            self.finish_detail(d, target);
                            d.task = Task::Idle;
                        }
                    }
                    Travel::Arrived => {
                        let labor = if desig == Desig::Chop { Labor::Woodcutting } else { Labor::Mining };
                        let base = if desig == Desig::Chop { 80 } else { self.world.t(target).mat.dig_ticks() };
                        if Self::labor(d, base, d.skill(labor)) {
                            let gem = self.world.t(target).kind == TileKind::Wall && self.world.t(target).mat.is_gem();
                            self.complete_dig(target);
                            if gem {
                                self.maybe_whispering_stone(d.id, target);
                            }
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
                        // A barrel or bin set down on loose goods gathers them up.
                        let kind = self.items.get(&item).map(|it| it.kind);
                        if kind.map_or(false, |k| k.is_container()) {
                            let ck = kind.unwrap();
                            let mut room = CONTAINER_CAP - self.contents(item);
                            for it in self.items.values_mut() {
                                if room == 0 {
                                    break;
                                }
                                if it.pos == dest && it.id != item && it.carried_by.is_none() && it.container.is_none()
                                    && it.claimed_by.is_none() && !it.forbidden && it.kind.stored_in() == Some(ck)
                                {
                                    it.container = Some(item);
                                    room -= 1;
                                }
                            }
                        }
                        self.reserved.remove(&dest);
                        d.gain_xp(Labor::Hauling, 5);
                        d.task = Task::Idle;
                    }
                },
            },
            Task::Construct { bld, item } => {
                let Some(site) = self.buildings.get(&bld).map(|b| self.work_spot(b.center())) else {
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
                                let (ikind, mat, quality) =
                                    self.items.remove(&item).map_or((ItemKind::Log, Mat::Wood, 0), |it| (it.kind, it.mat, it.quality));
                                d.carrying = None;
                                d.gain_xp(Labor::Building, 20);
                                d.task = Task::Idle;
                                let mut done = false;
                                if let Some(b) = self.buildings.get_mut(&bld) {
                                    b.worker = None;
                                    // The first part decides what it's made of; the last one arms a trap.
                                    if b.parts == 0 {
                                        b.mat = mat;
                                        b.quality = quality;
                                    }
                                    b.parts += 1;
                                    if b.parts >= b.kind.parts() {
                                        b.built = true;
                                        b.load = Some((ikind, mat, quality));
                                        done = true;
                                        let pos = b.pos;
                                        match b.kind {
                                            BuildingKind::Door => self.world.t_mut(pos).door = DOOR_OPEN,
                                            BuildingKind::Bridge => self.world.t_mut(pos).door = world::DOOR_NONE,
                                            // A new floodgate starts shut, and the builder steps off it.
                                            BuildingKind::Floodgate => {
                                                b.locked = true;
                                                self.world.t_mut(pos).door = world::FLOODGATE_SHUT;
                                                d.pos = self.work_spot_off(pos);
                                                d.path.clear();
                                                self.clear_tile(pos);
                                                self.dig_version += 1;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                if done {
                                    let name = self.buildings[&bld].name();
                                    self.msg(&format!("{} has been completed.", name), (160, 220, 255));
                                    if self.buildings[&bld].kind == BuildingKind::Monument {
                                        self.dedicate_monument(bld);
                                    }
                                }
                            }
                        }
                    },
                }
            }
            Task::Craft { bld, items, delivered } => {
                let Some(site) = self.buildings.get(&bld).map(|b| self.work_spot(b.center())) else {
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
                    d.satisfy(dwarf::NeedKind::Craft, 350);
                    d.task = Task::Idle;
                    let mut made: Vec<String> = Vec::new();
                    let quality = self.roll_quality(level * d.focus().max(0) as u32 / 100);
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
                    if quality == 5 {
                        let text = format!("The first masterwork: {}", made.first().cloned().unwrap_or_default());
                        self.milestone("masterwork", &text);
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
            Task::Gather { pos } => self.do_gather(d, pos),
            Task::Fish { stand } => self.do_fish(d, stand),
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
                                let crop = self.world.t(pos).crop.unwrap_or(Mat::Stoutcap);
                                let t = self.world.t_mut(pos);
                                t.farm = 0;
                                t.crop = None;
                                let n = 1 + (d.skill(Labor::Farming) >= 3) as u32 + self.rng.chance(0.5) as u32;
                                for _ in 0..n {
                                    self.spawn_item(ItemKind::Plant, crop, pos);
                                }
                            } else {
                                let crop = d.carrying.take().and_then(|seed| self.items.remove(&seed)).map_or(Mat::Stoutcap, |it| it.mat);
                                let t = self.world.t_mut(pos);
                                t.farm = 1;
                                t.crop = Some(crop);
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
                        let (kind, mat) = self.items.remove(&item).map_or((ItemKind::Plant, Mat::Stoutcap), |it| (it.kind, it.mat));
                        d.carrying = None;
                        d.hunger = 0;
                        d.task = Task::Idle;
                        self.eat_thoughts(d, kind, mat, None);
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
                        d.satisfy(dwarf::NeedKind::Drink, 500);
                        if self.zone_kind_at(d.pos) == Some(ZoneKind::Tavern) {
                            d.satisfy(dwarf::NeedKind::Party, 150);
                            d.think("drank in the tavern", -6, self.tick);
                        }
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
                        if self.beside_well(d.pos) {
                            d.think("drank clean water from the well; there was no ale", 6, self.tick);
                        } else {
                            d.think("had to drink water; there was no ale", 12, self.tick);
                        }
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
                                let (kind, mat) = self.items.remove(&item).map_or((ItemKind::Plant, Mat::Stoutcap), |it| (it.kind, it.mat));
                                d.carrying = None;
                                d.hunger = 0;
                                d.task = Task::Idle;
                                self.eat_thoughts(d, kind, mat, Some(seat));
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
                    d.satisfy(dwarf::NeedKind::Fight, 300);
                    d.task = Task::Idle;
                } else if d.work % 2 == 0 {
                    d.fight_xp += self.drill_xp(d);
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
            // Detailing goes through finish_detail, never here.
            Desig::None | Desig::Smooth | Desig::Engrave => {}
        }
        let tt = self.world.t_mut(q);
        tt.desig = Desig::None;
        // Digging cuts away any smoothing or engraving.
        tt.smooth = 0;
        tt.engraving = 0;
        if tt.kind == TileKind::Open {
            tt.farm = 0;
        }
        self.designations.remove(&q);
        self.claimed.remove(&q);
        if was_wall && t.desig != Desig::Chop {
            self.world.t_mut(q).dug = true;
            self.check_ceiling(q);
        }
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
        let wet = std::iter::once(q).chain(DIRS4.iter().map(|&(dx, dy)| q.offset(dx, dy, 0))).any(|p| self.world.get(p).map_or(false, |t| t.aquifer));
        if wet && self.world.aquifer > 0 {
            self.warn("aquifer", "You have breached an aquifer! Water is seeping in.", (90, 160, 255));
        }
        let broke_in = self.world.reveal_around(q) && !self.world.outside(q);
        if broke_in {
            self.check_vault();
        }
        if broke_in && self.near_magma(q) {
            self.warn(
                "magmasea",
                "You have breached the magma sea! Magma workshops can be built over it.",
                (255, 120, 40),
            );
            self.pause_request = true;
        } else if broke_in {
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
            ItemKind::Barrel => f.accepts(ItemKind::Plant),
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
        // This site has a heavy aquifer; this test is about ordinary digging.
        g.world.aquifer = 0;
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
        assert!(planted > 0 || count(&g, ItemKind::Plant) > 0, "farm untouched");
        // Stockpile used.
        let stocked = g.items.values().filter(|it| g.stockpile_accepts(it.pos, it.kind, it.mat)).count();
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
    fn large_world_and_far_corner_embark() {
        use crate::overworld::{WorldGen, WorldParams};
        let t = std::time::Instant::now();
        let ow = WorldGen::new(WorldParams { seed: 11, size: 2, ..WorldParams::default() }).finish();
        let took = t.elapsed();
        eprintln!("large world: {:?}, {} civs, {} sites, {} regions", took, ow.civs.len(), ow.sites.len(), ow.regions.len());
        assert_eq!((ow.w, ow.h), (256, 128));
        assert!(took.as_secs_f32() < 10.0, "large world took {:?}", took);
        assert!(ow.civs.len() >= 20, "too few civilizations for a large world");
        assert!(ow.regions.iter().any(|r| r.kind == crate::overworld::RegionKind::Mountains), "no named mountains");
        assert!(ow.regions.iter().all(|r| !r.name.is_empty()));
        let names: std::collections::HashSet<&str> = ow.regions.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names.len(), ow.regions.len(), "region names repeat");
        // The embarkable cell nearest the bottom-right corner.
        let (x, y) = (0..ow.h)
            .flat_map(|y| (0..ow.w).map(move |x| (x, y)))
            .filter(|&(x, y)| ow.embarkable(x, y))
            .max_by_key(|&(x, y)| x + y)
            .unwrap();
        assert!(x + y > (ow.w + ow.h) / 2, "nothing embarkable out in the far half");
        let mut g = Game::embark(11, ow, x, y);
        assert_eq!(g.alive_count(), 7);
        for _ in 0..DAY {
            g.step();
        }
        assert_eq!(g.alive_count(), 7);
    }

    #[test]
    fn smallest_and_largest_maps_run_a_season() {
        use crate::world::MapSize;
        for (i, seed) in [(0usize, 61u64), (2, 62)] {
            let ow = Overworld::generate(seed);
            let (x, y) = ow.default_site();
            let size = MapSize::ALL[i];
            let t = std::time::Instant::now();
            let mut g = Game::embark_with(seed, ow, x, y, size, &|_, _| {});
            assert_eq!((g.world.w, g.world.h, g.world.d), (size.w, size.h, size.d));
            assert!(g.dwarves.iter().all(|d| g.world.walkable(d.pos)), "{:?}: dwarves embarked inside rock", size);
            if size.d > 32 {
                assert_eq!(g.world.caverns.len(), 3, "deep maps have three cavern layers");
            }
            let e = g.embark;
            g.designate(e.offset(-10, -10, -1), e.offset(10, 10, 1), Desig::Chop);
            g.add_zone(e.offset(-6, 3, -1), e.offset(-1, 7, 1), ZoneKind::Stockpile(StockFilter::All));
            let mut farm = 0;
            for (dx, dy) in [(-10, -10), (2, -10), (-10, 10), (2, 10)] {
                if farm < 20 {
                    farm += g.add_zone(e.offset(dx, dy, -2), e.offset(dx + 7, dy + 5, 2), ZoneKind::Farm);
                }
            }
            assert!(farm > 0, "{:?}: no farmable soil", size);
            for _ in 0..SEASON {
                g.step();
            }
            eprintln!("{:?}: embark + season in {:?}, {} alive", size, t.elapsed(), g.alive_count());
            let dead: Vec<&str> = g.log.iter().filter(|l| l.color == (255, 60, 60)).map(|l| l.text.as_str()).collect();
            assert!(g.dwarves.iter().all(|d| d.alive), "{:?}: deaths in the first season: {:?}", size, dead);
            assert!(t.elapsed().as_secs() < 20, "{:?}: a season took {:?}", size, t.elapsed());
        }
    }

    #[test]
    fn saves_are_run_length_encoded() {
        let g = Game::new(9);
        let s = serde_json::to_string(&g).unwrap();
        assert!(s.len() < 4_000_000, "save is {} bytes", s.len());
        let g2: Game = serde_json::from_str(&s).unwrap();
        assert!(g2.world.tiles == g.world.tiles);
        assert_eq!(g2.world.caverns, g.world.caverns);
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
    fn one_year_timing_large() {
        let ow = Overworld::generate(5);
        let (x, y) = ow.default_site();
        let mut g = Game::embark_with(5, ow, x, y, crate::world::MapSize::ALL[2], &|_, _| {});
        let e = g.embark;
        g.designate(e.offset(-15, -15, -1), e.offset(15, 15, 1), Desig::Chop);
        g.add_zone(e.offset(-8, 3, -1), e.offset(-1, 9, 1), ZoneKind::Stockpile(StockFilter::All));
        g.add_zone(e.offset(-8, -8, -1), e.offset(-2, -3, 1), ZoneKind::Farm);
        let t = std::time::Instant::now();
        for _ in 0..YEAR {
            g.step();
        }
        eprintln!("large map, 1 year: {:?}, alive {}/{}, items {}", t.elapsed(), g.alive_count(), g.dwarves.len(), g.items.len());
        for l in g.log.iter().filter(|l| l.color == (255, 60, 60)).take(8) {
            eprintln!("  t{} {}", l.tick, l.text);
        }
        eprintln!("  creatures: {} ({} tame)", g.creatures.len(), g.creatures.values().filter(|c| c.tame).count());
        let s = serde_json::to_string(&g).unwrap();
        eprintln!("large map save: {} bytes", s.len());
    }

    /// A megabeast against a fresh fort with three spear-armed soldiers.
    #[test]
    #[ignore]
    fn megabeast_fight() {
        for seed in [81u64, 82, 83, 84, 85, 86, 87, 88] {
            let mut g = Game::new(seed);
            g.creatures.retain(|_, c| c.tame);
            while g.alive_count() < 20 {
                g.migrants();
            }
            for i in 0..6 {
                for k in [ItemKind::Spear, ItemKind::Helm, ItemKind::Breastplate] {
                    g.spawn_item(k, Mat::Iron, g.embark);
                }
                let id = g.dwarves[2 + i].id;
                g.toggle_soldier(id);
            }
            for _ in 0..DAY / 2 {
                g.step();
            }
            let Some(f) = g.overworld.figures.iter().find(|f| f.lair.is_some() && f.alive()).map(|f| f.id) else { continue };
            let Some(id) = g.spawn_megabeast(f) else { continue };
            let mut t = 0;
            while t < 6 * DAY && g.creatures.contains_key(&id) {
                g.step();
                t += 1;
            }
            let dead = g.dwarves.iter().filter(|d| !d.alive).count();
            eprintln!("BEAST seed {} {}: after {} ticks, beast {}, dwarves dead {}/{}", seed,
                g.overworld.figures[f as usize].name, t, if g.creatures.contains_key(&id) { "alive" } else { "gone" }, dead, g.dwarves.len());
        }
    }

    #[test]
    #[ignore]
    fn peaceful_year() {
        let mut g = Game::new(5);
        let e = g.embark;
        g.designate(e.offset(-15, -15, -1), e.offset(15, 15, 1), Desig::Chop);
        g.add_zone(e.offset(-8, 3, -1), e.offset(-1, 9, 1), ZoneKind::Stockpile(StockFilter::All));
        g.add_zone(e.offset(-8, -8, -1), e.offset(-2, -3, 1), ZoneKind::Farm);
        for i in 0..YEAR {
            g.step();
            if i % 50 == 0 {
                g.creatures.retain(|_, c| !c.hostile());
            }
        }
        let alive: Vec<&Dwarf> = g.dwarves.iter().filter(|d| d.alive).collect();
        let n = alive.len().max(1) as i32;
        eprintln!("PEACE alive {}/{} tantrums {} mean stress {} items {} drinks {} meals {}", alive.len(), g.dwarves.len(),
            g.log.iter().filter(|l| l.text.ends_with("is throwing a tantrum!")).count(),
            alive.iter().map(|d| d.stress).sum::<i32>() / n, g.items.len(),
            g.items.values().filter(|it| it.kind == ItemKind::Drink).count(),
            g.items.values().filter(|it| it.kind == ItemKind::Meal).count());
        eprintln!("PEACE married {} expecting {} children {} ghosts {}",
            g.dwarves.iter().filter(|d| d.alive && d.spouse.is_some()).count(),
            g.dwarves.iter().filter(|d| d.due.is_some()).count(),
            g.dwarves.iter().filter(|d| d.parents.is_some()).count(),
            g.dwarves.iter().filter(|d| d.ghost).count());
    }

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
        for l in g.log.iter().filter(|l| l.color == (255, 60, 60)).take(8) {
            eprintln!("  t{} {}", l.tick, l.text);
        }
        let tantrums = g.log.iter().filter(|l| l.text.ends_with("is throwing a tantrum!")).count();
        let alive: Vec<&Dwarf> = g.dwarves.iter().filter(|d| d.alive).collect();
        let n = alive.len().max(1) as i32;
        eprintln!(
            "  tantrums {}, mean stress {}, mean focus {}%, crafts {}",
            tantrums,
            alive.iter().map(|d| d.stress).sum::<i32>() / n,
            alive.iter().map(|d| d.focus()).sum::<i32>() / n,
            g.items.values().filter(|it| it.kind == ItemKind::Craft).count()
        );
        for d in &alive {
            let unmet: Vec<String> = d.needs.iter().filter(|x| x.level <= dwarf::NEED_UNMET).map(|x| format!("{}{}", x.kind.name(), x.strength)).collect();
            eprintln!("    {:<22} stress {:>4} focus {:>3}% unmet {:?}", d.name, d.stress, d.focus(), unmet);
        }
        for l in g.log.iter().rev().take(12) {
            eprintln!("  {}", l.text);
        }
    }
}

#[cfg(test)]
mod traffic_tests {
    use super::*;

    /// Busy fortress with long one-tile corridors: nobody should stand still for long
    /// while they have somewhere to go.
    #[test]
    fn no_dwarf_stalls() {
        let mut g = Game::new(42);
        let e = g.embark;
        g.designate(e.offset(-12, -12, -2), e.offset(12, 12, 2), Desig::Chop);
        let s = (1..10).map(|i| e.offset(i, 0, 0)).find(|&p| g.world.t(p).kind == TileKind::Floor).unwrap();
        g.designate(s, s.offset(0, 0, -6), Desig::StairUD);
        let b = s.offset(0, 0, -6);
        // Long 1-wide corridors with rooms at the ends.
        g.designate(b.offset(1, 0, 0), b.offset(25, 0, 0), Desig::Dig);
        g.designate(b.offset(26, -3, 0), b.offset(32, 3, 0), Desig::Dig);
        g.designate(b.offset(0, 1, 0), b.offset(0, 20, 0), Desig::Dig);
        g.designate(b.offset(-3, 21, 0), b.offset(3, 26, 0), Desig::Dig);
        g.add_zone(b.offset(26, -3, 0), b.offset(32, 3, 0), ZoneKind::Stockpile(StockFilter::All));
        g.add_zone(e.offset(-8, -8, -1), e.offset(-2, -3, 1), ZoneKind::Farm);
        // pos at last move, ticks spent wanting to move since then, tasks seen
        let mut st: HashMap<u32, (Pos, u64, Vec<String>)> = HashMap::new();
        let mut stalls = Vec::new();
        for _ in 0..(30 * DAY) {
            g.step();
            for d in g.dwarves.iter().filter(|d| d.alive) {
                let e = st.entry(d.id).or_insert((d.pos, 0, Vec::new()));
                if e.0 != d.pos {
                    *e = (d.pos, 0, Vec::new());
                    continue;
                }
                let wants = !d.working && !d.asleep && !matches!(d.task, Task::Idle | Task::Train | Task::Station { .. });
                if wants {
                    e.1 += 1;
                    let t = format!("{:?}", d.task);
                    let t = t.split(' ').next().unwrap().to_string();
                    if e.2.last() != Some(&t) && e.2.len() < 12 {
                        e.2.push(t);
                    }
                    if e.1 == 400 {
                        stalls.push(format!("t{} {} at {:?} tasks {:?}", g.tick, d.name, d.pos, e.2));
                    }
                }
            }
        }
        assert!(stalls.is_empty(), "stalled dwarves: {:#?}", stalls);
    }

    #[test]
    fn idle_dwarves_gather_at_meeting_area() {
        let mut g = Game::new(11);
        let e = g.embark;
        let n = g.add_zone(e.offset(-8, -8, -1), e.offset(-3, -3, 1), ZoneKind::Meeting);
        assert!(n > 4, "no room for a meeting area");
        // No work designated, so once fed and watered everyone should drift there.
        run(&mut g, 3 * DAY);
        let there = g.dwarves.iter().filter(|d| d.alive && g.in_meeting_area(d.pos)).count();
        assert!(there >= 4, "only {} of {} dwarves at the meeting area", there, g.alive_count());
        let chats = g.dwarves.iter().filter(|d| d.relations.len() >= 3).count();
        assert!(chats >= 3, "dwarves at the meeting area barely talked");
    }

    fn run(g: &mut Game, ticks: u64) {
        for _ in 0..ticks {
            g.step();
        }
    }
}

#[cfg(test)]
mod fort_check {
    use super::*;
    #[test]
    #[ignore]
    fn inspect_fort() {
        let path = std::env::var("FORT").unwrap();
        let mut g: Game = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        g.after_load();
        eprintln!("vault: {:?}", g.vault.as_ref().map(|v| (&v.name, v.min)));
        let mut kinds: BTreeMap<String, (usize, usize)> = BTreeMap::new();
        for b in g.buildings.values() {
            let e = kinds.entry(b.kind.name().to_string()).or_default();
            e.0 += 1;
            e.1 += b.built as usize;
        }
        eprintln!("buildings (total, built): {:?}", kinds);
        eprintln!("wealth: {}, alive {}", g.wealth(), g.alive_count());
        let mut zk: BTreeMap<String, usize> = BTreeMap::new();
        for z in g.zones.values() { *zk.entry(z.kind.name()).or_default() += z.tiles.len(); }
        eprintln!("zone tiles: {:?}", zk);
        eprintln!("designations {} constructions {} items {} creatures {}", g.designations.len(), g.constructions.len(), g.items.len(), g.creatures.len());
        for d in g.dwarves.iter() {
            eprintln!("  {:<20} {:<16} alive {} stress {:>4} task {:?} labors {:?}", d.name, d.profession(), d.alive, d.stress,
                format!("{:?}", d.task).split(' ').next().unwrap().to_string(),
                crate::dwarf::LABORS.iter().filter(|l| d.has(**l)).map(|l| l.short()).collect::<Vec<_>>());
        }
        let mut warns: BTreeMap<String, usize> = BTreeMap::new();
        for l in &g.log { *warns.entry(l.text.chars().take(60).collect()).or_default() += 1; }
        let mut w: Vec<_> = warns.into_iter().collect();
        w.sort_by_key(|x| std::cmp::Reverse(x.1));
        eprintln!("most frequent announcements:");
        for (t, n) in w.iter().take(15) { eprintln!("  {:>4}  {}", n, t); }
        // A month forward: does anything stall or blow up?
        let t = std::time::Instant::now();
        let mut idle = 0u64;
        for _ in 0..MONTH {
            g.step();
            idle += g.dwarves.iter().filter(|d| d.alive && matches!(d.task, Task::Idle | Task::Wander { .. })).count() as u64;
        }
        eprintln!("month simulated in {:?}; avg idle dwarves {:.1}; alive {}", t.elapsed(), idle as f64 / MONTH as f64, g.alive_count());
        for l in g.log.iter().rev().take(15).collect::<Vec<_>>().into_iter().rev() { eprintln!("  {}", l.text); }
    }
}

