mod app;
mod arts;
mod audio;
mod bestiary;
mod body;
mod buildings;
mod creature;
mod director;
mod dwarf;
mod game;
mod genshow;
mod geom;
mod items;
mod jobs;
mod material;
mod noise;
mod overworld;
mod path;
mod report;
mod rng;
mod saves;
mod terrain;
mod terrain_gen;
mod title;
mod ui;
#[cfg(feature = "window")]
mod window;
mod world;
mod worldgen;

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
    if let Some(i) = args.iter().position(|a| a == "--compare") {
        // `--compare before.json after.json`: two saved batches side by side.
        let load = |p: Option<&String>| -> Result<Vec<game::Trial>> {
            let p = p.ok_or_else(|| anyhow::anyhow!("--compare needs two batch files (from --out)"))?;
            Ok(serde_json::from_str(&std::fs::read_to_string(p)?)?)
        };
        let (a, b) = (load(args.get(i + 1))?, load(args.get(i + 2))?);
        let years = a.iter().chain(b.iter()).map(|t| t.alive_by_year.len()).max().unwrap_or(1).max(1);
        print!("{}", report::compare(&a, &b, years));
        return Ok(());
    }
    if args.iter().any(|a| a == "--sound-check") {
        audio::check();
        return Ok(());
    }
    if args.iter().any(|a| a == "--autoplay") {
        autoplay_batch(&args);
        return Ok(());
    }
    if args.iter().any(|a| a == "--adventure-trial") {
        game::adv::bot::trial_batch(&args);
        return Ok(());
    }
    let seed = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or_else(|| SystemTime::now().duration_since(UNIX_EPOCH).map_or(1, |d| d.as_secs()));

    let windowed = args.iter().any(|a| a == "--window");
    #[cfg(feature = "window")]
    if windowed && args.iter().any(|a| a == "--screenshot") {
        return window::screenshot(&args);
    }
    #[cfg(not(feature = "window"))]
    if windowed {
        anyhow::bail!("This build has no window mode (it was built without the `window` feature).");
    }

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        default_hook(info);
    }));

    let mut app = App::new(seed);
    // Sound only for the game itself (never the batch runs above, nor tests).
    app.audio = audio::Audio::start(&app.save_dir);

    #[cfg(feature = "window")]
    if windowed {
        let result = window::play(&mut app, &args);
        if result.is_ok() && !app.game.is_placeholder() {
            println!("Seed {} — {} dwarves survived to {}.", seed, app.game.alive_count(), app.game.date_string());
        }
        return result;
    }

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    // One write to the console a frame: unbuffered, a level change on a big
    // map is hundreds of small writes, which Windows consoles are slow at.
    let mut terminal = Terminal::new(CrosstermBackend::new(io::BufWriter::with_capacity(1 << 20, io::stdout())))?;
    terminal.hide_cursor()?;

    let result = run(&mut terminal, &mut app);

    restore_terminal();
    terminal.show_cursor()?;
    if result.is_ok() && !app.game.is_placeholder() {
        println!("Seed {} — {} dwarves survived to {}.", seed, app.game.alive_count(), app.game.date_string());
    }
    result
}

/// A fort's sagas, for `--sagas`.
fn print_sagas(sagas: &[game::sagas::WorldSaga]) {
    for s in sagas {
        let glyphs: String = s.chapters.iter().map(|c| c.1.glyph(s.doom && c.1 == game::sagas::Beat::Ending)).collect();
        let state = if s.fell {
            "ended the fort"
        } else if s.closing.is_empty() {
            "open"
        } else if s.doom {
            "grief"
        } else {
            "glory"
        };
        println!("           ☼ {} [{}] {} ({})", s.title, s.kind.name(), glyphs, state);
        for (date, _, text) in &s.chapters {
            println!("               {}: {}", date, text);
        }
        if !s.closing.is_empty() {
            println!("               ~ {} ~", s.closing);
        }
    }
    println!();
}

/// How many sagas the batch's forts told, and of which kinds.
fn sagas_summary(all: &[game::Trial]) {
    let n: usize = all.iter().map(|t| t.sagas.len()).sum();
    let mut kinds: std::collections::BTreeMap<String, (usize, usize, usize)> = std::collections::BTreeMap::new();
    for s in all.iter().flat_map(|t| t.sagas.iter()) {
        let e = kinds.entry(s.kind.name().to_string()).or_default();
        e.0 += 1;
        if s.doom {
            e.1 += 1;
        }
        if s.fell {
            e.2 += 1;
        }
    }
    let per: Vec<String> = all.iter().map(|t| t.sagas.len().to_string()).collect();
    println!("\n  Sagas: {} in {} forts ({:.1} a fort; per fort {}).", n, all.len(), n as f64 / all.len().max(1) as f64, per.join(" "));
    for (k, (c, doom, fell)) in kinds {
        println!("    {:<16} {:>3}  ({} in grief, {} ended the fort)", k, c, doom, fell);
    }
    let fallen = all.iter().filter(|t| t.alive == 0).count();
    let led = all.iter().filter(|t| t.alive == 0 && t.sagas.iter().any(|s| s.fell)).count();
    println!("    Fallen forts led by the saga that ended them: {} of {}.", led, fallen);
}

/// `--autoplay [--runs N] [--years Y] [--style NAME] [--traits A,B] [--seed S] [--story]
/// [--out FILE] [--knob name=v]... [--knobs FILE] [--list-knobs]`: play many
/// fortresses with the overseer, side by side, tell how each ended and why
/// forts fell; `--story` adds each fort's timeline, `--out` saves the batch
/// for `--compare`. `--knob` (any number of them) and `--knobs` (a flat JSON
/// map) change tunable numbers from their defaults; `--list-knobs` plays one
/// fort for 30 days and lists every knob it read.
fn autoplay_batch(args: &[String]) {
    use game::{Knobs, Style};
    let opt = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let mut knobs = Knobs::default();
    if let Some(path) = opt("--knobs") {
        match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|s| game::knobs::parse_knobs_file(&s)) {
            Ok(m) => knobs.set.extend(m),
            Err(e) => {
                eprintln!("Couldn't read the knobs in {}: {}", path, e);
                return;
            }
        }
    }
    for (i, a) in args.iter().enumerate() {
        if a == "--knob" {
            match args.get(i + 1).map(|s| game::knobs::parse_knob(s)) {
                Some(Ok((k, v))) => {
                    knobs.set.insert(k, v);
                }
                Some(Err(e)) => {
                    eprintln!("--knob: {}", e);
                    return;
                }
                None => {
                    eprintln!("--knob needs name=value");
                    return;
                }
            }
        }
    }
    if args.iter().any(|a| a == "--list-knobs") {
        let seed = opt("--seed").and_then(|s| s.parse().ok()).unwrap_or(1);
        let _ = game::trial_game_with(seed, Style::Steady, 30, &knobs);
        println!("Knobs read in 30 days of a Steady fort (name, default):");
        for (k, v) in game::knobs::seen() {
            println!("  {:<28} {}", k, v);
        }
        return;
    }
    let runs: u64 = opt("--runs").and_then(|s| s.parse().ok()).unwrap_or(10);
    let years: u64 = opt("--years").and_then(|s| s.parse().ok()).unwrap_or(5);
    let style = opt("--style").and_then(|s| Style::parse(&s));
    // `--traits cautious,vain`: every overseer of the batch has these two.
    let traits: Option<Vec<game::personality::Trait>> = opt("--traits").map(|t| t.split(',').filter_map(game::personality::Trait::parse).collect());
    let base: u64 = opt("--seed").and_then(|s| s.parse().ok()).unwrap_or_else(|| SystemTime::now().duration_since(UNIX_EPOCH).map_or(1, |d| d.as_secs() % 100_000));
    let threads = opt("--threads").and_then(|s| s.parse().ok()).unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get())).max(1);
    println!(
        "Autoplay: {} fortresses, up to {} years each, {} at a time{}{}",
        runs,
        years,
        threads,
        style.map_or(String::new(), |s| format!(", all {}", s.name())),
        if knobs.set.is_empty() { String::new() } else { format!("; knobs: {}", knobs.describe()) }
    );
    if let Some(t) = &traits {
        println!("Every overseer is {}.", t.iter().map(|t| t.name()).collect::<Vec<_>>().join(" and "));
    }
    let jobs: Vec<(u64, Style)> = (0..runs).map(|i| (base + i, style.unwrap_or(Style::ALL[(i % Style::ALL.len() as u64) as usize]))).collect();
    let queue = std::sync::Arc::new(std::sync::Mutex::new(jobs.into_iter()));
    let (tx, rx) = std::sync::mpsc::channel();
    let mut handles = Vec::new();
    for _ in 0..threads.min(runs as usize) {
        let queue = queue.clone();
        let tx = tx.clone();
        let knobs = knobs.clone();
        let traits = traits.clone();
        handles.push(std::thread::spawn(move || {
            loop {
                let next = queue.lock().unwrap().next();
                let Some((seed, style)) = next else { break };
                let _ = tx.send(game::run_trial_with(seed, style, years, &knobs, traits.clone()));
            }
            // The knobs this thread's forts read, to catch a misspelt --knob.
            game::knobs::seen()
        }));
    }
    drop(tx);
    let story = args.iter().any(|a| a == "--story");
    let sagas = args.iter().any(|a| a == "--sagas");
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
        if story {
            for (day, what) in &t.story {
                println!("           y{} d{:<3} {}", day / (game::YEAR / game::DAY) + 1, day % (game::YEAR / game::DAY), what);
            }
            println!();
        } else if !t.ambitions.is_empty() {
            println!("           {}: {}", t.overseer, t.ambitions.join(" → "));
        }
        if sagas {
            print_sagas(&t.sagas);
        }
        all.push(t);
        // Saved as it goes, so a batch cut short keeps what it has.
        if let Some(path) = opt("--out") {
            let _ = serde_json::to_string(&all).map(|s| std::fs::write(&path, s));
        }
    }
    let mut read = std::collections::BTreeSet::new();
    for h in handles {
        if let Ok(seen) = h.join() {
            read.extend(seen.into_keys());
        }
    }
    for k in knobs.set.keys().filter(|k| !read.contains(k.as_str())) {
        println!("  Warning: no fort ever read the knob {} (misspelt? see --list-knobs).", k);
    }
    all.sort_by_key(|t| t.seed);
    if sagas {
        sagas_summary(&all);
    }
    print!("{}", report::report(&all, years as usize));
    if let Some(path) = opt("--out") {
        match serde_json::to_string(&all).map_err(anyhow::Error::from).and_then(|s| std::fs::write(&path, s).map_err(anyhow::Error::from)) {
            Ok(()) => println!("\n  Saved the batch to {} (compare two with --compare A B).", path),
            Err(e) => println!("\n  Couldn't save the batch to {}: {}", path, e),
        }
    }
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::BufWriter<io::Stdout>>>, app: &mut App) -> Result<()> {
    let mut last = Instant::now();
    while !app.quit {
        let start = Instant::now();
        terminal.draw(|f| ui::draw(f, app))?;
        let draw = start.elapsed().as_secs_f64() * 1000.0;
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
        let (t, ticks0) = (Instant::now(), app.game.tick);
        // The director keeps time by the clock, not by frames.
        app.frame_dt = t.duration_since(last).as_secs_f64();
        last = t;
        app.frame();
        app.sound_frame();
        app.perf_frame(draw, t.elapsed().as_secs_f64() * 1000.0, (app.game.tick - ticks0) as u32);
    }
    Ok(())
}
