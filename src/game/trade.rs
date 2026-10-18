//! Caravans, the trade depot and bartering.

use super::*;
use crate::creature::Species;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Caravan {
    pub civ: u32,
    pub race: Race,
    pub civ_name: String,
    /// Goods for sale; they only enter the world when bought.
    pub goods: Vec<Item>,
    pub merchants: Vec<u32>,
    pub depot: u32,
    pub arrived: bool,
    pub leave_tick: u64,
}

/// Whether merchants will take an item off our hands.
pub fn tradeable(it: &Item) -> bool {
    use ItemKind::*;
    it.value() > 0
        && !matches!(it.kind, Corpse | Carcass | Seed)
        && it.carried_by.is_none()
}

impl Game {
    pub(super) fn legend(&mut self, text: String) {
        let year = self.start_year + (self.tick / YEAR) as i32;
        self.overworld.log(year, text);
    }

    pub fn depot(&self) -> Option<&Building> {
        self.buildings.values().find(|b| b.kind == BuildingKind::Depot && b.built)
    }

    pub fn at_depot(&self, p: Pos) -> bool {
        self.depot().map_or(false, |d| d.tiles().contains(&p))
    }

    pub(super) fn caravan_goods(&mut self, race: Race) -> Vec<Item> {
        use ItemKind::*;
        let list: Vec<(ItemKind, Mat, u32)> = match race {
            Race::Dwarf => vec![
                (Bar, Mat::Iron, 4),
                (Bar, Mat::Steel, 2),
                (Bar, Mat::Copper, 3),
                (Pick, Mat::Iron, 1),
                (Axe, Mat::Iron, 2),
                (Helm, Mat::Iron, 2),
                (Breastplate, Mat::Iron, 1),
                (Meal, Mat::Stoutcap, 6),
                (Drink, Mat::Stoutcap, 10),
                (Seed, Mat::Stoutcap, 6),
                (Charcoal, Mat::Coal, 5),
                (Barrel, Mat::Wood, 2),
            ],
            Race::Human => vec![
                (Meat, Mat::Flesh, 8),
                (Meal, Mat::Stoutcap, 6),
                (Log, Mat::Wood, 8),
                (Leather, Mat::Leather, 4),
                (Bar, Mat::Bronze, 3),
                (Spear, Mat::Bronze, 2),
                (Shield, Mat::Bronze, 2),
                (Bin, Mat::Wood, 2),
            ],
            _ => vec![
                (Drink, Mat::Stoutcap, 8),
                (Meal, Mat::Stoutcap, 4),
                (Seed, Mat::Stoutcap, 8),
                (Leather, Mat::Leather, 3),
                (Gem, Mat::Amethyst, 1),
            ],
        };
        let mut goods = Vec::new();
        for (kind, mat, n) in list {
            for _ in 0..n {
                let id = self.new_id();
                let label = match kind {
                    Meat | Leather => Some(if race == Race::Human { "cow".to_string() } else { "elk".to_string() }),
                    _ => None,
                };
                goods.push(Item {
                    id,
                    kind,
                    mat,
                    pos: Pos::default(),
                    carried_by: None,
                    claimed_by: None,
                    label,
                    container: None,
                    quality: self.rng.range(0, 3) as u8,
                    to_depot: false,
                    dump: false,
                    wear: 0,
                    forbidden: false,
                });
            }
        }
        goods
    }

    /// Spring brings elves, summer humans, autumn your own dwarves.
    pub(super) fn caravan_season(&mut self, season: u64) {
        if self.caravan.is_some() {
            return;
        }
        let want = match season {
            0 => Race::Elf,
            1 => Race::Human,
            2 => Race::Dwarf,
            _ => return,
        };
        let civ = if want == Race::Dwarf {
            self.civ
        } else {
            // (A people made by a villain keeps its own season: kin.rs.)
            self.neighbors.iter().find(|n| n.race == want && n.dist <= 30 && self.overworld.made_of(n.civ).is_none()).map(|n| n.civ)
        };
        let Some(civ) = civ else { return };
        if !self.overworld.civ(civ).alive {
            return;
        }
        if let Some(home) = self.civ {
            if self.overworld.civ(home).enemies.contains(&civ) {
                return;
            }
        }
        let civ_name = self.overworld.civ(civ).name.clone();
        let Some(depot) = self.depot().map(|d| d.id) else {
            self.msg(
                &format!("A caravan from the {} arrived, but with no trade depot it moved on.", civ_name),
                (230, 200, 90),
            );
            return;
        };
        let Some(entry) = self.edge_entry() else { return };
        let goods = self.caravan_goods(want);
        let mut merchants = Vec::new();
        for _ in 0..2 {
            let p = self.random_spot_near(entry, 1);
            let id = self.spawn_creature(Species::Merchant, p, DAY * 9);
            merchants.push(id);
        }
        self.msg(&format!("A caravan from the {} ({}) approaches!", civ_name, want.plural()), (240, 240, 160));
        self.caravan = Some(Caravan {
            civ,
            race: want,
            civ_name,
            goods,
            merchants,
            depot,
            arrived: false,
            leave_tick: self.tick + DAY * 9,
        });
        self.pause_request = true;
    }

    pub(super) fn update_caravan(&mut self) {
        let Some(c) = &self.caravan else { return };
        let alive = c.merchants.iter().any(|m| self.creatures.contains_key(m));
        if !alive {
            self.caravan = None;
            return;
        }
        if self.tick >= c.leave_tick {
            let ids = c.merchants.clone();
            for m in ids {
                if let Some(cr) = self.creatures.get_mut(&m) {
                    cr.leaving = true;
                }
            }
            if self.caravan.as_ref().unwrap().arrived {
                self.caravan.as_mut().unwrap().arrived = false;
                self.msg("The merchants are packing up to leave.", (240, 240, 160));
                self.word_with_caravan();
            }
        }
    }

    pub(super) fn merchant_think(&mut self, c: &mut Creature) {
        let Some(car) = &self.caravan else {
            c.leaving = true;
            return;
        };
        let Some(depot) = self.buildings.get(&car.depot).filter(|b| b.built).map(|b| b.center()) else {
            c.leaving = true;
            self.caravan.as_mut().unwrap().leave_tick = self.tick;
            return;
        };
        if self.tick >= car.leave_tick {
            return;
        }
        if adjacent_or_on(c.pos, depot) {
            if !car.arrived {
                let name = car.civ_name.clone();
                self.caravan.as_mut().unwrap().arrived = true;
                self.caravan.as_mut().unwrap().leave_tick = self.tick + DAY * 6;
                self.msg(&format!("Merchants from the {} have arrived at the depot.", name), (240, 240, 160));
                self.spotlight("Traders", &format!("Merchants from the {} have arrived at the depot. They'll stay six days: press t at the depot to trade.", name), Some(depot), (240, 240, 160));
            }
            c.path.clear();
            return;
        }
        if c.path.is_empty() {
            match path::astar(&self.world, c.pos, depot, SEARCH_LIMIT, Mover::Dwarf) {
                Some(p) => c.path = p.into_iter().rev().collect(),
                None => {
                    self.warn("depotpath", "The merchants cannot find a path to the trade depot!", (255, 150, 80));
                    c.think_cd = 200;
                }
            }
        }
    }

    pub(super) fn merchant_died(&mut self, c: &Creature) {
        let Some(car) = self.caravan.as_mut() else { return };
        car.merchants.retain(|&m| m != c.id);
        if car.merchants.is_empty() {
            // The wares spill where the last merchant fell.
            let goods = std::mem::take(&mut car.goods);
            let name = car.civ_name.clone();
            self.caravan = None;
            for mut it in goods {
                it.pos = self.world.settle(c.pos);
                self.items.insert(it.id, it);
            }
            self.msg(&format!("The caravan from the {} has been destroyed!", name), (255, 90, 90));
        }
    }

    /// Mark loose goods for the depot. Returns how many were marked.
    pub fn mark_for_trade(&mut self, on: bool) -> usize {
        use ItemKind::*;
        let mut n = 0;
        for it in self.items.values_mut() {
            let goods = matches!(it.kind, Craft | Gem | Bar | Block | Leather | Bed | Table | Chair | Door)
                || it.kind.is_equipment()
                || it.quality > 0;
            if on && goods && it.free() && it.quality < crate::items::ARTIFACT {
                if !it.to_depot {
                    it.to_depot = true;
                    it.container = None;
                    n += 1;
                }
            } else if !on && it.to_depot {
                it.to_depot = false;
                n += 1;
            }
        }
        n
    }

    pub(super) fn find_depot_haul(&mut self, d: &mut Dwarf) -> bool {
        let Some(tiles) = self.depot().map(|b| b.tiles()) else { return false };
        let tiles: HashSet<Pos> = tiles.into_iter().filter(|p| self.world.walkable(*p)).collect();
        let Some((item, path)) = self.find_item(d.pos, |it| it.to_depot && !tiles.contains(&it.pos)) else {
            return false;
        };
        let ipos = self.items[&item].pos;
        let Some((dest, _)) = path::search(&self.world, ipos, SEARCH_LIMIT, Mover::Dwarf, |p| tiles.contains(&p)) else {
            return false;
        };
        self.claim_item(item, d.id);
        d.task = Task::Haul { item, dest, into: None };
        Self::set_path(d, path);
        true
    }

    /// Goods of ours sitting in the depot, available to trade.
    pub fn depot_goods(&self) -> Vec<&Item> {
        let Some(b) = self.depot() else { return Vec::new() };
        let tiles = b.tiles();
        let mut v: Vec<&Item> = self
            .items
            .values()
            .filter(|it| tiles.contains(&it.pos) && tradeable(it) && it.container.is_none())
            .collect();
        v.sort_by_key(|it| std::cmp::Reverse(it.value()));
        v
    }

    /// Offer `ours` (item ids at the depot) for `theirs` (caravan goods ids).
    /// Merchants want a tenth on top. Returns a message either way.
    /// The percentage over value merchants insist on: a tenth, or less with
    /// a broker to haggle (a sociable, even-tempered one gets them down to nothing).
    pub fn trade_margin(&self) -> u32 {
        match self.holder(super::Position::Broker) {
            Some(b) => {
                let score = self.post_suitability(super::Position::Broker, b).0;
                10 - ((score + 40) / 9).clamp(0, 10) as u32
            }
            None => 10,
        }
    }

    /// Whether an offer of `offer` buys goods worth `ask`.
    pub fn offer_ok(&self, offer: u32, ask: u32) -> bool {
        offer * 100 >= ask * (100 + self.trade_margin())
    }

    pub fn trade(&mut self, ours: &[u32], theirs: &[u32]) -> Result<String, String> {
        let Some(car) = &self.caravan else { return Err("There is no caravan.".into()) };
        if !car.arrived {
            return Err("The merchants haven't reached the depot.".into());
        }
        let offer: u32 = ours.iter().filter_map(|id| self.items.get(id)).map(|it| it.value()).sum();
        let ask: u32 = car.goods.iter().filter(|g| theirs.contains(&g.id)).map(|it| it.value()).sum();
        if !self.offer_ok(offer, ask) {
            return Err(format!(
                "\"That's not enough,\" says the merchant. (offer ☼{} vs ☼{} + {}%)",
                offer,
                ask,
                self.trade_margin()
            ));
        }
        let depot_center = self.depot().map(|b| b.center()).unwrap_or(self.embark);
        let mut taken = Vec::new();
        for id in ours {
            if let Some(mut it) = self.items.remove(id) {
                it.to_depot = false;
                it.claimed_by = None;
                taken.push(it);
            }
        }
        let car = self.caravan.as_mut().unwrap();
        let mut bought = Vec::new();
        car.goods.retain(|g| {
            if theirs.contains(&g.id) {
                bought.push(g.clone());
                false
            } else {
                true
            }
        });
        car.goods.extend(taken);
        let n = bought.len();
        for mut it in bought {
            it.pos = depot_center;
            self.items.insert(it.id, it);
        }
        Ok(if n == 0 {
            format!("The merchants gratefully accept your gift (☼{}).", offer)
        } else {
            format!("Trade complete: ☼{} of goods for ☼{} of wares.", offer, ask)
        })
    }
}

fn adjacent_or_on(a: Pos, b: Pos) -> bool {
    a.z == b.z && (a.x - b.x).abs() <= 1 && (a.y - b.y).abs() <= 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overworld::Biome;

    fn run(g: &mut Game, ticks: u64) {
        for _ in 0..ticks {
            g.step();
        }
    }

    #[test]
    fn a_broker_haggles_the_merchants_down() {
        let mut g = Game::new(81);
        assert_eq!(g.trade_margin(), 10);
        assert!(!g.offer_ok(105, 100));
        let id = g.dwarves[0].id;
        g.dwarves[0].personality.sociability = 50;
        g.dwarves[0].personality.temper = 0;
        g.appoint(crate::game::Position::Broker, Some(id));
        assert_eq!(g.trade_margin(), 0, "a sociable broker gets full value");
        assert!(g.offer_ok(100, 100));
        g.dwarves[0].personality.sociability = 0;
        assert_eq!(g.trade_margin(), 6, "a middling broker gets some off");
        g.dwarves[0].personality.temper = 50;
        assert!(g.trade_margin() > 6, "a hot temper costs them");
        g.dwarves[0].personality.sociability = -50;
        assert_eq!(g.trade_margin(), 10, "a gruff, hot-tempered broker gets nowhere");
    }

    #[test]
    fn caravan_comes_to_the_depot_and_trades() {
        let mut g = Game::new(55);
        g.creatures.clear();
        g.neighbors.retain(|n| n.race != Race::Goblin);
        let e = g.embark;
        for _ in 0..6 {
            g.spawn_item(ItemKind::Log, Mat::Wood, e);
        }
        for _ in 0..8 {
            let id = g.spawn_item(ItemKind::Craft, Mat::Granite, e);
            g.items.get_mut(&id).unwrap().quality = 3;
        }
        // Place a depot near the wagon.
        let mut placed = None;
        'p: for r in 3..25 {
            for dy in -r..=r {
                for dx in -r..=r {
                    for dz in -1..=1 {
                        let p = e.offset(dx, dy, dz);
                        if g.place_building(BuildingKind::Depot, p) {
                            placed = Some(g.world.t(p).building);
                            break 'p;
                        }
                    }
                }
            }
        }
        let depot = placed.expect("no room for a depot");
        run(&mut g, DAY);
        assert!(g.buildings[&depot].built, "depot not built");
        g.caravan_season(2);
        assert!(g.caravan.is_some(), "no caravan sent");
        let marked = g.mark_for_trade(true);
        assert!(marked >= 8);
        run(&mut g, 4 * DAY);
        let car = g.caravan.as_ref().expect("caravan vanished");
        assert!(car.arrived, "merchants never reached the depot");
        let ours: Vec<u32> = g.depot_goods().iter().map(|it| it.id).collect();
        assert!(ours.len() >= 8, "goods not hauled to depot ({})", ours.len());
        // Ask for something cheap with everything we have: accepted.
        let cheap = car.goods.iter().filter(|g| g.value() > 0).min_by_key(|g| g.value()).unwrap().id;
        let before = g.items.len();
        let res = g.trade(&ours[..1], &[cheap]);
        let res = match res {
            Ok(m) => m,
            Err(_) => g.trade(&ours, &[cheap]).expect("generous offer refused"),
        };
        eprintln!("{}", res);
        assert!(g.items.contains_key(&cheap), "bought item didn't appear");
        assert!(g.items.len() <= before);
        // A lowball offer for something precious is refused.
        let dear = g.caravan.as_ref().unwrap().goods.iter().max_by_key(|g| g.value()).unwrap().id;
        assert!(g.trade(&[], &[dear]).is_err());
        // They eventually leave.
        run(&mut g, 9 * DAY);
        assert!(g.caravan.is_none(), "caravan never left");
        assert!(g.creatures.values().all(|c| c.species != Species::Merchant));
    }

    #[test]
    fn biomes_shape_the_map() {
        let ow = crate::overworld::Overworld::generate(8);
        let find = |b: Biome| {
            // A site whose whole map lies in that biome (it runs two cells east and one south).
            (0..ow.h - 1).flat_map(|y| (0..ow.w - 2).map(move |x| (x, y))).find(|&(x, y)| ow.embarkable(x, y) && (0..3).all(|dx| (0..2).all(|dy| ow.cell(x + dx, y + dy).biome == b && !ow.cell(x + dx, y + dy).evil)))
        };
        let trees = |g: &Game| g.world.tiles.iter().filter(|t| t.kind == TileKind::Tree && t.mat == Mat::Wood).count();
        if let (Some(f), Some(gr)) = (find(Biome::Forest), find(Biome::Grassland)) {
            let forest = Game::embark(8, ow.clone(), f.0, f.1);
            let plain = Game::embark(8, ow.clone(), gr.0, gr.1);
            eprintln!("forest trees {} grassland trees {}", trees(&forest), trees(&plain));
            assert!(trees(&forest) > trees(&plain));
            assert!(forest.overworld.events.last().unwrap().text.contains("founded the outpost"));
        }
    }
}
