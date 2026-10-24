//! The gatehouse (M2 W0-B: moved verbatim out of overseer.rs, so O04 and
//! the M1 follow-ups edit this file): the gate at the foot of the stair,
//! its drawbridge on a lever, the cage traps behind it, and barring the
//! gate and raising the bridge when an army comes.
//!
//! The gatehouse grows with the fort (O04, `Overseer.gate_stage`): stage 0
//! is the gate itself; stage 1 the trap hall, the corridor behind the cage
//! traps filled with tripwires, upright spikes on pressure plates and
//! weapon traps, named like a beast's mouth ("the Hall of Teeth"). A bold
//! or warlike overseer with a hall lets the enemy into it before it raises
//! the bridge behind them, and the toll it takes is told when the siege ends.

use super::personality::{NoteKind, Trait};
use super::overseer::{Overseer, Style};
use super::*;

/// The trap hall's pieces, by local column (x from the foot of the stair)
/// on both rows of the corridor: a foe running in from the gate trips at
/// x+8 and goes headlong onto the spikes at x+9, which the wire springs;
/// they spring again whenever one of those behind treads on the plates at
/// x+10; then the weapon traps at x+11. (Cage traps stand at x+4..7, only
/// on the far row once the overseer means to let them in; stone-fall traps
/// from Fortify at x+12..14; the inner lever just past them.)
const HALL: [(i32, BuildingKind); 4] = [(8, BuildingKind::Tripwire), (9, BuildingKind::Spikes), (10, BuildingKind::PressurePlate), (11, BuildingKind::WeaponTrap)];

/// Where the inner lever may go, best first: past the stone-fall traps,
/// beside the squads holding the far end of the hall (or else among them).
const INNER_LEVER: [(i32, i32); 9] = [(15, 1), (15, 0), (16, 1), (16, 0), (17, 1), (17, 0), (14, 1), (13, 1), (14, 0)];

/// Names for the hall, as for a beast's mouth.
const HALL_NAMES: [&str; 10] = [
    "the Hall of Teeth",
    "the Gullet",
    "the Long Bite",
    "the Hall of Knives",
    "the Red Throat",
    "the Grinding Way",
    "the Hall of Thorns",
    "the Iron Mouth",
    "the Maw",
    "the Narrow Death",
];

/// A count in words, for the toll ("Seven goblins fell").
fn count_words(n: u32) -> String {
    const W: [&str; 13] = ["No", "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine", "Ten", "Eleven", "Twelve"];
    W.get(n as usize).map_or(n.to_string(), |w| w.to_string())
}

impl Game {
    /// Bar the gate when enemies are close, and raise the drawbridge
    /// against an army (from `ov_alert`, every ten ticks).
    pub(super) fn ov_gate_alert(&mut self, o: &mut Overseer, top: Pos, base: Pos, coming: bool) {
        // Bar the gate when they're close (not before, so those outside can
        // still get in).
        let near = |c: &crate::creature::Creature| {
            c.alive && c.hostile() && !c.sneaking && {
                let p = c.pos;
                let by = |q: Pos, r: i32| (p.x - q.x).abs() <= r && (p.y - q.y).abs() <= r && (p.z - q.z).abs() <= 1;
                by(top, 10) || by(base, 12)
            }
        };
        let close = self.creatures.values().any(|c| (c.siege || c.legendary()) && near(c));
        // (A beast of legend is never let in.)
        let beast = self.creatures.values().any(|c| c.legendary() && near(c));
        // An overseer that lets them into the trap hall leaves the doors
        // open to them until the bridge goes up behind them.
        // (Not an army too big to take on in the hall: then the bridge
        // goes up as for any other overseer.)
        let army_size = self.creatures.values().filter(|c| c.alive && c.siege && !c.legendary() && c.hostile()).count();
        let let_in = self.lets_them_in(o) && army_size as f64 <= self.knob("gate.let_in_max", 12.0);
        let up = self.gate_bridges(base).iter().any(|id| self.buildings.get(id).map_or(false, |b| b.raised));
        let close = close && (beast || !(let_in && !up));
        let gate: Vec<u32> = self
            .buildings
            .values()
            .filter(|b| {
                b.kind == BuildingKind::Door
                    && b.built
                    && b.locked != (close && !self.sallying())
                    && (((b.pos.x - base.x).abs() <= 1 && (b.pos.y - base.y).abs() <= 1 && b.pos.z == base.z) || b.pos == top.offset(-3, 0, 0))
            })
            .map(|b| b.id)
            .collect();
        for id in gate {
            self.toggle_lock(id);
        }
        // Raise the drawbridge against an army, once everyone's inside or
        // when they're close; not for days on end without one at the gate.
        if o.did("linked") {
            let inside = self.dwarves.iter().filter(|d| d.alive && !d.soldier).all(|d| d.pos.z <= base.z && !self.world.outside(d.pos));
            let army = |near: bool| {
                self.creatures.values().any(|c| {
                    c.alive && c.siege && !c.legendary() && c.hostile() && (!near || ((c.pos.x - top.x).abs() <= 40 && (c.pos.y - top.y).abs() <= 40))
                })
            };
            let (any, at_gate) = (army(false) || coming, army(true));
            // How long the army has been on the map: a little while for those
            // outside to run in, then the bridge goes up regardless.
            if any && o.army_since == 0 {
                o.army_since = self.tick;
            } else if !any {
                o.army_since = 0;
            }
            // (Letting them into the trap hall, it waits for half the army to
            // be in it, and then shuts them in with its traps and the
            // squads; or, if they won't come, until it has waited long.)
            let hold = if let_in { self.knob("gate.let_in_wait", 900.0) as u64 } else { 0 };
            let waited = any && self.tick > o.army_since + self.bar_patience(o) + hold;
            let shut_in = let_in && {
                let hall = |p: Pos| p.z == base.z && (4..=14).contains(&(p.x - base.x)) && (0..=1).contains(&(p.y - base.y));
                let (mut n, mut all) = (0, 0);
                for c in self.creatures.values().filter(|c| c.alive && c.siege && !c.legendary() && c.hostile()) {
                    all += 1;
                    if hall(c.pos) {
                        n += 1;
                    }
                }
                n > 0 && n * 2 >= all
            };
            let long = self.tick > o.raised_at + DAY * 4;
            // (Not while the squads are out on a sally: sally.rs. Called
            // back, they're left to hunt down any who got inside.)
            let want = any && (((at_gate || inside) && !let_in) || waited || shut_in) && !(long && !at_gate && o.raised_at > 0) && !self.sallying();
            // Or because you said so, from a card (commands.rs).
            let want = want || (o.ordered(super::commands::Command::RaiseBridge, self.tick) && !self.sallying());
            // Told to stand down, it lowers the bridge.
            let want = want && !o.stood_down(self.tick, self.last_siege_tick());
            let bridges = self.gate_bridges(base);
            let raised = bridges.iter().any(|id| self.buildings.get(id).map_or(false, |b| b.raised));
            // (Letting them in, the lever behind the hall shuts them in.)
            let lever = if let_in { self.inner_lever(base).or_else(|| self.gate_lever(base)) } else { self.gate_lever(base) };
            if let Some(lever) = lever {
                // Anyone standing by the lever (the squad holds station there) throws it at once.
                let lp = self.buildings[&lever].pos;
                let by = self.dwarves.iter().any(|d| d.alive && d.pos.z == lp.z && (d.pos.x - lp.x).abs() <= 1 && (d.pos.y - lp.y).abs() <= 1);
                if raised != want && by {
                    self.pull_lever(lever);
                } else if raised != want && !self.buildings[&lever].pull {
                    self.request_pull(lever);
                }
                // A pull no longer wanted (or asked of the other lever) is
                // called off, or it would throw the bridge back.
                for l in [self.gate_lever(base), self.inner_lever(base)].into_iter().flatten() {
                    if l != lever || raised == want {
                        if let Some(b) = self.buildings.get_mut(&l) {
                            b.pull = false;
                        }
                    }
                }
                if want && !raised {
                    o.raised_at = self.tick;
                } else if !want && !raised {
                    o.raised_at = 0;
                }
            }
        }
    }

    pub(super) fn gate_lever(&self, base: Pos) -> Option<u32> {
        self.buildings.values().find(|b| b.kind == BuildingKind::Lever && b.built && b.pos == base.offset(3, 1, 0)).map(|b| b.id)
    }

    /// The second lever on the gate's bridge, behind the trap hall where the
    /// squads hold (O04): once it works both halves.
    pub(super) fn inner_lever(&self, base: Pos) -> Option<u32> {
        let bridges = self.gate_bridges(base);
        if bridges.is_empty() {
            return None;
        }
        INNER_LEVER
            .iter()
            .filter_map(|&(dx, dy)| match self.world.get(base.offset(dx, dy, 0)).map_or(0, |t| t.building) {
                0 => None,
                id => self.buildings.get(&id),
            })
            .find(|b| b.kind == BuildingKind::Lever && b.built && bridges.iter().all(|id| b.links.contains(id)))
            .map(|b| b.id)
    }

    pub(super) fn gate_bridges(&self, base: Pos) -> Vec<u32> {
        self.buildings.values().filter(|b| b.kind == BuildingKind::Bridge && b.pos.z == base.z && b.pos.x == base.x + 2 && (b.pos.y - base.y).abs() <= 1).map(|b| b.id).collect()
    }

    /// The gate at the foot of the stair, and a corridor of cage traps
    /// behind it.
    pub(super) fn ov_defences(&mut self, o: &mut Overseer) {
        if o.style == Style::Reckless {
            return;
        }
        self.ov_quarry(o);
        let base = o.base.unwrap();
        // The gate: a door on each row of the two-wide corridor. It isn't a
        // gate until both are there, and a door broken down is hung again.
        if self.dug_share(base.offset(1, 0, 0), base.offset(6, 1, 0)) >= 1.0 {
            let mut hung = 0;
            for dy in 0..2 {
                let p = base.offset(1, dy, 0);
                if self.building_at(p) == Some(BuildingKind::Door) || self.place_building(BuildingKind::Door, p) {
                    hung += 1;
                }
            }
            if hung == 2 && !o.did("gate") {
                o.done.push("gate".into());
                Self::note_as(o, self.tick, NoteKind::Gate, "Hung a gate at the foot of the stair, to bar when enemies come.".into());
            }
        }
        // Where the miners broke into a cavern, a door: beasts can't open one.
        // (Never on the gate's own tiles: a door where the bridge or lever
        // goes keeps the drawbridge from ever being built.)
        let gate_part = |p: Pos| p.z == base.z && (1..=3).contains(&(p.x - base.x)) && (0..=1).contains(&(p.y - base.y));
        for q in std::mem::take(&mut self.breaches) {
            let spots = std::iter::once(q).chain(crate::geom::DIRS4.iter().map(|&(dx, dy)| q.offset(dx, dy, 0)));
            let fit: Vec<Pos> = spots.filter(|&p| !gate_part(p) && self.world.get(p).map_or(false, |t| t.dug && t.kind == TileKind::Floor && t.building == 0)).collect();
            let hung = fit.into_iter().any(|p| self.place_building(BuildingKind::Door, p));
            if hung {
                Self::note_as(o, self.tick, NoteKind::Gate, "Hung a door where the miners broke into the caverns.".into());
            }
        }
        // A drawbridge behind the doors, on a lever: raised, nothing gets in.
        // Both halves and the lever, each put back if it's lost.
        let parts = [(BuildingKind::Bridge, base.offset(2, 0, 0)), (BuildingKind::Bridge, base.offset(2, 1, 0)), (BuildingKind::Lever, base.offset(3, 1, 0))];
        let missing = parts.iter().filter(|&&(k, p)| self.building_at(p) != Some(k)).count();
        if o.did("gate") && missing > 0 && self.stock_of(ItemKind::Mechanism) >= missing {
            let placed = parts.iter().filter(|&&(k, p)| self.building_at(p) == Some(k) || self.place_building(k, p)).count();
            if placed == parts.len() && !o.did("bridge") {
                o.done.push("bridge".into());
                Self::note_as(o, self.tick, NoteKind::Gate, "Began a drawbridge at the gate, worked by a lever inside.".into());
            }
        }
        // The lever works every half of the bridge (again, after a rebuild).
        if o.did("bridge") {
            let lever = self.gate_lever(base);
            let bridges = self.gate_bridges(base);
            let built = bridges.len() == 2 && bridges.iter().all(|id| self.buildings.get(id).map_or(false, |b| b.built));
            if let (Some(lever), true) = (lever, built) {
                let unlinked: Vec<u32> = bridges.iter().copied().filter(|id| !self.buildings[&lever].links.contains(id)).collect();
                for id in &unlinked {
                    let _ = self.link_lever(lever, *id);
                }
                // (Only once it works both halves: a link wants a spare
                // mechanism, and an overseer that thought its bridge linked
                // without one stopped asking for them, and never raised it.)
                if !unlinked.is_empty() && !o.did("linked") && self.gate_links_missing(base) == 0 {
                    o.done.push("linked".into());
                    Self::note_as(o, self.tick, NoteKind::Gate, "Linked the lever to the drawbridge. The gate can be shut fast.".into());
                }
            }
        }
        // Traps once the bridge is begun: it needs the mechanisms more.
        if o.did("bridge") && !self.gate_first(o) && !o.did("traps") && self.stock_of(ItemKind::Mechanism) >= 2 && self.stock_of(ItemKind::Cage) >= 2 {
            let mut n = 0;
            for dx in 4..8 {
                for dy in 0..2 {
                    if self.place_building(BuildingKind::CageTrap, base.offset(dx, dy, 0)) {
                        n += 1;
                    }
                }
            }
            if n > 0 {
                o.done.push("traps".into());
                Self::note_as(o, self.tick, NoteKind::Gate, format!("Set {} cage traps in the entrance corridor.", n));
            }
        }
        // The gatehouse grows: the trap hall behind the cages, then the
        // pit bridge past it (pits.rs).
        self.ov_trap_hall(o);
        self.ov_pit(o);
        // ...and towers at the gate (defences.rs).
        self.ov_gate_towers(o);
    }

    /// The name the trap hall takes, as for a beast's mouth (from hashes).
    pub(super) fn hall_name_for(&self, base: Pos) -> &'static str {
        HALL_NAMES[(super::names::hash(&[super::stories::hash(&self.fortress_name, 0), base.x as u64, base.y as u64, base.z as u64]) % HALL_NAMES.len() as u64) as usize]
    }

    /// Whether the trap hall is due (O04 step 2): a warlike or cautious
    /// overseer builds it as soon as the gate is linked; the rest once a
    /// siege has come, or half a year in. Never a reckless one.
    pub(super) fn trap_hall_due(&self, o: &Overseer) -> bool {
        if o.style == Style::Reckless || !o.did("linked") {
            return false;
        }
        let eager = o.style == Style::Warlord || o.has(Trait::Cautious);
        eager || !self.annals.sieges.is_empty() || self.tick >= self.knob("gate.hall_day", (YEAR / 2) as f64) as u64
    }

    /// Stage 1 of the gatehouse, the trap hall (ENG-04, -06, -08): laid out
    /// once it is due and the mechanisms are in hand, then kept: lost parts
    /// put back, the plates linked to the spikes as they're finished, and
    /// once a siege is over the bodies jamming its traps carried off (ENG-01).
    pub(super) fn ov_trap_hall(&mut self, o: &mut Overseer) {
        let Some(base) = o.base else { return };
        if self.gate_first(o) {
            return;
        }
        if o.gate_stage == 0 {
            if !self.trap_hall_due(o) {
                return;
            }
            // Mechanisms for the spikes, plates and weapon traps: ask for
            // them, and begin once they're made.
            let need = self.knob("gate.hall_mechs", 6.0) as usize;
            if self.stock_of(ItemKind::Mechanism) < need {
                let until = self.tick + SEASON;
                o.boosts.retain(|b| b.0 != crate::buildings::Recipe::Mechanism);
                o.boosts.push((crate::buildings::Recipe::Mechanism, need as u32 + 2, until));
                return;
            }
            let n = self.lay_trap_hall(base);
            if n == 0 {
                return;
            }
            o.gate_stage = 1;
            o.done.push("traphall".into());
            let name = self.hall_name_for(base);
            // (Named through `deed_at`, so the legends and the chronicle tell it: defences.rs.)
            self.name_defence(o, super::defences::Defence::Hall);
            Self::note_as(
                o,
                self.tick,
                NoteKind::Gate,
                format!("Began a trap hall behind the cages: tripwires, upright spikes on pressure plates, weapon traps. They will call it {}.", name),
            );
            return;
        }
        if self.tick / DAY % 3 == 0 && self.stock_of(ItemKind::Mechanism) >= 1 {
            self.lay_trap_hall(base);
        }
        // The plates and the wires fire every spike in the hall.
        let mut plates: Vec<u32> = self.hall_parts(base, BuildingKind::PressurePlate);
        plates.extend(self.hall_parts(base, BuildingKind::Tripwire));
        let spikes: Vec<u32> = self.hall_parts(base, BuildingKind::Spikes);
        for &p in &plates {
            for &sp in &spikes {
                if !self.buildings[&p].links.contains(&sp) {
                    let _ = self.link_lever(p, sp);
                }
            }
        }
        if self.lets_them_in(o) {
            self.open_the_way(o, base);
        }
        // The siege is over: carry the dead off the traps.
        if !self.alert && !self.creatures.values().any(|c| c.alive && c.siege) {
            let hall = |p: Pos| p.z == base.z && (4..=14).contains(&(p.x - base.x)) && (0..=1).contains(&(p.y - base.y));
            let bodies: Vec<u32> = self
                .items
                .values()
                .filter(|it| matches!(it.kind, ItemKind::Corpse | ItemKind::Carcass) && !it.dump && it.carried_by.is_none() && hall(it.pos) && !self.burial_corpse(it))
                .map(|it| it.id)
                .collect();
            if bodies.len() >= 2 {
                for &id in &bodies {
                    let _ = self.toggle_dump(id);
                }
                let name = self.trap_hall_name().unwrap_or_else(|| "the trap hall".into());
                Self::note_as(o, self.tick, NoteKind::Gate, format!("Had {} bodies dragged out of {}, so its traps can bite again.", bodies.len(), name));
            }
        }
    }

    /// For an overseer that lets the army in (O04): the cage traps taken up
    /// from the near row, so the army comes on along it into the hall
    /// rather than losing its first few to the cages and turning back; and
    /// a second lever on the bridge behind the hall, where the squads hold,
    /// to shut them in.
    fn open_the_way(&mut self, o: &mut Overseer, base: Pos) {
        let near_row: Vec<u32> = (4..8)
            .filter_map(|dx| match self.world.get(base.offset(dx, 0, 0)).map_or(0, |t| t.building) {
                0 => None,
                id => self.buildings.get(&id).filter(|b| b.kind == BuildingKind::CageTrap).map(|b| b.id),
            })
            .collect();
        if !near_row.is_empty() && !self.alert {
            for id in &near_row {
                self.remove_building(*id);
            }
            let name = self.trap_hall_name().unwrap_or_else(|| "the trap hall".into());
            Self::note_as(o, self.tick, NoteKind::Gate, format!("Took up {} cage traps from the near side of the corridor: let them walk freely into {}.", near_row.len(), name));
        }
        if self.inner_lever(base).is_some() || !o.did("linked") {
            return;
        }
        let bridges = self.gate_bridges(base);
        let built = bridges.len() == 2 && bridges.iter().all(|id| self.buildings.get(id).map_or(false, |b| b.built));
        let at = INNER_LEVER.iter().map(|&(dx, dy)| base.offset(dx, dy, 0)).find(|&p| self.building_at(p) == Some(BuildingKind::Lever));
        match at {
            None => {
                if built && self.stock_of(ItemKind::Mechanism) >= 1 {
                    if let Some(p) = INNER_LEVER.iter().map(|&(dx, dy)| base.offset(dx, dy, 0)).find(|&p| self.can_place(BuildingKind::Lever, p)) {
                        self.place_building(BuildingKind::Lever, p);
                    }
                }
            }
            Some(p) => {
                let Some(lever) = self.buildings.values().find(|b| b.kind == BuildingKind::Lever && b.pos == p && b.built).map(|b| b.id) else { return };
                if !built {
                    return;
                }
                for id in bridges {
                    if !self.buildings[&lever].links.contains(&id) {
                        let _ = self.link_lever(lever, id);
                    }
                }
                if self.inner_lever(base).is_some() && !o.did("innerlever") {
                    o.done.push("innerlever".into());
                    let name = self.trap_hall_name().unwrap_or_else(|| "the trap hall".into());
                    Self::note_as(o, self.tick, NoteKind::Gate, format!("Linked a second lever to the drawbridge, behind {}: let them in, then shut them in.", name));
                }
            }
        }
    }

    /// Place whatever of the trap hall isn't standing (a tile already
    /// holding some other trap keeps it); how many were placed.
    fn lay_trap_hall(&mut self, base: Pos) -> usize {
        let mut n = 0;
        for (dx, kind) in HALL {
            for dy in 0..2 {
                let p = base.offset(dx, dy, 0);
                if self.building_at(p).is_none() && self.place_building(kind, p) {
                    n += 1;
                }
            }
        }
        n
    }

    /// The built pieces of one kind in the trap hall.
    fn hall_parts(&self, base: Pos, kind: BuildingKind) -> Vec<u32> {
        let mut v: Vec<u32> = HALL
            .iter()
            .filter(|h| h.1 == kind)
            .flat_map(|&(dx, _)| (0..2).map(move |dy| base.offset(dx, dy, 0)))
            .filter_map(|p| match self.world.get(p).map_or(0, |t| t.building) {
                0 => None,
                id => self.buildings.get(&id).filter(|b| b.kind == kind && b.built).map(|b| b.id),
            })
            .collect();
        v.sort_unstable();
        v
    }

    /// The trap hall's name, if the fort has one: the place over its
    /// spikes and plates (renamed, it may be, for a deed done there).
    pub(super) fn trap_hall_name(&self) -> Option<String> {
        let mut plates: Vec<Pos> = self.buildings.values().filter(|b| b.kind == BuildingKind::PressurePlate).map(|b| b.pos).collect();
        plates.sort_unstable_by_key(|p| (p.z, p.y, p.x));
        plates.into_iter().find_map(|p| self.place_at(p).filter(|pl| pl.kind == super::places::PlaceKind::Gate).map(|pl| pl.name.clone()))
    }

    /// Whether the overseer lets an army into the trap hall before raising
    /// the bridge behind it (O04 step 2): a bold or warlike one, not a
    /// cautious one, once the hall's spikes stand on a linked plate.
    pub(super) fn lets_them_in(&self, o: &Overseer) -> bool {
        // (Any overseer whose pit bridge is ready lets them come on to it:
        // a pit is dug to be used. pits.rs.)
        let pit = self.pit_lever().is_some() && self.knob("gate.pit_lure", 1.0) > 0.0;
        if o.gate_stage == 0 || (o.has(Trait::Cautious) && !pit) || !(o.has(Trait::Bold) || o.style == Style::Warlord || pit) || self.knob("gate.let_in", 1.0) <= 0.0 {
            return false;
        }
        let Some(base) = o.base else { return false };
        self.hall_parts(base, BuildingKind::PressurePlate).iter().any(|p| !self.buildings[p].links.is_empty())
    }

    /// Where squad `s` holds during a siege: behind the trap hall when the
    /// overseer means to let them into it, else just inside the gate.
    pub(super) fn hold_station(&self, o: &Overseer, s: usize, base: Pos) -> Pos {
        if self.lets_them_in(o) {
            base.offset(14, s as i32 % 2, 0)
        } else {
            base.offset(4 + s as i32, 0, 0)
        }
    }

    /// A siege is over (`on_siege_end`): the trap hall's toll, told. "Seven
    /// goblins fell in the Hall of Teeth." If its traps took half the army
    /// or more, the siege broke on it.
    pub(super) fn trap_hall_toll(&mut self, i: usize) {
        let Some(s) = self.annals.sieges.get(i) else { return };
        let (n, caught, size, foe) = (s.trapped, s.foes_caught, s.size, s.foe.clone());
        if n == 0 {
            return;
        }
        let hall = self.trap_hall_name();
        let place = hall.clone().unwrap_or_else(|| "the traps at the gate".into());
        let who = if n == 1 { foe.trim_end_matches('s').to_string() } else { foe.clone() };
        let text = format!("{} {} fell in {}.", count_words(n), who, place);
        let broke = (n + caught) * 2 >= size.max(1);
        let at = self.overseer.as_ref().and_then(|o| o.base).map(|b| b.offset(10, 0, 0));
        if broke {
            let s = &mut self.annals.sieges[i];
            s.broke_on = Some(place.clone());
            if s.broke_at.is_none() {
                s.broke_at = at;
            }
        }
        self.count("m2.hall_tolls", 1);
        if n >= 3 {
            self.count("m2.hall_tolls3", 1);
        }
        self.msg_at(&text, (240, 170, 110), at, super::logkind::LogKind::Siege);
        self.record(super::memory::MemoryKind::SiegeEnd, &[], at, text.clone(), if broke { 6 } else { 4 });
        if n >= 3 || broke {
            let title = hall.map_or("The traps held".to_string(), |h| {
                let mut c = h.chars();
                c.next().map_or(h.clone(), |f| f.to_uppercase().collect::<String>() + c.as_str())
            });
            let more = if broke { format!(" The {} broke upon it.", foe) } else { String::new() };
            self.spotlight(&title, &format!("{}{}", text, more), at, (240, 170, 110));
        }
        if let Some(mut o) = self.overseer.take() {
            Self::note_as(&mut o, self.tick, NoteKind::Gate, text);
            self.overseer = Some(o);
        }
    }

    /// Whether the gate still wants mechanisms and stone is short: it holds
    /// the stone for them.
    pub(super) fn gate_wants_stone(&self, o: &Overseer) -> bool {
        if o.style == Style::Reckless || o.base.is_none() {
            return false;
        }
        let want = if o.did("linked") { self.gate_parts_missing(o) } else { 3 };
        let mechs = self.stock_of(ItemKind::Mechanism);
        let loose = self.items.values().filter(|it| it.kind == ItemKind::Boulder && it.mat.is_building_stone() && it.free()).count();
        mechs < want && loose < 12
    }

    /// Parts of the gate's drawbridge and lever not standing built (a half
    /// lost to a siege, say), and halves standing that the lever doesn't
    /// work, each needing a mechanism to put back or link.
    pub(super) fn gate_parts_missing(&self, o: &Overseer) -> usize {
        let Some(base) = o.base else { return 0 };
        let lost = [(BuildingKind::Bridge, base.offset(2, 0, 0)), (BuildingKind::Bridge, base.offset(2, 1, 0)), (BuildingKind::Lever, base.offset(3, 1, 0))]
            .iter()
            .filter(|&&(k, p)| !self.buildings.values().any(|b| b.kind == k && b.pos == p && b.built))
            .count();
        lost + self.gate_links_missing(base)
    }

    /// Whether the gate's drawbridge and lever still want mechanisms (to
    /// build, or link): they have the first of them, before traps, the trap
    /// hall, the pit or the cistern take any.
    pub(super) fn gate_first(&self, o: &Overseer) -> bool {
        o.style != Style::Reckless && o.base.is_some() && (!o.did("linked") || self.gate_parts_missing(o) > 0)
    }

    /// Built halves of the gate's drawbridge that its lever (built) doesn't
    /// work yet.
    pub(super) fn gate_links_missing(&self, base: Pos) -> usize {
        let Some(lever) = self.gate_lever(base) else { return 0 };
        self.gate_bridges(base).iter().filter(|id| self.buildings.get(id).map_or(false, |b| b.built) && !self.buildings[&lever].links.contains(id)).count()
    }

    /// Whether `p` is in the gatehouse's walls (two deep on each side of the
    /// corridor, and round the foot of the stair), which nothing may be cut
    /// from: it would open a way round the doors and the bridge.
    pub(super) fn gatehouse(base: Pos, p: Pos) -> bool {
        // (And the rock under the pit bridge, kept whole for the pit: pits.rs.)
        let pit = |p: Pos| {
            use super::pits::{PIT, PIT_DEPTH};
            p.z < base.z && p.z >= base.z - PIT_DEPTH && (PIT[0] - 1..=PIT[1] + 1).contains(&(p.x - base.x)) && (-1..=2).contains(&(p.y - base.y))
        };
        (p.z == base.z && (-2..=16).contains(&(p.x - base.x)) && (-2..=3).contains(&(p.y - base.y))) || pit(p)
    }
}

#[cfg(test)]
mod tests {
    use super::super::commands::Command;
    use super::super::personality::Trait;
    use super::*;

    /// O01: "b" on a siege card raises the bridge, though no army is near;
    /// a stubborn overseer may refuse, but says so.
    #[test]
    fn raise_the_bridge_from_the_card() {
        let (found, passed) = super::super::overseer::fort_where(Style::Steady, 5000, 12, 20, |g| g.overseer.as_ref().map_or(false, |o| o.did("linked")));
        let Some((seed, mut g)) = found else { panic!("no fort from 5000 on had its gate linked by day 20 (passed over {:?})", passed) };
        g.set_traits(vec![Trait::Cautious]).unwrap();
        let base = g.overseer.as_ref().unwrap().base.unwrap();
        let raised = |g: &Game| g.gate_bridges(base).iter().any(|id| g.buildings.get(id).map_or(false, |b| b.raised));
        assert!(!raised(&g), "seed {}: the bridge is up already", seed);
        let said = g.command_overseer(Command::RaiseBridge);
        let mut t = 0;
        while t < 200 && !raised(&g) {
            g.step();
            t += 1;
        }
        assert!(raised(&g), "seed {}: the bridge never went up ({})", seed, said);
        // Half a day on, the order has lapsed.
        for _ in 0..DAY / 2 {
            g.step();
        }
        assert!(!g.overseer.as_ref().unwrap().ordered(Command::RaiseBridge, g.tick));
        // A stubborn overseer may refuse; then it says so in its notes.
        g.set_traits(vec![Trait::Stubborn]).unwrap();
        for i in 0..6 {
            g.tick += i * 7;
            let said = g.command_overseer(Command::RaiseBridge);
            assert!(g.overseer.as_ref().unwrap().notes.iter().any(|n| n.1 == said), "{}", said);
        }
    }

    /// A lever is linked to the drawbridge only with a spare mechanism for
    /// each half. Without one the overseer doesn't count the gate linked:
    /// it keeps asking for mechanisms, and nothing else takes them first
    /// (it once thought it linked, stopped asking, and never raised it).
    #[test]
    fn the_gate_is_not_linked_without_the_mechanisms() {
        let (found, passed) = super::super::overseer::fort_where(Style::Steady, 5000, 12, 20, |g| g.overseer.as_ref().map_or(false, |o| o.did("linked")));
        let Some((seed, mut g)) = found else { panic!("no fort from 5000 on had its gate linked by day 20 (passed over {:?})", passed) };
        let mut o = g.overseer.take().unwrap();
        let base = o.base.unwrap();
        let lever = g.gate_lever(base).expect("no gate lever");
        assert_eq!(g.gate_bridges(base).len(), 2);
        assert_eq!(g.gate_parts_missing(&o), 0, "seed {}", seed);
        // Unlinked, with no mechanism to spare.
        g.buildings.get_mut(&lever).unwrap().links.clear();
        o.done.retain(|d| d != "linked");
        g.items.retain(|_, it| it.kind != ItemKind::Mechanism);
        g.ov_defences(&mut o);
        assert!(!o.did("linked"), "seed {}: counted linked with no mechanism to link it", seed);
        assert_eq!(g.gate_parts_missing(&o), 2);
        assert!(g.gate_first(&o));
        // Unlinked though it once was (a half rebuilt): still the gate first.
        o.done.push("linked".into());
        assert!(g.gate_first(&o));
        o.done.retain(|d| d != "linked");
        // With two to spare, it links both halves, and says so.
        let e = g.embark;
        for _ in 0..2 {
            g.spawn_item(ItemKind::Mechanism, Mat::Granite, e);
        }
        g.ov_defences(&mut o);
        assert!(o.did("linked"), "seed {}", seed);
        assert_eq!(g.gate_parts_missing(&o), 0);
        assert!(!g.gate_first(&o));
        g.overseer = Some(o);
    }

    /// A Warlord fort whose gate is linked, with the parts for the trap hall
    /// put in its hands and the hall laid out (not yet built).
    fn hall_fort() -> (u64, Game) {
        let (found, passed) = super::super::overseer::fort_where(Style::Warlord, 5000, 12, 20, |g| g.overseer.as_ref().map_or(false, |o| o.did("linked")));
        let Some((seed, mut g)) = found else { panic!("no Warlord fort from 5000 on had its gate linked by day 20 (passed over {:?})", passed) };
        let e = g.embark;
        for _ in 0..10 {
            g.spawn_item(ItemKind::Mechanism, Mat::Granite, e);
        }
        for _ in 0..2 {
            g.spawn_item(ItemKind::Spear, Mat::Iron, e);
            g.spawn_item(ItemKind::Thread, Mat::Flesh, e);
        }
        for _ in 0..3 * DAY {
            if g.overseer.as_ref().unwrap().gate_stage >= 1 {
                break;
            }
            g.step();
        }
        (seed, g)
    }

    /// Finish the hall's pieces at once, as if the mechanics had, and let
    /// the overseer link its plates.
    fn finish_hall(g: &mut Game) {
        let base = g.overseer.as_ref().unwrap().base.unwrap();
        for (dx, kind) in HALL {
            for dy in 0..2 {
                let p = base.offset(dx, dy, 0);
                let id = g.world.t(p).building;
                if let Some(b) = g.buildings.get_mut(&id).filter(|b| b.kind == kind) {
                    b.built = true;
                    b.parts = kind.parts();
                    b.load = Some(if kind == BuildingKind::Spikes || kind == BuildingKind::WeaponTrap { (ItemKind::Spear, Mat::Iron, 0) } else { (ItemKind::Mechanism, Mat::Granite, 0) });
                }
            }
        }
        let mut o = g.overseer.take().unwrap();
        g.ov_trap_hall(&mut o);
        g.overseer = Some(o);
    }

    /// O04 step 1a: once the gate's lever is linked, a Warlord lays out the
    /// trap hall behind the cages (tripwires, spikes, plates, weapon traps),
    /// names it, and links the plates to the spikes; a plain overseer waits
    /// for a siege or half a year.
    #[test]
    fn the_trap_hall_is_built_after_the_gate() {
        let (seed, mut g) = hall_fort();
        let o = g.overseer.as_ref().unwrap();
        let base = o.base.unwrap();
        assert_eq!(o.gate_stage, 1, "seed {}: no trap hall three days after the parts came", seed);
        assert!(o.did("traphall") && o.notes.iter().any(|n| n.1.contains("trap hall")), "seed {}: the overseer never said so", seed);
        for (dx, kind) in HALL {
            assert!((0..2).any(|dy| g.building_at(base.offset(dx, dy, 0)) == Some(kind)), "seed {}: no {:?} at x+{}", seed, kind, dx);
        }
        let name = g.trap_hall_name().expect("the hall has no name");
        assert!(HALL_NAMES.contains(&name.as_str()), "{}", name);
        // Built (here, at once), its plates fire every spike.
        finish_hall(&mut g);
        let plates = g.hall_parts(base, BuildingKind::PressurePlate);
        let spikes = g.hall_parts(base, BuildingKind::Spikes);
        assert!(!plates.is_empty() && !spikes.is_empty());
        assert!(plates.iter().all(|p| spikes.iter().all(|s| g.buildings[p].links.contains(s))), "the plates aren't linked to the spikes");
        // A plain overseer with no siege yet isn't in a hurry; a cautious one is.
        g.set_traits(vec![Trait::Cheerful]).unwrap();
        let mut o = g.overseer.take().unwrap();
        o.style = Style::Steady;
        g.annals.sieges.clear();
        assert!(!g.trap_hall_due(&o));
        o.style = Style::Reckless;
        assert!(!g.trap_hall_due(&o));
        o.style = Style::Steady;
        g.overseer = Some(o);
        g.set_traits(vec![Trait::Cautious]).unwrap();
        assert!(g.trap_hall_due(g.overseer.as_ref().unwrap()));
    }

    /// O04 step 2: a bold overseer with a trap hall leaves the bridge down
    /// for the army to come into it, where a cautious one raises it at
    /// once; and its squads hold behind the hall.
    #[test]
    fn a_bold_overseer_lets_them_into_the_killing_hall() {
        let (seed, mut g) = hall_fort();
        finish_hall(&mut g);
        let (top, base) = { let o = g.overseer.as_ref().unwrap(); (o.top.unwrap(), o.base.unwrap()) };
        let lever = g.gate_lever(base).expect("no gate lever");
        // When was the bridge called for, after an army was first seen?
        let called_at = |g: &mut Game, traits: Vec<Trait>| -> u64 {
            g.set_traits(traits).unwrap();
            let mut o = g.overseer.take().unwrap();
            o.army_since = 0;
            o.raised_at = 0;
            g.buildings.get_mut(&lever).unwrap().pull = false;
            let start = g.tick;
            let mut t = start;
            while t < start + 1000 {
                g.tick = t;
                g.ov_gate_alert(&mut o, top, base, true);
                let b = &g.buildings[&lever];
                if b.pull || b.raised {
                    break;
                }
                t += 10;
            }
            g.overseer = Some(o);
            if g.buildings[&lever].raised {
                g.pull_lever(lever);
            }
            g.buildings.get_mut(&lever).unwrap().pull = false;
            t - start
        };
        assert!(g.lets_them_in(g.overseer.as_ref().unwrap()), "seed {}: a Warlord with a hall should let them in", seed);
        assert_eq!(g.hold_station(g.overseer.as_ref().unwrap(), 0, base), base.offset(14, 0, 0));
        let bold = called_at(&mut g, vec![Trait::Bold]);
        let cautious = called_at(&mut g, vec![Trait::Cautious]);
        assert!(!g.lets_them_in(g.overseer.as_ref().unwrap()));
        assert!(bold >= cautious + 200, "seed {}: the bold overseer called the bridge up after {} ticks, the cautious one after {}", seed, bold, cautious);
    }

    /// O04, the hall made to kill: an overseer that lets them in takes the
    /// cage traps up from the near row, ties the wires to the spikes, and
    /// sets a second lever behind the hall, which it pulls to shut in an
    /// army that has come into it.
    #[test]
    fn a_bold_overseer_shuts_them_in_from_behind() {
        let (seed, mut g) = hall_fort();
        finish_hall(&mut g);
        let (top, base) = { let o = g.overseer.as_ref().unwrap(); (o.top.unwrap(), o.base.unwrap()) };
        // A cage trap on the near row, as the gate's first traps leave one.
        let _ = g.place_building(BuildingKind::CageTrap, base.offset(5, 0, 0));
        for _ in 0..4 {
            g.spawn_item(ItemKind::Mechanism, Mat::Granite, g.embark);
        }
        g.alert = false;
        let mut o = g.overseer.take().unwrap();
        assert!(g.lets_them_in(&o), "seed {}: a Warlord with a hall should let them in", seed);
        g.ov_trap_hall(&mut o);
        assert!((4..8).all(|dx| g.building_at(base.offset(dx, 0, 0)) != Some(BuildingKind::CageTrap)), "seed {}: cage traps left on the near row", seed);
        let wires = g.hall_parts(base, BuildingKind::Tripwire);
        assert!(!wires.is_empty() && wires.iter().all(|w| !g.buildings[w].links.is_empty()), "the wires aren't tied to the spikes");
        let lp = INNER_LEVER.iter().map(|&(dx, dy)| base.offset(dx, dy, 0)).find(|&p| g.building_at(p) == Some(BuildingKind::Lever));
        let lp = lp.unwrap_or_else(|| panic!("seed {}: no lever laid behind the hall", seed));
        let id = g.world.t(lp).building;
        {
            let b = g.buildings.get_mut(&id).unwrap();
            b.built = true;
            b.parts = BuildingKind::Lever.parts();
        }
        g.ov_trap_hall(&mut o);
        assert_eq!(g.inner_lever(base), Some(id), "seed {}: the inner lever isn't linked to the bridge", seed);
        assert!(o.did("innerlever"));
        // Most of an army in the hall: the inner lever is the one called for.
        g.creatures.retain(|_, c| !c.siege);
        for dy in 0..2 {
            let c = g.spawn_creature(crate::creature::Species::Goblin, base.offset(9, dy, 0), u64::MAX);
            g.creatures.get_mut(&c).unwrap().siege = true;
        }
        let e = g.embark;
        for d in g.dwarves.iter_mut() {
            d.pos = e;
        }
        g.ov_gate_alert(&mut o, top, base, true);
        g.overseer = Some(o);
        assert!(g.buildings[&id].pull, "seed {}: nobody was asked to pull the lever behind the hall", seed);
        assert!(g.gate_lever(base).map_or(true, |l| !g.buildings[&l].pull), "the outer lever was called for");
    }

    /// When the siege ends, the hall's toll is told, and a hall that took
    /// half the army is where the siege broke.
    #[test]
    fn the_trap_halls_toll_is_told() {
        let (_, mut g) = hall_fort();
        let name = g.trap_hall_name().unwrap();
        g.annals.sieges.push(super::super::annals::SiegeNote { foe: "goblins".into(), size: 10, trapped: 7, ended: Some(g.tick), outcome: "routed".into(), ..Default::default() });
        let i = g.annals.sieges.len() - 1;
        g.trap_hall_toll(i);
        let said = format!("Seven goblins fell in {}.", name);
        assert!(g.log.iter().any(|l| l.text == said), "nobody said: {}", said);
        assert_eq!(g.annals.sieges[i].broke_on.as_deref(), Some(name.as_str()));
        assert!(g.overseer.as_ref().unwrap().notes.iter().any(|n| n.1 == said));
    }

}
