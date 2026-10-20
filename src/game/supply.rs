//! Orders that think (O08; packages P9 and P18 fill this in): what feeds a
//! recipe, where each input comes from, and the ledger of what the fort
//! made and used. Wave 0 gives the ledger's place in `Game`; `sources`,
//! `have` and `feed` come with P9, the ledger's rows with P18.

use serde::{Deserialize, Serialize};

/// Records of production (`Game.ledger`, ECO-15).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Ledger {
    /// (season index, what, made, used), newest last.
    #[serde(default)]
    pub rows: Vec<(u32, String, u32, u32)>,
}
