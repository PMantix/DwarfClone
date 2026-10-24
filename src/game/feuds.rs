//! Feuds and factions (C03): grudges grow into fort politics.
//!
//! - *Brawls*: two dwarves come to blows (drunk, or at the end of a feud).
//!   They close and trade punches (bruises, not deaths) until one side is
//!   beaten or yields, a peacemaker pulls them apart, the alarm sounds, or
//!   they tire of it. The one who started it has an assault on the record.
//! - *Factions in a brawl* (SOC-04): onlookers who care more for one
//!   brawler than the other (kin count for a lot) wade in on that side; a
//!   calm, sociable dwarf may pull them apart instead. Afterwards everyone
//!   across the sides thinks less of each other, and a brawl big enough
//!   splits the fort into two factions for a year. Factions glare at each
//!   other at meals and won't stand together in a squad, and the overseer
//!   moves them apart.
//! - *The slow road to violence* (SOC-08): two dwarves who can't stand
//!   each other go from the cold shoulder to shouting, to slander told to a
//!   friend, and at last to blows when they next meet.

use super::combat::adjacent;
use super::overseer::Overseer;
use super::*;
use serde::{Deserialize, Serialize};

fn dist2(a: Pos, b: Pos) -> i32 {
    (a.x - b.x).pow(2) + (a.y - b.y).pow(2) + (a.z - b.z).pow(2)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrawlCause {
    #[default]
    Grudge,
    Drunk,
    Feud,
}

/// A fight with fists, going on now.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Brawl {
    pub id: u32,
    pub at: Pos,
    pub sides: [Vec<u32>; 2],
    pub cause: BrawlCause,
    pub started: u64,
    pub blows: u32,
    /// Those beaten or who gave up (still counted on their side after).
    #[serde(default)]
    pub out: Vec<u32>,
}

/// Two sides that came to blows, and hold it against each other.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Faction {
    pub sides: [Vec<u32>; 2],
    pub names: [String; 2],
    pub since: u64,
}

/// Two dwarves on the slow road from the cold shoulder to blows.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Feud {
    pub a: u32,
    pub b: u32,
    /// 1 cold shoulder, 2 shouting, 3 slander, 4 blows when next they meet.
    pub stage: u8,
    pub last: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Feuds {
    #[serde(default)]
    pub brawls: Vec<Brawl>,
    #[serde(default)]
    pub next: u32,
    #[serde(default)]
    pub factions: Vec<Faction>,
    #[serde(default)]
    pub ladder: Vec<Feud>,
    /// When each brawl began (the last year's), for the overseer's sense.
    #[serde(default)]
    pub brawl_log: Vec<u64>,
    /// Pairs who glared at each other today: (a, b, when).
    #[serde(default)]
    pub glared: Vec<(u32, u32, u64)>,
    /// Squadmates the overseer has already parted.
    #[serde(default)]
    pub parted: Vec<(u32, u32)>,
}

/// How long a brawl can go on.
const BRAWL_TICKS: u64 = 300;
/// How far onlookers see a brawl from.
const CROWD: i32 = 6;

const LIES: [&str; 6] = [
    "waters the ale",
    "steals from the stores",
    "cried when the goblins came",
    "was thrown out of their last fortress",
    "talks to the rats",
    "never washes their beard",
];

const SHOUTS: [&str; 4] = [
    "{a} and {b} had a shouting match in the dining hall.",
    "{a} and {b} were heard shouting at each other in the corridors.",
    "{a} threw a mug at {b}'s feet and stormed out.",
    "{a} and {b} nearly came to blows over a seat at the table.",
];

fn near(a: Pos, b: Pos, r: i32) -> bool {
    a.z == b.z && (a.x - b.x).abs() <= r && (a.y - b.y).abs() <= r
}

fn first(name: &str) -> &str {
    name.split(' ').next().unwrap_or(name)
}

impl Game {
    /// The brawl `id` is in, if any.
    pub fn in_brawl(&self, id: u32) -> Option<u32> {
        self.society.feuds.brawls.iter().find(|b| b.sides.iter().any(|s| s.contains(&id))).map(|b| b.id)
    }

    /// `a` (who may be out of `self.dwarves` for their turn) goes for `b`.
    pub(super) fn start_brawl(&mut self, a: &Dwarf, b: u32, cause: BrawlCause) -> Option<u32> {
        if self.in_brawl(a.id).is_some() || self.in_brawl(b).is_some() {
            return None;
        }
        let other = self.dwarves.iter().find(|o| o.id == b && o.alive)?;
        let (bname, at) = (other.called(), a.pos);
        let f = &mut self.society.feuds;
        let id = f.next;
        f.next += 1;
        f.brawls.push(Brawl { id, at, sides: [vec![a.id], vec![b]], cause, started: self.tick, blows: 0, out: Vec::new() });
        f.brawl_log.push(self.tick);
        let why = match cause {
            BrawlCause::Drunk => ", roaring drunk",
            BrawlCause::Feud => ", and the feud comes to blows",
            BrawlCause::Grudge => "",
        };
        let text = format!("{} has gone for {} with their fists{}!", a.called(), bname, why);
        self.msg_at(&text, (255, 150, 80), Some(at), logkind::LogKind::Combat);
        self.gossip(text);
        self.report_crime(CrimeKind::Assault, a.id, Some(b), format!("started a brawl with {}", bname));
        self.count("soc.brawl", 1);
        Some(id)
    }

    /// Every ten ticks: the open brawls go on.
    pub(super) fn brawls_tick(&mut self) {
        if self.society.feuds.brawls.is_empty() {
            return;
        }
        let tick = self.tick;
        let brawls = std::mem::take(&mut self.society.feuds.brawls);
        let mut kept = Vec::new();
        for mut br in brawls {
            let over = self.brawl_round(&mut br);
            if tick % 30 == 7 && over.is_none() {
                if let Some(peace) = self.brawl_crowd(&mut br) {
                    self.end_brawl(br, Some(peace));
                    continue;
                }
            }
            match over {
                Some(why) => self.end_brawl(br, why),
                None => kept.push(br),
            }
        }
        kept.extend(std::mem::take(&mut self.society.feuds.brawls));
        self.society.feuds.brawls = kept;
    }

    /// One round of punches. `Some(why)` once it's over.
    fn brawl_round(&mut self, br: &mut Brawl) -> Option<Option<String>> {
        let tick = self.tick;
        if self.raided() {
            return Some(Some("The alarm broke up the brawl.".into()));
        }
        if tick > br.started + BRAWL_TICKS {
            return Some(None);
        }
        // Those beaten or sick of it drop out.
        for s in 0..2 {
            let side = br.sides[s].clone();
            for id in side {
                if br.out.contains(&id) {
                    continue;
                }
                let Some(d) = self.dwarves.iter_mut().find(|d| d.id == id) else {
                    br.out.push(id);
                    continue;
                };
                let (hp, max) = (d.body.hp.iter().sum::<i32>(), d.body.max.iter().sum::<i32>());
                if !d.alive || d.body.unconscious() || d.asleep || hp * 4 < max * 3 || d.stress > 450 {
                    br.out.push(id);
                    if d.alive {
                        d.think("was beaten in a brawl", 10, tick);
                        if matches!(d.task, Task::Wander { .. }) {
                            d.task = Task::Idle;
                        }
                    }
                }
            }
        }
        if br.sides.iter().any(|s| s.iter().all(|x| br.out.contains(x))) {
            return Some(None);
        }
        // Each closes on the nearest foe, or swings.
        for s in 0..2 {
            for k in 0..br.sides[s].len() {
                let id = br.sides[s][k];
                if br.out.contains(&id) {
                    continue;
                }
                let Some(i) = self.dwarves.iter().position(|d| d.id == id) else { continue };
                let pos = self.dwarves[i].pos;
                let Some(j) = br.sides[1 - s].iter().filter(|f| !br.out.contains(f)).filter_map(|f| self.dwarves.iter().position(|d| d.id == *f)).min_by_key(|&j| (dist2(self.dwarves[j].pos, pos), j)) else { continue };
                let (fpos, fid) = (self.dwarves[j].pos, self.dwarves[j].id);
                let mut d = std::mem::take(&mut self.dwarves[i]);
                if adjacent(fpos, pos) || fpos == pos {
                    if d.task != Task::Idle {
                        self.release_task(&mut d);
                    }
                    d.task = Task::Idle;
                    d.idle_cd = 12;
                    d.path.clear();
                    if self.rng.chance(0.4) {
                        let (name, called) = (d.name.clone(), d.called());
                        let part = self.dwarves[j].body.pick_part(&mut self.rng);
                        let f = &mut self.dwarves[j];
                        f.body.wound(part, 1, false);
                        f.adjust_regard(id, -6);
                        f.think(&format!("was punched by {} in a brawl", first(&name)), 5, tick);
                        d.satisfy(crate::dwarf::NeedKind::Fight, 100);
                        br.blows += 1;
                        if br.blows == 1 || self.rng.chance(0.1) {
                            let fname = self.dwarves[j].called();
                            self.msg_at(&format!("{} punches {}!", called, fname), (255, 150, 80), Some(fpos), logkind::LogKind::Combat);
                        }
                    }
                } else if !matches!(d.task, Task::Wander { target } if target == fpos) {
                    self.release_task(&mut d);
                    d.task = Task::Wander { target: fpos };
                }
                self.dwarves[i] = d;
                let _ = fid;
            }
        }
        None
    }

    /// Onlookers take sides, or one of them makes peace (what they said).
    fn brawl_crowd(&mut self, br: &mut Brawl) -> Option<String> {
        let tick = self.tick;
        let at = self.dwarves.iter().find(|d| d.id == br.sides[0][0]).map_or(br.at, |d| d.pos);
        let fighting: Vec<u32> = self.society.feuds.brawls.iter().flat_map(|b| b.sides.iter().flatten().copied()).chain(br.sides.iter().flatten().copied()).collect();
        let crowd: Vec<usize> = (0..self.dwarves.len())
            .filter(|&i| {
                let o = &self.dwarves[i];
                o.alive && !o.asleep && !o.insane && !o.is_child() && o.jailed_until == 0 && o.tantrum == 0 && !(o.soldier && self.raided()) && near(o.pos, at, CROWD) && !fighting.contains(&o.id)
            })
            .collect();
        for i in crowd {
            let o = &self.dwarves[i];
            // How much they care for each side: their best friend in it, kin first.
            let care = |side: &Vec<u32>| {
                side.iter()
                    .filter_map(|m| self.dwarves.iter().find(|x| x.id == *m))
                    .map(|m| o.regard(m.id) + if super::society::close_kin(o, m) { 60 } else { 0 })
                    .max()
                    .unwrap_or(-100)
            };
            let (c0, c1) = (care(&br.sides[0]), care(&br.sides[1]));
            let (best, other) = if c0 >= c1 { (0, c1) } else { (1, c0) };
            let top = c0.max(c1);
            if top >= 60 && top >= other + 20 && o.personality.temper > -10 && br.sides[best].len() < 6 {
                let (oid, called) = (o.id, o.called());
                let friend = br.sides[best].iter().filter_map(|m| self.dwarves.iter().find(|x| x.id == *m)).max_by_key(|m| (o.regard(m.id) + if super::society::close_kin(o, m) { 60 } else { 0 }, m.id)).map(|m| m.called()).unwrap_or_default();
                br.sides[best].push(oid);
                let text = format!("{} waded in on {}'s side!", called, first(&friend));
                self.msg_at(&text, (255, 150, 80), Some(at), logkind::LogKind::Combat);
                self.gossip(text);
                self.count("soc.brawl_joined", 1);
                continue;
            }
            if o.personality.temper < -20 && o.personality.sociability > 20 && self.rng.chance(0.25) {
                let called = o.called();
                let o = &mut self.dwarves[i];
                o.think("broke up a brawl", -10, tick);
                return Some(format!("{} pulled them apart.", called));
            }
        }
        None
    }

    /// The brawl is over: grudges across the sides, and if it was a big
    /// one, two factions.
    fn end_brawl(&mut self, br: Brawl, why: Option<String>) {
        let tick = self.tick;
        for side in &br.sides {
            for &id in side {
                if let Some(d) = self.dwarves.iter_mut().find(|d| d.id == id) {
                    if matches!(d.task, Task::Wander { .. }) {
                        d.task = Task::Idle;
                    }
                }
            }
        }
        for &a in &br.sides[0] {
            for &b in &br.sides[1] {
                for (x, y) in [(a, b), (b, a)] {
                    if let Some(d) = self.dwarves.iter_mut().find(|d| d.id == x) {
                        d.adjust_regard(y, -15);
                    }
                }
            }
        }
        let name = |g: &Game, side: &Vec<u32>| side.first().and_then(|id| g.dwarves.iter().find(|d| d.id == *id)).map_or("someone".to_string(), |d| first(&d.called()).to_string());
        let (n0, n1) = (name(self, &br.sides[0]), name(self, &br.sides[1]));
        let many = br.sides[0].len() + br.sides[1].len();
        let mut text = why.unwrap_or_else(|| format!("The brawl between {} and {} is over.", n0, n1));
        if many >= 4 {
            text.push_str(&format!(" {} dwarves were in it, and {} blows were landed.", many, br.blows));
        }
        self.msg_at(&text, (230, 170, 110), Some(br.at), logkind::LogKind::Combat);
        self.gossip(text);
        if many >= 4 && br.sides.iter().all(|s| !s.is_empty()) {
            let names = [format!("{}'s lot", n0), format!("{}'s lot", n1)];
            let line = format!("The fort is split: {} against {}.", names[0], names[1]);
            self.msg(&line, (255, 170, 90));
            let who: Vec<u32> = br.sides.iter().flatten().copied().collect();
            self.record(super::memory::MemoryKind::Other, &who, Some(br.at), format!("A brawl split the fort into factions: {} and {}.", names[0], names[1]), 6);
            self.gossip(line);
            self.society.feuds.factions.push(Faction { sides: br.sides.clone(), names, since: tick });
            self.count("soc.faction", 1);
        }
        // Blood spilled settles a feud for a while.
        for f in self.society.feuds.ladder.iter_mut() {
            if br.sides.iter().any(|s| s.contains(&f.a)) && br.sides.iter().any(|s| s.contains(&f.b)) {
                f.stage = 0;
                f.last = tick + SEASON;
            }
        }
    }

    /// The side `a` is on against `b`, if they're of opposing factions.
    pub fn factions_apart(&self, a: u32, b: u32) -> bool {
        self.society.feuds.factions.iter().any(|f| (f.sides[0].contains(&a) && f.sides[1].contains(&b)) || (f.sides[1].contains(&a) && f.sides[0].contains(&b)))
    }

    /// Every fifty ticks: feuds that have reached blows meet, and factions
    /// glare at each other across the table.
    pub(super) fn feuds_tick(&mut self) {
        let tick = self.tick;
        if self.raided() {
            return;
        }
        // Stage 4: blows, when they next meet.
        for k in 0..self.society.feuds.ladder.len() {
            let f = self.society.feuds.ladder[k].clone();
            if f.stage < 4 {
                continue;
            }
            let (Some(a), Some(b)) = (self.dwarves.iter().position(|d| d.id == f.a && d.alive && !d.asleep), self.dwarves.iter().position(|d| d.id == f.b && d.alive && !d.asleep)) else { continue };
            if !near(self.dwarves[a].pos, self.dwarves[b].pos, 4) || self.dwarves[a].jailed_until > 0 || self.dwarves[b].jailed_until > 0 {
                continue;
            }
            let d = self.dwarves[a].clone();
            if self.start_brawl(&d, f.b, BrawlCause::Feud).is_some() {
                self.society.feuds.ladder[k].stage = 0;
                self.society.feuds.ladder[k].last = tick + SEASON;
            }
        }
        // At meals.
        if self.society.feuds.factions.is_empty() {
            return;
        }
        let eating: Vec<usize> = (0..self.dwarves.len()).filter(|&i| self.dwarves[i].alive && matches!(self.dwarves[i].task, Task::Eat { .. } | Task::Dine { .. })).collect();
        for &i in &eating {
            for &j in &eating {
                let (a, b) = (self.dwarves[i].id, self.dwarves[j].id);
                if a >= b || !near(self.dwarves[i].pos, self.dwarves[j].pos, 4) || !self.factions_apart(a, b) {
                    continue;
                }
                if self.society.feuds.glared.iter().any(|g| g.0 == a && g.1 == b && tick < g.2 + DAY) {
                    continue;
                }
                self.society.feuds.glared.push((a, b, tick));
                let (an, bn) = (self.dwarves[i].called(), self.dwarves[j].called());
                self.dwarves[i].think(&format!("glared at {} across the table", first(&bn)), 4, tick);
                self.dwarves[j].think(&format!("glared at {} across the table", first(&an)), 4, tick);
                if names::hash(&[a as u64, b as u64, tick / DAY]) % 3 == 0 {
                    self.gossip(format!("{} and {} ate in a cold silence, glaring across the table.", an, bn));
                }
                self.count("soc.glare", 1);
            }
        }
    }

    /// Daily: feuds climb a rung, old factions fade, squadmates on opposing
    /// sides grumble.
    pub(super) fn feuds_daily(&mut self) {
        let tick = self.tick;
        self.society.feuds.factions.retain(|f| tick < f.since + YEAR);
        self.society.feuds.brawl_log.retain(|&t| tick < t + YEAR);
        self.society.feuds.glared.retain(|g| tick < g.2 + DAY);
        let alive: Vec<u32> = self.dwarves.iter().filter(|d| d.alive && !d.insane && !d.is_child()).map(|d| d.id).collect();
        self.society.feuds.ladder.retain(|f| alive.contains(&f.a) && alive.contains(&f.b));
        // New feuds: mutual grudges.
        for i in 0..self.dwarves.len() {
            let d = &self.dwarves[i];
            if !alive.contains(&d.id) {
                continue;
            }
            for &(o, r) in &d.relations {
                if r > -40 || o <= d.id || !alive.contains(&o) || self.society.feuds.ladder.iter().any(|f| (f.a == d.id && f.b == o) || (f.a == o && f.b == d.id)) {
                    continue;
                }
                if self.dwarves.iter().find(|x| x.id == o).map_or(false, |x| x.regard(d.id) <= -40) {
                    let a = d.id;
                    self.society.feuds.ladder.push(Feud { a, b: o, stage: 0, last: tick });
                    break;
                }
            }
        }
        // Each climbs a rung now and then; a grudge that has eased lets it go.
        for k in 0..self.society.feuds.ladder.len() {
            let f = self.society.feuds.ladder[k].clone();
            let (Some(a), Some(b)) = (self.dwarves.iter().position(|d| d.id == f.a), self.dwarves.iter().position(|d| d.id == f.b)) else { continue };
            if self.dwarves[a].regard(f.b) > -30 && self.dwarves[b].regard(f.a) > -30 {
                self.society.feuds.ladder[k].stage = 0;
                continue;
            }
            // Only the hot-headed come to blows; the rest stop at slander.
            let hot = self.dwarves[a].personality.temper > 0 || self.dwarves[b].personality.temper > 0;
            if f.stage >= 4 || (f.stage >= 3 && !hot) || tick < f.last + DAY * 4 || names::hash(&[f.a as u64, f.b as u64, tick / DAY, 0xfe]) % 6 != 0 {
                continue;
            }
            let stage = f.stage + 1;
            self.society.feuds.ladder[k].stage = stage;
            self.society.feuds.ladder[k].last = tick;
            let (an, bn) = (self.dwarves[a].called(), self.dwarves[b].called());
            match stage {
                1 => {
                    self.dwarves[a].think(&format!("gave {} the cold shoulder", first(&bn)), 0, tick);
                    self.gossip(format!("{} and {} no longer speak.", an, bn));
                }
                2 => {
                    self.dwarves[a].think(&format!("shouted at {}", first(&bn)), 6, tick);
                    self.dwarves[b].think(&format!("shouted at {}", first(&an)), 6, tick);
                    let h = names::hash(&[f.a as u64, f.b as u64, 2]) as usize;
                    self.gossip(SHOUTS[h % SHOUTS.len()].replace("{a}", &an).replace("{b}", &bn));
                }
                3 => {
                    // Slander, told to a friend of the teller's.
                    let friend = self.dwarves[a].relations.iter().filter(|r| r.1 >= 30 && r.0 != f.b).max_by_key(|r| (r.1, r.0)).map(|r| r.0);
                    if let Some(j) = friend.and_then(|fr| self.dwarves.iter().position(|d| d.id == fr && d.alive)) {
                        self.dwarves[j].adjust_regard(f.b, -10);
                        let lie = LIES[names::hash(&[f.a as u64, f.b as u64, 3]) as usize % LIES.len()];
                        let jn = self.dwarves[j].called();
                        self.gossip(format!("{} has been telling {} that {} {}.", an, jn, bn, lie));
                    } else {
                        self.gossip(format!("{} mutters about {} to anyone who'll listen.", an, bn));
                    }
                }
                _ => {
                    self.gossip(format!("{} has sworn to knock {}'s teeth in the next time they meet.", an, bn));
                }
            }
        }
        self.count_feuding_squads();
    }

    /// Squadmates on opposing sides of a faction don't want to stand together.
    fn count_feuding_squads(&mut self) {
        let tick = self.tick;
        for s in 0..self.squads.len() {
            let m = self.squads[s].members.clone();
            for &a in &m {
                for &b in &m {
                    if a < b && self.factions_apart(a, b) {
                        for (x, y) in [(a, b), (b, a)] {
                            let yn = self.dwarves.iter().find(|d| d.id == y).map_or(String::new(), |d| d.called());
                            if let Some(d) = self.dwarves.iter_mut().find(|d| d.id == x && d.alive) {
                                d.think(&format!("would rather not stand beside {} in the squad", first(&yn)), 4, tick);
                            }
                        }
                    }
                }
            }
        }
    }

    /// The overseer's round: squadmates who brawled on opposing sides are
    /// moved apart, into another squad (once a pair).
    pub(super) fn ov_feuds(&mut self, o: &mut Overseer) {
        if self.society.feuds.factions.is_empty() || self.raided() {
            return;
        }
        for s in 0..self.squads.len() {
            let m = self.squads[s].members.clone();
            let parted = &self.society.feuds.parted;
            let pair = m.iter().flat_map(|&a| m.iter().map(move |&b| (a, b))).find(|&(a, b)| a < b && self.factions_apart(a, b) && !parted.contains(&(a, b)));
            let Some((a, b)) = pair else { continue };
            self.society.feuds.parted.push((a, b));
            // The weaker of the two goes.
            let xp = |id: u32| self.dwarves.iter().find(|d| d.id == id).map_or(0, |d| d.fight_xp);
            let go = if xp(a) < xp(b) { a } else { b };
            let stay = if go == a { b } else { a };
            let name = |id: u32| self.dwarves.iter().find(|d| d.id == id).map_or(String::new(), |d| d.called());
            let (gn, sn) = (name(go), name(stay));
            let other = (0..self.squads.len()).filter(|&t| t != s).find(|&t| !self.squads[t].members.iter().any(|&x| self.factions_apart(x, go)));
            // (With one squad, they'll have to stand it: a soldier short is worse.)
            let Some(t) = other else { continue };
            self.set_squad_member(t, go, true);
            let text = format!("{} and {} came to blows. I moved {} to {}.", first(&gn), first(&sn), first(&gn), self.squads[t].name);
            Self::note_as(o, self.tick, super::personality::NoteKind::Plain, text);
            self.count("soc.squad_split", 1);
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quiet(seed: u64) -> Game {
        let mut g = Game::new(seed);
        g.creatures.retain(|_, c| c.tame);
        for d in g.dwarves.iter_mut() {
            d.relations.clear();
            d.personality.temper = 0;
            d.personality.sociability = 0;
            d.stress = 0;
            d.spouse = None;
            d.parents = None;
        }
        g
    }

    #[test]
    fn a_brawl_ends_with_someone_beaten_and_assault_on_record() {
        let mut g = quiet(61);
        let p = g.dwarves[0].pos;
        g.dwarves[1].pos = p.offset(1, 0, 0);
        let a = g.dwarves[0].clone();
        let b = g.dwarves[1].id;
        assert!(g.start_brawl(&a, b, BrawlCause::Grudge).is_some());
        assert!(g.start_brawl(&a, b, BrawlCause::Grudge).is_none(), "two brawls at once");
        assert!(g.crimes.iter().any(|c| c.culprit == a.id && c.victim == Some(b)));
        for _ in 0..40 {
            g.tick += 10;
            g.brawls_tick();
        }
        assert!(g.society.feuds.brawls.is_empty(), "never ended");
        assert!(g.dwarves.iter().take(2).all(|d| d.alive), "a brawl killed");
        assert!(g.dwarves[0].regard(b) < 0 && g.dwarves[1].regard(a.id) < 0);
        assert!(g.log.iter().any(|l| l.text.contains("punches")));
    }

    /// The "done when": A and B brawl; C (a friend of A) and D (kin of B)
    /// join on their sides, E (who knows none of them) watches; afterwards
    /// C and D hold it against each other, and the fort has two factions.
    #[test]
    fn friends_join_a_brawl_on_their_side() {
        let mut g = quiet(62);
        assert!(g.dwarves.len() >= 5);
        let ids: Vec<u32> = g.dwarves.iter().take(5).map(|d| d.id).collect();
        let (a, b, c, d, e) = (ids[0], ids[1], ids[2], ids[3], ids[4]);
        let p = g.dwarves[0].pos;
        for (i, dx) in [(0, 0), (1, 1), (2, -1), (3, 2), (4, 0)] {
            g.dwarves[i].pos = p.offset(dx, if i == 4 { 2 } else { 0 }, 0);
        }
        g.dwarves[2].adjust_regard(a, 60);
        g.dwarves[3].parents = Some((b, 9999));
        let ad = g.dwarves[0].clone();
        g.start_brawl(&ad, b, BrawlCause::Grudge).unwrap();
        g.tick = 37;
        g.brawls_tick();
        let br = &g.society.feuds.brawls[0];
        assert!(br.sides[0].contains(&c), "the friend didn't join: {:?}", br.sides);
        assert!(br.sides[1].contains(&d), "the kin didn't join: {:?}", br.sides);
        assert!(!br.sides.iter().any(|s| s.contains(&e)), "the stranger joined");
        assert!(g.log.iter().any(|l| l.text.contains("waded in on")));
        // Over at last.
        for _ in 0..40 {
            g.tick += 10;
            g.brawls_tick();
        }
        assert!(g.society.feuds.brawls.is_empty());
        assert!(g.dwarves[2].regard(d) < 0 && g.dwarves[3].regard(c) < 0, "no grudge across the sides");
        assert!(g.factions_apart(c, d));
        assert!(g.log.iter().any(|l| l.text.starts_with("The fort is split")));
    }

    #[test]
    fn a_feud_climbs_stage_by_stage_into_blows() {
        let mut g = quiet(63);
        let (a, b) = (g.dwarves[0].id, g.dwarves[1].id);
        g.dwarves[0].adjust_regard(b, -70);
        g.dwarves[1].adjust_regard(a, -70);
        g.dwarves[0].personality.temper = 30;
        let c = g.dwarves[2].id;
        g.dwarves[0].adjust_regard(c, 50);
        for _ in 0..200 {
            g.tick += DAY;
            g.feuds_daily();
            if g.society.feuds.ladder.first().map_or(false, |f| f.stage >= 4) {
                break;
            }
        }
        assert_eq!(g.society.feuds.ladder[0].stage, 4);
        assert!(g.gossip.iter().any(|(_, t)| t.contains("no longer speak")));
        assert!(g.gossip.iter().any(|(_, t)| t.contains("has been telling")), "no slander");
        // They meet.
        let p = g.dwarves[0].pos;
        g.dwarves[1].pos = p.offset(1, 0, 0);
        g.feuds_tick();
        assert!(g.in_brawl(a).is_some() && g.in_brawl(b).is_some());
    }

    #[test]
    fn the_overseer_parts_feuding_squadmates() {
        let mut g = quiet(64);
        g.set_overseer(Some(Style::Steady));
        let (a, b) = (g.dwarves[0].id, g.dwarves[1].id);
        g.new_squad();
        g.new_squad();
        g.set_squad_member(0, a, true);
        g.set_squad_member(0, b, true);
        g.society.feuds.factions.push(Faction { sides: [vec![a], vec![b]], names: ["a".into(), "b".into()], since: g.tick });
        let mut o = g.overseer.take().unwrap();
        g.ov_feuds(&mut o);
        assert!(o.notes.iter().any(|n| n.1.contains("came to blows")), "{:?}", o.notes);
        g.overseer = Some(o);
        assert!(!(g.squads[0].members.contains(&a) && g.squads[0].members.contains(&b)));
    }
}
