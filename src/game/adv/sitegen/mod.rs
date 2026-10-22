//! What stands on a site, by what the cell holds: one generator per kind
//! (plan/adventure.md, A4 and A5). The land is the cell's own (`build`,
//! the land a fort embarked there would get); then `make` puts on it what
//! the world's history left there: a beast and its hoard (lair.rs), the
//! shell of a place that died (ruin.rs), a living town (town.rs) or the
//! wild (wild.rs). `lore.rs` tells each place's past from the legends.
//!
//! Site memory: a site is made again from the same seed each time, so
//! what the hero changed is kept as a `SiteMemo` (`remember` on leaving)
//! and laid over the new map (`recall` on entering): the slain stay dead,
//! the taken stay gone, and the bodies lie where they fell.

pub mod fallen;
pub mod lair;
pub mod lore;
pub mod ruin;
pub mod town;
pub mod wild;

use super::peoples::{self, Stance};
use super::{Adventure, SiteKey, SiteMemo, HERO_ID};
use crate::creature::Species;
use crate::game::{Game, SEASON, YEAR};
use crate::geom::Pos;
use crate::items::ItemKind;
use crate::material::Mat;
use crate::overworld::{Overworld, Race, SiteKind};
use crate::rng::Rng;
use crate::world::{Mover, World};
use std::collections::{HashSet, VecDeque};

/// The site's map and where the hero arrives on it.
pub fn build(adv: &Adventure, key: SiteKey, seed: u64) -> Result<(World, Pos), String> {
    match key {
        SiteKey::Fort(_) => fallen::build(adv, key, seed),
        SiteKey::Site(id) if !adv.ow().sites.iter().any(|s| s.id == id) => Err("No such site.".to_string()),
        _ => Ok(wild::build(adv, key, seed)),
    }
}

/// Who is coming in, and what they changed here before.
pub struct Ctx {
    pub key: SiteKey,
    pub seed: u64,
    pub race: Race,
    pub civ: Option<u32>,
    pub memo: SiteMemo,
    /// World time now.
    pub now: u64,
    /// What the hero can stand against: their fighting level, and one more
    /// for each companion (A16). Those who hold a place grow with it.
    pub might: u32,
    /// The adventure's knobs, for the generators (`Ctx::knob`).
    pub knobs: crate::game::Knobs,
}

impl Ctx {
    /// A tunable number: `default` unless the adventure set it.
    pub fn knob(&self, name: &'static str, default: f64) -> f64 {
        self.knobs.set.get(name).copied().unwrap_or(default)
    }
}

/// What a generator made: what it placed (for the site's memory), where
/// the hero comes in, and the place's story, told on arrival.
#[derive(Default)]
pub struct Made {
    /// (id, key): an item's loot key, `@` and a creature's name, `#` for
    /// scenery (rubble, ash), or `~` and a thing left from an earlier visit.
    pub placed: Vec<(u32, String)>,
    pub arrive: Option<Pos>,
    /// The arrival card's title ("The burnt halls of Kogsak").
    pub title: String,
    /// The place's past, a sentence a line, the first the headline.
    pub story: Vec<String>,
    /// Whoever is here is out for the hero's blood.
    pub hostile: bool,
}

/// Put on the land what the site holds, then what the hero did here.
pub fn make(g: &mut Game, ctx: &Ctx) -> Made {
    let mut made = match ctx.key {
        SiteKey::Wild(..) => wild::make(g, ctx),
        // A fallen fort is its own save (A5); this is one whose save is gone.
        SiteKey::Fort(_) => fallen::make(g, ctx),
        SiteKey::Site(id) => {
            let Some(site) = g.overworld.sites.iter().find(|s| s.id == id).cloned() else { return Made::default() };
            let stance = peoples::stance(&g.overworld, ctx.race, ctx.civ, &site);
            match site.kind {
                SiteKind::Lair => lair::make(g, ctx, &site),
                _ if site.ruined => ruin::make(g, ctx, &site, true),
                SiteKind::DarkFortress if stance == Stance::Hostile => ruin::make(g, ctx, &site, false),
                _ => town::make(g, ctx, &site, stance),
            }
        }
    };
    recall(g, ctx, &mut made);
    made
}

// ---------------------------------------------------------------- memory

/// Lay what the hero changed here before over the new map: taken loot is
/// not there, the slain are not there (their bodies are, or their bones
/// after a season), and a cleared site stays empty for a year.
pub fn recall(g: &mut Game, ctx: &Ctx, made: &mut Made) {
    let memo = &ctx.memo;
    let mut taken: Vec<&str> = memo.taken.iter().map(|s| s.as_str()).collect();
    let mut slain: Vec<&str> = memo.slain.iter().map(|s| s.as_str()).collect();
    let quiet = memo.cleared && ctx.now < memo.visited_tick + YEAR;
    let mut keep = Vec::with_capacity(made.placed.len());
    for (id, key) in std::mem::take(&mut made.placed) {
        if let Some(name) = key.strip_prefix('@') {
            if let Some(i) = slain.iter().position(|s| *s == name) {
                slain.swap_remove(i);
                drop_kit(g, id);
                continue;
            }
            if quiet {
                drop_kit(g, id);
                continue;
            }
        } else if let Some(i) = taken.iter().position(|s| *s == key) {
            taken.swap_remove(i);
            g.items.remove(&id);
            continue;
        }
        keep.push((id, key));
    }
    made.placed = keep;
    // What the hero dug before (climb.rs): the rock stays gone.
    for &(p, k) in &memo.dug {
        if g.world.in_bounds(p) {
            let t = g.world.t_mut(p);
            t.kind = k;
            t.dug = true;
            t.revealed = true;
        }
    }
    // What was left lying (the dead where they fell, a slain foe's gear,
    // what the hero dropped): bodies rot to bones after a season.
    let old = ctx.now >= memo.visited_tick + SEASON;
    for (p, what) in &memo.corpses {
        let Ok((kind, mat, quality, label)) = serde_json::from_str::<(ItemKind, Mat, u8, Option<String>)>(what) else { continue };
        let (kind, mat, label) = match kind {
            ItemKind::Corpse | ItemKind::Carcass if old => {
                let who = label.unwrap_or_else(|| "something".into());
                (ItemKind::BodyPart, Mat::Bone, Some(if who.starts_with("the bones of") { who } else { format!("the bones of {}", lore::a_or_an(&who)) }))
            }
            _ => (kind, mat, label),
        };
        let id = g.spawn_item(kind, mat, *p);
        if let Some(it) = g.items.get_mut(&id) {
            it.label = label;
            it.quality = quality;
        }
        made.placed.push((id, format!("~{}", what)));
    }
}

/// A creature that isn't here after all, and the gear it would have held.
fn drop_kit(g: &mut Game, id: u32) {
    if let Some(c) = g.creatures.remove(&id) {
        for i in c.weapon.iter().chain(c.armor.iter()) {
            g.items.remove(i);
        }
    }
}

/// Read what the hero changed from the visit as they leave: occupants
/// slain, loot taken, the bodies on the ground, and whether it is cleared.
pub fn remember(g: &Game, placed: &[(u32, String)], memo: &mut SiteMemo) {
    let mut any = false;
    let mut alive = false;
    // (Leftovers from an earlier visit are remembered again if still there.)
    let made: HashSet<u32> = placed.iter().filter(|(_, k)| !k.starts_with('~')).map(|(id, _)| *id).collect();
    for (id, key) in placed {
        if let Some(name) = key.strip_prefix('@') {
            any = true;
            if g.creatures.get(id).map_or(false, |c| c.alive) {
                alive = true;
            } else {
                memo.slain.push(name.to_string());
            }
        } else if !key.starts_with('~') && !key.starts_with('#') {
            let here = g.items.get(id).map_or(false, |it| it.carried_by.is_none() && loot_key(it.kind, it.mat, it.label.as_deref(), it.pos) == *key);
            if !here {
                memo.taken.push(key.clone());
            }
        }
    }
    if any {
        memo.cleared = !alive;
    }
    // What lies on the ground that the site wasn't made with.
    memo.corpses = g
        .items
        .values()
        .filter(|it| it.carried_by.is_none() && it.container.is_none() && !made.contains(&it.id))
        .filter_map(|it| serde_json::to_string(&(it.kind, it.mat, it.quality, it.label.clone())).ok().map(|s| (it.pos, s)))
        .take(60)
        .collect();
}

// ---------------------------------------------------------------- placing

/// The key that knows a piece of loot again on the next visit: its label
/// (an artifact's name, a victim's bones), or its kind, stuff and spot.
pub fn loot_key(kind: ItemKind, mat: Mat, label: Option<&str>, p: Pos) -> String {
    match label {
        Some(l) => format!("{}|{:?}", l, kind),
        None => format!("{:?}|{:?}|{},{},{}", kind, mat, p.x, p.y, p.z),
    }
}

/// Scenery made with the site (rubble, ash): neither loot nor leftovers.
pub fn scenery(g: &mut Game, made: &mut Made, kind: ItemKind, mat: Mat, at: Pos) {
    let id = g.spawn_item(kind, mat, at);
    made.placed.push((id, "#".to_string()));
}

/// Lay a piece of loot on the ground, remembered by its key.
pub fn loot(g: &mut Game, made: &mut Made, kind: ItemKind, mat: Mat, at: Pos, quality: u8, label: Option<String>) -> u32 {
    let id = g.spawn_item(kind, mat, at);
    if let Some(it) = g.items.get_mut(&id) {
        it.quality = quality;
        it.label = label.clone();
    }
    made.placed.push((id, loot_key(kind, mat, label.as_deref(), at)));
    id
}

/// Something that lives (or lurks) here. A hostile one hunts the hero
/// from the start: `siege` keeps it after them across the map and
/// `target` names them (wave 0's notes).
pub fn occupant(g: &mut Game, made: &mut Made, sp: Species, at: Pos, label: Option<String>, hostile: bool) -> u32 {
    let id = match sp {
        Species::Beast(_) => g.spawn_beast(sp, at, u64::MAX / 2),
        _ => g.spawn_creature(sp, at, u64::MAX / 2),
    };
    let name = label.clone().unwrap_or_else(|| sp.info().name.to_string());
    if let Some(c) = g.creatures.get_mut(&id) {
        c.label = label;
        if hostile {
            c.siege = true;
            c.target = Some(HERO_ID);
        }
    }
    made.placed.push((id, format!("@{}", name)));
    id
}

/// Put a weapon (or armour) in a creature's hands.
pub fn arm(g: &mut Game, cid: u32, kind: ItemKind, mat: Mat) {
    let Some(p) = g.creatures.get(&cid).map(|c| c.pos) else { return };
    let iid = g.spawn_item(kind, mat, p);
    if let Some(it) = g.items.get_mut(&iid) {
        it.carried_by = Some(cid);
    }
    if let Some(c) = g.creatures.get_mut(&cid) {
        if kind.weapon_damage().is_some() {
            c.weapon = Some(iid);
        } else {
            c.armor.push(iid);
        }
    }
}

/// The soldiers of a people, as the bestiary has them.
pub fn soldier(race: Race) -> Species {
    match race {
        Race::Dwarf => Species::Raider,
        Race::Human => Species::Human,
        Race::Elf => Species::Elf,
        Race::Goblin => Species::Goblin,
    }
}

/// The people of a civ.
pub fn civ_race(ow: &Overworld, civ: Option<u32>) -> Option<Race> {
    civ.and_then(|c| ow.civs.get(c as usize)).map(|c| c.race)
}

/// A name from a hash, never from the game's dice.
pub fn hashed_name(seed: u64, salt: u64, race: Race) -> String {
    let mut r = Rng::new(crate::game::names::hash(&[seed, salt]));
    crate::overworld::person_name(&mut r, race)
}

/// Ground one can walk to from `from`, nearest first (at most `n`).
pub fn reachable(w: &World, from: Pos, n: usize) -> Vec<Pos> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    let mut q = VecDeque::new();
    let mut nb = Vec::new();
    if !w.in_bounds(from) {
        return out;
    }
    seen.insert(from);
    q.push_back(from);
    while let Some(p) = q.pop_front() {
        out.push(p);
        if out.len() >= n {
            break;
        }
        w.neighbors(p, &mut nb, Mover::Dwarf);
        for &(r, _) in nb.iter() {
            if seen.insert(r) {
                q.push_back(r);
            }
        }
    }
    out
}

/// Dry, open ground under the sky.
pub fn open_ground(w: &World, p: Pos) -> bool {
    w.in_bounds(p) && w.walkable(p) && w.outside(p) && w.t(p).water == 0 && w.t(p).magma == 0
}

/// Where the hero comes in: open ground `lo`..`hi` steps from `from`
/// that can be walked to (the farthest reachable, if none is that far).
pub fn approach(w: &World, from: Pos, lo: i32, hi: i32, rng: &mut Rng) -> Pos {
    let all = reachable(w, from, 4000);
    let far: Vec<Pos> = all
        .iter()
        .copied()
        .filter(|p| open_ground(w, *p) && p.x > 1 && p.y > 1 && p.x < w.w - 2 && p.y < w.h - 2)
        .filter(|p| {
            let d = (p.x - from.x).abs().max((p.y - from.y).abs());
            d >= lo && d <= hi
        })
        .collect();
    if far.is_empty() {
        return all.last().copied().unwrap_or(from);
    }
    // On the same level, where there is one, so the place is in view.
    let level: Vec<Pos> = far.iter().copied().filter(|p| p.z == from.z).collect();
    *rng.pick(if level.is_empty() { &far } else { &level })
}
