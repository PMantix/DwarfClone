//! The wild land of a cell: the land a fort embarked there would get
//! (`sites::land`), its region's wildlife, and the dens they come out of
//! (`make_lairs`, the fort's own). With no world to stand on (tests, a
//! batch knob `site.flat`) it is a flat open field of grass.

use super::super::{Adventure, SiteKey};
use super::{lore, occupant, Ctx, Made};
use crate::game::Game;
use crate::geom::Pos;
use crate::material::Mat;
use crate::rng::Rng;
use crate::world::{Tile, TileKind, World};

/// The field's size, and the level it is on (rock below, sky above).
pub const W: i32 = 48;
pub const H: i32 = 48;
pub const Z: i32 = 1;

/// The land of the cell the site is in, and where one arrives on it.
pub fn build(adv: &Adventure, key: SiteKey, seed: u64) -> (World, Pos) {
    let ow = adv.ow();
    let cell = match key {
        SiteKey::Wild(x, y) => Some((x, y)),
        SiteKey::Site(id) => ow.sites.iter().find(|s| s.id == id).map(|s| (s.x, s.y)),
        SiteKey::Fort(i) => ow.forts.get(i as usize).map(|f| f.site),
    };
    match cell {
        Some((x, y)) if ow.w > 1 && x >= 0 && y >= 0 && x < ow.w && y < ow.h && adv.knob("site.flat", 0.0) == 0.0 => {
            let w = super::super::sites::land(ow, seed, x, y);
            let at = crate::terrain::embark_spot(&w);
            (w, at)
        }
        _ => flat(seed),
    }
}

/// The wild: its dens, and some of their beasts out about the land.
pub fn make(g: &mut Game, ctx: &Ctx) -> Made {
    let mut made = Made::default();
    let (x, y) = g.site;
    let name = g.fortress_name.clone();
    let (title, story) = lore::wild_story(&g.overworld, x, y, &name);
    made.title = title;
    made.story = story;
    if g.world.w < 24 || g.world.h < 24 {
        return made;
    }
    g.make_lairs(ctx.seed);
    let mut rng = Rng::new(ctx.seed ^ 0x3117_D);
    for i in 0..g.lairs.len() {
        let (pos, sp, cap) = (g.lairs[i].pos, g.lairs[i].species, g.lairs[i].cap);
        let n = if sp.info().aggro == 0 { (cap / 3).max(1) } else { rng.range(1, 3) as u32 }.min(cap);
        for _ in 0..n {
            let p = g.spot_near(pos, 2);
            let id = occupant(g, &mut made, sp, p, None, false);
            if let Some(c) = g.creatures.get_mut(&id) {
                c.lair = Some(i as u32);
            }
            g.lairs[i].pop = g.lairs[i].pop.saturating_sub(1);
        }
    }
    made
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
