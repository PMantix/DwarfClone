//! The shared world file (G03 A19, M8's FC6 built early): every game played
//! in a world writes what it added to the world's legends into
//! `saves/<World>.world.json`, and every new fort or adventure in that world
//! starts from it. So the fort embarked from the title years later still
//! has a bard who sings of the hero who bled out on the road, though that
//! adventure was never loaded again.
//!
//! Writes merge rather than overwrite: two games in one world can run apart
//! (a fort from the title, an adventure from a death page), and each keeps
//! what the other added. Only one game runs at a time, so there is no lock;
//! the file is written to a temporary name and renamed into place.

use crate::dwarf::Dwarf;
use crate::game::Game;
use crate::overworld::{Event, FortMark, Overworld};
use crate::saves;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// M8's name for a fort the world remembers (P01): the same record.
#[allow(dead_code)] // (M8 P01 takes it up)
pub type FortRecord = FortMark;

/// What `<World>.world.json` holds.
#[derive(Clone, Serialize, Deserialize)]
pub struct WorldRecord {
    /// The world's seed (the land under an embark is made from it).
    pub seed: u64,
    /// The newest overworld: its legends, figures, sagas, fallen forts and
    /// the bones of dead heroes, merged from every game that wrote it.
    pub overworld: Overworld,
    /// Heroes who settled down, and the town they settled in (for M4's
    /// visitors: a famous hero may come to a fort).
    #[serde(default)]
    pub wanderers: Vec<(Dwarf, String)>,
}

/// The load menu's line for a world: "Year 312: 3 fallen, 2 heroes, 4 sagas".
pub fn summary(ow: &Overworld) -> String {
    let heroes = ow.figures.iter().filter(|f| f.kind.ends_with("adventurer")).count();
    format!("Year {}: {} fallen, {} {}, {} sagas", ow.year, ow.forts.len(), heroes, if heroes == 1 { "hero" } else { "heroes" }, ow.sagas.len())
}

/// The world file for `ow` in `dir`, and what it holds if there is one: a
/// world of the same name and seed (a world of another seed that happens to
/// share the name gets its own file).
fn find(dir: &Path, ow: &Overworld) -> (String, Option<WorldRecord>) {
    let same = |r: &WorldRecord| r.seed == ow.seed || r.seed == 0 || ow.seed == 0;
    for m in saves::list_worlds(dir).into_iter().filter(|m| m.world == ow.name) {
        if let Ok(r) = saves::read_world(dir, &m.file) {
            if same(&r) {
                return (m.file, Some(r));
            }
        }
    }
    let base = saves::world_file(&ow.name);
    let file = if dir.join(&base).exists() { format!("{}-{}.world.json", base.trim_end_matches(".world.json"), ow.seed % 100_000) } else { base };
    (file, None)
}

/// Record what `ow` knows into its world's file, merged with what is there.
pub fn record(dir: &Path, ow: &Overworld) -> Result<String, String> {
    record_with(dir, ow, None)
}

/// As `record`, with a hero who settled down (and where) among the world's
/// wanderers.
pub fn record_with(dir: &Path, ow: &Overworld, wanderer: Option<(Dwarf, String)>) -> Result<String, String> {
    if ow.w <= 1 || ow.name.is_empty() {
        return Ok(String::new());
    }
    let (file, old) = find(dir, ow);
    let mut rec = match old {
        Some(mut r) => {
            r.overworld = merge(&r.overworld, ow);
            r
        }
        None => WorldRecord { seed: ow.seed, overworld: ow.clone(), wanderers: Vec::new() },
    };
    if rec.seed == 0 {
        rec.seed = ow.seed;
    }
    // The file is the world's truth: what one fort has forgotten is its own.
    rec.overworld.known.clear();
    if let Some((d, home)) = wanderer {
        rec.wanderers.retain(|(w, _)| w.name != d.name);
        rec.wanderers.push((d, home));
    }
    saves::write_world(dir, &file, &rec)?;
    Ok(file)
}

/// A new fort or adventure in `ow`'s world: what the world file knows is
/// merged in. Nothing changes without a file.
pub fn refresh(dir: &Path, ow: &mut Overworld) {
    if let (_, Some(r)) = find(dir, ow) {
        *ow = merge(&r.overworld, ow);
    }
}

/// A fort was saved (the game-over save, F5, quitting, and the spring
/// autosave): its world hears of it. Errors are the save's own business;
/// the fort's save is already written.
pub fn fort_saved(dir: &Path, g: &Game, autosave: bool) {
    let spring = (g.tick / crate::game::SEASON) % 4 == 0;
    if !autosave || g.fallen || spring {
        let _ = record(dir, &g.overworld);
    }
}

/// Two copies of one world merged: the newer (by year, then legends) is the
/// base, and the other's legends, sagas, figures, fallen forts, remains and
/// ruins are added to it. Pure: neither is changed.
pub fn merge(a: &Overworld, b: &Overworld) -> Overworld {
    let a_newer = (a.year, a.events.len()) >= (b.year, b.events.len());
    let (base, other) = if a_newer { (a, b) } else { (b, a) };
    let mut m = base.clone();
    m.year = a.year.max(b.year);

    // Figures: the same person (by id and name, or by name, kind and
    // birth) is one, and a death wins; anyone else is added with a new id.
    let mut remap: Vec<(u32, u32)> = Vec::new();
    for f in &other.figures {
        let same = |g: &crate::overworld::Figure| g.name == f.name && (g.id == f.id || (g.kind == f.kind && g.born == f.born));
        let hit = m.figures.iter().position(|g| g.id == f.id && same(g)).or_else(|| m.figures.iter().position(|g| same(g)));
        match hit {
            Some(i) => {
                let g = &mut m.figures[i];
                let id = g.id;
                if g.died.is_none() && f.died.is_some() {
                    g.died = f.died;
                    g.cause = f.cause.clone();
                }
                g.kills = g.kills.max(f.kills);
                if g.title.is_none() {
                    g.title = f.title.clone();
                }
                remap.push((f.id, id));
            }
            None => {
                let id = m.push_figure(f.clone());
                remap.push((f.id, id));
            }
        }
    }
    let fig = |id: u32| remap.iter().find(|(o, _)| *o == id).map_or(id, |(_, n)| *n);

    // Legends, by year and text; a tale grown further in one telling keeps
    // that telling. What either side knows is known.
    let keep_known = !base.known.is_empty() || !other.known.is_empty();
    let mut known: Vec<bool> = (0..m.events.len()).map(|i| base.is_known(i)).collect();
    for (j, e) in other.events.iter().enumerate() {
        match m.events.iter().position(|x| x.year == e.year && x.text == e.text) {
            Some(i) => {
                let x = &mut m.events[i];
                if e.fame > x.fame {
                    x.told = e.told.clone();
                    x.fame = e.fame;
                }
                known[i] |= other.is_known(j);
            }
            None => {
                m.events.push(Event { year: e.year, text: e.text.clone(), told: e.told.clone(), fame: e.fame, bard: e.bard });
                known.push(other.is_known(j));
            }
        }
    }
    m.known = if keep_known { known } else { Vec::new() };

    // Sagas, by title and teller; the fuller telling wins.
    for s in &other.sagas {
        match m.sagas.iter_mut().find(|x| x.title == s.title && x.fort == s.fort) {
            Some(x) => {
                if s.chapters.len() > x.chapters.len() {
                    *x = s.clone();
                }
            }
            None => m.sagas.push(s.clone()),
        }
    }

    // Fallen forts, by their save: a fall is kept, and every robbery.
    for f in &other.forts {
        let mut f = f.clone();
        if let Some(crate::overworld::Occupant::Beast { figure }) = f.occupant.as_mut() {
            *figure = fig(*figure);
        }
        match m.forts.iter_mut().find(|x| x.save_file == f.save_file) {
            Some(x) => {
                if x.fell.is_none() {
                    x.fell = f.fell;
                    x.killer = f.killer.clone();
                }
                if x.occupant.is_none() {
                    x.occupant = f.occupant.clone();
                }
                for r in f.robbed {
                    if !x.robbed.contains(&r) {
                        x.robbed.push(r);
                    }
                }
                x.robbed.sort_by_key(|r| r.year);
                x.peak_pop = x.peak_pop.max(f.peak_pop);
                if f.journal.len() > x.journal.len() {
                    x.journal = f.journal;
                }
            }
            None => m.forts.push(f),
        }
    }

    // The bones of the dead, where they lie.
    for r in &other.remains {
        let mut r = r.clone();
        r.figure = fig(r.figure);
        if !m.remains.iter().any(|x| x.figure == r.figure && x.x == r.x && x.y == r.y) {
            m.remains.push(r);
        }
    }

    // A ruin stays a ruin.
    for s in &other.sites {
        if let Some(x) = m.sites.iter_mut().find(|x| x.id == s.id && x.name == s.name) {
            x.ruined |= s.ruined;
        }
    }
    // A place founded since (M4 B17: the world turns in a fort) stands.
    for s in other.sites.iter().skip(m.sites.len()) {
        if s.id as usize == m.sites.len() && s.x < m.w && s.y < m.h && m.site_at(s.x, s.y).is_none() {
            let cell = (s.y * m.w + s.x) as usize;
            m.cells[cell].site = s.id + 1;
            m.sites.push(s.clone());
        }
    }

    // The ages: each by its first year (the base's name where both have one).
    for age in &other.ages {
        if !m.ages.iter().any(|x| x.from == age.from) {
            m.ages.push(age.clone());
        }
    }
    m.ages.sort_by_key(|x| x.from);
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overworld::{Figure, Remains, Robbery};

    fn tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("dwarfclone-world-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn world() -> Overworld {
        static W: std::sync::OnceLock<Overworld> = std::sync::OnceLock::new();
        W.get_or_init(|| crate::game::Game::new(5).overworld).clone()
    }

    fn mark(name: &str, save: &str, robbed: Vec<Robbery>) -> FortMark {
        FortMark { name: name.into(), site: (3, 4), civ: None, founded: 100, fell: Some(104), killer: Some("goblin".into()), artifacts: vec!["the Axe of Dawn".into()], save_file: save.into(), peak_pop: 12, journal: Vec::new(), occupant: None, robbed }
    }

    fn hero(name: &str, died: Option<i32>) -> Figure {
        Figure { id: 0, name: name.into(), kind: "dwarf adventurer".into(), civ: None, born: 90, died, cause: died.map(|_| "was slain".to_string()), title: None, kills: 3, lair: None }
    }

    /// A19 step 1: two copies of one world, each given its own legend, fort
    /// mark and hero: the merge holds both; a figure dead in one is dead in
    /// the merge; and an event both know keeps the taller telling.
    #[test]
    fn two_games_in_one_world_both_are_remembered() {
        let mut a = world();
        let mut b = world();
        assert!(!a.events.is_empty() && !a.figures.is_empty());
        a.log(a.year, "Urist Coppergate dug too deep.".into());
        b.log(b.year + 2, "Domas Stonefist slew the dragon of the Ash Hills.".into());
        b.year += 2;
        a.forts.push(mark("Copper Gate", "CopperGate.json", vec![Robbery { by: "Domas".into(), year: 210, took: vec!["a ring".into()] }]));
        b.forts.push(mark("Copper Gate", "CopperGate.json", vec![Robbery { by: "Kel".into(), year: 211, took: vec!["the Axe of Dawn".into()] }]));
        b.forts.push(mark("Iron Hollow", "IronHollow.json", Vec::new()));
        // A hero who lives in one game and died in the other; and two heroes
        // who took the same new id in their own games.
        b.push_figure(hero("Asob Inkfang", None));
        let ha = a.push_figure(hero("Domas Stonefist", None));
        let hb = b.push_figure(hero("Domas Stonefist", Some(212)));
        let kel = a.push_figure(hero("Kel Ashfoot", Some(209)));
        assert_eq!((ha + 1, kel), (hb, hb));
        a.remains.push(Remains { name: "Kel Ashfoot".into(), x: 5, y: 6, site: None, pos: (1, 2, 3), year: 209, gear: Vec::new(), figure: kel });
        // A first figure of history died in one game only.
        b.figures[0].died = Some(b.year);
        // A tale that grew in the telling in one game.
        a.events[0].told = Some("a taller tale".into());
        a.events[0].fame = 2;
        // A ruin in one.
        let site = a.sites.iter().position(|s| !s.ruined).unwrap();
        a.sites[site].ruined = true;
        let n = world().events.len();

        for m in [merge(&a, &b), merge(&b, &a)] {
            assert_eq!(m.year, b.year);
            assert!(m.events.iter().any(|e| e.text.contains("dug too deep")));
            assert!(m.events.iter().any(|e| e.text.contains("slew the dragon")));
            assert_eq!(m.events.len(), n + 2, "shared history is not doubled");
            assert_eq!(m.events[0].told.as_deref(), Some("a taller tale"));
            let domas: Vec<&Figure> = m.figures.iter().filter(|f| f.name == "Domas Stonefist").collect();
            assert_eq!(domas.len(), 1, "one hero, not two");
            assert_eq!(domas[0].died, Some(212), "a death wins");
            assert!(m.figures[0].died.is_some(), "a death wins");
            let k = m.figures.iter().find(|f| f.name == "Kel Ashfoot").expect("both heroes");
            assert_eq!(k.id as usize, m.figures.iter().position(|f| f.id == k.id).unwrap(), "ids stay indices");
            assert_eq!(m.remains.iter().find(|r| r.name == "Kel Ashfoot").map(|r| r.figure), Some(k.id), "the bones follow the new id");
            assert_eq!(m.forts.len(), 2);
            let cg = m.forts.iter().find(|f| f.save_file == "CopperGate.json").unwrap();
            assert_eq!(cg.robbed.iter().map(|r| r.by.as_str()).collect::<Vec<_>>(), ["Domas", "Kel"], "every robbery");
            assert!(m.sites[site].ruined);
        }
    }

    /// Recording twice from two games keeps both in the file, and a third
    /// game in the same world hears both on `refresh`.
    #[test]
    fn the_file_merges_and_a_new_game_hears_it() {
        let dir = tmp("record");
        let mut a = world();
        let mut b = world();
        a.log(a.year, "Urist Coppergate dug too deep.".into());
        b.log(b.year, "Domas Stonefist slew the dragon.".into());
        let file = record(&dir, &a).unwrap();
        assert_eq!(record(&dir, &b).unwrap(), file, "one world, one file");
        assert!(!dir.join(format!("{}.tmp", file)).exists());
        let worlds = saves::list_worlds(&dir);
        assert_eq!(worlds.len(), 1);
        assert_eq!(worlds[0].world, a.name);
        let mut c = world();
        refresh(&dir, &mut c);
        assert!(c.events.iter().any(|e| e.text.contains("dug too deep")));
        assert!(c.events.iter().any(|e| e.text.contains("slew the dragon")));
        // Another world of the same name keeps its own file.
        let mut other = world();
        other.seed ^= 1;
        let f2 = record(&dir, &other).unwrap();
        assert_ne!(f2, file);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
