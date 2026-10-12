mod app;
mod buildings;
mod dwarf;
mod game;
mod geom;
mod items;
mod material;
mod noise;
mod path;
mod rng;
mod ui;
mod world;

use anyhow::Result;
use app::App;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const FRAME: Duration = Duration::from_millis(33);

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
}

fn main() -> Result<()> {
    let seed = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or_else(|| SystemTime::now().duration_since(UNIX_EPOCH).map_or(1, |d| d.as_secs()));

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        default_hook(info);
    }));

    let mut app = App::new(seed);

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.hide_cursor()?;

    let result = run(&mut terminal, &mut app);

    restore_terminal();
    terminal.show_cursor()?;
    if result.is_ok() {
        println!("Seed {} — {} dwarves survived to {}.", seed, app.game.alive_count(), app.game.date_string());
    }
    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    while !app.quit {
        let start = Instant::now();
        terminal.draw(|f| ui::draw(f, app))?;
        loop {
            let left = FRAME.saturating_sub(start.elapsed());
            if !event::poll(left)? {
                break;
            }
            match event::read()? {
                // Windows reports key releases too; only act on presses.
                Event::Key(k) if k.kind != KeyEventKind::Release => app.on_key(k),
                Event::Mouse(m) => app.on_mouse(m),
                _ => {}
            }
            if app.quit {
                return Ok(());
            }
        }
        if !app.paused {
            for _ in 0..app.ticks_per_frame() {
                app.game.step();
                if app.game.pause_request {
                    app.game.pause_request = false;
                    app.paused = true;
                    break;
                }
            }
        }
    }
    Ok(())
}
