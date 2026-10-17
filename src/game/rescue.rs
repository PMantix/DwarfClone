//! First aid in the field (S02, HLT-07): a dwarf bleeding hard stops work and
//! presses the wound, and friends and passers-by bind it where they lie.

use super::*;

/// Standing on or next to `b`, on the same level.
fn beside(a: Pos, b: Pos) -> bool {
    a.z == b.z && (a.x - b.x).abs() <= 1 && (a.y - b.y).abs() <= 1
}

/// Far enough gone that a dwarf eats or drinks before anything else.
fn desperate(d: &Dwarf) -> bool {
    d.thirst > THIRSTY + 1500 || d.hunger > HUNGRY + 3000
}

impl Game {
    /// In `body_phase`: a dwarf bleeding hard, with no foe within five
    /// tiles, drops what they're doing and presses the wound (`Task::Press`).
    /// A dwarf already lying in bed just holds it.
    pub(super) fn maybe_press(&mut self, d: &mut Dwarf) {
        if d.task == Task::Press || d.body.bleed_rate() == 0 || self.tick % 5 != d.id as u64 % 5 {
            return;
        }
        let Some(worst) = d.body.worst_bleed() else { return };
        if d.asleep && d.path.is_empty() {
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

    /// A finder: any grown dwarf who doctors or hauls goes to a wounded
    /// dwarf nearby who is bleeding with nobody seeing to them and no foe
    /// within seven tiles, bringing a bandage if there's one to hand.
    pub(super) fn find_first_aid(&mut self, d: &mut Dwarf) -> bool {
        if d.is_child() || d.soldier || !(d.has(Labor::Medicine) || d.has(Labor::Hauling)) {
            return false;
        }
        let min = self.knob("firstaid.min", 2.0);
        let mut wounded: HashMap<Pos, (u32, bool)> = HashMap::new();
        for p in self.dwarves.iter() {
            if !p.alive || p.id == d.id || self.treating.contains(&p.id) {
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
        let Some((at, path)) = path::search(&self.world, d.pos, SEARCH_LIMIT / 4, Mover::Dwarf, close) else { return false };
        let (_, &(patient, pressed)) = wounded.iter().find(|(&p, _)| beside(at, p)).unwrap();
        let item = self.find_bandage(d.pos);
        // With nothing to bind it, a helper can only hold the wound, and
        // somebody already is.
        if item.is_none() && pressed {
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
        let found = self.dwarves.iter().find(|p| p.id == patient && p.alive).map(|p| (p.pos, p.body.bleed_rate() > 0));
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
        } else {
            p.body.press(part);
            p.think(&format!("had a wound held closed by {}", helper), -2, tick);
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

    /// From `release_task`: let go of the patient and anything claimed.
    pub(super) fn release_care(&mut self, d: &mut Dwarf) {
        if let Task::FirstAid { patient, item } = d.task.clone() {
            self.treating.remove(&patient);
            if let Some(it) = item {
                self.unclaim(it);
            }
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
}
