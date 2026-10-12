use crate::app::{look_entries, App, LookEntry, Mode, ZoneTool, DESIG_KEYS};
use crate::body::PARTS;
use crate::buildings::BuildingKind;
use crate::dwarf::{skill_level, skill_rank, Dwarf, Task, LABORS};
use crate::game::{Game, ZoneKind, DAY, HUNGRY, RIPE, THIRSTY, TIRED};
use crate::geom::Pos;
use crate::items::{Item, ItemKind, StockFilter};
use crate::material::Mat;
use crate::overworld::{Overworld, Race};
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
        Mode::Designate { corner, .. } | Mode::Stockpile { corner, .. } | Mode::Zone { corner, .. } => *corner,
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
    let mut units: HashMap<(i32, i32), Vec<&Dwarf>> = HashMap::new();
    for d in g.dwarves.iter().filter(|d| d.alive && d.pos.z == z) {
        units.entry((d.pos.x, d.pos.y)).or_default().push(d);
    }
    // Dwarves sharing a tile (passing in a corridor, chatting) take turns being drawn.
    let blink = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |t| (t.as_millis() / 500) as usize);
    let mut beasts: HashMap<(i32, i32), (char, Color, bool)> = HashMap::new();
    for c in g.creatures.values().filter(|c| c.alive && c.pos.z == z && g.world.t(c.pos).revealed) {
        let info = c.species.info();
        beasts.insert((c.pos.x, c.pos.y), (info.glyph, rgb(info.color), c.hostile()));
    }
    let mut items: HashMap<(i32, i32), (char, Color)> = HashMap::new();
    for it in g.items.values().filter(|it| it.pos.z == z && it.carried_by.is_none() && it.container.is_none()) {
        // Containers win the tile so stockpiles read as rows of barrels and bins.
        let key = (it.pos.x, it.pos.y);
        if it.kind.is_container() || !items.contains_key(&key) {
            items.insert(key, it.glyph());
        }
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
                    Some(ZoneKind::Meeting) => {
                        bg = Color::Rgb(20, 50, 40);
                        if ch == '.' || ch == ',' || ch == '`' || ch == '\'' {
                            ch = '·';
                            fg = Color::Rgb(70, 130, 110);
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
                    if t.building == 0 || !ch.is_ascii_uppercase() {
                        ch = ic;
                        fg = icol;
                    }
                }
            }
            if let Some(&(bc, bcol, hostile)) = beasts.get(&(wx, wy)) {
                ch = bc;
                fg = bcol;
                if hostile {
                    bg = Color::Rgb(90, 0, 0);
                }
            }
            if let Some(ds) = units.get(&(wx, wy)) {
                let d = ds[blink % ds.len()];
                ch = if d.soldier { '☻' } else { '☺' };
                fg = if d.asleep || d.body.unconscious() { dim(rgb(d.color), 0.5) } else { rgb(d.color) };
                if d.body.bleeding > 0 {
                    bg = Color::Rgb(70, 0, 0);
                }
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
        Task::Rest { .. } => "Resting injury".into(),
        Task::Equip { item } => format!("Picking up {}", iname(*item)),
        Task::Attack { target } => {
            format!("Fighting the {}", g.creatures.get(target).map_or("enemy", |c| c.name()))
        }
        Task::Station { .. } => "Going to station".into(),
        Task::Train => "Combat drill".into(),
        Task::Flee => "Fleeing!".into(),
        Task::Mood => {
            let m = d.mood.as_ref();
            match m {
                Some(m) if m.work_left > 0 => "Working on something mysterious".into(),
                Some(m) if m.items.len() < m.needs.len() => format!("Demanding {}", m.needs[m.items.len()].desc()),
                _ => "Gathering materials (fey)".into(),
            }
        }
        Task::Dine { .. } => "Eating at a table".into(),
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
            ZoneKind::Meeting => v.push(plain("Meeting area".into(), Color::Rgb(110, 200, 170))),
        }
    }
    if let Some(b) = g.buildings.get(&t.building) {
        let st = if b.built { "" } else { " (awaiting construction)" };
        v.push(plain(format!("{}{}", b.name(), st), b.kind.color()));
    }
    for c in g.creatures.values().filter(|c| c.alive && c.pos == p) {
        let info = c.species.info();
        let state = if c.body.unconscious() {
            "unconscious"
        } else if c.leaving {
            "leaving"
        } else if c.hostile() {
            "hostile"
        } else {
            "peaceful"
        };
        v.push(Line::from(vec![
            Span::styled(format!("{} ", info.glyph), Style::default().fg(rgb(info.color))),
            Span::styled(format!("{} ({})", info.name, state), Style::default().fg(TEXT)),
        ]));
    }
    for d in g.dwarves.iter().filter(|d| d.alive && d.pos == p) {
        v.push(Line::from(vec![
            Span::styled("☺ ", Style::default().fg(rgb(d.color))),
            Span::styled(format!("{} — {}", d.name, task_desc(g, d)), Style::default().fg(TEXT)),
        ]));
    }
    let here: Vec<_> = g
        .items
        .values()
        .filter(|it| it.pos == p && it.carried_by.is_none() && it.container.is_none())
        .collect();
    for it in here.iter().take(5) {
        let (c, col) = it.glyph();
        let extra = if it.kind.is_container() { format!(" ({}/10)", g.contents(it.id)) } else { String::new() };
        v.push(Line::from(vec![
            Span::styled(format!("{} ", c), Style::default().fg(col)),
            Span::styled(format!("{}{}", it.name(), extra), Style::default().fg(TEXT)),
        ]));
        if it.kind.is_container() {
            let mut inside: HashMap<String, usize> = HashMap::new();
            for c in g.items.values().filter(|c| c.container == Some(it.id)) {
                *inside.entry(c.name()).or_default() += 1;
            }
            let mut inside: Vec<_> = inside.into_iter().collect();
            inside.sort();
            for (name, n) in inside.into_iter().take(4) {
                v.push(plain(format!("    {} x{}", name, n), MUTED));
            }
        }
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
    let food = c(ItemKind::Mushroom) + c(ItemKind::Meat) + c(ItemKind::Meal);
    vec![
        header("Fortress"),
        Line::from(vec![
            Span::styled(format!("Dwarves {:<4}", g.alive_count()), Style::default().fg(TEXT)),
            Span::styled(format!("Food {:<4}", food), Style::default().fg(Mat::Stoutcap.color())),
            Span::styled(format!("Drink {}", c(ItemKind::Drink)), Style::default().fg(Color::Rgb(230, 170, 60))),
        ]),
        Line::from(vec![
            Span::styled(format!("Meals {:<5}", c(ItemKind::Meal)), Style::default().fg(Color::Rgb(250, 200, 110))),
            Span::styled(format!("Seed {:<4}", c(ItemKind::Seed)), Style::default().fg(Color::Rgb(160, 120, 170))),
            Span::styled(format!("Bars {}", c(ItemKind::Bar)), Style::default().fg(Mat::Iron.color())),
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
            v.push(key_line("i", "Zones: farm plots, meeting area", false));
            v.push(key_line("b", "Build", false));
            v.push(key_line("q", "Query building", false));
            v.push(key_line("k", "Look around", false));
            v.push(key_line("z", "Fortress status", false));
            v.push(key_line("u", "Unit list", false));
            v.push(key_line("y", "Labors & work details", false));
            v.push(key_line("m", "Military", false));
            v.push(key_line("n", "Nobles, mandates & rooms", false));
            v.push(key_line("L", "Legends of the world", false));
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
        Mode::Zone { tool, corner } => {
            title = "Zones".to_string();
            for t in ZoneTool::ALL {
                v.push(key_line(&t.key().to_string(), t.name(), t == *tool));
            }
            v.push(Line::default());
            match tool {
                ZoneTool::Farm => {
                    v.push(plain("Mark soil floor (grass, loam, clay,".into(), TEXT));
                    v.push(plain("sand) as farmland. Farmers plant".into(), TEXT));
                    v.push(plain("stoutcaps; brewers turn them to ale.".into(), TEXT));
                }
                ZoneTool::Meeting => {
                    v.push(plain("Dwarves with nothing to do gather".into(), TEXT));
                    v.push(plain("here and chat (and make friends)".into(), TEXT));
                    v.push(plain("instead of loitering in corridors.".into(), TEXT));
                    v.push(plain("Tables and chairs may stand in it.".into(), MUTED));
                }
                ZoneTool::Erase => {
                    v.push(plain("Removes farm plots and meeting".into(), TEXT));
                    v.push(plain("areas (stockpiles: p, then x).".into(), TEXT));
                }
            }
            let meet = g.zones.values().filter(|z| z.kind == ZoneKind::Meeting).map(|z| z.tiles.len()).sum::<usize>();
            v.push(plain(format!("Meeting area: {} tiles", meet), MUTED));
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
                    let who = b.owner.and_then(|id| g.dwarves.iter().find(|d| d.id == id));
                    v.push(plain(who.map_or("Unassigned".into(), |d| format!("Owned by {}", d.name)), TEXT));
                    v.push(plain(
                        if b.room_value > 0 { format!("Bedroom value ☼{}", b.room_value) } else { "Not in an enclosed room".into() },
                        MUTED,
                    ));
                } else if b.kind == BuildingKind::Table {
                    v.push(plain(
                        if b.room_value > 0 { format!("Dining room value ☼{}", b.room_value) } else { "Not in an enclosed room".into() },
                        MUTED,
                    ));
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
                if b.kind == BuildingKind::Depot && b.built {
                    match &g.caravan {
                        Some(c) if c.arrived => v.push(plain(format!("Merchants from the {} are here.", c.civ_name), Color::Rgb(240, 240, 160))),
                        Some(c) => v.push(plain(format!("A caravan from the {} is on its way.", c.civ_name), MUTED)),
                        None => v.push(plain("No caravan at present.".into(), MUTED)),
                    }
                    let waiting = g.items.values().filter(|it| it.to_depot && !g.at_depot(it.pos)).count();
                    v.push(plain(format!("{} goods at depot, {} on the way", g.depot_goods().len(), waiting), TEXT));
                    v.push(key_line("g", "Bring goods to the depot", false));
                    v.push(key_line("G", "Cancel bringing goods", false));
                    v.push(key_line("t", "Trade", false));
                }
                if b.kind == BuildingKind::Door && b.built {
                    v.push(key_line("l", if b.locked { "Unlock door (locked)" } else { "Lock door" }, b.locked));
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
        Mode::Unit { id, sel } => {
            title = "Dwarf".to_string();
            if let Some(d) = g.dwarves.iter().find(|d| d.id == *id) {
                v.extend(unit_view(g, d, *sel));
            }
        }
        Mode::Look { sel } => {
            title = "Look".to_string();
            v.extend(look_lines(g, app.cursor, *sel));
        }
        Mode::WorldMap { .. } | Mode::Legends { .. } | Mode::Trade { .. } | Mode::Status { .. } | Mode::Labors { .. } => {
            title = String::new();
        }
        Mode::Nobles => {
            title = "Nobles".to_string();
            v.push(plain(format!("Fortress wealth: ☼{}", g.wealth()), Color::Rgb(255, 215, 60)));
            let civ = g.civ.map_or("none".to_string(), |c| g.overworld.civ(c).name.clone());
            v.push(plain(format!("Civilization: {}", civ), TEXT));
            v.push(plain(
                format!("Site: {}{}", g.biome.name(), if g.evil { " (evil)" } else { "" }),
                if g.evil { Color::Rgb(200, 120, 255) } else { MUTED },
            ));
            for n in g.neighbors.iter().take(4) {
                let war = g.civ.map_or(false, |c| g.overworld.civ(c).enemies.contains(&n.civ)) || n.race == Race::Goblin;
                v.push(plain(
                    format!("  {} {} away{}", n.race.plural(), n.dist, if war { " (hostile)" } else { "" }),
                    if war { Color::Rgb(240, 110, 90) } else { rgb_of(n.race.color()) },
                ));
            }
            v.push(Line::default());
            v.push(header("Nobility"));
            let mut any = false;
            for d in g.dwarves.iter().filter(|d| d.alive && d.noble.is_some()) {
                any = true;
                let n = d.noble.unwrap();
                let room = g.buildings.values().find(|b| b.owner == Some(d.id)).map_or(0, |b| b.room_value);
                v.push(plain(format!("{}: {}", n.title(), d.name), Color::Rgb(255, 230, 120)));
                let ok = room >= n.room_demand();
                v.push(plain(
                    format!("  bedroom value {} (wants {})", room, n.room_demand()),
                    if ok { Color::Rgb(120, 220, 120) } else { Color::Rgb(240, 130, 100) },
                ));
            }
            if !any {
                v.push(plain("(none)".into(), MUTED));
            }
            v.push(Line::default());
            v.push(header("Mandate"));
            match &g.mandate {
                Some(m) => {
                    let left = m.deadline.saturating_sub(g.tick) / DAY;
                    v.push(plain(format!("Produce {}", m.describe()), TEXT));
                    v.push(plain(format!("  {} days remaining", left), MUTED));
                }
                None => v.push(plain("(none)".into(), MUTED)),
            }
            v.push(Line::default());
            v.push(header("Bedrooms"));
            let mut beds: Vec<_> = g.buildings.values().filter(|b| b.kind == BuildingKind::Bed && b.built).collect();
            beds.sort_by_key(|b| std::cmp::Reverse(b.room_value));
            for b in beds.iter().take(12) {
                let owner = b.owner.and_then(|o| g.dwarves.iter().find(|d| d.id == o)).map_or("unassigned".to_string(), |d| d.name.clone());
                let room = if b.room_value > 0 { format!("room ☼{}", b.room_value) } else { "no room".into() };
                v.push(plain(format!("Θ {} — {}", room, owner), TEXT));
            }
            if beds.is_empty() {
                v.push(plain("No beds yet. Dig rooms and".into(), MUTED));
                v.push(plain("place beds in them.".into(), MUTED));
            }
            v.push(Line::default());
            v.push(plain("A room is floor enclosed by rock".into(), MUTED));
            v.push(plain("and doors, underground.".into(), MUTED));
        }
        Mode::Military => {
            title = "Military".to_string();
            let alert = if g.alert { "ENGAGE hostiles" } else { "Stand down" };
            v.push(key_line("a", &format!("Orders: {}", alert), g.alert));
            v.push(key_line("h", if g.hunt { "Hunting game: ON" } else { "Hunting game: off" }, g.hunt));
            v.push(key_line("s", "Station soldiers at cursor", false));
            v.push(key_line("x", "Clear station", false));
            if let Some(st) = g.station {
                v.push(plain(format!("Station: ({}, {}, {})", st.x, st.y, st.z), MUTED));
            }
            v.push(Line::default());
            v.push(header("Soldiers"));
            let soldiers: Vec<&Dwarf> = g.dwarves.iter().filter(|d| d.alive && d.soldier).collect();
            if soldiers.is_empty() {
                v.push(plain("None. Draft dwarves from their".into(), MUTED));
                v.push(plain("unit screen (v or u, then m).".into(), MUTED));
            }
            for d in soldiers {
                let weapon = d.held.and_then(|id| g.items.get(&id)).map_or("unarmed".into(), |it| it.name());
                v.push(Line::from(vec![
                    Span::styled("☻ ", Style::default().fg(rgb(d.color))),
                    Span::styled(d.name.clone(), Style::default().fg(TEXT)),
                ]));
                v.push(plain(format!("   {} · {}", weapon, task_desc(g, d)), MUTED));
            }
            let hostiles = g.hostile_count();
            if hostiles > 0 {
                v.push(Line::default());
                v.push(plain(format!("⚠ {} hostile creatures active", hostiles), Color::Rgb(255, 90, 90)));
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

fn unit_view(g: &Game, d: &Dwarf, sel: usize) -> Vec<Line<'static>> {
    let (mood, mc) = d.mood();
    let mut v = vec![
        Line::from(Span::styled(d.name.clone(), Style::default().fg(rgb(d.color)).add_modifier(Modifier::BOLD))),
        plain(
            match d.noble {
                Some(n) => format!("{} ({})", n.title(), d.profession()),
                None => d.profession().to_string(),
            },
            MUTED,
        ),
        Line::from(vec![
            Span::styled("Mood: ", Style::default().fg(TEXT)),
            Span::styled(mood, Style::default().fg(rgb(mc))),
        ]),
        plain(if d.alive { task_desc(g, d) } else { "Deceased".into() }, KEY),
        Line::default(),
        bar("Thirst", d.thirst, THIRSTY),
        bar("Hunger", d.hunger, HUNGRY),
        bar("Fatigue", d.fatigue, TIRED),
    ];
    let hurt: Vec<Line> = PARTS
        .iter()
        .filter(|p| d.body.part_status(**p) != "fine")
        .map(|p| plain(format!("  {}: {}", p.name(), d.body.part_status(*p)), Color::Rgb(240, 130, 100)))
        .collect();
    if !hurt.is_empty() || d.body.bleeding > 0 {
        v.push(Line::default());
        v.push(header("Health"));
        if d.body.bleeding > 0 {
            v.push(plain(format!("  Bleeding (blood {}%)", d.body.blood * 100 / d.body.max_blood), Color::Rgb(255, 60, 60)));
        }
        v.extend(hurt);
    }
    v.push(Line::default());
    v.push(header("Personality"));
    for t in d.personality.describe() {
        v.push(plain(format!("  {}", t), TEXT));
    }
    let food = match d.likes_food {
        Some(ItemKind::Meal) => "prepared meals",
        Some(ItemKind::Meat) => "meat",
        _ => "stoutcaps",
    };
    v.push(plain(format!("  Likes {} and {}", d.likes_mat.name(), food), MUTED));
    let mut rel: Vec<(u32, i32)> = d.relations.iter().copied().filter(|r| r.1 >= 30 || r.1 <= -30).collect();
    rel.sort_by_key(|r| std::cmp::Reverse(r.1.abs()));
    for (oid, r) in rel.into_iter().take(3) {
        if let Some(o) = g.dwarves.iter().find(|o| o.id == oid) {
            let (what, col) = if r >= 70 {
                ("Close friend", Color::Rgb(120, 220, 255))
            } else if r >= 30 {
                ("Friend", Color::Rgb(150, 220, 200))
            } else {
                ("Grudge", Color::Rgb(240, 140, 90))
            };
            let dead = if o.alive { "" } else { " (dead)" };
            v.push(plain(format!("  {}: {}{}", what, o.name, dead), col));
        }
    }
    let room = g.buildings.values().find(|b| b.owner == Some(d.id)).map(|b| b.room_value);
    v.push(plain(
        match room {
            Some(0) => "  Owns a bed (no room)".to_string(),
            Some(v) => format!("  Owns a bedroom (☼{})", v),
            None => "  Has no bed of their own".to_string(),
        },
        MUTED,
    ));
    v.push(Line::default());
    v.push(header(if d.soldier { "Soldier (m: discharge)" } else { "Civilian (m: draft)" }));
    let held = d.held.and_then(|id| g.items.get(&id)).map_or("nothing".to_string(), |it| it.name());
    v.push(plain(format!("  Holds {}", held), TEXT));
    for it in d.armor.iter().filter_map(|id| g.items.get(id)) {
        v.push(plain(format!("  Wears {}", it.name()), TEXT));
    }
    v.push(plain(format!("  Fighter: {}", skill_rank(d.fight_xp)), MUTED));
    v.push(Line::default());
    v.push(header("Labors (Up/Down, Enter toggles)"));
    v.push(plain(
        if d.specialist { "  Specialist: no hauling (s)".into() } else { "  Hauls when idle (s: specialist)".into() },
        MUTED,
    ));
    for (i, l) in LABORS.iter().enumerate() {
        let on = d.labors[i];
        let xp = d.skills[i];
        let style = if i == sel { Style::default().bg(Color::Rgb(50, 60, 120)) } else { Style::default() };
        v.push(Line::from(vec![
            Span::styled(if on { " [x] " } else { " [ ] " }, style.fg(if on { Color::Rgb(90, 220, 90) } else { MUTED })),
            Span::styled(format!("{:<14}", l.name()), style.fg(TEXT)),
            Span::styled(if xp > 0 { skill_rank(xp) } else { "" }, style.fg(MUTED)),
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
        k("i", "zones: farm plots, meeting area"),
        k("k", "look: inspect anything at the cursor"),
        k("z", "fortress status: population, stocks, health"),
        k("b", "build workshops and furniture"),
        k("q", "query building: queue work orders"),
        k("u / v", "unit list / view dwarf (m drafts)"),
        k("y", "labors: who does what, work details"),
        k("m", "military: station point, engage orders"),
        k("n", "nobles, mandates, bedrooms, wealth"),
        k("L", "legends: history, civs, sites, figures"),
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
        plain(" 4. Farm stoutcaps (i, f), build a Still, brew ale.".into(), TEXT),
        plain("    Give idle dwarves a meeting area (i, m).".into(), TEXT),
        plain("    A Kitchen turns food into better meals.".into(), TEXT),
        plain("    Ore -> Smelter (+fuel) -> bars -> Forge -> arms.".into(), TEXT),
        plain(" 5. Carpenter's workshop -> beds. Dwarves sleep".into(), TEXT),
        plain("    better in beds; drinking water makes them sad.".into(), TEXT),
        plain(" 6. Wolves, cave beasts and goblins will come.".into(), TEXT),
        plain("    Draft soldiers, arm them, lock doors (q, l).".into(), TEXT),
        plain(" 7. Dig enclosed rooms for beds and a dining hall".into(), TEXT),
        plain("    (tables + chairs). Keep nobles content.".into(), TEXT),
        plain(" 8. Build a trade depot; caravans come each year.".into(), TEXT),
        Line::default(),
        plain("Mouse: click to move cursor / set corners; wheel".into(), MUTED),
        plain("changes z-level. Press any key to continue.".into(), MUTED),
    ]
}

fn rgb_of(c: Color) -> Color {
    c
}

fn draw_world(f: &mut Frame, app: &mut App, cx: i32, cy: i32) {
    let area = f.size();
    let ow = &app.game.overworld;
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(10)])
        .split(area);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(SIDEBAR_W + 4)])
        .split(rows[1]);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" ☼ DwarfClone ", Style::default().fg(KEY).add_modifier(Modifier::BOLD)),
            Span::styled(format!("│ {} │ Year {} │ Choose a site to embark", ow.name, ow.year), Style::default().fg(TEXT)),
        ]))
        .style(Style::default().bg(Color::Rgb(20, 20, 30))),
        rows[0],
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(MUTED))
        .title(Span::styled(" World map ", Style::default().fg(KEY)));
    let inner = block.inner(cols[0]);
    f.render_widget(block, cols[0]);
    // Each world cell is two characters wide so the map isn't squashed.
    let ox = inner.x as i32 + ((inner.width as i32 - ow.w * 2) / 2).max(0);
    let oy = inner.y as i32 + ((inner.height as i32 - ow.h) / 2).max(0);
    let buf = f.buffer_mut();
    for y in 0..ow.h {
        for x in 0..ow.w {
            let (sx, sy) = (ox + x * 2, oy + y);
            if sx + 1 >= (inner.x + inner.width) as i32 || sy >= (inner.y + inner.height) as i32 {
                continue;
            }
            let c = ow.cell(x, y);
            let (mut ch, mut fg) = c.biome.glyph();
            let mut bg = if c.evil { Color::Rgb(45, 10, 55) } else { BLACK };
            if c.site > 0 {
                let s = &ow.sites[(c.site - 1) as usize];
                ch = if s.ruined { '×' } else { s.kind.glyph() };
                fg = match s.civ {
                    Some(civ) if !s.ruined => ow.civs[civ as usize].race.color(),
                    _ => Color::Rgb(150, 150, 150),
                };
            }
            if x == cx && y == cy {
                bg = Color::Rgb(220, 200, 0);
                fg = BLACK;
            }
            for dx in 0..2 {
                let cell = buf.get_mut((sx + dx) as u16, sy as u16);
                cell.set_char(if dx == 0 { ch } else { ' ' }).set_fg(fg).set_bg(bg);
            }
        }
    }
    let lines = world_info(ow, cx, cy, &app.flash);
    f.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(MUTED))
                    .title(Span::styled(" Site ", Style::default().fg(KEY))),
            )
            .wrap(Wrap { trim: false }),
        cols[1],
    );
}

fn world_info(ow: &Overworld, x: i32, y: i32, flash: &Option<String>) -> Vec<Line<'static>> {
    let c = ow.cell(x, y);
    let (g, col) = c.biome.glyph();
    let mut v = vec![
        Line::from(vec![
            Span::styled(format!("{} ", g), Style::default().fg(col)),
            Span::styled(cap_first(c.biome.name()), Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
            Span::styled(format!("  ({}, {})", x, y), Style::default().fg(MUTED)),
        ]),
    ];
    if c.evil {
        v.push(plain("An evil taint lies on this land.".into(), Color::Rgb(200, 120, 255)));
    }
    if let Some(s) = ow.site_at(x, y) {
        let owner = s.civ.map_or("no one".to_string(), |c| format!("the {}", ow.civ(c).name));
        let state = if s.ruined { "ruined " } else { "" };
        v.push(plain(format!("{}: {}{} of {}", s.name, state, s.kind.name(), owner), Color::Rgb(255, 230, 120)));
    } else if c.biome != crate::overworld::Biome::Ocean {
        let p = c.biome.params();
        v.push(plain(
            format!(
                "Terrain: {}, trees: {}",
                if p.relief > 1.1 { "rugged" } else if p.relief < 0.6 { "flat" } else { "rolling" },
                if p.forest > 1.0 { "dense" } else if p.forest < 0.3 { "sparse" } else { "some" }
            ),
            MUTED,
        ));
        v.push(plain(
            format!("Wildlife: {}", if p.savagery + if c.evil { 0.3 } else { 0.0 } > 0.35 { "savage" } else { "calm" }),
            MUTED,
        ));
    }
    v.push(Line::default());
    v.push(header("Neighbors"));
    let ns = ow.neighbors(x, y, 30);
    if ns.is_empty() {
        v.push(plain("None within reach.".into(), MUTED));
    }
    for n in ns.iter().take(6) {
        let civ = ow.civ(n.civ);
        let note = match n.race {
            Race::Goblin => " — sieges!",
            Race::Dwarf => " — your kin",
            _ => " — traders",
        };
        v.push(plain(format!("{} ({}), {} away{}", civ.name, n.race.plural(), n.dist, note), n.race.color()));
    }
    v.push(Line::default());
    let parent = ow.parent_civ(x, y).map(|c| ow.civ(c).name.clone());
    v.push(plain(format!("Expedition from: {}", parent.unwrap_or_else(|| "nobody".into())), TEXT));
    v.push(Line::default());
    v.push(header("Keys"));
    v.push(key_line("↑↓←→", "move (Shift = x5)", false));
    v.push(key_line("Enter", "embark here", false));
    v.push(key_line("l", "legends of this world", false));
    v.push(key_line("Q", "quit", false));
    v.push(Line::default());
    v.push(plain("Ω dwarves  # + humans  ♠ elves".into(), MUTED));
    v.push(plain("Ψ goblins  & beast lair  × ruins".into(), MUTED));
    if let Some(f) = flash {
        v.push(Line::default());
        v.push(plain(f.clone(), Color::Rgb(255, 150, 90)));
    }
    v
}

fn cap_first(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

fn legends_lines(ow: &Overworld, tab: usize) -> Vec<Line<'static>> {
    match tab {
        0 => ow.events.iter().map(|e| {
            Line::from(vec![
                Span::styled(format!("{:>4}  ", e.year), Style::default().fg(MUTED)),
                Span::styled(e.text.clone(), Style::default().fg(TEXT)),
            ])
        }).collect(),
        1 => ow.civs.iter().flat_map(|c| {
            let sites = ow.sites.iter().filter(|s| s.civ == Some(c.id) && !s.ruined).count();
            let ruler = c.ruler.map(|r| ow.figures[r as usize].name.clone()).unwrap_or_else(|| "none".into());
            let enemies: Vec<String> = c.enemies.iter().map(|e| ow.civ(*e).name.clone()).collect();
            vec![
                Line::from(Span::styled(
                    format!("{} ({}){}", c.name, c.race.plural(), if c.alive { "" } else { " — fallen" }),
                    Style::default().fg(if c.alive { c.race.color() } else { MUTED }).add_modifier(Modifier::BOLD),
                )),
                plain(format!("   {} sites, ruler: {}", sites, ruler), TEXT),
                plain(format!("   at war with: {}", if enemies.is_empty() { "nobody".into() } else { enemies.join(", ") }), MUTED),
            ]
        }).collect(),
        2 => ow.sites.iter().map(|s| {
            let owner = s.civ.map_or("—".to_string(), |c| ow.civ(c).race.plural().to_string());
            Line::from(vec![
                Span::styled(format!("{} ", if s.ruined { '×' } else { s.kind.glyph() }), Style::default().fg(KEY)),
                Span::styled(format!("{:<20}", s.name), Style::default().fg(TEXT)),
                Span::styled(
                    format!("{} of {}{}, founded {}", s.kind.name(), owner, if s.ruined { " (ruined)" } else { "" }, s.founded),
                    Style::default().fg(MUTED),
                ),
            ])
        }).collect(),
        _ => ow.figures.iter().map(|fg| {
            let life = match fg.died {
                Some(d) => format!("{}–{}, {}", fg.born, d, fg.cause.clone().unwrap_or_default()),
                None => format!("born {}", fg.born),
            };
            let title = fg.title.clone().map(|t| format!(", {}", t)).unwrap_or_default();
            Line::from(vec![
                Span::styled(format!("{}", fg.name), Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
                Span::styled(format!(" the {}{} ({})", fg.kind, title, life), Style::default().fg(MUTED)),
            ])
        }).collect(),
    }
}

fn draw_legends(f: &mut Frame, area: Rect, ow: &Overworld, tab: usize, scroll: usize) {
    let w = area.width.saturating_sub(6).min(120);
    let h = area.height.saturating_sub(4);
    let r = Rect::new(area.x + (area.width - w) / 2, area.y + 2, w, h);
    let lines = legends_lines(ow, tab);
    let visible = h.saturating_sub(4) as usize;
    let max = lines.len().saturating_sub(visible);
    let scroll = scroll.min(max);
    let names = ["1 History", "2 Civilizations", "3 Sites", "4 Figures"];
    let mut tabs: Vec<Span> = Vec::new();
    for (i, n) in names.iter().enumerate() {
        let st = if i == tab { Style::default().fg(BLACK).bg(KEY) } else { Style::default().fg(TEXT) };
        tabs.push(Span::styled(format!(" {} ", n), st));
        tabs.push(Span::raw(" "));
    }
    let mut body = vec![Line::from(tabs), Line::default()];
    body.extend(lines.into_iter().skip(scroll).take(visible));
    f.render_widget(Clear, r);
    f.render_widget(
        Paragraph::new(body)
            .style(Style::default().bg(Color::Rgb(15, 15, 25)))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(KEY))
                    .title(Span::styled(format!(" Legends of {} ", ow.name), Style::default().fg(KEY)))
                    .title_bottom(Span::styled(" Tab/1-4 switch · ↑↓ PgUp/PgDn scroll · Esc back ", Style::default().fg(MUTED))),
            ),
        r,
    );
}

fn draw_trade(f: &mut Frame, area: Rect, app: &App, side: usize, sel: [usize; 2]) {
    let g = &app.game;
    let Some(car) = &g.caravan else { return };
    let w = area.width.saturating_sub(4).min(110);
    let h = area.height.saturating_sub(4);
    let r = Rect::new(area.x + (area.width - w) / 2, area.y + 2, w, h);
    f.render_widget(Clear, r);
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(KEY))
        .style(Style::default().bg(Color::Rgb(15, 15, 25)))
        .title(Span::styled(format!(" Trading with the {} ", car.civ_name), Style::default().fg(KEY)))
        .title_bottom(Span::styled(" Tab side · ↑↓ move · Space mark · o offer · Esc leave ", Style::default().fg(MUTED)));
    let inner = outer.inner(r);
    f.render_widget(outer, r);
    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(inner);
    let halves = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(parts[0]);
    let ours = g.depot_goods();
    let lists: [(Vec<(u32, String, u32)>, &std::collections::HashSet<u32>, &str); 2] = [
        (ours.iter().map(|it| (it.id, it.name(), it.value())).collect(), &app.trade_ours, " Your goods at the depot "),
        (car.goods.iter().map(|it| (it.id, it.name(), it.value())).collect(), &app.trade_theirs, " Their wares "),
    ];
    for (i, (list, marks, title)) in lists.iter().enumerate() {
        let rows = halves[i].height.saturating_sub(2) as usize;
        let start = sel[i].saturating_sub(rows.saturating_sub(1));
        let lines: Vec<Line> = list
            .iter()
            .enumerate()
            .skip(start)
            .take(rows)
            .map(|(j, (id, name, val))| {
                let marked = marks.contains(id);
                let mut st = Style::default().fg(if marked { Color::Rgb(120, 255, 160) } else { TEXT });
                if j == sel[i] && side == i {
                    st = st.bg(Color::Rgb(50, 60, 120));
                }
                Line::from(Span::styled(format!("{} {:<34} ☼{}", if marked { "■" } else { " " }, name, val), st))
            })
            .collect();
        let empty = if list.is_empty() {
            vec![plain(if i == 0 { "Nothing here. Query the depot and press g".into() } else { "Sold out.".into() }, MUTED)]
        } else {
            lines
        };
        f.render_widget(
            Paragraph::new(empty).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(if side == i { KEY } else { MUTED }))
                    .title(Span::styled(title.to_string(), Style::default().fg(TEXT))),
            ),
            halves[i],
        );
    }
    let offer: u32 = ours.iter().filter(|it| app.trade_ours.contains(&it.id)).map(|it| it.value()).sum();
    let ask: u32 = car.goods.iter().filter(|it| app.trade_theirs.contains(&it.id)).map(|it| it.value()).sum();
    let ok = offer * 10 >= ask * 11;
    let mut summary = vec![Line::from(vec![
        Span::styled(format!(" Offer ☼{}   ", offer), Style::default().fg(TEXT)),
        Span::styled(format!("Request ☼{}   ", ask), Style::default().fg(TEXT)),
        Span::styled(
            if ok { "The merchants look interested." } else { "They want at least a tenth more." },
            Style::default().fg(if ok { Color::Rgb(120, 255, 160) } else { Color::Rgb(240, 150, 90) }),
        ),
    ])];
    if let Some(fl) = &app.flash {
        summary.push(plain(format!(" {}", fl), KEY));
    }
    f.render_widget(Paragraph::new(summary), parts[1]);
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.size();
    if let Mode::WorldMap { x, y } = app.mode {
        draw_world(f, app, x, y);
        return;
    }
    if let Mode::Legends { tab, scroll, from_world: true } = app.mode {
        let (x, y) = app.game.site;
        draw_world(f, app, x, y);
        draw_legends(f, area, &app.game.overworld, tab, scroll);
        return;
    }
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
        if g.hostile_count() > 0 {
            Span::styled(format!(" ⚠ {} hostiles ", g.hostile_count()), Style::default().fg(Color::Rgb(255, 255, 255)).bg(Color::Rgb(160, 20, 20)))
        } else {
            Span::raw("")
        },
        Span::raw(" "),
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
        Mode::Legends { tab, scroll, .. } => draw_legends(f, area, &app.game.overworld, tab, scroll),
        Mode::Trade { side, sel } => draw_trade(f, area, app, side, sel),
        Mode::Status { tab, scroll } => draw_status(f, area, &app.game, tab, scroll),
        Mode::Labors { row, col, detail } => draw_labors(f, area, app, row, col, detail),
        Mode::Help => popup(f, area, 60, 47, " Help ", help_text()),
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

fn item_line(it: &Item, extra: &str, selected: bool) -> Line<'static> {
    let (c, col) = it.glyph();
    let style = if selected { Style::default().bg(Color::Rgb(50, 60, 120)) } else { Style::default() };
    Line::from(vec![
        Span::styled(format!("{} ", c), style.fg(col)),
        Span::styled(format!("{}{}", it.name(), extra), style.fg(TEXT)),
    ])
}

/// The look panel: terrain, then everything standing or lying on the tile.
fn look_lines(g: &Game, p: Pos, sel: usize) -> Vec<Line<'static>> {
    let mut v = cursor_info_terrain(g, p);
    let entries = look_entries(g, p);
    if entries.is_empty() {
        v.push(Line::default());
        v.push(plain("Nothing else here.".into(), MUTED));
        v.push(Line::default());
        v.push(plain("Arrows move · Esc back".into(), MUTED));
        return v;
    }
    let sel = sel.min(entries.len() - 1);
    v.push(Line::default());
    v.push(header(&format!("Here ({})", entries.len())));
    let hi = |i: usize| if i == sel { Style::default().bg(Color::Rgb(50, 60, 120)) } else { Style::default() };
    for (i, e) in entries.iter().enumerate().take(14) {
        let line = match *e {
            LookEntry::Dwarf(id) => {
                let Some(d) = g.dwarves.iter().find(|d| d.id == id) else { continue };
                Line::from(vec![
                    Span::styled(if d.soldier { "☻ " } else { "☺ " }, hi(i).fg(rgb(d.color))),
                    Span::styled(d.name.clone(), hi(i).fg(TEXT)),
                ])
            }
            LookEntry::Creature(id) => {
                let Some(c) = g.creatures.get(&id) else { continue };
                let info = c.species.info();
                Line::from(vec![
                    Span::styled(format!("{} ", info.glyph), hi(i).fg(rgb(info.color))),
                    Span::styled(cap(info.name), hi(i).fg(TEXT)),
                ])
            }
            LookEntry::Building(id) => {
                let Some(b) = g.buildings.get(&id) else { continue };
                Line::from(vec![
                    Span::styled(format!("{} ", b.glyph_at(p.x - b.pos.x, p.y - b.pos.y)), hi(i).fg(b.kind.color())),
                    Span::styled(b.name(), hi(i).fg(TEXT)),
                ])
            }
            LookEntry::Item(id) => {
                let Some(it) = g.items.get(&id) else { continue };
                item_line(it, "", i == sel)
            }
        };
        v.push(line);
    }
    if entries.len() > 14 {
        v.push(plain(format!("...and {} more", entries.len() - 14), MUTED));
    }
    v.push(Line::default());
    let detail_col = Color::Rgb(180, 180, 190);
    match entries[sel] {
        LookEntry::Dwarf(id) => {
            if let Some(d) = g.dwarves.iter().find(|d| d.id == id) {
                let (mood, mc) = d.mood();
                v.push(header(&d.name));
                v.push(plain(
                    match d.noble {
                        Some(n) => format!("{}, {}", n.title(), d.profession()),
                        None => d.profession().to_string(),
                    },
                    MUTED,
                ));
                v.push(plain(task_desc(g, d), KEY));
                v.push(plain(mood.to_string(), rgb(mc)));
                if let Some(t) = d.thoughts.last() {
                    v.push(plain(format!("Recently {}.", t.text), detail_col));
                }
                v.push(plain("Enter: full dwarf screen".into(), MUTED));
            }
        }
        LookEntry::Creature(id) => {
            if let Some(c) = g.creatures.get(&id) {
                let info = c.species.info();
                v.push(header(&cap(info.name)));
                let state = if c.body.unconscious() {
                    "Unconscious"
                } else if c.leaving {
                    "Leaving the area"
                } else if c.hostile() {
                    if c.siege { "Hostile (siege)" } else { "Hostile" }
                } else {
                    "Peaceful"
                };
                v.push(plain(state.into(), if c.hostile() { Color::Rgb(255, 100, 90) } else { detail_col }));
                if let Some(w) = c.weapon.and_then(|id| g.items.get(&id)) {
                    v.push(plain(format!("Wields {}", w.name()), detail_col));
                }
                for a in c.armor.iter().filter_map(|id| g.items.get(id)) {
                    v.push(plain(format!("Wears {}", a.name()), detail_col));
                }
                if c.body.bleeding > 0 {
                    v.push(plain("Bleeding".into(), Color::Rgb(255, 60, 60)));
                }
                for part in PARTS.iter().filter(|p| c.body.part_status(**p) != "fine") {
                    v.push(plain(format!("{}: {}", part.name(), c.body.part_status(*part)), Color::Rgb(240, 130, 100)));
                }
            }
        }
        LookEntry::Building(id) => {
            if let Some(b) = g.buildings.get(&id) {
                v.push(header(&b.name()));
                v.push(plain(
                    if b.built { format!("Worth ☼{}", g.building_value(b)) } else { format!("Awaiting construction ({})", b.kind.needs_desc()) },
                    detail_col,
                ));
                if let Some(o) = b.owner.and_then(|o| g.dwarves.iter().find(|d| d.id == o)) {
                    v.push(plain(format!("Owned by {}", o.name), detail_col));
                }
                if let Some(w) = b.worker.and_then(|o| g.dwarves.iter().find(|d| d.id == o)) {
                    v.push(plain(format!("In use by {}", w.name), detail_col));
                }
                if !b.orders.is_empty() {
                    v.push(plain(format!("{} work orders queued", b.orders.len()), detail_col));
                }
                v.push(plain("Enter: query building".into(), MUTED));
            }
        }
        LookEntry::Item(id) => {
            if let Some(it) = g.items.get(&id) {
                v.push(header(&cap(&it.name())));
                v.push(plain(format!("{}, worth ☼{}", cap(it.mat.name()), it.value()), detail_col));
                if let Some(d) = it.claimed_by.and_then(|o| g.dwarves.iter().find(|d| d.id == o)) {
                    v.push(plain(format!("Wanted by {}", d.name), detail_col));
                }
                if it.to_depot {
                    v.push(plain("Marked for the trade depot".into(), Color::Rgb(240, 240, 160)));
                }
                if it.kind.is_container() {
                    let mut inside: Vec<&Item> = g.items.values().filter(|c| c.container == Some(it.id)).collect();
                    inside.sort_by_key(|c| c.name());
                    v.push(plain(format!("Holds {} of {}", inside.len(), crate::items::CONTAINER_CAP), detail_col));
                    for c in inside {
                        v.push(item_line(c, "", false));
                    }
                }
            }
        }
    }
    v.push(Line::default());
    v.push(plain("Arrows move · Tab next thing · Esc back".into(), MUTED));
    v
}

/// The terrain part of the cursor description (no units or items).
fn cursor_info_terrain(g: &Game, p: Pos) -> Vec<Line<'static>> {
    let mut v = cursor_info(g, p);
    // cursor_info lists units and items after the tile itself; keep only the tile part.
    let t = g.world.t(p);
    let mut keep = 1;
    if t.revealed {
        keep += 1;
        keep += (t.water > 0) as usize;
        keep += (t.desig != Desig::None) as usize;
        keep += g.zones.contains_key(&t.zone) as usize;
        keep += g.buildings.contains_key(&t.building) as usize;
    } else {
        keep += 1 + (t.desig != Desig::None) as usize;
    }
    v.truncate(keep);
    if t.revealed && g.world.outside(p) {
        v.push(plain("Outside".into(), MUTED));
    } else if t.revealed && t.kind.has_floor() {
        v.push(plain("Underground".into(), MUTED));
    }
    v
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn status_lines(g: &Game, tab: usize) -> Vec<Line<'static>> {
    let mut v = Vec::new();
    let alive: Vec<&Dwarf> = g.dwarves.iter().filter(|d| d.alive).collect();
    let n = alive.len().max(1);
    match tab {
        0 => {
            let civ = g.civ.map_or("no civilization".to_string(), |c| format!("the {}", g.overworld.civ(c).name));
            v.push(header(&format!("{}, an outpost of {}", g.fortress_name, civ)));
            v.push(plain(g.date_string(), TEXT));
            v.push(plain(format!("Fortress wealth ☼{}", g.wealth()), Color::Rgb(255, 215, 60)));
            v.push(Line::default());

            let food = g.items.values().filter(|it| it.kind.is_food() && it.kind != ItemKind::Drink && it.kind != ItemKind::Seed).count();
            let drink = g.items.values().filter(|it| it.kind == ItemKind::Drink).count();
            v.push(header("Supplies"));
            // A dwarf eats about every HUNGRY ticks and drinks about every THIRSTY ticks.
            let days = |x: usize, every: u32| x as u64 * every as u64 / DAY / n as u64;
            let warn = |d: u64| if d < 20 { Color::Rgb(255, 120, 90) } else { TEXT };
            let (fd, dd) = (days(food, HUNGRY), days(drink, THIRSTY));
            v.push(plain(format!("Food  {:>4}  (about {} days for {} dwarves)", food, fd, alive.len()), warn(fd)));
            v.push(plain(format!("Drink {:>4}  (about {} days)", drink, dd), warn(dd)));
            let beds = g.buildings.values().filter(|b| b.kind == BuildingKind::Bed && b.built).count();
            v.push(plain(format!("Beds  {:>4}  for {} dwarves", beds, alive.len()), if beds < alive.len() { Color::Rgb(240, 190, 60) } else { TEXT }));
            let meet: usize = g.zones.values().filter(|z| z.kind == ZoneKind::Meeting).map(|z| z.tiles.len()).sum();
            v.push(plain(
                if meet > 0 { format!("Meeting area of {} tiles", meet) } else { "No meeting area (i, m to make one)".into() },
                if meet > 0 { TEXT } else { Color::Rgb(240, 190, 60) },
            ));
            v.push(Line::default());

            v.push(header(&format!("Population: {}", alive.len())));
            let mut profs: HashMap<&'static str, usize> = HashMap::new();
            for d in &alive {
                *profs.entry(if d.soldier { "Soldier" } else { d.profession() }).or_default() += 1;
            }
            let mut profs: Vec<_> = profs.into_iter().collect();
            profs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
            for (p, c) in profs {
                v.push(plain(format!("  {:>3}  {}", c, p), TEXT));
            }
            v.push(Line::default());

            v.push(header("Activity"));
            let mut acts: HashMap<&'static str, usize> = HashMap::new();
            for d in &alive {
                let a = match d.task {
                    Task::Idle | Task::Wander { .. } => {
                        if g.in_meeting_area(d.pos) { "At the meeting area" } else { "Idle" }
                    }
                    Task::Sleep { .. } => "Sleeping",
                    Task::Rest { .. } => "Resting injuries",
                    Task::Eat { .. } | Task::Dine { .. } | Task::DrinkItem { .. } | Task::DrinkWater { .. } => "Eating or drinking",
                    Task::Attack { .. } | Task::Station { .. } | Task::Train | Task::Equip { .. } => "Military duty",
                    Task::Flee => "Fleeing",
                    Task::Mood => "Strange mood",
                    _ => "Working",
                };
                *acts.entry(a).or_default() += 1;
            }
            let mut acts: Vec<_> = acts.into_iter().collect();
            acts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
            for (a, c) in acts {
                v.push(plain(format!("  {:>3}  {}", c, a), TEXT));
            }
            v.push(Line::default());

            v.push(header("Happiness"));
            let mut moods: Vec<(&'static str, (u8, u8, u8), usize)> = Vec::new();
            for d in &alive {
                let (m, c) = d.mood();
                match moods.iter_mut().find(|x| x.0 == m) {
                    Some(x) => x.2 += 1,
                    None => moods.push((m, c, 1)),
                }
            }
            moods.sort_by_key(|x| std::cmp::Reverse(x.2));
            for (m, c, k) in moods {
                v.push(plain(format!("  {:>3}  {}", k, m), rgb(c)));
            }
            let hostiles = g.hostile_count();
            if hostiles > 0 {
                v.push(Line::default());
                v.push(plain(format!("⚠ {} hostile creatures on the map", hostiles), Color::Rgb(255, 90, 90)));
            }
        }
        1 => {
            v.push(header("Stocks (everything in the fortress, by category)"));
            for f in StockFilter::ALL.iter().filter(|f| **f != StockFilter::All) {
                let mut names: HashMap<String, usize> = HashMap::new();
                let mut total = 0;
                for it in g.items.values().filter(|it| f.accepts(it.kind)) {
                    let base = Item { quality: 0, ..it.clone() };
                    let name = if it.quality >= crate::items::ARTIFACT { it.name() } else { base.name() };
                    *names.entry(name).or_default() += 1;
                    total += 1;
                }
                v.push(Line::default());
                v.push(header(&format!("{} ({})", cap(f.name()), total)));
                if names.is_empty() {
                    v.push(plain("  none".into(), MUTED));
                }
                let mut names: Vec<_> = names.into_iter().collect();
                names.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                for (name, c) in names {
                    v.push(plain(format!("  {:>4}  {}", c, name), TEXT));
                }
            }
        }
        2 => {
            v.push(header("Health"));
            let mut any = false;
            for d in &alive {
                let hurt: Vec<String> = PARTS
                    .iter()
                    .filter(|p| d.body.part_status(**p) != "fine")
                    .map(|p| format!("{} {}", p.name(), d.body.part_status(*p)))
                    .collect();
                let mut flags = Vec::new();
                if d.body.unconscious() {
                    flags.push("unconscious".to_string());
                }
                if d.body.bleeding > 0 {
                    flags.push(format!("bleeding (blood {}%)", d.body.blood * 100 / d.body.max_blood));
                }
                if matches!(d.task, Task::Rest { .. }) {
                    flags.push("resting".into());
                }
                if d.thirst > THIRSTY * 2 {
                    flags.push("dehydrated".into());
                }
                if d.hunger > HUNGRY * 2 {
                    flags.push("starving".into());
                }
                if d.fatigue > TIRED * 2 {
                    flags.push("exhausted".into());
                }
                if hurt.is_empty() && flags.is_empty() {
                    continue;
                }
                any = true;
                v.push(Line::default());
                v.push(Line::from(vec![
                    Span::styled("☺ ", Style::default().fg(rgb(d.color))),
                    Span::styled(d.name.clone(), Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
                    Span::styled(if flags.is_empty() { String::new() } else { format!("  {}", flags.join(", ")) }, Style::default().fg(Color::Rgb(255, 90, 80))),
                ]));
                for h in hurt {
                    v.push(plain(format!("    {}", h), Color::Rgb(240, 130, 100)));
                }
            }
            if !any {
                v.push(plain("Everyone is in good health.".into(), Color::Rgb(120, 220, 120)));
            }
            let dead: Vec<&Dwarf> = g.dwarves.iter().filter(|d| !d.alive).collect();
            if !dead.is_empty() {
                v.push(Line::default());
                v.push(header(&format!("Dead ({})", dead.len())));
                for d in dead {
                    v.push(plain(format!("  {}", d.name), MUTED));
                }
            }
        }
        _ => {
            v.push(header("Dwarves"));
            for d in &alive {
                let (mood, mc) = d.mood();
                v.push(Line::default());
                v.push(Line::from(vec![
                    Span::styled(if d.soldier { "☻ " } else { "☺ " }, Style::default().fg(rgb(d.color))),
                    Span::styled(format!("{:<22}", d.name), Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("{:<18}", d.profession()), Style::default().fg(MUTED)),
                    Span::styled(mood.to_string(), Style::default().fg(rgb(mc))),
                ]));
                v.push(plain(format!("    {}", task_desc(g, d)), KEY));
                for t in d.thoughts.iter().rev().take(2) {
                    let col = if t.stress < 0 { Color::Rgb(120, 220, 120) } else { Color::Rgb(230, 140, 100) };
                    v.push(plain(format!("    · {}", t.text), col));
                }
            }
        }
    }
    v
}

fn draw_labors(f: &mut Frame, area: Rect, app: &App, row: usize, col: usize, detail: usize) {
    let g = &app.game;
    let w = area.width.saturating_sub(4).min(110);
    let h = area.height.saturating_sub(2);
    let r = Rect::new(area.x + (area.width - w) / 2, area.y + 1, w, h);
    let alive: Vec<&Dwarf> = g.dwarves.iter().filter(|d| d.alive).collect();
    let row = row.min(alive.len().saturating_sub(1));
    let ncols = LABORS.len() + 1;
    let on_bg = Color::Rgb(30, 90, 40);
    let cursor_bg = Color::Rgb(220, 200, 0);

    let mut v: Vec<Line> = Vec::new();
    // Header: labor abbreviations, the active column highlighted.
    let mut hdr = vec![Span::styled(format!("{:<38}", "Dwarf"), Style::default().fg(MUTED))];
    for c in 0..ncols {
        let name = if c < LABORS.len() { LABORS[c].short() } else { "Spc" };
        let st = if c == col { Style::default().fg(BLACK).bg(KEY) } else { Style::default().fg(KEY) };
        hdr.push(Span::styled(format!("{:^4}", name), st));
    }
    v.push(Line::from(hdr));

    // Leave room for the cursor info and work details below the grid.
    let room = (h as usize).saturating_sub(4 + 6 + g.work_details.len().min(9));
    let first = row.saturating_sub(room.saturating_sub(1));
    for (i, d) in alive.iter().enumerate().skip(first).take(room.max(1)) {
        let sel_row = i == row;
        let name_st = if sel_row { Style::default().fg(TEXT).bg(Color::Rgb(50, 60, 120)) } else { Style::default().fg(TEXT) };
        let prof = if d.soldier { "Soldier" } else { d.profession() };
        let mut spans = vec![
            Span::styled(if d.soldier { "☻ " } else { "☺ " }, Style::default().fg(rgb(d.color))),
            Span::styled(format!("{:<19.19}", d.name), name_st),
            Span::styled(format!("{:<17.17}", prof), name_st.fg(MUTED)),
        ];
        for c in 0..ncols {
            let (text, mut st) = if c < LABORS.len() {
                let l = LABORS[c];
                let xp = d.skills[l.idx()];
                let lvl = if xp > 0 { format!("{}", skill_level(xp)) } else { String::new() };
                if d.has(l) {
                    (if lvl.is_empty() { "■".to_string() } else { lvl }, Style::default().fg(Color::Rgb(150, 255, 150)).bg(on_bg))
                } else {
                    (if lvl.is_empty() { "·".to_string() } else { lvl }, Style::default().fg(Color::Rgb(90, 90, 100)))
                }
            } else if d.specialist {
                ("S".to_string(), Style::default().fg(Color::Rgb(255, 200, 120)).bg(Color::Rgb(90, 60, 20)))
            } else {
                ("·".to_string(), Style::default().fg(Color::Rgb(90, 90, 100)))
            };
            if sel_row && c == col {
                st = st.fg(BLACK).bg(cursor_bg);
            }
            spans.push(Span::styled(format!("{:^4}", text), st));
        }
        v.push(Line::from(spans));
    }
    if alive.len() > first + room.max(1) {
        v.push(plain(format!("  ...{} more below", alive.len() - first - room.max(1)), MUTED));
    }

    v.push(Line::default());
    if let Some(d) = alive.get(row) {
        if col < LABORS.len() {
            let l = LABORS[col];
            let xp = d.skills[l.idx()];
            let skill = if xp > 0 { format!("{} {}", skill_rank(xp), l.name().to_lowercase()) } else { format!("no {} skill", l.name().to_lowercase()) };
            let state = if d.has(l) { "does" } else { "doesn't do" };
            let mut line = format!("{} {} {} ({}).", d.name, state, l.name().to_lowercase(), skill);
            if let Some(r) = l.exclusive_with() {
                line.push_str(&format!(" Excludes {}.", r.name().to_lowercase()));
            }
            v.push(plain(line, TEXT));
        } else {
            v.push(plain(
                format!("{} {}", d.name, if d.specialist { "is a specialist: never hauls." } else { "hauls when there's nothing else." }),
                TEXT,
            ));
        }
    }
    v.push(plain("Numbers are skill levels (0 dabbling .. 10 legendary). Green = enabled.".into(), MUTED));

    v.push(Line::default());
    v.push(header("Work details   [ ] select · w or 1-9 apply to dwarf · S save dwarf's labors · X delete"));
    for (i, wd) in g.work_details.iter().enumerate().take(9) {
        let st = if i == detail { Style::default().fg(BLACK).bg(KEY) } else { Style::default().fg(TEXT) };
        let labors: Vec<&str> = wd.labors.iter().map(|l| l.short()).collect();
        v.push(Line::from(vec![
            Span::styled(format!(" {} {:<14}", i + 1, wd.name), st),
            Span::styled(format!("  {}", labors.join(" ")), Style::default().fg(MUTED)),
        ]));
    }
    if let Some(fl) = &app.flash {
        v.push(Line::default());
        v.push(plain(fl.clone(), Color::Rgb(120, 255, 160)));
    }

    f.render_widget(Clear, r);
    f.render_widget(
        Paragraph::new(v)
            .style(Style::default().bg(Color::Rgb(15, 15, 25)))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(KEY))
                    .title(Span::styled(" Labors ", Style::default().fg(KEY)))
                    .title_bottom(Span::styled(
                        " Arrows move · Enter toggle · a everybody/nobody · Esc back ",
                        Style::default().fg(MUTED),
                    )),
            ),
        r,
    );
}

fn draw_status(f: &mut Frame, area: Rect, g: &Game, tab: usize, scroll: usize) {
    let w = area.width.saturating_sub(6).min(100);
    let h = area.height.saturating_sub(4);
    let r = Rect::new(area.x + (area.width - w) / 2, area.y + 2, w, h);
    let lines = status_lines(g, tab);
    let visible = h.saturating_sub(4) as usize;
    let scroll = scroll.min(lines.len().saturating_sub(visible));
    let names = ["1 Overview", "2 Stocks", "3 Health", "4 Dwarves"];
    let mut tabs: Vec<Span> = Vec::new();
    for (i, n) in names.iter().enumerate() {
        let st = if i == tab { Style::default().fg(BLACK).bg(KEY) } else { Style::default().fg(TEXT) };
        tabs.push(Span::styled(format!(" {} ", n), st));
        tabs.push(Span::raw(" "));
    }
    let mut body = vec![Line::from(tabs), Line::default()];
    body.extend(lines.into_iter().skip(scroll).take(visible));
    f.render_widget(Clear, r);
    f.render_widget(
        Paragraph::new(body)
            .style(Style::default().bg(Color::Rgb(15, 15, 25)))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(KEY))
                    .title(Span::styled(" Fortress status ", Style::default().fg(KEY)))
                    .title_bottom(Span::styled(" Tab/1-4 switch · ↑↓ PgUp/PgDn scroll · Esc back ", Style::default().fg(MUTED))),
            ),
        r,
    );
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
        let id = app.game.dwarves[0].id;
        app.game.toggle_soldier(id);
        app.game.dwarves[0].body.wound(crate::body::Part::LeftArm, 15, true);
        app.mode = Mode::Unit { id, sel: 2 };
        println!("{}", dump(&mut app, 130, 50));
        app.mode = Mode::Military;
        println!("{}", dump(&mut app, 130, 45));
        let (x, y) = app.game.site;
        app.mode = Mode::WorldMap { x, y };
        println!("{}", dump(&mut app, 140, 40));
        app.mode = Mode::Legends { tab: 0, scroll: usize::MAX, from_world: false };
        println!("{}", dump(&mut app, 130, 40));
    }

    #[test]
    fn render_look_and_status() {
        let mut app = App::new(42);
        let e = app.game.embark;
        app.game.add_zone(e.offset(-8, -8, -1), e.offset(-3, -3, 1), ZoneKind::Meeting);
        app.mode = Mode::Normal;
        for _ in 0..3000 {
            app.game.step();
        }
        app.game.dwarves[1].body.wound(crate::body::Part::LeftLeg, 12, true);
        // Look at a dwarf, then at the wagon's pile of supplies.
        app.cursor = app.game.dwarves[0].pos;
        app.mode = Mode::Look { sel: 0 };
        let s = dump(&mut app, 130, 45);
        println!("{}", s);
        assert!(s.contains(&app.game.dwarves[0].name));
        let pile = app.game.items.values().filter(|it| it.carried_by.is_none()).map(|it| it.pos).next().unwrap();
        app.cursor = pile;
        app.mode = Mode::Look { sel: 0 };
        println!("{}", dump(&mut app, 130, 45));
        for tab in 0..4 {
            app.mode = Mode::Status { tab, scroll: 0 };
            let s = dump(&mut app, 130, 45);
            println!("{}", s);
            assert!(s.contains("Fortress status"));
        }
        app.mode = Mode::Zone { tool: ZoneTool::Meeting, corner: None };
        println!("{}", dump(&mut app, 130, 45));
    }

    #[test]
    fn render_labors() {
        let mut app = App::new(42);
        app.mode = Mode::Labors { row: 1, col: 0, detail: 0 };
        // Toggle a cell and a whole column through the keyboard.
        let key = |c| crossterm::event::KeyEvent::new(c, crossterm::event::KeyModifiers::NONE);
        app.on_key(key(crossterm::event::KeyCode::Right));
        app.on_key(key(crossterm::event::KeyCode::Enter));
        app.on_key(key(crossterm::event::KeyCode::Char('a')));
        let s = dump(&mut app, 130, 45);
        println!("{}", s);
        assert!(s.contains("Labors") && s.contains("Hau") && s.contains("Woodworker"));
        assert!(matches!(app.mode, Mode::Labors { col: 1, .. }));
    }
}
