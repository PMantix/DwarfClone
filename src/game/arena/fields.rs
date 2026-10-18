//! The arena's fields: a little map, one level of it fought on, with each
//! side's ground to start from (west and east).

use super::super::*;
use super::Field;
use crate::world::{Tile, DOOR_OPEN};

/// The size of every field (it fits beside the panels on a small screen,
/// and the view follows the fighting on a smaller one).
pub const W: i32 = 60;
pub const H: i32 = 22;
/// The level the fighting is on (rock below, sky or roof above).
pub const Z: i32 = 1;

fn tile(kind: TileKind, mat: Mat) -> Tile {
    Tile { kind, mat, revealed: true, ..Tile::default() }
}

fn set(w: &mut World, x: i32, y: i32, t: Tile) {
    let p = Pos::new(x, y, Z);
    if w.in_bounds(p) {
        *w.t_mut(p) = t;
    }
}

/// Open space with nothing under it for a long way.
pub fn chasm(w: &World, p: Pos) -> bool {
    w.in_bounds(p) && p.z == Z && w.t(p).kind == TileKind::Open && w.t(p.down()).kind == TileKind::Open
}

/// The field, and the tiles each side starts on (nearest their middle first).
pub fn build(field: Field, seed: u64) -> (World, [Vec<Pos>; 2]) {
    let mut rng = Rng::new(seed.wrapping_mul(0x2545_F491).wrapping_add(field as u64 * 7919 + 3));
    let n = (W * H) as usize;
    let mut tiles = Vec::with_capacity(n * 3);
    // Rock below, the field, then open sky (a cavern's roof is rock).
    tiles.extend(std::iter::repeat(tile(TileKind::Wall, Mat::Granite)).take(n));
    let (floor, edge, roof) = match field {
        Field::Open => (tile(TileKind::Floor, Mat::Grass), tile(TileKind::Wall, Mat::Wood), tile(TileKind::Open, Mat::Air)),
        Field::Cavern => (tile(TileKind::Floor, Mat::Granite), tile(TileKind::Wall, Mat::Granite), tile(TileKind::Wall, Mat::Granite)),
        Field::Bridge => (tile(TileKind::Floor, Mat::Basalt), tile(TileKind::Wall, Mat::Basalt), tile(TileKind::Open, Mat::Air)),
        Field::Corridor | Field::Yard => (tile(TileKind::Floor, Mat::Granite), tile(TileKind::Wall, Mat::Granite), tile(TileKind::Open, Mat::Air)),
    };
    tiles.extend(std::iter::repeat(floor).take(n));
    tiles.extend(std::iter::repeat(roof).take(n));
    let mut w = World {
        w: W,
        h: H,
        d: 3,
        tiles,
        surface: vec![if field == Field::Cavern { 3 } else { Z }; n],
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
    let rock = tile(TileKind::Wall, edge.mat);
    for x in 0..W {
        set(&mut w, x, 0, edge);
        set(&mut w, x, H - 1, edge);
    }
    for y in 0..H {
        set(&mut w, 0, y, edge);
        set(&mut w, W - 1, y, edge);
    }
    let mid = H / 2;
    match field {
        Field::Open => {
            // A few trees, away from where the sides start.
            for _ in 0..7 {
                let (x, y) = (rng.range(14, W - 14), rng.range(2, H - 2));
                set(&mut w, x, y, tile(TileKind::Tree, Mat::Wood));
            }
        }
        Field::Corridor => {
            // Rock, with a hall at each end and a corridor between.
            for y in 1..H - 1 {
                for x in 1..W - 1 {
                    let hall = (x <= 14 || x >= W - 15) && (3..H - 3).contains(&y);
                    let corridor = y == mid && x > 14 && x < W - 15;
                    if !hall && !corridor {
                        set(&mut w, x, y, rock);
                    }
                }
            }
            let mut door = floor;
            door.door = DOOR_OPEN;
            door.mat = Mat::Wood;
            set(&mut w, W / 2, mid, door);
        }
        Field::Cavern => {
            // Rock grown into caves (a few rounds of "most neighbours rock").
            let mut solid: Vec<bool> = (0..n).map(|_| rng.chance(0.42)).collect();
            let at = |s: &Vec<bool>, x: i32, y: i32| x <= 0 || y <= 0 || x >= W - 1 || y >= H - 1 || s[(y * W + x) as usize];
            for _ in 0..4 {
                let mut next = solid.clone();
                for y in 0..H {
                    for x in 0..W {
                        let k = DIRS8.iter().filter(|&&(dx, dy)| at(&solid, x + dx, y + dy)).count();
                        next[(y * W + x) as usize] = if at(&solid, x, y) { k >= 4 } else { k >= 5 };
                    }
                }
                solid = next;
            }
            for y in 1..H - 1 {
                for x in 1..W - 1 {
                    // Each side's cave mouth stays open, and a winding way runs between.
                    let start = x <= 11 || x >= W - 12;
                    let wind = ((x as f64 / 6.0).sin() * 4.0).round() as i32;
                    let way = (y - mid - wind).abs() <= 1;
                    if solid[(y * W + x) as usize] && !start && !way {
                        set(&mut w, x, y, rock);
                    }
                }
            }
            for y in 1..H - 1 {
                for x in 1..12 {
                    if (y < 4 || y > H - 5) && rng.chance(0.5) {
                        set(&mut w, x, y, rock);
                        set(&mut w, W - 1 - x, y, rock);
                    }
                }
            }
        }
        Field::Bridge => {
            // A chasm down the middle, and a bridge two wide across it.
            for y in 1..H - 1 {
                for x in W / 2 - 5..W / 2 + 5 {
                    let p = Pos::new(x, y, Z);
                    if y == mid || y == mid - 1 {
                        let i = w.idx(p);
                        w.tiles[i] = tile(TileKind::Floor, Mat::Wood);
                        continue;
                    }
                    let i = w.idx(p);
                    w.tiles[i] = tile(TileKind::Open, Mat::Air);
                    let j = w.idx(p.down());
                    w.tiles[j] = tile(TileKind::Open, Mat::Air);
                }
            }
        }
        Field::Yard => {
            // Pillars in rows, and two low walls to hide behind.
            for &(x, y) in &[(20, 5), (20, H - 6), (W - 21, 5), (W - 21, H - 6), (W / 2, 3), (W / 2, H - 4)] {
                set(&mut w, x, y, rock);
                set(&mut w, x + 1, y, rock);
            }
            for y in mid - 3..=mid - 1 {
                set(&mut w, W / 2 - 6, y, rock);
            }
            for y in mid + 1..=mid + 3 {
                set(&mut w, W / 2 + 5, y, rock);
            }
        }
    }
    let starts = [starts(&w, 1, 13), starts(&w, W - 14, W - 2)];
    (w, starts)
}

/// Standing room between columns `x0` and `x1`, nearest the middle first.
fn starts(w: &World, x0: i32, x1: i32) -> Vec<Pos> {
    let (cx, cy) = ((x0 + x1) / 2, H / 2);
    let mut v: Vec<Pos> = (1..H - 1)
        .flat_map(|y| (x0..=x1).map(move |x| Pos::new(x, y, Z)))
        .filter(|&p| w.walkable(p) && w.t(p).door == crate::world::DOOR_NONE)
        .collect();
    v.sort_by_key(|p| ((p.x - cx).pow(2) * 2 + (p.y - cy).pow(2) * 3, p.x, p.y));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_field_has_room_and_a_way_across() {
        for &f in Field::ALL.iter() {
            for seed in 0..12 {
                let (w, s) = build(f, seed);
                assert!(s[0].len() >= 40 && s[1].len() >= 40, "{:?} seed {}: {} and {} places to start", f, seed, s[0].len(), s[1].len());
                let way = path::astar(&w, s[0][0], s[1][0], 20_000, Mover::Goblin);
                assert!(way.is_some(), "{:?} seed {}: no way from one side to the other", f, seed);
            }
        }
        let (w, _) = build(Field::Bridge, 1);
        assert!(chasm(&w, Pos::new(W / 2, 2, Z)));
        assert!(!chasm(&w, Pos::new(W / 2, H / 2, Z)), "the bridge is solid footing");
        let (w, _) = build(Field::Corridor, 1);
        assert_eq!(w.t(Pos::new(W / 2, H / 2, Z)).door, DOOR_OPEN);
        // Beasts can't open it: they must break it down.
        assert!(!w.passable(Pos::new(W / 2, H / 2, Z), Mover::Beast));
    }
}
