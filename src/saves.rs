//! Save slots: one file per fortress plus an autosave, each with a small
//! metadata file so the load menu can list saves without reading them.

use crate::game::Game;
use crate::jobs::Report;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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
    };
    let m = serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?;
    std::fs::write(meta_path(dir, &meta.file), m).map_err(|e| format!("Save failed: {}", e))?;
    report("Saved", 1.0);
    Ok(meta)
}

pub fn read(dir: &Path, file: &str, report: Report) -> Result<Game, String> {
    report("Reading the save file", 0.1);
    let data = std::fs::read_to_string(dir.join(file)).map_err(|e| format!("Load failed: {}", e))?;
    report("Unpacking the fortress", 0.5);
    let g: Game = serde_json::from_str(&data).map_err(|e| format!("Load failed (an older save?): {}", e))?;
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
