//! Following the vein (O10, DIG-11/12; package P10 fills this in): the
//! miners follow a vein of ore or gems the way a player designates one, and
//! what digging uncovers is noted as a find. Wave 0 gives the state and the
//! filter; `follow_vein` and the finds come with P10.

use super::*;
use serde::{Deserialize, Serialize};

/// What a vein designation follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VeinFilter {
    /// One ore (`Some`), or any ore (`None`).
    Ore(Option<Mat>),
    Gem,
}

/// The fort's veins (`Game.veins`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Veins {
    /// Dug tiles whose vein is still to be spread along, with its filter.
    #[serde(default)]
    pub follow: Vec<(Pos, VeinFilter)>,
    /// Ore and gems revealed by digging and not yet mined or followed.
    #[serde(default)]
    pub found: Vec<Pos>,
}
