//! The wider world: biomes, civilizations, sites, historical figures and
//! a couple of centuries of generated history.

use crate::noise::Noise;
use crate::rng::Rng;
use ratatui::style::Color;
use serde::{Deserialize, Serialize};

pub const OW_W: i32 = 64;
pub const OW_H: i32 = 32;
pub const HISTORY_YEARS: i32 = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Biome {
    Ocean,
    Mountain,
    Hills,
    Forest,
    Grassland,
    Desert,
    Tundra,
    Swamp,
}

/// How a biome shapes the embark region.
#[derive(Clone, Copy, Debug)]
pub struct EmbarkParams {
    /// Multiplier on hill height.
    pub relief: f64,
    /// Multiplier on tree density.
    pub forest: f64,
    /// Fraction of the map that lies under water.
    pub lake: f64,
    /// Chance of hostile animals among wildlife.
    pub savagery: f64,
}

impl Biome {
    pub fn name(self) -> &'static str {
        match self {
            Biome::Ocean => "ocean",
            Biome::Mountain => "mountains",
            Biome::Hills => "hills",
            Biome::Forest => "forest",
            Biome::Grassland => "grassland",
            Biome::Desert => "desert",
            Biome::Tundra => "tundra",
            Biome::Swamp => "swamp",
        }
    }

    pub fn glyph(self) -> (char, Color) {
        match self {
            Biome::Ocean => ('≈', Color::Rgb(40, 90, 200)),
            Biome::Mountain => ('▲', Color::Rgb(170, 170, 180)),
            Biome::Hills => ('∩', Color::Rgb(150, 120, 80)),
            Biome::Forest => ('♣', Color::Rgb(40, 160, 60)),
            Biome::Grassland => ('"', Color::Rgb(120, 200, 90)),
            Biome::Desert => ('~', Color::Rgb(220, 200, 110)),
            Biome::Tundra => ('·', Color::Rgb(220, 230, 240)),
            Biome::Swamp => ('%', Color::Rgb(70, 150, 130)),
        }
    }

    pub fn params(self) -> EmbarkParams {
        let p = |relief, forest, lake, savagery| EmbarkParams { relief, forest, lake, savagery };
        match self {
            Biome::Mountain => p(1.6, 0.3, 0.03, 0.3),
            Biome::Hills => p(1.2, 0.7, 0.06, 0.2),
            Biome::Forest => p(0.8, 1.7, 0.08, 0.25),
            Biome::Grassland => p(0.5, 0.25, 0.08, 0.15),
            Biome::Desert => p(0.6, 0.05, 0.0, 0.2),
            Biome::Tundra => p(0.6, 0.2, 0.05, 0.3),
            Biome::Swamp => p(0.3, 0.9, 0.25, 0.3),
            Biome::Ocean => p(0.4, 0.2, 0.5, 0.2),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Race {
    Dwarf,
    Human,
    Elf,
    Goblin,
}

impl Race {
    pub fn plural(self) -> &'static str {
        match self {
            Race::Dwarf => "dwarves",
            Race::Human => "humans",
            Race::Elf => "elves",
            Race::Goblin => "goblins",
        }
    }

    pub fn color(self) -> Color {
        match self {
            Race::Dwarf => Color::Rgb(255, 210, 90),
            Race::Human => Color::Rgb(240, 240, 240),
            Race::Elf => Color::Rgb(120, 255, 160),
            Race::Goblin => Color::Rgb(230, 70, 70),
        }
    }

    fn home(self, b: Biome, evil: bool) -> bool {
        match self {
            Race::Dwarf => matches!(b, Biome::Mountain | Biome::Hills),
            Race::Human => matches!(b, Biome::Grassland | Biome::Forest | Biome::Hills),
            Race::Elf => b == Biome::Forest && !evil,
            Race::Goblin => evil || matches!(b, Biome::Tundra | Biome::Mountain),
        }
    }

    /// Typical lifespan in years.
    fn lifespan(self) -> i32 {
        match self {
            Race::Dwarf => 150,
            Race::Human => 70,
            Race::Elf => 400,
            Race::Goblin => 60,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SiteKind {
    Fortress,
    Town,
    Hamlet,
    Retreat,
    DarkFortress,
    Lair,
}

impl SiteKind {
    pub fn name(self) -> &'static str {
        match self {
            SiteKind::Fortress => "fortress",
            SiteKind::Town => "town",
            SiteKind::Hamlet => "hamlet",
            SiteKind::Retreat => "forest retreat",
            SiteKind::DarkFortress => "dark fortress",
            SiteKind::Lair => "lair",
        }
    }

    pub fn glyph(self) -> char {
        match self {
            SiteKind::Fortress => 'Ω',
            SiteKind::Town => '#',
            SiteKind::Hamlet => '+',
            SiteKind::Retreat => '♠',
            SiteKind::DarkFortress => 'Ψ',
            SiteKind::Lair => '&',
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cell {
    pub biome: Biome,
    pub evil: bool,
    /// Site id + 1, or 0.
    pub site: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Civ {
    pub id: u32,
    pub name: String,
    pub race: Race,
    pub alive: bool,
    pub enemies: Vec<u32>,
    pub ruler: Option<u32>,
    #[serde(default)]
    pub capital: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Site {
    pub id: u32,
    pub name: String,
    pub kind: SiteKind,
    pub civ: Option<u32>,
    pub x: i32,
    pub y: i32,
    pub pop: u32,
    pub ruined: bool,
    pub founded: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Figure {
    pub id: u32,
    pub name: String,
    /// "dwarf", "dragon", ...
    pub kind: String,
    pub civ: Option<u32>,
    pub born: i32,
    pub died: Option<i32>,
    pub cause: Option<String>,
    pub title: Option<String>,
    pub kills: u32,
    /// Lair site for a megabeast.
    pub lair: Option<u32>,
}

impl Figure {
    pub fn alive(&self) -> bool {
        self.died.is_none()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub year: i32,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Neighbor {
    pub civ: u32,
    pub race: Race,
    pub dist: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Overworld {
    pub w: i32,
    pub h: i32,
    pub cells: Vec<Cell>,
    pub civs: Vec<Civ>,
    pub sites: Vec<Site>,
    pub figures: Vec<Figure>,
    pub events: Vec<Event>,
    pub year: i32,
    pub name: String,
}

// ---------------------------------------------------------------- names

const SYL: [&[&str]; 4] = [
    // dwarf
    &["ur", "ist", "dol", "bem", "kad", "ast", "tob", "zan", "mor", "thr", "eth", "rak", "gol", "nim", "ost", "lun"],
    // human
    &["al", "bar", "cen", "dor", "ed", "far", "gar", "hal", "ia", "jor", "ken", "lor", "mar", "nor", "ra", "wyn"],
    // elf
    &["ae", "li", "na", "the", "ri", "el", "va", "sil", "lo", "mia", "ne", "ira", "fa", "lua", "si", "ten"],
    // goblin
    &["snag", "ug", "zot", "grak", "bub", "nok", "ruz", "gash", "ok", "dug", "kra", "zub", "mog", "rat", "gro", "sk"],
];

fn race_idx(r: Race) -> usize {
    match r {
        Race::Dwarf => 0,
        Race::Human => 1,
        Race::Elf => 2,
        Race::Goblin => 3,
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

fn word(rng: &mut Rng, r: Race, syllables: i32) -> String {
    let set = SYL[race_idx(r)];
    let mut s = String::new();
    for _ in 0..syllables {
        s.push_str(*rng.pick(set));
    }
    cap(&s)
}

const PLACE_A: [&str; 12] = [
    "Iron", "Stone", "Ash", "Raven", "Silver", "Oak", "Frost", "Ember", "Gloom", "Bright", "Copper", "Thorn",
];
const PLACE_B: [&str; 12] = [
    "hold", "hall", "gate", "vale", "ford", "spire", "deep", "crag", "mere", "wood", "watch", "barrow",
];
fn civ_name(rng: &mut Rng, r: Race) -> String {
    let (a, b): (&[&str], &[&str]) = match r {
        Race::Dwarf => (
            &["Kingdom", "Clans", "Hold", "Union"],
            &["of the Anvil", "of the Deep Roads", "of the Iron Oath", "of Stone and Fire", "of the Hidden Vaults", "of Ashen Banners"],
        ),
        Race::Human => (
            &["Realm", "League", "Crown", "Commonwealth"],
            &["of the Golden Sheaf", "of the Long Road", "of Silver Rivers", "of the Morning Bell", "of Seven Towers"],
        ),
        Race::Elf => (
            &["Fellowship", "Covenant", "Circle"],
            &["of Whispering Leaves", "of Silver Lanterns", "of the Last Light", "of Falling Stars", "of the Green Song"],
        ),
        Race::Goblin => (
            &["Horde", "Brood", "Dominion", "Pit"],
            &["of Broken Teeth", "of Smoke", "of Red Knives", "of the Long Night", "of Rusted Chains", "of Screams"],
        ),
    };
    format!("{} {}", rng.pick(a), rng.pick(b))
}

const BEASTS: [(&str, &str); 5] = [
    ("dragon", "wing"),
    ("titan", "fist"),
    ("hydra", "maw"),
    ("roc", "talon"),
    ("colossus", "stride"),
];

pub fn person_name(rng: &mut Rng, r: Race) -> String {
    format!("{} {}", word(rng, r, 2), word(rng, r, 2))
}

fn place_name(rng: &mut Rng, r: Race) -> String {
    match r {
        Race::Dwarf | Race::Human => format!("{}{}", rng.pick(&PLACE_A), rng.pick(&PLACE_B)),
        _ => word(rng, r, 3),
    }
}

// ---------------------------------------------------------------- generation

impl Overworld {
    pub fn cell(&self, x: i32, y: i32) -> &Cell {
        &self.cells[(y * self.w + x) as usize]
    }

    fn cell_mut(&mut self, x: i32, y: i32) -> &mut Cell {
        let w = self.w;
        &mut self.cells[(y * w + x) as usize]
    }

    pub fn site_at(&self, x: i32, y: i32) -> Option<&Site> {
        let s = self.cell(x, y).site;
        (s > 0).then(|| &self.sites[(s - 1) as usize])
    }

    pub fn civ(&self, id: u32) -> &Civ {
        &self.civs[id as usize]
    }

    pub fn embarkable(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h && self.cell(x, y).biome != Biome::Ocean && self.cell(x, y).site == 0
    }

    /// Living civilizations with a site within `r` cells, nearest first.
    pub fn neighbors(&self, x: i32, y: i32, r: i32) -> Vec<Neighbor> {
        let mut best: Vec<Neighbor> = Vec::new();
        for s in self.sites.iter().filter(|s| !s.ruined) {
            let Some(civ) = s.civ else { continue };
            if !self.civs[civ as usize].alive {
                continue;
            }
            let d = (s.x - x).abs().max((s.y - y).abs());
            if d > r {
                continue;
            }
            match best.iter_mut().find(|n| n.civ == civ) {
                Some(n) => n.dist = n.dist.min(d),
                None => best.push(Neighbor { civ, race: self.civs[civ as usize].race, dist: d }),
            }
        }
        best.sort_by_key(|n| n.dist);
        best
    }

    pub fn log(&mut self, year: i32, text: String) {
        self.events.push(Event { year, text });
    }

    fn free_spot_near(&self, rng: &mut Rng, x: i32, y: i32, r: i32, race: Race) -> Option<(i32, i32)> {
        for _ in 0..60 {
            let nx = x + rng.range(-r, r + 1);
            let ny = y + rng.range(-r, r + 1);
            if nx < 0 || ny < 0 || nx >= self.w || ny >= self.h {
                continue;
            }
            let c = self.cell(nx, ny);
            if c.site == 0 && c.biome != Biome::Ocean && race.home(c.biome, c.evil) {
                return Some((nx, ny));
            }
        }
        None
    }

    fn add_site(&mut self, rng: &mut Rng, kind: SiteKind, civ: Option<u32>, x: i32, y: i32, year: i32) -> u32 {
        let id = self.sites.len() as u32;
        let race = civ.map(|c| self.civs[c as usize].race).unwrap_or(Race::Goblin);
        let name = place_name(rng, race);
        let pop = match kind {
            SiteKind::Town | SiteKind::DarkFortress => 300,
            SiteKind::Fortress | SiteKind::Retreat => 200,
            SiteKind::Hamlet => 60,
            SiteKind::Lair => 0,
        };
        self.sites.push(Site { id, name, kind, civ, x, y, pop, ruined: false, founded: year });
        self.cell_mut(x, y).site = id + 1;
        id
    }

    fn add_figure(&mut self, rng: &mut Rng, kind: &str, race: Option<Race>, civ: Option<u32>, born: i32) -> u32 {
        let id = self.figures.len() as u32;
        let name = match race {
            Some(r) => person_name(rng, r),
            None => format!("{}{}", word(rng, Race::Goblin, 1), word(rng, Race::Dwarf, 1).to_lowercase()),
        };
        self.figures.push(Figure {
            id,
            name,
            kind: kind.to_string(),
            civ,
            born,
            died: None,
            cause: None,
            title: None,
            kills: 0,
            lair: None,
        });
        id
    }

    fn site_kind_for(race: Race, capital: bool) -> SiteKind {
        match race {
            Race::Dwarf => SiteKind::Fortress,
            Race::Human => {
                if capital {
                    SiteKind::Town
                } else {
                    SiteKind::Hamlet
                }
            }
            Race::Elf => SiteKind::Retreat,
            Race::Goblin => SiteKind::DarkFortress,
        }
    }

    fn crown(&mut self, rng: &mut Rng, civ: u32, year: i32) {
        let race = self.civs[civ as usize].race;
        let born = year - rng.range(20, 60);
        let f = self.add_figure(rng, &format!("{:?}", race).to_lowercase(), Some(race), Some(civ), born);
        let title = match race {
            Race::Dwarf => "king",
            Race::Human => "queen",
            Race::Elf => "druid",
            Race::Goblin => "warlord",
        };
        self.figures[f as usize].title = Some(format!("{} of the {}", title, self.civs[civ as usize].name));
        self.civs[civ as usize].ruler = Some(f);
        let fname = self.figures[f as usize].name.clone();
        let cname = self.civs[civ as usize].name.clone();
        self.log(year, format!("{} became {} of the {}.", fname, title, cname));
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn generate(seed: u64) -> Overworld {
        Self::generate_with(seed, &|_, _| {})
    }

    /// A stand-in before any world exists (the title screen).
    pub fn empty() -> Overworld {
        Overworld {
            w: 1,
            h: 1,
            cells: vec![Cell { biome: Biome::Ocean, evil: false, site: 0 }],
            civs: Vec::new(),
            sites: Vec::new(),
            figures: Vec::new(),
            events: Vec::new(),
            year: 0,
            name: String::new(),
        }
    }

    /// Generate, reporting each stage and how far along it is (0..1).
    pub fn generate_with(seed: u64, report: &dyn Fn(&str, f32)) -> Overworld {
        report("Raising mountains and filling the seas", 0.0);
        let mut rng = Rng::new(seed ^ 0xBADC0DE);
        let elev = Noise::new(seed ^ 0xE1E7);
        let rain = Noise::new(seed ^ 0x7A1);
        let heat = Noise::new(seed ^ 0x4EA7);
        let evil = Noise::new(seed ^ 0xE711);
        let (w, h) = (OW_W, OW_H);
        let mut cells = Vec::with_capacity((w * h) as usize);
        for y in 0..h {
            for x in 0..w {
                let (fx, fy) = (x as f64 / 14.0, y as f64 / 9.0);
                // Shape a continent: lower toward the edges.
                let edge = {
                    let dx = (x as f64 / w as f64 - 0.5).abs() * 2.0;
                    let dy = (y as f64 / h as f64 - 0.5).abs() * 2.0;
                    dx.max(dy).powi(3) * 0.35
                };
                let e = elev.fbm2(fx, fy, 5) - edge;
                let r = rain.fbm2(fx * 1.3, fy * 1.3, 3);
                let t = 1.0 - (y as f64 / h as f64) * 0.9 + (heat.fbm2(fx, fy, 2) - 0.5) * 0.4;
                let biome = if e < 0.36 {
                    Biome::Ocean
                } else if e > 0.66 {
                    Biome::Mountain
                } else if e > 0.58 {
                    Biome::Hills
                } else if t < 0.3 {
                    Biome::Tundra
                } else if r < 0.36 && t > 0.55 {
                    Biome::Desert
                } else if r > 0.64 {
                    Biome::Swamp
                } else if r > 0.5 {
                    Biome::Forest
                } else {
                    Biome::Grassland
                };
                let ev = biome != Biome::Ocean && evil.fbm2(fx * 1.6, fy * 1.6, 2) > 0.66;
                cells.push(Cell { biome, evil: ev, site: 0 });
            }
        }
        let mut ow = Overworld {
            w,
            h,
            cells,
            civs: Vec::new(),
            sites: Vec::new(),
            figures: Vec::new(),
            events: Vec::new(),
            year: HISTORY_YEARS,
            name: format!("The {} {}", word(&mut rng, Race::Elf, 2), rng.pick(&["Continent", "Expanse", "Reaches", "Wilds"])),
        };

        report("Founding the first civilizations", 0.08);
        // Civilizations and their capitals.
        let plan = [Race::Dwarf, Race::Dwarf, Race::Human, Race::Human, Race::Elf, Race::Goblin, Race::Goblin];
        for race in plan {
            let mut spot = None;
            for _ in 0..400 {
                let (x, y) = (rng.range(1, w - 1), rng.range(1, h - 1));
                let c = ow.cell(x, y);
                if c.site == 0 && race.home(c.biome, c.evil) && ow.sites.iter().all(|s| (s.x - x).abs() + (s.y - y).abs() > 8) {
                    spot = Some((x, y));
                    break;
                }
            }
            let Some((x, y)) = spot else { continue };
            let id = ow.civs.len() as u32;
            let name = civ_name(&mut rng, race);
            ow.civs.push(Civ { id, name, race, alive: true, enemies: Vec::new(), ruler: None, capital: 0 });
            let sid = ow.add_site(&mut rng, Self::site_kind_for(race, true), Some(id), x, y, 0);
            ow.civs[id as usize].capital = sid;
            let (cname, sname) = (ow.civs[id as usize].name.clone(), ow.sites[sid as usize].name.clone());
            ow.log(0, format!("The {} of the {} founded {}.", race.plural(), cname, sname));
            ow.crown(&mut rng, id, 0);
        }
        // Goblins hate everyone.
        let ids: Vec<u32> = ow.civs.iter().map(|c| c.id).collect();
        for &a in &ids {
            for &b in &ids {
                if a != b && (ow.civs[a as usize].race == Race::Goblin || ow.civs[b as usize].race == Race::Goblin)
                    && ow.civs[a as usize].race != ow.civs[b as usize].race
                {
                    ow.civs[a as usize].enemies.push(b);
                }
            }
        }
        // Megabeasts in remote lairs.
        for i in 0..3 {
            let (kind, part) = BEASTS[(i + rng.range(0, 5) as usize) % BEASTS.len()];
            let mut spot = None;
            for _ in 0..300 {
                let (x, y) = (rng.range(0, w), rng.range(0, h));
                let c = ow.cell(x, y);
                if c.site == 0 && matches!(c.biome, Biome::Mountain | Biome::Swamp | Biome::Desert | Biome::Tundra) {
                    spot = Some((x, y));
                    break;
                }
            }
            let Some((x, y)) = spot else { continue };
            let born = -rng.range(200, 900);
            let f = ow.add_figure(&mut rng, kind, None, None, born);
            let name = format!("{}{}", ow.figures[f as usize].name, part);
            ow.figures[f as usize].name = name.clone();
            let lair = ow.add_site(&mut rng, SiteKind::Lair, None, x, y, 0);
            ow.sites[lair as usize].name = format!("{}'s lair", name);
            ow.figures[f as usize].lair = Some(lair);
            ow.log(0, format!("{} the {} made its lair in the {}.", name, kind, ow.cell(x, y).biome.name()));
        }

        report("Waking ancient beasts", 0.12);
        for year in 1..=HISTORY_YEARS {
            if year % 5 == 0 {
                report(&format!("Recording history: year {} of {}", year, HISTORY_YEARS), 0.15 + 0.85 * year as f32 / HISTORY_YEARS as f32);
            }
            ow.simulate_year(&mut rng, year);
        }
        report("History is written", 1.0);
        ow
    }

    fn living_sites(&self, civ: u32) -> Vec<u32> {
        self.sites.iter().filter(|s| s.civ == Some(civ) && !s.ruined).map(|s| s.id).collect()
    }

    fn kill_figure(&mut self, f: u32, year: i32, cause: String) {
        let fig = &mut self.figures[f as usize];
        if fig.died.is_some() {
            return;
        }
        fig.died = Some(year);
        fig.cause = Some(cause);
    }

    fn simulate_year(&mut self, rng: &mut Rng, year: i32) {
        let civ_ids: Vec<u32> = self.civs.iter().filter(|c| c.alive).map(|c| c.id).collect();
        for &c in &civ_ids {
            let race = self.civs[c as usize].race;
            let sites = self.living_sites(c);
            for &s in &sites {
                let site = &mut self.sites[s as usize];
                site.pop = (site.pop as f64 * 1.015) as u32 + 1;
            }
            // Expansion.
            if !sites.is_empty() && rng.chance(0.18) {
                let from = self.sites[*rng.pick(&sites) as usize].clone();
                if let Some((x, y)) = self.free_spot_near(rng, from.x, from.y, 6, race) {
                    let sid = self.add_site(rng, Self::site_kind_for(race, false), Some(c), x, y, year);
                    let text = format!(
                        "The {} of {} founded the {} of {}.",
                        race.plural(),
                        from.name,
                        self.sites[sid as usize].kind.name(),
                        self.sites[sid as usize].name
                    );
                    self.log(year, text);
                }
            }
            // Rulers age and die.
            if let Some(r) = self.civs[c as usize].ruler {
                let fig = &self.figures[r as usize];
                if fig.died.is_some() || year - fig.born > race.lifespan() + rng.range(-10, 10) {
                    if fig.died.is_none() {
                        let name = fig.name.clone();
                        self.kill_figure(r, year, "died of old age".into());
                        self.log(year, format!("{} died of old age.", name));
                    }
                    self.crown(rng, c, year);
                }
            }
            // Masterworks.
            if race == Race::Dwarf && rng.chance(0.04) && !sites.is_empty() {
                let born = year - rng.range(40, 100);
                let smith = self.add_figure(rng, "dwarf", Some(race), Some(c), born);
                let art = crate::dwarf::artifact_name(rng);
                let what = rng.pick(&["steel axe", "gold crown", "marble statue", "iron shield", "amethyst ring"]);
                let sname = self.sites[*rng.pick(&sites) as usize].name.clone();
                let fname = self.figures[smith as usize].name.clone();
                self.log(year, format!("{} created {}, a {}, in {}.", fname, art, what, sname));
            }
        }
        // Diplomacy among the non-goblins.
        for &a in &civ_ids {
            for &b in &civ_ids {
                if a >= b || self.civs[a as usize].race == Race::Goblin || self.civs[b as usize].race == Race::Goblin {
                    continue;
                }
                let at_war = self.civs[a as usize].enemies.contains(&b);
                if !at_war && rng.chance(0.01) {
                    self.civs[a as usize].enemies.push(b);
                    self.civs[b as usize].enemies.push(a);
                    let (an, bn) = (self.civs[a as usize].name.clone(), self.civs[b as usize].name.clone());
                    self.log(year, format!("The {} declared war on the {}.", an, bn));
                } else if at_war && rng.chance(0.12) {
                    self.civs[a as usize].enemies.retain(|&e| e != b);
                    self.civs[b as usize].enemies.retain(|&e| e != a);
                    let (an, bn) = (self.civs[a as usize].name.clone(), self.civs[b as usize].name.clone());
                    self.log(year, format!("The {} and the {} made peace.", an, bn));
                }
            }
        }
        // Battles.
        for &a in &civ_ids {
            if !self.civs[a as usize].alive {
                continue;
            }
            let enemies = self.civs[a as usize].enemies.clone();
            for b in enemies {
                if !self.civs[b as usize].alive || !rng.chance(0.12) {
                    continue;
                }
                self.battle(rng, a, b, year);
            }
        }
        // Megabeasts.
        let beasts: Vec<u32> = self.figures.iter().filter(|f| f.lair.is_some() && f.alive()).map(|f| f.id).collect();
        for f in beasts {
            if !rng.chance(0.07) {
                continue;
            }
            let lair = self.sites[self.figures[f as usize].lair.unwrap() as usize].clone();
            let target = self
                .sites
                .iter()
                .filter(|s| !s.ruined && s.civ.is_some() && (s.x - lair.x).abs().max((s.y - lair.y).abs()) <= 12)
                .min_by_key(|s| (s.x - lair.x).abs() + (s.y - lair.y).abs())
                .map(|s| s.id);
            let Some(t) = target else { continue };
            let (bname, bkind) = (self.figures[f as usize].name.clone(), self.figures[f as usize].kind.clone());
            let sname = self.sites[t as usize].name.clone();
            if rng.chance(0.2) {
                let civ = self.sites[t as usize].civ.unwrap();
                let race = self.civs[civ as usize].race;
                let born = year - rng.range(20, 60);
                let hero = self.add_figure(rng, &format!("{:?}", race).to_lowercase(), Some(race), Some(civ), born);
                self.figures[hero as usize].kills += 1;
                self.figures[hero as usize].title = Some(format!("slayer of {}", bname));
                self.kill_figure(f, year, format!("slain by {}", self.figures[hero as usize].name));
                let hname = self.figures[hero as usize].name.clone();
                self.log(year, format!("{} slew {} the {} at {}.", hname, bname, bkind, sname));
            } else {
                self.figures[f as usize].kills += rng.range(5, 40) as u32;
                let protected = self.civs.iter().any(|c| c.race == Race::Dwarf && c.capital == t);
                let s = &mut self.sites[t as usize];
                s.pop = s.pop * 2 / 3;
                if !protected && (s.pop < 40 || rng.chance(0.3)) {
                    s.ruined = true;
                    self.log(year, format!("{} the {} laid waste to {}.", bname, bkind, sname));
                    self.check_extinct(year);
                } else {
                    self.log(year, format!("{} the {} attacked {} and was driven off.", bname, bkind, sname));
                }
            }
        }
    }

    fn battle(&mut self, rng: &mut Rng, att: u32, def: u32, year: i32) {
        let mine = self.living_sites(att);
        let theirs = self.living_sites(def);
        if mine.is_empty() || theirs.is_empty() {
            return;
        }
        // Strike the closest enemy site within reach.
        let mut best: Option<(i32, u32, u32)> = None;
        for &m in &mine {
            for &t in &theirs {
                let (ms, ts) = (&self.sites[m as usize], &self.sites[t as usize]);
                let d = (ms.x - ts.x).abs().max((ms.y - ts.y).abs());
                if d <= 18 && best.map_or(true, |b| d < b.0) {
                    best = Some((d, m, t));
                }
            }
        }
        let Some((_, from, target)) = best else { return };
        let a_pow = self.sites[from as usize].pop as f64 * (0.5 + rng.f64());
        let d_pow = self.sites[target as usize].pop as f64 * (0.6 + rng.f64());
        let (an, ar) = (self.civs[att as usize].name.clone(), self.civs[att as usize].race);
        let tname = self.sites[target as usize].name.clone();
        let label = format!("{} {}", rng.pick(&["The Battle of", "The Siege of", "The Sack of"]), tname);
        // A dwarven mountainhome is never taken.
        let mountainhome = self.civs[def as usize].race == Race::Dwarf && self.civs[def as usize].capital == target;
        if a_pow > d_pow && mountainhome {
            let t = &mut self.sites[target as usize];
            t.pop = t.pop * 2 / 3;
            self.log(year, format!("{}: the {} of the {} broke upon the gates of the mountainhome.", label, ar.plural(), an));
        } else if a_pow > d_pow {
            let t = &mut self.sites[target as usize];
            t.pop = t.pop / 3;
            if ar == Race::Goblin && rng.chance(0.5) {
                t.civ = Some(att);
                t.kind = SiteKind::DarkFortress;
                self.log(year, format!("{}: the {} of the {} conquered {}.", label, ar.plural(), an, tname));
            } else {
                t.ruined = true;
                self.log(year, format!("{}: the {} of the {} razed {}.", label, ar.plural(), an, tname));
            }
            // Sometimes a ruler falls.
            if rng.chance(0.25) {
                if let Some(r) = self.civs[def as usize].ruler {
                    if self.figures[r as usize].alive() {
                        let rn = self.figures[r as usize].name.clone();
                        self.kill_figure(r, year, format!("fell defending {}", tname));
                        self.log(year, format!("{} fell defending {}.", rn, tname));
                        self.civs[def as usize].ruler = None;
                    }
                }
            }
            self.check_extinct(year);
        } else {
            let f = &mut self.sites[from as usize];
            f.pop = f.pop * 4 / 5;
            self.log(year, format!("{}: the {} of the {} were repulsed.", label, ar.plural(), an));
        }
    }

    fn check_extinct(&mut self, year: i32) {
        for i in 0..self.civs.len() {
            let id = self.civs[i].id;
            if self.civs[i].alive && self.living_sites(id).is_empty() {
                self.civs[i].alive = false;
                let text = format!("The {} of the {} have vanished from the world.", self.civs[i].race.plural(), self.civs[i].name);
                self.log(year, text);
            }
        }
        for i in 0..self.civs.len() {
            if self.civs[i].alive && self.civs[i].ruler.is_none() {
                // Crowned lazily in the next year's succession pass.
            }
        }
    }

    /// A sensible default embark: open land near a dwarven civilization.
    pub fn default_site(&self) -> (i32, i32) {
        let dwarf_sites: Vec<&Site> = self
            .sites
            .iter()
            .filter(|s| !s.ruined && s.civ.map_or(false, |c| self.civs[c as usize].race == Race::Dwarf && self.civs[c as usize].alive))
            .collect();
        let mut best = None;
        let mut best_score = i32::MIN;
        for y in 0..self.h {
            for x in 0..self.w {
                if !self.embarkable(x, y) {
                    continue;
                }
                let c = self.cell(x, y);
                let mut score = match c.biome {
                    Biome::Hills | Biome::Forest => 20,
                    Biome::Mountain | Biome::Grassland => 12,
                    _ => 0,
                };
                if c.evil {
                    score -= 30;
                }
                let d = dwarf_sites.iter().map(|s| (s.x - x).abs().max((s.y - y).abs())).min().unwrap_or(40);
                score -= (d - 4).abs() * 2;
                if score > best_score {
                    best_score = score;
                    best = Some((x, y));
                }
            }
        }
        best.unwrap_or((self.w / 2, self.h / 2))
    }

    /// The dwarven civilization that sends an expedition to (x, y).
    pub fn parent_civ(&self, x: i32, y: i32) -> Option<u32> {
        self.sites
            .iter()
            .filter(|s| !s.ruined)
            .filter_map(|s| s.civ.map(|c| (c, s)))
            .filter(|(c, _)| self.civs[*c as usize].race == Race::Dwarf && self.civs[*c as usize].alive)
            .min_by_key(|(_, s)| (s.x - x).abs() + (s.y - y).abs())
            .map(|(c, _)| c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_has_history() {
        for seed in 1..4 {
            let ow = Overworld::generate(seed);
            assert!(ow.civs.len() >= 4, "too few civs");
            assert!(ow.events.len() > 50, "history too thin: {}", ow.events.len());
            let (x, y) = ow.default_site();
            assert!(ow.embarkable(x, y));
            assert!(ow.parent_civ(x, y).is_some() || ow.civs.iter().all(|c| c.race != Race::Dwarf || !c.alive));
            if seed == 1 {
                for e in ow.events.iter().rev().take(10) {
                    eprintln!("{:4}: {}", e.year, e.text);
                }
                eprintln!("{} events, {} sites, {} figures", ow.events.len(), ow.sites.len(), ow.figures.len());
            }
        }
    }
}
