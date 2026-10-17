//! Growing the map after embark: new land on any side, continuing the old,
//! with everything in the fortress shifted to its new place.

use super::*;
use crate::jobs::Report;

/// Move every map position (a JSON object with exactly x, y and z) by
/// (dx, dy). World-map coordinates are pairs, so they're left alone.
fn shift_positions(v: &mut serde_json::Value, dx: i64, dy: i64) {
    use serde_json::Value;
    match v {
        Value::Object(m) => {
            if m.len() == 3 && m.contains_key("x") && m.contains_key("y") && m.contains_key("z") {
                if let (Some(x), Some(y)) = (m["x"].as_i64(), m["y"].as_i64()) {
                    m.insert("x".into(), Value::from(x + dx));
                    m.insert("y".into(), Value::from(y + dy));
                    return;
                }
            }
            for (_, child) in m.iter_mut() {
                shift_positions(child, dx, dy);
            }
        }
        Value::Array(a) => {
            for child in a.iter_mut() {
                shift_positions(child, dx, dy);
            }
        }
        _ => {}
    }
}

impl Game {
    fn world_size(&self) -> world::MapSize {
        world::MapSize { w: self.world.w, h: self.world.h, d: self.world.d }
    }

    /// Add land on each side: (north, east, south, west) tiles.
    pub fn grow_map(&mut self, (n, e, s, w): (i32, i32, i32, i32), report: Report) -> Result<String, String> {
        if n < 0 || e < 0 || s < 0 || w < 0 || n + e + s + w == 0 {
            return Err("Nothing to add.".into());
        }
        let (new_w, new_h) = (self.world.w + e + w, self.world.h + n + s);
        if new_w > 320 || new_h > 256 {
            return Err("The map can't grow past 320×256.".into());
        }
        report("Surveying the new land", 0.1);
        let (sx, sy) = self.site;
        let mut params = self.overworld.embark_params(self.world.gen_seed, sx, sy, self.world_size());
        if self.world.biomes.is_empty() {
            // An older map, made before the neighbours blended in: keep it one biome.
            params.around.clear();
        }
        params.aquifer = self.world.aquifer;
        params.features = self.world.features;
        params.size = world::MapSize { w: new_w, h: new_h, d: self.world.d };
        let old_base = self.world.base.unwrap_or((self.world.w, self.world.h));
        let origin = (self.world.origin.0 - w, self.world.origin.1 - n);
        // Older saves didn't keep these: work them out as the map was made.
        if self.world.water_level == 0 && params.lake > 0.0 {
            let mut sorted = self.world.surface.clone();
            sorted.sort_unstable();
            self.world.water_level = sorted[((sorted.len() as f64 * params.lake) as usize).min(sorted.len() - 1)];
        }
        let seed = if self.world.gen_seed != 0 {
            self.world.gen_seed
        } else {
            self.fortress_name.bytes().fold(0x5EED_u64, |h, b| h.wrapping_mul(31).wrapping_add(b as u64))
        };
        report("Raising the new land", 0.3);
        let grown = crate::terrain::generate_region(seed, &params, origin, old_base, Some((&self.world, (w, n))));
        report("Moving the fortress", 0.8);
        // Everything in the fortress moves with the old land.
        if w > 0 || n > 0 {
            let old_world = std::mem::replace(&mut self.world, World::empty());
            let mut v = serde_json::to_value(&*self).map_err(|e| format!("Couldn't grow the map: {}", e))?;
            shift_positions(&mut v, w as i64, n as i64);
            let moved: Game = match serde_json::from_value(v) {
                Ok(g) => g,
                Err(err) => {
                    self.world = old_world;
                    return Err(format!("Couldn't grow the map: {}", err));
                }
            };
            *self = moved;
        }
        self.world = grown;
        self.water_cells = None;
        self.magma_cells = None;
        self.fire_cells = None;
        self.miasma_cells = None;
        self.dig_version += 1;
        let text = format!("The fortress's lands now reach {}×{}.", new_w, new_h);
        self.msg(&text, (170, 220, 170));
        report("Done", 1.0);
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_map_grows_and_everything_moves_with_it() {
        // A site without chasms, mesas and the like, which have cliffs of their own.
        use crate::terrain::feat;
        let rough = feat::CHASM | feat::MESAS | feat::TORS | feat::PEAKS | feat::VOLCANO | feat::GLACIER;
        let mut g = (81..120).map(Game::new).find(|g| g.world.features & rough == 0).expect("a plain site");
        for _ in 0..200 {
            g.step();
        }
        let e = g.embark;
        let d0 = g.dwarves[0].pos;
        let item = *g.items.keys().next().unwrap();
        let ipos = g.items[&item].pos;
        let tile_at = |g: &Game, p: Pos| *g.world.t(p);
        let (w0, h0) = (g.world.w, g.world.h);
        let sample = Pos::new(e.x + 3, e.y + 2, e.z - 2);
        let before = tile_at(&g, sample);
        let surface_before = g.world.surface_z(e.x, e.y);
        g.grow_map((16, 16, 0, 32), &|_, _| {}).expect("it should grow");
        assert_eq!((g.world.w, g.world.h), (w0 + 48, h0 + 16));
        // The old land is exactly where it was, 32 east and 16 south.
        let shift = |p: Pos| p.offset(32, 16, 0);
        assert_eq!(g.embark, shift(e));
        assert_eq!(g.dwarves[0].pos, shift(d0));
        assert_eq!(g.items[&item].pos, shift(ipos));
        assert!(tile_at(&g, shift(sample)) == before, "the old map should be untouched");
        assert_eq!(g.world.surface_z(shift(e).x, shift(e).y), surface_before);
        // The new land meets the old without cliffs.
        for y in 0..g.world.h {
            for x in 0..g.world.w - 1 {
                let (a, b) = (g.world.surface_z(x, y), g.world.surface_z(x + 1, y));
                assert!((a - b).abs() <= 1 || g.world.t(Pos::new(x, y, a)).water > 0 || g.world.t(Pos::new(x + 1, y, b)).water > 0, "a cliff at ({}, {}): {} vs {}", x, y, a, b);
            }
        }
        // And the fortress carries on.
        for _ in 0..200 {
            g.step();
        }
        assert!(g.alive_count() > 0);
        // Growing east and south moves nothing.
        let e2 = g.embark;
        g.grow_map((0, 16, 16, 0), &|_, _| {}).unwrap();
        assert_eq!(g.embark, e2);
    }
}
