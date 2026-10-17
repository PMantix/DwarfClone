//! Archers who act like archers (MIL-05): a marksdwarf with bolts shoots
//! when loaded and steps back from a close foe while reloading, swings
//! the crossbow only when out of bolts, goes back for bolts when nothing
//! is close, and shoots from the station (through fortifications) at
//! whatever comes into sight. The overseer gives its second squad the
//! metal crossbows and posts it back from the gate with a clear shot.

use super::combat::{adjacent, dist2, near};
use super::military::{UniformWeapon, RANGE};
use super::overseer::Overseer;
use super::*;
use crate::material::Mat;
use crate::world::TileKind;

/// Closer than this, a reloading marksdwarf steps back.
const KEEP_OFF: i32 = 3;
/// No foe this close, and an empty quiver is filled before fighting on.
const SAFE_TO_RELOAD: i32 = 5;

fn gap(a: Pos, b: Pos) -> i32 {
    if a.z != b.z {
        return i32::MAX;
    }
    (a.x - b.x).abs().max((a.y - b.y).abs())
}

fn metal(m: Mat) -> bool {
    matches!(m, Mat::Copper | Mat::Bronze | Mat::Iron | Mat::Steel)
}

impl Game {
    /// A marksdwarf with a crossbow in hand and bolts in the quiver.
    pub(super) fn keeps_distance(&self, d: &Dwarf) -> bool {
        d.ammo > 0 && self.holds_crossbow(d)
    }

    /// The crossbow arm of `do_attack`. Returns true when it has seen to
    /// this tick; false leaves the dwarf to close in and swing as anyone
    /// would (out of bolts, with none to be had or the foe too close).
    ///
    /// A loaded crossbow is shot, even at a foe at their elbow; while it is
    /// reloaded they step back from anything closer than `KEEP_OFF`, and
    /// with nowhere to step they stand and finish reloading. So a
    /// marksdwarf with bolts never swings.
    pub(super) fn archer_attack(&mut self, d: &mut Dwarf, target: u32, cpos: Pos) -> bool {
        if !self.holds_crossbow(d) {
            return false;
        }
        if d.ammo == 0 {
            return self.draw_bolts(d);
        }
        if d.attack_cd > 0 && gap(cpos, d.pos) < KEEP_OFF {
            if let Some(stepped) = self.back_off(d) {
                if stepped {
                    self.count("archer.backoff", 1);
                    let what = self.creatures.get(&target).map_or("enemy", |c| c.name());
                    let text = format!("{} backs away from the {}, reloading.", d.called(), what);
                    self.warn(&format!("backoff{}", d.id), &text, (235, 225, 130));
                }
                return true;
            }
            if adjacent(cpos, d.pos) {
                // Cornered: stand and reload.
                return true;
            }
        }
        // In range and in sight: shoot, or wait out the reload where they stand.
        self.try_shoot(d, target)
    }

    /// A step that puts more ground between the marksdwarf and every foe
    /// near them (scored as `do_flee` scores it), keeping to their level so
    /// they can still shoot. Some(true) stepped, Some(false) waiting on
    /// their legs, None when there's nowhere better (cornered).
    fn back_off(&mut self, d: &mut Dwarf) -> Option<bool> {
        let foes: Vec<Pos> = self
            .creatures
            .values()
            .filter(|c| c.hostile() && self.creature_active(c) && near(c.pos, d.pos, 7))
            .map(|c| c.pos)
            .collect();
        if foes.is_empty() {
            return None;
        }
        let score = |p: Pos| foes.iter().map(|&f| dist2(f, p)).min().unwrap_or(0);
        let mut nb = Vec::new();
        self.world.neighbors(d.pos, &mut nb, Mover::Dwarf);
        let here = score(d.pos);
        let best = nb.iter().map(|&(q, _)| q).filter(|q| q.z == d.pos.z && !foes.contains(q)).max_by_key(|&q| score(q))?;
        if score(best) <= here {
            return None;
        }
        if d.move_cd > 0 {
            d.move_cd -= 1;
            return Some(false);
        }
        d.pos = best;
        d.path.clear();
        d.move_cd = MOVE_DELAY + d.body.limp();
        Some(true)
    }

    /// Out of bolts: go and get more, if there are any and nothing is close
    /// enough to catch them at it. Returns true if they set off.
    fn draw_bolts(&mut self, d: &mut Dwarf) -> bool {
        if !self.threats_near(d.pos, SAFE_TO_RELOAD).is_empty() {
            return false;
        }
        let Some((item, path)) = self.find_item(d.pos, super::military::is_bolts) else { return false };
        self.claim_item(item, d.id);
        d.task = Task::Equip { item };
        Self::set_path(d, path);
        self.count("archer.refill", 1);
        self.warn(&format!("bolts{}", d.id), &format!("{} runs back for more bolts.", d.called()), (235, 225, 130));
        true
    }

    /// A marksdwarf at their post shoots at any foe in range and in sight,
    /// whether or not it can get at them: from behind fortifications or
    /// across the moat. Called from `combat_awareness`.
    pub(super) fn archer_awareness(&mut self, d: &mut Dwarf) {
        if d.attack_cd > 0 || matches!(d.task, Task::Attack { .. }) || !self.keeps_distance(d) {
            return;
        }
        let mark = self
            .creatures
            .values()
            .filter(|c| c.hostile() && !c.sneaking && self.creature_active(c) && gap(c.pos, d.pos) <= RANGE)
            .filter(|c| self.clear_shot(d.pos, c.pos))
            .min_by_key(|c| (dist2(c.pos, d.pos), c.id))
            .map(|c| c.id);
        if let Some(cid) = mark {
            if self.try_shoot(d, cid) {
                self.count("archer.from_post", 1);
            }
        }
    }

    /// Where a squad on crossbows holds in an alert: back from the gate,
    /// a few tiles behind the drawbridge, with a clear shot at the gate
    /// (from behind a fortification if one stands in the line). None for
    /// any other squad, or when no such spot is dug.
    pub(super) fn archer_station(&self, squad: usize, base: Pos) -> Option<Pos> {
        if squad == 0 || self.squads.get(squad).map_or(true, |s| s.weapon != UniformWeapon::Crossbow) {
            return None;
        }
        let gate = base.offset(1, 0, 0);
        let fortified = |st: Pos| {
            let steps = gap(st, gate);
            (1..steps).any(|i| {
                let p = Pos::new(st.x + (gate.x - st.x) * i / steps, st.y + (gate.y - st.y) * i / steps, st.z);
                self.world.get(p).map_or(false, |t| t.kind == TileKind::Fortification)
            })
        };
        (5..=7)
            .flat_map(|dx| (0..=1).map(move |dy| base.offset(dx, dy, 0)))
            .filter(|&st| self.world.walkable(st) && self.clear_shot(st, gate))
            .min_by_key(|&st| (!fortified(st), st.x - base.x, st.y - base.y))
    }

    /// Metal crossbows in the fort, in hand or in stock.
    pub(super) fn metal_crossbows(&self) -> usize {
        self.items.values().filter(|it| it.kind == ItemKind::Crossbow && metal(it.mat)).count()
    }

    /// The uniform for squad `s` of the overseer's army: the second squad
    /// takes up crossbows once there are two of metal to go round.
    pub(super) fn archer_uniform(&self, o: &mut Overseer, s: usize) -> Option<UniformWeapon> {
        if s != 1 || self.metal_crossbows() < 2 {
            return None;
        }
        if self.squads[s].weapon != UniformWeapon::Crossbow {
            let text = format!("Gave {} the metal crossbows, to shoot from behind the gate.", self.squads[s].name);
            Self::note_ov(o, self.tick, text);
        }
        Some(UniformWeapon::Crossbow)
    }
}
