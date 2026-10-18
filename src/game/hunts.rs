//! The overseer hunts predators (MIL-03): when wild beasts prowl near the
//! fort and a squad is fit to face them, it sends the squad out after them,
//! one pack member at a time, and calls it home when an army comes, when
//! the beasts get away or when the squad is too hurt to go on. While a
//! pack prowls near, work outdoors waits. Legendary beasts are hunted
//! only by a bold overseer with a strong squad in full kit; otherwise the
//! fort shelters from them as ever.

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
    /// After a hunt that came to nothing, no new one before this tick.
    #[serde(default)]
    pub rest_until: u64,
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
    /// The pack members still alive, where they were last we looked, and
    /// whether they were making off.
    #[serde(default)]
    pub quarry: Vec<(u32, Pos, bool)>,
    /// A legendary beast: `name` is its title.
    #[serde(default)]
    pub legend: bool,
}

/// How far from the stair (or its foot) a beast is near enough to hunt.
const NEAR: i32 = 30;
/// How hard to look for a beast's way to the stair.
const REACH: usize = 8000;

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
            return self.hunt_goes_on(o, top, base, war);
        }
        if war || self.tick < self.hunts.rest_until {
            return;
        }
        if let Some(beast) = self.legend_about(top, base) {
            return self.hunt_legend(o, beast);
        }
        // Only beasts with a way to the stair: not those shut in the caverns.
        let open: Vec<(u32, Pos, u32)> = prowlers.into_iter().filter(|p| self.creatures.get(&p.0).map_or(false, |c| self.can_reach(c, top, REACH))).collect();
        if !open.is_empty() {
            self.start_hunt(o, &open);
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
        let near = |p: Pos| prowlers.iter().any(|&(_, q, _)| self.world.outside(q) && (q.x - p.x).abs() <= r && (q.y - p.y).abs() <= r && (q.z - p.z).abs() <= 6);
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

    /// The birds have gone quiet near a dwarf at `at` (wildlife.rs): an
    /// overseer who isn't reckless reads it as a prowler coming, and work
    /// outdoors waits a while (knob hunt.ban) when it's near the fort.
    pub(super) fn heed_quiet_woods(&mut self, at: Pos) {
        let Some(mut o) = self.overseer.take() else { return };
        let r = self.knob("hunt.ban_range", 20.0) as i32 * 2;
        let home = o.top.unwrap_or(self.embark);
        if o.style != Style::Reckless && (at.x - home.x).abs() <= r && (at.y - home.y).abs() <= r {
            if !self.outdoors_banned() {
                self.count("outdoor_bans", 1);
                Self::note_ov(&mut o, self.tick, "The woods went quiet: kept the work indoors.".to_string());
            }
            self.hunts.outdoor_ban_until = self.hunts.outdoor_ban_until.max(self.tick + self.knob("hunt.ban", 300.0) as u64);
        }
        self.overseer = Some(o);
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

    /// A legendary beast near the fort (as the alert check reckons it), the nearest.
    fn legend_about(&self, top: Pos, base: Pos) -> Option<u32> {
        let near = |p: Pos| {
            let by = |q: Pos| (p.x - q.x).abs() <= NEAR && (p.y - q.y).abs() <= NEAR;
            (by(top) || by(base)) && p.z >= base.z - 2 && p.z <= top.z + 2
        };
        self.creatures
            .values()
            .filter(|c| c.legendary() && c.hostile() && !c.siege && !c.leaving && self.creature_active(c) && near(c.pos) && self.can_reach(c, top, REACH))
            .min_by_key(|c| (crate::game::combat::dist2(c.pos, top), c.id))
            .map(|c| c.id)
    }

    /// The squad most fit to hunt: (index, fit members, how many in full kit,
    /// strength). Not one away on an expedition.
    fn best_hunters(&self) -> Option<(usize, Vec<u32>, usize, i32)> {
        let away = self.expedition.as_ref().map(|e| e.squad.clone());
        (0..self.squads.len())
            .filter(|&s| Some(&self.squads[s].name) != away.as_ref())
            .map(|s| {
                let (fit, ready) = self.hunters(s);
                let strength = self.squad_strength(&fit);
                (s, fit, ready, strength)
            })
            .max_by_key(|x| (x.2, x.1.len(), x.3, std::cmp::Reverse(x.0)))
    }

    /// A legendary beast at the door: only a bold overseer goes out to meet
    /// it, and only with a strong squad, every one of them armed, armoured
    /// and unhurt (knobs hunt.legend_squad, 4, and hunt.legend, twice the
    /// beast's strength). Otherwise everyone shelters, as the alert has it.
    fn hunt_legend(&mut self, o: &mut Overseer, beast: u32) {
        if !o.has(Trait::Bold) || o.has(Trait::Cautious) || self.squads.iter().any(|s| matches!(s.order, SquadOrder::Kill(_))) {
            return;
        }
        let Some((s, ids, ready, strength)) = self.best_hunters() else { return };
        let enough = ids.len() >= self.knob("hunt.legend_squad", 4.0) as usize && ready == ids.len();
        if !enough || (strength as f64) < self.beasts_threat(&[beast]) * self.knob("hunt.legend", 2.0) {
            return;
        }
        let Some(c) = self.creatures.get(&beast) else { return };
        let (title, at) = (c.title(), c.pos);
        let squad = self.squads[s].name.clone();
        self.squads[s].order = SquadOrder::Kill(beast);
        self.call_out(&ids);
        self.hunts.on = Some(Hunt { squad: self.squads[s].id, target: beast, pack: beast, name: title.clone(), plural: title.clone(), kills: 0, since: self.tick, quarry: vec![(beast, at, false)], legend: true });
        self.count("hunts", 1);
        self.count("hunts.legend", 1);
        let text = format!("{} march out to slay {}!", squad, title);
        self.msg(&text, (255, 200, 90));
        self.spotlight("The hunt for a legend", &format!("The overseer will not cower from {}. {}", title, text), Some(at), (255, 200, 90));
        Self::note_ov(o, self.tick, format!("Sent {} to slay {}.", squad, title));
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
        let Some((s, ids, ready, strength)) = self.best_hunters() else { return };
        let fit = ids.len();
        let (target, _, pack) = prowlers[0];
        // The whole pack, not only those already in sight.
        let pack_ids: Vec<u32> = self.creatures.values().filter(|c| c.alive && !c.leaving && (c.pack == pack || c.id == pack)).map(|c| c.id).collect();
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
        self.hunts.on = Some(Hunt { squad: self.squads[s].id, target, pack, name, plural, kills: 0, since: self.tick, quarry: pack_ids.iter().filter_map(|id| self.creatures.get(id)).map(|c| (c.id, c.pos, c.leaving)).collect(), legend: false });
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
        // The alert stations the squads while a legend is near; the hunt goes on.
        if h.legend && self.squads[s].order == SquadOrder::Station && !war {
            self.squads[s].order = SquadOrder::Kill(h.target);
        }
        let mine = self.squads[s].order == SquadOrder::Kill(h.target);
        if war {
            return self.end_hunt(o, s, h, "an army is coming");
        }
        // A chase runs on further than beasts are first sought (knob
        // hunt.chase), but those fleeing are let go once past NEAR + 10.
        let (chase, flee) = (self.knob("hunt.chase", 60.0) as i32, NEAR + 10);
        let near = |c: &Creature| {
            let r = if c.leaving { flee } else { chase };
            let by = |q: Pos| (c.pos.x - q.x).abs() <= r && (c.pos.y - q.y).abs() <= r && (c.pos.z - q.z).abs() <= 8;
            by(top) || by(base)
        };
        // Those of the pack slain (or caged) since the last look, wherever
        // they fell; those that made off over the edge of the map got away.
        // Mates that turn up join the quarry. Only those near the fort are
        // hunted.
        let (w, hh) = (self.world.w, self.world.h);
        let mut quarry = Vec::new();
        for (id, last, fled) in std::mem::take(&mut h.quarry) {
            match self.creatures.get(&id) {
                Some(c) if c.alive => quarry.push((id, c.pos, c.leaving)),
                _ if fled && (last.x <= 6 || last.y <= 6 || last.x >= w - 7 || last.y >= hh - 7) => {}
                _ => {
                    h.kills += 1;
                    self.count("hunt_kills", 1);
                }
            }
        }
        for c in self.creatures.values() {
            if c.alive && !c.siege && (c.pack == h.pack || c.id == h.pack) && near(c) && !quarry.iter().any(|q| q.0 == c.id) {
                quarry.push((c.id, c.pos, c.leaving));
            }
        }
        h.quarry = quarry;
        let hunted: Vec<u32> = h.quarry.iter().filter(|q| self.creatures.get(&q.0).map_or(false, |c| near(c))).map(|q| q.0).collect();
        let (fit, _) = self.hunters(s);
        if hunted.contains(&h.target) {
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
        let next = hunted.iter().filter_map(|id| self.creatures.get(id)).min_by_key(|c| (crate::game::combat::dist2(c.pos, top), c.id)).map(|c| c.id);
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
        if h.kills > 0 && h.legend {
            let text = format!("{} slew {} at the gates of {}.", squad, h.name, fort);
            self.msg(&text, (255, 200, 90));
            self.legend(text);
            Self::note_ov(o, self.tick, format!("{} came home from the hunt: {} is slain.", squad, h.name));
        } else if h.kills > 0 {
            let what = if h.kills == 1 { format!("the {}", h.name) } else { format!("{} {}", number(h.kills), h.plural) };
            let text = format!("{} killed {} that had been prowling about {}.", squad, what, fort);
            self.msg(&text, (160, 230, 160));
            self.legend(text);
            Self::note_ov(o, self.tick, format!("{} came home from the hunt: {} slain.", squad, what));
        } else if !why.is_empty() {
            // Not again straight away (knob hunt.rest).
            self.hunts.rest_until = self.tick + self.knob("hunt.rest", 600.0) as u64;
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

    /// Open ground about `r` tiles from the fort's stair: the beasts' spot.
    fn out_by(g: &Game, r: i32) -> Pos {
        let e = g.overseer.as_ref().and_then(|o| o.top).unwrap_or(g.embark);
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
        let (mut g, s, ids) = fort(21, Style::Steady);
        // Old hands at this, and the wolves on open, level ground a way off,
        // where the squad can run them down before they slip away.
        for d in g.dwarves.iter_mut().filter(|d| ids.contains(&d.id)) {
            d.fight_xp = 3000;
        }
        let at = g.open_patch(4, 3, 8).offset(1, 1, 0);
        // Open, level ground all round them, so the chase is a fair one.
        g.level_ground(at.x - 7, at.y - 7, at.x + 7, at.y + 7, at.z);
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
            // These wolves fight to the end (a wounded one would otherwise
            // run for the map's edge, and whether it gets there is luck).
            for w in &pack {
                if let Some(c) = g.creatures.get_mut(w) {
                    c.leaving = false;
                }
            }
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

    /// A woodcutter at work with a wolf 10-14 tiles off stops, and nobody
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
        let spot = (-14..=14)
            .flat_map(|dx| (-14..=14).flat_map(move |dy| (-3..=3).map(move |dz| at.offset(dx, dy, dz))))
            .filter(|p| (p.x - at.x).abs().max((p.y - at.y).abs()) >= 10)
            .find(|&p| g.world.in_bounds(p) && g.world.walkable(p) && g.world.outside(p))
            .expect("no open ground for the wolf");
        let wolf = g.spawn_creature(Species::Wolf, spot, u64::MAX);
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

    /// A cave horror at the door: no hunt for an overseer who isn't bold,
    /// nor for a bold one with three soldiers. Four veterans in kit go, and
    /// the alert doesn't call them back to the gate.
    #[test]
    fn only_the_bold_hunt_a_legend() {
        let (mut g, s, mut ids) = fort(21, Style::Steady);
        let at = out_by(&g, 15);
        let horror = g.spawn_creature(Species::CaveHorror, at, u64::MAX);
        assert!(g.creatures[&horror].legendary());
        let mut o = g.overseer.take().unwrap();
        o.traits = vec![Trait::Bold];
        g.ov_hunts(&mut o);
        assert!(g.hunts.on.is_none(), "three green soldiers sent after a cave horror");
        let fourth = g.dwarves.iter().find(|d| d.alive && !d.is_child() && !ids.contains(&d.id)).unwrap().id;
        g.set_squad_member(s, fourth, true);
        let pos = g.dwarves.iter().find(|d| d.id == fourth).unwrap().pos;
        let axe = g.spawn_item(ItemKind::Axe, Mat::Steel, pos);
        let mail = g.spawn_item(ItemKind::Breastplate, Mat::Steel, pos);
        for it in [axe, mail] {
            g.items.get_mut(&it).unwrap().carried_by = Some(fourth);
        }
        let d = g.dwarves.iter_mut().find(|d| d.id == fourth).unwrap();
        d.held = Some(axe);
        d.armor.push(mail);
        ids.push(fourth);
        for d in g.dwarves.iter_mut().filter(|d| ids.contains(&d.id)) {
            d.fight_xp = 2000;
        }
        o.traits = vec![Trait::Greedy];
        g.ov_hunts(&mut o);
        assert!(g.hunts.on.is_none(), "an overseer who isn't bold hunted a legend");
        o.traits = vec![Trait::Bold];
        g.ov_hunts(&mut o);
        assert_eq!(g.squads[s].order, SquadOrder::Kill(horror));
        assert!(g.spotlights.iter().any(|x| x.text.contains("slay")));
        assert_eq!(g.annals.counters.get("hunts.legend").copied(), Some(1));
        g.overseer = Some(o);
        for _ in 0..30 {
            g.step();
        }
        assert!(g.alert);
        assert_eq!(g.squads[s].order, SquadOrder::Kill(horror), "called back to the gate");
        // Slain: the chronicle has it.
        g.creatures.remove(&horror);
        for _ in 0..20 {
            g.step();
        }
        assert!(g.hunts.on.is_none());
        assert!(g.overworld.events.iter().any(|e| e.text.contains("slew the cave horror")), "no chronicle line");
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
        // On open ground a way off the fort's stair, where it can be seen and run down.
        let wagon = g.embark;
        g.embark = g.overseer.as_ref().and_then(|o| o.top).unwrap_or(wagon);
        let at = g.open_patch(3, 3, 18).offset(1, 1, 0);
        g.embark = wagon;
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
        // The army turns back; the wolf is still there, but the hunters rest a while first.
        g.incoming = None;
        let mut o = g.overseer.take().unwrap();
        g.ov_hunts(&mut o);
        assert!(g.hunts.on.is_none(), "straight back out after the wolf");
        g.tick = g.hunts.rest_until;
        g.ov_hunts(&mut o);
        assert!(g.hunts.on.is_some());
        g.overseer = Some(o);
    }

    /// A beast shut in the rock by the halls is no quarry, and no reason to
    /// keep anyone indoors.
    #[test]
    fn a_beast_shut_in_the_rock_is_left_alone() {
        let (mut g, _, _) = fort(21, Style::Steady);
        let base = g.overseer.as_ref().unwrap().base.unwrap();
        let pocket = (-12..=12)
            .flat_map(|dx| (-12..=12).map(move |dy| base.offset(dx, dy, 0)))
            .find(|&p| (-1..=1).all(|x| (-1..=1).all(|y| (-1..=1).all(|z| g.world.in_bounds(p.offset(x, y, z)) && g.world.solid(p.offset(x, y, z))))))
            .expect("no solid rock by the halls");
        g.world.t_mut(pocket).kind = TileKind::Floor;
        g.spawn_creature(Species::Wolf, pocket, u64::MAX);
        let o = g.overseer.as_ref().unwrap();
        assert_eq!(g.prowlers_about(o.top.unwrap(), o.base.unwrap()).len(), 1, "the wolf prowls");
        for _ in 0..100 {
            g.step();
        }
        assert!(g.hunts.on.is_none());
        assert!(!g.outdoors_banned());
    }
}
