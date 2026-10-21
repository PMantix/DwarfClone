//! Inspired engineers (O12-a, CRF-03): a skilled, artistic dwarf, one who
//! has made an artifact, or one who stood in awe of the monument is now and
//! then struck by an idea for a great work. They sit down at a table and
//! draw the plans; the overseer feels the pull of them (`Sense::Inspired`)
//! and may take them up as its Architect ambition. The work is cut to the
//! plans, named for its dreamer ("the Pillared Hall of Kib Ironbrow"), and
//! goes into the legends, and the dreamer is known after as the Architect.
//! Plans left to gather dust for a year sour their dreamer, and the fort's
//! faith in its overseer; after two years the dream is given up.
//!
//! Only a fort the overseer runs has inspirations (a player's fort has
//! nobody to take the plans up yet).

use super::ambitions::{Ambition, Sense};
use super::layout::Shape;
use super::memory::MemoryKind;
use super::names::hash;
use super::overseer::{Overseer, RoomUse};
use super::personality::{NoteKind, Trait};
use super::*;
use serde::{Deserialize, Serialize};

/// How long a dreamer sits over the plans (ticks at a table).
const DRAW_TICKS: u32 = 600;

/// The great work a dwarf dreams of.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrandWork {
    #[default]
    Hall,
    PillaredHall,
    Rotunda,
    Gallery,
    Temple,
    StatueGarden,
    GreatForge,
}

impl GrandWork {
    /// What the dwarf dreams of, for the news.
    pub fn dream(self) -> &'static str {
        match self {
            GrandWork::Hall => "a great hall cut deep in the rock",
            GrandWork::PillaredHall => "a hall of a hundred pillars",
            GrandWork::Rotunda => "a round hall about a great monument",
            GrandWork::Gallery => "a long gallery carved from wall to wall",
            GrandWork::Temple => "a temple in the shape of a cross",
            GrandWork::StatueGarden => "a garden of statues under the mountain",
            GrandWork::GreatForge => "a great forge to warm the deep halls",
        }
    }

    /// The work's name, before the dreamer's.
    pub fn title(self) -> &'static str {
        match self {
            GrandWork::Hall => "Great Hall",
            GrandWork::PillaredHall => "Pillared Hall",
            GrandWork::Rotunda => "Rotunda",
            GrandWork::Gallery => "Long Gallery",
            GrandWork::Temple => "Cross Temple",
            GrandWork::StatueGarden => "Statue Garden",
            GrandWork::GreatForge => "Great Forge",
        }
    }

    /// The shape of grand room it is cut as (`layout.rs: pick_shape`).
    pub fn shape(self) -> Shape {
        match self {
            GrandWork::Hall => Shape::Rect { w: 9, h: 7 },
            GrandWork::PillaredHall => Shape::Pillared { w: 13, h: 10 },
            GrandWork::Rotunda | GrandWork::StatueGarden => Shape::Round { r: 5 },
            GrandWork::Gallery => Shape::Rect { w: 3, h: 16 },
            GrandWork::Temple => Shape::Cross { arm: 5, width: 3 },
            GrandWork::GreatForge => Shape::Rect { w: 9, h: 7 },
        }
    }

    /// The work as it came out, when the fort's plan gave it another
    /// shape of room (a garden and a forge are what is put in them, so
    /// they keep their names).
    fn as_cut(self, shape: Option<Shape>) -> GrandWork {
        if matches!(self, GrandWork::StatueGarden | GrandWork::GreatForge) {
            return self;
        }
        match shape {
            Some(Shape::Pillared { .. }) => GrandWork::PillaredHall,
            Some(Shape::Round { .. }) => GrandWork::Rotunda,
            Some(Shape::Cross { .. }) => GrandWork::Temple,
            Some(Shape::Rect { w, h }) if w.min(h) <= 3 => GrandWork::Gallery,
            Some(_) => GrandWork::Hall,
            None => self,
        }
    }
}

/// One dwarf's idea for a great work.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Inspiration {
    pub dwarf: u32,
    pub tick: u64,
    /// "a clockwork guardian", "a hall of a hundred pillars".
    pub what: String,
    /// The finished work, once it stands.
    #[serde(default)]
    pub done: Option<u64>,
    #[serde(default)]
    pub work: GrandWork,
    /// Ticks spent at a table over the plans.
    #[serde(default)]
    pub drawing: u32,
    /// When the plans were finished.
    #[serde(default)]
    pub drawn: Option<u64>,
    /// When the overseer took them up (None again if it set them aside).
    #[serde(default)]
    pub adopted: Option<u64>,
    /// The grand room being cut to them (its corner).
    #[serde(default)]
    pub room: Option<Pos>,
    /// The work's name, once finished.
    #[serde(default)]
    pub name: Option<String>,
    /// The dreamer has already sulked over plans left gathering dust.
    #[serde(default)]
    pub sulked: bool,
    /// Given up: the dreamer died, or lost heart.
    #[serde(default)]
    pub abandoned: bool,
}

impl Inspiration {
    /// Still a live dream: not built, not given up.
    pub fn open(&self) -> bool {
        self.done.is_none() && !self.abandoned
    }

    /// Drawn, and waiting for the overseer to take it up.
    pub fn waiting(&self) -> bool {
        self.open() && self.drawn.is_some() && self.adopted.is_none()
    }
}

/// Every inspiration the fort has had (`Game.inspirations`).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Inspirations {
    #[serde(default)]
    pub list: Vec<Inspiration>,
}

impl Game {
    fn dreamer(&self, id: u32) -> Option<&Dwarf> {
        self.dwarves.iter().find(|d| d.id == id && d.alive)
    }

    /// At the turn of each season (`season_turns`): perhaps a dwarf is
    /// struck by an idea. One dream at a time.
    pub(super) fn inspire_season(&mut self) {
        if self.overseer.is_none() || self.tick < SEASON || self.inspirations.list.iter().any(|i| i.open()) {
            return;
        }
        let since = self.tick.saturating_sub(SEASON);
        let monument = self.monument.clone().map(|m| format!("admired {}", m));
        let made: Vec<u32> = self.memory_since(since).filter(|m| m.kind == MemoryKind::Artifact).flat_map(|m| m.who.clone()).collect();
        let (lvl, art, chance) = (self.knob("inspire.skill", 4.0) as u32, self.knob("inspire.artistry", 10.0) as i32, self.knob("inspire.chance", 0.25));
        let mut best: Option<(u64, u32, GrandWork)> = None;
        for d in self.dwarves.iter().filter(|d| d.alive && !d.is_child()) {
            let skills = [(Labor::Masonry, d.skill(Labor::Masonry)), (Labor::Engraving, d.skill(Labor::Engraving)), (Labor::Mechanics, d.skill(Labor::Mechanics))];
            let (labor, top) = skills.iter().copied().max_by_key(|x| x.1).unwrap();
            let skilled = top >= lvl && d.personality.artistry > art;
            let odds = if made.contains(&d.id) {
                1.0
            } else if skilled {
                chance
            } else if monument.as_ref().map_or(false, |m| d.thoughts.iter().any(|t| &t.text == m && t.tick >= since)) {
                0.1
            } else {
                continue;
            };
            // (A steady roll, so the game's own dice are untouched.)
            let h = hash(&[super::stories::hash(&self.fortress_name, 0), self.tick, d.id as u64, 0x1d3a]);
            if (h % 10_000) as f64 >= odds * 10_000.0 {
                continue;
            }
            let works: &[GrandWork] = match labor {
                Labor::Masonry => &[GrandWork::PillaredHall, GrandWork::Rotunda, GrandWork::Temple],
                Labor::Engraving => &[GrandWork::Gallery, GrandWork::StatueGarden, GrandWork::Temple],
                _ => &[GrandWork::GreatForge, GrandWork::PillaredHall, GrandWork::Rotunda],
            };
            let w = works[(h / 10_000 % works.len() as u64) as usize];
            if best.map_or(true, |b| h < b.0) {
                best = Some((h, d.id, w));
            }
        }
        let Some((_, id, work)) = best else { return };
        self.inspire(id, work);
    }

    /// `id` is struck by a vision of `work`, and goes to draw the plans.
    pub fn inspire(&mut self, id: u32, work: GrandWork) {
        let name = self.dreamer(id).map(|d| d.called()).unwrap_or_default();
        let at = self.dreamer(id).map(|d| d.pos);
        self.inspirations.list.push(Inspiration {
            dwarf: id,
            tick: self.tick,
            what: work.dream().to_string(),
            done: None,
            work,
            drawing: 0,
            drawn: None,
            adopted: None,
            room: None,
            name: None,
            sulked: false,
            abandoned: false,
        });
        self.count("m2.inspired", 1);
        self.msg_at(&format!("{} is struck by a vision: {}! They look for a table to draw the plans.", name, work.dream()), (190, 160, 255), at, super::logkind::LogKind::Other);
    }

    /// Every hundred ticks: the dreamer at a table over the plans; once a
    /// day, plans left gathering dust sour their dreamer.
    pub(super) fn inspire_tick(&mut self) {
        let tick = self.tick;
        for i in 0..self.inspirations.list.len() {
            let ins = self.inspirations.list[i].clone();
            if !ins.open() {
                continue;
            }
            let Some(di) = self.dwarves.iter().position(|d| d.id == ins.dwarf && d.alive) else {
                // The dreamer is dead: the dream dies with them, unless it is
                // already being cut (then it is finished in their memory).
                if ins.room.is_none() {
                    self.inspirations.list[i].abandoned = true;
                }
                continue;
            };
            if ins.drawn.is_none() {
                self.draw_plans(i, di);
            } else if tick % DAY < 100 {
                self.dusty_plans(i, di);
            }
        }
    }

    fn draw_plans(&mut self, i: usize, di: usize) {
        let tick = self.tick;
        let pos = self.dwarves[di].pos;
        let table = self
            .buildings
            .values()
            .filter(|b| b.built && b.kind == BuildingKind::Table && b.pos.z == pos.z)
            .map(|b| b.pos)
            .min_by_key(|p| ((p.x - pos.x).abs() + (p.y - pos.y).abs(), p.x, p.y))
            .or_else(|| self.buildings.values().filter(|b| b.built && b.kind == BuildingKind::Table).map(|b| b.pos).min_by_key(|p| ((p.z - pos.z).abs(), p.x, p.y)));
        let ins = &self.inspirations.list[i];
        // At a table (or anywhere, after a week of looking for one).
        let at_table = table.map_or(false, |t| t.z == pos.z && (t.x - pos.x).abs() <= 1 && (t.y - pos.y).abs() <= 1);
        if at_table || tick > ins.tick + DAY * 7 {
            self.inspirations.list[i].drawing += 100;
        } else if let Some(t) = table {
            let d = &mut self.dwarves[di];
            if matches!(d.task, Task::Idle) && !d.insane {
                d.task = Task::Wander { target: t };
            }
        }
        if self.inspirations.list[i].drawing < DRAW_TICKS {
            return;
        }
        let ins = &mut self.inspirations.list[i];
        ins.drawn = Some(tick);
        let (work, id) = (ins.work, ins.dwarf);
        let d = &mut self.dwarves[di];
        d.think("drew up the plans for a great work", -30, tick);
        let who = d.called();
        let text = format!("{} has drawn the plans for a {}: {}.", who, work.title().to_lowercase(), work.dream());
        self.msg_at(&text, (190, 160, 255), Some(pos), super::logkind::LogKind::Other);
        self.record(MemoryKind::Inspired, &[id], Some(pos), format!("{} drew the plans for {}.", who, work.dream()), 4);
        self.count("m2.inspirations_drawn", 1);
    }

    /// Once a day for drawn plans: a year untouched and the dreamer is
    /// bitter (and the fort's faith in the overseer drops); two years and
    /// they give the dream up.
    fn dusty_plans(&mut self, i: usize, di: usize) {
        let tick = self.tick;
        let ins = self.inspirations.list[i].clone();
        // (Plans being cut are not gathering dust.)
        if ins.room.is_some() {
            return;
        }
        let drawn = ins.drawn.unwrap_or(tick);
        let sulk = self.knob("inspire.sulk", 40.0) as i32;
        let what = ins.work.title().to_lowercase();
        if !ins.sulked && tick >= drawn + YEAR {
            self.inspirations.list[i].sulked = true;
            let d = &mut self.dwarves[di];
            d.think("was ignored by the overseer", sulk, tick);
            let who = d.called();
            let pos = d.pos;
            self.msg_at(&format!("{}'s plans for a {} gather dust. {} feels ignored by the overseer.", who, what, who), (200, 150, 120), Some(pos), super::logkind::LogKind::Other);
            self.count("m2.inspirations_ignored", 1);
            if let Some(mut o) = self.overseer.take() {
                self.faith(&mut o, -5, &format!("{}'s plans were ignored", who));
                Self::note_as(&mut o, tick, NoteKind::Ambition, format!("{} sulks: their plans for a {} still wait.", who, what));
                self.overseer = Some(o);
            }
        } else if tick >= drawn + 2 * YEAR {
            self.inspirations.list[i].abandoned = true;
            let d = &mut self.dwarves[di];
            d.think("gave up on a dream", sulk * 3 / 2, tick);
            let who = d.called();
            let pos = d.pos;
            self.msg_at(&format!("{} has torn up the plans for a {}. The dream is dead.", who, what), (200, 150, 120), Some(pos), super::logkind::LogKind::Other);
            if let Some(o) = self.overseer.as_mut() {
                if o.adopted == Some(i as u32) {
                    o.adopted = None;
                }
            }
        }
    }

    /// For the brain: drawn plans waiting pull the overseer to Architect (a
    /// vain or pious one more, a greedy or stubborn one less).
    pub(super) fn inspired_sense(&self, o: &Overseer) -> (Sense, f64, Vec<(Ambition, f64)>) {
        let waiting = self.inspirations.list.iter().any(|i| i.waiting() && self.dreamer(i.dwarf).is_some());
        let mut w = 1.5;
        for (t, by) in [(Trait::Vain, 0.5), (Trait::Pious, 0.3), (Trait::Greedy, -0.8), (Trait::Stubborn, -0.6)] {
            if o.has(t) {
                w += by;
            }
        }
        (Sense::Inspired, if waiting { 1.0 } else { 0.0 }, vec![(Ambition::Architect, w)])
    }

    /// When the overseer takes up an ambition (`begin_ambition`): as the
    /// Architect it takes up the waiting plans; otherwise it sets them aside
    /// (and says so).
    pub(super) fn weigh_plans(&mut self, o: &mut Overseer, pick: Ambition) {
        let tick = self.tick;
        let Some(i) = self.inspirations.list.iter().position(|i| i.waiting() && self.dreamer(i.dwarf).is_some()) else { return };
        let ins = self.inspirations.list[i].clone();
        let who = self.dreamer(ins.dwarf).map(|d| d.called()).unwrap_or_default();
        let what = ins.work.title().to_lowercase();
        if pick != Ambition::Architect {
            Self::note_as(o, tick, NoteKind::Ambition, format!("Set {}'s plans for a {} aside, for now.", who, what));
            return;
        }
        self.inspirations.list[i].adopted = Some(tick);
        o.adopted = Some(i as u32);
        self.count("m2.inspirations_adopted", 1);
        Self::note_as(o, tick, NoteKind::Ambition, format!("Took up {}'s plans for a {}: it will be cut as they drew it.", who, what));
        let at = self.dreamer(ins.dwarf).map(|d| d.pos);
        self.msg_at(&format!("The overseer takes up {}'s plans: {} will be built!", who, ins.work.dream()), (190, 160, 255), at, super::logkind::LogKind::Other);
        if let Some(d) = self.dwarves.iter_mut().find(|d| d.id == ins.dwarf) {
            d.think("saw their plans taken up", -40, tick);
        }
    }

    /// The shape the adopted plans want their grand room cut as (`pick_shape`).
    pub(super) fn inspired_shape(&self, o: &Overseer) -> Option<Shape> {
        let ins = self.inspirations.list.get(o.adopted? as usize)?;
        (ins.open() && ins.room.is_none()).then(|| ins.work.shape())
    }

    /// What the side panel says of the plans the overseer has taken up
    /// ("Kib's plans for a pillared hall"), if any.
    pub fn adopted_plans(&self) -> Option<String> {
        let o = self.overseer.as_ref()?;
        let ins = self.inspirations.list.get(o.adopted? as usize)?;
        let who = self.dwarves.iter().find(|d| d.id == ins.dwarf)?;
        Some(format!("{}'s plans for a {}", who.name.split(' ').next().unwrap_or(&who.name), ins.work.title().to_lowercase()))
    }

    /// Daily (`ov_daily_hooks`): the grand room cut to the plans is found,
    /// and once fitted, the work is named and celebrated; plans whose
    /// ambition passed with no room begun are set aside again.
    pub(super) fn ov_inspirations(&mut self, o: &mut Overseer) {
        let tick = self.tick;
        let Some(i) = o.adopted.map(|i| i as usize) else { return };
        let Some(ins) = self.inspirations.list.get(i).cloned() else {
            o.adopted = None;
            return;
        };
        if !ins.open() {
            o.adopted = None;
            return;
        }
        let since = ins.adopted.unwrap_or(tick);
        let claimed: Vec<Pos> = self.inspirations.list.iter().filter_map(|x| x.room).collect();
        let room = match ins.room {
            Some(a) => o.rooms.iter().find(|r| r.a == a).cloned(),
            None => {
                let r = o.rooms.iter().find(|r| r.kind == RoomUse::Grand && r.since >= since && r.stage != 2 && !claimed.contains(&r.a)).cloned();
                if let Some(r) = &r {
                    let x = &mut self.inspirations.list[i];
                    x.room = Some(r.a);
                    x.work = x.work.as_cut(r.shape);
                }
                r
            }
        };
        let Some(r) = room.filter(|r| r.stage != 2) else {
            // No rock for it (or the room was given up): set aside, to be
            // taken up another time.
            let blocked = o.blocked.iter().any(|&(k, t)| k == RoomUse::Grand && t >= since);
            if ins.room.is_some() || blocked {
                let x = &mut self.inspirations.list[i];
                x.room = None;
                x.adopted = None;
                o.adopted = None;
                // (Next time, a plainer hall that fits where the rock allows.)
                let smaller = !matches!(x.work, GrandWork::Hall | GrandWork::StatueGarden | GrandWork::GreatForge);
                if smaller {
                    x.work = GrandWork::Hall;
                }
                let who = self.dreamer(ins.dwarf).map(|d| d.called()).unwrap_or_default();
                let then = if smaller { " They will draw them smaller." } else { "" };
                Self::note_as(o, tick, NoteKind::Ambition, format!("Set {}'s plans aside: there was no rock to cut them from.{}", who, then));
            } else if !o.wants.contains(&RoomUse::Grand) {
                // (The plans outlast the ambition: the grand room stays
                // wanted until it is begun.)
                o.wants.insert(0, RoomUse::Grand);
            }
            return;
        };
        if r.stage == 1 {
            self.finish_work(o, i, &r);
        }
    }

    /// The work stands: extras for a garden or a forge, the name, the
    /// legend, the epithet, and the fort's pride.
    fn finish_work(&mut self, o: &mut Overseer, i: usize, r: &super::overseer::Room) {
        let tick = self.tick;
        let ins = self.inspirations.list[i].clone();
        let work = ins.work;
        match work {
            GrandWork::StatueGarden => {
                self.add_order_for(ItemKind::Statue, None, 4, tick + YEAR);
                let tiles: Vec<Pos> = if r.tiles.is_empty() { box_iter(r.a, r.b).collect() } else { r.tiles.clone() };
                let mid = Pos::new((r.a.x + r.b.x) / 2, (r.a.y + r.b.y) / 2, r.a.z);
                let mut n = 0;
                for p in tiles {
                    let d2 = (p.x - mid.x).pow(2) + (p.y - mid.y).pow(2);
                    if n < 6 && (9..=16).contains(&d2) && (p.x + p.y) % 2 == 0 && self.can_place(BuildingKind::Statue, p) && self.place_building(BuildingKind::Statue, p) {
                        n += 1;
                    }
                }
            }
            GrandWork::GreatForge => {
                self.place_building(BuildingKind::Forge, r.a.offset(1, 2, 0));
                self.place_building(BuildingKind::Smelter, r.a.offset(5, 2, 0));
            }
            _ => {}
        }
        let (who, full) = match self.dwarves.iter().find(|d| d.id == ins.dwarf) {
            Some(d) => (d.called(), d.name.clone()),
            None => ("a dwarf now dead".into(), "the Dreamer".into()),
        };
        let name = format!("the {} of {}", work.title(), full);
        let fort = self.fortress_name.clone();
        let mid = r.tiles.get(r.tiles.len() / 2).copied().unwrap_or(r.a);
        self.name_place(super::places::PlaceKind::Hall, r.a, r.b, name.clone(), format!("drawn by {}, who dreamed of {}", who, work.dream()), 8, false);
        {
            let x = &mut self.inspirations.list[i];
            x.done = Some(tick);
            x.name = Some(name.clone());
        }
        o.adopted = None;
        self.count("m2.inspirations_finished", 1);
        self.legend(format!("{} of {}, drawn by {}, was finished.", cap(&name), fort, who));
        self.record(MemoryKind::Inspired, &[ins.dwarf], Some(mid), format!("{}, drawn by {}, was finished.", cap(&name), who), 7);
        Self::note_as(o, tick, NoteKind::Ambition, format!("Finished {}, as {} drew it.", name, who));
        if let Some(di) = self.dwarves.iter().position(|d| d.id == ins.dwarf && d.alive) {
            let mut d = std::mem::take(&mut self.dwarves[di]);
            d.think("saw their dream built in stone", -150, tick);
            self.earn_epithet(&mut d, "Architect", super::heroes::RANK_FEAT + 1, &format!("dreamed {}", name));
            self.dwarves[di] = d;
        }
        for d in self.dwarves.iter_mut().filter(|d| d.alive && d.id != ins.dwarf) {
            d.think(&format!("marvelled at {}", name), -10, tick);
        }
        let text = format!("{} stands finished, just as {} drew it. It will carry their name.", cap(&name), who);
        self.msg_at(&text, (255, 215, 60), Some(mid), super::logkind::LogKind::Other);
        self.spotlight("A dream in stone", &text, Some(mid), (255, 215, 60));
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::overseer::Style;

    fn feed(g: &mut Game) {
        for d in g.dwarves.iter_mut() {
            d.hunger = 0;
            d.thirst = 0;
        }
    }

    /// The Done-when: a dug-in Builder; a dwarf's vision of a pillared hall
    /// is drawn, pulls the overseer to Architect, is taken up, cut and
    /// fitted, and stands named for its dreamer, who is the Architect.
    #[test]
    fn a_dwarfs_inspiration_becomes_an_ambition_and_a_work_bearing_their_name() {
        let mut g = Game::new(5003);
        g.set_overseer(Some(Style::Builder));
        // Dug in: half a year of the overseer's own plan.
        while g.tick < SEASON {
            g.step();
            feed(&mut g);
        }
        // (No dream of the fort's own gets in first.)
        g.inspirations.list.clear();
        let x = g.dwarves.iter().find(|d| d.alive && !d.is_child() && !d.soldier).map(|d| d.id).unwrap();
        let full = g.dwarves.iter().find(|d| d.id == x).unwrap().name.clone();
        g.inspire(x, GrandWork::PillaredHall);
        while g.inspirations.list[0].drawn.is_none() && g.tick < SEASON + DAY * 10 {
            g.step();
            feed(&mut g);
        }
        assert!(g.inspirations.list[0].drawn.is_some(), "the plans are drawn within the week of looking for a table");
        // The plans pull the overseer to Architect.
        let o = g.overseer.clone().unwrap();
        let brain = g.brain(&o);
        let pull = brain.links.iter().find(|l| l.0 == Ambition::Architect).and_then(|l| l.1.iter().find(|s| s.0 == Sense::Inspired)).map_or(0.0, |s| s.1);
        assert!(pull >= 1.5 - 0.8 - 0.6 - 1e-9 && (o.traits.iter().any(|t| matches!(t, Trait::Greedy | Trait::Stubborn)) || pull >= 1.5), "the plans link Architect at {}", pull);
        // Taken up (forced, if the odds are slow).
        if g.inspirations.list[0].adopted.is_none() {
            g.force_ambition(Ambition::Architect);
        }
        assert!(g.inspirations.list[0].adopted.is_some() && g.overseer.as_ref().unwrap().adopted == Some(0));
        assert!(g.adopted_plans().map_or(false, |p| p.contains("plans for a pillared hall")));
        let start = g.tick;
        while g.inspirations.list[0].done.is_none() && g.tick < start + 2 * SEASON {
            g.step();
            feed(&mut g);
        }
        let ins = g.inspirations.list[0].clone();
        if ins.done.is_none() {
            for n in g.overseer.as_ref().unwrap().notes.iter().filter(|n| n.0 >= start) {
                eprintln!("{} {}", n.0 / DAY, n.1);
            }
        }
        assert!(ins.done.is_some(), "the work is finished within two seasons: {:?}, rooms {:?}", ins, g.overseer.as_ref().unwrap().rooms.iter().filter(|r| r.kind == RoomUse::Grand).map(|r| (r.stage, r.since)).collect::<Vec<_>>());
        let name = ins.name.clone().unwrap();
        assert!(name.contains(&full), "{}", name);
        let a = ins.room.unwrap();
        assert_eq!(g.place_at(a).map(|p| p.name.clone()), Some(name.clone()), "the place carries the dreamer's name");
        let d = g.dwarves.iter().find(|d| d.id == x).unwrap();
        assert!(!d.alive || d.epithet.is_some(), "the dreamer earns a name");
        assert!(g.overworld.events.iter().any(|e| e.text.contains(&cap(&name)) && e.text.contains("was finished")), "the legends tell it");
        assert!(g.spotlights.iter().any(|s| s.title == "A dream in stone") || g.reel.clips.iter().any(|c| c.title == "A dream in stone"));
        eprintln!("day {}: {} ({})", g.tick / DAY, name, d.called());
    }

    /// Plans nobody takes up: a year on, the dreamer sulks and the fort's
    /// faith in the overseer falls; another year and they are torn up.
    #[test]
    fn plans_left_to_gather_dust_sour_their_dreamer() {
        let mut g = Game::new(5);
        g.set_overseer(Some(Style::Warlord));
        let x = g.dwarves[0].id;
        g.inspire(x, GrandWork::Rotunda);
        g.inspirations.list[0].drawn = Some(g.tick);
        let faith = g.overseer.as_ref().unwrap().faith;
        g.tick = YEAR + 50;
        g.inspire_tick();
        assert!(g.inspirations.list[0].sulked);
        assert!(g.dwarves[0].thoughts.iter().any(|t| t.text == "was ignored by the overseer" && t.stress > 0));
        assert!(g.overseer.as_ref().unwrap().faith < faith);
        g.tick = 2 * YEAR + 50;
        g.inspire_tick();
        assert!(g.inspirations.list[0].abandoned && !g.inspirations.list[0].open());
        // And a waiting plan no longer pulls.
        let o = g.overseer.clone().unwrap();
        assert_eq!(g.inspired_sense(&o).1, 0.0);
    }

    /// Old saves: an inspiration from wave 0 (no new fields) loads.
    #[test]
    fn a_wave_zero_inspiration_loads() {
        let old = r#"{"dwarf":3,"tick":10,"what":"a hall"}"#;
        let i: Inspiration = serde_json::from_str(old).unwrap();
        assert!(i.open() && !i.waiting() && i.work == GrandWork::Hall);
    }
}
