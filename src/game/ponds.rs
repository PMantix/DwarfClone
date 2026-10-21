//! The pond (ENG-03): once the pit under the bridge is cut, haulers fill it
//! from the well, a bucket at a time, poured down through the bridge's
//! planks, until it is deep enough that whatever falls in drowns
//! (`pits.rs: drown_tick`).

use super::*;

impl Game {
    /// Water the pond is filled to (of 7), at the bottom of the pit.
    fn pond_depth(&self) -> u8 {
        self.knob("pit.pond_depth", 5.0) as u8
    }

    /// The pit's bottom under deck `p`.
    fn pit_bottom(&self, p: Pos) -> Pos {
        self.world.settle(p.down())
    }

    /// A deck to pour from whose bottom wants water, the driest first (none
    /// while the bridge is up, the pit undug, or the pond full).
    pub(super) fn pond_target(&self) -> Option<Pos> {
        let want = self.pond_depth();
        self.pits
            .decks
            .iter()
            .copied()
            .filter(|&p| self.over_pit(p) && self.world.walkable(p))
            .map(|p| (self.world.t(self.pit_bottom(p)).water, p))
            .filter(|&(w, _)| w < want)
            .min_by_key(|&(w, p)| (w, p.x, p.y))
            .map(|(_, p)| p)
    }

    /// Whether the pit wants filling: every deck dug under, no siege on,
    /// and a well to draw from.
    pub(super) fn pond_wanted(&self) -> bool {
        self.pits.pond && !self.alert && self.pits.decks.iter().all(|&p| self.over_pit(p)) && self.pond_target().is_some()
    }

    /// The pond job (from the haul finder): take a bucket, fill it at the
    /// well, pour it into the pit. Two haulers at most.
    pub(super) fn find_pond(&mut self, d: &mut Dwarf) -> bool {
        if !self.pond_wanted() || self.well_stand(d.pos).is_none() {
            return false;
        }
        if self.dwarves.iter().filter(|x| matches!(x.task, Task::Pond { .. })).count() >= 2 {
            return false;
        }
        let Some((bucket, path)) = self.find_item(d.pos, |it| it.kind == ItemKind::Bucket) else { return false };
        self.claim_item(bucket, d.id);
        d.task = Task::Pond { bucket };
        Self::set_path(d, path);
        true
    }

    pub(super) fn do_pond(&mut self, d: &mut Dwarf, bucket: u32) {
        match self.fetch(d, bucket) {
            None => {
                self.release_task(d);
                return;
            }
            Some(false) => return,
            Some(true) => {}
        }
        let full = self.items.get(&bucket).map_or(false, |it| it.label.as_deref() == Some("water"));
        if !full {
            let Some(stand) = self.well_stand(d.pos) else {
                self.release_task(d);
                return;
            };
            match self.travel(d, stand) {
                Travel::Moving => {}
                Travel::Failed => self.release_task(d),
                Travel::Arrived => {
                    if Self::labor(d, 20, 0) {
                        if let Some(it) = self.items.get_mut(&bucket) {
                            it.label = Some("water".into());
                        }
                    }
                }
            }
            return;
        }
        let Some(deck) = self.pond_target().filter(|_| !self.alert) else {
            self.release_task(d);
            return;
        };
        match self.travel(d, deck) {
            Travel::Moving => {}
            Travel::Failed => self.release_task(d),
            Travel::Arrived => {
                if !Self::labor(d, 10, 0) {
                    return;
                }
                let bottom = self.pit_bottom(deck);
                let t = self.world.t_mut(bottom);
                t.water = (t.water + 1).min(7);
                self.water_cells = None;
                self.count("m2.pond_buckets", 1);
                if let Some(it) = self.items.get_mut(&bucket) {
                    it.label = None;
                }
                d.gain_xp(Labor::Hauling, 5);
                self.release_task(d);
                // Full: said once.
                if self.pond_target().is_none() && !self.pits.filled {
                    self.pits.filled = true;
                    let pit = self.pit_name();
                    let text = format!("The last bucket is poured: {} is a pond now, and whatever falls into it will drown.", pit);
                    self.msg_at(&text, (120, 170, 240), Some(deck), super::logkind::LogKind::Job);
                    if let Some(mut o) = self.overseer.take() {
                        Self::note_as(&mut o, self.tick, super::personality::NoteKind::Gate, format!("Had {} filled with water from the well, bucket by bucket.", pit));
                        self.overseer = Some(o);
                    }
                }
            }
        }
    }
}
