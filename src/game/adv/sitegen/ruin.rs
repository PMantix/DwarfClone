//! A ruined site or a dark fortress: broken walls, loot by the old pop, and whoever moved in (A4).
//! W0: the flat field.

use super::super::{Adventure, SiteKey};
use crate::geom::Pos;
use crate::world::World;

pub fn build(adv: &Adventure, key: SiteKey, seed: u64) -> (World, Pos) {
    super::wild::build(adv, key, seed)
}
