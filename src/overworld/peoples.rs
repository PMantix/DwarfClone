//! Peoples made by villains (Tarn's "an evil wizard creates a randomized
//! race"): somewhere in the world's history a necromancer or a dark wizard
//! raises a tower, and in it twists prisoners, beasts or the dead into a
//! new people with a name, a body, a look, a temper and a power of their
//! own. From then on they are a civilization like any other: they found
//! towns, go to war and make peace, and the legends keep it all. A fortress
//! near them meets them as besiegers or (the less warlike) as traders.
//!
//! Everything here draws on streams of its own seeded by the world, so a
//! world's made people are the same every time it is made, and the rest of
//! history up to their making is untouched.

use super::*;
use crate::game::horrors::{pick_powers, Power};

/// How a made people meets the world.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Temper {
    /// At war with everyone, like the goblins: they besiege.
    Warlike,
    /// Keep to themselves, and pick fights with their neighbours now and then.
    Wary,
    /// Trade, even with dwarves: their caravans come in the dead of winter.
    Mercantile,
}

impl Temper {
    pub fn name(self) -> &'static str {
        match self {
            Temper::Warlike => "warlike",
            Temper::Wary => "wary",
            Temper::Mercantile => "given to trade",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MadeRace {
    /// The civilization they make up.
    pub civ: u32,
    /// "vorlak", "vorlaks".
    pub singular: String,
    pub plural: String,
    /// Their maker, a historical figure.
    pub maker: u32,
    /// The year they were made.
    pub year: i32,
    /// The tower they were made in (a site).
    pub tower: u32,
    /// What they were made from ("prisoners of war").
    pub from: String,
    /// "hunched, four-armed things with ash-grey skin".
    pub body: String,
    pub glyph: char,
    pub color: (u8, u8, u8),
    pub temper: Temper,
    /// Their gift from their maker: every one of them has it.
    pub power: Power,
    /// Body size (a dwarf is 1).
    pub size: f32,
    /// The syllables their names are made of.
    pub syllables: Vec<String>,
}

impl MadeRace {
    /// A name in their own tongue.
    pub fn word(&self, rng: &mut Rng, n: i32) -> String {
        let mut s = String::new();
        for _ in 0..n.max(1) {
            s.push_str(rng.pick(&self.syllables).as_str());
        }
        cap(&s)
    }
}

const ONSET: [&str; 16] = ["gr", "th", "v", "sk", "z", "m", "dr", "k", "x", "l", "r", "sh", "n", "b", "q", "h"];
const VOWEL: [&str; 9] = ["a", "o", "u", "i", "e", "au", "ei", "y", "oo"];
const CODA: [&str; 8] = ["k", "th", "n", "sh", "r", "", "l", "g"];
const STOOP: [(&str, f32); 6] = [("hunched", 0.9), ("long-limbed", 1.1), ("squat", 0.8), ("towering", 1.5), ("gaunt", 1.0), ("bloated", 1.3)];
const FORM: [&str; 9] = ["four-armed", "tusked", "eyeless", "winged", "scaled", "horned", "many-jointed", "fanged", "spider-legged"];
const SKIN: [(&str, (u8, u8, u8)); 7] = [
    ("ash-grey", (170, 170, 160)),
    ("blood-red", (210, 60, 50)),
    ("bone-white", (235, 230, 210)),
    ("green-black", (80, 130, 80)),
    ("mottled violet", (170, 110, 200)),
    ("slate-blue", (110, 140, 210)),
    ("rust-brown", (190, 110, 60)),
];
const SOURCES: [&str; 6] = ["prisoners of war", "the beasts of the wild", "the dead of a fallen town", "cave things and clay", "stolen herds and their herders", "wolves and outlaws"];

fn plural_of(s: &str) -> String {
    if s.ends_with('s') || s.ends_with("sh") || s.ends_with('x') {
        format!("{}es", s)
    } else if s.ends_with('y') && !s.ends_with("ey") {
        format!("{}ies", &s[..s.len() - 1])
    } else {
        format!("{}s", s)
    }
}

/// Seeds for the villains' streams.
const MAKING: u64 = 0x5EED_0F_E7A1;
const DEEDS: u64 = 0xD33D_5_0F_EB11;

impl Overworld {
    /// The made people a civilization is, if it is one.
    pub fn made_of(&self, civ: u32) -> Option<&MadeRace> {
        self.made.iter().find(|m| m.civ == civ)
    }

    /// What a civilization's people are called: "goblins", or "vorlaks".
    pub fn people(&self, civ: u32) -> String {
        match self.made_of(civ) {
            Some(m) => m.plural.clone(),
            None => self.civs.get(civ as usize).map_or("people".to_string(), |c| c.race.plural().to_string()),
        }
    }

    /// The year a villain makes a people in this world (if history runs
    /// that long).
    fn making_year(&self) -> i32 {
        let mut r = Rng::new(self.seed ^ MAKING);
        if r.chance(0.12) {
            return i32::MAX;
        }
        r.range(18, 70)
    }

    /// Once the world's history is written: the villains' tale, told year by
    /// year from the making on. It runs after the rest of history (and on
    /// streams of its own), so a world's other peoples, towns and wars up to
    /// its last year are the same with or without it; the events are then
    /// put back in order of their years.
    pub(super) fn villains_history(&mut self, end: i32) {
        let first = self.making_year();
        if first > end {
            return;
        }
        for year in first..=end {
            if year == first {
                let mut r = Rng::new(self.seed ^ MAKING ^ 0x77);
                self.make_people(&mut r, year);
            }
            // A bigger world may see a second villain, later.
            if self.w * self.h >= 192 * 80 && year == first.saturating_add(55) {
                let mut r = Rng::new(self.seed ^ MAKING ^ 0x99);
                self.make_people(&mut r, year);
            }
            let mut r = Rng::new(self.seed ^ DEEDS ^ (year as u64).wrapping_mul(0x9E37_79B9));
            for k in 0..self.made.len() {
                self.made_deeds(&mut r, k, year);
            }
        }
        self.events.sort_by_key(|e| e.year);
    }

    /// A tower on dark land far from anyone, a villain in it, and a new
    /// people of their making beside it.
    pub(crate) fn make_people(&mut self, r: &mut Rng, year: i32) -> Option<u32> {
        let temper = match r.range(0, 10) {
            0..=4 => Temper::Warlike,
            5..=7 => Temper::Wary,
            _ => Temper::Mercantile,
        };
        let base = if temper == Temper::Warlike { Race::Goblin } else { Race::Human };
        // The tower: evil land first, then lonely heights and wastes, never
        // on top of anyone.
        let mut spot = None;
        for pass in 0..3 {
            for _ in 0..600 {
                let (x, y) = (r.range(1, self.w - 1), r.range(1, self.h - 1));
                let c = self.cell(x, y);
                let lonely = self.sites.iter().all(|s| (s.x - x).abs().max((s.y - y).abs()) > if pass < 2 { 4 } else { 2 });
                let dark = match pass {
                    0 => c.evil,
                    1 => matches!(c.biome, Biome::Mountain | Biome::Tundra | Biome::Badlands | Biome::Swamp | Biome::Desert),
                    _ => true,
                };
                if c.site == 0 && !c.biome.is_water() && lonely && dark {
                    spot = Some((x, y));
                    break;
                }
            }
            if spot.is_some() {
                break;
            }
        }
        let (tx, ty) = spot?;
        // The villain.
        let villain_race = *r.pick(&[Race::Human, Race::Elf, Race::Dwarf, Race::Goblin]);
        let kind = if r.chance(0.5) { "necromancer" } else { "dark wizard" };
        let born = year - r.range(60, 300);
        let maker = self.add_figure(r, kind, Some(villain_race), None, born);
        let tower = self.add_site(r, SiteKind::Lair, None, tx, ty, year);
        let mname = self.figures[maker as usize].name.clone();
        let first = mname.split(' ').next().unwrap_or(&mname).to_string();
        self.sites[tower as usize].name = format!("the Tower of {}", first);
        let land = self.region_at(tx, ty).map_or_else(|| self.cell(tx, ty).biome.name().to_string(), |g| g.name.clone());
        self.log(year, format!("{}, a {} {}, raised the Tower of {} in {}.", mname, villain_race_word(villain_race), kind, first, land));
        // The people.
        let syllables: Vec<String> = (0..8).map(|_| format!("{}{}{}", r.pick(&ONSET), r.pick(&VOWEL), r.pick(&CODA))).collect();
        let singular = format!("{}{}", syllables[0], syllables[1].trim_end_matches(|c: char| "aeiouy".contains(c))).to_lowercase();
        let singular = if singular.len() < 4 { format!("{}{}", singular, syllables[2]) } else { singular };
        let plural = plural_of(&singular);
        let (stoop, size) = *r.pick(&STOOP);
        let form = *r.pick(&FORM);
        let (skin, color) = *r.pick(&SKIN);
        let body = format!("{}, {} things with {} skin", stoop, form, skin);
        let from = r.pick(&SOURCES).to_string();
        let power = pick_powers(r, &format!("{} {} {}", stoop, form, skin), 1, 1)[0];
        let glyph = singular.chars().next().unwrap_or('x').to_ascii_uppercase();
        let id = self.civs.len() as u32;
        let cname = format!("{} of {}", r.pick(&["Children", "Brood", "Host", "Get", "Spawn"]), first);
        self.civs.push(Civ { id, name: cname.clone(), race: base, alive: true, enemies: Vec::new(), ruler: None, capital: 0 });
        self.made.push(MadeRace { civ: id, singular: singular.clone(), plural: plural.clone(), maker, year, tower, from: from.clone(), body: body.clone(), glyph, color, temper, power, size, syllables });
        self.figures[maker as usize].civ = Some(id);
        self.figures[maker as usize].title = Some(format!("maker of the {}", plural));
        self.log(year, format!("In the Tower of {}, {} twisted {} into a new people: the {}, {}. They were called the {}.", first, mname, from, cap(&plural), body, cname));
        // Their first town, beside the tower.
        let home = self.free_spot_near(r, tx, ty, 3, base).or_else(|| self.any_spot_near(r, tx, ty, 4));
        let Some((x, y)) = home else {
            self.civs[id as usize].alive = false;
            return Some(id);
        };
        let kind = if temper == Temper::Warlike { SiteKind::DarkFortress } else { SiteKind::Town };
        let sid = self.add_site(r, kind, Some(id), x, y, year);
        self.civs[id as usize].capital = sid;
        let sname = self.sites[sid as usize].name.clone();
        self.log(year, format!("The {} of the {} founded {} in the shadow of the tower.", plural, cname, sname));
        self.crown(r, id, year);
        // The warlike hate everyone, and everyone them.
        if temper == Temper::Warlike {
            for o in 0..id {
                if self.civs[o as usize].alive {
                    self.civs[id as usize].enemies.push(o);
                    self.civs[o as usize].enemies.push(id);
                }
            }
            self.log(year, format!("The {} swore war on every people of the world.", cname));
        }
        Some(id)
    }

    /// Any dry, empty cell near (x, y).
    fn any_spot_near(&self, r: &mut Rng, x: i32, y: i32, rad: i32) -> Option<(i32, i32)> {
        for _ in 0..80 {
            let (nx, ny) = (x + r.range(-rad, rad + 1), y + r.range(-rad, rad + 1));
            if nx < 0 || ny < 0 || nx >= self.w || ny >= self.h {
                continue;
            }
            let c = self.cell(nx, ny);
            if c.site == 0 && !c.biome.is_water() {
                return Some((nx, ny));
            }
        }
        None
    }

    /// A year in the life of a made people: the wary pick fights, the
    /// traders make peace, the maker grows more towns or meets an end.
    fn made_deeds(&mut self, r: &mut Rng, k: usize, year: i32) {
        let m = self.made[k].clone();
        let civ = m.civ;
        if !self.civs[civ as usize].alive {
            return;
        }
        let cname = self.civs[civ as usize].name.clone();
        let mine = self.living_sites(civ);
        let near = |ow: &Overworld, o: u32| {
            ow.living_sites(o).iter().any(|&s| mine.iter().any(|&t| (ow.sites[s as usize].x - ow.sites[t as usize].x).abs().max((ow.sites[s as usize].y - ow.sites[t as usize].y).abs()) <= 25))
        };
        let others: Vec<u32> = self.civs.iter().filter(|c| c.alive && c.id != civ).map(|c| c.id).collect();
        match m.temper {
            Temper::Wary => {
                let targets: Vec<u32> = others.iter().copied().filter(|&o| !self.civs[civ as usize].enemies.contains(&o) && near(self, o)).collect();
                if !targets.is_empty() && r.chance(0.06) {
                    let o = *r.pick(&targets);
                    self.civs[civ as usize].enemies.push(o);
                    self.civs[o as usize].enemies.push(civ);
                    let oname = self.civs[o as usize].name.clone();
                    self.log(year, format!("The {} of the {} fell upon the {} without warning.", cap(&m.plural), cname, oname));
                }
            }
            Temper::Mercantile => {
                let foes: Vec<u32> = self.civs[civ as usize].enemies.clone();
                for o in foes {
                    if self.civs[o as usize].race != Race::Goblin && r.chance(0.2) {
                        self.civs[civ as usize].enemies.retain(|&e| e != o);
                        self.civs[o as usize].enemies.retain(|&e| e != civ);
                        let oname = self.civs[o as usize].name.clone();
                        self.log(year, format!("The {} bought peace from the {} with strange goods.", cname, oname));
                    }
                }
            }
            Temper::Warlike => {}
        }
        // Their wars: they march on their foes, and their foes on them
        // (only on towns standing in that year).
        let foes = self.civs[civ as usize].enemies.clone();
        for o in foes {
            if !self.civs[civ as usize].alive {
                break;
            }
            if self.civs[o as usize].alive && self.fair_fight(civ, o, year) && r.chance(0.12) {
                self.battle(r, civ, o, year);
            }
            if self.civs[civ as usize].alive && self.civs[o as usize].alive && self.fair_fight(o, civ, year) && r.chance(0.08) {
                self.battle(r, o, civ, year);
            }
        }
        if !self.civs[civ as usize].alive {
            return;
        }
        // A fallen overlord is followed by another.
        let ruler_dead = self.civs[civ as usize].ruler.map_or(true, |f| !self.figures[f as usize].alive());
        if ruler_dead && !self.living_sites(civ).is_empty() {
            self.crown(r, civ, year);
        }
        // The maker breeds more of them: a new town now and then.
        let maker_lives = self.figures[m.maker as usize].alive();
        if maker_lives && !mine.is_empty() && r.chance(0.12) {
            let from = self.sites[*r.pick(&mine) as usize].clone();
            if let Some((x, y)) = self.any_spot_near(r, from.x, from.y, 5) {
                let kind = if m.temper == Temper::Warlike { SiteKind::DarkFortress } else { SiteKind::Hamlet };
                let sid = self.add_site(r, kind, Some(civ), x, y, year);
                let sname = self.sites[sid as usize].name.clone();
                self.log(year, format!("{} bred more {}, and they founded {}.", self.figures[m.maker as usize].name.clone(), m.plural, sname));
            }
        }
        // Makers don't live for ever: their own creatures, or a hero, end them.
        if maker_lives && year - m.year > 15 && r.chance(0.03) {
            let mname = self.figures[m.maker as usize].name.clone();
            if r.chance(0.5) {
                self.kill_figure(m.maker, year, format!("torn apart by the {} of their own making", m.plural));
                self.log(year, format!("The {} turned on their maker {} and tore them apart.", m.plural, mname));
            } else {
                let race = *r.pick(&[Race::Dwarf, Race::Human, Race::Elf]);
                let born = year - r.range(25, 60);
                let hero = self.add_figure(r, &format!("{:?}", race).to_lowercase(), Some(race), None, born);
                let hname = self.figures[hero as usize].name.clone();
                self.figures[hero as usize].kills += 1;
                self.figures[hero as usize].title = Some(format!("bane of {}", mname));
                self.kill_figure(m.maker, year, format!("slain by {}", hname));
                self.log(year, format!("{} climbed the Tower of {} and slew {}, maker of the {}.", hname, mname.split(' ').next().unwrap_or(""), mname, m.plural));
            }
        }
    }

    /// Whether `att` could strike `def` in `year`: the two towns `battle`
    /// would pick (the closest pair in marching reach) both stood then.
    fn fair_fight(&self, att: u32, def: u32, year: i32) -> bool {
        let mine = self.living_sites(att);
        let theirs = self.living_sites(def);
        let mut best: Option<(i32, u32, u32)> = None;
        for &m in &mine {
            for &t in &theirs {
                let (ms, ts) = (&self.sites[m as usize], &self.sites[t as usize]);
                let d = (ms.x - ts.x).abs().max((ms.y - ts.y).abs());
                if d <= 18 && best.map_or(true, |b| d < b.0) {
                    best = Some((d, m, t));
                }
            }
        }
        best.map_or(false, |(_, m, t)| self.sites[m as usize].founded <= year && self.sites[t as usize].founded <= year)
    }

    /// Lines for the legends' civilization tab: who made them, what they are.
    pub fn made_lines(&self, civ: u32) -> Vec<String> {
        let Some(m) = self.made_of(civ) else { return Vec::new() };
        let maker = &self.figures[m.maker as usize];
        let fate = if maker.alive() { String::new() } else { format!(" (now dead: {})", maker.cause.clone().unwrap_or_default()) };
        vec![
            format!("   made by {} the {} from {} in {}{}", maker.name, maker.kind, m.from, m.year, fate),
            format!("   {}; {}; every one of them {}", m.body, m.temper.name(), m.power.known()),
        ]
    }

    /// A place name in a made people's tongue, for a site of theirs.
    pub(super) fn made_place(&self, r: &mut Rng, civ: u32) -> Option<String> {
        let m = self.made_of(civ)?;
        let w = m.word(r, 2);
        Some(match r.range(0, 3) {
            0 => w,
            1 => format!("{} Pits", w),
            _ => format!("{}'s Hollow", w),
        })
    }

    /// A person's name in a made people's tongue.
    pub(super) fn made_person(&self, r: &mut Rng, civ: u32) -> Option<String> {
        let m = self.made_of(civ)?;
        Some(format!("{} {}", m.word(r, 1), m.word(r, 2)))
    }
}

fn villain_race_word(r: Race) -> &'static str {
    match r {
        Race::Dwarf => "dwarven",
        Race::Human => "human",
        Race::Elf => "elven",
        Race::Goblin => "goblin",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world(seed: u64) -> Overworld {
        WorldGen::new(WorldParams { seed, ..WorldParams::default() }).finish()
    }

    #[test]
    fn a_villain_makes_a_people_and_they_live_in_the_legends() {
        // Most worlds have one; find one.
        let ow = (1..30u64).map(world).find(|ow| !ow.made.is_empty()).expect("no world made a people");
        let m = &ow.made[0];
        let civ = ow.civ(m.civ);
        assert!(civ.name.contains(" of "), "{}", civ.name);
        assert!(m.plural.len() > m.singular.len());
        let maker = &ow.figures[m.maker as usize];
        assert!(maker.kind == "necromancer" || maker.kind == "dark wizard");
        assert!(maker.title.as_deref().unwrap().contains(&m.plural));
        // The legends tell of the making and of their towns.
        assert!(ow.events.iter().any(|e| e.year == m.year && e.text.contains("twisted") && e.text.contains(&cap(&m.plural))));
        assert!(ow.sites.iter().any(|s| s.civ == Some(m.civ)), "no towns of theirs");
        assert!(ow.sites[m.tower as usize].name.starts_with("the Tower of"));
        assert!(!ow.made_lines(m.civ).is_empty());
        assert_eq!(ow.people(m.civ), m.plural);
        // Their first town has a name in their own tongue.
        let first = &ow.sites[civ.capital as usize].name;
        assert!(m.syllables.iter().any(|s| first.to_lowercase().contains(s.as_str())), "{} isn't in their tongue", first);
    }

    /// Prints the made peoples of a few worlds and their history
    /// (cargo test --release show_made_peoples -- --ignored --nocapture).
    #[test]
    #[ignore]
    fn show_made_peoples() {
        for seed in 1..6u64 {
            let ow = world(seed);
            for m in &ow.made {
                let c = ow.civ(m.civ);
                println!("seed {}: {} ({}), {} {}, {:?}", seed, c.name, m.plural, m.glyph, m.body, m.temper);
                for l in ow.made_lines(m.civ) {
                    println!("{}", l);
                }
                for e in ow.events.iter().filter(|e| e.text.contains(&c.name) || e.text.contains(&m.plural) || e.text.contains(&ow.figures[m.maker as usize].name)) {
                    println!("  {:>4} {}", e.year, e.text);
                }
            }
        }
    }

    #[test]
    fn made_peoples_are_the_same_in_the_same_world() {
        for seed in [3u64, 7, 12] {
            let a = world(seed);
            let b = world(seed);
            assert_eq!(a.made, b.made, "seed {}", seed);
        }
    }

    #[test]
    fn over_the_years_they_found_towns_and_go_to_war() {
        let worlds: Vec<Overworld> = (1..40u64).map(world).filter(|ow| !ow.made.is_empty()).collect();
        assert!(worlds.len() >= 20, "only {} of 39 worlds made a people", worlds.len());
        let towns: usize = worlds.iter().map(|ow| ow.made.iter().map(|m| ow.sites.iter().filter(|s| s.civ == Some(m.civ) && s.founded > m.year).count()).sum::<usize>()).sum();
        assert!(towns > 0, "no made people ever founded a second town");
        let wars = worlds.iter().filter(|ow| ow.made.iter().any(|m| !ow.civ(m.civ).enemies.is_empty() || ow.events.iter().any(|e| e.text.contains(&ow.civ(m.civ).name) && (e.text.contains("war") || e.text.contains("fell upon") || e.text.contains("Battle") || e.text.contains("Siege") || e.text.contains("Sack"))))).count();
        assert!(wars > 0, "no made people ever went to war");
        // Every temper turns up.
        for t in [Temper::Warlike, Temper::Wary, Temper::Mercantile] {
            assert!(worlds.iter().any(|ow| ow.made.iter().any(|m| m.temper == t)), "no {:?} people", t);
        }
    }

    #[test]
    fn history_speaks_of_them_by_their_own_name() {
        let ow = (1..30u64).map(world).find(|ow| ow.made.iter().any(|m| ow.sites.iter().filter(|s| s.civ == Some(m.civ)).count() > 1)).expect("no made people grew");
        let m = &ow.made[0];
        for e in &ow.events {
            if e.text.contains(&ow.civ(m.civ).name) && e.year > m.year {
                let base = ow.civ(m.civ).race.plural();
                assert!(!e.text.contains(&format!("the {} of the {}", base, ow.civ(m.civ).name)), "called {} in: {}", base, e.text);
            }
        }
    }
}
