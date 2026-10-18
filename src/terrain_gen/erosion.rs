//! Weathering: rain running off the land and carving it (each droplet
//! picks up soil where it speeds downhill and drops it where it slows, so
//! valleys deepen and flats fill with alluvium), then slopes too steep for
//! their rock crumbling to scree until they stand (hard rock holds a
//! cliff, soft earth slumps to a gentle bank).

use super::field::Grid;
use crate::rng::Rng;

/// What the rain did: soil laid down (alluvium, positive) or worn away.
pub struct Wear {
    pub deposit: Grid,
}

/// Hydraulic erosion by droplets: about `per_cell` drops for each column.
pub fn rain(hg: &mut Grid, hard: &Grid, rng: &mut Rng, per_cell: f32, fixed: Option<&[bool]>) -> Wear {
    let (w, h) = (hg.w, hg.h);
    let mut deposit = Grid::new(w, h, 0.0);
    let drops = ((w * h) as f32 * per_cell) as usize;
    const INERTIA: f32 = 0.08;
    const CAPACITY: f32 = 1.2;
    const MIN_SLOPE: f32 = 0.02;
    const DEPOSIT: f32 = 0.25;
    const ERODE: f32 = 0.3;
    const GRAVITY: f32 = 4.0;
    const EVAPORATE: f32 = 0.025;
    const STEPS: usize = 48;
    // The brush a droplet wears the land with: radius 2, weighted to the middle.
    let mut brush: Vec<(i32, i32, f32)> = Vec::new();
    for dy in -2..=2i32 {
        for dx in -2..=2i32 {
            let d = ((dx * dx + dy * dy) as f32).sqrt();
            if d <= 2.0 {
                brush.push((dx, dy, 2.0 - d + 0.3));
            }
        }
    }
    let total: f32 = brush.iter().map(|b| b.2).sum();
    for b in brush.iter_mut() {
        b.2 /= total;
    }
    let frozen = |x: i32, y: i32| fixed.map_or(false, |f| f[(y * w + x) as usize]);
    // Nothing wears deeper than its rock allows: a few levels in soft ground, less in hard.
    let floor: Vec<f32> = (0..hg.v.len()).map(|i| hg.v[i] - (1.0 + 4.0 * (1.0 - hard.v[i]))).collect();
    for _ in 0..drops {
        let mut px = rng.f64() as f32 * (w - 1) as f32;
        let mut py = rng.f64() as f32 * (h - 1) as f32;
        let (mut dx, mut dy) = (0.0f32, 0.0f32);
        let (mut speed, mut water, mut sed) = (1.0f32, 1.0f32, 0.0f32);
        for _ in 0..STEPS {
            let (xi, yi) = (px.floor() as i32, py.floor() as i32);
            let (hh, gx, gy) = hg.sample(px, py);
            dx = dx * INERTIA - gx * (1.0 - INERTIA);
            dy = dy * INERTIA - gy * (1.0 - INERTIA);
            let len = (dx * dx + dy * dy).sqrt();
            if len < 1e-6 {
                let a = rng.f64() as f32 * std::f32::consts::TAU;
                dx = a.cos();
                dy = a.sin();
            } else {
                dx /= len;
                dy /= len;
            }
            let (nx, ny) = (px + dx, py + dy);
            if nx < 0.0 || ny < 0.0 || nx >= (w - 1) as f32 || ny >= (h - 1) as f32 {
                break;
            }
            let nh = hg.sample(nx, ny).0;
            let dh = nh - hh;
            let cap = (-dh).max(MIN_SLOPE) * speed * water * CAPACITY;
            if sed > cap || dh > 0.0 {
                // Slowing, or running into a rise: lay soil down.
                let amt = if dh > 0.0 { dh.min(sed) } else { (sed - cap) * DEPOSIT };
                sed -= amt;
                if !frozen(xi, yi) {
                    hg.splat(px, py, amt);
                    deposit.splat(px, py, amt);
                }
            } else {
                // Speeding downhill: wear the ground away, softer rock faster.
                let hd = hard.at(xi, yi);
                let amt = ((cap - sed) * ERODE * (1.0 - hd * 0.85)).min(-dh * 0.5);
                let mut took = 0.0;
                for &(bx, by, k) in &brush {
                    let (x, y) = (xi + bx, yi + by);
                    if hg.inside(x, y) && !frozen(x, y) {
                        let i = hg.idx(x, y);
                        let cut = (amt * k).min(hg.v[i] - floor[i]).max(0.0);
                        hg.v[i] -= cut;
                        deposit.v[i] -= cut;
                        took += cut;
                    }
                }
                sed += took;
            }
            speed = (speed * speed + dh * -GRAVITY).max(0.0).sqrt();
            water *= 1.0 - EVAPORATE;
            px = nx;
            py = ny;
        }
        // What it still carries settles where it stops (off the map, it's gone).
        if sed > 0.0 && px >= 0.0 && py >= 0.0 && px < (w - 1) as f32 && py < (h - 1) as f32 && !frozen(px as i32, py as i32) {
            hg.splat(px, py, sed);
            deposit.splat(px, py, sed);
        }
    }
    Wear { deposit }
}

/// The steepest a slope of this rock stands, in levels per tile.
pub fn talus(hard: f32) -> f32 {
    0.75 + 3.6 * hard.powf(2.2)
}

/// Thermal weathering: anything steeper than its rock can hold crumbles
/// downhill. Returns where the scree came to rest.
pub fn crumble(hg: &mut Grid, hard: &Grid, rounds: usize, fixed: Option<&[bool]>) -> Grid {
    let (w, h) = (hg.w, hg.h);
    let mut scree = Grid::new(w, h, 0.0);
    let mut delta = vec![0.0f32; hg.v.len()];
    let dirs: [(i32, i32, f32); 8] = [(-1, -1, 1.414), (0, -1, 1.0), (1, -1, 1.414), (-1, 0, 1.0), (1, 0, 1.0), (-1, 1, 1.414), (0, 1, 1.0), (1, 1, 1.414)];
    for _ in 0..rounds {
        delta.iter_mut().for_each(|d| *d = 0.0);
        let mut moved = 0.0f32;
        for y in 0..h {
            for x in 0..w {
                let i = hg.idx(x, y);
                if fixed.map_or(false, |f| f[i]) {
                    continue;
                }
                let here = hg.v[i];
                let t = talus(hard.v[i]);
                // Share what's too steep among the lower neighbours.
                let mut excess = [0.0f32; 8];
                let mut sum = 0.0;
                let mut most = 0.0f32;
                for (k, &(dx, dy, dist)) in dirs.iter().enumerate() {
                    let (nx, ny) = (x + dx, y + dy);
                    if !hg.inside(nx, ny) {
                        continue;
                    }
                    let e = here - hg.at(nx, ny) - t * dist;
                    if e > 0.0 {
                        excess[k] = e;
                        sum += e;
                        most = most.max(e);
                    }
                }
                if sum <= 0.0 {
                    continue;
                }
                let amt = most * 0.45;
                delta[i] -= amt;
                for (k, &(dx, dy, _)) in dirs.iter().enumerate() {
                    if excess[k] > 0.0 {
                        let j = hg.idx(x + dx, y + dy);
                        delta[j] += amt * excess[k] / sum;
                    }
                }
                moved += amt;
            }
        }
        for i in 0..delta.len() {
            if fixed.map_or(false, |f| f[i]) {
                continue;
            }
            hg.v[i] += delta[i];
            if delta[i] > 0.0 {
                scree.v[i] += delta[i];
            }
        }
        if moved < 0.01 {
            break;
        }
    }
    scree
}
