use crate::buildings::BuildingKind;
use crate::dwarf::LABORS;
use crate::game::{BuildMat, ConstructKind, Game, ZoneKind};
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

pub const SPEEDS: [u32; 6] = [1, 2, 5, 10, 25, 60];
pub const SAVE_DIR: &str = "saves";
pub const TITLE_MENU: [&str; 4] = ["New world", "Continue", "Load game", "Quit"];

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
            ZoneTool::Office => &["A room for a noble. Assign it with", "k then o. Nobles will want one."],
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
    /// Choose an embark site on the world map.
    WorldMap { x: i32, y: i32 },
    /// Browse history. `from_world` returns to the world map on Esc.
    Legends { tab: usize, scroll: usize, from_world: bool },
    /// Barter at the depot. side 0 = our goods, 1 = theirs.
    Trade { side: usize, sel: [usize; 2] },
    Help,
    ConfirmQuit,
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
    /// Index into `MapSize::ALL` for the next embark.
    pub embark_size: usize,
    /// The squad selected on the military screen.
    pub squad_sel: usize,
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
            embark_size: 1,
            squad_sel: 0,
        }
    }

    // ------------------------------------------------------------ jobs

    /// Advance one frame: finish jobs, run the simulation, autosave.
    pub fn frame(&mut self) {
        self.poll_job();
        if self.mode == Mode::WorldGen {
            if let Some(g) = self.worldgen.as_mut() {
                // Paced so the world can be watched taking shape; Space skips ahead.
                let (rows, years) = if self.gen_fast { (usize::MAX, 40) } else { (1, 1) };
                g.step(rows, years);
            }
            return;
        }
        if self.job.is_some() || self.game.is_placeholder() || !self.in_fortress() {
            return;
        }
        if !self.paused {
            for _ in 0..self.ticks_per_frame() {
                self.game.step();
                if self.game.pause_request {
                    self.game.pause_request = false;
                    self.paused = true;
                    break;
                }
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
                | Mode::Legends { from_world: true, .. }
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
        self.worldgen = Some(WorldGen::new(self.world_params));
        self.gen_fast = false;
        self.mode = Mode::WorldGen;
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
        let done = self.worldgen.as_ref().map_or(false, |g| g.done());
        match k.code {
            KeyCode::Enter if done => {
                let ow = self.worldgen.take().unwrap().finish();
                self.game = Game::placeholder(ow);
                let (x, y) = self.game.site;
                self.mode = Mode::WorldMap { x, y };
            }
            KeyCode::Char(' ') | KeyCode::Enter => self.gen_fast = true,
            KeyCode::Char('r') => {
                self.world_params.seed = random_seed();
                self.start_worldgen();
            }
            KeyCode::Esc | KeyCode::Char('e') => {
                self.worldgen = None;
                self.mode = Mode::WorldSetup { sel: 0 };
            }
            _ => {}
        }
    }

    fn start_embark(&mut self, x: i32, y: i32) {
        let ow = self.game.overworld.clone();
        let seed = self.seed;
        let size = crate::world::MapSize::ALL[self.embark_size];
        self.run_job("Embarking", move |report| Ok(JobDone::Embarked(Game::embark_with(seed, ow, x, y, size, report))));
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
                self.paused = true;
                self.mode = Mode::Help;
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

    fn uses_map_cursor(&self) -> bool {
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

    pub fn on_mouse(&mut self, m: MouseEvent) {
        let r = self.map_rect;
        match m.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if m.column >= r.x && m.row >= r.y && m.column < r.x + r.width && m.row < r.y + r.height {
                    if !self.uses_map_cursor() {
                        return;
                    }
                    self.cursor.x = self.view_x + (m.column - r.x) as i32;
                    self.cursor.y = self.view_y + (m.row - r.y) as i32;
                    self.clamp_cursor();
                    if !matches!(self.mode, Mode::Normal) {
                        self.confirm();
                    }
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
        self.flash = None;
        if k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL) {
            self.quit = true;
            return;
        }
        match self.mode.clone() {
            Mode::Loading => return,
            Mode::WorldSetup { sel } => return self.key_world_setup(k, sel),
            Mode::WorldGen => return self.key_worldgen(k),
            Mode::Title { sel } => return self.key_title(k, sel),
            Mode::LoadMenu { sel, confirm } => return self.key_load_menu(k, sel, confirm),
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
                KeyCode::Tab | KeyCode::Right => self.mode = Mode::Status { tab: (tab + 1) % 4, scroll: 0 },
                KeyCode::Left => self.mode = Mode::Status { tab: (tab + 3) % 4, scroll: 0 },
                KeyCode::Char(c @ '1'..='4') => self.mode = Mode::Status { tab: c as usize - '1' as usize, scroll: 0 },
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
                _ => {}
            },
            Mode::Military => self.key_military(k),
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
                    KeyCode::Tab | KeyCode::Right => self.mode = Mode::Legends { tab: (tab + 1) % 5, scroll: 0, from_world },
                    KeyCode::Left => self.mode = Mode::Legends { tab: (tab + 4) % 5, scroll: 0, from_world },
                    KeyCode::Char(c @ '1'..='5') => {
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
            Mode::Title { .. } | Mode::LoadMenu { .. } | Mode::Loading | Mode::WorldSetup { .. } | Mode::WorldGen => {}
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
                let zone = self.game.zone_at(self.cursor).filter(|z| z.kind.ownable()).map(|z| (z.id, z.kind));
                self.flash = Some(match zone {
                    Some((zid, kind)) => {
                        let who = self.game.cycle_zone_owner(zid).unwrap_or_default();
                        format!("{} assigned to {}.", kind.name(), who)
                    }
                    None => "No bedroom or office here.".into(),
                });
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
                self.embark_size = (self.embark_size + 1) % crate::world::MapSize::ALL.len();
                return;
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                self.mode = Mode::Legends { tab: 0, scroll: usize::MAX, from_world: true };
                return;
            }
            KeyCode::Char('?') => {
                self.flash = Some("Arrows pick a site, Enter embarks, l opens legends, Esc back to the title.".into());
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
                if ow.embarkable(x, y) {
                    self.start_embark(x, y);
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
                let n = self.game.designate(a, c, kind);
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
