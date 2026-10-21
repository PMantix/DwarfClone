//! Orders that think (O08; packages P9 and P18 fill this in): what feeds a
//! recipe, where each input comes from, and the ledger of what the fort
//! made and used. An order whose workshop waits on an input queues the
//! orders that make it (steel → iron bars → hematite and charcoal → logs)
//! and sends miners or woodcutters for what has to be gathered.

use super::*;
use crate::buildings::{Input, Recipe};

/// Records of production (`Game.ledger`, ECO-15) live in ledger.rs.
pub use super::ledger::Ledger;

/// Where an input comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// A workshop job makes it.
    Craft(Recipe),
    /// Miners dig it out.
    Mine(MatFilter),
    Chop,
    Farm,
    Hunt,
    Fish,
    /// The butcher's shop, from a carcass.
    Butcher,
    /// Only a caravan brings it.
    Buy,
}

/// What a mining request looks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatFilter {
    /// One ore (`Some`), or any that smelts.
    Ore(Option<Mat>),
    Flux,
    Gem,
    BuildingStone,
}

impl MatFilter {
    pub fn matches(self, m: Mat) -> bool {
        match self {
            MatFilter::Ore(Some(o)) => m == o,
            MatFilter::Ore(None) => m.smelts_to().is_some(),
            MatFilter::Flux => m == Mat::Limestone,
            MatFilter::Gem => m.is_gem(),
            MatFilter::BuildingStone => m.is_building_stone(),
        }
    }

    pub fn name(self) -> String {
        match self {
            MatFilter::Ore(Some(m)) => m.name().to_string(),
            MatFilter::Ore(None) => "ore".into(),
            MatFilter::Flux => "flux".into(),
            MatFilter::Gem => "gems".into(),
            MatFilter::BuildingStone => "stone".into(),
        }
    }
}

/// One input an order waits on, and what was done about it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Want {
    pub input: Input,
    pub via: Source,
    /// Something is on its way: a child order, miners or woodcutters sent.
    pub queued: bool,
}

impl Want {
    /// "flux (miners sent)", "charcoal (queued)", "thread (nobody can)".
    pub fn describe(&self) -> String {
        let what = match self.input {
            Input::Fuel => "fuel",
            Input::Flux => "flux",
            Input::Ore => "ore",
            Input::IronBar => "iron bars",
            Input::Bar => "metal bars",
            Input::Stone => "stone",
            Input::Log => "logs",
            i => i.desc(),
        };
        let how = match (self.via, self.queued) {
            (Source::Craft(r), true) if r == Recipe::Charcoal => "charcoal queued",
            (Source::Craft(_), true) | (Source::Butcher, true) => "queued",
            (Source::Mine(f @ MatFilter::Ore(Some(_))), true) => return format!("{} (miners sent to the {})", what, f.name()),
            (Source::Mine(_), true) => "miners sent",
            (Source::Chop, true) => "woodcutters sent",
            (Source::Hunt, true) => "hunters out",
            (Source::Farm, _) => "the fields",
            (Source::Buy, _) => "a caravan",
            (Source::Craft(_), false) | (Source::Butcher, false) => "no workshop",
            _ => "none to be had",
        };
        format!("{} ({})", what, how)
    }
}

/// What makes something `input` accepts, best first.
pub fn sources(input: Input) -> &'static [Source] {
    use Source::*;
    match input {
        Input::Log => &[Chop, Buy],
        Input::Stone => &[Mine(MatFilter::BuildingStone)],
        Input::Brewable | Input::Fiber | Input::Grain => &[Farm],
        Input::Edible => &[Farm, Hunt, Fish],
        Input::Carcass => &[Hunt],
        Input::Ore => &[Mine(MatFilter::Ore(None))],
        Input::Fuel => &[Craft(Recipe::Charcoal), Mine(MatFilter::Ore(Some(Mat::Coal))), Buy],
        Input::Bar | Input::IronBar => &[Craft(Recipe::SmeltOre), Buy],
        Input::Flux => &[Mine(MatFilter::Flux)],
        Input::Bone | Input::Hide | Input::Tallow => &[Butcher, Hunt],
        Input::RoughGem => &[Mine(MatFilter::Gem)],
        Input::Leather => &[Craft(Recipe::Tan), Buy],
        Input::Thread => &[Craft(Recipe::SpinThread)],
        Input::Cloth => &[Craft(Recipe::Weave)],
        Input::Ash => &[Craft(Recipe::Ash)],
        Input::CutGem => &[Craft(Recipe::CutGem)],
        // (The jeweler takes what furniture there is; none is made for it.)
        Input::Furniture => &[],
        Input::Mechanism => &[Craft(Recipe::Mechanism)],
    }
}

/// The ore a metal is smelted from.
pub fn ore_for(metal: Mat) -> Option<Mat> {
    [Mat::Hematite, Mat::Malachite, Mat::NativeGold].into_iter().find(|o| o.smelts_to() == Some(metal))
}

/// How deep a chain of orders may go (steel → bars → charcoal → logs).
const MAX_DEPTH: u8 = 3;

/// Every input, for counting them all in one pass.
const INPUTS: [Input; 23] = [
    Input::Log,
    Input::Stone,
    Input::Brewable,
    Input::Fiber,
    Input::Edible,
    Input::Carcass,
    Input::Ore,
    Input::Fuel,
    Input::Bar,
    Input::IronBar,
    Input::Flux,
    Input::Bone,
    Input::RoughGem,
    Input::Hide,
    Input::Leather,
    Input::Thread,
    Input::Cloth,
    Input::Ash,
    Input::Tallow,
    Input::Grain,
    Input::CutGem,
    Input::Furniture,
    Input::Mechanism,
];

/// Free items by input, counted once for a round of orders.
pub struct Stock {
    n: [usize; INPUTS.len()],
}

impl Stock {
    pub fn get(&self, input: Input) -> usize {
        INPUTS.iter().position(|&i| i == input).map_or(0, |k| self.n[k])
    }
}

impl Game {
    /// How many of `input` the fort has free.
    pub fn have_input(&self, input: Input) -> usize {
        self.items.values().filter(|it| it.free() && input.accepts(it)).count()
    }

    /// Free items of every input, in one pass.
    pub fn stock_by_input(&self) -> Stock {
        let mut n = [0; INPUTS.len()];
        for it in self.items.values().filter(|it| it.free()) {
            for (k, i) in INPUTS.iter().enumerate() {
                if i.accepts(it) {
                    n[k] += 1;
                }
            }
        }
        Stock { n }
    }

    /// How many of `input` of one material the fort has free.
    pub fn have_of(&self, input: Input, mat: Option<Mat>) -> usize {
        match mat {
            None => self.have_input(input),
            Some(m) => self.items.values().filter(|it| it.free() && it.mat == m && input.accepts(it)).count(),
        }
    }

    /// Make sure `n` jobs of `recipe` can be fed: queue child orders for missing
    /// inputs, ask for gathering, and return what is still waited on.
    #[allow(dead_code)]
    pub(super) fn feed(&mut self, recipe: Recipe, n: u32, depth: u8) -> Vec<Want> {
        let stock = self.stock_by_input();
        self.feed_mat(recipe, None, n, depth, &stock)
    }

    /// `feed`, for an order whose first input must be of `mat`.
    pub(super) fn feed_mat(&mut self, recipe: Recipe, mat: Option<Mat>, n: u32, depth: u8, stock: &Stock) -> Vec<Want> {
        let mut wants = Vec::new();
        if n == 0 || depth > MAX_DEPTH {
            return wants;
        }
        let shops: Vec<BuildingKind> = BuildingKind::ALL.iter().copied().filter(|k| k.recipes().contains(&recipe)).collect();
        let Some(shop) = shops.iter().copied().find(|&k| self.buildings.values().any(|b| b.built && b.kind == k)).or(shops.first().copied()) else {
            return wants;
        };
        let inputs = recipe.inputs_at(shop);
        let mut seen: Vec<Input> = Vec::new();
        for (k, &input) in inputs.iter().enumerate() {
            if seen.contains(&input) {
                continue;
            }
            seen.push(input);
            let per = inputs.iter().filter(|&&i| i == input).count();
            let first_mat = if k == 0 { mat } else { None };
            let need = per * n as usize;
            let have = if first_mat.is_some() { self.have_of(input, first_mat) } else { stock.get(input) };
            if have >= need {
                continue;
            }
            let short = need - have;
            // The metal a bar must be, and so the ore it comes from.
            let metal = match input {
                Input::IronBar => Some(Mat::Iron),
                Input::Bar => first_mat,
                _ => None,
            };
            let mut chosen: Option<Want> = None;
            for &src in sources(input) {
                let queued = match src {
                    Source::Craft(r) => self.feed_child(r, metal.and_then(ore_for), recipe, short, depth, stock),
                    Source::Butcher => self.feed_child(Recipe::Butcher, None, recipe, 1, depth, stock),
                    Source::Mine(f) => {
                        let f = match (f, input == Input::Ore, first_mat) {
                            (MatFilter::Ore(None), true, Some(m)) => MatFilter::Ore(Some(m)),
                            _ => f,
                        };
                        let ok = self.gather_mine(f, short.min(4));
                        if ok {
                            chosen = Some(Want { input, via: Source::Mine(f), queued: true });
                            break;
                        }
                        false
                    }
                    Source::Chop => self.gather_chop(),
                    // Soldiers hunt for an overseer's fort; a player says so himself.
                    Source::Hunt => {
                        let butcher = self.buildings.values().any(|b| b.built && b.kind == BuildingKind::Butcher);
                        if self.overseer.is_some() && butcher {
                            self.hunt = true;
                            true
                        } else {
                            false
                        }
                    }
                    Source::Farm | Source::Fish | Source::Buy => false,
                };
                if queued {
                    chosen = Some(Want { input, via: src, queued: true });
                    break;
                }
            }
            let want = chosen.unwrap_or(Want { input, via: sources(input)[0], queued: false });
            if !want.queued {
                if let Some(l) = troubles::Lack::from_input(input) {
                    self.trouble(l);
                }
            }
            wants.push(want);
        }
        wants
    }

    /// A child order for `r` (of ore `mat`), wanted by `parent` for `short`
    /// more of its input; and what that one needs in turn. False when no
    /// workshop can do it.
    fn feed_child(&mut self, r: Recipe, mat: Option<Mat>, parent: Recipe, short: usize, depth: u8, stock: &Stock) -> bool {
        if depth >= MAX_DEPTH || !self.buildings.values().any(|b| b.built && b.kind.recipes().contains(&r)) {
            return false;
        }
        let (_, per) = r.product();
        let per = per.max(1);
        let jobs = (short + per - 1) / per;
        // Keep the product in stock up to what the parent needs.
        let keep = (self.order_have_of(r, mat) + short) as u32;
        let top = self.work_orders.iter().any(|o| o.recipe == r && o.mat == mat && o.parent.is_none());
        if !top {
            match self.work_orders.iter_mut().find(|o| o.recipe == r && o.mat == mat && o.parent.is_some()) {
                Some(o) => {
                    if o.keep < keep {
                        o.keep = keep;
                    }
                }
                None => {
                    let mut o = WorkOrder::new(r, keep);
                    o.mat = mat;
                    o.parent = Some(parent);
                    self.work_orders.push(o);
                }
            }
        }
        self.feed_mat(r, mat, jobs as u32, depth + 1, stock);
        true
    }

    /// Designate the nearest known tiles of what `f` wants for mining (up to
    /// `n`). True if some are designated already or now.
    fn gather_mine(&mut self, f: MatFilter, n: usize) -> bool {
        // Plain stone is quarried as rooms are (the overseer's storeroom, a
        // player's own digging): the nearest stone wall is too often a
        // corridor's, and a way round the gate.
        if f == MatFilter::BuildingStone {
            return false;
        }
        let pending = self
            .designations
            .iter()
            .filter(|p| {
                let t = self.world.t(**p);
                t.desig == Desig::Dig && t.kind == TileKind::Wall && f.matches(t.mat)
            })
            .count();
        if pending > 0 {
            return true;
        }
        // (Looked for at most twice a day: the scan is a big box.)
        if !self.due_once(&format!("mine{:?}", f), DAY / 2) {
            return false;
        }
        let base = self.overseer.as_ref().and_then(|o| o.base);
        // (Nor a room's walls: flux is limestone, and halls are cut in it.)
        let rooms: Vec<(Pos, Pos)> = self.overseer.as_ref().map_or(Vec::new(), |o| o.rooms.iter().map(|r| (r.a, r.b)).collect());
        let by_room = |p: Pos| rooms.iter().any(|&(a, b)| p.z == a.z && p.x >= a.x.min(b.x) - 1 && p.x <= a.x.max(b.x) + 1 && p.y >= a.y.min(b.y) - 1 && p.y <= a.y.max(b.y) + 1);
        let from = base.unwrap_or(self.embark);
        let mut found: Vec<(i32, Pos)> = Vec::new();
        let reach = 40;
        for dz in -12..=2 {
            for dy in -reach..=reach {
                for dx in -reach..=reach {
                    let p = from.offset(dx, dy, dz);
                    let Some(t) = self.world.get(p) else { continue };
                    if !(t.revealed && t.kind == TileKind::Wall && t.desig == Desig::None && t.built_from.is_none() && f.matches(t.mat)) {
                        continue;
                    }
                    // Never the gatehouse's walls, nor the rock round the waterworks.
                    if base.map_or(false, |b| Self::gatehouse(b, p)) || self.by_waterworks(p) || by_room(p) {
                        continue;
                    }
                    // Only what a miner can stand beside.
                    let reachable = DIRS8.iter().any(|&(ex, ey)| self.world.get(p.offset(ex, ey, 0)).map_or(false, |q| q.dug && !q.kind.solid() && q.water <= 3));
                    if reachable {
                        found.push((dx.abs() + dy.abs() + dz.abs() * 3, p));
                    }
                }
            }
        }
        found.sort_by_key(|f| (f.0, f.1.z, f.1.y, f.1.x));
        let mut any = false;
        for &(_, p) in found.iter().take(n.max(1)) {
            any |= self.designate(p, p, Desig::Dig) > 0;
        }
        any
    }

    /// Trees marked for felling, or a ring of them marked now.
    fn gather_chop(&mut self) -> bool {
        if self.designations.iter().any(|p| self.world.t(*p).desig == Desig::Chop) {
            return true;
        }
        if !self.due_once("chopfeed", DAY / 2) {
            return false;
        }
        let e = self.embark;
        self.designate(e.offset(-30, -30, -2), e.offset(30, 30, 2), Desig::Chop) > 0
    }

    /// True at most once per `every` ticks for a key (and marks it).
    pub(super) fn due_once(&mut self, key: &str, every: u64) -> bool {
        let last = self.warns.get(key).copied();
        if last.map_or(true, |t| self.tick >= t + every) {
            self.warns.insert(key.to_string(), self.tick);
            true
        } else {
            false
        }
    }

    /// Announce at most once per `every` ticks for a key.
    pub(super) fn warn_once(&mut self, key: &str, text: &str, every: u64, color: (u8, u8, u8)) {
        if self.due_once(key, every) {
            self.msg(text, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fort with a smelter, wood furnace and forge built beside the
    /// embark, a manager, and hematite and limestone showing in the rock.
    fn smithy() -> (Game, Pos, Pos) {
        let mut g = Game::new(77);
        g.creatures.retain(|_, c| c.tame);
        let e = g.embark;
        for k in [BuildingKind::Smelter, BuildingKind::WoodFurnace, BuildingKind::Forge] {
            let site = g.ov_spot(k, e.offset(3, 0, 0), 25).expect("room for a workshop");
            assert!(g.place_building(k, site));
            let id = g.world.t(site).building;
            g.buildings.get_mut(&id).unwrap().built = true;
        }
        let m = g.dwarves[0].id;
        g.dwarves[0].personality.diligence = 40;
        g.appoint(super::super::Position::Manager, Some(m));
        // Two veins in a dug pocket's wall, in sight of the embark.
        let hole = e.offset(-6, 6, 0);
        let mut veins = (Pos::new(0, 0, 0), Pos::new(0, 0, 0));
        for (i, mat) in [Mat::Hematite, Mat::Limestone].into_iter().enumerate() {
            let p = hole.offset(i as i32 * 3, 0, 0);
            {
                let t = g.world.t_mut(p);
                t.kind = TileKind::Wall;
                t.mat = mat;
                t.revealed = true;
                t.desig = Desig::None;
            }
            let q = p.offset(0, 1, 0);
            let t = g.world.t_mut(q);
            t.kind = TileKind::Floor;
            t.dug = true;
            t.water = 0;
            if i == 0 { veins.0 = p } else { veins.1 = p }
        }
        g.items.retain(|_, it| !matches!(it.kind, ItemKind::Bar | ItemKind::Charcoal) && !(it.kind == ItemKind::Boulder && (it.mat.smelts_to().is_some() || it.mat == Mat::Limestone || it.mat == Mat::Coal)));
        (g, veins.0, veins.1)
    }

    #[test]
    fn an_order_for_steel_queues_iron_charcoal_and_flux() {
        let (mut g, hematite, limestone) = smithy();
        g.add_work_order(Recipe::MakeSteel, 2);
        g.manage_orders();
        let o = &g.work_orders;
        assert!(o.iter().any(|w| w.recipe == Recipe::SmeltOre && w.parent == Some(Recipe::MakeSteel) && w.mat == Some(Mat::Hematite)), "no iron ordered: {:?}", o);
        assert!(o.iter().any(|w| w.recipe == Recipe::Charcoal), "no charcoal ordered: {:?}", o);
        assert_eq!(g.world.t(hematite).desig, Desig::Dig, "nobody sent for the hematite");
        assert_eq!(g.world.t(limestone).desig, Desig::Dig, "nobody sent for the flux");
        let steel = o.iter().find(|w| w.recipe == Recipe::MakeSteel).unwrap();
        let said: Vec<String> = steel.waiting.iter().map(|w| w.describe()).collect();
        assert!(said.iter().any(|s| s.contains("flux (miners sent)")), "{:?}", said);
        // Met or removed, the parent takes its children with it.
        g.work_orders.retain(|w| w.recipe != Recipe::MakeSteel);
        for b in g.buildings.values_mut() {
            b.orders.clear();
        }
        g.manage_orders();
        assert!(!g.work_orders.iter().any(|w| w.parent.is_some()), "orphans left: {:?}", g.work_orders);
    }

    #[test]
    fn no_announcement_repeats_a_missing_input() {
        let (mut g, _, limestone) = smithy();
        // No flux anywhere: the vein is gone.
        g.world.t_mut(limestone).mat = Mat::Granite;
        for _ in 0..3 {
            let e = g.embark;
            g.spawn_item(ItemKind::Bar, Mat::Iron, e);
            g.spawn_item(ItemKind::Charcoal, Mat::Coal, e);
        }
        g.add_work_order(Recipe::MakeSteel, 2);
        let start = g.log.len();
        for _ in 0..10 * DAY {
            g.step();
        }
        // (The prospector's report names the flux too; it isn't a complaint.)
        let wants = |l: &&super::LogEntry| l.text.contains("flux") && !l.text.starts_with("The prospector");
        let flux = g.log[start..].iter().filter(wants).count();
        assert!(flux <= 1, "said {} times: {:?}", flux, g.log[start..].iter().filter(wants).map(|l| &l.text).collect::<Vec<_>>());
    }
}
