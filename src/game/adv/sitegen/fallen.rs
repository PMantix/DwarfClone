//! A fallen fort: the fort's own save is the site (A5). Entering one loads
//! it on the job thread and makes it a ruin (`ruin_prepare`); leaving one
//! the hero took from writes it back, after a backup of the original
//! (decision 3, plan/adventure.md). W0: the flat field, and `scan` finds
//! nothing yet.
//!
//! A5 note: a fort's site is a whole loaded `Game`, not a `World` built
//! with `from_world`, so A5 gives `sites::enter_from` its `SiteKey::Fort`
//! branch (load, prepare, carry in) rather than going through `build`.

use super::super::{Adventure, SiteKey};
use crate::geom::Pos;
use crate::overworld::Overworld;
use crate::world::World;
use std::path::Path;

pub fn build(adv: &Adventure, key: SiteKey, seed: u64) -> Result<(World, Pos), String> {
    Ok(super::wild::build(adv, key, seed))
}

/// Fallen forts of this world saved before `FortMark`s existed: read the
/// saves folder and add their marks (A5).
pub fn scan(_dir: &Path, _ow: &mut Overworld) -> usize {
    0
}
