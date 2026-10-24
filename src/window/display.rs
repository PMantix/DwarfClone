//! The window's display settings: which font, how big the cells are, how
//! wide and heavy the letters are drawn, how big the adventure map's tiles
//! are. Kept next to the saves in `display.cfg`, the way `sound.cfg` is.

use serde::{Deserialize, Serialize};
use std::path::Path;

pub const SETTINGS_FILE: &str = "display.cfg";

/// How far letters are stretched across their square cell. A monospace
/// letter is about half as wide as it is tall, so in a square cell text
/// looks letter-spaced; stretching it closes the gaps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Width {
    /// The font's own shape.
    Narrow,
    #[default]
    Normal,
    Wide,
}

impl Width {
    pub const ALL: [Width; 3] = [Width::Narrow, Width::Normal, Width::Wide];

    pub fn name(self) -> &'static str {
        match self {
            Width::Narrow => "narrow",
            Width::Normal => "normal",
            Width::Wide => "wide",
        }
    }

    /// How much of the cell a letter's advance should fill, and the most
    /// it may be stretched to get there.
    pub fn stretch(self) -> (f32, f32) {
        match self {
            Width::Narrow => (0.0, 1.0),
            Width::Normal => (0.72, 1.3),
            Width::Wide => (0.88, 1.7),
        }
    }
}

/// How heavy letters are drawn: thin fonts look faint at small sizes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Weight {
    #[default]
    Normal,
    /// Edges filled in: the same shapes, brighter.
    Bright,
    /// Thickened by a pixel, and brighter.
    Bold,
}

impl Weight {
    pub const ALL: [Weight; 3] = [Weight::Normal, Weight::Bright, Weight::Bold];

    pub fn name(self) -> &'static str {
        match self {
            Weight::Normal => "normal",
            Weight::Bright => "bright",
            Weight::Bold => "bold",
        }
    }

    /// Draw the coverage mask `m` (`s` pixels square) this heavy.
    pub fn apply(self, m: &mut [u8], s: usize) {
        if self == Weight::Bold {
            // Each pixel takes the stronger of itself and its left neighbour:
            // every stroke a pixel wider.
            for row in m.chunks_mut(s) {
                for x in (1..row.len()).rev() {
                    row[x] = row[x].max(row[x - 1]);
                }
            }
        }
        if self != Weight::Normal {
            for v in m.iter_mut() {
                if *v != 0 && *v != 255 {
                    *v = (255.0 * (*v as f32 / 255.0).powf(0.6)).round() as u8;
                }
            }
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Display {
    /// The text font's file; None for the first of the usual ones found.
    pub font: Option<String>,
    /// The cell size in pixels; None to fit the screen's scaling.
    pub cell: Option<usize>,
    pub width: Width,
    pub weight: Weight,
    /// The adventure map's tiles in pixels; None for a size to suit the cells.
    pub tile: Option<usize>,
}

impl Display {
    /// The saved settings, or the defaults if there are none (or they can't be read).
    pub fn load(dir: &Path) -> Display {
        std::fs::read_to_string(dir.join(SETTINGS_FILE)).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn save(&self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        std::fs::write(dir.join(SETTINGS_FILE), serde_json::to_string_pretty(self).map_err(std::io::Error::other)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_survive_a_round_trip() {
        let dir = std::env::temp_dir().join(format!("dwarfclone-display-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(Display::load(&dir), Display::default(), "nothing saved: the defaults");
        let d = Display { font: Some("C:\\Windows\\Fonts\\consola.ttf".into()), cell: Some(20), width: Width::Wide, weight: Weight::Bold, tile: Some(40) };
        d.save(&dir).unwrap();
        assert_eq!(Display::load(&dir), d);
        // An older or hand-edited file: what's missing takes its default.
        std::fs::write(dir.join(SETTINGS_FILE), r#"{ "width": "narrow" }"#).unwrap();
        assert_eq!(Display::load(&dir), Display { width: Width::Narrow, ..Display::default() });
        std::fs::write(dir.join(SETTINGS_FILE), "not json").unwrap();
        assert_eq!(Display::load(&dir), Display::default());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn weight_brightens_and_thickens() {
        let s = 4;
        let base = vec![0, 100, 0, 0, 0, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut m = base.clone();
        Weight::Normal.apply(&mut m, s);
        assert_eq!(m, base);
        Weight::Bright.apply(&mut m, s);
        assert!(m[1] > 100 && m[5] == 255 && m[2] == 0);
        let mut m = base.clone();
        Weight::Bold.apply(&mut m, s);
        assert_eq!(m[6], 255, "a stroke a pixel wider");
        assert!(m[2] > 100);
        assert_eq!(m[4], 0, "nothing wraps to the next row");
    }
}
