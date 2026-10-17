//! The overseer hunts predators (MIL-03): when wild beasts prowl near the
//! fort and a squad is fit to face them, it sends the squad out after them,
//! one pack member at a time, and calls it home when an army comes, when
//! the beasts get away or when the squad is too hurt to go on. While a
//! pack prowls near, work outdoors waits.

use super::military::SquadOrder;
use super::overseer::{Overseer, Style};
use super::personality::Trait;
use super::*;
use serde::{Deserialize, Serialize};

/// The overseer's hunting: the hunt under way, if any, and a short ban on
/// work outdoors while beasts prowl near.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Hunts {
    /// Nobody starts work outdoors (chopping, gathering, fishing) before this tick.
    #[serde(default)]
    pub outdoor_ban_until: u64,
    /// The hunt under way.
    #[serde(default)]
    pub on: Option<Hunt>,
}

/// A squad sent after a pack.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hunt {
    /// The squad's id (not its index).
    pub squad: u32,
    /// The beast they're after now.
    pub target: u32,
    /// Its pack: the next one is taken from here.
    pub pack: u32,
    /// "wolf", "wolves".
    pub name: String,
    pub plural: String,
    pub kills: u32,
    pub since: u64,
    /// The pack members still about the fort, last we looked.
    #[serde(default)]
    pub quarry: Vec<u32>,
}

/// How far from the stair (or its foot) a beast is near enough to hunt.
const NEAR: i32 = 30;

fn number(n: u32) -> String {
    match n {
        2 => "two".into(),
        3 => "three".into(),
        4 => "four".into(),
        5 => "five".into(),
        6 => "six".into(),
        n => n.to_string(),
    }
}

impl Game {
    /// Work outdoors waits: beasts prowl near (the finder table checks this).
    pub(super) fn outdoors_banned(&self) -> bool {
        self.tick < self.hunts.outdoor_ban_until
    }

    /// Every ten ticks (overseer_hooks.rs): keep a hunt going, or start one.
    pub(super) fn ov_hunts(&mut self, o: &mut Overseer) {
        // The reckless pay the beasts no mind.
        if o.style == Style::Reckless {
            return;
        }
        let (Some(top), Some(base)) = (o.top, o.base) else { return };
        let war = self.incoming.is_some() || self.creatures.values().any(|c| c.alive && c.siege && c.hostile());
        let prowlers = self.prowlers_about(top, base);
        self.keep_indoors(top, &prowlers);
        if self.hunts.on.is_some() {
            self.hunt_goes_on(o, top, base, war);
        } else if !war && !prowlers.is_empty() {
            self.start_hunt(o, &prowlers);
        }
    }

    /// Prowlers about the top of the stair or its foot, nearest the top first.
    fn prowlers_about(&self, top: Pos, base: Pos) -> Vec<(u32, Pos, u32)> {
        let mut v = self.prowlers(top, NEAR);
        for p in self.prowlers(base, NEAR) {
            if !v.iter().any(|q| q.0 == p.0) {
                v.push(p);
            }
        }
        v
    }

    /// Beasts within 20 (knob hunt.ban_range) of the stair or of anyone
    /// working outdoors: those at outdoor work stop, and nobody starts any for
    /// a little while (knob hunt.ban, 300 ticks). Those closer still flee as
    /// ever.
    fn keep_indoors(&mut self, top: Pos, prowlers: &[(u32, Pos, u32)]) {
        if prowlers.is_empty() {
            return;
        }
        let r = self.knob("hunt.ban_range", 20.0) as i32;
        let near = |p: Pos| prowlers.iter().any(|&(_, q, _)| (q.x - p.x).abs() <= r && (q.y - p.y).abs() <= r && (q.z - p.z).abs() <= 6);
        let outdoor = |g: &Game, t: &Task| match *t {
            Task::Dig { target, .. } => g.world.outside(target),
            Task::Gather { .. } | Task::Fish { .. } => true,
            Task::Plant { pos, .. } | Task::Harvest { pos } => g.world.outside(pos),
            _ => false,
        };
        let workers: Vec<usize> = (0..self.dwarves.len())
            .filter(|&i| {
                let d = &self.dwarves[i];
                d.alive && !d.soldier && self.world.outside(d.pos) && near(d.pos) && outdoor(self, &d.task)
            })
            .collect();
        if workers.is_empty() && !near(top) {
            return;
        }
        if !self.outdoors_banned() {
            let what = self.creatures.get(&prowlers[0].0).map_or("beasts", |c| c.species.info().plural);
            self.warn("outdoor_ban", &format!("The {} prowl near the fort: work outdoors waits.", what), (240, 200, 120));
            self.count("outdoor_bans", 1);
        }
        self.hunts.outdoor_ban_until = self.tick + self.knob("hunt.ban", 300.0) as u64;
        for i in workers {
            let mut d = std::mem::take(&mut self.dwarves[i]);
            self.release_task(&mut d);
            d.task = Task::Idle;
            d.path.clear();
            self.dwarves[i] = d;
        }
    }

    /// A soldier fit to go hunting: here, on their feet, unhurt and armed.
    fn fit_to_hunt(&self, d: &Dwarf) -> bool {
        d.alive
            && d.soldier
            && !d.is_child()
            && !d.insane
            && d.jailed_until == 0
            && !d.body.unconscious()
            && !d.body.needs_rest()
            && d.held.and_then(|id| self.items.get(&id)).map_or(false, |it| it.kind.weapon_damage().is_some() && it.kind != ItemKind::Pick)
    }

    /// The squad's members fit to hunt, and how many of them are kit-ready.
    fn hunters(&self, s: usize) -> (Vec<u32>, usize) {
        let fit: Vec<u32> = self.squads[s].members.iter().copied().filter(|&id| self.dwarves.iter().any(|d| d.id == id && self.fit_to_hunt(d))).collect();
        let ready = fit.iter().filter(|&&id| self.kit_ready(id)).count();
        (fit, ready)
    }

    /// How strong a squad must be to take on these beasts (knob hunt.beast
    /// per beast, by its size).
    fn beasts_threat(&self, ids: &[u32]) -> f64 {
        let per = self.knob("hunt.beast", 4.0);
        ids.iter().filter_map(|id| self.creatures.get(id)).map(|c| per * (c.species.info().scale as f64).max(0.5)).sum()
    }

    /// Send the best squad after the nearest pack, if it's up to it. A
    /// cautious overseer wants three soldiers in full kit and the odds well
    /// in hand; a bold one goes with two and narrower odds.
    fn start_hunt(&mut self, o: &mut Overseer, prowlers: &[(u32, Pos, u32)]) {
        // Not while a monster is at the gates, nor over the player's own orders.
        if self.alert || self.squads.iter().any(|s| matches!(s.order, SquadOrder::Kill(_))) {
            return;
        }
        let away = self.expedition.as_ref().map(|e| e.squad.clone());
        let best = (0..self.squads.len())
            .filter(|&s| Some(&self.squads[s].name) != away.as_ref())
            .map(|s| {
                let (fit, ready) = self.hunters(s);
                let strength = self.squad_strength(&fit);
                (ready, fit.len(), strength, s, fit)
            })
            .max_by_key(|x| (x.0, x.1, x.2, std::cmp::Reverse(x.3)));
        let Some((ready, fit, strength, s, ids)) = best else { return };
        let (target, _, pack) = prowlers[0];
        let pack_ids: Vec<u32> = prowlers.iter().filter(|p| p.2 == pack).map(|p| p.0).collect();
        let (bold, cautious) = (o.has(Trait::Bold), o.has(Trait::Cautious));
        let (enough, odds) = if cautious {
            (ready >= 3, self.knob("hunt.cautious", 1.5))
        } else if bold {
            (fit >= 2, self.knob("hunt.bold", 0.75))
        } else {
            (fit >= 2, 1.0)
        };
        if !enough || (strength as f64) < self.beasts_threat(&pack_ids) * odds {
            return;
        }
        let Some(c) = self.creatures.get(&target) else { return };
        let info = c.species.info();
        let (name, plural) = (info.name.to_string(), info.plural.to_string());
        let what = if pack_ids.len() > 1 { plural.clone() } else { name.clone() };
        let squad = self.squads[s].name.clone();
        self.squads[s].order = SquadOrder::Kill(target);
        self.call_out(&ids);
        self.hunts.on = Some(Hunt { squad: self.squads[s].id, target, pack, name, plural, kills: 0, since: self.tick, quarry: pack_ids });
        self.count("hunts", 1);
        self.msg(&format!("{} set out after the {}.", squad, what), (240, 200, 120));
        Self::note_ov(o, self.tick, format!("Sent {} after the {}.", squad, what));
    }

    /// Hunters at drill, on station or idling go at once, rather than when
    /// they next look for something to do.
    fn call_out(&mut self, ids: &[u32]) {
        for i in 0..self.dwarves.len() {
            if !ids.contains(&self.dwarves[i].id) || !matches!(self.dwarves[i].task, Task::Idle | Task::Wander { .. } | Task::Station { .. } | Task::Train) {
                continue;
            }
            let mut d = std::mem::take(&mut self.dwarves[i]);
            self.release_task(&mut d);
            d.task = Task::Idle;
            d.path.clear();
            d.idle_cd = 0;
            self.dwarves[i] = d;
        }
    }

    /// The hunt under way: the next of the pack once one falls, and home
    /// when it's done, an army comes, the beasts get away or the hunters are
    /// too hurt to go on.
    fn hunt_goes_on(&mut self, o: &mut Overseer, top: Pos, base: Pos, war: bool) {
        let Some(mut h) = self.hunts.on.take() else { return };
        // A squad disbanded mid-hunt: nothing more to say.
        let Some(s) = self.squads.iter().position(|q| q.id == h.squad) else { return };
        let mine = self.squads[s].order == SquadOrder::Kill(h.target);
        if war {
            return self.end_hunt(o, s, h, "an army is coming");
        }
        // (Further than they're first sought: a chase runs on, knob hunt.chase.)
        let r = self.knob("hunt.chase", 60.0) as i32;
        let near = |p: Pos| {
            let by = |q: Pos| (p.x - q.x).abs() <= r && (p.y - q.y).abs() <= r && (p.z - q.z).abs() <= 8;
            by(top) || by(base)
        };
        // Those of the pack slain (or caged) since the last look; those gone
        // off from the fort have got away. Mates that turn up join the quarry.
        let mut quarry = Vec::new();
        for id in std::mem::take(&mut h.quarry) {
            match self.creatures.get(&id) {
                Some(c) if c.alive => {
                    if near(c.pos) {
                        quarry.push(id);
                    }
                }
                _ => {
                    h.kills += 1;
                    self.count("hunt_kills", 1);
                }
            }
        }
        for c in self.creatures.values() {
            if c.alive && !c.siege && (c.pack == h.pack || c.id == h.pack) && near(c.pos) && !quarry.contains(&c.id) {
                quarry.push(c.id);
            }
        }
        h.quarry = quarry;
        let (fit, _) = self.hunters(s);
        if h.quarry.contains(&h.target) {
            // Someone else gave the squad its orders: their hunt now.
            if !mine {
                return self.end_hunt(o, s, h, "");
            }
            if fit.is_empty() {
                return self.end_hunt(o, s, h, "the hunters are hurt");
            }
            if self.tick > h.since + (self.knob("hunt.days", 1.0) * DAY as f64) as u64 {
                return self.end_hunt(o, s, h, "the trail went cold");
            }
            self.hunts.on = Some(h);
            return;
        }
        // The next of the pack, even as it scatters, while it's near the fort.
        let next = h.quarry.iter().filter_map(|id| self.creatures.get(id)).min_by_key(|c| (crate::game::combat::dist2(c.pos, top), c.id)).map(|c| c.id);
        match next {
            Some(t) if !fit.is_empty() && matches!(self.squads[s].order, SquadOrder::Kill(_) | SquadOrder::Train) => {
                h.target = t;
                self.squads[s].order = SquadOrder::Kill(t);
                self.call_out(&fit);
                self.hunts.on = Some(h);
            }
            _ => self.end_hunt(o, s, h, "the beasts got away"),
        }
    }

    /// The hunt is over: the squad comes home, and a hunt that killed
    /// something goes in the chronicle.
    fn end_hunt(&mut self, o: &mut Overseer, s: usize, h: Hunt, why: &str) {
        if matches!(self.squads[s].order, SquadOrder::Kill(_)) && !why.is_empty() {
            self.squads[s].order = SquadOrder::Train;
        }
        let squad = self.squads[s].name.clone();
        let fort = self.fortress_name.clone();
        if h.kills > 0 {
            let what = if h.kills == 1 { format!("the {}", h.name) } else { format!("{} {}", number(h.kills), h.plural) };
            let text = format!("{} killed {} that had been prowling about {}.", squad, what, fort);
            self.msg(&text, (160, 230, 160));
            self.legend(text);
            Self::note_ov(o, self.tick, format!("{} came home from the hunt: {} slain.", squad, what));
        } else if !why.is_empty() {
            self.msg(&format!("{} come home from the hunt: {}.", squad, why), (200, 200, 160));
            Self::note_ov(o, self.tick, format!("Called {} home from the hunt: {}.", squad, why));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::Species;
    use crate::material::Mat;

    /// An overseer fort (Steady, no traits) with its stair dug in, no wild
    /// beasts, and a squad of three in iron axes and helms. The squad's index.
    fn fort(seed: u64, style: Style) -> (Game, usize, Vec<u32>) {
        let mut g = Game::new(seed);
        g.creatures.retain(|_, c| c.tame);
        g.set_overseer(Some(style));
        g.overseer.as_mut().unwrap().traits = Vec::new();
        while g.overseer.as_ref().unwrap().base.is_none() {
            g.step();
        }
        let ids: Vec<u32> = g.dwarves.iter().filter(|d| d.alive && !d.is_child()).take(3).map(|d| d.id).collect();
        let s = g.new_squad();
        for &id in &ids {
            g.set_squad_member(s, id, true);
            let at = g.dwarves.iter().find(|d| d.id == id).unwrap().pos;
            let axe = g.spawn_item(ItemKind::Axe, Mat::Iron, at);
            let helm = g.spawn_item(ItemKind::Helm, Mat::Iron, at);
            for it in [axe, helm] {
                g.items.get_mut(&it).unwrap().carried_by = Some(id);
            }
            let d = g.dwarves.iter_mut().find(|d| d.id == id).unwrap();
            d.held = Some(axe);
            d.armor.push(helm);
        }
        for &id in &ids {
            assert!(g.kit_ready(id));
        }
        (g, s, ids)
    }

    /// Open ground about `r` tiles from the embark: the beasts' spot.
    fn out_by(g: &Game, r: i32) -> Pos {
        let e = g.embark;
        (0..64)
            .flat_map(|k| {
                let a = k as f64 * std::f64::consts::TAU / 64.0;
                (-3..=3).map(move |dz| e.offset((a.cos() * r as f64) as i32, (a.sin() * r as f64) as i32, dz))
            })
            .find(|&p| g.world.in_bounds(p) && g.world.walkable(p) && g.world.outside(p))
            .expect("no open ground out there")
    }

    fn wolves(g: &mut Game, at: Pos, n: usize) -> Vec<u32> {
        let ids: Vec<u32> = (0..n).map(|i| g.spawn_creature(Species::Wolf, at.offset(i as i32, 0, 0), u64::MAX)).collect();
        for &w in &ids {
            g.creatures.get_mut(&w).unwrap().pack = ids[0];
        }
        ids
    }

    #[test]
    fn the_overseer_sends_a_squad_after_wolves() {
        let (mut g, s, _) = fort(21, Style::Steady);
        let at = out_by(&g, 15);
        let pack = wolves(&mut g, at, 2);
        let start = g.tick;
        while g.tick < start + 100 && !matches!(g.squads[s].order, SquadOrder::Kill(_)) {
            g.step();
        }
        assert!(matches!(g.squads[s].order, SquadOrder::Kill(t) if pack.contains(&t)), "no kill order: {:?}", g.squads[s].order);
        let notes = &g.overseer.as_ref().unwrap().notes;
        assert!(notes.iter().any(|n| n.1.starts_with("Sent ") && n.1.ends_with("after the wolves.")), "{:?}", notes);
        assert_eq!(g.annals.counters.get("hunts").copied(), Some(1));
        while g.tick < start + DAY && pack.iter().any(|w| g.creatures.get(w).map_or(false, |c| c.alive)) {
            g.step();
        }
        assert!(pack.iter().all(|w| g.creatures.get(w).map_or(true, |c| !c.alive)), "the wolves live on");
        for _ in 0..20 {
            g.step();
        }
        let log: Vec<String> = g.log.iter().filter(|l| l.tick >= start).map(|l| format!("{} {}", l.tick, l.text)).collect();
        assert_eq!(g.annals.counters.get("hunt_kills").copied(), Some(2), "{:#?}", log);
        // The hunt is over and told.
        for _ in 0..20 {
            g.step();
        }
        assert!(g.hunts.on.is_none());
        assert!(g.overworld.events.iter().any(|e| e.text.contains("killed two wolves")), "no chronicle line");
    }

    /// A woodcutter at work with a wolf 12 tiles off stops, and nobody
    /// takes up chopping until the ban runs out.
    #[test]
    fn work_outdoors_waits_while_wolves_prowl() {
        let (mut g, _, squad) = fort(21, Style::Steady);
        let at = out_by(&g, 10);
        let id = g.dwarves.iter().find(|d| d.alive && !d.is_child() && !squad.contains(&d.id)).unwrap().id;
        {
            let d = g.dwarves.iter_mut().find(|d| d.id == id).unwrap();
            d.pos = at;
            d.task = Task::Dig { target: at, stand: at };
            d.labors[Labor::Woodcutting.idx()] = true;
        }
        assert!(!g.outdoors_banned());
        let wolf = g.spawn_creature(Species::Wolf, at.offset(12, 0, 0), u64::MAX);
        let mut o = g.overseer.take().unwrap();
        g.ov_hunts(&mut o);
        assert!(g.outdoors_banned());
        assert_eq!(g.annals.counters.get("outdoor_bans").copied(), Some(1));
        let i = g.dwarves.iter().position(|d| d.id == id).unwrap();
        let mut d = std::mem::take(&mut g.dwarves[i]);
        assert_eq!(d.task, Task::Idle);
        let chop = super::super::finders::FINDERS.iter().find(|f| f.0 == "chop").unwrap().1;
        assert!(!chop(&mut g, &mut d), "chopping while the wolf prowls");
        // The wolf moves on; the ban runs out.
        g.creatures.remove(&wolf);
        g.tick += 301;
        g.ov_hunts(&mut o);
        assert!(!g.outdoors_banned());
        g.dwarves[i] = d;
        g.overseer = Some(o);
    }

    #[test]
    fn reckless_never_hunts() {
        let (mut g, s, _) = fort(21, Style::Reckless);
        let at = out_by(&g, 15);
        wolves(&mut g, at, 2);
        for _ in 0..300 {
            g.step();
        }
        assert!(!matches!(g.squads[s].order, SquadOrder::Kill(_)));
        assert!(g.hunts.on.is_none());
        assert_eq!(g.annals.counters.get("hunts"), None);
    }

    /// Three soldiers in kit against two wolves: a bold overseer sends them,
    /// a cautious one wants the odds better still.
    #[test]
    fn caution_wants_better_odds() {
        let (mut g, s, ids) = fort(21, Style::Steady);
        let at = out_by(&g, 15);
        let pack = wolves(&mut g, at, 2);
        let prowlers = g.prowlers_about(g.overseer.as_ref().unwrap().top.unwrap(), g.overseer.as_ref().unwrap().base.unwrap());
        assert_eq!(prowlers.len(), 2);
        g.alert = false;
        let mut o = g.overseer.take().unwrap();
        o.traits = vec![Trait::Cautious];
        // Three in kit against two wolves: 6 strength, 4.8 × 1.5 needed.
        g.start_hunt(&mut o, &prowlers);
        assert!(g.hunts.on.is_none(), "the cautious overseer sent them");
        o.traits = vec![Trait::Bold];
        g.start_hunt(&mut o, &prowlers);
        assert!(matches!(g.squads[s].order, SquadOrder::Kill(t) if pack.contains(&t)));
        let _ = ids;
    }

    /// An army on the way calls the hunters home.
    #[test]
    fn an_army_calls_the_hunt_off() {
        let (mut g, s, _) = fort(21, Style::Steady);
        let at = out_by(&g, 25);
        wolves(&mut g, at, 1);
        let start = g.tick;
        while g.tick < start + 100 && g.hunts.on.is_none() {
            g.step();
        }
        assert!(g.hunts.on.is_some());
        g.incoming = Some((g.tick + DAY, crate::overworld::Neighbor { civ: 0, race: crate::overworld::Race::Goblin, dist: 1 }, 5, "Snaga".into()));
        for _ in 0..20 {
            g.step();
        }
        assert!(g.hunts.on.is_none());
        assert!(!matches!(g.squads[s].order, SquadOrder::Kill(_)));
    }
}
