//! Old battlefields remembered (O09 step 2, WLD-12). Where a named fight
//! left many dead, the ground keeps it: the place takes a name if the
//! fight didn't give it one ("the Crow Field"), the foes' bones are left
//! where they lie, and dwarves passing think of the dead by their nature:
//! a veteran with pride, a friend of the fallen with grief, an anxious
//! one with a shudder. A year to the day after, the fort keeps the
//! anniversary: a feast on the field for a victory, a vigil for a
//! slaughter, and on that night the unquiet dead of the field may walk.
//! The dead of a field rise as ghosts sooner and more surely than others.
//! The overseer may raise a memorial slab there (a sentimental or pious
//! one always, others for a great slaughter), which lays one of the
//! field's dead to rest and is named in the legends; a vain one renames
//! the field grandly when it does.
//!
//! The fields live in `Game.stories.fields`, beside the places they name
//! (`stories.rs`). Names come from hashes; only the ghosts are chance.

use super::overseer::Overseer;
use super::personality::Trait;
use super::stories::{first_name, hash, pick, EpisodeKind, Episode, Place};
use super::*;
use serde::{Deserialize, Serialize};

/// A battlefield: where a named fight left many dead.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Field {
    /// The place's name: "Kolvar's Stand", "the Crow Field".
    pub place: String,
    /// The fights fought here, the first first: "the Rout at Copper Gate".
    pub fights: Vec<String>,
    pub at: Pos,
    pub r: i32,
    /// When the first fight here ended: its anniversary.
    pub tick: u64,
    /// Ours who died here, by name and id.
    #[serde(default)]
    pub dead: Vec<(u32, String)>,
    /// Foes who died here.
    #[serde(default)]
    pub foes: u32,
    /// Who fought here and lived.
    #[serde(default)]
    pub veterans: Vec<u32>,
    /// The memorial slab the overseer raised (a building id), once placed.
    #[serde(default)]
    pub memorial: Option<u32>,
    /// The memorial is finished and dedicated.
    #[serde(default)]
    pub dedicated: bool,
    /// The last year its anniversary was kept.
    #[serde(default)]
    pub kept: i32,
}

impl Field {
    /// More of ours died here than the field is glad of: its day is a vigil.
    pub fn mournful(&self) -> bool {
        self.dead.len() * 3 >= self.foes as usize || self.dead.len() >= 3
    }

    fn covers(&self, p: Pos) -> bool {
        (p.z - self.at.z).abs() <= 1 && (p.x - self.at.x).abs() <= self.r && (p.y - self.at.y).abs() <= self.r
    }

    /// "the Rout at Copper Gate", or "the Rout at Copper Gate and two
    /// fights since".
    fn told(&self) -> String {
        match self.fights.len() {
            0 => "a forgotten fight".to_string(),
            1 => self.fights[0].clone(),
            2 => format!("{} and {}", self.fights[0], self.fights[1]),
            n => format!("{} and {} fights since", self.fights[0], n - 1),
        }
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

fn names(list: &[(u32, String)]) -> String {
    let n: Vec<&str> = list.iter().map(|d| d.1.as_str()).collect();
    match n.len() {
        0 => String::new(),
        1 => n[0].to_string(),
        k => format!("{} and {}", n[..k - 1].join(", "), n[k - 1]),
    }
}

impl Game {
    fn year_now(&self) -> i32 {
        self.start_year + (self.tick / YEAR) as i32
    }

    /// The battlefield covering `p`, if any.
    pub fn field_at(&self, p: Pos) -> Option<&Field> {
        self.stories.fields.iter().find(|f| f.covers(p))
    }

    /// A named fight has been told (`stories.rs: tell_episode`): if it left
    /// enough dead, the ground is a battlefield from now on.
    pub(super) fn remember_field(&mut self, e: &Episode) {
        if e.kind == EpisodeKind::Hunt {
            return;
        }
        let tick = self.tick;
        let foes = match e.siege.and_then(|i| self.annals.sieges.get(i)) {
            Some(s) => s.foes_slain,
            None => self.stories.marks.iter().filter(|m| !m.died && m.tick + 10 * DAY >= e.tick && (m.at.x - e.at.x).abs() <= 6 && (m.at.y - e.at.y).abs() <= 6).count() as u32,
        };
        let dead: Vec<(u32, String)> = self.dwarves.iter().filter(|d| !d.alive && e.dead.contains(&d.called())).map(|d| (d.id, d.called())).collect();
        if ((dead.len() as u32 + foes) as f64) < self.knob("field.dead", 4.0) {
            return;
        }
        // A second fight on the same ground deepens the field.
        if let Some(f) = self.stories.fields.iter_mut().find(|f| f.covers(e.at)) {
            f.fights.push(e.name.clone());
            f.dead.extend(dead);
            f.foes += foes;
            for v in &e.veterans {
                if !f.veterans.contains(v) {
                    f.veterans.push(*v);
                }
            }
            let text = format!("{} was fought again on {}.", cap(&e.name), f.place);
            self.legend(text);
            return;
        }
        // The fight named the place already, or it is named for the dead.
        let place = match self.story_place(e.at) {
            Some(pl) => pl.name.clone(),
            None => {
                let h = hash(&e.name, e.tick ^ 0x4649_454c);
                let noun = self.place_noun(e.at);
                let noun = if matches!(noun, "Hall" | "Deeps" | "Tunnel" | "Door") { "Ground" } else { noun };
                let name = format!("the {} {}", pick(h, 7, &["Crow", "Bone", "Barrow", "Cairn", "Red", "Grim", "Ashen", "Raven"]), noun);
                let deed = format!("where {} were left lying after {}", if dead.is_empty() { format!("{} foes", foes) } else { format!("{} and {} foes", names(&dead), foes) }, e.name);
                let year = self.year_now();
                self.stories.places.push(Place { name: name.clone(), at: e.at, r: 4, deed, year, kind: super::places::PlaceKind::Battlefield, b: None, weight: 5, by_player: false });
                name
            }
        };
        let fort = self.fortress_name.clone();
        let fell = if dead.is_empty() { format!("{} foes", foes) } else { format!("{} of {} and {} foes", dead.len(), fort, foes) };
        self.legend(format!("The ground where {} was fought, where {} fell, became {}; the bones were left where they lay.", e.name, fell, place));
        self.count("fields.made", 1);
        self.stories.fields.push(Field { place, fights: vec![e.name.clone()], at: e.at, r: 5, tick, dead, foes, veterans: e.veterans.clone(), memorial: None, dedicated: false, kept: self.year_now() });
    }

    /// Daily: the foes' bones lie where they fell, the memorial is
    /// dedicated when it stands, and an anniversary is kept.
    pub(super) fn battlefield_daily(&mut self) {
        if self.stories.fields.is_empty() {
            return;
        }
        self.leave_the_bones();
        for i in 0..self.stories.fields.len() {
            self.dedicate_memorial(i);
            self.anniversary(i);
        }
    }

    /// Foes' remains in the open on a field are forbidden to haulers: the
    /// bones stay where they fell. Ours still go to the tomb.
    fn leave_the_bones(&mut self) {
        let mut forbid = Vec::new();
        for it in self.items.values() {
            if it.forbidden || !it.free() || !matches!(it.kind, ItemKind::Corpse | ItemKind::Bone | ItemKind::BodyPart) || !self.world.in_bounds(it.pos) || !self.world.outside(it.pos) {
                continue;
            }
            if self.burial_corpse(it) || it.label.as_deref().map_or(false, |l| self.dwarves.iter().any(|d| l.ends_with(&d.name))) {
                continue;
            }
            if self.field_at(it.pos).is_some() {
                forbid.push(it.id);
            }
        }
        for id in forbid {
            if let Some(it) = self.items.get_mut(&id) {
                it.forbidden = true;
            }
        }
    }

    /// The bones lying loose on field `i`.
    fn bones_on(&self, f: &Field) -> usize {
        self.items.values().filter(|it| it.forbidden && matches!(it.kind, ItemKind::Corpse | ItemKind::Bone | ItemKind::BodyPart) && f.covers(it.pos)).count()
    }

    /// Every hundred ticks: dwarves crossing a field think of its dead,
    /// each by their nature, once in a few days.
    pub(super) fn battlefield_tick(&mut self) {
        if self.stories.fields.is_empty() {
            return;
        }
        let tick = self.tick;
        for f in self.stories.fields.iter() {
            let won = !f.mournful();
            let fight = f.fights.first().cloned().unwrap_or_default();
            for d in self.dwarves.iter_mut().filter(|d| d.alive && !d.asleep && !d.is_child() && f.covers(d.pos)) {
                if d.thoughts.iter().any(|t| tick < t.tick + 5 * DAY && t.text.contains(&f.place)) {
                    continue;
                }
                let friend = f.dead.iter().filter(|(id, _)| d.family_of(*id) || d.spouse == Some(*id) || d.regard(*id) >= 30).map(|x| x.1.clone()).next();
                let (text, s) = if let Some(name) = friend {
                    (format!("grieved for {} on {}", first_name(&name), f.place), 6)
                } else if f.veterans.contains(&d.id) {
                    if won {
                        (format!("remembered the victory on {}", f.place), -5)
                    } else {
                        (format!("remembered the fallen of {} on {}", fight, f.place), -2)
                    }
                } else if d.personality.anxiety >= 25 {
                    (format!("shuddered crossing {}, where the dead lie", f.place), 4)
                } else if won {
                    (format!("walked proudly over {}", f.place), -2)
                } else {
                    (format!("walked softly over {}", f.place), 0)
                };
                d.think(&text, s, tick);
            }
        }
    }

    /// On the day the first fight here ended, each year after: a feast on
    /// the field for a victory, a vigil for a slaughter. The field's
    /// unquiet dead walk that night.
    fn anniversary(&mut self, i: usize) {
        let tick = self.tick;
        let year = self.year_now();
        let f = &self.stories.fields[i];
        if (tick % YEAR) / DAY != (f.tick % YEAR) / DAY || tick / YEAR <= f.tick / YEAR || f.kept >= year {
            return;
        }
        let f = f.clone();
        self.stories.fields[i].kept = year;
        let years = tick / YEAR - f.tick / YEAR;
        let ago = if years == 1 { "a year ago".to_string() } else { format!("{} years ago", years) };
        let fight = f.fights[0].clone();
        let alive: Vec<u32> = f.veterans.iter().copied().filter(|v| self.dwarves.iter().any(|d| d.id == *v && d.alive)).collect();
        let (title, text) = if f.mournful() {
            let who = if f.dead.is_empty() { "the fallen".to_string() } else { names(&f.dead) };
            (
                format!("The vigil of {}", f.place),
                format!("{} ago to the day, {} was fought on {}. The fort keeps a vigil there for {}.", cap(&ago), fight, f.place, who),
            )
        } else {
            (
                format!("The feast of {}", f.place),
                format!("{} ago to the day, {} was won on {}. The fort feasts on the field, and {} veteran{} tell{} it again.", cap(&ago), fight, f.place, alive.len(), if alive.len() == 1 { "" } else { "s" }, if alive.len() == 1 { "s" } else { "" }),
            )
        };
        self.msg_at(&text, if f.mournful() { (180, 190, 230) } else { (255, 215, 60) }, Some(f.at), super::logkind::LogKind::Legend);
        self.spotlight(&title, &text, Some(f.at), if f.mournful() { (180, 190, 230) } else { (255, 215, 60) });
        self.year_log.stories.push(text.clone());
        if years == 1 || years % 10 == 0 {
            self.legend(text.clone());
        }
        // Everyone thinks of it: the veterans and friends of the dead most.
        let words = if f.mournful() { format!("kept the vigil of {}", f.place) } else { format!("feasted the victory of {}", f.place) };
        for d in self.dwarves.iter_mut().filter(|d| d.alive && !d.is_child()) {
            let close = f.dead.iter().any(|(id, _)| d.family_of(*id) || d.spouse == Some(*id) || d.regard(*id) >= 30);
            let s = match (f.mournful(), close, alive.contains(&d.id)) {
                (true, true, _) => 4,
                (true, false, true) => -3,
                (true, false, false) => -1,
                (false, _, true) => -8,
                (false, _, false) => -3,
            };
            d.think(&words, s, tick);
        }
        // A victory is feasted on the very field, when it's safe to go out.
        if !f.mournful() && !self.alert && self.festival.is_none() {
            self.start_festival(super::festivals::FestivalKind::Victory);
            let ground: Vec<Pos> = (-3..=3).flat_map(|dy| (-3..=3).map(move |dx| (dx, dy))).map(|(dx, dy)| f.at.offset(dx, dy, 0)).filter(|&p| self.world.walkable(p)).collect();
            if let Some(fest) = self.festival.as_mut().filter(|_| ground.len() >= 4) {
                fest.venue = ground;
            }
        }
        if let Some(mut o) = self.overseer.take() {
            let line = if f.mournful() { format!("Kept the vigil of {}, {}.", f.place, ago) } else { format!("Feasted on {}, {} after {}.", f.place, ago, fight) };
            Self::note_ov(&mut o, tick, line);
            self.overseer = Some(o);
        }
        self.count("fields.rites", 1);
        // The unquiet dead of the field rise on its night.
        let restless: Vec<usize> = (0..self.dwarves.len()).filter(|&k| f.dead.iter().any(|x| x.0 == self.dwarves[k].id) && !self.dwarves[k].alive && !self.dwarves[k].rested && !self.dwarves[k].ghost).collect();
        if let Some(&k) = restless.first() {
            let name = self.dwarves[k].name.clone();
            let at = self.field_spot(&f).unwrap_or(f.at);
            self.dwarves[k].ghost = true;
            self.dwarves[k].pos = at;
            let text = format!("On the night of {}, the ghost of {} walks {} again!", fight, name, f.place);
            self.msg_at(&text, (180, 200, 255), Some(at), super::logkind::LogKind::Legend);
            self.year_log.stories.push(text);
            self.count("fields.ghosts", 1);
        }
    }

    /// A walkable tile near the middle of a field.
    fn field_spot(&self, f: &Field) -> Option<Pos> {
        (0..=f.r).flat_map(|r| (-r..=r).flat_map(move |k| [(k, -r), (k, r), (-r, k), (r, k)])).map(|(dx, dy)| f.at.offset(dx, dy, 0)).find(|&p| self.world.walkable(p))
    }

    /// How long a dwarf's body waits before its ghost may rise, and the
    /// daily chance it does then, and where (`family.rs: ghosts_tick`): the
    /// dead of a battlefield rise sooner, more surely, and on the field.
    pub(super) fn ghost_terms(&self, i: usize, days: u64, chance: f64) -> (u64, f64, Option<Pos>) {
        let id = self.dwarves[i].id;
        match self.stories.fields.iter().find(|f| f.dead.iter().any(|x| x.0 == id) && f.memorial.is_none()) {
            Some(f) => (days / 2, (chance * self.knob("field.ghosts", 1.8)).min(0.9), self.field_spot(f)),
            None => (days, chance, None),
        }
    }

    /// The memorial on field `i` is finished: dedicate it to one of the
    /// field's dead (a ghost first), and tell it.
    fn dedicate_memorial(&mut self, i: usize) {
        let f = &self.stories.fields[i];
        let Some(bid) = f.memorial.filter(|_| !f.dedicated) else { return };
        let Some(b) = self.buildings.get(&bid) else {
            // Torn down before it was finished: it may be raised again.
            self.stories.fields[i].memorial = None;
            return;
        };
        if !b.built {
            return;
        }
        let f = f.clone();
        self.stories.fields[i].dedicated = true;
        let restless = (0..self.dwarves.len()).filter(|&k| f.dead.iter().any(|x| x.0 == self.dwarves[k].id) && !self.dwarves[k].alive && !self.dwarves[k].rested).min_by_key(|&k| !self.dwarves[k].ghost);
        let first = f.dead.first().map(|x| x.0);
        if let Some(b) = self.buildings.get_mut(&bid) {
            if b.occupant.is_none() {
                b.occupant = restless.map(|k| self.dwarves[k].id).or(first);
            }
        }
        if let Some(k) = restless {
            let how = format!("is remembered on the memorial of {}", f.place);
            self.lay_to_rest(k, &how, false);
        }
        let carved = if f.dead.is_empty() { format!("Here {} was fought.", f.fights[0]) } else { format!("Here fell {} in {}.", names(&f.dead), f.fights[0]) };
        let text = format!("A memorial stands on {} now. Carved on it: \"{}\"", f.place, carved);
        self.msg_at(&text, (220, 220, 255), Some(f.at), super::logkind::LogKind::Legend);
        self.spotlight("A memorial", &text, Some(f.at), (220, 220, 255));
        self.year_log.stories.push(text);
        self.legend(format!("{} raised a memorial on {}: \"{}\"", self.fortress_name, f.place, carved));
        self.record(super::memory::MemoryKind::Named, &f.dead.iter().map(|x| x.0).collect::<Vec<_>>(), Some(f.at), format!("A memorial was raised on {}.", f.place), 3);
        self.count("fields.memorials", 1);
    }

    /// Daily, the overseer: a sentimental or pious one raises a memorial
    /// slab on every field where ours died, another only for a slaughter
    /// (three dead or more), a greedy one never; a vain one renames the
    /// field grandly as it does.
    pub(super) fn ov_memorial(&mut self, o: &mut Overseer) {
        let tick = self.tick;
        let wants = |f: &Field| {
            !f.dead.is_empty() && f.memorial.is_none() && !o.has(Trait::Greedy) && (o.has(Trait::Sentimental) || o.has(Trait::Pious) || f.dead.len() >= 3)
        };
        let Some(i) = self.stories.fields.iter().position(wants) else { return };
        let f = self.stories.fields[i].clone();
        // A spot on the field for the slab, nearest the middle.
        let site = (0..=f.r).flat_map(|r| (-r..=r).flat_map(move |k| [(k, -r), (k, r), (-r, k), (r, k)])).map(|(dx, dy)| f.at.offset(dx, dy, 0)).find(|&p| self.world.walkable(p) && self.can_place(BuildingKind::Slab, p));
        let Some(site) = site else { return };
        if !self.place_building(BuildingKind::Slab, site) {
            return;
        }
        let bid = self.world.t(site).building;
        self.stories.fields[i].memorial = Some(bid);
        if self.stock_of(ItemKind::Slab) == 0 && !o.boosts.iter().any(|b| b.0 == crate::buildings::Recipe::Slab) {
            o.boosts.push((crate::buildings::Recipe::Slab, 1, tick + SEASON));
        }
        let why = if o.has(Trait::Sentimental) {
            format!("{} must not be forgotten.", names(&f.dead))
        } else if o.has(Trait::Pious) {
            "the dead must have their stone, or they will not lie still.".to_string()
        } else {
            format!("{} of ours died there.", f.dead.len())
        };
        Self::note_ov(o, tick, format!("A memorial slab goes up on {}: {}", f.place, why));
        // A vain overseer gives the field a grander name.
        if o.has(Trait::Vain) {
            let h = hash(&f.place, f.tick);
            let grand = format!("the {} of {}", pick(h, 3, &["Hallowed Field", "Field of Glory", "Field of Heroes", "Proud Field"]), self.fortress_name);
            if let Some(pl) = self.stories.places.iter_mut().find(|pl| pl.b.is_none() && pl.name == f.place) {
                pl.name = grand.clone();
            }
            for g in self.stories.fields.iter_mut().filter(|g| g.place == f.place) {
                g.place = grand.clone();
            }
            Self::note_ov(o, tick, format!("{} shall be called {} from now on.", cap(&f.place), grand));
            self.legend(format!("The overseer of {} renamed {} {}.", self.fortress_name, f.place, grand));
        }
    }

    /// What the look panel says of a battlefield: who fell and what lies
    /// there.
    pub fn field_look(&self, p: Pos) -> Option<String> {
        let f = self.field_at(p)?;
        let mut s = format!("Battlefield of {}: ", f.told());
        s.push_str(&match f.dead.len() {
            0 => format!("{} foes fell here", f.foes),
            n => format!("{} of ours and {} foes fell here", n, f.foes),
        });
        let bones = self.bones_on(f);
        if bones > 0 {
            s.push_str(&format!("; {} remains lie unburied", bones));
        }
        if f.dedicated {
            s.push_str("; a memorial stands");
        } else if f.memorial.is_some() {
            s.push_str("; a memorial is going up");
        }
        s.push('.');
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quiet(seed: u64) -> Game {
        let mut g = Game::new(seed);
        g.creatures.retain(|_, c| c.tame);
        g
    }

    #[test]
    fn dwarves_passing_a_battlefield_remember_it() {
        let mut g = quiet(5);
        super::super::stories::tests::fought_siege(&mut g, 2, 4);
        let f = g.stories.fields.last().expect("the ground should be a battlefield").clone();
        assert_eq!(f.dead.len(), 2);
        assert!(g.place_at(f.at).is_some(), "the field has a name");
        assert!(g.overworld.events.iter().any(|e| e.text.contains("bones were left")), "not in the legends");
        // The look panel tells it.
        let look = g.field_look(f.at).expect("look shows the field");
        assert!(look.contains("2 of ours"), "{}", look);
        // A living dwarf walks onto it and thinks of it.
        let i = g.dwarves.iter().position(|d| d.alive).unwrap();
        g.dwarves[i].pos = f.at;
        g.dwarves[i].asleep = false;
        g.battlefield_tick();
        assert!(g.dwarves[i].thoughts.iter().any(|t| t.text.contains(&f.place)), "{:?}", g.dwarves[i].thoughts);
        // Not again the next moment.
        let n = g.dwarves[i].thoughts.len();
        g.battlefield_tick();
        assert_eq!(g.dwarves[i].thoughts.len(), n);
        // A goblin's bones on the field are left lying.
        let bone = g.spawn_item(ItemKind::Bone, Mat::Bone, f.at);
        g.items.get_mut(&bone).unwrap().label = Some("remains of goblin".into());
        let outside = g.world.outside(f.at);
        g.battlefield_daily();
        assert_eq!(g.items[&bone].forbidden, outside);
    }

    #[test]
    fn a_battle_is_kept_on_its_anniversary_and_its_dead_walk() {
        let mut g = quiet(7);
        super::super::stories::tests::fought_siege(&mut g, 4, 1);
        let f = g.stories.fields.last().expect("a field").clone();
        assert!(f.mournful());
        // The dead are not buried, and a year goes by to the day.
        g.tick = f.tick + YEAR;
        g.tick -= g.tick % DAY;
        g.battlefield_daily();
        assert_eq!(g.stories.fields.last().unwrap().kept, g.start_year + (g.tick / YEAR) as i32);
        assert!(g.year_log.stories.iter().any(|s| s.contains("vigil")), "{:?}", g.year_log.stories);
        assert!(g.dwarves.iter().any(|d| d.ghost), "the field's dead walk on its night");
        assert!(g.overworld.events.iter().any(|e| e.text.contains("to the day")));
        // Kept once a year only.
        let n = g.year_log.stories.len();
        g.battlefield_daily();
        assert_eq!(g.year_log.stories.len(), n);
    }

    #[test]
    fn a_won_field_is_feasted() {
        let mut g = quiet(5);
        g.knobs.set.insert("field.dead".into(), 1.0);
        super::super::stories::tests::fought_siege(&mut g, 0, 4);
        let f = g.stories.fields.last().expect("a field").clone();
        assert!(!f.mournful());
        g.tick = f.tick + YEAR;
        g.tick -= g.tick % DAY;
        g.alert = false;
        g.festival = None;
        g.battlefield_daily();
        assert!(g.year_log.stories.iter().any(|s| s.contains("feasts on the field")), "{:?}", g.year_log.stories);
        assert!(g.festival.is_some());
    }

    #[test]
    fn a_sentimental_overseer_raises_a_memorial_that_lays_the_dead() {
        let mut g = quiet(7);
        g.overseer = Some(Overseer::new(super::super::overseer::Style::Steady));
        super::super::stories::tests::fought_siege(&mut g, 2, 2);
        let mut o = g.overseer.take().unwrap();
        o.traits = vec![Trait::Sentimental, Trait::Vain];
        let before = g.stories.fields.last().expect("a field").place.clone();
        g.ov_memorial(&mut o);
        let f = g.stories.fields.last().unwrap().clone();
        let bid = f.memorial.expect("a slab is placed on the field");
        assert_ne!(f.place, before, "a vain overseer renames it");
        assert!(o.boosts.iter().any(|b| b.0 == crate::buildings::Recipe::Slab) || g.stock_of(ItemKind::Slab) > 0);
        g.buildings.get_mut(&bid).unwrap().built = true;
        g.battlefield_daily();
        assert!(g.stories.fields.last().unwrap().dedicated);
        assert!(g.buildings[&bid].occupant.is_some());
        assert!(g.dwarves.iter().any(|d| f.dead.iter().any(|x| x.0 == d.id) && d.rested), "one of the dead is laid to rest");
        assert!(g.field_look(f.at).unwrap().contains("memorial stands"));
        assert!(g.year_log.stories.iter().any(|s| s.contains("memorial")));
    }
}
