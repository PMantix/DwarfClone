//! Traps that fire on creatures, and levers that work doors, drawbridges and floodgates.
//!
//! The trap hall (O04) adds upright spikes fired by a pressure plate or a
//! lever, tripwires that throw a foe down under the traps behind, stone-fall
//! traps that drop their stone three times, and traps jammed by the bodies
//! and cages piled on them until someone clears them away.

use super::{Game, Travel, SEARCH_LIMIT};
use crate::buildings::BuildingKind;
use crate::creature::Species;
use crate::dwarf::{Dwarf, Labor, Task};
use crate::geom::{Pos, DIRS8};
use crate::items::ItemKind;
use crate::material::Mat;
use crate::path;
use crate::world::{Mover, BRIDGE_RAISED, DOOR_LOCKED, DOOR_NONE, DOOR_OPEN, FLOODGATE_SHUT};

/// `Building.timer` on a trap jammed with bodies.
const JAMMED: u64 = 1;
/// Drops a stone-fall trap's stone makes before it is spent.
const STONE_DROPS: u64 = 3;
/// Ticks a tripped creature lies on the floor.
const TRIPPED: u32 = 30;

/// TRAP_TRACE set: the trap hall's doings, on stderr.
fn tracing() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("TRAP_TRACE").is_ok())
}

impl Game {
    /// Every tick: armed traps go off on whatever creature stands on them.
    /// Dwarves know where their traps are and never set them off.
    pub(super) fn traps_tick(&mut self) {
        // (Spikes wait for their plate or lever.)
        let traps: Vec<(u32, Pos, BuildingKind)> = self
            .buildings
            .values()
            .filter(|b| b.kind.is_trap() && b.kind != BuildingKind::Spikes && b.built && b.trap_cd <= self.tick)
            .map(|b| (b.id, b.pos, b.kind))
            .collect();
        for (tid, pos, kind) in traps {
            // A pressure plate is set to feel a foe's weight, not a cat's;
            // a tripwire is strung at a goblin's shin, not a rat's.
            let plate = matches!(kind, BuildingKind::PressurePlate | BuildingKind::Tripwire);
            let victim = self
                .creatures
                .values()
                .find(|c| c.alive && c.pos == pos && c.species != Species::Merchant && !c.tame && (!plate || c.hostile()))
                .map(|c| c.id);
            if let Some(cid) = victim {
                if self.jammed(tid) {
                    if tracing() {
                        eprintln!("  T{} {:?} at {:?} jammed", self.tick, kind, pos);
                    }
                    continue;
                }
                if tracing() {
                    eprintln!("  T{} {:?} {} springs on {}", self.tick, kind, tid, cid);
                }
                self.spring_trap(tid, cid);
            }
        }
        self.hall_entries();
    }

    /// Measure (`m2.hall_entered`): each besieger that sets foot in the
    /// trap hall, counted once, against `m2.trap_kills`. The ids seen are
    /// kept per thread, for the fort being played on it (not saved).
    fn hall_entries(&mut self) {
        thread_local! {
            static SEEN: std::cell::RefCell<(u64, std::collections::HashSet<u32>)> = Default::default();
        }
        let Some(base) = self.overseer.as_ref().filter(|o| o.gate_stage >= 1).and_then(|o| o.base) else { return };
        let fresh: Vec<u32> = self
            .creatures
            .values()
            .filter(|c| c.alive && c.siege && c.pos.z == base.z && (8..=11).contains(&(c.pos.x - base.x)) && (0..=1).contains(&(c.pos.y - base.y)))
            .map(|c| c.id)
            .collect();
        if self.tick % 5 == 0 && tracing() {
            self.trace_hall(base);
        }
        if fresh.is_empty() {
            return;
        }
        let fort = super::stories::hash(&self.fortress_name, base.x as u64 * 1000 + base.y as u64);
        let n = SEEN.with(|s| {
            let mut s = s.borrow_mut();
            if s.0 != fort {
                *s = (fort, Default::default());
            }
            fresh.into_iter().filter(|id| s.1.insert(*id)).count()
        });
        if n > 0 {
            self.count("m2.hall_entered", n as u32);
        }
    }

    /// Whether two or more bodies or cages lie on a trap's tile (ENG-01):
    /// it can't fire until they're cleared away. Said once, when it jams.
    pub(super) fn jammed(&mut self, tid: u32) -> bool {
        let Some(b) = self.buildings.get(&tid) else { return false };
        if matches!(b.kind, BuildingKind::PressurePlate | BuildingKind::Tripwire) {
            return false;
        }
        let pos = b.pos;
        let heaped = self
            .items
            .values()
            .filter(|it| it.pos == pos && it.carried_by.is_none() && matches!(it.kind, ItemKind::Corpse | ItemKind::Carcass | ItemKind::Cage))
            .count();
        let jammed = heaped >= 2;
        let (was, name) = (b.timer == JAMMED, b.kind.name().to_lowercase());
        let tick = self.tick;
        let b = self.buildings.get_mut(&tid).unwrap();
        if jammed {
            // Look again in a little while, not every tick.
            b.trap_cd = tick + 20;
        }
        // (A stone-fall trap counts its drops in `timer`, and spikes their
        // time up: neither is marked.)
        if !matches!(b.kind, BuildingKind::StoneTrap | BuildingKind::Spikes) {
            if jammed && !was {
                b.timer = JAMMED;
                self.msg_at(&format!("A {} is jammed with bodies!", name), (230, 170, 120), Some(pos), super::logkind::LogKind::Siege);
            } else if !jammed && was {
                b.timer = 0;
            }
        }
        jammed
    }

    pub(super) fn spring_trap(&mut self, tid: u32, cid: u32) {
        let Some(b) = self.buildings.get(&tid) else { return };
        let (kind, pos, quality) = (b.kind, b.pos, b.quality);
        let load = b.load.unwrap_or((ItemKind::Boulder, Mat::Granite, 0));
        let name = self.creatures.get(&cid).map_or("creature", |c| c.name()).to_string();
        match kind {
            BuildingKind::CageTrap => {
                let Some(mut c) = self.creatures.remove(&cid) else { return };
                for id in c.weapon.take().into_iter().chain(c.armor.drain(..)) {
                    self.drop_item(id, pos);
                }
                let cage = self.spawn_item(ItemKind::Cage, load.1, pos);
                if let Some(it) = self.items.get_mut(&cage) {
                    it.label = Some(name.clone());
                }
                self.msg(&format!("A {} has been caught in a cage trap!", name), (160, 230, 160));
                if c.siege || c.species.person() {
                    self.take_captive(&c, cage);
                }
                self.rearm(tid);
            }
            BuildingKind::WeaponTrap | BuildingKind::StoneTrap => {
                let stone = kind == BuildingKind::StoneTrap;
                let (dmg, verb, edged, hits) = if stone {
                    (14.0 * load.1.weapon_mult(), "crushes", false, 1)
                } else {
                    // Better-made weapons hit harder.
                    let d = load.0.weapon_damage().unwrap_or(8.0) * load.1.weapon_mult() * (1.0 + 0.1 * load.2 as f32);
                    (d, load.0.weapon_verb(), load.0.edged(), 3)
                };
                // A mechanism strikes true far more often than a fighter does.
                let att = super::combat::Fighter {
                    name: if stone { "the falling stone".into() } else { "the weapon trap".into() },
                    skill: 8 + quality as u32,
                    dmg,
                    verb,
                    edged,
                    helm: 0,
                    mail: 0,
                    shield: false,
                    size: if stone { 2.0 } else { 1.0 },
                    teeth: false,
                };
                for _ in 0..hits {
                    let Some(c) = self.creatures.get(&cid) else { break };
                    if !c.alive {
                        break;
                    }
                    let def = self.creature_fighter(c);
                    let att = self.sprawled(&att, c);
                    let mut body = c.body.clone();
                    let (cpos, csp, owner, seen) = (c.pos, c.species, super::gore::owner_of_creature(c), super::melee::a_or_name(c));
                    let hp: i32 = body.hp.iter().sum();
                    let blow = self.strike(&att, &def, &mut body);
                    if body.hp.iter().sum::<i32>() < hp || body.dead {
                        self.trap_hurt(cid);
                    }
                    if let Some(m) = blow.msg.as_ref().filter(|_| blow.notable) {
                        self.msg(m, (235, 200, 130));
                    }
                    self.aftermath(&blow, cpos, cpos, &owner, &seen, Some(csp), false);
                    let dead = body.dead;
                    if let Some(c) = self.creatures.get_mut(&cid) {
                        c.body = body;
                    }
                    if dead {
                        if let Some(c) = self.creatures.remove(&cid) {
                            self.trap_kill(c);
                        }
                        break;
                    }
                }
                if stone {
                    // Winched back up and dropped again (ENG-19), three
                    // times; then the stone is spent, left on the tile, and
                    // the trap needs another.
                    let drops = self.buildings.get(&tid).map_or(STONE_DROPS, |b| b.timer + 1);
                    if drops >= STONE_DROPS {
                        self.spawn_item(ItemKind::Boulder, load.1, pos);
                        if let Some(b) = self.buildings.get_mut(&tid) {
                            b.timer = 0;
                        }
                        self.rearm(tid);
                    } else if let Some(b) = self.buildings.get_mut(&tid) {
                        b.timer = drops;
                        b.trap_cd = self.tick + 30;
                    }
                } else if let Some(b) = self.buildings.get_mut(&tid) {
                    b.trap_cd = self.tick + 25;
                }
            }
            BuildingKind::PressurePlate => {
                // Click: everything it's linked to fires.
                if let Some(b) = self.buildings.get_mut(&tid) {
                    b.trap_cd = self.tick + 15;
                }
                let links = self.buildings.get(&tid).map_or(Vec::new(), |b| b.links.clone());
                let fired: Vec<u32> = links.into_iter().filter(|&s| self.fire_spikes(s)).collect();
                if !fired.is_empty() && self.creatures.get(&cid).map_or(false, |c| c.siege) {
                    let hall = self.trap_hall_name().map_or(String::new(), |h| format!(" in {}", h));
                    self.msg_at(&format!("A {} treads on a pressure plate: spikes spring up{}!", name, hall), (240, 150, 110), Some(pos), super::logkind::LogKind::Siege);
                }
            }
            BuildingKind::Tripwire => {
                // Down it goes, headlong onto the next tile it was making
                // for if the spikes wait there, and lies there a while.
                let ahead = self.creatures.get(&cid).and_then(|c| c.path.last().copied()).filter(|&q| {
                    q.z == pos.z && (q.x - pos.x).abs() <= 1 && (q.y - pos.y).abs() <= 1 && q != pos && self.building_at(q) == Some(BuildingKind::Spikes)
                });
                if let Some(c) = self.creatures.get_mut(&cid) {
                    c.move_cd = c.move_cd.max(TRIPPED);
                    c.attack_cd = c.attack_cd.max(TRIPPED);
                    if let Some(q) = ahead {
                        c.pos = q;
                        c.path.pop();
                    }
                }
                // (Reset once it's up and has had time to stagger on.)
                if let Some(b) = self.buildings.get_mut(&tid) {
                    b.trap_cd = self.tick + TRIPPED as u64 + 30;
                }
                self.count("m2.tripped", 1);
                if self.creatures.get(&cid).map_or(false, |c| c.siege) {
                    let text = if ahead.is_some() { format!("A {} trips on a wire and sprawls headlong onto the spikes.", name) } else { format!("A {} trips on a wire and sprawls on the floor.", name) };
                    self.msg_at(&text, (220, 200, 160), Some(pos), super::logkind::LogKind::Siege);
                }
                // The wire is tied to the spikes' catch: they spring up.
                let links = self.buildings.get(&tid).map_or(Vec::new(), |b| b.links.clone());
                for s in links {
                    self.fire_spikes(s);
                }
            }
            _ => {}
        }
    }

    /// Upright spikes thrust up through whatever stands on them (ENG-04):
    /// three thrusts at each creature there, then they sink back. Whether
    /// they fired (not if unbuilt, still up, or jammed).
    pub fn fire_spikes(&mut self, sid: u32) -> bool {
        let Some(b) = self.buildings.get(&sid) else { return false };
        if b.kind != BuildingKind::Spikes || !b.built || b.trap_cd > self.tick {
            return false;
        }
        if self.jammed(sid) {
            return false;
        }
        let Some(b) = self.buildings.get(&sid) else { return false };
        let (pos, quality) = (b.pos, b.quality);
        let load = b.load.unwrap_or((ItemKind::Spear, Mat::Iron, 0));
        let tick = self.tick;
        if let Some(b) = self.buildings.get_mut(&sid) {
            b.trap_cd = tick + 30;
            b.timer = tick + 30;
        }
        // A bed of bolt points thrusts up as hard as a spear's head, and even
        // wooden ones are driven up with all the mechanism's weight.
        let base = 9.0;
        let mult = if load.0 == ItemKind::Bolt { load.1.weapon_mult().max(self.knob("trap.bolt_spike", 1.0) as f32) } else { load.1.weapon_mult() };
        if tracing() {
            let on = self.creatures.values().filter(|c| c.alive && c.pos == pos).count();
            eprintln!("  T{} spikes {} fire ({:?} {:?}) on {}", tick, sid, load.0, load.1, on);
        }
        let att = super::combat::Fighter {
            name: "the spikes".into(),
            skill: 8 + quality as u32,
            dmg: base * mult * (1.0 + 0.1 * load.2 as f32),
            verb: "impales",
            edged: true,
            helm: 0,
            mail: 0,
            shield: false,
            size: 1.0,
            teeth: false,
        };
        let on: Vec<u32> = self.creatures.values().filter(|c| c.alive && c.pos == pos && c.species != Species::Merchant && !c.tame).map(|c| c.id).collect();
        for cid in on {
            for _ in 0..3 {
                let Some(c) = self.creatures.get(&cid) else { break };
                if !c.alive {
                    break;
                }
                let def = self.creature_fighter(c);
                let att = self.sprawled(&att, c);
                let mut body = c.body.clone();
                let (cpos, csp, owner, seen) = (c.pos, c.species, super::gore::owner_of_creature(c), super::melee::a_or_name(c));
                let hp: i32 = body.hp.iter().sum();
                let blow = self.strike(&att, &def, &mut body);
                if body.hp.iter().sum::<i32>() < hp || body.dead {
                    self.trap_hurt(cid);
                }
                if let Some(m) = blow.msg.as_ref().filter(|_| blow.notable) {
                    self.msg(m, (235, 200, 130));
                }
                self.aftermath(&blow, cpos, cpos, &owner, &seen, Some(csp), false);
                let dead = body.dead;
                if let Some(c) = self.creatures.get_mut(&cid) {
                    c.body = body;
                }
                if dead {
                    if let Some(c) = self.creatures.remove(&cid) {
                        self.trap_kill(c);
                    }
                    break;
                }
            }
        }
        true
    }

    /// A trap's blow has killed `c` on the spot (its toll is taken in
    /// `hall_death`, as it dies).
    fn trap_kill(&mut self, c: crate::creature::Creature) {
        if c.siege && !c.legendary() {
            self.count("m2.trap_blow_kills", 1);
        }
        self.creature_died(c);
    }

    /// A trap has wounded creature `cid`: a besieger is marked, so its death
    /// is the trap hall's even if it bleeds out somewhere else.
    fn trap_hurt(&mut self, cid: u32) {
        if !self.creatures.get(&cid).map_or(false, |c| c.siege && !c.legendary()) {
            return;
        }
        if let Some(s) = self.annals.sieges.iter_mut().rev().find(|s| s.ended.is_none()) {
            if !s.trap_hurt.contains(&cid) {
                s.trap_hurt.push(cid);
            }
        }
    }

    /// A besieger has died (`note_foe_slain`): it counts toward the siege's
    /// toll in the trap hall (`SiegeNote.trapped`) if a trap wounded it, or
    /// if it fell in the hall itself (x+8..11), wherever the blow came from.
    pub(super) fn hall_death(&mut self, c: &crate::creature::Creature) {
        let in_hall = self.overseer.as_ref().filter(|o| o.gate_stage >= 1).and_then(|o| o.base).map_or(false, |b| {
            c.pos.z == b.z && (8..=11).contains(&(c.pos.x - b.x)) && (0..=1).contains(&(c.pos.y - b.y))
        });
        let Some(s) = self.annals.sieges.iter_mut().rev().find(|s| s.ended.is_none()) else { return };
        let hurt = match s.trap_hurt.iter().position(|&id| id == c.id) {
            Some(i) => {
                s.trap_hurt.swap_remove(i);
                true
            }
            None => false,
        };
        if hurt || in_hall {
            s.trapped += 1;
            self.count("m2.trap_kills", 1);
            if !hurt {
                self.count("m2.hall_floor_kills", 1);
            }
        }
    }

    /// A foe lying where a tripwire threw it (or a blow sent it sprawling)
    /// can't twist away: a trap's blow finds it as it would a helpless one.
    fn sprawled(&self, att: &super::combat::Fighter, c: &crate::creature::Creature) -> super::combat::Fighter {
        let down = c.move_cd > 10;
        super::combat::Fighter {
            name: att.name.clone(),
            skill: att.skill + if down { self.knob("trap.sprawl_skill", 8.0) as u32 } else { 0 },
            dmg: att.dmg * if down { self.knob("trap.sprawl_dmg", 1.5) as f32 } else { 1.0 },
            ..*att
        }
    }

    /// TRAP_TRACE: the besiegers near the gate, and the hall's traps.
    fn trace_hall(&self, base: Pos) {
        let near: Vec<String> = self
            .creatures
            .values()
            .filter(|c| c.alive && c.siege && c.pos.z == base.z && (-3..=20).contains(&(c.pos.x - base.x)) && (-3..=4).contains(&(c.pos.y - base.y)))
            .map(|c| format!("{}@{},{} mv{}", c.id, c.pos.x - base.x, c.pos.y - base.y, c.move_cd))
            .collect();
        if near.is_empty() {
            return;
        }
        let traps: Vec<String> = self
            .buildings
            .values()
            .filter(|b| b.pos.z == base.z && (1..=16).contains(&(b.pos.x - base.x)) && (0..=1).contains(&(b.pos.y - base.y)))
            .map(|b| format!("{:?}{},{}{}{} L{} cd{} {:?}", b.kind, b.pos.x - base.x, b.pos.y - base.y, if b.built { "" } else { "(unbuilt)" }, if b.raised { "(up)" } else { "" }, b.links.len(), b.trap_cd.saturating_sub(self.tick), b.load.map(|l| (l.0, l.1))))
            .collect();
        let bodies = self.items.values().filter(|it| it.pos.z == base.z && (4..=14).contains(&(it.pos.x - base.x)) && (0..=1).contains(&(it.pos.y - base.y)) && matches!(it.kind, ItemKind::Corpse | ItemKind::Carcass)).count();
        let total = self.creatures.values().filter(|c| c.alive && c.siege).count();
        eprintln!("T{} [{} of {}] {} | bodies {} | {}", self.tick, near.len(), total, near.join(" "), bodies, traps.join(" "));
    }

    /// A spent cage or stone trap needs its last part delivered again.
    fn rearm(&mut self, tid: u32) {
        if let Some(b) = self.buildings.get_mut(&tid) {
            b.built = false;
            b.parts = b.kind.parts() - 1;
            b.load = None;
            b.retry_tick = 0;
        }
    }

    // ------------------------------------------------------------------ levers

    /// Link a lever to a door or drawbridge. Uses up a mechanism from stock.
    pub fn link_lever(&mut self, lever: u32, target: u32) -> Result<String, String> {
        let Some(t) = self.buildings.get(&target) else { return Err("Nothing to link there.".into()) };
        let plate = self.buildings.get(&lever).map_or(false, |l| matches!(l.kind, BuildingKind::PressurePlate | BuildingKind::Tripwire));
        if plate && t.kind != BuildingKind::Spikes {
            return Err("Pressure plates and tripwires work upright spikes.".into());
        }
        if !matches!(t.kind, BuildingKind::Door | BuildingKind::Bridge | BuildingKind::Floodgate | BuildingKind::Spikes) {
            return Err("Levers work doors, drawbridges, floodgates and spikes.".into());
        }
        let tname = t.name();
        if self.buildings.get(&lever).map_or(true, |l| !matches!(l.kind, BuildingKind::Lever | BuildingKind::PressurePlate | BuildingKind::Tripwire) || !l.built) {
            return Err("The lever isn't built yet.".into());
        }
        if self.buildings[&lever].links.contains(&target) {
            return Err(format!("Already linked to the {}.", tname.to_lowercase()));
        }
        // A plate's own mechanism does the work: linking it takes none.
        if plate {
            self.buildings.get_mut(&lever).unwrap().links.push(target);
            return Ok(format!("Plate linked to the {}.", tname.to_lowercase()));
        }
        let Some(mech) = self
            .items
            .values()
            .find(|it| it.kind == ItemKind::Mechanism && it.free())
            .map(|it| it.id)
        else {
            return Err("Linking needs a spare mechanism.".into());
        };
        self.items.remove(&mech);
        self.buildings.get_mut(&lever).unwrap().links.push(target);
        Ok(format!("Lever linked to the {}.", tname.to_lowercase()))
    }

    pub fn request_pull(&mut self, lever: u32) -> bool {
        match self.buildings.get_mut(&lever) {
            Some(b) if b.kind == BuildingKind::Lever && b.built && !b.links.is_empty() => {
                b.pull = true;
                true
            }
            _ => false,
        }
    }

    pub(super) fn find_lever_pull(&mut self, d: &mut Dwarf) -> bool {
        let levers: std::collections::HashMap<Pos, u32> = self
            .buildings
            .values()
            .filter(|b| b.kind == BuildingKind::Lever && b.pull && b.worker.is_none())
            .map(|b| (b.pos, b.id))
            .collect();
        if levers.is_empty() {
            return false;
        }
        let Some((p, path)) = path::search(&self.world, d.pos, SEARCH_LIMIT, Mover::Dwarf, |p| levers.contains_key(&p)) else {
            return false;
        };
        let bld = levers[&p];
        self.buildings.get_mut(&bld).unwrap().worker = Some(d.id);
        d.task = Task::PullLever { bld };
        Self::set_path(d, path);
        true
    }

    pub(super) fn do_pull_lever(&mut self, d: &mut Dwarf, bld: u32) {
        let Some(pos) = self.buildings.get(&bld).map(|b| b.pos) else {
            self.release_task(d);
            return;
        };
        match self.travel(d, pos) {
            Travel::Moving => {}
            Travel::Failed => self.release_task(d),
            Travel::Arrived => {
                // (Thrown already by someone standing by it, or no longer
                // wanted: then leave it be, or it goes back the other way.)
                if !self.buildings.get(&bld).map_or(false, |b| b.pull) {
                    if let Some(b) = self.buildings.get_mut(&bld).filter(|b| b.worker == Some(d.id)) {
                        b.worker = None;
                    }
                    d.task = Task::Idle;
                } else if Self::labor(d, 25, d.skill(Labor::Mechanics)) {
                    d.task = Task::Idle;
                    self.pull_lever(bld);
                }
            }
        }
    }

    /// The lever is thrown: linked doors lock or unlock, bridges rise or fall.
    pub fn pull_lever(&mut self, bld: u32) {
        let Some(l) = self.buildings.get_mut(&bld) else { return };
        l.pull = false;
        l.worker = None;
        l.raised = !l.raised;
        let links = l.links.clone();
        for t in links {
            let Some(b) = self.buildings.get_mut(&t) else { continue };
            let pos = b.pos;
            match b.kind {
                BuildingKind::Door if b.built => {
                    b.locked = !b.locked;
                    self.world.t_mut(pos).door = if b.locked { DOOR_LOCKED } else { DOOR_OPEN };
                    // A way in or out changed (path_throttle.rs).
                    self.dig_version += 1;
                }
                BuildingKind::Bridge if b.built => {
                    b.raised = !b.raised;
                    let raised = b.raised;
                    self.world.t_mut(pos).door = if raised { BRIDGE_RAISED } else { DOOR_NONE };
                    if raised {
                        self.clear_tile(pos);
                    }
                    self.dig_version += 1;
                }
                BuildingKind::Floodgate if b.built => {
                    b.locked = !b.locked;
                    let shut = b.locked;
                    self.world.t_mut(pos).door = if shut { FLOODGATE_SHUT } else { DOOR_NONE };
                    if shut {
                        self.clear_tile(pos);
                    }
                    self.dig_version += 1;
                }
                BuildingKind::Spikes => {
                    self.fire_spikes(t);
                }
                _ => {}
            }
        }
        self.msg("A lever has been pulled.", (220, 200, 120));
    }

    /// Whoever stands on a rising bridge is thrown off to one side.
    pub(super) fn clear_tile(&mut self, pos: Pos) {
        let spot = DIRS8.iter().map(|&(dx, dy)| pos.offset(dx, dy, 0)).find(|&q| self.world.walkable(q));
        let Some(to) = spot else { return };
        for d in self.dwarves.iter_mut().filter(|d| d.pos == pos) {
            d.pos = to;
            d.path.clear();
        }
        for c in self.creatures.values_mut().filter(|c| c.pos == pos) {
            c.pos = to;
            c.path.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::DAY;
    use crate::world::TileKind;

    /// A free floor tile near the wagon with nothing on it.
    fn spot(g: &Game, skip: &[Pos]) -> Pos {
        let e = g.embark;
        for r in 3..25 {
            for dy in -r..=r {
                for dx in -r..=r {
                    let p = e.offset(dx, dy, 0);
                    let p = Pos::new(p.x, p.y, g.world.surface_z(p.x, p.y));
                    let t = g.world.t(p);
                    if t.kind == TileKind::Floor && t.water == 0 && t.building == 0 && t.zone == 0 && !skip.contains(&p) && g.items.values().all(|it| it.pos != p) {
                        return p;
                    }
                }
            }
        }
        panic!("no free spot");
    }

    fn build(g: &mut Game, kind: BuildingKind, at: Pos) -> u32 {
        assert!(g.place_building(kind, at));
        let id = *g.buildings.keys().max().unwrap();
        for _ in 0..3 * DAY {
            g.step();
            if g.buildings[&id].built {
                return id;
            }
        }
        panic!("{:?} never got built (parts {})", kind, g.buildings[&id].parts);
    }

    #[test]
    fn weapon_and_cage_traps_work() {
        let mut g = Game::new(51);
        let e = g.embark;
        for _ in 0..2 {
            g.spawn_item(ItemKind::Mechanism, Mat::Granite, e);
        }
        g.spawn_item(ItemKind::Spear, Mat::Iron, e);
        g.spawn_item(ItemKind::Cage, Mat::Wood, e);
        let a = spot(&g, &[]);
        let wt = build(&mut g, BuildingKind::WeaponTrap, a);
        // Whatever weapon was closest: the wagon's spare axe or our spear.
        assert!(g.buildings[&wt].load.map_or(false, |l| l.0.weapon_damage().is_some()), "trap not loaded with a weapon");
        let b = spot(&g, &[a]);
        let ct = build(&mut g, BuildingKind::CageTrap, b);

        // A goblin walks onto the weapon trap.
        let gob = g.spawn_creature(Species::Goblin, a, u64::MAX);
        let blood = g.creatures[&gob].body.blood;
        for _ in 0..5 {
            g.traps_tick();
            g.tick += 1;
        }
        let hurt = g.creatures.get(&gob).map_or(true, |c| c.body.blood < blood || c.body.bleeding > 0 || !c.alive || c.body.badly_hurt());
        assert!(hurt, "the weapon trap never hurt the goblin");

        // A wolf steps into the cage trap and ends up in a cage.
        let wolf = g.spawn_creature(Species::Wolf, b, u64::MAX);
        g.traps_tick();
        assert!(!g.creatures.contains_key(&wolf), "wolf not caught");
        assert!(g.items.values().any(|it| it.kind == ItemKind::Cage && it.label.as_deref() == Some("wolf")));
        assert!(!g.buildings[&ct].built, "cage trap should need a new cage");
    }

    #[test]
    fn lever_raises_and_lowers_a_drawbridge() {
        let mut g = Game::new(52);
        let e = g.embark;
        for _ in 0..3 {
            g.spawn_item(ItemKind::Mechanism, Mat::Granite, e);
        }
        let a = spot(&g, &[]);
        let bridge = build(&mut g, BuildingKind::Bridge, a);
        let l = spot(&g, &[a]);
        let lever = build(&mut g, BuildingKind::Lever, l);
        assert!(g.link_lever(lever, bridge).is_ok());
        assert!(g.world.passable(a, Mover::Goblin));
        assert!(g.request_pull(lever));
        for _ in 0..DAY {
            g.step();
            if !g.buildings[&lever].pull {
                break;
            }
        }
        assert!(g.buildings[&bridge].raised, "nobody pulled the lever");
        assert!(!g.world.passable(a, Mover::Goblin) && !g.world.passable(a, Mover::Dwarf));
        g.pull_lever(lever);
        assert!(!g.buildings[&bridge].raised);
        assert!(g.world.passable(a, Mover::Goblin));
    }

    #[test]
    fn goblin_charging_down_a_trapped_corridor_gets_hurt() {
        let mut g = Game::new(53);
        let e = g.embark;
        let z = e.z;
        // A closed corridor east of the wagon: the only way out is west, over the trap.
        for x in 3..=16 {
            for dy in -1..=1 {
                let p = Pos::new(e.x + x, e.y + dy, z);
                let open = dy == 0 && x < 16;
                g.world.t_mut(p).kind = if open { TileKind::Floor } else { TileKind::Wall };
                g.world.t_mut(p).water = 0;
                g.world.t_mut(p.down()).kind = TileKind::Wall;
                g.world.t_mut(p.up()).kind = TileKind::Open;
            }
        }
        let trap_at = Pos::new(e.x + 9, e.y, z);
        assert!(g.place_building(BuildingKind::WeaponTrap, trap_at));
        let tid = *g.buildings.keys().max().unwrap();
        {
            let b = g.buildings.get_mut(&tid).unwrap();
            b.built = true;
            b.parts = 2;
            b.load = Some((ItemKind::Sword, Mat::Steel, 3));
        }
        let gob = g.spawn_creature(Species::Goblin, Pos::new(e.x + 15, e.y, z), u64::MAX);
        let start_blood = g.creatures[&gob].body.blood;
        let mut hurt = false;
        for _ in 0..DAY / 2 {
            g.step();
            match g.creatures.get(&gob) {
                None => {
                    hurt = true;
                    break;
                }
                Some(c) if !c.alive || c.body.blood < start_blood || c.body.bleeding > 0 => {
                    hurt = true;
                    break;
                }
                Some(c) if c.pos.x < trap_at.x => break,
                _ => {}
            }
        }
        assert!(hurt, "the goblin got past the weapon trap unharmed");
    }

    /// Stand a trap on `at` already built and loaded, as if a mechanic had.
    fn built(g: &mut Game, kind: BuildingKind, at: Pos, load: Option<(ItemKind, Mat, u8)>) -> u32 {
        assert!(g.place_building(kind, at), "can't place a {:?}", kind);
        let id = *g.buildings.keys().max().unwrap();
        let b = g.buildings.get_mut(&id).unwrap();
        b.built = true;
        b.parts = kind.parts();
        b.mat = Mat::Iron;
        b.load = load;
        id
    }

    fn hurt(g: &Game, cid: u32, blood: i32) -> bool {
        g.creatures.get(&cid).map_or(true, |c| !c.alive || c.body.blood < blood || c.body.bleeding > 0 || c.body.badly_hurt())
    }

    /// O04 (ENG-04, ENG-06): a goblin treading on the pressure plate fires
    /// the spikes it's linked to, running through the goblin behind it; a
    /// tame dog on the plate doesn't, and a lever fires them too.
    #[test]
    fn a_pressure_plate_fires_the_spikes() {
        let mut g = Game::new(55);
        g.creatures.retain(|_, c| c.tame);
        let a = spot(&g, &[]);
        let spikes = built(&mut g, BuildingKind::Spikes, a, Some((ItemKind::Spear, Mat::Iron, 0)));
        let b = spot(&g, &[a]);
        let plate = built(&mut g, BuildingKind::PressurePlate, b, Some((ItemKind::Mechanism, Mat::Granite, 0)));
        let mechs = g.items.values().filter(|it| it.kind == ItemKind::Mechanism).count();
        assert!(g.link_lever(plate, spikes).is_ok());
        assert_eq!(g.items.values().filter(|it| it.kind == ItemKind::Mechanism).count(), mechs, "a plate's link takes no mechanism");
        assert!(g.link_lever(plate, plate).is_err(), "plates only work spikes");
        // A goblin waits on the spikes: nothing happens while nobody treads the plate.
        let gob = g.spawn_creature(Species::Goblin, a, u64::MAX);
        let blood = g.creatures[&gob].body.blood;
        for _ in 0..5 {
            g.traps_tick();
            g.tick += 1;
        }
        assert!(!hurt(&g, gob, blood), "the spikes fired with nobody on the plate");
        // A tame dog on the plate is too light (or too friendly).
        let dog = g.spawn_creature(Species::Dog, b, u64::MAX);
        g.creatures.get_mut(&dog).unwrap().tame = true;
        g.traps_tick();
        assert!(!hurt(&g, gob, blood), "a tame dog set off the plate");
        g.creatures.remove(&dog);
        // A second goblin treads on it.
        let other = g.spawn_creature(Species::Goblin, b, u64::MAX);
        g.traps_tick();
        assert!(hurt(&g, gob, blood), "the plate never fired the spikes");
        assert!(g.creatures.contains_key(&other), "the goblin on the plate was taken");
        // A lever works them as well, once they've sunk back.
        g.creatures.remove(&other);
        let gob2 = g.spawn_creature(Species::Goblin, a, u64::MAX);
        let blood2 = g.creatures[&gob2].body.blood;
        g.creatures.retain(|&id, c| c.tame || id == gob2);
        let l = spot(&g, &[a, b]);
        let lever = built(&mut g, BuildingKind::Lever, l, None);
        g.spawn_item(ItemKind::Mechanism, Mat::Granite, g.embark);
        assert!(g.link_lever(lever, spikes).is_ok());
        g.tick += 100;
        g.pull_lever(lever);
        assert!(hurt(&g, gob2, blood2), "the lever never fired the spikes");
    }

    /// ENG-01: a weapon trap with two bodies heaped on it won't fire until
    /// they are carried off; a tripwire throws a goblin down (ENG-08).
    #[test]
    fn a_jammed_trap_does_not_fire_until_cleared() {
        let mut g = Game::new(56);
        g.creatures.retain(|_, c| c.tame);
        let a = spot(&g, &[]);
        let wt = built(&mut g, BuildingKind::WeaponTrap, a, Some((ItemKind::Sword, Mat::Steel, 3)));
        let bodies: Vec<u32> = (0..2).map(|_| g.spawn_item(ItemKind::Corpse, Mat::Flesh, a)).collect();
        let gob = g.spawn_creature(Species::Goblin, a, u64::MAX);
        let blood = g.creatures[&gob].body.blood;
        for _ in 0..40 {
            g.traps_tick();
            g.tick += 1;
        }
        assert!(!hurt(&g, gob, blood), "a jammed trap fired");
        assert!(g.log.iter().any(|l| l.text.contains("jammed with bodies")), "nobody said it was jammed");
        for id in bodies {
            g.items.remove(&id);
        }
        g.tick += 40;
        g.traps_tick();
        assert!(hurt(&g, gob, blood), "the cleared trap didn't fire");
        assert_eq!(g.buildings[&wt].timer, 0, "still marked jammed");
        // A tripwire lays a goblin on the floor a while.
        let b = spot(&g, &[a]);
        built(&mut g, BuildingKind::Tripwire, b, Some((ItemKind::Thread, Mat::Flesh, 0)));
        let gob2 = g.spawn_creature(Species::Goblin, b, u64::MAX);
        g.traps_tick();
        assert!(g.creatures[&gob2].move_cd >= TRIPPED, "the goblin didn't trip");
    }

    /// O04: a goblin tripped on a wire goes headlong onto the spikes ahead,
    /// which the wire springs; one a trap wounded counts in the siege's toll
    /// when it dies later, wherever it falls, and one no trap touched doesn't.
    #[test]
    fn a_tripwire_throws_a_goblin_onto_the_spikes() {
        let mut g = Game::new(58);
        g.creatures.retain(|_, c| c.tame);
        let mut skip = Vec::new();
        let (a, b) = loop {
            let a = spot(&g, &skip);
            let b = a.offset(1, 0, 0);
            let t = g.world.t(b);
            if t.kind == TileKind::Floor && t.water == 0 && t.building == 0 && t.zone == 0 && g.world.walkable(b) {
                break (a, b);
            }
            skip.push(a);
        };
        let wire = built(&mut g, BuildingKind::Tripwire, a, Some((ItemKind::Thread, Mat::Flesh, 0)));
        let spikes = built(&mut g, BuildingKind::Spikes, b, Some((ItemKind::Bolt, Mat::Wood, 0)));
        assert!(g.link_lever(wire, spikes).is_ok(), "a wire can't be tied to the spikes");
        g.annals.sieges.push(crate::game::annals::SiegeNote { foe: "goblins".into(), size: 6, ..Default::default() });
        let mut wounded = None;
        for _ in 0..20 {
            let gob = g.spawn_creature(Species::Goblin, a, u64::MAX);
            let blood = g.creatures[&gob].body.blood;
            {
                let c = g.creatures.get_mut(&gob).unwrap();
                c.siege = true;
                c.path = vec![b];
            }
            g.tick += 100;
            g.traps_tick();
            assert!(g.creatures.get(&gob).map_or(true, |c| c.pos == b), "the goblin didn't go headlong");
            if g.creatures.get(&gob).map_or(false, |c| c.alive) && hurt(&g, gob, blood) {
                wounded = Some(gob);
                break;
            }
            g.creatures.retain(|_, c| c.tame);
        }
        assert!(g.log.iter().any(|l| l.text.contains("headlong onto the spikes")), "nobody saw it go onto the spikes");
        let gob = wounded.expect("twenty goblins thrown on the spikes and none left wounded");
        let s = g.annals.sieges.last().unwrap();
        assert!(s.trap_hurt.contains(&gob), "the wounded goblin isn't marked");
        let before = s.trapped;
        // It bleeds out far from the hall: still the trap's.
        let mut c = g.creatures.remove(&gob).unwrap();
        c.pos = g.embark;
        g.creature_died(c);
        let s = g.annals.sieges.last().unwrap();
        assert_eq!(s.trapped, before + 1, "the bleed-out wasn't the trap's");
        assert!(s.trap_hurt.is_empty());
        // One no trap touched isn't.
        let other = g.spawn_creature(Species::Goblin, g.embark, u64::MAX);
        let mut c = g.creatures.remove(&other).unwrap();
        c.siege = true;
        g.creature_died(c);
        assert_eq!(g.annals.sieges.last().unwrap().trapped, before + 1);
    }

    /// ENG-19: a stone-fall trap drops its stone three times before it is
    /// spent and needs another.
    #[test]
    fn a_stone_fall_trap_drops_three_times() {
        let mut g = Game::new(57);
        g.creatures.retain(|_, c| c.tame);
        let a = spot(&g, &[]);
        let st = built(&mut g, BuildingKind::StoneTrap, a, Some((ItemKind::Boulder, Mat::Granite, 0)));
        for drop in 1..=3 {
            assert!(g.buildings[&st].built, "spent after {} drops", drop - 1);
            // A fresh goblin on it each time (and the last one's body cleared,
            // or the trap jams).
            g.creatures.retain(|_, c| c.tame);
            g.items.retain(|_, it| it.pos != a);
            g.spawn_creature(Species::Goblin, a, u64::MAX);
            g.tick += 100;
            g.traps_tick();
        }
        assert!(!g.buildings[&st].built, "the stone should be spent after three drops");
    }

    #[test]
    fn mechanics_make_mechanisms() {
        let mut g = Game::new(54);
        let e = g.embark;
        let site = (0..60)
            .map(|i| e.offset(-10 + i % 12, -12 + i / 12 * 4, 0))
            .find(|&p| g.can_place(BuildingKind::Mechanic, p))
            .expect("no room for a workshop");
        assert!(g.place_building(BuildingKind::Mechanic, site));
        let shop = *g.buildings.keys().max().unwrap();
        g.buildings.get_mut(&shop).unwrap().built = true;
        for _ in 0..3 {
            g.spawn_item(ItemKind::Boulder, Mat::Granite, e);
        }
        let id = g.dwarves[0].id;
        g.set_labor(id, Labor::Mechanics, true);
        g.add_order(shop, crate::buildings::Recipe::Mechanism);
        for _ in 0..DAY {
            g.step();
        }
        assert!(g.items.values().any(|it| it.kind == ItemKind::Mechanism), "no mechanism made");
    }
}
