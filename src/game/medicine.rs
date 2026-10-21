//! Doctors, hospitals and treatment: sutures, cleaning, splints and crutches.

use super::{Game, Travel, ZoneKind, HUNGRY, SEARCH_LIMIT, THIRSTY};
use crate::body::{BleedParams, Body, Part, PARTS};
use crate::dwarf::{Dwarf, Labor, Task};
use crate::geom::Pos;
use crate::items::{Item, ItemKind};
use crate::path;
use crate::world::Mover;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Treatment {
    /// Close bleeding wounds (needs thread).
    Suture,
    /// Wash out a dirty wound so it doesn't fester.
    Clean,
    /// Set a mangled limb (needs a splint).
    Splint,
    /// Fit a crutch for a lost leg (needs a crutch).
    Crutch,
    /// Bring a bucket of well water to someone lying in bed.
    Water,
    /// Bring drink (or a meal) to someone lying in bed.
    Feed,
}

impl Treatment {
    pub fn name(self) -> &'static str {
        match self {
            Treatment::Suture => "suturing",
            Treatment::Clean => "cleaning wounds",
            Treatment::Splint => "setting a bone",
            Treatment::Crutch => "fitting a crutch",
            Treatment::Water => "bringing water",
            Treatment::Feed => "bringing food and drink",
        }
    }

    pub fn supply(self) -> Option<ItemKind> {
        match self {
            Treatment::Suture => Some(ItemKind::Thread),
            Treatment::Clean => None,
            Treatment::Splint => Some(ItemKind::Splint),
            Treatment::Crutch => Some(ItemKind::Crutch),
            Treatment::Water => Some(ItemKind::Bucket),
            // (Drink or food, whichever the patient wants: see `feeds`.)
            Treatment::Feed => None,
        }
    }

    fn work(self) -> u32 {
        match self {
            Treatment::Suture => 40,
            Treatment::Clean => 30,
            Treatment::Splint => 70,
            Treatment::Crutch => 30,
            Treatment::Water => 15,
            Treatment::Feed => 15,
        }
    }
}

/// Whether `it` would do for a patient abed who is `thirsty` and/or `hungry`.
fn feeds(thirsty: bool, hungry: bool, it: &Item) -> bool {
    thirsty && it.kind == ItemKind::Drink || hungry && it.is_food()
}

/// How many lines of a dwarf's medical history are kept.
const MED_LINES: usize = 20;

/// Write a line in a dwarf's medical history.
pub(crate) fn chart(d: &mut Dwarf, tick: u64, text: String) {
    d.med.push((tick, text));
    if d.med.len() > MED_LINES {
        d.med.remove(0);
    }
}

/// What a blow did to a body, for the history: "gashed in the left leg",
/// "lost the right arm". None if it did no harm.
pub fn wound_words(before: &Body, after: &Body) -> Option<String> {
    let i = (0..PARTS.len()).filter(|&i| after.hp[i] < before.hp[i]).max_by_key(|&i| before.hp[i] - after.hp[i])?;
    let part = PARTS[i].name();
    let opened = after.bleeds[i] > before.bleeds[i];
    Some(if after.lost[i] && !before.lost[i] {
        format!("lost the {}", part)
    } else if after.hp[i] <= 0 && before.hp[i] > 0 {
        format!("had the {} mangled", part)
    } else if (before.hp[i] - after.hp[i]) * 3 >= after.max[i] {
        format!("{} in the {}", if opened { "gashed" } else { "badly bruised" }, part)
    } else {
        format!("{} on the {}", if opened { "cut" } else { "bruised" }, part)
    })
}

fn leg(p: Part) -> bool {
    matches!(p, Part::LeftLeg | Part::RightLeg)
}

impl Game {
    /// How wounds bleed and clot, from the knobs (read once, then kept).
    pub(crate) fn bleed_params(&self) -> BleedParams {
        if let Some(p) = self.bleed_params.get() {
            return p;
        }
        let d = BleedParams::default();
        let k = |name: &'static str, v: i32| self.knob(name, v as f64).round() as i32;
        let p = BleedParams {
            cut: k("bleed.cut", d.cut),
            gash: k("bleed.gash", d.gash),
            mangle: k("bleed.mangle", d.mangle),
            sever: k("bleed.sever", d.sever),
            vital_pct: (self.knob("bleed.vital", d.vital_pct as f64 / 100.0) * 100.0).round() as i32,
            clot_still: k("bleed.clot_still", d.clot_still),
            clot_moving: k("bleed.clot_moving", d.clot_moving),
            // Only while the gore hurts are on (`gore.harm`): otherwise an
            // artery is told but bleeds as the wound does.
            artery_floor: if self.knob("gore.harm", 0.0) > 0.0 { k("gore.artery_floor", 0) } else { 0 },
        };
        self.bleed_params.set(Some(p));
        p
    }

    /// How strongly the overseer feels its wounded (the `Wounded` sense):
    /// nothing for one, rising from two dwarves bleeding or abed with
    /// wounds, full at five.
    pub(super) fn wounded_sense(&self) -> f64 {
        let n = self
            .dwarves
            .iter()
            .filter(|d| d.alive && (d.body.bleed_rate() > 0 || matches!(d.task, Task::Rest { .. }) && d.body.needs_rest()))
            .count();
        if n < 2 {
            0.0
        } else {
            ((n - 1) as f64 / 4.0).min(1.0)
        }
    }
}

/// What a patient still needs, most urgent first.
pub fn needed(d: &Dwarf) -> Vec<Treatment> {
    let b = &d.body;
    let mut v = Vec::new();
    if b.bleeding > 0 {
        v.push(Treatment::Suture);
    }
    if b.dirty {
        v.push(Treatment::Clean);
    }
    if PARTS.iter().any(|p| p.is_limb() && !b.lost[p.idx()] && b.hp[p.idx()] <= 0 && !b.splint[p.idx()]) {
        v.push(Treatment::Splint);
    }
    if !b.crutch && PARTS.iter().any(|&p| leg(p) && b.lost[p.idx()]) {
        v.push(Treatment::Crutch);
    }
    v
}

impl Game {
    /// Base time for a treatment: a chief medical dwarf runs a tighter
    /// hospital, the more so the better they suit the post.
    pub(super) fn treat_time(&self, base: u32) -> u32 {
        match self.post_fit(super::Position::ChiefMedic) {
            Some(super::Fit::Good) => base * 3 / 5,
            Some(super::Fit::Fair) => base * 3 / 4,
            Some(super::Fit::Poor) => base * 9 / 10,
            None => base,
        }
    }

    /// Hospital beds are shared: nobody owns them.
    pub(super) fn in_hospital(&self, p: Pos) -> bool {
        self.zone_kind_at(p) == Some(ZoneKind::Hospital)
    }

    /// Beds a wounded dwarf should head for: hospital beds when there are any free.
    pub(super) fn hospital_beds(&self) -> HashMap<Pos, u32> {
        self.buildings
            .values()
            .filter(|b| b.kind == crate::buildings::BuildingKind::Bed && b.built && b.occupant.is_none() && self.in_hospital(b.pos))
            .map(|b| (b.pos, b.id))
            .collect()
    }

    /// A doctor looks for a patient lying down who needs something they can give.
    pub(super) fn find_patient(&mut self, d: &mut Dwarf) -> bool {
        let mut patients: HashMap<Pos, (u32, Treatment, (bool, bool))> = HashMap::new();
        let wells = !self.working_wells().is_empty();
        for p in self.dwarves.iter().filter(|p| p.alive && p.id != d.id && !self.treating.contains(&p.id)) {
            let lying = (p.asleep && matches!(p.task, Task::Rest { .. })) || p.body.unconscious() || p.task == Task::Catatonic;
            if !lying {
                continue;
            }
            let mut wants = needed(p);
            // With a well, nobody has to get out of bed for a drink.
            if wells && p.thirst > THIRSTY / 2 {
                wants.push(Treatment::Water);
            }
            // Without a well (or a bucket), drink or a meal brought to the bed.
            let hunger = (p.thirst > THIRSTY / 2, p.hunger > HUNGRY / 2);
            if !p.body.unconscious() && (hunger.0 || hunger.1) {
                wants.push(Treatment::Feed);
            }
            for t in wants {
                let have = match t {
                    Treatment::Feed => self.items.values().any(|it| it.free() && feeds(hunger.0, hunger.1, it)),
                    _ => t.supply().map_or(true, |k| self.items.values().any(|it| it.kind == k && it.free())),
                };
                if have {
                    patients.insert(p.pos, (p.id, t, hunger));
                    break;
                }
            }
        }
        if patients.is_empty() {
            return false;
        }
        let near = |q: Pos| patients.keys().any(|p| p.z == q.z && (p.x - q.x).abs() <= 1 && (p.y - q.y).abs() <= 1);
        let Some((at, _)) = path::search(&self.world, d.pos, SEARCH_LIMIT, Mover::Dwarf, near) else { return false };
        let (_, &(patient, what, (thirsty, hungry))) =
            patients.iter().find(|(p, _)| p.z == at.z && (p.x - at.x).abs() <= 1 && (p.y - at.y).abs() <= 1).unwrap();
        let mut item = None;
        if what == Treatment::Feed {
            // Drink first: thirst kills sooner.
            let drink = self.find_item(d.pos, |it| feeds(thirsty, false, it));
            let Some((id, path)) = drink.or_else(|| self.find_item(d.pos, |it| feeds(false, hungry, it))) else { return false };
            self.claim_item(id, d.id);
            Self::set_path(d, path);
            item = Some(id);
        } else if let Some(kind) = what.supply() {
            let Some((id, path)) = self.find_item(d.pos, |it| it.kind == kind) else { return false };
            self.claim_item(id, d.id);
            Self::set_path(d, path);
            item = Some(id);
        } else if what == Treatment::Clean {
            // Soap if there is any: it clears infection outright.
            if let Some((id, path)) = self.find_item(d.pos, |it| it.kind == ItemKind::Soap) {
                self.claim_item(id, d.id);
                Self::set_path(d, path);
                item = Some(id);
            }
        }
        self.treating.insert(patient);
        d.task = Task::Treat { patient, what, item };
        true
    }

    pub(super) fn do_treat(&mut self, d: &mut Dwarf, patient: u32, what: Treatment, item: Option<u32>) {
        let Some(ppos) = self.dwarves.iter().find(|p| p.id == patient && p.alive).map(|p| p.pos) else {
            self.release_task(d);
            return;
        };
        if let Some(item) = item {
            match self.fetch(d, item) {
                None => {
                    self.release_task(d);
                    return;
                }
                Some(false) => return,
                Some(true) => {}
            }
        }
        if what == Treatment::Water && item.map_or(false, |b| self.items.get(&b).map_or(false, |it| it.label.is_none())) {
            // Fill the bucket at the nearest well first.
            let Some(stand) = self.well_stand(d.pos) else {
                self.release_task(d);
                return;
            };
            match self.travel(d, stand) {
                Travel::Moving => {}
                Travel::Failed => self.release_task(d),
                Travel::Arrived => {
                    if Self::labor(d, 20, 0) {
                        if let Some(it) = item.and_then(|b| self.items.get_mut(&b)) {
                            it.label = Some("water".into());
                        }
                    }
                }
            }
            return;
        }
        let beside = ppos.z == d.pos.z && (ppos.x - d.pos.x).abs() <= 1 && (ppos.y - d.pos.y).abs() <= 1;
        if !beside {
            // Walk to the patient (who may have moved).
            if let Travel::Failed = self.travel(d, ppos) {
                self.release_task(d);
            }
            return;
        }
        if !Self::labor(d, self.treat_time(what.work()), d.skill(Labor::Medicine)) {
            return;
        }
        let brought = item.and_then(|id| self.items.get(&id)).map(|it| (it.kind, it.name()));
        if let Some(id) = d.carrying.take() {
            if what == Treatment::Water {
                // The bucket is set down empty, ready for next time.
                let pos = self.world.settle(d.pos);
                if let Some(it) = self.items.get_mut(&id) {
                    it.label = None;
                    it.carried_by = None;
                    it.claimed_by = None;
                    it.pos = pos;
                }
            } else {
                self.items.remove(&id);
            }
        }
        let doctor = d.name.clone();
        let skill = d.skill(Labor::Medicine);
        d.gain_xp(Labor::Medicine, 25);
        d.task = Task::Idle;
        self.treating.remove(&patient);
        let tick = self.tick;
        if what == Treatment::Feed {
            let Some((kind, name)) = brought else { return };
            d.think("cared for the wounded", -3, tick);
            let Some(p) = self.dwarves.iter_mut().find(|p| p.id == patient) else { return };
            let drink = kind == ItemKind::Drink;
            if drink {
                p.thirst = 0;
                p.think("was brought a drink while abed", -5, tick);
            } else {
                p.hunger = 0;
                p.think("was brought a meal while abed", -5, tick);
            }
            let pname = p.name.clone();
            self.count(if drink { "abed.drinks" } else { "abed.meals" }, 1);
            self.msg(&format!("{} brings {} to {} in bed.", doctor, name.to_lowercase(), pname), (150, 220, 200));
            return;
        }
        let Some(p) = self.dwarves.iter_mut().find(|p| p.id == patient) else { return };
        let b = &mut p.body;
        match what {
            Treatment::Suture => b.suture(),
            Treatment::Clean => {
                b.dirty = false;
                b.infection = if item.is_some() { 0 } else { b.infection / 2 };
            }
            Treatment::Splint => {
                for part in PARTS {
                    if part.is_limb() && !b.lost[part.idx()] && b.hp[part.idx()] <= 0 {
                        b.splint[part.idx()] = true;
                    }
                }
            }
            Treatment::Crutch => b.crutch = true,
            Treatment::Water => p.thirst = 0,
            Treatment::Feed => {}
        }
        let done = match what {
            Treatment::Suture => "wounds sutured",
            Treatment::Clean => "wounds cleaned",
            Treatment::Splint => "a bone set",
            Treatment::Crutch => "fitted with a crutch",
            Treatment::Water | Treatment::Feed => "",
        };
        if !done.is_empty() {
            chart(p, tick, format!("{} by {}", done, doctor));
        }
        let pname = p.name.clone();
        p.think(&format!("was cared for by {}", doctor), -6 - skill as i32, tick);
        self.msg(&format!("{} finished {} for {}.", doctor, what.name(), pname), (150, 220, 200));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::DAY;
    use crate::material::Mat;

    /// Wound one dwarf badly and see how long the bleeding lasts.
    fn bleed_out(with_doctor: bool) -> (u64, i32) {
        let mut g = Game::new(81);
        let e = g.embark;
        // Only dwarf 1 may doctor, and only in the treated run.
        for d in g.dwarves.iter_mut() {
            d.labors[Labor::Medicine.idx()] = false;
        }
        if with_doctor {
            g.dwarves[1].labors[Labor::Medicine.idx()] = true;
            g.spawn_item(ItemKind::Thread, Mat::Stoutcap, e);
            g.dwarves[1].pos = e;
        }
        let patient = g.dwarves[0].id;
        g.dwarves[0].pos = e.offset(1, 0, 0);
        g.dwarves[0].body.bleeds[Part::UpperBody.idx()] = 40;
        g.dwarves[0].body.bleeding = 4;
        g.dwarves[0].body.dirty = true;
        let start = g.tick;
        for _ in 0..DAY {
            g.step();
            let p = g.dwarves.iter().find(|d| d.id == patient).unwrap();
            if p.body.bleeding == 0 {
                return (g.tick - start, p.body.blood);
            }
        }
        panic!("still bleeding after a day");
    }

    #[test]
    fn a_doctor_stops_bleeding_sooner() {
        let (untreated, blood_untreated) = bleed_out(false);
        let (treated, blood_treated) = bleed_out(true);
        assert!(treated < untreated, "sutured after {} ticks, untreated stopped after {}", treated, untreated);
        assert!(blood_treated > blood_untreated, "treated lost more blood ({} vs {})", blood_treated, blood_untreated);
    }

    #[test]
    fn a_patient_in_bed_is_brought_ale() {
        // No well: the doctor carries drink from the stores to the bed.
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        assert!(g.working_wells().is_empty());
        let e = g.embark;
        g.spawn_item(ItemKind::Drink, Mat::Stoutcap, e.offset(2, 0, 0));
        for d in g.dwarves.iter_mut() {
            d.labors[Labor::Medicine.idx()] = false;
        }
        g.dwarves[1].labors[Labor::Medicine.idx()] = true;
        let id = g.dwarves[0].id;
        let arm = Part::LeftArm.idx();
        {
            let d = &mut g.dwarves[0];
            d.body.hp[arm] = 1;
            d.thirst = THIRSTY / 2 + 50;
            d.task = Task::Rest { bed: None, stand: d.pos };
            d.asleep = true;
        }
        let mut brought = false;
        for _ in 0..DAY / 2 {
            g.step();
            let d = g.dwarves.iter_mut().find(|d| d.id == id).unwrap();
            d.body.hp[arm] = 1;
            d.hunger = 0;
            assert!(matches!(d.task, Task::Rest { .. }), "the patient got up: {:?}", d.task);
            if d.thirst < 100 {
                brought = true;
                break;
            }
        }
        assert!(brought, "nobody brought the patient a drink");
        let d = g.dwarves.iter().find(|d| d.id == id).unwrap();
        assert!(d.thoughts.iter().any(|t| t.text.contains("brought a drink while abed")));
        assert!(g.log.iter().any(|m| m.text.contains("in bed")));
        assert_eq!(g.annals.counters.get("abed.drinks"), Some(&1));
    }

    #[test]
    fn history_records_care() {
        // A wolf's bite and the doctor's thread both go in the history.
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let e = g.embark;
        for d in g.dwarves.iter_mut() {
            d.labors[Labor::Medicine.idx()] = false;
        }
        g.dwarves[1].labors[Labor::Medicine.idx()] = true;
        g.dwarves[1].pos = e;
        g.spawn_item(ItemKind::Thread, Mat::Stoutcap, e);
        let doctor = g.dwarves[1].name.clone();
        let wolf = crate::creature::Creature::new(9999, crate::creature::Species::Wolf, e, 0);
        let id = g.dwarves[0].id;
        let mut d = std::mem::take(&mut g.dwarves[0]);
        d.pos = e.offset(1, 0, 0);
        let before = d.body.clone();
        d.body.wound(Part::LeftLeg, 9, true);
        g.note_wound(&mut d, &wolf, &before);
        // (Lying still, as the wounded do once helped to bed.)
        d.task = Task::Rest { bed: None, stand: d.pos };
        d.asleep = true;
        g.dwarves[0] = d;
        assert_eq!(g.dwarves[0].med.last().map(|m| m.1.as_str()), Some("gashed in the left leg by a wolf"));
        for _ in 0..DAY / 2 {
            g.step();
            if g.dwarves.iter().find(|d| d.id == id).unwrap().body.bleed_rate() == 0 {
                break;
            }
        }
        let p = g.dwarves.iter().find(|d| d.id == id).unwrap();
        let lines: Vec<&str> = p.med.iter().map(|m| m.1.as_str()).collect();
        assert!(lines.contains(&format!("wounds sutured by {}", doctor).as_str()), "history: {:?}", lines);
        // The history keeps only the last few lines.
        let mut p = p.clone();
        for i in 0..30 {
            chart(&mut p, i, format!("line {}", i));
        }
        assert_eq!(p.med.len(), MED_LINES);
        assert_eq!(p.med.last().unwrap().1, "line 29");
    }

    #[test]
    fn the_overseer_feels_its_bleeding_wounded() {
        let mut g = Game::new(5);
        g.set_overseer(Some(crate::game::Style::Steady));
        let wounded = |g: &Game| g.overseer_brain().unwrap().senses.iter().find(|s| s.0 == crate::game::ambitions::Sense::Wounded).unwrap().1;
        g.dwarves[0].body.wound(Part::LeftLeg, 9, true);
        assert_eq!(wounded(&g), 0.0, "one cut leg is no worry");
        for i in 1..5 {
            g.dwarves[i].body.wound(Part::LowerBody, 12, true);
        }
        assert_eq!(wounded(&g), 1.0);
        for d in g.dwarves.iter_mut() {
            d.body.suture();
        }
        assert_eq!(wounded(&g), 0.0, "sutured, nobody is bleeding");
    }

    #[test]
    fn dirty_wounds_fester_unless_cleaned() {
        let mut b = crate::body::Body::new(1.0);
        b.wound(Part::LeftArm, 8, true);
        assert!(b.dirty, "an open cut should be dirty");
        b.suture();
        for _ in 0..400 {
            b.tick();
        }
        assert!(b.infected(), "a dirty wound left alone never festered");
        let hp = b.hp[Part::LeftArm.idx()];
        b.heal(5);
        assert_eq!(b.hp[Part::LeftArm.idx()], hp, "infected wounds shouldn't heal");
        b.dirty = false;
        for _ in 0..400 {
            b.tick();
        }
        assert!(!b.infected(), "a cleaned wound should recover from infection");
    }
}
