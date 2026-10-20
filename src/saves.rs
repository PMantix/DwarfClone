//! Save slots: one file per fortress plus an autosave, each with a small
//! metadata file so the load menu can list saves without reading them.

use crate::game::adv::Adventure;
use crate::game::Game;
use crate::jobs::Report;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// What a save holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SaveKind {
    #[default]
    Fort,
    /// An adventure (`<Hero>.adv.json`, G03).
    Adventure,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveMeta {
    pub fortress: String,
    pub date: String,
    pub dwarves: usize,
    pub world: String,
    pub saved_unix: u64,
    pub autosave: bool,
    /// File name of the save itself, inside the saves folder.
    pub file: String,
    #[serde(default)]
    pub kind: SaveKind,
}

fn slot(fortress: &str, autosave: bool) -> String {
    let base: String = fortress.chars().filter(|c| c.is_alphanumeric()).collect();
    let base = if base.is_empty() { "fortress".to_string() } else { base };
    if autosave {
        format!("{}-autosave", base)
    } else {
        base
    }
}

/// The file a fort's own save goes to (not the autosave).
pub fn fort_file(fortress: &str) -> String {
    format!("{}.json", slot(fortress, false))
}

fn meta_path(dir: &Path, file: &str) -> PathBuf {
    dir.join(format!("{}.meta.json", file.trim_end_matches(".json")))
}

pub fn now_unix() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

pub fn write(dir: &Path, game: &Game, autosave: bool, report: Report) -> Result<SaveMeta, String> {
    report("Packing up the fortress", 0.1);
    std::fs::create_dir_all(dir).map_err(|e| format!("Can't create {}: {}", dir.display(), e))?;
    let data = serde_json::to_string(game).map_err(|e| format!("Save failed: {}", e))?;
    report("Writing to disk", 0.6);
    let file = format!("{}.json", slot(&game.fortress_name, autosave));
    // Write to a temporary file first so a crash can't leave half a save behind.
    let tmp = dir.join(format!("{}.tmp", file));
    std::fs::write(&tmp, data).map_err(|e| format!("Save failed: {}", e))?;
    std::fs::rename(&tmp, dir.join(&file)).map_err(|e| format!("Save failed: {}", e))?;
    let meta = SaveMeta {
        fortress: game.fortress_name.clone(),
        date: game.date_string(),
        dwarves: game.alive_count(),
        world: game.overworld.name.clone(),
        saved_unix: now_unix(),
        autosave,
        file,
        kind: SaveKind::Fort,
    };
    let m = serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?;
    std::fs::write(meta_path(dir, &meta.file), m).map_err(|e| format!("Save failed: {}", e))?;
    report("Saved", 1.0);
    Ok(meta)
}

#[allow(dead_code)] // G03 A11 (F5, autosaves) calls it; tests already do
/// Save an adventure as `<Hero>.adv.json` (or its autosave), with its meta.
pub fn write_adventure(dir: &Path, adv: &Adventure, autosave: bool, report: Report) -> Result<SaveMeta, String> {
    report("Packing the hero's things", 0.1);
    std::fs::create_dir_all(dir).map_err(|e| format!("Can't create {}: {}", dir.display(), e))?;
    let data = serde_json::to_string(adv).map_err(|e| format!("Save failed: {}", e))?;
    report("Writing to disk", 0.6);
    let hero = &adv.hero_dwarf().name;
    let file = format!("{}.adv.json", slot(hero, autosave));
    let tmp = dir.join(format!("{}.tmp", file));
    std::fs::write(&tmp, data).map_err(|e| format!("Save failed: {}", e))?;
    std::fs::rename(&tmp, dir.join(&file)).map_err(|e| format!("Save failed: {}", e))?;
    let place = adv.visit.as_ref().map_or_else(|| "on the road".to_string(), |v| format!("in {}", v.g.fortress_name));
    let meta = SaveMeta {
        fortress: hero.clone(),
        date: format!("Year {}, {}", adv.year(), place),
        dwarves: usize::from(adv.hero_dwarf().alive),
        world: adv.ow().name.clone(),
        saved_unix: now_unix(),
        autosave,
        file,
        kind: SaveKind::Adventure,
    };
    let m = serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?;
    std::fs::write(meta_path(dir, &meta.file), m).map_err(|e| format!("Save failed: {}", e))?;
    report("Saved", 1.0);
    Ok(meta)
}

pub fn read_adventure(dir: &Path, file: &str, report: Report) -> Result<Adventure, String> {
    report("Reading the save file", 0.1);
    let data = std::fs::read_to_string(dir.join(file)).map_err(|e| format!("Load failed: {}", e))?;
    report("Unpacking the hero's things", 0.5);
    let mut a: Adventure = serde_json::from_str(&data).map_err(|e| format!("Load failed (an older save?): {}", e))?;
    if let Some(v) = a.visit.as_mut() {
        v.g.after_load();
    }
    report("Loaded", 1.0);
    Ok(a)
}

pub fn read(dir: &Path, file: &str, report: Report) -> Result<Game, String> {
    report("Reading the save file", 0.1);
    let data = std::fs::read_to_string(dir.join(file)).map_err(|e| format!("Load failed: {}", e))?;
    report("Unpacking the fortress", 0.5);
    let mut g: Game = serde_json::from_str(&data).map_err(|e| format!("Load failed (an older save?): {}", e))?;
    g.after_load();
    report("Loaded", 1.0);
    Ok(g)
}

/// Every save in the folder, newest first.
pub fn list(dir: &Path) -> Vec<SaveMeta> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut v: Vec<SaveMeta> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".meta.json"))
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .filter_map(|s| serde_json::from_str::<SaveMeta>(&s).ok())
        .filter(|m| dir.join(&m.file).exists())
        .collect();
    v.sort_by(|a, b| b.saved_unix.cmp(&a.saved_unix));
    v
}

pub fn delete(dir: &Path, meta: &SaveMeta) -> Result<(), String> {
    std::fs::remove_file(dir.join(&meta.file)).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(meta_path(dir, &meta.file));
    Ok(())
}

/// "just now", "5 minutes ago", "3 days ago".
pub fn ago(unix: u64) -> String {
    let s = now_unix().saturating_sub(unix);
    match s {
        0..=59 => "just now".into(),
        60..=3599 => format!("{} min ago", s / 60),
        3600..=86_399 => format!("{} h ago", s / 3600),
        _ => format!("{} days ago", s / 86_400),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dwarf::Task;

    /// A save written by the game as it was before Milestone 1 (Goldfist,
    /// two days in, on a small map; its map and world were left out to keep
    /// the file small) still loads, with the new fields at their defaults.
    #[test]
    fn a_save_from_before_m1_loads() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures");
        let g = read(&dir, "goldfist-before-m1.json", &|_, _| {}).unwrap();
        assert_eq!((g.fortress_name.as_str(), g.tick, g.alive_count()), ("Goldfist", 2400, 7));
        assert!(g.knobs.set.is_empty() && g.annals.counters.is_empty());
        assert!(g.dwarves.iter().any(|d| matches!(d.task, Task::Dig { .. })));
        assert!(g.overseer.is_some());
        // And it saves again as it is now.
        let back: Game = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        assert_eq!((back.tick, back.dwarves.len()), (g.tick, g.dwarves.len()));
    }

    /// A save's meta and world from before adventurer mode (no `kind`, no
    /// `forts` or `remains` in its world) load, as a fort, with them empty.
    #[test]
    fn old_saves_load_without_adventure_fields() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures");
        let g = read(&dir, "goldfist-before-m1.json", &|_, _| {}).unwrap();
        assert!(g.overworld.forts.is_empty() && g.overworld.remains.is_empty());
        let old = r#"{"fortress":"Goldfist","date":"1st Granite","dwarves":7,"world":"The Wilds","saved_unix":5,"autosave":false,"file":"Goldfist.json"}"#;
        let m: SaveMeta = serde_json::from_str(old).unwrap();
        assert_eq!(m.kind, SaveKind::Fort);
        // And a world written without them still reads.
        let mut ow = serde_json::to_value(&Game::new(5).overworld).unwrap();
        ow.as_object_mut().unwrap().retain(|k, _| k != "forts" && k != "remains");
        let ow: crate::overworld::Overworld = serde_json::from_value(ow).unwrap();
        assert!(ow.forts.is_empty() && ow.remains.is_empty() && ow.fort_at(0, 0).is_none());
    }

    /// Drop from `new` whatever `old` (the same thing as an older save wrote
    /// it) doesn't have, so it loads the way an older save does: the fields
    /// added since come back at their defaults.
    fn as_old(new: &mut serde_json::Value, old: &serde_json::Value) {
        use serde_json::Value;
        match (new, old) {
            (Value::Object(n), Value::Object(o)) => {
                n.retain(|k, _| o.contains_key(k));
                for (k, v) in n.iter_mut() {
                    as_old(v, &o[k]);
                }
            }
            (Value::Array(n), Value::Array(o)) if !o.is_empty() => {
                // A short tuple (a run of tiles: count and tile) goes by
                // position; a list by its first element.
                let tuple = o.len() == n.len() && o.len() <= 3;
                for (i, v) in n.iter_mut().enumerate() {
                    as_old(v, if tuple { &o[i] } else { &o[0] });
                }
            }
            _ => {}
        }
    }

    /// The same old save, made before the worlds were built from fields
    /// (no fields, rivers, regions or region tiles in its world), plays on.
    /// Its map and world were left out of the file, so it is given today's,
    /// written as that older game wrote them (nothing it didn't know of: no
    /// fields, no region populations, no rivers in the cells), and its
    /// fortress is moved onto the new ground by the wagon. A few days go
    /// by with the overseer at work and nobody lost; and what it has become
    /// saves and plays on again.
    #[test]
    fn a_save_from_before_m1_plays_on() {
        use crate::game::DAY;
        use crate::geom::Pos;
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures");
        let mut g = read(&dir, "goldfist-before-m1.json", &|_, _| {}).unwrap();
        let old: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("goldfist-before-m1.json")).unwrap()).unwrap();
        let fresh = Game::new(5);
        let mut world = serde_json::to_value(&fresh.world).unwrap();
        as_old(&mut world, &old["world"]);
        g.world = serde_json::from_value(world).unwrap();
        let mut ow = serde_json::to_value(&fresh.overworld).unwrap();
        as_old(&mut ow, &old["overworld"]);
        g.overworld = serde_json::from_value(ow).unwrap();
        assert!(g.overworld.fields.is_empty() && g.overworld.rivers.is_empty() && g.overworld.seed == 0, "the world is as an old save had it");
        g.site = fresh.site;
        // The fortress, moved so its wagon stands where today's would.
        let d = (fresh.embark.x - g.embark.x, fresh.embark.y - g.embark.y, fresh.embark.z - g.embark.z);
        let mv = |p: Pos| p.offset(d.0, d.1, d.2);
        g.embark = fresh.embark;
        let mut ground: Vec<Pos> = Vec::new();
        for dw in g.dwarves.iter_mut() {
            dw.pos = mv(dw.pos);
            dw.path.clear();
            ground.push(dw.pos);
        }
        for it in g.items.values_mut() {
            it.pos = mv(it.pos);
            ground.push(it.pos);
        }
        for c in g.creatures.values_mut() {
            c.pos = mv(c.pos);
            c.path.clear();
            ground.push(c.pos);
        }
        let mut stamps: Vec<(Pos, u32, bool)> = Vec::new();
        for b in g.buildings.values_mut() {
            b.pos = mv(b.pos);
            let s = b.kind.size();
            for dy in 0..s {
                for dx in 0..s {
                    stamps.push((b.pos.offset(dx, dy, 0), b.id, true));
                }
            }
        }
        for z in g.zones.values_mut() {
            for t in z.tiles.iter_mut() {
                *t = mv(*t);
                stamps.push((*t, z.id, false));
            }
        }
        // (What was designated was marked on the old map's tiles.)
        g.designations.clear();
        if let Some(o) = g.overseer.as_mut() {
            o.base = o.base.map(mv);
            o.top = o.top.map(mv);
        }
        // Ground under all of it: a floor where each thing stands.
        for &p in ground.iter().chain(stamps.iter().map(|s| &s.0)) {
            if g.world.get(p).is_some() {
                let under = !g.world.outside(p);
                let t = g.world.t_mut(p);
                if t.kind.solid() || t.kind == crate::world::TileKind::Open {
                    t.kind = crate::world::TileKind::Floor;
                    t.dug = under;
                }
                t.revealed = true;
                t.water = 0;
            }
        }
        for (p, id, building) in stamps {
            if g.world.get(p).is_some() {
                let t = g.world.t_mut(p);
                if building {
                    t.building = id;
                } else {
                    t.zone = id;
                }
            }
        }
        g.after_load();
        let start = g.tick;
        let alive = g.alive_count();
        while g.tick < start + 3 * DAY {
            g.step();
        }
        assert_eq!(g.tick, start + 3 * DAY);
        assert_eq!(g.alive_count(), alive, "nobody lost in three quiet days");
        assert!(g.overseer.as_ref().map_or(false, |o| o.notes.len() > old["overseer"]["notes"].as_array().map_or(0, |n| n.len())), "the overseer went on with its work");
        // And it saves, loads and plays on again.
        let mut back: Game = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        back.after_load();
        for _ in 0..DAY {
            back.step();
        }
        assert_eq!(back.tick, start + 4 * DAY);
    }
}
