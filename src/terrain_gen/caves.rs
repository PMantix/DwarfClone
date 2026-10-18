//! Caves in the upper rock: winding tunnels worn by old water, some
//! opening onto a cliff face or a valley side, and the lava tubes a
//! volcano leaves when its flows drain away.

use crate::geom::{Pos, DIRS8};
use crate::material::Mat;
use crate::noise::Noise;
use crate::rng::Rng;
use crate::world::{TileKind, World};

/// One tunnel: where it starts, which way it heads, how long and wide.
struct Worm {
    x: f32,
    y: f32,
    z: f32,
    yaw: f32,
    len: usize,
    r: f32,
    lava: bool,
}

/// Bore the caves. `count` tunnels in all; none comes within `keep_clear`
/// of `centre` (where the fortress goes), none cuts through the surface
/// except at its mouth, none reaches down to `floor_z`.
pub fn bore(world: &mut World, seed: u64, count: usize, centre: (i32, i32), keep_clear: i32, floor_z: i32, volcano: Option<(i32, i32)>) -> usize {
    let (w, h) = (world.w, world.h);
    let mut rng = Rng::new(seed ^ 0xCA7E);
    let turn_n = Noise::new(seed ^ 0xCA7F);
    let mut worms: Vec<Worm> = Vec::new();
    // Mouths on cliff faces and steep valley sides, looking into the hill.
    let mut faces: Vec<(i32, i32, i32, f32)> = Vec::new();
    for y in 3..h - 3 {
        for x in 3..w - 3 {
            let s = world.surface_z(x, y);
            let (mut lo, mut dir) = (s, (0, 0));
            for &(dx, dy) in DIRS8.iter() {
                let n = world.surface_z(x + dx, y + dy);
                if n < lo {
                    lo = n;
                    dir = (dx, dy);
                }
            }
            let dry = world.t(Pos::new(x + dir.0, y + dir.1, lo)).water == 0;
            if s - lo >= 2 && lo - 1 > floor_z && dry && world.t(Pos::new(x - dir.0, y - dir.1, lo)).kind == TileKind::Wall {
                // Facing out of the face: the tunnel runs the other way.
                let yaw = ((-dir.1) as f32).atan2((-dir.0) as f32);
                faces.push((x, y, lo, yaw));
            }
        }
    }
    let far = |x: i32, y: i32| (x - centre.0).pow(2) + (y - centre.1).pow(2) > keep_clear * keep_clear;
    faces.retain(|f| far(f.0, f.1));
    for k in 0..count {
        let from_face = !faces.is_empty() && (k % 2 == 0 || rng.chance(0.4));
        let worm = if from_face {
            let f = faces[rng.range(0, faces.len() as i32) as usize];
            Worm { x: f.0 as f32, y: f.1 as f32, z: f.2 as f32 + 0.2, yaw: f.3, len: rng.range(30, 90) as usize, r: 0.9 + rng.f64() as f32 * 0.6, lava: false }
        } else {
            let (x, y) = (rng.range(4, w - 4), rng.range(4, h - 4));
            if !far(x, y) {
                continue;
            }
            let s = world.surface_z(x, y);
            let z = s - rng.range(3, 7);
            if z <= floor_z + 1 {
                continue;
            }
            Worm { x: x as f32, y: y as f32, z: z as f32, yaw: rng.f64() as f32 * std::f32::consts::TAU, len: rng.range(40, 110) as usize, r: 0.8 + rng.f64() as f32 * 0.7, lava: false }
        };
        worms.push(worm);
    }
    // Lava tubes run out from under a volcano.
    if let Some((vx, vy)) = volcano {
        if vx > 2 && vy > 2 && vx < w - 3 && vy < h - 3 {
            let rim = world.surface_z(vx, vy);
            for k in 0..rng.range(2, 5) {
                let yaw = k as f32 * 1.7 + rng.f64() as f32;
                worms.push(Worm { x: vx as f32 + yaw.cos() * 4.0, y: vy as f32 + yaw.sin() * 4.0, z: (rim - 5).max(floor_z + 2) as f32, yaw, len: rng.range(25, 50) as usize, r: 1.3, lava: true });
            }
        }
    }
    let mut carved: Vec<Pos> = Vec::new();
    for (wi, mut m) in worms.into_iter().enumerate() {
        let mut pitch = if m.lava { 0.02 } else { -0.05 };
        for step in 0..m.len {
            let t = step as f64 * 0.11 + wi as f64 * 13.7;
            m.yaw += ((turn_n.fbm2(t, 0.5, 2) - 0.5) * 1.1) as f32;
            pitch = (pitch + ((turn_n.fbm2(t, 7.5, 2) - 0.5) * 0.12) as f32).clamp(-0.25, 0.12);
            m.x += m.yaw.cos() * 0.8;
            m.y += m.yaw.sin() * 0.8;
            m.z += pitch;
            let (xi, yi) = (m.x.round() as i32, m.y.round() as i32);
            if xi < 2 || yi < 2 || xi >= w - 2 || yi >= h - 2 || !far(xi, yi) || (m.z as i32) <= floor_z {
                break;
            }
            // Now and then the tunnel opens into a chamber.
            let r = if !m.lava && step % 29 == 17 { m.r + 1.2 } else { m.r };
            let ri = r.ceil() as i32;
            for dy in -ri..=ri {
                for dx in -ri..=ri {
                    for dz in 0..=1 {
                        let (x, y, z) = (xi + dx, yi + dy, m.z.floor() as i32 + dz);
                        let hz = if dz == 1 { 0.9 } else { 0.0 };
                        if (dx * dx + dy * dy) as f32 + hz * hz > r * r || dz == 1 && r < 1.6 {
                            continue;
                        }
                        if x < 1 || y < 1 || x >= w - 1 || y >= h - 1 || z <= floor_z || z >= world.d - 1 {
                            continue;
                        }
                        let p = Pos::new(x, y, z);
                        // Keep a roof of rock under the ground (but open out of
                        // a face, where the land beside lies lower).
                        if z >= world.surface_z(x, y) - 1 {
                            continue;
                        }
                        let t = world.t(p);
                        if t.kind != TileKind::Wall || t.mat == Mat::Bedrock || t.aquifer || t.water > 0 || t.magma > 0 {
                            continue;
                        }
                        // Not where water or magma would pour in.
                        let wet = [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1)].iter().any(|&(a, b, c)| world.get(p.offset(a, b, c)).map_or(false, |q| q.water > 0 || q.magma > 0));
                        if wet {
                            continue;
                        }
                        let tm = world.t_mut(p);
                        tm.kind = TileKind::Open;
                        tm.mat = Mat::Air;
                        carved.push(p);
                    }
                }
            }
        }
    }
    // Floors where the tunnel rests on rock, ramps where it steps up, and
    // the mouths open to the sky are known.
    carved.sort();
    carved.dedup();
    for &p in &carved {
        if world.solid(p.down()) {
            let floor = world.t(p.down()).mat;
            let t = world.t_mut(p);
            t.kind = TileKind::Floor;
            t.mat = if volcano.is_some() && floor == Mat::Basalt { Mat::Obsidian } else { floor };
        }
    }
    for &p in &carved {
        if world.t(p).kind == TileKind::Floor && world.kind(p.up()) == Some(TileKind::Open) {
            let step = DIRS8.iter().any(|&(dx, dy)| world.solid(p.offset(dx, dy, 0)) && world.walkable(p.offset(dx, dy, 1)));
            if step {
                world.t_mut(p).kind = TileKind::Ramp;
            }
        }
    }
    carved.len()
}
