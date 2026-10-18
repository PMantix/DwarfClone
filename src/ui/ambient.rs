//! Ambient particles over the open ground of the map: rain streaks, snow
//! drifting, leaves falling under the trees in autumn, fireflies at summer
//! dusk, dust blowing over dry land. Purely for the eye and cheap: a few
//! hundred points worked out from a clock (no state per particle), only on
//! open tiles under the sky and only over bare ground, never over a dwarf,
//! a creature, an item or a building. They stand still while the game is
//! paused. If drawing them ever costs more than `BUDGET_MS` a frame they
//! switch themselves off; the F3 overlay says which are falling and what
//! they cost.

use super::rgb;
use crate::app::App;
use crate::game::{Game, Weather, DAY, SEASON, YEAR};
use crate::geom::Pos;
use crate::overworld::Biome;
use crate::world::TileKind;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::Frame;
use std::cell::Cell;

/// What's in the air.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Air {
    Rain,
    BloodRain,
    Snow,
    Leaves,
    Fireflies,
    Dust,
}

impl Air {
    pub fn name(self) -> &'static str {
        match self {
            Air::Rain => "rain",
            Air::BloodRain => "blood rain",
            Air::Snow => "snow",
            Air::Leaves => "falling leaves",
            Air::Fireflies => "fireflies",
            Air::Dust => "dust",
        }
    }

    /// Screen cells per particle.
    fn sparseness(self) -> i32 {
        match self {
            Air::Rain | Air::BloodRain => 28,
            Air::Snow => 40,
            Air::Leaves => 90,
            Air::Fireflies => 120,
            Air::Dust => 110,
        }
    }
}

/// The most a frame's particles may cost before they turn themselves off.
pub const BUDGET_MS: f64 = 2.0;

thread_local! {
    /// Seconds of unpaused time, for moving the particles.
    static CLOCK: Cell<f64> = const { Cell::new(0.0) };
    /// What's falling, its cost a frame (smoothed, ms), and whether it's off.
    static COST: Cell<(Option<Air>, f64, bool)> = const { Cell::new((None, 0.0, false)) };
}

/// What the air holds now, if anything worth drawing.
pub fn air(g: &Game) -> Option<Air> {
    let season = (g.tick % YEAR) / SEASON;
    let dusk = g.tick % DAY >= DAY * 3 / 4;
    let dry = matches!(g.biome, Biome::Desert | Biome::Badlands | Biome::Savanna | Biome::Shrubland) || g.drought;
    match g.weather {
        Weather::Rain | Weather::Storm => Some(Air::Rain),
        Weather::BloodRain => Some(Air::BloodRain),
        Weather::Snow | Weather::Blizzard => Some(Air::Snow),
        Weather::Clear if season == 1 && dusk && !dry && !g.biome.cold() => Some(Air::Fireflies),
        Weather::Clear if season == 2 && !matches!(g.biome, Biome::Desert | Biome::Glacier | Biome::Tundra) => Some(Air::Leaves),
        Weather::Clear if dry && season <= 1 => Some(Air::Dust),
        _ => None,
    }
}

fn hash(i: u32, k: u32) -> u32 {
    let mut h = i.wrapping_mul(0x9E37_79B9) ^ k.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^ (h >> 12)
}

/// Where each particle is at time `t` in a `w`×`h` view, with its look.
/// Positions are screen cells; the caller keeps only those over open sky.
pub fn particles(kind: Air, w: i32, h: i32, t: f64) -> Vec<(i32, i32, char, (u8, u8, u8))> {
    if w <= 0 || h <= 0 {
        return Vec::new();
    }
    let n = ((w * h) / kind.sparseness()).max(1) as u32;
    let mut v = Vec::with_capacity(n as usize);
    for i in 0..n {
        let (a, b, c) = (hash(i, 1), hash(i, 2), hash(i, 3));
        let x0 = (a % w as u32) as f64;
        let y0 = (b % h as u32) as f64;
        let phase = (c % 1000) as f64 / 1000.0 * std::f64::consts::TAU;
        let wrap = |v: f64, m: i32| v.rem_euclid(m as f64) as i32;
        let p = match kind {
            Air::Rain | Air::BloodRain => {
                // Fast, a little slanted by the wind.
                let y = y0 + t * 18.0;
                let x = x0 + t * 4.0;
                let col = if kind == Air::Rain { (110, 150, 230) } else { (210, 40, 40) };
                (wrap(x, w), wrap(y, h), if i % 3 == 0 { '¦' } else { '\'' }, col)
            }
            Air::Snow => {
                let y = y0 + t * 2.0;
                let x = x0 + (t * 1.1 + phase).sin() * 1.5 + t * 0.6;
                (wrap(x, w), wrap(y, h), if i % 4 == 0 { '*' } else { '·' }, (235, 240, 255))
            }
            Air::Leaves => {
                let y = y0 + t * 1.3;
                let x = x0 + (t * 0.9 + phase).sin() * 2.0 + t * 0.8;
                let cols = [(220, 120, 40), (200, 70, 30), (230, 190, 60), (170, 100, 40)];
                (wrap(x, w), wrap(y, h), [',', '`', '\'', ','][(a % 4) as usize], cols[(b % 4) as usize])
            }
            Air::Fireflies => {
                // Wander about their spot and blink.
                if (t * 2.3 + phase * 3.0).sin() < 0.2 {
                    continue;
                }
                let x = x0 + (t * 0.5 + phase).sin() * 2.0;
                let y = y0 + (t * 0.4 + phase * 1.7).cos() * 1.2;
                (wrap(x, w), wrap(y, h), '·', (200, 255, 90))
            }
            Air::Dust => {
                let x = x0 + t * 5.0;
                let y = y0 + (t * 0.7 + phase).sin() * 0.8;
                (wrap(x, w), wrap(y, h), if i % 5 == 0 { '∙' } else { '·' }, (190, 170, 120))
            }
        };
        v.push(p);
    }
    v
}

/// Glyphs a particle may pass over: bare ground and plants, not things.
fn bare(sym: &str) -> bool {
    matches!(sym, "." | "," | "`" | "'" | "·" | "\"" | ";" | " " | "♣" | "♠" | "τ" | "↑" | "Γ" | "♧" | "≈" | "~" | "-" | "=" | "*" | "▒" | "░")
}

/// Draw the particles over the map view `inner` (after the map is drawn).
pub(super) fn draw(f: &mut Frame, app: &App, inner: Rect) {
    let g = &app.game;
    let Some(kind) = air(g) else {
        COST.with(|c| c.set((None, 0.0, false)));
        return;
    };
    let (prev, avg, off) = COST.with(|c| c.get());
    // New weather gets a fresh chance.
    let (avg, off) = if prev == Some(kind) { (avg, off) } else { (0.0, false) };
    if off {
        COST.with(|c| c.set((Some(kind), avg, true)));
        return;
    }
    let t0 = std::time::Instant::now();
    let t = CLOCK.with(|c| {
        if !app.paused {
            c.set(c.get() + app.frame_dt);
        }
        c.get()
    });
    let z = app.cursor.z;
    let (vw, vh) = (inner.width as i32, inner.height as i32);
    let cursor = Color::Rgb(220, 200, 0);
    let buf = f.buffer_mut();
    for (sx, sy, ch, col) in particles(kind, vw, vh, t) {
        let p = Pos::new(app.view_x + sx, app.view_y + sy, z);
        if !g.world.in_bounds(p) {
            continue;
        }
        let tile = g.world.t(p);
        if !tile.revealed || tile.kind.solid() || !g.world.outside(p) {
            continue;
        }
        // Leaves only fall under trees: one in the four tiles above.
        if kind == Air::Leaves && !(1..=4).any(|k| g.world.get(Pos::new(p.x, p.y - k, z)).map_or(false, |t| t.kind == TileKind::Tree)) {
            continue;
        }
        let cell = buf.get_mut(inner.x + sx as u16, inner.y + sy as u16);
        if cell.bg == cursor || !bare(cell.symbol()) {
            continue;
        }
        cell.set_char(ch).set_fg(rgb(col));
    }
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    COST.with(|c| c.set(judge((Some(kind), avg, false), ms)));
}

/// The running cost after a frame that took `ms`: smoothed, so one slow
/// frame (the first, a hitch) doesn't switch the particles off, but a run
/// of them does, until the weather changes.
fn judge(cost: (Option<Air>, f64, bool), ms: f64) -> (Option<Air>, f64, bool) {
    let avg = cost.1 * 0.9 + ms * 0.1;
    (cost.0, avg, cost.2 || avg > BUDGET_MS)
}

/// For the F3 overlay: what's in the air and what it costs.
pub(super) fn perf_note() -> String {
    match COST.with(|c| c.get()) {
        (None, _, _) => "ambient: still air".into(),
        (Some(k), _, true) => format!("ambient: {} off (over {:.1} ms a frame)", k.name(), BUDGET_MS),
        (Some(k), ms, false) => format!("ambient: {} {:.2} ms", k.name(), ms),
    }
}

#[cfg(test)]
pub(super) fn reset_for_test() {
    CLOCK.with(|c| c.set(0.0));
    COST.with(|c| c.set((None, 0.0, false)));
}

#[cfg(test)]
mod tests {
    use super::super::draw as draw_all;
    use super::*;
    use crate::app::Mode;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn map_cells(app: &mut App, w: u16, h: u16) -> Vec<String> {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| draw_all(f, app)).unwrap();
        let buf = term.backend().buffer().clone();
        let r = app.map_rect;
        (r.y..r.y + r.height).map(|y| (r.x..r.x + r.width).map(|x| buf.get(x, y).symbol().to_string()).collect()).collect()
    }

    #[test]
    fn the_weather_and_season_choose_what_falls() {
        let mut g = Game::new(5);
        g.weather = Weather::Snow;
        assert_eq!(air(&g), Some(Air::Snow));
        g.weather = Weather::Rain;
        assert_eq!(air(&g), Some(Air::Rain));
        g.weather = Weather::Clear;
        g.tick = SEASON * 2 + 10;
        if !matches!(g.biome, Biome::Desert | Biome::Glacier | Biome::Tundra) {
            assert_eq!(air(&g), Some(Air::Leaves));
        }
        // Particles move with the clock and stay on screen.
        let a = particles(Air::Snow, 80, 20, 0.0);
        let b = particles(Air::Snow, 80, 20, 3.0);
        assert_eq!(a.len(), 80 * 20 / 40);
        assert_ne!(a, b);
        assert!(b.iter().all(|p| p.0 >= 0 && p.0 < 80 && p.1 >= 0 && p.1 < 20));
    }

    #[test]
    fn snow_falls_on_the_surface_only_and_cheaply() {
        let mut app = App::with_game_for_tests(Game::new(5));
        app.mode = Mode::Normal;
        app.game.weather = Weather::Snow;
        app.game.tick = 0;
        app.paused = false;
        // Keep the old flicker of falling weather off, to see only ours.
        let surface = app.game.world.surface_z(app.cursor.x, app.cursor.y);
        for (w, h) in [(80u16, 24u16), (200, 60)] {
            reset_for_test();
            app.cursor.z = surface;
            app.frame_dt = 0.0;
            let still = map_cells(&mut app, w, h);
            app.frame_dt = 0.7;
            let moved = map_cells(&mut app, w, h);
            assert_ne!(still, moved, "the snow drifts at {}x{}", w, h);
            let flakes = moved.iter().map(|r| r.matches('*').count()).sum::<usize>();
            assert!(flakes > 0, "snowflakes on the surface at {}x{}", w, h);
            // Underground, below the lowest ground anywhere, nothing falls.
            let world = &app.game.world;
            let lowest = (0..world.h).flat_map(|y| (0..world.w).map(move |x| (x, y))).map(|(x, y)| world.surface_z(x, y)).min().unwrap();
            app.cursor.z = (lowest - 2).max(0);
            let deep = map_cells(&mut app, w, h);
            let deep2 = map_cells(&mut app, w, h);
            assert_eq!(deep, deep2, "nothing drifts below ground");
        }
        // Cheap: the smoothed cost is well under the budget.
        app.cursor.z = surface;
        reset_for_test();
        for _ in 0..20 {
            map_cells(&mut app, 200, 60);
        }
        let (k, ms, off) = COST.with(|c| c.get());
        assert_eq!(k, Some(Air::Snow));
        assert!(!off && ms < BUDGET_MS, "{} ms", ms);
        assert!(perf_note().starts_with("ambient: snow"));
    }

    #[test]
    fn one_slow_frame_is_forgiven_but_a_run_of_them_is_not() {
        let snow = (Some(Air::Snow), 0.0, false);
        let after_hitch = judge(snow, 15.0);
        assert!(!after_hitch.2, "{:?}", after_hitch);
        let calm = (0..30).fold(after_hitch, |c, _| judge(c, 0.2));
        assert!(!calm.2 && calm.1 < 1.0);
        let slow = (0..30).fold(calm, |c, _| judge(c, 5.0));
        assert!(slow.2, "{:?}", slow);
        // Off stays off for this weather; drawing with new weather starts over.
        assert!(judge(slow, 0.0).2);
        let mut app = App::with_game_for_tests(Game::new(5));
        app.mode = Mode::Normal;
        app.game.weather = Weather::Rain;
        app.cursor.z = app.game.world.surface_z(app.cursor.x, app.cursor.y);
        COST.with(|c| c.set(slow));
        map_cells(&mut app, 80, 24);
        let (k, _, off) = COST.with(|c| c.get());
        assert_eq!((k, off), (Some(Air::Rain), false));
    }

    #[test]
    fn particles_stand_still_while_paused() {
        let mut app = App::with_game_for_tests(Game::new(5));
        app.mode = Mode::Normal;
        app.game.weather = Weather::Snow;
        app.cursor.z = app.game.world.surface_z(app.cursor.x, app.cursor.y);
        // (Dwarves sharing a tile take turns on the wall clock: none here.)
        app.game.dwarves.clear();
        reset_for_test();
        app.paused = true;
        app.frame_dt = 0.5;
        let a = map_cells(&mut app, 120, 40);
        let b = map_cells(&mut app, 120, 40);
        assert_eq!(a, b);
    }
}
