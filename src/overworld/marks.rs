//! What the world keeps of the forts that fell in it and of the adventurers
//! who died in it (G03): a `FortMark` for each fallen fort, written once when
//! it falls (`sagas_fall`), and the `Remains` of a dead hero where they lie.
//!
//! `FortMark` is shaped to be Milestone 8's `FortRecord` (same field names):
//! M8 adds its own fields here with `serde(default)` rather than making
//! another record.

use super::{Figure, Overworld};
use crate::items::ItemKind;
use crate::material::Mat;
use serde::{Deserialize, Serialize};

/// A fort that fell, as the world remembers it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FortMark {
    pub name: String,
    /// Its world cell.
    pub site: (i32, i32),
    pub civ: Option<u32>,
    pub founded: i32,
    /// The year it fell (None while it stands: M8's retired forts).
    pub fell: Option<i32>,
    /// What killed the last of it, if anything did ("goblin", "dragon").
    pub killer: Option<String>,
    /// The names of its artifacts, as they were when it fell.
    pub artifacts: Vec<String>,
    /// Its save, in the saves folder.
    pub save_file: String,
    pub peak_pop: usize,
    /// Chronicle titles and the overseer's last notes (A5's books).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub journal: Vec<String>,
    /// What lives in it now (A5; M8's `Occupant`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occupant: Option<Occupant>,
    /// Adventurers who took from it, oldest first (A5 writes the save back).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub robbed: Vec<Robbery>,
}

/// An adventurer took things from a fallen fort.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Robbery {
    /// "Urist Dragonslayer".
    pub by: String,
    pub year: i32,
    /// What was taken, by name.
    pub took: Vec<String>,
}

/// What moved into a fallen fort (M8 P01's variants).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Occupant {
    Army { civ: u32, n: u32 },
    Beast { figure: u32 },
    Horror { label: String },
    Dead,
    Empty,
}

/// Where a dead adventurer lies, with what they carried.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Remains {
    pub name: String,
    /// The world cell.
    pub x: i32,
    pub y: i32,
    /// The site they fell in (None: the wild, or a fort).
    pub site: Option<u32>,
    /// Where on that cell's map.
    pub pos: (i32, i32, i32),
    pub year: i32,
    pub gear: Vec<(ItemKind, Mat, String)>,
    /// Their figure in the legends.
    pub figure: u32,
}

impl Overworld {
    /// The fallen fort on this cell, if any (the latest to fall there).
    pub fn fort_at(&self, x: i32, y: i32) -> Option<usize> {
        self.forts.iter().rposition(|f| f.site == (x, y))
    }

    /// Add a figure made in play (an adventurer, a nemesis), with the next
    /// id (a figure's id is its index); history's own figures are added
    /// while the world is made.
    #[allow(dead_code)] // G03 A10 (the hero's figure), A12 (a nemesis)
    pub fn push_figure(&mut self, mut f: Figure) -> u32 {
        let id = self.figures.len() as u32;
        f.id = id;
        self.figures.push(f);
        id
    }
}
