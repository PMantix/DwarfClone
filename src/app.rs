use crate::buildings::BuildingKind;
use crate::dwarf::LABORS;
use crate::game::{Game, ZoneKind};
use crate::geom::Pos;
use crate::items::StockFilter;
use crate::world::Desig;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

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

#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    Normal,
    Designate { kind: Desig, corner: Option<Pos> },
    /// `filter == None` erases zones.
    Stockpile { filter: Option<StockFilter>, corner: Option<Pos> },
    Farm { corner: Option<Pos> },
    BuildMenu,
    Place(BuildingKind),
    Query { bld: u32, adding: bool },
    Units { sel: usize },
    Unit { id: u32 },
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
}

impl App {
    pub fn new(seed: u64) -> App {
        let game = Game::new(seed);
        let cursor = game.embark;
        App {
            game,
            cursor,
            view_x: 0,
            view_y: 0,
            mode: Mode::Help,
            speed: 2,
            paused: true,
            quit: false,
            map_rect: Rect::default(),
            flash: None,
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
            Mode::Normal | Mode::Designate { .. } | Mode::Stockpile { .. } | Mode::Farm { .. } | Mode::Place(_)
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
            return;
        }
        // Time controls work everywhere except the quit prompt.
        if self.mode != Mode::ConfirmQuit {
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
            Mode::Farm { corner } => match k.code {
                KeyCode::Enter => self.confirm(),
                KeyCode::Esc => {
                    self.mode = if corner.is_some() { Mode::Farm { corner: None } } else { Mode::Normal }
                }
                _ => {}
            },
            Mode::BuildMenu => match k.code {
                KeyCode::Esc => self.mode = Mode::Normal,
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
                            self.mode = Mode::Unit { id: d.id };
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
            Mode::Unit { id } => match k.code {
                KeyCode::Esc => self.mode = Mode::Normal,
                KeyCode::Char('c') => {
                    if let Some(d) = self.game.dwarves.iter().find(|d| d.id == id) {
                        self.cursor = d.pos;
                    }
                    self.mode = Mode::Normal;
                }
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let i = c as usize - '1' as usize;
                    if let Some(&l) = LABORS.get(i) {
                        self.game.toggle_labor(id, l);
                    }
                }
                _ => {}
            },
            Mode::Help => self.mode = Mode::Normal,
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
            KeyCode::Char('z') => self.mode = Mode::Farm { corner: None },
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
            KeyCode::Char('v') => {
                let c = self.cursor;
                if let Some(d) = self.game.dwarves.iter().find(|d| d.alive && d.pos == c) {
                    self.mode = Mode::Unit { id: d.id };
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
            Mode::Farm { corner: None } => self.mode = Mode::Farm { corner: Some(c) },
            Mode::Farm { corner: Some(a) } => {
                let n = self.game.add_zone(a, c, ZoneKind::Farm);
                self.flash = Some(if n > 0 {
                    format!("Farm plot of {} tiles.", n)
                } else {
                    "Farm plots need revealed soil floor.".into()
                });
                self.mode = Mode::Farm { corner: None };
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
