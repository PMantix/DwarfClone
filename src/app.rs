use crate::buildings::BuildingKind;
use crate::dwarf::LABORS;
use crate::game::{BuildMat, ConstructKind, Game, Loadout, ZoneKind};
use crate::geom::Pos;
use crate::items::StockFilter;
use crate::jobs::{AfterSave, Job, JobDone};
use crate::overworld::{Overworld, WorldGen, WorldParams};
use crate::saves::{self, SaveMeta};
use std::path::PathBuf;
use crate::world::Desig;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use std::collections::HashSet;

pub mod arena;

pub const SPEEDS: [u32; 6] = [1, 2, 5, 10, 25, 60];
pub const SAVE_DIR: &str = "saves";
pub const TITLE_MENU: [&str; 6] = ["New world", "Continue", "Load game", "Watch a fort", "Arena", "Quit"];

/// Rows of the world settings screen.
pub const SETUP_ROWS: [&str; 8] = ["Preset", "Seed", "World size", "History", "Civilizations", "Savagery", "Minerals", "Generate world"];
pub const HISTORY_CHOICES: [i32; 5] = [50, 100, 200, 300, 500];
pub const LEVELS: [[&str; 3]; 4] =
    [["Few", "Normal", "Many"], ["Calm", "Normal", "Savage"], ["Sparse", "Normal", "Rich"], ["Small", "Medium", "Large"]];

/// (name, history, civs, savagery, minerals)
pub const PRESETS: [(&str, i32, u8, u8, u8); 4] = [
    ("Quick", 50, 0, 1, 1),
    ("Standard", 200, 1, 1, 1),
    ("Long history", 500, 2, 1, 1),
    ("Harsh", 300, 1, 2, 0),
];

pub fn preset_name(p: &WorldParams) -> &'static str {
    PRESETS
        .iter()
        .find(|(_, h, c, s, m)| *h == p.history && *c == p.civs && *s == p.savagery && *m == p.minerals)
        .map_or("Custom", |x| x.0)
}

fn random_seed() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64) % 1_000_000_000
}

pub const DESIG_KEYS: [(char, Desig); 10] = [
    ('d', Desig::Dig),
    ('h', Desig::Channel),
    ('u', Desig::StairUp),
    ('j', Desig::StairDown),
    ('i', Desig::StairUD),
    ('r', Desig::Ramp),
    ('t', Desig::Chop),
    ('s', Desig::Smooth),
    ('e', Desig::Engrave),
    ('x', Desig::None),
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ZoneTool {
    Farm,
    Meeting,
    Bedroom,
    Dormitory,
    Dining,
    Office,
    Dump,
    Barracks,
    Hospital,
    Pasture,
    Temple,
    Tavern,
    Tomb,
    Gathering,
    Jail,
    Erase,
}

impl ZoneTool {
    pub const ALL: [ZoneTool; 16] = [
        ZoneTool::Meeting,
        ZoneTool::Bedroom,
        ZoneTool::Dormitory,
        ZoneTool::Dining,
        ZoneTool::Office,
        ZoneTool::Farm,
        ZoneTool::Dump,
        ZoneTool::Barracks,
        ZoneTool::Hospital,
        ZoneTool::Pasture,
        ZoneTool::Temple,
        ZoneTool::Tavern,
        ZoneTool::Tomb,
        ZoneTool::Gathering,
        ZoneTool::Jail,
        ZoneTool::Erase,
    ];

    pub fn key(self) -> char {
        match self {
            ZoneTool::Farm => 'f',
            ZoneTool::Meeting => 'm',
            ZoneTool::Bedroom => 'b',
            ZoneTool::Dormitory => 'r',
            ZoneTool::Dining => 'h',
            ZoneTool::Office => 'o',
            ZoneTool::Dump => 'g',
            ZoneTool::Barracks => 'a',
            ZoneTool::Hospital => 'p',
            ZoneTool::Pasture => 'u',
            ZoneTool::Temple => 't',
            ZoneTool::Tavern => 'v',
            ZoneTool::Tomb => 'c',
            ZoneTool::Gathering => 'w',
            ZoneTool::Jail => 'j',
            ZoneTool::Erase => 'x',
        }
    }

    pub fn kind(self) -> Option<ZoneKind> {
        match self {
            ZoneTool::Farm => Some(ZoneKind::Farm),
            ZoneTool::Meeting => Some(ZoneKind::Meeting),
            ZoneTool::Bedroom => Some(ZoneKind::Bedroom),
            ZoneTool::Dormitory => Some(ZoneKind::Dormitory),
            ZoneTool::Dining => Some(ZoneKind::Dining),
            ZoneTool::Office => Some(ZoneKind::Office),
            ZoneTool::Dump => Some(ZoneKind::Dump),
            ZoneTool::Barracks => Some(ZoneKind::Barracks),
            ZoneTool::Hospital => Some(ZoneKind::Hospital),
            ZoneTool::Pasture => Some(ZoneKind::Pasture),
            ZoneTool::Temple => Some(ZoneKind::Temple),
            ZoneTool::Tavern => Some(ZoneKind::Tavern),
            ZoneTool::Tomb => Some(ZoneKind::Tomb),
            ZoneTool::Gathering => Some(ZoneKind::Gathering),
            ZoneTool::Jail => Some(ZoneKind::Jail),
            ZoneTool::Erase => None,
        }
    }

    pub fn name(self) -> String {
        match self.kind() {
            Some(k) => k.name(),
            None => "Remove zones".into(),
        }
    }

    /// What the zone is for, a few short lines for the sidebar.
    pub fn help(self) -> &'static [&'static str] {
        match self {
            ZoneTool::Farm => &["Mark soil floor (grass, loam, clay,", "sand) as farmland. Farmers plant", "seed that suits; k, c picks a crop."],
            ZoneTool::Meeting => &["Dwarves with nothing to do gather", "here and chat (and make friends)", "instead of loitering in corridors."],
            ZoneTool::Bedroom => &["Lay over a bed. Give it to a dwarf", "with k then o; otherwise beds are", "handed out automatically."],
            ZoneTool::Dormitory => &["Beds here are shared and never", "owned. Better than the floor, but", "nobody likes a crowded dormitory."],
            ZoneTool::Dining => &["Lay over tables and chairs. Dwarves", "eat here first; a richer hall", "makes for happier meals."],
            ZoneTool::Office => &["Nobles and post holders move in", "by themselves (or k then o).", "An office helps them work."],
            ZoneTool::Dump => &["Items marked for dumping (k, then", "d or D) are thrown here and left."],
            ZoneTool::Barracks => &["Soldiers with nothing to do train", "here, and spar with each other,", "which teaches them faster."],
            ZoneTool::Hospital => &["Lay over beds. The wounded rest", "here first; doctors (Medicine) suture,", "clean, splint and fit crutches."],
            ZoneTool::Pasture => &["Goats without a pasture move in.", "Assign other animals with k, then p.", "Kept animals breed here."],
            ZoneTool::Temple => &["Dwarves who need to pray come here", "on their breaks. Unmet needs hurt", "focus, which slows work."],
            ZoneTool::Tavern => &["A meeting hall for making merry.", "Drinking here cheers dwarves more;", "it also counts as a meeting area."],
            ZoneTool::Tomb => &["Lay over coffins (b, y). The dead", "are buried here first, which", "comforts those who mourn them."],
            ZoneTool::Gathering => &["Lay over wild shrubs. Farmers pick", "whatever grows there this season;", "shrubs bear again next season."],
            ZoneTool::Jail => &["With a jail, the sheriff sentences", "culprits to days inside instead of", "a beating. Prisoners don't work."],
            ZoneTool::Erase => &["Removes zones other than", "stockpiles (stockpiles: p, then x)."],
        }
    }
}

/// Something under the look cursor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LookEntry {
    Dwarf(u32),
    Creature(u32),
    Building(u32),
    Item(u32),
}

/// Everything at `p`, units first.
pub fn look_entries(g: &Game, p: Pos) -> Vec<LookEntry> {
    let mut v: Vec<LookEntry> = Vec::new();
    if !g.world.in_bounds(p) {
        return v;
    }
    v.extend(g.dwarves.iter().filter(|d| (d.alive || d.ghost) && d.pos == p).map(|d| LookEntry::Dwarf(d.id)));
    if g.world.t(p).revealed {
        v.extend(g.creatures.values().filter(|c| c.alive && !c.sneaking && c.pos == p).map(|c| LookEntry::Creature(c.id)));
        let b = g.world.t(p).building;
        if b != 0 {
            v.push(LookEntry::Building(b));
        }
        v.extend(
            g.items
                .values()
                .filter(|it| it.pos == p && it.carried_by.is_none() && it.container.is_none())
                .map(|it| LookEntry::Item(it.id)),
        );
    }
    v
}

#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    /// The opening menu.
    Title { sel: usize },
    /// Pick a save. `confirm` asks before deleting the selected one.
    LoadMenu { sel: usize, confirm: bool },
    /// A job is running; the loading screen shows its progress.
    Loading,
    /// World creation settings.
    WorldSetup { sel: usize },
    /// Watching the world being made; it runs a little each frame.
    WorldGen,
    Normal,
    Designate { kind: Desig, corner: Option<Pos> },
    /// `filter == None` erases zones.
    Stockpile { filter: Option<StockFilter>, corner: Option<Pos> },
    Zone { tool: ZoneTool, corner: Option<Pos> },
    /// Inspect whatever is under the cursor; `sel` picks one of several things there.
    Look { sel: usize },
    /// Fortress overview screens.
    Status { tab: usize, scroll: usize },
    /// Paint burrows. `sel` is the burrow being edited; `erase` paints it away.
    Burrows { sel: usize, corner: Option<Pos>, erase: bool },
    /// Choose who belongs to a burrow.
    BurrowMembers { burrow: usize, sel: usize },
    /// Who may work at a workshop.
    Profile { bld: u32, sel: usize },
    /// What one stockpile takes.
    PileSettings { zone: u32, sel: usize },
    /// Fortress-wide work orders; `pick` is the cursor in the recipe list
    /// while choosing a new one.
    Orders { sel: usize, pick: Option<usize> },
    /// Pick a door, bridge or floodgate for a lever to work.
    LinkLever { lever: u32 },
    /// Pick a creature for a squad to kill.
    KillTarget { squad: usize },
    /// Choose who serves in a squad.
    SquadMembers { squad: usize, sel: usize },
    /// Paint traffic levels (0 normal, 1 high, 2 low, 3 restricted).
    Traffic { level: u8, corner: Option<Pos> },
    BuildMenu,
    /// Plan constructions in a box. `kind == None` cancels plans.
    Construct { kind: Option<ConstructKind>, mat: BuildMat, corner: Option<Pos> },
    Place(BuildingKind),
    Query { bld: u32, adding: bool },
    Units { sel: usize },
    /// Labor grid: rows are dwarves, columns are labors (plus the specialist column).
    /// `detail` is the highlighted work detail.
    Labors { row: usize, col: usize, detail: usize },
    Unit { id: u32, sel: usize },
    Military,
    Nobles,
    /// Choose who holds a post (`pos` indexes `Position::ALL`); row 0 of
    /// `sel` leaves it vacant, the rest are `Game::candidates`.
    Appoint { pos: usize, sel: usize },
    /// Choose who owns a bedroom or office: row 0 is nobody, the rest are
    /// `Game::owner_candidates`.
    PickOwner { zone: u32, sel: usize },
    /// A page of the fortress's chronicle (index into `Game::chronicles`).
    Chronicle { idx: usize },
    /// How many tiles to add on each side of the map.
    GrowMap { n: i32, e: i32, s: i32, w: i32 },
    /// Past announcements: `filter` picks which, `scroll` counts back from
    /// the newest.
    Reports { filter: usize, scroll: usize },
    /// Hand the fortress to the overseer, pick its style, or take it back.
    Autoplay { sel: usize },
    /// The overseer's mind: senses, ambitions, and your nudges.
    Brain { sel: usize },
    /// Choose where a squad goes on an expedition.
    Expedition { squad: usize, sel: usize },
    /// Choose an embark site on the world map.
    WorldMap { x: i32, y: i32 },
    /// Embark here? The site's summary, then Play now, Prepare carefully
    /// or Back (`sel` is the highlighted one).
    Embark { x: i32, y: i32, sel: usize },
    /// Spend points on the expedition: rows are the founders, then the
    /// supplies and animals; `col` picks a founder's main or second skill.
    Prepare { x: i32, y: i32, sel: usize, col: usize },
    /// Browse history. `from_world` returns to the world map on Esc.
    Legends { tab: usize, scroll: usize, from_world: bool },
    /// Barter at the depot. side 0 = our goods, 1 = theirs.
    Trade { side: usize, sel: [usize; 2] },
    Help,
    ConfirmQuit,
    /// Every dwarf is dead: the fortress has fallen.
    GameOver,
    /// The arena: set up a battle and watch it (`app::arena`).
    Arena,
    /// The highlight reel, playing clip `sel` (`ui/highlights.rs`).
    Highlights { sel: usize },
    /// A dwarf's life story and portrait (`ui/life.rs`).
    Life { id: u32, scroll: usize },
}

/// Timings gathered for the performance overlay, and the last second's summary.
#[derive(Clone, Copy, Debug, Default)]
pub struct Perf {
    pub since: Option<std::time::Instant>,
    pub frames: u32,
    pub draw: f64,
    pub sim: f64,
    pub ticks: u32,
    pub searches: u64,
    pub nodes: u64,
    pub shown: Option<PerfShown>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PerfShown {
    pub fps: f64,
    /// Milliseconds a frame spends drawing and simulating.
    pub draw: f64,
    pub sim: f64,
    /// Simulation ticks a second.
    pub tps: f64,
    /// Path searches and nodes searched a second.
    pub paths: f64,
    pub nodes: f64,
    /// Milliseconds a frame in water, overseer, dwarves, creatures, the rest;
    /// and the slowest single tick.
    pub parts: [f64; 6],
}

/// A mouse drag under way: where it last was, whether it's drawing a box
/// (true) or panning the map, and whether it has moved.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drag {
    pub col: u16,
    pub row: u16,
    pub boxing: bool,
    pub moved: bool,
}

pub struct App {
    pub game: Game,
    pub cursor: Pos,
    pub view_x: i32,
    pub view_y: i32,
    pub mode: Mode,
    pub speed: usize,
    pub paused: bool,
    pub quit: bool,
    pub map_rect: Rect,
    pub flash: Option<String>,
    pub seed: u64,
    pub trade_ours: HashSet<u32>,
    pub trade_theirs: HashSet<u32>,
    /// The long job behind the loading screen, if any.
    pub job: Option<Job>,
    /// Where to go back to after an in-game save.
    pub resume: Mode,
    pub save_dir: PathBuf,
    /// The seed from the command line is used for the first new world only.
    seed_used: bool,
    /// Settings for the next world, kept between worlds.
    pub world_params: WorldParams,
    /// The world being generated on the creation screen.
    pub worldgen: Option<WorldGen>,
    /// Skip the pacing and generate as fast as possible.
    pub gen_fast: bool,
    /// The world's making played back while it's made, and how far it has
    /// played (seconds).
    pub genshow: Option<crate::genshow::Show>,
    pub show_t: f64,
    /// Index into `MapSize::ALL` for the next embark.
    /// The fortress map to embark on.
    pub embark_size: crate::world::MapSize,
    /// The expedition being prepared carefully (kept while choosing a site).
    pub loadout: Loadout,
    /// The squad selected on the military screen.
    pub squad_sel: usize,
    /// Where the mouse is, and any drag under way.
    pub hover: Option<(u16, u16)>,
    pub drag: Option<Drag>,
    /// The view was panned with the mouse: it stays where it was put and the
    /// cursor is hidden until a click or an arrow key brings it back.
    pub free_view: bool,
    /// The performance overlay (F3).
    pub perf: Option<Perf>,
    /// The world map drawn in colour, two cells to a character (`v`).
    pub world_blocks: bool,
    /// Designations as they were before each recent one (Ctrl+Z puts them back).
    pub undo: Vec<Vec<(Pos, Desig)>>,
    /// The big moment being shown, and for how many frames.
    pub spot: Option<(crate::game::spotlight::Spotlight, u32)>,
    /// Status, brain, reports, chronicle and legends open as a side pane
    /// beside the map (true) or over the whole screen.
    pub docked: bool,
    /// Where the cursor was when the pan began; moving it any other way ends the pan.
    pub pan_from: (i32, i32),
    /// The camera that tours the fort when nobody touches anything.
    pub director: crate::director::Director,
    /// Seconds since the last frame (set by the main loop; tests use the default).
    pub frame_dt: f64,
    /// The arena's screens, once opened.
    pub arena: Option<Box<arena::ArenaScreen>>,
}

impl App {
    /// Start at the title screen; nothing is generated until asked.
    pub fn new(seed: u64) -> App {
        let mut app = App::with_game(seed, Game::placeholder(Overworld::empty()));
        app.mode = Mode::Title { sel: if saves::list(&app.save_dir).is_empty() { 0 } else { 1 } };
        app
    }

    /// Straight onto the world map of a freshly generated world with a default embark (tests).
    #[cfg(test)]
    pub fn generated(seed: u64) -> App {
        let mut app = App::with_game(seed, Game::new(seed));
        app.seed_used = true;
        app
    }

    #[cfg(test)]
    pub fn with_game_for_tests(game: Game) -> App {
        App::with_game(1, game)
    }

    fn with_game(seed: u64, game: Game) -> App {
        let cursor = game.embark;
        let (x, y) = game.site;
        App {
            game,
            cursor,
            view_x: 0,
            view_y: 0,
            mode: Mode::WorldMap { x, y },
            speed: 2,
            paused: true,
            quit: false,
            map_rect: Rect::default(),
            flash: None,
            seed,
            trade_ours: HashSet::new(),
            trade_theirs: HashSet::new(),
            job: None,
            resume: Mode::Normal,
            save_dir: PathBuf::from(SAVE_DIR),
            seed_used: false,
            world_params: WorldParams { seed, ..WorldParams::default() },
            worldgen: None,
            gen_fast: false,
            genshow: None,
            show_t: 0.0,
            embark_size: crate::world::MapSize::MEDIUM,
            loadout: Loadout::prepared(),
            squad_sel: 0,
            hover: None,
            drag: None,
            free_view: false,
            spot: None,
            docked: true,
            perf: None,
            undo: Vec::new(),
            world_blocks: false,
            pan_from: (0, 0),
            director: crate::director::Director::default(),
            frame_dt: 1.0 / 30.0,
            arena: None,
        }
    }

    // ------------------------------------------------------------ jobs

    /// Advance one frame: finish jobs, run the simulation, autosave.
    pub fn frame(&mut self) {
        self.poll_job();
        if self.mode == Mode::Arena {
            return self.arena_frame();
        }
        if self.mode == Mode::WorldGen {
            if let Some(g) = self.worldgen.as_mut() {
                // The world is made a step at a time, well ahead of the show
                // that plays its making back; Space skips to the end.
                if self.gen_fast {
                    g.step(usize::MAX, 40);
                } else {
                    let t0 = std::time::Instant::now();
                    while !g.done() && t0.elapsed().as_millis() < 12 {
                        g.step(usize::MAX, 1);
                    }
                }
                if self.genshow.is_none() && g.land_ready() {
                    self.genshow = Some(crate::genshow::Show::new(g));
                }
                if let Some(s) = &self.genshow {
                    self.show_t = if self.gen_fast { s.end() } else { (self.show_t + self.frame_dt.clamp(0.0, 0.1)).min(s.ready_until(g)) };
                }
            }
            return;
        }
        if self.job.is_some() || self.game.is_placeholder() || !self.in_fortress() {
            return;
        }
        if !self.paused {
            let watching = self.game.overseer.is_some();
            for _ in 0..self.ticks_per_frame() {
                self.game.step();
                if self.game.pause_request {
                    self.game.pause_request = false;
                    // The overseer doesn't stop for news; you can.
                    if !watching {
                        self.paused = true;
                        break;
                    }
                }
            }
        }
        // The fortress has fallen.
        if self.in_fortress() && !self.game.fallen && !self.game.dwarves.is_empty() && self.game.alive_count() == 0 {
            self.game.fallen = true;
            self.game.spotlights.clear();
            self.spot = None;
            self.paused = true;
            self.mode = Mode::GameOver;
            return;
        }
        // A big moment: turn the camera to it and tell it. When the overseer
        // plays, the card goes by on its own after a few seconds.
        if let Some((_, frames)) = self.spot.as_mut() {
            *frames += 1;
            if self.game.overseer.is_some() && *frames > 100 {
                self.spot = None;
            }
        }
        if self.spot.is_none() && self.mode == Mode::Normal {
            if let Some(s) = self.game.take_spotlight() {
                if let Some(p) = s.at.filter(|p| self.game.world.in_bounds(*p)) {
                    self.free_view = false;
                    self.cursor = p;
                    self.view_x = p.x - self.map_rect.width as i32 / 2;
                    self.view_y = p.y - self.map_rect.height as i32 / 2;
                }
                self.spot = Some((s, 0));
            }
        }
        // Nobody at the keys: the camera tours the fort (src/director.rs).
        self.direct();
        if self.game.chronicle_ready {
            self.game.chronicle_ready = false;
            if matches!(self.mode, Mode::Normal) {
                self.paused = self.game.overseer.is_none();
                self.mode = Mode::Chronicle { idx: self.game.chronicles.len() - 1 };
            }
        }
        if self.game.autosave_request {
            self.game.autosave_request = false;
            self.start_save(true, AfterSave::Resume);
        }
    }

    /// Screens that belong to a running fortress (not menus, the world map or loading).
    pub fn in_fortress(&self) -> bool {
        !matches!(
            self.mode,
            Mode::Title { .. }
                | Mode::LoadMenu { .. }
                | Mode::Loading
                | Mode::WorldSetup { .. }
                | Mode::WorldGen
                | Mode::WorldMap { .. }
                | Mode::Embark { .. }
                | Mode::Prepare { .. }
                | Mode::Legends { from_world: true, .. }
                | Mode::Arena
        )
    }

    fn run_job<F>(&mut self, title: &str, work: F)
    where
        F: FnOnce(crate::jobs::Report) -> Result<JobDone, String> + Send + 'static,
    {
        self.job = Some(Job::spawn(title, work));
        self.mode = Mode::Loading;
    }

    /// Open the world settings, with a fresh seed unless the command line gave one.
    pub fn open_world_setup(&mut self) {
        if self.seed_used {
            self.world_params.seed = random_seed();
        }
        self.seed_used = true;
        self.mode = Mode::WorldSetup { sel: SETUP_ROWS.len() - 1 };
    }

    pub fn start_worldgen(&mut self) {
        self.seed = self.world_params.seed;
        self.worldgen = Some(WorldGen::traced(self.world_params));
        self.gen_fast = false;
        self.genshow = None;
        self.show_t = 0.0;
        self.mode = Mode::WorldGen;
    }

    /// Whether the world is made and its making has played to the end.
    pub fn worldgen_finished(&self) -> bool {
        let made = self.worldgen.as_ref().map_or(false, |g| g.done());
        made && self.genshow.as_ref().map_or(false, |s| self.show_t >= s.end())
    }

    fn key_world_setup(&mut self, k: KeyEvent, sel: usize) {
        let n = SETUP_ROWS.len();
        let p = &mut self.world_params;
        let step = |v: u8, right: bool| if right { (v + 1).min(2) } else { v.saturating_sub(1) };
        let horiz = matches!(k.code, KeyCode::Left | KeyCode::Right);
        let right = k.code == KeyCode::Right;
        match k.code {
            KeyCode::Esc => self.mode = Mode::Title { sel: 0 },
            KeyCode::Up => self.mode = Mode::WorldSetup { sel: (sel + n - 1) % n },
            KeyCode::Down | KeyCode::Tab => self.mode = Mode::WorldSetup { sel: (sel + 1) % n },
            KeyCode::Enter => self.start_worldgen(),
            KeyCode::Char('r') => p.seed = random_seed(),
            KeyCode::Char(c @ '0'..='9') if sel == 1 => {
                if p.seed < 100_000_000_000_000_000 {
                    p.seed = p.seed * 10 + (c as u64 - '0' as u64);
                }
            }
            KeyCode::Backspace if sel == 1 => p.seed /= 10,
            _ if horiz => match sel {
                0 => {
                    let cur = PRESETS.iter().position(|x| x.0 == preset_name(p));
                    let i = match (cur, right) {
                        (Some(i), true) => (i + 1) % PRESETS.len(),
                        (Some(i), false) => (i + PRESETS.len() - 1) % PRESETS.len(),
                        (None, _) => 1,
                    };
                    let (_, h, c, s, m) = PRESETS[i];
                    (p.history, p.civs, p.savagery, p.minerals) = (h, c, s, m);
                }
                1 => p.seed = random_seed(),
                2 => p.size = step(p.size, right),
                3 => {
                    let i = HISTORY_CHOICES.iter().position(|&h| h >= p.history).unwrap_or(2);
                    let i = if right { (i + 1).min(HISTORY_CHOICES.len() - 1) } else { i.saturating_sub(1) };
                    p.history = HISTORY_CHOICES[i];
                }
                4 => p.civs = step(p.civs, right),
                5 => p.savagery = step(p.savagery, right),
                6 => p.minerals = step(p.minerals, right),
                _ => {}
            },
            _ => {}
        }
    }

    fn key_worldgen(&mut self, k: KeyEvent) {
        let done = self.worldgen_finished();
        match k.code {
            KeyCode::Enter if done => {
                self.genshow = None;
                let ow = self.worldgen.take().unwrap().finish();
                self.game = Game::placeholder(ow);
                let (x, y) = self.game.site;
                self.mode = Mode::WorldMap { x, y };
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                // Skip to the end: the rest is made as fast as it can be.
                self.gen_fast = true;
                if let Some(s) = &self.genshow {
                    self.show_t = s.end();
                }
            }
            KeyCode::Char('r') => {
                self.world_params.seed = random_seed();
                self.start_worldgen();
            }
            KeyCode::Esc | KeyCode::Char('e') => {
                self.worldgen = None;
                self.genshow = None;
                self.mode = Mode::WorldSetup { sel: 0 };
            }
            _ => {}
        }
    }

    fn start_embark(&mut self, x: i32, y: i32, loadout: Loadout) {
        let ow = self.game.overworld.clone();
        let seed = self.seed;
        let size = self.embark_size;
        self.run_job("Embarking", move |report| Ok(JobDone::Embarked(Game::embark_prepared(seed, ow, x, y, size, &loadout, report))));
    }

    /// A fresh world and fortress, played by the overseer in a random style.
    pub fn start_watch(&mut self) {
        let seed = random_seed();
        self.seed = seed;
        let style = crate::game::Style::ALL[(seed % crate::game::Style::ALL.len() as u64) as usize];
        self.run_job("Embarking", move |report| {
            let ow = Overworld::generate(seed);
            let (x, y) = ow.default_site();
            let mut g = Game::embark_with(seed, ow, x, y, crate::world::MapSize::ALL[1], report);
            g.set_overseer(Some(style));
            Ok(JobDone::Embarked(g))
        });
    }

    /// Grow the map on a worker thread (it's a whole new map's worth of work).
    pub fn start_grow(&mut self, sides: (i32, i32, i32, i32)) {
        let mut game = std::mem::replace(&mut self.game, Game::placeholder(Overworld::empty()));
        self.run_job("Growing the map", move |report| match game.grow_map(sides, report) {
            Ok(_) => Ok(JobDone::Grown(game, (sides.3, sides.0))),
            Err(e) => {
                // Hand the fortress back unharmed.
                game.msg(&e, (240, 150, 110));
                Ok(JobDone::Grown(game, (0, 0)))
            }
        });
    }

    /// Save on a worker thread; the game is handed back when it's done.
    pub fn start_save(&mut self, autosave: bool, after: AfterSave) {
        if self.game.is_placeholder() {
            return;
        }
        if !matches!(self.mode, Mode::Loading) {
            self.resume = self.mode.clone();
        }
        let game = std::mem::replace(&mut self.game, Game::placeholder(Overworld::empty()));
        let dir = self.save_dir.clone();
        let title = if autosave { "Autosaving" } else { "Saving" };
        self.run_job(title, move |report| {
            // Hand the game back even if writing failed, so nothing is lost.
            match saves::write(&dir, &game, autosave, report) {
                Ok(meta) => Ok(JobDone::Saved(game, meta, after)),
                Err(e) => {
                    let meta = SaveMeta {
                        fortress: game.fortress_name.clone(),
                        date: e,
                        dwarves: 0,
                        world: String::new(),
                        saved_unix: 0,
                        autosave,
                        file: String::new(),
                    };
                    Ok(JobDone::Saved(game, meta, AfterSave::Resume))
                }
            }
        });
    }

    pub fn start_load(&mut self, file: String) {
        let dir = self.save_dir.clone();
        self.run_job("Loading", move |report| saves::read(&dir, &file, report).map(JobDone::Loaded));
    }

    fn poll_job(&mut self) {
        let Some(result) = self.job.as_ref().and_then(|j| j.poll()) else { return };
        self.job = None;
        match result {
            Ok(JobDone::Embarked(g)) => {
                self.game = g;
                self.cursor = self.game.embark;
                if self.game.overseer.is_some() {
                    // Watching: straight in, and quick.
                    self.paused = false;
                    self.speed = 4;
                    self.mode = Mode::Normal;
                } else {
                    self.paused = true;
                    self.mode = Mode::Help;
                }
            }
            Ok(JobDone::Saved(g, meta, after)) => {
                self.game = g;
                self.flash = Some(if meta.file.is_empty() {
                    meta.date
                } else if meta.autosave {
                    "Autosaved.".into()
                } else {
                    format!("Saved to {}.", self.save_dir.join(&meta.file).display())
                });
                match after {
                    AfterSave::Resume => self.mode = std::mem::replace(&mut self.resume, Mode::Normal),
                    AfterSave::Quit => self.quit = true,
                    AfterSave::Title => {
                        self.game = Game::placeholder(Overworld::empty());
                        self.mode = Mode::Title { sel: 1 };
                    }
                }
            }
            Ok(JobDone::Grown(g, (dx, dy))) => {
                self.game = g;
                self.cursor = self.cursor.offset(dx, dy, 0);
                self.view_x += dx;
                self.view_y += dy;
                self.paused = true;
                self.mode = Mode::Normal;
                self.flash = Some(format!("The map is now {}×{}.", self.game.world.w, self.game.world.h));
            }
            Ok(JobDone::Loaded(g)) => {
                self.game = g;
                self.cursor = self.game.embark;
                self.paused = true;
                self.mode = Mode::Normal;
                self.flash = Some("Game loaded.".into());
            }
            Err(e) => {
                self.flash = Some(e);
                self.mode = if self.game.is_placeholder() { Mode::Title { sel: 0 } } else { Mode::Normal };
            }
        }
    }

    pub fn saves(&self) -> Vec<SaveMeta> {
        saves::list(&self.save_dir)
    }

    fn key_title(&mut self, k: KeyEvent, sel: usize) {
        let n = TITLE_MENU.len();
        match k.code {
            KeyCode::Up => self.mode = Mode::Title { sel: (sel + n - 1) % n },
            KeyCode::Down => self.mode = Mode::Title { sel: (sel + 1) % n },
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => self.quit = true,
            KeyCode::Enter => match sel {
                0 => self.open_world_setup(),
                1 => match self.saves().into_iter().next() {
                    Some(m) => self.start_load(m.file),
                    None => self.flash = Some("No saved games yet.".into()),
                },
                2 => {
                    if self.saves().is_empty() {
                        self.flash = Some("No saved games yet.".into());
                    } else {
                        self.mode = Mode::LoadMenu { sel: 0, confirm: false };
                    }
                }
                3 => self.start_watch(),
                4 => self.open_arena(),
                _ => self.quit = true,
            },
            _ => {}
        }
    }

    fn key_load_menu(&mut self, k: KeyEvent, sel: usize, confirm: bool) {
        let list = self.saves();
        let n = list.len().max(1);
        let sel = sel.min(n - 1);
        if confirm {
            if matches!(k.code, KeyCode::Char('y') | KeyCode::Char('Y')) {
                if let Some(m) = list.get(sel) {
                    self.flash = Some(match saves::delete(&self.save_dir, m) {
                        Ok(()) => format!("Deleted {}.", m.file),
                        Err(e) => format!("Couldn't delete: {}", e),
                    });
                }
                if self.saves().is_empty() {
                    self.mode = Mode::Title { sel: 0 };
                    return;
                }
            }
            self.mode = Mode::LoadMenu { sel: sel.saturating_sub(1).min(self.saves().len().saturating_sub(1)), confirm: false };
            return;
        }
        match k.code {
            KeyCode::Esc => self.mode = Mode::Title { sel: 2 },
            KeyCode::Up => self.mode = Mode::LoadMenu { sel: sel.saturating_sub(1), confirm: false },
            KeyCode::Down => self.mode = Mode::LoadMenu { sel: (sel + 1).min(n - 1), confirm: false },
            KeyCode::Enter => {
                if let Some(m) = list.get(sel) {
                    self.start_load(m.file.clone());
                }
            }
            KeyCode::Char('x') | KeyCode::Char('X') | KeyCode::Delete => {
                self.mode = Mode::LoadMenu { sel, confirm: true };
            }
            _ => {}
        }
    }

    pub fn ticks_per_frame(&self) -> u32 {
        SPEEDS[self.speed]
    }

    fn clamp_cursor(&mut self) {
        let w = &self.game.world;
        self.cursor.x = self.cursor.x.clamp(0, w.w - 1);
        self.cursor.y = self.cursor.y.clamp(0, w.h - 1);
        self.cursor.z = self.cursor.z.clamp(0, w.d - 1);
    }

    /// One frame's timings from the main loop: drawing, the simulation, and
    /// how many ticks it ran.
    pub fn perf_frame(&mut self, draw: f64, sim: f64, ticks: u32) {
        let Some(p) = self.perf.as_mut() else { return };
        p.frames += 1;
        p.draw += draw;
        p.sim += sim;
        p.ticks += ticks;
        let now = std::time::Instant::now();
        let since = p.since.get_or_insert(now);
        let secs = now.duration_since(*since).as_secs_f64();
        if secs >= 1.0 {
            use std::sync::atomic::Ordering::Relaxed;
            let (s, n) = (crate::path::SEARCHES.load(Relaxed) + crate::path::ASTARS.load(Relaxed), crate::path::SEARCH_NODES.load(Relaxed) + crate::path::ASTAR_NODES.load(Relaxed));
            let f = p.frames.max(1) as f64;
            let parts = self.game.prof.replace([0.0; 6]).unwrap_or([0.0; 6]);
            p.shown = Some(PerfShown {
                fps: p.frames as f64 / secs,
                draw: p.draw / f,
                sim: p.sim / f,
                tps: p.ticks as f64 / secs,
                paths: (s - p.searches) as f64 / secs,
                nodes: (n - p.nodes) as f64 / secs,
                parts: {
                    let mut x = parts.map(|v| v / f);
                    x[4] = (x[4] - x[0] - x[1] - x[2] - x[3]).max(0.0);
                    x[5] = parts[5];
                    x
                },
            });
            p.searches = s;
            p.nodes = n;
            p.frames = 0;
            p.draw = 0.0;
            p.sim = 0.0;
            p.ticks = 0;
            p.since = Some(now);
        }
    }

    /// Screens that can sit beside the map as a side pane.
    pub fn dockable(&self) -> bool {
        matches!(self.mode, Mode::Status { .. } | Mode::Brain { .. } | Mode::Reports { .. } | Mode::Chronicle { .. } | Mode::Legends { from_world: false, .. } | Mode::Life { .. })
    }

    pub fn uses_map_cursor(&self) -> bool {
        matches!(
            self.mode,
            Mode::Normal
                | Mode::Designate { .. }
                | Mode::Stockpile { .. }
                | Mode::Zone { .. }
                | Mode::Burrows { .. }
                | Mode::Traffic { .. }
                | Mode::LinkLever { .. }
                | Mode::KillTarget { .. }
                | Mode::Construct { .. }
                | Mode::Look { .. }
                | Mode::Place(_)
                | Mode::Military
        )
    }

    fn move_cursor(&mut self, k: &KeyEvent) -> bool {
        let step = if k.modifiers.contains(KeyModifiers::SHIFT) { 10 } else { 1 };
        if self.free_view && matches!(k.code, KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right) {
            // Back from a pan: the cursor comes back in the middle of the view.
            let r = self.map_rect;
            self.cursor.x = self.view_x + r.width as i32 / 2;
            self.cursor.y = self.view_y + r.height as i32 / 2;
            self.free_view = false;
            self.clamp_cursor();
            return true;
        }
        match k.code {
            KeyCode::Up => self.cursor.y -= step,
            KeyCode::Down => self.cursor.y += step,
            KeyCode::Left => self.cursor.x -= step,
            KeyCode::Right => self.cursor.x += step,
            KeyCode::Char('<') | KeyCode::Char(',') | KeyCode::PageUp => self.cursor.z += 1,
            KeyCode::Char('>') | KeyCode::Char('.') | KeyCode::PageDown => self.cursor.z -= 1,
            _ => return false,
        }
        self.clamp_cursor();
        true
    }

    /// Pick row `i` of the list on screen, as if moved there and Enter pressed.
    fn select_row(&mut self, i: usize) {
        self.mode = match self.mode.clone() {
            Mode::Title { .. } => Mode::Title { sel: i },
            Mode::Autoplay { .. } => Mode::Autoplay { sel: i },
            Mode::Appoint { pos, .. } => Mode::Appoint { pos, sel: i },
            Mode::PickOwner { zone, .. } => Mode::PickOwner { zone, sel: i },
            Mode::Expedition { squad, .. } => Mode::Expedition { squad, sel: i },
            Mode::Highlights { .. } => Mode::Highlights { sel: i },
            other => other,
        };
        self.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    }

    /// Modes where a click and drag on the map draws a box.
    fn boxing(&self) -> bool {
        matches!(
            self.mode,
            Mode::Designate { .. } | Mode::Stockpile { .. } | Mode::Zone { .. } | Mode::Traffic { .. } | Mode::Construct { .. } | Mode::Burrows { .. }
        )
    }

    fn box_corner(&self) -> Option<Pos> {
        match &self.mode {
            Mode::Designate { corner, .. }
            | Mode::Stockpile { corner, .. }
            | Mode::Zone { corner, .. }
            | Mode::Traffic { corner, .. }
            | Mode::Construct { corner, .. }
            | Mode::Burrows { corner, .. } => *corner,
            _ => None,
        }
    }

    pub fn on_mouse(&mut self, m: MouseEvent) {
        self.director.touch();
        let r = self.map_rect;
        let (col, row) = (m.column, m.row);
        let in_map = col >= r.x && row >= r.y && col < r.x + r.width && row < r.y + r.height;
        let tile = |app: &App| Pos::new(app.view_x + col as i32 - r.x as i32, app.view_y + row as i32 - r.y as i32, app.cursor.z);
        match m.kind {
            MouseEventKind::Moved => self.hover = Some((col, row)),
            MouseEventKind::Down(MouseButton::Left) => {
                self.hover = Some((col, row));
                // Menus, lists and buttons first.
                match crate::ui::target_at(col, row) {
                    Some(crate::ui::Target::Key(k)) => return self.on_key(KeyEvent::new(k, KeyModifiers::NONE)),
                    Some(crate::ui::Target::Row(i)) => return self.select_row(i),
                    Some(crate::ui::Target::Block) => return,
                    Some(crate::ui::Target::Dwarf(id)) => return self.jump_to_dwarf(id),
                    None => {}
                }
                if !in_map || !self.uses_map_cursor() {
                    return;
                }
                if self.boxing() {
                    // Press to start a box, drag, let go to finish it.
                    self.cursor = tile(self);
                    self.clamp_cursor();
                    let starting = self.box_corner().is_none();
                    self.confirm();
                    if starting {
                        self.drag = Some(Drag { col, row, boxing: true, moved: false });
                    }
                    return;
                }
                self.drag = Some(Drag { col, row, boxing: false, moved: false });
            }
            MouseEventKind::Down(MouseButton::Middle) => self.drag = Some(Drag { col, row, boxing: false, moved: false }),
            MouseEventKind::Drag(_) => {
                self.hover = Some((col, row));
                let Some(mut d) = self.drag else { return };
                if d.boxing {
                    if in_map {
                        self.cursor = tile(self);
                        self.clamp_cursor();
                        d.moved = true;
                    }
                } else {
                    // Pan: the map follows the mouse.
                    let (dx, dy) = (col as i32 - d.col as i32, row as i32 - d.row as i32);
                    if dx != 0 || dy != 0 {
                        self.view_x -= dx;
                        self.view_y -= dy;
                        if !self.free_view {
                            self.free_view = true;
                            self.pan_from = (self.cursor.x, self.cursor.y);
                        }
                        d.col = col;
                        d.row = row;
                        d.moved = true;
                    }
                }
                self.drag = Some(d);
            }
            MouseEventKind::Up(_) => {
                let Some(d) = self.drag.take() else { return };
                if d.boxing {
                    // Let go after dragging: the box ends here. Let go without
                    // moving: a click, which marks just that tile (boxes are
                    // drawn by dragging, or with the keys).
                    if self.box_corner().is_some() {
                        if d.moved && in_map {
                            self.cursor = tile(self);
                            self.clamp_cursor();
                        }
                        self.confirm();
                    }
                } else if !d.moved && in_map {
                    // A click: move there; a second click on the same tile looks at it.
                    let p = tile(self);
                    let again = p == self.cursor && !self.free_view;
                    self.free_view = false;
                    self.cursor = p;
                    self.clamp_cursor();
                    match self.mode {
                        Mode::Normal if again => self.mode = Mode::Look { sel: 0 },
                        Mode::Look { .. } => self.mode = Mode::Look { sel: 0 },
                        Mode::Place(_) | Mode::LinkLever { .. } | Mode::KillTarget { .. } => self.confirm(),
                        _ => {}
                    }
                }
            }
            MouseEventKind::Down(MouseButton::Right) => self.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            MouseEventKind::ScrollUp if matches!(self.mode, Mode::Reports { .. }) => {
                if let Mode::Reports { filter, scroll } = self.mode {
                    self.mode = Mode::Reports { filter, scroll: scroll + 3 };
                }
            }
            MouseEventKind::ScrollDown if matches!(self.mode, Mode::Reports { .. }) => {
                if let Mode::Reports { filter, scroll } = self.mode {
                    self.mode = Mode::Reports { filter, scroll: scroll.saturating_sub(3) };
                }
            }
            MouseEventKind::ScrollUp => {
                self.cursor.z += 1;
                self.clamp_cursor();
            }
            MouseEventKind::ScrollDown => {
                self.cursor.z -= 1;
                self.clamp_cursor();
            }
            _ => {}
        }
    }

    pub fn on_key(&mut self, k: KeyEvent) {
        self.director.touch();
        self.flash = None;
        if k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL) {
            self.quit = true;
            return;
        }
        // Ctrl+Z: undo the last designation.
        if k.code == KeyCode::Char('z') && k.modifiers.contains(KeyModifiers::CONTROL) {
            if self.in_fortress() {
                self.flash = Some(match self.undo.pop() {
                    Some(was) => format!("Undone: {} tiles back as they were.", self.game.restore_designations(&was)),
                    None => "Nothing to undo.".to_string(),
                });
            }
            return;
        }
        // F3: frames per second and where the time goes.
        if k.code == KeyCode::F(3) {
            self.perf = match self.perf {
                Some(_) => {
                    self.game.prof = None;
                    None
                }
                None => {
                    self.game.prof = Some([0.0; 6]);
                    Some(Perf::default())
                }
            };
            return;
        }
        // A big moment on screen: any key goes on (Space unpauses too).
        if self.spot.is_some() && self.mode == Mode::Normal {
            self.spot = None;
            if k.code == KeyCode::Char(' ') {
                self.paused = false;
            }
            return;
        }
        // The side panes open full screen, and back.
        if k.code == KeyCode::Char('\\') && self.dockable() {
            self.docked = !self.docked;
            return;
        }
        match self.mode.clone() {
            Mode::GameOver => {
                match k.code {
                    KeyCode::Char('c') if !self.game.chronicles.is_empty() => self.mode = Mode::Chronicle { idx: self.game.chronicles.len() - 1 },
                    KeyCode::Char('l') => self.mode = Mode::Legends { tab: 0, scroll: usize::MAX, from_world: false },
                    KeyCode::Char('r') => self.mode = Mode::Reports { filter: 0, scroll: 0 },
                    KeyCode::Char('h') => self.mode = Mode::Highlights { sel: usize::MAX },
                    KeyCode::Char('t') => self.start_save(false, AfterSave::Title),
                    KeyCode::Char('w') | KeyCode::Esc => self.mode = Mode::Normal,
                    _ => {}
                }
                return;
            }
            Mode::Loading => return,
            Mode::WorldSetup { sel } => return self.key_world_setup(k, sel),
            Mode::WorldGen => return self.key_worldgen(k),
            Mode::Embark { x, y, sel } => return self.key_embark(k, x, y, sel),
            Mode::Prepare { x, y, sel, col } => return self.key_prepare(k, x, y, sel, col),
            Mode::Title { sel } => return self.key_title(k, sel),
            Mode::LoadMenu { sel, confirm } => return self.key_load_menu(k, sel, confirm),
            Mode::Arena => return self.key_arena(k),
            _ => {}
        }
        if self.uses_map_cursor() && self.move_cursor(&k) {
            if let Mode::Look { .. } = self.mode {
                self.mode = Mode::Look { sel: 0 };
            }
            return;
        }
        // Time controls work everywhere except the quit prompt and pre-embark screens.
        let pre_embark = matches!(self.mode, Mode::WorldMap { .. } | Mode::Legends { from_world: true, .. });
        if self.mode != Mode::ConfirmQuit && !pre_embark {
            match k.code {
                KeyCode::Char(' ') => {
                    self.paused = !self.paused;
                    return;
                }
                KeyCode::Char('+') | KeyCode::Char('=') => {
                    self.speed = (self.speed + 1).min(SPEEDS.len() - 1);
                    return;
                }
                KeyCode::Char('-') => {
                    self.speed = self.speed.saturating_sub(1);
                    return;
                }
                _ => {}
            }
        }
        let mode = self.mode.clone();
        match mode {
            Mode::GameOver | Mode::Arena => {}
            Mode::Normal => self.key_normal(k),
            Mode::Designate { kind, corner } => match k.code {
                KeyCode::Enter => self.confirm(),
                KeyCode::Esc => {
                    self.mode = if corner.is_some() { Mode::Designate { kind, corner: None } } else { Mode::Normal }
                }
                KeyCode::Char(c) => {
                    if let Some(&(_, d)) = DESIG_KEYS.iter().find(|(kc, _)| *kc == c) {
                        self.mode = Mode::Designate { kind: d, corner };
                    }
                }
                _ => {}
            },
            Mode::Stockpile { filter, corner } => match k.code {
                KeyCode::Enter => self.confirm(),
                KeyCode::Esc => {
                    self.mode = if corner.is_some() { Mode::Stockpile { filter, corner: None } } else { Mode::Normal }
                }
                KeyCode::Char('x') => self.mode = Mode::Stockpile { filter: None, corner },
                KeyCode::Char(c) => {
                    if let Some(&f) = StockFilter::ALL.iter().find(|f| f.key() == c) {
                        self.mode = Mode::Stockpile { filter: Some(f), corner };
                    }
                }
                _ => {}
            },
            Mode::Zone { tool, corner } => match k.code {
                KeyCode::Enter => self.confirm(),
                KeyCode::Esc => {
                    self.mode = if corner.is_some() { Mode::Zone { tool, corner: None } } else { Mode::Normal }
                }
                KeyCode::Char(c) => {
                    if let Some(&t) = ZoneTool::ALL.iter().find(|t| t.key() == c) {
                        self.mode = Mode::Zone { tool: t, corner };
                    }
                }
                _ => {}
            },
            Mode::Look { sel } => self.key_look(k, sel),
            Mode::Status { tab, scroll } => match k.code {
                KeyCode::Esc | KeyCode::Char('z') => self.mode = Mode::Normal,
                KeyCode::Tab | KeyCode::Right => self.mode = Mode::Status { tab: (tab + 1) % 5, scroll: 0 },
                KeyCode::Left => self.mode = Mode::Status { tab: (tab + 4) % 5, scroll: 0 },
                KeyCode::Char(c @ '1'..='5') => self.mode = Mode::Status { tab: c as usize - '1' as usize, scroll: 0 },
                KeyCode::Up => self.mode = Mode::Status { tab, scroll: scroll.saturating_sub(1) },
                KeyCode::Down => self.mode = Mode::Status { tab, scroll: scroll + 1 },
                KeyCode::PageUp => self.mode = Mode::Status { tab, scroll: scroll.saturating_sub(20) },
                KeyCode::PageDown => self.mode = Mode::Status { tab, scroll: scroll + 20 },
                _ => {}
            },
            Mode::Construct { kind, mat, corner } => match k.code {
                KeyCode::Enter => self.confirm(),
                KeyCode::Esc => {
                    self.mode = if corner.is_some() { Mode::Construct { kind, mat, corner: None } } else { Mode::BuildMenu }
                }
                KeyCode::Char('m') => self.mode = Mode::Construct { kind, mat: mat.next(), corner },
                KeyCode::Char('c') => self.mode = Mode::Construct { kind: None, mat, corner },
                KeyCode::Char(c) => {
                    if let Some(&ck) = ConstructKind::ALL.iter().find(|ck| ck.key() == c) {
                        self.mode = Mode::Construct { kind: Some(ck), mat, corner };
                    }
                }
                _ => {}
            },
            Mode::BuildMenu => match k.code {
                KeyCode::Esc => self.mode = Mode::Normal,
                KeyCode::Char('C') => {
                    self.mode = Mode::Construct { kind: Some(ConstructKind::Wall), mat: BuildMat::Any, corner: None }
                }
                KeyCode::Char(c) => {
                    if let Some(&b) = BuildingKind::ALL.iter().find(|b| b.key() == c) {
                        self.mode = Mode::Place(b);
                    }
                }
                _ => {}
            },
            Mode::Place(_) => match k.code {
                KeyCode::Enter => self.confirm(),
                KeyCode::Esc => self.mode = Mode::BuildMenu,
                _ => {}
            },
            Mode::Query { bld, adding } => self.key_query(k, bld, adding),
            Mode::Orders { sel, pick } => self.key_orders(k, sel, pick),
            Mode::Profile { bld, sel } => {
                let ids: Vec<u32> = self.game.dwarves.iter().filter(|d| d.alive && !d.is_child()).map(|d| d.id).collect();
                let n = ids.len().max(1);
                match k.code {
                    KeyCode::Esc => self.mode = Mode::Query { bld, adding: false },
                    KeyCode::Up => self.mode = Mode::Profile { bld, sel: sel.saturating_sub(1) },
                    KeyCode::Down => self.mode = Mode::Profile { bld, sel: (sel + 1).min(n - 1) },
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if let Some(&id) = ids.get(sel) {
                            self.game.toggle_worker(bld, id);
                        }
                    }
                    KeyCode::Char('a') => {
                        if let Some(b) = self.game.buildings.get_mut(&bld) {
                            b.workers.clear();
                        }
                        self.flash = Some("Anyone with the labor may work here.".into());
                    }
                    _ => {}
                }
            }
            Mode::PileSettings { zone, sel } => {
                let rows = self.game.pile_rows(zone);
                let n = rows.len().max(1);
                match k.code {
                    KeyCode::Esc => self.mode = Mode::Look { sel: 0 },
                    KeyCode::Up => self.mode = Mode::PileSettings { zone, sel: sel.saturating_sub(1) },
                    KeyCode::Down => self.mode = Mode::PileSettings { zone, sel: (sel + 1).min(n - 1) },
                    KeyCode::PageUp => self.mode = Mode::PileSettings { zone, sel: sel.saturating_sub(10) },
                    KeyCode::PageDown => self.mode = Mode::PileSettings { zone, sel: (sel + 10).min(n - 1) },
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if let Some(&(_, _, t)) = rows.get(sel) {
                            self.game.toggle_pile(zone, t);
                        }
                    }
                    KeyCode::Char('a') => {
                        if let Some(z) = self.game.zones.get_mut(&zone) {
                            z.deny.clear();
                            z.deny_mats.clear();
                        }
                        self.flash = Some("This stockpile takes everything in its category again.".into());
                    }
                    _ => {}
                }
            }
            Mode::Units { sel } => {
                let n = self.game.dwarves.len();
                match k.code {
                    KeyCode::Esc => self.mode = Mode::Normal,
                    KeyCode::Up => self.mode = Mode::Units { sel: sel.saturating_sub(1) },
                    KeyCode::Down => self.mode = Mode::Units { sel: (sel + 1).min(n.saturating_sub(1)) },
                    KeyCode::Enter => {
                        if let Some(d) = self.game.dwarves.get(sel) {
                            self.mode = Mode::Unit { id: d.id, sel: 0 };
                        }
                    }
                    KeyCode::Char('c') => {
                        if let Some(d) = self.game.dwarves.get(sel) {
                            self.cursor = d.pos;
                            self.mode = Mode::Normal;
                        }
                    }
                    _ => {}
                }
            }
            Mode::Unit { id, sel } => match k.code {
                KeyCode::Esc => self.mode = Mode::Normal,
                KeyCode::Char('c') => {
                    if let Some(d) = self.game.dwarves.iter().find(|d| d.id == id) {
                        self.cursor = d.pos;
                    }
                    self.mode = Mode::Normal;
                }
                KeyCode::Up => self.mode = Mode::Unit { id, sel: sel.saturating_sub(1) },
                KeyCode::Down => self.mode = Mode::Unit { id, sel: (sel + 1).min(LABORS.len() - 1) },
                KeyCode::Enter => {
                    if let Some(lost) = self.game.toggle_labor(id, LABORS[sel]) {
                        self.flash = Some(format!("{} dropped: one tool labor per dwarf.", lost.name()));
                    }
                }
                KeyCode::Char('s') => {
                    let on = self.game.toggle_specialist(id);
                    self.flash = Some(if on { "Specialist: won't haul.".into() } else { "Will haul again.".into() });
                }
                KeyCode::Char('m') => self.game.toggle_soldier(id),
                KeyCode::Char('l') => self.mode = Mode::Life { id, scroll: 0 },
                _ => {}
            },
            Mode::Military => self.key_military(k),
            Mode::Highlights { sel } => self.key_highlights(k, sel),
            Mode::Life { id, scroll } => self.key_life(k, id, scroll),
            Mode::KillTarget { squad } => match k.code {
                KeyCode::Esc => self.mode = Mode::Military,
                KeyCode::Enter => {
                    let c = self.cursor;
                    let target = self
                        .game
                        .creatures
                        .values()
                        .filter(|cr| cr.alive && cr.pos.z == c.z && (cr.pos.x - c.x).abs() <= 1 && (cr.pos.y - c.y).abs() <= 1)
                        .min_by_key(|cr| (cr.pos.x - c.x).abs() + (cr.pos.y - c.y).abs())
                        .map(|cr| cr.id);
                    self.flash = Some(match target.map(|t| self.game.order_kill(squad, t)) {
                        Some(Ok(m)) => {
                            self.mode = Mode::Military;
                            m
                        }
                        Some(Err(e)) => e,
                        None => "No creature under the cursor.".into(),
                    });
                }
                _ => {}
            },
            Mode::SquadMembers { squad, sel } => {
                let ids: Vec<u32> = self.game.dwarves.iter().filter(|d| d.alive).map(|d| d.id).collect();
                let n = ids.len().max(1);
                match k.code {
                    KeyCode::Esc => self.mode = Mode::Military,
                    KeyCode::Up => self.mode = Mode::SquadMembers { squad, sel: sel.saturating_sub(1) },
                    KeyCode::Down => self.mode = Mode::SquadMembers { squad, sel: (sel + 1).min(n - 1) },
                    KeyCode::Enter => {
                        if let Some(&id) = ids.get(sel) {
                            let member = self.game.squads.get(squad).map_or(false, |s| s.members.contains(&id));
                            self.game.set_squad_member(squad, id, !member);
                        }
                    }
                    _ => {}
                }
            }
            Mode::Labors { row, col, detail } => self.key_labors(k, row, col, detail),
            Mode::Burrows { sel, corner, erase } => self.key_burrows(k, sel, corner, erase),
            Mode::LinkLever { lever } => match k.code {
                KeyCode::Esc => self.mode = Mode::Query { bld: lever, adding: false },
                KeyCode::Enter => {
                    let target = self.game.world.t(self.cursor).building;
                    self.flash = Some(match self.game.link_lever(lever, target) {
                        Ok(m) => {
                            self.mode = Mode::Query { bld: lever, adding: false };
                            m
                        }
                        Err(e) => e,
                    });
                }
                _ => {}
            },
            Mode::BurrowMembers { burrow, sel } => {
                let ids: Vec<u32> = self.game.dwarves.iter().filter(|d| d.alive).map(|d| d.id).collect();
                let n = ids.len().max(1);
                match k.code {
                    KeyCode::Esc => self.mode = Mode::Burrows { sel: burrow, corner: None, erase: false },
                    KeyCode::Up => self.mode = Mode::BurrowMembers { burrow, sel: sel.saturating_sub(1) },
                    KeyCode::Down => self.mode = Mode::BurrowMembers { burrow, sel: (sel + 1).min(n - 1) },
                    KeyCode::Enter => {
                        if let Some(&id) = ids.get(sel) {
                            self.game.toggle_burrow_member(burrow, id);
                        }
                    }
                    KeyCode::Char('a') => {
                        // Everyone in, or (if everyone already is) everyone out.
                        let all_in = self.game.burrows.get(burrow).map_or(false, |b| ids.iter().all(|i| b.members.contains(i)));
                        for &id in &ids {
                            let member = self.game.burrows.get(burrow).map_or(false, |b| b.members.contains(&id));
                            if member == all_in {
                                self.game.toggle_burrow_member(burrow, id);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Mode::Traffic { level, corner } => match k.code {
                KeyCode::Enter => self.confirm(),
                KeyCode::Esc => {
                    self.mode = if corner.is_some() { Mode::Traffic { level, corner: None } } else { Mode::Normal }
                }
                KeyCode::Char('h') => self.mode = Mode::Traffic { level: 1, corner },
                KeyCode::Char('n') => self.mode = Mode::Traffic { level: 0, corner },
                KeyCode::Char('l') => self.mode = Mode::Traffic { level: 2, corner },
                KeyCode::Char('r') => self.mode = Mode::Traffic { level: 3, corner },
                _ => {}
            },
            Mode::Help => self.mode = Mode::Normal,
            Mode::Nobles => match k.code {
                KeyCode::Char(c @ '1'..='6') => {
                    let pos = (c as u8 - b'1') as usize;
                    let p = crate::game::Position::ALL[pos];
                    let cands = self.game.candidates(p);
                    // Start on whoever holds it now, else the best suited.
                    let sel = match self.game.holder(p) {
                        Some(h) => cands.iter().position(|&id| id == h.id).map_or(0, |i| i + 1),
                        None => (!cands.is_empty()) as usize,
                    };
                    self.mode = Mode::Appoint { pos, sel };
                }
                KeyCode::Char('x') => {
                    for p in crate::game::Position::ALL {
                        self.game.appoint(p, None);
                    }
                    self.flash = Some("Every post is vacant.".into());
                }
                _ => self.mode = Mode::Normal,
            },
            Mode::Appoint { pos, sel } => {
                let p = crate::game::Position::ALL[pos];
                let cands = self.game.candidates(p);
                let n = cands.len() + 1;
                match k.code {
                    KeyCode::Esc => self.mode = Mode::Nobles,
                    KeyCode::Up => self.mode = Mode::Appoint { pos, sel: (sel + n - 1) % n },
                    KeyCode::Down => self.mode = Mode::Appoint { pos, sel: (sel + 1) % n },
                    KeyCode::PageUp | KeyCode::Home => self.mode = Mode::Appoint { pos, sel: 0 },
                    KeyCode::PageDown | KeyCode::End => self.mode = Mode::Appoint { pos, sel: n - 1 },
                    KeyCode::Enter => {
                        let who = if sel == 0 { None } else { cands.get(sel - 1).copied() };
                        self.flash = Some(self.game.appoint_msg(p, who));
                        self.mode = Mode::Nobles;
                    }
                    _ => {}
                }
            }
            Mode::GrowMap { mut n, mut e, mut s, mut w } => {
                let by = if k.modifiers.contains(KeyModifiers::SHIFT) { -16 } else { 16 };
                let (ww, wh) = (self.game.world.w, self.game.world.h);
                match k.code {
                    KeyCode::Up => n = (n + by).max(0),
                    KeyCode::Down => s = (s + by).max(0),
                    KeyCode::Right => e = (e + by).max(0),
                    KeyCode::Left => w = (w + by).max(0),
                    KeyCode::Enter if n + e + s + w > 0 => {
                        self.start_grow((n, e, s, w));
                        return;
                    }
                    _ => {
                        self.mode = Mode::Normal;
                        return;
                    }
                }
                // Keep within bounds.
                if ww + e + w > 320 {
                    e = e.min(320 - ww - w).max(0);
                }
                if wh + n + s > 256 {
                    s = s.min(256 - wh - n).max(0);
                }
                self.mode = Mode::GrowMap { n, e, s, w };
            }
            Mode::Reports { filter, scroll } => {
                let n = crate::ui::REPORT_FILTERS.len();
                let page = 20;
                self.mode = match k.code {
                    KeyCode::Up => Mode::Reports { filter, scroll: scroll + 1 },
                    KeyCode::Down => Mode::Reports { filter, scroll: scroll.saturating_sub(1) },
                    KeyCode::PageUp => Mode::Reports { filter, scroll: scroll + page },
                    KeyCode::PageDown => Mode::Reports { filter, scroll: scroll.saturating_sub(page) },
                    KeyCode::Home => Mode::Reports { filter, scroll: usize::MAX / 2 },
                    KeyCode::End => Mode::Reports { filter, scroll: 0 },
                    KeyCode::Tab | KeyCode::Right => Mode::Reports { filter: (filter + 1) % n, scroll: 0 },
                    KeyCode::Left => Mode::Reports { filter: (filter + n - 1) % n, scroll: 0 },
                    KeyCode::Char(c @ '1'..='9') if (c as usize - '1' as usize) < n => Mode::Reports { filter: c as usize - '1' as usize, scroll: 0 },
                    KeyCode::Char(' ') => {
                        self.paused = !self.paused;
                        Mode::Reports { filter, scroll }
                    }
                    _ => Mode::Normal,
                };
            }
            Mode::Brain { sel } => {
                use crate::game::Ambition;
                let n = Ambition::ALL.len();
                let a = Ambition::ALL[sel.min(n - 1)];
                match k.code {
                    KeyCode::Up => self.mode = Mode::Brain { sel: (sel + n - 1) % n },
                    KeyCode::Down => self.mode = Mode::Brain { sel: (sel + 1) % n },
                    KeyCode::Right | KeyCode::Char('+') => self.game.nudge_ambition(a, 1),
                    KeyCode::Left | KeyCode::Char('-') => self.game.nudge_ambition(a, -1),
                    KeyCode::Char('f') => {
                        self.game.force_ambition(a);
                        self.flash = Some(format!("You've talked the overseer into: {}.", a.name()));
                    }
                    KeyCode::Char('r') => self.game.clear_nudges(),
                    KeyCode::Char(' ') => self.paused = !self.paused,
                    _ => self.mode = Mode::Normal,
                }
            }
            Mode::Autoplay { sel } => {
                let n = crate::game::Style::ALL.len() + 1;
                match k.code {
                    KeyCode::Up => self.mode = Mode::Autoplay { sel: (sel + n - 1) % n },
                    KeyCode::Down => self.mode = Mode::Autoplay { sel: (sel + 1) % n },
                    KeyCode::Enter => {
                        let style = if sel == 0 { None } else { Some(crate::game::Style::ALL[sel - 1]) };
                        self.game.set_overseer(style);
                        self.mode = Mode::Normal;
                    }
                    KeyCode::Char('b') if self.game.overseer.is_some() => self.mode = Mode::Brain { sel: 0 },
                    // Another overseer of the same style: a new name and traits.
                    KeyCode::Char('r') if self.game.overseer.is_some() => {
                        let mut o = self.game.overseer.take().unwrap();
                        self.game.roll_personality(&mut o);
                        self.flash = Some(format!("The overseer is now {}.", o.title()));
                        self.game.overseer = Some(o);
                    }
                    _ => self.mode = Mode::Normal,
                }
            }
            Mode::Expedition { squad, sel } => {
                let dests = self.game.destinations();
                let n = dests.len().max(1);
                match k.code {
                    KeyCode::Esc => self.mode = Mode::Military,
                    KeyCode::Up => self.mode = Mode::Expedition { squad, sel: (sel + n - 1) % n },
                    KeyCode::Down => self.mode = Mode::Expedition { squad, sel: (sel + 1) % n },
                    KeyCode::Enter => {
                        if let Some(d) = dests.get(sel) {
                            self.flash = Some(match self.game.send_expedition(squad, d.clone()) {
                                Ok(m) | Err(m) => m,
                            });
                        }
                        self.mode = Mode::Military;
                    }
                    _ => {}
                }
            }
            Mode::Chronicle { idx } => {
                let n = self.game.chronicles.len().max(1);
                match k.code {
                    KeyCode::Left => self.mode = Mode::Chronicle { idx: idx.saturating_sub(1) },
                    KeyCode::Right => self.mode = Mode::Chronicle { idx: (idx + 1).min(n - 1) },
                    _ => self.mode = Mode::Normal,
                }
            }
            Mode::PickOwner { zone, sel } => {
                let cands = self.game.owner_candidates();
                let n = cands.len() + 1;
                match k.code {
                    KeyCode::Esc => self.mode = Mode::Look { sel: 0 },
                    KeyCode::Up => self.mode = Mode::PickOwner { zone, sel: (sel + n - 1) % n },
                    KeyCode::Down => self.mode = Mode::PickOwner { zone, sel: (sel + 1) % n },
                    KeyCode::PageUp | KeyCode::Home => self.mode = Mode::PickOwner { zone, sel: 0 },
                    KeyCode::PageDown | KeyCode::End => self.mode = Mode::PickOwner { zone, sel: n - 1 },
                    KeyCode::Enter => {
                        let who = if sel == 0 { None } else { cands.get(sel - 1).copied() };
                        let kind = self.game.zones.get(&zone).map_or("Room".to_string(), |z| z.kind.name());
                        self.game.set_zone_owner(zone, who);
                        let name = who.and_then(|id| self.game.dwarves.iter().find(|d| d.id == id)).map_or("nobody".to_string(), |d| d.name.clone());
                        self.flash = Some(format!("{} assigned to {}.", kind, name));
                        self.mode = Mode::Look { sel: 0 };
                    }
                    _ => {}
                }
            }
            Mode::WorldMap { x, y } => self.key_world(k, x, y),
            Mode::Legends { tab, scroll, from_world } => {
                let back = if from_world {
                    let (x, y) = self.game.site;
                    Mode::WorldMap { x, y }
                } else {
                    Mode::Normal
                };
                match k.code {
                    KeyCode::Esc | KeyCode::Char('q') => self.mode = back,
                    KeyCode::Tab | KeyCode::Right => self.mode = Mode::Legends { tab: (tab + 1) % 6, scroll: 0, from_world },
                    KeyCode::Left => self.mode = Mode::Legends { tab: (tab + 5) % 6, scroll: 0, from_world },
                    KeyCode::Char(c @ '1'..='6') => {
                        self.mode = Mode::Legends { tab: c as usize - '1' as usize, scroll: 0, from_world }
                    }
                    KeyCode::Up => self.mode = Mode::Legends { tab, scroll: scroll.saturating_sub(1), from_world },
                    KeyCode::Down => self.mode = Mode::Legends { tab, scroll: scroll + 1, from_world },
                    KeyCode::PageUp => self.mode = Mode::Legends { tab, scroll: scroll.saturating_sub(20), from_world },
                    KeyCode::PageDown => self.mode = Mode::Legends { tab, scroll: scroll + 20, from_world },
                    _ => {}
                }
            }
            Mode::Trade { side, sel } => self.key_trade(k, side, sel),
            Mode::ConfirmQuit => match k.code {
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.mode = Mode::Normal;
                    self.start_save(false, AfterSave::Quit);
                }
                KeyCode::Char('t') | KeyCode::Char('T') => {
                    self.mode = Mode::Normal;
                    self.start_save(false, AfterSave::Title);
                }
                KeyCode::Char('y') | KeyCode::Char('Y') => self.quit = true,
                _ => self.mode = Mode::Normal,
            },
            Mode::Title { .. } | Mode::LoadMenu { .. } | Mode::Loading | Mode::WorldSetup { .. } | Mode::WorldGen | Mode::Embark { .. } | Mode::Prepare { .. } => {}
        }
    }

    fn key_normal(&mut self, k: KeyEvent) {
        match k.code {
            KeyCode::Char('d') => self.mode = Mode::Designate { kind: Desig::Dig, corner: None },
            KeyCode::Char('p') => self.mode = Mode::Stockpile { filter: Some(StockFilter::All), corner: None },
            KeyCode::Char('i') => self.mode = Mode::Zone { tool: ZoneTool::Meeting, corner: None },
            KeyCode::Char('k') => self.mode = Mode::Look { sel: 0 },
            KeyCode::Char('z') => self.mode = Mode::Status { tab: 0, scroll: 0 },
            KeyCode::Char('b') => self.mode = Mode::BuildMenu,
            KeyCode::Char('q') => {
                let bid = self.game.world.t(self.cursor).building;
                if bid != 0 {
                    self.mode = Mode::Query { bld: bid, adding: false };
                } else {
                    self.flash = Some("No building under the cursor.".into());
                }
            }
            KeyCode::Char('u') => self.mode = Mode::Units { sel: 0 },
            KeyCode::Char('y') => self.mode = Mode::Labors { row: 0, col: 0, detail: 0 },
            KeyCode::Char('w') => self.mode = Mode::Burrows { sel: 0, corner: None, erase: false },
            KeyCode::Char('o') => self.mode = Mode::Traffic { level: 2, corner: None },
            KeyCode::Char('m') => self.mode = Mode::Military,
            KeyCode::Char('n') => self.mode = Mode::Nobles,
            KeyCode::Char('r') => self.mode = Mode::Reports { filter: 0, scroll: 0 },
            KeyCode::Char('M') => self.mode = Mode::GrowMap { n: 0, e: 0, s: 0, w: 0 },
            KeyCode::Char('B') => {
                if self.game.overseer.is_some() {
                    self.mode = Mode::Brain { sel: 0 };
                } else {
                    self.flash = Some("No overseer: press A to hand over the fortress first.".into());
                }
            }
            KeyCode::Char('A') => {
                let sel = self.game.overseer.as_ref().map_or(0, |o| 1 + crate::game::Style::ALL.iter().position(|&s| s == o.style).unwrap_or(0));
                self.mode = Mode::Autoplay { sel };
            }
            KeyCode::Char('c') => match self.game.chronicles.len() {
                0 => self.flash = Some("No chronicle yet: the chronicler writes at the turn of each year.".into()),
                n => self.mode = Mode::Chronicle { idx: n - 1 },
            },
            KeyCode::Char('j') => self.mode = Mode::Orders { sel: 0, pick: None },
            KeyCode::Char('L') => self.mode = Mode::Legends { tab: 0, scroll: usize::MAX, from_world: false },
            KeyCode::Char('v') => {
                let c = self.cursor;
                if let Some(d) = self.game.dwarves.iter().find(|d| d.alive && d.pos == c) {
                    self.mode = Mode::Unit { id: d.id, sel: 0 };
                } else {
                    self.flash = Some("No dwarf under the cursor.".into());
                }
            }
            KeyCode::Tab => {
                let alive: Vec<Pos> = self.game.dwarves.iter().filter(|d| d.alive).map(|d| d.pos).collect();
                if !alive.is_empty() {
                    let cur = alive.iter().position(|&p| p == self.cursor);
                    let next = cur.map_or(0, |i| (i + 1) % alive.len());
                    self.cursor = alive[next];
                }
            }
            KeyCode::Char('?') => self.mode = Mode::Help,
            KeyCode::Char('Q') => self.mode = Mode::ConfirmQuit,
            KeyCode::Char('H') => self.cursor = self.game.embark,
            KeyCode::Char('h') => self.mode = Mode::Highlights { sel: usize::MAX },
            KeyCode::F(5) => self.start_save(false, AfterSave::Resume),
            KeyCode::F(9) => {
                // Reload this fortress's latest save.
                let name = self.game.fortress_name.clone();
                match self.saves().into_iter().find(|m| m.fortress == name) {
                    Some(m) => self.start_load(m.file),
                    None => self.flash = Some("No save of this fortress yet (F5 saves).".into()),
                }
            }
            _ => {}
        }
    }

    fn key_labors(&mut self, k: KeyEvent, row: usize, col: usize, detail: usize) {
        let ids: Vec<u32> = self.game.dwarves.iter().filter(|d| d.alive).map(|d| d.id).collect();
        let rows = ids.len().max(1);
        // One column per labor, then the specialist column.
        let cols = LABORS.len() + 1;
        let ndet = self.game.work_details.len().max(1);
        let (mut row, mut col, mut detail) = (row.min(rows - 1), col, detail.min(ndet - 1));
        let id = ids.get(row).copied();
        match k.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                return;
            }
            KeyCode::Up => row = row.saturating_sub(1),
            KeyCode::Down => row = (row + 1).min(rows - 1),
            KeyCode::Left => col = (col + cols - 1) % cols,
            KeyCode::Right => col = (col + 1) % cols,
            KeyCode::Enter => {
                if let Some(id) = id {
                    if col == LABORS.len() {
                        self.game.toggle_specialist(id);
                    } else if let Some(lost) = self.game.toggle_labor(id, LABORS[col]) {
                        self.flash = Some(format!("{} dropped: one tool labor per dwarf.", lost.name()));
                    }
                }
            }
            KeyCode::Char('a') if col < LABORS.len() => {
                let l = LABORS[col];
                let on = self.game.toggle_labor_all(l);
                self.flash = Some(if on { format!("Everybody does {}.", l.name()) } else { format!("Nobody does {}.", l.name()) });
            }
            KeyCode::Char('[') => detail = (detail + ndet - 1) % ndet,
            KeyCode::Char(']') => detail = (detail + 1) % ndet,
            KeyCode::Char('w') => {
                if let Some(id) = id {
                    self.game.apply_detail(id, detail);
                }
            }
            KeyCode::Char(c @ '1'..='9') => {
                let n = c as usize - '1' as usize;
                if let Some(id) = id {
                    if self.game.apply_detail(id, n) {
                        detail = n;
                    }
                }
            }
            KeyCode::Char('S') => {
                if let Some(id) = id {
                    self.flash = Some(match self.game.save_detail(id) {
                        Some(name) => {
                            detail = self.game.work_details.len() - 1;
                            format!("Saved work detail \"{}\".", name)
                        }
                        None => "A work detail with these labors already exists.".into(),
                    });
                }
            }
            KeyCode::Char('X') => {
                if let Some(name) = self.game.delete_detail(detail) {
                    self.flash = Some(format!("Deleted work detail \"{}\".", name));
                    detail = detail.saturating_sub(1);
                }
            }
            _ => {}
        }
        self.mode = Mode::Labors { row, col, detail };
    }

    fn key_military(&mut self, k: KeyEvent) {
        let n = self.game.squads.len();
        let sel = self.squad_sel.min(n.saturating_sub(1));
        self.squad_sel = sel;
        match k.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Char('n') => {
                self.squad_sel = self.game.new_squad();
                let name = self.game.squads[self.squad_sel].name.clone();
                self.flash = Some(format!("{} formed. Add soldiers with u.", name));
            }
            KeyCode::Char(']') | KeyCode::Tab if n > 0 => self.squad_sel = (sel + 1) % n,
            KeyCode::Char('[') if n > 0 => self.squad_sel = (sel + n - 1) % n,
            KeyCode::Char('u') if n > 0 => self.mode = Mode::SquadMembers { squad: sel, sel: 0 },
            KeyCode::Char('X') if n > 0 => {
                let name = self.game.squads[sel].name.clone();
                self.game.delete_squad(sel);
                self.flash = Some(format!("{} disbanded.", name));
            }
            KeyCode::Char('s') if n > 0 => {
                self.game.order_station(sel, self.cursor);
                self.flash = Some(format!("{} will hold the cursor position.", self.game.squads[sel].name));
            }
            KeyCode::Char('s') => {
                self.game.station = Some(self.cursor);
                self.flash = Some("Soldiers will station here.".into());
            }
            KeyCode::Char('x') => {
                self.game.station = None;
                self.flash = Some("Station cleared; soldiers train where they stand.".into());
            }
            KeyCode::Char('t') if n > 0 => {
                self.game.order_train(sel);
                self.flash = Some(format!("{} goes back to training.", self.game.squads[sel].name));
            }
            KeyCode::Char('k') if n > 0 => self.mode = Mode::KillTarget { squad: sel },
            KeyCode::Char('e') if n > 0 => {
                if self.game.expedition.is_some() {
                    self.flash = Some("An expedition is already away.".into());
                } else if self.game.destinations().is_empty() {
                    self.flash = Some("There's nowhere within reach to send them.".into());
                } else {
                    self.mode = Mode::Expedition { squad: sel, sel: 0 };
                }
            }
            KeyCode::Char('w') if n > 0 => {
                let w = self.game.squads[sel].weapon.next();
                self.game.squads[sel].weapon = w;
            }
            KeyCode::Char('r') if n > 0 => {
                let a = !self.game.squads[sel].armor;
                self.game.squads[sel].armor = a;
            }
            KeyCode::Char('a') => self.game.alert = !self.game.alert,
            KeyCode::Char('c') => {
                self.flash = Some(match self.game.toggle_civ_alert() {
                    Ok(true) => "Civilian alert raised: everyone into the alert burrow.".into(),
                    Ok(false) => "Civilian alert lowered.".into(),
                    Err(e) => e,
                });
            }
            KeyCode::Char('h') => self.game.hunt = !self.game.hunt,
            _ => {}
        }
    }

    fn key_burrows(&mut self, k: KeyEvent, sel: usize, corner: Option<Pos>, erase: bool) {
        let n = self.game.burrows.len();
        let sel = sel.min(n.saturating_sub(1));
        match k.code {
            KeyCode::Esc => {
                self.mode = if corner.is_some() { Mode::Burrows { sel, corner: None, erase } } else { Mode::Normal };
                return;
            }
            KeyCode::Enter if n > 0 => {
                match corner {
                    None => self.mode = Mode::Burrows { sel, corner: Some(self.cursor), erase },
                    Some(a) => {
                        let c = self.game.paint_burrow(sel, a, self.cursor, !erase);
                        let name = self.game.burrows[sel].name.clone();
                        self.flash = Some(format!("{} tiles {} {}.", c, if erase { "removed from" } else { "added to" }, name));
                        self.mode = Mode::Burrows { sel, corner: None, erase };
                    }
                }
                return;
            }
            KeyCode::Enter => self.flash = Some("Make a burrow first (n).".into()),
            KeyCode::Char('n') => match self.game.new_burrow() {
                Some(i) => {
                    self.mode = Mode::Burrows { sel: i, corner: None, erase: false };
                    return;
                }
                None => self.flash = Some(format!("At most {} burrows.", crate::game::MAX_BURROWS)),
            },
            KeyCode::Char('X') if n > 0 => {
                let name = self.game.burrows[sel].name.clone();
                self.game.delete_burrow(sel);
                self.flash = Some(format!("Deleted {}.", name));
                self.mode = Mode::Burrows { sel: sel.saturating_sub(1), corner: None, erase: false };
                return;
            }
            KeyCode::Char(']') | KeyCode::Tab if n > 0 => {
                self.mode = Mode::Burrows { sel: (sel + 1) % n, corner: None, erase };
                return;
            }
            KeyCode::Char('[') if n > 0 => {
                self.mode = Mode::Burrows { sel: (sel + n - 1) % n, corner: None, erase };
                return;
            }
            KeyCode::Char('e') => {
                self.mode = Mode::Burrows { sel, corner, erase: !erase };
                return;
            }
            KeyCode::Char('u') if n > 0 => {
                self.mode = Mode::BurrowMembers { burrow: sel, sel: 0 };
                return;
            }
            KeyCode::Char('A') if n > 0 => {
                self.game.set_alert_burrow(sel);
                self.flash = Some(format!("{} is the civilian alert burrow (raise it with m, then c).", self.game.burrows[sel].name));
            }
            _ => {}
        }
        self.mode = Mode::Burrows { sel, corner, erase };
    }

    fn key_orders(&mut self, k: KeyEvent, sel: usize, pick: Option<usize>) {
        let recipes = crate::game::all_recipes();
        if let Some(p) = pick {
            match k.code {
                KeyCode::Esc => self.mode = Mode::Orders { sel, pick: None },
                KeyCode::Up => self.mode = Mode::Orders { sel, pick: Some(p.saturating_sub(1)) },
                KeyCode::Down => self.mode = Mode::Orders { sel, pick: Some((p + 1).min(recipes.len() - 1)) },
                KeyCode::PageUp => self.mode = Mode::Orders { sel, pick: Some(p.saturating_sub(10)) },
                KeyCode::PageDown => self.mode = Mode::Orders { sel, pick: Some((p + 10).min(recipes.len() - 1)) },
                KeyCode::Enter => {
                    let r = recipes[p];
                    let (kind, per) = r.product();
                    let keep = (self.game.stock_of(kind) as u32 + per as u32).max(5);
                    self.game.add_work_order(r, keep);
                    let at = self.game.work_orders.iter().position(|o| o.recipe == r).unwrap_or(0);
                    self.flash = Some(format!("Keeping {} in stock: {}.", keep, r.name()));
                    self.mode = Mode::Orders { sel: at, pick: None };
                }
                _ => {}
            }
            return;
        }
        let n = self.game.work_orders.len();
        let step = if k.modifiers.contains(KeyModifiers::SHIFT) { 10 } else { 1 };
        match k.code {
            KeyCode::Esc | KeyCode::Char('j') => self.mode = Mode::Normal,
            KeyCode::Up => self.mode = Mode::Orders { sel: sel.saturating_sub(1), pick: None },
            KeyCode::Down => self.mode = Mode::Orders { sel: (sel + 1).min(n.saturating_sub(1)), pick: None },
            KeyCode::Char('n') => self.mode = Mode::Orders { sel, pick: Some(0) },
            KeyCode::Right | KeyCode::Char('+') | KeyCode::Char('=') => {
                if let Some(o) = self.game.work_orders.get_mut(sel) {
                    o.keep = (o.keep + step).min(999);
                }
            }
            KeyCode::Left | KeyCode::Char('-') => {
                if let Some(o) = self.game.work_orders.get_mut(sel) {
                    o.keep = o.keep.saturating_sub(step);
                }
            }
            KeyCode::Char('x') | KeyCode::Delete => {
                if sel < n {
                    self.game.work_orders.remove(sel);
                    self.mode = Mode::Orders { sel: sel.min(n.saturating_sub(2)), pick: None };
                }
            }
            _ => {}
        }
    }

    fn key_look(&mut self, k: KeyEvent, sel: usize) {
        let entries = look_entries(&self.game, self.cursor);
        let n = entries.len().max(1);
        match k.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Tab | KeyCode::Char(']') => self.mode = Mode::Look { sel: (sel + 1) % n },
            KeyCode::BackTab | KeyCode::Char('[') => self.mode = Mode::Look { sel: (sel + n - 1) % n },
            KeyCode::Char('c') => {
                let farm = self.game.zone_at(self.cursor).filter(|z| z.kind == crate::game::ZoneKind::Farm).map(|z| z.id);
                self.flash = Some(match farm.and_then(|id| self.game.cycle_farm_crop(id)) {
                    Some(s) => s,
                    None => "No farm here.".into(),
                });
            }
            KeyCode::Char('o') => {
                let zone = self.game.zone_at(self.cursor).filter(|z| z.kind.ownable()).map(|z| (z.id, z.owner));
                match zone {
                    Some((zone, owner)) => {
                        let cands = self.game.owner_candidates();
                        let sel = owner.and_then(|o| cands.iter().position(|&id| id == o)).map_or(0, |i| i + 1);
                        self.mode = Mode::PickOwner { zone, sel };
                        return;
                    }
                    None => self.flash = Some("No bedroom or office here.".into()),
                }
            }
            KeyCode::Char('d') => {
                if let Some(LookEntry::Item(id)) = entries.get(sel.min(n - 1)) {
                    self.flash = Some(match self.game.toggle_dump(*id) {
                        Some(true) => "Marked for the garbage dump.".into(),
                        Some(false) => "No longer marked for dumping.".into(),
                        None => "That's being carried.".into(),
                    });
                }
            }
            KeyCode::Char('p') => {
                if let Some(LookEntry::Creature(id)) = entries.get(sel.min(n - 1)) {
                    self.flash = Some(match self.game.cycle_pasture(*id) {
                        Some(z) => format!("Assigned to {}.", z),
                        None => "Only tame animals can be pastured.".into(),
                    });
                }
            }
            KeyCode::Char('w') => {
                if let Some(LookEntry::Creature(id)) = entries.get(sel.min(n - 1)) {
                    self.flash = Some(match self.game.cycle_dog_owner(*id) {
                        Some(who) => format!("War dog of {}.", who),
                        None => "Only tame dogs can be war dogs.".into(),
                    });
                }
            }
            KeyCode::Char('b') => {
                if let Some(LookEntry::Creature(id)) = entries.get(sel.min(n - 1)) {
                    self.flash = Some(match self.game.toggle_slaughter(*id) {
                        Some(true) => "Marked for slaughter.".into(),
                        Some(false) => "No longer marked for slaughter.".into(),
                        None => "Only tame animals can be slaughtered.".into(),
                    });
                }
            }
            KeyCode::Char('t') => {
                if let Some(LookEntry::Item(id)) = entries.get(sel.min(n - 1)) {
                    self.flash = Some(match self.game.tame_from_cage(*id) {
                        Ok(m) | Err(m) => m,
                    });
                }
            }
            KeyCode::Char('D') => {
                let k = self.game.dump_tile(self.cursor);
                self.flash = Some(format!("{} items marked for the garbage dump.", k));
            }
            KeyCode::Char('s') => {
                match self.game.zone_at(self.cursor).filter(|z| matches!(z.kind, crate::game::ZoneKind::Stockpile(_))).map(|z| z.id) {
                    Some(zone) => self.mode = Mode::PileSettings { zone, sel: 0 },
                    None => self.flash = Some("No stockpile here.".into()),
                }
            }
            KeyCode::Char('f') => {
                if let Some(LookEntry::Item(id)) = entries.get(sel.min(n - 1)) {
                    self.flash = Some(match self.game.toggle_forbid(*id) {
                        Some(true) => "Forbidden: dwarves will leave it alone.".into(),
                        Some(false) => "No longer forbidden.".into(),
                        None => "That's being carried.".into(),
                    });
                }
            }
            KeyCode::Char('F') => {
                let (k, on) = self.game.forbid_tile(self.cursor);
                self.flash = Some(format!("{} items {}.", k, if on { "forbidden" } else { "allowed again" }));
            }
            KeyCode::Enter => match entries.get(sel.min(n - 1)) {
                Some(LookEntry::Dwarf(id)) => self.mode = Mode::Unit { id: *id, sel: 0 },
                Some(LookEntry::Building(b)) => self.mode = Mode::Query { bld: *b, adding: false },
                Some(_) => self.flash = Some("Nothing more to see.".into()),
                None => {}
            },
            _ => {}
        }
    }

    fn key_query(&mut self, k: KeyEvent, bld: u32, adding: bool) {
        let Some(b) = self.game.buildings.get(&bld) else {
            self.mode = Mode::Normal;
            return;
        };
        let kind = b.kind;
        match k.code {
            KeyCode::Esc => {
                self.mode = if adding { Mode::Query { bld, adding: false } } else { Mode::Normal };
            }
            KeyCode::Char(c) if adding => {
                if let Some(&r) = kind.recipes().iter().find(|r| r.key() == c) {
                    self.game.add_order(bld, r);
                    self.mode = Mode::Query { bld, adding: false };
                }
            }
            KeyCode::Char('a') if b.built && !kind.recipes().is_empty() => {
                self.mode = Mode::Query { bld, adding: true };
            }
            KeyCode::Char('l') => self.game.toggle_lock(bld),
            KeyCode::Char('k') if kind == BuildingKind::Lever => {
                self.cursor = self.game.buildings[&bld].pos;
                self.mode = Mode::LinkLever { lever: bld };
            }
            KeyCode::Char('p') if kind == BuildingKind::Lever => {
                self.flash = Some(if self.game.request_pull(bld) {
                    "Someone will go and pull it.".into()
                } else {
                    "Link the lever to something first (k).".into()
                });
            }
            KeyCode::Char('g') if kind == BuildingKind::Depot => {
                let n = self.game.mark_for_trade(true);
                self.flash = Some(format!("{} items marked for the depot.", n));
            }
            KeyCode::Char('G') if kind == BuildingKind::Depot => {
                let n = self.game.mark_for_trade(false);
                self.flash = Some(format!("{} items unmarked.", n));
            }
            KeyCode::Char('t') if kind == BuildingKind::Depot => {
                if self.game.caravan.as_ref().map_or(false, |c| c.arrived) {
                    self.trade_ours.clear();
                    self.trade_theirs.clear();
                    self.mode = Mode::Trade { side: 0, sel: [0, 0] };
                } else {
                    self.flash = Some("No merchants at the depot.".into());
                }
            }
            KeyCode::Char('w') if kind.is_workshop() && kind != BuildingKind::Depot => {
                self.mode = Mode::Profile { bld, sel: 0 };
            }
            KeyCode::Char('r') => {
                if let Some(o) = self.game.buildings.get_mut(&bld).and_then(|b| b.orders.last_mut()) {
                    o.repeat = !o.repeat;
                }
            }
            KeyCode::Char('x') => {
                if let Some(b) = self.game.buildings.get_mut(&bld) {
                    // Keep the order being worked on.
                    if b.orders.len() > 1 || b.worker.is_none() {
                        b.orders.pop();
                    }
                }
            }
            KeyCode::Char('X') => {
                self.game.remove_building(bld);
                self.flash = Some("Building removed.".into());
                self.mode = Mode::Normal;
            }
            _ => {}
        }
    }

    fn key_world(&mut self, k: KeyEvent, x: i32, y: i32) {
        let ow = &self.game.overworld;
        let step = if k.modifiers.contains(KeyModifiers::SHIFT) { (ow.w / 12).max(5) } else { 1 };
        let (mut nx, mut ny) = (x, y);
        match k.code {
            KeyCode::Up => ny -= step,
            KeyCode::Down => ny += step,
            KeyCode::Left => nx -= step,
            KeyCode::Right => nx += step,
            KeyCode::Char('s') | KeyCode::Char('S') => {
                // The next preset.
                use crate::world::MapSize;
                let i = MapSize::ALL.iter().position(|&m| m == self.embark_size).map_or(0, |i| (i + 1) % MapSize::ALL.len());
                self.embark_size = MapSize::ALL[i];
                return;
            }
            KeyCode::Char('[') | KeyCode::Char(']') => {
                let s = &mut self.embark_size;
                s.w = (s.w + if k.code == KeyCode::Char(']') { 16 } else { -16 }).clamp(48, 256);
                return;
            }
            KeyCode::Char('{') | KeyCode::Char('}') => {
                let s = &mut self.embark_size;
                s.h = (s.h + if k.code == KeyCode::Char('}') { 16 } else { -16 }).clamp(48, 192);
                return;
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                let s = &mut self.embark_size;
                s.d = match s.d {
                    32 => 48,
                    48 => 64,
                    _ => 32,
                };
                return;
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                self.mode = Mode::Legends { tab: 0, scroll: usize::MAX, from_world: true };
                return;
            }
            KeyCode::Char('v') | KeyCode::Char('V') => {
                self.world_blocks = !self.world_blocks;
                return;
            }
            KeyCode::Char('?') => {
                self.flash = Some("Arrows pick a site, Enter to embark (play now or prepare carefully). Map size: s presets, [ ] width, { } height, d depth. l legends, v colour view, Esc back.".into());
                return;
            }
            KeyCode::Char('Q') => {
                self.quit = true;
                return;
            }
            KeyCode::Esc => {
                self.mode = Mode::Title { sel: 0 };
                return;
            }
            KeyCode::Enter => {
                // Nothing commits yet: first the site's card and the choices.
                if ow.embarkable(x, y) {
                    self.flash = None;
                    self.mode = Mode::Embark { x, y, sel: 0 };
                } else {
                    self.flash = Some("You can't embark on water or an existing site.".into());
                }
                return;
            }
            _ => return,
        }
        nx = nx.clamp(0, ow.w - 1);
        ny = ny.clamp(0, ow.h - 1);
        self.mode = Mode::WorldMap { x: nx, y: ny };
    }

    /// The embark card: Play now, Prepare carefully, or Back.
    fn key_embark(&mut self, k: KeyEvent, x: i32, y: i32, sel: usize) {
        let choose = match k.code {
            KeyCode::Up => {
                self.mode = Mode::Embark { x, y, sel: (sel + 2) % 3 };
                return;
            }
            KeyCode::Down | KeyCode::Tab => {
                self.mode = Mode::Embark { x, y, sel: (sel + 1) % 3 };
                return;
            }
            KeyCode::Enter => sel,
            KeyCode::Char('p') | KeyCode::Char('P') => 0,
            KeyCode::Char('c') | KeyCode::Char('C') => 1,
            KeyCode::Esc | KeyCode::Char('b') | KeyCode::Char('B') => 2,
            KeyCode::Char('Q') => {
                self.quit = true;
                return;
            }
            _ => return,
        };
        match choose {
            0 => self.start_embark(x, y, Loadout::standard()),
            1 => self.mode = Mode::Prepare { x, y, sel: 0, col: 0 },
            _ => self.mode = Mode::WorldMap { x, y },
        }
    }

    /// Preparing carefully: ↑↓ rows, ←→ less or more, Tab main/second
    /// skill, Space (or l / L) another labor, r start over, Enter embark.
    fn key_prepare(&mut self, k: KeyEvent, x: i32, y: i32, sel: usize, col: usize) {
        use crate::game::loadout::{ANIMALS, SUPPLIES};
        let founders = self.loadout.founders.len();
        let rows = founders + SUPPLIES.len() + ANIMALS.len();
        let (mut sel, mut col) = (sel.min(rows - 1), col.min(1));
        match k.code {
            KeyCode::Up => sel = (sel + rows - 1) % rows,
            KeyCode::Down => sel = (sel + 1) % rows,
            KeyCode::PageUp => sel = sel.saturating_sub(7),
            KeyCode::PageDown => sel = (sel + 7).min(rows - 1),
            KeyCode::Tab | KeyCode::BackTab => col = 1 - col,
            KeyCode::Left | KeyCode::Right | KeyCode::Char('-') | KeyCode::Char('+') | KeyCode::Char('=') => {
                let up = matches!(k.code, KeyCode::Right | KeyCode::Char('+') | KeyCode::Char('='));
                if sel < founders {
                    self.loadout.adjust_skill(sel, col, up);
                } else if sel < founders + SUPPLIES.len() {
                    self.loadout.adjust_supply(sel - founders, up);
                } else {
                    self.loadout.adjust_animal(sel - founders - SUPPLIES.len(), up);
                }
            }
            KeyCode::Char(' ') | KeyCode::Char('l') | KeyCode::Char('L') => {
                if sel < founders {
                    self.loadout.cycle_labor(sel, col, k.code != KeyCode::Char('L'));
                }
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.loadout = Loadout::prepared();
                self.flash = Some("Back to the standard expedition.".into());
            }
            KeyCode::Enter => {
                let left = self.loadout.points_left();
                if left < 0 {
                    self.flash = Some(format!("That's {} points over the budget: leave something behind.", -left));
                } else {
                    self.flash = None;
                    self.start_embark(x, y, self.loadout.clone());
                    return;
                }
            }
            KeyCode::Esc => {
                self.flash = None;
                self.mode = Mode::Embark { x, y, sel: 1 };
                return;
            }
            _ => return,
        }
        self.mode = Mode::Prepare { x, y, sel, col };
    }

    fn key_trade(&mut self, k: KeyEvent, side: usize, mut sel: [usize; 2]) {
        let ours: Vec<u32> = self.game.depot_goods().iter().map(|it| it.id).collect();
        let theirs: Vec<u32> = self.game.caravan.as_ref().map_or(Vec::new(), |c| c.goods.iter().map(|g| g.id).collect());
        let len = [ours.len(), theirs.len()];
        match k.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                return;
            }
            KeyCode::Tab | KeyCode::Left | KeyCode::Right => {
                self.mode = Mode::Trade { side: 1 - side, sel };
                return;
            }
            KeyCode::Up => sel[side] = sel[side].saturating_sub(1),
            KeyCode::Down => sel[side] = (sel[side] + 1).min(len[side].saturating_sub(1)),
            KeyCode::Enter | KeyCode::Char(' ') => {
                let (list, marks) = if side == 0 { (&ours, &mut self.trade_ours) } else { (&theirs, &mut self.trade_theirs) };
                if let Some(&id) = list.get(sel[side]) {
                    if !marks.insert(id) {
                        marks.remove(&id);
                    }
                }
            }
            KeyCode::Char('o') => {
                let o: Vec<u32> = self.trade_ours.iter().copied().filter(|id| ours.contains(id)).collect();
                let t: Vec<u32> = self.trade_theirs.iter().copied().filter(|id| theirs.contains(id)).collect();
                match self.game.trade(&o, &t) {
                    Ok(m) => {
                        self.game.msg(&m, (240, 240, 160));
                        self.flash = Some(m);
                        self.trade_ours.clear();
                        self.trade_theirs.clear();
                        sel = [0, 0];
                    }
                    Err(m) => self.flash = Some(m),
                }
            }
            _ => {}
        }
        self.mode = Mode::Trade { side, sel };
    }

    /// Enter/click: set a corner or apply the current tool.
    fn confirm(&mut self) {
        let c = self.cursor;
        match self.mode.clone() {
            Mode::Designate { kind, corner: None } => self.mode = Mode::Designate { kind, corner: Some(c) },
            Mode::Designate { kind, corner: Some(a) } => {
                let was: Vec<(Pos, Desig)> = crate::geom::box_iter(a, c).filter(|&p| self.game.world.in_bounds(p)).map(|p| (p, self.game.world.t(p).desig)).collect();
                let n = self.game.designate(a, c, kind);
                if n > 0 {
                    self.undo.push(was);
                    if self.undo.len() > 30 {
                        self.undo.remove(0);
                    }
                }
                self.flash = Some(format!("{} tiles: {}", n, kind.name()));
                self.mode = Mode::Designate { kind, corner: None };
            }
            Mode::Stockpile { filter, corner: None } => self.mode = Mode::Stockpile { filter, corner: Some(c) },
            Mode::Stockpile { filter, corner: Some(a) } => {
                let n = match filter {
                    Some(f) => self.game.add_zone(a, c, ZoneKind::Stockpile(f)),
                    None => self.game.remove_zones(a, c),
                };
                self.flash = Some(match filter {
                    Some(f) => format!("Stockpile ({}) of {} tiles.", f.name(), n),
                    None => format!("Removed {} zone tiles.", n),
                });
                self.mode = Mode::Stockpile { filter, corner: None };
            }
            Mode::Traffic { level, corner: None } => self.mode = Mode::Traffic { level, corner: Some(c) },
            Mode::Traffic { level, corner: Some(a) } => {
                let mut n = 0;
                for p in crate::geom::box_iter(a, c) {
                    if let Some(t) = self.game.world.get(p) {
                        if t.revealed && t.traffic != level {
                            self.game.world.t_mut(p).traffic = level;
                            n += 1;
                        }
                    }
                }
                self.flash = Some(format!("{} tiles of {} traffic.", n, crate::game::TRAFFIC_NAMES[level as usize].to_lowercase()));
                self.mode = Mode::Traffic { level, corner: None };
            }
            Mode::Construct { kind, mat, corner: None } => self.mode = Mode::Construct { kind, mat, corner: Some(c) },
            Mode::Construct { kind, mat, corner: Some(a) } => {
                self.flash = Some(match kind {
                    Some(k) => match self.game.plan_constructions(a, c, k, mat) {
                        0 => match k {
                            ConstructKind::Remove => "Nothing built there to remove.".into(),
                            ConstructKind::Floor => "Floors go over open space next to something solid.".into(),
                            _ => "Needs clear, revealed floor outside zones.".into(),
                        },
                        n if k == ConstructKind::Remove => format!("{} constructions to take down.", n),
                        n => format!("{} × {} planned, from {}.", n, k.name().to_lowercase(), mat.name()),
                    },
                    None => format!("Cancelled {} planned constructions.", self.game.cancel_constructions(a, c)),
                });
                self.mode = Mode::Construct { kind, mat, corner: None };
            }
            Mode::Zone { tool, corner: None } => self.mode = Mode::Zone { tool, corner: Some(c) },
            Mode::Zone { tool, corner: Some(a) } => {
                self.flash = Some(match tool.kind() {
                    None => {
                        // Leave stockpiles to the stockpile tool.
                        let n = self.game.remove_zones_where(a, c, |k| !matches!(k, ZoneKind::Stockpile(_)));
                        format!("Removed {} zone tiles.", n)
                    }
                    Some(kind) => match self.game.add_zone(a, c, kind) {
                        0 if kind == ZoneKind::Farm => "Farm plots need revealed soil floor.".into(),
                        0 => "Zones need clear, revealed floor not already zoned.".into(),
                        n => {
                            if kind == ZoneKind::Pasture {
                                let zid = self.game.world.t(c).zone;
                                let moved = self.game.assign_grazers(zid);
                                if moved > 0 {
                                    self.game.msg(&format!("{} animals moved into the new pasture.", moved), (160, 230, 160));
                                }
                            }
                            let hint = match kind {
                                ZoneKind::Meeting => " Idle dwarves will gather here.",
                                k if k.ownable() => " Assign it with k, then o.",
                                ZoneKind::Dump => " Mark items with k, then d.",
                                _ => "",
                            };
                            format!("{} of {} tiles.{}", kind.name(), n, hint)
                        }
                    },
                });
                self.mode = Mode::Zone { tool, corner: None };
            }
            Mode::Place(kind) => {
                let s = kind.size() / 2;
                let tl = c.offset(-s, -s, 0);
                if self.game.place_building(kind, tl) {
                    self.flash = Some(format!("{} placed; it needs {}.", kind.name(), kind.needs_desc()));
                    self.mode = Mode::BuildMenu;
                } else {
                    self.flash = Some("Can't build there: needs clear, dry, revealed floor.".into());
                }
            }
            _ => {}
        }
    }

}
