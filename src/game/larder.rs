//! The overseer's larder: food whatever the land and the season. Fields sown
//! with what grows there now (edible crops first when food is short), a plot
//! cut under the ground where the surface won't take a field, wild plants
//! gathered, fish and game, the harvest cooked for its seed rather than
//! brewed away, and a shortage seen to while there's still food in hand
//! (the announcement that nobody has anything to eat comes too late).

use super::overseer::{Overseer, Style};
use super::troubles::Lack;
use super::*;
use crate::buildings::Recipe;
use crate::material::CROPS;

/// Food in hand per dwarf under which it counts as short: a dwarf eats about
/// every seven days, so three weeks' worth.
const SHORT_PER_DWARF: usize = 3;

/// What a field hand still does while food is short: the larder's own work,
/// hauling, and tending the wounded. (Building, mining, felling and the
/// workshops come before the fields in a dwarf's day, so a farmer who also
/// builds never gets to them while there's building to do.)
const FIELD_HAND_KEEPS: [Labor; 7] = [Labor::Farming, Labor::Brewing, Labor::Cooking, Labor::Butchery, Labor::Fishing, Labor::Hauling, Labor::Medicine];

/// How far from the wagon wild plants are gathered (further when short).
const GATHER_R: i32 = 18;

/// The size of a field cut under the ground.
const PLOT_W: i32 = 6;
const PLOT_H: i32 = 4;

impl Game {
    /// Food a dwarf could eat now: meals, edible plants, meat and fish.
    pub(super) fn food_in_hand(&self) -> usize {
        self.items.values().filter(|it| it.free() && it.is_food()).count()
    }

    /// Whether the larder is running low for the mouths there are.
    pub(super) fn food_short(&self) -> bool {
        self.food_in_hand() < self.alive_count().max(1) * SHORT_PER_DWARF
    }

    /// While food is short, the farmers kept to the fields (the most
    /// skilled first): `ov_labors` gives them nothing else to do.
    pub(super) fn field_hands(&self) -> Vec<u32> {
        if !self.food_short() {
            return Vec::new();
        }
        let want = 2 + self.alive_count() / 6;
        let mut v: Vec<(u32, u32)> = self
            .dwarves
            .iter()
            .filter(|d| d.alive && !d.soldier && !d.insane && !d.is_child() && d.has(Labor::Farming))
            .map(|d| (u32::MAX - d.skills[Labor::Farming.idx()], d.id))
            .collect();
        v.sort();
        v.into_iter().take(want).map(|x| x.1).collect()
    }

    /// Whether a field hand may keep (or be given) `labor` while food is short.
    pub(super) fn field_hand_may(labor: Labor) -> bool {
        FIELD_HAND_KEEPS.contains(&labor)
    }

    fn season(&self) -> usize {
        ((self.tick / SEASON) % 4) as usize
    }

    /// Daily (`overseer_hooks.rs`).
    pub(super) fn ov_larder(&mut self, o: &mut Overseer) {
        if o.base.is_none() {
            return;
        }
        let short = self.food_short();
        // Seen to while there's still food in hand.
        if short {
            self.trouble(Lack::Food);
            // Field hands to the fields only.
            for id in self.field_hands() {
                for l in crate::dwarf::LABORS {
                    if !Self::field_hand_may(l) && self.dwarves.iter().any(|d| d.id == id && d.has(l)) {
                        self.set_labor(id, l, false);
                    }
                }
            }
        }
        if !self.zones.values().any(|z| z.kind == ZoneKind::Gathering) {
            self.zone_wild_plants(GATHER_R);
        }
        self.sow_for_the_season(short);
        self.seed_from_the_harvest(o);
        self.dig_a_cave_farm(o, false);
    }

    /// What `Lack::Food` does (troubles.rs): whatever gets food into the
    /// larder here, now. The words for what it did.
    pub(super) fn fix_food(&mut self, o: &mut Overseer) -> String {
        let pop = self.alive_count() as u32;
        let tick = self.tick;
        o.boosts.retain(|b| b.0 != Recipe::Cook);
        o.boosts.push((Recipe::Cook, 15 + pop * 2, tick + DAY * 20));
        let mut did: Vec<&str> = Vec::new();
        self.hands_for_food(Labor::Cooking, 1);
        if self.hands_for_food(Labor::Farming, 3 + pop as usize / 6) {
            did.push("put more hands to the fields");
        }
        self.sow_for_the_season(true);
        if self.zone_wild_plants(GATHER_R * 2) > 0 {
            did.push("had wild plants gathered further out");
        }
        if self.water_to_fish() && self.hands_for_food(Labor::Fishing, 2) {
            did.push("sent fishers to the water");
        }
        if o.style != Style::Reckless {
            self.hunt = true;
            if self.buildings.values().all(|b| b.kind != BuildingKind::Butcher) {
                let e = self.embark;
                if let Some(p) = self.ov_spot(BuildingKind::Butcher, e.offset(6, 6, 0), 16) {
                    self.place_building(BuildingKind::Butcher, p);
                    did.push("had a butcher's shop built for the hunters' game");
                }
            } else {
                did.push("sent hunters out");
            }
        }
        if self.dig_a_cave_farm(o, true) {
            did.push("had a field cut under the ground");
        }
        did.push("set the kitchen to cook");
        let n = did.len();
        let mut text = String::new();
        for (i, d) in did.iter().enumerate() {
            if i > 0 {
                text.push_str(if i + 1 == n { " and " } else { ", " });
            }
            text.push_str(d);
        }
        text.push('.');
        text
    }

    /// `troubles::hands_for`, from here.
    fn hands_for_food(&mut self, labor: Labor, n: usize) -> bool {
        let have = self.dwarves.iter().filter(|d| d.alive && !d.insane && !d.is_child() && d.has(labor)).count();
        if have >= n {
            return false;
        }
        let mut cands: Vec<(usize, u32, u32)> = self
            .dwarves
            .iter()
            .filter(|d| d.alive && !d.insane && !d.is_child() && !d.soldier && !d.has(labor) && labor.exclusive_with().map_or(true, |r| !d.has(r)))
            .map(|d| (crate::dwarf::LABORS.iter().filter(|l| d.has(**l)).count(), u32::MAX - d.skills[labor.idx()], d.id))
            .collect();
        cands.sort();
        for (_, _, id) in cands.into_iter().take(n - have) {
            self.set_labor(id, labor, true);
        }
        true
    }

    /// Deep water somewhere on the map to fish from.
    fn water_to_fish(&self) -> bool {
        let w = &self.world;
        (0..w.h).any(|y| (0..w.w).any(|x| (-1..=0).any(|dz| w.get(Pos::new(x, y, w.surface_z(x, y) + dz)).map_or(false, |t| t.water >= 4 && !t.frozen))))
    }

    /// Mark the wild shrubs within `r` of the wagon (on the surface) for
    /// gathering. How many were added.
    fn zone_wild_plants(&mut self, r: i32) -> usize {
        let e = self.embark;
        let mut tiles = Vec::new();
        for y in (e.y - r).max(0)..=(e.y + r).min(self.world.h - 1) {
            for x in (e.x - r).max(0)..=(e.x + r).min(self.world.w - 1) {
                let z0 = self.world.surface_z(x, y);
                for z in z0 - 1..=z0 + 1 {
                    let p = Pos::new(x, y, z);
                    if self.world.get(p).map_or(false, |t| t.kind == TileKind::Shrub && t.zone == 0 && t.revealed && t.water == 0) {
                        tiles.push(p);
                    }
                }
            }
        }
        self.add_zone_tiles(tiles, ZoneKind::Gathering)
    }

    /// Each field to a crop that grows there this season: when food is
    /// short, the edible crop with most seed in hand; else whatever suits
    /// (the farmers' own choice).
    fn sow_for_the_season(&mut self, short: bool) {
        let season = self.season();
        let mut seeds: BTreeMap<Mat, usize> = BTreeMap::new();
        for it in self.items.values().filter(|it| it.kind == ItemKind::Seed && it.free()) {
            *seeds.entry(it.mat).or_default() += 1;
        }
        let outside: HashMap<u32, bool> = self.zones.values().filter(|z| z.kind == ZoneKind::Farm).filter_map(|z| z.tiles.first().map(|&p| (z.id, self.world.outside(p)))).collect();
        for z in self.zones.values_mut().filter(|z| z.kind == ZoneKind::Farm) {
            let Some(&out) = outside.get(&z.id) else { continue };
            let best = CROPS
                .iter()
                .copied()
                .filter(|m| m.crop().map_or(false, |c| c.edible && c.grows(out, season)) && seeds.get(m).map_or(false, |&n| n > 0))
                .max_by_key(|m| seeds[m]);
            z.crop = if short { best } else { None };
        }
    }

    /// Fields lying fallow for want of seed that grows there now, and plants
    /// of such a crop in the stores: cook them (the seed comes back).
    fn seed_from_the_harvest(&mut self, o: &mut Overseer) {
        let season = self.season();
        let mut fallow: Vec<bool> = Vec::new();
        for z in self.zones.values().filter(|z| z.kind == ZoneKind::Farm) {
            let Some(&first) = z.tiles.first() else { continue };
            if z.tiles.iter().any(|&p| self.world.t(p).farm == 0) {
                fallow.push(self.world.outside(first));
            }
        }
        if fallow.is_empty() {
            return;
        }
        let grows = |m: Mat, out: bool| m.crop().map_or(false, |c| c.grows(out, season));
        let seed = |m: Mat| self.items.values().filter(|it| it.kind == ItemKind::Seed && it.mat == m && it.free()).count();
        let wanting = fallow.iter().any(|&out| CROPS.iter().all(|&m| !grows(m, out) || seed(m) < 3));
        let plants = self
            .items
            .values()
            .filter(|it| it.kind == ItemKind::Plant && it.free() && it.is_food() && fallow.iter().any(|&out| grows(it.mat, out)))
            .count() as u32;
        if wanting && plants >= 2 && !o.boosts.iter().any(|b| b.0 == Recipe::Cook) {
            let meals = self.stock_of(ItemKind::Meal) as u32;
            o.boosts.push((Recipe::Cook, meals + plants.min(20) * 2, self.tick + DAY * 5));
        }
    }

    /// A field under the ground, cut in soil by the halls, when the surface
    /// has too little field for the fortress (rock or sand to the horizon,
    /// or a wide river), or when food is short with the fields outside out
    /// of season. Once dug it is sown. Whether a plot was begun.
    fn dig_a_cave_farm(&mut self, o: &mut Overseer, short: bool) -> bool {
        let Some(base) = o.base else { return false };
        // A plot under way: sow it once it's dug.
        if let Some(i) = o.done.iter().position(|d| d.starts_with("caveplot ")) {
            let v: Vec<i32> = o.done[i]["caveplot ".len()..].split(',').filter_map(|s| s.parse().ok()).collect();
            if v.len() == 3 {
                let (a, b) = (Pos::new(v[0], v[1], v[2]), Pos::new(v[0] + PLOT_W - 1, v[1] + PLOT_H - 1, v[2]));
                if box_iter(a, b).all(|p| self.world.get(p).map_or(true, |t| !t.kind.solid())) {
                    o.done.remove(i);
                    if self.add_zone(a, b, ZoneKind::Farm) > 0 {
                        o.done.push("cavefarm".into());
                        Self::note_ov(o, self.tick, "Sowed a field under the ground.".into());
                    }
                } else if box_iter(a, b).all(|p| self.world.get(p).map_or(true, |t| t.desig == Desig::None || !t.kind.solid())) && box_iter(a, b).any(|p| self.world.t(p).kind.solid()) {
                    // The designations were lost (pruned): mark it again.
                    self.designate(a, b, Desig::Dig);
                }
            }
            return false;
        }
        if o.did("cavefarm") || o.did("nocaveplot") {
            return false;
        }
        let pop = self.alive_count().max(1);
        let season = self.season();
        let outside_farm: usize = self.zones.values().filter(|z| z.kind == ZoneKind::Farm && z.tiles.first().map_or(false, |&p| self.world.outside(p))).map(|z| z.tiles.len()).sum();
        // Only stoutcaps (and grain and reed) grow under the ground; fields
        // outside grow nothing edible in winter but stoutcaps either.
        let out_of_season = !CROPS.iter().any(|&m| m.crop().map_or(false, |c| c.edible && c.grows(true, season) && m != Mat::Stoutcap));
        if outside_farm >= pop * 3 && !(short && out_of_season) {
            return false;
        }
        let Some((a, b)) = self.cave_plot(base) else {
            o.done.push("nocaveplot".into());
            return false;
        };
        self.designate(a, b, Desig::Dig);
        o.done.push(format!("caveplot {},{},{}", a.x, a.y, a.z));
        Self::note_ov(o, self.tick, "Too little field under the sky: had a plot cut in the soil by the halls, to farm under the ground.".into());
        true
    }

    /// A block of soil by the halls to cut a field from: on the halls' level
    /// or the one under it, beside ground already dug, nowhere near water,
    /// the sky or the gatehouse. Its corners.
    fn cave_plot(&self, base: Pos) -> Option<(Pos, Pos)> {
        let w = &self.world;
        let soil = |p: Pos| w.get(p).map_or(false, |t| t.kind == TileKind::Wall && t.mat.is_soil() && !t.aquifer && t.water == 0 && t.desig == Desig::None) && !w.outside(p);
        let mut best: Option<(i32, Pos)> = None;
        for z in [base.z, base.z - 1] {
            for y in (base.y - 30).max(1)..=(base.y + 30).min(w.h - PLOT_H - 1) {
                for x in (base.x - 20).max(1)..=(base.x + 60).min(w.w - PLOT_W - 1) {
                    let a = Pos::new(x, y, z);
                    let d = (x - base.x).abs() + (y - base.y).abs() + (base.z - z) * 10;
                    if best.map_or(false, |b| d >= b.0) {
                        continue;
                    }
                    let b = a.offset(PLOT_W - 1, PLOT_H - 1, 0);
                    // Clear of the gatehouse, and a ring of soil or rock round it
                    // (no water or sky beside it), with dug floor on one side.
                    if box_iter(a.offset(-2, -2, 0), b.offset(2, 2, 0)).any(|p| Self::gatehouse(base, p)) {
                        continue;
                    }
                    if !box_iter(a, b).all(soil) {
                        continue;
                    }
                    let ring: Vec<Pos> = box_iter(a.offset(-1, -1, 0), b.offset(1, 1, 0)).filter(|p| p.x < a.x || p.x > b.x || p.y < a.y || p.y > b.y).collect();
                    if ring.iter().any(|&p| w.get(p).map_or(true, |t| t.water > 0 || t.magma > 0) || w.outside(p)) {
                        continue;
                    }
                    if !ring.iter().any(|&p| w.get(p).map_or(false, |t| t.dug && w.walkable(p))) {
                        continue;
                    }
                    // Solid under it, so the field doesn't hang over a cave.
                    if !box_iter(a.down(), b.down()).all(|p| w.solid(p)) {
                        continue;
                    }
                    best = Some((d, a));
                }
            }
        }
        best.map(|(_, a)| (a, a.offset(PLOT_W - 1, PLOT_H - 1, 0)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::overseer::Style;

    #[test]
    fn a_short_larder_is_seen_to_before_the_food_runs_out() {
        let mut g = Game::new(5);
        g.set_overseer(Some(Style::Steady));
        for _ in 0..DAY {
            g.step();
        }
        // Most of the food gone, some left.
        let pop = g.alive_count();
        let food: Vec<u32> = g.items.values().filter(|it| it.is_food()).map(|it| it.id).collect();
        for id in food.iter().skip(pop) {
            g.items.remove(id);
        }
        assert!(g.food_short() && g.food_in_hand() > 0);
        let mut o = g.overseer.take().unwrap();
        g.ov_larder(&mut o);
        g.overseer = Some(o);
        assert!(g.troubles.iter().any(|t| t.0 == Lack::Food), "a short larder is a trouble before anyone goes hungry");
        // And seeing to it puts edible crops in the fields and gathers wild plants.
        let mut o = g.overseer.take().unwrap();
        let said = g.fix_food(&mut o);
        g.overseer = Some(o);
        assert!(said.contains("cook"), "{}", said);
        let season = ((g.tick / SEASON) % 4) as usize;
        for z in g.zones.values().filter(|z| z.kind == ZoneKind::Farm) {
            if let Some(m) = z.crop {
                let out = g.world.outside(z.tiles[0]);
                assert!(m.crop().map_or(false, |c| c.edible && c.grows(out, season)), "{:?} doesn't grow there now", m);
            }
        }
        assert!(g.zones.values().any(|z| z.kind == ZoneKind::Gathering) || !g.world.tiles.iter().any(|t| t.kind == TileKind::Shrub));
    }
}
