//! Adventurer mode's screens (G03). The mode is a self-contained sub-app,
//! like the arena: `App.adv` holds the adventure and its view, its keys are
//! routed here before the fort's, and `ui/adventure.rs` draws it.
//!
//! One key function per view, each in its own `impl App` block, so the
//! wave-1 packages each edit only their own: `key_create` (A1), `key_local`
//! (A2), `key_travel` (A3), `key_end` (A10), `key_talk` (A7),
//! `key_company` (A9).

use super::{App, Mode, TITLE_ADVENTURE};
use crate::body::{Part, PARTS};
use crate::game::adv::turn::{Action, Outcome, Target};
use crate::game::adv::{self, Adventure, View};
use crate::game::{Game, DAY};
use crate::jobs::JobDone;
use crate::overworld::Overworld;
use crossterm::event::{KeyCode, KeyEvent};

/// The adventure on screen.
pub struct AdvScreen {
    pub adv: Adventure,
    pub view: View,
    /// A line for the player (refusals, interrupts).
    pub flash: Option<String>,
    /// The character maker's choices, while the hero is being made (A1).
    pub maker: Option<Box<adv::create::Maker>>,
    /// The local view's open list (A2): get, drop, wear, aim... and what
    /// it offers, chosen by letter.
    pub pick: Option<(Pick, Vec<u32>)>,
    /// Sneaking: every step takes twice as long.
    pub sneak: bool,
    /// Resting until this tick (a slice a frame), unless woken; and
    /// whether to stop once healed.
    pub rest_until: Option<(u64, bool)>,
    /// The help card is up.
    pub help: bool,
    /// When the last act began: the log's lines since are this turn's.
    pub turn_from: u64,
    /// Something met on the road, waiting on fight or flight (A3).
    pub meet: Option<adv::travel::Encounter>,
}

/// What a letter list in the local view is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Get,
    Drop,
    Wear,
    Remove,
    /// A foe in reach to aim at (`A`)...
    Aim,
    /// ...then the part of them (the list holds part indices).
    AimPart(Target),
    Talk,
}

impl Pick {
    pub fn title(self) -> &'static str {
        match self {
            Pick::Get => "Pick up what?",
            Pick::Drop => "Drop what?",
            Pick::Wear => "Wear or wield what?",
            Pick::Remove => "Take off what?",
            Pick::Aim => "Attack whom?",
            Pick::AimPart(_) => "Aim where?",
            Pick::Talk => "Talk to whom?",
        }
    }
}

impl AdvScreen {
    pub fn new(adv: Adventure) -> AdvScreen {
        let maker = adv::create::unmade(&adv).then(|| Box::new(adv::create::Maker::new(&adv.ow, adv.seed)));
        let view = if maker.is_some() {
            View::Create
        } else if adv.over.is_some() {
            View::Dead
        } else if adv.visit.is_some() {
            View::Local
        } else {
            View::Travel
        };
        let turn_from = adv.now();
        AdvScreen { adv, view, flash: None, maker, pick: None, sneak: false, rest_until: None, help: false, turn_from, meet: None }
    }
}

impl App {
    /// From the title screen: a world made from the seed, then the character
    /// maker (A1).
    pub fn open_adventure(&mut self) {
        let seed = self.seed;
        self.run_job("Making the world", move |report| {
            let ow = Overworld::generate_with(seed, report);
            Ok(JobDone::Visited(Box::new(adv::create::unmade_in(ow, seed))))
        });
    }

    /// The quick start: an adventure in a fresh world from `seed`, its hero
    /// made for them (the window's `--adventure`, and tests).
    pub fn start_adventure_seeded(&mut self, seed: u64) {
        self.run_job("Setting out", move |report| {
            let ow = Overworld::generate_with(seed, report);
            report("Setting out", 0.95);
            Ok(JobDone::Visited(Box::new(adv::quick_start(ow, seed))))
        });
    }

    /// The game-over page's `a` (after the save): an adventurer into this
    /// fallen fort's world.
    pub fn adventure_in_this_world(&mut self) {
        let ow = std::mem::replace(&mut self.game.overworld, Overworld::empty());
        let seed = self.seed ^ 0xA0;
        self.game = crate::game::Game::placeholder(Overworld::empty());
        self.run_job("Setting out", move |_| Ok(JobDone::Visited(Box::new(adv::quick_start(ow, seed)))));
    }

    /// An adventure is ready (begun, a site entered, or loaded).
    pub fn adventure_ready(&mut self, a: Box<Adventure>) {
        self.adv = Some(Box::new(AdvScreen::new(*a)));
        self.mode = Mode::Adventure;
    }

    /// The window's `--adventure --fight` picture: a goblin raider comes at
    /// the hero, who closes and trades blows until one of them is hurt, and
    /// an iron helm lies on the ground.
    pub fn adventure_skirmish(&mut self) {
        let Some(s) = self.adv.as_mut() else { return };
        let Some(v) = s.adv.visit.as_mut() else { return };
        let h = v.hero;
        let p = v.g.dwarves[h].pos;
        v.g.spawn_item(crate::items::ItemKind::Helm, crate::material::Mat::Iron, p.offset(-1, 0, 0));
        let gob = v.g.raider_for(h);
        for _ in 0..40 {
            let Some(s) = self.adv.as_ref() else { return };
            let Some(v) = s.adv.visit.as_ref() else { return };
            let Some(fp) = v.g.creatures.get(&gob).filter(|c| c.alive).map(|c| c.pos) else { break };
            let d = &v.g.dwarves[h];
            let hurt = |b: &crate::body::Body| PARTS.iter().any(|&p| b.part_status(p) != "fine");
            if !d.alive || s.view == View::Dead || (hurt(&d.body) && v.g.creatures.get(&gob).map_or(true, |c| hurt(&c.body))) {
                break;
            }
            let (dx, dy) = ((fp.x - d.pos.x).signum(), (fp.y - d.pos.y).signum());
            self.adv_act(Action::Move(dx, dy));
        }
    }

    /// Each frame while the adventure is up: nothing runs while waiting for
    /// a key, but a rest goes on a slice at a time until healed, woken or
    /// done.
    pub fn adv_frame(&mut self) {
        let Some(s) = self.adv.as_mut() else { return };
        let Some((until, mend)) = s.rest_until else { return };
        let Some(v) = s.adv.visit.as_mut() else {
            s.rest_until = None;
            return;
        };
        let h = v.hero;
        let left = until.saturating_sub(v.g.tick);
        if left == 0 || (mend && whole(&v.g.dwarves[h])) {
            s.rest_until = None;
            s.flash = Some(if left == 0 { "You rise from your rest." } else { "You feel rested and whole." }.into());
            return;
        }
        let out = v.g.act(h, Action::Wait(left.min(REST_SLICE) as u32));
        v.g.reveal_seen(h);
        match out {
            Outcome::Done { .. } => {}
            Outcome::Interrupted { why, .. } | Outcome::Refused(why) => {
                s.rest_until = None;
                s.flash = Some(format!("Your rest is broken: {}", why));
            }
            Outcome::Died => {
                s.rest_until = None;
                s.view = View::Dead;
            }
        }
    }

    /// Every key while the adventure is up, by view.
    pub fn key_adventure(&mut self, k: KeyEvent) {
        let Some(s) = self.adv.as_mut() else {
            self.mode = Mode::Title { sel: TITLE_ADVENTURE };
            return;
        };
        s.flash = None;
        match s.view.clone() {
            View::Create => self.key_create(k),
            View::Local | View::Look(_) | View::Inventory { .. } | View::Status | View::Menu { .. } => self.key_local(k),
            View::Travel => self.key_travel(k),
            View::Dead => self.key_end(k),
            View::Talk { .. } => self.key_talk(k),
            View::Company => self.key_company(k),
            _ => {
                if k.code == KeyCode::Esc {
                    if let Some(s) = self.adv.as_mut() {
                        s.view = View::Local;
                    }
                }
            }
        }
    }

    /// Leave the adventure for the title (A11 saves first).
    fn adventure_to_title(&mut self) {
        self.adv = None;
        self.mode = Mode::Title { sel: TITLE_ADVENTURE };
    }
}

/// A direction from a movement key: arrows, numpad digits (as characters,
/// in the terminal and the window alike) and Home/End/PgUp/PgDn diagonals.
pub fn step_of(k: &KeyEvent) -> Option<(i32, i32)> {
    Some(match k.code {
        KeyCode::Up | KeyCode::Char('8') => (0, -1),
        KeyCode::Down | KeyCode::Char('2') => (0, 1),
        KeyCode::Left | KeyCode::Char('4') => (-1, 0),
        KeyCode::Right | KeyCode::Char('6') => (1, 0),
        KeyCode::Home | KeyCode::Char('7') => (-1, -1),
        KeyCode::PageUp | KeyCode::Char('9') => (1, -1),
        KeyCode::End | KeyCode::Char('1') => (-1, 1),
        KeyCode::PageDown | KeyCode::Char('3') => (1, 1),
        _ => return None,
    })
}

impl App {
    /// The character maker (A1): ↑↓ a row, ←→ or `[` `]` change it (a
    /// skill: less/more), `n` name, `r` another face and name, Enter set out.
    fn key_create(&mut self, k: KeyEvent) {
        use crate::game::adv::create::ROWS;
        let Some(s) = self.adv.as_mut() else { return };
        let ow = &s.adv.ow;
        let Some(m) = s.maker.as_mut() else { return };
        if m.naming {
            match k.code {
                KeyCode::Enter | KeyCode::Esc => {
                    m.naming = false;
                    if m.name.trim().is_empty() {
                        m.name = m.rolled_name();
                    }
                }
                KeyCode::Backspace => {
                    m.name.pop();
                }
                KeyCode::Char(c) if m.name.chars().count() < 24 && (c.is_alphabetic() || c == ' ' || c == '\'' || c == '-') => m.name.push(c),
                _ => {}
            }
            return;
        }
        let by = match k.code {
            KeyCode::Left | KeyCode::Char('[') | KeyCode::Char('4') => -1,
            KeyCode::Right | KeyCode::Char(']') | KeyCode::Char('6') => 1,
            _ => 0,
        };
        let rows = m.rows();
        match k.code {
            KeyCode::Esc => return self.adventure_to_title(),
            KeyCode::Up | KeyCode::Char('8') => m.sel = (m.sel + rows - 1) % rows,
            KeyCode::Down | KeyCode::Char('2') => m.sel = (m.sel + 1) % rows,
            KeyCode::Char('n') => m.naming = true,
            KeyCode::Char('r') => m.reroll(),
            KeyCode::Enter if m.sel == 2 => m.naming = true,
            KeyCode::Enter => return self.set_out(),
            _ if by != 0 => {
                let step = |i: usize, n: usize| (i as i32 + by).rem_euclid(n.max(1) as i32) as usize;
                match m.sel {
                    0 => {
                        let i = step(m.race, m.races.len());
                        m.set_race(ow, i);
                    }
                    1 => m.set_calling(step(m.calling, crate::game::adv::create::CALLINGS.len())),
                    3 => m.home = step(m.home, m.homes.len()),
                    i if i >= ROWS.len() && i < ROWS.len() + m.skills.len() => {
                        if !m.adjust(i - ROWS.len(), by) {
                            s.flash = Some(if by > 0 { "Not enough points left.".into() } else { "Already none.".into() });
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    /// The hero is made: into the world, at dawn, at home.
    fn set_out(&mut self) {
        let Some(s) = self.adv.as_mut() else { return };
        let Some(m) = s.maker.as_ref() else { return };
        if m.home_id().is_none() {
            s.flash = Some("No people of this world has a home to set out from.".into());
            return;
        }
        let hero = m.hero(&s.adv.ow);
        let (ow, seed) = (std::mem::replace(&mut s.adv.ow, Overworld::empty()), s.adv.seed);
        self.adv = None;
        self.run_job("Setting out", move |_| Ok(JobDone::Visited(Box::new(adv::create::begin(ow, seed, hero)))));
    }
}

/// The most of a rest run in one frame.
const REST_SLICE: u64 = 600;
/// What a sneaking step costs on top of the step.
const SNEAK_EXTRA: u32 = 3;
/// How near someone must stand to be talked to.
const TALK_REACH: i32 = 3;
/// The menu's rows (A11 adds saving).
pub const MENU: [&str; 2] = ["Back to the adventure", "Quit to the title"];

impl App {
    /// The local map and its pages (look, inventory, status, menu): move,
    /// bump to fight, and DF's adventure keys.
    fn key_local(&mut self, k: KeyEvent) {
        let Some(s) = self.adv.as_mut() else { return };
        // Any key wakes a rest and closes the help card.
        if s.rest_until.take().is_some() {
            s.flash = Some("You stop resting.".into());
            return;
        }
        if s.help {
            s.help = false;
            return;
        }
        if s.pick.is_some() {
            return self.adv_key_pick(k);
        }
        match s.view.clone() {
            View::Look(p) => return self.adv_key_look(k, p),
            View::Inventory { sel } => return self.adv_key_inventory(k, sel),
            View::Menu { sel } => return self.adv_key_menu(k, sel),
            View::Status => {
                s.view = View::Local;
                return;
            }
            _ => {}
        }
        let Some(v) = s.adv.visit.as_ref() else {
            if k.code == KeyCode::Esc {
                s.view = View::Menu { sel: 0 };
            }
            return;
        };
        let (h, g) = (v.hero, &v.g);
        let act = if let Some((dx, dy)) = step_of(&k) {
            if self.adv_off_edge(dx, dy) {
                return;
            }
            Action::Move(dx, dy)
        } else {
            match k.code {
                KeyCode::Char('.') | KeyCode::Char('5') => Action::Wait(10),
                KeyCode::Char('<') => Action::Climb(1),
                KeyCode::Char('>') => Action::Climb(-1),
                KeyCode::Esc => {
                    s.view = View::Menu { sel: 0 };
                    return;
                }
                KeyCode::Char('?') => {
                    s.help = true;
                    return;
                }
                KeyCode::Char('z') => {
                    s.view = View::Status;
                    return;
                }
                KeyCode::Char('i') => {
                    s.view = View::Inventory { sel: 0 };
                    return;
                }
                KeyCode::Char('l') => {
                    s.view = View::Look(g.dwarves[h].pos);
                    return;
                }
                KeyCode::Char('s') => {
                    s.sneak = !s.sneak;
                    s.flash = Some(if s.sneak { "You move quietly now, keeping low." } else { "You stop sneaking." }.into());
                    return;
                }
                KeyCode::Char('Z') => {
                    if let Some(why) = g.alarm(h, g.tick) {
                        s.flash = Some(format!("You can't rest now: {}", why));
                        return;
                    }
                    let hurt = !whole(&g.dwarves[h]);
                    let span = if hurt { DAY } else { crate::game::clock::HOUR * 2 };
                    s.rest_until = Some((g.tick + span, hurt));
                    s.flash = Some(if hurt { "You lie down to rest your wounds..." } else { "You sit down and rest a while..." }.into());
                    return;
                }
                KeyCode::Char('g') => return self.adv_open_pick(Pick::Get),
                KeyCode::Char('d') => return self.adv_open_pick(Pick::Drop),
                KeyCode::Char('w') => return self.adv_open_pick(Pick::Wear),
                KeyCode::Char('r') => return self.adv_open_pick(Pick::Remove),
                KeyCode::Char('A') => return self.adv_open_pick(Pick::Aim),
                KeyCode::Char('k') => return self.adv_open_pick(Pick::Talk),
                KeyCode::Char('T') => return self.adv_take_road(),
                _ => return,
            }
        };
        self.adv_act(act);
    }

    /// The hero acts: the world runs, what they see is remembered, and a
    /// refusal or an interruption is said.
    fn adv_act(&mut self, act: Action) {
        let Some(s) = self.adv.as_mut() else { return };
        let Some(v) = s.adv.visit.as_mut() else { return };
        let h = v.hero;
        s.turn_from = v.g.tick;
        let step = matches!(act, Action::Move(..) | Action::Climb(_));
        let was = v.g.dwarves[h].pos;
        let mut before = Bodies::take(&v.g, h);
        before.swung = match &act {
            Action::Move(dx, dy) => {
                let to = was.offset(*dx, *dy, 0);
                v.g.creatures.values().find(|c| c.alive && c.pos == to).map(|c| c.id)
            }
            Action::Attack { target: Target::Creature(id), .. } => Some(*id),
            _ => None,
        };
        let mut out = v.g.act(h, act);
        // Sneaking: each step taken slowly, as long again.
        if s.sneak && step && v.g.dwarves[h].pos != was && matches!(out, Outcome::Done { .. }) {
            out = v.g.act(h, Action::Wait(SNEAK_EXTRA));
        }
        before.tell(&mut v.g, h, s.turn_from);
        v.g.reveal_seen(h);
        match out {
            Outcome::Done { .. } => {}
            Outcome::Interrupted { why, .. } | Outcome::Refused(why) => s.flash = Some(why),
            Outcome::Died => {
                s.pick = None;
                s.view = View::Dead;
            }
        }
    }

    /// Open a letter list, or say why there's nothing to choose.
    fn adv_open_pick(&mut self, kind: Pick) {
        let Some(s) = self.adv.as_mut() else { return };
        let Some(v) = s.adv.visit.as_ref() else { return };
        let ids = pick_choices(&v.g, v.hero, kind);
        if ids.is_empty() {
            let why = match kind {
                Pick::Get => "There is nothing here to pick up.",
                Pick::Drop => "You carry nothing.",
                Pick::Wear => "You have nothing more to wear or wield.",
                Pick::Remove => "You wear and hold nothing.",
                Pick::Aim | Pick::AimPart(_) => "There is no one in reach to attack.",
                Pick::Talk => "There is no one near enough to talk to.",
            };
            s.flash = Some(why.into());
            return;
        }
        s.pick = Some((kind, ids));
    }

    /// A letter chooses from the open list; anything else closes it.
    fn adv_key_pick(&mut self, k: KeyEvent) {
        let Some(s) = self.adv.as_mut() else { return };
        let Some((kind, ids)) = s.pick.take() else { return };
        let i = match k.code {
            KeyCode::Char(c) if c.is_ascii_lowercase() => (c as u8 - b'a') as usize,
            KeyCode::Enter => 0,
            _ => return,
        };
        let Some(&id) = ids.get(i) else {
            s.pick = Some((kind, ids));
            return;
        };
        let act = match kind {
            Pick::Get => Action::Get(vec![id]),
            Pick::Drop => Action::Drop(vec![id]),
            Pick::Wear => Action::Wear(id),
            Pick::Remove => Action::Remove(id),
            Pick::Aim => {
                s.pick = Some((Pick::AimPart(Target::Creature(id)), (0..PARTS.len() as u32).collect()));
                return;
            }
            Pick::AimPart(target) => Action::Attack { target, part: Some(PARTS[id as usize]) },
            Pick::Talk => Action::Talk(id),
        };
        self.adv_act(act);
    }

    /// The look cursor: moved by the movement keys; Esc or `l` closes it.
    fn adv_key_look(&mut self, k: KeyEvent, p: crate::geom::Pos) {
        let Some(s) = self.adv.as_mut() else { return };
        if let Some((dx, dy)) = step_of(&k) {
            if let Some(v) = s.adv.visit.as_ref() {
                let q = p.offset(dx, dy, 0);
                if v.g.world.in_bounds(q) {
                    s.view = View::Look(q);
                }
            }
            return;
        }
        if matches!(k.code, KeyCode::Esc | KeyCode::Char('l') | KeyCode::Enter) {
            s.view = View::Local;
        }
    }

    /// The inventory: choose a thing (arrows or its letter), then wear,
    /// remove or drop it.
    fn adv_key_inventory(&mut self, k: KeyEvent, sel: usize) {
        let Some(s) = self.adv.as_mut() else { return };
        let Some(v) = s.adv.visit.as_ref() else { return };
        let ids = pick_choices(&v.g, v.hero, Pick::Drop);
        let n = ids.len();
        let sel = sel.min(n.saturating_sub(1));
        let act = match k.code {
            KeyCode::Esc | KeyCode::Char('i') => {
                s.view = View::Local;
                return;
            }
            KeyCode::Up | KeyCode::Char('8') => {
                s.view = View::Inventory { sel: sel.saturating_sub(1) };
                return;
            }
            KeyCode::Down | KeyCode::Char('2') => {
                s.view = View::Inventory { sel: (sel + 1).min(n.saturating_sub(1)) };
                return;
            }
            KeyCode::Char(c) if n > 0 && matches!(c, 'w' | 'r' | 'd') => {
                let id = ids[sel];
                match c {
                    'w' => Action::Wear(id),
                    'r' => Action::Remove(id),
                    _ => Action::Drop(vec![id]),
                }
            }
            KeyCode::Char(c) if c.is_ascii_lowercase() && ((c as u8 - b'a') as usize) < n => {
                s.view = View::Inventory { sel: (c as u8 - b'a') as usize };
                return;
            }
            _ => return,
        };
        self.adv_act(act);
    }

    /// The menu: back, or quit to the title (A11 adds saving).
    fn adv_key_menu(&mut self, k: KeyEvent, sel: usize) {
        let Some(s) = self.adv.as_mut() else { return };
        match k.code {
            KeyCode::Esc => s.view = back(s),
            KeyCode::Up | KeyCode::Char('8') => s.view = View::Menu { sel: sel.saturating_sub(1) },
            KeyCode::Down | KeyCode::Char('2') => s.view = View::Menu { sel: (sel + 1).min(MENU.len() - 1) },
            KeyCode::Enter => match sel {
                0 => s.view = back(s),
                _ => self.adventure_to_title(),
            },
            _ => {}
        }
    }
}

/// The bodies round the hero before an act, so the combat log can tell
/// every blow of it: which part, how bad, a limb lost, a foe slain.
struct Bodies {
    hero: crate::body::Body,
    /// Creatures near the hero: id, name, body, alive.
    near: Vec<(u32, String, crate::body::Body, bool)>,
    /// Whom the hero struck at (a blow that did nothing missed).
    swung: Option<u32>,
}

impl Bodies {
    fn take(g: &Game, h: usize) -> Bodies {
        let d = &g.dwarves[h];
        let near = g
            .creatures
            .values()
            .filter(|c| c.alive && c.pos.z == d.pos.z && (c.pos.x - d.pos.x).abs() <= 3 && (c.pos.y - d.pos.y).abs() <= 3)
            .map(|c| (c.id, c.label.clone().unwrap_or_else(|| format!("the {}", c.species.info().name)), c.body.clone(), c.alive))
            .collect();
        Bodies { hero: d.body.clone(), near, swung: None }
    }

    /// The act's blows, as lines in the log: the hero's in gold, the foe's
    /// in red, a death in bright red.
    fn tell(self, g: &mut Game, h: usize, since: u64) {
        let mut lines: Vec<(String, (u8, u8, u8))> = Vec::new();
        for (id, name, b0, _) in &self.near {
            let Some(c) = g.creatures.get(id) else {
                lines.push((format!("{} falls dead!", cap_first(name)), (255, 90, 70)));
                continue;
            };
            let hurt = wounds(b0, &c.body);
            let by_me = c.body.hp != b0.hp || c.body.lost != b0.lost;
            if by_me {
                for (part, how) in hurt {
                    lines.push((format!("You strike {} in the {}: {}.", name, part, how), (240, 210, 120)));
                }
            } else if self.swung == Some(*id) {
                lines.push((format!("Your blow misses {}.", name), (170, 160, 130)));
            }
            if !c.alive {
                lines.push((format!("{} falls dead!", cap_first(name)), (255, 90, 70)));
            }
        }
        let d = &g.dwarves[h];
        let by = d.attacker.filter(|(_, t)| *t >= since).and_then(|(cid, _)| self.near.iter().find(|n| n.0 == cid)).map_or("something".to_string(), |n| n.1.clone());
        for (part, how) in wounds(&self.hero, &d.body) {
            lines.push((format!("{} hits you in the {}: {}.", cap_first(&by), part, how), (235, 110, 90)));
        }
        if d.body.bleeding > 0 && self.hero.bleeding == 0 {
            lines.push(("You are bleeding.".into(), (230, 60, 60)));
        }
        // The turn engine's one-line "is wounded by" says less than these.
        if !lines.is_empty() {
            let first = format!("{} is wounded by", d.name.split(' ').next().unwrap_or(""));
            g.log.retain(|e| e.tick < since || !e.text.starts_with(&first));
        }
        for (t, c) in lines {
            g.msg(&t, c);
        }
    }
}

/// Each part hurt from `a` to `b`, and how it is now.
fn wounds(a: &crate::body::Body, b: &crate::body::Body) -> Vec<(&'static str, &'static str)> {
    PARTS
        .iter()
        .filter(|p| b.hp[p.idx()] < a.hp[p.idx()] || (b.lost[p.idx()] && !a.lost[p.idx()]))
        .map(|&p| (p.name(), if b.lost[p.idx()] && !a.lost[p.idx()] { "cut clean off!" } else { b.part_status(p) }))
        .collect()
}

fn cap_first(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

/// Healed and with all their blood: a rest has nothing left to mend.
fn whole(d: &crate::dwarf::Dwarf) -> bool {
    d.body.healed() && d.body.blood >= d.body.max_blood
}

/// What a letter list offers, in its order: items (get, drop, wear,
/// remove), creatures in reach (aim) or people near (talk).
pub fn pick_choices(g: &Game, hero: usize, kind: Pick) -> Vec<u32> {
    let d = &g.dwarves[hero];
    let (me, p) = (d.id, d.pos);
    let mut mine: Vec<u32> = g.items.values().filter(|it| it.carried_by == Some(me)).map(|it| it.id).collect();
    mine.sort();
    let worn = |id: &u32| d.held == Some(*id) || d.armor.contains(id) || d.clothes.contains(id);
    match kind {
        Pick::Get => {
            let mut v: Vec<u32> = g.items.values().filter(|it| it.pos == p && it.carried_by.is_none() && it.container.is_none()).map(|it| it.id).collect();
            v.sort();
            v
        }
        Pick::Drop => mine,
        Pick::Wear => mine.into_iter().filter(|id| !worn(id)).collect(),
        Pick::Remove => mine.into_iter().filter(|id| worn(id)).collect(),
        Pick::Aim => {
            let mut v: Vec<(bool, u32)> = g
                .creatures
                .values()
                .filter(|c| c.alive && c.pos.z == p.z && c.pos != p && (c.pos.x - p.x).abs() <= 1 && (c.pos.y - p.y).abs() <= 1)
                .map(|c| (!c.hostile(), c.id))
                .collect();
            v.sort();
            v.into_iter().map(|(_, id)| id).collect()
        }
        Pick::AimPart(_) => (0..PARTS.len() as u32).collect(),
        Pick::Talk => {
            let mut v: Vec<(i32, u32)> = g
                .dwarves
                .iter()
                .filter(|o| o.id != me && o.alive && !o.ghost && o.pos.z == p.z && (o.pos.x - p.x).abs().max((o.pos.y - p.y).abs()) <= TALK_REACH)
                .map(|o| ((o.pos.x - p.x).pow(2) + (o.pos.y - p.y).pow(2), o.id))
                .collect();
            v.sort();
            v.into_iter().map(|(_, id)| id).collect()
        }
    }
}

/// The chance-to-hit penalty for aiming at a part, in percent (A6 makes it
/// real in `strike_at`): a small target is harder to hit.
pub fn aim_penalty(p: Part) -> i32 {
    match p {
        Part::Head => 20,
        Part::LeftArm | Part::RightArm => 10,
        Part::LeftLeg | Part::RightLeg => 5,
        _ => 0,
    }
}

/// Out of the menu: back to the map the hero is on.
fn back(s: &AdvScreen) -> View {
    if s.adv.visit.is_some() {
        View::Local
    } else {
        View::Travel
    }
}

/// How near the edge of a site's map the hero must be to take the road.
const EDGE: i32 = 3;

impl App {
    /// The world map (A3): the 8 directions walk a cell, Enter or `>`
    /// enters the cell (its site, else its wild land), `Z` camps until
    /// dawn, `s` travels quietly, `?` the keys, Esc the menu. Something met
    /// on the road waits on Enter (stand and fight) or Esc (slip away).
    fn key_travel(&mut self, k: KeyEvent) {
        let Some(s) = self.adv.as_mut() else { return };
        if s.help {
            s.help = false;
            return;
        }
        if let Some(e) = s.meet.clone() {
            match k.code {
                KeyCode::Enter | KeyCode::Char('f') | KeyCode::Char('>') => self.adv_meet(e),
                KeyCode::Esc => {
                    let quiet = s.sneak;
                    if s.adv.evade(&e, quiet) {
                        s.meet = None;
                        s.flash = Some("You slip away.".into());
                    } else {
                        self.adv_meet(e);
                    }
                }
                _ => {}
            }
            return;
        }
        if let Some((dx, dy)) = step_of(&k) {
            let quiet = s.sneak;
            let out = s.adv.travel_with(dx, dy, quiet);
            self.adv_after_road(out);
            return;
        }
        match k.code {
            KeyCode::Enter | KeyCode::Char('>') => self.adv_enter_cell(),
            KeyCode::Char('Z') => {
                if let Some(e) = s.adv.camp() {
                    s.meet = Some(e);
                } else if s.adv.over.is_some() {
                    s.view = View::Dead;
                }
            }
            KeyCode::Char('s') => {
                s.sneak = !s.sneak;
                s.flash = Some(if s.sneak { "You keep to cover: slower, but fewer will see you." } else { "You take the open road again." }.into());
            }
            KeyCode::Char('?') => s.help = true,
            KeyCode::Esc => s.view = View::Menu { sel: 0 },
            _ => {}
        }
    }

    /// What a step on the road came to: a refusal said, an encounter
    /// waiting on the player, or the end.
    fn adv_after_road(&mut self, out: adv::travel::TravelOutcome) {
        use adv::travel::TravelOutcome;
        let Some(s) = self.adv.as_mut() else { return };
        s.view = View::Travel;
        match out {
            TravelOutcome::Arrived => {}
            TravelOutcome::Blocked(why) => s.flash = Some(why),
            TravelOutcome::Encounter(e) => s.meet = Some(e),
            TravelOutcome::Died(_) => s.view = View::Dead,
        }
    }

    /// Into the cell the hero stands in: its site (or the fallen fort
    /// there), else the wild. Building the place takes a moment, so it is
    /// a job; a place that can't be entered leaves them on the road.
    fn adv_enter_cell(&mut self) {
        let Some(s) = self.adv.take() else { return };
        let mut a = s.adv;
        let (x, y) = a.at;
        let ow = a.ow();
        let key = match (ow.fort_at(x, y), ow.site_at(x, y)) {
            (Some(i), _) => adv::SiteKey::Fort(i as u32),
            (None, Some(site)) => adv::SiteKey::Site(site.id),
            _ => adv::SiteKey::Wild(x, y),
        };
        let name = adv::sites::site_name(ow, key);
        self.run_job(&format!("Into {}", name), move |_| {
            if let Err(why) = a.enter(key) {
                a.say(&why, (240, 150, 110));
                // (A fallen fort that can't be opened: its wild land instead.)
                if !matches!(key, adv::SiteKey::Wild(..)) && a.enter(adv::SiteKey::Wild(x, y)).is_err() {
                    a.leave();
                }
            }
            Ok(JobDone::Visited(Box::new(a)))
        });
    }

    /// Stand and face what was met: into the wild land, foes at the edge.
    fn adv_meet(&mut self, e: adv::travel::Encounter) {
        let Some(s) = self.adv.take() else { return };
        let mut a = s.adv;
        self.run_job("To arms", move |_| {
            if let Err(why) = a.meet(&e) {
                a.say(&why, (240, 150, 110));
            }
            Ok(JobDone::Visited(Box::new(a)))
        });
    }

    /// `T` in a site: out onto the world map, from anywhere under the sky
    /// in the wild, or from near the edge of a site; never with a foe in
    /// sight.
    fn adv_take_road(&mut self) {
        let Some(s) = self.adv.as_mut() else { return };
        let Some(v) = s.adv.visit.as_ref() else { return };
        let (g, h) = (&v.g, v.hero);
        let p = g.dwarves[h].pos;
        let w = &g.world;
        let edge = p.x < EDGE || p.y < EDGE || p.x >= w.w - EDGE || p.y >= w.h - EDGE;
        let wild = matches!(v.key, adv::SiteKey::Wild(..)) && w.in_bounds(p) && w.outside(p);
        if !edge && !wild {
            s.flash = Some("Walk to the edge of the place to take the road.".into());
            return;
        }
        if let Some(why) = g.alarm(h, s.turn_from) {
            s.flash = Some(format!("You can't leave now: {}", why));
            return;
        }
        s.adv.take_road();
        s.view = View::Travel;
        s.pick = None;
        s.rest_until = None;
    }

    /// A step off the edge of a site's map walks on into the next cell
    /// (DF-like). True if it did.
    fn adv_off_edge(&mut self, dx: i32, dy: i32) -> bool {
        let Some(s) = self.adv.as_mut() else { return false };
        let Some(v) = s.adv.visit.as_ref() else { return false };
        let (g, h) = (&v.g, v.hero);
        let to = g.dwarves[h].pos.offset(dx, dy, 0);
        if g.world.in_bounds(to) {
            return false;
        }
        if let Some(why) = g.alarm(h, s.turn_from) {
            s.flash = Some(format!("You can't leave now: {}", why));
            return true;
        }
        s.adv.take_road();
        let quiet = s.sneak;
        let out = s.adv.travel_with(dx, dy, quiet);
        self.adv_after_road(out);
        true
    }

    /// The window's `--adventure --travel` picture: out of the home site
    /// and a few days on the road, east where the land allows, with a
    /// wound that bleeds on the way.
    pub fn adventure_journey(&mut self) {
        let Some(s) = self.adv.as_mut() else { return };
        s.adv.knobs.set.insert("travel.encounters".into(), 0.0);
        s.adv.take_road();
        s.view = View::Travel;
        let mut dir = (1, 0);
        for i in 0..9 {
            // (Hurt on the way: a mangled arm, still bleeding at the end.)
            if i == 7 {
                s.adv.hero.dwarf.body.wound(Part::LeftArm, 20, true);
                s.adv.say("A loose stone turns under you; you fall hard on the rocks and your arm is torn open.", (235, 90, 80));
            }
            let out = s.adv.travel_with(dir.0, dir.1, false);
            if let adv::travel::TravelOutcome::Blocked(_) = out {
                dir = adv::travel::DIRS[(i * 3 + 1) % 8];
            }
            if s.adv.over.is_some() {
                break;
            }
        }
        s.adv.knobs.set.remove("travel.encounters");
    }
}

impl App {
    /// The death page (A10).
    fn key_end(&mut self, k: KeyEvent) {
        if matches!(k.code, KeyCode::Esc | KeyCode::Char('t')) {
            self.adventure_to_title();
        }
    }
}

impl App {
    /// A conversation (A7).
    fn key_talk(&mut self, k: KeyEvent) {
        if k.code == KeyCode::Esc {
            if let Some(s) = self.adv.as_mut() {
                s.view = View::Local;
            }
        }
    }
}

impl App {
    /// Companions' orders (A9).
    fn key_company(&mut self, k: KeyEvent) {
        if k.code == KeyCode::Esc {
            if let Some(s) = self.adv.as_mut() {
                s.view = View::Local;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn key(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::NONE)
    }

    /// Wait out the loading screen.
    fn settle(app: &mut App) {
        let t = std::time::Instant::now();
        while app.mode == Mode::Loading && t.elapsed().as_secs() < 120 {
            app.frame();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    #[test]
    fn the_title_offers_an_adventure() {
        let mut app = App::new(5);
        app.save_dir = std::env::temp_dir().join("dwarfclone-no-saves-here");
        app.mode = Mode::Title { sel: 0 };
        while app.mode != (Mode::Title { sel: TITLE_ADVENTURE }) {
            app.on_key(key(KeyCode::Down));
        }
        app.on_key(key(KeyCode::Enter));
        settle(&mut app);
        assert_eq!(app.mode, Mode::Adventure);
        // The character maker first (A1); Enter sets out as it stands.
        assert_eq!(app.adv.as_ref().unwrap().view, View::Create);
        app.on_key(key(KeyCode::Enter));
        settle(&mut app);
        let s = app.adv.as_ref().unwrap();
        assert_eq!(s.view, View::Local);
        let v = s.adv.visit.as_ref().expect("in a site");
        assert_eq!(v.g.dwarves[v.hero].id, adv::HERO_ID);
        assert!(s.adv.ow().w > 1, "a real world");
        // Keys move the hero: each direction by arrow and by numpad digit.
        for (k, (dx, dy)) in [(KeyCode::Right, (1, 0)), (KeyCode::Char('4'), (-1, 0)), (KeyCode::Char('3'), (1, 1)), (KeyCode::Home, (-1, -1))] {
            let before = app.adv.as_ref().map(|s| s.adv.hero_dwarf().pos).unwrap();
            app.on_key(key(k));
            let after = app.adv.as_ref().map(|s| s.adv.hero_dwarf().pos).unwrap();
            assert_eq!((after.x - before.x, after.y - before.y), (dx, dy), "{:?}", k);
        }
        let t0 = app.adv.as_ref().unwrap().adv.now();
        app.on_key(key(KeyCode::Char('.')));
        assert_eq!(app.adv.as_ref().unwrap().adv.now(), t0 + 10);
        // Drawn, at a small terminal and a large one.
        for (w, h) in [(80, 24), (130, 40)] {
            let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
            term.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
            let text: String = term.backend().buffer().content().iter().map(|c| c.symbol().to_string()).collect();
            assert!(text.contains('☺'), "the hero is drawn at {}x{}", w, h);
        }
        // Esc is the menu; its last row quits to the title.
        app.on_key(key(KeyCode::Esc));
        assert_eq!(app.adv.as_ref().unwrap().view, View::Menu { sel: 0 });
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.mode, Mode::Title { sel: TITLE_ADVENTURE });
    }

    /// An app with a hero standing in a quiet field (no beasts about).
    fn field_app(seed: u64) -> App {
        static WORLD: std::sync::OnceLock<Overworld> = std::sync::OnceLock::new();
        let ow = WORLD.get_or_init(|| Overworld::generate(5)).clone();
        let mut app = App::new(seed);
        app.save_dir = std::env::temp_dir().join("dwarfclone-no-saves-here");
        let mut a = adv::quick_start(ow, seed);
        let (x, y) = a.at;
        a.enter(adv::SiteKey::Wild(x, y)).unwrap();
        a.visit.as_mut().unwrap().g.creatures.clear();
        app.adventure_ready(Box::new(a));
        app
    }

    fn game(app: &App) -> &Game {
        &app.adv.as_ref().unwrap().adv.visit.as_ref().unwrap().g
    }

    fn game_mut(app: &mut App) -> &mut Game {
        &mut app.adv.as_mut().unwrap().adv.visit.as_mut().unwrap().g
    }

    fn hero(app: &App) -> &crate::dwarf::Dwarf {
        app.adv.as_ref().unwrap().adv.hero_dwarf()
    }

    /// The screen as text.
    fn screen(app: &mut App, w: u16, h: u16) -> String {
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
        term.draw(|f| crate::ui::draw(f, app)).unwrap();
        let b = term.backend().buffer();
        let mut s = String::new();
        for y in 0..h {
            for x in 0..w {
                s.push_str(b.get(x, y).symbol());
            }
            s.push('\n');
        }
        s
    }

    #[test]
    fn keys_move_the_hero() {
        let mut app = field_app(21);
        let dirs = [
            (KeyCode::Up, KeyCode::Char('8'), (0, -1)),
            (KeyCode::Down, KeyCode::Char('2'), (0, 1)),
            (KeyCode::Left, KeyCode::Char('4'), (-1, 0)),
            (KeyCode::Right, KeyCode::Char('6'), (1, 0)),
            (KeyCode::Home, KeyCode::Char('7'), (-1, -1)),
            (KeyCode::PageUp, KeyCode::Char('9'), (1, -1)),
            (KeyCode::End, KeyCode::Char('1'), (-1, 1)),
            (KeyCode::PageDown, KeyCode::Char('3'), (1, 1)),
        ];
        for (arrow, digit, (dx, dy)) in dirs {
            for k in [arrow, digit] {
                let before = hero(&app).pos;
                app.on_key(key(k));
                let after = hero(&app).pos;
                assert_eq!((after.x - before.x, after.y - before.y), (dx, dy), "{:?}", k);
                // And back again, so the hero stays in the open.
                game_mut(&mut app).dwarves.iter_mut().find(|d| d.id == adv::HERO_ID).unwrap().pos = before;
            }
        }
        // `.` and numpad 5 wait ten ticks.
        for k in [KeyCode::Char('.'), KeyCode::Char('5')] {
            let t0 = game(&app).tick;
            app.on_key(key(k));
            assert_eq!(game(&app).tick, t0 + 10);
        }
        // Sneaking: a step takes twice as long.
        let t0 = game(&app).tick;
        app.on_key(key(KeyCode::Right));
        let step = game(&app).tick - t0;
        app.on_key(key(KeyCode::Char('s')));
        assert!(app.adv.as_ref().unwrap().sneak);
        let t0 = game(&app).tick;
        app.on_key(key(KeyCode::Left));
        assert_eq!(game(&app).tick - t0, step * 2, "a sneaking step");
    }

    #[test]
    fn pick_up_wear_and_drop_by_keys() {
        let mut app = field_app(22);
        let p = hero(&app).pos;
        let helm = game_mut(&mut app).spawn_item(crate::items::ItemKind::Helm, crate::material::Mat::Iron, p);
        let ch = |c: char| key(KeyCode::Char(c));
        // g: the list of what lies here; a takes the helm.
        app.on_key(ch('g'));
        assert_eq!(app.adv.as_ref().unwrap().pick, Some((Pick::Get, vec![helm])));
        app.on_key(ch('a'));
        assert_eq!(game(&app).items[&helm].carried_by, Some(adv::HERO_ID));
        assert!(app.adv.as_ref().unwrap().pick.is_none());
        // w: the axe is in hand already, so the helm is the first choice.
        app.on_key(ch('w'));
        app.on_key(ch('a'));
        assert!(hero(&app).armor.contains(&helm), "the helm is worn");
        let text = screen(&mut app, 130, 40);
        assert!(text.contains("helm (worn)"), "the sidebar shows it worn:\n{}", text);
        // z: the status page, any key back.
        app.on_key(ch('z'));
        assert!(screen(&mut app, 130, 40).contains("Skills"));
        app.on_key(ch('z'));
        assert_eq!(app.adv.as_ref().unwrap().view, View::Local);
        // r takes it off, and d drops it (its letter in the drop list).
        app.on_key(ch('r'));
        let ids = app.adv.as_ref().unwrap().pick.clone().unwrap().1;
        app.on_key(ch((b'a' + ids.iter().position(|&i| i == helm).unwrap() as u8) as char));
        assert!(!hero(&app).armor.contains(&helm));
        app.on_key(ch('d'));
        let ids = app.adv.as_ref().unwrap().pick.clone().unwrap().1;
        app.on_key(ch((b'a' + ids.iter().position(|&i| i == helm).unwrap() as u8) as char));
        assert_eq!(game(&app).items[&helm].carried_by, None);
        assert_eq!(game(&app).items[&helm].pos, hero(&app).pos);
        // Esc closes a list without acting.
        app.on_key(ch('g'));
        assert!(app.adv.as_ref().unwrap().pick.is_some());
        let t0 = game(&app).tick;
        app.on_key(key(KeyCode::Esc));
        assert!(app.adv.as_ref().unwrap().pick.is_none());
        assert_eq!(game(&app).tick, t0);
        // The inventory: the axe, chosen by its letter and put away.
        app.on_key(ch('i'));
        assert!(screen(&mut app, 80, 24).contains("Inventory"));
        app.on_key(ch('a'));
        app.on_key(ch('r'));
        assert_eq!(hero(&app).held, None, "the axe put away");
        app.on_key(ch('w'));
        assert!(hero(&app).held.is_some(), "and taken up again");
        app.on_key(key(KeyCode::Esc));
        assert_eq!(app.adv.as_ref().unwrap().view, View::Local);
    }

    #[test]
    fn aim_look_talk_and_rest() {
        let mut app = field_app(23);
        let ch = |c: char| key(KeyCode::Char(c));
        // Nobody in reach: A says so and costs nothing.
        app.on_key(ch('A'));
        assert!(app.adv.as_ref().unwrap().pick.is_none());
        assert!(app.adv.as_ref().unwrap().flash.is_some());
        // A goblin beside the hero: A a, then the head (a).
        let h = app.adv.as_ref().unwrap().adv.visit.as_ref().unwrap().hero;
        let gob = game_mut(&mut app).raider_for(h);
        let next = hero(&app).pos.offset(1, 0, 0);
        game_mut(&mut app).creatures.get_mut(&gob).unwrap().pos = next;
        app.on_key(ch('A'));
        assert_eq!(app.adv.as_ref().unwrap().pick, Some((Pick::Aim, vec![gob])));
        app.on_key(ch('a'));
        assert!(matches!(app.adv.as_ref().unwrap().pick, Some((Pick::AimPart(_), _))));
        assert!(screen(&mut app, 80, 24).contains("Aim where?"));
        let t0 = game(&app).tick;
        app.on_key(ch('a'));
        assert!(game(&app).tick > t0, "the blow took time");
        assert!(
            game(&app).log.iter().any(|e| e.tick >= t0 && (e.text.starts_with("You strike the goblin") || e.text.starts_with("Your blow misses"))),
            "the combat log tells the blow: {:?}",
            game(&app).log.iter().rev().take(4).map(|e| &e.text).collect::<Vec<_>>()
        );
        // Look: a cursor that moves, and Esc closes it.
        app.on_key(ch('l'));
        app.on_key(key(KeyCode::Right));
        assert_eq!(app.adv.as_ref().unwrap().view, View::Look(hero(&app).pos.offset(1, 0, 0)));
        assert!(screen(&mut app, 130, 40).contains("Look"));
        assert!(screen(&mut app, 80, 24).contains("Look"));
        app.on_key(key(KeyCode::Esc));
        assert_eq!(app.adv.as_ref().unwrap().view, View::Local);
        // With a foe in sight there's no rest.
        app.on_key(ch('Z'));
        assert!(app.adv.as_ref().unwrap().rest_until.is_none());
        assert!(app.adv.as_ref().unwrap().flash.as_deref().unwrap_or("").starts_with("You can't rest"));
        // The foe gone and an arm hurt: Z rests, a slice a frame, until whole.
        game_mut(&mut app).creatures.clear();
        let arm = Part::LeftArm.idx();
        let hid = adv::HERO_ID;
        {
            let d = game_mut(&mut app).dwarves.iter_mut().find(|d| d.id == hid).unwrap();
            d.body.hp = d.body.max;
            d.body.lost = [false; 7];
            d.body.hp[arm] = d.body.max[arm] * 3 / 4;
            d.body.bleeds = [0; 7];
            d.body.bleeding = 0;
            d.body.blood = d.body.max_blood;
            d.attacker = None;
        }
        let t0 = game(&app).tick;
        app.on_key(ch('Z'));
        assert!(app.adv.as_ref().unwrap().rest_until.is_some());
        for _ in 0..10 {
            app.frame();
        }
        assert!(app.adv.as_ref().unwrap().rest_until.is_none(), "the rest ended");
        assert!(hero(&app).body.healed(), "healed by resting");
        assert!(game(&app).tick > t0 + 200 && game(&app).tick <= t0 + DAY);
        // Talk: nobody near.
        app.on_key(ch('k'));
        assert!(app.adv.as_ref().unwrap().flash.as_deref().unwrap_or("").contains("no one"));
    }

    #[test]
    fn the_local_view_draws_at_every_size() {
        let mut app = field_app(24);
        app.adventure_skirmish();
        let views = [View::Local, View::Status, View::Inventory { sel: 0 }, View::Menu { sel: 1 }, View::Look(hero(&app).pos)];
        for (w, hh) in [(80, 24), (130, 40), (200, 60), (40, 12)] {
            for v in views.iter() {
                app.adv.as_mut().unwrap().view = v.clone();
                let text = screen(&mut app, w, hh);
                if *v == View::Local && w >= 80 {
                    assert!(text.contains('☺'), "the hero at {}x{}", w, hh);
                    assert!(text.contains("Log"), "the log at {}x{}", w, hh);
                    assert!(text.contains("Blood"), "the body at {}x{}", w, hh);
                }
                if *v == View::Local && w >= 118 {
                    assert!(text.contains("Gear") && text.contains("In sight") && text.contains("Goblin"), "the sidebar at {}x{}:\n{}", w, hh, text);
                }
            }
        }
        app.adv.as_mut().unwrap().view = View::Local;
        app.adv.as_mut().unwrap().help = true;
        assert!(screen(&mut app, 80, 24).contains("Keys"));
    }

    /// An app with a hero in the wild of a cell with three cells of land to
    /// the east, nothing about, and nothing met on the road.
    fn road_app(seed: u64) -> (App, (i32, i32)) {
        static WORLD: std::sync::OnceLock<Overworld> = std::sync::OnceLock::new();
        let ow = WORLD.get_or_init(|| Overworld::generate(5)).clone();
        let mut app = App::new(seed);
        app.save_dir = std::env::temp_dir().join("dwarfclone-no-saves-here");
        let mut a = adv::quick_start(ow, seed);
        a.leave();
        let ow = a.ow();
        let land = |x: i32, y: i32| x >= 0 && y >= 0 && x < ow.w && y < ow.h && !ow.cell(x, y).biome.is_water();
        let start = (0..ow.h).flat_map(|y| (0..ow.w).map(move |x| (x, y))).find(|&(x, y)| (0..4).all(|i| land(x + i, y)) && ow.site_at(x + 3, y).is_none() && ow.fort_at(x + 3, y).is_none()).expect("land");
        a.at = start;
        a.knobs.set.insert("travel.encounters".into(), 0.0);
        a.enter(adv::SiteKey::Wild(start.0, start.1)).unwrap();
        a.visit.as_mut().unwrap().g.creatures.clear();
        app.adventure_ready(Box::new(a));
        (app, start)
    }

    #[test]
    fn travel_by_keys() {
        let (mut app, start) = road_app(12);
        // `T` in the open wild: out onto the world map.
        app.on_key(key(KeyCode::Char('T')));
        let s = app.adv.as_ref().unwrap();
        assert_eq!(s.view, View::Travel, "{:?}", s.flash);
        assert!(s.adv.visit.is_none());
        assert_eq!(s.adv.at, start);
        let t0 = s.adv.clock;
        // Three cells east, by arrow and numpad: time passes, the road is told.
        for k in [KeyCode::Right, KeyCode::Char('6'), KeyCode::Right] {
            app.on_key(key(k));
        }
        let s = app.adv.as_ref().unwrap();
        assert_eq!(s.adv.at, (start.0 + 3, start.1), "{:?}", s.flash);
        assert!(s.adv.clock >= t0 + DAY, "three cells take more than a day");
        assert_eq!(s.adv.road().len(), 4, "the start and three steps");
        let text = screen(&mut app, 150, 45);
        assert!(text.contains('@') && text.contains("The road so far"), "{}", text);
        assert!(screen(&mut app, 80, 24).contains('@'), "drawn small too");
        // Enter: into the land of that cell.
        app.on_key(key(KeyCode::Enter));
        settle(&mut app);
        let s = app.adv.as_ref().unwrap();
        assert_eq!(s.view, View::Local);
        let v = s.adv.visit.as_ref().expect("in the wild");
        assert_eq!(v.key, adv::SiteKey::Wild(start.0 + 3, start.1));
        assert_eq!(v.g.site, (start.0 + 3, start.1));
        // A step off the edge of the map walks on into the next cell.
        let h = v.hero;
        let hero = game(&app).dwarves[h].pos;
        game_mut(&mut app).dwarves[h].pos = crate::geom::Pos::new(0, hero.y, hero.z);
        app.on_key(key(KeyCode::Left));
        let s = app.adv.as_ref().unwrap();
        assert_eq!(s.view, View::Travel, "{:?}", s.flash);
        assert_eq!(s.adv.at, (start.0 + 2, start.1));
    }

    #[test]
    fn met_on_the_road_by_keys() {
        let (mut app, start) = road_app(13);
        app.on_key(key(KeyCode::Char('T')));
        // Something is sure to come; Enter stands to meet it.
        app.adv.as_mut().unwrap().adv.knobs.set.insert("travel.encounters".into(), 50.0);
        // (Back and forth until something comes: the chance tops out at 90%.)
        for i in 0..20 {
            app.on_key(key(if i % 2 == 0 { KeyCode::Right } else { KeyCode::Left }));
            if app.adv.as_ref().unwrap().meet.is_some() {
                break;
            }
        }
        let s = app.adv.as_ref().unwrap();
        let e = s.meet.clone().expect("met something");
        let at = s.adv.at;
        assert!(screen(&mut app, 150, 45).contains("stand and fight"));
        app.on_key(key(KeyCode::Enter));
        settle(&mut app);
        let s = app.adv.as_ref().unwrap();
        assert_eq!(s.view, View::Local);
        let v = s.adv.visit.as_ref().expect("in the wild");
        assert_eq!(v.key, adv::SiteKey::Wild(at.0, at.1));
        assert!(at.1 == start.1 && (at.0 == start.0 || at.0 == start.0 + 1));
        let foes = v.g.creatures.values().filter(|c| c.target == Some(adv::HERO_ID)).count();
        assert_eq!(foes, e.foes.iter().map(|f| f.1 as usize).sum::<usize>(), "{:?}", e);
        // And the road can't be taken with foes at your heels.
        let near = v.g.creatures.values().find(|c| c.target == Some(adv::HERO_ID)).map(|c| c.pos).unwrap();
        let h = v.hero;
        game_mut(&mut app).dwarves[h].pos = near.offset(1, 0, 0);
        app.on_key(key(KeyCode::Char('T')));
        let s = app.adv.as_ref().unwrap();
        assert_eq!(s.view, View::Local);
        assert!(s.flash.as_deref().map_or(false, |f| f.starts_with("You can't leave now")), "{:?}", s.flash);
    }
}
