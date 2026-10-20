//! A lair: a den dug into the high ground, the figure itself, its hoard and the bones of those it killed (A4, A13).
//! W0: the flat field.

use super::super::{Adventure, SiteKey};
use crate::geom::Pos;
use crate::world::World;

pub fn build(adv: &Adventure, key: SiteKey, seed: u64) -> (World, Pos) {
    super::wild::build(adv, key, seed)
}
