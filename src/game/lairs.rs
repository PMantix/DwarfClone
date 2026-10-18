//! Lairs and living populations (CRE-23, CRE-29): the wild come out of
//! dens, lairs and grazing grounds about the map's edge, and go back to
//! them. Each holds so many beasts; those the fort kills are gone, a lair
//! that has lost them all falls silent for a long while, and the rest grow
//! back a little each season. A beast followed home shows the fort where
//! its den is, and the overseer can send a squad to clear it out.

use super::combat::near;
use super::*;
use crate::creature::{Creature, Species};

/// A den, lair or grazing ground: where some of the land's wild live.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lair {
    pub pos: Pos,
    pub species: Species,
    /// How many are at home now.
    pub pop: u32,
    /// How many it holds when the land is left alone.
    pub cap: u32,
    /// Known to the fort: come upon, or a beast was followed home to it.
    pub found: bool,
    /// "the north den".
    #[serde(default)]
    pub name: String,
    /// Packs (or herds) it has sent out.
    #[serde(default)]
    pub sent: u32,
    /// Its beasts that have died.
    #[serde(default)]
    pub kills: u32,
    /// Hunted out: nothing grows back before this tick (0: it never was).
    #[serde(default)]
    pub quiet_until: u64,
}

impl Lair {
    /// Its people: "wolves".
    pub fn plural(&self) -> &'static str {
        self.species.info().plural
    }

    /// Hunted out and not yet grown back.
    pub fn silent(&self, tick: u64) -> bool {
        self.quiet_until > tick
    }
}

impl Game {
    /// At embark: a grazing ground, one or two dens of the land's pack
    /// hunters and a lair of its rare beast, out towards the map's edge,
    /// in woods or on high ground where there is some. Its own generator,
    /// so the rest of the embark is as it was.
    pub(super) fn make_lairs(&mut self, seed: u64) {
        let mut rng = Rng::new(seed ^ 0x1A15_D3E5);
        let (calm, pack, rare) = self.fauna();
        let mut want = vec![(calm, rng.range(8, 13) as u32, "grazing grounds")];
        for _ in 0..rng.range(1, 3) {
            want.push((pack, rng.range(5, 9) as u32, "den"));
        }
        want.push((rare, rng.range(1, 3) as u32, if rare == pack { "den" } else { "lair" }));
        for (species, cap, kind) in want {
            let Some(pos) = self.lair_spot(&mut rng) else { continue };
            let mut name = format!("the {} {}", self.compass(pos), kind);
            if self.lairs.iter().any(|l| l.name == name) {
                name = format!("the far {} {}", self.compass(pos), kind);
            }
            self.lairs.push(Lair { pos, species, pop: cap, cap, found: false, name, sent: 0, kills: 0, quiet_until: 0 });
        }
    }

    /// Open ground under the sky near the map's edge, well away from the
    /// wagon and from the other lairs: the most wooded, highest such spot of
    /// a few hundred tried.
    fn lair_spot(&self, rng: &mut Rng) -> Option<Pos> {
        let (w, h) = (self.world.w, self.world.h);
        let mut best: Option<(i32, Pos)> = None;
        for _ in 0..300 {
            let depth = rng.range(2, 10);
            let (x, y) = match rng.range(0, 4) {
                0 => (depth, rng.range(2, h - 2)),
                1 => (w - 1 - depth, rng.range(2, h - 2)),
                2 => (rng.range(2, w - 2), depth),
                _ => (rng.range(2, w - 2), h - 1 - depth),
            };
            let p = Pos::new(x, y, self.world.surface_z(x, y));
            if !self.world.in_bounds(p) || !self.world.walkable(p) || !self.world.outside(p) || self.world.t(p).water > 0 {
                continue;
            }
            let apart = |q: Pos, r: i32| (q.x - p.x).abs() > r || (q.y - p.y).abs() > r;
            if !apart(self.embark, 35) || !self.lairs.iter().all(|l| apart(l.pos, 25)) {
                continue;
            }
            let mut trees = 0;
            for dx in -3..=3 {
                for dy in -3..=3 {
                    for dz in 0..=1 {
                        let q = p.offset(dx, dy, dz);
                        if self.world.in_bounds(q) && self.world.t(q).kind == TileKind::Tree {
                            trees += 1;
                        }
                    }
                }
            }
            let score = trees + 2 * (p.z - self.embark.z).max(0);
            if best.map_or(true, |(b, _)| score > b) {
                best = Some((score, p));
            }
        }
        best.map(|b| b.1)
    }

    /// Which way `p` lies from the middle of the map: "north", "south-east".
    pub fn compass(&self, p: Pos) -> &'static str {
        let (dx, dy) = (p.x - self.world.w / 2, p.y - self.world.h / 2);
        let ns = if dy < 0 { "north" } else { "south" };
        let ew = if dx < 0 { "west" } else { "east" };
        if dx.abs() > 2 * dy.abs() {
            ew
        } else if dy.abs() > 2 * dx.abs() {
            ns
        } else {
            match (dy < 0, dx < 0) {
                (true, true) => "north-west",
                (true, false) => "north-east",
                (false, true) => "south-west",
                (false, false) => "south-east",
            }
        }
    }

    /// Where a party of `n` of `species` comes from: `Some(None)` from the
    /// map's edge (it has no lair here), `Some(Some((lair, how many)))` out
    /// of the fullest lair with any at home, and `None` when its lairs are
    /// all empty or silent: nothing comes.
    pub(super) fn lair_sends(&mut self, species: Species, n: u32) -> Option<Option<(usize, u32)>> {
        let tick = self.tick;
        let mine: Vec<usize> = (0..self.lairs.len()).filter(|&i| self.lairs[i].species == species).collect();
        if mine.is_empty() {
            return Some(None);
        }
        let i = mine.into_iter().filter(|&i| self.lairs[i].pop > 0 && !self.lairs[i].silent(tick)).max_by_key(|&i| (self.lairs[i].pop, std::cmp::Reverse(i)))?;
        let l = &mut self.lairs[i];
        let n = n.min(l.pop);
        l.pop -= n;
        l.sent += 1;
        Some(Some((i, n)))
    }

    /// The lair a creature came out of.
    pub fn lair_of(&self, c: &Creature) -> Option<&Lair> {
        c.lair.and_then(|i| self.lairs.get(i as usize))
    }

    /// A leaving beast with a lair walks home to it rather than to the
    /// map's edge (from `creature_tick`). `None`: it has no lair, or no way
    /// there, and makes for the edge as ever; `Some(false)`: it is home, and
    /// gone from the map; `Some(true)`: still on its way.
    pub(super) fn go_home(&mut self, c: &mut Creature) -> Option<bool> {
        let i = c.lair? as usize;
        let at = self.lairs.get(i)?.pos;
        if near(c.pos, at, 1) && c.pos.z == at.z {
            self.home_again(c, i);
            return Some(false);
        }
        if c.path.is_empty() && c.think_cd == 0 {
            c.think_cd = 50;
            match path::search(&self.world, c.pos, SEARCH_LIMIT, c.mover(), |q| near(q, at, 1) && q.z == at.z) {
                Some((_, path)) => c.path = path.into_iter().rev().collect(),
                None => {
                    c.lair = None;
                    return None;
                }
            }
        }
        c.think_cd = c.think_cd.saturating_sub(1);
        let delay = c.species.info().move_delay;
        self.creature_step(c, delay);
        Some(true)
    }

    /// A beast is back in its lair: one more at home. If a dwarf saw it go
    /// in (within 20), or it was being hunted, the fort now knows the place.
    fn home_again(&mut self, c: &Creature, i: usize) {
        let l = &mut self.lairs[i];
        l.pop = (l.pop + 1).min(l.cap);
        if self.lairs[i].found {
            return;
        }
        let hunted = self.hunts.on.as_ref().map_or(false, |h| h.quarry.iter().any(|q| q.0 == c.id));
        let seen = self.dwarves.iter().any(|d| d.alive && near(d.pos, c.pos, 20));
        if hunted || seen {
            self.lairs[i].found = true;
            self.count("lairs_found", 1);
            let l = &self.lairs[i];
            let text = format!("Tracked the {} to their home: {}.", l.plural(), l.name);
            self.msg(&text, (240, 200, 120));
        }
    }

    /// Daily: a grown dwarf under the sky within 10 of a lair nobody knows
    /// comes upon it.
    pub(super) fn come_upon_lairs(&mut self) {
        for i in 0..self.lairs.len() {
            if self.lairs[i].found {
                continue;
            }
            let at = self.lairs[i].pos;
            let Some(d) = self.dwarves.iter().find(|d| d.alive && !d.is_child() && self.world.outside(d.pos) && near(d.pos, at, 10) && (d.pos.z - at.z).abs() <= 3) else { continue };
            let first = d.name.split(' ').next().unwrap_or("someone").to_string();
            self.lairs[i].found = true;
            self.count("lairs_found", 1);
            let l = &self.lairs[i];
            let text = format!("{} has come upon {}, where {} live.", first, l.name, l.plural());
            self.msg(&text, (240, 200, 120));
        }
    }

    /// A wild beast has died (`creature_died`): its pack may break, and its
    /// lair has one fewer. When the last of a lair is gone, the woods about
    /// it fall silent for a long while (knob wild.quiet, in years).
    pub(super) fn wild_death(&mut self, c: &Creature) {
        self.pack_breaks(c);
        let Some(i) = c.lair.map(|i| i as usize).filter(|&i| i < self.lairs.len()) else { return };
        self.lairs[i].kills += 1;
        if near(c.pos, self.lairs[i].pos, 8) {
            self.count("lair_kills", 1);
        }
        let out = self.creatures.values().any(|o| o.alive && o.id != c.id && o.lair == Some(i as u32));
        if self.lairs[i].pop == 0 && !out {
            self.hunted_out(i);
        }
    }

    /// The last of a lair is dead: the forest goes quiet there.
    fn hunted_out(&mut self, i: usize) {
        let years = self.knob("wild.quiet", 1.0);
        let fort = self.fortress_name.clone();
        let l = &mut self.lairs[i];
        l.quiet_until = self.tick + (years * YEAR as f64) as u64;
        l.found = true;
        let (plural, name, pos) = (l.plural(), l.name.clone(), l.pos);
        let wher = self.compass(pos);
        self.count("lairs_hunted_out", 1);
        self.msg(&format!("The woods to the {} have gone quiet: the {} of {} are no more.", wher, plural, name), (200, 200, 150));
        self.legend(format!("The {} of {} were hunted out by the folk of {}.", plural, name, fort));
        self.spotlight("The woods fall silent", &format!("The last of the {} of {} is dead. Nothing stirs there now.", plural, name), Some(pos), (200, 200, 150));
    }

    /// The turn of a season: each lair not hunted out grows by one (knob
    /// wild.regrow) towards what it holds. A known den with young in spring
    /// is news; a silent one coming back to life is too.
    pub(super) fn lairs_season(&mut self, season: u64) {
        let tick = self.tick;
        let grow = self.knob("wild.regrow", 1.0) as u32;
        for i in 0..self.lairs.len() {
            let l = &mut self.lairs[i];
            if l.silent(tick) || l.pop >= l.cap || grow == 0 {
                continue;
            }
            let was_quiet = l.quiet_until > 0;
            l.quiet_until = 0;
            l.pop = (l.pop + grow).min(l.cap);
            let (found, species, name) = (l.found, l.species, l.name.clone());
            if was_quiet {
                self.msg(&format!("Life returns to {}: the {} are back.", name, species.info().plural), (160, 230, 160));
            } else if found && season == 0 && species.info().aggro > 0 {
                let young = match species.young() {
                    "young" => format!("Young {}", species.info().plural),
                    y => cap_first(&format!("{} {}s", species.info().name, y)),
                };
                self.msg(&format!("{} have been seen about {}.", young, name), (240, 200, 120));
            }
        }
    }

    /// No prey left in the land: every grazing ground is empty or silent,
    /// and hungry hunters take more chances (`predator_think`).
    pub(super) fn prey_scarce(&self) -> bool {
        let tick = self.tick;
        let mut prey = self.lairs.iter().filter(|l| l.species.info().aggro == 0).peekable();
        prey.peek().is_some() && prey.all(|l| l.pop == 0 || l.silent(tick))
    }

    /// A known lair at `p`, for the map: its glyph and colour.
    pub fn lair_mark(&self, p: Pos) -> Option<(char, (u8, u8, u8))> {
        let l = self.lairs.iter().find(|l| l.found && l.pos == p)?;
        Some(('Ω', if l.silent(self.tick) { (90, 90, 80) } else { (170, 130, 90) }))
    }

    /// What the look panel says of a known lair at `p`.
    pub fn lair_note(&self, p: Pos) -> Option<String> {
        let l = self.lairs.iter().find(|l| l.found && l.pos == p)?;
        let what = cap_first(&l.name);
        Some(if l.silent(self.tick) {
            format!("{}: hunted out, silent", what)
        } else if l.pop == 0 {
            format!("{}: the {} are out", what, l.plural())
        } else {
            format!("{}: {} {} at home", what, l.pop, if l.pop == 1 { l.species.info().name } else { l.plural() })
        })
    }
}

fn cap_first(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fort with no beasts about, and the index of a lair of the land's
    /// pack hunters.
    fn wild(seed: u64) -> (Game, usize) {
        let mut g = Game::new(seed);
        g.creatures.retain(|_, c| c.tame);
        let (_, pack, _) = g.fauna();
        let i = g.lairs.iter().position(|l| l.species == pack).expect("no den");
        (g, i)
    }

    #[test]
    fn packs_come_from_and_return_to_their_lair() {
        let (mut g, _) = wild(2);
        let (calm, pack, rare) = g.fauna();
        let (w, h) = (g.world.w, g.world.h);
        assert!(g.lairs.len() >= 3);
        for l in &g.lairs {
            assert!(l.pos.x < 12 || l.pos.y < 12 || l.pos.x >= w - 12 || l.pos.y >= h - 12, "{} is not out by the edge", l.name);
            assert!(l.pop == l.cap && l.cap > 0 && !l.found);
            assert!([calm, pack, rare].contains(&l.species));
        }
        assert!(g.lairs.iter().any(|l| l.species == calm && l.name.ends_with("grazing grounds")));
        // A year in, the hunters come calling: out of their den.
        g.tick = YEAR + DAY / 2;
        g.savagery = 0.9;
        g.knobs.set.insert("wildlife.chance".into(), 1.0);
        let full: u32 = g.lairs.iter().filter(|l| l.species == pack).map(|l| l.pop).sum();
        let mut tries = 0;
        while !g.creatures.values().any(|c| c.species == pack) && tries < 40 {
            g.creatures.retain(|_, c| c.tame);
            for l in g.lairs.iter_mut() {
                l.pop = l.cap;
            }
            g.wildlife();
            tries += 1;
        }
        let ids: Vec<u32> = g.creatures.values().filter(|c| c.species == pack).map(|c| c.id).collect();
        assert!(!ids.is_empty(), "no pack in 40 days");
        let j = g.creatures[&ids[0]].lair.expect("the pack has no lair") as usize;
        assert_eq!(g.lairs[j].species, pack);
        for id in &ids {
            let c = &g.creatures[id];
            assert_eq!(c.lair, Some(j as u32));
            assert!(near(c.pos, g.lairs[j].pos, 3), "it came from somewhere else: {:?}", c.pos);
        }
        let home: u32 = g.lairs.iter().filter(|l| l.species == pack).map(|l| l.pop).sum();
        assert_eq!(home + ids.len() as u32, full, "those out are not at home");
        // They roam a while, far from anyone, and then make for home.
        let d = &mut g.dwarves[0];
        d.pos = g.embark;
        g.dwarves.truncate(1);
        for _ in 0..400 {
            g.update_creatures();
            g.tick += 1;
        }
        // A dwarf out by the den sees them go in.
        let at = g.lairs[j].pos;
        let spot = (-14..=14)
            .flat_map(|dx| (-14..=14).map(move |dy| (dx, dy)))
            .map(|(dx, dy)| Pos::new(at.x + dx, at.y + dy, 0))
            .filter(|p| g.world.in_bounds(*p))
            .map(|p| Pos::new(p.x, p.y, g.world.surface_z(p.x, p.y)))
            .find(|&p| (p.x - at.x).abs().max((p.y - at.y).abs()) >= 11 && g.world.walkable(p))
            .expect("no ground by the den");
        g.dwarves[0].pos = spot;
        for id in &ids {
            if let Some(c) = g.creatures.get_mut(id) {
                c.leaving = true;
                c.target = None;
                c.stalk = None;
                c.path.clear();
            }
        }
        let mut t = 0;
        while ids.iter().any(|id| g.creatures.contains_key(id)) && t < 3000 {
            g.update_creatures();
            g.tick += 1;
            t += 1;
        }
        assert!(ids.iter().all(|id| !g.creatures.contains_key(id)), "they never got home");
        let home: u32 = g.lairs.iter().filter(|l| l.species == pack).map(|l| l.pop).sum();
        assert_eq!(home, full, "home again, all of them");
        assert!(g.lairs[j].found, "followed home, the den is known");
        assert!(g.log.iter().any(|l| l.text.starts_with("Tracked the") && l.text.contains(&g.lairs[j].name)));
        // Now it shows on the map and in the look panel.
        assert!(g.lair_mark(at).is_some());
        assert!(g.lair_note(at).unwrap().contains("at home"));
    }

    #[test]
    fn a_hunted_out_lair_sends_nothing() {
        let (mut g, i) = wild(2);
        let pack = g.lairs[i].species;
        for l in g.lairs.iter_mut().filter(|l| l.species == pack) {
            l.pop = 0;
        }
        assert_eq!(g.lair_sends(pack, 3), None, "an empty den sends nothing");
        // A beast with no lair here still comes from the map's edge.
        assert_eq!(g.lair_sends(Species::Husk, 3), Some(None));
        // The last of the den dies: the woods there go quiet.
        let at = g.lairs[i].pos;
        let last = g.spawn_creature(pack, at, u64::MAX);
        let other = g.spawn_creature(pack, at, u64::MAX);
        for id in [last, other] {
            g.creatures.get_mut(&id).unwrap().lair = Some(i as u32);
        }
        let c = g.creatures.remove(&other).unwrap();
        g.creature_died(c);
        assert_eq!(g.lairs[i].quiet_until, 0, "one of them is still out");
        let c = g.creatures.remove(&last).unwrap();
        g.creature_died(c);
        assert!(g.lairs[i].silent(g.tick));
        assert!(g.lairs[i].found);
        assert!(g.log.iter().any(|l| l.text.starts_with("The woods to the ") && l.text.contains("have gone quiet")));
        assert!(g.overworld.events.iter().any(|e| e.text.contains("were hunted out")), "no chronicle line");
        assert!(g.spotlights.iter().any(|s| s.title == "The woods fall silent"));
        assert_eq!(g.annals.counters.get("lair_kills"), Some(&2));
        assert_eq!(g.annals.counters.get("lairs_hunted_out"), Some(&1));
        assert!(g.lair_note(at).unwrap().contains("hunted out"));
        // Seasons pass, and nothing grows back for a long while.
        g.knobs.set.insert("wildlife.chance".into(), 1.0);
        g.lairs_season(1);
        assert_eq!(g.lairs[i].pop, 0);
        // A year on, life returns, one at a time.
        g.tick = g.lairs[i].quiet_until;
        g.lairs_season(2);
        assert_eq!(g.lairs[i].pop, 1);
        assert!(g.log.iter().any(|l| l.text.starts_with("Life returns to")));
        assert!(g.lair_sends(pack, 3).is_some());
    }

    /// The grazing grounds emptied, the hunters grow hungry and bold.
    #[test]
    fn with_the_prey_gone_the_hunters_grow_bold() {
        let (mut g, _) = wild(2);
        assert!(!g.prey_scarce());
        for l in g.lairs.iter_mut().filter(|l| l.species.info().aggro == 0) {
            l.pop = 0;
        }
        assert!(g.prey_scarce());
    }

    /// A dwarf out by a lair nobody knows comes upon it; a spring later its
    /// young are about.
    #[test]
    fn a_dwarf_comes_upon_a_den() {
        let (mut g, i) = wild(2);
        let at = g.lairs[i].pos;
        g.dwarves[0].pos = at;
        g.come_upon_lairs();
        assert!(g.lairs[i].found);
        assert!(g.log.iter().any(|l| l.text.contains(&format!("has come upon {}", g.lairs[i].name))));
        assert_eq!(g.annals.counters.get("lairs_found"), Some(&1));
        g.lairs[i].pop = 0;
        g.lairs_season(0);
        assert_eq!(g.lairs[i].pop, 1);
        assert!(g.log.iter().any(|l| l.text.contains("have been seen about")), "no young");
    }
}
