//! The pit bridge (O04 stage 2; ENG-03, ENG-07, ENG-20): past the trap
//! hall the overseer lays a two-wide drawbridge on the corridor floor and
//! then has the miners cut a pit two levels deep under it, so the way is
//! never broken. Lowered, the bridge's deck is the floor; raised, there is
//! nothing under whoever stood on it, and they fall. Its own lever stands
//! behind it, where the squads hold, and is thrown when the enemy crowds
//! onto the bridge: "The bridge rose beneath them: four goblins fell into
//! the Belly." Those that live through the fall are stuck down there.
//!
//! Any drawbridge works so: channel under a built bridge and the pit is cut
//! beneath its deck (`channel_under_bridge`), and the bridge drops whatever
//! stands on it like a trapdoor when it is raised (`traps.rs: pull_lever`).
//!
//! The pond: once the pit is cut the overseer marks it, and haulers carry
//! water to it from the well bucket by bucket (`ponds.rs`), until whatever
//! falls in drowns.

use super::overseer::{Overseer, Style};
use super::personality::{NoteKind, Trait};
use super::*;
use serde::{Deserialize, Serialize};

/// The pit's columns, local x from the foot of the stair, on both rows of
/// the corridor: past the trap hall's weapon traps (x+11), where the
/// stone-fall traps were, before the squads' line at x+14.
pub(super) const PIT: [i32; 2] = [12, 13];
/// How deep the pit is cut under a bridge.
pub const PIT_DEPTH: i32 = 2;
/// Water this deep (of 7) drowns what can't swim.
pub const DROWN_DEPTH: u8 = 4;
/// Ticks a creature lies stunned after a fall.
const STUNNED: u32 = 60;

/// Where the pit's lever may go, best first: beside the squads' line at
/// x+14, on the fort side.
/// (Else on the squads' line itself.)
const PIT_LEVER: [(i32, i32); 10] = [(15, 0), (15, 1), (16, 0), (16, 1), (17, 0), (17, 1), (18, 0), (18, 1), (14, 0), (14, 1)];

/// Names for the pit: the trap hall is the mouth, this is the belly.
const PIT_NAMES: [&str; 10] = [
    "the Belly",
    "the Gut",
    "the Long Drop",
    "the Swallow",
    "the Deep Throat",
    "the Craw",
    "the Gulf",
    "the Black Gullet",
    "the Pit of Bones",
    "the Hungry Dark",
];

/// The fort's pit bridge (`Game.pits`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Pits {
    /// The deck tiles of the gatehouse's pit bridge.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decks: Vec<Pos>,
    /// Where its lever stands.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lever: Option<Pos>,
    /// What the fort calls it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Foes alive in the pit: (creature, the siege they came with).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prisoners: Vec<(u32, usize)>,
    /// This year's falls, for the chronicle (drained when it is written).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub told: Vec<String>,
    /// Every foe that has fallen, and drowned, in it.
    #[serde(default)]
    pub fell: u32,
    #[serde(default)]
    pub drowned: u32,
    /// The pit is to be a pond (`ponds.rs`), and has been filled.
    #[serde(default)]
    pub pond: bool,
    #[serde(default)]
    pub filled: bool,
    /// How deep the gatehouse's pit is cut (0: `PIT_DEPTH`, from old saves).
    #[serde(default)]
    pub depth: i32,
}

/// A count in words ("Four goblins fell").
pub(super) fn count_words(n: usize) -> String {
    const W: [&str; 13] = ["no", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "eleven", "twelve"];
    W.get(n).map_or(n.to_string(), |w| w.to_string())
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

impl Game {
    /// Whether the bridge at `p` stands over a pit: nothing under its deck.
    pub fn over_pit(&self, p: Pos) -> bool {
        match self.world.kind(p.down()) {
            Some(TileKind::Open) => true,
            // (A shallow pit, a level deep: its dug floor right under the deck.)
            Some(TileKind::Floor) => self.pits.depth == 1 && self.pits.decks.contains(&p) && self.world.t(p.down()).dug,
            _ => false,
        }
    }

    /// The name the pit takes, the trap hall's belly (from hashes).
    pub(super) fn pit_name_for(&self, base: Pos) -> &'static str {
        PIT_NAMES[(super::names::hash(&[super::stories::hash(&self.fortress_name, 0), base.x as u64, base.y as u64, 77]) % PIT_NAMES.len() as u64) as usize]
    }

    /// The pit's name, or what to call it.
    pub(super) fn pit_name(&self) -> String {
        self.pits.name.clone().unwrap_or_else(|| "the pit".into())
    }

    /// A miner has channelled the tile under a drawbridge (`complete_dig`):
    /// the pit is cut beneath its deck, up to `PIT_DEPTH` levels through
    /// rock, and the deck stays a floor while the bridge is down. Where the
    /// rubble lands, or None if there is no bridge here.
    pub(super) fn channel_under_bridge(&mut self, q: Pos) -> Option<Pos> {
        let id = self.world.t(q).building;
        let raised = self.buildings.get(&id).filter(|b| b.kind == BuildingKind::Bridge)?.raised;
        let mut bottom = q;
        // (The gatehouse's pit may be a shallow one: pit_depth_at.)
        let depth = if self.pits.depth > 0 && self.pits.decks.contains(&q) { self.pits.depth } else { PIT_DEPTH };
        for k in 1..=depth {
            let r = q.offset(0, 0, -k);
            if !self.world.get(r).map_or(false, |t| t.kind == TileKind::Wall && t.mat.diggable() && t.water == 0) {
                break;
            }
            if bottom != q {
                self.world.t_mut(bottom).kind = TileKind::Open;
            }
            let t = self.world.t_mut(r);
            t.kind = TileKind::Floor;
            t.dug = true;
            t.revealed = true;
            bottom = r;
        }
        self.world.t_mut(q).kind = if raised { TileKind::Open } else { TileKind::Floor };
        Some(bottom)
    }

    /// Stage 2 of the gatehouse (O04): due once the trap hall stands and the
    /// fort is rich or has seen two sieges; a warlike or cautious overseer
    /// from day 120.
    pub(super) fn pit_due(&self, o: &Overseer) -> bool {
        if o.style == Style::Reckless || o.gate_stage < 1 || self.knob("gate.pit", 1.0) <= 0.0 {
            return false;
        }
        let eager = o.style == Style::Warlord || o.has(Trait::Cautious);
        let day = self.knob("gate.pit_day", 120.0) as u64;
        (eager && self.tick >= day * DAY) || self.annals.sieges.len() >= 2 || self.wealth() as f64 >= self.knob("gate.pit_wealth", 8000.0)
    }

    /// The pit's ground is sound: the corridor floor is there to lay the
    /// bridge on, and under it and all round, two levels down, is dry rock,
    /// so the pit opens into no cave and lets nothing out.
    fn pit_ground(&self, base: Pos) -> bool {
        self.pit_depth_at(base).is_some()
    }

    /// How deep a pit the rock past the trap hall takes: two levels where it
    /// is sound all round that deep, else one where the first level is (and
    /// nothing wet lies under it). None: hollow or wet.
    fn pit_depth_at(&self, base: Pos) -> Option<i32> {
        let w = &self.world;
        let sound = |depth: i32| {
            (1..=depth).all(|dz| {
                (PIT[0] - 1..=PIT[1] + 1).all(|dx| {
                    (-1..=2).all(|dy| w.get(base.offset(dx, dy, -dz)).map_or(false, |t| t.kind == TileKind::Wall && t.mat.diggable() && t.water == 0 && !t.aquifer))
                })
            })
        };
        let dry_under = PIT.iter().all(|&dx| (0..2).all(|dy| w.get(base.offset(dx, dy, -2)).map_or(true, |t| t.water == 0 && !t.aquifer)));
        let depth = if sound(PIT_DEPTH) {
            PIT_DEPTH
        } else if self.knob("pit.shallow", 1.0) > 0.0 && sound(1) && dry_under {
            1
        } else {
            return None;
        };
        // (Nothing on the floor but traps, which can be taken up.)
        let free = |p: Pos| self.building_at(p).map_or(true, |k| k.is_trap() || k == BuildingKind::Bridge);
        let floor = PIT.iter().all(|&dx| (0..2).all(|dy| {
            let p = base.offset(dx, dy, 0);
            w.get(p).map_or(false, |t| t.kind == TileKind::Floor && t.water == 0) && free(p)
        }));
        floor.then_some(depth)
    }

    /// Daily (from `ov_defences`): begin the pit bridge when it is due, and
    /// keep it: the bridge laid on the corridor floor first, the pit cut
    /// under it once it stands, then the lever behind, linked.
    pub(super) fn ov_pit(&mut self, o: &mut Overseer) {
        let Some(base) = o.base else { return };
        if o.gate_stage == 1 {
            if !self.pit_due(o) || o.did("nopit") {
                return;
            }
            self.count("m2.pit_due_days", 1);
            if !self.pit_ground(base) {
                // (Why, for the batch: a tile dug out, a cave or water, other rock.)
                let mut why = "m2.pit_rock_other";
                for dz in 1..=PIT_DEPTH {
                    for dx in PIT[0] - 1..=PIT[1] + 1 {
                        for dy in -1..=2 {
                            if let Some(t) = self.world.get(base.offset(dx, dy, -dz)) {
                                if t.water > 0 || t.aquifer {
                                    why = "m2.pit_rock_wet";
                                } else if t.kind != TileKind::Wall && t.dug {
                                    why = "m2.pit_rock_dug";
                                } else if t.kind != TileKind::Wall {
                                    why = "m2.pit_rock_cave";
                                }
                            }
                        }
                    }
                }
                self.count(why, 1);
                o.done.push("nopit".into());
                Self::note_as(o, self.tick, NoteKind::Gate, "Sounded the rock past the trap hall for a pit: it is hollow or wet down there. No pit.".into());
                return;
            }
            // A mechanism for each half of the bridge, the lever and each
            // link; and a log or blocks for each deck.
            let need = self.knob("gate.pit_mechs", 9.0) as usize;
            if self.stock_of(ItemKind::Mechanism) < need {
                self.count("m2.pit_wait_mechs", 1);
                let until = self.tick + SEASON;
                o.boosts.retain(|b| b.0 != crate::buildings::Recipe::Mechanism);
                o.boosts.push((crate::buildings::Recipe::Mechanism, need as u32 + 2, until));
                return;
            }
            if self.stock_of(ItemKind::Log) + self.stock_of(ItemKind::Block) < 4 || self.alert {
                self.count(if self.alert { "m2.pit_wait_alert" } else { "m2.pit_wait_wood" }, 1);
                return;
            }
            o.gate_stage = 2;
            o.done.push("pit".into());
            self.pits.depth = self.pit_depth_at(base).unwrap_or(PIT_DEPTH);
            if self.pits.depth < PIT_DEPTH {
                self.count("m2.pit_shallow", 1);
            }
            self.pits.decks = PIT.iter().flat_map(|&dx| (0..2).map(move |dy| base.offset(dx, dy, 0))).collect();
            let name = self.pit_name_for(base);
            self.pits.name = Some(name.to_string());
            // (Named through `deed_at`, so the legends and the chronicle tell it: defences.rs.)
            self.name_defence(o, super::defences::Defence::Pit);
            let hall = self.trap_hall_name().unwrap_or_else(|| "the trap hall".into());
            Self::note_as(
                o,
                self.tick,
                NoteKind::Gate,
                format!("Began a drawbridge past {}, and a pit {} deep to dig under it, on its own lever. They will call the pit {}.", hall, if self.pits.depth >= 2 { "two levels" } else { "a level" }, name),
            );
        }
        if o.gate_stage < 2 || self.pits.decks.is_empty() {
            return;
        }
        let decks = self.pits.decks.clone();
        // The bridge first, on the corridor floor (whatever trap stood
        // there taken up): the way is never broken.
        let mut standing = 0;
        for &p in &decks {
            match self.building_at(p) {
                Some(BuildingKind::Bridge) => {
                    let built = self.buildings.get(&self.world.t(p).building).map_or(false, |b| b.built);
                    standing += built as usize;
                    // Built and not yet dug under: the miners cut the pit.
                    if built && !self.over_pit(p) && self.world.kind(p) == Some(TileKind::Floor) && self.world.t(p).desig == Desig::None && !self.alert {
                        self.designate(p, p, Desig::Channel);
                    }
                }
                Some(k) if k.is_trap() && !self.alert => {
                    let id = self.world.t(p).building;
                    self.remove_building(id);
                }
                Some(_) => {}
                None => {
                    if self.world.kind(p) == Some(TileKind::Floor) && self.stock_of(ItemKind::Mechanism) >= 1 {
                        self.place_building(BuildingKind::Bridge, p);
                    } else if self.world.kind(p) == Some(TileKind::Floor) && self.over_pit(p) {
                        // A deck with no bridge left under it (broken):
                        // it is open air.
                        self.world.t_mut(p).kind = TileKind::Open;
                        self.drop_through(&[p]);
                    }
                }
            }
        }
        // Once it's cut, a pond in it, if there's a well to fill it from.
        if !self.pits.pond && decks.iter().all(|&p| self.over_pit(p)) && !self.working_wells().is_empty() && self.stock_of(ItemKind::Bucket) >= 1 {
            self.pits.pond = true;
            let name = self.pit_name();
            Self::note_as(o, self.tick, NoteKind::Gate, format!("Marked {} for a pond: the haulers are to fill it from the well.", name));
        }
        // The lever behind it, once the bridge stands, linked to each half.
        if standing < decks.len() {
            return;
        }
        let lever = self.pits.lever.and_then(|lp| self.buildings.values().find(|b| b.kind == BuildingKind::Lever && b.pos == lp).map(|b| (b.id, b.built)));
        match lever {
            None => {
                if self.stock_of(ItemKind::Mechanism) >= 1 {
                    if let Some(p) = PIT_LEVER.iter().map(|&(dx, dy)| base.offset(dx, dy, 0)).find(|&p| self.can_place(BuildingKind::Lever, p)) {
                        self.place_building(BuildingKind::Lever, p);
                        self.pits.lever = Some(p);
                    }
                }
            }
            Some((id, true)) => {
                let bridges: Vec<u32> = decks.iter().map(|&p| self.world.t(p).building).filter(|b| !self.buildings[&id].links.contains(b)).collect();
                for b in bridges {
                    let _ = self.link_lever(id, b);
                }
                if self.pit_lever().is_some() && !o.did("pitlever") {
                    o.done.push("pitlever".into());
                    let name = self.pit_name();
                    Self::note_as(o, self.tick, NoteKind::Gate, format!("Linked a lever to the bridge over {}: let them crowd onto it, then pull.", name));
                }
            }
            Some(_) => {}
        }
    }

    /// At the end of a trial (`m2_stats`): how far the pit got, or why not.
    pub(super) fn pit_stats(&mut self) {
        let Some(o) = self.overseer.as_ref() else { return };
        let (nopit, due, stage) = (o.did("nopit"), self.pit_due(o), o.gate_stage);
        if nopit {
            self.count("m2.pit_nopit", 1);
        }
        if stage == 1 && !nopit && !due {
            self.count("m2.pit_notdue", 1);
        }
        if stage >= 2 && !nopit {
            let dug = !self.pits.decks.is_empty() && self.pits.decks.iter().all(|&p| self.over_pit(p));
            self.count("m2.pit_dug", dug as u32);
            self.count("m2.pit_ready", self.pit_lever().is_some() as u32);
        }
    }

    /// The pit's lever, built and linked to every half of its bridge.
    pub(super) fn pit_lever(&self) -> Option<u32> {
        let lp = self.pits.lever?;
        let l = self.buildings.values().find(|b| b.kind == BuildingKind::Lever && b.pos == lp && b.built)?;
        let all = !self.pits.decks.is_empty() && self.pits.decks.iter().all(|&p| l.links.contains(&self.world.t(p).building) && self.world.t(p).building != 0);
        all.then_some(l.id)
    }

    /// Every ten ticks (`ov_alert_hooks`): throw the pit's lever when the
    /// enemy crowds onto its bridge (never with one of ours on it), and
    /// lower it again once none are left on the gate's side.
    pub(super) fn ov_pit_alert(&mut self, o: &mut Overseer) {
        let Some(lever) = self.pit_lever() else { return };
        let Some(base) = o.base else { return };
        let decks = &self.pits.decks;
        let raised = decks.iter().any(|&p| self.buildings.get(&self.world.t(p).building).map_or(false, |b| b.raised));
        let on_deck = self.creatures.values().filter(|c| c.alive && c.hostile() && decks.contains(&c.pos)).count();
        let ours = self.dwarves.iter().any(|d| d.alive && decks.contains(&d.pos));
        // Foes on the gate's side of the pit, in the corridor.
        let gate_side = self
            .creatures
            .values()
            .filter(|c| c.alive && c.hostile() && !c.leaving && c.pos.z == base.z && (-1..PIT[0]).contains(&(c.pos.x - base.x)) && (-1..=2).contains(&(c.pos.y - base.y)))
            .count();
        let army = self.creatures.values().any(|c| c.alive && c.siege && c.hostile() && !c.leaving);
        let gate_up = self.gate_bridges(base).iter().any(|id| self.buildings.get(id).map_or(false, |b| b.raised));
        let safe = self.knob("pit.safe", 1.0) > 0.0;
        let want = if raised && safe {
            // Kept up while the alert lasts and they're at it; down again
            // when the way is clear, or once the alert is over (a beast or
            // a straggler left in the corridor mustn't keep the fort cut off
            // from its gate, its fields and its water).
            self.alert && (gate_side > 0 || (army && !gate_up))
        } else if raised {
            gate_side > 0 || (army && !gate_up && self.alert)
        } else {
            let min = self.knob("pit.crowd", 2.0) as usize;
            on_deck >= min || (on_deck >= 1 && gate_side == 0)
        };
        let want = if !raised && ours { false } else { want };
        if want != raised && std::env::var("TRIAL_PIT").is_ok() {
            eprintln!("PIT day {} tick {}: raised {} -> want {} | on deck {} gate side {} army {} gate up {} alert {} ours {}", self.tick / DAY, self.tick, raised, want, on_deck, gate_side, army, gate_up, self.alert, ours);
        }
        if raised && self.tick % DAY < 10 && std::env::var("TRIAL_PIT").is_ok() {
            eprintln!("PIT day {}: still raised | gate side {} army {} alert {}", self.tick / DAY, gate_side, army, self.alert);
        }
        if on_deck > 0 && !raised {
            self.count("m2.pit_foe_on_deck", 1);
        }
        let lp = self.buildings[&lever].pos;
        if want == raised {
            self.buildings.get_mut(&lever).unwrap().pull = false;
            return;
        }
        let by = self.dwarves.iter().any(|d| d.alive && d.pos.z == lp.z && (d.pos.x - lp.x).abs() <= 1 && (d.pos.y - lp.y).abs() <= 1);
        if by {
            self.pull_lever(lever);
        } else if !self.buildings[&lever].pull {
            self.request_pull(lever);
        }
    }

    /// Every ten ticks: one of ours who lived through a fall into the pit
    /// climbs out once the stun wears off, onto the corridor on the fort's
    /// side (a dwarf can climb what a goblin can't; else they'd die of
    /// thirst down there).
    fn climb_out(&mut self) {
        if self.pits.decks.is_empty() || self.knob("pit.safe", 1.0) <= 0.0 {
            return;
        }
        let decks = self.pits.decks.clone();
        // (Only the pit's own floor: a hall further down under it isn't the pit.)
        let depth = if self.pits.depth > 0 { self.pits.depth } else { PIT_DEPTH };
        let in_pit = |p: Pos| decks.iter().any(|d| d.x == p.x && d.y == p.y && p.z < d.z && p.z >= d.z - depth);
        let out: Option<Pos> = decks
            .iter()
            .flat_map(|d| crate::geom::DIRS4.iter().map(move |&(dx, dy)| d.offset(dx, dy, 0)))
            .filter(|p| !decks.contains(p) && self.world.walkable(*p))
            .max_by_key(|p| (p.x, p.y));
        let Some(out) = out else { return };
        for i in 0..self.dwarves.len() {
            let d = &self.dwarves[i];
            if !d.alive || d.move_cd > 0 || !in_pit(d.pos) {
                continue;
            }
            let name = d.name.clone();
            let d = &mut self.dwarves[i];
            d.pos = out;
            d.path.clear();
            let pit = self.pit_name();
            self.msg_at(&format!("{} climbed out of {}.", cap(&name), pit), (200, 200, 140), Some(out), super::logkind::LogKind::Job);
            self.count("m2.pit_climbed", 1);
        }
    }

    /// The decks at `at` have just gone from under whoever stood on them
    /// (a pit bridge raised): they fall, and what happened is told.
    pub(super) fn drop_through(&mut self, at: &[Pos]) {
        let mut fallen: Vec<(String, bool, bool)> = Vec::new(); // (plural or name, foe, died)
        let mut foes = 0;
        let mut dead_foes = 0;
        let mut plural = String::new();
        let mut where_ = None;
        for &p in at {
            self.drop_items_at(p);
            let cids: Vec<u32> = self.creatures.values().filter(|c| c.alive && c.pos == p).map(|c| c.id).collect();
            for cid in cids {
                let foe = self.creatures.get(&cid).map_or(false, |c| c.hostile());
                if foe && plural.is_empty() {
                    plural = self.creatures[&cid].species.info().plural.to_string();
                }
                let name = self.creatures.get(&cid).map_or(String::new(), |c| c.title());
                let died = self.fall_creature(cid, p);
                if foe {
                    foes += 1;
                    dead_foes += died as usize;
                    where_ = Some(p);
                }
                fallen.push((name, foe, died));
            }
            let dids: Vec<usize> = (0..self.dwarves.len()).filter(|&i| self.dwarves[i].alive && self.dwarves[i].pos == p).collect();
            for i in dids {
                let name = self.dwarves[i].name.clone();
                self.count("m2.pit_dwarf_fell", 1);
                if std::env::var("TRIAL_PIT").is_ok() {
                    eprintln!("PIT day {}: {} fell", self.tick / DAY, name);
                }
                let died = self.fall_dwarf(i, p);
                fallen.push((name, false, died));
            }
        }
        if fallen.is_empty() {
            return;
        }
        let pit = self.pit_name();
        let at = where_.or(at.first().copied());
        if foes > 0 {
            let who = if foes == 1 { fallen.iter().find(|f| f.1).map_or("a foe".into(), |f| f.0.clone()) } else { format!("{} {}", count_words(foes), plural) };
            let text = if foes == 1 { format!("The bridge rose beneath {}: down into {} it went.", who, pit) } else { format!("The bridge rose beneath them: {} fell into {}.", who, pit) };
            let more = match dead_foes {
                0 => String::new(),
                n if n == foes && n == 1 => " It broke on the stones below.".into(),
                n if n == foes => " Not one of them got up.".into(),
                n => format!(" {} broke on the stones below.", cap(&count_words(n))),
            };
            let told = format!("{}{}", text, more);
            self.msg_at(&told, (240, 140, 90), at, super::logkind::LogKind::Siege);
            self.spotlight(&cap(&pit), &told, at, (240, 140, 90));
            self.record(super::memory::MemoryKind::Other, &[], at, told.clone(), if foes >= 3 { 5 } else { 3 });
            self.pits.told.push(told);
            self.count("m2.pit_drops", 1);
        }
        for (name, foe, died) in fallen.iter().filter(|f| !f.1) {
            let _ = foe;
            let text = if *died { format!("{} fell into {} and was killed.", cap(name), pit) } else { format!("{} fell into {}.", cap(name), pit) };
            self.msg_at(&text, (255, 120, 100), at, super::logkind::LogKind::Combat);
        }
        self.dig_version += 1;
    }

    /// Where something falling from `p` comes down, and how far it fell.
    fn landing(&self, p: Pos) -> (Pos, i32) {
        let to = self.world.settle(p);
        (to, p.z - to.z)
    }

    /// A blow from the fall itself: one per level fallen, unless it fell
    /// into deep water.
    fn fall_blow(&self, levels: i32) -> super::combat::Fighter {
        super::combat::Fighter {
            name: "the fall".into(),
            skill: 30,
            dmg: self.knob("pit.fall_dmg", 11.0) as f32 * (1.0 + 0.25 * (levels - 1).max(0) as f32),
            verb: "smashes",
            edged: false,
            helm: 0,
            mail: 0,
            shield: false,
            size: 2.0,
            teeth: false,
            weapon: None,
            held: None,
        }
    }

    /// Creature `cid` falls from `p`: down, stunned, one blow per level
    /// (water breaks the fall). A foe of a siege counts toward its toll,
    /// and one that lives is left in the pit. Whether it died.
    fn fall_creature(&mut self, cid: u32, p: Pos) -> bool {
        let (to, levels) = self.landing(p);
        let wet = self.world.t(to).water >= DROWN_DEPTH;
        let Some(c) = self.creatures.get_mut(&cid) else { return false };
        c.pos = to;
        c.path.clear();
        c.move_cd = c.move_cd.max(STUNNED);
        c.attack_cd = c.attack_cd.max(STUNNED);
        let foe = c.siege && !c.legendary();
        if foe {
            if let Some(s) = self.annals.sieges.iter_mut().rev().find(|s| s.ended.is_none()) {
                s.fell += 1;
            }
            self.pits.fell += 1;
            self.count("m2.pit_fell", 1);
        }
        let blows = if wet { 0 } else { levels };
        let att = self.fall_blow(levels);
        for _ in 0..blows {
            let Some(c) = self.creatures.get(&cid) else { break };
            let mut def = self.creature_fighter(c);
            def.shield = false;
            let mut body = c.body.clone();
            let (owner, seen, csp) = (super::gore::owner_of_creature(c), super::melee::a_or_name(c), c.species);
            let blow = self.strike(&att, &def, &mut body);
            if let Some(m) = blow.msg.as_ref().filter(|_| blow.notable) {
                self.msg_at(m, (235, 200, 130), Some(to), super::logkind::LogKind::Combat);
            }
            self.aftermath(&blow, to, to, &owner, &seen, Some(csp), false);
            let dead = body.dead;
            if let Some(c) = self.creatures.get_mut(&cid) {
                c.body = body;
                c.pos = to;
            }
            if dead {
                if let Some(c) = self.creatures.remove(&cid) {
                    if foe {
                        self.count("m2.pit_kills", 1);
                    }
                    self.creature_died(c);
                }
                return true;
            }
        }
        // Alive at the bottom: no way out. It is no longer part of the
        // siege (which can end without it), but its death will be.
        if foe {
            let i = self.annals.sieges.iter().rposition(|s| s.ended.is_none()).unwrap_or(self.annals.sieges.len().saturating_sub(1));
            if let Some(c) = self.creatures.get_mut(&cid) {
                c.siege = false;
            }
            self.pits.prisoners.push((cid, i));
        }
        false
    }

    /// Dwarf `i` falls from `p`. Whether they died.
    fn fall_dwarf(&mut self, i: usize, p: Pos) -> bool {
        let (to, levels) = self.landing(p);
        let wet = self.world.t(to).water >= DROWN_DEPTH;
        let mut d = std::mem::take(&mut self.dwarves[i]);
        d.pos = to;
        d.path.clear();
        self.release_task(&mut d);
        d.move_cd = d.move_cd.max(STUNNED);
        let att = self.fall_blow(levels);
        let mut died = false;
        for _ in 0..if wet { 0 } else { levels } {
            let mut def = self.dwarf_fighter(&d);
            def.shield = false;
            let blow = self.strike(&att, &def, &mut d.body);
            if let Some(m) = blow.msg.as_ref().filter(|_| blow.notable) {
                self.msg_at(m, (255, 120, 100), Some(to), super::logkind::LogKind::Combat);
            }
            if d.body.dead {
                let cause = format!("fell to their death in {}", self.pit_name());
                self.kill(&mut d, &cause);
                died = true;
                break;
            }
        }
        if !died {
            d.think("fell into a pit", 8, self.tick);
        }
        self.dwarves[i] = d;
        died
    }

    /// Every ten ticks (`traps_tick`): whatever can't swim drowns in deep
    /// water (the pond), as dwarves do; a foe of a siege counts toward it.
    pub(super) fn drown_tick(&mut self) {
        self.climb_out();
        let wet: Vec<u32> = self
            .creatures
            .values()
            .filter(|c| c.alive && !c.tame && self.world.get(c.pos).map_or(false, |t| t.water >= DROWN_DEPTH && !t.frozen))
            .filter(|c| !c.species.beast().map_or(false, |b| b.can(crate::bestiary::SWIM) || b.flier()))
            .map(|c| c.id)
            .collect();
        let limit = self.knob("pit.drown_ticks", 300.0) as u32;
        for cid in wet {
            let Some(c) = self.creatures.get_mut(&cid) else { continue };
            c.drown += 10;
            if c.drown < limit {
                continue;
            }
            let Some(c) = self.creatures.remove(&cid) else { continue };
            let at = c.pos;
            let pit = self.pits.decks.iter().any(|d| d.x == at.x && d.y == at.y);
            let siege = self.pits.prisoners.iter().position(|p| p.0 == cid);
            let foe = siege.is_some() || (c.siege && !c.legendary());
            let name = cap(&c.title());
            let place = if pit { self.pit_name() } else { "the water".into() };
            self.msg_at(&format!("{} has drowned in {}.", name, place), (120, 170, 240), Some(at), super::logkind::LogKind::Siege);
            if foe {
                let i = siege.map(|k| self.pits.prisoners.swap_remove(k).1);
                let s = match i {
                    Some(i) => self.annals.sieges.get_mut(i),
                    None => self.annals.sieges.iter_mut().rev().find(|s| s.ended.is_none()),
                };
                if let Some(s) = s {
                    s.drowned += 1;
                    if s.ended.is_some() {
                        s.foes_slain += 1;
                    }
                }
                self.pits.drowned += 1;
                self.count("m2.pit_drowned", 1);
                if pit {
                    self.pits.told.push(format!("{} drowned in {}.", name, place));
                }
            }
            self.creature_died(c);
        }
        // Prisoners that died some other way are let go of.
        let alive: Vec<u32> = self.pits.prisoners.iter().map(|p| p.0).filter(|id| self.creatures.get(id).map_or(false, |c| c.alive)).collect();
        self.pits.prisoners.retain(|p| alive.contains(&p.0));
    }

    /// The chronicle's lines for the pit this year (drained).
    pub(super) fn lines_pit(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pits.told).into_iter().take(4).collect()
    }

    /// A siege is over (`on_siege_end`): how many fell into the pit, told.
    pub(super) fn pit_toll(&mut self, i: usize) {
        let Some(s) = self.annals.sieges.get(i) else { return };
        let (fell, foe) = (s.fell, s.foe.clone());
        if fell == 0 {
            return;
        }
        let left = self.pits.prisoners.iter().filter(|p| p.1 == i).count();
        let pit = self.pit_name();
        let text = if left > 0 {
            format!("{} {} fell into {}; {} still prowl the bottom of it.", cap(&count_words(fell as usize)), foe, pit, count_words(left))
        } else {
            format!("{} {} fell into {}.", cap(&count_words(fell as usize)), foe, pit)
        };
        let at = self.pits.decks.first().copied();
        self.msg_at(&text, (240, 140, 90), at, super::logkind::LogKind::Siege);
        if let Some(mut o) = self.overseer.take() {
            Self::note_as(&mut o, self.tick, NoteKind::Gate, text);
            self.overseer = Some(o);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::Species;
    use crate::world::Mover;

    /// A built drawbridge on open ground near the wagon, on a lever, with
    /// rock (soil) two levels deep under it. (bridge pos, bridge, lever)
    fn bridge(g: &mut Game) -> (Pos, u32, u32) {
        let e = g.embark;
        let ok = |g: &Game, q: Pos| {
            let t = g.world.t(q);
            t.kind == TileKind::Floor && t.water == 0 && t.building == 0 && g.items.values().all(|it| it.pos != q)
        };
        for r in 3..25 {
            for dx in -r..=r {
                let p = e.offset(dx, r, 0);
                let p = Pos::new(p.x, p.y, g.world.surface_z(p.x, p.y));
                let l = p.offset(0, 2, 0);
                let rock = (1..=PIT_DEPTH).all(|k| g.world.get(p.offset(0, 0, -k)).map_or(false, |t| t.kind == TileKind::Wall && t.mat.diggable()));
                let flat = (-1..=1).all(|dy| ok(g, p.offset(-1, dy, 0)) && ok(g, p.offset(1, dy, 0)));
                if !(ok(g, p) && ok(g, l) && rock && flat) {
                    continue;
                }
                for (k, q) in [(BuildingKind::Bridge, p), (BuildingKind::Lever, l)] {
                    assert!(g.place_building(k, q));
                    let id = g.world.t(q).building;
                    let b = g.buildings.get_mut(&id).unwrap();
                    b.built = true;
                    b.parts = k.parts();
                }
                let (bid, lid) = (g.world.t(p).building, g.world.t(l).building);
                g.spawn_item(ItemKind::Mechanism, Mat::Granite, e);
                g.link_lever(lid, bid).unwrap();
                return (p, bid, lid);
            }
        }
        panic!("nowhere for a bridge");
    }

    /// O04 1b: channelled under, a lowered drawbridge's deck is floor over
    /// a pit two levels deep, and the way across it is walkable; raised,
    /// there is nothing there.
    #[test]
    fn a_lowered_bridge_over_a_pit_is_walkable() {
        let mut g = Game::new(51);
        let (p, _, lever) = bridge(&mut g);
        assert_eq!(g.channel_under_bridge(p), Some(p.offset(0, 0, -2)));
        assert!(g.over_pit(p));
        assert_eq!(g.world.kind(p.down()), Some(TileKind::Open));
        assert_eq!(g.world.kind(p.offset(0, 0, -2)), Some(TileKind::Floor));
        assert!(g.world.walkable(p));
        let (a, b) = (p.offset(-1, 0, 0), p.offset(1, 0, 0));
        let way = crate::path::astar(&g.world, a, b, 1000, Mover::Goblin).expect("no way across the bridge");
        assert!(way.len() <= 3, "went round: {:?}", way);
        g.pull_lever(lever);
        assert!(!g.world.walkable(p), "the raised bridge's gap is walkable");
        assert!(!g.world.passable(p, Mover::Dwarf));
        g.pull_lever(lever);
        assert!(g.world.walkable(p), "lowered again, the deck is back");
    }

    /// The pit's lever never raises the bridge with one of ours on it, and
    /// a dwarf who lived through a fall climbs back out.
    #[test]
    fn the_pit_spares_the_fort() {
        let mut g = Game::new(51);
        let (p, _, lever) = bridge(&mut g);
        g.channel_under_bridge(p);
        g.pits.decks = vec![p];
        g.pits.lever = Some(g.buildings[&lever].pos);
        assert_eq!(g.pit_lever(), Some(lever));
        g.dwarves[0].pos = p;
        g.pull_lever(lever);
        assert!(g.world.walkable(p) && g.dwarves[0].pos == p, "the bridge rose under a dwarf");
        let out = g.embark;
        g.dwarves[0].pos = out;
        g.pull_lever(lever);
        assert!(!g.world.walkable(p), "with nobody on it, it rises");
        // One fallen in, the stun worn off: they climb out.
        g.dwarves[0].pos = p.offset(0, 0, -PIT_DEPTH);
        g.dwarves[0].move_cd = 0;
        g.drown_tick();
        let at = g.dwarves[0].pos;
        assert!(at.z == p.z && g.world.walkable(at), "still down the pit at {:?}", at);
        assert!(g.log.iter().any(|l| l.text.contains("climbed out of")));
        // And the cause of a death by falling is told as such.
        assert_eq!(super::super::annals::cause_of("fell to their death in the Belly"), "fell into a pit");
    }

    /// Over hollow rock two levels down, the gatehouse's pit is cut a level
    /// deep: the deck stands over its dug floor, the miners don't channel it
    /// again, and raised, the bridge drops a goblin one level.
    #[test]
    fn a_shallow_pit_still_drops_them() {
        let mut g = Game::new(51);
        let (p, _, lever) = bridge(&mut g);
        g.pits.decks = vec![p];
        g.pits.depth = 1;
        assert_eq!(g.channel_under_bridge(p), Some(p.down()));
        assert_eq!(g.world.kind(p.down()), Some(TileKind::Floor));
        assert!(g.over_pit(p), "a shallow pit's deck isn't over it");
        assert!(g.world.walkable(p));
        let gob = g.spawn_creature(crate::creature::Species::Goblin, p, u64::MAX);
        g.pull_lever(lever);
        assert!(!g.world.walkable(p));
        assert_eq!(g.creatures.get(&gob).map(|c| c.pos), Some(p.down()), "the goblin didn't fall a level");
    }

    /// O04 1b: raising the bridge over the pit drops whoever is on it:
    /// down two levels, hurt by the fall, the siege's toll counted, and the
    /// moment told.
    #[test]
    fn raising_the_pit_bridge_drops_whoever_is_on_it() {
        let mut g = Game::new(51);
        g.creatures.retain(|_, c| c.tame);
        let (p, _, lever) = bridge(&mut g);
        g.channel_under_bridge(p);
        g.annals.sieges.push(super::super::annals::SiegeNote { foe: "goblins".into(), size: 6, ..Default::default() });
        let gobs: Vec<u32> = (0..4)
            .map(|_| {
                let c = g.spawn_creature(Species::Goblin, p, u64::MAX);
                g.creatures.get_mut(&c).unwrap().siege = true;
                c
            })
            .collect();
        let hp0: i32 = gobs.iter().map(|id| g.creatures[id].body.hp.iter().sum::<i32>()).sum();
        g.pull_lever(lever);
        let bottom = p.offset(0, 0, -2);
        let alive: Vec<&crate::creature::Creature> = gobs.iter().filter_map(|id| g.creatures.get(id)).collect();
        assert!(alive.iter().all(|c| c.pos == bottom), "not all at the bottom");
        assert!(alive.iter().all(|c| !c.siege && c.move_cd >= STUNNED));
        let hp1: i32 = alive.iter().map(|c| c.body.hp.iter().sum::<i32>()).sum::<i32>() + (4 - alive.len() as i32) * 1000;
        assert!(hp1 < hp0 || alive.len() < 4, "the fall hurt nobody");
        assert_eq!(g.annals.sieges[0].fell, 4);
        let said: Vec<String> = g.log.iter().rev().take(12).map(|l| l.text.clone()).collect();
        assert!(said.iter().any(|t| t.starts_with("The bridge rose beneath them: four goblins fell into the pit.")), "{:?}", said);
        assert_eq!(g.pits.prisoners.len(), alive.len());
        // The siege can end without those left down there.
        assert!(!g.creatures.values().any(|c| c.alive && c.siege));
        // A dwarf on it falls too (the overseer never raises it under one).
        g.pull_lever(lever);
        let i = g.dwarves.iter().position(|d| d.alive).unwrap();
        g.dwarves[i].pos = p;
        g.pull_lever(lever);
        assert_eq!(g.dwarves[i].pos.z, bottom.z);
    }

    /// The pond: a goblin in deep water at the bottom of the pit drowns,
    /// and the siege it came with counts it.
    #[test]
    fn a_foe_in_the_pond_drowns() {
        let mut g = Game::new(51);
        g.creatures.retain(|_, c| c.tame);
        let (p, _, lever) = bridge(&mut g);
        g.channel_under_bridge(p);
        g.pits.decks = vec![p];
        let bottom = p.offset(0, 0, -2);
        g.world.t_mut(bottom).water = 5;
        g.annals.sieges.push(super::super::annals::SiegeNote { foe: "goblins".into(), size: 6, ..Default::default() });
        let c = g.spawn_creature(Species::Goblin, p, u64::MAX);
        g.creatures.get_mut(&c).unwrap().siege = true;
        let hp0: i32 = g.creatures[&c].body.hp.iter().sum();
        g.pull_lever(lever);
        assert_eq!(g.creatures[&c].body.hp.iter().sum::<i32>(), hp0, "the water should break the fall");
        for _ in 0..40 {
            g.tick += 10;
            g.drown_tick();
        }
        assert!(!g.creatures.contains_key(&c), "it never drowned");
        assert_eq!(g.annals.sieges[0].drowned, 1);
        assert!(g.log.iter().any(|l| l.text.contains("has drowned in the pit")));
    }

    /// O04 1b: haulers fill the pit from the well, a bucket at a time,
    /// until it is a pond.
    #[test]
    fn haulers_fill_a_pond() {
        let mut g = Game::new(51);
        g.creatures.retain(|_, c| c.tame);
        let (p, _, _) = bridge(&mut g);
        g.channel_under_bridge(p);
        // A well over water by the wagon.
        let e = g.embark;
        let mut well = None;
        'find: for r in 2..20 {
            for dx in -r..=r {
                let q = e.offset(dx, -r, 0);
                let q = Pos::new(q.x, q.y, g.world.surface_z(q.x, q.y));
                let (a, b) = (q.down(), q.down().down());
                if g.world.walkable(q) && g.world.t(q).building == 0 && g.world.solid(a) && g.world.solid(b) && g.world.walkable(q.offset(-1, 0, 0)) {
                    well = Some(q);
                    break 'find;
                }
            }
        }
        let w = well.expect("nowhere for a well");
        g.world.t_mut(w).kind = TileKind::Open;
        g.world.t_mut(w.down()).kind = TileKind::Open;
        g.world.t_mut(w.down()).water = 7;
        g.water_cells = None;
        assert!(g.place_building(BuildingKind::Well, w));
        let id = g.world.t(w).building;
        g.buildings.get_mut(&id).unwrap().built = true;
        g.spawn_item(ItemKind::Bucket, Mat::Wood, e);
        g.pits.decks = vec![p];
        g.pits.pond = true;
        g.alert = false;
        let bottom = p.offset(0, 0, -2);
        for _ in 0..3 * DAY {
            g.step();
            if g.pits.filled {
                break;
            }
        }
        assert!(g.pits.filled, "the pond was never filled (water {})", g.world.t(bottom).water);
        assert!(g.world.t(bottom).water >= DROWN_DEPTH);
    }
}

#[cfg(test)]
mod fort_tests {
    use super::*;
    use crate::creature::Species;
    use crate::world::Mover;

    /// O04 stage 2 in a fort: a Warlord with its trap hall begun lays a
    /// drawbridge past it, has the pit cut under it, sets its lever behind
    /// and links it; and when goblins crowd onto the bridge, it is raised
    /// under them.
    #[test]
    fn a_warlord_digs_a_pit_under_a_bridge_past_the_trap_hall() {
        let (found, passed) = super::super::overseer::fort_where(Style::Warlord, 5000, 16, 20, |g| {
            g.overseer.as_ref().map_or(false, |o| o.did("linked") && o.base.map_or(false, |b| g.pit_ground(b)))
        });
        let Some((seed, mut g)) = found else { panic!("no Warlord fort from 5000 on had its gate linked over sound rock by day 20 (passed over {:?})", passed) };
        g.knobs.set.insert("gate.pit_day".into(), 0.0);
        let e = g.embark;
        for _ in 0..24 {
            g.spawn_item(ItemKind::Mechanism, Mat::Granite, e);
        }
        for _ in 0..6 {
            g.spawn_item(ItemKind::Log, Mat::Wood, e);
        }
        for _ in 0..2 {
            g.spawn_item(ItemKind::Spear, Mat::Iron, e);
            g.spawn_item(ItemKind::Thread, Mat::Flesh, e);
        }
        let ready = |g: &Game| g.pit_lever().is_some() && g.pits.decks.iter().all(|&p| g.over_pit(p) && g.world.walkable(p));
        for _ in 0..16 * DAY {
            if ready(&g) {
                break;
            }
            g.step();
        }
        let o = g.overseer.as_ref().unwrap();
        let base = o.base.unwrap();
        assert!(o.gate_stage >= 2, "seed {}: no pit begun", seed);
        let name = g.pits.name.clone().unwrap();
        assert!(o.notes.iter().any(|n| n.1.contains(&name)), "seed {}: the overseer never named the pit", seed);
        assert!(ready(&g), "seed {}: the pit bridge isn't ready after 16 days (decks {:?}, over pit {:?}, lever {:?})", seed, g.pits.decks, g.pits.decks.iter().map(|&p| g.over_pit(p)).collect::<Vec<_>>(), g.pits.lever);
        // The way from the gate into the fort still goes over it.
        assert!(crate::path::astar(&g.world, base.offset(5, 0, 0), base.offset(16, 0, 0), 5000, Mover::Dwarf).is_some(), "seed {}: the pit cut the corridor", seed);
        // Three goblins on the bridge, a dwarf by the lever: it goes up.
        g.creatures.retain(|_, c| !c.siege);
        let decks = g.pits.decks.clone();
        for &p in decks.iter().take(3) {
            let c = g.spawn_creature(Species::Goblin, p, u64::MAX);
            g.creatures.get_mut(&c).unwrap().siege = true;
        }
        let lp = g.pits.lever.unwrap();
        for d in g.dwarves.iter_mut() {
            d.pos = e;
        }
        let i = g.dwarves.iter().position(|d| d.alive).unwrap();
        g.dwarves[i].pos = lp.offset(0, 0, 0);
        let mut o = g.overseer.take().unwrap();
        g.ov_pit_alert(&mut o);
        g.overseer = Some(o);
        assert!(decks.iter().all(|&p| !g.world.walkable(p)), "seed {}: the bridge didn't go up", seed);
        assert!(!g.creatures.values().any(|c| c.alive && decks.contains(&c.pos)), "seed {}: goblins still on the bridge", seed);
        assert!(g.pits.fell >= 3);
    }
}
