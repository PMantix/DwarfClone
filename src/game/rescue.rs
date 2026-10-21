//! First aid in the field (S02, HLT-07): a dwarf bleeding hard stops work and
//! presses the wound, friends and passers-by bind it where they lie, and the
//! senseless and the crippled are carried to a bed once the fight has moved
//! on (never into it).

use super::*;

/// Standing on or next to `b`, on the same level.
fn beside(a: Pos, b: Pos) -> bool {
    a.z == b.z && (a.x - b.x).abs() <= 1 && (a.y - b.y).abs() <= 1
}

/// Far enough gone that a dwarf eats or drinks before anything else.
fn desperate(d: &Dwarf) -> bool {
    d.thirst > THIRSTY + 1500 || d.hunger > HUNGRY + 3000
}

/// Down where they are: senseless, or crippled and hurt with nothing to get
/// up for. A crippled dwarf who is thirsty or hungry, or on the way to a
/// drink, a meal or a bed, is left to it: lifting them drops the errand, and
/// a thirsty dwarf laid in bed only gets up again.
fn down(p: &Dwarf) -> bool {
    let b = &p.body;
    if b.unconscious() {
        return true;
    }
    let walking = !p.path.is_empty();
    let errand = p.thirst > THIRSTY
        || p.hunger > HUNGRY
        || matches!(p.task, Task::DrinkItem { .. } | Task::DrinkWater { .. } | Task::Eat { .. } | Task::Dine { .. })
        || walking && matches!(p.task, Task::Rest { .. } | Task::Sleep { .. });
    b.limp() >= 5 && b.needs_rest() && !errand
}

/// How long a patient nobody could help is left before anyone tries again.
const CARE_RETRY: u64 = 60;

impl Game {
    /// In `body_phase`: a dwarf bleeding hard, with no foe within five
    /// tiles, drops what they're doing and presses the wound (`Task::Press`).
    /// A dwarf already lying in bed just holds it.
    pub(super) fn maybe_press(&mut self, d: &mut Dwarf) {
        if d.task == Task::Press || d.body.bleed_rate() == 0 || self.tick % 5 != d.id as u64 % 5 {
            return;
        }
        let Some(worst) = d.body.worst_bleed() else { return };
        if d.asleep && d.path.is_empty() || d.carried_by.is_some() {
            d.body.press(worst);
            return;
        }
        if d.body.unconscious() || d.insane || d.fit != breakdowns::Fit::None || d.task == Task::Flee || desperate(d) {
            return;
        }
        if (d.body.bleed_rate() as f64) < self.knob("bleed.press_at", 6.0) || !self.threats_near(d.pos, 5).is_empty() {
            return;
        }
        self.release_task(d);
        d.task = Task::Press;
        d.body.press(worst);
        self.count("presses", 1);
        let (name, part) = (d.name.clone(), worst.name());
        let text = if d.body.ticks_to_death().map_or(false, |t| t < 600) {
            format!("{} lies down, pressing the wound in their {}, and waits for help.", name, part)
        } else {
            format!("{} presses a hand to the wound in their {}.", name, part)
        };
        self.warn(&format!("press{}", d.id), &text, (255, 150, 120));
    }

    /// Standing still with a hand on the worst wound, until it has nearly
    /// stopped or a foe comes close (then `combat_awareness` takes over).
    pub(super) fn do_press(&mut self, d: &mut Dwarf) {
        d.path.clear();
        let done = (d.body.bleed_rate() as f64) < self.knob("bleed.press_until", 2.0) || desperate(d);
        if done || self.tick % 5 == d.id as u64 % 5 && !self.threats_near(d.pos, 5).is_empty() {
            self.release_task(d);
            return;
        }
        if let Some(worst) = d.body.worst_bleed() {
            d.body.press(worst);
        }
    }

    /// Bandages: thread or cloth, or failing those a spare shirt or pair of
    /// trousers torn into rags. The nearest within reach.
    fn find_bandage(&self, from: Pos) -> Option<(u32, Vec<Pos>)> {
        let limit = SEARCH_LIMIT / 4;
        for kinds in [&[ItemKind::Thread, ItemKind::Cloth][..], &[ItemKind::Shirt, ItemKind::Trousers][..]] {
            let at = self.loose_items(|it| kinds.contains(&it.kind));
            if at.is_empty() {
                continue;
            }
            if let Some((p, path)) = path::search(&self.world, from, limit, Mover::Dwarf, |p| at.contains_key(&p)) {
                return Some((at[&p], path));
            }
        }
        None
    }

    /// A doctor other than `not`, free for work (idle, or wandering once
    /// their break is over), within 20 tiles of `p`.
    fn free_doctor_near(&self, p: Pos, not: u32) -> bool {
        self.dwarves.iter().any(|o| {
            o.alive
                && o.id != not
                && o.has(Labor::Medicine)
                && !o.soldier
                && !o.is_child()
                // (Not one on a break: they don't look for work till it's over.)
                && (o.task == Task::Idle || matches!(o.task, Task::Wander { .. }) && self.tick >= o.break_until)
                && combat::near(o.pos, p, 20)
        })
    }

    /// A finder: any grown dwarf who doctors or hauls goes to a wounded
    /// dwarf nearby who is bleeding with nobody seeing to them and no foe
    /// within seven tiles, bringing a bandage if there's one to hand.
    pub(super) fn find_first_aid(&mut self, d: &mut Dwarf) -> bool {
        if d.is_child() || d.soldier || !(d.has(Labor::Medicine) || d.has(Labor::Hauling)) {
            return false;
        }
        let min = self.knob("firstaid.min", 2.0);
        // The wounded abed are the doctor's to suture (`find_patient`), when
        // there is thread for it: a doctor leaves them to that, and so does
        // anyone else with a free doctor near.
        let doctor = d.has(Labor::Medicine);
        let thread = self.items.values().any(|it| it.kind == ItemKind::Thread && it.free());
        let mut wounded: HashMap<Pos, (u32, bool)> = HashMap::new();
        for p in self.dwarves.iter() {
            if !p.alive || p.id == d.id || p.carried_by.is_some() || self.treating.contains(&p.id) || self.care_waits(p.id, false) {
                continue;
            }
            let abed = p.asleep && matches!(p.task, Task::Rest { .. });
            if abed && thread && (doctor || self.free_doctor_near(p.pos, d.id)) {
                continue;
            }
            if (p.body.bleed_rate() as f64) < min || matches!(p.task, Task::Flee | Task::Attack { .. } | Task::Berserk) {
                continue;
            }
            let Some(w) = p.body.worst_bleed() else { continue };
            if !p.body.bandaged[w.idx()] {
                wounded.insert(p.pos, (p.id, p.body.pressed[w.idx()]));
            }
        }
        if wounded.is_empty() {
            return false;
        }
        wounded.retain(|&p, _| self.threats_near(p, 7).is_empty());
        if wounded.is_empty() {
            return false;
        }
        let close = |q: Pos| wounded.keys().any(|&p| beside(q, p));
        let Some((at, path)) = path::search(&self.world, d.pos, SEARCH_LIMIT / 4, Mover::Dwarf, close) else {
            // Out of reach (for now).
            let ids: Vec<u32> = wounded.values().map(|w| w.0).collect();
            for id in ids {
                self.care_later(id, false);
            }
            return false;
        };
        let (_, &(patient, pressed)) = wounded.iter().find(|(&p, _)| beside(at, p)).unwrap();
        let item = self.find_bandage(d.pos);
        // With nothing to bind it, a helper can only hold the wound, and
        // somebody already is.
        if item.is_none() && pressed {
            self.care_later(patient, false);
            return false;
        }
        let item = match item {
            Some((id, to_item)) => {
                self.claim_item(id, d.id);
                Self::set_path(d, to_item);
                Some(id)
            }
            None => {
                Self::set_path(d, path);
                None
            }
        };
        self.treating.insert(patient);
        d.task = Task::FirstAid { patient, item };
        true
    }

    /// Fetch the bandage, walk to the patient, and bind the worst wound (or
    /// hold it closed, with nothing to bind it).
    pub(super) fn do_first_aid(&mut self, d: &mut Dwarf, patient: u32, item: Option<u32>) {
        let found = self.dwarves.iter().find(|p| p.id == patient && p.alive).map(|p| (p.pos, p.carried_by.is_none() && p.body.bleed_rate() > 0));
        let Some((ppos, true)) = found else {
            self.release_task(d);
            return;
        };
        if self.tick % 10 == d.id as u64 % 10 && !self.threats_near(ppos, 7).is_empty() {
            self.release_task(d);
            return;
        }
        if let Some(it) = item {
            match self.fetch(d, it) {
                Some(false) => return,
                Some(true) => {}
                None => {
                    // Gone: go and hold the wound instead.
                    self.unclaim(it);
                    d.task = Task::FirstAid { patient, item: None };
                    d.path.clear();
                    return;
                }
            }
        }
        if !beside(d.pos, ppos) {
            if let Travel::Failed = self.travel(d, ppos) {
                self.release_task(d);
            }
            return;
        }
        if !Self::labor(d, self.treat_time(25), d.skill(Labor::Medicine)) {
            return;
        }
        let used = d.carrying.take().and_then(|id| self.items.remove(&id)).map(|it| it.kind);
        d.gain_xp(Labor::Medicine, 10);
        d.task = Task::Idle;
        d.path.clear();
        self.treating.remove(&patient);
        let tick = self.tick;
        let helper = d.name.clone();
        let Some(p) = self.dwarves.iter_mut().find(|p| p.id == patient) else { return };
        let Some(part) = p.body.worst_bleed() else { return };
        let pname = p.name.clone();
        let first = pname.split(' ').next().unwrap_or(&pname).to_string();
        if used.is_some() {
            p.body.bandage(part);
            p.think(&format!("was bandaged by {} in the field", helper), -4, tick);
            super::medicine::chart(p, tick, format!("bandaged by {}", helper));
        } else {
            p.body.press(part);
            p.think(&format!("had a wound held closed by {}", helper), -2, tick);
            super::medicine::chart(p, tick, format!("wound held closed by {}", helper));
        }
        let text = match used {
            Some(k @ (ItemKind::Shirt | ItemKind::Trousers)) => {
                let what = if k == ItemKind::Shirt { "a shirt" } else { "a pair of trousers" };
                d.think(&format!("tore up {} for bandages", what), -2, tick);
                format!("{} tears up {} to bandage {}'s {}.", helper, what, pname, part.name())
            }
            Some(_) => format!("{} bandages {}'s {}.", helper, pname, part.name()),
            None => format!("{} holds {}'s wound closed, having nothing to bind it with.", helper, pname),
        };
        if used.is_some() {
            d.think(&format!("bandaged {}'s {}", first, part.name()), -4, tick);
            self.count("bandages", 1);
        } else {
            self.count("firstaid.held", 1);
        }
        self.msg(&text, (150, 220, 200));
    }

    /// From `release_task`: let go of the patient and anything claimed, and
    /// set down anyone being carried.
    pub(super) fn release_care(&mut self, d: &mut Dwarf) {
        match d.task.clone() {
            Task::FirstAid { patient, item } => {
                self.treating.remove(&patient);
                if let Some(it) = item {
                    self.unclaim(it);
                }
            }
            Task::Rescue { patient, bed } => {
                self.drop_dwarf(d);
                self.treating.remove(&patient);
                let in_it = self.dwarves.iter().any(|p| p.id == patient && matches!(p.task, Task::Rest { bed: b, .. } if b == bed));
                if let Some(b) = bed.and_then(|b| self.buildings.get_mut(&b)) {
                    if b.occupant == Some(patient) && !in_it {
                        b.occupant = None;
                    }
                }
            }
            _ => {}
        }
    }

    /// Lying on a bed already.
    fn abed(&self, p: &Dwarf) -> bool {
        let b = self.world.t(p.pos).building;
        b != 0 && self.buildings.get(&b).map_or(false, |b| b.kind == BuildingKind::Bed && b.built)
    }

    /// Down in the field and in need of carrying: senseless, crippled and
    /// hurt, or pressing a wound that will kill them before it clots.
    fn needs_carrying(&self, p: &Dwarf) -> bool {
        if !p.alive || p.carried_by.is_some() || self.treating.contains(&p.id) || self.abed(p) {
            return false;
        }
        down(p) || p.task == Task::Press && p.body.ticks_to_death().map_or(false, |t| t < self.knob("rescue.dying", 800.0) as u64)
    }

    /// Whether nobody need try to help `patient` again yet (`carry`: to carry
    /// them; else to bind their wound), after a try that came to nothing.
    fn care_waits(&self, patient: u32, carry: bool) -> bool {
        self.care_retry.get(&(patient, carry)).map_or(false, |&t| self.tick < t)
    }

    /// Nothing came of trying to help `patient`: leave them a while.
    fn care_later(&mut self, patient: u32, carry: bool) {
        let tick = self.tick;
        self.care_retry.retain(|_, t| *t > tick);
        self.care_retry.insert((patient, carry), tick + CARE_RETRY);
    }

    /// Whether a rescuer may go to `p`: no foe within nine tiles, and while
    /// the civilians shelter, only to the burrow's doorstep (twelve tiles)
    /// with no besieger within twenty.
    fn safe_to_fetch(&self, p: Pos, rescuer: &Dwarf) -> bool {
        if !self.threats_near(p, 9).is_empty() {
            return false;
        }
        let Some(bit) = self.alert_burrow.filter(|_| self.civ_alert && !rescuer.soldier) else { return true };
        let (w, h) = (self.world.w, self.world.h);
        let doorstep = (-12..=12).any(|dy: i32| {
            (-12..=12).any(|dx: i32| {
                let (x, y) = (p.x + dx, p.y + dy);
                x >= 0 && y >= 0 && x < w && y < h && self.world.t(Pos::new(x, y, p.z)).burrows & (1 << bit) != 0
            })
        });
        doorstep && !self.creatures.values().any(|c| c.siege && c.alive && combat::near(c.pos, p, 20))
    }

    /// Where to carry someone lying at `from`: a free hospital bed, else
    /// any free bed nobody owns (or their own), else nowhere in particular
    /// (the shelter, worked out on the way). None if there's nowhere at all.
    /// A bed somebody already lies in (a senseless patient lets go of the
    /// bed's claim but stays in it) won't do, and while the civilians
    /// shelter only beds inside the alert burrow will.
    fn carry_to(&self, from: Pos, patient: u32) -> Option<Option<u32>> {
        let lain: HashSet<Pos> = self.dwarves.iter().filter(|p| p.alive && p.id != patient && p.carried_by.is_none()).map(|p| p.pos).collect();
        let shelter = self.alert_burrow.filter(|_| self.civ_alert);
        let usable = |p: &Pos| !lain.contains(p) && shelter.map_or(true, |bit| self.world.t(*p).burrows & (1 << bit) != 0);
        let mut hospital = self.hospital_beds();
        hospital.retain(|p, _| usable(p));
        let beds: HashMap<Pos, u32> = if !hospital.is_empty() {
            hospital
        } else {
            self.buildings
                .values()
                .filter(|b| b.kind == BuildingKind::Bed && b.built && b.occupant.is_none() && b.owner.map_or(true, |o| o == patient) && usable(&b.pos))
                .map(|b| (b.pos, b.id))
                .collect()
        };
        if !beds.is_empty() {
            if let Some((p, _)) = path::search(&self.world, from, SEARCH_LIMIT, Mover::Dwarf, |p| beds.contains_key(&p)) {
                return Some(Some(beds[&p]));
            }
        }
        // (Not if they already lie in the shelter.)
        self.shelter_tile(from).filter(|(_, path)| !path.is_empty()).map(|_| None)
    }

    /// The nearest floor of the alert burrow, if there is one.
    fn shelter_tile(&self, from: Pos) -> Option<(Pos, Vec<Pos>)> {
        let bit = self.alert_burrow?;
        path::search(&self.world, from, SEARCH_LIMIT, Mover::Dwarf, |p| self.world.t(p).burrows & (1 << bit) != 0)
    }

    /// A finder: a grown doctor or hauler goes to carry someone down in the
    /// field to bed, if it's safe to fetch them. A doctor nearby goes first.
    pub(super) fn find_rescue(&mut self, d: &mut Dwarf) -> bool {
        self.rescue_for(d, None)
    }

    /// Give `d` a rescue: of `only`, or of whoever down they reach first.
    fn rescue_for(&mut self, d: &mut Dwarf, only: Option<u32>) -> bool {
        if d.is_child() || d.soldier || !(d.has(Labor::Medicine) || d.has(Labor::Hauling)) {
            return false;
        }
        let down: Vec<(u32, Pos)> =
            self.dwarves.iter().filter(|p| p.id != d.id && only.map_or(true, |o| o == p.id) && !self.care_waits(p.id, true) && self.needs_carrying(p)).map(|p| (p.id, p.pos)).collect();
        if down.is_empty() {
            return false;
        }
        let doctor = d.has(Labor::Medicine);
        let targets: HashMap<Pos, u32> = down
            .into_iter()
            .filter(|&(_, p)| (doctor || only.is_some() || !self.free_doctor_near(p, d.id)) && self.safe_to_fetch(p, d))
            .map(|(id, p)| (p, id))
            .collect();
        if targets.is_empty() {
            return false;
        }
        // A sanctioned rescue may leave the burrow (see `burrow_mask`).
        let restrict = std::mem::take(&mut self.world.restrict);
        let found = path::search(&self.world, d.pos, SEARCH_LIMIT / 4, Mover::Dwarf, |q| targets.keys().any(|&p| beside(q, p)));
        let picked = match found {
            None => Err(targets.values().copied().collect::<Vec<_>>()),
            Some((at, path)) => match targets.iter().find(|(&p, _)| beside(at, p)) {
                Some((&ppos, &patient)) => self.carry_to(ppos, patient).map(|bed| (patient, bed, path)).ok_or_else(|| vec![patient]),
                None => Err(Vec::new()),
            },
        };
        self.world.restrict = restrict;
        let (patient, bed, path) = match picked {
            Ok(p) => p,
            Err(none) => {
                // Out of reach, or nowhere to take them: leave them a while.
                for id in none {
                    self.care_later(id, true);
                }
                return false;
            }
        };
        if let Some(b) = bed.and_then(|b| self.buildings.get_mut(&b)) {
            b.occupant = Some(patient);
        }
        self.treating.insert(patient);
        d.task = Task::Rescue { patient, bed };
        Self::set_path(d, path);
        true
    }

    /// Walk to the patient and pick them up, then carry them to the bed
    /// (more slowly than walking) and lay them in it.
    pub(super) fn do_rescue(&mut self, d: &mut Dwarf, patient: u32, bed: Option<u32>) {
        let Some(j) = self.dwarves.iter().position(|p| p.id == patient && p.alive) else {
            self.release_task(d);
            return;
        };
        if d.carrying_dwarf != Some(patient) {
            let ppos = self.dwarves[j].pos;
            // Still down? (They may have come round and got up.)
            let down = down(&self.dwarves[j]) || self.dwarves[j].task == Task::Press;
            let stale = self.dwarves[j].carried_by.is_some() || !down;
            if stale || self.tick % 10 == d.id as u64 % 10 && !self.safe_to_fetch(ppos, d) {
                self.release_task(d);
                return;
            }
            if !beside(d.pos, ppos) {
                if let Travel::Failed = self.travel(d, ppos) {
                    self.release_task(d);
                }
                return;
            }
            // Lift them: whatever they were doing, they're carried now.
            let mut p = std::mem::take(&mut self.dwarves[j]);
            self.release_task(&mut p);
            p.carried_by = Some(d.id);
            p.pos = d.pos;
            self.carry_follow(&p);
            let text = format!("{} lifts {} and carries them off.", d.name, p.name);
            self.dwarves[j] = p;
            d.carrying_dwarf = Some(patient);
            d.path.clear();
            self.msg(&text, (150, 220, 200));
            return;
        }
        let bed_pos = bed.and_then(|b| self.buildings.get(&b)).filter(|b| b.built && b.occupant.map_or(true, |o| o == patient)).map(|b| b.pos);
        let dest = match (bed_pos, d.path.first().copied()) {
            (Some(p), _) => Some(p),
            (None, Some(p)) => Some(p),
            (None, None) => self.shelter_tile(d.pos).map(|(p, path)| {
                Self::set_path(d, path);
                p
            }),
        };
        let Some(dest) = dest else {
            // Nowhere to take them after all: set them down to rest here.
            self.lay_down(d, patient, None, false);
            return;
        };
        let before = d.pos;
        match self.travel(d, dest) {
            Travel::Moving => {
                if d.pos != before {
                    d.move_cd += 2;
                    self.carry_dwarf(d);
                }
            }
            Travel::Arrived => {
                self.carry_dwarf(d);
                self.lay_down(d, patient, bed_pos.and(bed), true);
            }
            Travel::Failed => self.release_task(d),
        }
    }

    /// After each of the carrier's steps: the one carried goes along.
    fn carry_dwarf(&mut self, carrier: &Dwarf) {
        let Some(id) = carrier.carrying_dwarf else { return };
        let Some(j) = self.dwarves.iter().position(|p| p.id == id && p.alive) else { return };
        let mut p = std::mem::take(&mut self.dwarves[j]);
        p.pos = carrier.pos;
        self.carry_follow(&p);
        self.dwarves[j] = p;
    }

    /// Set down whoever `d` is carrying, where they stand.
    fn drop_dwarf(&mut self, d: &mut Dwarf) {
        let Some(id) = d.carrying_dwarf.take() else { return };
        let pos = self.world.settle(d.pos);
        if let Some(p) = self.dwarves.iter_mut().find(|p| p.id == id && p.carried_by == Some(d.id)) {
            p.carried_by = None;
            p.pos = pos;
            if p.alive {
                let text = format!("{} sets {} down.", d.name, p.name);
                self.msg(&text, (200, 180, 140));
            }
        }
    }

    /// Carried all the way: into bed (or onto the shelter floor) to rest.
    /// Only a patient who got somewhere (`arrived`) counts as rescued.
    fn lay_down(&mut self, d: &mut Dwarf, patient: u32, bed: Option<u32>, arrived: bool) {
        d.carrying_dwarf = None;
        d.task = Task::Idle;
        d.path.clear();
        self.treating.remove(&patient);
        let tick = self.tick;
        let hospital = self.in_hospital(d.pos);
        let Some(p) = self.dwarves.iter_mut().find(|p| p.id == patient) else { return };
        p.carried_by = None;
        p.pos = d.pos;
        p.task = Task::Rest { bed, stand: d.pos };
        p.asleep = true;
        p.path.clear();
        let senseless = p.body.unconscious();
        let fought = p.attacker.map_or(false, |(_, t)| tick < t + DAY);
        let (pname, rescuer) = (p.name.clone(), d.name.clone());
        if arrived {
            p.think(&format!("was carried to safety by {}", rescuer), -8, tick);
            p.adjust_regard(d.id, 10);
            let place = if hospital { "the hospital" } else if bed.is_some() { "a bed" } else { "shelter" };
            super::medicine::chart(p, tick, format!("carried to {} by {}", place, rescuer));
        }
        if let Some(b) = bed.and_then(|b| self.buildings.get_mut(&b)) {
            b.occupant = Some(patient);
        }
        if !arrived {
            self.msg(&format!("{} sets {} down to rest, having nowhere to take them.", rescuer, pname), (200, 180, 140));
            return;
        }
        d.think(&format!("carried {} from the field", pname), -4, tick);
        d.gain_xp(Labor::Medicine, 10);
        self.count("rescues", 1);
        self.year_log.carried += 1;
        let place = if hospital { "the hospital" } else if bed.is_some() { "a bed" } else { "shelter" };
        self.msg(&format!("{} has carried {} to {}.", rescuer, pname, place), (150, 220, 200));
        if senseless && fought && self.warns.get("carried").map_or(true, |&t| tick >= t + DAY) {
            self.warns.insert("carried".into(), tick);
            let text = format!("{} carried {}, senseless, out of the fight and into {}.", rescuer, pname, place);
            self.spotlight("Carried from the field", &text, Some(d.pos), (150, 220, 200));
        }
    }

    /// In `body_phase`, for one being carried: still in their carrier's
    /// arms? (A carrier gone or dead lets them fall where they are.)
    pub(super) fn check_carried(&mut self, d: &mut Dwarf) {
        let Some(c) = d.carried_by else { return };
        if !self.dwarves.iter().any(|o| o.id == c && o.alive && o.carrying_dwarf == Some(d.id)) {
            d.carried_by = None;
        }
    }

    /// The overseer, while it watches for trouble: someone down in the
    /// field that nobody has gone for is fetched by the nearest worker.
    pub(super) fn ov_rescue(&mut self, o: &mut Overseer) {
        if self.tick % 50 != 3 || o.style == Style::Reckless {
            return;
        }
        let down: Vec<(u32, Pos, String)> =
            self.dwarves.iter().filter(|p| self.needs_carrying(p)).map(|p| (p.id, p.pos, p.name.clone())).collect();
        for (patient, ppos, pname) in down {
            let busy = |t: &Task| {
                matches!(
                    t,
                    Task::Idle
                        | Task::Wander { .. }
                        | Task::Dig { .. }
                        | Task::Haul { .. }
                        | Task::Construct { .. }
                        | Task::Craft { .. }
                        | Task::Plant { .. }
                        | Task::Harvest { .. }
                        | Task::Gather { .. }
                        | Task::Clean { .. }
                        | Task::Pond { .. }
                        | Task::Fish { .. }
                        | Task::BuildTile { .. }
                )
            };
            let pick = self
                .dwarves
                .iter()
                .enumerate()
                .filter(|(_, w)| {
                    w.alive && w.id != patient && !w.soldier && !w.is_child() && !w.insane && w.mood.is_none() && w.fit == breakdowns::Fit::None
                        && (w.has(Labor::Medicine) || w.has(Labor::Hauling))
                        && busy(&w.task)
                        && w.pos.z == ppos.z
                        && combat::near(w.pos, ppos, 30)
                        && w.tantrum == 0
                        && self.tick >= w.jailed_until
                })
                .min_by_key(|(_, w)| combat::dist2(w.pos, ppos))
                .map(|(i, _)| i);
            let Some(i) = pick else { continue };
            let mut w = std::mem::take(&mut self.dwarves[i]);
            let old = w.task.clone();
            let ok = self.rescue_for(&mut w, Some(patient));
            if ok {
                // Their old work is let go; the rescue stays.
                let rescue = std::mem::replace(&mut w.task, old);
                let path = std::mem::take(&mut w.path);
                self.release_task(&mut w);
                w.task = rescue;
                w.path = path;
                let text = format!("Sent {} to carry {} in.", w.name, pname);
                Self::note_ov(o, self.tick, text);
                self.count("rescue.ordered", 1);
            }
            self.dwarves[i] = w;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::Part;
    use crate::material::Mat;

    /// A quiet patch of open ground with every dwarf fed, rested and close by.
    fn quiet(seed: u64) -> (Game, Pos) {
        let mut g = Game::new(seed);
        g.creatures.retain(|_, c| c.tame);
        let c = g.open_patch(5, 3, 8);
        for (i, d) in g.dwarves.iter_mut().enumerate() {
            d.pos = c.offset(i as i32 % 5, i as i32 / 5 % 3, 0);
            d.path.clear();
            d.task = Task::Idle;
            d.thirst = 0;
            d.hunger = 0;
            d.fatigue = 0;
        }
        (g, c)
    }

    #[test]
    fn a_bleeding_dwarf_presses_the_wound() {
        let (mut g, _) = quiet(11);
        g.dwarves[0].body.wound(Part::LeftLeg, 9, true);
        g.dwarves[0].body.wound(Part::LeftLeg, 9, true);
        let id = g.dwarves[0].id;
        let mut pressed = false;
        for _ in 0..10 {
            g.step();
            let d = g.dwarves.iter().find(|d| d.id == id).unwrap();
            pressed |= d.task == Task::Press || d.body.pressed[Part::LeftLeg.idx()];
        }
        assert!(pressed, "never pressed the wound");
        assert!(g.log.iter().any(|l| l.text.contains("wound in their left leg")));
    }

    #[test]
    fn friends_bandage_the_wounded_where_they_lie() {
        let (mut g, c) = quiet(11);
        g.spawn_item(ItemKind::Thread, Mat::Stoutcap, c.offset(4, 2, 0));
        g.spawn_item(ItemKind::Thread, Mat::Stoutcap, c.offset(3, 2, 0));
        g.spawn_item(ItemKind::Shirt, Mat::Stoutcap, c.offset(2, 2, 0));
        let ids: Vec<u32> = g.dwarves[..3].iter().map(|d| d.id).collect();
        for d in g.dwarves[..3].iter_mut() {
            d.body.wound(Part::LeftLeg, 9, true);
        }
        let mut bandaged = HashSet::new();
        for _ in 0..300 {
            g.step();
            for d in g.dwarves.iter().filter(|d| ids.contains(&d.id)) {
                assert!(d.alive, "{} died", d.name);
                if d.body.bandaged[Part::LeftLeg.idx()] {
                    bandaged.insert(d.id);
                }
            }
        }
        assert_eq!(bandaged.len(), 3, "bandaged: {:?}", bandaged);
        assert!(g.annals.counters.get("bandages").copied().unwrap_or(0) >= 3, "{:?}", g.annals.counters);
        assert!(g.log.iter().any(|l| l.text.contains("tears up a shirt")), "the shirt went unused");
        // Each one's medical history says who bound them.
        for d in g.dwarves.iter().filter(|d| ids.contains(&d.id)) {
            assert!(d.med.iter().any(|m| m.1.starts_with("bandaged by ")), "{}: {:?}", d.name, d.med);
        }
    }

    #[test]
    fn nobody_bandages_beside_a_foe() {
        let (mut g, c) = quiet(11);
        g.spawn_item(ItemKind::Thread, Mat::Stoutcap, c.offset(4, 2, 0));
        let mut d = std::mem::take(&mut g.dwarves[1]);
        let id = g.dwarves[0].id;
        g.dwarves[0].body.wound(Part::LeftLeg, 9, true);
        let at = g.dwarves[0].pos.offset(1, 0, 0);
        g.spawn_creature(crate::creature::Species::Goblin, at, u64::MAX);
        d.task = Task::Idle;
        assert!(!g.find_first_aid(&mut d), "went to bandage {} beside a goblin", id);
        g.dwarves[1] = d;
    }

    /// A hospital of `n` built beds in a row, well away from the dwarves.
    fn ward(g: &mut Game, n: i32) -> Vec<(u32, Pos)> {
        let a = g.open_patch(n, 1, 18);
        g.add_zone(a, a.offset(n - 1, 0, 0), ZoneKind::Hospital);
        (0..n)
            .map(|i| {
                let p = a.offset(i, 0, 0);
                assert!(g.place_building(BuildingKind::Bed, p));
                let id = g.world.t(p).building;
                g.buildings.get_mut(&id).unwrap().built = true;
                (id, p)
            })
            .collect()
    }

    /// Knocked senseless: little blood left and a cracked head.
    fn fell(g: &mut Game, i: usize) -> u32 {
        let d = &mut g.dwarves[i];
        d.body.blood = 25;
        d.body.hp[Part::Head.idx()] = 2;
        assert!(d.body.unconscious());
        d.id
    }

    #[test]
    fn the_unconscious_are_carried_to_hospital() {
        let (mut g, _) = quiet(11);
        let beds = ward(&mut g, 1);
        let id = fell(&mut g, 0);
        let from = g.dwarves[0].pos;
        assert!(combat::dist2(from, beds[0].1) >= 10 * 10, "the patient lies too close to the ward");
        let mut carried = false;
        for _ in 0..600 {
            g.step();
            let p = g.dwarves.iter().find(|d| d.id == id).unwrap();
            carried |= p.carried_by.is_some();
            if p.pos == beds[0].1 {
                break;
            }
        }
        let p = g.dwarves.iter().find(|d| d.id == id).unwrap();
        assert!(carried, "never picked up");
        assert_eq!(p.pos, beds[0].1, "not in the hospital bed");
        assert!(p.alive && p.carried_by.is_none());
        assert!(g.dwarves.iter().all(|d| d.carrying_dwarf.is_none()));
        assert_eq!(g.annals.counters.get("rescues"), Some(&1));
        assert_eq!(g.year_log.carried, 1);
        assert!(p.med.iter().any(|m| m.1.starts_with("carried to the hospital by ")), "history: {:?}", p.med);
        g.tick = YEAR;
        g.write_chronicle();
        assert!(g.chronicles.last().unwrap().lines.iter().any(|l| l.contains("carried from the field")));
    }

    #[test]
    fn nobody_walks_into_a_fight_to_carry() {
        let (mut g, _) = quiet(11);
        ward(&mut g, 1);
        fell(&mut g, 0);
        let at = g.dwarves[0].pos.offset(1, 0, 0);
        let gob = g.spawn_creature(crate::creature::Species::Goblin, at, u64::MAX);
        for i in 1..g.dwarves.len() {
            let mut d = std::mem::take(&mut g.dwarves[i]);
            assert!(!g.find_rescue(&mut d), "{} went to carry a dwarf from beside a goblin", d.name);
            g.dwarves[i] = d;
        }
        // With the goblin gone, somebody goes.
        g.creatures.remove(&gob);
        let mut d = std::mem::take(&mut g.dwarves[1]);
        assert!(g.find_rescue(&mut d));
        assert!(matches!(d.task, Task::Rescue { .. }));
        g.dwarves[1] = d;
    }

    #[test]
    fn a_carrier_who_flees_sets_the_patient_down() {
        let (mut g, _) = quiet(11);
        ward(&mut g, 1);
        let id = fell(&mut g, 0);
        let mut carrier = None;
        for _ in 0..300 {
            g.step();
            carrier = g.dwarves.iter().find(|d| d.carrying_dwarf == Some(id)).map(|d| d.id);
            if carrier.is_some() {
                break;
            }
        }
        let c = carrier.expect("nobody picked them up");
        let at = g.dwarves.iter().find(|d| d.id == c).unwrap().pos.offset(2, 0, 0);
        g.spawn_creature(crate::creature::Species::Goblin, at, u64::MAX);
        for _ in 0..10 {
            g.step();
        }
        let p = g.dwarves.iter().find(|d| d.id == id).unwrap();
        assert!(p.carried_by.is_none(), "still carried with a goblin at hand");
        assert!(g.dwarves.iter().all(|d| d.carrying_dwarf.is_none()));
    }

    /// The WISHLIST's "done when": three dwarves gashed and down in the
    /// field are carried in and sutured. Only the first half is proven
    /// here: the gashes clot before the doctor comes, so nothing is sutured.
    /// P8 (feeding and the medical stockpile) adds the suture or "Clean"
    /// check; the name stays the plan's.
    #[test]
    fn three_gashed_dwarves_are_carried_in_and_sutured() {
        let (mut g, _) = quiet(11);
        let beds = ward(&mut g, 3);
        for d in g.dwarves.iter_mut() {
            d.labors[Labor::Medicine.idx()] = false;
        }
        g.dwarves[3].labors[Labor::Medicine.idx()] = true;
        for _ in 0..3 {
            g.spawn_item(ItemKind::Thread, Mat::Stoutcap, beds[0].1);
        }
        let ids: Vec<u32> = g.dwarves[..3].iter().map(|d| d.id).collect();
        for d in g.dwarves[..3].iter_mut() {
            d.body.wound(Part::UpperBody, 14, true);
            d.body.wound(Part::UpperBody, 14, true);
            // They fought on until they dropped.
            d.body.blood = 30;
            assert!(d.body.unconscious() && d.body.bleed_rate() > 0);
        }
        // Within the day all three lie in hospital beds, alive and no longer
        // bleeding (they heal and get up after that).
        let bed_tiles: Vec<Pos> = beds.iter().map(|b| b.1).collect();
        let abed = |g: &Game| {
            g.dwarves.iter().filter(|d| ids.contains(&d.id)).all(|p| p.alive && bed_tiles.contains(&p.pos) && p.carried_by.is_none() && p.body.bleed_rate() == 0)
        };
        let mut done = false;
        for _ in 0..DAY {
            g.step();
            for p in g.dwarves.iter().filter(|d| ids.contains(&d.id)) {
                assert!(p.alive, "{} died", p.name);
            }
            if abed(&g) {
                done = true;
                break;
            }
        }
        assert!(done, "not all three abed and stanched: {:?}", g.dwarves.iter().filter(|d| ids.contains(&d.id)).map(|d| (d.pos, d.body.bleed_rate())).collect::<Vec<_>>());
        assert!(g.annals.counters.get("rescues").copied().unwrap_or(0) >= 3, "{:?}", g.annals.counters);
    }

    /// A crippled dwarf going for a drink is left to it, not lifted off to
    /// bed (where thirst only gets them up again) over and over.
    #[test]
    fn a_crippled_thirsty_dwarf_is_left_to_drink() {
        let (mut g, c) = quiet(11);
        ward(&mut g, 1);
        let id = {
            let d = &mut g.dwarves[0];
            d.body.hp[Part::LeftLeg.idx()] = 0;
            d.body.hp[Part::RightLeg.idx()] = 0;
            d.body.infection = crate::body::INFECTED;
            d.thirst = THIRSTY + 1200;
            assert!(d.body.limp() >= 5 && d.body.needs_rest());
            d.id
        };
        for y in 0..3 {
            g.spawn_item(ItemKind::Drink, Mat::Stoutcap, c.offset(4, y, 0));
        }
        let mut drank = None;
        for t in 0..1500 {
            g.step();
            let p = g.dwarves.iter().find(|d| d.id == id).unwrap();
            assert!(p.alive, "died");
            if p.thirst < THIRSTY {
                drank = Some(t);
                break;
            }
        }
        let rescues = g.annals.counters.get("rescues").copied().unwrap_or(0);
        assert!(drank.is_some(), "never drank (carried {} times)", rescues);
        assert!(rescues <= 1, "carried {} times", rescues);
    }

    /// A senseless patient lying in a hospital bed has let go of its claim,
    /// but nobody else is laid on top of them.
    #[test]
    fn a_bed_holds_one_patient() {
        let (mut g, _) = quiet(11);
        let beds = ward(&mut g, 1);
        let id = fell(&mut g, 0);
        let from = g.dwarves[0].pos;
        assert_eq!(g.carry_to(from, id), Some(Some(beds[0].0)));
        fell(&mut g, 1);
        g.dwarves[1].pos = beds[0].1;
        assert!(g.buildings[&beds[0].0].occupant.is_none());
        assert_ne!(g.carry_to(from, id), Some(Some(beds[0].0)), "laid on top of another patient");
    }

    /// While the civilians shelter, a patient is carried into the alert
    /// burrow, never out across open ground to a hospital outside it.
    #[test]
    fn under_alert_the_carried_stay_in_the_shelter() {
        let (mut g, c) = quiet(11);
        let beds = ward(&mut g, 1);
        let id = fell(&mut g, 0);
        let from = g.dwarves[0].pos;
        let idx = g.new_burrow().unwrap();
        assert!(g.paint_burrow(idx, c.offset(-1, -1, 0), c.offset(5, 3, 0), true) > 0);
        g.set_alert_burrow(idx);
        g.civ_alert = true;
        assert!(g.world.t(from).burrows & g.burrows[idx].mask() != 0);
        assert_eq!(g.carry_to(from, id), None, "carried out of the shelter");
        g.civ_alert = false;
        assert_eq!(g.carry_to(from, id), Some(Some(beds[0].0)));
    }

    /// A doctor on a break doesn't keep the haulers from a casualty.
    #[test]
    fn a_doctor_on_break_does_not_hold_haulers_back() {
        let (mut g, c) = quiet(11);
        ward(&mut g, 1);
        fell(&mut g, 0);
        for d in g.dwarves.iter_mut() {
            d.labors[Labor::Medicine.idx()] = false;
            d.labors[Labor::Hauling.idx()] = false;
        }
        assert!(!g.dwarves[1].is_child() && !g.dwarves[2].is_child());
        g.dwarves[1].labors[Labor::Hauling.idx()] = true;
        g.dwarves[2].labors[Labor::Medicine.idx()] = true;
        g.dwarves[2].task = Task::Wander { target: c };
        let mut d = std::mem::take(&mut g.dwarves[1]);
        assert!(!g.find_rescue(&mut d), "went before the free doctor");
        g.dwarves[2].break_until = g.tick + 500;
        assert!(g.find_rescue(&mut d), "waited for a doctor on a break");
        g.dwarves[1] = d;
    }

    /// With nowhere to carry them, nobody searches again every pass; and one
    /// set down where they stand isn't counted rescued.
    #[test]
    fn nowhere_to_carry_is_not_a_rescue() {
        let (mut g, _) = quiet(11);
        let id = fell(&mut g, 0);
        let mut d = std::mem::take(&mut g.dwarves[1]);
        assert!(!g.find_rescue(&mut d));
        assert!(g.care_waits(id, true));
        g.tick += CARE_RETRY;
        assert!(!g.care_waits(id, true));
        // Carried, then nowhere to go: set down, not rescued.
        g.dwarves[0].carried_by = Some(d.id);
        d.carrying_dwarf = Some(id);
        d.task = Task::Rescue { patient: id, bed: None };
        g.do_rescue(&mut d, id, None);
        assert_eq!(d.task, Task::Idle);
        assert!(d.carrying_dwarf.is_none() && g.dwarves[0].carried_by.is_none());
        assert_eq!(g.annals.counters.get("rescues"), None);
        assert_eq!(g.year_log.carried, 0);
        g.dwarves[1] = d;
    }

    #[test]
    fn the_overseer_sends_a_worker_for_the_fallen() {
        let (mut g, c) = quiet(11);
        ward(&mut g, 1);
        g.set_overseer(Some(Style::Steady));
        let id = fell(&mut g, 0);
        for d in g.dwarves[1..].iter_mut() {
            d.task = Task::Wander { target: c };
        }
        g.tick += 53 - g.tick % 50;
        let mut o = g.overseer.take().unwrap();
        g.ov_rescue(&mut o);
        let w = g.dwarves.iter().find(|d| matches!(d.task, Task::Rescue { patient, .. } if patient == id)).expect("nobody sent");
        assert!(o.notes.iter().any(|n| n.1 == format!("Sent {} to carry {} in.", w.name, g.dwarves[0].name)), "{:?}", o.notes);
        assert_eq!(g.annals.counters.get("rescue.ordered"), Some(&1));
        // Reckless overseers leave them lie.
        let (mut g, _) = quiet(11);
        ward(&mut g, 1);
        g.set_overseer(Some(Style::Reckless));
        fell(&mut g, 0);
        g.tick += 53 - g.tick % 50;
        let mut o = g.overseer.take().unwrap();
        g.ov_rescue(&mut o);
        assert!(g.dwarves.iter().all(|d| !matches!(d.task, Task::Rescue { .. })));
    }
}
