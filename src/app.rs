use crate::buildings::BuildingKind;
use crate::dwarf::LABORS;
use crate::game::{BuildMat, ConstructKind, Game, ZoneKind};
use crate::geom::Pos;
use crate::items::StockFilter;
use crate::world::Desig;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use std::collections::HashSet;

pub const SPEEDS: [u32; 6] = [1, 2, 5, 10, 25, 60];
pub const SAVE_FILE: &str = "dwarfclone_save.json";

pub const DESIG_KEYS: [(char, Desig); 8] = [
    ('d', Desig::Dig),
    ('h', Desig::Channel),
    ('u', Desig::StairUp),
    ('j', Desig::StairDown),
    ('i', Desig::StairUD),
    ('r', Desig::Ramp),
    ('t', Desig::Chop),
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
    Erase,
}

impl ZoneTool {
    pub const ALL: [ZoneTool; 8] = [
        ZoneTool::Meeting,
        ZoneTool::Bedroom,
        ZoneTool::Dormitory,
        ZoneTool::Dining,
        ZoneTool::Office,
        ZoneTool::Farm,
        ZoneTool::Dump,
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
            ZoneTool::Farm => &["Mark soil floor (grass, loam, clay,", "sand) as farmland. Farmers plant", "stoutcaps; brewers turn them to ale."],
            ZoneTool::Meeting => &["Dwarves with nothing to do gather", "here and chat (and make friends)", "instead of loitering in corridors."],
            ZoneTool::Bedroom => &["Lay over a bed. Give it to a dwarf", "with k then o; otherwise beds are", "handed out automatically."],
            ZoneTool::Dormitory => &["Beds here are shared and never", "owned. Better than the floor, but", "nobody likes a crowded dormitory."],
            ZoneTool::Dining => &["Lay over tables and chairs. Dwarves", "eat here first; a richer hall", "makes for happier meals."],
            ZoneTool::Office => &["A room for a noble. Assign it with", "k then o. Nobles will want one."],
            ZoneTool::Dump => &["Items marked for dumping (k, then", "d or D) are thrown here and left."],
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
    v.extend(g.dwarves.iter().filter(|d| d.alive && d.pos == p).map(|d| LookEntry::Dwarf(d.id)));
    if g.world.t(p).revealed {
        v.extend(g.creatures.values().filter(|c| c.alive && c.pos == p).map(|c| LookEntry::Creature(c.id)));
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
    Normal,
    Designate { kind: Desig, corner: Option<Pos> },
    /// `filter == None` erases zones.
    Stockpile { filter: Option<StockFilter>, corner: Option<Pos> },
    Zone { tool: ZoneTool, corner: Option<Pos> },
    /// Inspect whatever is under the cursor; `sel` picks one of several things there.
    Look { sel: usize },
    /// Fortress overview screens.
    Status { tab: usize, scroll: usize },
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
}

impl App {
    pub fn new(seed: u64) -> App {
        let game = Game::new(seed);
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
            Mode::Military => match k.code {
                KeyCode::Esc => self.mode = Mode::Normal,
                KeyCode::Char('s') => {
                    self.game.station = Some(self.cursor);
                    self.flash = Some("Soldiers will station here.".into());
                }
                KeyCode::Char('x') => {
                    self.game.station = None;
                    self.flash = Some("Station cleared; soldiers train where they stand.".into());
                }
                KeyCode::Char('a') => self.game.alert = !self.game.alert,
                KeyCode::Char('h') => self.game.hunt = !self.game.hunt,
                _ => {}
            },
            Mode::Labors { row, col, detail } => self.key_labors(k, row, col, detail),
            Mode::Help | Mode::Nobles => self.mode = Mode::Normal,
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
                    KeyCode::Tab | KeyCode::Right => self.mode = Mode::Legends { tab: (tab + 1) % 4, scroll: 0, from_world },
                    KeyCode::Left => self.mode = Mode::Legends { tab: (tab + 3) % 4, scroll: 0, from_world },
                    KeyCode::Char(c @ '1'..='4') => {
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
            Mode::ConfirmQuit => {
                if matches!(k.code, KeyCode::Char('y') | KeyCode::Char('Y')) {
                    self.quit = true;
                } else {
                    self.mode = Mode::Normal;
                }
            }
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
            KeyCode::Char('m') => self.mode = Mode::Military,
            KeyCode::Char('n') => self.mode = Mode::Nobles,
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
            KeyCode::F(5) => self.save(),
            KeyCode::F(9) => self.load(),
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

    fn key_look(&mut self, k: KeyEvent, sel: usize) {
        let entries = look_entries(&self.game, self.cursor);
        let n = entries.len().max(1);
        match k.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Tab | KeyCode::Char(']') => self.mode = Mode::Look { sel: (sel + 1) % n },
            KeyCode::BackTab | KeyCode::Char('[') => self.mode = Mode::Look { sel: (sel + n - 1) % n },
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
            KeyCode::Char('D') => {
                let k = self.game.dump_tile(self.cursor);
                self.flash = Some(format!("{} items marked for the garbage dump.", k));
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
        let step = if k.modifiers.contains(KeyModifiers::SHIFT) { 5 } else { 1 };
        let ow = &self.game.overworld;
        let (mut nx, mut ny) = (x, y);
        match k.code {
            KeyCode::Up => ny -= step,
            KeyCode::Down => ny += step,
            KeyCode::Left => nx -= step,
            KeyCode::Right => nx += step,
            KeyCode::Char('l') | KeyCode::Char('L') => {
                self.mode = Mode::Legends { tab: 0, scroll: usize::MAX, from_world: true };
                return;
            }
            KeyCode::Char('?') => {
                self.flash = Some("Arrows pick a site, Enter embarks, l opens legends.".into());
                return;
            }
            KeyCode::Char('Q') => {
                self.quit = true;
                return;
            }
            KeyCode::Enter => {
                if ow.embarkable(x, y) {
                    let ow = self.game.overworld.clone();
                    self.game = Game::embark(self.seed, ow, x, y);
                    self.cursor = self.game.embark;
                    self.paused = true;
                    self.mode = Mode::Help;
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

    fn save(&mut self) {
        let result = serde_json::to_string(&self.game)
            .map_err(anyhow::Error::from)
            .and_then(|s| std::fs::write(SAVE_FILE, s).map_err(anyhow::Error::from));
        self.flash = Some(match result {
            Ok(()) => format!("Saved to {}.", SAVE_FILE),
            Err(e) => format!("Save failed: {}", e),
        });
    }

    fn load(&mut self) {
        let result = std::fs::read_to_string(SAVE_FILE)
            .map_err(anyhow::Error::from)
            .and_then(|s| serde_json::from_str::<Game>(&s).map_err(anyhow::Error::from));
        match result {
            Ok(g) => {
                self.game = g;
                self.cursor = self.game.embark;
                self.mode = Mode::Normal;
                self.paused = true;
                self.flash = Some("Game loaded.".into());
            }
            Err(e) => self.flash = Some(format!("Load failed: {}", e)),
        }
    }
}
