//! `--window`: the game in a window of square tiles instead of the terminal.
//! The same `ui::draw` paints a ratatui buffer; our backend turns its cells
//! into pixels (text from the system's monospace font, walls, water, trees
//! and box lines from our own tileset), and the window's keys and mouse come
//! back as the crossterm events `App` already handles.

mod backend;
mod glyphs;
mod input;
mod png;
mod tiles;

use crate::app::App;
use crate::ui;
use anyhow::{anyhow, Result};
use backend::TileBackend;
use crossterm::event::{KeyCode, KeyModifiers};
use glyphs::Fonts;
use ratatui::backend::Backend;
use ratatui::Terminal;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::ModifiersState;
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
use winit::window::{Window, WindowId};

/// The cell size a window starts at, in pixels at 100% display scaling.
const START_CELL: usize = 16;

fn opt<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(|s| s.as_str())
}

fn load_fonts(args: &[String]) -> Result<Fonts> {
    Fonts::load(opt(args, "--font")).map_err(|e| anyhow!(e))
}

/// The window and its drawing surface, and the events it has had since they
/// were last handled.
#[derive(Default)]
struct Shell {
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    events: Vec<WindowEvent>,
    error: Option<String>,
    /// The surface's size, to resize it only when the window changes.
    surface_size: (u32, u32),
}

impl ApplicationHandler for Shell {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes().with_title("DwarfClone").with_inner_size(LogicalSize::new(1280.0, 800.0)).with_maximized(true);
        let made = event_loop.create_window(attrs).map_err(|e| e.to_string()).and_then(|w| {
            let w = Rc::new(w);
            let ctx = softbuffer::Context::new(w.clone()).map_err(|e| e.to_string())?;
            let surface = softbuffer::Surface::new(&ctx, w.clone()).map_err(|e| e.to_string())?;
            Ok((w, surface))
        });
        match made {
            Ok((w, s)) => {
                self.window = Some(w);
                self.surface = Some(s);
            }
            Err(e) => self.error = Some(e),
        }
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        self.events.push(event);
    }
}

impl Shell {
    /// Show the backend's picture in the window.
    fn present(&mut self, b: &TileBackend) {
        let Some(surface) = self.surface.as_mut() else { return };
        let (aw, ah) = b.avail;
        let (Some(w), Some(h)) = (NonZeroU32::new(aw as u32), NonZeroU32::new(ah as u32)) else { return };
        if self.surface_size != (aw as u32, ah as u32) {
            if surface.resize(w, h).is_err() {
                return;
            }
            self.surface_size = (aw as u32, ah as u32);
        }
        let Ok(mut buf) = surface.buffer_mut() else { return };
        // The picture catches up with a resized window on the next frame;
        // until then copy what overlaps and leave the rest black.
        let cw = aw.min(b.width);
        for y in 0..ah {
            let row = &mut buf[y * aw..(y + 1) * aw];
            if y < b.height {
                row[..cw].copy_from_slice(&b.pixels[y * b.width..y * b.width + cw]);
                row[cw..].fill(0);
            } else {
                row.fill(0);
            }
        }
        let _ = buf.present();
    }
}

/// Play `app` in a window until it quits or the window is closed.
pub fn play(app: &mut App, args: &[String]) -> Result<()> {
    let fonts = load_fonts(args)?;
    let mut event_loop = EventLoop::new()?;
    let mut shell = Shell::default();
    let t0 = Instant::now();
    while shell.window.is_none() {
        if let Some(e) = shell.error.take() {
            return Err(anyhow!("Couldn't open a window: {}", e));
        }
        if t0.elapsed() > Duration::from_secs(10) {
            return Err(anyhow!("Couldn't open a window."));
        }
        event_loop.pump_app_events(Some(Duration::from_millis(10)), &mut shell);
    }
    let window = shell.window.clone().unwrap();
    // Cells as big on a high-DPI screen as on a plain one, an even number of pixels.
    let cell = ((START_CELL as f64 * window.scale_factor()).round() as usize / 2 * 2).clamp(backend::MIN_CELL, backend::MAX_CELL);
    let size = window.inner_size();
    let backend = TileBackend::new(size.width.max(1) as usize, size.height.max(1) as usize, cell, Box::new(move |c, s| fonts.raster(c, s)));
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    let mut mods = ModifiersState::empty();
    let mut mouse = input::Mouse::default();
    let mut last = Instant::now();
    while !app.quit {
        let start = Instant::now();
        terminal.draw(|f| ui::draw(f, app))?;
        let b = terminal.backend_mut();
        if b.dirty {
            shell.present(b);
            b.dirty = false;
        }
        let draw = start.elapsed().as_secs_f64() * 1000.0;
        loop {
            let left = crate::FRAME.saturating_sub(start.elapsed());
            if let PumpStatus::Exit(_) = event_loop.pump_app_events(Some(left), &mut shell) {
                return Ok(());
            }
            let events: Vec<WindowEvent> = shell.events.drain(..).collect();
            for ev in events {
                let cell = terminal.backend().cell();
                let km = input::modifiers(mods);
                let mut zoom = 0i32;
                match ev {
                    WindowEvent::CloseRequested => return Ok(()),
                    WindowEvent::Resized(s) => {
                        if s.width > 0 && s.height > 0 {
                            terminal.backend_mut().avail = (s.width as usize, s.height as usize);
                        }
                    }
                    WindowEvent::RedrawRequested => shell.present(terminal.backend()),
                    WindowEvent::ModifiersChanged(m) => mods = m.state(),
                    WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                        if let Some(k) = input::key_event(&event.logical_key, mods) {
                            match k.code {
                                // Ctrl and + or - zoom the tiles.
                                KeyCode::Char('+' | '=') if k.modifiers.contains(KeyModifiers::CONTROL) => zoom = 1,
                                KeyCode::Char('-') if k.modifiers.contains(KeyModifiers::CONTROL) => zoom = -1,
                                _ => app.on_key(k),
                            }
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        if let Some(m) = mouse.moved(position.x, position.y, cell, km) {
                            app.on_mouse(m);
                        }
                    }
                    WindowEvent::CursorLeft { .. } => mouse.left(),
                    WindowEvent::MouseInput { state, button, .. } => {
                        let i = match button {
                            winit::event::MouseButton::Left => 0,
                            winit::event::MouseButton::Right => 1,
                            winit::event::MouseButton::Middle => 2,
                            _ => continue,
                        };
                        if let Some(m) = mouse.button(i, state == ElementState::Pressed, cell, km) {
                            app.on_mouse(m);
                        }
                    }
                    WindowEvent::MouseWheel { delta, .. } => {
                        let dy = match delta {
                            MouseScrollDelta::LineDelta(_, y) => y as f64,
                            MouseScrollDelta::PixelDelta(p) => p.y,
                        };
                        if dy != 0.0 {
                            if mods.control_key() {
                                zoom = if dy > 0.0 { 1 } else { -1 };
                            } else if let Some(m) = mouse.wheel(dy > 0.0, cell, km) {
                                app.on_mouse(m);
                            }
                        }
                    }
                    _ => {}
                }
                if zoom != 0 {
                    let b = terminal.backend_mut();
                    let to = (cell as i32 + 2 * zoom).clamp(backend::MIN_CELL as i32, backend::MAX_CELL as i32) as usize;
                    if to != cell {
                        b.set_cell(to);
                        terminal.clear()?;
                    }
                }
                if app.quit {
                    return Ok(());
                }
            }
            if start.elapsed() >= crate::FRAME {
                break;
            }
        }
        let (t, ticks0) = (Instant::now(), app.game.tick);
        app.frame_dt = t.duration_since(last).as_secs_f64();
        last = t;
        app.frame();
        app.sound_frame();
        app.perf_frame(draw, t.elapsed().as_secs_f64() * 1000.0, (app.game.tick - ticks0) as u32);
    }
    Ok(())
}

/// `--window --screenshot FILE [--seed N] [--days D] [--size WxH] [--cell PX]
/// [--plan] [--saga | --sagas] [--adventure [--fight | --travel]]`: watch a seeded fort for a few days, draw it
/// the way the window would, save a PNG and print how long a frame took to draw.
/// `--plan` shows the main level with the plan over it, `--saga` the fort's
/// latest saga on its card, `--sagas` the Sagas screen (a great siege is told
/// first if the fort has no saga yet).
pub fn screenshot(args: &[String]) -> Result<()> {
    let out = opt(args, "--screenshot").ok_or_else(|| anyhow!("--screenshot needs a file name"))?;
    let seed: u64 = opt(args, "--seed").and_then(|s| s.parse().ok()).unwrap_or(7);
    let days: u64 = opt(args, "--days").and_then(|s| s.parse().ok()).unwrap_or(20);
    let cell: usize = opt(args, "--cell").and_then(|s| s.parse().ok()).unwrap_or(START_CELL);
    let (w, h) = opt(args, "--size").and_then(|s| s.split_once('x')).and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?))).unwrap_or((1920usize, 1080usize));
    let fonts = load_fonts(args)?;
    let mut app = App::new(seed);
    // `--adventure`: a seeded adventure, the hero set out from home.
    let adventure = args.iter().any(|a| a == "--adventure");
    if adventure {
        app.start_adventure_seeded(seed);
    } else {
        app.start_watch_seeded(seed);
    }
    while app.mode == crate::app::Mode::Loading {
        app.frame();
        std::thread::sleep(Duration::from_millis(20));
    }
    // (`--fight`: a goblin raider comes at the hero, for the combat log.)
    if adventure && args.iter().any(|a| a == "--fight") {
        app.adventure_skirmish();
    }
    // (`--travel`: out on the road for a few days, wounded, for the travel view.)
    if adventure && args.iter().any(|a| a == "--travel") {
        app.adventure_journey();
    }
    for _ in 0..if adventure { 0 } else { days * crate::game::DAY } {
        app.game.step();
    }
    // (`--plan`: down on the main level of halls, the plan shown over it;
    // `--halls`: the same view without the plan, to see the halls as built.)
    let halls = args.iter().any(|a| a == "--halls");
    if halls || args.iter().any(|a| a == "--plan") {
        if let Some(b) = app.game.overseer.as_ref().and_then(|o| o.base) {
            // (Centred on the main level's halls, some way along them.)
            let lp = app.game.plan_at(b.z).cloned();
            let mid = lp.as_ref().map_or(b.offset(24, 0, 0), |l| l.facing.at(b.offset(14, 0, 0), 14, 0));
            app.cursor = mid;
            app.view_x = mid.x - 48;
            app.view_y = mid.y - 26;
            app.free_view = false;
            app.ghost = !halls;
            if let Some(l) = lp {
                println!("Main level: {} facing {}, {} levels planned.", l.layout.name(), l.facing.name(), app.game.overseer.as_ref().map_or(0, |o| o.levels.len()));
            }
        }
    }
    let card = args.iter().any(|a| a == "--saga");
    if card || args.iter().any(|a| a == "--sagas") {
        if app.game.told_sagas().is_empty() {
            app.game.saga_demo();
        }
        app.game.spotlights.clear();
        if card {
            let s = app.game.told_sagas()[0];
            let color = if s.doom { (220, 50, 60) } else { (240, 190, 70) };
            let spot = crate::game::spotlight::Spotlight { title: "A saga is told".into(), text: s.title.clone(), at: None, color, saga: Some(s.id), actions: Vec::new() };
            app.spot = Some((spot, 10_000));
        } else {
            app.mode = crate::app::Mode::Sagas { sel: 0, open: false, from_end: false };
        }
    }
    let mut terminal = Terminal::new(TileBackend::new(w, h, cell, Box::new(move |c, s| fonts.raster(c, s))))?;
    terminal.draw(|f| ui::draw(f, &mut app))?;
    // A whole screen drawn from nothing, a few times over, glyphs cached...
    let n = 10;
    let t = Instant::now();
    for _ in 0..n {
        terminal.clear()?;
        terminal.draw(|f| ui::draw(f, &mut app))?;
    }
    let full = t.elapsed().as_secs_f64() * 1000.0 / n as f64;
    // ...and frames as play goes on, where only what changed is painted.
    let t = Instant::now();
    for _ in 0..n {
        if adventure {
            app.frame();
        } else {
            app.game.step();
        }
        terminal.draw(|f| ui::draw(f, &mut app))?;
    }
    let frame = t.elapsed().as_secs_f64() * 1000.0 / n as f64;
    let b = terminal.backend();
    std::fs::write(out, png::encode(&b.pixels, b.width, b.height))?;
    let grid = b.size()?;
    println!(
        "Saved {} ({}x{} px, {}x{} cells of {} px). Drawing the whole screen: {:.1} ms; a frame of play: {:.1} ms.",
        out, b.width, b.height, grid.width, grid.height, cell, full, frame
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adventure_screenshot_draws() {
        let ow = crate::overworld::Overworld::generate(5);
        let mut app = App::new(5);
        app.adventure_ready(Box::new(crate::game::adv::quick_start(ow, 5)));
        // A stand-in font: every glyph a block in the middle of its cell.
        let font = Box::new(|_: char, s: usize| {
            let mut m = vec![0; s * s];
            for y in s / 4..s * 3 / 4 {
                for x in s / 4..s * 3 / 4 {
                    m[y * s + x] = 255;
                }
            }
            m
        });
        let mut term = Terminal::new(TileBackend::new(130 * 16, 40 * 16, 16, font)).unwrap();
        term.draw(|f| ui::draw(f, &mut app)).unwrap();
        let b = term.backend();
        assert!(b.pixels.iter().any(|&p| p != 0), "a frame was drawn");
        let race = app.adv.as_ref().unwrap().adv.hero.race.color();
        let ratatui::style::Color::Rgb(r, g, bl) = race else { panic!("an RGB colour") };
        let lit = (r as u32) << 16 | (g as u32) << 8 | bl as u32;
        assert!(b.pixels.iter().any(|&p| p == lit), "the hero's cell is lit in their people's colour");
    }
}
