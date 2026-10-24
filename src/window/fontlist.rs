//! The fonts a player can choose between: the monospace ones on their own
//! machine, and anything they drop in a `fonts` folder next to the game.
//! Nothing is shipped; only each file's name table and pitch flag are read,
//! so looking through a few hundred system fonts takes a moment.

use super::glyphs::{font_dir, MONO};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// A font file that can draw the game's text.
#[derive(Clone, Debug, PartialEq)]
pub struct FontFile {
    /// Its name as the font gives it ("Consolas Bold").
    pub name: String,
    pub path: PathBuf,
    /// One of the usual fonts the game picks by itself.
    pub usual: bool,
    /// From the player's own `fonts` folder.
    pub mine: bool,
}

/// The player's own fonts: a `fonts` folder next to the game.
pub fn my_dir() -> Option<PathBuf> {
    Some(std::env::current_exe().ok()?.parent()?.join("fonts"))
}

/// Every monospace-friendly font on the machine, the usual ones first (in
/// the order the game tries them), then the player's own, then the rest by name.
pub fn scan() -> Vec<FontFile> {
    let mut dirs = vec![(font_dir(), false)];
    // (Fonts installed for one user only live in their profile.)
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        dirs.push((Path::new(&local).join("Microsoft").join("Windows").join("Fonts"), false));
    }
    if let Some(d) = my_dir() {
        dirs.push((d, true));
    }
    scan_dirs(&dirs)
}

/// The fonts in `dirs`; a folder marked as the player's lists every font
/// in it, the others only the monospace ones.
pub fn scan_dirs(dirs: &[(PathBuf, bool)]) -> Vec<FontFile> {
    let mut out: Vec<FontFile> = Vec::new();
    for (dir, mine) in dirs {
        let Ok(rd) = std::fs::read_dir(dir) else { continue };
        let mut files: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
        files.sort();
        for path in files {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
            if ext != "ttf" && ext != "otf" {
                continue;
            }
            let Some((name, fixed)) = read_info(&path) else { continue };
            if !*mine && !fixed && !looks_mono(&name) {
                continue;
            }
            let file = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
            let usual = !*mine && MONO.iter().any(|m| m.eq_ignore_ascii_case(file));
            // (The same font in two places: the first found.)
            if out.iter().any(|f| f.name == name) {
                continue;
            }
            out.push(FontFile { name, path, usual, mine: *mine });
        }
    }
    let rank = |f: &FontFile| {
        let file = f.path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_ascii_lowercase();
        let usual = MONO.iter().position(|m| m.to_ascii_lowercase() == file).filter(|_| f.usual);
        (usual.unwrap_or(MONO.len()), !f.mine, f.name.to_ascii_lowercase())
    };
    out.sort_by_key(rank);
    out
}

/// A font file's name, or failing that the file's.
pub fn name_of(path: &Path) -> String {
    read_info(path).map(|(n, _)| n).unwrap_or_else(|| path.file_stem().map_or(String::new(), |s| s.to_string_lossy().into_owned()))
}

/// Names that say "monospace" when a font forgets to set its flag.
fn looks_mono(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    ["mono", "consol", "courier", "code", "terminal", "fixed"].iter().any(|w| n.contains(w))
}

/// A font file's full name and whether it says it is monospace, read from
/// its table directory, `post` and `name` tables without loading the rest.
fn read_info(path: &Path) -> Option<(String, bool)> {
    let mut f = std::fs::File::open(path).ok()?;
    let mut head = [0u8; 12];
    f.read_exact(&mut head).ok()?;
    let tag = &head[0..4];
    if tag != [0, 1, 0, 0] && tag != b"OTTO" && tag != b"true" {
        return None;
    }
    let n = u16::from_be_bytes([head[4], head[5]]) as usize;
    let mut dir = vec![0u8; n * 16];
    f.read_exact(&mut dir).ok()?;
    let table = |want: &[u8; 4]| -> Option<(u64, usize)> {
        dir.chunks(16).find(|r| &r[0..4] == want).map(|r| (u32::from_be_bytes([r[8], r[9], r[10], r[11]]) as u64, u32::from_be_bytes([r[12], r[13], r[14], r[15]]) as usize))
    };
    let mut read = |(at, len): (u64, usize)| -> Option<Vec<u8>> {
        if len > 1 << 20 {
            return None;
        }
        let mut b = vec![0u8; len];
        f.seek(SeekFrom::Start(at)).ok()?;
        f.read_exact(&mut b).ok()?;
        Some(b)
    };
    let fixed = table(b"post").and_then(&mut read).map_or(false, |p| p.len() >= 16 && p[12..16] != [0, 0, 0, 0]);
    let name = table(b"name").and_then(&mut read).and_then(|t| full_name(&t))?;
    Some((name, fixed))
}

/// The font's full name (name 4, else its family, name 1) from a `name`
/// table, preferring the Windows English record.
fn full_name(t: &[u8]) -> Option<String> {
    let u16at = |i: usize| -> Option<usize> { Some(u16::from_be_bytes([*t.get(i)?, *t.get(i + 1)?]) as usize) };
    let count = u16at(2)?;
    let strings = u16at(4)?;
    let mut best: Option<(u32, String)> = None;
    for r in 0..count {
        let at = 6 + r * 12;
        let (platform, lang, id, len, off) = (u16at(at)?, u16at(at + 4)?, u16at(at + 6)?, u16at(at + 8)?, u16at(at + 10)?);
        if id != 4 && id != 1 {
            continue;
        }
        let Some(bytes) = t.get(strings + off..strings + off + len) else { continue };
        let text = match platform {
            // Windows and Unicode records are UTF-16, big-endian.
            0 | 3 => String::from_utf16_lossy(&bytes.chunks(2).filter(|c| c.len() == 2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect::<Vec<_>>()),
            1 => bytes.iter().map(|&b| b as char).collect(),
            _ => continue,
        };
        let text = text.trim().to_string();
        if text.is_empty() {
            continue;
        }
        let score = (id == 4) as u32 * 4 + (platform == 3) as u32 * 2 + (lang == 0x409) as u32;
        if best.as_ref().map_or(true, |(s, _)| score > *s) {
            best = Some((score, text));
        }
    }
    best.map(|(_, n)| n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_usual_fonts_are_found_first() {
        // Only on Windows, where the usual fonts come with the system.
        if !cfg!(windows) || !font_dir().join("consola.ttf").exists() {
            return;
        }
        let list = scan_dirs(&[(font_dir(), false)]);
        let consolas = list.iter().position(|f| f.name == "Consolas").expect("Consolas is listed by its name");
        assert!(list[consolas].usual);
        assert!(list[..consolas].iter().all(|f| f.usual), "the usual fonts come first");
        if font_dir().join("lucon.ttf").exists() {
            assert!(list.iter().any(|f| f.name == "Lucida Console" && f.usual));
        }
        // Monospace fonts only: no Arial, no Segoe UI.
        assert!(!list.iter().any(|f| f.name.starts_with("Arial") || f.name.starts_with("Segoe UI")), "{:?}", list.iter().map(|f| &f.name).collect::<Vec<_>>());
        // Each one is a font the game can draw with.
        for f in list.iter().take(4) {
            assert!(super::super::glyphs::Fonts::load(f.path.to_str()).is_ok(), "{} won't load", f.name);
        }
    }

    #[test]
    fn a_players_own_folder_lists_every_font_in_it() {
        if !font_dir().join("arial.ttf").exists() {
            return;
        }
        let mine = std::env::temp_dir().join(format!("dwarfclone-fonts-{}", std::process::id()));
        std::fs::create_dir_all(&mine).unwrap();
        std::fs::copy(font_dir().join("arial.ttf"), mine.join("my font.ttf")).unwrap();
        std::fs::write(mine.join("broken.ttf"), b"not a font").unwrap();
        std::fs::write(mine.join("notes.txt"), b"hi").unwrap();
        let list = scan_dirs(&[(mine.clone(), true)]);
        assert_eq!(list.len(), 1, "{:?}", list);
        assert!(list[0].mine && !list[0].usual && list[0].name.starts_with("Arial"));
        let _ = std::fs::remove_dir_all(&mine);
    }
}
