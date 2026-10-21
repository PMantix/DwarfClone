//! The adventure bot and its batch (A18): `--adventure-trial`.
//!
//! The bot plays an adventurer the way a cautious player would. It makes a
//! hero with the character maker (A1), picks a goal near home (a fallen
//! fort, a beast's lair or a ruin), and walks there by the world map (A3),
//! camping at night and when hurt. On the road it slips away from what it
//! can't fight and stands against the rest. Inside a site it hunts down
//! whatever holds the place with aimed blows (A6), binds its wounds when
//! the blood runs low, runs for the road when it runs lower, takes the
//! best of what lies there, and walks home.
//!
//! `trial` plays one such life to its end and writes an `AdvTrial`: how it
//! ended (home, dead where and to what, or lost on the road), the days, the
//! kills, the loot and the fame. `trial_batch` plays many side by side, as
//! `--autoplay` does for forts, and `report::adventures` sums them up. It
//! is the measure for the danger tuning (A16): fun-hard, or just lethal.

use super::super::*;
use super::create::{self, Maker, CALLINGS};
use super::fight::{Aim, AIMS};
use super::travel::{cheb, EncKind, TravelOutcome, DIRS};
use super::turn::{Action, Outcome, Target};
use super::{sitegen, AdvEnd, Adventure, Calling, SiteKey, HERO_ID};
use crate::body::Part;
use crate::items::ARTIFACT;
use crate::overworld::SiteKind;
use crate::path::astar;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

/// What the bot sets out for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Goal {
    /// The ruin of a fort that fell (one is made for the run: `fallen_world`).
    Fort,
    /// The den of a beast of legend that still lives.
    Lair,
    /// A town or hamlet that died.
    Ruin,
}

impl Goal {
    pub const ALL: [Goal; 3] = [Goal::Fort, Goal::Lair, Goal::Ruin];

    pub fn name(self) -> &'static str {
        match self {
            Goal::Fort => "fort",
            Goal::Lair => "lair",
            Goal::Ruin => "ruin",
        }
    }

    fn parse(s: &str) -> Option<Goal> {
        Goal::ALL.iter().copied().find(|g| g.name() == s)
    }
}

/// The part of the journey the hero was on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Leg {
    /// Walking out, before reaching the goal.
    Road,
    /// At the goal itself.
    Site,
    /// Walking home, with or without the spoils.
    WayHome,
}

impl Leg {
    pub const ALL: [Leg; 3] = [Leg::Road, Leg::Site, Leg::WayHome];

    pub fn words(self) -> &'static str {
        match self {
            Leg::Road => "on the road out",
            Leg::Site => "at the goal",
            Leg::WayHome => "on the way home",
        }
    }
}

/// How a run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Fate {
    /// Came home alive.
    Home,
    Died,
    /// Still out there when the days ran out (or no way on was found).
    Lost,
}

/// One adventurer's life, for the batch file.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdvTrial {
    pub seed: u64,
    /// The hero's name, people and calling.
    pub hero: String,
    pub race: String,
    pub calling: String,
    pub home: String,
    pub goal: Goal,
    /// The goal's name.
    pub target: String,
    /// World cells from home to the goal, by the shortest road.
    pub dist: u32,
    /// Got there and went in.
    pub reached: bool,
    /// Nothing that held it was left standing (or within reach).
    pub cleared: bool,
    pub fate: Fate,
    /// Where they died, or were lost.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leg: Option<Leg>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub killer: Option<String>,
    /// Days from setting out to the end.
    pub days: f32,
    pub cells: u32,
    /// Fights on the road (encounters stood against, camps fallen on), and
    /// the encounters slipped.
    pub fights: u32,
    pub evaded: u32,
    /// What the hero's own blows slew.
    pub slain: Vec<String>,
    /// Kills to the hero's name (foes that bled out included).
    pub kills: u32,
    /// What was carried home (or carried when they fell).
    pub loot: Vec<String>,
    pub loot_value: u32,
    pub artifact: bool,
    /// Fame (A10's; zero while that is a stub).
    pub renown: i32,
    pub deeds: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epithet: Option<String>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub knobs: std::collections::BTreeMap<String, f64>,
    /// The run's timeline, a line a turn of the story.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub story: Vec<String>,
}

/// How to play a batch.
#[derive(Clone, Debug)]
pub struct BotOpts {
    /// The goal, or none to take turns (fort, lair, ruin) by seed.
    pub goal: Option<Goal>,
    pub race: Option<Race>,
    pub calling: Calling,
    /// Give up after this many days out.
    pub max_days: u32,
    pub knobs: Knobs,
    /// Days the fallen fort is played before it falls.
    pub fort_days: u64,
    /// Where the fallen forts' saves go (a folder per run beneath it).
    pub dir: PathBuf,
}

impl Default for BotOpts {
    fn default() -> Self {
        BotOpts {
            goal: None,
            race: None,
            calling: Calling::Warrior,
            max_days: 120,
            knobs: Knobs::default(),
            fort_days: 3,
            dir: std::env::temp_dir().join(format!("dwarfclone-advtrial-{}", std::process::id())),
        }
    }
}

/// The goal of run `seed`: the one asked for, or fort, lair and ruin in turn.
pub fn goal_of(seed: u64, opts: &BotOpts) -> Goal {
    opts.goal.unwrap_or(Goal::ALL[(seed % 3) as usize])
}

/// Most acts in one site before the bot gives up on it.
const SITE_ACTS: usize = 3000;
/// Most steps chasing one foe before it is let go.
const CHASE: u32 = 120;

// ---------------------------------------------------------------- one life

/// The run's own record as it goes.
struct Run {
    goal: Goal,
    target: SiteKey,
    goal_cell: (i32, i32),
    home: u32,
    home_cell: (i32, i32),
    leg: Leg,
    reached: bool,
    cleared: bool,
    looted: bool,
    fights: u32,
    evaded: u32,
    slain: Vec<String>,
    /// (name, worth, a work of legend)
    loot: Vec<(String, u32, bool)>,
    story: Vec<String>,
    /// Foes given up on (out of reach, or run off).
    ignored: BTreeSet<u32>,
    /// Items that can't be reached.
    unreachable: BTreeSet<u32>,
}

/// Play one adventurer of world `seed` to the end.
pub fn trial(seed: u64, opts: &BotOpts) -> AdvTrial {
    let goal = goal_of(seed, opts);
    let dir = opts.dir.join(format!("run-{}", seed));
    let ow = if goal == Goal::Fort { fallen_world(seed, opts, &dir) } else { None }.unwrap_or_else(|| Overworld::generate(seed));
    let out = play(ow, seed, goal, opts, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    out
}

/// A world with a fort in it that has just fallen, to goblins (or, one
/// time in three, to nobody anyone saw): played `fort_days` under a
/// steady overseer, then every dwarf struck down, and saved to `dir` as
/// its ruin.
pub fn fallen_world(seed: u64, opts: &BotOpts, dir: &Path) -> Option<Overworld> {
    let _ = std::fs::remove_dir_all(dir);
    let ow = Overworld::generate(seed);
    let (x, y) = ow.default_site();
    let mut g = Game::embark_with(seed, ow, x, y, crate::world::MapSize::ALL[0], &|_, _| {});
    g.set_overseer(Some(Style::Steady));
    for _ in 0..opts.fort_days * DAY {
        g.step();
    }
    let killer = (names::hash(&[seed, 0xFA11]) % 3 != 0).then(|| "goblin".to_string());
    for i in 0..g.dwarves.len() {
        let mut d = std::mem::take(&mut g.dwarves[i]);
        if d.alive {
            if let Some(k) = &killer {
                g.annals.wounded_by.push((d.id, k.clone(), g.tick));
            }
            g.kill(&mut d, "was struck down");
        }
        g.dwarves[i] = d;
    }
    for _ in 0..10 {
        g.step();
    }
    g.fallen = true;
    crate::saves::write(dir, &g, false, &|_, _| {}).ok()?;
    Some(std::mem::replace(&mut g.overworld, Overworld::empty()))
}

/// The goals of a kind in the world: their keys and cells.
fn goals(ow: &Overworld, goal: Goal) -> Vec<(SiteKey, (i32, i32))> {
    match goal {
        Goal::Fort => ow.forts.iter().enumerate().filter(|(_, f)| f.fell.is_some()).map(|(i, f)| (SiteKey::Fort(i as u32), f.site)).collect(),
        Goal::Lair => ow
            .sites
            .iter()
            .filter(|s| s.kind == SiteKind::Lair && ow.figures.iter().any(|f| f.alive() && f.lair == Some(s.id)))
            .map(|s| (SiteKey::Site(s.id), (s.x, s.y)))
            .collect(),
        Goal::Ruin => ow.sites.iter().filter(|s| s.ruined && s.kind != SiteKind::Lair).map(|s| (SiteKey::Site(s.id), (s.x, s.y))).collect(),
    }
}

/// A cell one can walk into (not sea or lake).
fn passable(a: &Adventure, (x, y): (i32, i32)) -> bool {
    let ow = a.ow();
    x >= 0 && y >= 0 && x < ow.w && y < ow.h && a.crossing_days(x, y).is_some()
}

/// Steps by land from `from` to every cell (-1: no way there).
fn road_steps(a: &Adventure, from: (i32, i32)) -> Vec<i32> {
    let (w, h) = (a.ow().w, a.ow().h);
    let mut dist = vec![-1; (w * h).max(1) as usize];
    if !(from.0 >= 0 && from.1 >= 0 && from.0 < w && from.1 < h) {
        return dist;
    }
    let at = |(x, y): (i32, i32)| (y * w + x) as usize;
    dist[at(from)] = 0;
    let mut q = VecDeque::from([from]);
    while let Some(c) = q.pop_front() {
        for (dx, dy) in DIRS {
            let n = (c.0 + dx, c.1 + dy);
            if passable(a, n) && dist[at(n)] < 0 {
                dist[at(n)] = dist[at(c)] + 1;
                q.push_back(n);
            }
        }
    }
    dist
}

/// The first step of the shortest road from where the hero is to `to`.
fn next_step(a: &Adventure, to: (i32, i32)) -> Option<(i32, i32)> {
    let w = a.ow().w;
    let dist = road_steps(a, to);
    let here = dist.get((a.at.1 * w + a.at.0) as usize).copied().unwrap_or(-1);
    if here <= 0 {
        return None;
    }
    DIRS.iter().copied().filter(|&(dx, dy)| passable(a, (a.at.0 + dx, a.at.1 + dy))).find(|&(dx, dy)| dist[((a.at.1 + dy) * w + a.at.0 + dx) as usize] == here - 1)
}

/// Make the hero with the character maker: a people by seed (or the one
/// asked for), the calling asked for, and the home nearest a goal of the
/// kind wanted, at least two cells off. The maker, the goal, its cell and
/// the steps to it.
fn make_hero(ow: &Overworld, seed: u64, goal: Goal, opts: &BotOpts) -> Option<(Maker, SiteKey, (i32, i32), u32)> {
    let mut m = Maker::new(ow, seed);
    if m.races.is_empty() {
        return None;
    }
    let ri = match opts.race {
        Some(r) => m.races.iter().position(|&x| x == r)?,
        None => (names::hash(&[seed, 0x4ACE]) % m.races.len() as u64) as usize,
    };
    m.set_race(ow, ri);
    m.set_calling(CALLINGS.iter().position(|&c| c == opts.calling).unwrap_or(0));
    let targets = goals(ow, goal);
    // (Adventure's own crossing times, on a borrowed copy of the world.)
    let probe = Adventure::new(ow.clone(), seed, super::Hero::new(Default::default(), m.race()), None);
    let mut best: Option<(u32, usize, SiteKey, (i32, i32))> = None;
    for (hi, &home) in m.homes.iter().enumerate().take(12) {
        let Some(s) = ow.sites.iter().find(|s| s.id == home) else { continue };
        let dist = road_steps(&probe, (s.x, s.y));
        for &(key, cell) in &targets {
            let d = dist.get((cell.1 * ow.w + cell.0) as usize).copied().unwrap_or(-1);
            if d >= 2 && best.map_or(true, |b| (d as u32) < b.0) {
                best = Some((d as u32, hi, key, cell));
            }
        }
    }
    let (d, hi, key, cell) = best?;
    m.home = hi;
    Some((m, key, cell, d))
}

fn play(ow: Overworld, seed: u64, goal: Goal, opts: &BotOpts, dir: &Path) -> AdvTrial {
    // A world with no goal of the kind in reach sends the hero to another.
    let mut tries = std::iter::once(goal).chain(Goal::ALL.into_iter().filter(|&g| g != goal && g != Goal::Fort));
    let Some((goal, (maker, target, goal_cell, dist))) = tries.find_map(|g| make_hero(&ow, seed, g, opts).map(|m| (g, m))) else {
        return nowhere(seed, goal, opts, "no home with a goal in reach");
    };
    let hero = maker.hero(&ow);
    let home = hero.home.unwrap_or(0);
    let mut a = create::begin(ow, seed, hero);
    a.saves_dir = Some(dir.to_path_buf());
    a.knobs = opts.knobs.clone();
    if let Some(v) = a.visit.as_mut() {
        v.g.knobs = opts.knobs.clone();
    }
    let home_cell = a.at;
    let mut r = Run {
        goal,
        target,
        goal_cell,
        home,
        home_cell,
        leg: Leg::Road,
        reached: false,
        cleared: false,
        looted: false,
        fights: 0,
        evaded: 0,
        slain: Vec::new(),
        loot: Vec::new(),
        story: Vec::new(),
        ignored: BTreeSet::new(),
        unreachable: BTreeSet::new(),
    };
    let tname = super::sites::site_name(a.ow(), target);
    let (hname, race, calling) = (a.hero_dwarf().name.clone(), create::race_noun(a.hero.race), a.hero.calling.name());
    note(&mut r, &a, &format!("{} the {} {} sets out for {} ({} cells).", hname, race, calling, tname, dist));
    let fate = journey(&mut a, &mut r, opts);
    finish(a, r, seed, fate, tname, dist, opts)
}

/// A run that never set out.
fn nowhere(seed: u64, goal: Goal, opts: &BotOpts, why: &str) -> AdvTrial {
    AdvTrial {
        seed,
        hero: String::new(),
        race: String::new(),
        calling: String::new(),
        home: String::new(),
        goal,
        target: String::new(),
        dist: 0,
        reached: false,
        cleared: false,
        fate: Fate::Lost,
        leg: None,
        cause: Some(why.to_string()),
        killer: None,
        days: 0.0,
        cells: 0,
        fights: 0,
        evaded: 0,
        slain: Vec::new(),
        kills: 0,
        loot: Vec::new(),
        loot_value: 0,
        artifact: false,
        renown: 0,
        deeds: 0,
        epithet: None,
        knobs: opts.knobs.set.clone(),
        story: Vec::new(),
    }
}

/// Days since setting out.
fn days(a: &Adventure) -> f32 {
    let now = match &a.visit {
        Some(v) => sitegen::fallen::world_clock(v, a.start_year),
        None => a.clock,
    };
    now.saturating_sub(clock::DAWN_TICK) as f32 / DAY as f32
}

fn note(r: &mut Run, a: &Adventure, text: &str) {
    r.story.push(format!("d{:<5.1} {}", days(a), text));
}

/// Out and back: the whole life. Ends home, dead, or lost.
fn journey(a: &mut Adventure, r: &mut Run, opts: &BotOpts) -> Fate {
    let mut camps = 0;
    for _ in 0..4000 {
        if a.over.is_some() {
            return Fate::Died;
        }
        if days(a) > opts.max_days as f32 {
            note(r, a, "The days ran out with the hero still out in the world.");
            return Fate::Lost;
        }
        // In a site: home, the goal, or a fight on the road.
        if let Some(key) = a.visit.as_ref().map(|v| v.key) {
            if key == SiteKey::Site(r.home) && r.leg == Leg::WayHome {
                note(r, a, "Home again.");
                return Fate::Home;
            }
            if key == r.target {
                r.leg = Leg::Site;
            }
            if !in_site(a, r) {
                return Fate::Died;
            }
            continue;
        }
        let goal = if r.leg == Leg::Road { r.goal_cell } else { r.home_cell };
        // Home is safe to walk into hurt; the goal is not (mend first, below).
        let ready = r.leg == Leg::WayHome || (a.hero.dwarf.body.healed() && a.hero.dwarf.body.bleed_rate() == 0) || camps >= 4;
        if a.at == goal && ready {
            let key = if r.leg == Leg::Road { r.target } else { SiteKey::Site(r.home) };
            if let Err(e) = a.enter(key) {
                note(r, a, &format!("Couldn't go in: {}", e));
                return Fate::Lost;
            }
            if r.leg == Leg::Road {
                r.reached = true;
                let title = a.visit.as_ref().map_or(String::new(), |v| v.g.fortress_name.clone());
                note(r, a, &format!("Goes into {}.", title));
            }
            continue;
        }
        // A wound that bleeds is bound off the road, in the land itself.
        if a.hero.dwarf.body.bleed_rate() > 0 {
            let (x, y) = a.at;
            if a.enter(SiteKey::Wild(x, y)).is_ok() {
                if let Some(v) = a.visit.as_mut() {
                    let h = v.hero;
                    v.g.act(h, Action::Bind);
                    if !v.g.dwarves[h].alive {
                        record_death(a, r);
                        return Fate::Died;
                    }
                }
                a.take_road();
            }
        }
        // Camp by night, and while hurt (a few nights in a row at most).
        let hurt = !a.hero.dwarf.body.healed();
        if (clock::is_night(a.clock) || hurt) && camps < 4 {
            camps += 1;
            if let Some(e) = a.camp() {
                r.fights += 1;
                note(r, a, &e.text);
                let _ = a.meet(&e);
            }
            continue;
        }
        camps = 0;
        let Some((dx, dy)) = next_step(a, goal) else {
            note(r, a, "No road leads on.");
            return Fate::Lost;
        };
        match a.travel(dx, dy) {
            TravelOutcome::Arrived => {}
            TravelOutcome::Blocked(why) => {
                note(r, a, &why);
                return Fate::Lost;
            }
            TravelOutcome::Died(cause) => {
                note(r, a, &format!("Dies: {}.", cause));
                return Fate::Died;
            }
            TravelOutcome::Encounter(e) => {
                // Slip away from a beast of legend, or when already hurt.
                let b = &a.hero.dwarf.body;
                let wary = e.kind == EncKind::Legend || b.blood * 3 < b.max_blood * 2 || !b.healed();
                if wary && a.evade(&e, false) {
                    r.evaded += 1;
                    continue;
                }
                r.fights += 1;
                note(r, a, &e.text);
                let _ = a.meet(&e);
            }
        }
    }
    Fate::Lost
}

/// Turns inside a site until the hero leaves it (true) or dies (false).
/// At the goal: hunt what holds it, take the best of what lies there, and
/// go. Anywhere else (home, a fight on the road): win, mend, and go on.
fn in_site(a: &mut Adventure, r: &mut Run) -> bool {
    let at_goal = r.leg == Leg::Site;
    let flee = a.knob("bot.flee", 0.33);
    let bind_at = a.knob("bot.bind", 0.5);
    let loot_n = a.knob("bot.loot", 3.0) as usize;
    let head = a.knob("bot.head", 3.0);
    let entry = a.visit.as_ref().map_or(Pos::new(0, 0, 0), |v| v.g.dwarves[v.hero].pos);
    let mut fleeing = false;
    let mut chase: (u32, u32) = (0, 0);
    let mut rests = 0;
    for _ in 0..SITE_ACTS {
        let Some(v) = a.visit.as_ref() else { return true };
        let h = v.hero;
        if !v.g.dwarves[h].alive {
            record_death(a, r);
            return false;
        }
        let d = &v.g.dwarves[h];
        let (me, blood, bleeding, healed) = (d.pos, d.body.blood as f64 / d.body.max_blood.max(1) as f64, (0..7).any(|i| d.body.bleeds[i] > 0 && !d.body.bandaged[i]), d.body.healed());
        let foes = foes(&v.g, h, &v.placed, &r.ignored);
        // In reach: finish one before the next (the most hurt of them).
        let hurt = |id: u32| v.g.creatures.get(&id).map_or(0, |c| c.body.blood * 100 / c.body.max_blood.max(1));
        let close = foes.iter().copied().filter(|&(_, p)| p.z == me.z && cheb((p.x, p.y), (me.x, me.y)) <= 1).min_by_key(|&(id, _)| (hurt(id), id));
        if blood < flee && !foes.is_empty() && !fleeing {
            fleeing = true;
            note(r, a, "Badly hurt, the hero runs for it.");
        }
        let g = &a.visit.as_ref().unwrap().g;
        // Bleeding and none in reach: bind it first, before the road (a
        // hero who walks off bleeding dies in the wilds), or mid-fight when
        // it runs too fast (A16).
        let act = if bleeding && close.is_none() && (foes.is_empty() || !fleeing && blood < bind_at) {
            Action::Bind
        } else if fleeing || (at_goal && r.looted) || (!at_goal && foes.is_empty() && (healed || rests >= 6)) {
            match way_out(g, h, entry) {
                Some(act) => act,
                None => {
                    leave(a, r);
                    return true;
                }
            }
        } else if let Some((cid, _)) = close {
            Action::Attack { target: Target::Creature(cid), aim: Some(aim(g, h, cid, head)) }
        } else if let Some(&(cid, p)) = foes.first() {
            if chase.0 != cid {
                chase = (cid, 0);
            }
            chase.1 += 1;
            match step_to(g, h, p).filter(|_| chase.1 < CHASE) {
                Some(act) => act,
                None => {
                    r.ignored.insert(cid);
                    continue;
                }
            }
        } else if !healed && rests < 6 {
            rests += 1;
            Action::Rest
        } else {
            // At the goal, and nothing holds it any more: the spoils.
            if !r.cleared {
                r.cleared = true;
                note(r, a, "Nothing that held the place is left standing.");
            }
            let found = if r.loot.len() >= loot_n { None } else { next_loot(g, h, &mut r.unreachable) };
            match found {
                Some((id, p)) if p == me => Action::Get(vec![id]),
                Some((id, p)) => match step_to(g, h, p) {
                    Some(act) => act,
                    None => {
                        r.unreachable.insert(id);
                        continue;
                    }
                },
                None => {
                    r.looted = true;
                    continue;
                }
            }
        };
        let v = a.visit.as_mut().unwrap();
        let before = v.g.tick;
        let out = v.g.act(h, act.clone());
        // `BOT_TRACE=1`: every act and how it went, for tuning by eye, with
        // what the log told meanwhile (the blows).
        if std::env::var("BOT_TRACE").is_ok() {
            for e in v.g.log.iter().filter(|e| e.tick > before) {
                eprintln!("      | {}", e.text);
            }
            let d = &v.g.dwarves[h];
            eprintln!("t{} {:?} -> {:?} blood {}/{} foes {} held {:?}", v.g.tick, act, out, d.body.blood, d.body.max_blood, foes.len(), d.held.and_then(|i| v.g.items.get(&i)).map(|i| i.name()));
        }
        let mut news = Vec::new();
        match &act {
            Action::Attack { target: Target::Creature(id), .. } => {
                if let Some(c) = v.g.creatures.get(id).filter(|c| !c.alive) {
                    let name = c.label.clone().unwrap_or_else(|| c.species.info().name.to_string());
                    news.push(format!("Slays {}.", a_or_an(&name)));
                    r.slain.push(name);
                }
            }
            Action::Get(ids) => {
                for it in ids.iter().filter_map(|id| v.g.items.get(id)).filter(|it| it.carried_by == Some(HERO_ID)) {
                    let name = it.label.clone().unwrap_or_else(|| it.name());
                    news.push(format!("Takes {}.", name));
                    r.loot.push((name, it.value(), it.quality >= ARTIFACT));
                }
            }
            _ => {}
        }
        for n in news {
            note(r, a, &n);
        }
        match out {
            Outcome::Died => {
                record_death(a, r);
                return false;
            }
            Outcome::Refused(_) => {
                // Can't, here: rest no more, and let a moment pass rather than spin.
                if act == Action::Rest {
                    rests = 6;
                }
                let v = a.visit.as_mut().unwrap();
                if v.g.act(h, Action::Wait(10)) == Outcome::Died {
                    record_death(a, r);
                    return false;
                }
            }
            _ => {}
        }
    }
    // Too long in one place: out, whatever is left.
    leave(a, r);
    true
}

/// Off the site and onto the road; leaving the goal starts the way home.
fn leave(a: &mut Adventure, r: &mut Run) {
    let at_goal = a.visit.as_ref().map_or(false, |v| v.key == r.target);
    a.take_road();
    if at_goal {
        r.leg = Leg::WayHome;
        let what = if r.loot.is_empty() { "empty-handed" } else { "with the spoils" };
        note(r, a, &format!("Leaves for home {}.", what));
    }
}

/// What is out for the hero's blood: whatever holds the place, whatever
/// came after the hero, and any hostile thing in sight. Nearest first.
fn foes(g: &Game, h: usize, placed: &[(u32, String)], ignored: &BTreeSet<u32>) -> Vec<(u32, Pos)> {
    let me = g.dwarves[h].pos;
    let seen: BTreeSet<u32> = g.in_sight(h).into_iter().collect();
    let tenants: BTreeSet<u32> = placed.iter().filter(|(_, k)| k.starts_with('@')).map(|(id, _)| *id).collect();
    let mut v: Vec<(i32, u32, Pos)> = g
        .creatures
        .values()
        .filter(|c| c.alive && !c.tame && c.pet_of.is_none() && !ignored.contains(&c.id))
        .filter(|c| (c.hostile() || c.siege) && (tenants.contains(&c.id) || c.target == Some(HERO_ID) || seen.contains(&c.id)))
        .map(|c| (cheb((c.pos.x, c.pos.y), (me.x, me.y)) + 8 * (c.pos.z - me.z).abs(), c.id, c.pos))
        .collect();
    v.sort();
    v.into_iter().map(|(_, id, p)| (id, p)).collect()
}

/// Where to strike: the mark with the best hope, the odds weighed by what
/// a hit there is worth (the head kills, the body is broad, the weapon arm
/// disarms). The head is worth `head` (`bot.head`, 3: a third of the body's
/// blows end a fight there, and a hard one leaves the foe senseless), so a
/// hero goes for it unless the odds are long; the body is for the clumsy.
fn aim(g: &Game, h: usize, cid: u32, head: f64) -> Aim {
    let lost = g.creatures.get(&cid).map(|c| c.body.lost).unwrap_or_default();
    let worth = |a: Aim| match a {
        Aim::Part(Part::Head) => head,
        Aim::Eyes => 1.3,
        Aim::Teeth => 0.8,
        Aim::Part(Part::UpperBody) => 1.4,
        Aim::Part(Part::LowerBody) => 1.3,
        Aim::Part(Part::RightArm) => 1.15,
        Aim::Part(Part::LeftArm) => 0.9,
        Aim::Part(_) => 1.0,
    };
    let score = |a: Aim| g.hit_odds(h, cid, Some(a), false) as f64 * worth(a);
    AIMS.iter().copied().filter(|a| !lost[a.part().idx()]).max_by(|&x, &y| score(x).total_cmp(&score(y))).unwrap_or(Aim::Part(Part::UpperBody))
}

/// The act that takes the hero one step along the way to `to`.
fn step_to(g: &Game, h: usize, to: Pos) -> Option<Action> {
    let me = g.dwarves[h].pos;
    let path = astar(&g.world, me, to, 20_000, Mover::Dwarf)?;
    let next = *path.first()?;
    Some(if next.x == me.x && next.y == me.y { Action::Climb(next.z - me.z) } else { Action::Move(next.x - me.x, next.y - me.y) })
}

/// The way out: none once under the open sky (the road is right there),
/// else a step back toward where the hero came in.
fn way_out(g: &Game, h: usize, entry: Pos) -> Option<Action> {
    let me = g.dwarves[h].pos;
    if me == entry || g.world.outside(me) {
        return None;
    }
    step_to(g, h, entry)
}

/// The best thing lying loose that the hero can reach: a work of legend
/// first, then by worth. Bodies and boulders aside.
fn next_loot(g: &Game, h: usize, unreachable: &mut BTreeSet<u32>) -> Option<(u32, Pos)> {
    let me = g.dwarves[h].pos;
    let mut v: Vec<(bool, u32, u32, Pos)> = g
        .items
        .values()
        .filter(|it| it.carried_by.is_none() && it.container.is_none() && !unreachable.contains(&it.id))
        .filter(|it| !matches!(it.kind, ItemKind::Corpse | ItemKind::Carcass | ItemKind::Boulder))
        .map(|it| (it.quality >= ARTIFACT, it.value(), it.id, it.pos))
        .filter(|&(art, value, _, _)| art || value >= 20)
        .collect();
    v.sort_by(|a, b| (b.0, b.1).cmp(&(a.0, a.1)).then(a.2.cmp(&b.2)));
    for (_, _, id, p) in v.into_iter().take(8) {
        if p == me || astar(&g.world, me, p, 20_000, Mover::Dwarf).is_some() {
            return Some((id, p));
        }
        unreachable.insert(id);
    }
    None
}

/// The hero died in a site: the end, with the cause and the killer from
/// the visit's own death notes.
///
/// A10 merge note: while `hero_died` is wave 0's stub, the bot writes
/// `AdvEnd::Died` itself, as travel.rs does for a death on the road. Once
/// A10's death record lands, this one function is where to switch over:
/// let `hero_died` write the end, and read the cause back from it.
fn record_death(a: &mut Adventure, r: &mut Run) {
    if a.over.is_some() {
        return;
    }
    let (cause, killer) = death_note(a);
    let place = a.visit.as_ref().map_or(String::new(), |v| v.g.fortress_name.clone());
    let text = match &killer {
        Some(k) => format!("{} by {} in {}", cause, a_or_an(k), place),
        None => format!("{} in {}", cause, place),
    };
    let tick = a.now();
    a.over = Some(AdvEnd::Died { cause: text.clone(), tick });
    a.hero_died(&text);
    note(r, a, &format!("Dies: {}.", text));
}

/// ("struck down", Some("goblin")): the hero's death note in the visit.
fn death_note(a: &Adventure) -> (String, Option<String>) {
    let Some(v) = a.visit.as_ref() else { return ("died".into(), None) };
    let name = &v.g.dwarves[v.hero].name;
    match v.g.annals.deaths.iter().rev().find(|n| &n.name == name) {
        Some(n) => (n.cause.clone(), n.killer.clone()),
        None => ("died".into(), None),
    }
}

/// "a goblin", "an ogre"; a name as it is.
fn a_or_an(k: &str) -> String {
    if k.starts_with(|c: char| c.is_uppercase()) {
        k.to_string()
    } else if k.starts_with(['a', 'e', 'i', 'o', 'u']) {
        format!("an {}", k)
    } else {
        format!("a {}", k)
    }
}

/// The record of the life, once it is over.
fn finish(a: Adventure, r: Run, seed: u64, fate: Fate, target: String, dist: u32, opts: &BotOpts) -> AdvTrial {
    let d = a.hero_dwarf();
    let (cause, killer) = match (&a.over, fate) {
        (Some(AdvEnd::Died { cause, .. }), _) => (Some(cause.clone()), if a.visit.is_some() { death_note(&a).1 } else { None }),
        (_, Fate::Lost) => (r.story.last().map(|s| s.splitn(2, ' ').nth(1).unwrap_or("").trim().to_string()), None),
        _ => (None, None),
    };
    AdvTrial {
        seed,
        hero: d.name.clone(),
        race: create::race_noun(a.hero.race).to_string(),
        calling: a.hero.calling.name().to_string(),
        home: a.ow().sites.iter().find(|s| s.id == r.home).map_or(String::new(), |s| s.name.clone()),
        goal: r.goal,
        target,
        dist,
        reached: r.reached,
        cleared: r.cleared,
        fate,
        leg: (fate != Fate::Home).then_some(r.leg),
        cause,
        killer,
        days: (days(&a) * 10.0).round() / 10.0,
        cells: a.stats.cells,
        fights: r.fights,
        evaded: r.evaded,
        slain: r.slain,
        kills: d.kills,
        loot: r.loot.iter().map(|l| l.0.clone()).collect(),
        loot_value: r.loot.iter().map(|l| l.1).sum(),
        artifact: r.loot.iter().any(|l| l.2),
        renown: a.fame.renown,
        deeds: a.fame.deeds.len(),
        epithet: d.epithet.clone(),
        knobs: opts.knobs.set.clone(),
        story: r.story,
    }
}

// ---------------------------------------------------------------- the batch

/// `--adventure-trial [--runs N] [--seed S] [--threads T] [--goal fort|lair|ruin]
/// [--race dwarf|human|elf|goblin] [--calling warrior|hunter|wanderer|peasant]
/// [--days D] [--story] [--out FILE] [--knob name=v]... [--knobs FILE]`:
/// many adventurers side by side, each to their end, then how they ended.
/// `--adventure-trial --compare A B` puts two saved batches side by side.
pub fn trial_batch(args: &[String]) {
    let opt = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    if let Some(i) = args.iter().position(|a| a == "--compare") {
        let load = |p: Option<&String>| -> Result<Vec<AdvTrial>, String> {
            let p = p.ok_or("--compare needs two batch files (from --out)")?;
            serde_json::from_str(&std::fs::read_to_string(p).map_err(|e| format!("{}: {}", p, e))?).map_err(|e| format!("{}: {}", p, e))
        };
        match (load(args.get(i + 1)), load(args.get(i + 2))) {
            (Ok(a), Ok(b)) => print!("{}", crate::report::adventures(&a, Some(&b))),
            (Err(e), _) | (_, Err(e)) => eprintln!("{}", e),
        }
        return;
    }
    let mut opts = BotOpts::default();
    if let Some(path) = opt("--knobs") {
        match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|s| knobs::parse_knobs_file(&s)) {
            Ok(m) => opts.knobs.set.extend(m),
            Err(e) => return eprintln!("Couldn't read the knobs in {}: {}", path, e),
        }
    }
    for (i, a) in args.iter().enumerate() {
        if a == "--knob" {
            match args.get(i + 1).map(|s| knobs::parse_knob(s)) {
                Some(Ok((k, v))) => {
                    opts.knobs.set.insert(k, v);
                }
                Some(Err(e)) => return eprintln!("--knob: {}", e),
                None => return eprintln!("--knob needs name=value"),
            }
        }
    }
    if let Some(g) = opt("--goal") {
        match Goal::parse(&g) {
            Some(g) => opts.goal = Some(g),
            None => return eprintln!("--goal is fort, lair or ruin"),
        }
    }
    if let Some(r) = opt("--race") {
        match [Race::Dwarf, Race::Human, Race::Elf, Race::Goblin].into_iter().find(|&x| create::race_noun(x) == r) {
            Some(x) => opts.race = Some(x),
            None => return eprintln!("--race is dwarf, human, elf or goblin"),
        }
    }
    if let Some(c) = opt("--calling") {
        match CALLINGS.into_iter().find(|x| x.name().eq_ignore_ascii_case(&c)) {
            Some(x) => opts.calling = x,
            None => return eprintln!("--calling is warrior, hunter, wanderer or peasant"),
        }
    }
    if let Some(d) = opt("--days").and_then(|s| s.parse().ok()) {
        opts.max_days = d;
    }
    let runs: u64 = opt("--runs").and_then(|s| s.parse().ok()).unwrap_or(10);
    let base: u64 = opt("--seed").and_then(|s| s.parse().ok()).unwrap_or(9000);
    let threads = opt("--threads").and_then(|s| s.parse().ok()).unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get())).max(1);
    let story = args.iter().any(|a| a == "--story");
    println!(
        "Adventure trial: {} adventurers, {}, {} at a time{}",
        runs,
        opts.goal.map_or("to a fort, a lair and a ruin in turn".to_string(), |g| format!("each to a {}", g.name())),
        threads,
        if opts.knobs.set.is_empty() { String::new() } else { format!("; knobs: {}", opts.knobs.describe()) }
    );
    let queue = std::sync::Arc::new(std::sync::Mutex::new(base..base + runs));
    let (tx, rx) = std::sync::mpsc::channel();
    let mut handles = Vec::new();
    for _ in 0..threads.min(runs as usize) {
        let (queue, tx, opts) = (queue.clone(), tx.clone(), opts.clone());
        handles.push(std::thread::spawn(move || loop {
            let next = queue.lock().unwrap().next();
            let Some(seed) = next else { break };
            let _ = tx.send(trial(seed, &opts));
        }));
    }
    drop(tx);
    let mut all = Vec::new();
    for t in rx {
        println!("  {}", line(&t));
        if story {
            for s in &t.story {
                println!("           {}", s);
            }
            println!();
        }
        all.push(t);
        // Saved as it goes, so a batch cut short keeps what it has.
        if let Some(path) = opt("--out") {
            let _ = serde_json::to_string(&all).map(|s| std::fs::write(&path, s));
        }
    }
    for h in handles {
        let _ = h.join();
    }
    let _ = std::fs::remove_dir_all(&opts.dir);
    all.sort_by_key(|t| t.seed);
    print!("{}", crate::report::adventures(&all, None));
    if let Some(path) = opt("--out") {
        match serde_json::to_string(&all).map_err(|e| e.to_string()).and_then(|s| std::fs::write(&path, s).map_err(|e| e.to_string())) {
            Ok(()) => println!("\n  Saved the batch to {} (compare two with --adventure-trial --compare A B).", path),
            Err(e) => println!("\n  Couldn't save the batch to {}: {}", path, e),
        }
    }
}

/// One run in a line: the fate, who, where to, and what came of it.
pub fn line(t: &AdvTrial) -> String {
    let fate = match (t.fate, t.leg) {
        (Fate::Home, _) => "HOME".to_string(),
        (Fate::Died, Some(l)) => format!("DIED {}", l.words()),
        (Fate::Died, None) => "DIED".to_string(),
        (Fate::Lost, _) => "LOST".to_string(),
    };
    let who = format!("{}, {} {}", t.hero.split(' ').next().unwrap_or(""), t.race, t.calling.to_lowercase());
    let got = if t.loot.is_empty() { String::new() } else { format!("  loot {} (☼{}{})", t.loot.len(), t.loot_value, if t.artifact { ", a relic" } else { "" }) };
    let fame = if t.renown != 0 { format!("  renown {}", t.renown) } else { String::new() };
    let why = t.cause.as_ref().map_or(String::new(), |c| format!("  — {}", c));
    format!("{:<24} seed {:<6} {:<22} → {} {} ({} cells)  {:>5.1} days  kills {}{}{}{}", fate, t.seed, who, t.goal.name(), t.target, t.dist, t.days, t.kills, got, fame, why)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calm(tag: &str) -> BotOpts {
        let mut o = BotOpts::default();
        o.knobs.set.insert("travel.encounters".into(), 0.0);
        o.dir = std::env::temp_dir().join(format!("dwarfclone-bot-{}-{}", tag, std::process::id()));
        o
    }

    /// On a calm world (nothing met on the road), against a weak target (a
    /// fallen fort held by one goblin), the bot walks there, deals with
    /// what holds it, takes the spoils, and comes home.
    #[test]
    fn the_bot_goes_and_comes_back() {
        let mut o = calm("back");
        o.goal = Some(Goal::Fort);
        o.knobs.set.insert("fallen.band".into(), 1.0);
        let mut home = None;
        for seed in [9000, 9003, 9006, 9009, 9012] {
            let t = trial(seed, &o);
            assert!(!t.hero.is_empty() && t.dist >= 2, "{:?}", t);
            assert!(t.days > 0.0 && t.cells >= t.dist, "it walked: {:?}", t);
            if t.fate == Fate::Home {
                home = Some(t);
                break;
            }
        }
        let t = home.expect("one came home");
        assert!(t.reached && t.cleared && !t.loot.is_empty(), "went in, won and took: {:?}", t);
        assert_eq!((t.leg, t.cause.as_deref()), (None, None));
        assert!(t.story.iter().any(|s| s.contains("Home again")));
    }

    /// A death says where and to what, survives the batch file, and the
    /// report counts it.
    #[test]
    fn a_death_says_where_and_to_what() {
        let mut o = calm("death");
        o.goal = Some(Goal::Lair);
        o.calling = Calling::Peasant;
        let t = (9000..9010).map(|s| trial(s, &o)).find(|t| t.fate == Fate::Died).expect("a peasant dies at a beast's den");
        assert!(t.leg.is_some() && t.cause.as_deref().map_or(false, |c| !c.is_empty()), "{:?}", t);
        let back: Vec<AdvTrial> = serde_json::from_str(&serde_json::to_string(&vec![t.clone()]).unwrap()).unwrap();
        assert_eq!(back[0].cause, t.cause);
        let rep = crate::report::adventures(&back, None);
        assert!(rep.contains("died") && rep.contains(t.leg.unwrap().words()), "{}", rep);
        let cmp = crate::report::adventures(&back, Some(&back));
        assert!(cmp.contains("before") && cmp.contains("after") && cmp.contains(t.leg.unwrap().words()), "{}", cmp);
    }
}
