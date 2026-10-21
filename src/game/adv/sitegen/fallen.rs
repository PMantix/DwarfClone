//! A fallen fort: the fort's own save is the site (A5). Entering one loads
//! it, halls, stores, artifacts, dead and all, and on the first visit makes
//! it a ruin (`ruin_prepare`): the bodies rot where they fell, fires are
//! out, and whatever killed the fort lives there now. The saga that ended
//! it is told on arrival; the overseer's journal, the chronicle and a tablet
//! of the fall lie where they can be found and read (`R`), and the walls
//! still tell the fort's history.
//!
//! Looting changes the world (decision 3, plan/adventure.md): leaving a
//! fort the hero took from, or killed in, writes the visit back to the
//! fort's own save, after a one-time backup of the player's original
//! (`<save>.pre-adventure.json`, never overwritten), and the ruin remembers
//! who robbed it (`Robbery`, on its `FortMark` in both worlds).
//!
//! Time: a fort keeps its own dating (`start_year`, its ticks), so its
//! annals, sagas and journal read as they did. The visit's clock is the
//! adventure's moved on by whole years (`offset`), so the hour is the same.
//!
//! Engravings point into their world's legends by index; the fort's own
//! world is set aside while the adventure's is in the visit, so they are
//! pointed at the same legends there (`remap_engravings`) and back again on
//! writing.

use super::super::items::carry_in;
use super::super::{Adventure, SiteKey, Visit, HERO_ID};
use super::{arm, reachable, Ctx, Made};
use crate::creature::{Creature, Species};
use crate::game::combat::{faction, Faction};
use crate::game::detail::{ENGRAVED, GEM_SET};
use crate::game::sagas::Beat;
use crate::game::spotlight::Spotlight;
use crate::game::{Game, SEASON, YEAR};
use crate::geom::Pos;
use crate::items::{Item, ItemKind, ARTIFACT};
use crate::material::Mat;
use crate::overworld::{FortMark, Occupant, Overworld, Race, Robbery, Site, SiteKind};
use crate::rng::Rng;
use crate::world::World;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// The books left in a fort, by their labels.
pub const JOURNAL: &str = "the overseer's journal";
const CHRONICLE: &str = "the chronicle of ";
const TABLET: &str = "a tablet: ";
/// `Visit.placed`'s mark of the first id the hero's things were given:
/// a fort's own things have smaller ids.
const FIRST: &str = "^";

/// The land of a fort's cell, for a fort whose save is gone (then `make`
/// lays a plain ruin on it).
pub fn build(adv: &Adventure, key: SiteKey, seed: u64) -> Result<(World, Pos), String> {
    Ok(super::wild::build(adv, key, seed))
}

/// A fort whose save is gone: the shell of a fortress, as any ruin.
pub fn make(g: &mut Game, ctx: &Ctx) -> Made {
    let SiteKey::Fort(i) = ctx.key else { return Made::default() };
    let Some(m) = g.overworld.forts.get(i as usize).cloned() else { return Made::default() };
    let site = Site { id: u32::MAX, name: m.name.clone(), kind: SiteKind::Fortress, civ: m.civ, x: m.site.0, y: m.site.1, pop: m.peak_pop as u32, ruined: true, founded: m.founded };
    let mut made = super::ruin::make(g, ctx, &site, true);
    made.title = format!("The ruin of {}", m.name);
    made.story.insert(0, format!("Nothing is left of {} but the gate.", m.name));
    made
}

/// Where the fallen forts' saves are.
pub fn saves_dir(adv: &Adventure) -> PathBuf {
    adv.saves_dir.clone().unwrap_or_else(|| PathBuf::from(crate::app::SAVE_DIR))
}

/// The backup of a fort's save, made before the first time an adventure
/// writes it: "Kogsak.json" -> "Kogsak.pre-adventure.json".
pub fn backup_file(save: &str) -> String {
    format!("{}.pre-adventure.json", save.trim_end_matches(".json"))
}

// ---------------------------------------------------------------- finding

/// Fallen forts of this world saved before `FortMark`s existed: read the
/// saves folder and add their marks. Runs once, when an adventure begins.
pub fn scan(dir: &Path, ow: &mut Overworld) -> usize {
    let mut n = 0;
    for m in crate::saves::list(dir) {
        if m.kind != crate::saves::SaveKind::Fort || m.autosave || m.world != ow.name || ow.forts.iter().any(|f| f.save_file == m.file) {
            continue;
        }
        let Ok(mut g) = crate::saves::read(dir, &m.file, &|_, _| {}) else { continue };
        if !(g.fallen || (!g.dwarves.is_empty() && g.alive_count() == 0)) {
            continue;
        }
        let killer = g.annals.deaths.last().and_then(|d| d.killer.clone());
        g.mark_fallen(killer);
        if let Some(mut mark) = g.overworld.forts.iter().rev().find(|f| f.name == g.fortress_name).cloned() {
            mark.save_file = m.file.clone();
            ow.forts.push(mark);
            n += 1;
        }
    }
    n
}

// ---------------------------------------------------------------- entering

/// Whole years from a fort's dating to the adventure's, in ticks.
fn offset(adv_start: i32, fort_start: i32) -> u64 {
    (adv_start - fort_start).max(0) as u64 * YEAR
}

/// The adventure's clock for a visit's game (a fort's runs on its own dating).
pub fn world_clock(v: &Visit, adv_start: i32) -> u64 {
    match v.key {
        SiteKey::Fort(_) if v.placed.iter().any(|(_, k)| k == FIRST) => v.g.tick.saturating_sub(offset(adv_start, v.g.start_year)),
        _ => v.g.tick,
    }
}

/// Go into fallen fort `i`, from its save. None when the save is gone.
pub fn enter(adv: &mut Adventure, i: u32, from: Option<(i32, i32)>) -> Option<Result<(), String>> {
    let mark = adv.ow.forts.get(i as usize)?.clone();
    let dir = saves_dir(adv);
    if !dir.join(&mark.save_file).exists() {
        return None;
    }
    Some(enter_save(adv, i, &mark, &dir, from))
}

fn enter_save(adv: &mut Adventure, i: u32, mark: &FortMark, dir: &Path, from: Option<(i32, i32)>) -> Result<(), String> {
    let key = SiteKey::Fort(i);
    let mut g = crate::saves::read(dir, &mark.save_file, &|_, _| {})?;
    // Its own world goes aside (it is read again on writing back), and the
    // adventure's comes in.
    let own = std::mem::replace(&mut g.overworld, std::mem::replace(&mut adv.ow, Overworld::empty()));
    // Prepared already: an adventurer wrote it back.
    let first = own.forts.iter().rev().find(|f| f.save_file == mark.save_file).map_or(true, |f| f.robbed.is_empty());
    remap_engravings(&mut g.world, &own, &mut g.overworld);
    drop(own);
    // The adventure's hour, in the fort's dating.
    let last = g.tick;
    g.tick = g.tick.max(adv.clock + offset(adv.start_year, g.start_year));
    g.weather = adv.weather;
    g.spotlights.clear();
    let seed = super::super::sites::site_seed(adv.seed, key);
    let band = adv.knob("fallen.band", 4.0).max(1.0) as usize;
    let elapsed = g.tick - last;
    let occupant = if first { ruin_prepare(&mut g, mark, elapsed, seed, band) } else { mark.occupant.clone().unwrap_or(Occupant::Empty) };
    // The journal as the overseer left it, and what lives here now.
    let journal = if g.journal_last_page().is_empty() { mark.journal.clone() } else { g.journal_last_page() };
    if let Some(m) = g.overworld.forts.get_mut(i as usize) {
        m.journal = journal;
        m.occupant = Some(occupant.clone());
    }
    let mut made = Made::default();
    foes(&g, &mut made);
    // The hero comes in at the old wagon spot.
    let arrive = nearest_open(&g, g.embark);
    let at = match from {
        Some((dx, dy)) => super::super::sites::edge_spot(&g.world, arrive, dx, dy),
        None => arrive,
    };
    made.placed.push((g.next_id, FIRST.to_string()));
    let hero = carry_in(&mut g, &mut adv.hero, at);
    for c in adv.company.iter_mut() {
        let p = g.spot_near(at, 2);
        carry_in(&mut g, c, p);
    }
    // The arrival: its story, and the saga that ended it the first time.
    let again = adv.memos.get(&key).map_or(false, |m| m.visited_tick > 0);
    let now = g.start_year + (g.tick / YEAR) as i32;
    let ours = adv.hero.civ.is_some() && adv.hero.civ == mark.civ;
    let mark_now = g.overworld.forts.get(i as usize).cloned().unwrap_or_else(|| mark.clone());
    let (title, story) = story(&g, &mark_now, now, &occupant, ours);
    let color = (220, 90, 110);
    if again {
        g.msg(&format!("You return to the fallen halls of {}.", mark.name), color);
    }
    for (k, line) in story.iter().enumerate() {
        g.msg(line, if k == 0 { color } else { (200, 190, 170) });
    }
    if !again {
        if let Some(id) = g.sagas.fell_by.filter(|id| g.saga(*id).is_some()) {
            g.spotlights.push(Spotlight { title: title.clone(), text: story.join(" "), at: Some(at), color, saga: Some(id), actions: Vec::new() });
        }
    }
    adv.at = mark.site;
    adv.visit = Some(Box::new(Visit { key, g, hero, placed: made.placed }));
    Ok(())
}

/// Open ground a hero can stand on, nearest `p`.
fn nearest_open(g: &Game, p: Pos) -> Pos {
    if g.world.in_bounds(p) && g.world.walkable(p) {
        return p;
    }
    for r in 1..12 {
        for dz in [0, 1, -1] {
            for dy in -r..=r {
                for dx in -r..=r {
                    let q = p.offset(dx, dy, dz);
                    if g.world.in_bounds(q) && g.world.walkable(q) {
                        return q;
                    }
                }
            }
        }
    }
    p
}

/// Whoever holds the ruin, remembered so the visit knows whom the hero
/// slew (the land's own beasts roam as they will, and don't count).
fn foes(g: &Game, made: &mut Made) {
    for c in g.creatures.values().filter(|c| holds(c)) {
        let name = c.label.clone().unwrap_or_else(|| c.species.info().name.to_string());
        made.placed.push((c.id, format!("@{}", name)));
    }
}

/// One of those who hold the ruin: a siege that never left, a beast of
/// legend, the dead, or what moved in since.
fn holds(c: &Creature) -> bool {
    c.alive && !c.tame && c.pet_of.is_none() && (c.siege || c.figure.is_some() || matches!(faction(c), Faction::Monster | Faction::Dead | Faction::Army(_)))
}

/// The arrival card: "Copper Gate. It fell in 213, to the goblins. Seven
/// years of silence."
fn story(g: &Game, m: &FortMark, now: i32, occupant: &Occupant, ours: bool) -> (String, Vec<String>) {
    let title = format!("The fallen halls of {}", m.name);
    let fell = m.fell.unwrap_or(now);
    let to = match (occupant, &m.killer) {
        (Occupant::Beast { figure }, _) => g.overworld.figures.get(*figure as usize).map_or("to a beast of legend".to_string(), |f| format!("to {} the {}", f.name, f.kind)),
        (_, Some(k)) => match species_named(k) {
            Some(sp) if sp.person() || matches!(sp, Species::Troglodyte) => format!("to the {}", sp.info().plural),
            _ => format!("to {}", super::lore::a_or_an(k)),
        },
        (_, None) => "when the last of them died".to_string(),
    };
    let mut v = vec![format!("{}. It fell in {}, {}.", m.name, fell, to)];
    v.push(match now - fell {
        n if n <= 0 => "Its silence is new: not a year has passed.".to_string(),
        1 => "One year of silence.".to_string(),
        n => format!("{} years of silence.", cap(&words(n))),
    });
    if ours {
        v.push("Your own people cut these halls.".to_string());
    }
    if m.peak_pop > 0 {
        v.push(format!("{} lived here, and died here.", if m.peak_pop == 1 { "One soul".to_string() } else { format!("{} souls", cap(&words(m.peak_pop as i32))) }));
    }
    if let Some(r) = m.robbed.last() {
        v.push(match r.took.first() {
            Some(t) => format!("{} took {} from it in {}.", r.by, t, r.year),
            None => format!("{} came this way in {}.", r.by, r.year),
        });
    }
    v.push(
        match occupant {
            Occupant::Beast { .. } => "What killed it has made the place its own.",
            Occupant::Army { .. } => "Its killers still keep its halls.",
            Occupant::Horror { .. } => "Something has come up from below.",
            Occupant::Dead => "The dead do not rest here.",
            Occupant::Empty => "Nothing stirs but the dust.",
        }
        .to_string(),
    );
    (title, v)
}

// ---------------------------------------------------------------- the ruin

/// Make a fallen fort's game a ruin, the first time an adventurer comes:
/// the last of them dead, the fort's business over, fires out, the bodies
/// rotted to bones once a season has passed (`elapsed` ticks since it
/// fell), whatever killed it in residence, and its books left lying.
/// Returns what lives in it now. Draws only from its own seeded dice.
pub fn ruin_prepare(g: &mut Game, mark: &FortMark, elapsed: u64, seed: u64, band: usize) -> Occupant {
    let mut rng = Rng::new(crate::game::names::hash(&[seed, 0xFA11]));
    // Anyone still breathing (a save from before the end) dies alone.
    for i in 0..g.dwarves.len() {
        if g.dwarves[i].alive && g.dwarves[i].id < HERO_ID {
            let mut d = std::mem::take(&mut g.dwarves[i]);
            g.kill(&mut d, "perished alone in the silence");
            g.dwarves[i] = d;
        }
    }
    // The fort's business is over.
    g.squads.clear();
    g.alert = false;
    g.civ_alert = false;
    g.caravan = None;
    g.captives.clear();
    g.incoming = None;
    g.festival = None;
    g.mandate = None;
    g.designations.clear();
    g.claimed.clear();
    g.reserved.clear();
    g.spotlights.clear();
    g.flames.clear();
    for t in g.world.tiles.iter_mut() {
        t.fire = 0;
    }
    g.fire_cells = None;
    // Where things are: the halls the hero can walk to from the wagon spot.
    let gate = nearest_open(g, g.embark);
    let reach = reachable(&g.world, gate, 30_000);
    let hall = main_hall(g, &reach);
    // The killers that never left stay, and hunt whoever comes.
    let mut stayed: Vec<u32> = g.creatures.values().filter(|c| holds(c)).map(|c| c.id).collect();
    stayed.sort();
    for id in &stayed {
        if let Some(c) = g.creatures.get_mut(id) {
            c.leave_tick = u64::MAX / 2;
            c.leaving = false;
            c.target = Some(HERO_ID);
        }
    }
    let occupant = if let Some(c) = stayed.iter().filter_map(|id| g.creatures.get(id)).find(|c| c.figure.is_some()) {
        Occupant::Beast { figure: c.figure.unwrap_or(0) }
    } else if !stayed.is_empty() {
        let sp = g.creatures[&stayed[0]].species;
        Occupant::Army { civ: civ_of(&g.overworld, sp).unwrap_or(0), n: stayed.len() as u32 }
    } else {
        move_in(g, mark, &reach, hall, band, &mut rng)
    };
    // A season on, the dead are bones where they fell.
    if elapsed >= SEASON {
        let bodies: Vec<u32> = g.items.values().filter(|it| matches!(it.kind, ItemKind::Corpse | ItemKind::Carcass) && it.carried_by.is_none() && it.container.is_none()).map(|it| it.id).collect();
        for id in bodies {
            let Some(it) = g.items.get_mut(&id) else { continue };
            let who = it.label.clone();
            if it.kind == ItemKind::Corpse {
                it.kind = ItemKind::BodyPart;
                it.label = Some(format!("the bones of {}", who.unwrap_or_else(|| "a dwarf".into())));
            } else {
                it.kind = ItemKind::Bone;
            }
            it.mat = Mat::Bone;
            it.wear = 0;
        }
    }
    leave_books(g, mark, &reach, hall);
    occupant
}

/// The fort's heart: the overseer's base (or the wagon spot), where the
/// hero can walk to.
fn main_hall(g: &Game, reach: &[Pos]) -> Pos {
    let want = g.overseer.as_ref().and_then(|o| o.base).unwrap_or(g.embark);
    reach.iter().copied().min_by_key(|p| (p.x - want.x).abs() + (p.y - want.y).abs() + 3 * (p.z - want.z).abs()).unwrap_or(want)
}

/// Ground within `r` of `c` on its level, that the hero can walk to.
fn near(reach: &[Pos], c: Pos, r: i32) -> Vec<Pos> {
    reach.iter().copied().filter(|p| p.z == c.z && (p.x - c.x).abs() <= r && (p.y - c.y).abs() <= r && *p != c).collect()
}

/// A people's nearest civ, for an army's mark.
fn civ_of(ow: &Overworld, sp: Species) -> Option<u32> {
    let race = match sp {
        Species::Goblin => Race::Goblin,
        Species::Human => Race::Human,
        Species::Elf => Race::Elf,
        Species::Raider => Race::Dwarf,
        _ => return None,
    };
    ow.civs.iter().position(|c| c.race == race).map(|i| i as u32)
}

/// A creature kind by the name the annals give its killer ("goblin",
/// "troglodyte", "cave horror", "giant cave spider", a bestiary beast).
fn species_named(k: &str) -> Option<Species> {
    let k = k.to_lowercase();
    if k.is_empty() {
        return None;
    }
    const KINDS: [Species; 17] = [
        Species::Goblin,
        Species::Troglodyte,
        Species::Human,
        Species::Elf,
        Species::Raider,
        Species::Undead,
        Species::Husk,
        Species::CaveHorror,
        Species::GiantSpider,
        Species::Wolf,
        Species::Bear,
        Species::Jaguar,
        Species::Lion,
        Species::Alligator,
        Species::PolarBear,
        Species::Scorpion,
        Species::Viper,
    ];
    KINDS
        .iter()
        .copied()
        .find(|s| s.info().name == k || s.info().name.split(' ').next() == Some(k.as_str()))
        .or_else(|| (0..crate::bestiary::BEASTS.len() as u16).map(Species::Beast).find(|s| s.info().name == k))
}

/// Whatever killed the fort moves in: a beast of legend makes it its lair,
/// an army camps in the great hall, a horror nests in the deep; with no
/// killer the dead rise (on evil land) or cave things come up.
fn move_in(g: &mut Game, mark: &FortMark, reach: &[Pos], hall: Pos, band: usize, rng: &mut Rng) -> Occupant {
    let killer = mark.killer.clone().unwrap_or_default().to_lowercase();
    // A beast of legend of that kind, still living: the nearest.
    let (sx, sy) = mark.site;
    let figure = if killer.is_empty() {
        None
    } else {
        let ow = &g.overworld;
        let dist = |f: &crate::overworld::Figure| f.lair.and_then(|l| ow.sites.iter().find(|s| s.id == l)).map_or(i32::MAX / 2, |s| (s.x - sx).abs().max((s.y - sy).abs()));
        ow.figures.iter().filter(|f| f.alive() && f.kind.to_lowercase() == killer && f.civ.is_none()).min_by_key(|f| dist(f)).map(|f| (f.id, f.name.clone(), f.kind.clone()))
    };
    let spots = |c: Pos, r: i32| {
        let v = near(reach, c, r);
        if v.is_empty() {
            vec![c]
        } else {
            v
        }
    };
    if let Some((id, name, kind)) = figure {
        let at = *rng.pick(&spots(hall, 4));
        let cid = g.spawn_creature(Species::Megabeast, at, u64::MAX / 2);
        if let Some(c) = g.creatures.get_mut(&cid) {
            c.figure = Some(id);
            c.label = Some(format!("{} the {}", name, kind));
            c.breathes_fire = kind == "dragon";
            c.siege = true;
            c.target = Some(HERO_ID);
        }
        return Occupant::Beast { figure: id };
    }
    let deep = reach.iter().copied().min_by_key(|p| (p.z, (p.x - hall.x).abs() + (p.y - hall.y).abs())).unwrap_or(hall);
    match species_named(&killer) {
        Some(Species::CaveHorror) => {
            let cid = g.make_cave_horror(deep);
            if let Some(c) = g.creatures.get_mut(&cid) {
                c.leave_tick = u64::MAX / 2;
                c.target = Some(HERO_ID);
            }
            let label = g.creatures.get(&cid).and_then(|c| c.label.clone()).unwrap_or_else(|| "a cave horror".into());
            Occupant::Horror { label }
        }
        Some(sp) if sp != Species::Undead && sp != Species::Husk => {
            let n = if sp.person() { band } else { band.div_ceil(2) };
            let at = spots(hall, 6);
            for _ in 0..n {
                let p = *rng.pick(&at);
                let cid = spawn_foe(g, sp, p);
                if sp.person() {
                    let (kind, mat) = *rng.pick(&[(ItemKind::Spear, Mat::Iron), (ItemKind::Sword, Mat::Copper), (ItemKind::Axe, Mat::Iron)]);
                    arm(g, cid, kind, mat);
                }
            }
            match civ_of(&g.overworld, sp) {
                Some(civ) => Occupant::Army { civ, n: n as u32 },
                None => Occupant::Horror { label: sp.info().plural.to_string() },
            }
        }
        sp if g.evil || sp.is_some() => {
            // The unburied rise where they lay.
            let mut bodies: Vec<(Pos, Option<String>, u32)> = g.items.values().filter(|it| it.kind == ItemKind::Corpse && it.carried_by.is_none() && it.container.is_none()).map(|it| (it.pos, it.label.clone(), it.id)).collect();
            bodies.truncate(band);
            if bodies.is_empty() {
                bodies.push((*rng.pick(&spots(hall, 6)), None, 0));
            }
            for (p, who, id) in bodies {
                g.items.remove(&id);
                let cid = spawn_foe(g, Species::Undead, p);
                if let (Some(c), Some(w)) = (g.creatures.get_mut(&cid), who) {
                    c.label = Some(format!("the risen corpse of {}", w));
                }
            }
            Occupant::Dead
        }
        _ => {
            // Cave things come up into the empty halls.
            let at = spots(deep, 6);
            let sp = if g.world.caverns.is_empty() { Species::Wolf } else { Species::Troglodyte };
            for _ in 0..band.div_ceil(2) {
                let p = *rng.pick(&at);
                spawn_foe(g, sp, p);
            }
            Occupant::Empty
        }
    }
}

/// One who lives here now and comes for whoever enters.
fn spawn_foe(g: &mut Game, sp: Species, at: Pos) -> u32 {
    let id = match sp {
        Species::Beast(_) => g.spawn_beast(sp, at, u64::MAX / 2),
        _ => g.spawn_creature(sp, at, u64::MAX / 2),
    };
    if let Some(c) = g.creatures.get_mut(&id) {
        c.siege = true;
        c.target = Some(HERO_ID);
    }
    id
}

/// The fort's books where they lay: the overseer's journal and the
/// chronicle on the nearest table to the main hall, and the tablet of its
/// fall beside the last of them to die.
fn leave_books(g: &mut Game, mark: &FortMark, reach: &[Pos], hall: Pos) {
    let walk: HashSet<Pos> = reach.iter().copied().collect();
    let table = g
        .buildings
        .values()
        .filter(|b| b.kind == crate::buildings::BuildingKind::Table && b.built && walk.contains(&b.pos))
        .map(|b| b.pos)
        .min_by_key(|p| (p.x - hall.x).abs() + (p.y - hall.y).abs() + 4 * (p.z - hall.z).abs())
        .unwrap_or(hall);
    if !g.journal_last_page().is_empty() || !mark.journal.is_empty() {
        book(g, table, Mat::Leather, JOURNAL.to_string());
    }
    if !g.chronicles.is_empty() {
        let label = format!("{}{}", CHRONICLE, g.fortress_name);
        let at = near(reach, table, 1).first().copied().unwrap_or(table);
        book(g, at, Mat::Leather, label);
    }
    if let Some(s) = g.sagas.fell_by.and_then(|id| g.saga(id)) {
        let label = format!("{}{}", TABLET, s.title);
        let last = g.annals.deaths.last().map(|d| d.name.clone());
        let by = last.and_then(|n| {
            let theirs = format!("the bones of {}", n);
            g.items.values().find(|it| it.carried_by.is_none() && it.label.as_deref().map_or(false, |l| l == n || l == theirs)).map(|it| it.pos)
        });
        book(g, by.unwrap_or(hall), Mat::Granite, label);
    }
}

fn book(g: &mut Game, at: Pos, mat: Mat, label: String) {
    let id = g.spawn_item(ItemKind::Product, mat, at);
    if let Some(it) = g.items.get_mut(&id) {
        it.label = Some(label);
    }
}

// ---------------------------------------------------------------- reading

/// Something to read, as a page: a saga, a journal, a chronicle, the walls.
#[derive(Clone, Debug, PartialEq)]
pub struct Writing {
    pub title: String,
    /// "a saga of a last stand · Kogsak, 203".
    pub sub: String,
    pub doom: bool,
    /// (date, beat, text).
    pub chapters: Vec<(String, Beat, String)>,
    pub closing: String,
    /// It tells the end of its fort.
    pub fell: bool,
}

/// A saga of this fort as a page.
pub fn saga_writing(g: &Game, id: u32) -> Option<Writing> {
    let s = g.saga(id)?;
    let year = g.start_year + (s.chapters.first().map_or(g.tick, |c| c.tick) / YEAR) as i32;
    Some(Writing {
        title: s.title.clone(),
        sub: format!("a saga of {} · {}, {}", s.kind.name(), g.fortress_name, year),
        doom: s.doom || s.open(),
        chapters: s.chapters.iter().map(|c| (g.short_date(c.tick).trim().to_string(), c.beat, c.text.clone())).collect(),
        closing: if s.open() { "The saga breaks off here.".to_string() } else { s.closing.clone() },
        fell: g.sagas.fell_by == Some(id),
    })
}

/// The saga told on arrival in a fallen fort, if one is waiting.
pub fn take_reading(g: &mut Game) -> Option<Writing> {
    let i = g.spotlights.iter().position(|s| s.saga.is_some())?;
    let s = g.spotlights.remove(i);
    saga_writing(g, s.saga?)
}

/// What a book of the fort says.
pub fn writing(g: &Game, it: &Item) -> Option<Writing> {
    if it.kind != ItemKind::Product {
        return None;
    }
    let label = it.label.as_deref()?;
    if label == JOURNAL {
        return Some(journal(g));
    }
    if label.starts_with(CHRONICLE) {
        let mut chapters = Vec::new();
        let n = g.chronicles.len();
        for c in &g.chronicles[n.saturating_sub(3)..] {
            chapters.push((format!("{}", c.year), Beat::Cause, c.title.clone()));
            for l in c.lines.iter().take(8) {
                chapters.push((String::new(), Beat::Escalation, l.clone()));
            }
        }
        return Some(Writing { title: format!("The Chronicle of {}", g.fortress_name), sub: "kept year by year".into(), doom: false, chapters, closing: "The last pages are blank.".into(), fell: false });
    }
    if let Some(title) = label.strip_prefix(TABLET) {
        let id = g.sagas.list.iter().find(|s| s.title == title).map(|s| s.id)?;
        return saga_writing(g, id);
    }
    None
}

/// The overseer's journal, open at its last page.
fn journal(g: &Game) -> Writing {
    use crate::game::journal::EntryKind;
    match g.overseer.as_ref().and_then(|o| o.journal.last()) {
        Some(p) => {
            let written: Vec<_> = p.entries.iter().filter(|e| e.kind != EntryKind::Jot).collect();
            let chapters = written[written.len().saturating_sub(10)..]
                .iter()
                .map(|e| {
                    let beat = match e.kind {
                        EntryKind::Dead => Beat::Turn,
                        EntryKind::Fear | EntryKind::Secret => Beat::Escalation,
                        _ => Beat::Cause,
                    };
                    (g.short_date(e.at).trim().to_string(), beat, e.text.clone())
                })
                .collect();
            Writing {
                title: format!("The Journal of {}", p.styled()),
                sub: format!("overseer of {} · the last page", g.fortress_name),
                doom: true,
                chapters,
                closing: "Here the hand ends.".into(),
                fell: true,
            }
        }
        None => {
            let lines = g.overworld.forts.iter().rev().find(|f| f.name == g.fortress_name).map(|f| f.journal.clone()).unwrap_or_default();
            Writing {
                title: lines.first().cloned().unwrap_or_else(|| "The Overseer's Journal".into()),
                sub: format!("{} · the last page", g.fortress_name),
                doom: true,
                chapters: lines.iter().skip(1).map(|l| (String::new(), Beat::Cause, l.clone())).collect(),
                closing: "Here the hand ends.".into(),
                fell: true,
            }
        }
    }
}

/// `R`: read a book underfoot or carried, or else the engravings on the
/// walls around.
pub fn read_here(g: &Game, hero: usize) -> Result<Writing, String> {
    let d = &g.dwarves[hero];
    let mut books: Vec<&Item> = g.items.values().filter(|it| it.carried_by == Some(d.id) || (it.carried_by.is_none() && it.container.is_none() && it.pos == d.pos)).filter(|it| writing(g, it).is_some()).collect();
    books.sort_by_key(|it| (it.carried_by.is_some(), it.id));
    if let Some(w) = books.first().and_then(|it| writing(g, it)) {
        return Ok(w);
    }
    let mut chapters = Vec::new();
    let mut seen = HashSet::new();
    for dy in -1..=1 {
        for dx in -1..=1 {
            let p = d.pos.offset(dx, dy, 0);
            if let Some((q, year, text)) = g.engraving_at(p) {
                if seen.insert(text.clone()) {
                    let qual = crate::items::quality_prefix(q).trim().to_string();
                    let beat = if q >= 4 { Beat::Turn } else { Beat::Escalation };
                    chapters.push((if qual.is_empty() { format!("{}", year) } else { format!("{} · {}", year, qual) }, beat, text));
                }
            }
        }
    }
    if chapters.is_empty() {
        return Err("There is nothing here to read.".into());
    }
    Ok(Writing { title: "Carved in the Stone".into(), sub: format!("the walls of {}", g.fortress_name), doom: false, chapters, closing: "The stone remembers.".into(), fell: false })
}

// ---------------------------------------------------------------- leaving

/// What the hero did in a fallen fort: what they carry out of it, and whom
/// they slew there.
#[derive(Clone, Debug, Default)]
pub struct Spoils {
    /// The names of what was taken, the finest first.
    pub took: Vec<String>,
    /// The artifacts among them, by name.
    pub artifacts: Vec<String>,
    pub slain: Vec<String>,
    /// No one is left alive of those who lived here.
    pub cleared: bool,
}

/// Read what the hero takes from a fort as they leave (None for any other
/// site): call before they are carried out.
pub fn spoils(v: &Visit) -> Option<Spoils> {
    if !matches!(v.key, SiteKey::Fort(_)) {
        return None;
    }
    let first = v.placed.iter().find(|(_, k)| k == FIRST).map(|(id, _)| *id)?;
    let g = &v.g;
    let ours = |id: Option<u32>| id.map_or(false, |i| i >= HERO_ID);
    let mut taken: Vec<&Item> = g.items.values().filter(|it| it.id < first && ours(it.carried_by)).collect();
    taken.sort_by_key(|it| (std::cmp::Reverse(it.quality >= ARTIFACT), std::cmp::Reverse(it.value()), it.id));
    let mut sp = Spoils::default();
    for it in taken {
        let name = if it.quality >= ARTIFACT { it.label.clone().unwrap_or_else(|| it.name()) } else { super::lore::a_or_an(&it.name()) };
        if it.quality >= ARTIFACT {
            sp.artifacts.push(name.clone());
        }
        sp.took.push(name);
    }
    let mut any = false;
    let mut alive = false;
    for (id, key) in &v.placed {
        let Some(name) = key.strip_prefix('@') else { continue };
        any = true;
        if g.creatures.get(id).map_or(false, |c| c.alive) {
            alive = true;
        } else {
            sp.slain.push(name.to_string());
        }
    }
    sp.cleared = any && !alive;
    Some(sp)
}

/// "a, b and c".
fn list(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        n => format!("{} and {}", items[..n - 1].join(", "), items[n - 1]),
    }
}

/// Only the overworld of a fort's save.
#[derive(Deserialize)]
struct OwnWorld {
    overworld: Overworld,
}

/// The hero left a fort they took from or killed in (`v`, the hero
/// already out of it, the adventure's world back in `adv.ow`): the ruin
/// remembers it, in the legends, on its mark in both worlds, and in its
/// own save, written over the player's after a one-time backup.
pub fn write_back(adv: &mut Adventure, v: &mut Visit, sp: &Spoils) -> Result<(), String> {
    let SiteKey::Fort(i) = v.key else { return Ok(()) };
    let i = i as usize;
    if sp.took.is_empty() && sp.slain.is_empty() {
        return Ok(());
    }
    let Some(mark) = adv.ow.forts.get(i).cloned() else { return Ok(()) };
    let dir = saves_dir(adv);
    let path = dir.join(&mark.save_file);
    if !path.exists() {
        return Ok(());
    }
    // The player's own save, kept once and never written over.
    let backup = dir.join(backup_file(&mark.save_file));
    if !backup.exists() {
        std::fs::copy(&path, &backup).map_err(|e| format!("Could not keep a copy of {}: {}", mark.save_file, e))?;
    }
    let year = adv.year();
    let by = match adv.hero.home.and_then(|h| adv.ow.sites.iter().find(|s| s.id == h)) {
        Some(s) => format!("{} of {}", adv.hero.dwarf.called(), s.name),
        None => adv.hero.dwarf.called(),
    };
    let robbery = Robbery { by: by.clone(), year, took: sp.took.iter().take(6).cloned().collect() };
    let legend = if !sp.artifacts.is_empty() {
        format!("{} took {} from the fallen halls of {}.", by, list(&sp.artifacts), mark.name)
    } else if sp.cleared && !sp.slain.is_empty() {
        format!("{} slew {} in the fallen halls of {}.", by, list(&sp.slain.iter().take(3).cloned().collect::<Vec<_>>()), mark.name)
    } else if !sp.took.is_empty() {
        format!("{} plundered the fallen halls of {}.", by, mark.name)
    } else {
        format!("{} fought in the fallen halls of {}.", by, mark.name)
    };
    let update = |m: &mut FortMark| {
        m.robbed.push(robbery.clone());
        m.artifacts.retain(|a| !sp.artifacts.iter().any(|t| t == a));
        if sp.cleared {
            m.occupant = Some(Occupant::Empty);
        }
    };
    update(&mut adv.ow.forts[i]);
    adv.ow.log(year, legend.clone());
    // The fort's own world: only its mark and the legend change.
    let data = std::fs::read_to_string(&path).map_err(|e| format!("Could not read {}: {}", mark.save_file, e))?;
    let mut own = serde_json::from_str::<OwnWorld>(&data).map_err(|e| format!("Could not read {}: {}", mark.save_file, e))?.overworld;
    drop(data);
    let now = adv.ow.forts[i].clone();
    match own.forts.iter().rposition(|f| f.save_file == mark.save_file) {
        Some(k) => {
            let m = &mut own.forts[k];
            m.journal = now.journal.clone();
            m.robbed = now.robbed.clone();
            m.artifacts = now.artifacts.clone();
            m.occupant = now.occupant.clone();
        }
        None => own.forts.push(now),
    }
    own.log(year, legend);
    remap_engravings(&mut v.g.world, &adv.ow, &mut own);
    v.g.overworld = own;
    v.g.fallen = true;
    v.g.spotlights.clear();
    crate::saves::write(&dir, &v.g, false, &|_, _| {}).map(|_| ())
}

// ---------------------------------------------------------------- legends

/// Point every engraving in `w` from its legend in `from` to the same
/// legend in `to` (added there if `to` has never heard it).
pub fn remap_engravings(w: &mut World, from: &Overworld, to: &mut Overworld) {
    let mut map: HashMap<u32, u32> = HashMap::new();
    let mut index: Option<HashMap<(i32, String), usize>> = None;
    for t in w.tiles.iter_mut() {
        let old = t.engraving & !GEM_SET;
        if t.smooth < ENGRAVED || old == 0 {
            continue;
        }
        let new = *map.entry(old).or_insert_with(|| {
            let Some(e) = from.events.get(old as usize - 1) else { return old };
            if to.events.get(old as usize - 1).map_or(false, |f| f.year == e.year && f.text == e.text) {
                return old;
            }
            let ix = index.get_or_insert_with(|| to.events.iter().enumerate().map(|(i, e)| ((e.year, e.text.clone()), i)).collect());
            let i = match ix.get(&(e.year, e.text.clone())) {
                Some(&i) => i,
                None => {
                    to.events.push(e.clone());
                    ix.insert((e.year, e.text.clone()), to.events.len() - 1);
                    to.events.len() - 1
                }
            };
            i as u32 + 1
        });
        t.engraving = new | (t.engraving & GEM_SET);
    }
}

/// The window's `--fallen` picture: a fort of `seed` run `days` under its
/// overseer, every dwarf struck down by goblins, saved to `dir`; then a
/// hero of its world walks into its fallen halls and stands by the
/// overseer's journal.
pub fn demo(seed: u64, days: u64, dir: &Path) -> Result<Adventure, String> {
    let _ = std::fs::remove_dir_all(dir);
    let ow = Overworld::generate(seed);
    let (x, y) = ow.default_site();
    let mut g = Game::embark_with(seed, ow, x, y, crate::world::MapSize::ALL[1], &|_, _| {});
    g.set_overseer(Some(crate::game::Style::Steady));
    for _ in 0..days * crate::game::DAY {
        g.step();
    }
    for i in 0..g.dwarves.len() {
        let mut d = std::mem::take(&mut g.dwarves[i]);
        if d.alive {
            g.annals.wounded_by.push((d.id, "goblin".into(), g.tick));
            g.kill(&mut d, "was struck down");
        }
        g.dwarves[i] = d;
    }
    for _ in 0..10 {
        g.step();
    }
    g.fallen = true;
    crate::saves::write(dir, &g, false, &|_, _| {})?;
    let ow = std::mem::replace(&mut g.overworld, Overworld::empty());
    let i = ow.forts.len().saturating_sub(1) as u32;
    let mut a = super::super::quick_start(ow, seed);
    a.saves_dir = Some(dir.to_path_buf());
    a.enter(SiteKey::Fort(i))?;
    if let Some(v) = a.visit.as_mut() {
        let at = v.g.items.values().find(|it| it.label.as_deref() == Some(JOURNAL)).map(|it| it.pos);
        if let Some(p) = at {
            let h = v.hero;
            let q = reachable(&v.g.world, p, 9).into_iter().find(|q| *q != p).unwrap_or(p);
            v.g.dwarves[h].pos = q;
        }
    }
    Ok(a)
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

/// "seven", "twelve", "40".
fn words(n: i32) -> String {
    const W: [&str; 13] = ["no", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve"];
    W.get(n.max(0) as usize).map_or_else(|| n.to_string(), |s| s.to_string())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::super::super::tests::test_hero;
    use super::super::super::turn::{Action, Outcome, Target};
    use super::*;
    use crate::game::DAY;

    pub(crate) const PRIZE: &str = "the Ember of Dusk";

    /// A fresh folder for a test's saves.
    pub(crate) fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("dwarfclone-fallen-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    /// A fort run ten days under an overseer, with a work of legend made
    /// and the floor by it engraved with the fort's own story; then every
    /// dwarf struck down (by `killer`'s choice of foe, if any), the fort
    /// fallen and saved to `dir`. Its world.
    pub(crate) fn fallen_fort(seed: u64, killer: impl Fn(&Overworld) -> Option<String>, dir: &Path) -> Overworld {
        let mut g = Game::new(seed);
        g.overseer = Some(crate::game::overseer::Overseer::new(crate::game::overseer::Style::Steady));
        for _ in 0..DAY * 10 {
            g.step();
        }
        let spot = reachable(&g.world, nearest_open(&g, g.embark), 400).into_iter().rev().find(|p| g.world.walkable(*p)).unwrap_or(g.embark);
        let id = g.spawn_item(ItemKind::Craft, Mat::Gold, spot);
        let it = g.items.get_mut(&id).unwrap();
        it.quality = ARTIFACT;
        it.label = Some(PRIZE.to_string());
        let e = g.overworld.events.iter().rposition(|e| e.text.contains(&g.fortress_name)).expect("the fort's founding is a legend");
        let i = g.world.idx(spot);
        g.world.tiles[i].smooth = ENGRAVED + 2;
        g.world.tiles[i].engraving = e as u32 + 1;
        let k = killer(&g.overworld);
        for i in 0..g.dwarves.len() {
            let mut d = std::mem::take(&mut g.dwarves[i]);
            if d.alive {
                if let Some(k) = &k {
                    g.annals.wounded_by.push((d.id, k.clone(), g.tick));
                }
                g.kill_for_test(&mut d, "was struck down");
            }
            g.dwarves[i] = d;
        }
        for _ in 0..10 {
            g.step();
        }
        g.fallen = true;
        assert_eq!(g.overworld.forts.len(), 1, "the fall is marked");
        crate::saves::write(dir, &g, false, &|_, _| {}).unwrap();
        g.overworld
    }

    pub(crate) fn adventure(ow: Overworld, seed: u64, dir: &Path) -> Adventure {
        let mut a = Adventure::new(ow, seed, test_hero(seed), None);
        a.saves_dir = Some(dir.to_path_buf());
        a
    }

    fn log_text(g: &Game) -> String {
        g.log.iter().map(|e| e.text.clone()).collect::<Vec<_>>().join("\n")
    }

    /// Stand the hero on an item and pick it up.
    fn take(a: &mut Adventure, id: u32) {
        let v = a.visit.as_mut().unwrap();
        let h = v.hero;
        v.g.dwarves[h].pos = v.g.items[&id].pos;
        let out = v.g.act(h, Action::Get(vec![id]));
        assert!(!matches!(out, Outcome::Refused(_)), "{:?}", out);
        assert_eq!(v.g.items[&id].carried_by, Some(HERO_ID), "picked up");
    }

    fn prize(g: &Game) -> Option<u32> {
        g.items.values().find(|it| it.label.as_deref() == Some(PRIZE)).map(|it| it.id)
    }

    #[test]
    fn a_fallen_fort_is_found_and_entered() {
        let dir = tmp("found");
        let mut ow = fallen_fort(31, |_| Some("goblin".into()), &dir);
        let mark = ow.forts[0].clone();
        assert!(mark.artifacts.iter().any(|a| a == PRIZE));
        // A world from before the marks: the saves folder knows it.
        ow.forts.clear();
        assert_eq!(scan(&dir, &mut ow), 1);
        assert_eq!(scan(&dir, &mut ow), 0, "once");
        assert_eq!((ow.forts[0].name.as_str(), ow.forts[0].save_file.as_str(), ow.forts[0].site), (mark.name.as_str(), mark.save_file.as_str(), mark.site));
        let saved = crate::saves::read(&dir, &mark.save_file, &|_, _| {}).unwrap();
        let bytes = std::fs::read(dir.join(&mark.save_file)).unwrap();
        let mut a = adventure(ow, 31, &dir);
        assert!(a.start_year > mark.fell.unwrap(), "the adventure comes after the fall");
        a.enter(SiteKey::Fort(0)).unwrap();
        let v = a.visit.as_mut().unwrap();
        assert_eq!(v.g.fortress_name, saved.fortress_name);
        assert!(v.g.world.tiles == saved.world.tiles, "its own halls");
        assert!(prize(&v.g).is_some(), "its artifact");
        assert_eq!(v.g.dwarves.iter().filter(|d| !d.alive).count(), saved.dwarves.len(), "its dead");
        assert_eq!(v.g.dwarves[v.hero].id, HERO_ID);
        let text = log_text(&v.g);
        assert!(text.contains(&format!("{}. It fell in {}", mark.name, mark.fell.unwrap())), "{}", text);
        assert!(text.contains("goblins"), "{}", text);
        let w = take_reading(&mut v.g).expect("the saga of its fall is told");
        assert!(w.fell && !w.chapters.is_empty());
        assert_eq!(a.at, mark.site);
        // The clock runs on in the fort's dating, and comes back out whole.
        let y0 = a.year();
        let v = a.visit.as_mut().unwrap();
        let (h, t0) = (v.hero, v.g.tick);
        v.g.act(h, Action::Wait(10));
        let spent = v.g.tick - t0;
        assert_eq!(a.year(), y0);
        let c0 = a.clock;
        a.leave();
        assert_eq!(a.clock, c0 + spent);
        // Nothing taken, nothing slain: the save is as it was.
        assert!(!dir.join(backup_file(&mark.save_file)).exists());
        assert!(std::fs::read(dir.join(&mark.save_file)).unwrap() == bytes);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn what_killed_it_lives_there_now() {
        // A beast of legend.
        let dir = tmp("beast");
        let ow = fallen_fort(32, |ow| ow.figures.iter().find(|f| f.alive() && f.lair.is_some() && f.civ.is_none()).map(|f| f.kind.clone()), &dir);
        let kind = ow.forts[0].killer.clone().expect("a killer");
        let mut a = adventure(ow, 32, &dir);
        a.enter(SiteKey::Fort(0)).unwrap();
        let g = &a.visit.as_ref().unwrap().g;
        let c = g.creatures.values().find(|c| c.species == Species::Megabeast && c.alive).expect("the beast is home");
        let f = c.figure.expect("a beast of legend");
        assert_eq!(g.overworld.figures[f as usize].kind, kind);
        assert_eq!(g.overworld.forts[0].occupant, Some(Occupant::Beast { figure: f }), "the legends know where it lairs");
        assert_eq!(c.target, Some(HERO_ID));
        assert!(log_text(g).contains(&g.overworld.figures[f as usize].name));
        let _ = std::fs::remove_dir_all(&dir);
        // An army.
        let dir = tmp("army");
        let ow = fallen_fort(33, |_| Some("goblin".into()), &dir);
        let mut a = adventure(ow, 33, &dir);
        a.enter(SiteKey::Fort(0)).unwrap();
        let v = a.visit.as_ref().unwrap();
        let gobs = v.g.creatures.values().filter(|c| c.species == Species::Goblin && c.alive).count();
        assert!(gobs >= 1, "goblins keep its halls");
        assert!(matches!(v.g.overworld.forts[0].occupant, Some(Occupant::Army { .. })));
        assert_eq!(v.placed.iter().filter(|(_, k)| k.starts_with('@')).count(), gobs, "each remembered as a foe");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_overseers_last_notes_are_left_behind() {
        let dir = tmp("notes");
        let ow = fallen_fort(34, |_| None, &dir);
        let mut a = adventure(ow, 34, &dir);
        a.enter(SiteKey::Fort(0)).unwrap();
        let v = a.visit.as_mut().unwrap();
        let h = v.hero;
        let book = |g: &Game, label: &str| g.items.values().find(|it| it.kind == ItemKind::Product && it.label.as_deref().map_or(false, |l| l.starts_with(label))).map(|it| it.pos);
        // The journal, open at its last page.
        let at = book(&v.g, JOURNAL).expect("the journal is left behind");
        v.g.dwarves[h].pos = at;
        let w = read_here(&v.g, h).unwrap();
        assert!(w.title.starts_with("The Journal of"), "{:?}", w);
        assert!(!w.chapters.is_empty(), "{:?}", w);
        // The tablet of its fall.
        let at = book(&v.g, TABLET).expect("the tablet of its fall");
        v.g.dwarves[h].pos = at;
        v.g.items.retain(|_, it| !(it.pos == at && it.label.as_deref() == Some(JOURNAL)));
        let w = read_here(&v.g, h).unwrap();
        assert!(w.fell, "{:?}", w);
        // The engraving tells the fort's own story.
        let fort = v.g.fortress_name.clone();
        let p = (0..v.g.world.tiles.len()).map(|i| v.g.world.pos_of(i)).find(|p| v.g.engraving_at(*p).map_or(false, |e| e.2.contains(&fort))).expect("the fort's story on its floor, in the adventure's legends");
        v.g.items.retain(|_, it| !(it.pos == p && it.kind == ItemKind::Product));
        v.g.dwarves[h].pos = p;
        let w = read_here(&v.g, h).unwrap();
        assert_eq!(w.title, "Carved in the Stone");
        assert!(w.chapters.iter().any(|c| c.2.contains(&v.g.fortress_name)), "{:?}", w);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_looted_fort_stays_looted() {
        let dir = tmp("looted");
        let ow = fallen_fort(35, |_| Some("goblin".into()), &dir);
        let file = ow.forts[0].save_file.clone();
        let original = std::fs::read(dir.join(&file)).unwrap();
        let mut a = adventure(ow, 35, &dir);
        a.enter(SiteKey::Fort(0)).unwrap();
        let id = prize(&a.visit.as_ref().unwrap().g).unwrap();
        let books = |g: &Game| g.items.values().filter(|it| it.label.as_deref() == Some(JOURNAL)).count();
        assert_eq!(books(&a.visit.as_ref().unwrap().g), 1);
        take(&mut a, id);
        let name = a.hero_dwarf().name.clone();
        a.leave();
        assert!(a.hero.pack.iter().any(|it| it.label.as_deref() == Some(PRIZE)), "carried out");
        // The player's save is kept, once.
        let backup = dir.join(backup_file(&file));
        assert!(std::fs::read(&backup).unwrap() == original, "the backup is the original");
        // The save no longer holds it, and remembers who took it.
        let saved = crate::saves::read(&dir, &file, &|_, _| {}).unwrap();
        assert!(prize(&saved).is_none(), "gone from the save");
        assert!(saved.fallen && saved.alive_count() == 0);
        let r = saved.overworld.forts[0].robbed.last().expect("the fort's own world remembers");
        assert!(r.by.contains(&name) && r.took.iter().any(|t| t == PRIZE), "{:?}", r);
        assert!(saved.overworld.events.iter().any(|e| e.text.contains(PRIZE)));
        assert_eq!(a.ow.forts[0].robbed.len(), 1);
        assert!(!a.ow.forts[0].artifacts.iter().any(|t| t == PRIZE));
        assert!(a.ow.events.last().unwrap().text.contains(PRIZE));
        assert!(crate::saves::list(&dir).iter().all(|m| !m.file.contains("pre-adventure")), "the backup is not a save to load");
        // A second hero finds it gone, and hears who took it.
        let mut b = adventure(a.ow.clone(), 36, &dir);
        b.enter(SiteKey::Fort(0)).unwrap();
        let g = &b.visit.as_ref().unwrap().g;
        assert!(prize(g).is_none(), "it stays gone");
        assert_eq!(books(g), 1, "the ruin was made once");
        assert!(log_text(g).contains(&format!("took {} from it", PRIZE)), "{}", log_text(g));
        // They take something too: the backup stays the original.
        let other = g.items.values().find(|it| it.carried_by.is_none() && it.container.is_none() && it.kind == ItemKind::Product).map(|it| it.id).expect("something to take");
        take(&mut b, other);
        b.leave();
        assert!(std::fs::read(&backup).unwrap() == original, "never written over");
        let saved = crate::saves::read(&dir, &file, &|_, _| {}).unwrap();
        assert_eq!(saved.overworld.forts[0].robbed.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// G03's Done-when: a hero leaves a town, finds the ruin of a fallen
    /// fort, fights what lives there and comes home with its treasure,
    /// recorded in the legends. (The walking is A3's and the bot A18's: here
    /// the hero goes straight in and is stood by the prize.)
    #[test]
    fn a_hero_brings_home_the_treasure_of_a_fallen_fort() {
        let base = tmp("g03");
        let ow = fallen_fort(37, |_| Some("goblin".into()), &base);
        let file = ow.forts[0].save_file.clone();
        let mut won = 0;
        for seed in 40..46u64 {
            let dir = tmp(&format!("g03-{}", seed));
            std::fs::create_dir_all(&dir).unwrap();
            for f in [file.clone(), format!("{}.meta.json", file.trim_end_matches(".json"))] {
                std::fs::copy(base.join(&f), dir.join(&f)).unwrap();
            }
            let mut a = crate::game::adv::quick_start(ow.clone(), seed);
            a.saves_dir = Some(dir.clone());
            a.knobs.set.insert("fallen.band".into(), 1.0);
            let Some(SiteKey::Site(home)) = a.visit.as_ref().map(|v| v.key) else { panic!("set out from a town") };
            // Armour from home.
            let v = a.visit.as_mut().unwrap();
            let (h, p) = (v.hero, v.g.dwarves[v.hero].pos);
            for kind in [ItemKind::Helm, ItemKind::Breastplate] {
                let id = v.g.spawn_item(kind, Mat::Iron, p);
                v.g.items.get_mut(&id).unwrap().carried_by = Some(HERO_ID);
                v.g.dwarves[h].armor.push(id);
            }
            a.enter(SiteKey::Fort(0)).unwrap();
            // Fight what lives there.
            let mut dead = false;
            for _ in 0..600 {
                let v = a.visit.as_mut().unwrap();
                let h = v.hero;
                let me = v.g.dwarves[h].pos;
                let foe = v.placed.iter().filter(|(_, k)| k.starts_with('@')).filter_map(|(id, _)| v.g.creatures.get(id)).filter(|c| c.alive).min_by_key(|c| (c.pos.x - me.x).abs().max((c.pos.y - me.y).abs()) + 10 * (c.pos.z - me.z).abs()).map(|c| (c.id, c.pos));
                let Some((id, fp)) = foe else { break };
                let act = if fp.z == me.z && (fp.x - me.x).abs() <= 1 && (fp.y - me.y).abs() <= 1 {
                    Action::Attack { target: Target::Creature(id), aim: None }
                } else {
                    Action::Wait(10)
                };
                if v.g.act(h, act) == Outcome::Died {
                    dead = true;
                    break;
                }
            }
            let v = a.visit.as_ref().unwrap();
            let standing = v.placed.iter().filter(|(_, k)| k.starts_with('@')).any(|(id, _)| v.g.creatures.get(id).map_or(false, |c| c.alive));
            if dead || standing {
                eprintln!("seed {}: {}", seed, if dead { "the hero died" } else { "the foe still stands" });
                let _ = std::fs::remove_dir_all(&dir);
                continue;
            }
            // Take its treasure, and go home.
            let id = prize(&a.visit.as_ref().unwrap().g).expect("the treasure is there");
            take(&mut a, id);
            let name = a.hero_dwarf().name.clone();
            a.leave();
            a.enter(SiteKey::Site(home)).unwrap();
            let v = a.visit.as_ref().unwrap();
            assert!(v.g.items.values().any(|it| it.carried_by == Some(HERO_ID) && it.label.as_deref() == Some(PRIZE)), "brought home");
            let first = name.split(' ').next().unwrap();
            assert!(a.ow().events.iter().any(|e| e.text.contains(PRIZE) && e.text.contains(first)), "in the legends");
            let m = &a.ow().forts[0];
            assert_eq!(m.occupant, Some(Occupant::Empty), "its killers slain");
            assert!(m.robbed.last().map_or(false, |r| r.by.contains(first)));
            won += 1;
            let _ = std::fs::remove_dir_all(&dir);
            break;
        }
        let _ = std::fs::remove_dir_all(&base);
        assert!(won > 0, "no hero came home");
    }
}
