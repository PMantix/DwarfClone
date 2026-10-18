//! The director: when the overseer is playing and nobody has touched the
//! keyboard or mouse for a while, the camera wanders the fortress on its own
//! like a screensaver, cutting to fights, sieges, rescues and moods, and in
//! quiet times to dwarves at honest work or beasts in the wild.
//!
//! It only ever moves the camera (the cursor and the view); it never pauses,
//! unpauses or gives an order. Time comes in from outside (`App::frame_dt`),
//! so tests run on a fake clock.

use crate::app::{App, Mode};
use crate::creature::{Creature, Species};
use crate::dwarf::Task;
use crate::game::{Game, ZoneKind, DAY};
use crate::geom::Pos;
use crate::world::TileKind;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Seconds without a key or mouse event before the director takes over.
pub const IDLE_SECS: f64 = 20.0;
/// Seconds on a calm subject before moving on.
pub const DWELL_SECS: f64 = 9.0;
/// Seconds to linger on a fight after it ends.
pub const FIGHT_LINGER_SECS: f64 = 2.0;

/// What kind of thing is being watched (for scoring and variety).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Fight,
    Legend,
    Siege,
    Berserk,
    Monster,
    Gate,
    Rescue,
    FirstAid,
    Mood,
    /// Where blood was freshly spilled: bodies, parts, pools.
    Battlefield,
    Arrivals,
    Care,
    Caravan,
    Festival,
    Broken,
    Building,
    Construction,
    /// The curtain wall, a tower, a grand work.
    Works,
    Work,
    Tavern,
    /// A hall the overseer has just finished.
    Hall,
    Wildlife,
    /// Nothing else at all: someone going about their day.
    Idle,
    /// A wedding and its feast (`stories.rs`).
    Wedding,
    /// The feast after a broken siege.
    Feast,
}

impl Kind {
    /// How much it's worth watching, before a little jitter.
    pub fn score(self) -> i32 {
        match self {
            Kind::Fight => 100,
            Kind::Legend => 95,
            Kind::Siege => 90,
            Kind::Berserk => 85,
            Kind::Monster => 75,
            Kind::Gate => 70,
            Kind::Rescue => 70,
            Kind::FirstAid => 65,
            Kind::Mood => 60,
            Kind::Battlefield => 58,
            Kind::Arrivals => 55,
            Kind::Care => 45,
            Kind::Caravan => 45,
            Kind::Festival => 40,
            Kind::Broken => 35,
            Kind::Building => 35,
            Kind::Construction => 30,
            Kind::Works => 25,
            Kind::Work => 20,
            Kind::Tavern => 18,
            Kind::Hall => 28,
            Kind::Wildlife => 15,
            Kind::Idle => 5,
            Kind::Wedding => 50,
            Kind::Feast => 50,
        }
    }
}

/// Who or what the camera is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Who {
    Dwarf(u32),
    Creature(u32),
    Spot(Pos),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Subject {
    pub who: Who,
    pub kind: Kind,
    /// "Urist Ironfist trades blows with a goblin".
    pub what: String,
}

#[derive(Clone, Debug)]
pub struct Director {
    /// Seconds since the last key or mouse event.
    pub idle: f64,
    /// Touring now.
    pub active: bool,
    pub subject: Option<Subject>,
    /// Seconds on the current subject.
    since: f64,
    /// Seconds since the fight being watched ended, if it has.
    over_for: Option<f64>,
    /// The last few subjects, newest last.
    recent: Vec<Who>,
    last_kind: Option<Kind>,
    rng: u64,
}

impl Default for Director {
    fn default() -> Self {
        Director { idle: 0.0, active: false, subject: None, since: 0.0, over_for: None, recent: Vec::new(), last_kind: None, rng: 0x9E37_79B9_7F4A_7C15 }
    }
}

impl Director {
    /// A key or the mouse: hand the camera back at once.
    pub fn touch(&mut self) {
        self.idle = 0.0;
        self.stop();
    }

    fn stop(&mut self) {
        self.active = false;
        self.subject = None;
        self.over_for = None;
    }

    fn jitter(&mut self, n: i32) -> i32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng % n.max(1) as u64) as i32
    }

    /// The line shown over the map while touring.
    pub fn caption(&self) -> Option<String> {
        self.subject.as_ref().filter(|_| self.active).map(|s| format!("Watching: {}", s.what))
    }

    pub fn caption_line(&self) -> Option<Line<'static>> {
        let c = self.caption()?;
        Some(Line::from(vec![
            Span::styled(format!(" {} ", c), Style::default().fg(Color::Rgb(255, 225, 150)).add_modifier(Modifier::BOLD)),
            Span::styled("(any key to take back control) ", Style::default().fg(Color::Rgb(120, 120, 135))),
        ]))
    }

    /// One frame, `dt` seconds after the last. `eligible`: the overseer
    /// plays, nothing is open over the map. Returns where to look.
    pub fn update(&mut self, g: &Game, dt: f64, eligible: bool) -> Option<Pos> {
        self.idle += dt;
        if !eligible || self.idle < IDLE_SECS {
            if self.active {
                self.stop();
            }
            return None;
        }
        self.active = true;
        self.since += dt;
        // A fight breaks in on anything calmer.
        let fighting = self.subject.as_ref().map_or(false, |s| s.kind == Kind::Fight);
        if !fighting {
            if let Some(f) = self.pick(g, true) {
                self.start(f);
            }
        }
        let due = match &self.subject {
            None => true,
            Some(s) if position(g, s.who).is_none() => true,
            Some(s) if s.kind == Kind::Fight => {
                let on = matches!(s.who, Who::Dwarf(id) if fight_of(g, id).is_some());
                if on {
                    self.over_for = None;
                    false
                } else {
                    let t = self.over_for.unwrap_or(0.0) + dt;
                    self.over_for = Some(t);
                    t >= FIGHT_LINGER_SECS
                }
            }
            Some(_) => self.since >= DWELL_SECS,
        };
        if due {
            match self.pick(g, false) {
                Some(s) => self.start(s),
                None => self.subject = None,
            }
        }
        let s = self.subject.as_ref()?;
        position(g, s.who)
    }

    fn start(&mut self, s: Subject) {
        self.recent.push(s.who);
        if self.recent.len() > 4 {
            self.recent.remove(0);
        }
        self.last_kind = Some(s.kind);
        self.subject = Some(s);
        self.since = 0.0;
        self.over_for = None;
    }

    /// The best thing to watch next (only fights if `fights_only`).
    fn pick(&mut self, g: &Game, fights_only: bool) -> Option<Subject> {
        let all = if fights_only { fights(g) } else { candidates(g) };
        if all.is_empty() {
            return None;
        }
        let last = self.recent.last().copied();
        let last_kind = self.last_kind;
        // Not the same subject, nor the same kind, twice running (fights excepted);
        // loosen that only if nothing else is going on.
        let fresh = |s: &Subject, kind_too: bool| s.kind == Kind::Fight || (Some(s.who) != last && (!kind_too || Some(s.kind) != last_kind));
        let pool: Vec<&Subject> = {
            let strict: Vec<&Subject> = all.iter().filter(|s| fresh(s, true)).collect();
            if !strict.is_empty() {
                strict
            } else {
                let loose: Vec<&Subject> = all.iter().filter(|s| fresh(s, false)).collect();
                if loose.is_empty() { all.iter().collect() } else { loose }
            }
        };
        let mut best: Option<(i32, &Subject)> = None;
        for s in pool {
            let mut score = s.kind.score() + self.jitter(12);
            if s.kind != Kind::Fight && self.recent.contains(&s.who) {
                score -= 15;
            }
            if best.map_or(true, |(b, _)| score > b) {
                best = Some((score, s));
            }
        }
        best.map(|(_, s)| s.clone())
    }
}

/// Where a subject is now (None once it's gone).
pub fn position(g: &Game, who: Who) -> Option<Pos> {
    match who {
        Who::Dwarf(id) => g.dwarves.iter().find(|d| d.id == id && d.alive).map(|d| d.pos),
        Who::Creature(id) => g.creatures.get(&id).filter(|c| c.alive).map(|c| c.pos),
        Who::Spot(p) => Some(p),
    }
}

fn dist(a: Pos, b: Pos) -> i32 {
    (a.x - b.x).abs().max((a.y - b.y).abs())
}

fn seen(g: &Game, c: &Creature) -> bool {
    c.alive && !c.body.unconscious() && !c.sneaking && g.world.get(c.pos).map_or(false, |t| t.revealed)
}

/// "a goblin", "an elk", or its own name.
fn a_creature(c: &Creature) -> String {
    if let Some(l) = &c.label {
        return l.clone();
    }
    let n = c.name();
    let an = n.starts_with(|ch: char| "aeiou".contains(ch.to_ascii_lowercase()));
    format!("{} {}", if an { "an" } else { "a" }, n)
}

fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_lowercase().chain(c).collect(),
        None => String::new(),
    }
}

fn name_of(g: &Game, id: u32) -> String {
    g.dwarves.iter().find(|d| d.id == id).map_or("someone".to_string(), |d| d.name.clone())
}

/// The creature a dwarf is fighting, and whether they're at close quarters.
pub fn fight_of(g: &Game, id: u32) -> Option<(u32, bool)> {
    let d = g.dwarves.iter().find(|d| d.id == id && d.alive)?;
    let near = |c: &Creature| c.pos.z == d.pos.z && dist(c.pos, d.pos) <= 1;
    if let Task::Attack { target } = d.task {
        if let Some(c) = g.creatures.get(&target).filter(|c| c.alive && c.pos.z == d.pos.z && dist(c.pos, d.pos) <= 12) {
            return Some((c.id, near(c)));
        }
    }
    // Struck lately by something still beside them.
    if let Some((by, when)) = d.attacker {
        if g.tick <= when + 60 {
            if let Some(c) = g.creatures.get(&by).filter(|c| c.alive && near(c)) {
                return Some((c.id, true));
            }
        }
    }
    None
}

fn fights(g: &Game) -> Vec<Subject> {
    let mut v: Vec<Subject> = g
        .dwarves
        .iter()
        .filter(|d| d.alive)
        .filter_map(|d| {
            let (cid, close) = fight_of(g, d.id)?;
            let c = &g.creatures[&cid];
            let what = if close { format!("{} trades blows with {}", d.name, a_creature(c)) } else { format!("{} charges at {}", d.name, a_creature(c)) };
            Some(Subject { who: Who::Dwarf(d.id), kind: Kind::Fight, what })
        })
        .collect();
    // Beasts and armies at each other's throats, where the fort can see.
    for c in g.creatures.values().filter(|c| seen(g, c)) {
        let Some(o) = c.foe.and_then(|f| g.creatures.get(&f)).filter(|o| seen(g, o) && o.pos.z == c.pos.z && dist(o.pos, c.pos) <= 1) else { continue };
        if v.iter().any(|s| s.who == Who::Creature(o.id)) || v.len() >= 6 {
            continue;
        }
        let what = format!("{} fights {}", cap(&a_creature(c)), a_creature(o));
        v.push(Subject { who: Who::Creature(c.id), kind: Kind::Fight, what });
    }
    v
}

/// The middle of the fort: the gate the overseer built, or the wagon.
fn home(g: &Game) -> Pos {
    g.overseer.as_ref().and_then(|o| o.top).unwrap_or(g.embark)
}

/// Everything worth a look right now.
pub fn candidates(g: &Game) -> Vec<Subject> {
    let mut v = fights(g);
    let home = home(g);
    let near_fort = |p: Pos| dist(p, home) <= 30 && (p.z - home.z).abs() <= 3;

    // The enemy: an army on the map, legends and monsters near the fort.
    let mut army: Option<(&Creature, i32)> = None;
    for c in g.creatures.values().filter(|c| seen(g, c) && c.hostile()) {
        if c.siege {
            let d = dist(c.pos, home);
            if army.map_or(true, |(_, b)| d < b) {
                army = Some((c, d));
            }
        } else if c.legendary() && near_fort(c.pos) {
            v.push(Subject { who: Who::Creature(c.id), kind: Kind::Legend, what: format!("{} stalks the fort", cap(&c.title())) });
        } else if near_fort(c.pos) {
            v.push(Subject { who: Who::Creature(c.id), kind: Kind::Monster, what: format!("{} prowls near the fort", a_creature(c)) });
        }
    }
    if let Some((c, d)) = army {
        let where_ = if d <= 15 { "at the gates" } else { "on the march" };
        v.push(Subject { who: Who::Creature(c.id), kind: Kind::Siege, what: format!("the {} army {}", c.name(), where_) });
    } else if let Some((when, nb, n, _)) = &g.incoming {
        if *when > g.tick && *when <= g.tick + 2 * DAY && g.world.in_bounds(home) {
            v.push(Subject { who: Who::Spot(home), kind: Kind::Gate, what: format!("the gate, braced for {} {}", n, nb.race.plural()) });
        }
    }

    // Dwarves: madness, mercy, moods and work.
    for d in g.dwarves.iter().filter(|d| d.alive) {
        let me = Who::Dwarf(d.id);
        let (kind, what) = match &d.task {
            Task::Berserk => (Kind::Berserk, format!("{} has gone berserk", d.name)),
            Task::Catatonic => (Kind::Broken, format!("{} stares at nothing", d.name)),
            Task::Rescue { patient, .. } => (Kind::Rescue, format!("{} carries {} to safety", d.name, name_of(g, *patient))),
            Task::FirstAid { patient, .. } => (Kind::FirstAid, format!("{} binds {}'s wounds", d.name, name_of(g, *patient))),
            Task::Treat { patient, .. } => (Kind::Care, format!("{} tends to {}", d.name, name_of(g, *patient))),
            Task::Coax { patient } => (Kind::Care, format!("{} talks {} back to their senses", d.name, name_of(g, *patient))),
            Task::Mood if d.mood.is_some() => (Kind::Mood, format!("{}, in a strange mood, at work on an artifact", d.name)),
            Task::Construct { bld, .. } => match g.buildings.get(bld) {
                Some(b) => (Kind::Building, format!("{} building a {}", d.name, lower_first(b.kind.name()))),
                None => continue,
            },
            Task::BuildTile { con, .. } => match g.constructions.get(con) {
                Some(c) => match g.work_at(c.pos) {
                    Some(w) => (Kind::Construction, format!("{} raising {}", d.name, w)),
                    None => (Kind::Construction, format!("{} raising a {}", d.name, lower_first(c.kind.name()))),
                },
                None => continue,
            },
            Task::Craft { bld, .. } if d.working => match g.buildings.get(bld) {
                Some(b) => (Kind::Work, format!("{} at the {}", d.name, lower_first(b.kind.name()))),
                None => continue,
            },
            Task::Dig { target, .. } if dist(*target, d.pos) <= 1 => {
                if g.world.get(*target).map_or(false, |t| t.kind == TileKind::Tree) {
                    (Kind::Work, format!("{} felling a tree", d.name))
                } else {
                    (Kind::Work, format!("{} at the mining face", d.name))
                }
            }
            Task::Plant { .. } | Task::Harvest { .. } => (Kind::Work, format!("{} working the fields", d.name)),
            Task::Fish { .. } => (Kind::Work, format!("{} fishing", d.name)),
            Task::Gather { .. } => (Kind::Work, format!("{} gathering plants", d.name)),
            Task::Train => (Kind::Work, format!("{} at drill with the squad", d.name)),
            _ => continue,
        };
        v.push(Subject { who: me, kind, what });
    }

    // The walls and towers: a watch kept, a tower going up.
    for (who, at, what) in g.works_sights() {
        let who = match who {
            Some(id) => Who::Dwarf(id),
            None => Who::Spot(at),
        };
        if v.iter().all(|s| s.who != who) {
            v.push(Subject { who, kind: Kind::Works, what });
        }
    }

    // Newcomers: migrants just in, a caravan at the depot.
    let fresh_news = |s: &str| g.log.iter().rev().take_while(|e| e.tick + DAY >= g.tick).any(|e| e.text.contains(s));
    if fresh_news("migrants have arrived") {
        if let Some(d) = g.dwarves.iter().filter(|d| d.alive).max_by_key(|d| d.id) {
            v.push(Subject { who: Who::Dwarf(d.id), kind: Kind::Arrivals, what: format!("migrants arriving, {} among them", d.name) });
        }
    }
    if let Some(cv) = &g.caravan {
        if let Some(m) = cv.merchants.iter().filter_map(|id| g.creatures.get(id)).find(|c| seen(g, c)) {
            v.push(Subject { who: Who::Creature(m.id), kind: Kind::Caravan, what: format!("the {} caravan", cv.civ_name) });
        }
    }

    // Good times: a festival, the tavern.
    if let Some((id, wedding, what)) = g.occasion_sight() {
        v.push(Subject { who: Who::Dwarf(id), kind: if wedding { Kind::Wedding } else { Kind::Feast }, what });
    }
    if let Some(f) = g.festival.as_ref().filter(|_| g.occasion_sight().is_none()) {
        let who = f.attended.iter().find(|id| g.dwarves.iter().any(|d| d.id == **id && d.alive)).map(|id| Who::Dwarf(*id)).or(f.venue.first().map(|p| Who::Spot(*p)));
        if let Some(who) = who {
            v.push(Subject { who, kind: Kind::Festival, what: format!("the {}", f.kind.name()) });
        }
    }
    for z in g.zones.values().filter(|z| z.kind == ZoneKind::Tavern) {
        if let Some(d) = g.dwarves.iter().filter(|d| d.alive && !matches!(d.task, Task::Sleep { .. })).find(|d| z.tiles.contains(&d.pos)) {
            v.push(Subject { who: Who::Dwarf(d.id), kind: Kind::Tavern, what: format!("the tavern, {} among the drinkers", d.name) });
            break;
        }
    }

    // Halls newly finished: the great dining hall, the tavern, a suite.
    for (p, name) in g.new_halls(DAY * 20).into_iter().take(2) {
        v.push(Subject { who: Who::Spot(p), kind: Kind::Hall, what: format!("{}, newly finished", name) });
    }

    // The field of a fight just over, while the blood is fresh.
    if v.iter().all(|s| s.kind != Kind::Fight) {
        if let Some(p) = g.fresh_battlefield() {
            let parts = g.items.values().filter(|it| it.kind == crate::items::ItemKind::BodyPart && it.carried_by.is_none() && dist(it.pos, p) <= 6 && it.pos.z == p.z).count();
            let what = match parts {
                0 => "the field of the last fight, still red".to_string(),
                1 => "the field of the last fight, a severed limb in the blood".to_string(),
                n => format!("the field of the last fight, {} severed parts in the blood", n),
            };
            v.push(Subject { who: Who::Spot(p), kind: Kind::Battlefield, what });
        }
    }

    // Beasts in the wild, a pack counted as one.
    let mut packs: Vec<(u32, Species, Pos, usize)> = Vec::new();
    for c in g.creatures.values().filter(|c| seen(g, c) && wild(c)) {
        match packs.iter_mut().find(|p| p.1 == c.species && p.2.z == c.pos.z && dist(p.2, c.pos) <= 8) {
            Some(p) => p.3 += 1,
            None => packs.push((c.id, c.species, c.pos, 1)),
        }
        if packs.len() >= 24 {
            break;
        }
    }
    for (id, sp, at, n) in packs {
        if near_fort(at) && g.creatures.get(&id).map_or(false, |c| c.hostile()) {
            continue; // already a monster near the fort
        }
        let place = if trees_near(g, at) { "among the trees" } else { "in the wild" };
        let what = if n >= 2 { format!("{} {} {}", n, sp.info().plural, place) } else { format!("{} {}", a_creature(&g.creatures[&id]), place) };
        v.push(Subject { who: Who::Creature(id), kind: Kind::Wildlife, what });
    }
    if v.is_empty() {
        for d in g.dwarves.iter().filter(|d| d.alive) {
            v.push(Subject { who: Who::Dwarf(d.id), kind: Kind::Idle, what: format!("{} going about the day", d.name) });
        }
    }
    v
}

fn wild(c: &Creature) -> bool {
    !c.tame && !c.siege && c.owner.is_none() && c.figure.is_none() && !matches!(c.species, Species::Merchant | Species::Visitor | Species::Dog | Species::Cat | Species::Goat | Species::Undead | Species::Skulker)
}

fn trees_near(g: &Game, p: Pos) -> bool {
    (-3..=3).any(|dy| (-3..=3).any(|dx| g.world.get(p.offset(dx, dy, 0)).map_or(false, |t| t.kind == TileKind::Tree)))
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// One frame of camera travel from `cur` toward `to`: a cut when it's on
/// another level or far off, otherwise a few tiles closer.
pub fn approach(cur: Pos, to: Pos, (vw, vh): (i32, i32)) -> Pos {
    let far = vw.max(vh).max(40);
    let d = dist(cur, to);
    if cur.z != to.z || d > far {
        return to;
    }
    let step = (d / 5).max(2);
    Pos::new(cur.x + (to.x - cur.x).clamp(-step, step), cur.y + (to.y - cur.y).clamp(-step, step), cur.z)
}

impl App {
    /// The director's turn each frame: count idle time and, when touring,
    /// move the camera toward the subject. Never touches the game.
    pub fn direct(&mut self) {
        let eligible = self.game.overseer.is_some() && self.mode == Mode::Normal && self.spot.is_none() && !self.game.fallen && self.in_fortress();
        let Some(to) = self.director.update(&self.game, self.frame_dt, eligible) else { return };
        let r = self.map_rect;
        let cur = approach(self.cursor, to, (r.width as i32, r.height as i32));
        if !self.game.world.in_bounds(cur) {
            return;
        }
        self.free_view = false;
        self.cursor = cur;
        self.view_x = cur.x - r.width as i32 / 2;
        self.view_y = cur.y - r.height as i32 / 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    const DT: f64 = 1.0 / 30.0;

    fn app() -> App {
        let mut app = App::generated(42);
        app.mode = Mode::Normal;
        app.game.set_overseer(Some(crate::game::Style::Steady));
        app.map_rect = ratatui::layout::Rect::new(1, 2, 80, 30);
        app.frame_dt = DT;
        app
    }

    /// Frames until idle long enough, game paused so nothing moves.
    fn go_idle(app: &mut App) {
        for _ in 0..((IDLE_SECS / DT) as usize + 2) {
            app.direct();
        }
    }

    fn spawn(app: &mut App, sp: Species, at: Pos) -> u32 {
        let id = app.game.next_id;
        app.game.next_id += 1;
        let mut c = Creature::new(id, sp, at, u64::MAX);
        if sp == Species::Goblin {
            c.siege = false;
        }
        app.game.creatures.insert(id, c);
        id
    }

    /// Strip every dwarf to idle, so only what a test sets up is going on.
    fn quiet(app: &mut App) {
        for d in app.game.dwarves.iter_mut() {
            d.task = Task::Idle;
            d.working = false;
            d.mood = None;
            d.attacker = None;
        }
        app.game.creatures.clear();
        app.game.caravan = None;
        app.game.festival = None;
        app.game.incoming = None;
        app.game.log.clear();
        app.game.zones.retain(|_, z| z.kind != ZoneKind::Tavern);
    }

    #[test]
    fn idling_with_the_overseer_starts_the_tour_and_a_key_ends_it() {
        let mut app = app();
        app.direct();
        assert!(!app.director.active);
        go_idle(&mut app);
        assert!(app.director.active, "idle {} s", app.director.idle);
        assert!(app.director.caption().is_some());
        // Without the overseer, no tour.
        let mut mine = App::generated(42);
        mine.mode = Mode::Normal;
        mine.frame_dt = DT;
        go_idle(&mut mine);
        assert!(!mine.director.active);
        // A key hands control back, and still does its job.
        let before = app.cursor;
        app.on_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
        assert!(!app.director.active && app.director.caption().is_none());
        assert_eq!(app.cursor.x, before.x + 1);
        app.direct();
        assert!(!app.director.active);
        // A menu open keeps it off too.
        go_idle(&mut app);
        assert!(app.director.active);
        app.on_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        assert!(matches!(app.mode, Mode::Status { .. }));
        go_idle(&mut app);
        assert!(!app.director.active);
    }

    #[test]
    fn a_fight_draws_the_camera_and_it_follows_the_fighter() {
        let mut app = app();
        quiet(&mut app);
        // Something calm first: a dwarf at work.
        let worker = app.game.dwarves[1].id;
        app.game.dwarves[1].task = Task::Train;
        go_idle(&mut app);
        assert_eq!(app.director.subject.as_ref().map(|s| s.who), Some(Who::Dwarf(worker)));
        // A goblin falls on dwarf 0.
        let d = app.game.dwarves[0].pos;
        let gob = spawn(&mut app, Species::Goblin, d.offset(1, 0, 0));
        app.game.creatures.get_mut(&gob).unwrap().pack = 0;
        app.game.world.t_mut(d.offset(1, 0, 0)).revealed = true;
        app.game.dwarves[0].task = Task::Attack { target: gob };
        let fighter = app.game.dwarves[0].id;
        app.direct();
        let s = app.director.subject.clone().unwrap();
        assert_eq!((s.who, s.kind), (Who::Dwarf(fighter), Kind::Fight));
        let cap = app.director.caption().unwrap();
        assert!(cap.contains(&app.game.dwarves[0].name) && cap.contains("trades blows with a goblin"), "{}", cap);
        for _ in 0..30 {
            app.direct();
        }
        assert_eq!(app.cursor, app.game.dwarves[0].pos);
        // The fighter moves; the camera goes along.
        let to = d.offset(2, 1, 0);
        app.game.dwarves[0].pos = to;
        app.game.creatures.get_mut(&gob).unwrap().pos = to.offset(1, 0, 0);
        app.direct();
        assert_eq!(app.cursor, to);
        // Long after a calm subject would have changed, still on the fight.
        for _ in 0..((DWELL_SECS * 2.0 / DT) as usize) {
            app.direct();
        }
        assert_eq!(app.director.subject.as_ref().map(|s| s.kind), Some(Kind::Fight));
        // The goblin dies: a moment more, then on to something else.
        app.game.creatures.get_mut(&gob).unwrap().alive = false;
        app.director_frames(((FIGHT_LINGER_SECS - 0.5) / DT) as usize);
        assert_eq!(app.director.subject.as_ref().map(|s| s.kind), Some(Kind::Fight));
        app.director_frames((1.0 / DT) as usize);
        assert_ne!(app.director.subject.as_ref().map(|s| s.kind), Some(Kind::Fight));
    }

    #[test]
    fn calm_times_show_work_and_move_on_without_repeating() {
        let mut app = app();
        quiet(&mut app);
        let ids: Vec<u32> = app.game.dwarves.iter().take(3).map(|d| d.id).collect();
        for d in app.game.dwarves.iter_mut().take(3) {
            d.task = Task::Fish { stand: d.pos };
        }
        go_idle(&mut app);
        let mut seen = vec![app.director.subject.clone().unwrap()];
        for _ in 0..6 {
            // A new subject once the dwell time is up, not before.
            let mut frames = 0;
            while app.director.subject.as_ref() == seen.last() {
                app.direct();
                frames += 1;
                assert!(frames < 1000, "stuck on {:?}", seen.last());
            }
            let secs = frames as f64 * DT;
            assert!((secs - DWELL_SECS).abs() < 0.1, "changed after {} s", secs);
            seen.push(app.director.subject.clone().unwrap());
        }
        for s in &seen {
            assert!(matches!(s.who, Who::Dwarf(id) if ids.contains(&id)), "{:?}", s);
            assert_eq!(s.kind, Kind::Work);
            assert!(s.what.ends_with("fishing"));
        }
        for w in seen.windows(2) {
            assert_ne!(w[0].who, w[1].who, "the same subject twice running");
        }
        // The camera got there, on the subject's level.
        let at = position(&app.game, seen.last().unwrap().who).unwrap();
        app.director_frames(60);
        assert_eq!(app.cursor, at);
    }

    #[test]
    fn variety_prefers_another_kind() {
        let mut app = app();
        quiet(&mut app);
        app.game.dwarves[0].task = Task::Fish { stand: app.game.dwarves[0].pos };
        app.game.dwarves[1].task = Task::Fish { stand: app.game.dwarves[1].pos };
        app.game.dwarves[2].task = Task::Catatonic;
        go_idle(&mut app);
        let mut kinds = vec![app.director.subject.as_ref().unwrap().kind];
        for _ in 0..4 {
            app.director_frames(((DWELL_SECS + 0.5) / DT) as usize);
            kinds.push(app.director.subject.as_ref().unwrap().kind);
        }
        for w in kinds.windows(2) {
            assert_ne!(w[0], w[1], "{:?}", kinds);
        }
        assert!(kinds.contains(&Kind::Broken));
    }

    #[test]
    fn the_caption_shows_over_the_map() {
        let mut app = app();
        quiet(&mut app);
        app.game.dwarves[0].task = Task::Fish { stand: app.game.dwarves[0].pos };
        go_idle(&mut app);
        let name = app.game.dwarves[0].name.clone();
        let backend = ratatui::backend::TestBackend::new(140, 45);
        let mut term = ratatui::Terminal::new(backend).unwrap();
        term.draw(|f| crate::ui::draw(f, &mut app)).unwrap();
        let buf = term.backend().buffer().clone();
        let text: String = (0..buf.area.height)
            .map(|y| (0..buf.area.width).map(|x| buf.get(x, y).symbol().to_string()).collect::<String>() + "\n")
            .collect();
        assert!(text.contains(&format!("Watching: {} fishing", name)), "{}", text);
        assert!(text.contains("any key to take back control"));
    }

    /// Snapshot of what the simulation has done.
    fn snapshot(g: &Game) -> String {
        let mut s = format!("{} {}\n", g.tick, g.log.len());
        for d in &g.dwarves {
            s += &format!("{} {:?} {:?} {}\n", d.id, d.pos, d.task, d.alive);
        }
        for c in g.creatures.values() {
            s += &format!("{} {:?} {}\n", c.id, c.pos, c.alive);
        }
        s
    }

    #[test]
    fn the_director_never_changes_the_game() {
        let run = |tour: bool| {
            let mut app = app();
            app.paused = false;
            app.director.idle = if tour { IDLE_SECS } else { f64::NEG_INFINITY };
            for _ in 0..400 {
                app.frame();
            }
            assert_eq!(app.director.active, tour);
            assert!(!app.paused);
            (snapshot(&app.game), app.paused, app.speed)
        };
        assert_eq!(run(true), run(false));
    }

    impl App {
        fn director_frames(&mut self, n: usize) {
            for _ in 0..n {
                self.direct();
            }
        }
    }
}
