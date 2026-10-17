use crate::app::{look_entries, preset_name, App, LookEntry, Mode, ZoneTool, DESIG_KEYS, LEVELS, SETUP_ROWS, TITLE_MENU};
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
use std::collections::{HashMap, HashSet};

const SIDEBAR_W: u16 = 38;

/// What a mouse click on part of the screen does, gathered as the frame is
/// drawn: press a key, pick a row in a list, or nothing (a panel that sits
/// over what's beneath).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Key(crossterm::event::KeyCode),
    Row(usize),
    Block,
}

thread_local! {
    static TARGETS: std::cell::RefCell<Vec<(Rect, Target)>> = std::cell::RefCell::new(Vec::new());
}

fn add_target(r: Rect, t: Target) {
    TARGETS.with(|v| v.borrow_mut().push((r, t)));
}

/// The topmost thing at a screen cell, if any.
pub fn target_at(col: u16, row: u16) -> Option<Target> {
    TARGETS.with(|v| v.borrow().iter().rev().find(|(r, _)| col >= r.x && row >= r.y && col < r.x + r.width && row < r.y + r.height).map(|x| x.1))
}

/// The key a `key_line` offers, if this line is one.
fn key_of(line: &Line) -> Option<crossterm::event::KeyCode> {
    use crossterm::event::KeyCode;
    let first = line.spans.first()?;
    if first.style.fg != Some(KEY) || !first.style.add_modifier.contains(Modifier::BOLD) {
        return None;
    }
    let k = first.content.trim();
    let mut chars = k.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Some(KeyCode::Char(c)),
        _ => match k {
            "Esc" => Some(KeyCode::Esc),
            "Enter" => Some(KeyCode::Enter),
            "Tab" => Some(KeyCode::Tab),
            "Space" => Some(KeyCode::Char(' ')),
            _ => None,
        },
    }
}

/// Make each key line drawn in `inner` clickable.
fn register_lines(inner: Rect, lines: &[Line], wrap: bool) {
    let width = inner.width.max(1) as usize;
    let mut y = inner.y;
    for l in lines {
        if y >= inner.y + inner.height {
            break;
        }
        let rows = if wrap { ((l.width().max(1) + width - 1) / width) as u16 } else { 1 };
        if let Some(k) = key_of(l) {
            add_target(Rect::new(inner.x, y, inner.width, rows.min(inner.y + inner.height - y)), Target::Key(k));
        }
        y += rows;
    }
}

/// A short note on what's under the mouse, beside it.
fn draw_tooltip(f: &mut Frame, app: &App) {
    let Some((col, row)) = app.hover else { return };
    let r = app.map_rect;
    if app.drag.is_some() || !app.uses_map_cursor() || col < r.x || row < r.y || col >= r.x + r.width || row >= r.y + r.height {
        return;
    }
    let g = &app.game;
    let p = Pos::new(app.view_x + (col - r.x) as i32, app.view_y + (row - r.y) as i32, app.cursor.z);
    if !g.world.in_bounds(p) || !g.world.t(p).revealed {
        return;
    }
    let lines: Vec<Line<'static>> = cursor_info(g, p).into_iter().skip(1).filter(|l| l.width() > 0).take(8).collect();
    if lines.is_empty() {
        return;
    }
    let w = (lines.iter().map(|l| l.width()).max().unwrap_or(10) as u16 + 2).min(46);
    let h = lines.len() as u16 + 2;
    let area = f.size();
    let x = if col + 2 + w < area.width { col + 2 } else { col.saturating_sub(w + 1) };
    let y = if row + 1 + h < area.height { row + 1 } else { row.saturating_sub(h) };
    let tip = Rect::new(x, y, w, h);
    f.render_widget(Clear, tip);
    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(Color::Rgb(18, 18, 30))).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(MUTED))),
        tip,
    );
}

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

/// The season's look for trees and grass under the open sky: blossom in
/// spring, autumn colours, bare branches in winter, fallen leaves.
/// `year_frac` runs 0..1 through the year from the first day of spring.
fn foliage(w: &World, p: Pos, year_frac: f32, cold: bool) -> Option<(char, Color)> {
    use crate::overworld::Biome as B;
    let t = w.t(p);
    if !t.revealed || !w.outside(p) {
        return None;
    }
    let h = hash2(p.x * 3 + 11, p.y * 5 + 7);
    // What grows here depends on the land: cacti in the desert, conifers in
    // the taiga, acacias on the savanna, dense green in the jungle, and
    // twisted, sickly growth where evil lies on the land.
    let (biome, evil) = w.biome_at(p.x, p.y).unwrap_or((B::Forest, false));
    let cold = cold || biome.cold();
    let pick = |opts: &[(char, (u8, u8, u8))]| {
        let (c, col) = opts[(h as usize / 5) % opts.len()];
        (c, Color::Rgb(col.0, col.1, col.2))
    };
    if evil && matches!(t.kind, TileKind::Tree | TileKind::Shrub) {
        return Some(pick(&[('♣', (120, 95, 125)), ('¥', (105, 85, 110)), ('♠', (95, 80, 100))]));
    }
    if t.kind == TileKind::Shrub {
        return Some(match biome {
            B::Desert | B::Badlands => pick(&[('*', (170, 150, 90)), ('"', (150, 140, 80))]),
            B::Swamp => pick(&[('"', (120, 160, 90)), ('|', (110, 140, 80))]),
            B::Jungle => pick(&[('"', (40, 190, 70)), ('♣', (60, 170, 60))]),
            B::Tundra | B::Glacier | B::Taiga => pick(&[('"', (110, 140, 120)), ('·', (130, 150, 130))]),
            B::Savanna => pick(&[('"', (180, 165, 70)), ('*', (160, 150, 70))]),
            _ => ('"', Color::Rgb(90, 190, 90)),
        });
    }
    if t.kind == TileKind::Tree {
        match biome {
            B::Desert | B::Badlands => return Some(pick(&[('Ψ', (90, 165, 95)), ('Ψ', (110, 175, 100)), ('¥', (150, 130, 90))])),
            B::Jungle => return Some(pick(&[('♠', (25, 120, 45)), ('♣', (35, 165, 55)), ('Γ', (60, 150, 60)), ('♣', (20, 140, 70))])),
            B::Savanna => return Some(pick(&[('τ', (150, 150, 60)), ('τ', (130, 140, 55)), ('♣', (120, 140, 60))])),
            B::Taiga | B::Tundra | B::Glacier => return Some(pick(&[('♠', (35, 105, 65)), ('↑', (45, 115, 75)), ('♠', (55, 125, 85))])),
            B::Swamp => return Some(pick(&[('♣', (60, 120, 90)), ('♣', (70, 110, 70)), ('♠', (50, 100, 80))])),
            _ => {}
        }
    }
    // Each tree turns a little ahead of or behind its neighbours.
    let jitter = (h % 100) as f32 / 100.0 * 0.06 - 0.03;
    let f = (year_frac + jitter).rem_euclid(1.0);
    let evergreen = if cold { h % 4 != 3 } else { h % 4 == 0 };
    match t.kind {
        TileKind::Tree if evergreen => Some(('♠', Color::Rgb(30, 120, 60))),
        TileKind::Tree => Some(match f {
            // Early spring: some trees blossom, the rest bud pale green.
            f if f < 0.14 => {
                if h % 3 == 0 {
                    ('♣', [Color::Rgb(255, 175, 205), Color::Rgb(245, 235, 240)][(h / 3 % 2) as usize])
                } else {
                    ('♣', Color::Rgb(150, 215, 110))
                }
            }
            f if f < 0.53 => ('♣', Color::Rgb(40, 170, 50)),
            // Autumn: yellow, orange, red and gold, then bare.
            f if f < 0.70 => (
                '♣',
                [Color::Rgb(235, 205, 60), Color::Rgb(235, 135, 40), Color::Rgb(205, 60, 40), Color::Rgb(205, 165, 50)][(h % 4) as usize],
            ),
            f if f < 0.97 => ('♣', Color::Rgb(125, 95, 70)),
            _ => ('♣', Color::Rgb(150, 215, 110)),
        }),
        // Leaves lie under the trees from late autumn until the snow.
        TileKind::Floor if t.mat == Mat::Grass && !t.snow && (0.62..0.85).contains(&f) && h % 3 != 0 => {
            let near = crate::geom::DIRS8.iter().any(|&(dx, dy)| w.kind(p.offset(dx, dy, 0)) == Some(TileKind::Tree));
            near.then(|| {
                (
                    [',', '\'', '`'][(h % 3) as usize],
                    [Color::Rgb(200, 120, 40), Color::Rgb(180, 70, 40), Color::Rgb(210, 175, 60)][(h / 7 % 3) as usize],
                )
            })
        }
        _ => None,
    }
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
    let magma_bg = Color::Rgb(120, 20, 0);
    let magma_fg = Color::Rgb(255, 150, 40);
    if t.magma > 0 {
        return (water_glyph(t.magma), magma_fg, magma_bg);
    }
    if t.frozen && t.water > 0 {
        return ('≈', Color::Rgb(220, 240, 255), Color::Rgb(70, 100, 130));
    }
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
                if tq.magma > 0 {
                    return ('≈', dim(magma_fg, f.max(0.45)), BLACK);
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
        TileKind::Floor if t.built_from.is_some() => ('+', dim(mc, 0.85), BLACK),
        TileKind::Wall if t.smooth >= crate::game::ENGRAVED => ('▒', mc, Color::Rgb(25, 22, 30)),
        TileKind::Wall if t.smooth == 1 => ('█', mc, BLACK),
        TileKind::Floor if t.smooth >= crate::game::ENGRAVED => ('÷', mc, BLACK),
        TileKind::Floor if t.smooth == 1 => ('·', mc, BLACK),
        TileKind::Wall if t.built_from.is_some() => {
            if t.built_from == Some(ItemKind::Block) {
                ('▓', mc, BLACK)
            } else {
                ('█', mc, BLACK)
            }
        }
        TileKind::Fortification => ('#', mc, BLACK),
        TileKind::Floor => {
            use crate::overworld::Biome as B;
            let h = hash2(p.x, p.y);
            let shade = 0.75 + (h % 7) as f32 * 0.05;
            if t.mat == Mat::Grass {
                // Grass by its land: tall and golden on the savanna, short and
                // pale on the tundra, dark under the pines, sickly where evil lies.
                let (biome, evil) = w.biome_at(p.x, p.y).unwrap_or((B::Grassland, false));
                let (glyphs, (r, gg, b)): (&[char], (u8, u8, u8)) = match biome {
                    B::Savanna => (&['"', '"', ',', '\'', '"'], (190, 172, 80)),
                    B::Tundra => (&['.', ',', '`', '.', '\''], (150, 166, 142)),
                    B::Taiga => (&[',', '\'', '.', ',', '`'], (72, 132, 88)),
                    B::Jungle => (&['"', ',', '"', '\'', '"'], (40, 168, 72)),
                    B::Swamp => (&['"', ',', '"', '.', '\''], (88, 138, 106)),
                    B::Mountain | B::Hills => (&[',', '.', '`', '\'', ','], (112, 152, 90)),
                    B::Desert | B::Badlands => (&['.', ',', '`', '.', '\''], (172, 160, 92)),
                    _ => (&[',', '.', '`', '\'', ','], (70, 160, 60)),
                };
                let (r, gg, b) = if evil { (126, 104, 136) } else { (r, gg, b) };
                (glyphs[(h % 5) as usize], dim(Color::Rgb(r, gg, b), shade), BLACK)
            } else if w.outside(p) && matches!(t.mat, Mat::Sand | Mat::Clay) {
                // Dunes and dry earth under the sky.
                let g = if t.mat == Mat::Sand { ['.', '~', '.', ',', '~', '.'][(h % 6) as usize] } else { ['.', ',', '.', '`', '.', '_'][(h % 6) as usize] };
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
    if t.aquifer && t.kind == TileKind::Wall {
        return (g, fg, Color::Rgb(10, 30, 70));
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
        Mode::Designate { corner, .. }
        | Mode::Stockpile { corner, .. }
        | Mode::Zone { corner, .. }
        | Mode::Burrows { corner, .. }
        | Mode::Traffic { corner, .. }
        | Mode::Construct { corner, .. } => *corner,
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
    // Keep the cursor inside the view with a margin (unless the view was panned).
    let margin = 4;
    if app.free_view && (app.cursor.x, app.cursor.y) != app.pan_from {
        app.free_view = false;
    }
    let follow = !app.free_view;
    if vw >= ww {
        app.view_x = -(vw - ww) / 2;
    } else {
        if follow && app.cursor.x < app.view_x + margin {
            app.view_x = app.cursor.x - margin;
        }
        if follow && app.cursor.x > app.view_x + vw - 1 - margin {
            app.view_x = app.cursor.x - vw + 1 + margin;
        }
        app.view_x = app.view_x.clamp(0, ww - vw);
    }
    if vh >= wh {
        app.view_y = -(vh - wh) / 2;
    } else {
        if follow && app.cursor.y < app.view_y + margin {
            app.view_y = app.cursor.y - margin;
        }
        if follow && app.cursor.y > app.view_y + vh - 1 - margin {
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
    let ghosts: HashSet<(i32, i32)> = g.dwarves.iter().filter(|d| d.ghost && d.pos.z == z).map(|d| (d.pos.x, d.pos.y)).collect();
    let mut beasts: HashMap<(i32, i32), (char, Color, bool)> = HashMap::new();
    for c in g.creatures.values().filter(|c| c.alive && !c.sneaking && c.pos.z == z && g.world.t(c.pos).revealed) {
        let info = c.species.info();
        let (glyph, color) = if c.breathes_fire {
            ('D', (255, 90, 40))
        } else if c.species == crate::creature::Species::Undead && c.general.is_some() {
            ('N', (190, 110, 240))
        } else {
            (info.glyph, info.color)
        };
        beasts.insert((c.pos.x, c.pos.y), (glyph, rgb(color), c.hostile()));
    }
    let mut items: HashMap<(i32, i32), (char, Color)> = HashMap::new();
    for it in g.items.values().filter(|it| it.pos.z == z && it.carried_by.is_none() && it.container.is_none()) {
        // Containers win the tile so stockpiles read as rows of barrels and bins.
        let key = (it.pos.x, it.pos.y);
        if it.kind.is_container() || !items.contains_key(&key) {
            items.insert(key, it.glyph());
        }
    }
    let planned: HashMap<(i32, i32), crate::game::ConstructKind> =
        g.constructions.values().filter(|c| c.pos.z == z).map(|c| ((c.pos.x, c.pos.y), c.kind)).collect();
    let sel = selection_box(app);
    let placing = match app.mode {
        Mode::Place(kind) => {
            let s = kind.size() / 2;
            let tl = app.cursor.offset(-s, -s, 0);
            Some((tl, kind.size(), g.can_place(kind, tl)))
        }
        _ => None,
    };

    let year_frac = (g.tick % crate::game::YEAR) as f32 / crate::game::YEAR as f32;
    let cold = g.biome.cold();
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
            if (t.kind == TileKind::Tree || t.kind == TileKind::Shrub || t.mat == Mat::Grass) && t.water == 0 {
                if let Some((c, col)) = foliage(&g.world, p, year_frac, cold) {
                    (ch, fg) = (c, col);
                }
            }

            if t.revealed && t.zone != 0 {
                match g.zones.get(&t.zone).map(|z| z.kind) {
                    Some(ZoneKind::Farm) => {
                        bg = Color::Rgb(55, 35, 15);
                        let cc = t.crop.unwrap_or(Mat::Stoutcap).color();
                        (ch, fg) = if t.farm == 0 {
                            ('·', Color::Rgb(150, 110, 70))
                        } else if t.farm >= RIPE {
                            ('♠', cc)
                        } else if t.farm > RIPE / 2 {
                            ('"', dim(cc, 0.75))
                        } else {
                            (',', dim(cc, 0.55))
                        };
                    }
                    Some(ZoneKind::Stockpile(_)) => {
                        bg = Color::Rgb(30, 30, 50);
                        if ch == '.' || ch == ',' || ch == '`' || ch == '\'' {
                            ch = '=';
                            fg = Color::Rgb(70, 70, 100);
                        }
                    }
                    Some(k) => {
                        let (zbg, zfg) = match k {
                            ZoneKind::Meeting => ((20, 50, 40), (70, 130, 110)),
                            ZoneKind::Bedroom => ((45, 30, 55), (120, 90, 150)),
                            ZoneKind::Dormitory => ((35, 30, 50), (100, 90, 130)),
                            ZoneKind::Dining => ((55, 40, 20), (150, 120, 70)),
                            ZoneKind::Office => ((25, 40, 60), (90, 120, 170)),
                            _ => ((45, 40, 20), (120, 110, 60)),
                        };
                        bg = rgb(zbg);
                        if ch == '.' || ch == ',' || ch == '`' || ch == '\'' {
                            ch = '·';
                            fg = rgb(zfg);
                        }
                    }
                    None => {}
                }
            }
            if t.snow && t.revealed && t.zone == 0 {
                fg = Color::Rgb(235, 240, 250);
                if ch == '.' || ch == ',' || ch == '`' || ch == '\'' || ch == '·' {
                    ch = '·';
                }
            }
            // Falling rain or snow, flickering over open ground.
            if g.weather != crate::game::Weather::Clear && t.revealed && g.world.outside(p) && !t.kind.solid() {
                let h = hash2(wx * 7 + (g.tick / 3) as i32 % 97, wy * 13 + (g.tick / 5) as i32 % 89);
                if h % 23 == 0 {
                    use crate::game::Weather as Wx;
                    (ch, fg) = match g.weather {
                        Wx::Snow | Wx::Blizzard => ('*', Color::Rgb(240, 240, 255)),
                        Wx::BloodRain => ('\'', Color::Rgb(220, 30, 30)),
                        Wx::Storm => ('/', Color::Rgb(140, 170, 255)),
                        _ => ('\'', Color::Rgb(110, 150, 255)),
                    };
                } else if g.weather == crate::game::Weather::Blizzard && h % 5 == 0 {
                    (ch, fg) = ('·', Color::Rgb(220, 230, 255));
                } else if g.weather == crate::game::Weather::Storm && (g.tick / 3) % 97 == 0 {
                    // A flash of lightning lights up the land.
                    bg = Color::Rgb(70, 70, 110);
                }
            }
            if g.flames.iter().any(|&(q, at)| q == p && at + 8 > g.tick) {
                ch = ['*', '☼', '≈'][(hash2(wx, wy + g.tick as i32) % 3) as usize];
                fg = Color::Rgb(255, 230, 120);
                bg = Color::Rgb(200, 60, 0);
            }
            if t.miasma > 0 && t.revealed && t.fire == 0 {
                let k = t.miasma as u32;
                bg = Color::Rgb((25 + 14 * k) as u8, (8 + 3 * k) as u8, (30 + 16 * k) as u8);
                if ch == '.' || ch == '·' || ch == ' ' {
                    ch = '░';
                    fg = Color::Rgb(150, 90, 170);
                }
            }
            if t.fire > 0 && t.revealed {
                let flick = hash2(wx + (g.tick / 4) as i32 % 50, wy) % 3;
                ch = if flick == 0 { '▲' } else { '^' };
                fg = [Color::Rgb(255, 220, 80), Color::Rgb(255, 140, 30), Color::Rgb(240, 60, 20)][flick as usize];
                bg = Color::Rgb(90, 20, 0);
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
            if ghosts.contains(&(wx, wy)) {
                ch = '☺';
                fg = Color::Rgb(170, 200, 255);
                bg = Color::Rgb(20, 30, 70);
            }
            if let Some(ds) = units.get(&(wx, wy)) {
                let d = ds[blink % ds.len()];
                ch = if d.soldier { '☻' } else { '☺' };
                fg = if d.asleep || d.body.unconscious() { dim(rgb(d.color), 0.5) } else { rgb(d.color) };
                if d.body.bleeding > 0 {
                    bg = Color::Rgb(70, 0, 0);
                }
            }
            match app.mode {
                Mode::Burrows { sel, .. } => {
                    if let Some(b) = g.burrows.get(sel) {
                        if t.burrows & b.mask() != 0 {
                            bg = Color::Rgb(70, 40, 100);
                        }
                    }
                }
                Mode::Traffic { .. } => match t.traffic {
                    1 => bg = Color::Rgb(20, 70, 30),
                    2 => bg = Color::Rgb(80, 70, 10),
                    3 => bg = Color::Rgb(90, 20, 20),
                    _ => {}
                },
                _ => {}
            }
            if let Some(k) = planned.get(&(wx, wy)) {
                bg = Color::Rgb(25, 40, 90);
                if units.get(&(wx, wy)).is_none() {
                    ch = k.glyph();
                    fg = Color::Rgb(150, 180, 255);
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
            if wx == app.cursor.x && wy == app.cursor.y && !app.free_view {
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

/// A soldier's kit as a row of the squads screen's grid: W H M S, green
/// worn, yellow waiting in the stores, red missing.
fn kit_line<'a>(g: &Game, id: u32) -> Line<'a> {
    use crate::game::readiness::{KitPart, Slot3};
    let k = g.kit(id);
    let mut spans = vec![Span::raw("     ")];
    for p in KitPart::ALL {
        let c = match k.get(p) {
            Slot3::Worn => Color::Rgb(120, 220, 120),
            Slot3::InStock => Color::Rgb(230, 210, 90),
            Slot3::None => Color::Rgb(220, 90, 80),
        };
        spans.push(Span::styled(format!("{} ", p.letter()), Style::default().fg(c)));
    }
    let (word, c) = if g.kit_ready(id) { ("ready", Color::Rgb(120, 220, 120)) } else { ("not ready", MUTED) };
    spans.push(Span::styled(format!("· {}", word), Style::default().fg(c)));
    Line::from(spans)
}

fn task_desc(g: &Game, d: &Dwarf) -> String {
    if d.tantrum > 0 {
        return "Tantrum!".into();
    }
    let iname = |id: u32| g.items.get(&id).map_or("something".to_string(), |it| it.name());
    let who = |id: u32| g.dwarves.iter().find(|o| o.id == id).map_or("someone".to_string(), |o| o.name.clone());
    if g.tick < d.jailed_until && matches!(d.task, Task::Idle | Task::Wander { .. }) {
        return format!("In jail ({} days left)", (d.jailed_until - g.tick) / crate::game::DAY + 1);
    }
    match &d.task {
        Task::Wander { .. } | Task::Idle if g.tick < d.break_until => match g.zone_kind_at(d.pos) {
            Some(ZoneKind::Temple) => "Praying".into(),
            Some(ZoneKind::Tavern) => "Making merry".into(),
            _ => "On break".into(),
        },
        Task::Idle => "No job".into(),
        Task::Wander { .. } => "Wandering".into(),
        Task::Dig { target, .. } => {
            let ds = g.world.t(*target).desig;
            match ds {
                Desig::Chop => "Felling tree".into(),
                Desig::Channel => "Digging channel".into(),
                Desig::StairUp | Desig::StairDown | Desig::StairUD => "Carving stairway".into(),
                Desig::Ramp => "Carving ramp".into(),
                Desig::Smooth => "Smoothing stone".into(),
                Desig::Engrave => "Engraving".into(),
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
        Task::Gather { .. } => "Gathering plants".into(),
        Task::Fish { .. } => "Fishing".into(),
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
        Task::PullLever { .. } => "Pulling a lever".into(),
        Task::Slaughter { creature } => format!("Slaughtering a {}", g.creatures.get(creature).map_or("beast", |c| c.name())),
        Task::Punish { culprit } => format!("Bringing {} to justice", g.dwarves.iter().find(|d| d.id == *culprit).map_or("someone", |d| d.name.as_str())),
        Task::Bury { item, .. } => format!("Burying the {}", iname(*item)),
        Task::Treat { patient, what, .. } => {
            let who = g.dwarves.iter().find(|p| p.id == *patient).map_or("a patient".to_string(), |p| p.name.clone());
            format!("Treating {}: {}", who, what.name())
        }
        Task::BuildTile { con, .. } => match g.constructions.get(con).map(|c| c.kind) {
            Some(crate::game::ConstructKind::Remove) => "Removing construction".into(),
            Some(k) => format!("Constructing {}", k.name().to_lowercase()),
            None => "Constructing".into(),
        },
        Task::FirstAid { patient, .. } => format!("Giving first aid to {}", who(*patient)),
        Task::Rescue { patient, .. } => format!("Carrying {} to safety", who(*patient)),
        Task::Press => "Pressing a wound".into(),
        Task::Catatonic => "Catatonic".into(),
        Task::Berserk => "Berserk!".into(),
        Task::Coax { patient } => format!("Talking {} round", who(*patient)),
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
        k if t.built_from.is_some() => format!("constructed {} {}", t.mat.name(), k.name()),
        k => format!("{} {}", t.mat.name(), k.name()),
    };
    v.push(plain(desc, t.mat.color()));
    if t.water > 0 {
        let what = if t.frozen { "Ice" } else { "Water" };
        v.push(plain(format!("{} {}/7", what, t.water), Color::Rgb(90, 160, 255)));
    }
    if t.magma > 0 {
        v.push(plain(format!("Magma {}/7", t.magma), Color::Rgb(255, 150, 40)));
    }
    if let Some((q, year, text)) = g.engraving_at(p) {
        let qual = crate::items::quality_prefix(q).trim_end();
        let qual = if qual.is_empty() { "plain" } else { qual };
        v.push(plain(format!("Engraved ({}), year {}: {}", qual, year, text), Color::Rgb(210, 190, 255)));
    } else if t.smooth == 1 {
        v.push(plain("Smoothed".into(), MUTED));
    }
    if t.aquifer {
        v.push(plain("Aquifer: water seeps from this rock".into(), Color::Rgb(90, 160, 255)));
    }
    if t.fire > 0 {
        v.push(plain("On fire!".into(), Color::Rgb(255, 120, 40)));
    }
    if t.miasma > 0 {
        v.push(plain(format!("Miasma ({}/7): the stench of rot", t.miasma), Color::Rgb(190, 120, 220)));
    }
    if t.desig != Desig::None {
        v.push(plain(format!("Designated: {}", t.desig.name()), KEY));
    }
    if let Some(c) = g.construction_at(p) {
        v.push(plain(format!("Planned: {} ({})", c.kind.name(), c.mat.name()), Color::Rgb(150, 180, 255)));
    }
    let in_burrows: Vec<&str> = g.burrows.iter().filter(|b| t.burrows & b.mask() != 0).map(|b| b.name.as_str()).collect();
    if !in_burrows.is_empty() || t.traffic != 0 {
        let mut s = Vec::new();
        if !in_burrows.is_empty() {
            s.push(format!("In {}", in_burrows.join(", ")));
        }
        if t.traffic != 0 {
            s.push(format!("{} traffic", crate::game::TRAFFIC_NAMES[t.traffic as usize]));
        }
        v.push(plain(s.join(" · "), Color::Rgb(190, 150, 230)));
    }
    if let Some(z) = g.zones.get(&t.zone) {
        match z.kind {
            ZoneKind::Farm => {
                let plant = t.crop.unwrap_or(Mat::Stoutcap).crop().map_or("stoutcap", |c| c.plant);
                let s = if t.farm == 0 {
                    "fallow".to_string()
                } else if t.farm >= RIPE {
                    format!("{} ripe!", plant)
                } else {
                    format!("{} growing {}%", plant, t.farm as u32 * 100 / RIPE as u32)
                };
                v.push(plain(format!("Farm plot ({})", s), Color::Rgb(200, 150, 90)));
                let grows = z.crop.and_then(|m| m.crop()).map_or("whatever suits the season".to_string(), |c| format!("{} ({})", c.plant, c.where_when()));
                v.push(plain(format!("Grows {} (k, c: change)", grows), MUTED));
            }
            ZoneKind::Gathering => {
                let s = if t.kind != TileKind::Shrub {
                    "nothing to pick".to_string()
                } else if t.farm > 0 {
                    "picked this season".to_string()
                } else {
                    g.wild_crop(p).and_then(|m| m.crop()).map_or("bare this season".to_string(), |c| format!("bearing {}", c.plant))
                };
                v.push(plain(format!("Gathering area ({})", s), Color::Rgb(120, 190, 100)));
            }
            ZoneKind::Stockpile(f) => {
                let custom = if z.deny.is_empty() && z.deny_mats.is_empty() { "" } else { ", customized" };
                v.push(plain(format!("Stockpile: {}{} (k, s: settings)", f.name(), custom), Color::Rgb(150, 150, 220)));
            }
            k => {
                let mut s = k.name();
                if k.ownable() {
                    let who = z.owner.and_then(|o| g.dwarves.iter().find(|d| d.id == o)).map_or("unassigned".to_string(), |d| d.name.clone());
                    s.push_str(&format!(" ({})", who));
                }
                v.push(plain(s, Color::Rgb(110, 200, 170)));
                if k.holds_furniture() && k != ZoneKind::Meeting {
                    let tiles = z.tiles.iter().copied().collect();
                    v.push(plain(format!("Room value ☼{}", g.area_value(&tiles)), MUTED));
                }
            }
        }
    }
    if let Some(b) = g.buildings.get(&t.building) {
        let st = if b.built { "" } else { " (awaiting construction)" };
        v.push(plain(format!("{}{}", b.name(), st), b.kind.color()));
    }
    for c in g.creatures.values().filter(|c| c.alive && !c.sneaking && c.pos == p) {
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
    for d in g.dwarves.iter().filter(|d| d.ghost && d.pos == p) {
        v.push(plain(format!("☺ The ghost of {} (bury them, or engrave a slab)", d.name), Color::Rgb(170, 200, 255)));
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
    let food = c(ItemKind::Plant) + c(ItemKind::Meat) + c(ItemKind::Meal);
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
        Mode::Normal | Mode::Help | Mode::ConfirmQuit | Mode::Chronicle { .. } | Mode::Autoplay { .. } | Mode::Brain { .. } | Mode::Reports { .. } | Mode::GrowMap { .. } | Mode::GameOver => {
            title = "Main".to_string();
            v.extend(stocks(g));
            v.push(Line::default());
            v.push(header("Commands"));
            v.push(key_line("d", "Designate (dig, chop...)", false));
            v.push(key_line("p", "Stockpiles", false));
            v.push(key_line("i", "Zones: rooms, farms, meeting, dump", false));
            v.push(key_line("b", "Build", false));
            v.push(key_line("q", "Query building", false));
            v.push(key_line("k", "Look around", false));
            v.push(key_line("z", "Fortress status", false));
            v.push(key_line("u", "Unit list", false));
            v.push(key_line("j", "Work orders (keep in stock)", false));
            v.push(key_line("y", "Labors & work details", false));
            v.push(key_line("w", "Burrows", false));
            v.push(key_line("o", "Traffic", false));
            v.push(key_line("m", "Military", false));
            v.push(key_line("n", "Nobles, mandates & rooms", false));
            v.push(key_line("c", "Chronicle of the years", false));
            v.push(key_line("r", "Reports: past announcements", false));
            v.push(key_line("M", "Grow the map", false));
            v.push(key_line("A", "Autoplay: let the overseer play", g.overseer.is_some()));
            if g.overseer.is_some() {
                v.push(key_line("B", "The overseer's brain", false));
            }
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
                v.push(key_line(&t.key().to_string(), &t.name(), t == *tool));
            }
            v.push(Line::default());
            for line in tool.help() {
                v.push(plain(line.to_string(), TEXT));
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
            v.push(plain("Pick from the list.".into(), TEXT));
            v.push(plain("Esc: back".into(), MUTED));
        }
        Mode::Construct { kind, mat, corner } => {
            title = "Constructions".to_string();
            for ck in crate::game::ConstructKind::ALL {
                v.push(key_line(&ck.key().to_string(), ck.name(), *kind == Some(ck)));
            }
            v.push(key_line("c", "Cancel planned constructions", kind.is_none()));
            v.push(Line::default());
            v.push(key_line("m", &format!("Material: {}", mat.name()), false));
            v.push(plain("Built from rough stone, blocks or logs.".into(), MUTED));
            v.push(plain("Floors bridge open space beside".into(), MUTED));
            v.push(plain("something solid. Fortifications stop".into(), MUTED));
            v.push(plain("walkers but let water through.".into(), MUTED));
            v.push(Line::default());
            let n = g.constructions.len();
            let busy = g.constructions.values().filter(|c| c.worker.is_some()).count();
            v.push(plain(format!("{} planned, {} being built", n, busy), TEXT));
            v.push(plain(
                if corner.is_some() { "Enter: set second corner".into() } else { "Enter: set first corner".into() },
                KEY,
            ));
            v.push(plain("Esc: back".into(), MUTED));
            v.push(Line::default());
            v.extend(cursor_info(g, app.cursor));
        }
        Mode::Place(kind) => {
            title = "Place".to_string();
            v.push(plain(format!("Placing: {}", kind.name()), KEY));
            v.push(plain(format!("Needs {}", kind.needs_desc()), TEXT));
            if *kind == BuildingKind::Well {
                v.push(plain("Place over a channel (open space)".into(), MUTED));
                v.push(plain("with water up to 10 levels below.".into(), MUTED));
            }
            if kind.uses_magma() {
                v.push(plain("The middle tile must sit over magma".into(), MUTED));
                v.push(plain("(open or floored); needs no fuel.".into(), MUTED));
            }
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
                    let total = b.kind.parts();
                    let progress = if total > 1 { format!(", part {} of {}", b.parts + 1, total) } else { String::new() };
                    v.push(plain(format!("Awaiting {}{}", b.kind.part_desc(b.parts), progress), KEY));
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
                            v.push(key_line(&r.key().to_string(), &format!("{} ({})", r.name(), r.input_desc_at(b.kind)), false));
                        }
                        v.push(plain("Esc: back".into(), MUTED));
                    } else {
                        v.push(key_line("a", "Add order", false));
                        v.push(key_line("r", "Toggle repeat on last", false));
                        v.push(key_line("x", "Remove last order", false));
                        let who: Vec<&str> =
                            b.workers.iter().filter_map(|id| g.dwarves.iter().find(|d| d.id == *id)).map(|d| d.name.as_str()).collect();
                        v.push(Line::default());
                        v.push(plain(
                            if who.is_empty() { "Workers: anyone with the labor".into() } else { format!("Workers: {}", who.join(", ")) },
                            MUTED,
                        ));
                        v.push(key_line("w", "Choose who may work here", false));
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
                if b.kind.is_trap() && b.built {
                    let what = b.load.map_or("nothing".to_string(), |(k, m, _)| {
                        crate::items::Item { id: 0, kind: k, mat: m, pos: b.pos, carried_by: None, claimed_by: None, label: None, container: None, quality: 0, to_depot: false, dump: false, wear: 0, forbidden: false }.name()
                    });
                    v.push(plain(format!("Armed with {}", what), TEXT));
                    v.push(plain("Fires on any creature but merchants;".into(), MUTED));
                    v.push(plain("dwarves never set off their own traps.".into(), MUTED));
                }
                if b.kind == BuildingKind::Well {
                    if g.well_has_water(b.pos) {
                        v.push(plain("Water below: clean drinking water".into(), Color::Rgb(110, 170, 255)));
                    } else {
                        v.push(plain("Dry: no water within 10 levels below".into(), Color::Rgb(230, 200, 60)));
                    }
                    v.push(plain("Doctors bring well water to patients in bed.".into(), MUTED));
                }
                if b.kind == BuildingKind::Floodgate && b.built {
                    v.push(plain(if b.locked { "Shut: holds back water and walkers".into() } else { "Open: water and walkers pass".into() }, TEXT));
                    v.push(plain("Link a lever to it to open or shut it.".into(), MUTED));
                }
                if b.kind == BuildingKind::Bridge && b.built {
                    v.push(plain(if b.raised { "Raised: nobody can pass".into() } else { "Lowered: open to all".into() }, TEXT));
                    v.push(plain("Link a lever to it to raise it.".into(), MUTED));
                }
                if b.kind == BuildingKind::Lever && b.built {
                    v.push(header("Linked to"));
                    if b.links.is_empty() {
                        v.push(plain("(nothing)".into(), MUTED));
                    }
                    for t in &b.links {
                        if let Some(tb) = g.buildings.get(t) {
                            let state = match tb.kind {
                                BuildingKind::Bridge => if tb.raised { " (raised)" } else { " (lowered)" },
                                BuildingKind::Door => if tb.locked { " (locked)" } else { " (open)" },
                                BuildingKind::Floodgate => if tb.locked { " (shut)" } else { " (open)" },
                                _ => "",
                            };
                            v.push(plain(format!("  {} at ({}, {}){}", tb.name(), tb.pos.x, tb.pos.y, state), TEXT));
                        }
                    }
                    v.push(key_line("k", "Link to a door, bridge or floodgate (uses a mechanism)", false));
                    v.push(key_line("p", if b.pull { "Pull requested..." } else { "Pull the lever" }, b.pull));
                }
                v.push(key_line("X", "Remove building", false));
                v.push(plain("Esc: back".into(), MUTED));
            }
        }
        Mode::Profile { bld, sel } => {
            title = "Workshop profile".to_string();
            if let Some(b) = g.buildings.get(bld) {
                v.push(plain(b.name(), b.kind.color()));
                let labors: Vec<crate::dwarf::Labor> = b.kind.recipes().iter().map(|r| r.labor()).fold(Vec::new(), |mut acc, l| {
                    if !acc.contains(&l) {
                        acc.push(l);
                    }
                    acc
                });
                v.push(plain(if b.workers.is_empty() { "Anyone with the labor may work here.".into() } else { "Only those ticked may work here.".into() }, TEXT));
                for (i, d) in g.dwarves.iter().filter(|d| d.alive && !d.is_child()).enumerate() {
                    let style = if i == *sel { Style::default().bg(Color::Rgb(50, 60, 120)) } else { Style::default() };
                    let on = b.workers.contains(&d.id);
                    let skill = labors.iter().map(|&l| d.skill(l)).max().unwrap_or(0);
                    let able = labors.iter().any(|&l| d.has(l));
                    v.push(Line::from(vec![
                        Span::styled(if on { "[x] " } else { "[ ] " }, style.fg(KEY)),
                        Span::styled(format!("{:<20}", d.name), style.fg(if able { TEXT } else { MUTED })),
                        Span::styled(format!(" skill {}", skill), style.fg(MUTED)),
                    ]));
                }
                v.push(Line::default());
                v.push(plain("Grey: lacks the labor (set it with y).".into(), MUTED));
                v.push(plain("Enter toggle · a anyone · Esc back".into(), MUTED));
            }
        }
        Mode::PileSettings { zone, sel } => {
            title = "Stockpile".to_string();
            let rows = g.pile_rows(*zone);
            if let Some(z) = g.zones.get(zone) {
                v.push(plain(z.kind.name(), Color::Rgb(150, 150, 220)));
            }
            v.push(plain("What this stockpile takes:".into(), TEXT));
            let first = sel.saturating_sub(14);
            for (i, (label, on, t)) in rows.iter().enumerate().skip(first).take(30) {
                let style = if i == *sel { Style::default().bg(Color::Rgb(50, 60, 120)) } else { Style::default() };
                let mark = if *on { "[x] " } else { "[ ] " };
                let col = match t {
                    crate::game::PileToggle::Mat(m) if *on => m.color(),
                    _ if *on => TEXT,
                    _ => MUTED,
                };
                v.push(Line::from(vec![Span::styled(mark, style.fg(KEY)), Span::styled(label.clone(), style.fg(col))]));
            }
            if rows.len() > first + 30 {
                v.push(plain(format!("  ...{} more", rows.len() - first - 30), MUTED));
            }
            v.push(Line::default());
            v.push(plain("Enter toggle · a allow all · Esc back".into(), MUTED));
        }
        Mode::Orders { sel, pick } => {
            title = "Work orders".to_string();
            if let Some(p) = pick {
                let recipes = crate::game::all_recipes();
                v.push(plain("Keep in stock — pick a job:".into(), TEXT));
                let first = p.saturating_sub(12);
                for (i, r) in recipes.iter().enumerate().skip(first).take(26) {
                    let style = if i == *p { Style::default().bg(Color::Rgb(50, 60, 120)) } else { Style::default() };
                    let shop = BuildingKind::ALL.iter().find(|b| b.recipes().contains(r)).map_or("", |b| b.name());
                    v.push(Line::from(vec![
                        Span::styled(format!("{:<26.26}", r.name()), style.fg(TEXT)),
                        Span::styled(format!(" {:.14}", shop), style.fg(MUTED)),
                    ]));
                }
                v.push(Line::default());
                v.push(plain("Up/Down, PgUp/PgDn · Enter add · Esc back".into(), MUTED));
            } else {
                match (g.holder(crate::game::Position::Manager), g.holder(crate::game::Position::Bookkeeper)) {
                    (None, _) => v.push(plain("No manager: orders wait (n to appoint).".into(), Color::Rgb(240, 130, 100))),
                    (Some(m), None) => v.push(plain(format!("Manager {}; checked daily.", m.name), MUTED)),
                    (Some(m), Some(_)) => v.push(plain(format!("Manager {}; bookkeeper counts often.", m.name), MUTED)),
                }
                if g.work_orders.is_empty() {
                    v.push(plain("No standing orders.".into(), MUTED));
                }
                for (i, o) in g.work_orders.iter().enumerate() {
                    let (kind, _) = o.recipe.product();
                    let have = g.stock_of(kind);
                    let queued = g.queued(o.recipe);
                    let style = if i == *sel { Style::default().bg(Color::Rgb(50, 60, 120)) } else { Style::default() };
                    let col = if have >= o.keep as usize { Color::Rgb(150, 220, 150) } else { Color::Rgb(230, 200, 90) };
                    v.push(Line::from(vec![
                        Span::styled(format!("{:<24.24}", o.recipe.name()), style.fg(TEXT)),
                        Span::styled(format!(" {:>3}/{:<3}", have, o.keep), style.fg(col)),
                        Span::styled(if queued > 0 { format!(" +{} queued", queued) } else { String::new() }, style.fg(MUTED)),
                    ]));
                }
                v.push(Line::default());
                v.push(plain("Each order keeps that many of its product".into(), MUTED));
                v.push(plain("in stock, queuing jobs at whichever".into(), MUTED));
                v.push(plain("workshop can do them.".into(), MUTED));
                v.push(Line::default());
                v.push(key_line("n", "New order", false));
                v.push(key_line("←/→", "Fewer/more (Shift ×10)", false));
                v.push(key_line("x", "Delete order", false));
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
        Mode::KillTarget { squad } => {
            title = "Kill order".to_string();
            v.push(plain(format!("Who should {} kill?", g.squads.get(*squad).map_or("the squad", |s| s.name.as_str())), TEXT));
            v.push(plain("Move onto a creature, Enter.".into(), TEXT));
            v.push(plain("Esc: back".into(), MUTED));
            v.push(Line::default());
            v.extend(cursor_info(g, app.cursor));
        }
        Mode::SquadMembers { squad, sel } => {
            let s = g.squads.get(*squad);
            title = s.map_or("Squad".into(), |s| s.name.clone());
            v.push(plain("Who serves in this squad:".into(), TEXT));
            for (i, d) in g.dwarves.iter().filter(|d| d.alive).enumerate() {
                let on = s.map_or(false, |s| s.members.contains(&d.id));
                let other = if !on { g.squad_of(d.id).map(|o| format!(" ({})", o.name)) } else { None };
                let style = if i == *sel { Style::default().bg(Color::Rgb(50, 60, 120)) } else { Style::default() };
                v.push(Line::from(vec![
                    Span::styled(if on { " [x] " } else { " [ ] " }, style.fg(if on { Color::Rgb(90, 220, 90) } else { MUTED })),
                    Span::styled(format!("{:<20}", d.name), style.fg(TEXT)),
                    Span::styled(format!("{} fighter{}", skill_rank(d.fight_xp), other.unwrap_or_default()), style.fg(MUTED)),
                ]));
            }
            v.push(Line::default());
            v.push(plain("Enter: draft into / discharge from".into(), MUTED));
            v.push(plain("this squad · Esc back".into(), MUTED));
        }
        Mode::LinkLever { .. } => {
            title = "Link lever".to_string();
            v.push(plain("Move the cursor onto a door, drawbridge".into(), TEXT));
            v.push(plain("or floodgate and press Enter.".into(), TEXT));
            v.push(plain("Esc: back".into(), MUTED));
            v.push(Line::default());
            v.extend(cursor_info(g, app.cursor));
        }
        Mode::Burrows { sel, corner, erase } => {
            title = "Burrows".to_string();
            if g.burrows.is_empty() {
                v.push(plain("No burrows yet.".into(), MUTED));
            }
            for (i, b) in g.burrows.iter().enumerate() {
                let alert = if g.alert_burrow == Some(b.bit) { " [alert]" } else { "" };
                v.push(key_line(
                    &format!("{}", i + 1),
                    &format!("{}: {} tiles, {} dwarves{}", b.name, b.tiles, b.members.len(), alert),
                    i == *sel,
                ));
            }
            v.push(Line::default());
            v.push(key_line("n", "New burrow", false));
            v.push(key_line("[ ]", "Select burrow", false));
            v.push(key_line("e", if *erase { "Mode: ERASE tiles" } else { "Mode: add tiles" }, *erase));
            v.push(key_line("u", "Who must stay inside", false));
            v.push(key_line("A", "Use as civilian alert burrow", false));
            v.push(key_line("X", "Delete burrow", false));
            v.push(Line::default());
            v.push(plain("Members only take jobs inside. When".into(), MUTED));
            v.push(plain("the civilian alert is raised (m, c)".into(), MUTED));
            v.push(plain("every civilian goes to the alert one.".into(), MUTED));
            v.push(plain(
                if corner.is_some() { "Enter: set second corner".into() } else { "Enter: set first corner".into() },
                KEY,
            ));
            v.push(Line::default());
            v.extend(cursor_info(g, app.cursor));
        }
        Mode::BurrowMembers { burrow, sel } => {
            let b = g.burrows.get(*burrow);
            title = b.map_or("Burrow".into(), |b| b.name.clone());
            v.push(plain("Who must stay inside:".into(), TEXT));
            for (i, d) in g.dwarves.iter().filter(|d| d.alive).enumerate() {
                let on = b.map_or(false, |b| b.members.contains(&d.id));
                let style = if i == *sel { Style::default().bg(Color::Rgb(50, 60, 120)) } else { Style::default() };
                v.push(Line::from(vec![
                    Span::styled(if on { " [x] " } else { " [ ] " }, style.fg(if on { Color::Rgb(90, 220, 90) } else { MUTED })),
                    Span::styled(format!("{:<20}", d.name), style.fg(TEXT)),
                    Span::styled(d.profession().to_string(), style.fg(MUTED)),
                ]));
            }
            v.push(Line::default());
            v.push(plain("Enter toggle · a everyone · Esc back".into(), MUTED));
        }
        Mode::Traffic { level, corner } => {
            title = "Traffic".to_string();
            for (key, l) in [("n", 0u8), ("h", 1), ("l", 2), ("r", 3)] {
                v.push(key_line(key, crate::game::TRAFFIC_NAMES[l as usize], *level == l));
            }
            v.push(Line::default());
            v.push(plain("Dwarves prefer high-traffic tiles,".into(), MUTED));
            v.push(plain("avoid low ones and only cross".into(), MUTED));
            v.push(plain("restricted ones when there's no".into(), MUTED));
            v.push(plain("other way. Keeps them out of".into(), MUTED));
            v.push(plain("bedrooms and off trap corridors.".into(), MUTED));
            v.push(plain(
                if corner.is_some() { "Enter: set second corner".into() } else { "Enter: set first corner".into() },
                KEY,
            ));
            v.push(Line::default());
            v.extend(cursor_info(g, app.cursor));
        }
        Mode::Look { sel } => {
            title = "Look".to_string();
            v.extend(look_lines(g, app.cursor, *sel));
        }
        Mode::PickOwner { .. } => {
            title = "Look".to_string();
            v.extend(look_lines(g, app.cursor, 0));
        }
        Mode::WorldMap { .. }
        | Mode::Legends { .. }
        | Mode::Trade { .. }
        | Mode::Status { .. }
        | Mode::Labors { .. }
        | Mode::Title { .. }
        | Mode::LoadMenu { .. }
        | Mode::Loading
        | Mode::WorldSetup { .. }
        | Mode::WorldGen => {
            title = String::new();
        }
        Mode::Nobles | Mode::Appoint { .. } => {
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
                for m in g.demands.iter().filter(|m| m.noble == d.id) {
                    let days = m.deadline.saturating_sub(g.tick) / crate::game::DAY;
                    v.push(plain(format!("  demands a {} ({} days left)", m.describe(), days), Color::Rgb(240, 200, 100)));
                }
                let office = g.zones.values().find(|z| z.kind == ZoneKind::Office && z.owner == Some(d.id));
                v.push(plain(
                    match office {
                        Some(z) => format!("  office value {}", g.area_value(&z.tiles.iter().copied().collect())),
                        None => "  no office (i, o to zone one)".into(),
                    },
                    MUTED,
                ));
            }
            if !any {
                v.push(plain("(none)".into(), MUTED));
            }
            v.push(Line::default());
            v.push(header("Positions"));
            for (i, p) in crate::game::Position::ALL.iter().enumerate() {
                let who = g.holder(*p).map_or("vacant".to_string(), |d| d.name.clone());
                v.push(Line::from(vec![
                    Span::styled(format!("{} ", i + 1), Style::default().fg(KEY)),
                    Span::styled(format!("{}: ", p.title()), Style::default().fg(TEXT)),
                    Span::styled(who, Style::default().fg(if g.holder(*p).is_some() { Color::Rgb(255, 230, 120) } else { MUTED })),
                ]));
                v.push(plain(format!("   {}", p.duty()), MUTED));
                if let Some(f) = g.post_fit(*p) {
                    v.push(plain(format!("   {}: {}", f.name(), p.effect(f)), rgb(f.color())));
                    if let Some(h) = g.holder(*p) {
                        if g.office_of(h.id).is_none() {
                            v.push(plain("   no office: zone one (i, o); it goes to them".into(), MUTED));
                        }
                    }
                }
            }
            v.push(plain("1-6: choose who holds a post · x: vacate all".into(), MUTED));
            v.push(Line::default());
            v.push(header(&format!("Justice ({} unpunished)", g.unpunished())));
            if g.crimes.is_empty() {
                v.push(plain("No crimes on the record.".into(), MUTED));
            }
            for c in g.crimes.iter().rev().take(6) {
                let who = g.dwarves.iter().find(|d| d.id == c.culprit).map_or("someone".to_string(), |d| d.name.clone());
                v.push(plain(
                    format!("{}: {} {}{}", c.kind.name(), who, c.what, if c.punished { " (punished)" } else { "" }),
                    if c.punished { MUTED } else { Color::Rgb(240, 150, 110) },
                ));
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
        Mode::Military | Mode::Expedition { .. } => {
            title = "Military".to_string();
            let alert = if g.alert { "ENGAGE hostiles" } else { "Stand down" };
            v.push(key_line("a", &format!("Orders: {}", alert), g.alert));
            v.push(key_line("h", if g.hunt { "Hunting game: ON" } else { "Hunting game: off" }, g.hunt));
            v.push(key_line(
                "c",
                if g.civ_alert { "Civilian alert: RAISED" } else { "Civilian alert: off" },
                g.civ_alert,
            ));
            v.push(Line::default());
            let sel = app.squad_sel.min(g.squads.len().saturating_sub(1));
            v.push(header("Squads   n new · [ ] select"));
            if g.squads.is_empty() {
                v.push(plain("No squads yet (n).".into(), MUTED));
            }
            for (i, s) in g.squads.iter().enumerate() {
                let order = match s.order {
                    crate::game::SquadOrder::Train => "training".to_string(),
                    crate::game::SquadOrder::Station => "holding station".to_string(),
                    crate::game::SquadOrder::Kill(t) => {
                        format!("hunting the {}", g.creatures.get(&t).map_or("quarry", |c| c.name()))
                    }
                };
                let style = if i == sel { Style::default().fg(BLACK).bg(KEY) } else { Style::default().fg(TEXT) };
                v.push(Line::from(Span::styled(format!(" {} ({}) ", s.name, s.members.len()), style)));
                v.push(plain(format!("   {} · {}{}", order, s.weapon.name(), if s.armor { ", armored" } else { "" }), MUTED));
                if i == sel {
                    for m in &s.members {
                        if let Some(d) = g.dwarves.iter().find(|d| d.id == *m && d.alive) {
                            let weapon = d.held.and_then(|id| g.items.get(&id)).map_or("unarmed".into(), |it| it.name());
                            let ammo = if d.ammo > 0 { format!(", {} bolts", d.ammo) } else { String::new() };
                            v.push(Line::from(vec![
                                Span::styled("   ☻ ", Style::default().fg(rgb(d.color))),
                                Span::styled(d.name.clone(), Style::default().fg(TEXT)),
                            ]));
                            v.push(plain(format!("     {}{} · {}", weapon, ammo, task_desc(g, d)), MUTED));
                            v.push(kit_line(g, d.id));
                        }
                    }
                }
            }
            if !g.squads.is_empty() {
                v.push(Line::default());
                v.push(key_line("u", "Members", false));
                v.push(key_line("s", "Station at the cursor", false));
                v.push(key_line("k", "Kill a creature", false));
                v.push(key_line("e", "Send on an expedition", false));
                v.push(key_line("t", "Back to training", false));
                v.push(key_line("w", "Uniform: next weapon", false));
                v.push(key_line("r", "Uniform: armor on/off", false));
                v.push(key_line("X", "Disband", false));
            }
            if let Some(ex) = &g.expedition {
                v.push(Line::default());
                let days = ex.back.saturating_sub(g.tick) / crate::game::DAY + 1;
                v.push(plain(format!("⚑ {} ({}) away at {}", ex.squad, ex.members.len(), ex.dest.name), Color::Rgb(200, 220, 255)));
                v.push(plain(format!("   back in about {} day{}", days, if days == 1 { "" } else { "s" }), MUTED));
            }
            let loose: Vec<&Dwarf> = g.dwarves.iter().filter(|d| d.alive && d.soldier && g.squad_of(d.id).is_none()).collect();
            if !loose.is_empty() {
                v.push(Line::default());
                v.push(header("Soldiers without a squad"));
                for d in loose {
                    v.push(plain(format!(" ☻ {} · {}", d.name, task_desc(g, d)), TEXT));
                }
                if g.squads.is_empty() {
                    v.push(key_line("s", "Station them at the cursor", false));
                } else {
                    v.push(plain("Add them to a squad with u.".into(), MUTED));
                }
            }
            for (i, l) in g.cage_lines().into_iter().enumerate() {
                v.push(if i == 0 { Line::default() } else { plain(l, Color::Rgb(220, 200, 150)) });
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

fn focus_line(d: &Dwarf) -> Line<'static> {
    let f = d.focus();
    let col = if f >= 104 {
        Color::Rgb(120, 220, 120)
    } else if f > 92 {
        TEXT
    } else if f > 76 {
        Color::Rgb(230, 200, 60)
    } else {
        Color::Rgb(240, 120, 80)
    };
    let word = d.focus_word();
    Line::from(vec![
        Span::styled("Focus: ", Style::default().fg(TEXT)),
        Span::styled(if word.is_empty() { format!("{}%", f) } else { format!("{}% ({})", f, word) }, Style::default().fg(col)),
    ])
}

fn unit_view(g: &Game, d: &Dwarf, sel: usize) -> Vec<Line<'static>> {
    let (mood, mc) = d.mood();
    let mut v = vec![
        Line::from(Span::styled(d.called(), Style::default().fg(rgb(d.color)).add_modifier(Modifier::BOLD))),
        plain(
            match (d.noble, g.position_of(d.id)) {
                (Some(n), _) => format!("{} ({})", n.title(), d.profession()),
                (None, Some(p)) => format!("{} ({})", p.title(), d.profession()),
                (None, None) => d.profession().to_string(),
            },
            MUTED,
        ),
        Line::from(vec![
            Span::styled("Mood: ", Style::default().fg(TEXT)),
            Span::styled(mood, Style::default().fg(rgb(mc))),
        ]),
        plain(if d.alive { task_desc(g, d) } else { "Deceased".into() }, KEY),
        focus_line(d),
        Line::default(),
        bar("Thirst", d.thirst, THIRSTY),
        bar("Hunger", d.hunger, HUNGRY),
        bar("Fatigue", d.fatigue, TIRED),
    ];
    if let (Some(deed), true) = (&d.deed, d.epithet.is_some()) {
        v.insert(2, plain(format!("Hero: {}", deed), Color::Rgb(255, 215, 60)));
    }
    let hurt: Vec<Line> = PARTS
        .iter()
        .filter(|p| d.body.part_status(**p) != "fine")
        .map(|p| plain(format!("  {}: {}", p.name(), d.body.part_status(*p)), Color::Rgb(240, 130, 100)))
        .collect();
    if !hurt.is_empty() || d.body.bleeding > 0 {
        v.push(Line::default());
        v.push(header("Health"));
        if d.body.bleeding > 0 {
            let from = d.body.worst_bleed().map_or(String::new(), |p| format!(" from the {}", p.name()));
            let help = if d.body.ticks_to_death().is_some() { ", needs help" } else { "" };
            v.push(plain(format!("  Bleeding{} (blood {}%{})", from, d.body.blood * 100 / d.body.max_blood, help), Color::Rgb(255, 60, 60)));
        }
        if d.body.infected() {
            v.push(plain("  An infected wound".into(), Color::Rgb(220, 160, 60)));
        } else if d.body.dirty {
            v.push(plain("  A dirty wound (needs cleaning)".into(), Color::Rgb(200, 180, 90)));
        }
        v.extend(hurt);
    }
    v.push(Line::default());
    v.push(header("Personality"));
    if let Some(age) = d.age {
        let sex = match d.female {
            Some(true) => ", female",
            Some(false) => ", male",
            None => "",
        };
        v.push(plain(format!("  Age {}{}", age, sex), MUTED));
    }
    let name_of = |id: u32| g.dwarves.iter().find(|o| o.id == id).map(|o| (o.name.clone(), o.alive));
    if let Some((n, alive)) = d.spouse.and_then(name_of) {
        v.push(plain(if alive { format!("  Married to {}", n) } else { format!("  Widowed ({})", n) }, Color::Rgb(255, 200, 220)));
    }
    if let Some((m, f)) = d.parents {
        let m = name_of(m).map_or("?".into(), |x| x.0);
        let f = name_of(f).map_or("?".into(), |x| x.0);
        v.push(plain(format!("  Child of {} and {}", m, f), MUTED));
    }
    let kids: Vec<String> = g.dwarves.iter().filter(|o| o.alive && o.family_of(d.id)).map(|o| o.name.split(' ').next().unwrap_or("").to_string()).collect();
    if !kids.is_empty() {
        v.push(plain(format!("  Children: {}", kids.join(", ")), MUTED));
    }
    if d.due.is_some() {
        v.push(plain("  Expecting a child".into(), Color::Rgb(150, 255, 150)));
    }
    for t in d.personality.describe() {
        v.push(plain(format!("  {}", t), TEXT));
    }
    let food = match d.likes_food {
        Some(ItemKind::Meal) => "prepared meals",
        Some(ItemKind::Meat) => "meat",
        _ => "stoutcaps",
    };
    v.push(plain(format!("  Likes {} and {}", d.likes_mat.name(), food), MUTED));
    if !d.needs.is_empty() {
        v.push(Line::default());
        v.push(header("Needs"));
        for n in &d.needs {
            let col = match n.level {
                l if l >= 150 => Color::Rgb(120, 220, 120),
                l if l > crate::dwarf::NEED_UNMET => MUTED,
                l if l > crate::dwarf::NEED_BADLY => Color::Rgb(230, 200, 60),
                _ => Color::Rgb(240, 120, 80),
            };
            // Dots show how much the need matters to this dwarf.
            let weight = "•".repeat(n.strength.min(3) as usize);
            v.push(plain(format!("  {:<4}{:<11}{}", weight, n.kind.name(), n.status()), col));
        }
    }
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
    for it in d.clothes.iter().filter_map(|id| g.items.get(id)) {
        let rags = it.wear >= crate::items::TATTERED;
        v.push(plain(format!("  Wears {}", it.name()), if rags { Color::Rgb(230, 200, 60) } else { MUTED }));
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
        k("i", "zones: meeting, bedroom, dining, dump..."),
        k("k", "look: inspect anything at the cursor"),
        k("z", "fortress status: population, stocks, health"),
        k("b", "build workshops, furniture; C constructions"),
        k("q", "query building: queue work orders"),
        k("u / v", "unit list / view dwarf (m drafts)"),
        k("y", "labors: who does what, work details"),
        k("w / o", "burrows / traffic designations"),
        k("m", "military: squads, orders, uniforms, alerts"),
        k("n", "nobles, mandates, bedrooms, wealth"),
        k("c", "the chronicle: each year told as a story"),
        k("r", "reports: every past announcement, filtered"),
        k("F3", "performance: frames per second, and where the time goes"),
        k("\\", "status, brain, reports, chronicle, legends: side pane or full screen"),
        k("M", "grow the map north, east, south or west"),
        k("A", "autoplay: the overseer plays (pick a style)"),
        k("B", "the overseer's brain: senses, ambitions, nudges"),
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
    let view = paint_world(f, inner, ow, ow.h, Some((cx, cy)));
    // Outline the land the fortress map will cover, east and south of the cursor.
    let size = app.embark_size;
    let (fw, fh) = embark_footprint(size);
    paint_footprint(f, inner, ow, view, (cx, cy), (fw, fh));
    let mut lines = world_info(ow, cx, cy, &app.flash);
    // What the land holds here.
    let biome = ow.cell(cx, cy).biome;
    let _ = biome;
    let bits = ow.site_features(app.seed, cx, cy);
    if bits != 0 {
        let mut feats = vec![Line::from(Span::styled("The land here:", Style::default().fg(MUTED)))];
        for (b, _, what) in crate::terrain::FEATURES.iter() {
            if bits & b != 0 {
                feats.push(plain(format!(" · {}", what), Color::Rgb(170, 220, 170)));
            }
        }
        let at = 2.min(lines.len());
        for (i, l) in feats.into_iter().enumerate() {
            lines.insert(at + i, l);
        }
    }
    lines.insert(
        1,
        Line::from(vec![
            Span::styled("Embark: ", Style::default().fg(MUTED)),
            Span::styled(
                format!(
                    "{} {}×{}, {} deep",
                    crate::world::MapSize::ALL.iter().position(|&m| m == size).map_or("Custom", |i| crate::world::MapSize::NAMES[i]),
                    size.w,
                    size.h,
                    size.d
                ),
                Style::default().fg(KEY),
            ),
            Span::styled("  (s [ ] { } d)", Style::default().fg(MUTED)),
        ]),
    );
    let mut side = cols[1];
    if view.cols < ow.w || view.rows < ow.h {
        let mh = (side.width.saturating_sub(2) as i32 * ow.h / ow.w.max(1)).clamp(4, 18) as u16 + 2;
        let parts = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(mh), Constraint::Min(5)])
            .split(side);
        let mb = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(MUTED))
            .title(Span::styled(format!(" {}×{} ", ow.w, ow.h), Style::default().fg(KEY)));
        let mi = mb.inner(parts[0]);
        f.render_widget(mb, parts[0]);
        paint_minimap(f, mi, ow, view);
        side = parts[1];
    }
    f.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(MUTED))
                    .title(Span::styled(" Site ", Style::default().fg(KEY))),
            )
            .wrap(Wrap { trim: false }),
        side,
    );
}

/// What part of the world a map view shows: top-left cell, size in cells,
/// and how many world cells each character stands for.
#[derive(Clone, Copy)]
struct WorldView {
    x0: i32,
    y0: i32,
    cols: i32,
    rows: i32,
    scale: i32,
}

/// Draw the world map into `inner`. Only the first `rows` rows exist yet
/// (during generation). With a cursor the view scrolls to keep it in sight;
/// without one the whole world is shrunk to fit.
fn paint_world(f: &mut Frame, inner: Rect, ow: &Overworld, rows: i32, cursor: Option<(i32, i32)>) -> WorldView {
    let (iw, ih) = (inner.width as i32, inner.height as i32);
    let rows = rows.min(ow.cells.len() as i32 / ow.w.max(1));
    // One character a cell, like a page of an atlas.
    let fits = ow.w <= iw && ow.h <= ih;
    let cw = 1;
    let mut view = if fits || cursor.is_some() {
        let (cols, vrows) = ((iw / cw).min(ow.w), ih.min(ow.h));
        let (cx, cy) = cursor.unwrap_or((ow.w / 2, ow.h / 2));
        WorldView {
            x0: (cx - cols / 2).clamp(0, (ow.w - cols).max(0)),
            y0: (cy - vrows / 2).clamp(0, (ow.h - vrows).max(0)),
            cols,
            rows: vrows,
            scale: 1,
        }
    } else {
        let scale = ((ow.w + iw - 1) / iw.max(1)).max((ow.h + ih - 1) / ih.max(1)).max(1);
        WorldView { x0: 0, y0: 0, cols: (ow.w + scale - 1) / scale, rows: (ow.h + scale - 1) / scale, scale }
    };
    view.cols = view.cols.min(iw / cw);
    let ox = inner.x as i32 + ((iw - view.cols * cw) / 2).max(0);
    let oy = inner.y as i32 + ((ih - view.rows) / 2).max(0);
    let buf = f.buffer_mut();
    for vy in 0..view.rows {
        for vx in 0..view.cols {
            let (x, y) = (view.x0 + vx * view.scale, view.y0 + vy * view.scale);
            if y >= rows || x >= ow.w {
                continue;
            }
            // When shrunk, a settlement anywhere in the block shows through.
            let mut pick = (x, y);
            'block: for by in y..(y + view.scale).min(rows) {
                for bx in x..(x + view.scale).min(ow.w) {
                    if ow.cell(bx, by).site > 0 {
                        pick = (bx, by);
                        break 'block;
                    }
                }
            }
            let c = ow.cell(pick.0, pick.1);
            let (mut ch, mut fg) = ow.look(pick.0, pick.1);
            let mut bg = if c.evil { Color::Rgb(45, 10, 55) } else { BLACK };
            if c.site > 0 {
                let st = &ow.sites[(c.site - 1) as usize];
                ch = if st.ruined { '×' } else { st.kind.glyph() };
                fg = match st.civ {
                    Some(civ) if !st.ruined => ow.civs[civ as usize].race.color(),
                    _ => Color::Rgb(150, 150, 150),
                };
            }
            if let Some((cx, cy)) = cursor {
                if cx >= x && cx < x + view.scale && cy >= y && cy < y + view.scale {
                    bg = Color::Rgb(220, 200, 0);
                    fg = BLACK;
                }
            }
            let (sx, sy) = (ox + vx * cw, oy + vy);
            buf.get_mut(sx as u16, sy as u16).set_char(ch).set_fg(fg).set_bg(bg);
        }
    }
    view
}

/// How many world cells an embark of each size covers (wide, high).
/// World cells a fortress map covers (about 48 tiles to a cell).
fn embark_footprint(size: crate::world::MapSize) -> (i32, i32) {
    (((size.w + 40) / 48).max(1), ((size.h + 16) / 48).max(1))
}

/// Tint the cells an embark at `at` would cover (the cursor cell itself is already marked).
fn paint_footprint(f: &mut Frame, inner: Rect, ow: &Overworld, view: WorldView, at: (i32, i32), (fw, fh): (i32, i32)) {
    if view.scale != 1 {
        return;
    }
    let _ = ow;
    let cw = 1;
    let ox = inner.x as i32 + ((inner.width as i32 - view.cols * cw) / 2).max(0);
    let oy = inner.y as i32 + ((inner.height as i32 - view.rows) / 2).max(0);
    let buf = f.buffer_mut();
    for y in at.1..at.1 + fh {
        for x in at.0..at.0 + fw {
            if (x, y) == at || x >= ow.w || y >= ow.h || x < view.x0 || y < view.y0 || x >= view.x0 + view.cols || y >= view.y0 + view.rows {
                continue;
            }
            let (sx, sy) = (ox + (x - view.x0) * cw, oy + (y - view.y0));
            for dx in 0..cw {
                buf.get_mut((sx + dx) as u16, sy as u16).set_bg(Color::Rgb(120, 105, 20));
            }
        }
    }
}

/// The whole world shrunk into `area`, with the part the main map shows outlined.
fn paint_minimap(f: &mut Frame, area: Rect, ow: &Overworld, shown: WorldView) {
    let mini = paint_world(f, area, ow, ow.h, None);
    let (iw, ih) = (area.width as i32, area.height as i32);
    let ox = area.x as i32 + ((iw - mini.cols) / 2).max(0);
    let oy = area.y as i32 + ((ih - mini.rows) / 2).max(0);
    let buf = f.buffer_mut();
    let (x1, y1) = (shown.x0 + shown.cols - 1, shown.y0 + shown.rows - 1);
    for vy in 0..mini.rows {
        for vx in 0..mini.cols {
            let (x, y) = (vx * mini.scale, vy * mini.scale);
            let inside = |a: i32, lo: i32, hi: i32| a + mini.scale > lo && a <= hi;
            if !(inside(x, shown.x0, x1) && inside(y, shown.y0, y1)) {
                continue;
            }
            let on_edge = !inside(x - mini.scale, shown.x0, x1)
                || !inside(x + mini.scale, shown.x0, x1)
                || !inside(y - mini.scale, shown.y0, y1)
                || !inside(y + mini.scale, shown.y0, y1);
            if on_edge {
                buf.get_mut((ox + vx) as u16, (oy + vy) as u16).set_bg(Color::Rgb(90, 80, 30));
            }
        }
    }
}

fn draw_world_setup(f: &mut Frame, app: &App, sel: usize) {
    let area = f.size();
    backdrop(f, area);
    let p = &app.world_params;
    let values = [
        preset_name(p).to_string(),
        format!("{}", p.seed),
        {
            let (w, h) = p.dims();
            format!("{} {}×{}", LEVELS[3][p.size.min(2) as usize], w, h)
        },
        format!("{} years", p.history),
        LEVELS[0][p.civs.min(2) as usize].to_string(),
        LEVELS[1][p.savagery.min(2) as usize].to_string(),
        LEVELS[2][p.minerals.min(2) as usize].to_string(),
        String::new(),
    ];
    let notes = [
        "Quick, Standard, Long history, Harsh",
        "type digits · r random",
        "bigger worlds hold more peoples",

        "years simulated before you arrive",
        "how many peoples share the world",
        "how dangerous the wildlife is",
        "how much ore and gem lies in the rock",
        "",
    ];
    let mut v: Vec<Line> = vec![Line::default()];
    for (i, row) in SETUP_ROWS.iter().enumerate() {
        let on = i == sel;
        if i == SETUP_ROWS.len() - 1 {
            v.push(Line::default());
            let st = if on { Style::default().fg(BLACK).bg(KEY).add_modifier(Modifier::BOLD) } else { Style::default().fg(KEY) };
            v.push(Line::from(Span::styled(format!("   ▶ {}   ", row), st)));
            continue;
        }
        let st = if on { Style::default().fg(BLACK).bg(Color::Rgb(90, 110, 170)) } else { Style::default().fg(TEXT) };
        let arrows = if on { ("◀ ", " ▶") } else { ("  ", "  ") };
        v.push(Line::from(vec![
            Span::styled(format!("  {:<15}", row), Style::default().fg(MUTED)),
            Span::styled(format!("{}{:^16}{}", arrows.0, values[i], arrows.1), st),
            Span::styled(format!("  {}", notes[i]), Style::default().fg(Color::Rgb(100, 100, 110))),
        ]));
    }
    v.push(Line::default());
    v.push(plain("  ↑↓ choose · ←→ change · Enter generate · Esc back".into(), MUTED));
    let r = centered(area, 82, v.len() as u16 + 2);
    f.render_widget(Clear, r);
    f.render_widget(
        Paragraph::new(v).style(Style::default().bg(Color::Rgb(12, 12, 20))).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(KEY))
                .title(Span::styled(" Create a new world ", Style::default().fg(KEY))),
        ),
        r,
    );
}

fn draw_worldgen(f: &mut Frame, app: &App) {
    let area = f.size();
    let Some(g) = &app.worldgen else { return };
    let ow = &g.ow;
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(10)])
        .split(area);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(SIDEBAR_W + 8)])
        .split(rows[1]);
    let (stage, frac) = g.progress();
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" ☼ DwarfClone ", Style::default().fg(KEY).add_modifier(Modifier::BOLD)),
            Span::styled(format!("│ Creating {} │ seed {}", ow.name, g.params.seed), Style::default().fg(TEXT)),
        ]))
        .style(Style::default().bg(Color::Rgb(20, 20, 30))),
        rows[0],
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(MUTED))
        .title(Span::styled(format!(" Year {} ", ow.year), Style::default().fg(KEY)));
    let inner = block.inner(cols[0]);
    f.render_widget(block, cols[0]);
    paint_world(f, inner, ow, g.rows_done(), None);

    // Running totals, then the latest events.
    let width = 28usize;
    let filled = ((frac * width as f32).round() as usize).min(width);
    let alive_civs = ow.civs.iter().filter(|c| c.alive).count();
    let towns = ow.sites.iter().filter(|s| s.civ.is_some() && !s.ruined).count();
    let ruins = ow.sites.iter().filter(|s| s.ruined).count();
    let figures = ow.figures.len();
    let mut v = vec![
        plain(stage, TEXT),
        Line::from(vec![
            Span::styled("█".repeat(filled), Style::default().fg(Color::Rgb(255, 200, 80))),
            Span::styled("░".repeat(width - filled), Style::default().fg(Color::Rgb(60, 55, 50))),
            Span::styled(format!(" {:>3}%", (frac * 100.0) as u32), Style::default().fg(MUTED)),
        ]),
        Line::default(),
        plain(format!("Year          {:>5} of {}", ow.year, g.params.history), TEXT),
        plain(format!("Civilizations {:>5} ({} fallen)", alive_civs, ow.civs.len() - alive_civs), TEXT),
        plain(format!("Settlements   {:>5}", towns), TEXT),
        plain(format!("Ruins         {:>5}", ruins), TEXT),
        plain(format!("Figures       {:>5}", figures), TEXT),
        plain(format!("Events        {:>5}", ow.events.len()), TEXT),
        Line::default(),
        header("Lately"),
    ];
    // Events wrap to about two lines each; show as many as fit.
    let fit = (cols[1].height as usize).saturating_sub(v.len() + 3) / 2;
    for e in ow.events.iter().rev().take(fit).collect::<Vec<_>>().into_iter().rev() {
        v.push(Line::from(vec![
            Span::styled(format!("{:>4} ", e.year), Style::default().fg(MUTED)),
            Span::styled(e.text.clone(), Style::default().fg(Color::Rgb(190, 190, 200))),
        ]));
    }
    let keys = if g.done() { " Enter accept · r another world · Esc settings " } else { " Space hurry · r new seed · Esc settings " };
    f.render_widget(
        Paragraph::new(v)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(MUTED))
                    .title(Span::styled(if g.done() { " The world is ready " } else { " Generating " }, Style::default().fg(KEY)))
                    .title_bottom(Span::styled(keys, Style::default().fg(KEY))),
            ),
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
    if let Some(r) = ow.region_at(x, y) {
        v.push(plain(format!("{}, a {} of {} cells", r.name, r.kind.name(), r.size), Color::Rgb(170, 200, 230)));
    }
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
        let aq = ow.aquifer(x, y);
        v.push(plain(
            format!("Aquifer: {}", ["none", "light", "heavy"][aq.min(2) as usize]),
            if aq > 0 { Color::Rgb(90, 160, 255) } else { MUTED },
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
    if tab == 4 {
        let mut regions: Vec<&crate::overworld::Region> = ow.regions.iter().collect();
        regions.sort_by(|a, b| b.size.cmp(&a.size));
        return regions
            .into_iter()
            .map(|r| {
                Line::from(vec![
                    Span::styled(format!("{:<34}", r.name), Style::default().fg(TEXT)),
                    Span::styled(format!("{:<16}", r.kind.name()), Style::default().fg(Color::Rgb(170, 200, 230))),
                    Span::styled(format!("{:>6} cells  around ({}, {})", r.size, r.x, r.y), Style::default().fg(MUTED)),
                ])
            })
            .collect();
    }
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
    let names = ["1 History", "2 Civilizations", "3 Sites", "4 Figures", "5 Regions"];
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
                    .title_bottom(Span::styled(" Tab/1-5 switch · ↑↓ PgUp/PgDn scroll · Esc back ", Style::default().fg(MUTED))),
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
    let ok = app.game.offer_ok(offer, ask);
    let margin = app.game.trade_margin();
    let broker = app.game.holder(crate::game::Position::Broker).map(|d| d.name.clone());
    let mut summary = vec![Line::from(vec![
        Span::styled(format!(" Offer ☼{}   ", offer), Style::default().fg(TEXT)),
        Span::styled(format!("Request ☼{}   ", ask), Style::default().fg(TEXT)),
        Span::styled(
            if ok { "The merchants look interested.".to_string() } else { format!("They want {}% over the value.", margin) },
            Style::default().fg(if ok { Color::Rgb(120, 255, 160) } else { Color::Rgb(240, 150, 90) }),
        ),
        Span::styled(
            match broker {
                Some(b) => format!("   Broker {} haggles ({}% markup)", b, margin),
                None => "   No broker: 10% markup".to_string(),
            },
            Style::default().fg(MUTED),
        ),
    ])];
    if let Some(fl) = &app.flash {
        summary.push(plain(format!(" {}", fl), KEY));
    }
    f.render_widget(Paragraph::new(summary), parts[1]);
}

pub fn draw(f: &mut Frame, app: &mut App) {
    TARGETS.with(|v| v.borrow_mut().clear());
    let area = f.size();
    match app.mode {
        Mode::Title { sel } => return draw_title(f, app, sel),
        Mode::LoadMenu { sel, confirm } => return draw_load_menu(f, app, sel, confirm),
        Mode::Loading => return draw_loading(f, app),
        Mode::WorldSetup { sel } => return draw_world_setup(f, app, sel),
        Mode::WorldGen => return draw_worldgen(f, app),
        _ => {}
    }
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
    // A side pane takes the sidebar's place and more, leaving the map live beside it.
    let pane_w = if app.docked && app.dockable() && area.width >= 100 { Some((area.width * 3 / 5).clamp(64, area.width - 36)) } else { None };
    let mid = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(pane_w.unwrap_or(SIDEBAR_W))])
        .split(rows[1]);
    let pane = pane_w.map(|w| Rect::new(area.x + area.width - w, rows[1].y, w, rows[1].height + rows[2].height));

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
        match &g.overseer {
            Some(o) => Span::styled(
                match o.ambition {
                    Some(a) => format!("│ AUTO: {} · {} ", o.style.name(), a.name()),
                    None => format!("│ AUTO: {} ", o.style.name()),
                },
                Style::default().fg(BLACK).bg(Color::Rgb(120, 190, 255)),
            ),
            None => Span::raw(""),
        },
        if g.weather != crate::game::Weather::Clear {
            Span::styled(format!("│ {} ", g.weather.name()), Style::default().fg(Color::Rgb(150, 180, 230)))
        } else {
            Span::raw("")
        },
        match &g.festival {
            Some(f) => Span::styled(format!("│ ☼ {}! ", cap_first(f.kind.name())), Style::default().fg(Color::Rgb(255, 215, 60)).add_modifier(Modifier::BOLD)),
            None => Span::raw(""),
        },
        if g.hostile_count() > 0 {
            Span::styled(format!(" ⚠ {} hostiles ", g.hostile_count()), Style::default().fg(Color::Rgb(255, 255, 255)).bg(Color::Rgb(160, 20, 20)))
        } else {
            Span::raw("")
        },
        if g.civ_alert {
            Span::styled(" CIVILIAN ALERT ", Style::default().fg(BLACK).bg(Color::Rgb(255, 170, 60)))
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
    if pane.is_none() {
    register_lines(Rect::new(mid[1].x + 1, mid[1].y + 1, mid[1].width.saturating_sub(2), mid[1].height.saturating_sub(2)), &lines, true);
    let side = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(MUTED))
                .title(Span::styled(format!(" {} ", title), Style::default().fg(KEY))),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(side, mid[1]);
    }

    // Announcements, and beside them what the overseer has been doing.
    let g = &app.game;
    let (news, deeds) = match &g.overseer {
        Some(_) => {
            let halves = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(55), Constraint::Percentage(45)]).split(rows[2]);
            (halves[0], Some(halves[1]))
        }
        None => (rows[2], None),
    };
    if let (Some(r), Some(o)) = (deeds, &g.overseer) {
        // Newest at the bottom, each wrapped under its age in days.
        let n = r.height.saturating_sub(2) as usize;
        let w = (r.width as usize).saturating_sub(6).max(10);
        let mut lines: Vec<Line> = Vec::new();
        for (t, text) in o.notes.iter().rev() {
            let mut these: Vec<Line> = wrap_words(text, w)
                .into_iter()
                .enumerate()
                .map(|(i, s)| {
                    let age = if i == 0 { format!("{:>3} ", g.tick.saturating_sub(*t) / crate::game::DAY) } else { "    ".to_string() };
                    Line::from(vec![Span::styled(age, Style::default().fg(MUTED)), Span::styled(s, Style::default().fg(Color::Rgb(150, 210, 255)))])
                })
                .collect();
            these.append(&mut lines);
            lines = these;
            if lines.len() >= n {
                break;
            }
        }
        let lines: Vec<Line> = lines.split_off(lines.len().saturating_sub(n));
        f.render_widget(
            Paragraph::new(lines).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(MUTED))
                    .title(Span::styled(format!(" The overseer, {} (days ago) ", o.title()), Style::default().fg(Color::Rgb(150, 210, 255)))),
            ),
            r,
        );
    }
    let rows = [rows[0], rows[1], news];
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
                .title(Span::styled(" Announcements (r: all reports) ", Style::default().fg(KEY))),
        ),
        rows[2],
    );
    add_target(rows[2], Target::Key(crossterm::event::KeyCode::Char('r')));

    // Side-pane screens draw into the pane; the rest over the whole screen.
    let full = area;
    let area = match pane {
        Some(p) => {
            f.render_widget(Clear, p);
            f.render_widget(Block::default().style(Style::default().bg(Color::Rgb(12, 12, 20))), p);
            p
        }
        None => full,
    };
    match app.mode {
        Mode::GameOver => draw_game_over(f, full, &app.game),
        Mode::Legends { tab, scroll, .. } => draw_legends(f, area, &app.game.overworld, tab, scroll),
        Mode::Trade { side, sel } => draw_trade(f, area, app, side, sel),
        Mode::Status { tab, scroll } => draw_status(f, area, &app.game, tab, scroll),
        Mode::Labors { row, col, detail } => draw_labors(f, area, app, row, col, detail),
        Mode::Appoint { pos, sel } => draw_appoint(f, area, &app.game, pos, sel),
        Mode::PickOwner { zone, sel } => draw_pick_owner(f, area, &app.game, zone, sel),
        Mode::Chronicle { idx } => draw_chronicle(f, area, &app.game, idx),
        Mode::Expedition { squad, sel } => draw_expedition(f, area, &app.game, squad, sel),
        Mode::Autoplay { sel } => draw_autoplay(f, area, &app.game, sel),
        Mode::Brain { sel } => draw_brain(f, area, &app.game, sel),
        Mode::GrowMap { n, e, s, w } => draw_grow(f, area, &app.game, (n, e, s, w)),
        Mode::Reports { filter, scroll } => draw_reports(f, area, &app.game, filter, scroll),
        Mode::BuildMenu => draw_build_menu(f, area),
        Mode::Help => {
            popup(f, area, 60, 47, " Help ", help_text());
        }
        Mode::ConfirmQuit => {
            popup(
            f,
            area,
            40,
            7,
            " Quit ",
            vec![
                key_line("s", "Save and quit", false),
                key_line("t", "Save and return to the title", false),
                key_line("y", "Quit without saving", false),
                plain("  any other key: keep playing".into(), MUTED),
            ],
            );
        }
        _ => {}
    }
    // Full-screen views sit over everything else.
    if matches!(app.mode, Mode::Legends { .. } | Mode::Trade { .. } | Mode::Status { .. } | Mode::Labors { .. } | Mode::Chronicle { .. } | Mode::Brain { .. } | Mode::Reports { .. } | Mode::GameOver) {
        add_target(area, Target::Block);
    }
    if let Some(p) = pane {
        let hint = " \\ full screen · Esc close ";
        let w = hint.chars().count() as u16;
        if p.width > w + 2 {
            f.render_widget(Paragraph::new(Span::styled(hint, Style::default().fg(MUTED))), Rect::new(p.x + p.width - w - 1, p.y + p.height - 1, w, 1));
        }
    } else if app.dockable() && !app.docked {
        let hint = " \\ side pane ";
        let w = hint.chars().count() as u16;
        f.render_widget(Paragraph::new(Span::styled(hint, Style::default().fg(MUTED))), Rect::new(full.x + full.width.saturating_sub(w + 1), full.y + full.height - 1, w, 1));
    }
    if let Some(p) = app.perf.and_then(|p| p.shown) {
        draw_perf(f, full, &p);
    }
    draw_tooltip(f, app);
    if app.in_fortress() && matches!(app.mode, Mode::Normal) {
        let over = if app.map_rect.width > 0 { app.map_rect } else { area };
        draw_banner(f, over, &app.game);
        if let Some((s, _)) = &app.spot {
            draw_spotlight(f, over, s, app.game.overseer.is_some());
        }
    }
}

/// The performance overlay (F3): frames per second and where the time goes.
fn draw_perf(f: &mut Frame, area: Rect, p: &crate::app::PerfShown) {
    let names = ["water", "overseer", "dwarves", "creatures", "the rest"];
    let mut parts: Vec<(usize, f64)> = p.parts.iter().copied().enumerate().take(5).collect();
    parts.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    let fps_col = if p.fps >= 25.0 { Color::Rgb(120, 220, 120) } else if p.fps >= 12.0 { Color::Rgb(240, 200, 80) } else { Color::Rgb(240, 90, 80) };
    let lines = vec![
        Line::from(vec![
            Span::styled(format!(" {:>5.1} fps ", p.fps), Style::default().fg(fps_col).add_modifier(Modifier::BOLD)),
            Span::styled(format!("frame: sim {:.1} ms, draw {:.1} ms", p.sim, p.draw), Style::default().fg(TEXT)),
        ]),
        plain(format!(" {:.0} ticks/s · slowest tick {:.1} ms · {:.0} path searches/s ({:.0}k nodes)", p.tps, p.parts[5], p.paths, p.nodes / 1000.0), MUTED),
        plain(format!(" sim: {}", parts.iter().map(|(i, v)| format!("{} {:.1}", names[*i], v)).collect::<Vec<_>>().join(" · ")), MUTED),
    ];
    let w = 80.min(area.width);
    let r = Rect::new(area.x + area.width - w, area.y + 1, w, 5);
    f.render_widget(Clear, r);
    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(Color::Rgb(10, 10, 16))).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(MUTED)).title(Span::styled(" Performance (F3) ", Style::default().fg(KEY)))),
        r,
    );
}

/// A big moment, told in a card over the top of the map (the camera is on it).
fn draw_spotlight(f: &mut Frame, map: Rect, s: &crate::game::spotlight::Spotlight, auto: bool) {
    let w = (map.width.saturating_sub(6)).min(70).max(30);
    let col = rgb(s.color);
    let inner = w.saturating_sub(4) as usize;
    let mut lines = vec![Line::default()];
    for l in wrap_words(&s.text, inner) {
        lines.push(Line::from(Span::styled(format!(" {}", l), Style::default().fg(TEXT))));
    }
    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        if auto { " (the overseer carries on)" } else { " Enter: go on · Space: go on and unpause" },
        Style::default().fg(MUTED),
    )));
    let h = lines.len() as u16 + 2;
    let r = Rect::new(map.x + (map.width.saturating_sub(w)) / 2, map.y + 1, w.min(map.width), h.min(map.height));
    f.render_widget(Clear, r);
    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(Color::Rgb(18, 12, 10))).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Double)
                .border_style(Style::default().fg(col))
                .title(Span::styled(format!(" {} ", s.title), Style::default().fg(col).add_modifier(Modifier::BOLD))),
        ),
        r,
    );
}

/// Split text into lines of at most `w` characters, on spaces.
fn wrap_words(text: &str, w: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > w {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

/// The fortress has fallen: how long it stood, who lived there and how they died.
fn draw_game_over(f: &mut Frame, area: Rect, g: &Game) {
    let years = g.tick as f64 / crate::game::YEAR as f64;
    let mut causes: Vec<(String, usize)> = Vec::new();
    for d in &g.annals.deaths {
        let what = match (&d.killer, d.cause.as_str()) {
            (Some(k), "struck down") => format!("struck down by {}s", k),
            (Some(k), "bled to death") => format!("bled from {} wounds", k),
            _ => d.cause.clone(),
        };
        match causes.iter_mut().find(|c| c.0 == what) {
            Some(c) => c.1 += 1,
            None => causes.push((what, 1)),
        }
    }
    causes.sort_by(|a, b| b.1.cmp(&a.1));
    let red = Color::Rgb(220, 70, 60);
    let gold = Color::Rgb(255, 215, 60);
    let mut v = vec![
        Line::default(),
        Line::from(Span::styled(format!("The fortress of {} has fallen.", g.fortress_name), Style::default().fg(red).add_modifier(Modifier::BOLD))).alignment(ratatui::layout::Alignment::Center),
        Line::from(Span::styled(g.date_string(), Style::default().fg(MUTED))).alignment(ratatui::layout::Alignment::Center),
        Line::default(),
        plain(format!(" It stood {:.1} years, and {} dwarves lived there.", years, g.dwarves.len()), TEXT),
        plain(format!(" When it fell it was worth ☼{}.", g.wealth()), TEXT),
    ];
    let sieges = g.annals.sieges.len();
    if sieges > 0 {
        let broken = g.annals.sieges.iter().filter(|s| s.outcome == "routed").count();
        v.push(plain(format!(" It weathered {} siege{} and broke {}.", sieges, if sieges == 1 { "" } else { "s" }, broken), TEXT));
    }
    if let Some(m) = &g.monument {
        v.push(plain(format!(" It raised {}.", m), gold));
    }
    let heroes: Vec<String> = g.dwarves.iter().filter(|d| d.epithet.is_some()).map(|d| d.called()).collect();
    if !heroes.is_empty() {
        v.push(plain(format!(" Its heroes: {}.", heroes.join(", ")), gold));
    }
    v.push(Line::default());
    v.push(plain(" How they died:".into(), KEY));
    for (what, n) in causes.iter().take(6) {
        v.push(plain(format!("   {:>3}  {}", n, what), TEXT));
    }
    if let Some(l) = g.log.iter().rev().find(|l| l.color == (255, 60, 60)) {
        v.push(Line::default());
        v.push(plain(format!(" The last to fall: {}", l.text), red));
    }
    v.push(Line::default());
    let mut keys = vec![];
    if !g.chronicles.is_empty() {
        keys.push(key_line("c", "Read the chronicle", false));
    }
    keys.push(key_line("l", "Legends of the world", false));
    keys.push(key_line("r", "Every report", false));
    keys.push(key_line("w", "Look upon the ruins", false));
    keys.push(key_line("t", "Return to the title (the ruin is saved)", false));
    v.extend(keys);
    let h = v.len() as u16 + 2;
    popup(f, area, 78, h, " ☠ Game over ☠ ", v);
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
    if g.zone_at(p).map_or(false, |z| z.kind.ownable()) {
        v.push(plain("o: choose who owns it".into(), KEY));
    }
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
                if d.ghost {
                    Line::from(vec![
                        Span::styled("☺ ", hi(i).fg(Color::Rgb(170, 200, 255))),
                        Span::styled(format!("The ghost of {}", d.name), hi(i).fg(TEXT)),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(if d.soldier { "☻ " } else { "☺ " }, hi(i).fg(rgb(d.color))),
                        Span::styled(d.name.clone(), hi(i).fg(TEXT)),
                    ])
                }
            }
            LookEntry::Creature(id) => {
                let Some(c) = g.creatures.get(&id) else { continue };
                let info = c.species.info();
                Line::from(vec![
                    Span::styled(format!("{} ", info.glyph), hi(i).fg(rgb(info.color))),
                    Span::styled(c.label.clone().unwrap_or_else(|| cap(info.name)), hi(i).fg(TEXT)),
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
                item_line(it, if it.forbidden { " (forbidden)" } else { "" }, i == sel)
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
            if let Some(d) = g.dwarves.iter().find(|d| d.id == id && d.ghost) {
                v.push(header(&format!("The ghost of {}", d.name)));
                v.push(plain("Restless, and frightening everyone it".into(), detail_col));
                v.push(plain("drifts near. Bury the body in a coffin".into(), detail_col));
                v.push(plain("(b, y) or carve a memorial slab (b, g).".into(), detail_col));
            } else if let Some(d) = g.dwarves.iter().find(|d| d.id == id) {
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
                v.push(header(&c.label.clone().unwrap_or_else(|| cap(info.name))));
                if let Some(f) = c.figure.and_then(|f| g.overworld.figures.get(f as usize)) {
                    v.push(plain(format!("Born in the year {}; {} kills in the legends.", f.born, f.kills), Color::Rgb(255, 200, 120)));
                }
                if let Some(desc) = &c.desc {
                    v.push(plain(desc.clone(), Color::Rgb(210, 160, 255)));
                }
                let state = if c.body.unconscious() {
                    "Unconscious"
                } else if c.tame {
                    "Tame"
                } else if c.leaving {
                    "Leaving the area"
                } else if c.hostile() {
                    if c.siege { "Hostile (siege)" } else { "Hostile" }
                } else {
                    "Peaceful"
                };
                v.push(plain(state.into(), if c.hostile() { Color::Rgb(255, 100, 90) } else { detail_col }));
                if let Some(mood) = g.wild_mood(c) {
                    v.push(plain(mood, Color::Rgb(255, 190, 110)));
                }
                if let Some(w) = c.weapon.and_then(|id| g.items.get(&id)) {
                    v.push(plain(format!("Wields {}", w.name()), detail_col));
                }
                for a in c.armor.iter().filter_map(|id| g.items.get(id)) {
                    v.push(plain(format!("Wears {}", a.name()), detail_col));
                }
                if c.tame {
                    let sex = if c.female { "female" } else { "male" };
                    let age = if c.adult { "adult" } else { "young" };
                    v.push(plain(format!("{} {}{}", cap(age), sex, if c.pregnant > 0 { ", expecting" } else { "" }), detail_col));
                    if let Some(z) = c.pasture.and_then(|z| g.zones.get(&z)) {
                        v.push(plain(format!("Kept in a pasture of {} tiles", z.tiles.len()), detail_col));
                    }
                    if let Some(o) = c.owner.and_then(|o| g.dwarves.iter().find(|d| d.id == o)) {
                        v.push(plain(format!("War dog of {}", o.name), detail_col));
                    }
                    if c.slaughter {
                        v.push(plain("Marked for slaughter".into(), Color::Rgb(220, 140, 120)));
                    }
                    v.push(plain("p pasture · b slaughter".into(), MUTED));
                    if c.species == crate::creature::Species::Dog {
                        v.push(plain("w war dog of the next soldier".into(), MUTED));
                    }
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
                if matches!(b.kind, BuildingKind::Coffin | BuildingKind::Slab) {
                    let who = b.occupant.and_then(|o| g.dwarves.iter().find(|d| d.id == o)).map(|d| d.name.clone());
                    v.push(plain(
                        match (who, b.kind) {
                            (Some(n), BuildingKind::Coffin) => format!("Here lies {}", n),
                            (Some(n), _) => format!("In memory of {}", n),
                            (None, _) => "Not yet in use".into(),
                        },
                        detail_col,
                    ));
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
                if it.dump {
                    v.push(plain("Marked for the garbage dump".into(), Color::Rgb(200, 170, 90)));
                }
                if it.forbidden {
                    v.push(plain("Forbidden: nobody will touch it".into(), Color::Rgb(230, 120, 120)));
                }
                if it.kind == ItemKind::Cage && it.label.is_some() {
                    v.push(plain("t: tame what's inside".into(), KEY));
                }
                v.push(plain("d: dump this · D: dump everything here".into(), MUTED));
                v.push(plain("f: forbid/allow · F: forbid everything here".into(), MUTED));
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
        keep += (t.magma > 0) as usize;
        keep += t.aquifer as usize;
        keep += (t.smooth > 0) as usize;
        keep += (t.fire > 0) as usize;
        keep += (t.miasma > 0) as usize;
        keep += (t.desig != Desig::None) as usize;
        keep += g.construction_at(p).is_some() as usize;
        keep += (g.burrows.iter().any(|b| t.burrows & b.mask() != 0) || t.traffic != 0) as usize;
        if let Some(z) = g.zones.get(&t.zone) {
            // Room zones add a value line under their name.
            keep += 1 + (z.kind.holds_furniture() && z.kind != ZoneKind::Meeting) as usize;
        }
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
        4 => {
            v.push(header("What the fortress is talking about"));
            if g.gossip.is_empty() {
                v.push(plain("Nothing worth repeating yet.".into(), MUTED));
            }
            for (t, s) in g.gossip.iter().rev() {
                let day = match g.tick.saturating_sub(*t) / crate::game::DAY {
                    0 => "today".to_string(),
                    1 => "yesterday".to_string(),
                    n => format!("{} days ago", n),
                };
                v.push(Line::from(vec![Span::styled(format!("{:<12}", day), Style::default().fg(MUTED)), Span::styled(s.clone(), Style::default().fg(TEXT))]));
            }
        }
        0 => {
            let civ = g.civ.map_or("no civilization".to_string(), |c| format!("the {}", g.overworld.civ(c).name));
            v.push(header(&format!("{}, an outpost of {}", g.fortress_name, civ)));
            v.push(plain(g.date_string(), TEXT));
            v.push(plain(format!("Fortress wealth ☼{}", g.wealth()), Color::Rgb(255, 215, 60)));
            if !g.milestones.is_empty() {
                v.push(Line::default());
                v.push(header("Milestones"));
                for (_, text, _) in &g.milestones {
                    v.push(plain(format!("☼ {}", text), Color::Rgb(230, 200, 110)));
                }
            }
            let feats = crate::terrain::feature_names(g.world.features);
            if !feats.is_empty() {
                v.push(plain(format!("The land: {}", feats.join(", ")), Color::Rgb(170, 220, 170)));
            }
            v.push(plain(format!("Map {}×{}, {} deep (M to grow it)", g.world.w, g.world.h, g.world.d), MUTED));
            if let Some(n) = g.necromancer.as_ref().filter(|n| n.known && n.alive) {
                v.push(plain(format!("The necromancer {} dwells in {}. Bury your dead.", n.name, n.tower), Color::Rgb(190, 120, 230)));
            }
            v.push(Line::default());

            let food = g.items.values().filter(|it| it.is_food() && it.kind != ItemKind::Drink && it.kind != ItemKind::Seed).count();
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
                    Task::Idle | Task::Wander { .. } => match g.zone_kind_at(d.pos) {
                        Some(ZoneKind::Temple) => "Praying",
                        Some(ZoneKind::Tavern) => "In the tavern",
                        _ if g.in_meeting_area(d.pos) => "At the meeting area",
                        _ => "Idle",
                    },
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
            v.push(Line::default());
            let avg = if alive.is_empty() { 100 } else { alive.iter().map(|d| d.focus()).sum::<i32>() / alive.len() as i32 };
            v.push(header(&format!("Focus: {}% on average", avg)));
            let mut unmet: Vec<(crate::dwarf::NeedKind, usize)> = Vec::new();
            for d in &alive {
                for n in d.needs.iter().filter(|n| n.level <= crate::dwarf::NEED_UNMET) {
                    match unmet.iter_mut().find(|u| u.0 == n.kind) {
                        Some(u) => u.1 += 1,
                        None => unmet.push((n.kind, 1)),
                    }
                }
            }
            unmet.sort_by_key(|u| std::cmp::Reverse(u.1));
            if unmet.is_empty() {
                v.push(plain("  Everyone's needs are being met".into(), MUTED));
            }
            for (k, n) in unmet {
                let hint = match k {
                    crate::dwarf::NeedKind::Pray if !g.zones.values().any(|z| z.kind == ZoneKind::Temple) => " (no temple: i, t)",
                    crate::dwarf::NeedKind::Party if !g.zones.values().any(|z| z.kind == ZoneKind::Tavern) => " (no tavern: i, v)",
                    _ => "",
                };
                v.push(plain(format!("  {:>3}  unable to {}{}", n, k.verb(), hint), Color::Rgb(230, 200, 60)));
            }
            let mut herd: Vec<(&'static str, usize)> = Vec::new();
            for c in g.creatures.values().filter(|c| c.tame && c.alive) {
                match herd.iter_mut().find(|h| h.0 == c.species.info().plural) {
                    Some(h) => h.1 += 1,
                    None => herd.push((c.species.info().plural, 1)),
                }
            }
            if !herd.is_empty() {
                v.push(Line::default());
                v.push(header("Animals"));
                for (name, n) in herd {
                    v.push(plain(format!("  {:>3}  {}", n, name), TEXT));
                }
            }
            let hostiles = g.hostile_count();
            if hostiles > 0 {
                v.push(Line::default());
                v.push(plain(format!("⚠ {} hostile creatures on the map", hostiles), Color::Rgb(255, 90, 90)));
            }
            let ghosts = g.dwarves.iter().filter(|d| d.ghost).count();
            let unburied = g.dwarves.iter().filter(|d| !d.alive && !d.rested && !d.ghost).count();
            if ghosts > 0 {
                let who = if ghosts == 1 { "A ghost haunts".to_string() } else { format!("{} ghosts haunt", ghosts) };
                v.push(plain(format!("⚠ {} the fortress (coffins: b, y; slabs: b, g)", who), Color::Rgb(170, 200, 255)));
            }
            if unburied > 0 {
                v.push(plain(format!("  {} dead await burial", unburied), Color::Rgb(230, 200, 60)));
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
                if d.body.infected() {
                    flags.push("infected wound".into());
                } else if d.body.dirty {
                    flags.push("dirty wound".into());
                }
                if d.body.splint.iter().any(|s| *s) {
                    flags.push("splinted".into());
                }
                if d.body.crutch {
                    flags.push("on a crutch".into());
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
                    let (state, col) = if d.ghost {
                        ("a restless ghost!", Color::Rgb(170, 200, 255))
                    } else if d.rested {
                        ("at rest", MUTED)
                    } else {
                        ("unburied", Color::Rgb(230, 200, 60))
                    };
                    v.push(plain(format!("  {:<24}{}", d.name, state), col));
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
    let w = area.width.saturating_sub(4).min(40 + 4 * (LABORS.len() as u16 + 1));
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
    let names = ["1 Overview", "2 Stocks", "3 Health", "4 Dwarves", "5 Gossip"];
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

const BANNER: [&str; 5] = [
    "█▀▀▄ █   █ ▄▀▀▄ █▀▀▄ █▀▀▀   ▄▀▀▀ █    ▄▀▀▄ █▄  █ █▀▀▀",
    "█  █ █ ▄ █ █▄▄█ █▄▄▀ █▄▄    █    █    █  █ █ ▀▄█ █▄▄ ",
    "█  █ ██▀██ █  █ █ ▀▄ █      █    █    █  █ █   █ █   ",
    "█▄▄▀ █   █ █  █ █  █ █      ▀▄▄▄ █▄▄▄ ▀▄▄▀ █   █ █▄▄▄",
    "",
];

const TIPS: [&str; 10] = [
    "Dwarves who sleep on the floor wake up grumpy.",
    "A meeting area keeps idle dwarves out of the corridors.",
    "Eating raw stoutcaps leaves spawn to plant again.",
    "Goblins come from the nearest dark fortress. Closer means sooner.",
    "Locked doors stop everyone, until goblins break them down.",
    "A dwarf in a strange mood will not be denied.",
    "Fortifications stop walkers but let water through.",
    "Specialists never haul. Somebody has to.",
    "Every artifact has a name, and the world will remember it.",
    "Diagonal steps slip between two walls.",
];

/// Paint a quiet field of stone behind the menus.
fn backdrop(f: &mut Frame, area: Rect) {
    let buf = f.buffer_mut();
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let h = hash2(x as i32 * 7, y as i32 * 13);
            let (ch, c) = match h % 23 {
                0 => ('▲', Color::Rgb(70, 64, 58)),
                1 => ('·', Color::Rgb(55, 55, 60)),
                2 => ('£', Color::Rgb(90, 70, 40)),
                3 => ('░', Color::Rgb(40, 38, 36)),
                _ => (' ', BLACK),
            };
            buf.get_mut(x, y).set_char(ch).set_fg(c).set_bg(Color::Rgb(8, 8, 12));
        }
    }
}

fn centered(area: Rect, w: u16, h: u16) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    Rect::new(area.x + (area.width - w) / 2, area.y + (area.height - h) / 2, w, h)
}

fn draw_title(f: &mut Frame, app: &App, sel: usize) {
    let area = f.size();
    let t = crate::title::now();
    let bw = BANNER[0].chars().count() as i32;
    let keep = Rect::new((((area.width as i32 - bw).max(0) / 2) as u16).saturating_sub(1), 0, bw as u16 + 2, 8).intersection(Rect::new(0, 0, area.width, area.height));
    let sky = crate::title::draw(f.buffer_mut(), area, t, keep);
    // The name across the sky, with a glint passing over it.
    let bx = area.x as i32 + (area.width as i32 - bw).max(0) / 2;
    let sweep = (t % (bw as u64 + 90)) as i32 - 10;
    let buf = f.buffer_mut();
    for (row, l) in BANNER.iter().take(4).enumerate() {
        let y = area.y + 1 + row as u16;
        if y >= area.y + area.height {
            break;
        }
        for (i, ch) in l.chars().enumerate() {
            let x = bx + i as i32;
            if ch == ' ' || x < area.x as i32 || x >= (area.x + area.width) as i32 {
                continue;
            }
            let d = (i as i32 - row as i32 - sweep).abs();
            let base = [(255, 215, 90), (250, 195, 70), (235, 170, 50), (210, 140, 40)][row];
            let c = if d < 3 { (255, 250, 220) } else if d < 6 { (255, 232, 160) } else { base };
            buf.get_mut(x as u16, y).set_char(ch).set_fg(rgb(c)).set_style(Style::default().add_modifier(Modifier::BOLD));
        }
    }
    let tag = "a fortress in the deep places of the world";
    let ty = area.y + 6;
    if ty < area.y + sky.saturating_sub(1).max(6) {
        let tx = area.x + area.width.saturating_sub(tag.len() as u16) / 2;
        buf.set_string(tx, ty, tag, Style::default().fg(Color::Rgb(170, 160, 200)));
    }

    let saves = app.saves();
    let mut v: Vec<Line> = vec![Line::default()];
    for (i, item) in TITLE_MENU.iter().enumerate() {
        let detail = match i {
            0 => Some("forge a world and its history".to_string()),
            1 => saves.first().map(|m| format!("{} ({})", m.fortress, m.date)),
            2 if !saves.is_empty() => Some(format!("{} saved", saves.len())),
            3 => Some("the overseer plays a fortress for you".to_string()),
            4 => Some("until next time".to_string()),
            _ => None,
        };
        // Same width on every line so the centered menu lines up.
        let detail: String = detail.unwrap_or_default().chars().take(44).collect();
        let usable = !matches!(i, 1 | 2) || !saves.is_empty();
        let st = if i == sel {
            Style::default().fg(BLACK).bg(KEY).add_modifier(Modifier::BOLD)
        } else if usable {
            Style::default().fg(TEXT)
        } else {
            Style::default().fg(Color::Rgb(80, 80, 90))
        };
        let mark = if i == sel { ["☼ ", " ☼"][((t / 15) % 2) as usize] } else { "  " };
        v.push(Line::from(vec![
            Span::styled(mark.to_string(), Style::default().fg(Color::Rgb(255, 215, 60))),
            Span::styled(format!(" {:<11}", item), st),
            Span::styled(format!(" {:<44}", detail), Style::default().fg(if i == sel { TEXT } else { MUTED })),
        ]));
    }
    v.push(Line::default());
    // A line of lore, typed out and turned over every few seconds.
    let n = crate::title::LINES.len() as u64;
    let line = crate::title::LINES[((t / 240) % n) as usize];
    let typed: String = line.chars().take(((t % 240) as usize) * 2 / 3).collect();
    let cursor = if typed.len() < line.len() && (t / 8) % 2 == 0 { "▌" } else { " " };
    v.push(plain(format!("{:<60}", format!("{}{}", typed, cursor)), Color::Rgb(200, 180, 120)));
    v.push(Line::default());
    if let Some(fl) = &app.flash {
        v.push(plain(fl.clone(), Color::Rgb(240, 150, 110)));
    } else {
        v.push(plain("↑↓ choose · Enter · Esc quits".into(), MUTED));
    }
    let h = v.len() as u16 + 2;
    let w = 66.min(area.width);
    let top = (area.y + sky + 1).min(area.y + area.height.saturating_sub(h));
    let r = Rect::new(area.x + (area.width - w) / 2, top, w, h.min(area.height));
    for i in 0..TITLE_MENU.len() {
        add_target(Rect::new(r.x + 1, r.y + 2 + i as u16, r.width.saturating_sub(2), 1), Target::Row(i));
    }
    f.render_widget(Clear, r);
    f.render_widget(
        Paragraph::new(v).wrap(Wrap { trim: false }).alignment(ratatui::layout::Alignment::Center).style(Style::default().bg(Color::Rgb(12, 12, 20))).block(
            Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Rgb(160, 130, 70))),
        ),
        r,
    );
}

fn draw_load_menu(f: &mut Frame, app: &App, sel: usize, confirm: bool) {
    let area = f.size();
    backdrop(f, area);
    let saves = app.saves();
    let mut v: Vec<Line> = vec![Line::default()];
    for (i, m) in saves.iter().enumerate() {
        let st = if i == sel { Style::default().fg(BLACK).bg(KEY) } else { Style::default().fg(TEXT) };
        let tag = if m.autosave { " (autosave)" } else { "" };
        v.push(Line::from(vec![
            Span::styled(format!(" {:<18}", format!("{}{}", m.fortress, tag)), st.add_modifier(Modifier::BOLD)),
            Span::styled(format!(" {:<34}", m.date), st),
            Span::styled(format!(" {:>3} dwarves ", m.dwarves), st),
            Span::styled(format!(" {:<12}", saves::ago(m.saved_unix)), st),
        ]));
        v.push(plain(format!("   {}", m.world), MUTED));
    }
    v.push(Line::default());
    if confirm {
        let name = saves.get(sel).map_or(String::new(), |m| m.file.clone());
        v.push(plain(format!("Delete {}? This can't be undone. (y/n)", name), Color::Rgb(255, 120, 90)));
    } else if let Some(fl) = &app.flash {
        v.push(plain(fl.clone(), Color::Rgb(240, 150, 110)));
    }
    let r = centered(area, 90, (v.len() as u16 + 2).max(8));
    f.render_widget(Clear, r);
    f.render_widget(
        Paragraph::new(v).style(Style::default().bg(Color::Rgb(12, 12, 20))).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(KEY))
                .title(Span::styled(" Load game ", Style::default().fg(KEY)))
                .title_bottom(Span::styled(" ↑↓ choose · Enter load · x delete · Esc back ", Style::default().fg(MUTED))),
        ),
        r,
    );
}

use crate::saves;

fn draw_loading(f: &mut Frame, app: &App) {
    let area = f.size();
    backdrop(f, area);
    let (title, stage, frac, secs) = match &app.job {
        Some(j) => {
            let (s, p) = j.status();
            (j.title.clone(), s, p, j.started.elapsed().as_secs_f32())
        }
        None => ("Working".to_string(), String::new(), 1.0, 0.0),
    };
    const SPIN: [char; 8] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];
    let spin = SPIN[(secs * 10.0) as usize % SPIN.len()];
    let width = 44usize;
    let filled = ((frac * width as f32).round() as usize).min(width);
    let tip = TIPS[(secs / 4.0) as usize % TIPS.len()];
    let v = vec![
        Line::default(),
        Line::from(vec![
            Span::styled(format!("{} ", spin), Style::default().fg(KEY)),
            Span::styled(format!("{}...", title), Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
        ]),
        Line::default(),
        Line::from(vec![
            Span::styled("█".repeat(filled), Style::default().fg(Color::Rgb(255, 200, 80))),
            Span::styled("░".repeat(width - filled), Style::default().fg(Color::Rgb(60, 55, 50))),
            Span::styled(format!(" {:>3}%", (frac * 100.0) as u32), Style::default().fg(MUTED)),
        ]),
        plain(stage, Color::Rgb(180, 180, 190)),
        Line::default(),
        plain(format!("Tip: {}", tip), MUTED),
    ];
    let r = centered(area, 60, 10);
    f.render_widget(Clear, r);
    f.render_widget(
        Paragraph::new(v).alignment(ratatui::layout::Alignment::Center).style(Style::default().bg(Color::Rgb(12, 12, 20))).block(
            Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Rgb(120, 100, 60))),
        ),
        r,
    );
}

/// Choosing how much land to add on each side.
fn draw_grow(f: &mut Frame, area: Rect, g: &Game, (n, e, s, w): (i32, i32, i32, i32)) {
    let (nw, nh) = (g.world.w + e + w, g.world.h + n + s);
    let edge = |label: &str, by: i32| {
        Line::from(vec![
            Span::styled(format!("   {:<6}", label), Style::default().fg(TEXT)),
            Span::styled(if by > 0 { format!("+{} tiles", by) } else { "—".into() }, Style::default().fg(if by > 0 { Color::Rgb(120, 220, 120) } else { MUTED })),
        ])
    };
    let v = vec![
        plain(format!(" The map is {}×{}. New land continues the old.", g.world.w, g.world.h), TEXT),
        Line::default(),
        edge("North", n),
        edge("East", e),
        edge("South", s),
        edge("West", w),
        Line::default(),
        plain(format!(" New size: {}×{}, {} deep (at most 320×256)", nw, nh, g.world.d), Color::Rgb(255, 215, 60)),
        Line::default(),
        plain(" ↑ → ↓ ← add 16 tiles on that side".into(), MUTED),
        plain(" Shift+arrow takes 16 back".into(), MUTED),
        plain(" Enter grow it · Esc cancel".into(), MUTED),
    ];
    let h = v.len() as u16 + 2;
    popup(f, area, 60, h, " Grow the map ", v);
}

/// Filters for the reports screen.
pub const REPORT_FILTERS: [&str; 5] = ["All", "News", "Combat", "Deaths", "People"];

fn report_kind(text: &str, color: (u8, u8, u8)) -> [bool; 5] {
    let has = |words: &[&str]| words.iter().any(|w| text.contains(w));
    let death = color == (255, 60, 60) || has(&["has died", "bled to death", "starved to death", "struck down", "has been slain", "fell on the expedition"]);
    let blow = has(&[" in the head", " in the upper body", " in the lower body", " in the left ", " in the right ", "blocks", "deflects", "misses"]);
    let combat = blow || death || has(&["siege", "goblin", "flees", "fleeing", "attack", "army", "breathes fire", "cage trap", "raises the dead", "duel"]);
    let people = has(&["arrived", "born", "married", "mood", "artifact", "known as", "elected", "made", "hero", "visitor", "bard", "stranger", "joined", "come of age", "festival", "feast"]) && !blow;
    let routine = blow || has(&["has made ", "has been completed", "Work order", "waiting to be built", "needs ", "It has started", "skies have"]);
    [true, !routine, combat, death, people]
}

/// Every announcement still on record, newest at the bottom.
fn draw_reports(f: &mut Frame, area: Rect, g: &Game, filter: usize, scroll: usize) {
    let r = Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), area.height.saturating_sub(2));
    f.render_widget(Clear, r);
    let tabs: Vec<Span> = REPORT_FILTERS
        .iter()
        .enumerate()
        .flat_map(|(i, n)| {
            let st = if i == filter { Style::default().fg(BLACK).bg(KEY) } else { Style::default().fg(TEXT) };
            [Span::styled(format!(" {} {} ", i + 1, n), st), Span::raw(" ")]
        })
        .collect();
    let shown: Vec<&crate::game::LogEntry> = g.log.iter().filter(|e| report_kind(&e.text, e.color)[filter]).collect();
    let rows = r.height.saturating_sub(4) as usize;
    let max_scroll = shown.len().saturating_sub(rows);
    let scroll = scroll.min(max_scroll);
    let end = shown.len() - scroll;
    let start = end.saturating_sub(rows);
    let mut v: Vec<Line<'static>> = vec![Line::from(tabs), Line::default()];
    let mut last_day = u64::MAX;
    for e in &shown[start..end] {
        let day = e.tick / DAY;
        let date = if day != last_day { g.short_date(e.tick) } else { String::new() };
        last_day = day;
        v.push(Line::from(vec![Span::styled(format!("{:<16}", date), Style::default().fg(MUTED)), Span::styled(e.text.clone(), Style::default().fg(rgb(e.color)))]));
    }
    let title = format!(
        " Reports: {} of {} ({}) · ↑↓ PgUp PgDn scroll · Tab/1-5 filter · Esc back ",
        end,
        shown.len(),
        if scroll == 0 { "newest at the bottom".to_string() } else { format!("{} back", scroll) }
    );
    f.render_widget(
        Paragraph::new(v).style(Style::default().bg(Color::Rgb(12, 12, 20))).block(
            Block::default().borders(Borders::ALL).border_style(Style::default().fg(KEY)).title(Span::styled(title, Style::default().fg(KEY))),
        ),
        r,
    );
    for i in 0..REPORT_FILTERS.len() {
        let x = r.x + 1 + REPORT_FILTERS[..i].iter().map(|n| n.len() as u16 + 5).sum::<u16>();
        add_target(Rect::new(x, r.y + 1, REPORT_FILTERS[i].len() as u16 + 4, 1), Target::Key(crossterm::event::KeyCode::Char((b'1' + i as u8) as char)));
    }
}

/// The overseer's mind as a little network: senses on the left fire into
/// the ambitions on the right, green pulling towards, red pushing away.
fn draw_brain(f: &mut Frame, area: Rect, g: &Game, sel: usize) {
    use crate::game::{Ambition, Sense};
    use ratatui::symbols::Marker;
    use ratatui::widgets::canvas::{Canvas, Circle, Line as CLine};
    let Some(brain) = g.overseer_brain() else { return };
    let Some(o) = &g.overseer else { return };
    let chosen = Ambition::ALL[sel.min(Ambition::ALL.len() - 1)];
    let r = Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), area.height.saturating_sub(2));
    f.render_widget(Clear, r);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(120, 190, 255)))
        .title(Span::styled(format!(" The mind of {} ({}) ", o.title(), o.style.name()), Style::default().fg(Color::Rgb(120, 190, 255))));
    let inner = block.inner(r);
    f.render_widget(block, r);
    let cols = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Min(40), Constraint::Length(44)]).split(inner);
    // Where each node sits on a 0..100 canvas.
    let sense_y = |i: usize| 95.0 - i as f64 * 90.0 / (Sense::ALL.len() - 1) as f64;
    let amb_y = |i: usize| 92.0 - i as f64 * 84.0 / (Ambition::ALL.len() - 1) as f64;
    let (sx, ax) = (24.0, 74.0);
    let tick = g.tick;
    let side = brain.clone();
    let canvas = Canvas::default()
        .marker(Marker::Braille)
        .x_bounds([0.0, 100.0])
        .y_bounds([0.0, 100.0])
        .background_color(Color::Rgb(8, 10, 18))
        .paint(move |ctx| {
            // The links: brighter for stronger, the chosen ambition's in full.
            for (ai, &a) in Ambition::ALL.iter().enumerate() {
                for &(s, v) in brain.links_of(a) {
                    let si = Sense::ALL.iter().position(|&x| x == s).unwrap();
                    let strength = (v.abs() / 2.0).clamp(0.15, 1.0);
                    let focus = if a == chosen { 1.0 } else { 0.35 };
                    let k = strength * focus;
                    let color = if v >= 0.0 {
                        Color::Rgb((40.0 * k) as u8, (90.0 + 165.0 * k) as u8, (60.0 + 60.0 * k) as u8)
                    } else {
                        Color::Rgb((90.0 + 165.0 * k) as u8, (40.0 * k) as u8, (40.0 * k) as u8)
                    };
                    ctx.draw(&CLine { x1: sx + 2.0, y1: sense_y(si), x2: ax - 2.0, y2: amb_y(ai), color });
                }
            }
            ctx.layer();
            // Senses: lit by how strongly they fire.
            for &(s, level) in brain.senses.iter() {
                let i = Sense::ALL.iter().position(|&x| x == s).unwrap_or(0);
                let c = (60.0 + 195.0 * level) as u8;
                ctx.draw(&Circle { x: sx, y: sense_y(i), radius: 1.6, color: Color::Rgb(c, c, (c as f64 * 0.6) as u8) });
                ctx.print(1.0, sense_y(i), Span::styled(format!("{:<11}", s.name()), Style::default().fg(Color::Rgb(c.max(120), c.max(120), 150))));
            }
            // Ambitions: sized by their odds, the current one pulsing gold.
            for (i, &a) in Ambition::ALL.iter().enumerate() {
                let p = brain.odds_of(a);
                let now = o.ambition == Some(a);
                let pulse = if now { 1.0 + ((tick / 8) % 6) as f64 * 0.25 } else { 1.0 };
                let color = if now {
                    Color::Rgb(255, 215, 60)
                } else if a == chosen {
                    Color::Rgb(150, 210, 255)
                } else {
                    Color::Rgb(90, 120, 170)
                };
                ctx.draw(&Circle { x: ax, y: amb_y(i), radius: (1.2 + p * 6.0) * pulse, color });
                let nudge = o.nudge(a);
                let tag = match nudge {
                    0 => String::new(),
                    n if n > 0 => format!(" {}", "▲".repeat(n as usize)),
                    n => format!(" {}", "▼".repeat((-n) as usize)),
                };
                ctx.print(
                    ax + 5.0,
                    amb_y(i),
                    Span::styled(format!("{:<10}{:>3.0}%{}", a.name(), p * 100.0, tag), Style::default().fg(color).add_modifier(if now { Modifier::BOLD } else { Modifier::empty() })),
                );
            }
        });
    f.render_widget(canvas, cols[0]);
    // The side panel: the chosen ambition, what moves it, and your hand on it.
    let mut v: Vec<Line<'static>> = Vec::new();
    let now = o.ambition.map_or("nothing yet".to_string(), |a| {
        format!("{} ({} days)", a.name(), g.tick.saturating_sub(o.ambition_since) / crate::game::DAY)
    });
    v.push(plain(format!(" Now: {}", now), Color::Rgb(255, 215, 60)));
    v.push(Line::default());
    v.push(Line::from(Span::styled(format!(" {}", chosen.name()), Style::default().fg(Color::Rgb(150, 210, 255)).add_modifier(Modifier::BOLD))));
    v.push(plain(format!(" {}", chosen.announce().split(": ").nth(1).unwrap_or(chosen.announce())), MUTED));
    v.push(plain(format!(" Chance it's next: {:.0}%", side.odds_of(chosen) * 100.0), TEXT));
    v.push(Line::default());
    let mut links: Vec<(Sense, f64)> = side.links_of(chosen).to_vec();
    links.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()));
    for (s, val) in links {
        let (sign, col) = if val >= 0.0 { ("+", Color::Rgb(110, 220, 120)) } else { ("−", Color::Rgb(235, 110, 100)) };
        let shown = if val <= -8.0 { "blocked".to_string() } else { format!("{}{:.1}", sign, val.abs()) };
        v.push(Line::from(vec![Span::styled(format!(" {:<11}", s.name()), Style::default().fg(TEXT)), Span::styled(shown, Style::default().fg(col))]));
    }
    v.push(Line::default());
    let n = o.nudge(chosen);
    let bar: String = (-3..=3).map(|i| if i == 0 { '│' } else if (n > 0 && i > 0 && i <= n) || (n < 0 && i < 0 && i >= n) { '●' } else { '·' }).collect();
    v.push(plain(format!(" Your nudge  ◀ {} ▶", bar), Color::Rgb(255, 215, 60)));
    v.push(Line::default());
    v.push(plain(" ↑↓ pick an ambition".into(), MUTED));
    v.push(plain(" ←→ nudge it (−3..+3)".into(), MUTED));
    v.push(plain(" f  make it do this now".into(), MUTED));
    v.push(plain(" r  clear all nudges".into(), MUTED));
    v.push(plain(" Space pause · Esc back".into(), MUTED));
    f.render_widget(Paragraph::new(v).wrap(Wrap { trim: false }).style(Style::default().bg(Color::Rgb(12, 12, 20))), cols[1]);
}

/// The overseer: who's playing, in what style, and what it's been doing.
fn draw_autoplay(f: &mut Frame, area: Rect, g: &Game, sel: usize) {
    use crate::game::Style as Pace;
    let mut v: Vec<Line<'static>> = vec![plain(" The overseer plays the fortress with the same commands you have.".into(), MUTED), Line::default()];
    let cur = g.overseer.as_ref().map(|o| o.style);
    let mut rows: Vec<(String, String, bool)> = vec![("Off".into(), "you play".into(), cur.is_none())];
    for s in Pace::ALL {
        rows.push((s.name().into(), s.blurb().into(), cur == Some(s)));
    }
    for (i, (name, blurb, on)) in rows.into_iter().enumerate() {
        let hi = if i == sel { Style::default().bg(Color::Rgb(50, 60, 120)) } else { Style::default() };
        v.push(Line::from(vec![
            Span::styled(if on { " ► " } else { "   " }, hi.fg(KEY)),
            Span::styled(format!("{:<10}", name), hi.fg(TEXT).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{:<50}", blurb), hi.fg(MUTED)),
        ]));
    }
    if let Some(o) = &g.overseer {
        v.push(Line::default());
        v.push(plain(format!(" The overseer: {}  (r: another)", o.title()), Color::Rgb(150, 210, 255)));
        for t in &o.traits {
            v.push(plain(format!("   {}: {}", t.name(), t.blurb()), MUTED));
        }
        if let Some(a) = o.ambition {
            let days = g.tick.saturating_sub(o.ambition_since) / crate::game::DAY;
            v.push(Line::default());
            v.push(plain(format!(" Its heart is set on: {} (for {} days). {}", a.name(), days, a.announce().split(": ").nth(1).unwrap_or("")), Color::Rgb(150, 210, 255)));
            let path: Vec<&str> = o.ambitions.iter().rev().take(8).map(|x| x.1.name()).collect::<Vec<_>>().into_iter().rev().collect();
            v.push(plain(format!(" So far: {}", path.join(" → ")), MUTED));
        }
        v.push(Line::default());
        v.push(header("What it's been doing"));
        for (_, n) in o.notes.iter().rev().take(8) {
            v.push(plain(format!(" {}", n), TEXT));
        }
    }
    v.push(Line::default());
    v.push(plain(" ↑↓ choose · Enter · b its brain · Esc back".into(), MUTED));
    let h = v.len() as u16 + 2;
    let r = popup(f, area, 72, h, " Autoplay ", v);
    for i in 0..=Pace::ALL.len() {
        add_target(Rect::new(r.x + 1, r.y + 3 + i as u16, r.width.saturating_sub(2), 1), Target::Row(i));
    }
}

/// Everything that can be built, in two columns.
fn draw_build_menu(f: &mut Frame, area: Rect) {
    // Two columns, as wide as the screen allows.
    let w = area.width.min(104);
    let col = ((w.saturating_sub(4) / 2) as usize).saturating_sub(3).max(16);
    let all: Vec<Line<'static>> = BuildingKind::ALL
        .iter()
        .map(|b| {
            Line::from(vec![
                Span::styled(format!(" {} ", b.key()), Style::default().fg(KEY)),
                Span::styled(format!("{:<col$}", format!("{} ({})", b.name(), b.needs_desc()).chars().take(col - 1).collect::<String>(), col = col), Style::default().fg(TEXT)),
            ])
        })
        .collect();
    let half = (all.len() + 1) / 2;
    let mut v: Vec<Line<'static>> = vec![key_line("C", "Constructions: walls, floors, stairs, ramps...", false), Line::default()];
    for i in 0..half {
        let mut spans = all[i].spans.clone();
        if let Some(r) = all.get(i + half) {
            spans.extend(r.spans.clone());
        }
        v.push(Line::from(spans));
    }
    v.push(Line::default());
    v.push(plain(" Press a key to place it · Esc back".into(), MUTED));
    let h = v.len() as u16 + 2;
    popup(f, area, w, h, " Build ", v);
}

/// Where a squad could go, with the danger against their strength.
fn draw_expedition(f: &mut Frame, area: Rect, g: &Game, squad: usize, sel: usize) {
    let Some(s) = g.squads.get(squad) else { return };
    let strength = g.squad_strength(&s.members);
    let mut v: Vec<Line<'static>> = vec![
        plain(format!(" {} · {} dwarves · strength {}", s.name, s.members.len(), strength), TEXT),
        plain(" They leave the map; loot, wounds and stories come back with them.".into(), MUTED),
        Line::default(),
    ];
    for (i, d) in g.destinations().iter().enumerate() {
        let hi = if i == sel { Style::default().bg(Color::Rgb(50, 60, 120)) } else { Style::default() };
        let kind = match d.goal {
            crate::game::expeditions_goal::Ruin => "explore",
            crate::game::expeditions_goal::Raid => "raid",
            crate::game::expeditions_goal::Beast(_) => "hunt",
        };
        let odds = strength as f64 / (strength as f64 + d.danger as f64 * 3.0);
        let (word, col) = if odds >= 0.65 {
            ("likely", Color::Rgb(110, 220, 110))
        } else if odds >= 0.4 {
            ("risky", Color::Rgb(230, 200, 90))
        } else {
            ("deadly", Color::Rgb(230, 110, 90))
        };
        v.push(Line::from(vec![
            Span::styled(format!(" {:<8}", kind), hi.fg(KEY)),
            Span::styled(format!("{:<44}", d.name.chars().take(43).collect::<String>()), hi.fg(TEXT)),
            Span::styled(format!("{:>3} days  ", d.days()), hi.fg(MUTED)),
            Span::styled(format!("{:<8}", "☠".repeat(d.danger.clamp(1, 8) as usize)), hi.fg(Color::Rgb(230, 110, 90))),
            Span::styled(word.to_string(), hi.fg(col)),
        ]));
    }
    v.push(Line::default());
    v.push(plain(" ↑↓ choose · Enter send them · Esc cancel".into(), MUTED));
    let h = v.len() as u16 + 2;
    let r = popup(f, area, 84, h, " Send on an expedition ", v);
    for i in 0..g.destinations().len() {
        add_target(Rect::new(r.x + 1, r.y + 4 + i as u16, r.width.saturating_sub(2), 1), Target::Row(i));
    }
}

/// A milestone's banner across the top of the screen while it's fresh.
fn draw_banner(f: &mut Frame, area: Rect, g: &Game) {
    let Some((text, at)) = &g.banner else { return };
    if g.tick >= at + crate::game::milestones_banner_ticks() {
        return;
    }
    let line = format!("☼  {}  ☼", text);
    let w = (line.chars().count() as u16 + 6).min(area.width);
    let r = Rect::new(area.x + (area.width - w) / 2, area.y + 2, w, 3);
    let glint = ((g.tick - at) / 4) as usize % (w as usize + 10);
    let spans: Vec<Span> = line
        .chars()
        .enumerate()
        .map(|(i, ch)| {
            let near = (i as i64 - glint as i64).abs() < 3;
            Span::styled(ch.to_string(), Style::default().fg(if near { Color::Rgb(255, 255, 220) } else { Color::Rgb(255, 215, 60) }).add_modifier(Modifier::BOLD))
        })
        .collect();
    f.render_widget(Clear, r);
    f.render_widget(
        Paragraph::new(Line::from(spans)).alignment(ratatui::layout::Alignment::Center).style(Style::default().bg(Color::Rgb(40, 25, 5))).block(
            Block::default().borders(Borders::ALL).border_type(ratatui::widgets::BorderType::Double).border_style(Style::default().fg(Color::Rgb(255, 200, 80))),
        ),
        r,
    );
}

/// A page of the chronicle, like a leaf of parchment.
fn draw_chronicle(f: &mut Frame, area: Rect, g: &Game, idx: usize) {
    let Some(c) = g.chronicles.get(idx) else { return };
    let ink = Color::Rgb(70, 45, 25);
    let paper = Color::Rgb(232, 214, 170);
    let mut v: Vec<Line<'static>> = vec![
        Line::default(),
        Line::from(Span::styled(c.title.clone(), Style::default().fg(Color::Rgb(130, 30, 20)).add_modifier(Modifier::BOLD))).alignment(ratatui::layout::Alignment::Center),
        Line::from(Span::styled(format!("~ the chronicle of {}, year {} ~", g.fortress_name, c.year), Style::default().fg(Color::Rgb(120, 90, 60)))).alignment(ratatui::layout::Alignment::Center),
        Line::default(),
    ];
    for l in &c.lines {
        v.push(Line::from(Span::styled(l.clone(), Style::default().fg(ink))));
        v.push(Line::default());
    }
    let nav = format!(
        "{}  page {} of {}  {}   any other key closes",
        if idx > 0 { "◀" } else { " " },
        idx + 1,
        g.chronicles.len(),
        if idx + 1 < g.chronicles.len() { "▶" } else { " " }
    );
    v.push(Line::from(Span::styled(nav, Style::default().fg(Color::Rgb(120, 90, 60)))).alignment(ratatui::layout::Alignment::Center));
    let w = 76.min(area.width);
    let inner = w.saturating_sub(4).max(1) as usize;
    let rows: usize = v.iter().map(|l| (l.width().max(1) + inner - 1) / inner).sum();
    let h = (rows as u16 + 2).min(area.height);
    let r = centered(area, w, h);
    f.render_widget(Clear, r);
    f.render_widget(
        Paragraph::new(v).wrap(Wrap { trim: true }).style(Style::default().bg(paper)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Double)
                .border_style(Style::default().fg(Color::Rgb(140, 100, 50)).bg(paper))
                .padding(ratatui::widgets::Padding::horizontal(1)),
        ),
        r,
    );
}

/// The list of dwarves who could own a bedroom or office.
fn draw_pick_owner(f: &mut Frame, area: Rect, g: &Game, zone: u32, sel: usize) {
    let Some(z) = g.zones.get(&zone) else { return };
    let kind = z.kind;
    let cands = g.owner_candidates();
    let value = g.area_value(&z.tiles.iter().copied().collect());
    let mut v: Vec<Line<'static>> = Vec::new();
    v.push(plain(
        format!(
            " {} worth {}. {}",
            kind.name(),
            value,
            if kind == ZoneKind::Office { "Nobles want one; post holders work better from one." } else { "Its beds go to its owner." }
        ),
        MUTED,
    ));
    v.push(Line::default());
    let room = (area.height as usize).saturating_sub(9).max(3);
    let rows = cands.len() + 1;
    let first = sel.saturating_sub(room / 2).min(rows.saturating_sub(room));
    for row in first..rows.min(first + room) {
        let hi = if row == sel { Style::default().bg(Color::Rgb(50, 60, 120)) } else { Style::default() };
        if row == 0 {
            v.push(Line::from(vec![
                Span::styled(if z.owner.is_none() { " ► " } else { "   " }, hi.fg(KEY)),
                Span::styled(format!("{:<60}", "(nobody)"), hi.fg(MUTED)),
            ]));
            continue;
        }
        let id = cands[row - 1];
        let Some(d) = g.dwarves.iter().find(|d| d.id == id) else { continue };
        let role = match (d.noble, g.position_of(id)) {
            (Some(n), _) => n.title().to_string(),
            (None, Some(p)) => p.title().to_string(),
            _ => String::new(),
        };
        let has = g.zones.values().find(|o| o.kind == kind && o.owner == Some(id) && o.id != zone).map(|o| {
            format!("has a{} {} worth {}", if kind == ZoneKind::Office { "n" } else { "" }, kind.name().to_lowercase(), g.area_value(&o.tiles.iter().copied().collect()))
        });
        v.push(Line::from(vec![
            Span::styled(if z.owner == Some(id) { " ► " } else { "   " }, hi.fg(KEY)),
            Span::styled(format!("{:<20}", d.name), hi.fg(TEXT)),
            Span::styled(format!("{:<20}", role), hi.fg(Color::Rgb(255, 230, 120))),
            Span::styled(has.unwrap_or_default(), hi.fg(Color::Rgb(240, 160, 100))),
        ]));
    }
    if rows > room {
        v.push(plain(format!(" ({} of {} shown)", room.min(rows), rows), MUTED));
    }
    v.push(Line::default());
    v.push(plain(" ↑↓ choose · Enter assign · Esc cancel   ► owns it now".into(), MUTED));
    let h = v.len() as u16 + 2;
    let r = popup(f, area, 76, h, &format!(" Who owns this {}? ", kind.name().to_lowercase()), v);
    for row in first..rows.min(first + room) {
        add_target(Rect::new(r.x + 1, r.y + 3 + (row - first) as u16, r.width.saturating_sub(2), 1), Target::Row(row));
    }
}

/// The list of dwarves who could take a post, best suited first.
fn draw_appoint(f: &mut Frame, area: Rect, g: &Game, pos: usize, sel: usize) {
    use crate::game::Fit;
    let p = crate::game::Position::ALL[pos];
    let cands = g.candidates(p);
    let holder = g.holder(p).map(|d| d.id);
    let mut v: Vec<Line<'static>> = Vec::new();
    v.push(plain(format!(" {}", p.duty()), MUTED));
    for f in Fit::ALL {
        v.push(Line::from(vec![
            Span::styled(format!("   {:<9} ", f.name()), Style::default().fg(rgb(f.color()))),
            Span::styled(p.effect(f).to_string(), Style::default().fg(MUTED)),
        ]));
    }
    v.push(Line::default());
    // Keep the selection in view on a short screen.
    let room = (area.height as usize).saturating_sub(12).max(3);
    let rows = cands.len() + 1;
    let first = sel.saturating_sub(room / 2).min(rows.saturating_sub(room));
    for row in first..rows.min(first + room) {
        let hi = if row == sel { Style::default().bg(Color::Rgb(50, 60, 120)) } else { Style::default() };
        if row == 0 {
            let cur = holder.is_none();
            v.push(Line::from(vec![
                Span::styled(if cur { " ► " } else { "   " }, hi.fg(KEY)),
                Span::styled(format!("{:<56}", "(leave vacant)"), hi.fg(MUTED)),
            ]));
            continue;
        }
        let id = cands[row - 1];
        let Some(d) = g.dwarves.iter().find(|d| d.id == id) else { continue };
        let (score, notes) = g.post_suitability(p, d);
        let fit = Fit::of(score);
        let other = g.position_of(id).filter(|&q| q != p).map(|q| format!(" [now {}]", q.title().to_lowercase()));
        let mut spans = vec![
            Span::styled(if holder == Some(id) { " ► " } else { "   " }, hi.fg(KEY)),
            Span::styled(format!("{:<20}", d.name), hi.fg(TEXT)),
            Span::styled(format!("{:<9} ", fit.name()), hi.fg(rgb(fit.color()))),
            Span::styled(notes.join(", "), hi.fg(MUTED)),
        ];
        if let Some(o) = other {
            spans.push(Span::styled(o, hi.fg(Color::Rgb(240, 160, 100))));
        }
        v.push(Line::from(spans));
    }
    if rows > room {
        v.push(plain(format!(" ({} of {} shown)", room.min(rows), rows), MUTED));
    }
    v.push(Line::default());
    v.push(plain(" ↑↓ choose · Enter appoint · Esc cancel   ► holds it now".into(), MUTED));
    let h = v.len() as u16 + 2;
    let r = popup(f, area, 72, h, &format!(" Appoint {} ", p.title().to_lowercase()), v);
    for row in first..rows.min(first + room) {
        add_target(Rect::new(r.x + 1, r.y + 6 + (row - first) as u16, r.width.saturating_sub(2), 1), Target::Row(row));
    }
}

fn popup(f: &mut Frame, area: Rect, w: u16, h: u16, title: &str, lines: Vec<Line<'static>>) -> Rect {
    let w = w.min(area.width);
    let h = h.min(area.height);
    let r = Rect::new(area.x + (area.width - w) / 2, area.y + (area.height - h) / 2, w, h);
    add_target(r, Target::Block);
    register_lines(Rect::new(r.x + 1, r.y + 1, r.width.saturating_sub(2), r.height.saturating_sub(2)), &lines, false);
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
    r
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
    #[ignore]
    fn frame_time_by_level() {
        let path = std::env::var("FORT").unwrap();
        let mut g: Game = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        g.after_load();
        let mut app = App::with_game_for_tests(g);
        app.mode = Mode::Normal;
        let mut term = Terminal::new(TestBackend::new(200, 60)).unwrap();
        for z in 0..app.game.world.d {
            app.cursor.z = z;
            let t0 = std::time::Instant::now();
            for _ in 0..5 {
                term.draw(|f| draw(f, &mut app)).unwrap();
            }
            println!("z {:>2}: {:>7.2} ms a frame", z, t0.elapsed().as_secs_f64() * 200.0);
        }
    }

    /// Where a saved fort's simulation spends its time, day by day:
    /// FORT=saves/X.json [DAYS=n].
    #[test]
    #[ignore]
    fn profile_a_save() {
        use std::sync::atomic::Ordering::Relaxed;
        let path = std::env::var("FORT").unwrap();
        let days: u64 = std::env::var("DAYS").ok().and_then(|s| s.parse().ok()).unwrap_or(10);
        let mut g: Game = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        g.after_load();
        println!("{} dwarves, {} creatures, {} items, overseer {:?}", g.dwarves.iter().filter(|d| d.alive).count(), g.creatures.len(), g.items.len(), g.overseer.as_ref().map(|o| o.title()));
        let mut nodes0 = crate::path::SEARCH_NODES.load(Relaxed) + crate::path::ASTAR_NODES.load(Relaxed);
        for day in 0..days {
            g.prof = Some([0.0; 6]);
            let mut worst = (0.0, 0u64);
            for _ in 0..crate::game::DAY {
                let t = std::time::Instant::now();
                g.step();
                let ms = t.elapsed().as_secs_f64() * 1000.0;
                if ms > worst.0 {
                    worst = (ms, g.tick % crate::game::DAY);
                }
            }
            let p = g.prof.unwrap();
            let per = crate::game::DAY as f64;
            let n = crate::path::SEARCH_NODES.load(Relaxed) + crate::path::ASTAR_NODES.load(Relaxed);
            println!(
                "day {:>2}: {:.3} ms/tick (worst {:.1} at {})  water {:.3} overseer {:.3} dwarves {:.3} creatures {:.3} rest {:.3}  nodes/tick {:.0}",
                day,
                p[4] / per,
                worst.0,
                worst.1,
                p[0] / per,
                p[1] / per,
                p[2] / per,
                p[3] / per,
                (p[4] - p[0] - p[1] - p[2] - p[3]) / per,
                (n - nodes0) as f64 / per
            );
            nodes0 = n;
        }
    }

    /// What changing level costs the terminal: bytes written for the frame.
    #[test]
    #[ignore]
    fn bytes_per_level_change() {
        use std::io::Write;
        #[derive(Clone, Default)]
        struct Count(std::rc::Rc<std::cell::Cell<usize>>);
        impl Write for Count {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                self.0.set(self.0.get() + b.len());
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let path = std::env::var("FORT").unwrap();
        let mut g: Game = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        g.after_load();
        let mut app = App::with_game_for_tests(g);
        app.mode = Mode::Normal;
        let n = Count::default();
        let backend = ratatui::backend::CrosstermBackend::new(n.clone());
        let mut term = Terminal::with_options(backend, ratatui::TerminalOptions { viewport: ratatui::Viewport::Fixed(Rect::new(0, 0, 200, 60)) }).unwrap();
        let mut tot = 0;
        for z in (0..app.game.world.d).rev() {
            app.cursor.z = z;
            n.0.set(0);
            term.draw(|f| draw(f, &mut app)).unwrap();
            println!("z {:>2}: {:>7} bytes", z, n.0.get());
            tot += n.0.get();
        }
        println!("average {}", tot / app.game.world.d as usize);
    }

    #[test]
    #[ignore]
    fn show_default_site() {
        let seed: u64 = std::env::var("SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(42);
        let g = Game::new(seed);
        let w = &g.world;
        let e = g.embark;
        eprintln!("seed {} site {:?} {:?} embark {:?} features {:?}", seed, g.site, g.biome, e, crate::terrain::feature_names(w.features));
        for y in (e.y - 12).max(0)..(e.y + 12).min(w.h) {
            let row: String = ((e.x - 30).max(0)..(e.x + 30).min(w.w))
                .map(|x| {
                    let s = w.surface_z(x, y);
                    if (x, y) == (e.x, e.y) { return 'W'; }
                    let t = w.t(Pos::new(x, y, s));
                    if t.water > 0 { '≈' } else if t.kind == TileKind::Tree { '♣' } else if t.kind == TileKind::Ramp { '^' } else { char::from_digit(((s - e.z).rem_euclid(10)) as u32, 10).unwrap() }
                })
                .collect();
            eprintln!("  {}", row);
        }
    }

    /// One site of each biome, embarked and drawn at the surface by the wagon.
    #[test]
    #[ignore]
    fn show_biome_sites() {
        use crate::overworld::{Biome, Overworld};
        let only = std::env::var("BIOME").ok();
        let ow = Overworld::generate(42);
        for b in Biome::ALL {
            if b == Biome::Ocean || only.as_deref().map_or(false, |o| !b.name().contains(o)) {
                continue;
            }
            // A site of this biome, preferably with something different next door.
            let found = (0..ow.h).flat_map(|y| (0..ow.w).map(move |x| (x, y))).filter(|&(x, y)| ow.embarkable(x, y) && ow.cell(x, y).biome == b).max_by_key(|&(x, y)| {
                let mut diff = 0;
                for (dx, dy) in [(1, 0), (0, 1), (1, 1)] {
                    if x + dx < ow.w && y + dy < ow.h && ow.cell(x + dx, y + dy).biome != b {
                        diff += 1;
                    }
                }
                diff
            });
            let Some((x, y)) = found else { continue };
            let next: Vec<&str> = [(1, 0), (0, 1), (1, 1)].iter().filter(|&&(dx, dy)| x + dx < ow.w && y + dy < ow.h).map(|&(dx, dy)| ow.cell(x + dx, y + dy).biome.name()).collect();
            let g = Game::embark_with(42, ow.clone(), x, y, crate::world::MapSize::MEDIUM, &|_, _| {});
            // Every column's top, seen from above.
            let w = &g.world;
            println!("== {} at ({}, {}), east/south/southeast: {:?}", b.name(), x, y, next);
            for yy in (0..w.h).step_by(2) {
                let row: String = (0..w.w)
                    .map(|xx| {
                        let mut p = Pos::new(xx, yy, w.surface_z(xx, yy));
                        while p.z + 1 < w.d && w.t(p.up()).water > 0 {
                            p = p.up();
                        }
                        let (mut ch, _, _) = tile_look(w, p);
                        if let Some((c, _)) = foliage(w, p, 0.3, false) {
                            ch = c;
                        }
                        if w.t(p).water > 0 {
                            ch = '≈';
                        }
                        ch
                    })
                    .collect();
                println!("  {}", row);
            }
        }
    }

    #[test]
    #[ignore]
    fn show_world_map() {
        let seed = std::env::var("SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(42);
        let mut app = App::generated(seed);
        println!("{}", dump(&mut app, 170, 60));
        let ow = &app.game.overworld;
        let mut counts = std::collections::BTreeMap::new();
        for c in &ow.cells {
            *counts.entry(c.biome.name()).or_insert(0) += 1;
        }
        let (sx, sy) = ow.default_site();
        println!("default site {:?} {:?} feats {:?}", (sx, sy), ow.cell(sx, sy).biome, crate::terrain::feature_names(ow.site_features(seed, sx, sy)));
        println!("{:?} rivers {} volcanoes {} sites {}", counts, ow.cells.iter().filter(|c| c.river != 0).count(), ow.cells.iter().filter(|c| c.volcano).count(), ow.sites.len());
    }

    #[test]
    fn render_snapshot() {
        let mut app = App::generated(42);
        app.mode = Mode::Normal;
        for _ in 0..3000 {
            app.game.step();
        }
        println!("{}", dump(&mut app, 130, 45));
        let id = app.game.dwarves[0].id;
        app.game.toggle_soldier(id);
        app.game.dwarves[0].body.wound(crate::body::Part::LeftArm, 15, true);
        app.mode = Mode::Unit { id, sel: 2 };
        let card = dump(&mut app, 130, 50);
        println!("{}", card);
        assert!(card.contains("Bleeding from the left arm"), "the card should say where the dwarf bleeds");
        let s1 = app.game.new_squad();
        let s2 = app.game.new_squad();
        let (a, b) = (app.game.dwarves[1].id, app.game.dwarves[2].id);
        app.game.set_squad_member(s1, a, true);
        app.game.set_squad_member(s2, b, true);
        app.game.squads[s2].weapon = crate::game::military_weapon_for_tests();
        app.mode = Mode::Military;
        let s = dump(&mut app, 130, 45);
        println!("{}", s);
        assert!(s.contains("Squads") && s.contains(&app.game.squads[s1].name));
        let (x, y) = app.game.site;
        app.mode = Mode::WorldMap { x, y };
        println!("{}", dump(&mut app, 140, 40));
        app.mode = Mode::Legends { tab: 0, scroll: usize::MAX, from_world: false };
        println!("{}", dump(&mut app, 130, 40));
    }

    #[test]
    fn render_look_and_status() {
        let mut app = App::generated(42);
        let e = app.game.embark;
        let a = app.game.flat_patch(6, 6, 3);
        app.game.add_zone(a, a.offset(5, 5, 0), ZoneKind::Meeting);
        app.mode = Mode::Normal;
        for _ in 0..3000 {
            app.game.step();
        }
        app.game.dwarves[1].body.wound(crate::body::Part::LeftLeg, 12, true);
        // A ghost beside dwarf 0, and dwarf 0 married to dwarf 3.
        app.game.dwarves[2].alive = false;
        app.game.dwarves[2].ghost = true;
        app.game.dwarves[2].pos = app.game.dwarves[0].pos.offset(1, 0, 0);
        let (s0, s3) = (app.game.dwarves[0].id, app.game.dwarves[3].id);
        app.game.dwarves[0].spouse = Some(s3);
        app.game.dwarves[3].spouse = Some(s0);
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
        app.cursor = app.game.dwarves[2].pos;
        let s = dump(&mut app, 130, 45);
        assert!(s.contains("The ghost of"), "look mode doesn't show the ghost");
        for tab in 0..4 {
            app.mode = Mode::Status { tab, scroll: 0 };
            let s = dump(&mut app, 130, 45);
            println!("{}", s);
            assert!(s.contains("Fortress status"));
            if tab == 0 {
                assert!(s.contains("ghost haunts"), "no ghost warning");
            }
        }
        let planned = (2..12)
            .map(|dy| e.offset(2, dy, 0))
            .find(|&a| app.game.plan_constructions(a, a.offset(4, 0, 0), crate::game::ConstructKind::Wall, crate::game::BuildMat::Any) > 0)
            .is_some();
        assert!(planned, "nowhere to plan a wall");
        app.cursor = app.game.constructions.values().next().unwrap().pos;
        app.mode = Mode::Construct { kind: Some(crate::game::ConstructKind::Wall), mat: crate::game::BuildMat::Any, corner: None };
        let s = dump(&mut app, 130, 45);
        println!("{}", s);
        assert!(s.contains("Constructions") && s.contains("Planned: Wall"));
        app.mode = Mode::Zone { tool: ZoneTool::Bedroom, corner: None };
        println!("{}", dump(&mut app, 130, 45));
        // A bedroom handed to a dwarf from look mode.
        let _ = e;
        let b = app.game.flat_patch(2, 2, 2);
        assert!(app.game.add_zone(b, b.offset(1, 1, 0), ZoneKind::Bedroom) > 0);
        let zid = *app.game.zones.keys().max().unwrap();
        app.cursor = app.game.zones[&zid].tiles[0];
        app.mode = Mode::Look { sel: 0 };
        for c in [crossterm::event::KeyCode::Char('o'), crossterm::event::KeyCode::Down, crossterm::event::KeyCode::Enter] {
            app.on_key(crossterm::event::KeyEvent::new(c, crossterm::event::KeyModifiers::NONE));
        }
        let s = dump(&mut app, 130, 45);
        println!("{}", s);
        assert!(s.contains("Bedroom (") && !s.contains("unassigned"), "bedroom not assigned");
    }

    #[test]
    fn render_work_orders() {
        let mut app = App::generated(42);
        app.mode = Mode::Normal;
        let key = |c| crossterm::event::KeyEvent::new(c, crossterm::event::KeyModifiers::NONE);
        app.on_key(key(crossterm::event::KeyCode::Char('j')));
        app.on_key(key(crossterm::event::KeyCode::Char('n')));
        let s = dump(&mut app, 130, 45);
        assert!(s.contains("pick a job") && s.contains("Make wooden bed"), "{}", s);
        app.on_key(key(crossterm::event::KeyCode::Enter));
        app.on_key(key(crossterm::event::KeyCode::Right));
        let s = dump(&mut app, 130, 45);
        println!("{}", s);
        assert_eq!(app.game.work_orders.len(), 1);
        assert_eq!(app.game.work_orders[0].keep, 6, "bed order starts at 5, Right adds one");
        assert!(s.contains("Make wooden bed") && s.contains("/6"));
    }

    #[test]
    fn render_stockpile_settings() {
        let mut app = App::generated(42);
        let e = app.game.embark;
        assert!(app.game.add_zone(e.offset(-8, 3, -1), e.offset(-1, 9, 1), ZoneKind::Stockpile(StockFilter::Stone)) > 0);
        let zone = *app.game.zones.keys().last().unwrap();
        app.cursor = app.game.zones[&zone].tiles[0];
        app.mode = Mode::Look { sel: 0 };
        let key = |c| crossterm::event::KeyEvent::new(c, crossterm::event::KeyModifiers::NONE);
        app.on_key(key(crossterm::event::KeyCode::Char('s')));
        app.on_key(key(crossterm::event::KeyCode::Enter));
        let s = dump(&mut app, 130, 45);
        println!("{}", s);
        assert!(s.contains("[ ] boulders") && s.contains("[x] blocks"), "{}", s);
    }

    #[test]
    fn trees_turn_with_the_seasons() {
        let g = Game::new(42);
        let trees: Vec<Pos> = (0..g.world.h)
            .flat_map(|y| (0..g.world.w).map(move |x| (x, y)))
            .map(|(x, y)| Pos::new(x, y, g.world.surface_z(x, y)))
            .filter(|&p| g.world.t(p).kind == TileKind::Tree)
            .filter(|&p| { let mut w = g.world.clone(); w.t_mut(p).revealed = true; foliage(&w, p, 0.0, false).is_some() })
            .take(200)
            .collect();
        assert!(trees.len() > 20, "too few trees: {}", trees.len());
        let mut w = g.world.clone();
        for &p in &trees {
            w.t_mut(p).revealed = true;
        }
        let looks = |f: f32| -> HashSet<String> { trees.iter().map(|&p| format!("{:?}", foliage(&w, p, f, false).unwrap())).collect() };
        let summer = looks(0.35);
        let autumn = looks(0.62);
        let winter = looks(0.85);
        assert!(autumn.len() >= 4, "autumn should be colourful: {:?}", autumn);
        assert!(summer.len() <= 2, "summer is green: {:?}", summer);
        assert!(winter.iter().any(|s| s.contains("125, 95, 70")), "bare in winter: {:?}", winter);
        assert!(looks(0.05).iter().any(|s| s.contains("255, 175, 205") || s.contains("245, 235, 240")), "blossom in spring");
    }

    #[test]
    fn pick_an_office_owner_from_a_list() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut app = App::generated(42);
        let e = app.game.embark;
        app.game.add_zone(e.offset(-3, -3, 0), e.offset(-2, -2, 0), ZoneKind::Office);
        let zone = app.game.zones.values().find(|z| z.kind == ZoneKind::Office).unwrap();
        let (zid, at) = (zone.id, zone.tiles[0]);
        app.game.zones.get_mut(&zid).unwrap().owner = None;
        app.cursor = at;
        app.mode = Mode::Look { sel: 0 };
        let key = |c| KeyEvent::new(c, KeyModifiers::NONE);
        app.on_key(key(KeyCode::Char('o')));
        assert_eq!(app.mode, Mode::PickOwner { zone: zid, sel: 0 });
        let s = dump(&mut app, 130, 50);
        println!("{}", s);
        assert!(s.contains("Who owns this office?") && s.contains("(nobody)"), "{}", s);
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Enter));
        let first = app.game.owner_candidates()[0];
        assert_eq!(app.game.zones[&zid].owner, Some(first));
        assert_eq!(app.mode, Mode::Look { sel: 0 });
        // Esc changes nothing.
        app.on_key(key(KeyCode::Char('o')));
        assert_eq!(app.mode, Mode::PickOwner { zone: zid, sel: 1 }, "opens on the owner");
        app.on_key(key(KeyCode::Up));
        app.on_key(key(KeyCode::Esc));
        assert_eq!(app.game.zones[&zid].owner, Some(first));
    }

    #[test]
    fn the_chronicle_opens_at_the_new_year() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut app = App::generated(42);
        app.mode = Mode::Normal;
        app.on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
        assert_eq!(app.mode, Mode::Normal, "nothing to read yet");
        app.game.tick = crate::game::YEAR;
        app.game.year_log.festivals.push("harvest feast".into());
        app.game.write_chronicle_for_test();
        app.paused = false;
        app.frame();
        assert_eq!(app.mode, Mode::Chronicle { idx: 0 });
        assert!(app.paused);
        let s = dump(&mut app, 120, 40);
        println!("{}", s);
        assert!(s.contains("The Year of Feasting") && s.contains("harvest feast"), "{}", s);
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn send_a_squad_on_an_expedition() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut app = App::generated(81);
        let key = |c| KeyEvent::new(c, KeyModifiers::NONE);
        let sq = app.game.new_squad();
        let id = app.game.dwarves[0].id;
        app.game.set_squad_member(sq, id, true);
        app.mode = Mode::Military;
        app.on_key(key(KeyCode::Char('e')));
        assert_eq!(app.mode, Mode::Expedition { squad: 0, sel: 0 });
        let s = dump(&mut app, 130, 45);
        println!("{}", s);
        assert!(s.contains("Send on an expedition") && s.contains("days"), "{}", s);
        app.on_key(key(KeyCode::Enter));
        assert!(app.game.expedition.is_some());
        let s = dump(&mut app, 130, 45);
        assert!(s.contains("back in about"), "{}", s);
    }

    #[test]
    fn a_milestone_banner_shows_while_fresh() {
        let mut app = App::generated(42);
        app.mode = Mode::Normal;
        app.game.banner = Some(("The first masterwork: a granite door".into(), app.game.tick));
        let s = dump(&mut app, 120, 40);
        println!("{}", s);
        assert!(s.contains("The first masterwork"), "{}", s);
        app.game.tick += crate::game::milestones_banner_ticks();
        let s = dump(&mut app, 120, 40);
        assert!(!s.contains("The first masterwork"));
    }

    #[test]
    fn the_build_menu_offers_a_monument() {
        let mut app = App::generated(42);
        app.mode = Mode::BuildMenu;
        let s = dump(&mut app, 130, 50);
        assert!(s.contains("Great monument") && s.contains("Millstone"), "{}", s);
        let s = dump(&mut app, 80, 30);
        println!("{}", s);
        assert!(s.contains(" G ") && s.contains(" M "), "{}", s);
    }

    #[test]
    fn hand_the_fort_to_the_overseer_and_back() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut app = App::generated(42);
        app.mode = Mode::Normal;
        let key = |c| KeyEvent::new(c, KeyModifiers::NONE);
        app.on_key(key(KeyCode::Char('A')));
        assert_eq!(app.mode, Mode::Autoplay { sel: 0 });
        let s = dump(&mut app, 130, 45);
        assert!(s.contains("Reckless") && s.contains("you play"), "{}", s);
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.game.overseer.as_ref().map(|o| o.style), Some(crate::game::Style::Steady));
        let s = dump(&mut app, 130, 45);
        assert!(s.contains("AUTO: Steady"), "{}", s);
        app.on_key(key(KeyCode::Char('A')));
        app.on_key(key(KeyCode::Up));
        app.on_key(key(KeyCode::Enter));
        assert!(app.game.overseer.is_none());
    }

    #[test]
    fn the_brain_screen_shows_senses_and_takes_nudges() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut app = App::generated(42);
        app.game.set_overseer(Some(crate::game::Style::Warlord));
        for _ in 0..200 {
            app.game.step();
        }
        app.mode = Mode::Normal;
        let key = |c| KeyEvent::new(c, KeyModifiers::NONE);
        app.on_key(key(KeyCode::Char('B')));
        assert_eq!(app.mode, Mode::Brain { sel: 0 });
        let s = dump(&mut app, 140, 45);
        println!("{}", s);
        assert!(s.contains("Hunger") && s.contains("Provision") && s.contains("Your nudge"), "{}", s);
        // Nudge Muster up three times, and force Venture.
        let muster = crate::game::Ambition::ALL.iter().position(|&a| a == crate::game::Ambition::Muster).unwrap();
        for _ in 0..muster {
            app.on_key(key(KeyCode::Down));
        }
        let before = app.game.overseer_brain().unwrap().odds_of(crate::game::Ambition::Muster);
        for _ in 0..3 {
            app.on_key(key(KeyCode::Right));
        }
        let after = app.game.overseer_brain().unwrap().odds_of(crate::game::Ambition::Muster);
        assert!(after > before, "nudging should raise the odds ({} -> {})", before, after);
        app.on_key(key(KeyCode::Char('f')));
        assert_eq!(app.game.overseer.as_ref().unwrap().ambition, Some(crate::game::Ambition::Muster));
    }

    #[test]
    fn the_mouse_drags_boxes_clicks_menus_and_shows_tooltips() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
        let mut app = App::generated(42);
        app.mode = Mode::Normal;
        dump(&mut app, 130, 45);
        let mouse = |kind, column, row| MouseEvent { kind, column, row, modifiers: KeyModifiers::NONE };
        // Click the "Build" entry in the side panel.
        let s = dump(&mut app, 130, 45);
        let (by, line) = s.lines().enumerate().find(|(_, l)| l.contains("Build")).expect("a Build entry");
        let bx = line.char_indices().position(|(_, c)| c == 'B').unwrap() as u16;
        app.on_mouse(mouse(MouseEventKind::Down(MouseButton::Left), bx, by as u16));
        assert_eq!(app.mode, Mode::BuildMenu, "clicking the menu entry should open it");
        app.on_mouse(mouse(MouseEventKind::Down(MouseButton::Right), 5, 5));
        assert_eq!(app.mode, Mode::Normal, "right click goes back");
        // Drag a dig box across the map.
        app.on_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        dump(&mut app, 130, 45);
        let r = app.map_rect;
        let before = app.game.designations.len();
        app.on_mouse(mouse(MouseEventKind::Down(MouseButton::Left), r.x + 5, r.y + 5));
        app.on_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), r.x + 9, r.y + 8));
        app.on_mouse(mouse(MouseEventKind::Up(MouseButton::Left), r.x + 9, r.y + 8));
        let a = Pos::new(app.view_x + 5, app.view_y + 5, app.cursor.z);
        assert!(matches!(app.mode, Mode::Designate { corner: None, .. }), "the box should be finished");
        assert_eq!(app.cursor, Pos::new(app.view_x + 9, app.view_y + 8, a.z));
        let _ = before;
        // Drag to pan in normal mode.
        app.mode = Mode::Normal;
        let c0 = app.cursor;
        app.on_mouse(mouse(MouseEventKind::Down(MouseButton::Left), r.x + 20, r.y + 10));
        app.on_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), r.x + 17, r.y + 10));
        app.on_mouse(mouse(MouseEventKind::Up(MouseButton::Left), r.x + 17, r.y + 10));
        assert_eq!(app.cursor, c0, "panning leaves the cursor alone");
        assert!(app.free_view, "and hides it");
        app.on_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        assert!(!app.free_view);
        assert_eq!(app.cursor.x, (app.view_x + r.width as i32 / 2).clamp(0, app.game.world.w - 1), "an arrow brings the cursor back mid-view");
        // Hovering over the wagon shows what's there.
        dump(&mut app, 130, 45);
        let e = app.game.embark;
        app.cursor = e;
        dump(&mut app, 130, 45);
        let (hx, hy) = ((e.x - app.view_x) as u16 + r.x, (e.y - app.view_y) as u16 + r.y);
        app.on_mouse(mouse(MouseEventKind::Moved, hx, hy));
        let s = dump(&mut app, 130, 45);
        assert!(s.lines().count() > 0);
        let tip_rows = s.lines().skip(hy as usize + 1).take(3).collect::<Vec<_>>().join(" ");
        assert!(tip_rows.contains('┌') || tip_rows.contains('│'), "a tooltip should appear by the mouse: {}", tip_rows);
    }

    #[test]
    fn a_siege_stops_the_game_and_turns_the_camera_to_it() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut app = App::generated(42);
        app.mode = Mode::Normal;
        dump(&mut app, 130, 45);
        let foe = app.game.neighbors.first().cloned().expect("a neighbour");
        let mut foe = foe;
        foe.race = crate::overworld::Race::Goblin;
        app.game.lay_siege(&foe);
        let general = app.game.creatures.values().find(|c| c.general.is_some()).unwrap().pos;
        app.paused = false;
        app.frame();
        assert!(app.paused, "a siege should pause the game");
        let (s, _) = app.spot.clone().expect("the siege should be in the spotlight");
        assert_eq!(s.title, "Siege!");
        assert!((app.cursor.x - general.x).abs() <= 4 && (app.cursor.y - general.y).abs() <= 4, "the camera should be on the army");
        let screen = dump(&mut app, 130, 45);
        assert!(screen.contains("Siege!") && screen.contains("Enter: go on"), "{}", screen);
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.spot.is_none());
        // With the overseer playing it goes by on its own.
        app.game.set_overseer(Some(crate::game::Style::Steady));
        app.game.spotlight_for_test("Traders", "Merchants are here.");
        app.frame();
        assert!(app.spot.is_some());
        for _ in 0..120 {
            app.frame();
        }
        assert!(app.spot.is_none(), "the card should go by on its own when the overseer plays");
    }

    #[test]
    #[ignore]
    fn show_new_screens() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut app = App::generated(42);
        app.mode = Mode::Normal;
        for _ in 0..2000 {
            app.game.step();
        }
        let foe = app.game.neighbors.first().cloned().unwrap();
        let mut foe = foe;
        foe.race = crate::overworld::Race::Goblin;
        dump(&mut app, 150, 46);
        app.game.lay_siege(&foe);
        app.paused = false;
        app.frame();
        println!("{}", dump(&mut app, 150, 46));
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        app.on_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        println!("{}", dump(&mut app, 150, 46));
        app.mode = Mode::Normal;
        for i in 0..app.game.dwarves.len() {
            let mut d = std::mem::take(&mut app.game.dwarves[i]);
            app.game.kill_for_test(&mut d, "has been struck down by the goblin");
            app.game.dwarves[i] = d;
        }
        app.frame();
        println!("{}", dump(&mut app, 150, 46));
    }

    #[test]
    fn the_overseers_doings_sit_beside_the_news_and_f3_shows_the_timings() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut app = App::generated(42);
        app.mode = Mode::Normal;
        let s = dump(&mut app, 150, 46);
        assert!(!s.contains("The overseer,"), "no overseer, no panel");
        app.game.set_overseer(Some(crate::game::Style::Steady));
        for _ in 0..crate::game::DAY * 3 {
            app.game.step();
        }
        assert!(!app.game.overseer.as_ref().unwrap().notes.is_empty());
        let s = dump(&mut app, 150, 46);
        assert!(s.contains("The overseer,"), "{}", s);
        // F3: the overlay comes up once a second's timings are in.
        app.on_key(KeyEvent::new(KeyCode::F(3), KeyModifiers::NONE));
        assert!(app.perf.is_some() && app.game.prof.is_some());
        app.paused = false;
        let start = std::time::Instant::now();
        while start.elapsed().as_secs_f64() < 1.1 {
            let t = std::time::Instant::now();
            app.frame();
            app.perf_frame(0.5, t.elapsed().as_secs_f64() * 1000.0, 1);
        }
        let s = dump(&mut app, 150, 46);
        assert!(s.contains("Performance (F3)") && s.contains("fps"), "{}", s);
        println!("{}", s);
        app.on_key(KeyEvent::new(KeyCode::F(3), KeyModifiers::NONE));
        assert!(app.perf.is_none() && app.game.prof.is_none());
    }

    #[test]
    fn when_the_last_dwarf_dies_the_game_is_over() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut app = App::generated(42);
        app.mode = Mode::Normal;
        for i in 0..app.game.dwarves.len() {
            let mut d = std::mem::take(&mut app.game.dwarves[i]);
            app.game.kill_for_test(&mut d, "has been struck down by the goblin");
            app.game.dwarves[i] = d;
        }
        app.frame();
        assert_eq!(app.mode, Mode::GameOver);
        let s = dump(&mut app, 130, 45);
        assert!(s.contains("has fallen") && s.contains("Game over") && s.contains("How they died"), "{}", s);
        app.on_key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));
        assert_eq!(app.mode, Mode::Normal);
        app.frame();
        assert_eq!(app.mode, Mode::Normal, "it's told once");
    }

    #[test]
    fn status_and_brain_open_beside_the_map() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut app = App::generated(42);
        app.mode = Mode::Normal;
        dump(&mut app, 170, 50);
        let full_map = app.map_rect.width;
        app.on_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        assert!(matches!(app.mode, Mode::Status { .. }));
        let s = dump(&mut app, 170, 50);
        assert!(app.map_rect.width > 20 && app.map_rect.width < full_map, "the map should stay, narrower: {}", app.map_rect.width);
        assert!(s.contains("full screen"), "{}", s);
        app.on_key(KeyEvent::new(KeyCode::Char('\\'), KeyModifiers::NONE));
        assert!(!app.docked);
        let s = dump(&mut app, 170, 50);
        assert!(s.contains("side pane"), "{}", s);
    }

    #[test]
    fn reports_scroll_back_through_old_announcements() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut app = App::generated(42);
        for i in 0..200 {
            app.game.tick += 300;
            app.game.msg(&format!("Report number {}", i), (200, 200, 200));
        }
        app.game.msg("Urist Testbeard has been struck down by the goblin.", (255, 60, 60));
        app.mode = Mode::Normal;
        // Full screen, so only the reports are on it (docked, the announcements show beside).
        app.docked = false;
        let key = |c| KeyEvent::new(c, KeyModifiers::NONE);
        app.on_key(key(KeyCode::Char('r')));
        assert_eq!(app.mode, Mode::Reports { filter: 0, scroll: 0 });
        let s = dump(&mut app, 130, 40);
        println!("{}", s);
        assert!(s.contains("struck down") && s.contains("Report number 199"), "{}", s);
        for _ in 0..10 {
            app.on_key(key(KeyCode::PageUp));
        }
        let s = dump(&mut app, 130, 40);
        assert!(s.contains("Report number 0 ") || s.contains("Report number 0\u{20}") || s.contains("Report number 0"), "{}", s);
        assert!(!s.contains("Report number 199"));
        app.on_key(key(KeyCode::Char('4')));
        let s = dump(&mut app, 130, 40);
        assert!(s.contains("struck down") && !s.contains("Report number 5"), "deaths only: {}", s);
    }

    #[test]
    fn render_positions() {
        let mut app = App::generated(42);
        app.mode = Mode::Nobles;
        let key = |c| crossterm::event::KeyEvent::new(c, crossterm::event::KeyModifiers::NONE);
        app.on_key(key(crossterm::event::KeyCode::Char('1')));
        assert!(matches!(app.mode, Mode::Appoint { pos: 0, sel: 1 }));
        let s = dump(&mut app, 130, 60);
        println!("{}", s);
        assert!(s.contains("Appoint manager") && s.contains("(leave vacant)") && s.contains("fit"), "{}", s);
        let best = app.game.candidates(crate::game::Position::Manager)[0];
        let best = app.game.dwarves.iter().find(|d| d.id == best).unwrap().name.clone();
        app.on_key(key(crossterm::event::KeyCode::Enter));
        assert!(matches!(app.mode, Mode::Nobles));
        let s = dump(&mut app, 130, 60);
        assert!(s.contains(&format!("Manager: {}", best)), "{}", s);
        assert!(s.contains("Broker: vacant"));
        // Esc leaves a post as it was; the vacant row empties it.
        app.on_key(key(crossterm::event::KeyCode::Char('1')));
        assert!(matches!(app.mode, Mode::Appoint { pos: 0, sel: 1 }), "opens on the holder");
        app.on_key(key(crossterm::event::KeyCode::Esc));
        assert!(app.game.holder(crate::game::Position::Manager).is_some());
        app.on_key(key(crossterm::event::KeyCode::Char('1')));
        app.on_key(key(crossterm::event::KeyCode::Up));
        app.on_key(key(crossterm::event::KeyCode::Enter));
        assert!(app.game.holder(crate::game::Position::Manager).is_none());
    }

    #[test]
    fn render_labors() {
        let mut app = App::generated(42);
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

    /// Title → new world → embark → play → save → title → continue, drawing
    /// every frame on the way: none may be blank.
    #[test]
    fn render_title() {
        let mut app = App::new(4242);
        app.mode = Mode::Title { sel: 0 };
        for (w, h) in [(40, 12), (80, 24), (130, 45)] {
            let s = dump(&mut app, w, h);
            println!("{}", s);
            assert!(s.contains("New world") && s.contains("Load game"), "{}", s);
        }
    }

    #[test]
    fn title_to_fortress_and_back() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let dir = std::env::temp_dir().join(format!("dwarfclone-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut app = App::new(4242);
        app.save_dir = dir.clone();
        app.mode = Mode::Title { sel: 0 };
        let key = |c| KeyEvent::new(c, KeyModifiers::NONE);
        let check = |app: &mut App, want: &str| {
            let s = dump(app, 120, 40);
            assert!(s.chars().any(|c| !c.is_whitespace()), "blank frame in {:?}", app.mode);
            assert!(s.contains(want), "expected {:?} on screen in {:?}:
{}", want, app.mode, s);
        };
        // Run frames until the loading screen goes away, checking each one.
        let wait = |app: &mut App, loading: &str| {
            let t = std::time::Instant::now();
            while app.mode == Mode::Loading {
                check(app, loading);
                app.frame();
                std::thread::sleep(std::time::Duration::from_millis(5));
                assert!(t.elapsed().as_secs() < 120, "{} never finished", loading);
            }
        };

        check(&mut app, "New world");
        app.on_key(key(KeyCode::Enter));
        assert!(matches!(app.mode, Mode::WorldSetup { .. }), "{:?}", app.mode);
        check(&mut app, "Create a new world");
        assert_eq!(app.world_params.seed, 4242, "the command-line seed should be used first");
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.mode, Mode::WorldGen);
        // Watch a few paced frames, then hurry it along.
        for _ in 0..5 {
            check(&mut app, "Creating");
            app.frame();
        }
        app.on_key(key(KeyCode::Char(' ')));
        let t = std::time::Instant::now();
        while !app.worldgen.as_ref().unwrap().done() {
            check(&mut app, "Year");
            app.frame();
            assert!(t.elapsed().as_secs() < 120, "world generation never finished");
        }
        check(&mut app, "The world is ready");
        app.on_key(key(KeyCode::Enter));
        assert!(matches!(app.mode, Mode::WorldMap { .. }), "after worldgen: {:?}", app.mode);
        check(&mut app, "Choose a site");
        check(&mut app, "Embark: Medium 96×64");
        app.on_key(key(KeyCode::Char('s')));
        check(&mut app, "Embark: Large 144×96, 48 deep");
        app.on_key(key(KeyCode::Char('s')));
        check(&mut app, "Embark: Small 64×48");

        app.on_key(key(KeyCode::Enter));
        wait(&mut app, "Embarking");
        assert_eq!((app.game.world.w, app.game.world.h), (64, 48), "embarked at the wrong size");
        assert_eq!(app.mode, Mode::Help);
        app.on_key(key(KeyCode::Esc));
        let name = app.game.fortress_name.clone();
        assert!(!name.is_empty());
        app.paused = false;
        for _ in 0..30 {
            app.frame();
        }
        let tick = app.game.tick;
        assert!(tick > 0, "the fortress never ran");

        // F5: save and carry on.
        app.on_key(key(KeyCode::F(5)));
        wait(&mut app, "Saving");
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.game.fortress_name, name, "game not handed back after saving");
        assert_eq!(app.saves().len(), 1);

        // Save and return to the title, then continue.
        app.on_key(key(KeyCode::Char('Q')));
        app.on_key(key(KeyCode::Char('t')));
        wait(&mut app, "Saving");
        assert!(matches!(app.mode, Mode::Title { .. }), "after save-to-title: {:?}", app.mode);
        check(&mut app, &name);
        app.mode = Mode::Title { sel: 1 };
        app.on_key(key(KeyCode::Enter));
        wait(&mut app, "Loading");
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.game.fortress_name, name);
        assert!(app.game.tick >= tick);
        check(&mut app, &name);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
