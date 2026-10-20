//! A living town, hamlet, fortress or retreat: houses, a hall, named folk and guards (A4).
//! W0: the flat field.

use super::super::{Adventure, SiteKey};
use crate::geom::Pos;
use crate::world::World;

pub fn build(adv: &Adventure, key: SiteKey, seed: u64) -> (World, Pos) {
    super::wild::build(adv, key, seed)
}
