//! The world's rivers on the embark map. Each world cell a river passes
//! through is linked to its neighbours (north, east, south, west); on the
//! map the river runs from the middle of each cell to the middle of the
//! side it links across, so it comes in and goes out where the world map
//! says. It flows from the higher cell to the lower, meanders as it goes,
//! and its water level only ever falls along the way. Before the rain
//! weathers the land the river cuts its valley: broad and gentle in soft
//! ground, a gorge where the rock is hard.

use super::field::{cell_hash, fbm, Grid};
use super::Ctx;
use crate::noise::Noise;
use crate::overworld::Biome;

/// A point along a river, in map coordinates, with its water level.
#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub x: f32,
    pub y: f32,
    pub level: f32,
    /// Half the river's width here.
    pub half: f32,
    /// Distance from the start of its reach (for spacing fords).
    pub along: f32,
}

/// One reach, upstream first.
pub struct Reach {
    pub samples: Vec<Sample>,
}

/// The river network over the map.
pub struct Net {
    pub reaches: Vec<Reach>,
}

/// For each column near a river: the nearest sample (reach, index) and how
/// far it lies.
pub struct Near {
    pub dist: Vec<f32>,
    pub at: Vec<(u32, u32)>,
}

const DIRS: [(i32, i32, u8); 4] = [(0, -1, 1), (1, 0, 2), (0, 1, 4), (-1, 0, 8)];

/// Lay out the rivers of every world cell the map touches.
pub fn network(cx: &Ctx, hg: &Grid) -> Net {
    let cell = super::CELL;
    let home = cx.p.home;
    // A site said to have a river with no world round it (made by hand):
    // straight across the map, one way or the other.
    let lone = cx.p.around.is_empty() && cx.p.river == 0 && cx.p.features & crate::terrain::feat::RIVER != 0;
    let across = cell_hash(home.0, home.1, cx.seed ^ 0xC3E4) < 0.5;
    let links = |c: (i32, i32)| -> u8 {
        if lone {
            return match (across, c.1 == home.1, c.0 == home.0) {
                (true, true, _) => 2 | 8,
                (false, _, true) => 1 | 4,
                _ => 0,
            };
        }
        if c == home {
            return cx.p.river | cx.around(c.0, c.1).map_or(0, |a| a.river);
        }
        cx.around(c.0, c.1).map_or(0, |a| a.river)
    };
    let elev = |c: (i32, i32)| -> (i32, i32) {
        // Lower cells are downstream; the sea and the world's edge are lowest.
        match cx.around(c.0, c.1) {
            _ if cx.p.around.is_empty() => (cx.home_elev as i32, -(c.1 * 4096 + c.0)),
            Some(a) if a.biome != Biome::Ocean => (a.elev as i32, c.1 * 4096 + c.0),
            _ => (-1, c.1 * 4096 + c.0),
        }
    };
    // Cells the map touches, and one more all round.
    let (cx0, cy0) = (home.0 + (cx.ox as f64 / cell).floor() as i32 - 1, home.1 + (cx.oy as f64 / cell).floor() as i32 - 1);
    let (cx1, cy1) = (home.0 + ((cx.ox + cx.w) as f64 / cell).floor() as i32 + 1, home.1 + ((cx.oy + cx.h) as f64 / cell).floor() as i32 + 1);
    let centre = |c: (i32, i32)| -> (f32, f32) {
        let jx = (cell_hash(c.0, c.1, 0xC3E1) - 0.5) * 14.0;
        let jy = (cell_hash(c.0, c.1, 0xC3E2) - 0.5) * 14.0;
        (((c.0 - home.0) as f64 * cell + cell / 2.0 + jx) as f32 - cx.ox as f32, ((c.1 - home.1) as f64 * cell + cell / 2.0 + jy) as f32 - cx.oy as f32)
    };
    // The middle of the side two cells share, nudged the same way for both.
    let side = |a: (i32, i32), b: (i32, i32)| -> (f32, f32) {
        let (lo, hi) = if (a.1, a.0) < (b.1, b.0) { (a, b) } else { (b, a) };
        let j = (cell_hash(lo.0 * 3 + hi.0, lo.1 * 3 + hi.1, 0xC3E3) - 0.5) * 20.0;
        if lo.1 == hi.1 {
            (((hi.0 - home.0) as f64 * cell) as f32 - cx.ox as f32, ((lo.1 - home.1) as f64 * cell + cell / 2.0 + j) as f32 - cx.oy as f32)
        } else {
            (((lo.0 - home.0) as f64 * cell + cell / 2.0 + j) as f32 - cx.ox as f32, ((hi.1 - home.1) as f64 * cell) as f32 - cx.oy as f32)
        }
    };
    // Nodes: cell centres and shared sides; edges run downstream.
    let mut nodes: Vec<(f32, f32)> = Vec::new();
    let mut keys: Vec<((i32, i32), (i32, i32))> = Vec::new();
    let mut node = |k: ((i32, i32), (i32, i32)), at: (f32, f32), nodes: &mut Vec<(f32, f32)>| -> usize {
        if let Some(i) = keys.iter().position(|&q| q == k) {
            return i;
        }
        keys.push(k);
        nodes.push(at);
        nodes.len() - 1
    };
    let mut edges: Vec<(usize, usize)> = Vec::new();
    for cy in cy0..=cy1 {
        for cxx in cx0..=cx1 {
            let c = (cxx, cy);
            let l = links(c);
            if l == 0 {
                continue;
            }
            let me = node((c, c), centre(c), &mut nodes);
            for &(dx, dy, bit) in DIRS.iter() {
                if l & bit == 0 {
                    continue;
                }
                let n = (cxx + dx, cy + dy);
                let key = if (c.1, c.0) < (n.1, n.0) { (c, n) } else { (n, c) };
                let mid = node(key, side(c, n), &mut nodes);
                let down = elev(n) < elev(c);
                if down {
                    edges.push((me, mid));
                    // Into the sea (or off the world): run on to its middle.
                    if elev(n).0 < 0 {
                        let sea = node((n, n), centre(n), &mut nodes);
                        if !edges.contains(&(mid, sea)) {
                            edges.push((mid, sea));
                        }
                    }
                } else {
                    edges.push((mid, me));
                }
            }
        }
    }
    edges.sort();
    edges.dedup();

    // The land's level at a point: the heightmap where there is one, the
    // world's own height beyond the map.
    let land = |x: f32, y: f32| -> f32 { hg.at_clamped(x.round() as i32, y.round() as i32) };
    let low_round = |x: f32, y: f32| -> f32 {
        let mut m = f32::MAX;
        for dy in -3..=3 {
            for dx in -3..=3 {
                m = m.min(land(x + dx as f32, y + dy as f32));
            }
        }
        m
    };
    // Each reach as a meandering line of samples.
    let mean_n = Noise::new(cx.seed ^ 0x5A11);
    let wide_n = Noise::new(cx.seed ^ 0x5A12);
    let half_base = if cx.w * cx.h > 12_000 { 2.2 } else { 1.8 };
    let paths: Vec<Vec<(f32, f32)>> = edges
        .iter()
        .enumerate()
        .map(|(ei, &(a, b))| {
            let (ax, ay) = nodes[a];
            let (bx, by) = nodes[b];
            let len = ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt().max(1.0);
            let (px, py) = (-(by - ay) / len, (bx - ax) / len);
            let amp = (len * 0.28).min(11.0);
            let n = (len * 2.0) as usize + 2;
            // Seeded by the edge's place, so the same reach meanders the same
            // on every map that sees it.
            let off = (ax + cx.ox as f32) as f64 * 0.37 + (ay + cx.oy as f32) as f64 * 0.11 + ei as f64 * 0.0;
            (0..=n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    let s = (t * len) as f64;
                    let bend = fbm(&mean_n, s / 19.0 + off, 3.3, 3) as f32 * 2.2 + (s as f32 / 9.0 + off as f32).sin() * 0.25;
                    let m = amp * bend * (std::f32::consts::PI * t).sin();
                    (ax + (bx - ax) * t + px * m, ay + (by - ay) * t + py * m)
                })
                .collect()
        })
        .collect();

    // Levels: the network in order from the springs down, each reach's
    // water falling (never rising) as the land beneath it falls.
    let mut level: Vec<f32> = nodes.iter().map(|&(x, y)| low_round(x, y) - 1.5).collect();
    let mut indeg = vec![0usize; nodes.len()];
    for &(_, b) in &edges {
        indeg[b] += 1;
    }
    let mut ready: Vec<usize> = (0..nodes.len()).filter(|&i| indeg[i] == 0).collect();
    let mut order: Vec<usize> = Vec::new();
    while let Some(n) = ready.pop() {
        for (ei, &(a, b)) in edges.iter().enumerate() {
            if a == n {
                order.push(ei);
                indeg[b] -= 1;
                if indeg[b] == 0 {
                    ready.push(b);
                }
            }
        }
    }
    let mut reaches = Vec::new();
    for &ei in &order {
        let (a, b) = edges[ei];
        let path = &paths[ei];
        let start = level[a];
        let mut run = start;
        let mut lv: Vec<f32> = Vec::with_capacity(path.len());
        for &(x, y) in path {
            run = run.min(land(x, y) - 1.5);
            lv.push(run);
        }
        let end = run.min(level[b]);
        level[b] = end;
        let n = path.len().max(2) - 1;
        let mut samples = Vec::with_capacity(path.len());
        let mut along = 0.0;
        for (i, &(x, y)) in path.iter().enumerate() {
            let t = i as f32 / n as f32;
            if i > 0 {
                along += ((x - path[i - 1].0).powi(2) + (y - path[i - 1].1).powi(2)).sqrt();
            }
            let l = lv[i].min(start + (end - start) * t);
            let half = half_base + fbm(&wide_n, (x + cx.ox as f32) as f64 / 15.0, (y + cx.oy as f32) as f64 / 15.0, 2) as f32 * 1.4;
            samples.push(Sample { x, y, level: l, half: half.max(1.1), along });
        }
        reaches.push(Reach { samples });
    }
    Net { reaches }
}

/// The nearest river sample to every column within `reach` tiles.
pub fn nearest(net: &Net, w: i32, h: i32, reach: f32) -> Near {
    let mut dist = vec![f32::MAX; (w * h) as usize];
    let mut at = vec![(u32::MAX, 0u32); (w * h) as usize];
    let r = reach.ceil() as i32;
    for (ri, rc) in net.reaches.iter().enumerate() {
        for (si, s) in rc.samples.iter().enumerate() {
            let (sx, sy) = (s.x.round() as i32, s.y.round() as i32);
            if sx < -r || sy < -r || sx >= w + r || sy >= h + r {
                continue;
            }
            for y in (sy - r).max(0)..=(sy + r).min(h - 1) {
                for x in (sx - r).max(0)..=(sx + r).min(w - 1) {
                    let d = ((x as f32 - s.x).powi(2) + (y as f32 - s.y).powi(2)).sqrt();
                    let i = (y * w + x) as usize;
                    if d < dist[i] && d <= reach {
                        dist[i] = d;
                        at[i] = (ri as u32, si as u32);
                    }
                }
            }
        }
    }
    Near { dist, at }
}

/// Cut the river's valley into the unweathered land: the channel below
/// its water, the banks rising away from it as steeply as the rock allows.
pub fn carve_valley(net: &Net, near: &Near, hg: &mut Grid, hard: &Grid, fixed: Option<&[bool]>) {
    for i in 0..hg.v.len() {
        if near.at[i].0 == u32::MAX || fixed.map_or(false, |f| f[i]) {
            continue;
        }
        let s = net.reaches[near.at[i].0 as usize].samples[near.at[i].1 as usize];
        let d = near.dist[i];
        let hd = hard.v[i];
        let k = 0.14 + 1.1 * hd * hd;
        let bank = if d <= s.half { s.level - 0.6 } else { s.level + 1.0 + (d - s.half).powf(1.3) * k };
        hg.v[i] = hg.v[i].min(bank);
    }
}
