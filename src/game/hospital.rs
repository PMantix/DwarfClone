//! A hospital ready before it's needed: the fortress notices when the
//! wounded have nobody to tend them (`Lack::Doctor`), so the overseer can
//! name a doctor. The rest of the hospital is the overseer's plan (it digs
//! one right after the first beds), its stocks (`StockFilter::Medical`) and
//! the doctor's rounds in `medicine.rs`.

use super::overseer::{Overseer, RoomUse, Style};
use crate::buildings::Recipe;
use super::troubles::Lack;
use super::*;

impl Game {
    /// Every little while: someone bleeding or senseless, and nobody who
    /// can doctor? Say so, and remember the want.
    pub(super) fn doctor_check(&mut self) {
        let hurt = self.dwarves.iter().any(|d| d.alive && (d.body.bleed_rate() > 0 || d.body.unconscious()));
        if !hurt || self.has_doctor() {
            return;
        }
        self.trouble(Lack::Doctor);
        self.count("nodoctor", 1);
        self.warn("nodoctor", "The wounded lie untended: nobody in the fortress is a doctor!", (255, 120, 60));
    }

    /// Daily: a careful overseer wants its hospital dug as soon as the
    /// first beds are, ahead of whatever else it has its heart set on, so
    /// it's ready before the first siege. (Reckless makes do with the
    /// dormitory.)
    pub(super) fn ov_hospital(&mut self, o: &mut Overseer) {
        if o.style == Style::Reckless {
            return;
        }
        self.ov_dressings(o);
        // (Not while it's found no rock to cut one from: it would only ask
        // again, and again, to no effect.)
        if o.wants.contains(&RoomUse::Hospital) || o.blocked_lately(RoomUse::Hospital, self.tick) {
            return;
        }
        // (After the first beds, or after a while anyway: other wants can
        // keep the plan from ever reaching the dormitory.)
        let has = |k: RoomUse| o.rooms.iter().any(|r| r.kind == k && r.stage < 2);
        let late = self.tick as f64 >= self.knob("hospital.by_day", 60.0) * DAY as f64;
        if has(RoomUse::Hospital) || !(has(RoomUse::Dormitory) || late) {
            return;
        }
        o.wants.insert(0, RoomUse::Hospital);
        Self::note_ov(o, self.tick, "Wanted a hospital ready before the first siege.".into());
    }

    /// Once the hospital is fitted, keep thread or cloth on hand to dress
    /// wounds with: spinning jumps the craftsdwarf's queue (when there's
    /// fibre to spin), and with no craftsdwarf's workshop one goes up by
    /// the hospital.
    fn ov_dressings(&mut self, o: &mut Overseer) {
        let Some(h) = o.rooms.iter().find(|r| r.kind == RoomUse::Hospital && r.stage == 1) else { return };
        let have = self.stock_of(ItemKind::Thread) + self.stock_of(ItemKind::Cloth);
        if have as f64 >= self.knob("hospital.dressings", 4.0) {
            return;
        }
        let shop = self.buildings.values().filter(|b| b.kind == BuildingKind::Craftsdwarf).min_by_key(|b| (!b.built, b.id)).map(|b| (b.id, b.built));
        match shop {
            None => {
                let c = Pos::new((h.a.x + h.b.x) / 2, (h.a.y + h.b.y) / 2, h.a.z);
                if let Some(p) = self.ov_spot(BuildingKind::Craftsdwarf, c, 16) {
                    self.place_building(BuildingKind::Craftsdwarf, p);
                    Self::note_ov(o, self.tick, "Had a craftsdwarf's workshop built to spin thread for the hospital.".into());
                }
            }
            Some((id, true)) => {
                let fibre = self.items.values().any(|it| it.free() && crate::buildings::Input::Fiber.accepts(it));
                let b = self.buildings.get_mut(&id).unwrap();
                if fibre && b.orders.first().map_or(true, |x| x.recipe != Recipe::SpinThread) {
                    // (Taken from further back in the queue if it's there.)
                    let i = b.orders.iter().position(|x| x.recipe == Recipe::SpinThread);
                    let order = i.map(|i| b.orders.remove(i)).unwrap_or(crate::buildings::Order { recipe: Recipe::SpinThread, repeat: false, mat: None });
                    b.orders.insert(0, order);
                    self.count("hospital.thread", 1);
                }
            }
            Some((_, false)) => {}
        }
    }

    /// Whether some grown, sane dwarf can tend the wounded.
    pub(super) fn has_doctor(&self) -> bool {
        self.dwarves.iter().any(|d| d.alive && !d.insane && !d.is_child() && d.has(Labor::Medicine))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::Part;

    #[test]
    fn the_wounded_without_a_doctor_are_a_trouble() {
        let mut g = Game::new(5);
        for d in g.dwarves.iter_mut() {
            d.labors[Labor::Medicine.idx()] = false;
        }
        g.doctor_check();
        assert!(!g.troubles.iter().any(|t| t.0 == Lack::Doctor), "nobody is hurt yet");
        g.dwarves[0].body.wound(Part::LeftLeg, 9, true);
        g.doctor_check();
        assert!(g.troubles.iter().any(|t| t.0 == Lack::Doctor), "a bleeding dwarf and no doctor");
        assert!(g.log.iter().any(|m| m.text.contains("nobody in the fortress is a doctor")));
    }
}
