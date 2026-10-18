//! Horrors with real powers (CRE-05): a cave horror's look is no longer
//! only words. Each one made gets one to three powers, picked to fit what it
//! looks like and fixed by the world's seed: it breathes fire, spits webs
//! that hold a dwarf fast, bites with a venom that worsens the wound for a
//! day, knits its wounds shut, turns flesh stiff as stone with its gaze,
//! roars until the fort runs, or wears a hide that turns blades like
//! armour. The spotlight card and the look panel describe the signs ("Embers
//! glow between its teeth"); what they mean the fort learns in the first
//! clash, and the legends keep it ("The dwarves of Boatmurdered learned that
//! Ozthra the cave horror breathes fire").

use super::*;
use crate::body::PARTS;
use crate::creature::Creature;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Power {
    /// A gout of flame down a clear line (as a dragon's).
    Fire,
    /// Spat webs: whoever they hit is held fast, and the strands stay.
    Webs,
    /// A bite whose venom worsens the wound for a day.
    Venom,
    /// Its wounds close and its bleeding stops.
    Regrowth,
    /// A dwarf it fixes with its eyes stiffens like stone for a while.
    Gaze,
    /// A roar that sends all but the steadiest running.
    Roar,
    /// A hide like armour.
    Hide,
}

impl Power {
    pub const ALL: [Power; 7] = [Power::Fire, Power::Webs, Power::Venom, Power::Regrowth, Power::Gaze, Power::Roar, Power::Hide];

    /// What anyone can see of it.
    pub fn sign(self) -> &'static str {
        match self {
            Power::Fire => "Embers glow between its teeth.",
            Power::Webs => "Grey silk trails behind it.",
            Power::Venom => "Its fangs drip.",
            Power::Regrowth => "Its old scars have closed without a mark.",
            Power::Gaze => "Its eyes are flat and grey as slate.",
            Power::Roar => "Its chest heaves like a forge's bellows.",
            Power::Hide => "Its hide is plated like armour.",
        }
    }

    /// What the fort knows once it has seen it: "breathes fire".
    pub fn known(self) -> &'static str {
        match self {
            Power::Fire => "breathes fire",
            Power::Webs => "spits webs that hold fast",
            Power::Venom => "bites with a venom that festers",
            Power::Regrowth => "knits its wounds shut",
            Power::Gaze => "turns flesh to stone with its gaze",
            Power::Roar => "roars until the bravest run",
            Power::Hide => "wears a hide that turns blades",
        }
    }
}

/// One creature's powers, and which of them the fort has seen.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Gift {
    pub powers: Vec<Power>,
    #[serde(default)]
    pub known: Vec<Power>,
    /// How much armour its hide is worth (0 without one).
    #[serde(default)]
    pub hide: i32,
    /// No power again before this tick.
    #[serde(default)]
    pub next: u64,
}

impl Gift {
    pub fn has(&self, p: Power) -> bool {
        self.powers.contains(&p)
    }
}

/// A dwarf held where they stand: in a web, or stiff from a gaze.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hold {
    pub dwarf: u32,
    pub until: u64,
    pub stone: bool,
}

/// Venom working in a dwarf: a dose every so often until it runs out.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Venom {
    pub dwarf: u32,
    /// Whose venom ("Ozthra the cave horror").
    pub by: String,
    pub doses: u8,
    pub next: u64,
}

/// Everything the horrors leave about (saved with the fort).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Horrors {
    /// Each gifted creature's powers, by its id.
    #[serde(default)]
    pub gifts: BTreeMap<u32, Gift>,
    /// How many have been gifted here (each draws its own stream).
    #[serde(default)]
    pub made: u32,
    /// Webs on the ground, and when they rot away.
    #[serde(default)]
    pub webs: Vec<(Pos, u64)>,
    #[serde(default)]
    pub held: Vec<Hold>,
    #[serde(default)]
    pub venom: Vec<Venom>,
    /// Creatures of a made people (soldiers, merchants), by id: their civ.
    #[serde(default)]
    pub kin: BTreeMap<u32, u32>,
    /// What the fort knows of each made people's gift: (civ, power).
    #[serde(default)]
    pub kin_known: Vec<(u32, Power)>,
    /// The thing in the night (haunt.rs).
    #[serde(default)]
    pub haunt: Option<super::haunt::Haunt>,
}

/// How often the horrors' round runs (schedule.rs).
pub const HORROR_TICK: u64 = 20;

impl Game {
    /// A stream of its own for the `n`th horror gifted at this site: the
    /// same world, site and order give the same powers, and the fort's own
    /// dice are left alone.
    pub(super) fn horror_rng(&self, n: u32, salt: u64) -> Rng {
        let (x, y) = self.site;
        let seed = self.overworld.seed ^ ((x as u64) << 24) ^ ((y as u64) << 40) ^ salt;
        Rng::new(crate::rng::splitmix(seed).wrapping_add(n as u64 * 0x9E37_79B9))
    }

    /// Give a new one-of-a-kind creature its powers (`lo..=hi` of them),
    /// leaning on what it looks like, and add their signs to its look.
    pub(super) fn gift_horror(&mut self, id: u32, lo: i32, hi: i32) {
        let n = self.horrors.made;
        self.horrors.made += 1;
        let mut rng = self.horror_rng(n, 0x40_77);
        let desc = self.creatures.get(&id).and_then(|c| c.desc.clone()).unwrap_or_default();
        let powers = pick_powers(&mut rng, &desc, lo, hi);
        self.gift_powers(id, powers, &mut rng);
    }

    /// Give `id` exactly these powers.
    pub(super) fn gift_powers(&mut self, id: u32, powers: Vec<Power>, rng: &mut Rng) {
        let hide = if powers.contains(&Power::Hide) { rng.range(3, 6) } else { 0 };
        if let Some(c) = self.creatures.get_mut(&id) {
            let signs: Vec<&str> = powers.iter().map(|p| p.sign()).collect();
            let desc = c.desc.clone().unwrap_or_default();
            c.desc = Some(if desc.is_empty() { signs.join(" ") } else { format!("{} {}", desc, signs.join(" ")) });
        }
        self.horrors.gifts.insert(id, Gift { powers, known: Vec::new(), hide, next: 0 });
    }

    /// Its hide counts as armour on every part (`creature_fighter`).
    pub(super) fn gifted_fighter(&self, c: &Creature, f: &mut super::combat::Fighter) {
        let Some(g) = self.horrors.gifts.get(&c.id) else { return };
        if g.hide > 0 {
            f.helm += g.hide;
            f.mail += g.hide;
        }
    }

    /// The fort has seen one of its powers at work: noted once, in the
    /// announcements and the legends.
    pub(super) fn learn_power(&mut self, id: u32, title: &str, p: Power) {
        let Some(g) = self.horrors.gifts.get_mut(&id) else { return };
        if !g.has(p) || g.known.contains(&p) {
            return;
        }
        g.known.push(p);
        // Seen in one of a made people, it's known of them all (kin.rs).
        let title = match self.kin_of(id) {
            Some((civ, all)) => {
                self.kin_learned(civ, p);
                all
            }
            None => title.to_string(),
        };
        self.count("horrors.learned", 1);
        let fort = self.fortress_name.clone();
        self.msg(&format!("Now we know: {} {}!", title, p.known()), (230, 160, 255));
        self.legend(format!("The dwarves of {} learned that {} {}.", fort, title, p.known()));
    }

    /// A creature has died (the end of `creature_died`): its powers go
    /// with it, and one of a made people's body is named for them.
    pub(super) fn horror_died(&mut self, c: &Creature) {
        self.horrors.gifts.remove(&c.id);
        self.kin_died(c);
        self.haunt_died(c);
    }

    /// Before it closes or strikes: a power aimed at the dwarf it hunts, at
    /// `dpos`, if one is ready and fits the range. True if it used one.
    pub(super) fn power_turn(&mut self, c: &mut Creature, dpos: Pos) -> bool {
        let Some(g) = self.horrors.gifts.get(&c.id) else { return false };
        if c.attack_cd > 0 || self.tick < g.next || c.body.unconscious() {
            return false;
        }
        let powers = g.powers.clone();
        let beside = super::combat::adjacent(c.pos, dpos);
        let within = |r: i32| super::combat::near(dpos, c.pos, r) && dpos.z == c.pos.z;
        let line = self.clear_shot(c.pos, dpos);
        let title = c.title();
        let delay = c.species.info().attack_delay;
        for p in powers {
            let used = match p {
                Power::Fire if !beside && within(7) && line => {
                    self.breathe_fire(c, dpos);
                    self.set_next(c.id, self.knob("horror.fire_cd", 120.0));
                    true
                }
                Power::Webs if !beside && within(6) && line => {
                    self.spit_web(c, dpos);
                    self.set_next(c.id, self.knob("horror.web_cd", 150.0));
                    true
                }
                Power::Gaze if within(5) && line => {
                    self.stony_gaze(c, dpos);
                    self.set_next(c.id, self.knob("horror.gaze_cd", 260.0));
                    true
                }
                Power::Roar if within(8) && self.rng.chance(0.5) => {
                    self.roar(c);
                    self.set_next(c.id, self.knob("horror.roar_cd", 400.0));
                    true
                }
                _ => false,
            };
            if used {
                c.attack_cd = delay;
                self.learn_power(c.id, &title, p);
                return true;
            }
        }
        false
    }

    fn set_next(&mut self, id: u32, ticks: f64) {
        let tick = self.tick;
        if let Some(g) = self.horrors.gifts.get_mut(&id) {
            g.next = tick + ticks as u64;
        }
    }

    /// The dwarf standing at `p`, if any (index).
    fn dwarf_at(&self, p: Pos) -> Option<usize> {
        self.dwarves.iter().position(|d| d.alive && d.pos == p)
    }

    /// Hold a dwarf fast until `until` (a web, or stone).
    pub(super) fn hold(&mut self, dwarf: u32, ticks: u64, stone: bool) {
        let until = self.tick + ticks;
        match self.horrors.held.iter_mut().find(|h| h.dwarf == dwarf) {
            Some(h) => {
                h.until = h.until.max(until);
                h.stone |= stone;
            }
            None => self.horrors.held.push(Hold { dwarf, until, stone }),
        }
    }

    /// Whether a dwarf is held fast just now.
    pub fn held_fast_now(&self, dwarf: u32) -> Option<&Hold> {
        self.horrors.held.iter().find(|h| h.dwarf == dwarf && h.until > self.tick)
    }

    /// A glob of web at whoever it hunts: they're stuck, and strands lie
    /// about where they stand.
    fn spit_web(&mut self, c: &Creature, to: Pos) {
        let rot = self.tick + DAY;
        let mut spots = vec![to];
        for &(dx, dy) in DIRS8.iter() {
            let q = to.offset(dx, dy, 0);
            if self.world.in_bounds(q) && self.world.walkable(q) && self.rng.chance(0.3) {
                spots.push(q);
            }
        }
        for p in spots {
            if !self.horrors.webs.iter().any(|w| w.0 == p) {
                self.horrors.webs.push((p, rot));
            }
        }
        let held = self.knob("horror.web_hold", 150.0) as u64;
        let tick = self.tick;
        let mut stuck = None;
        if let Some(i) = self.dwarf_at(to) {
            let id = self.dwarves[i].id;
            self.dwarves[i].think("was caught in a monster's web", 15, tick);
            stuck = Some((id, self.dwarves[i].called()));
        }
        match stuck {
            Some((id, name)) => {
                self.hold(id, held, false);
                self.horrors.webs.retain(|w| w.0 != to);
                self.count("horrors.webbed", 1);
                self.msg(&format!("{} spits a web! {} is stuck fast.", cap(&c.title()), name), (220, 220, 235));
            }
            None => self.msg(&format!("{} spits a web!", cap(&c.title())), (220, 220, 235)),
        }
    }

    /// It fixes the dwarf at `to` with its eyes, and they stiffen like stone.
    fn stony_gaze(&mut self, c: &Creature, to: Pos) {
        let Some(i) = self.dwarf_at(to) else { return };
        let tick = self.tick;
        let (id, name) = (self.dwarves[i].id, self.dwarves[i].called());
        self.dwarves[i].think("felt their limbs turn to stone", 25, tick);
        let held = self.knob("horror.gaze_hold", 220.0) as u64;
        self.hold(id, held, true);
        self.count("horrors.petrified", 1);
        self.msg(&format!("{} fixes {} with its gaze. {} stiffens like stone!", cap(&c.title()), name, name), (200, 200, 210));
    }

    /// A roar: everyone near who isn't a hardened fighter runs.
    fn roar(&mut self, c: &Creature) {
        let tick = self.tick;
        let title = c.title();
        let steady = self.knob("horror.roar_steady", 5.0) as u32;
        let mut fled = 0;
        for i in 0..self.dwarves.len() {
            let d = &self.dwarves[i];
            if !d.alive || d.body.unconscious() || !super::combat::near(d.pos, c.pos, 8) {
                continue;
            }
            if d.soldier && d.fight_level() >= steady {
                continue;
            }
            let mut d = std::mem::take(&mut self.dwarves[i]);
            d.think(&format!("was terrified by the roar of {}", title), 20, tick);
            if d.task != Task::Flee {
                self.release_task(&mut d);
                d.task = Task::Flee;
                d.path.clear();
            }
            self.dwarves[i] = d;
            fled += 1;
        }
        self.count("horrors.roared", fled);
        let tail = if fled > 0 { format!(" {} flee before it.", if fled == 1 { "One dwarf".to_string() } else { format!("{} dwarves", fled) }) } else { String::new() };
        self.msg(&format!("{} roars, and the halls shake!{}", cap(&title), tail), (255, 170, 90));
    }

    /// After its blow lands on a dwarf: venom goes in with the bite.
    pub(super) fn gifted_blow(&mut self, c: &Creature, d: &mut Dwarf, hurt: bool) {
        let Some(g) = self.horrors.gifts.get(&c.id) else { return };
        if !hurt || d.body.dead || !g.has(Power::Venom) {
            return;
        }
        let doses = self.knob("horror.venom_doses", 5.0) as u8;
        let next = self.tick + self.knob("horror.venom_every", 160.0) as u64;
        let title = c.title();
        match self.horrors.venom.iter_mut().find(|v| v.dwarf == d.id) {
            Some(v) => v.doses = v.doses.max(doses),
            None => {
                self.horrors.venom.push(Venom { dwarf: d.id, by: title.clone(), doses, next });
                self.msg(&format!("{} has been poisoned by {}!", d.called(), title), (170, 230, 90));
                self.count("horrors.poisoned", 1);
            }
        }
        self.learn_power(c.id, &title, Power::Venom);
    }

    /// A dwarf struck at it: its hide shows, and its bane bites (haunt.rs).
    pub(super) fn struck_horror(&mut self, id: u32, d: &Dwarf, body: &mut crate::body::Body, hp0: i32) {
        let hurt = body.hp.iter().sum::<i32>() < hp0;
        self.bane_blow(id, d, body, hurt);
        let Some(g) = self.horrors.gifts.get(&id) else { return };
        if g.has(Power::Hide) && !g.known.contains(&Power::Hide) {
            let title = self.creatures.get(&id).map_or_else(|| "it".to_string(), |c| c.title());
            self.learn_power(id, &title, Power::Hide);
        }
    }

    /// Each dwarf, every tick (schedule `body_phase`): held fast by a web or
    /// stiff as stone, they can neither step nor strike.
    pub(super) fn held_fast(&mut self, d: &mut Dwarf) {
        if self.horrors.held.is_empty() {
            return;
        }
        if self.horrors.held.iter().any(|h| h.dwarf == d.id && h.until > self.tick) {
            d.move_cd = d.move_cd.max(2);
            d.attack_cd = d.attack_cd.max(2);
        }
    }

    /// Every `HORROR_TICK` ticks: wounds knit, venom works, webs catch the
    /// careless and rot, and the held come free.
    pub(super) fn horrors_tick(&mut self) {
        let h = &self.horrors;
        if h.gifts.is_empty() && h.webs.is_empty() && h.held.is_empty() && h.venom.is_empty() {
            return;
        }
        self.bane_tick();
        let tick = self.tick;
        // The dead and the departed keep no powers.
        let creatures = &self.creatures;
        self.horrors.gifts.retain(|id, _| creatures.get(id).map_or(false, |c| c.alive));
        self.regrowth();
        self.venom_doses();
        // Webs: rot away, or catch whoever walks into them.
        self.horrors.webs.retain(|w| w.1 > tick);
        if !self.horrors.webs.is_empty() {
            let caught: Vec<(usize, Pos)> = self
                .dwarves
                .iter()
                .enumerate()
                .filter(|(_, d)| d.alive && self.horrors.webs.iter().any(|w| w.0 == d.pos) && !self.horrors.held.iter().any(|h| h.dwarf == d.id))
                .map(|(i, d)| (i, d.pos))
                .collect();
            for (i, p) in caught {
                let id = self.dwarves[i].id;
                let name = self.dwarves[i].called();
                self.horrors.webs.retain(|w| w.0 != p);
                let held = self.knob("horror.web_hold", 150.0) as u64 / 2;
                self.hold(id, held, false);
                self.dwarves[i].think("blundered into a sticky web", 8, tick);
                self.msg(&format!("{} has blundered into a web and is stuck fast!", name), (220, 220, 235));
            }
        }
        let freed: Vec<Hold> = self.horrors.held.iter().filter(|h| h.until <= tick).cloned().collect();
        self.horrors.held.retain(|h| h.until > tick);
        for h in freed {
            if let Some(d) = self.dwarves.iter().find(|d| d.id == h.dwarf && d.alive) {
                let what = if h.stone { "can move again as the stone leaves their limbs" } else { "has torn free of the web" };
                let text = format!("{} {}.", d.called(), what);
                self.msg(&text, (200, 200, 210));
            }
        }
    }

    /// Wounds close on those that knit them (seen to by the fort once a
    /// dwarf is near enough to watch).
    fn regrowth(&mut self) {
        let ids: Vec<u32> = self.horrors.gifts.iter().filter(|(_, g)| g.has(Power::Regrowth)).map(|(id, _)| *id).collect();
        let rate = self.knob("horror.regrow", 2.0) as i32;
        for id in ids {
            if self.regrowth_checked(id) {
                continue;
            }
            let Some(c) = self.creatures.get_mut(&id) else { continue };
            let hurt = c.body.bleeding > 0 || PARTS.iter().any(|p| !c.body.lost[p.idx()] && c.body.hp[p.idx()] < c.body.max[p.idx()]);
            if !hurt {
                continue;
            }
            c.body.bleeds = [0; 7];
            c.body.bleeding = 0;
            c.body.blood = (c.body.blood + rate * 2).min(c.body.max_blood);
            for p in PARTS {
                let i = p.idx();
                if !c.body.lost[i] {
                    c.body.hp[i] = (c.body.hp[i] + rate).min(c.body.max[i]);
                }
            }
            let (pos, title) = (c.pos, c.title());
            if self.dwarves.iter().any(|d| d.alive && super::combat::near(d.pos, pos, 6)) {
                self.learn_power(id, &title, Power::Regrowth);
            }
        }
    }

    /// Whether something stops its wounds closing just now (see `haunt.rs`
    /// for the bane that does; fire always does).
    fn regrowth_checked(&self, id: u32) -> bool {
        let Some(c) = self.creatures.get(&id) else { return true };
        let burning = DIRS8.iter().map(|&(dx, dy)| c.pos.offset(dx, dy, 0)).chain([c.pos]).any(|q| self.world.get(q).map_or(false, |t| t.fire > 0));
        burning || self.bane_stops_regrowth(id)
    }

    /// Venom works its way in: a dose hurts somewhere new, until it's spent.
    fn venom_doses(&mut self) {
        if self.horrors.venom.is_empty() {
            return;
        }
        let tick = self.tick;
        let every = self.knob("horror.venom_every", 160.0) as u64;
        let dmg = self.knob("horror.venom", 2.0) as i32;
        let mut done = Vec::new();
        for k in 0..self.horrors.venom.len() {
            let v = self.horrors.venom[k].clone();
            if v.next > tick {
                continue;
            }
            let Some(i) = self.dwarves.iter().position(|d| d.id == v.dwarf && d.alive) else {
                done.push(k);
                continue;
            };
            let mut d = std::mem::take(&mut self.dwarves[i]);
            // It works on the worst of their wounds, and thins the blood.
            let part = PARTS
                .iter()
                .copied()
                .filter(|p| !d.body.lost[p.idx()])
                .min_by_key(|p| d.body.hp[p.idx()] * 100 / d.body.max[p.idx()].max(1))
                .unwrap_or(crate::body::Part::UpperBody);
            d.body.wound(part, dmg, false);
            d.body.blood -= dmg * 3;
            if d.body.blood <= 0 {
                d.body.dead = true;
            }
            d.think("felt the venom burning in their veins", 6, tick);
            if d.body.dead {
                self.kill(&mut d, &format!("has died of the venom of {}", v.by));
                self.count("horrors.venom_deaths", 1);
            }
            self.dwarves[i] = d;
            let left = v.doses.saturating_sub(1);
            if left == 0 {
                done.push(k);
            } else {
                self.horrors.venom[k].doses = left;
                self.horrors.venom[k].next = tick + every;
            }
        }
        for k in done.into_iter().rev() {
            let v = self.horrors.venom.remove(k);
            if let Some(d) = self.dwarves.iter().find(|d| d.id == v.dwarf && d.alive) {
                let text = format!("The venom in {} has run its course.", d.called());
                self.msg(&text, (170, 230, 90));
            }
        }
    }

    /// A web lies here.
    pub fn web_at(&self, p: Pos) -> bool {
        self.horrors.webs.iter().any(|w| w.0 == p)
    }

    /// Lines for the look panel: its powers as the fort knows them.
    pub fn horror_lines(&self, c: &Creature) -> Vec<String> {
        let Some(g) = self.horrors.gifts.get(&c.id) else { return Vec::new() };
        let mut v = self.haunt_look(c);
        if !g.known.is_empty() {
            let known: Vec<&str> = g.powers.iter().filter(|p| g.known.contains(p)).map(|p| p.known()).collect();
            v.push(format!("Known: it {}.", join_and(&known)));
        }
        let unknown = g.powers.iter().filter(|p| !g.known.contains(p)).count();
        if unknown > 0 {
            v.push(format!("{} more {} the fort has yet to learn.", if unknown == 1 { "One" } else { "Some" }, if unknown == 1 { "power" } else { "powers" }));
        }
        v
    }

    /// Lines for a dwarf's look panel: held fast or poisoned.
    pub fn affliction_lines(&self, d: &Dwarf) -> Vec<String> {
        let mut v = Vec::new();
        if let Some(h) = self.held_fast_now(d.id) {
            v.push(if h.stone { "Stiff as stone from a monster's gaze".to_string() } else { "Stuck fast in a web".to_string() });
        }
        if let Some(p) = self.horrors.venom.iter().find(|p| p.dwarf == d.id) {
            v.push(format!("Poisoned by the venom of {}", p.by));
        }
        v
    }
}

/// "a, b and c".
pub fn join_and(v: &[&str]) -> String {
    match v.len() {
        0 => String::new(),
        1 => v[0].to_string(),
        n => format!("{} and {}", v[..n - 1].join(", "), v[n - 1]),
    }
}

/// `lo..=hi` powers, likelier the ones its look suggests: a spider spins,
/// a thing of iron or crystal is plated, many eyes stare, a maw roars, ash
/// burns, slime and fungus fester and knit.
pub fn pick_powers(rng: &mut Rng, look: &str, lo: i32, hi: i32) -> Vec<Power> {
    let l = look.to_lowercase();
    let mut weights: Vec<(Power, i32)> = Power::ALL.iter().map(|&p| (p, 4)).collect();
    let mut lean = |p: Power, w: i32| {
        if let Some(x) = weights.iter_mut().find(|x| x.0 == p) {
            x.1 += w;
        }
    };
    for (word, p) in [
        ("spider", Power::Webs),
        ("bat", Power::Roar),
        ("worm", Power::Venom),
        ("serpent", Power::Venom),
        ("beetle", Power::Hide),
        ("lizard", Power::Regrowth),
        ("crystal", Power::Hide),
        ("iron", Power::Hide),
        ("bone", Power::Hide),
        ("ash", Power::Fire),
        ("slime", Power::Regrowth),
        ("fungus", Power::Venom),
        ("shadow", Power::Gaze),
        ("eyes", Power::Gaze),
        ("maw", Power::Roar),
        ("tentacles", Power::Webs),
        ("veins", Power::Fire),
    ] {
        if l.contains(word) {
            lean(p, 10);
        }
    }
    let n = rng.range(lo, hi + 1).max(1) as usize;
    let mut out = Vec::new();
    while out.len() < n.min(Power::ALL.len()) {
        let total: i32 = weights.iter().filter(|w| !out.contains(&w.0)).map(|w| w.1).sum();
        let mut r = rng.range(0, total.max(1));
        for &(p, w) in weights.iter().filter(|w| !out.contains(&w.0)) {
            r -= w;
            if r < 0 {
                out.push(p);
                break;
            }
        }
    }
    out.sort();
    out
}

fn cap(s: &str) -> String {
    super::combat::cap(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::Species;

    /// A horror on open ground two tiles from a dwarf, with these powers.
    fn face_off(seed: u64, powers: Vec<Power>, gap: i32) -> (Game, u32, u32) {
        let mut g = Game::new(seed);
        g.creatures.retain(|_, c| c.tame);
        g.weather = Weather::Clear;
        let a = g.open_patch(12, 5, 6);
        let did = g.dwarves[0].id;
        g.dwarves[0].pos = a.offset(1 + gap, 2, 0);
        g.dwarves[0].path.clear();
        g.dwarves[0].task = Task::Idle;
        let id = g.spawn_creature(Species::CaveHorror, a.offset(1, 2, 0), u64::MAX);
        g.creatures.get_mut(&id).unwrap().label = Some("Ozthra the cave horror".into());
        let mut rng = Rng::new(1);
        g.gift_powers(id, powers, &mut rng);
        (g, id, did)
    }

    #[test]
    fn the_same_world_and_site_make_the_same_horror() {
        let mut a = Game::new(81);
        let mut b = Game::new(81);
        let pa = a.cavern_floors()[0];
        let ia = a.make_cave_horror(pa);
        let ib = b.make_cave_horror(pa);
        assert_eq!(a.horrors.gifts[&ia].powers, b.horrors.gifts[&ib].powers);
        assert!(!a.horrors.gifts[&ia].powers.is_empty());
        // Its look carries the signs of its powers.
        let desc = a.creatures[&ia].desc.clone().unwrap();
        for p in &a.horrors.gifts[&ia].powers {
            assert!(desc.contains(p.sign()), "{} lacks the sign of {:?}", desc, p);
        }
        // And they keep through a save.
        let back: Horrors = serde_json::from_str(&serde_json::to_string(&a.horrors).unwrap()).unwrap();
        assert_eq!(back.gifts[&ia], a.horrors.gifts[&ia]);
        // The second horror at a site is another draw.
        let many: Vec<Vec<Power>> = (0..6).map(|_| { let id = a.make_cave_horror(pa); a.horrors.gifts[&id].powers.clone() }).collect();
        assert!(many.iter().any(|p| *p != many[0]), "every horror alike: {:?}", many);
    }

    #[test]
    fn a_spider_spins_and_plated_things_wear_hides() {
        let mut webs = 0;
        let mut hides = 0;
        for s in 0..200 {
            let mut r = Rng::new(s);
            webs += pick_powers(&mut r, "A bloated spider of slime with many eyes.", 1, 1).contains(&Power::Webs) as i32;
            let mut r = Rng::new(s);
            hides += pick_powers(&mut r, "A hulking beetle of iron with a gaping maw.", 1, 1).contains(&Power::Hide) as i32;
        }
        assert!(webs > 40 && hides > 40, "webs {} hides {}", webs, hides);
    }

    #[test]
    fn its_breath_burns_and_the_fort_learns_it() {
        let (mut g, id, did) = face_off(81, vec![Power::Fire], 4);
        let mut c = g.creatures.remove(&id).unwrap();
        let dpos = g.dwarves[0].pos;
        assert!(g.power_turn(&mut c, dpos), "the breath should be ready");
        g.creatures.insert(id, c);
        assert!(g.log.iter().any(|l| l.text.ends_with("breathes fire!")));
        assert!(g.dwarves.iter().find(|d| d.id == did).unwrap().thoughts.iter().any(|t| t.text.contains("scorched")));
        assert_eq!(g.horrors.gifts[&id].known, vec![Power::Fire]);
        assert!(g.overworld.events.iter().any(|e| e.text.contains("learned that Ozthra the cave horror breathes fire")));
        // Not again until it's ready.
        let mut c = g.creatures.remove(&id).unwrap();
        c.attack_cd = 0;
        assert!(!g.power_turn(&mut c, dpos));
        // The look panel says what's known.
        g.creatures.insert(id, c);
        let lines = g.horror_lines(&g.creatures[&id]);
        assert!(lines[0].contains("breathes fire"), "{:?}", lines);
    }

    #[test]
    fn a_web_holds_a_dwarf_fast_until_they_tear_free() {
        let (mut g, id, did) = face_off(81, vec![Power::Webs], 4);
        let mut c = g.creatures.remove(&id).unwrap();
        let start = g.dwarves[0].pos;
        assert!(g.power_turn(&mut c, start));
        g.creatures.insert(id, c);
        assert!(g.held_fast_now(did).is_some(), "the dwarf should be stuck");
        // They can't step while it holds, whatever they try.
        g.creatures.remove(&id);
        let away = start.offset(4, 0, 0);
        for _ in 0..100 {
            let mut d = std::mem::take(&mut g.dwarves[0]);
            g.held_fast(&mut d);
            let _ = g.travel(&mut d, away);
            g.dwarves[0] = d;
        }
        assert_eq!(g.dwarves[0].pos, start, "a webbed dwarf walked off");
        assert!(g.affliction_lines(&g.dwarves[0])[0].contains("web"));
        // Then they tear free.
        g.tick += 400;
        g.horrors_tick();
        assert!(g.held_fast_now(did).is_none());
        assert!(g.log.iter().any(|l| l.text.contains("torn free")));
        // The strands left about catch whoever walks into them.
        let strand = g.horrors.webs.first().map(|w| w.0);
        if let Some(p) = strand {
            g.dwarves[0].pos = p;
            g.horrors_tick();
            assert!(g.held_fast_now(did).is_some(), "walked into a web and wasn't caught");
        }
    }

    #[test]
    fn venom_worsens_the_wound_and_can_kill() {
        let (mut g, id, did) = face_off(81, vec![Power::Venom], 1);
        let c = g.creatures[&id].clone();
        let mut d = std::mem::take(&mut g.dwarves[0]);
        g.gifted_blow(&c, &mut d, true);
        g.dwarves[0] = d;
        assert_eq!(g.horrors.venom.len(), 1);
        assert!(g.horrors.gifts[&id].known.contains(&Power::Venom));
        let hp0: i32 = g.dwarves[0].body.hp.iter().sum();
        for _ in 0..3 {
            g.tick += 200;
            g.venom_doses();
        }
        let hp1: i32 = g.dwarves[0].body.hp.iter().sum();
        assert!(hp1 < hp0, "the venom did nothing ({} -> {})", hp0, hp1);
        // Strong enough venom kills, and the annals say whose it was.
        g.knobs.set.insert("horror.venom".into(), 40.0);
        g.horrors.venom[0].doses = 10;
        for _ in 0..10 {
            g.tick += 200;
            g.venom_doses();
        }
        let d = g.dwarves.iter().find(|d| d.id == did).unwrap();
        assert!(!d.alive, "a deadly venom should kill");
        assert!(g.log.iter().any(|l| l.text.contains("died of the venom of Ozthra")));
        assert!(g.horrors.venom.is_empty());
    }

    #[test]
    fn its_wounds_knit_unless_it_stands_in_fire() {
        let (mut g, id, _) = face_off(81, vec![Power::Regrowth], 2);
        let c = g.creatures.get_mut(&id).unwrap();
        c.body.wound(crate::body::Part::LeftArm, 10, true);
        c.body.wound(crate::body::Part::UpperBody, 8, true);
        assert!(c.body.bleeding > 0);
        let hurt: i32 = c.body.hp.iter().sum();
        for _ in 0..5 {
            g.horrors_tick();
        }
        let c = &g.creatures[&id];
        assert_eq!(c.body.bleeding, 0, "it should stop bleeding");
        assert!(c.body.hp.iter().sum::<i32>() > hurt);
        assert!(g.horrors.gifts[&id].known.contains(&Power::Regrowth), "a dwarf two tiles off should see it");
        // Fire at its feet stops it.
        let p = g.creatures[&id].pos;
        g.creatures.get_mut(&id).unwrap().body.wound(crate::body::Part::LeftLeg, 6, false);
        let before: i32 = g.creatures[&id].body.hp.iter().sum();
        g.world.t_mut(p.offset(1, 0, 0)).fire = 10;
        g.horrors_tick();
        assert_eq!(g.creatures[&id].body.hp.iter().sum::<i32>(), before, "it knit while burning");
    }

    #[test]
    fn a_gaze_stiffens_and_a_roar_scatters() {
        let (mut g, id, did) = face_off(81, vec![Power::Gaze], 3);
        let mut c = g.creatures.remove(&id).unwrap();
        let dpos = g.dwarves[0].pos;
        assert!(g.power_turn(&mut c, dpos));
        g.creatures.insert(id, c);
        assert!(g.held_fast_now(did).map_or(false, |h| h.stone));
        assert!(g.log.iter().any(|l| l.text.contains("stiffens like stone")));
        let (mut g, id, did) = face_off(81, vec![Power::Roar], 3);
        g.dwarves[0].soldier = false;
        let mut c = g.creatures.remove(&id).unwrap();
        let dpos = g.dwarves[0].pos;
        let mut roared = false;
        for _ in 0..40 {
            c.attack_cd = 0;
            if g.power_turn(&mut c, dpos) {
                roared = true;
                break;
            }
        }
        assert!(roared, "it never roared");
        assert_eq!(g.dwarves.iter().find(|d| d.id == did).unwrap().task, Task::Flee);
        assert!(g.horrors.gifts[&id].known.contains(&Power::Roar));
    }

    #[test]
    fn a_hide_turns_blows_like_armour() {
        let (g, id, _) = face_off(81, vec![Power::Hide], 1);
        let c = &g.creatures[&id];
        let f = g.creature_fighter(c);
        let hide = g.horrors.gifts[&id].hide;
        assert!(hide >= 3);
        assert_eq!((f.helm, f.mail), (hide, hide));
        let (g2, id2, _) = face_off(81, vec![Power::Fire], 1);
        let f2 = g2.creature_fighter(&g2.creatures[&id2]);
        assert_eq!((f2.helm, f2.mail), (0, 0));
    }

    #[test]
    fn a_horror_fights_with_its_powers_in_a_real_fight() {
        // Left to it, a horror with a ranged power uses it on the dwarves.
        let (mut g, id, _) = face_off(81, vec![Power::Webs, Power::Gaze], 5);
        g.creatures.get_mut(&id).unwrap().siege = true;
        for _ in 0..1500 {
            g.step();
            if g.horrors.gifts.get(&id).map_or(false, |x| !x.known.is_empty()) {
                break;
            }
        }
        assert!(g.horrors.gifts.get(&id).map_or(false, |x| !x.known.is_empty()), "it never used a power");
    }
}
