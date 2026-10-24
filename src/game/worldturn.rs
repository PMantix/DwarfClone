//! The world turns (M4 B17) and armies march on it (M4 B01).
//!
//! Once a year, in spring, the world's history rolls on a year past the
//! embark: towns grow and are founded, wars are declared and ended, sieges
//! raze and conquer, beasts lay places waste, rulers die. The fort hears the
//! news that matters to it ("News from the world"), its neighbours are worked
//! out again (a razed foe sends no more armies, a new war brings new ones),
//! and the changed world goes into the shared world file with the spring
//! autosave, so a hero in the same world later finds the ruin.
//!
//! An army that marches on the fort is a `Party` on the world map: it leaves
//! a real town of its people, walks toward the fort at `CELLS_PER_DAY`, and
//! the scouts sight it days out. Each morning brings word of how near it is,
//! and the atlas (`W`) shows it coming.
//!
//! The world's rolls never touch the fort's own `rng`: each year's pass is
//! seeded from the world, the embark and the year, so two runs of one fort
//! turn the same world the same way.

use super::*;
use crate::overworld::Race;

/// How far an army walks in a day, in world cells.
pub const CELLS_PER_DAY: i32 = 3;
/// The longest march (in days) an army is seen on: from further off it is
/// still days away when it is seen, and no more.
pub const MAX_MARCH_DAYS: u64 = 3;
/// How many lines of news the spring card holds.
const NEWS_CARD: usize = 4;
/// At most so many years of history roll at once (an old save catching up).
const CATCH_UP: i32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartyKind {
    /// An army marching on the fort.
    Army,
}

/// A band travelling the world map.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Party {
    pub kind: PartyKind,
    pub civ: u32,
    /// The site it set out from, if it came from one.
    #[serde(default)]
    pub from: Option<u32>,
    /// World cells: where it set out and where it is going.
    pub start: (i32, i32),
    pub dest: (i32, i32),
    pub size: u32,
    pub leader: String,
    /// Game ticks: when it set out and when it arrives.
    pub depart: u64,
    pub eta: u64,
    /// The days-out last told to the fort (so each morning says it once).
    #[serde(default)]
    pub told: u64,
}

impl Party {
    /// Where it is on the world map at `tick`: on the line from start to
    /// destination, as far along as its time.
    pub fn at(&self, tick: u64) -> (i32, i32) {
        let span = self.eta.saturating_sub(self.depart).max(1);
        let done = tick.saturating_sub(self.depart).min(span) as f64 / span as f64;
        let lerp = |a: i32, b: i32| a + ((b - a) as f64 * done).round() as i32;
        (lerp(self.start.0, self.dest.0), lerp(self.start.1, self.dest.1))
    }

    /// Whole days until it arrives (rounded up), 0 once it is here.
    pub fn days_out(&self, tick: u64) -> u64 {
        self.eta.saturating_sub(tick).div_ceil(DAY)
    }
}

/// What the world's turning has left with the fort.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorldTurn {
    /// Bands on the world map.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parties: Vec<Party>,
    /// The news heard each spring: the year it happened and the line.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub news: Vec<(i32, String)>,
    /// How often the world changed since the embark (a site fell, rose or
    /// changed hands, a war or a peace, a people gone).
    #[serde(default)]
    pub changes: u32,
}

/// How much a line of history matters to a fort: its own people's doings
/// most, then its neighbours', then the great events anywhere.
fn weight(text: &str, home: &str, near: &[String]) -> i32 {
    let mut w = 0;
    if !home.is_empty() && text.contains(home) {
        w += 6;
    }
    if near.iter().any(|n| !n.is_empty() && text.contains(n.as_str())) {
        w += 3;
    }
    for (word, n) in [("razed", 3), ("conquered", 3), ("laid waste", 3), ("vanished", 4), ("declared war", 3), ("made peace", 2), ("fell defending", 2), ("slew", 2), ("founded", 1), ("died", 1), ("became", 1)] {
        if text.contains(word) {
            w += n;
        }
    }
    w
}

impl Game {
    /// Spring: the world's history rolls on through the year just gone, and
    /// the fort hears what changed.
    pub(super) fn world_yearly(&mut self) {
        let last = self.start_year + (self.tick / YEAR) as i32 - 1;
        if self.overworld.w <= 1 || self.overworld.civs.is_empty() || self.overworld.year >= last {
            return;
        }
        let first = (self.overworld.year + 1).max(last - CATCH_UP + 1);
        let n0 = self.overworld.events.len();
        let before: Vec<(bool, Option<u32>)> = self.overworld.sites.iter().map(|s| (s.ruined, s.civ)).collect();
        let wars: Vec<Vec<u32>> = self.overworld.civs.iter().map(|c| c.enemies.clone()).collect();
        let alive: Vec<bool> = self.overworld.civs.iter().map(|c| c.alive).collect();
        for y in first..=last {
            let seed = names::hash(&[self.overworld.seed, self.site.0 as u64, self.site.1 as u64, y as u64]);
            let mut rng = crate::rng::Rng::new(seed);
            self.overworld.simulate_year(&mut rng, y);
        }
        self.overworld.year = last;
        // What changed: places fallen, risen or taken; wars and peaces; peoples gone.
        let ow = &self.overworld;
        let mut changes = ow.sites.len().saturating_sub(before.len()) as u32;
        changes += before.iter().zip(&ow.sites).filter(|((r, c), s)| *r != s.ruined || *c != s.civ).count() as u32;
        changes += wars.iter().zip(&ow.civs).map(|(w, c)| (c.enemies.iter().filter(|e| !w.contains(e)).count() + w.iter().filter(|e| !c.enemies.contains(e)).count()) as u32).sum::<u32>() / 2;
        changes += alive.iter().zip(&ow.civs).filter(|(a, c)| **a != c.alive).count() as u32;
        self.worldturn.changes += changes;
        self.count("world.changes", changes);
        // Old foes may be gone and new ones near.
        self.neighbors = self.overworld.neighbors(self.site.0, self.site.1, 30);
        self.hear_the_year(n0);
    }

    /// The news of the year's new history the fort cares about, on a card.
    fn hear_the_year(&mut self, n0: usize) {
        let ow = &self.overworld;
        let home = self.civ.map(|c| ow.civ(c).name.clone()).unwrap_or_default();
        let mut near: Vec<String> = self.neighbors.iter().map(|n| ow.civ(n.civ).name.clone()).collect();
        let (sx, sy) = self.site;
        near.extend(ow.sites.iter().filter(|s| (s.x - sx).abs().max((s.y - sy).abs()) <= 20).map(|s| s.name.clone()));
        let mut picked: Vec<(i32, usize)> = ow.events[n0..].iter().enumerate().map(|(i, e)| (weight(&e.text, &home, &near), n0 + i)).filter(|(w, _)| *w >= 3).collect();
        // The weightiest first; among equals, the first told.
        picked.sort_by_key(|&(w, i)| (-w, i));
        picked.truncate(NEWS_CARD);
        picked.sort_by_key(|&(_, i)| i);
        if picked.is_empty() {
            return;
        }
        let lines: Vec<(i32, String)> = picked.iter().map(|&(_, i)| (ow.events[i].year, ow.events[i].text.clone())).collect();
        self.count("world.news", lines.len() as u32);
        for (_, l) in &lines {
            self.msg(&format!("News from the world: {}", l), (190, 200, 255));
        }
        let text = lines.iter().map(|(_, l)| l.as_str()).collect::<Vec<_>>().join(" ");
        self.spotlight("News from the world", &text, None, (190, 200, 255));
        self.worldturn.news.extend(lines);
        let cut = self.worldturn.news.len().saturating_sub(40);
        self.worldturn.news.drain(..cut);
    }

    /// The army `foe` sends from its nearest town, and when it arrives: as
    /// soon as it would have (`arrive`), plus the days its march takes
    /// beyond the first. (B01: armies come from somewhere.)
    pub(super) fn march_from(&self, foe: &Neighbor, arrive: u64) -> (u64, Option<u32>) {
        let (sx, sy) = self.site;
        let from = self
            .overworld
            .sites
            .iter()
            .filter(|s| s.civ == Some(foe.civ) && !s.ruined)
            .min_by_key(|s| ((s.x - sx).abs().max((s.y - sy).abs()), s.id))
            .map(|s| s.id);
        let Some(f) = from else { return (arrive, None) };
        let s = &self.overworld.sites[f as usize];
        let dist = (s.x - sx).abs().max((s.y - sy).abs());
        let days = ((dist + CELLS_PER_DAY - 1) / CELLS_PER_DAY).clamp(1, MAX_MARCH_DAYS as i32) as u64;
        (arrive + (days - 1) * DAY, Some(f))
    }

    /// Put the army on the world map, marching from `from` to arrive at
    /// `eta`, and tell the overseer.
    pub(super) fn dispatch_army(&mut self, foe: &Neighbor, from: Option<u32>, size: u32, leader: &str, eta: u64) {
        let dest = self.site;
        let start = from.map_or(dest, |f| (self.overworld.sites[f as usize].x, self.overworld.sites[f as usize].y));
        let p = Party { kind: PartyKind::Army, civ: foe.civ, from, start, dest, size, leader: leader.to_string(), depart: self.tick, eta, told: 0 };
        let days = p.days_out(self.tick);
        self.worldturn.parties.retain(|q| q.kind != PartyKind::Army);
        self.worldturn.parties.push(p);
        if days >= 2 {
            self.count("world.warned_days", days as u32);
            let civ = self.overworld.civ(foe.civ).name.clone();
            let soldiers: usize = self.squads.iter().map(|s| s.members.len()).sum();
            let tick = self.tick;
            if let Some(o) = self.overseer.as_mut() {
                let how = if size as usize > soldiers { "called the squads in to make ready: they are outnumbered" } else { "set the squads to make ready" };
                Game::note_ov(o, tick, format!("Saw the army of the {} on the march, {} days out, and {}.", civ, days, how));
            }
        }
    }

    /// The scouts' words for a march: where it set out and how far off it is.
    pub(super) fn march_words(&self, from: Option<u32>, eta: u64) -> Option<String> {
        let days = eta.saturating_sub(self.tick) / DAY;
        let f = from?;
        (days >= 2).then(|| format!("They set out from {} and are {} days' march away.", self.overworld.sites[f as usize].name, days))
    }

    /// How much sooner than the usual the alert goes up for the army on the
    /// march: half a day when it outnumbers the squads (B01's ready phase).
    pub(super) fn march_lead(&self) -> u64 {
        let soldiers: usize = self.squads.iter().map(|s| s.members.len()).sum();
        let due = self.incoming.as_ref().map(|i| i.0);
        let big = self.worldturn.parties.iter().any(|p| p.kind == PartyKind::Army && Some(p.eta) == due && p.size as usize > soldiers);
        let bold = self.overseer.as_ref().map_or(false, |o| o.style == Style::Reckless);
        if big && !bold {
            DAY / 2
        } else {
            0
        }
    }

    /// Each morning: armies come nearer, and the fort hears how near.
    pub(super) fn world_daily(&mut self) {
        let due = self.incoming.as_ref().map(|i| i.0);
        let tick = self.tick;
        // An army that has arrived, or was turned back, leaves the map.
        self.worldturn.parties.retain(|p| p.kind != PartyKind::Army || (Some(p.eta) == due && p.eta > tick));
        let mut lines = Vec::new();
        for p in self.worldturn.parties.iter_mut() {
            let days = p.days_out(tick);
            if days >= 1 && (p.told == 0 || days < p.told) {
                if p.told != 0 {
                    lines.push((p.civ, p.size, days));
                }
                p.told = days;
            }
        }
        for (civ, size, days) in lines {
            let name = self.overworld.civ(civ).name.clone();
            let when = if days == 1 { "a day away".to_string() } else { format!("{} days away", days) };
            self.msg(&format!("The army of the {}, {} strong, is {}.", name, size, when), (255, 150, 90));
        }
    }

    /// The race of a party's people (for its colour on the atlas).
    pub fn party_race(&self, p: &Party) -> Race {
        self.overworld.civs.get(p.civ as usize).map_or(Race::Goblin, |c| c.race)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// B17 step 1: after three years the legends hold history made since
    /// the embark, the fort heard the news of it, and the world is the same
    /// world on a second run (the fort's own rolls untouched).
    #[test]
    fn after_three_years_the_legends_show_wars_and_deaths_elsewhere_that_the_fort_heard_about() {
        let run = || {
            let mut g = Game::new(11);
            let y0 = g.overworld.year;
            let n0 = g.overworld.events.len();
            for _ in 0..3 {
                g.tick = (g.tick / YEAR + 1) * YEAR;
                g.world_yearly();
            }
            (g, y0, n0)
        };
        let (g, y0, n0) = run();
        assert_eq!(g.overworld.year, y0 + 3, "three years of history rolled");
        let new: Vec<&crate::overworld::Event> = g.overworld.events[n0..].iter().collect();
        assert!(new.iter().all(|e| e.year > y0 && e.year <= y0 + 3));
        assert!(new.len() >= 5, "the world did things: {:?}", new.iter().map(|e| &e.text).collect::<Vec<_>>());
        assert!(!g.worldturn.news.is_empty(), "the fort heard some of it");
        for (y, line) in &g.worldturn.news {
            assert!(new.iter().any(|e| e.year == *y && &e.text == line), "each piece of news is history: {}", line);
        }
        assert!(g.spotlights.iter().any(|s| s.title == "News from the world"));
        // Neighbours are worked out again from the world as it is now.
        assert_eq!(g.neighbors.len(), g.overworld.neighbors(g.site.0, g.site.1, 30).len());
        let (g2, ..) = run();
        assert_eq!(g2.overworld.events.len(), g.overworld.events.len(), "the same world turns the same way");
        assert_eq!(g2.worldturn.news, g.worldturn.news);
        // A spring with nothing to roll does nothing.
        let mut g3 = g2;
        let n = g3.overworld.events.len();
        g3.world_yearly();
        assert_eq!(g3.overworld.events.len(), n);
    }

    /// B01 step 2: a siege comes from a real town of the foe's, is seen days
    /// ahead on the world map, comes nearer each morning, and arrives when
    /// it said it would.
    #[test]
    fn a_siege_is_announced_days_ahead_and_seen_approaching() {
        // A fort whose nearest foe is far enough off to march for days.
        let mut found = None;
        for seed in 1..40 {
            let g = Game::new(seed);
            let far = g.neighbors.iter().filter(|n| g.at_war_with(n)).map(|n| n.dist).min().map_or(false, |d| d > CELLS_PER_DAY * 2);
            if far {
                found = Some(g);
                break;
            }
        }
        let mut g = found.expect("a fort with a far foe");
        g.creatures.retain(|_, c| c.tame);
        g.tick = SEASON * 2 + 1;
        for _ in 0..200 {
            if g.incoming.is_some() {
                break;
            }
            g.maybe_siege();
        }
        let (at, foe, n, leader) = g.incoming.clone().expect("an army comes");
        assert!(at >= g.tick + 2 * DAY, "warned at least two days ahead");
        let p = g.worldturn.parties.iter().find(|p| p.kind == PartyKind::Army).expect("the army is on the world map").clone();
        assert_eq!((p.civ, p.size, p.eta, p.leader.as_str()), (foe.civ, n, at, leader.as_str()));
        let from = &g.overworld.sites[p.from.expect("it set out from a town") as usize];
        assert_eq!(from.civ, Some(foe.civ), "a town of theirs");
        assert!(g.log.iter().any(|l| l.text.contains("days' march away") && l.text.contains(&from.name)));
        // Each morning it is nearer, and the fort hears so.
        let home = g.site;
        let dist = |c: (i32, i32)| (c.0 - home.0).abs().max((c.1 - home.1).abs());
        let mut last = dist(p.at(g.tick));
        while g.tick + DAY < at {
            g.tick = (g.tick / DAY + 1) * DAY;
            g.world_daily();
            let now = dist(g.worldturn.parties[0].at(g.tick));
            assert!(now < last, "the army comes nearer");
            last = now;
        }
        assert!(g.log.iter().any(|l| l.text.contains("a day away")));
        assert!(g.creatures.values().all(|c| !c.siege), "nobody's here yet");
        g.tick = at;
        g.army_arrives();
        assert!(g.creatures.values().any(|c| c.siege), "the siege begins at its time");
        g.world_daily();
        assert!(g.worldturn.parties.is_empty(), "and it leaves the map");
    }

    /// The turned world goes into the shared world file: a game that starts
    /// from the world as it was at the embark (an adventure) finds the new
    /// history, the ruins and the new towns, even after it writes its own
    /// older copy back.
    #[test]
    fn the_turned_world_reaches_the_world_file() {
        let mut g = Game::new(11);
        let old = g.overworld.clone();
        for _ in 0..3 {
            g.tick = (g.tick / YEAR + 1) * YEAR;
            g.world_yearly();
        }
        assert!(g.worldturn.changes > 0, "three years changed something");
        let dir = std::env::temp_dir().join(format!("dwarfclone-worldturn-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        crate::worldfile::record(&dir, &g.overworld).unwrap();
        crate::worldfile::record(&dir, &old).unwrap();
        let mut a = old.clone();
        crate::worldfile::refresh(&dir, &mut a);
        let now = &g.overworld;
        assert_eq!(a.year, now.year);
        assert_eq!(a.sites.len(), now.sites.len(), "new places stand");
        for (s, t) in a.sites.iter().zip(&now.sites) {
            assert_eq!((s.ruined, s.civ, &s.name), (t.ruined, t.civ, &t.name));
        }
        for c in &now.civs {
            assert_eq!(a.civ(c.id).alive, c.alive);
        }
        assert!(now.events[old.events.len()..].iter().all(|e| a.events.iter().any(|x| x.text == e.text)), "the new history is there");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
