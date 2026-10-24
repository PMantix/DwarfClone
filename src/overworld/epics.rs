//! The great events of history: rare turns that change the world for good.
//! A mountain bursts into a new volcano and buries the towns below it; a
//! star falls and blights the land; a great beast goes on a rampage and
//! ends a people; a plague empties a realm; a realm splits in two; a hero
//! conquers a neighbour and crowns themself emperor.
//!
//! Each world rolls for them on streams of its own (seed and year), so the
//! same seed and settings always bring the same ones. They are rare: a
//! world of two hundred years sees one or two at the usual setting, some
//! none. Each is kept in `Overworld::epics` (for the world-creation show
//! to linger on) and told in the legends.

use super::{ClashEnd, Overworld, Race, SiteKind, WorldParams};
use crate::rng::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EpicKind {
    Eruption,
    Starfall,
    Rampage,
    Plague,
    Sundering,
    Empire,
}

impl EpicKind {
    /// A word for the kind, for the show's banner.
    pub fn title(self) -> &'static str {
        match self {
            EpicKind::Eruption => "Cataclysm",
            EpicKind::Starfall => "Cataclysm",
            EpicKind::Rampage => "Doom",
            EpicKind::Plague => "Plague",
            EpicKind::Sundering => "Sundering",
            EpicKind::Empire => "Empire",
        }
    }
}

/// One great event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Epic {
    pub year: i32,
    pub kind: EpicKind,
    /// "The Burning of Ironhold".
    pub name: String,
    /// What happened, as the legends tell it.
    pub text: String,
    /// Where it struck.
    pub x: i32,
    pub y: i32,
    /// The sites it ruined or changed hands.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sites: Vec<u32>,
}

/// The chance in a year, by setting, and the years that must pass between two.
const RATE: [f64; 3] = [0.0, 1.0 / 110.0, 1.0 / 40.0];
const QUIET: [i32; 3] = [0, 45, 18];

fn hash(parts: &[u64]) -> u64 {
    crate::game::names::hash(parts)
}

fn pick<'a>(h: u64, salt: u64, list: &[&'a str]) -> &'a str {
    list[(hash(&[h, salt]) % list.len() as u64) as usize]
}

impl Overworld {
    /// Roll for a great event in `year` of history before the embark.
    pub(super) fn epic_year(&mut self, p: &WorldParams, year: i32) {
        let lvl = p.epics.min(2) as usize;
        if lvl == 0 || year < 12 {
            return;
        }
        if self.epics.last().map_or(false, |e| year - e.year < QUIET[lvl]) {
            return;
        }
        let mut r = Rng::new(self.seed ^ 0xE91C_0000 ^ (year as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        if !r.chance(RATE[lvl]) {
            return;
        }
        // The kinds in a shuffled order; the first the world allows happens.
        let mut kinds = vec![EpicKind::Eruption, EpicKind::Starfall, EpicKind::Rampage, EpicKind::Plague, EpicKind::Sundering, EpicKind::Empire];
        for i in (1..kinds.len()).rev() {
            let j = r.range(0, i as i32 + 1) as usize;
            kinds.swap(i, j);
        }
        for k in kinds {
            let done = match k {
                EpicKind::Eruption => self.eruption(&mut r, p, year),
                EpicKind::Starfall => self.starfall(&mut r, p, year),
                EpicKind::Rampage => self.rampage(&mut r, year),
                EpicKind::Plague => self.plague(&mut r, year),
                EpicKind::Sundering => self.sundering(&mut r, year),
                EpicKind::Empire => self.empire(&mut r, year),
            };
            if let Some(e) = done {
                self.log(year, e.text.clone());
                self.epics.push(e);
                self.check_extinct(year);
                return;
            }
        }
    }

    /// Standing sites of peoples within `r` (map distance) of (x, y).
    fn sites_within(&self, x: i32, y: i32, r: i32) -> Vec<u32> {
        self.sites.iter().filter(|s| !s.ruined && s.civ.is_some() && super::spread::gap_between((s.x, s.y), (x, y)) <= r).map(|s| s.id).collect()
    }

    /// Ruin the sites, noting each for the show.
    fn ruin(&mut self, sites: &[u32], from: u32, year: i32) {
        for &s in sites {
            self.sites[s as usize].ruined = true;
            self.sites[s as usize].pop = 0;
            self.clash(year, from, s, ClashEnd::Fell);
        }
    }

    fn names_of(&self, sites: &[u32]) -> String {
        let names: Vec<&str> = sites.iter().map(|&s| self.sites[s as usize].name.as_str()).collect();
        match names.len() {
            0 => String::new(),
            1 => names[0].to_string(),
            n => format!("{} and {}", names[..n - 1].join(", "), names[n - 1]),
        }
    }

    /// A mountain near peoples bursts open: a new volcano, the towns at its
    /// feet buried in ash and fire.
    fn eruption(&mut self, r: &mut Rng, p: &WorldParams, year: i32) -> Option<Epic> {
        let reach = (4.0 * p.reach()).round() as i32;
        let mut best: Option<(usize, usize)> = None;
        for _ in 0..300 {
            let (x, y) = (r.range(1, self.w - 1), r.range(1, self.h - 1));
            let c = self.cell(x, y);
            if c.biome != super::Biome::Mountain || c.site != 0 {
                continue;
            }
            let n = self.sites_within(x, y, reach).len();
            if n > 0 && best.map_or(true, |b| n > b.1) {
                best = Some(((y * self.w + x) as usize, n));
            }
        }
        let (i, _) = best?;
        let (x, y) = (i as i32 % self.w, i as i32 / self.w);
        let hit = self.sites_within(x, y, reach);
        let peak = match self.cells[i].region {
            0 => "the mountains".to_string(),
            g => self.regions.get(g as usize - 1).map_or("the mountains".to_string(), |g| g.name.clone()),
        };
        self.cells[i].volcano = true;
        let first = hit[0];
        self.ruin(&hit, first, year);
        let h = hash(&[self.seed, year as u64, 0xE7]);
        let name = format!("The {} of {}", pick(h, 1, &["Burning", "Ashfall", "Fire", "Wrath"]), self.sites[first as usize].name);
        let text = format!("{}: a mountain of {} split open and poured fire down its slopes; {} {} buried in ash. A new volcano smokes there still.", name, peak.replacen("The ", "the ", 1), self.names_of(&hit), if hit.len() == 1 { "was" } else { "were" });
        Some(Epic { year, kind: EpicKind::Eruption, name, text, x, y, sites: hit })
    }

    /// A star falls among the peoples: the sites near it are gone and the
    /// land round the crater is blighted for ever.
    fn starfall(&mut self, r: &mut Rng, p: &WorldParams, year: i32) -> Option<Epic> {
        let standing: Vec<u32> = self.sites.iter().filter(|s| !s.ruined && s.civ.is_some()).map(|s| s.id).collect();
        if standing.len() < 4 {
            return None;
        }
        let at = self.sites[*r.pick(&standing) as usize].clone();
        let (x, y) = ((at.x + r.range(-3, 4)).clamp(0, self.w - 1), (at.y + r.range(-2, 3)).clamp(0, self.h - 1));
        let reach = (3.0 * p.reach()).round() as i32;
        let hit = self.sites_within(x, y, reach);
        if hit.is_empty() {
            return None;
        }
        for dy in -reach / 2..=reach / 2 {
            for dx in -reach..=reach {
                let (cx, cy) = (x + dx, y + dy);
                if cx >= 0 && cy >= 0 && cx < self.w && cy < self.h && super::spread::gap_between((cx, cy), (x, y)) <= reach && !self.cell(cx, cy).biome.is_water() {
                    self.cell_mut(cx, cy).evil = true;
                }
            }
        }
        self.ruin(&hit, hit[0], year);
        let h = hash(&[self.seed, year as u64, 0x57A2]);
        let name = format!("The {}", pick(h, 2, &["Starfall", "Night of Falling Fire", "Sky's Wound", "Black Star"]));
        let text = format!("{}: a star fell burning from the sky near {}. {} {} no more, and the land round the crater is blighted to this day.", name, at.name, self.names_of(&hit), if hit.len() == 1 { "is" } else { "are" });
        Some(Epic { year, kind: EpicKind::Starfall, name, text, x, y, sites: hit })
    }

    /// A great beast wakes in fury and lays waste to a whole people, its
    /// capital first, mountainhome or not.
    fn rampage(&mut self, r: &mut Rng, year: i32) -> Option<Epic> {
        let beasts: Vec<u32> = self.figures.iter().filter(|f| f.lair.is_some() && f.alive()).map(|f| f.id).collect();
        if beasts.is_empty() {
            return None;
        }
        let b = *r.pick(&beasts);
        let lair = self.sites[self.figures[b as usize].lair? as usize].clone();
        // The people nearest its lair.
        let civ = self
            .sites
            .iter()
            .filter(|s| !s.ruined && s.civ.map_or(false, |c| self.civs[c as usize].alive))
            .min_by_key(|s| super::spread::gap_between((s.x, s.y), (lair.x, lair.y)))?
            .civ?;
        let mut theirs = self.living_sites(civ);
        theirs.sort_by_key(|&s| super::spread::gap_between((self.sites[s as usize].x, self.sites[s as usize].y), (lair.x, lair.y)));
        let n = theirs.len().min(r.range(3, 7) as usize);
        let mut hit: Vec<u32> = theirs[..n].to_vec();
        let cap = self.civs[civ as usize].capital;
        if !hit.contains(&cap) && !self.sites[cap as usize].ruined {
            hit.push(cap);
        }
        self.ruin(&hit, lair.id, year);
        let (bname, bkind) = (self.figures[b as usize].name.clone(), self.figures[b as usize].kind.clone());
        let cname = self.civs[civ as usize].name.clone();
        self.figures[b as usize].kills += 100 + 50 * hit.len() as u32;
        self.figures[b as usize].title = Some(format!("Doom of the {}", cname));
        let ends = if self.living_sites(civ).is_empty() { " Of that people nothing was left." } else { "" };
        let name = format!("The Rampage of {}", bname);
        let text = format!("{}: {} the {} came down from its lair and laid waste to the {} of the {}; {} fell to fire and claw.{}", name, bname, bkind, self.people(civ), cname, self.names_of(&hit), ends);
        let (x, y) = (self.sites[cap as usize].x, self.sites[cap as usize].y);
        Some(Epic { year, kind: EpicKind::Rampage, name, text, x, y, sites: hit })
    }

    /// A sickness sweeps a realm: its towns emptied by half, the small ones
    /// abandoned, its ruler dead.
    fn plague(&mut self, r: &mut Rng, year: i32) -> Option<Epic> {
        let big: Vec<u32> = self.civs.iter().filter(|c| c.alive && self.living_sites(c.id).len() >= 3).map(|c| c.id).collect();
        if big.is_empty() {
            return None;
        }
        let civ = *r.pick(&big);
        let mut emptied = Vec::new();
        for s in self.living_sites(civ) {
            let site = &mut self.sites[s as usize];
            site.pop /= 2;
            if site.pop < 60 && self.civs[civ as usize].capital != s {
                emptied.push(s);
            }
        }
        let cap = self.civs[civ as usize].capital;
        self.ruin(&emptied, cap, year);
        if let Some(f) = self.civs[civ as usize].ruler {
            if self.figures[f as usize].alive() {
                self.kill_figure(f, year, "died of plague".into());
                self.civs[civ as usize].ruler = None;
                self.crown(r, civ, year);
            }
        }
        let h = hash(&[self.seed, year as u64, 0x9A6]);
        let name = format!("The {}", pick(h, 3, &["Red Cough", "Grey Sleep", "Weeping Death", "Black Fever", "Hollow Sickness"]));
        let cname = self.civs[civ as usize].name.clone();
        let empty = if emptied.is_empty() { String::new() } else { format!(" {} stood empty after.", self.names_of(&emptied)) };
        let text = format!("{}: a sickness swept the lands of the {}. Half the {} died, their ruler among them.{}", name, cname, self.people(civ), empty);
        let (x, y) = (self.sites[cap as usize].x, self.sites[cap as usize].y);
        Some(Epic { year, kind: EpicKind::Plague, name, text, x, y, sites: emptied })
    }

    /// A great realm splits: its far settlements swear to a rival ruler and
    /// a new people of the same blood rises against the old.
    fn sundering(&mut self, r: &mut Rng, year: i32) -> Option<Epic> {
        let big: Vec<u32> = self.civs.iter().filter(|c| c.alive && self.made_of(c.id).is_none() && self.living_sites(c.id).len() >= 4).map(|c| c.id).collect();
        if big.is_empty() {
            return None;
        }
        let old = *r.pick(&big);
        let cap = self.sites[self.civs[old as usize].capital as usize].clone();
        let mut sites = self.living_sites(old);
        sites.retain(|&s| s != cap.id);
        sites.sort_by_key(|&s| -super::spread::gap_between((self.sites[s as usize].x, self.sites[s as usize].y), (cap.x, cap.y)));
        let gone: Vec<u32> = sites[..(sites.len() + 1) / 2].to_vec();
        let race = self.civs[old as usize].race;
        let id = self.civs.len() as u32;
        let mut name = super::civ_name(r, race);
        for _ in 0..8 {
            if !self.civs.iter().any(|c| c.name == name) {
                break;
            }
            name = super::civ_name(r, race);
        }
        self.civs.push(super::Civ { id, name: name.clone(), race, alive: true, enemies: vec![old], ruler: None, capital: gone[0] });
        self.civs[old as usize].enemies.push(id);
        // Old enemies stay enemies of both.
        let foes = self.civs[old as usize].enemies.clone();
        for f in foes.into_iter().filter(|&f| f != id) {
            self.civs[id as usize].enemies.push(f);
            self.civs[f as usize].enemies.push(id);
        }
        for &s in &gone {
            self.sites[s as usize].civ = Some(id);
            self.clash(year, cap.id, s, ClashEnd::Taken);
        }
        self.crown(r, id, year);
        let oname = self.civs[old as usize].name.clone();
        let h = hash(&[self.seed, year as u64, 0x5D]);
        let ename = format!("The {}", pick(h, 4, &["Sundering", "Breaking of Oaths", "Great Schism", "War of Two Crowns"]));
        let text = format!("{}: the {} of the {} broke apart. {} swore to a new crown and rose as the {}, sworn foes of the old realm.", ename, race.plural(), oname, self.names_of(&gone), name);
        let (x, y) = (self.sites[gone[0] as usize].x, self.sites[gone[0] as usize].y);
        Some(Epic { year, kind: EpicKind::Sundering, name: ename, text, x, y, sites: gone })
    }

    /// A hero of one people conquers a neighbouring one whole and is
    /// crowned emperor over both.
    fn empire(&mut self, r: &mut Rng, year: i32) -> Option<Epic> {
        let alive: Vec<u32> = self.civs.iter().filter(|c| c.alive && self.made_of(c.id).is_none()).map(|c| c.id).collect();
        let mut pairs: Vec<(i32, u32, u32)> = Vec::new();
        for &a in &alive {
            for &b in &alive {
                if a == b || self.civs[a as usize].race == self.civs[b as usize].race {
                    continue;
                }
                let (ma, mb) = (self.living_sites(a), self.living_sites(b));
                if ma.len() < 2 || mb.is_empty() || mb.len() > ma.len() {
                    continue;
                }
                let d = ma.iter().flat_map(|&x| mb.iter().map(move |&y| (x, y))).map(|(x, y)| super::spread::gap_between((self.sites[x as usize].x, self.sites[x as usize].y), (self.sites[y as usize].x, self.sites[y as usize].y))).min().unwrap_or(i32::MAX);
                pairs.push((d, a, b));
            }
        }
        pairs.sort();
        let &(_, a, b) = pairs.first()?;
        let race = self.civs[a as usize].race;
        let born = year - r.range(25, 45);
        let hero = self.add_figure(r, &format!("{:?}", race).to_lowercase(), Some(race), Some(a), born);
        let won = self.living_sites(b);
        for &s in &won {
            self.sites[s as usize].civ = Some(a);
            if race == Race::Goblin {
                self.sites[s as usize].kind = SiteKind::DarkFortress;
            }
            self.clash(year, self.civs[a as usize].capital, s, ClashEnd::Taken);
        }
        let (aname, bname) = (self.civs[a as usize].name.clone(), self.civs[b as usize].name.clone());
        let title = if race == Race::Goblin { "dread emperor" } else { "emperor" };
        self.figures[hero as usize].title = Some(format!("{} of the {}", title, aname));
        self.figures[hero as usize].kills += r.range(20, 60) as u32;
        if let Some(old) = self.civs[a as usize].ruler {
            if self.figures[old as usize].alive() {
                self.kill_figure(old, year, format!("set aside by {}", self.figures[hero as usize].name));
            }
        }
        self.civs[a as usize].ruler = Some(hero);
        self.civs[a as usize].enemies.retain(|&e| e != b);
        self.civs[b as usize].alive = false;
        let hname = self.figures[hero as usize].name.clone();
        let name = format!("The Empire of {}", hname.split(' ').next().unwrap_or(&hname));
        let text = format!("{}: {} of the {} led the {} to conquer the {} of the {} whole, and was crowned {} over all their lands.", name, hname, aname, race.plural(), self.people(b), bname, title);
        let cap = self.civs[b as usize].capital;
        let (x, y) = (self.sites[cap as usize].x, self.sites[cap as usize].y);
        Some(Epic { year, kind: EpicKind::Empire, name, text, x, y, sites: won })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overworld::WorldGen;

    /// Great events are rare at the usual setting, more often when asked
    /// for, never when turned off, and each one changes the world.
    #[test]
    fn epics_are_rare_and_leave_their_mark() {
        let (mut rare, mut common, mut kinds) = (0, 0, std::collections::HashSet::new());
        for seed in 1..=8u64 {
            let p = WorldParams { seed, history: 250, ..WorldParams::default() };
            let ow = WorldGen::new(p).finish();
            rare += ow.epics.len();
            assert!(ow.epics.len() <= 5, "seed {}: {} epics at 'rare'", seed, ow.epics.len());
            let ow = WorldGen::new(WorldParams { epics: 2, ..p }).finish();
            common += ow.epics.len();
            for e in &ow.epics {
                kinds.insert(e.kind);
                // Told in the legends, in its year.
                assert!(ow.events.iter().any(|v| v.year == e.year && v.text == e.text), "seed {}: {} not in the legends", seed, e.name);
                match e.kind {
                    EpicKind::Eruption => assert!(ow.cell(e.x, e.y).volcano),
                    EpicKind::Starfall => assert!(ow.cell(e.x, e.y).evil || ow.cell(e.x, e.y).biome.is_water()),
                    _ => {}
                }
                if matches!(e.kind, EpicKind::Rampage | EpicKind::Starfall | EpicKind::Eruption | EpicKind::Empire | EpicKind::Sundering) {
                    assert!(!e.sites.is_empty(), "seed {}: {} changed no site", seed, e.name);
                }
            }
            let none = WorldGen::new(WorldParams { epics: 0, ..p }).finish();
            assert!(none.epics.is_empty());
        }
        eprintln!("8 worlds of 250 years: {} rare epics, {} common, kinds {:?}", rare, common, kinds);
        assert!((2..=20).contains(&rare), "{} epics in 8 worlds at 'rare'", rare);
        assert!(common > rare, "common {} vs rare {}", common, rare);
        assert!(kinds.len() >= 3, "only {:?}", kinds);
    }
}
