//! Adventurer mode's local view (G03 A2; `app::adventure` has the keys):
//! the site round the hero, what they see now bright and what they
//! remember dim; a sidebar with their body, gear, time and whatever is in
//! sight; the combat log under the map, this turn's lines marked; and the
//! pages drawn over it (look, inventory, status, the letter lists, help,
//! the menu). Below 118 columns the sidebar folds into two lines under the
//! map. Each other view gets its own file (`adv_create`, `adv_travel`,
//! `adv_talk`, `adv_end`).

use super::health::{history_lines, part_lines};
use super::looks::{foliage, tile_look};
use super::{backdrop, cursor_info_terrain, dim, header, item_line, key_line, look_lines, plain, popup, rgb, wrap_words, BLACK, KEY, MUTED, TEXT};
use crate::app::adventure::{pick_choices, AdvScreen, Pick, MENU, REPLAY_MS};
use crate::game::adv::fight::{Aim, AIMS};
use crate::app::App;
use crate::body::{Body, Part, PARTS};
use crate::dwarf::{skill_rank, Dwarf, LABORS};
use crate::game::adv::turn::Sight;
use crate::game::adv::{Adventure, QuestState, View};
use crate::game::{clock, Game};
use crate::geom::Pos;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

/// The sidebar's width, as the fort's.
const SIDE_W: u16 = 38;
/// Rows of the log box under the map (with its border).
const LOG_ROWS: u16 = 8;
const RED: Color = Color::Rgb(235, 70, 60);
const WARN: Color = Color::Rgb(240, 150, 110);

pub(super) fn draw(f: &mut Frame, app: &App) {
    let area = f.size();
    let Some(s) = app.adv.as_deref() else {
        backdrop(f, area);
        return;
    };
    if s.view == View::Create {
        return super::adv_create::draw(f, area, s);
    }
    // Between sites: the world map and the road (A3).
    let Some(v) = s.adv.visit.as_ref() else {
        return super::adv_travel::draw(f, area, s);
    };
    let side = if area.width >= 80 + SIDE_W { SIDE_W } else { 0 };
    let fold: u16 = if side == 0 { 2 } else { 0 };
    let log_h = LOG_ROWS.min(area.height / 4).max(3);
    let left_w = area.width - side;
    let map_h = area.height.saturating_sub(log_h + fold);
    let map = Rect::new(area.x, area.y, left_w, map_h);
    let sight = v.g.seen_tiles(v.hero);
    let look = match s.view {
        View::Look(p) => Some(p),
        _ => None,
    };
    // The last act played back: its blows, sprays and bolts at the tick
    // being shown (the map itself is as it is now).
    let show = match s.replay {
        Some((from, Some(t0))) => (from + t0.elapsed().as_millis() as u64 / REPLAY_MS).min(v.g.tick),
        Some((from, None)) => from,
        None => v.g.tick,
    };
    draw_local(f, map, &s.adv, &sight, look, show);
    draw_log(f, Rect::new(area.x, area.y + map_h + fold, left_w, log_h), s);
    if side > 0 {
        let r = Rect::new(area.x + left_w, area.y, side, area.height);
        match look {
            Some(p) => draw_look(f, r, &v.g, &sight, p),
            None => draw_side(f, r, s, &sight),
        }
    } else {
        draw_strip(f, Rect::new(area.x, area.y + map_h, left_w, fold), s);
        if let Some(p) = look {
            // (Folded: the look panel over the map's right side.)
            let w = 40.min(area.width);
            draw_look(f, Rect::new(area.x + area.width - w, area.y, w, map_h.min(18)), &v.g, &sight, p);
        }
    }
    match s.view {
        View::Inventory { sel } => draw_inventory(f, area, s, sel),
        View::Status => draw_status(f, area, &s.adv),
        View::Menu { sel } => draw_menu(f, area, sel),
        _ => {}
    }
    if let Some((kind, ids)) = &s.pick {
        draw_pick(f, area, &v.g, v.hero, *kind, ids);
    }
    if s.help {
        draw_help(f, area);
    }
    if let Some(w) = &s.page {
        draw_writing(f, map, w);
    }
}

/// A page being read (A5), on parchment over the map: a fallen fort's
/// saga, its overseer's journal, its chronicle, its walls.
fn draw_writing(f: &mut Frame, map: Rect, w: &crate::game::adv::sitegen::fallen::Writing) {
    let p = super::saga::Page { title: w.title.clone(), doom: w.doom, open: false, sub: w.sub.clone(), chapters: w.chapters.clone(), closing: w.closing.clone(), fell: w.fell };
    let width = map.width.saturating_sub(4).clamp(map.width.min(30), 76);
    let h = super::saga::rows_needed(&p, width).min(map.height);
    let r = Rect::new(map.x + map.width.saturating_sub(width) / 2, map.y + map.height.saturating_sub(h) / 2, width, h);
    super::saga::draw_page(f, r, map, &p, None, "any key: put it down");
}

/// The site round the hero (or the look cursor) on the hero's level: what
/// is in sight in full colour, what is remembered dim and empty of
/// creatures, and the rest dark.
fn draw_local(f: &mut Frame, area: Rect, a: &Adventure, sight: &Sight, look: Option<Pos>, show: u64) {
    let Some(v) = a.visit.as_ref() else { return };
    let g = &v.g;
    let hero = &g.dwarves[v.hero];
    let me = hero.pos;
    let title = format!(" {} · {} ", g.fortress_name, cap(clock::time_words(g.tick)));
    let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(MUTED)).title(Span::styled(title, Style::default().fg(KEY)));
    let r = block.inner(area);
    f.render_widget(block, area);
    let w = &g.world;
    let (vw, vh) = (r.width as i32, r.height as i32);
    if vw <= 0 || vh <= 0 {
        return;
    }
    // Centred on the hero, unless the look cursor has wandered off screen.
    let mut focus = me;
    if let Some(p) = look {
        if (p.x - me.x).abs() > vw / 2 - 2 || (p.y - me.y).abs() > vh / 2 - 2 {
            focus = p;
        }
    }
    let ox = if vw >= w.w { -(vw - w.w) / 2 } else { (focus.x - vw / 2).clamp(0, w.w - vw) };
    let oy = if vh >= w.h { -(vh - w.h) / 2 } else { (focus.y - vh / 2).clamp(0, w.h - vh) };
    let z = me.z;
    // Under the sky the light of the hour tints what is seen.
    let sky = w.in_bounds(me) && w.outside(me);
    let light = if sky { 0.5 + 0.5 * clock::light(g.tick, true) as f32 / 255.0 } else { 0.85 };
    let year_frac = (g.tick % crate::game::YEAR) as f32 / crate::game::YEAR as f32;
    let cold = g.biome.cold();
    // Blows flashing, blood and severed parts flying (gore.rs), and bolts
    // in flight (drills.rs), at the tick shown; what is still in the air
    // isn't drawn where it lands.
    let mut fx: std::collections::HashMap<(i32, i32), (char, (u8, u8, u8), Option<(u8, u8, u8)>)> = Default::default();
    let mut flying = std::collections::HashSet::new();
    for e in g.gore.fx.iter().filter(|e| e.from.z == z) {
        if let Some(q) = e.pos_at(show) {
            fx.insert((q.x, q.y), (e.glyph, e.rgb, e.bg));
        }
        if let Some(id) = e.item.filter(|_| show < e.at + e.len) {
            flying.insert(id);
        }
    }
    for b in g.drills.flights.iter().filter(|b| b.from.z == z) {
        if let Some(q) = b.at_tick(show) {
            fx.insert((q.x, q.y), (b.glyph(), (225, 205, 150), None));
        }
    }
    let tup = |c: Color| match c {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (128, 128, 128),
    };
    let mut items: std::collections::HashMap<(i32, i32), (char, Color)> = Default::default();
    for it in g.items.values().filter(|it| it.carried_by.is_none() && it.container.is_none() && it.pos.z == z && !flying.contains(&it.id)) {
        items.entry((it.pos.x, it.pos.y)).or_insert_with(|| it.glyph());
    }
    let mut beasts: std::collections::HashMap<(i32, i32), (char, Color, bool)> = Default::default();
    for c in g.creatures.values().filter(|c| c.alive && !c.sneaking && sight.sees(c.pos)) {
        let info = c.species.info();
        let (glyph, color) = if c.breathes_fire {
            ('D', (255, 90, 40))
        } else if let Some(look) = g.kin_look(c) {
            look
        } else {
            (info.glyph, info.color)
        };
        beasts.insert((c.pos.x, c.pos.y), (glyph, rgb(color), c.hostile() && !c.leaving));
    }
    let mut folk: std::collections::HashMap<(i32, i32), &Dwarf> = Default::default();
    for d in g.dwarves.iter().filter(|d| d.alive && sight.sees(d.pos)) {
        folk.insert((d.pos.x, d.pos.y), d);
    }
    let race = a.hero.race.color();
    let buf = f.buffer_mut();
    for sy in 0..vh {
        for sx in 0..vw {
            let (x, y) = (ox + sx, oy + sy);
            let cell = buf.get_mut(r.x + sx as u16, r.y + sy as u16);
            let p = Pos::new(x, y, z);
            if !w.in_bounds(p) {
                cell.set_char(' ').set_fg(BLACK).set_bg(BLACK);
                continue;
            }
            let t = w.t(p);
            let seen = sight.sees(p);
            if !seen && !t.revealed {
                cell.set_char(' ').set_fg(BLACK).set_bg(BLACK);
                if look == Some(p) {
                    cell.set_char('X').set_fg(BLACK).set_bg(Color::Rgb(220, 200, 0));
                }
                continue;
            }
            let (mut ch, mut fg, mut bg) = tile_look(w, p);
            if (t.kind == crate::world::TileKind::Tree || t.kind == crate::world::TileKind::Shrub || t.mat == crate::material::Mat::Grass) && t.water == 0 {
                if let Some((c, col)) = foliage(w, p, year_frac, cold) {
                    (ch, fg) = (c, col);
                }
            }
            if t.snow {
                fg = Color::Rgb(235, 240, 250);
                if matches!(ch, '.' | ',' | '`' | '\'' | '·') {
                    ch = '·';
                }
            }
            // Blood dashed up a wall, in streaks through the stone.
            if let Some(st) = g.wall_stain(p) {
                let (c, f2, b2) = crate::game::gore::wall_look(st, ch, tup(fg));
                (ch, fg, bg) = (c, rgb(f2), rgb(b2));
            }
            if !t.kind.solid() {
                // Blood on the ground: drops, a pool where it's thick, footprints.
                if let Some(st) = g.gore.stains.get(&p) {
                    let (c, f2, b2) = crate::game::gore::floor_look(st, ch, tup(fg), tup(bg));
                    (ch, fg, bg) = (c, rgb(f2), rgb(b2));
                }
                let k = g.smoke_at(p) as u32;
                if k > 0 && seen {
                    bg = Color::Rgb((30 + 14 * k) as u8, (30 + 14 * k) as u8, (34 + 14 * k) as u8);
                    ch = if k >= 4 { '▒' } else { '░' };
                    fg = Color::Rgb(175, 175, 180);
                }
            }
            if g.web_at(p) {
                ch = '#';
                fg = Color::Rgb(225, 225, 235);
            }
            if t.fire > 0 || g.flames.iter().any(|&(q, at)| q == p && at + 8 > g.tick) {
                ch = if (g.tick / 4 + x as u64) % 2 == 0 { '▲' } else { '^' };
                fg = Color::Rgb(255, 170, 40);
                bg = Color::Rgb(90, 20, 0);
            }
            if t.building != 0 {
                if let Some(b) = g.buildings.get(&t.building) {
                    ch = b.glyph_at(x - b.pos.x, y - b.pos.y);
                    fg = if b.kind.is_workshop() { b.kind.color() } else { b.mat.color() };
                }
            }
            if let Some(&(ic, icol)) = items.get(&(x, y)) {
                if t.building == 0 || !ch.is_ascii_uppercase() {
                    ch = ic;
                    fg = icol;
                }
            }
            if seen {
                if let Some(&(bc, bcol, hostile)) = beasts.get(&(x, y)) {
                    ch = bc;
                    fg = bcol;
                    if hostile {
                        bg = Color::Rgb(90, 0, 0);
                    }
                }
                if let Some(d) = folk.get(&(x, y)) {
                    ch = if d.soldier { '☻' } else { '☺' };
                    fg = if d.body.unconscious() { dim(rgb(d.color), 0.5) } else { rgb(d.color) };
                    if d.body.bleeding > 0 {
                        bg = Color::Rgb(70, 0, 0);
                    }
                }
                if light < 0.99 {
                    fg = dim(fg, light.max(0.6));
                    bg = dim(bg, light);
                }
            } else {
                // Remembered, not seen: grey and dim.
                fg = dim(grey(fg), 0.45);
                bg = dim(bg, 0.3);
            }
            if p == me {
                ch = '☺';
                fg = if !hero.alive {
                    Color::Rgb(120, 30, 30)
                } else if hero.body.unconscious() {
                    dim(race, 0.5)
                } else {
                    race
                };
                bg = if hero.body.bleeding > 0 { Color::Rgb(90, 0, 0) } else { Color::Rgb(30, 30, 45) };
            }
            if let Some(&(gc, gcol, gbg)) = fx.get(&(x, y)).filter(|_| seen) {
                // (The hero stays in sight under a blow: it shows round them.)
                if p != me {
                    ch = gc;
                    fg = rgb(gcol);
                }
                if let Some(c) = gbg {
                    bg = rgb(c);
                }
            }
            if look == Some(p) {
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

/// A colour washed towards grey (for what is remembered, not seen).
fn grey(c: Color) -> Color {
    match c {
        Color::Rgb(r, g, b) => {
            let l = (r as u16 + g as u16 + b as u16) / 3;
            let mix = |v: u8| ((v as u16 + l * 2) / 3) as u8;
            Color::Rgb(mix(r), mix(g), mix(b))
        }
        other => other,
    }
}

/// The hero's body as a little figure, each part in the colour of its
/// state: green whole, pink bruised, yellow badly wounded, orange mangled,
/// a red dot where it's gone; a part that bleeds on a red ground.
fn doll(b: &Body) -> [Vec<Span<'static>>; 4] {
    let part = |p: Part, c: char| -> Span<'static> {
        let i = p.idx();
        let status = b.part_status(p);
        let fg = match status {
            "missing" => Color::Rgb(140, 30, 30),
            "mangled" => Color::Rgb(255, 150, 50),
            "badly wounded" => Color::Rgb(235, 215, 70),
            "bruised" => Color::Rgb(220, 160, 140),
            _ => Color::Rgb(110, 190, 110),
        };
        let ch = if status == "missing" { '·' } else { c };
        let mut st = Style::default().fg(fg);
        if b.bleeds[i] > 0 {
            st = st.bg(Color::Rgb(110, 0, 0));
        }
        Span::styled(ch.to_string(), st)
    };
    let sp = |s: &str| Span::raw(s.to_string());
    [
        vec![sp("  "), part(Part::Head, 'O'), sp("  ")],
        vec![sp(" "), part(Part::LeftArm, '/'), part(Part::UpperBody, '█'), part(Part::RightArm, '\\'), sp(" ")],
        vec![sp("  "), part(Part::LowerBody, '█'), sp("  ")],
        vec![sp(" "), part(Part::LeftLeg, '/'), sp(" "), part(Part::RightLeg, '\\'), sp(" ")],
    ]
}

/// "Blood ███████░░░ 70%", green, yellow or red.
fn blood_bar(b: &Body, width: usize) -> Vec<Span<'static>> {
    let pct = (b.blood.max(0) * 100 / b.max_blood.max(1)).min(100);
    let col = if pct > 70 {
        Color::Rgb(120, 200, 120)
    } else if pct > 40 {
        Color::Rgb(230, 200, 80)
    } else {
        RED
    };
    let full = (pct as usize * width + 50) / 100;
    vec![
        Span::styled("Blood ", Style::default().fg(MUTED)),
        Span::styled("█".repeat(full), Style::default().fg(col)),
        Span::styled("░".repeat(width - full), Style::default().fg(Color::Rgb(60, 60, 70))),
        Span::styled(format!(" {}%", pct), Style::default().fg(col)),
    ]
}

/// The body in words, worst first: out cold, bleeding out (and how long
/// that leaves), limping, a useless sword arm.
fn body_words(d: &Dwarf) -> Vec<(String, Color)> {
    let b = &d.body;
    let mut v = Vec::new();
    if !d.alive {
        v.push(("Dead".to_string(), RED));
        return v;
    }
    if b.unconscious() {
        v.push(("Out cold".to_string(), RED));
    }
    let rate = b.bleed_rate();
    if rate > 0 || b.bleeding > 0 {
        match b.ticks_to_death() {
            Some(t) => v.push((format!("Bleeding out! (~{} h)", (t / clock::HOUR).max(1)), RED)),
            None if rate >= 4 => v.push(("Bleeding badly".to_string(), RED)),
            None => v.push(("Bleeding".to_string(), Color::Rgb(230, 110, 90))),
        }
    }
    if b.limp() > 0 {
        v.push(("Limping".to_string(), Color::Rgb(235, 200, 80)));
    }
    if !b.can_hold() {
        v.push(("Can't grip a weapon".to_string(), Color::Rgb(235, 200, 80)));
    }
    if b.infected() {
        v.push(("Infected".to_string(), Color::Rgb(180, 200, 90)));
    }
    if v.is_empty() {
        let hurt = PARTS.iter().any(|&p| b.part_status(p) != "fine");
        v.push(if hurt { ("Hurt, but on your feet".to_string(), Color::Rgb(220, 160, 140)) } else { ("Unhurt".to_string(), Color::Rgb(120, 200, 120)) });
    }
    v
}

/// The figure beside the blood bar and the words.
pub(super) fn body_block(d: &Dwarf, bar_w: usize) -> Vec<Line<'static>> {
    let words = body_words(d);
    let mut right: Vec<Vec<Span<'static>>> = vec![blood_bar(&d.body, bar_w)];
    for (w, c) in words.into_iter().take(3) {
        right.push(vec![Span::styled(w, Style::default().fg(c))]);
    }
    let fig = doll(&d.body);
    fig.into_iter()
        .enumerate()
        .map(|(i, mut l)| {
            l.push(Span::raw("  "));
            if let Some(r) = right.get(i) {
                l.extend(r.iter().cloned());
            }
            Line::from(l)
        })
        .collect()
}

/// The hero, their body, gear, the time and place, and what they see.
fn draw_side(f: &mut Frame, r: Rect, s: &AdvScreen, sight: &Sight) {
    let a = &s.adv;
    let Some(vis) = a.visit.as_ref() else { return };
    let g = &vis.g;
    let d = &g.dwarves[vis.hero];
    let mut v: Vec<Line> = Vec::new();
    v.push(Line::from(Span::styled(d.called(), Style::default().fg(KEY).add_modifier(Modifier::BOLD))));
    let mut who = format!("A {} {}", race_word(a), calling_word(a));
    if s.sneak {
        who.push_str(" · sneaking");
    }
    if s.rest_until.is_some() {
        who.push_str(" · resting");
    }
    v.push(plain(who, MUTED));
    v.push(plain(g.fortress_name.clone(), TEXT));
    v.push(plain(g.date_string(), MUTED));
    v.push(plain(format!("{}, {}", cap(clock::time_words(g.tick)), clock::moon_words(g.tick)), MUTED));
    v.push(Line::default());
    v.push(header("Body"));
    v.extend(body_block(d, 10));
    v.extend(part_lines(d, " ", false));
    v.push(Line::default());
    v.push(header("Gear"));
    let mine = pick_choices(g, vis.hero, Pick::Drop);
    let mut shown = 0;
    for id in mine.iter() {
        let Some(it) = g.items.get(id) else { continue };
        let how = if d.held == Some(*id) {
            " (in hand)"
        } else if d.armor.contains(id) || d.clothes.contains(id) {
            " (worn)"
        } else {
            continue;
        };
        v.push(item_line(it, how, false));
        shown += 1;
    }
    if shown == 0 {
        v.push(plain("Empty-handed".into(), MUTED));
    }
    // A crossbow in hand: the bolts left to shoot.
    if d.held.and_then(|id| g.items.get(&id)).map_or(false, |it| it.kind == crate::items::ItemKind::Crossbow) {
        let n = g.bolts(vis.hero).len();
        v.push(plain(format!("{} bolt{} in the quiver{}", n, if n == 1 { "" } else { "s" }, if n > 0 { " (f)" } else { "" }), if n > 0 { TEXT } else { WARN }));
    }
    if mine.len() > shown {
        v.push(plain(format!("{} more in the pack (i)", mine.len() - shown), MUTED));
    }
    // Whatever is in sight, nearest first, foes marked.
    let seen = g.in_sight(vis.hero);
    let people: Vec<&Dwarf> = g.dwarves.iter().filter(|o| o.alive && o.id != d.id && sight.sees(o.pos)).collect();
    if !seen.is_empty() || !people.is_empty() {
        v.push(Line::default());
        v.push(header("In sight"));
        for c in seen.iter().filter_map(|id| g.creatures.get(id)).filter(|c| !c.sneaking).take(5) {
            let info = c.species.info();
            let dist = (c.pos.x - d.pos.x).abs().max((c.pos.y - d.pos.y).abs());
            let name = c.label.clone().unwrap_or_else(|| cap(info.name));
            let (tag, tc) = if c.hostile() && !c.leaving { (" hostile", RED) } else { ("", MUTED) };
            v.push(Line::from(vec![
                Span::styled(format!("{} ", info.glyph), Style::default().fg(rgb(info.color))),
                Span::styled(name, Style::default().fg(TEXT)),
                Span::styled(format!(" {} away", dist), Style::default().fg(MUTED)),
                Span::styled(tag, Style::default().fg(tc)),
            ]));
        }
        for o in people.iter().take(3) {
            v.push(Line::from(vec![Span::styled("☺ ", Style::default().fg(rgb(o.color))), Span::styled(o.name.clone(), Style::default().fg(TEXT))]));
        }
    }
    if !a.company.is_empty() {
        v.push(Line::default());
        v.push(header("Company"));
        for c in &a.company {
            v.push(plain(c.dwarf.name.clone(), TEXT));
        }
    }
    if let Some(q) = a.quests.iter().find(|q| q.state == QuestState::Open) {
        v.push(Line::default());
        v.push(header("Quest"));
        v.push(plain(format!("{:?}: {}", q.kind, q.target), TEXT));
    }
    // The keys, along the bottom.
    let inner_h = r.height.saturating_sub(2) as usize;
    let keys = vec![
        plain("move: arrows · numpad · 7 9 1 3".into(), MUTED),
        plain("l look  g get  i pack  z status".into(), MUTED),
        plain("A aim  f fire  b bind  s sneak".into(), MUTED),
        plain("Z rest  ? help".into(), MUTED),
    ];
    if v.len() + keys.len() < inner_h {
        v.resize(inner_h - keys.len(), Line::default());
    }
    v.extend(keys);
    let p = Paragraph::new(v).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(MUTED)));
    f.render_widget(p, r);
}

/// The sidebar folded into two lines (below 118 columns).
fn draw_strip(f: &mut Frame, r: Rect, s: &AdvScreen) {
    if r.height == 0 {
        return;
    }
    let a = &s.adv;
    let d = a.hero_dwarf();
    let mut l1 = vec![Span::styled(d.called(), Style::default().fg(KEY).add_modifier(Modifier::BOLD)), Span::raw("  ")];
    l1.extend(blood_bar(&d.body, 6));
    if let Some((w, c)) = body_words(d).into_iter().next() {
        l1.push(Span::styled(format!("  {}", w), Style::default().fg(c)));
    }
    if let Some(vis) = a.visit.as_ref() {
        if let Some(it) = d.held.and_then(|id| vis.g.items.get(&id)) {
            l1.push(Span::styled(format!("  holds {}", it.name()), Style::default().fg(TEXT)));
        }
    }
    let place = a.visit.as_ref().map_or(String::new(), |v| v.g.fortress_name.clone());
    let mut l2 = format!("{} · {}", place, cap(clock::time_words(a.now())));
    if s.sneak {
        l2.push_str(" · sneaking");
    }
    l2.push_str("   l look · i pack · z status · ? help");
    f.render_widget(Paragraph::new(vec![Line::from(l1), plain(l2, MUTED)]), r);
}

/// The visit's newest announcements, the flash first; this turn's lines
/// are marked and bright, older ones fade.
fn draw_log(f: &mut Frame, r: Rect, s: &AdvScreen) {
    let block = Block::default().borders(Borders::TOP).border_style(Style::default().fg(MUTED)).title(Span::styled(" Log ", Style::default().fg(MUTED)));
    let inner = block.inner(r);
    f.render_widget(block, r);
    let w = (inner.width as usize).saturating_sub(2).max(10);
    let mut head: Vec<Line> = Vec::new();
    if let Some(fl) = &s.flash {
        head.push(plain(format!("» {}", fl), WARN));
    }
    if s.view == View::Dead {
        head.push(plain("You are dead. Esc returns to the title.".into(), RED));
    }
    let log = s.adv.visit.as_ref().map_or(&s.adv.log, |v| &v.g.log);
    let room = (inner.height as usize).saturating_sub(head.len());
    let mut body: Vec<Line> = Vec::new();
    for e in log.iter().rev() {
        if body.len() >= room {
            break;
        }
        let fresh = e.tick >= s.turn_from && e.tick > 0;
        let col = if fresh { rgb(e.color) } else { dim(rgb(e.color), 0.6) };
        let mut rows = wrap_words(&e.text, w);
        for (i, row) in rows.drain(..).enumerate().collect::<Vec<_>>().into_iter().rev() {
            let mark = if i > 0 { "  " } else if fresh { "» " } else { "  " };
            body.push(Line::from(vec![Span::styled(mark, Style::default().fg(KEY)), Span::styled(row, Style::default().fg(col))]));
        }
    }
    body.truncate(room);
    body.reverse();
    head.extend(body);
    f.render_widget(Paragraph::new(head), inner);
}

/// The look panel: the tile under the cursor and what's on it, if seen.
fn draw_look(f: &mut Frame, r: Rect, g: &Game, sight: &Sight, p: Pos) {
    let mut v = if sight.sees(p) {
        look_lines(g, p, 0).into_iter().filter(|l| !l.spans.iter().any(|s| s.content.contains("Esc back") || s.content.starts_with("o: ") || s.content.starts_with("N: "))).collect()
    } else if g.world.t(p).revealed {
        let mut v = cursor_info_terrain(g, p);
        v.push(Line::default());
        v.push(plain("You remember it so; it is out of sight.".into(), MUTED));
        v
    } else {
        vec![plain("You can't see there.".into(), MUTED)]
    };
    v.push(Line::default());
    v.push(plain("move the cursor · Esc back".into(), MUTED));
    let p = Paragraph::new(super::wrap_lines(v, r.width.saturating_sub(2) as usize))
        .style(Style::default().bg(Color::Rgb(15, 15, 25)))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(KEY)).title(Span::styled(" Look ", Style::default().fg(KEY))));
    f.render_widget(ratatui::widgets::Clear, r);
    f.render_widget(p, r);
}

/// The letter for row `i` of a list.
fn letter(i: usize) -> char {
    (b'a' + i.min(25) as u8) as char
}

/// What the hero carries, with letters: in hand, worn, or in the pack.
fn draw_inventory(f: &mut Frame, area: Rect, s: &AdvScreen, sel: usize) {
    let Some(vis) = s.adv.visit.as_ref() else { return };
    let g = &vis.g;
    let d = &g.dwarves[vis.hero];
    let ids = pick_choices(g, vis.hero, Pick::Drop);
    let mut v: Vec<Line<'static>> = Vec::new();
    if ids.is_empty() {
        v.push(plain("You carry nothing at all.".into(), MUTED));
    }
    let sel = sel.min(ids.len().saturating_sub(1));
    for (i, id) in ids.iter().enumerate().take(26) {
        let Some(it) = g.items.get(id) else { continue };
        let how = if d.held == Some(*id) {
            "  (in hand)"
        } else if d.armor.contains(id) || d.clothes.contains(id) {
            "  (worn)"
        } else {
            ""
        };
        let mut l = item_line(it, how, i == sel);
        l.spans.insert(0, Span::styled(format!(" {} ", letter(i)), Style::default().fg(KEY)));
        v.push(l);
    }
    if s.adv.hero.coins > 0 {
        v.push(Line::default());
        v.push(plain(format!(" {} coins in the purse", s.adv.hero.coins), Color::Rgb(240, 200, 90)));
    }
    v.push(Line::default());
    v.push(plain(" letter/↑↓ choose · w wear/wield · r remove · d drop · Esc back".into(), MUTED));
    let h = v.len() as u16 + 2;
    popup(f, area, 66, h, " Inventory ", v);
}

/// The status page: body and wounds, skills, fame, and the medical history.
fn draw_status(f: &mut Frame, area: Rect, a: &Adventure) {
    let d = a.hero_dwarf();
    let mut v: Vec<Line<'static>> = Vec::new();
    v.push(Line::from(Span::styled(d.called(), Style::default().fg(KEY).add_modifier(Modifier::BOLD))));
    let mut who = format!("A {} {}", race_word(a), calling_word(a));
    if let Some(age) = d.age {
        who.push_str(&format!(", {} years old", age));
    }
    v.push(plain(who, MUTED));
    if let Some(deed) = &d.deed {
        v.push(plain(deed.clone(), Color::Rgb(240, 200, 90)));
    }
    v.push(Line::default());
    v.push(header("Body"));
    v.extend(body_block(d, 20));
    let parts = part_lines(d, "  ", true);
    if parts.is_empty() {
        v.push(plain("  Not a scratch on you.".into(), MUTED));
    }
    v.extend(parts);
    v.push(Line::default());
    v.push(header("Skills"));
    let mut sk: Vec<(u32, String)> = vec![(d.fight_xp, format!("Fighting: {}", skill_rank(d.fight_xp)))];
    for &l in LABORS.iter() {
        let xp = d.skills[l.idx()];
        if xp > 0 {
            sk.push((xp, format!("{}: {}", cap(l.name()), skill_rank(xp))));
        }
    }
    sk.sort_by(|a, b| b.0.cmp(&a.0));
    for pair in sk.chunks(2).take(4) {
        let left = format!("  {:<30}", pair[0].1);
        let right = pair.get(1).map_or(String::new(), |p| p.1.clone());
        v.push(plain(format!("{}{}", left, right), TEXT));
    }
    v.push(Line::default());
    v.push(header("Fame"));
    v.push(plain(format!("  Renown {} · infamy {} · {} slain", a.fame.renown, a.fame.infamy, d.kills), TEXT));
    for deed in a.fame.deeds.iter().rev().take(3) {
        v.push(plain(format!("  {}: {}", deed.year, deed.text), MUTED));
    }
    let hist = history_lines(d, 3, "  ");
    if !hist.is_empty() {
        v.push(Line::default());
        v.push(header("Wounds remembered"));
        v.extend(hist);
    }
    v.push(Line::default());
    v.push(plain("any key: back".into(), MUTED));
    let h = v.len() as u16 + 2;
    popup(f, area, 74, h, " Status ", v);
}

/// A letter list: what to get, drop, wear, take off, whom to attack and
/// where, whom to talk to.
fn draw_pick(f: &mut Frame, area: Rect, g: &Game, hero: usize, kind: Pick, ids: &[u32]) {
    let mut v: Vec<Line<'static>> = Vec::new();
    let me = &g.dwarves[hero];
    for (i, &id) in ids.iter().enumerate().take(26) {
        let key = Span::styled(format!(" {} ", letter(i)), Style::default().fg(KEY));
        let mut l = match kind {
            Pick::Get | Pick::Drop | Pick::Wear | Pick::Remove => {
                let Some(it) = g.items.get(&id) else { continue };
                let how = if me.held == Some(id) {
                    "  (in hand)"
                } else if me.armor.contains(&id) || me.clothes.contains(&id) {
                    "  (worn)"
                } else {
                    ""
                };
                item_line(it, how, false)
            }
            Pick::Aim | Pick::Fire => {
                let Some(c) = g.creatures.get(&id) else { continue };
                let info = c.species.info();
                let hurt = if c.body.badly_hurt() { ", badly hurt" } else { "" };
                Line::from(vec![
                    Span::styled(format!("{} ", info.glyph), Style::default().fg(rgb(info.color))),
                    Span::styled(c.label.clone().unwrap_or_else(|| cap(info.name)), Style::default().fg(TEXT)),
                    Span::styled(format!("{}{}", if c.hostile() { "  hostile" } else { "  peaceful" }, hurt), Style::default().fg(if c.hostile() { RED } else { MUTED })),
                ])
            }
            Pick::AimPart(target) | Pick::FirePart(target) => {
                let aim = AIMS[id as usize];
                let part = aim.part();
                let cid = match target {
                    crate::game::adv::turn::Target::Creature(c) => Some(c),
                    crate::game::adv::turn::Target::Dwarf(_) => None,
                };
                let body = cid.and_then(|c| g.creatures.get(&c)).map(|c| &c.body);
                let gone = body.map_or(false, |b| b.lost[part.idx()]);
                let status = match aim {
                    _ if gone => "gone",
                    Aim::Eyes => body.map_or("", |b| ["", "one put out", "both put out"][b.harm.eyes.min(2) as usize]),
                    Aim::Teeth => body.map_or("", |b| if b.harm.teeth > 0 { "some knocked out" } else { "" }),
                    _ => body.map_or("", |b| b.part_status(part)),
                };
                let odds = cid.map_or(0, |c| g.hit_odds(hero, c, Some(aim), matches!(kind, Pick::FirePart(_))));
                let col = if gone { MUTED } else if odds >= 50 { Color::Rgb(150, 220, 140) } else if odds >= 30 { Color::Rgb(235, 200, 80) } else { WARN };
                Line::from(vec![
                    Span::styled(format!("{:<11}", aim.name()), Style::default().fg(if gone { MUTED } else { TEXT })),
                    Span::styled(if gone { "   -  ".to_string() } else { format!("{:>3}%  ", odds) }, Style::default().fg(col)),
                    Span::styled(format!("{:<24}", aim.hope()), Style::default().fg(MUTED)),
                    Span::styled(if status == "fine" { String::new() } else { status.to_string() }, Style::default().fg(Color::Rgb(235, 200, 80))),
                ])
            }
            Pick::Talk => {
                let Some(o) = g.dwarves.iter().find(|d| d.id == id) else { continue };
                Line::from(vec![Span::styled("☺ ", Style::default().fg(rgb(o.color))), Span::styled(o.name.clone(), Style::default().fg(TEXT))])
            }
        };
        l.spans.insert(0, key);
        v.push(l);
    }
    v.push(Line::default());
    v.push(plain(" a-z choose · Esc cancel".into(), MUTED));
    if kind == Pick::Get {
        v.insert(v.len() - 1, plain(" * take all but the bodies".into(), MUTED));
    }
    let w = if matches!(kind, Pick::AimPart(_) | Pick::FirePart(_)) { 72 } else { 54 };
    let h = v.len() as u16 + 2;
    popup(f, area, w, h, &format!(" {} ", kind.title()), v);
}

/// The menu (A11 adds saving).
pub(super) fn draw_menu(f: &mut Frame, area: Rect, sel: usize) {
    let mut v: Vec<Line<'static>> = Vec::new();
    for (i, m) in MENU.iter().enumerate() {
        let st = if i == sel { Style::default().fg(BLACK).bg(KEY) } else { Style::default().fg(TEXT) };
        v.push(Line::from(Span::styled(format!("  {:<30}", m), st)));
    }
    v.push(Line::default());
    v.push(plain("  ↑↓ choose · Enter · Esc back".into(), MUTED));
    let h = v.len() as u16 + 2;
    popup(f, area, 36, h, " Adventure ", v);
}

/// Every key of the local view.
fn draw_help(f: &mut Frame, area: Rect) {
    let rows = [
        ("1-9", "move (numpad; 7 9 1 3 and Home/End/PgUp/PgDn diagonal)"),
        ("arrows", "move; into a foe to strike it"),
        (". 5", "wait a moment"),
        ("< >", "up or down a stair or ramp"),
        ("A", "attack: a foe in reach, then where: head, eyes, teeth, arm, leg..."),
        ("f", "fire the crossbow: a foe in sight, then where"),
        ("b", "bind your bleeding wounds (cloth from the pack, or your shirt)"),
        ("l", "look: a cursor over the map"),
        ("k", "talk to someone near"),
        ("g d", "get what lies here (* all the dead's gear) · drop something"),
        ("i", "inventory"),
        ("R", "read a book here, or the engraved walls around"),
        ("w r", "wear or wield · take off or put away"),
        ("z", "status: body, wounds, skills, fame"),
        ("Z", "rest until healed or disturbed"),
        ("s", "sneak (slower, quieter)"),
        ("Esc", "menu"),
    ];
    let mut v: Vec<Line<'static>> = rows.iter().map(|(k, d)| key_line(k, d, false)).collect();
    v.push(Line::default());
    v.push(plain("  any key: back".into(), MUTED));
    let h = v.len() as u16 + 2;
    popup(f, area, 70, h, " Keys ", v);
}

fn race_word(a: &Adventure) -> &'static str {
    match a.hero.race {
        crate::overworld::Race::Dwarf => "dwarf",
        crate::overworld::Race::Human => "human",
        crate::overworld::Race::Elf => "elf",
        crate::overworld::Race::Goblin => "goblin",
    }
}

fn calling_word(a: &Adventure) -> String {
    format!("{:?}", a.hero.calling).to_lowercase()
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}
