//! Water a fort can rely on (ENG-18): the overseer sinks a well inside the
//! shelter over water it finds below, so dwarves sheltering from a siege can
//! still drink when the barrels run dry.
//!
//! Water here only ever falls and spreads sideways, never rises, so a shaft
//! down to water can't flood the hall it opens from. What can flood is
//! digging *into* water sideways: a shaft is only planned through sound rock,
//! with nothing dug or wet beside it.

use super::overseer::{Overseer, RoomUse, Style};
use super::water::WELL_DEPTH;
use super::*;
use serde::{Deserialize, Serialize};

/// A well being sunk: the shaft under `top` reaches water at `water_z`, dug
/// as channels level by level from a stair beside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WellPlan {
    /// Where the well goes, in a corner of a hall.
    pub top: Pos,
    /// The stair beside the shaft the diggers work from (none when the
    /// water lies right under the floor).
    pub stair: Option<Pos>,
    /// The level the water is at (or seeps in at, for an aquifer).
    pub water_z: i32,
    /// When it was begun.
    pub since: u64,
}

/// The fort's waterworks (`Game.waterworks`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Waterworks {
    /// A well being sunk.
    #[serde(default)]
    pub well: Option<WellPlan>,
}

impl WellPlan {
    /// The tiles that become open shaft, top down.
    fn shaft(&self) -> impl Iterator<Item = Pos> + '_ {
        (self.water_z + 1..=self.top.z).rev().map(move |z| Pos::new(self.top.x, self.top.y, z))
    }
}

/// Rooms a well may go in, best first.
fn well_room(kind: RoomUse) -> Option<u8> {
    Some(match kind {
        RoomUse::Dining => 0,
        RoomUse::Meeting | RoomUse::Tavern | RoomUse::Grand => 1,
        RoomUse::Hospital => 2,
        RoomUse::Stockpile | RoomUse::Workshops | RoomUse::Barracks | RoomUse::Temple => 3,
        _ => return None,
    })
}

impl Game {
    /// Sound rock to dig a shaft through: a natural wall, no aquifer.
    fn shaft_rock(&self, p: Pos) -> bool {
        self.world.get(p).map_or(false, |t| t.kind == TileKind::Wall && t.mat.diggable() && t.built_from.is_none() && !t.aquifer && t.magma == 0)
    }

    /// Nothing open, wet, seeping or marked for digging beside `p` on its
    /// level, except `but`.
    fn sealed(&self, p: Pos, but: Option<Pos>) -> bool {
        DIRS8.iter().map(|&(dx, dy)| p.offset(dx, dy, 0)).filter(|&q| Some(q) != but).all(|q| {
            self.world.get(q).map_or(true, |t| {
                t.kind.solid() && t.water == 0 && t.magma == 0 && t.desig == Desig::None && !(t.aquifer && self.world.aquifer >= 2)
            })
        })
    }

    /// Water under `p` within a well's reach, through sound rock (and beside
    /// `stair`'s column, when there is one): the level it's at.
    fn water_below(&self, p: Pos, stair: Option<Pos>) -> Option<i32> {
        for k in 1..=WELL_DEPTH {
            let q = p.offset(0, 0, -k);
            let t = self.world.get(q)?;
            let heavy = t.aquifer && self.world.aquifer >= 2 && t.kind == TileKind::Wall;
            if (t.water >= 3 && !t.frozen && t.magma == 0) || heavy {
                // Right below the floor needs no stair; deeper needs one, its
                // foot on rock.
                return match (k, stair) {
                    (1, _) => Some(q.z),
                    (_, Some(s)) if self.world.solid(Pos::new(s.x, s.y, q.z)) => Some(q.z),
                    _ => None,
                };
            }
            let s = stair.map(|s| Pos::new(s.x, s.y, q.z));
            if !self.shaft_rock(q) || !self.sealed(q, s) {
                return None;
            }
            if let Some(s) = s {
                if !self.shaft_rock(s) || !self.sealed(s, Some(q)) {
                    return None;
                }
            }
        }
        None
    }

    /// Where a well could be sunk inside the fort, over water a shaft can
    /// reach: a corner of a fitted hall in the shelter, the dining hall first.
    pub(super) fn find_well_spot(&self, o: &Overseer) -> Option<WellPlan> {
        let mut rooms: Vec<(u8, &super::overseer::Room)> = o.rooms.iter().filter(|r| r.stage >= 1).filter_map(|r| well_room(r.kind).map(|k| (k, r))).collect();
        rooms.sort_by_key(|(k, _)| *k);
        let free = |g: &Game, p: Pos| {
            g.world.get(p).map_or(false, |t| t.kind == TileKind::Floor && t.building == 0 && t.water == 0 && t.desig == Desig::None && !g.world.outside(p))
                && !o.anchors.contains(&p)
                && !o.rooms.iter().any(|r| r.doors.contains(&p))
        };
        let mut best: Option<(i32, u8, WellPlan)> = None;
        for (rank, r) in rooms {
            let tiles: Vec<Pos> = if r.tiles.is_empty() { box_iter(r.a, r.b).collect() } else { r.tiles.clone() };
            for p in tiles {
                if !free(self, p) {
                    continue;
                }
                // A corner, out of the way: walls on two sides, the room on the other two.
                let solid = |dx: i32, dy: i32| self.world.solid(p.offset(dx, dy, 0));
                let ox = match (solid(-1, 0), solid(1, 0)) {
                    (true, false) => 1,
                    (false, true) => -1,
                    _ => continue,
                };
                let oy = match (solid(0, -1), solid(0, 1)) {
                    (true, false) => 1,
                    (false, true) => -1,
                    _ => continue,
                };
                if ![(ox, 0), (0, oy), (ox, oy)].iter().all(|&(dx, dy)| self.world.walkable(p.offset(dx, dy, 0))) {
                    continue;
                }
                for s in [p.offset(ox, 0, 0), p.offset(0, oy, 0)] {
                    let stair = Some(s).filter(|&s| free(self, s));
                    let Some(wz) = self.water_below(p, stair) else { continue };
                    let depth = p.z - wz;
                    let plan = WellPlan { top: p, stair: if depth > 1 { stair } else { None }, water_z: wz, since: self.tick };
                    if best.as_ref().map_or(true, |b| (depth, rank) < (b.0, b.1)) {
                        best = Some((depth, rank, plan));
                    }
                    break;
                }
            }
        }
        best.map(|b| b.2)
    }

    /// Lay out the dig for a well: the stair beside the shaft, and the shaft
    /// channelled down level by level.
    fn begin_well(&mut self, plan: WellPlan) {
        let top = plan.top;
        if let Some(s) = plan.stair {
            let foot = Pos::new(s.x, s.y, plan.water_z + 1);
            self.designate(s, s, Desig::StairDown);
            if foot.z < s.z - 1 {
                self.designate(s.down(), Pos::new(s.x, s.y, foot.z + 1), Desig::StairUD);
            }
            self.designate(foot, foot, Desig::StairUp);
        }
        for p in plan.shaft().collect::<Vec<_>>() {
            self.designate(p, p, Desig::Channel);
        }
        self.waterworks.well = Some(plan);
        let _ = top;
    }

    /// Start on water inside the fort, if there's none yet and none under
    /// way and the overseer can find some: whether it began.
    pub(super) fn start_waterworks(&mut self, o: &mut Overseer) -> bool {
        if o.style == Style::Reckless || o.did("well") || self.waterworks.well.is_some() {
            return false;
        }
        self.sink_well(o)
    }

    /// Start sinking a well if there's a spot for one. Whether one was begun.
    fn sink_well(&mut self, o: &mut Overseer) -> bool {
        let Some(plan) = self.find_well_spot(o) else { return false };
        self.begin_well(plan);
        // It needs a bucket to draw with.
        if self.stock_of(ItemKind::Bucket) == 0 && !o.boosts.iter().any(|b| b.0 == crate::buildings::Recipe::Bucket) {
            o.boosts.push((crate::buildings::Recipe::Bucket, 2, self.tick + DAY * 20));
        }
        let room = o.rooms.iter().find(|r| r.tiles.contains(&plan.top) || (r.tiles.is_empty() && box_iter(r.a, r.b).any(|p| p == plan.top))).map_or("the halls".to_string(), |r| format!("the {:?}", r.kind).to_lowercase());
        let text = o.voiced(&format!("Set miners to sink a well in {}, {} levels down to water.", room.replace("dining", "dining hall"), plan.top.z - plan.water_z));
        Self::note_ov(o, self.tick, text);
        true
    }

    /// Once a day (`overseer_hooks.rs`): see the waterworks along.
    pub(super) fn ov_water(&mut self, o: &mut Overseer) {
        if o.style == Style::Reckless {
            return;
        }
        // Once there's a shelter to keep water in, find some.
        if o.did("burrow") {
            self.start_waterworks(o);
        }
        self.tend_well(o);
    }

    /// The well under way: put the well over the shaft once it's open, and
    /// call it done when it draws water.
    fn tend_well(&mut self, o: &mut Overseer) {
        let Some(plan) = self.waterworks.well else { return };
        let top = plan.top;
        let tile = *self.world.t(top);
        let bld = self.buildings.get(&tile.building).filter(|b| b.kind == BuildingKind::Well);
        match bld {
            Some(b) if b.built && self.well_has_water(top) => {
                self.waterworks.well = None;
                o.done.push("well".into());
                self.count("wells", 1);
                let fort = self.fortress_name.clone();
                let text = o.voiced("Sank a well inside the fort: water from below, even in a siege.");
                Self::note_ov(o, self.tick, text);
                self.msg(&format!("A well has been sunk in {}: clean water inside the walls.", fort), (110, 170, 255));
                self.legend(format!("A well was sunk in {}.", fort));
                return;
            }
            Some(_) => {}
            None if tile.kind == TileKind::Open => {
                self.place_building(BuildingKind::Well, top);
            }
            None => {
                // Channels pruned (or never valid): mark the shaft again.
                if tile.desig == Desig::None && tile.kind == TileKind::Floor {
                    self.designate(top, top, Desig::Channel);
                }
            }
        }
        // Given up on after two months (a spot that wasn't what it seemed).
        if self.tick > plan.since + self.knob("well.give_up_days", 60.0) as u64 * DAY {
            self.waterworks.well = None;
            if !o.did("nowell") {
                o.done.push("nowell".into());
            }
            Self::note_ov(o, self.tick, "Gave up on the well: the shaft never reached water.".into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fort with its halls dug and its shelter marked, and a pocket of
    /// water put two levels under a corner of one of them.
    fn fort_over_water(seed: u64) -> (Game, Pos) {
        let mut g = trial_game(seed, Style::Steady, 40);
        let o = g.overseer.clone().unwrap();
        assert!(o.did("burrow") && !o.did("well"));
        assert!(g.find_well_spot(&o).is_none(), "already water below");
        let tiles: Vec<Pos> = o.rooms.iter().filter(|r| r.stage >= 1).flat_map(|r| if r.tiles.is_empty() { box_iter(r.a, r.b).collect() } else { r.tiles.clone() }).collect();
        for p in tiles {
            let q = p.offset(0, 0, -2);
            if !g.world.solid(q) {
                continue;
            }
            let before = *g.world.t(q);
            g.world.t_mut(q).kind = TileKind::Open;
            g.world.t_mut(q).water = 7;
            if g.find_well_spot(&o).map_or(false, |w| w.top == p) {
                g.water_cells = None;
                return (g, p);
            }
            *g.world.t_mut(q) = before;
        }
        panic!("no corner to put water under");
    }

    #[test]
    fn the_overseer_sinks_a_well_over_water() {
        let (mut g, top) = fort_over_water(5000);
        let e = g.embark;
        g.spawn_item(ItemKind::Bucket, Mat::Wood, e);
        g.spawn_item(ItemKind::Block, Mat::Granite, e);
        let start = g.tick;
        while g.tick < start + 3 * DAY && g.working_wells().is_empty() {
            g.step();
        }
        assert_eq!(g.working_wells(), vec![top], "no well after {} ticks; plan {:?}", g.tick - start, g.waterworks.well);
        // Done, told, and counted.
        for _ in 0..DAY {
            g.step();
        }
        let o = g.overseer.as_ref().unwrap();
        assert!(o.did("well"), "the overseer didn't call it done");
        assert!(o.notes.iter().any(|n| n.1.contains("Sank a well")));
        assert_eq!(g.annals.counters.get("wells"), Some(&1));
        assert!(g.waterworks.well.is_none());
        // And never under water itself: the hall is dry.
        assert_eq!(g.world.t(top.offset(1, 0, 0)).water + g.world.t(top.offset(-1, 0, 0)).water, 0);
    }

    /// How many of the batch sites have water a well could reach, once the
    /// halls are dug.
    #[test]
    #[ignore]
    fn well_spots_at_the_batch_sites() {
        let mut found = 0;
        for seed in 5000..5025u64 {
            let style = [Style::Steady, Style::Builder, Style::Delver, Style::Warlord, Style::Reckless][(seed % 5) as usize];
            let g = trial_game(seed, style, 40);
            let Some(o) = g.overseer.as_ref() else { continue };
            let spot = g.find_well_spot(o);
            found += spot.is_some() as u32;
            eprintln!("seed {} {:?} aquifer {} rooms {} spot {:?}", seed, style, g.world.aquifer, o.rooms.len(), spot);
        }
        eprintln!("{} of 25 have a well spot", found);
    }
}
