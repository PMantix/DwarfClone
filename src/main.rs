mod app;
mod body;
mod buildings;
mod creature;
mod dwarf;
mod game;
mod geom;
mod items;
mod jobs;
mod material;
mod noise;
mod overworld;
mod path;
mod rng;
mod saves;
mod terrain;
mod title;
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
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--autoplay") {
        autoplay_batch(&args);
        return Ok(());
    }
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
    if result.is_ok() && !app.game.is_placeholder() {
        println!("Seed {} — {} dwarves survived to {}.", seed, app.game.alive_count(), app.game.date_string());
    }
    result
}

/// `--autoplay [--runs N] [--years Y] [--style NAME] [--seed S]`: play many
/// fortresses with the overseer, side by side, and tell how each ended.
fn autoplay_batch(args: &[String]) {
    use game::{run_trial, Style};
    let opt = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let runs: u64 = opt("--runs").and_then(|s| s.parse().ok()).unwrap_or(10);
    let years: u64 = opt("--years").and_then(|s| s.parse().ok()).unwrap_or(5);
    let style = opt("--style").and_then(|s| Style::parse(&s));
    let base: u64 = opt("--seed").and_then(|s| s.parse().ok()).unwrap_or_else(|| SystemTime::now().duration_since(UNIX_EPOCH).map_or(1, |d| d.as_secs() % 100_000));
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).max(1);
    println!(
        "Autoplay: {} fortresses, up to {} years each, {} at a time{}",
        runs,
        years,
        threads,
        style.map_or(String::new(), |s| format!(", all {}", s.name()))
    );
    let jobs: Vec<(u64, Style)> = (0..runs).map(|i| (base + i, style.unwrap_or(Style::ALL[(i % Style::ALL.len() as u64) as usize]))).collect();
    let queue = std::sync::Arc::new(std::sync::Mutex::new(jobs.into_iter()));
    let (tx, rx) = std::sync::mpsc::channel();
    let mut handles = Vec::new();
    for _ in 0..threads.min(runs as usize) {
        let queue = queue.clone();
        let tx = tx.clone();
        handles.push(std::thread::spawn(move || loop {
            let next = queue.lock().unwrap().next();
            let Some((seed, style)) = next else { break };
            let _ = tx.send(run_trial(seed, style, years));
        }));
    }
    drop(tx);
    let mut all = Vec::new();
    for t in rx {
        println!(
            "  {:<8} {:<9} seed {:<6} {:<15} {:>4.1} yrs  pop {:>2}/{:<2}  ☼{:>7}  ★{:<2} heroes {:<2} {}",
            t.verdict,
            t.style.name(),
            t.seed,
            t.fort,
            t.years,
            t.alive,
            t.peak_pop,
            t.peak_wealth,
            t.milestones.len(),
            t.heroes.len(),
            t.epitaph
        );
        if !t.ambitions.is_empty() {
            println!("           its ambitions: {}", t.ambitions.join(" → "));
        }
        all.push(t);
    }
    for h in handles {
        let _ = h.join();
    }
    println!();
    println!("  {:<9} {:>5} {:>6} {:>6} {:>8} {:>10}", "style", "runs", "glory", "ruin", "avg yrs", "avg ☼peak");
    for s in Style::ALL {
        let mine: Vec<_> = all.iter().filter(|t| t.style == s).collect();
        if mine.is_empty() {
            continue;
        }
        let n = mine.len() as f64;
        println!(
            "  {:<9} {:>5} {:>6} {:>6} {:>8.1} {:>10.0}",
            s.name(),
            mine.len(),
            mine.iter().filter(|t| t.verdict == "Glory").count(),
            mine.iter().filter(|t| t.verdict == "Ruin").count(),
            mine.iter().map(|t| t.years).sum::<f64>() / n,
            mine.iter().map(|t| t.peak_wealth as f64).sum::<f64>() / n
        );
    }
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
        app.frame();
    }
    Ok(())
}
