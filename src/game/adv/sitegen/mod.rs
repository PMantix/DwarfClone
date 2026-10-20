//! What stands on a site, by what the cell holds: one generator per kind
//! (plan/adventure.md, A4 and A5). W0: every kind is the flat field.

pub mod fallen;
pub mod lair;
pub mod ruin;
pub mod town;
pub mod wild;

use super::{Adventure, SiteKey};
use crate::geom::Pos;
use crate::world::World;

/// The site's map and where the hero arrives on it.
pub fn build(adv: &Adventure, key: SiteKey, seed: u64) -> Result<(World, Pos), String> {
    match key {
        SiteKey::Wild(..) => Ok(wild::build(adv, key, seed)),
        SiteKey::Fort(_) => fallen::build(adv, key, seed),
        SiteKey::Site(id) => {
            let ow = adv.ow();
            let site = ow.sites.iter().find(|s| s.id == id).ok_or_else(|| "No such site.".to_string())?;
            use crate::overworld::SiteKind;
            Ok(match site.kind {
                SiteKind::Lair => lair::build(adv, key, seed),
                _ if site.ruined => ruin::build(adv, key, seed),
                SiteKind::DarkFortress => ruin::build(adv, key, seed),
                _ => town::build(adv, key, seed),
            })
        }
    }
}
