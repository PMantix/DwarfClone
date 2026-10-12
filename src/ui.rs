use crate::app::{App, Mode, DESIG_KEYS};
use crate::buildings::BuildingKind;
use crate::dwarf::{skill_rank, Dwarf, Task, LABORS};
use crate::game::{Game, ZoneKind, HUNGRY, RIPE, THIRSTY, TIRED};
use crate::geom::Pos;
use crate::items::{ItemKind, StockFilter};
use crate::material::Mat;
use crate::world::{Desig, TileKind, World};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;
use std::collections::HashMap;

const SIDEBAR_W: u16 = 38;

fn rgb(c: (u8, u8, u8)) -> Color {
    Color::Rgb(c.0, c.1, c.2)
}

fn dim(c: Color, f: f32) -> Color {
    match c {
        Color::Rgb(r, g, b) => Color::Rgb((r as f32 * f) as u8, (g as f32 * f) as u8, (b as f32 * f) as u8),
        other => other,
    }
}

const BLACK: Color = Color::Rgb(0, 0, 0);
const KEY: Color = Color::Rgb(255, 210, 90);
const TEXT: Color = Color::Rgb(210, 210, 210);
const MUTED: Color = Color::Rgb(130, 130, 140);

fn hash2(x: i32, y: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^ (h >> 12)
}

/// Appearance of a bare tile (no units, items or buildings).
fn tile_look(w: &World, p: Pos) -> (char, Color, Color) {
    let t = w.t(p);
    if !t.revealed {
        return (' ', BLACK, BLACK);
    }
    let mc = t.mat.color();
    let water_bg = Color::Rgb(0, 30, 90);
    let water_fg = Color::Rgb(90, 160, 255);
    let (g, fg, bg) = match t.kind {
        TileKind::Open => {
            if t.water > 0 {
                return (water_glyph(t.water), water_fg, water_bg);
            }
            // Look down through open space.
            for k in 1..=14 {
                let q = p.offset(0, 0, -k);
                let Some(tq) = w.get(q) else { break };
                if !tq.revealed {
                    break;
                }
                let f = (0.62 - 0.035 * k as f32).max(0.22);
                if tq.water > 0 {
                    return ('≈', dim(water_fg, f), BLACK);
                }
                let c = dim(tq.mat.color(), f);
                return match tq.kind {
                    TileKind::Open => continue,
                    TileKind::Ramp => ('▼', c, BLACK),
                    TileKind::Tree => ('♣', c, BLACK),
                    TileKind::StairUp | TileKind::StairUD => ('<', c, BLACK),
                    _ => ('·', c, BLACK),
                };
            }
            (' ', BLACK, BLACK)
        }
        TileKind::Floor => {
            if t.mat == Mat::Grass {
                let h = hash2(p.x, p.y);
                let g = [',', '.', '`', '\'', ','][(h % 5) as usize];
                let shade = 0.75 + (h % 7) as f32 * 0.05;
                (g, dim(mc, shade), BLACK)
            } else {
                ('.', dim(mc, 0.8), BLACK)
            }
        }
        TileKind::Wall => {
            if t.mat == Mat::Bedrock {
                ('▓', mc, BLACK)
            } else if t.mat.is_gem() {
                ('☼', mc, Color::Rgb(40, 20, 50))
            } else if t.mat.is_ore() {
                ('£', mc, Color::Rgb(35, 30, 30))
            } else {
                ('█', dim(mc, 0.75), BLACK)
            }
        }
        TileKind::Ramp => ('▲', mc, BLACK),
        TileKind::StairUp => ('<', mc, BLACK),
        TileKind::StairDown => ('>', mc, BLACK),
        TileKind::StairUD => ('X', mc, BLACK),
        TileKind::Tree => {
            if w.outside(p) {
                ('♣', Color::Rgb(40, 170, 50), BLACK)
            } else {
                ('♠', Color::Rgb(170, 120, 210), BLACK)
            }
        }
        TileKind::Shrub => ('"', Color::Rgb(90, 190, 90), BLACK),
    };
    if t.water > 0 {
        return (water_glyph(t.water), water_fg, water_bg);
    }
    (g, fg, bg)
}

fn water_glyph(level: u8) -> char {
    if level >= 4 {
        '≈'
    } else {
        (b'0' + level) as char
    }
}

fn selection_box(app: &App) -> Option<(Pos, Pos)> {
    let corner = match &app.mode {
        Mode::Designate { corner, .. } | Mode::Stockpile { corner, .. } | Mode::Farm { corner } => *corner,
        _ => None,
    }?;
    Some((corner, app.cursor))
}

fn in_box(p: Pos, (a, b): (Pos, Pos)) -> bool {
    p.x >= a.x.min(b.x)
        && p.x <= a.x.max(b.x)
        && p.y >= a.y.min(b.y)
        && p.y <= a.y.max(b.y)
        && p.z >= a.z.min(b.z)
        && p.z <= a.z.max(b.z)
}

fn draw_map(f: &mut Frame, app: &mut App, area: Rect) {
    let g = &app.game;
    let z = app.cursor.z;
    let surface_here = g.world.surface_z(app.cursor.x, app.cursor.y);
    let rel = z - surface_here;
    let depth = if rel >= 0 { format!("+{}", rel) } else { format!("{}", rel) };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(MUTED))
        .title(Span::styled(
            format!(" {} — z {} ({} from surface) ", g.fortress_name, z, depth),
            Style::default().fg(KEY),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);
    app.map_rect = inner;

    let (vw, vh) = (inner.width as i32, inner.height as i32);
    let (ww, wh) = (g.world.w, g.world.h);
    // Keep the cursor inside the view with a margin.
    let margin = 4;
    if vw >= ww {
        app.view_x = -(vw - ww) / 2;
    } else {
        if app.cursor.x < app.view_x + margin {
            app.view_x = app.cursor.x - margin;
        }
        if app.cursor.x > app.view_x + vw - 1 - margin {
            app.view_x = app.cursor.x - vw + 1 + margin;
        }
        app.view_x = app.view_x.clamp(0, ww - vw);
    }
    if vh >= wh {
        app.view_y = -(vh - wh) / 2;
    } else {
        if app.cursor.y < app.view_y + margin {
            app.view_y = app.cursor.y - margin;
        }
        if app.cursor.y > app.view_y + vh - 1 - margin {
            app.view_y = app.cursor.y - vh + 1 + margin;
        }
        app.view_y = app.view_y.clamp(0, wh - vh);
    }

    let g = &app.game;
    let mut units: HashMap<(i32, i32), &Dwarf> = HashMap::new();
    for d in g.dwarves.iter().filter(|d| d.alive && d.pos.z == z) {
        units.insert((d.pos.x, d.pos.y), d);
    }
    let mut items: HashMap<(i32, i32), (char, Color)> = HashMap::new();
    for it in g.items.values().filter(|it| it.pos.z == z && it.carried_by.is_none()) {
        items.insert((it.pos.x, it.pos.y), it.glyph());
    }
    let sel = selection_box(app);
    let placing = match app.mode {
        Mode::Place(kind) => {
            let s = kind.size() / 2;
            let tl = app.cursor.offset(-s, -s, 0);
            Some((tl, kind.size(), g.can_place(kind, tl)))
        }
        _ => None,
    };

    let buf = f.buffer_mut();
    for sy in 0..vh {
        for sx in 0..vw {
            let (wx, wy) = (app.view_x + sx, app.view_y + sy);
            let cell = buf.get_mut(inner.x + sx as u16, inner.y + sy as u16);
            let p = Pos::new(wx, wy, z);
            if !g.world.in_bounds(p) {
                cell.set_char(' ').set_bg(BLACK).set_fg(BLACK);
                continue;
            }
            let t = g.world.t(p);
            let (mut ch, mut fg, mut bg) = tile_look(&g.world, p);

            if t.revealed && t.zone != 0 {
                match g.zones.get(&t.zone).map(|z| z.kind) {
                    Some(ZoneKind::Farm) => {
                        bg = Color::Rgb(55, 35, 15);
                        (ch, fg) = if t.farm == 0 {
                            ('·', Color::Rgb(150, 110, 70))
                        } else if t.farm >= RIPE {
                            ('♠', Color::Rgb(210, 130, 240))
                        } else if t.farm > RIPE / 2 {
                            ('"', Color::Rgb(150, 110, 170))
                        } else {
                            (',', Color::Rgb(120, 100, 130))
                        };
                    }
                    Some(ZoneKind::Stockpile(_)) => {
                        bg = Color::Rgb(30, 30, 50);
                        if ch == '.' || ch == ',' || ch == '`' || ch == '\'' {
                            ch = '=';
                            fg = Color::Rgb(70, 70, 100);
                        }
                    }
                    None => {}
                }
            }
            if t.building != 0 {
                if let Some(b) = g.buildings.get(&t.building) {
                    ch = b.glyph_at(wx - b.pos.x, wy - b.pos.y);
                    fg = if b.built {
                        if b.kind.is_workshop() { b.kind.color() } else { b.mat.color() }
                    } else {
                        Color::Rgb(90, 90, 90)
                    };
                }
            }
            if t.revealed {
                if let Some(&(ic, icol)) = items.get(&(wx, wy)) {
                    if t.building == 0 || !matches!(ch, 'C' | 'M' | 'S') {
                        ch = ic;
                        fg = icol;
                    }
                }
            }
            if let Some(d) = units.get(&(wx, wy)) {
                ch = '☺';
                fg = if d.asleep { dim(rgb(d.color), 0.5) } else { rgb(d.color) };
            }
            if t.desig != Desig::None {
                bg = Color::Rgb(110, 85, 0);
                if !t.revealed {
                    ch = '▒';
                    fg = Color::Rgb(160, 130, 40);
                }
            }
            if let Some(b) = sel {
                if in_box(p, b) {
                    bg = Color::Rgb(50, 60, 150);
                }
            }
            if let Some((tl, s, ok)) = placing {
                if wx >= tl.x && wx < tl.x + s && wy >= tl.y && wy < tl.y + s {
                    bg = if ok { Color::Rgb(20, 110, 20) } else { Color::Rgb(130, 20, 20) };
                }
            }
            if wx == app.cursor.x && wy == app.cursor.y {
                bg = Color::Rgb(220, 200, 0);
                fg = BLACK;
                if ch == ' ' {
                    ch = 'X';
                }
            }
            cell.set_char(ch).set_fg(fg).set_bg(bg);
        }
    }
}

fn key_line<'a>(k: &str, desc: &str, active: bool) -> Line<'a> {
    let style = if active {
        Style::default().fg(BLACK).bg(KEY)
    } else {
        Style::default().fg(TEXT)
    };
    Line::from(vec![
        Span::styled(format!(" {:>3} ", k), Style::default().fg(KEY).add_modifier(Modifier::BOLD)),
        Span::styled(desc.to_string(), style),
    ])
}

fn header<'a>(s: &str) -> Line<'a> {
    Line::from(Span::styled(s.to_string(), Style::default().fg(Color::Rgb(140, 200, 255)).add_modifier(Modifier::BOLD)))
}

fn plain<'a>(s: String, c: Color) -> Line<'a> {
    Line::from(Span::styled(s, Style::default().fg(c)))
}

fn task_desc(g: &Game, d: &Dwarf) -> String {
    if d.tantrum > 0 {
        return "Tantrum!".into();
    }
    let iname = |id: u32| g.items.get(&id).map_or("something".to_string(), |it| it.name());
    match &d.task {
        Task::Idle => "No job".into(),
        Task::Wander { .. } => "Wandering".into(),
        Task::Dig { target, .. } => {
            let ds = g.world.t(*target).desig;
            match ds {
                Desig::Chop => "Felling tree".into(),
                Desig::Channel => "Digging channel".into(),
                Desig::StairUp | Desig::StairDown | Desig::StairUD => "Carving stairway".into(),
                Desig::Ramp => "Carving ramp".into(),
                _ => "Mining".into(),
            }
        }
        Task::Haul { item, .. } => format!("Hauling {}", iname(*item)),
        Task::Construct { bld, .. } => {
            format!("Building {}", g.buildings.get(bld).map_or("?", |b| b.kind.name()))
        }
        Task::Craft { bld, .. } => g
            .buildings
            .get(bld)
            .and_then(|b| b.orders.first())
            .map_or("Crafting".into(), |o| o.recipe.name().to_string()),
        Task::Plant { .. } => "Planting stoutcaps".into(),
        Task::Harvest { .. } => "Harvesting".into(),
        Task::Eat { .. } => "Eating".into(),
        Task::DrinkItem { .. } | Task::DrinkWater { .. } => "Drinking".into(),
        Task::Sleep { .. } => if d.asleep { "Sleeping".into() } else { "Going to sleep".into() },
    }
}

fn cursor_info(g: &Game, p: Pos) -> Vec<Line<'static>> {
    let mut v = vec![header(&format!("Cursor ({}, {}, {})", p.x, p.y, p.z))];
    let t = g.world.t(p);
    if !t.revealed {
        v.push(plain("Unexplored".into(), MUTED));
        if t.desig != Desig::None {
            v.push(plain(format!("Designated: {}", t.desig.name()), KEY));
        }
        return v;
    }
    let desc = match t.kind {
        TileKind::Open => "open space".to_string(),
        TileKind::Tree => if g.world.outside(p) { "tree".into() } else { "tower mushroom".into() },
        TileKind::Shrub => "shrub".into(),
        k => format!("{} {}", t.mat.name(), k.name()),
    };
    v.push(plain(desc, t.mat.color()));
    if t.water > 0 {
        v.push(plain(format!("Water {}/7", t.water), Color::Rgb(90, 160, 255)));
    }
    if t.desig != Desig::None {
        v.push(plain(format!("Designated: {}", t.desig.name()), KEY));
    }
    if let Some(z) = g.zones.get(&t.zone) {
        match z.kind {
            ZoneKind::Farm => {
                let s = if t.farm == 0 {
                    "fallow".to_string()
                } else if t.farm >= RIPE {
                    "ripe!".to_string()
                } else {
                    format!("growing {}%", t.farm as u32 * 100 / RIPE as u32)
                };
                v.push(plain(format!("Farm plot ({})", s), Color::Rgb(200, 150, 90)));
            }
            ZoneKind::Stockpile(f) => v.push(plain(format!("Stockpile: {}", f.name()), Color::Rgb(150, 150, 220))),
        }
    }
    if let Some(b) = g.buildings.get(&t.building) {
        let st = if b.built { "" } else { " (awaiting construction)" };
        v.push(plain(format!("{}{}", b.name(), st), b.kind.color()));
    }
    for d in g.dwarves.iter().filter(|d| d.alive && d.pos == p) {
        v.push(Line::from(vec![
            Span::styled("☺ ", Style::default().fg(rgb(d.color))),
            Span::styled(format!("{} — {}", d.name, task_desc(g, d)), Style::default().fg(TEXT)),
        ]));
    }
    let here: Vec<_> = g.items.values().filter(|it| it.pos == p && it.carried_by.is_none()).collect();
    for it in here.iter().take(5) {
        let (c, col) = it.glyph();
        v.push(Line::from(vec![
            Span::styled(format!("{} ", c), Style::default().fg(col)),
            Span::styled(it.name(), Style::default().fg(TEXT)),
        ]));
    }
    if here.len() > 5 {
        v.push(plain(format!("...and {} more", here.len() - 5), MUTED));
    }
    v
}

fn stocks(g: &Game) -> Vec<Line<'static>> {
    let mut counts: HashMap<ItemKind, usize> = HashMap::new();
    for it in g.items.values() {
        *counts.entry(it.kind).or_default() += 1;
    }
    let c = |k| counts.get(&k).copied().unwrap_or(0);
    let stone = g.items.values().filter(|it| it.kind == ItemKind::Boulder && it.mat.is_building_stone()).count();
    vec![
        header("Fortress"),
        Line::from(vec![
            Span::styled(format!("Dwarves {:<4}", g.alive_count()), Style::default().fg(TEXT)),
            Span::styled(format!("Food {:<4}", c(ItemKind::Mushroom)), Style::default().fg(Mat::Stoutcap.color())),
            Span::styled(format!("Drink {}", c(ItemKind::Drink)), Style::default().fg(Color::Rgb(230, 170, 60))),
        ]),
        Line::from(vec![
            Span::styled(format!("Logs {:<6}", c(ItemKind::Log)), Style::default().fg(Mat::Wood.color())),
            Span::styled(format!("Stone {:<4}", stone), Style::default().fg(Mat::Granite.color())),
            Span::styled(format!("Beds {}", c(ItemKind::Bed)), Style::default().fg(TEXT)),
        ]),
    ]
}

fn sidebar(app: &App) -> (String, Vec<Line<'static>>) {
    let g = &app.game;
    let mut v: Vec<Line> = Vec::new();
    let title;
    match &app.mode {
        Mode::Normal | Mode::Help | Mode::ConfirmQuit => {
            title = "Main".to_string();
            v.extend(stocks(g));
            v.push(Line::default());
            v.push(header("Commands"));
            v.push(key_line("d", "Designate (dig, chop...)", false));
            v.push(key_line("p", "Stockpiles", false));
            v.push(key_line("z", "Farm plots", false));
            v.push(key_line("b", "Build", false));
            v.push(key_line("q", "Query building", false));
            v.push(key_line("u", "Unit list", false));
            v.push(key_line("v", "View dwarf at cursor", false));
            v.push(key_line("Tab", "Jump to next dwarf", false));
            v.push(key_line("?", "Help", false));
            v.push(Line::default());
            v.extend(cursor_info(g, app.cursor));
        }
        Mode::Designate { kind, corner } => {
            title = "Designate".to_string();
            for (k, d) in DESIG_KEYS.iter() {
                v.push(key_line(&k.to_string(), d.name(), d == kind));
            }
            v.push(Line::default());
            v.push(plain(
                if corner.is_some() { "Enter: set second corner".into() } else { "Enter: set first corner".into() },
                KEY,
            ));
            v.push(plain("Corners may be on different z-levels.".into(), MUTED));
            v.push(plain("Esc: cancel".into(), MUTED));
            v.push(Line::default());
            v.extend(cursor_info(g, app.cursor));
        }
        Mode::Stockpile { filter, corner } => {
            title = "Stockpiles".to_string();
            for f in StockFilter::ALL {
                v.push(key_line(&f.key().to_string(), f.name(), *filter == Some(f)));
            }
            v.push(key_line("x", "remove zones", filter.is_none()));
            v.push(Line::default());
            v.push(plain(
                if corner.is_some() { "Enter: set second corner".into() } else { "Enter: set first corner".into() },
                KEY,
            ));
            v.push(Line::default());
            v.extend(cursor_info(g, app.cursor));
        }
        Mode::Farm { corner } => {
            title = "Farm plots".to_string();
            v.push(plain("Mark soil floor (grass, loam, clay,".into(), TEXT));
            v.push(plain("sand) as farmland. Farmers plant".into(), TEXT));
            v.push(plain("stoutcaps; brewers turn them to ale.".into(), TEXT));
            v.push(Line::default());
            v.push(plain(
                if corner.is_some() { "Enter: set second corner".into() } else { "Enter: set first corner".into() },
                KEY,
            ));
            v.push(Line::default());
            v.extend(cursor_info(g, app.cursor));
        }
        Mode::BuildMenu => {
            title = "Build".to_string();
            for b in BuildingKind::ALL {
                v.push(key_line(&b.key().to_string(), &format!("{} ({})", b.name(), b.needs_desc()), false));
            }
            v.push(Line::default());
            v.push(plain("Esc: back".into(), MUTED));
        }
        Mode::Place(kind) => {
            title = "Place".to_string();
            v.push(plain(format!("Placing: {}", kind.name()), KEY));
            v.push(plain(format!("Needs {}", kind.needs_desc()), TEXT));
            v.push(Line::default());
            v.push(plain("Move the cursor; Enter to place.".into(), TEXT));
            v.push(plain("Esc: back".into(), MUTED));
            v.push(Line::default());
            v.extend(cursor_info(g, app.cursor));
        }
        Mode::Query { bld, adding } => {
            title = "Building".to_string();
            if let Some(b) = g.buildings.get(bld) {
                v.push(plain(b.name(), b.kind.color()));
                if !b.built {
                    v.push(plain(format!("Awaiting construction ({})", b.kind.needs_desc()), KEY));
                } else if b.kind == BuildingKind::Bed {
                    let who = b.occupant.and_then(|id| g.dwarves.iter().find(|d| d.id == id));
                    v.push(plain(who.map_or("Unoccupied".into(), |d| format!("In use by {}", d.name)), TEXT));
                }
                if let Some(w) = b.worker.and_then(|id| g.dwarves.iter().find(|d| d.id == id)) {
                    v.push(plain(format!("Worker: {}", w.name), TEXT));
                }
                if b.kind.is_workshop() && b.built {
                    v.push(Line::default());
                    v.push(header("Orders"));
                    if b.orders.is_empty() {
                        v.push(plain("(none)".into(), MUTED));
                    }
                    for (i, o) in b.orders.iter().enumerate() {
                        let rep = if o.repeat { " [repeat]" } else { "" };
                        let col = if i == 0 && b.worker.is_some() { KEY } else { TEXT };
                        v.push(plain(format!("{}. {}{}", i + 1, o.recipe.name(), rep), col));
                    }
                    v.push(Line::default());
                    if *adding {
                        v.push(header("Add order"));
                        for r in b.kind.recipes() {
                            v.push(key_line(&r.key().to_string(), &format!("{} ({})", r.name(), r.input_desc()), false));
                        }
                        v.push(plain("Esc: back".into(), MUTED));
                    } else {
                        v.push(key_line("a", "Add order", false));
                        v.push(key_line("r", "Toggle repeat on last", false));
                        v.push(key_line("x", "Remove last order", false));
                    }
                }
                v.push(key_line("X", "Remove building", false));
                v.push(plain("Esc: back".into(), MUTED));
            }
        }
        Mode::Units { sel } => {
            title = "Units".to_string();
            for (i, d) in g.dwarves.iter().enumerate() {
                let (mood, mc) = d.mood();
                let style = if i == *sel { Style::default().bg(Color::Rgb(50, 60, 120)) } else { Style::default() };
                let status = if d.alive { task_desc(g, d) } else { "Dead".into() };
                v.push(Line::from(vec![
                    Span::styled("☺ ", style.fg(if d.alive { rgb(d.color) } else { MUTED })),
                    Span::styled(format!("{:<20}", d.name), style.fg(TEXT)),
                    Span::styled(format!("{:.12}", d.profession()), style.fg(MUTED)),
                ]));
                if i == *sel {
                    v.push(Line::from(vec![
                        Span::styled(format!("   {} · ", status), Style::default().fg(TEXT)),
                        Span::styled(mood, Style::default().fg(rgb(mc))),
                    ]));
                }
            }
            v.push(Line::default());
            v.push(plain("Up/Down select · Enter view · c center".into(), MUTED));
        }
        Mode::Unit { id } => {
            title = "Dwarf".to_string();
            if let Some(d) = g.dwarves.iter().find(|d| d.id == *id) {
                v.extend(unit_view(g, d));
            }
        }
    }
    if let Some(f) = &app.flash {
        v.push(Line::default());
        v.push(plain(f.clone(), Color::Rgb(120, 255, 160)));
    }
    (title, v)
}

fn bar(label: &str, value: u32, warn: u32) -> Line<'static> {
    let frac = (value as f32 / (warn as f32 * 2.0)).min(1.0);
    let n = (frac * 16.0) as usize;
    let col = if value > warn * 2 {
        Color::Rgb(255, 60, 60)
    } else if value > warn {
        Color::Rgb(240, 190, 60)
    } else {
        Color::Rgb(90, 200, 90)
    };
    Line::from(vec![
        Span::styled(format!("{:<8}", label), Style::default().fg(TEXT)),
        Span::styled("█".repeat(n), Style::default().fg(col)),
        Span::styled("░".repeat(16 - n), Style::default().fg(Color::Rgb(50, 50, 50))),
    ])
}

fn unit_view(g: &Game, d: &Dwarf) -> Vec<Line<'static>> {
    let (mood, mc) = d.mood();
    let mut v = vec![
        Line::from(Span::styled(d.name.clone(), Style::default().fg(rgb(d.color)).add_modifier(Modifier::BOLD))),
        plain(d.profession().to_string(), MUTED),
        Line::from(vec![
            Span::styled("Mood: ", Style::default().fg(TEXT)),
            Span::styled(mood, Style::default().fg(rgb(mc))),
        ]),
        plain(if d.alive { task_desc(g, d) } else { "Deceased".into() }, KEY),
        Line::default(),
        bar("Thirst", d.thirst, THIRSTY),
        bar("Hunger", d.hunger, HUNGRY),
        bar("Fatigue", d.fatigue, TIRED),
        Line::default(),
        header("Labors (number to toggle)"),
    ];
    for (i, l) in LABORS.iter().enumerate() {
        let on = d.labors[i];
        let xp = d.skills[i];
        v.push(Line::from(vec![
            Span::styled(format!(" {} ", i + 1), Style::default().fg(KEY)),
            Span::styled(if on { "[x] " } else { "[ ] " }, Style::default().fg(if on { Color::Rgb(90, 220, 90) } else { MUTED })),
            Span::styled(format!("{:<13}", l.name()), Style::default().fg(TEXT)),
            Span::styled(if xp > 0 { skill_rank(xp) } else { "" }, Style::default().fg(MUTED)),
        ]));
    }
    v.push(Line::default());
    v.push(header("Recent thoughts"));
    for t in d.thoughts.iter().rev().take(5) {
        let col = if t.stress < 0 { Color::Rgb(120, 220, 120) } else { Color::Rgb(230, 140, 100) };
        v.push(plain(format!("· {}", t.text), col));
    }
    v.push(Line::default());
    v.push(plain("c: center on dwarf · Esc: back".into(), MUTED));
    v
}

fn help_text() -> Vec<Line<'static>> {
    let k = |a: &str, b: &str| {
        Line::from(vec![
            Span::styled(format!("  {:<16}", a), Style::default().fg(KEY)),
            Span::styled(b.to_string(), Style::default().fg(TEXT)),
        ])
    };
    vec![
        header("DWARFCLONE — fortress mode"),
        Line::default(),
        plain("You don't control dwarves directly. Mark work and".into(), TEXT),
        plain("they'll do it when they're able (and in the mood).".into(), TEXT),
        Line::default(),
        k("Arrows", "move cursor (Shift = x10)"),
        k("< > or , .", "up / down a z-level (also PgUp/PgDn)"),
        k("Space", "pause / resume"),
        k("+ / -", "game speed"),
        k("d", "designate: mine, channel, stairs, chop"),
        k("p", "stockpiles"),
        k("z", "farm plots (on soil)"),
        k("b", "build workshops and furniture"),
        k("q", "query building: queue work orders"),
        k("u / v", "unit list / view dwarf at cursor"),
        k("Tab", "jump to next dwarf"),
        k("H", "jump back to the wagon"),
        k("F5 / F9", "save / load"),
        k("Q", "quit"),
        Line::default(),
        header("Getting started"),
        plain(" 1. Chop some trees (d, t) for logs.".into(), TEXT),
        plain(" 2. Dig down: designate up/down stairs (d, i)".into(), TEXT),
        plain("    across several z-levels, then mine rooms.".into(), TEXT),
        plain(" 3. Make food and stone stockpiles (p).".into(), TEXT),
        plain(" 4. Farm stoutcaps (z), build a Still, brew ale.".into(), TEXT),
        plain(" 5. Carpenter's workshop -> beds. Dwarves sleep".into(), TEXT),
        plain("    better in beds; drinking water makes them sad.".into(), TEXT),
        Line::default(),
        plain("Mouse: click to move cursor / set corners; wheel".into(), MUTED),
        plain("changes z-level. Press any key to continue.".into(), MUTED),
    ]
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.size();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(10), Constraint::Length(7)])
        .split(area);
    let mid = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(SIDEBAR_W)])
        .split(rows[1]);

    // Status bar.
    let g = &app.game;
    let speed = if app.paused {
        Span::styled(" PAUSED ", Style::default().fg(BLACK).bg(Color::Rgb(230, 80, 80)))
    } else {
        Span::styled(format!(" x{} ", app.ticks_per_frame()), Style::default().fg(BLACK).bg(Color::Rgb(90, 200, 90)))
    };
    let status = Line::from(vec![
        Span::styled(" ☼ DwarfClone ", Style::default().fg(KEY).add_modifier(Modifier::BOLD)),
        Span::styled(format!("│ {} ", g.date_string()), Style::default().fg(TEXT)),
        Span::styled(format!("│ {} dwarves ", g.alive_count()), Style::default().fg(TEXT)),
        speed,
        Span::styled("  ? help", Style::default().fg(MUTED)),
    ]);
    f.render_widget(Paragraph::new(status).style(Style::default().bg(Color::Rgb(20, 20, 30))), rows[0]);

    draw_map(f, app, mid[0]);

    let (title, lines) = sidebar(app);
    let side = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(MUTED))
                .title(Span::styled(format!(" {} ", title), Style::default().fg(KEY))),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(side, mid[1]);

    // Announcements.
    let g = &app.game;
    let n = rows[2].height.saturating_sub(2) as usize;
    let log: Vec<Line> = g
        .log
        .iter()
        .rev()
        .take(n)
        .rev()
        .map(|e| plain(e.text.clone(), rgb(e.color)))
        .collect();
    f.render_widget(
        Paragraph::new(log).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(MUTED))
                .title(Span::styled(" Announcements ", Style::default().fg(KEY))),
        ),
        rows[2],
    );

    match app.mode {
        Mode::Help => popup(f, area, 60, 34, " Help ", help_text()),
        Mode::ConfirmQuit => popup(
            f,
            area,
            40,
            5,
            " Quit ",
            vec![plain("Really abandon the fortress? (y/n)".into(), TEXT), plain("(F5 saves first)".into(), MUTED)],
        ),
        _ => {}
    }
}

fn popup(f: &mut Frame, area: Rect, w: u16, h: u16, title: &str, lines: Vec<Line<'static>>) {
    let w = w.min(area.width);
    let h = h.min(area.height);
    let r = Rect::new(area.x + (area.width - w) / 2, area.y + (area.height - h) / 2, w, h);
    f.render_widget(Clear, r);
    f.render_widget(
        Paragraph::new(lines)
            .style(Style::default().bg(Color::Rgb(15, 15, 25)))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(KEY))
                    .title(Span::styled(title.to_string(), Style::default().fg(KEY))),
            ),
        r,
    );
}


#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn dump(app: &mut App, w: u16, h: u16) -> String {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| draw(f, app)).unwrap();
        let buf = term.backend().buffer().clone();
        let mut s = String::new();
        for y in 0..h {
            for x in 0..w {
                s.push_str(buf.get(x, y).symbol());
            }
            s.push('\n');
        }
        s
    }

    #[test]
    fn render_snapshot() {
        let mut app = App::new(42);
        app.mode = Mode::Normal;
        for _ in 0..3000 {
            app.game.step();
        }
        println!("{}", dump(&mut app, 130, 45));
        app.mode = Mode::Help;
        println!("{}", dump(&mut app, 130, 45));
        app.cursor.z -= 1;
        app.mode = Mode::Normal;
        println!("{}", dump(&mut app, 130, 45));
    }
}
