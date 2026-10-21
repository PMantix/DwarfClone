//! Fortress-wide work orders: "keep N in stock", "when drink runs below
//! 40", "after the thread is spun", "every season", queued at whichever
//! workshop can do the job. An order whose inputs run short queues the
//! orders that make them (`supply.rs`).

use super::supply::Want;
use crate::buildings::Input;
use super::*;
use serde::{Deserialize, Serialize};

/// When a work order queues its jobs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cond {
    /// Keep `keep` of the product in stock.
    #[default]
    Keep,
    /// Whenever the fort holds fewer than `n` of `kind`.
    Below { kind: ItemKind, n: u32 },
    /// Like `Keep`, but only once the order for that recipe is met.
    After(Recipe),
    /// `keep` jobs every `ticks`, `times` times (0: for ever); `done` so far,
    /// the last at `last`.
    Every { ticks: u64, times: u32, done: u32, last: u64 },
}

impl Cond {
    /// "keep 6", "drink < 40", "after Spin thread", "every season ×2".
    pub fn describe(&self, keep: u32) -> String {
        match *self {
            Cond::Keep => format!("keep {}", keep),
            Cond::Below { kind, n } => format!("{} < {}", kind.plural(), n),
            Cond::After(r) => format!("after {}", r.name().to_lowercase()),
            Cond::Every { ticks, times, done, .. } => {
                let when = if ticks >= YEAR { "year" } else if ticks >= SEASON { "season" } else if ticks >= MONTH { "month" } else { "day" };
                let left = if times > 0 { format!(" ({}/{})", done, times) } else { String::new() };
                format!("{} every {}{}", keep, when, left)
            }
        }
    }
}

/// Keep at least `keep` of what `recipe` makes in stock (or as its `cond` says).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkOrder {
    pub recipe: Recipe,
    pub keep: u32,
    /// The first input must be of this (a granite table; iron from hematite).
    #[serde(default)]
    pub mat: Option<Mat>,
    #[serde(default)]
    pub cond: Cond,
    /// Queued to feed an order for this recipe; dropped when that one is met.
    #[serde(default)]
    pub parent: Option<Recipe>,
    /// Dropped at this tick (0: never).
    #[serde(default)]
    pub until: u64,
    /// What its jobs wait on, for the screen.
    #[serde(skip)]
    pub waiting: Vec<Want>,
    /// 1 (urgent) .. 7 (when there's nothing else); 4 is normal (priority.rs).
    #[serde(default = "normal_prio", skip_serializing_if = "is_normal_prio")]
    pub prio: u8,
}

fn normal_prio() -> u8 {
    super::priority::NORMAL
}

fn is_normal_prio(p: &u8) -> bool {
    *p == super::priority::NORMAL
}

impl WorkOrder {
    pub fn new(recipe: Recipe, keep: u32) -> WorkOrder {
        WorkOrder { recipe, keep, mat: None, cond: Cond::Keep, parent: None, until: 0, waiting: Vec::new(), prio: normal_prio() }
    }

    /// "Make steel", "Carve stone table (granite)".
    pub fn name(&self) -> String {
        match self.mat {
            Some(m) => format!("{} ({})", self.recipe.name(), m.name()),
            None => self.recipe.name().to_string(),
        }
    }
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

/// The recipe that makes `kind`, of `mat` if given (stone, wood, bone…).
pub fn recipe_for(kind: ItemKind, mat: Option<Mat>) -> Option<Recipe> {
    let fits = |r: &Recipe| {
        let first = r.inputs().first().copied();
        match mat {
            Some(m) if m.is_building_stone() => first == Some(Input::Stone),
            Some(Mat::Wood) => first == Some(Input::Log),
            Some(Mat::Bone) => first == Some(Input::Bone),
            Some(Mat::Leather) => first == Some(Input::Leather),
            Some(m) if m.smelts_to().is_none() && matches!(m, Mat::Iron | Mat::Steel | Mat::Copper | Mat::Bronze | Mat::Gold) => {
                matches!(first, Some(Input::Bar))
            }
            Some(_) => true,
            // Wood or stone, before the rarer stuff.
            None => matches!(first, Some(Input::Log | Input::Stone)),
        }
    };
    let all: Vec<Recipe> = all_recipes().into_iter().filter(|r| r.product().0 == kind).collect();
    all.iter().copied().find(|r| fits(r)).or(if mat.is_none() { all.first().copied() } else { None })
}

impl Recipe {
    /// The main thing it makes, and how many of it one job gives.
    pub fn product(self) -> (ItemKind, usize) {
        let out = self.produce(&[]);
        let kind = out.first().map_or(ItemKind::Boulder, |o| o.0);
        (kind, out.iter().filter(|o| o.0 == kind).count())
    }

    /// What its product is made of, when its first input is `mat`.
    pub fn product_mat(self, mat: Option<Mat>) -> Option<Mat> {
        match self {
            Recipe::SmeltOre => mat.and_then(|m| m.smelts_to()),
            Recipe::MakeSteel => Some(Mat::Steel),
            _ => mat,
        }
    }
}

impl Game {
    /// Items of a kind the fortress holds (not worn or wielded by anyone).
    pub fn stock_of(&self, kind: ItemKind) -> usize {
        self.stock_of_mat(kind, None)
    }

    /// Items of a kind (and material, if given) the fortress holds.
    pub fn stock_of_mat(&self, kind: ItemKind, mat: Option<Mat>) -> usize {
        let in_use: HashSet<u32> = self
            .dwarves
            .iter()
            .filter(|d| d.alive)
            .flat_map(|d| d.held.iter().chain(d.armor.iter()).chain(d.clothes.iter()).copied())
            .collect();
        self.items.values().filter(|it| it.kind == kind && mat.map_or(true, |m| it.mat == m) && !in_use.contains(&it.id)).count()
    }

    /// What an order for `recipe` with first input `mat` counts as stock.
    pub(super) fn order_have_of(&self, recipe: Recipe, mat: Option<Mat>) -> usize {
        self.stock_of_mat(recipe.product().0, recipe.product_mat(mat))
    }

    /// Jobs for `recipe` waiting in workshop queues.
    pub fn queued(&self, recipe: Recipe) -> usize {
        self.buildings.values().flat_map(|b| b.orders.iter()).filter(|o| o.recipe == recipe).count()
    }

    /// Jobs waiting for this order: of its material, if it names one.
    pub fn queued_for(&self, o: &WorkOrder) -> usize {
        match o.mat {
            None => self.queued(o.recipe),
            Some(m) => self.buildings.values().flat_map(|b| b.orders.iter()).filter(|x| x.recipe == o.recipe && x.mat == Some(m)).count(),
        }
    }

    /// Set (or make) the plain order for `recipe`.
    pub fn add_work_order(&mut self, recipe: Recipe, keep: u32) {
        match self.work_orders.iter_mut().find(|o| o.recipe == recipe && o.mat.is_none() && o.parent.is_none()) {
            Some(o) => o.keep = keep,
            None => self.work_orders.push(WorkOrder::new(recipe, keep)),
        }
    }

    /// An order for `kind` of `mat` (a mandate's, say): `n` more than the
    /// fort holds now, until `until`. False when no recipe makes it.
    pub fn add_order_for(&mut self, kind: ItemKind, mat: Option<Mat>, n: u32, until: u64) -> bool {
        let Some(r) = recipe_for(kind, mat) else { return false };
        let keep = self.order_have_of(r, mat) as u32 + n;
        match self.work_orders.iter_mut().find(|o| o.recipe == r && o.mat == mat && o.parent.is_none()) {
            Some(o) => {
                o.keep = o.keep.max(keep);
                o.until = if o.until == 0 { 0 } else { o.until.max(until) };
            }
            None => {
                let mut o = WorkOrder::new(r, keep);
                o.mat = mat;
                o.until = until;
                self.work_orders.push(o);
            }
        }
        true
    }

    /// A mandate becomes an order when the overseer runs the fort.
    pub(super) fn mandate_to_order(&mut self) {
        if self.overseer.is_none() {
            return;
        }
        let Some(m) = self.mandate.clone() else { return };
        self.add_order_for(m.kind, m.mat, m.count, m.deadline);
    }

    /// Jobs `o` wants queued now, on top of those queued already.
    pub fn due(&self, o: &WorkOrder) -> u32 {
        let (kind, per) = o.recipe.product();
        let per = per.max(1);
        let queued = self.queued_for(o);
        let keep_up = |target: u32, have: usize, made: usize| -> u32 {
            let short = (target as usize).saturating_sub(have + made);
            ((short + per - 1) / per) as u32
        };
        match o.cond {
            Cond::Keep => keep_up(o.keep, self.order_have_of(o.recipe, o.mat), queued * per),
            Cond::Below { kind: k, n } => {
                let have = self.stock_of(k);
                if have >= n as usize {
                    0
                } else if k == kind {
                    keep_up(n, have, queued * per)
                } else {
                    (n as usize - have).saturating_sub(queued) as u32
                }
            }
            Cond::After(r) => {
                let open = self.queued(r) > 0
                    || self.work_orders.iter().any(|w| w.recipe == r && w.cond == Cond::Keep && self.order_have_of(w.recipe, w.mat) < w.keep as usize);
                if open {
                    0
                } else {
                    keep_up(o.keep, self.order_have_of(o.recipe, o.mat), queued * per)
                }
            }
            Cond::Every { ticks, times, done, last } => {
                let fresh = done == 0 && last == 0;
                if (times > 0 && done >= times) || !(fresh || self.tick >= last + ticks.max(1)) {
                    0
                } else {
                    o.keep
                }
            }
        }
    }

    /// Queue a job of `recipe` (its first input of `mat`) at a workshop.
    fn queue_job(&mut self, bld: u32, recipe: Recipe, mat: Option<Mat>) {
        if let Some(b) = self.buildings.get_mut(&bld) {
            b.orders.push(Order { recipe, repeat: false, mat });
        }
    }

    /// Drop orders past their time, finished `Every`s, and children whose
    /// parent is gone or met.
    fn prune_orders(&mut self) {
        let tick = self.tick;
        self.work_orders.retain(|o| {
            let over = matches!(o.cond, Cond::Every { times, done, .. } if times > 0 && done >= times);
            !(o.until > 0 && tick >= o.until) && !over
        });
        // (Children of children go with them.)
        loop {
            let live: Vec<Recipe> = self
                .work_orders
                .iter()
                .filter(|w| self.queued(w.recipe) > 0 || self.due(w) > 0)
                .map(|w| w.recipe)
                .collect();
            let n = self.work_orders.len();
            self.work_orders.retain(|o| o.parent.map_or(true, |p| live.contains(&p)));
            if self.work_orders.len() == n {
                break;
            }
        }
    }

    /// A few times a day: top up stocks that have fallen below their mark by
    /// queuing jobs at the least busy workshop that can do them, and see to
    /// the inputs those jobs wait on.
    pub(super) fn manage_orders(&mut self) {
        self.prune_orders();
        if self.work_orders.is_empty() {
            return;
        }
        if !self.has_post(super::Position::Manager) {
            self.trouble(super::troubles::Lack::Manager);
            self.warn("nomanager", "Work orders wait for a manager (n to appoint one).", (230, 200, 90));
            return;
        }
        // A poor manager only queues one job per order at a time.
        let careless = self.post_fit(super::Position::Manager) == Some(super::Fit::Poor);
        let stock = self.stock_by_input();
        let mut i = 0;
        while i < self.work_orders.len() {
            let o = self.work_orders[i].clone();
            let recipe = o.recipe;
            let mut due = self.due(&o) as usize;
            let mut queued = self.queued_for(&o);
            if let Cond::Every { ticks, times, done, .. } = o.cond {
                if due > 0 {
                    self.work_orders[i].cond = Cond::Every { ticks, times, done: done + 1, last: self.tick };
                    queued = 0;
                }
            }
            let most = if matches!(o.cond, Cond::Every { .. }) { MAX_QUEUED.max(due) } else { MAX_QUEUED };
            let cap = if careless { (queued + 1).min(most) } else { most };
            while queued < cap && due > 0 {
                let shop = self
                    .buildings
                    .values()
                    .filter(|b| b.built && b.kind.recipes().contains(&recipe))
                    .min_by_key(|b| (b.orders.len(), b.id))
                    .map(|b| b.id);
                let Some(shop) = shop else {
                    if let Some(k) = BuildingKind::ALL.iter().find(|k| k.recipes().contains(&recipe)) {
                        self.trouble(super::troubles::Lack::Workshop(*k));
                    }
                    self.warn(
                        &format!("order{:?}", recipe),
                        &format!("Work order \"{}\": no workshop can do it.", recipe.name()),
                        (230, 200, 90),
                    );
                    break;
                };
                self.queue_job(shop, recipe, o.mat);
                queued += 1;
                due -= 1;
            }
            // Its jobs' inputs: what's short is ordered or sent for.
            let jobs = self.queued_for(&o) as u32;
            let before = self.work_orders.len();
            let waiting = if jobs > 0 { self.feed_mat(recipe, o.mat, jobs, 0, &stock) } else { Vec::new() };
            if o.parent.is_none() && self.work_orders.len() > before {
                self.note_chain(&o, &waiting);
            }
            if let Some(w) = self.work_orders.get_mut(i) {
                w.waiting = waiting;
            }
            i += 1;
        }
    }

    /// The overseer's terms for an order it set: food and drink by what the
    /// stores hold, crafts for the caravans each season once there is a
    /// depot, and what a shortage raised only until `until` (0: standing).
    pub(super) fn ov_order_terms(&mut self, r: Recipe, keep: u32, until: u64) {
        let depot = self.buildings.values().any(|b| b.built && b.kind == BuildingKind::Depot);
        let Some(o) = self.work_orders.iter_mut().find(|o| o.recipe == r && o.mat.is_none() && o.parent.is_none()) else { return };
        o.until = until;
        match r {
            Recipe::Brew => o.cond = Cond::Below { kind: ItemKind::Drink, n: keep },
            Recipe::Cook => o.cond = Cond::Below { kind: ItemKind::Meal, n: keep },
            Recipe::StoneCraft if depot => {
                if !matches!(o.cond, Cond::Every { .. }) {
                    o.cond = Cond::Every { ticks: SEASON, times: 0, done: 0, last: 0 };
                }
            }
            _ => {}
        }
    }

    /// Iron or the ore for it is in the stores.
    pub(super) fn iron_known(&self) -> bool {
        self.items.values().any(|it| (it.kind == ItemKind::Bar && it.mat == Mat::Iron) || (it.kind == ItemKind::Boulder && it.mat == Mat::Hematite))
    }

    /// Sensible conditions for every order (the orders screen's `a`): food
    /// and drink below twice the population, furniture below half of it,
    /// weapons and armour kept at the squads' size.
    pub fn auto_conditions(&mut self) {
        let pop = self.alive_count() as u32;
        let soldiers = self.squads.iter().map(|s| s.members.len() as u32).sum::<u32>().max(2);
        for o in self.work_orders.iter_mut().filter(|o| o.parent.is_none()) {
            let (kind, _) = o.recipe.product();
            match kind {
                ItemKind::Drink | ItemKind::Meal => o.cond = Cond::Below { kind, n: (2 * pop).max(10) },
                ItemKind::Bed | ItemKind::Table | ItemKind::Chair | ItemKind::Door | ItemKind::Cabinet | ItemKind::Chest => {
                    o.cond = Cond::Below { kind, n: (pop / 2).max(2) }
                }
                ItemKind::Axe | ItemKind::Sword | ItemKind::Spear | ItemKind::Hammer | ItemKind::Crossbow | ItemKind::Helm | ItemKind::Breastplate | ItemKind::Shield => {
                    o.cond = Cond::Keep;
                    o.keep = soldiers;
                }
                _ => {}
            }
        }
    }

    /// The orders screen's `c`: Keep → Below → After (the order above) →
    /// Every season → Keep.
    pub fn cycle_condition(&mut self, i: usize) {
        let before = if i > 0 { self.work_orders.get(i - 1).map(|o| o.recipe) } else { None };
        let Some(o) = self.work_orders.get_mut(i) else { return };
        let kind = o.recipe.product().0;
        o.cond = match o.cond {
            Cond::Keep => Cond::Below { kind, n: o.keep },
            Cond::Below { .. } => match before {
                Some(r) if r != o.recipe => Cond::After(r),
                _ => Cond::Every { ticks: SEASON, times: 0, done: 0, last: 0 },
            },
            Cond::After(_) => Cond::Every { ticks: SEASON, times: 0, done: 0, last: 0 },
            Cond::Every { .. } => Cond::Keep,
        };
    }

    /// The orders screen's `m`: the next material in stock for the order's
    /// first input, then any again.
    pub fn cycle_order_mat(&mut self, i: usize) {
        let Some(o) = self.work_orders.get(i) else { return };
        let Some(&first) = o.recipe.inputs().first() else { return };
        let mut mats: Vec<Mat> = self.items.values().filter(|it| it.free() && first.accepts(it)).map(|it| it.mat).collect();
        mats.sort();
        mats.dedup();
        let next = match o.mat {
            None => mats.first().copied(),
            Some(m) => mats.iter().copied().find(|&x| x > m),
        };
        self.work_orders[i].mat = next;
    }

    /// The overseer says what a new chain of orders is for, once a season.
    fn note_chain(&mut self, o: &WorkOrder, waiting: &[Want]) {
        let tick = self.tick;
        if waiting.is_empty() || !self.due_once(&format!("chain{:?}", o.recipe), SEASON) {
            return;
        }
        let parts: Vec<String> = waiting.iter().map(|w| w.describe()).collect();
        let list = match parts.len() {
            1 => parts[0].clone(),
            n => format!("{} and {}", parts[..n - 1].join(", "), parts[n - 1]),
        };
        let text = format!("To {}: first {}.", o.name().to_lowercase(), list);
        self.count("m2.chained_orders", 1);
        if let Some(ov) = self.overseer.as_mut() {
            Self::note_ov(ov, tick, text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A built workshop of `kind` by the embark, and a manager.
    fn shop(seed: u64, kind: BuildingKind) -> (Game, u32) {
        let mut g = Game::new(seed);
        g.creatures.retain(|_, c| c.tame);
        let e = g.embark;
        let site = g.ov_spot(kind, e.offset(3, 0, 0), 20).expect("room for a workshop");
        assert!(g.place_building(kind, site));
        let id = g.world.t(site).building;
        g.buildings.get_mut(&id).unwrap().built = true;
        let m = g.dwarves[0].id;
        g.dwarves[0].personality.diligence = 40;
        g.appoint(super::super::Position::Manager, Some(m));
        (g, id)
    }

    #[test]
    fn below_after_and_every_conditions_queue_when_due() {
        let (mut g, _) = shop(77, BuildingKind::Carpenter);
        let barrels = g.stock_of(ItemKind::Barrel) as u32;
        // Below: only while the stock is under the mark.
        let mut o = WorkOrder::new(Recipe::Barrel, 0);
        o.cond = Cond::Below { kind: ItemKind::Barrel, n: barrels + 2 };
        assert_eq!(g.due(&o), 2);
        o.cond = Cond::Below { kind: ItemKind::Barrel, n: barrels };
        assert_eq!(g.due(&o), 0, "stock is at the mark");
        // After: waits while the barrels are still wanted.
        g.add_work_order(Recipe::Barrel, barrels + 2);
        let mut bins = WorkOrder::new(Recipe::Bin, g.stock_of(ItemKind::Bin) as u32 + 1);
        bins.cond = Cond::After(Recipe::Barrel);
        assert_eq!(g.due(&bins), 0, "the barrels come first");
        g.work_orders[0].keep = 0;
        assert_eq!(g.due(&bins), 1, "barrels done: now the bin");
        // Every: two jobs a day, twice, then the order is done.
        g.work_orders.clear();
        let mut b = WorkOrder::new(Recipe::Bucket, 2);
        b.cond = Cond::Every { ticks: DAY, times: 2, done: 0, last: 0 };
        g.work_orders.push(b);
        g.manage_orders();
        assert_eq!(g.queued(Recipe::Bucket), 2);
        g.manage_orders();
        assert_eq!(g.queued(Recipe::Bucket), 2, "not again the same day");
        g.tick += DAY;
        g.manage_orders();
        assert_eq!(g.queued(Recipe::Bucket), 4, "the next day's two");
        g.tick += DAY;
        g.manage_orders();
        assert_eq!(g.queued(Recipe::Bucket), 4);
        assert!(g.work_orders.is_empty(), "done twice, the order goes");
    }

    #[test]
    fn a_granite_table_order_uses_granite() {
        let (mut g, id) = shop(77, BuildingKind::Mason);
        let at = g.buildings[&id].center();
        // Limestone at the door, granite a walk away.
        g.items.retain(|_, it| it.kind != ItemKind::Boulder);
        g.spawn_item(ItemKind::Boulder, Mat::Limestone, at);
        g.spawn_item(ItemKind::Boulder, Mat::Granite, g.embark.offset(-4, 3, 0));
        for d in g.dwarves.iter_mut() {
            d.labors[Labor::Masonry.idx()] = true;
        }
        assert_eq!(recipe_for(ItemKind::Table, Some(Mat::Granite)), Some(Recipe::StoneTable));
        assert!(g.add_order_for(ItemKind::Table, Some(Mat::Granite), 1, 0));
        g.manage_orders();
        assert_eq!(g.buildings[&id].orders.first().map(|o| o.mat), Some(Some(Mat::Granite)));
        for _ in 0..2 * DAY {
            g.step();
        }
        let tables: Vec<Mat> = g.items.values().filter(|it| it.kind == ItemKind::Table).map(|it| it.mat).collect();
        assert!(tables.contains(&Mat::Granite), "no granite table: {:?}", tables);
        assert!(!tables.contains(&Mat::Limestone), "made of the nearer limestone");
    }

    #[test]
    fn a_mandate_becomes_an_order() {
        let mut g = Game::new(77);
        let noble = g.dwarves[0].id;
        let deadline = g.tick + SEASON;
        g.mandate = Some(Mandate { noble, kind: ItemKind::Chair, mat: Some(Mat::Marble), count: 3, baseline: 0, deadline });
        g.mandate_to_order();
        assert!(g.work_orders.is_empty(), "a player's mandates are the player's business");
        g.set_overseer(Some(super::super::overseer::Style::Steady));
        g.mandate_to_order();
        let o = g.work_orders.iter().find(|o| o.recipe == Recipe::StoneChair).expect("no order for the chairs");
        assert_eq!((o.mat, o.until), (Some(Mat::Marble), deadline));
        assert!(o.keep >= 3);
    }

    #[test]
    fn the_overseers_orders_carry_conditions() {
        let mut g = Game::new(5);
        g.set_overseer(Some(super::super::overseer::Style::Steady));
        for _ in 0..DAY * 3 {
            g.step();
        }
        while g.tick % 600 != 0 {
            g.step();
        }
        // A shortage it raised an order for, at a workshop it has.
        let have: Vec<Recipe> = g.work_orders.iter().map(|o| o.recipe).collect();
        let r = g
            .buildings
            .values()
            .filter(|b| b.built)
            .flat_map(|b| b.kind.recipes().iter().copied())
            .find(|r| !have.contains(r))
            .expect("a recipe it hasn't ordered");
        let end = g.tick + DAY * 20;
        g.overseer.as_mut().unwrap().boosts.push((r, 3, end));
        g.oversee();
        let o = g.work_orders.iter().find(|o| o.recipe == r).expect("the boost became an order");
        assert_eq!(o.until, end, "it lasts as long as the boost");
        if let Some(b) = g.work_orders.iter().find(|o| o.recipe == Recipe::Brew) {
            assert!(matches!(b.cond, Cond::Below { kind: ItemKind::Drink, .. }), "{:?}", b.cond);
        }
        assert!(g.work_orders.iter().any(|o| o.recipe == Recipe::Brew || o.recipe == Recipe::Cook), "no food or drink ordered");
        // Past its time it is gone.
        g.tick = end;
        g.manage_orders();
        assert!(!g.work_orders.iter().any(|o| o.recipe == r && o.until > 0));
    }

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
        let site = g.ov_spot(BuildingKind::Carpenter, e.offset(3, 0, 0), 20).expect("room for a workshop");
        assert!(g.place_building(BuildingKind::Carpenter, site));
        let shop = g.world.t(site).building;
        for _ in 0..2 * DAY {
            g.step();
        }
        assert!(g.buildings[&shop].built);
        let have = g.stock_of(ItemKind::Barrel);
        g.add_work_order(Recipe::Barrel, have as u32 + 2);
        g.manage_orders();
        assert_eq!(g.queued(Recipe::Barrel), 0, "no manager, no work orders");
        let m = g.dwarves[0].id;
        g.dwarves[0].personality.diligence = 40;
        g.appoint(super::super::Position::Manager, Some(m));
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
        let site = g.ov_spot(BuildingKind::Carpenter, e.offset(3, 0, 0), 20).expect("room for a workshop");
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
