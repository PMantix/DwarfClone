//! How far a fort has come, and what it rebuilds once it's established.
//!
//! A fort is founded (a dormitory, workshops by the wagon), settles (the
//! essentials dug and fitted), becomes established (people, wealth, a
//! season of peace) and in time comes of age. Once established the
//! overseer rebuilds its rough beginnings properly: an industry quarter
//! below ground, one hall for each chain of workshops (food, wood, stone,
//! metal, cloth) with the stockpile they draw on beside them; the makeshift
//! workshops by the wagon and the piles in the open are cleared once their
//! replacements stand, never while someone is at work in them; and a
//! hospital wing takes over from the first small ward.

use super::architect::{room_tiles, taste};
use super::overseer::{Overseer, Room, RoomUse, Style};
use super::*;

/// A chain of workshops that belong together.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Chain {
    Food,
    Wood,
    Stone,
    Metal,
    Cloth,
}

impl Chain {
    pub const ALL: [Chain; 5] = [Chain::Food, Chain::Wood, Chain::Stone, Chain::Metal, Chain::Cloth];

    pub fn shops(self) -> &'static [BuildingKind] {
        use BuildingKind::*;
        match self {
            Chain::Food => &[Still, Kitchen, Butcher],
            Chain::Wood => &[Carpenter, WoodFurnace],
            Chain::Stone => &[Mason, Mechanic, Craftsdwarf],
            Chain::Metal => &[Smelter, Forge],
            Chain::Cloth => &[Loom, Clothier, Leatherworks],
        }
    }

    /// The stockpile beside its workshops.
    pub fn pile(self) -> StockFilter {
        match self {
            Chain::Food => StockFilter::Food,
            Chain::Wood => StockFilter::Wood,
            Chain::Stone => StockFilter::Stone,
            Chain::Metal => StockFilter::Metal,
            Chain::Cloth => StockFilter::Goods,
        }
    }

    pub fn hall(self) -> &'static str {
        match self {
            Chain::Food => "the Kitchens",
            Chain::Wood => "the Woodworks",
            Chain::Stone => "the Stoneworks",
            Chain::Metal => "the Forge Hall",
            Chain::Cloth => "the Weavers' Hall",
        }
    }

    pub fn wish(self) -> &'static str {
        match self {
            Chain::Food => "kitchens for the industry quarter",
            Chain::Wood => "a woodworks for the industry quarter",
            Chain::Stone => "a stoneworks for the industry quarter",
            Chain::Metal => "a forge hall for the industry quarter",
            Chain::Cloth => "a weavers' hall for the industry quarter",
        }
    }
}

impl Game {
    /// Daily: how far the fort has come. It only ever goes up.
    pub(super) fn ov_tier(&mut self, o: &mut Overseer) {
        let t = taste(o);
        let day = self.tick / DAY;
        let pop = self.alive_count();
        let fitted = o.rooms.iter().filter(|r| r.stage == 1).count();
        let peace = o.last_siege == 0 || self.tick > o.last_siege + SEASON / 2;
        let k = |name: &'static str, d: f64| self.knob(name, d);
        // (A builder's overseer is quicker to think of its halls.)
        let quick = if o.style == Style::Builder { 0.75 } else { 1.0 };
        let mut tier = 0u8;
        if day as f64 >= k("interior.settled_day", 40.0) * quick && pop >= 7 && fitted >= 4 {
            tier = 1;
        }
        if tier == 1 && day as f64 >= k("interior.established_day", 168.0) * quick && pop >= 12 && fitted >= 8 && peace && self.wealth() as f64 >= k("interior.established_wealth", 10000.0) * quick {
            tier = 2;
        }
        if tier == 2 && day as f64 >= k("interior.mature_day", 336.0) * quick && pop >= 20 && peace && self.wealth() as f64 >= k("interior.mature_wealth", 20000.0) * quick {
            tier = 3;
        }
        if t.plain {
            tier = tier.min(1);
        }
        if tier > o.interior.tier {
            o.interior.tier = tier;
            o.interior.tier_since = self.tick;
            let what = match tier {
                1 => "The fortress has found its feet: time the dwarves had rooms of their own.",
                2 => "The fortress is established: it will build properly now, and clear away the rough early work.",
                _ => "The fortress has come of age: its halls should show it.",
            };
            Self::note_ov(o, self.tick, what.into());
            self.count(&format!("interior.tier{}", tier), 1);
        }
    }

    /// The chain the next hall of the industry quarter is for: one whose
    /// workshops the fort has, not yet housed in the quarter.
    pub(super) fn quarter_wish(&self, o: &Overseer) -> Option<Chain> {
        if taste(o).plain || o.interior.tier < 2 || o.rooms.iter().any(|r| r.kind == RoomUse::Industry && r.stage == 0) {
            return None;
        }
        self.next_chain(o)
    }

    fn next_chain(&self, o: &Overseer) -> Option<Chain> {
        Chain::ALL.iter().copied().find(|c| !o.interior.quarter.iter().any(|q| q.1 == *c) && c.shops().iter().any(|k| self.buildings.values().any(|b| b.kind == *k && b.built)))
    }

    /// Fit out a hall of the quarter: its chain's workshops in a row along
    /// one wall, its stockpile across the rest.
    pub(super) fn fit_quarter(&mut self, o: &mut Overseer, r: &Room) {
        let chain = self.next_chain(o).unwrap_or(Chain::Stone);
        self.fit_quarter_for(o, r, chain);
    }

    pub(super) fn fit_quarter_for(&mut self, o: &mut Overseer, r: &Room, chain: Chain) {
        let (w, h) = (r.b.x - r.a.x + 1, r.b.y - r.a.y + 1);
        let tiles = room_tiles(r);
        // Shops along the long side, three tiles deep; the pile beyond a lane.
        let along_x = w >= h;
        let (l, d) = if along_x { (w, h) } else { (h, w) };
        let at = |s: i32, q: i32| if along_x { r.a.offset(s, q, 0) } else { r.a.offset(q, s, 0) };
        // The stockpile first: it can't be laid over the shops.
        if d >= 5 {
            let pile: Vec<Pos> = (0..l).flat_map(|s| (4..d).map(move |q| (s, q))).map(|(s, q)| at(s, q)).filter(|p| tiles.contains(p)).collect();
            self.add_zone_tiles(pile, ZoneKind::Stockpile(chain.pile()));
        }
        let mut s = 0;
        let mut shops = chain.shops().iter().copied().peekable();
        while let Some(&k) = shops.peek() {
            if s + 2 >= l {
                break;
            }
            if self.place_building(k, at(s, 0)) {
                shops.next();
                s += 4;
            } else {
                s += 1;
            }
        }
        o.interior.quarter.push((r.a, chain));
        self.count("interior.quarter", 1);
    }

    /// Whether the fort has outgrown its first ward (and has asked for no
    /// wing yet).
    pub(super) fn wing_wanted(&self, o: &Overseer) -> bool {
        o.style != Style::Reckless
            && !o.did("wing")
            && self.alive_count() >= 15
            && o.rooms.iter().any(|r| r.kind == RoomUse::Hospital && r.stage == 1)
            && o.rooms.iter().filter(|r| r.kind == RoomUse::Hospital && r.stage < 2).all(|r| room_tiles(r).len() < 50)
    }

    /// Daily, once established: the rough beginnings give way to what
    /// replaced them.
    pub(super) fn ov_mature(&mut self, o: &mut Overseer) {
        if taste(o).plain || o.interior.tier < 2 {
            return;
        }
        self.retire_rough_shops(o);
        self.retire_open_piles(o);
        self.retire_old_ward(o);
    }

    /// A workshop by the wagon whose like now stands built in the quarter
    /// is taken down (its queue handed to the new one): one a day, and never
    /// while somebody works at it.
    pub(super) fn retire_rough_shops(&mut self, o: &mut Overseer) -> bool {
        let quarter: Vec<(Vec<Pos>, Chain)> = o.interior.quarter.iter().filter_map(|&(a, c)| o.rooms.iter().find(|r| r.a == a && r.kind == RoomUse::Industry).map(|r| (room_tiles(r), c))).collect();
        let in_quarter = |p: Pos| quarter.iter().any(|q| q.0.contains(&p));
        let new: Vec<(BuildingKind, u32, Chain)> = self.buildings.values().filter(|b| b.built && in_quarter(b.pos)).filter_map(|b| quarter.iter().find(|q| q.0.contains(&b.pos)).map(|q| (b.kind, b.id, q.1))).collect();
        for (kind, new_id, chain) in new {
            let busy = |g: &Game, id: u32| {
                g.buildings.get(&id).map_or(false, |b| b.worker.is_some())
                    || g.dwarves.iter().any(|d| d.alive && matches!(d.task, Task::Craft { bld, .. } | Task::Construct { bld, .. } if bld == id))
            };
            let old = self.buildings.values().filter(|b| b.kind == kind && b.id != new_id && !in_quarter(b.pos) && self.world.outside(b.pos)).map(|b| b.id).min();
            let Some(old) = old else { continue };
            if busy(self, old) {
                continue;
            }
            let orders = std::mem::take(&mut self.buildings.get_mut(&old).unwrap().orders);
            if let Some(b) = self.buildings.get_mut(&new_id) {
                b.orders.extend(orders);
            }
            let name = self.buildings[&old].kind.name().to_lowercase();
            self.remove_building(old);
            Self::note_ov(o, self.tick, format!("Took down the makeshift {} by the wagon: the new one in {} has taken over.", name, chain.hall()));
            self.count("interior.shop_retired", 1);
            return true;
        }
        false
    }

    /// The wood and stone piles in the open by the wagon go once the
    /// quarter has its own.
    pub(super) fn retire_open_piles(&mut self, o: &mut Overseer) {
        for chain in [Chain::Wood, Chain::Stone] {
            let housed = o.interior.quarter.iter().any(|q| q.1 == chain) && self.zones.values().any(|z| z.kind == ZoneKind::Stockpile(chain.pile()) && z.tiles.iter().all(|p| !self.world.outside(*p)));
            if !housed {
                continue;
            }
            let open: Vec<u32> = self.zones.values().filter(|z| z.kind == ZoneKind::Stockpile(chain.pile()) && z.tiles.iter().all(|p| self.world.outside(*p))).map(|z| z.id).collect();
            for id in open {
                let tiles = self.zones[&id].tiles.clone();
                for p in tiles {
                    self.remove_zones_where(p, p, |k| k == ZoneKind::Stockpile(chain.pile()));
                }
                let what = if chain == Chain::Wood { "the logs go to the Woodworks now" } else { "the stone goes to the Stoneworks now" };
                Self::note_ov(o, self.tick, format!("Cleared the old {} pile by the wagon: {}.", if chain == Chain::Wood { "wood" } else { "stone" }, what));
            }
        }
    }

    /// Room it wanted and found no rock for is made over from a storeroom
    /// (when the fort has three or more for everything): the quarter's
    /// halls, a suite, the great dining hall, the hospital wing, a tavern...
    /// At most one a week.
    pub(super) fn ov_repurpose(&mut self, o: &mut Overseer) {
        if taste(o).plain || o.interior.tier < 2 || self.tick < o.interior.repurposed + DAY * 7 {
            return;
        }
        let tick = self.tick;
        let have = |k: RoomUse| o.rooms.iter().filter(|r| r.kind == k && r.stage < 2).count();
        let blocked = |k: RoomUse| o.blocked_lately(k, tick);
        // Most wanted first: a noble's rooms, a tavern, the great hall, a
        // temple, the quarter, the rest.
        let mut want: Vec<RoomUse> = Vec::new();
        if blocked(RoomUse::Suite) {
            want.push(RoomUse::Suite);
        }
        if blocked(RoomUse::Tavern) && have(RoomUse::Tavern) == 0 {
            want.push(RoomUse::Tavern);
        }
        if blocked(RoomUse::Dining) && o.interior.great.is_none() && o.interior.great_wanted > 0 {
            want.push(RoomUse::Dining);
        }
        if blocked(RoomUse::Temple) && have(RoomUse::Temple) == 0 {
            want.push(RoomUse::Temple);
        }
        if blocked(RoomUse::Industry) && self.next_chain(o).is_some() {
            want.push(RoomUse::Industry);
        }
        if blocked(RoomUse::Hospital) && o.did("wing") && !o.rooms.iter().any(|r| r.kind == RoomUse::Hospital && r.stage == 1 && room_tiles(r).len() >= 50) {
            want.push(RoomUse::Hospital);
        }
        for k in [RoomUse::Barracks, RoomUse::Library, RoomUse::Meeting] {
            if blocked(k) && have(k) == 0 {
                want.push(k);
            }
        }
        let Some(&kind) = want.first() else { return };
        let general: Vec<usize> = (0..o.rooms.len())
            .filter(|&i| {
                let r = &o.rooms[i];
                r.stage == 1 && r.kind == RoomUse::Stockpile && room_tiles(r).iter().any(|p| self.zone_kind_at(*p) == Some(ZoneKind::Stockpile(StockFilter::All)))
            })
            .collect();
        // (Two kept for everything else; a noble's rooms, a tavern or the quarter may take the third.)
        // (The quarter's halls keep piles of their own.)
        if general.len() < if matches!(kind, RoomUse::Suite | RoomUse::Tavern | RoomUse::Industry) { 2 } else { 3 } {
            return;
        }
        let need = match kind {
            RoomUse::Dining | RoomUse::Hospital => 50,
            RoomUse::Industry => 40,
            RoomUse::Suite => 35,
            _ => 30,
        };
        let size = |i: &usize| room_tiles(&o.rooms[*i]).len();
        // (Workshops can't stand among pillars.)
        let pillared = |i: &usize| matches!(o.rooms[*i].shape, Some(super::layout::Shape::Pillared { .. }));
        let fits: Vec<usize> = general.iter().copied().filter(|i| size(i) >= need && !(kind == RoomUse::Industry && pillared(i))).collect();
        // The biggest for a great hall or a ward, the smallest that will do otherwise.
        let pick = if matches!(kind, RoomUse::Dining | RoomUse::Hospital) { fits.iter().copied().max_by_key(|i| size(i)) } else { fits.iter().copied().min_by_key(|i| size(i)) };
        let Some(i) = pick else { return };
        let r = o.rooms[i].clone();
        self.remove_zones_where(r.a, r.b, |k| matches!(k, ZoneKind::Stockpile(_)));
        o.rooms[i].kind = kind;
        o.rooms[i].since = tick;
        let room = o.rooms[i].clone();
        if kind == RoomUse::Dining {
            o.interior.great = Some(room.a);
        }
        if let Some(z) = self.fit_interior(o, &room) {
            self.add_zone(room.a, room.b, z);
        }
        o.blocked.retain(|b| b.0 != kind);
        o.interior.repurposed = tick;
        let what = match kind {
            RoomUse::Industry => format!("Cleared out a storeroom to make {}.", o.interior.quarter.last().map_or("a workshop hall", |q| q.1.hall())),
            RoomUse::Dining => "Cleared out the biggest storeroom to make a great dining hall.".to_string(),
            RoomUse::Hospital => "Cleared out the biggest storeroom to make a hospital wing.".to_string(),
            k => format!("Cleared out a storeroom to make a {}.", super::overseer::room_name(k)),
        };
        Self::note_ov(o, tick, what);
        self.count("interior.repurposed", 1);
    }

    /// Once the hospital wing is fitted, the first small ward is emptied
    /// (when nobody lies in it) and becomes a parlour.
    fn retire_old_ward(&mut self, o: &mut Overseer) {
        if !o.did("wing") || o.did("ward-retired") {
            return;
        }
        let wards: Vec<usize> = (0..o.rooms.len()).filter(|&i| o.rooms[i].kind == RoomUse::Hospital && o.rooms[i].stage == 1).collect();
        let Some(&wing) = wards.iter().max_by_key(|&&i| room_tiles(&o.rooms[i]).len()) else { return };
        if wards.len() < 2 || room_tiles(&o.rooms[wing]).len() < 50 {
            return;
        }
        let old = *wards.iter().find(|&&i| i != wing).unwrap();
        let r = o.rooms[old].clone();
        let tiles = room_tiles(&r);
        let beds: Vec<u32> = self.buildings.values().filter(|b| b.kind == BuildingKind::Bed && tiles.contains(&b.pos)).map(|b| b.id).collect();
        let laid_up = self.dwarves.iter().any(|d| d.alive && (tiles.contains(&d.pos) && d.asleep || matches!(d.task, Task::Sleep { bed: Some(b), .. } | Task::Rest { bed: Some(b), .. } if beds.contains(&b))));
        // (And the wing must have beds standing first.)
        let wing_tiles = room_tiles(&o.rooms[wing]);
        let wing_beds = self.buildings.values().filter(|b| b.kind == BuildingKind::Bed && b.built && wing_tiles.contains(&b.pos)).count();
        if laid_up || wing_beds < 3 {
            return;
        }
        for b in beds {
            self.remove_building(b);
        }
        self.remove_zones_where(r.a, r.b, |k| matches!(k, ZoneKind::Hospital | ZoneKind::Stockpile(StockFilter::Medical)));
        o.rooms[old].kind = RoomUse::Parlour;
        o.rooms[old].since = self.tick;
        let room = o.rooms[old].clone();
        if let Some(z) = self.fit_interior(o, &room) {
            self.add_zone(room.a, room.b, z);
        }
        o.done.push("ward-retired".into());
        Self::note_ov(o, self.tick, "Moved the sick to the new hospital wing: the old ward will be a parlour.".into());
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::buildings::Order;
    use crate::game::overseer::trial_game;

    /// A fitted storeroom of a dug-in fort, emptied, to build in.
    pub(crate) fn empty_room(g: &mut Game, o: &Overseer, size: usize) -> usize {
        let plain = |r: &Room| !matches!(r.shape, Some(super::super::layout::Shape::Pillared { .. }));
        let i = o.rooms.iter().position(|r| r.stage == 1 && r.kind == RoomUse::Stockpile && plain(r) && room_tiles(r).len() >= size).expect("a fitted storeroom");
        let r = o.rooms[i].clone();
        g.remove_zones(r.a, r.b);
        let tiles = room_tiles(&r);
        let inside: Vec<u32> = g.buildings.values().filter(|b| b.tiles().iter().any(|p| tiles.contains(p))).map(|b| b.id).collect();
        for id in inside {
            g.remove_building(id);
        }
        i
    }

    #[test]
    fn a_makeshift_workshop_gives_way_to_the_quarter_but_never_while_busy() {
        let mut g = trial_game(5003, Style::Steady, 45);
        let mut o = g.overseer.take().unwrap();
        o.interior.tier = 2;
        let old = g.buildings.values().find(|b| b.kind == BuildingKind::Carpenter && g.world.outside(b.pos)).map(|b| b.id).expect("a carpenter's workshop by the wagon");
        g.buildings.get_mut(&old).unwrap().orders.push(Order { recipe: Recipe::WoodBed, repeat: false });
        // A storeroom made into the woodworks.
        let i = empty_room(&mut g, &o, 40);
        o.rooms[i].kind = RoomUse::Industry;
        let r = o.rooms[i].clone();
        g.fit_quarter_for(&mut o, &r, Chain::Wood);
        let tiles = room_tiles(&r);
        let new = g.buildings.values().find(|b| b.kind == BuildingKind::Carpenter && tiles.contains(&b.pos)).map(|b| b.id).expect("a carpenter's workshop in the woodworks");
        assert!(tiles.iter().any(|p| g.zone_kind_at(*p) == Some(ZoneKind::Stockpile(StockFilter::Wood))), "a wood pile beside it");
        // Not built yet: the old one stays.
        for d in g.dwarves.iter_mut() {
            d.task = Task::Idle;
        }
        assert!(!g.retire_rough_shops(&mut o));
        assert!(g.buildings.contains_key(&old), "torn down before its replacement stood");
        g.buildings.get_mut(&new).unwrap().built = true;
        // Somebody at work in the old one: it stays.
        let w = g.dwarves[0].id;
        g.buildings.get_mut(&old).unwrap().worker = Some(w);
        assert!(!g.retire_rough_shops(&mut o));
        assert!(g.buildings.contains_key(&old), "torn down while in use");
        // Free: down it comes, and its queue goes to the new one.
        g.buildings.get_mut(&old).unwrap().worker = None;
        assert!(g.retire_rough_shops(&mut o));
        assert!(!g.buildings.contains_key(&old));
        assert!(g.buildings[&new].orders.iter().any(|x| x.recipe == Recipe::WoodBed), "the old queue moved over");
        assert!(o.notes.iter().any(|n| n.1.contains("Took down the makeshift carpenter's workshop")), "{:?}", o.notes.last());
        // And the wood pile by the wagon goes too.
        let open = |g: &Game| g.zones.values().any(|z| z.kind == ZoneKind::Stockpile(StockFilter::Wood) && z.tiles.iter().all(|p| g.world.outside(*p)));
        assert!(open(&g), "a wood pile by the wagon to begin with");
        g.retire_open_piles(&mut o);
        assert!(!open(&g), "the wood pile by the wagon was cleared");
    }

    #[test]
    fn every_chain_has_shops_and_a_pile() {
        for c in Chain::ALL {
            assert!(!c.shops().is_empty() && c.shops().iter().all(|k| k.is_workshop()));
            assert!(c.hall().starts_with("the "));
            let _ = c.pile();
        }
    }
}
