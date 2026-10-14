//! Fortress-wide work orders: "keep N in stock", queued at whichever
//! workshop can do the job.

use super::*;
use serde::{Deserialize, Serialize};

/// Keep at least `keep` of what `recipe` makes in stock.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkOrder {
    pub recipe: Recipe,
    pub keep: u32,
}

/// Jobs queued at most at once for one work order.
const MAX_QUEUED: usize = 3;

/// Every recipe any workshop knows, in build-menu order.
pub fn all_recipes() -> Vec<Recipe> {
    let mut v: Vec<Recipe> = Vec::new();
    for b in BuildingKind::ALL {
        for &r in b.recipes() {
            if !v.contains(&r) {
                v.push(r);
            }
        }
    }
    v
}

impl Recipe {
    /// The main thing it makes, and how many of it one job gives.
    pub fn product(self) -> (ItemKind, usize) {
        let out = self.produce(&[]);
        let kind = out.first().map_or(ItemKind::Boulder, |o| o.0);
        (kind, out.iter().filter(|o| o.0 == kind).count())
    }
}

impl Game {
    /// Items of a kind the fortress holds (not worn or wielded by anyone).
    pub fn stock_of(&self, kind: ItemKind) -> usize {
        let in_use: HashSet<u32> = self
            .dwarves
            .iter()
            .filter(|d| d.alive)
            .flat_map(|d| d.held.iter().chain(d.armor.iter()).chain(d.clothes.iter()).copied())
            .collect();
        self.items.values().filter(|it| it.kind == kind && !in_use.contains(&it.id)).count()
    }

    /// Jobs for `recipe` waiting in workshop queues.
    pub fn queued(&self, recipe: Recipe) -> usize {
        self.buildings.values().flat_map(|b| b.orders.iter()).filter(|o| o.recipe == recipe).count()
    }

    pub fn add_work_order(&mut self, recipe: Recipe, keep: u32) {
        match self.work_orders.iter_mut().find(|o| o.recipe == recipe) {
            Some(o) => o.keep = keep,
            None => self.work_orders.push(WorkOrder { recipe, keep }),
        }
    }

    /// A few times a day: top up stocks that have fallen below their mark by
    /// queuing jobs at the least busy workshop that can do them.
    pub(super) fn manage_orders(&mut self) {
        for i in 0..self.work_orders.len() {
            let WorkOrder { recipe, keep } = self.work_orders[i];
            let (kind, per_job) = recipe.product();
            let have = self.stock_of(kind);
            let mut queued = self.queued(recipe);
            while queued < MAX_QUEUED && have + queued * per_job.max(1) < keep as usize {
                let shop = self
                    .buildings
                    .values()
                    .filter(|b| b.built && b.kind.recipes().contains(&recipe))
                    .min_by_key(|b| (b.orders.len(), b.id))
                    .map(|b| b.id);
                let Some(shop) = shop else {
                    self.warn(
                        &format!("order{:?}", recipe),
                        &format!("Work order \"{}\": no workshop can do it.", recipe.name()),
                        (230, 200, 90),
                    );
                    break;
                };
                self.add_order(shop, recipe);
                queued += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_recipe_is_listed_once_and_knows_its_product() {
        let all = all_recipes();
        assert!(all.contains(&Recipe::WoodBed) && all.contains(&Recipe::Mill));
        assert_eq!(Recipe::Blocks.product(), (ItemKind::Block, 4));
        assert_eq!(Recipe::WoodBed.product(), (ItemKind::Bed, 1));
    }

    #[test]
    fn a_keep_in_stock_order_queues_jobs_until_the_stock_is_met() {
        let mut g = Game::new(77);
        g.creatures.retain(|_, c| c.tame);
        let e = g.embark;
        for _ in 0..4 {
            g.spawn_item(ItemKind::Log, Mat::Wood, e);
        }
        g.spawn_item(ItemKind::Log, Mat::Wood, e);
        let site = (2..20)
            .flat_map(|r| [e.offset(r, 0, 0), e.offset(-r - 2, 0, 0), e.offset(0, r, 0)])
            .find(|&p| g.can_place(BuildingKind::Carpenter, p))
            .unwrap();
        assert!(g.place_building(BuildingKind::Carpenter, site));
        let shop = g.world.t(site).building;
        for _ in 0..2 * DAY {
            g.step();
        }
        assert!(g.buildings[&shop].built);
        let have = g.stock_of(ItemKind::Barrel);
        g.add_work_order(Recipe::Barrel, have as u32 + 2);
        g.manage_orders();
        assert_eq!(g.queued(Recipe::Barrel), 2, "should queue just enough barrels");
        g.manage_orders();
        assert_eq!(g.queued(Recipe::Barrel), 2, "shouldn't queue twice");
        for _ in 0..2 * DAY {
            g.step();
        }
        assert!(g.stock_of(ItemKind::Barrel) >= have + 2, "barrels weren't made");
        g.manage_orders();
        assert_eq!(g.queued(Recipe::Barrel), 0, "stock is met; nothing more to queue");
    }

    #[test]
    fn a_workshop_profile_keeps_everyone_else_out() {
        let mut g = Game::new(77);
        g.creatures.retain(|_, c| c.tame);
        let e = g.embark;
        for _ in 0..6 {
            g.spawn_item(ItemKind::Log, Mat::Wood, e);
        }
        let site = (2..20)
            .flat_map(|r| [e.offset(r, 0, 0), e.offset(-r - 2, 0, 0), e.offset(0, r, 0)])
            .find(|&p| g.can_place(BuildingKind::Carpenter, p))
            .unwrap();
        assert!(g.place_building(BuildingKind::Carpenter, site));
        let shop = g.world.t(site).building;
        for _ in 0..2 * DAY {
            g.step();
        }
        assert!(g.buildings[&shop].built);
        for d in g.dwarves.iter_mut() {
            d.labors[Labor::Carpentry.idx()] = true;
        }
        let chosen = g.dwarves[3].id;
        g.toggle_worker(shop, chosen);
        let before = g.dwarves.iter().find(|d| d.id == chosen).unwrap().skills[Labor::Carpentry.idx()];
        let others: Vec<(u32, u32)> = g.dwarves.iter().filter(|d| d.id != chosen).map(|d| (d.id, d.skills[Labor::Carpentry.idx()])).collect();
        for _ in 0..3 {
            g.add_order(shop, Recipe::Barrel);
        }
        let mut seen = HashSet::new();
        for _ in 0..2 * DAY {
            g.step();
            if let Some(w) = g.buildings[&shop].worker {
                seen.insert(w);
            }
        }
        assert!(seen.iter().all(|&w| w == chosen), "someone else worked the shop: {:?}", seen);
        assert!(g.dwarves.iter().find(|d| d.id == chosen).unwrap().skills[Labor::Carpentry.idx()] > before, "the chosen carpenter never worked");
        for (id, xp) in others {
            assert_eq!(g.dwarves.iter().find(|d| d.id == id).unwrap().skills[Labor::Carpentry.idx()], xp);
        }
    }
}
