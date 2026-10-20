//! The wild land of a cell (A4: its region's wildlife and a den). W0: a
//! flat open field of grass with a few trees, so the turn engine has
//! somewhere to run.

use super::super::{Adventure, SiteKey};
use crate::geom::Pos;
use crate::material::Mat;
use crate::rng::Rng;
use crate::world::{Tile, TileKind, World};

/// The field's size, and the level it is on (rock below, sky above).
pub const W: i32 = 48;
pub const H: i32 = 48;
pub const Z: i32 = 1;

pub fn build(_adv: &Adventure, _key: SiteKey, seed: u64) -> (World, Pos) {
    flat(seed)
}

/// A flat field: grass on rock under open sky, a few trees away from the
/// middle, where one arrives.
pub fn flat(seed: u64) -> (World, Pos) {
    let tile = |kind: TileKind, mat: Mat| Tile { kind, mat, revealed: true, ..Tile::default() };
    let n = (W * H) as usize;
    let mut tiles = Vec::with_capacity(n * 3);
    tiles.extend(std::iter::repeat(tile(TileKind::Wall, Mat::Granite)).take(n));
    tiles.extend(std::iter::repeat(tile(TileKind::Floor, Mat::Grass)).take(n));
    tiles.extend(std::iter::repeat(tile(TileKind::Open, Mat::Air)).take(n));
    let mut w = World {
        w: W,
        h: H,
        d: 3,
        tiles,
        surface: vec![Z; n],
        caverns: Vec::new(),
        restrict: 0,
        aquifer: 0,
        origin: (0, 0),
        base: None,
        water_level: 0,
        features: 0,
        gen_seed: seed,
        biomes: Vec::new(),
        river_beds: Vec::new(),
    };
    let mid = Pos::new(W / 2, H / 2, Z);
    let mut rng = Rng::new(seed ^ 0x51_7E);
    for _ in 0..24 {
        let (x, y) = (rng.range(1, W - 1), rng.range(1, H - 1));
        if (x - mid.x).abs() + (y - mid.y).abs() > 12 {
            *w.t_mut(Pos::new(x, y, Z)) = tile(TileKind::Tree, Mat::Wood);
        }
    }
    (w, mid)
}
