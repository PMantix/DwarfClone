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
}

/// What the shelter's larder takes: drink and meals, nothing else.
const LARDER_DENIES: [ItemKind; 5] = [ItemKind::Plant, ItemKind::Seed, ItemKind::Meat, ItemKind::Fish, ItemKind::Flour];

impl Game {
    /// The shelter's burrow mask, if there is a shelter.
    pub(super) fn shelter_bits(&self) -> Option<u8> {
        self.burrows.iter().find(|b| Some(b.bit) == self.alert_burrow).map(|b| b.mask())
    }

    /// Whether `p` lies in the shelter.
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
        self.ov_larder_in_shelter(o, base);
    }

    /// Every ten ticks (`overseer_hooks.rs`): when the alert is raised, the
    /// shelter is made ready at once.
    pub(super) fn ov_shelter_alert(&mut self, o: &mut Overseer) {
        if self.civ_alert == self.shelter.was_alert {
            return;
        }
        self.shelter.was_alert = self.civ_alert;
        if let (true, Some(base)) = (self.civ_alert, o.base) {
            self.gate_out_of_shelter(base);
        }
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
