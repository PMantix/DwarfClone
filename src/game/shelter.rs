//! The shelter holds: what keeps the alert burrow safe while an army sits
//! outside. The gate's own tiles (the foot of the stair, the doors, the
//! bridge) are no part of it: a dwarf who stopped there when the bridge went
//! up was shut out with the enemy, and a third of those who died "inside
//! the shelter" died on the gate's doorstep that way.
//!
//! The shelter keeps a larder: a stockpile inside it for drink and meals,
//! filled before the enemy comes and topped up from the stores outside, so
//! a long siege doesn't end in thirst with the barrels out of reach.

use super::overseer::Overseer;
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// What the overseer keeps for the shelter (`Game.shelter`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Shelter {
    /// The civilian alert as it was at the last check, to see it raised.
    #[serde(default)]
    pub was_alert: bool,
    /// The shelter's larder (a stockpile zone), once it has one.
    #[serde(default)]
    pub larder: Option<u32>,
    /// Drink and meals still wanted in the shelter, and when that was
    /// counted (the haulers ask often; it's counted now and then).
    #[serde(skip)]
    pub want: (u64, u32, u32),
    /// Ways into the shelter past the gate walled up so far.
    #[serde(default)]
    pub walled: u32,
    /// When the ways in were last looked for during an alert.
    #[serde(skip)]
    pub ways_checked: u64,
    /// When the alert now on went up.
    #[serde(default)]
    pub alert_since: u64,
    /// The shelter has sung in the alert now on.
    #[serde(default)]
    pub sang: bool,
}

/// What the shelter's larder takes: drink and meals, nothing else.
const LARDER_DENIES: [ItemKind; 5] = [ItemKind::Plant, ItemKind::Seed, ItemKind::Meat, ItemKind::Fish, ItemKind::Flour];

impl Game {
    /// The shelter's burrow mask, if there is a shelter.
    pub(super) fn shelter_bits(&self) -> Option<u8> {
        self.burrows.iter().find(|b| Some(b.bit) == self.alert_burrow).map(|b| b.mask())
    }

    /// Whether `p` lies in the shelter.
    #[cfg(test)]
    pub(super) fn in_shelter(&self, p: Pos) -> bool {
        self.shelter_bits().map_or(false, |m| self.world.get(p).map_or(false, |t| t.burrows & m != 0))
    }

    /// The gate's own tiles, outside the drawbridge: the foot of the stair,
    /// the doors and the bridge itself, and the rock round them.
    fn gate_tiles(base: Pos) -> impl Iterator<Item = Pos> {
        box_iter(base.offset(-2, -1, 0), base.offset(2, 2, 0))
    }

    /// Daily (`overseer_hooks.rs`).
    pub(super) fn ov_shelter(&mut self, o: &mut Overseer) {
        let Some(base) = o.base else { return };
        if self.alert_burrow.is_none() {
            return;
        }
        self.gate_out_of_shelter(base);
        self.wall_up_ways_in(o, base);
        self.ov_larder_in_shelter(o, base);
    }

    /// Every ten ticks (`overseer_hooks.rs`): when the alert is raised, the
    /// shelter is made ready at once.
    /// While it's up, the ways in are looked for again now and then.
    pub(super) fn ov_shelter_alert(&mut self, o: &mut Overseer) {
        let Some(base) = o.base else { return };
        if self.civ_alert != self.shelter.was_alert {
            self.shelter.was_alert = self.civ_alert;
            if self.civ_alert {
                self.shelter.alert_since = self.tick;
                self.shelter.sang = false;
                self.gate_out_of_shelter(base);
                self.shelter.ways_checked = self.tick;
                self.wall_up_ways_in(o, base);
            }
        } else if self.civ_alert && self.tick >= self.shelter.ways_checked + 100 {
            self.shelter.ways_checked = self.tick;
            self.wall_up_ways_in(o, base);
        }
        if self.civ_alert && !self.shelter.sang && self.tick >= self.shelter.alert_since + self.knob("shelter.sing_after", 400.0) as u64 {
            self.shelter_sings();
        }
    }

    /// The shelter sings: a while into a siege, those waiting it out in the
    /// shelter take up a song together, and take heart from it; a lament,
    /// if the siege has already cost the fort someone.
    pub(super) fn shelter_sings(&mut self) {
        let Some(mask) = self.shelter_bits() else { return };
        let inside: Vec<usize> = (0..self.dwarves.len())
            .filter(|&i| {
                let d = &self.dwarves[i];
                d.alive && !d.soldier && !d.insane && self.world.get(d.pos).map_or(false, |t| t.burrows & mask != 0)
            })
            .collect();
        if inside.len() < 3 {
            return;
        }
        self.shelter.sang = true;
        let siege = self.annals.sieges.iter().rposition(|s| s.ended.is_none());
        let fallen = siege.map_or(0, |i| self.annals.deaths.iter().filter(|d| d.siege == Some(i)).count());
        let leader = inside[self.rng.range(0, inside.len() as i32) as usize];
        let who = self.dwarves[leader].name.clone();
        let fort = self.fortress_name.clone();
        let tick = self.tick;
        let (text, thought, stress) = if fallen > 0 {
            (
                format!("In the shelter of {}, {} begins a lament for the fallen, and one by one the others take it up.", fort, who),
                "sang a lament for the fallen in the shelter",
                -4,
            )
        } else {
            (
                format!("In the shelter of {}, {} starts up an old song, and soon the whole shelter is singing while the enemy waits outside.", fort, who),
                "sang with the others in the shelter while the enemy waited outside",
                -8,
            )
        };
        for i in inside.iter().copied() {
            self.dwarves[i].think(thought, stress, tick);
        }
        self.msg(&text, (170, 200, 255));
        self.count("shelter.songs", 1);
    }

    /// The ways into the shelter that don't pass the gate: for each, the
    /// tile in the shelter and the one outside it that a foe would step in
    /// from. Found walking from the top of the stair (where an army comes)
    /// over everything but the shelter and the gate's tiles, through any
    /// door but a locked one; a raised bridge stops it too. (A pit open to
    /// the sky that nothing outside can walk into is no way in.)
    #[cfg(test)]
    pub fn ways_in(&self, base: Pos, top: Pos) -> Vec<(Pos, Pos)> {
        self.search_ways_in(base, top).0
    }

    /// `ways_in`, and the way back to the top of the stair from every tile
    /// it walked.
    fn search_ways_in(&self, base: Pos, top: Pos) -> (Vec<(Pos, Pos)>, HashMap<Pos, Pos>) {
        let mut from: HashMap<Pos, Pos> = HashMap::new();
        let mut found: Vec<(Pos, Pos)> = Vec::new();
        let Some(mask) = self.shelter_bits() else { return (found, from) };
        let w = &self.world;
        let inside = |p: Pos| w.get(p).map_or(false, |t| t.burrows & mask != 0);
        let gate: HashSet<Pos> = Self::gate_tiles(base).collect();
        let mut todo: VecDeque<Pos> = VecDeque::new();
        let mut nb = Vec::new();
        if inside(top) || gate.contains(&top) {
            return (found, from);
        }
        from.insert(top, top);
        todo.push_back(top);
        let mut budget = self.knob("shelter.ways_search", 400_000.0) as usize;
        while let Some(p) = todo.pop_front() {
            budget = budget.saturating_sub(1);
            if budget == 0 {
                break;
            }
            w.neighbors(p, &mut nb, Mover::Goblin);
            for &(q, _) in nb.iter() {
                if inside(q) {
                    if !found.contains(&(q, p)) {
                        found.push((q, p));
                    }
                } else if !gate.contains(&q) && !from.contains_key(&q) {
                    from.insert(q, p);
                    todo.push_back(q);
                }
            }
        }
        (found, from)
    }

    /// Whether walling `s` leaves the shelter round it in one piece.
    fn shelter_whole_without(&self, s: Pos, mask: u8) -> bool {
        let w = &self.world;
        let inside = |p: Pos| p != s && w.get(p).map_or(false, |t| t.burrows & mask != 0);
        let mut nb = Vec::new();
        w.neighbors(s, &mut nb, Mover::Dwarf);
        let mut want: HashSet<Pos> = nb.iter().map(|&(q, _)| q).filter(|&q| inside(q)).collect();
        let Some(&first) = want.iter().next() else { return true };
        let mut seen: HashSet<Pos> = HashSet::from([first]);
        let mut todo = vec![first];
        while let Some(p) = todo.pop() {
            want.remove(&p);
            if want.is_empty() {
                return true;
            }
            if seen.len() > 3000 {
                return false;
            }
            w.neighbors(p, &mut nb, Mover::Dwarf);
            for &(q, _) in nb.iter() {
                if inside(q) && seen.insert(q) {
                    todo.push(q);
                }
            }
        }
        false
    }

    /// Wall up every way into the shelter but the gate: a wall on the way
    /// in, under ground, just outside the shelter; else on the shelter's
    /// own last tile, if that leaves it in one piece; else that tile leaves
    /// the shelter, and the way is walled further in next time. How many
    /// walls were planned.
    fn wall_up_ways_in(&mut self, o: &mut Overseer, base: Pos) -> usize {
        let Some(top) = o.top else { return 0 };
        let Some(mask) = self.shelter_bits() else { return 0 };
        let (ways, from) = self.search_ways_in(base, top);
        let larder = self.larder_tiles();
        let mut n = 0;
        let mut open = 0;
        let mut given_up: Vec<Pos> = Vec::new();
        for (s, q) in ways {
            // The way back out, a few steps.
            let mut chain = vec![q];
            while chain.len() < 6 {
                let last = *chain.last().unwrap();
                match from.get(&last) {
                    Some(&p) if p != last => chain.push(p),
                    _ => break,
                }
            }
            let walled = |g: &Game, p: Pos| g.world.t(p).kind == TileKind::Wall || g.construction_at(p).map_or(false, |c| c.kind == ConstructKind::Wall);
            if walled(self, s) || chain.iter().any(|&p| walled(self, p)) {
                continue;
            }
            let gate = |p: Pos| Self::gate_tiles(base).any(|g| g == p);
            let spot = chain.iter().copied().find(|&p| !self.world.outside(p) && !gate(p) && self.can_construct(p, ConstructKind::Wall));
            let spot = spot.or_else(|| (!gate(s) && self.can_construct(s, ConstructKind::Wall) && self.shelter_whole_without(s, mask)).then_some(s));
            match spot {
                Some(p) if self.plan_constructions(p, p, ConstructKind::Wall, BuildMat::Any) > 0 => n += 1,
                _ => {
                    open += 1;
                    if !larder.contains(&s) && !gate(s) && given_up.len() < 4 && !given_up.contains(&s) {
                        given_up.push(s);
                    }
                }
            }
        }
        // Tiles that can't be walled leave the shelter, so the way in is
        // walled further in.
        if let Some(i) = self.burrows.iter().position(|b| Some(b.bit) == self.alert_burrow) {
            for &p in &given_up {
                self.world.t_mut(p).burrows &= !mask;
            }
            self.burrows[i].tiles = self.burrows[i].tiles.saturating_sub(given_up.len() as u32);
        }
        if n > 0 {
            self.shelter.walled += n as u32;
            self.count("shelter_ways_walled", n as u32);
            let text = format!("Ordered {} way{} into the shelter walled up, so the gate is the only way in.", n, if n == 1 { "" } else { "s" });
            if self.civ_alert {
                self.msg(&text, (255, 200, 90));
            }
            Self::note_ov(o, self.tick, text);
        }
        // (Counted on the daily round only.)
        if open > 0 && !self.civ_alert {
            self.count("shelter_ways_open", open as u32);
        }
        n
    }

    /// Take the gate's tiles out of the shelter, so those running in go on
    /// past the bridge. How many were taken out.
    fn gate_out_of_shelter(&mut self, base: Pos) -> u32 {
        let Some(i) = self.burrows.iter().position(|b| Some(b.bit) == self.alert_burrow) else { return 0 };
        let mask = self.burrows[i].mask();
        let mut n = 0;
        for p in Self::gate_tiles(base) {
            if self.world.get(p).map_or(false, |t| t.burrows & mask != 0) {
                self.world.t_mut(p).burrows &= !mask;
                n += 1;
            }
        }
        self.burrows[i].tiles = self.burrows[i].tiles.saturating_sub(n);
        n
    }

    /// Drink, and food a dwarf could eat, lying in the shelter.
    pub fn shelter_supplies(&self) -> (usize, usize) {
        let Some(mask) = self.shelter_bits() else { return (0, 0) };
        let inside = |p: Pos| self.world.get(p).map_or(false, |t| t.burrows & mask != 0);
        let mut v = (0, 0);
        for it in self.items.values().filter(|it| it.carried_by.is_none() && !it.forbidden) {
            if it.kind == ItemKind::Drink && inside(it.pos) {
                v.0 += 1;
            } else if it.is_food() && inside(it.pos) {
                v.1 += 1;
            }
        }
        v
    }

    /// How much drink and food the shelter should hold for the dwarves
    /// there are: a siege's worth (a dwarf drinks every four days or so and
    /// eats about weekly).
    fn shelter_wants(&self) -> (usize, usize) {
        let pop = self.alive_count() as f64;
        ((pop * self.knob("shelter.drink", 4.0)) as usize, (pop * self.knob("shelter.food", 2.0)) as usize)
    }

    /// The larder's tiles, if it still stands.
    pub fn larder_tiles(&self) -> Vec<Pos> {
        self.shelter.larder.and_then(|z| self.zones.get(&z)).map_or(Vec::new(), |z| z.tiles.clone())
    }

    /// Daily: lay out the larder in the shelter, and count what it wants.
    fn ov_larder_in_shelter(&mut self, o: &mut Overseer, base: Pos) {
        if self.shelter.larder.map_or(false, |z| !self.zones.contains_key(&z)) {
            self.shelter.larder = None;
        }
        if self.shelter.larder.is_none() {
            let Some(tiles) = self.larder_spot(base) else { return };
            // Carved from a stockpile room's general pile, if it has to be.
            for &p in &tiles {
                let z = self.world.t(p).zone;
                if z != 0 {
                    self.world.t_mut(p).zone = 0;
                    if let Some(zone) = self.zones.get_mut(&z) {
                        zone.tiles.retain(|q| *q != p);
                    }
                }
            }
            self.zones.retain(|_, z| !z.tiles.is_empty());
            if self.add_zone_tiles(tiles.clone(), ZoneKind::Stockpile(StockFilter::Food)) == 0 {
                return;
            }
            let id = self.world.t(tiles[0]).zone;
            if let Some(z) = self.zones.get_mut(&id) {
                z.deny = LARDER_DENIES.to_vec();
            }
            self.shelter.larder = Some(id);
            self.count("shelter_larders", 1);
            Self::note_ov(o, self.tick, "Set a larder inside the shelter, so the sheltering can always reach drink and a meal.".into());
        }
        let (drink, food) = self.shelter_supplies();
        let (wd, wf) = self.shelter_wants();
        self.shelter.want = (self.tick, wd.saturating_sub(drink) as u32, wf.saturating_sub(food) as u32);
    }

    /// Where the larder goes: a block of three by three floor inside the
    /// shelter, clear of the gatehouse corridor, as near its inner end as
    /// can be. Free floor first; else carved from a general stockpile.
    fn larder_spot(&self, base: Pos) -> Option<Vec<Pos>> {
        let mask = self.shelter_bits()?;
        let w = &self.world;
        let general = |z: u32| self.zones.get(&z).map_or(false, |z| z.kind == ZoneKind::Stockpile(StockFilter::All));
        // 0: free floor, 1: a general pile's, 2: no good.
        let grade = |p: Pos| -> u8 {
            let Some(t) = w.get(p) else { return 2 };
            if t.burrows & mask == 0 || t.kind != TileKind::Floor || t.building != 0 || t.desig != Desig::None || t.water > 0 || w.outside(p) || Self::gatehouse(base, p) {
                return 2;
            }
            match t.zone {
                0 => 0,
                z if general(z) => 1,
                _ => 2,
            }
        };
        let inner = base.offset(17, 0, 0);
        let mut best: Option<(i32, Pos)> = None;
        for (k, t) in w.tiles.iter().enumerate() {
            if t.burrows & mask == 0 {
                continue;
            }
            let a = w.pos_of(k);
            let d = (a.x - inner.x).abs() + (a.y - inner.y).abs() + (a.z - inner.z).abs() * 20;
            if best.map_or(false, |b| d >= b.0) {
                continue;
            }
            let grades: Vec<u8> = box_iter(a, a.offset(2, 2, 0)).map(grade).collect();
            if grades.iter().any(|&g| g == 2) {
                continue;
            }
            // Free floor beats carving a pile.
            let score = d + if grades.iter().any(|&g| g == 1) { 60 } else { 0 };
            if best.map_or(true, |b| score < b.0) {
                best = Some((score, a));
            }
        }
        best.map(|(_, a)| box_iter(a, a.offset(2, 2, 0)).collect())
    }

    /// A siege has ended (`overseer_hooks.rs`): if the shelter was called
    /// and nobody in it was lost, it held, and the fort is told so.
    pub(super) fn shelter_after_siege(&mut self, i: usize) {
        let Some(s) = self.annals.sieges.get(i) else { return };
        if !s.shelter || s.outcome == "the fortress fell" {
            return;
        }
        let lost = self.annals.deaths.iter().filter(|d| d.siege == Some(i) && d.sheltered).count();
        if lost == 0 {
            let text = format!("The shelter held: not one of those sheltering was lost to the {}.", s.foe);
            self.count("shelter_held", 1);
            self.msg(&text, (150, 220, 150));
        } else {
            self.count("shelter_lost", lost as u32);
        }
    }

    /// A hauler's job (`finders.rs`): drink or a meal from outside the
    /// shelter into its larder, while it holds less than it should. Not
    /// during an alert (nobody may leave the shelter then).
    pub(super) fn find_larder_haul(&mut self, d: &mut Dwarf) -> bool {
        if self.civ_alert || self.shelter.larder.is_none() {
            return false;
        }
        // Counted afresh now and then.
        if self.tick >= self.shelter.want.0 + 200 {
            let (drink, food) = self.shelter_supplies();
            let (wd, wf) = self.shelter_wants();
            self.shelter.want = (self.tick, wd.saturating_sub(drink) as u32, wf.saturating_sub(food) as u32);
        }
        let (_, wd, wf) = self.shelter.want;
        if wd == 0 && wf == 0 {
            return false;
        }
        let Some(mask) = self.shelter_bits() else { return false };
        let tiles: Vec<Pos> = self.larder_tiles().into_iter().filter(|p| self.world.walkable(*p)).collect();
        if tiles.is_empty() {
            return false;
        }
        // Where it can go: a barrel there with room, or a bare tile.
        let mut barrels: HashMap<Pos, u32> = HashMap::new();
        let mut taken: HashSet<Pos> = HashSet::new();
        for it in self.items.values().filter(|it| it.carried_by.is_none()) {
            if tiles.contains(&it.pos) {
                taken.insert(it.pos);
                if it.kind == ItemKind::Barrel && it.container.is_none() && self.contents(it.id) < CONTAINER_CAP {
                    barrels.insert(it.pos, it.id);
                }
            }
        }
        let bare: HashSet<Pos> = tiles.iter().copied().filter(|p| !taken.contains(p) && !self.reserved.contains(p)).collect();
        if barrels.is_empty() && bare.is_empty() {
            return false;
        }
        let w = &self.world;
        let outside_shelter = |p: Pos| w.get(p).map_or(false, |t| t.burrows & mask == 0);
        let wanted = |it: &Item| outside_shelter(it.pos) && !it.dump && !it.to_depot && ((wd > 0 && it.kind == ItemKind::Drink) || (wf > 0 && it.kind == ItemKind::Meal));
        let Some((item, path)) = self.find_item(d.pos, wanted) else {
            // Nothing to fetch: ask again tomorrow.
            self.shelter.want = (self.tick + DAY, 0, 0);
            return false;
        };
        let ipos = self.items[&item].pos;
        let Some((dest, _)) = path::search(&self.world, ipos, SEARCH_LIMIT, Mover::Dwarf, |p| barrels.contains_key(&p) || bare.contains(&p)) else {
            return false;
        };
        let into = barrels.get(&dest).copied();
        if into.is_none() {
            self.reserved.insert(dest);
        }
        if self.items[&item].kind == ItemKind::Drink {
            self.shelter.want.1 = wd.saturating_sub(1);
        } else {
            self.shelter.want.2 = wf.saturating_sub(1);
        }
        self.claim_item(item, d.id);
        d.task = Task::Haul { item, dest, into };
        Self::set_path(d, path);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::overseer::Style;

    /// A Steady fort with its shelter marked (and the overseer taken out).
    pub(crate) fn sheltered_fort() -> (Game, Overseer) {
        for seed in 5000..5010 {
            let mut g = crate::game::trial_game(seed, Style::Steady, 30);
            let o = g.overseer.take().unwrap();
            if o.did("burrow") && o.did("gate") && g.alive_count() >= 5 {
                return (g, o);
            }
        }
        panic!("no Steady fort from 5000 on has its shelter by day 30");
    }

    /// The gate is no part of the shelter, and someone called in from the
    /// stair goes on past the bridge rather than stopping on the doors.
    #[test]
    fn the_gate_is_outside_the_shelter() {
        let (mut g, mut o) = sheltered_fort();
        let base = o.base.unwrap();
        // (As an old save had it: the doors in the shelter.)
        let i = g.burrows.iter().position(|b| Some(b.bit) == g.alert_burrow).unwrap();
        g.burrow_add(i, &[base.offset(1, 0, 0), base.offset(1, 1, 0), base.offset(2, 0, 0)]);
        g.ov_shelter(&mut o);
        for p in Game::gate_tiles(base) {
            assert!(!g.in_shelter(p), "{:?} (base {:?}) is in the shelter", p, base);
        }
        assert!(g.in_shelter(base.offset(4, 0, 0)), "the corridor behind the bridge left the shelter");
        // A civilian on the stair when the alert goes up.
        let id = g.dwarves.iter().find(|d| d.alive && !d.soldier).unwrap().id;
        g.creatures.retain(|_, c| !c.hostile());
        assert_eq!(g.toggle_civ_alert(), Ok(true));
        let di = g.dwarves.iter().position(|d| d.id == id).unwrap();
        g.dwarves[di].pos = base;
        g.dwarves[di].path.clear();
        g.dwarves[di].task = crate::dwarf::Task::Idle;
        for _ in 0..400 {
            g.step();
        }
        let d = &g.dwarves[di];
        assert!(g.in_shelter(d.pos), "{} never got into the shelter: {:?}", d.name, d.pos);
        assert!(!Game::gate_tiles(base).any(|p| p == d.pos), "{} stopped on the gate at {:?}", d.name, d.pos);
    }

    /// A long siege doesn't end in thirst while there's drink: with every
    /// drink outside the shelter, the overseer sets a larder in it and the
    /// haulers fill it, and the sheltering drink from it all through a
    /// siege of ten days.
    #[test]
    fn a_long_siege_does_not_end_in_thirst() {
        let (mut g, mut o) = sheltered_fort();
        g.creatures.retain(|_, c| !c.hostile());
        let pop = g.alive_count();
        // Every drink out by the wagon, under the sky, and plenty of it.
        let e = g.embark;
        let spot = (0..40).map(|i| e.offset(i % 5, -2 - i / 5, 0)).map(|p| Pos::new(p.x, p.y, g.world.surface_z(p.x, p.y))).find(|p| g.world.walkable(*p) && g.world.outside(*p)).expect("ground by the wagon");
        let proto = g.items.values().find(|it| it.kind == ItemKind::Drink).cloned().expect("some drink");
        let drinks: Vec<u32> = g.items.values().filter(|it| it.kind == ItemKind::Drink).map(|it| it.id).collect();
        for id in &drinks {
            g.items.remove(id);
        }
        for _ in 0..pop * 6 {
            let id = g.new_id();
            let mut it = proto.clone();
            it.id = id;
            it.pos = spot;
            it.container = None;
            it.claimed_by = None;
            it.carried_by = None;
            g.items.insert(id, it);
        }
        assert_eq!(g.shelter_supplies().0, 0);
        // The overseer's day: a larder, and haulers to fill it.
        g.ov_shelter(&mut o);
        assert!(!g.larder_tiles().is_empty(), "no larder in the shelter");
        assert!(g.larder_tiles().iter().all(|p| g.in_shelter(*p)));
        for _ in 0..DAY * 3 {
            g.step();
        }
        let (drink, _) = g.shelter_supplies();
        assert!(drink >= pop * 3, "only {} drinks in the shelter for {} dwarves", drink, pop);
        // Ten days shut in.
        assert_eq!(g.toggle_civ_alert(), Ok(true));
        let dead = g.annals.deaths.len();
        let mut worst = 0;
        for _ in 0..DAY * 10 {
            g.step();
            if g.tick % 100 == 0 {
                worst = worst.max(g.dwarves.iter().filter(|d| d.alive && !d.soldier && !d.insane).map(|d| d.thirst).max().unwrap_or(0));
            }
        }
        assert!(g.civ_alert);
        let thirsty: Vec<&String> = g.annals.deaths[dead..].iter().filter(|d| d.cause == "thirst" || d.cause == "starvation").map(|d| &d.name).collect();
        assert!(thirsty.is_empty(), "died of thirst or hunger in the shelter: {:?}", thirsty);
        assert!(worst <= THIRSTY + 3000, "a sheltering dwarf went thirsty: {}", worst);
        assert!(g.shelter_supplies().0 < drink, "nobody drank from the shelter's stores");
    }

    /// A hall broken through to the sky is a way in past the gate: when the
    /// alert goes up it's walled, and the gate is the only way in again.
    #[test]
    fn a_way_in_past_the_gate_is_walled_in_an_alert() {
        let (mut g, mut o) = sheltered_fort();
        g.creatures.retain(|_, c| !c.hostile());
        let (base, top) = (o.base.unwrap(), o.top.unwrap());
        let before = g.ways_in(base, top).len();
        // A tunnel out of the shelter, six tiles through the rock, and a
        // stair at its end up to the ground.
        let mask = g.shelter_bits().unwrap();
        let (s, dir) = (0..g.world.tiles.len())
            .map(|k| g.world.pos_of(k))
            .filter(|&p| p.z == base.z && g.world.t(p).burrows & mask != 0 && g.world.t(p).kind == TileKind::Floor && g.world.t(p).building == 0)
            .filter(|&p| !Game::gatehouse(base, p))
            .find_map(|p| {
                DIRS4.iter().copied().find(|&(dx, dy)| (1..=7).all(|i| {
                    let q = p.offset(dx * i, dy * i, 0);
                    g.world.in_bounds(q) && g.world.t(q).kind == TileKind::Wall && g.world.t(q).burrows == 0 && !g.world.outside(q) && !Game::gatehouse(base, q)
                }) && (1..=7).all(|i| {
                    // (Rock on either side, so it stays one tunnel.)
                    let q = p.offset(dx * i, dy * i, 0);
                    g.world.solid(q.offset(dy, dx, 0)) && g.world.solid(q.offset(-dy, -dx, 0))
                })).map(|d| (p, d))
            })
            .expect("a shelter wall with rock behind it");
        for i in 1..=6 {
            let q = s.offset(dir.0 * i, dir.1 * i, 0);
            let t = g.world.t_mut(q);
            t.kind = TileKind::Floor;
            t.dug = true;
            t.revealed = true;
        }
        let foot = s.offset(dir.0 * 6, dir.1 * 6, 0);
        let sz = g.world.surface_z(foot.x, foot.y);
        assert!(sz > foot.z, "the tunnel's end is under the sky already");
        for z in foot.z..=sz {
            let t = g.world.t_mut(Pos::new(foot.x, foot.y, z));
            t.kind = if z == foot.z { TileKind::StairUp } else if z == sz { TileKind::StairDown } else { TileKind::StairUD };
            t.dug = true;
            t.revealed = true;
        }
        g.dig_version += 1;
        let ways = g.ways_in(base, top);
        assert!(ways.contains(&(s, s.offset(dir.0, dir.1, 0))), "the tunnel at {:?} isn't seen as a way in: {:?}", s, ways);
        // The alert: it's walled straight away.
        // (The overseer's alert check, as it runs every ten ticks.)
        assert_eq!(g.toggle_civ_alert(), Ok(true));
        g.ov_shelter_alert(&mut o);
        let door = s.offset(dir.0, dir.1, 0);
        assert!(g.construction_at(door).map_or(false, |c| c.kind == ConstructKind::Wall), "no wall planned at {:?}", door);
        for _ in 0..DAY * 2 {
            g.step();
        }
        assert_eq!(g.world.t(door).kind, TileKind::Wall, "the way in at {:?} was never walled", door);
        assert!(g.ways_in(base, top).len() <= before, "ways in left: {:?}", g.ways_in(base, top));
        assert!(g.log.iter().any(|l| l.text.contains("walled up")), "the player was never told");
    }

    /// A siege sheltered through without loss is told; one with a death in
    /// the shelter isn't.
    #[test]
    fn a_shelter_that_held_is_told() {
        let mut g = Game::new(5);
        let note = |g: &mut Game| {
            g.annals.sieges.push(crate::game::annals::SiegeNote { foe: "goblins".into(), shelter: true, outcome: "routed".into(), ended: Some(g.tick), ..Default::default() });
            g.annals.sieges.len() - 1
        };
        let i = note(&mut g);
        g.shelter_after_siege(i);
        assert!(g.log.iter().any(|l| l.text.starts_with("The shelter held")));
        assert_eq!(g.annals.counters.get("shelter_held").copied(), Some(1));
        let j = note(&mut g);
        g.annals.deaths.push(crate::game::annals::DeathNote { siege: Some(j), sheltered: true, ..Default::default() });
        g.shelter_after_siege(j);
        assert_eq!(g.annals.counters.get("shelter_held").copied(), Some(1));
        assert_eq!(g.annals.counters.get("shelter_lost").copied(), Some(1));
    }

    /// The shelter sings: a while into an alert, those in the shelter sing
    /// together and take heart, once an alert; a lament once someone fell.
    #[test]
    fn the_shelter_sings() {
        let (mut g, mut o) = sheltered_fort();
        let mask = g.shelter_bits().unwrap();
        let spot = g.world.tiles.iter().enumerate().find(|(_, t)| t.burrows & mask != 0 && t.kind == TileKind::Floor).map(|(i, _)| g.world.pos_of(i)).unwrap();
        for d in g.dwarves.iter_mut().filter(|d| d.alive && !d.soldier) {
            d.pos = spot;
        }
        g.civ_alert = true;
        g.ov_shelter_alert(&mut o);
        assert_eq!(g.annals.counters.get("shelter.songs"), None, "sang the moment the alert went up");
        g.tick += 500;
        g.ov_shelter_alert(&mut o);
        g.ov_shelter_alert(&mut o);
        assert_eq!(g.annals.counters.get("shelter.songs"), Some(&1));
        assert!(g.log.iter().any(|l| l.text.contains("whole shelter is singing")));
        let sang = g.dwarves.iter().filter(|d| d.thoughts.iter().any(|t| t.text.contains("sang with the others") && t.stress < 0)).count();
        assert!(sang >= 3, "only {} took heart from the song", sang);
        // A new alert, a siege that has cost a life: a lament.
        g.civ_alert = false;
        g.ov_shelter_alert(&mut o);
        g.civ_alert = true;
        g.ov_shelter_alert(&mut o);
        g.annals.sieges.push(crate::game::annals::SiegeNote { foe: "goblins".into(), ..Default::default() });
        g.annals.deaths.push(crate::game::annals::DeathNote { siege: Some(g.annals.sieges.len() - 1), ..Default::default() });
        g.tick += 500;
        g.ov_shelter_alert(&mut o);
        assert_eq!(g.annals.counters.get("shelter.songs"), Some(&2));
        assert!(g.log.iter().any(|l| l.text.contains("lament for the fallen")));
    }

    /// Where the drink and food lie in a grown fort: SEED, STYLE, DAYS.
    #[test]
    #[ignore]
    fn supplies_census() {
        let env = |k: &str, d: &str| std::env::var(k).unwrap_or(d.into());
        let seed: u64 = env("SEED", "5000").parse().unwrap();
        let style = Style::parse(&env("STYLE", "steady")).unwrap();
        let days: u64 = env("DAYS", "500").parse().unwrap();
        let g = crate::game::trial_game(seed, style, days);
        for (what, f) in [("drink", (|it: &Item| it.kind == ItemKind::Drink) as fn(&Item) -> bool), ("food", |it: &Item| it.is_food())] {
            let mut by: BTreeMap<String, usize> = BTreeMap::new();
            for it in g.items.values().filter(|it| f(it)) {
                let p = it.pos;
                let zone = g.zone_kind_at(p).map_or("no zone".to_string(), |k| format!("{:?}", k));
                let key = format!("{} {} {}", if g.in_shelter(p) { "SHELTER" } else { "outside-shelter" }, if g.world.outside(p) { "sky" } else { "under" }, zone);
                *by.entry(key).or_default() += 1;
            }
            eprintln!("{} (day {}): {:?}", what, g.tick / DAY, by);
        }
    }

    /// How besiegers get into the shelter: run batch forts (SEEDS, comma
    /// separated, styles as the batch gives them) for YEARS and print where
    /// each besieger first stepped into the alert burrow, and every death
    /// inside it.
    #[test]
    #[ignore]
    fn shelter_breach_trace() {
        let env = |k: &str, d: &str| std::env::var(k).unwrap_or(d.into());
        let seeds: Vec<u64> = env("SEEDS", "5000").split(',').filter_map(|s| s.trim().parse().ok()).collect();
        let years: u64 = env("YEARS", "2").parse().unwrap();
        for seed in seeds {
            let style = Style::ALL[((seed - 5000) % 5) as usize];
            let mut g = Game::new(seed);
            g.set_overseer(Some(style));
            let mut last: HashMap<u32, Pos> = HashMap::new();
            let mut deaths = 0;
            let mut dead: HashSet<u32> = HashSet::new();
            eprintln!("== seed {} {:?}", seed, style);
            while g.tick < years * YEAR && g.alive_count() > 0 {
                g.step();
                let Some(o) = g.overseer.as_ref() else { break };
                let (Some(top), Some(base)) = (o.top, o.base) else { continue };
                let siege = g.annals.sieges.iter().rposition(|s| s.ended.is_none());
                let ids: Vec<(u32, Pos, String, bool)> = g.creatures.values().filter(|c| c.alive && c.hostile()).map(|c| (c.id, c.pos, crate::game::annals::kind_of(c), c.siege)).collect();
                for (id, p, kind, sieger) in ids {
                    let was = last.insert(id, p);
                    if let Some(q) = was {
                        if g.in_shelter(p) && !g.in_shelter(q) {
                            let raised = g.buildings.values().any(|b| b.kind == BuildingKind::Bridge && b.raised);
                            // The way it came: the nearest open sky from where it stood.
                            let sky = crate::path::search(&g.world, q, 4000, crate::world::Mover::Goblin, |r| g.world.outside(r)).map(|(r, path)| (r, path.len()));
                            eprintln!(
                                "  day {:>4} {} {}{} civ {} in at {:?} from {:?} (rel base {},{},{}) {:?} bridge-up {} sky {:?}",
                                g.tick / DAY, kind, if sieger { "besieger" } else { "beast" }, siege.map_or(String::new(), |s| format!(" siege {}", s)), g.civ_alert as u8,
                                p, q, p.x - base.x, p.y - base.y, p.z - base.z, g.world.t(q).kind, raised, sky
                            );
                        }
                    }
                }
                let newly: Vec<&Dwarf> = g.dwarves.iter().filter(|d| !d.alive && !dead.contains(&d.id)).collect();
                for d in newly {
                    dead.insert(d.id);
                    let killer = d.attacker.and_then(|(cid, _)| g.creatures.get(&cid)).map(|c| (crate::game::annals::kind_of(c), c.pos, g.in_shelter(c.pos)));
                    eprintln!(
                        "  day {:>4} {} died at {:?} (rel base {},{},{}) sheltered-tile {} civ {} soldier {} killer {:?}",
                        g.tick / DAY, d.name, d.pos, d.pos.x - base.x, d.pos.y - base.y, d.pos.z - base.z, g.in_shelter(d.pos), g.civ_alert as u8, d.soldier, killer
                    );
                }
                let _ = top;
                while deaths < g.annals.deaths.len() {
                    let d = &g.annals.deaths[deaths];
                    if d.sheltered {
                        eprintln!("      (annals: {} {} by {:?} siege {:?})", d.name, d.cause, d.killer, d.siege);
                    }
                    deaths += 1;
                }
            }
            eprintln!("  end day {} alive {} sieges {}", g.tick / DAY, g.alive_count(), g.annals.sieges.len());
        }
    }
}
