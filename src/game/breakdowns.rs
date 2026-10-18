//! Breakdowns that shift and mend (S09: MND-06, MND-31). Madness is no
//! longer a one-way door: a raving dwarf's rage can burn out into grief
//! (and grief can lift), while grief that festers turns to raving.
//!
//! `insane` and `melancholy` keep their meaning: melancholy is
//! `insane && melancholy`, raving is `insane && !melancholy`. A `Fit` is
//! the breakdown's acute form on top of that.

use super::combat::{adjacent, near};
use super::overseer::Overseer;
use super::*;
use serde::{Deserialize, Serialize};

/// The acute form a breakdown takes, if any.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fit {
    #[default]
    None,
    /// A berserk rage (not madness: it passes, one way or another).
    /// `holds` counts the blows that have landed on them from those trying
    /// to bring them down.
    Berserk {
        until: u64,
        #[serde(default)]
        holds: u32,
    },
    /// Lying where they fell, unreachable (they are `insane` meanwhile).
    /// `coaxed` counts today's sittings by a friend.
    Catatonic {
        since: u64,
        #[serde(default)]
        coaxed: u32,
    },
}

/// What the status line says of a broken dwarf (None for a whole one).
pub fn breakdown_mood(d: &Dwarf) -> Option<(&'static str, (u8, u8, u8))> {
    match d.fit {
        Fit::Berserk { .. } => Some(("Berserk", (255, 40, 40))),
        Fit::Catatonic { .. } => Some(("Catatonic", (140, 140, 210))),
        Fit::None if d.insane && d.melancholy => Some(("Stricken by melancholy", (150, 150, 255))),
        Fit::None if d.insane => Some(("Stark raving mad", (255, 110, 200))),
        Fit::None => None,
    }
}

impl Game {
    /// Someone who cares for them is still alive: a spouse, or a friend.
    pub(super) fn is_loved(&self, d: &Dwarf) -> bool {
        let alive = |id: u32| self.dwarves.iter().any(|o| o.id == id && o.alive);
        d.spouse.map_or(false, alive) || d.relations.iter().any(|&(o, r)| r >= 30 && alive(o))
    }

    /// Daily: madness shifts. Raving can burn out into melancholy (from which
    /// the old lifting applies), and melancholy that festers turns to raving.
    /// The catatonic may be brought back by those who sat with them.
    pub(super) fn shift_daily(&mut self) {
        let tick = self.tick;
        for i in 0..self.dwarves.len() {
            if self.dwarves[i].alive && matches!(self.dwarves[i].fit, Fit::Catatonic { .. }) {
                self.catatonia_daily(i);
                continue;
            }
            let d = &self.dwarves[i];
            if !d.alive || !d.insane || d.fit != Fit::None || tick < d.mad_since + 10 * DAY {
                continue;
            }
            if !d.melancholy {
                let p = self.knob("breakdown.ease", 0.06) * if self.is_loved(d) { 2.0 } else { 1.0 };
                if self.rng.chance(p) {
                    let d = &mut self.dwarves[i];
                    d.melancholy = true;
                    d.mad_since = tick;
                    let name = d.name.clone();
                    self.msg(&format!("{}'s rage has burned out into grief.", name), (170, 170, 255));
                    self.count("breakdown.eased", 1);
                }
            } else if d.stress >= 650 && self.rng.chance(self.knob("breakdown.worsen", 0.03)) {
                let d = &mut self.dwarves[i];
                d.melancholy = false;
                d.mad_since = tick;
                let name = d.name.clone();
                self.msg(&format!("{}'s grief has turned to raving madness.", name), (255, 110, 200));
                self.count("breakdown.worsened", 1);
            }
        }
    }
}

/// Knock a body senseless instead of killing it: the blow is pulled.
fn knock_out(b: &mut crate::body::Body) {
    let h = crate::body::Part::Head.idx();
    b.hp[h] = (b.max[h] / 4 - 1).max(0).min(b.hp[h].max(0));
}

fn dist2(a: Pos, b: Pos) -> i32 {
    (a.x - b.x).pow(2) + (a.y - b.y).pow(2) + 4 * (a.z - b.z).pow(2)
}

impl Game {
    /// In `update_dwarf`, after the psyche: a dwarf in a fit spends the turn
    /// on it (true). Also lets go of any job a mad dwarf picked up while
    /// sipping, which they would otherwise hold forever, and ends any
    /// tantrum (the mad don't throw them; one left running would go on
    /// punching people for ever).
    pub(super) fn in_a_fit(&mut self, d: &mut Dwarf) -> bool {
        if d.insane {
            d.tantrum = 0;
        }
        match d.fit {
            Fit::None => {
                if d.insane && !matches!(d.task, Task::Idle | Task::Wander { .. } | Task::DrinkItem { .. } | Task::DrinkWater { .. } | Task::Eat { .. }) {
                    self.release_task(d);
                }
                false
            }
            Fit::Berserk { until, .. } => {
                if self.tick >= until {
                    self.end_berserk(d, false);
                    return true;
                }
                if d.task != Task::Berserk {
                    self.release_task(d);
                    d.task = Task::Berserk;
                }
                self.do_berserk(d);
                true
            }
            Fit::Catatonic { .. } => {
                // Only desperate thirst or hunger stirs them, as it does the mad.
                let sipping = matches!(d.task, Task::DrinkItem { .. } | Task::DrinkWater { .. } | Task::Eat { .. });
                if sipping || d.thirst > THIRSTY + 6000 || d.hunger > HUNGRY + 12000 {
                    if d.task == Task::Catatonic {
                        d.task = Task::Idle;
                    }
                    return false;
                }
                if d.task != Task::Catatonic {
                    self.release_task(d);
                    d.task = Task::Catatonic;
                }
                d.path.clear();
                true
            }
        }
    }

    /// The tantrum trigger in `update_dwarf`: a hot-tempered dwarf who
    /// already had a tantrum in the last ten days goes berserk instead
    /// (true). Either way the tantrum is remembered.
    pub(super) fn goes_berserk(&mut self, d: &mut Dwarf) -> bool {
        let tick = self.tick;
        let again = d.last_tantrum > 0 && tick < d.last_tantrum + 10 * DAY;
        d.last_tantrum = tick;
        if !again || d.insane || d.is_child() || (d.personality.temper as f64) <= self.knob("berserk.temper", 30.0) {
            return false;
        }
        self.release_task(d);
        let ticks = self.knob("berserk.ticks", 600.0) as u64;
        d.fit = Fit::Berserk { until: tick + ticks, holds: 0 };
        d.task = Task::Berserk;
        d.tantrum = 0;
        d.stress -= 120;
        let name = d.name.clone();
        self.msg(&format!("{} has gone berserk!", name), (255, 60, 60));
        self.spotlight(
            "Berserk!",
            &format!("{} has gone berserk, and is lashing out at anyone near. Soldiers and friends will try to bring them down.", name),
            Some(d.pos),
            (255, 60, 60),
        );
        self.count("berserk", 1);
        true
    }

    /// A berserk dwarf's turn: go for the nearest dwarf on their feet and
    /// strike; with nobody near, wreck what's at hand.
    pub(super) fn do_berserk(&mut self, d: &mut Dwarf) {
        if !matches!(d.fit, Fit::Berserk { .. }) {
            self.release_task(d);
            return;
        }
        let target = self
            .dwarves
            .iter()
            .enumerate()
            .filter(|(_, o)| o.alive && o.id != d.id && !o.body.unconscious() && near(o.pos, d.pos, 15))
            .min_by_key(|(_, o)| dist2(o.pos, d.pos))
            .map(|(j, o)| (j, o.pos));
        match target {
            Some((j, p)) if adjacent(p, d.pos) => {
                if d.attack_cd == 0 {
                    self.berserk_blow(d, j);
                }
            }
            Some((_, p)) => {
                if let Travel::Failed = self.travel(d, p) {
                    d.path.clear();
                }
            }
            None => {
                if self.tick % 50 == d.id as u64 % 50 {
                    self.tantrum_mischief(d);
                }
            }
        }
    }

    /// The berserker strikes `self.dwarves[j]` with whatever is in hand.
    /// A blow that would kill is mostly pulled into a knockout: a rage
    /// wounds far more often than it slays.
    fn berserk_blow(&mut self, d: &mut Dwarf, j: usize) {
        let mut att = self.dwarf_fighter(d);
        att.dmg *= self.knob("berserk.dmg", 0.8) as f32;
        let def = self.dwarf_fighter(&self.dwarves[j]);
        let before = self.dwarves[j].body.clone();
        let mut body = before.clone();
        let mut msg = self.strike(&att, &def, &mut body).msg;
        d.attack_cd = 9 + d.body.limp();
        d.satisfy(crate::dwarf::NeedKind::Fight, 60);
        let (vid, vname) = (self.dwarves[j].id, self.dwarves[j].name.clone());
        if body.dead && !self.rng.chance(self.knob("berserk.lethal", 0.25)) {
            body = before;
            knock_out(&mut body);
            msg = Some(format!("{} knocks {} senseless!", d.called(), vname));
        }
        let tick = self.tick;
        let first = !self.crimes.iter().any(|c| c.culprit == d.id && c.victim == Some(vid) && tick < c.tick + DAY);
        {
            let v = &mut self.dwarves[j];
            v.body = body;
            v.attacker = Some((d.id, tick));
            if first {
                v.think(&format!("was attacked by {} in a berserk rage", d.name), 20, tick);
                v.adjust_regard(d.id, -30);
            }
        }
        if let Some(m) = msg {
            self.msg(&m, (255, 120, 90));
        }
        if first {
            self.report_crime(super::CrimeKind::Assault, d.id, Some(vid), format!("attacked {} in a berserk rage", vname));
        }
        if self.dwarves[j].body.dead {
            let mut v = std::mem::take(&mut self.dwarves[j]);
            self.kill(&mut v, &format!("was struck down by {} in a berserk rage", d.name));
            self.dwarves[j] = v;
            d.think(&format!("killed {} in a rage", vname), 80, tick);
            self.count("berserk.kills", 1);
            let fort = self.fortress_name.clone();
            self.legend(format!("{} slew {} in a berserk rage at {}.", d.called(), vname, fort));
        }
    }

    /// A soldier or a friend lands a blunt, held-back blow on the berserker
    /// `self.dwarves[j]`, to bring them down without killing them. After a
    /// few that land, they're wrestled to the ground.
    fn subdue_blow(&mut self, d: &mut Dwarf, j: usize) {
        let mut att = self.dwarf_fighter(d);
        att.edged = false;
        att.verb = if d.held.is_some() { "clubs" } else { "punches" };
        att.dmg *= self.knob("berserk.subdue_dmg", 0.5) as f32;
        let def = self.dwarf_fighter(&self.dwarves[j]);
        let before = self.dwarves[j].body.clone();
        let mut body = before.clone();
        let msg = self.strike(&att, &def, &mut body).msg;
        if body.dead {
            body = before;
            knock_out(&mut body);
        }
        d.attack_cd = 9 + d.body.limp();
        d.fight_xp += 4;
        let landed = msg.as_ref().map_or(false, |m| !m.contains("blocks") && !m.contains("deflects"));
        if let Some(m) = msg {
            self.msg(&m, (235, 225, 130));
        }
        let holds = self.knob("berserk.holds", 4.0) as u32;
        let b = &mut self.dwarves[j];
        b.body = body;
        let mut down = b.body.unconscious();
        if let Fit::Berserk { holds: h, .. } = &mut b.fit {
            *h += landed as u32;
            down |= *h >= holds;
        }
        if down {
            let mut b = std::mem::take(&mut self.dwarves[j]);
            let tick = self.tick;
            self.msg(&format!("{} has wrestled {} to the ground.", d.called(), b.name), (200, 200, 255));
            b.think(&format!("was subdued by {}", d.name), 10, tick);
            self.end_berserk(&mut b, true);
            self.dwarves[j] = b;
        }
    }

    /// A berserk fit ends: the rage spent, the berserker brought down, or
    /// knocked out by anything else. Then most come back to themselves; some
    /// sink into melancholy, a few go raving mad.
    pub(super) fn end_berserk(&mut self, d: &mut Dwarf, subdued: bool) {
        if !matches!(d.fit, Fit::Berserk { .. }) {
            return;
        }
        d.fit = Fit::None;
        d.tantrum = 0;
        if d.task == Task::Berserk {
            d.task = Task::Idle;
        }
        d.path.clear();
        d.stress = (d.stress - 300).max(crate::dwarf::STRESS_FLOOR);
        self.count("berserk.ended", 1);
        let r = self.rng.f64();
        if r < 0.5 {
            let name = d.name.clone();
            self.msg(&format!("{}'s rage has passed.", name), (170, 200, 255));
            self.count("mended", 1);
            let jail = self.zones.values().any(|z| z.kind == ZoneKind::Jail);
            if subdued && jail {
                d.jailed_until = self.tick + 3 * DAY;
                self.msg(&format!("{} has been locked in the jail to cool off.", name), (200, 170, 255));
            }
        } else if r < 0.8 {
            self.go_insane(d, "has sunk into melancholy after their rage");
        } else {
            self.go_insane(d, "has gone stark raving mad after their rage");
        }
    }

    /// In `body_phase`: a berserker knocked senseless by anything is out of it.
    pub(super) fn fit_body(&mut self, d: &mut Dwarf) {
        if matches!(d.fit, Fit::Berserk { .. }) && d.body.unconscious() {
            self.end_berserk(d, true);
        }
    }

    /// Berserk dwarves (not counting the one being updated) near `p`, nearest first.
    pub(super) fn berserk_near(&self, p: Pos, r: i32) -> Vec<(usize, Pos)> {
        let mut v: Vec<(usize, Pos)> = self
            .dwarves
            .iter()
            .enumerate()
            .filter(|(_, o)| o.alive && matches!(o.fit, Fit::Berserk { .. }) && near(o.pos, p, r))
            .map(|(j, o)| (j, o.pos))
            .collect();
        v.sort_by_key(|(_, q)| dist2(*q, p));
        v
    }

    /// At the top of `combat_awareness`: a dwarf in a fit neither flees nor
    /// fights the usual way, and a berserker nearby is a threat. Soldiers
    /// and the sheriff close in and bring them down, close friends and
    /// anyone struck grapple with them, everyone else gets out of reach.
    /// True when it has decided what the dwarf does.
    pub(super) fn fit_awareness(&mut self, d: &mut Dwarf) -> bool {
        if d.fit != Fit::None {
            return true;
        }
        if d.insane || d.is_child() || !self.dwarves.iter().any(|o| matches!(o.fit, Fit::Berserk { .. })) {
            return false;
        }
        let guard = d.soldier || self.position_of(d.id) == Some(super::Position::Sheriff);
        let Some(&(j, bp)) = self.berserk_near(d.pos, if guard { 14 } else { 4 }).first() else { return false };
        let bid = self.dwarves[j].id;
        let struck = d.attacker.map_or(false, |(a, t)| a == bid && self.tick < t + 40);
        if adjacent(bp, d.pos) && (guard || struck || d.regard(bid) >= 50) {
            if d.attack_cd == 0 {
                self.subdue_blow(d, j);
            }
            return true;
        }
        if guard {
            if matches!(d.task, Task::Attack { .. }) || (d.task.is_need() && d.task != Task::Flee) {
                return false;
            }
            if d.task != (Task::Station { pos: bp }) {
                self.release_task(d);
                d.task = Task::Station { pos: bp };
            }
            return true;
        }
        if matches!(d.task, Task::Wander { .. }) && !adjacent(bp, d.pos) {
            return true;
        }
        let w = &self.world;
        if let Some((q, path)) = path::search(w, d.pos, 1500, Mover::Dwarf, |q| dist2(q, bp) >= 64) {
            self.release_task(d);
            d.task = Task::Wander { target: q };
            Self::set_path(d, path);
            let name = d.name.clone();
            let bname = self.dwarves[j].name.clone();
            self.warn(&format!("flee{}", d.id), &format!("{} runs from the berserk {}!", name, bname), (255, 170, 80));
        }
        true
    }

    /// The overseer's alert check: a dwarf who has raged for a while gets
    /// the squad sent after them (when no enemy needs it).
    pub(super) fn ov_berserk(&mut self, o: &mut Overseer) {
        if self.squads.is_empty() || self.alert {
            return;
        }
        let ticks = self.knob("berserk.ticks", 600.0) as u64;
        let tick = self.tick;
        let raging = self.dwarves.iter().find_map(|d| match d.fit {
            Fit::Berserk { until, .. } if d.alive && tick + ticks > until + 100 => Some((d.name.clone(), d.pos)),
            _ => None,
        });
        let Some((name, pos)) = raging else { return };
        self.order_station(0, pos);
        let said = o.notes.iter().rev().take(5).any(|(t, n)| tick < t + ticks && n.contains(&name));
        if !said {
            Self::note_ov(o, tick, format!("Sent the squad to bring down {}, gone berserk.", name));
            self.count("berserk.squad", 1);
        }
    }
}

impl Game {
    /// Where misery would make melancholy: now and then (more often for an
    /// anxious dwarf) it is catatonia instead, and they lie where they are
    /// (true). Called from `psyche_tick`.
    pub(super) fn goes_catatonic(&mut self, d: &mut Dwarf) -> bool {
        let mut p = self.knob("catatonia.chance", 0.15);
        if d.personality.anxiety > 25 {
            p += self.knob("catatonia.anxious", 0.1);
        }
        if !self.rng.chance(p) {
            return false;
        }
        self.go_insane(d, "has gone catatonic, and lies where they are");
        d.fit = Fit::Catatonic { since: self.tick, coaxed: 0 };
        d.task = Task::Catatonic;
        d.path.clear();
        self.count("catatonic", 1);
        true
    }

    /// The day's sittings a catatonic dwarf can take in.
    fn coax_cap(&self) -> u32 {
        self.knob("coax.per_day", 2.0) as u32
    }

    /// Catatonic, and nobody sitting with them yet (or on the way).
    fn wants_coaxing(&self, p: &Dwarf) -> bool {
        p.alive
            && matches!(p.fit, Fit::Catatonic { coaxed, .. } if coaxed < self.coax_cap())
            && !self.dwarves.iter().any(|o| o.task == (Task::Coax { patient: p.id }))
    }

    /// Daily for each catatonic dwarf: each sitting with a friend gives a
    /// chance to bring them back (and a little time heals on its own).
    fn catatonia_daily(&mut self, i: usize) {
        let Fit::Catatonic { since, coaxed } = self.dwarves[i].fit else { return };
        let p = self.knob("coax.chance", 0.25) * coaxed as f64 + self.knob("catatonia.lift", 0.03);
        let back = self.rng.chance(p.min(1.0));
        let tick = self.tick;
        let d = &mut self.dwarves[i];
        d.fit = Fit::Catatonic { since, coaxed: 0 };
        if !back {
            return;
        }
        d.fit = Fit::None;
        d.insane = false;
        d.melancholy = false;
        d.misery = 0;
        if d.task == Task::Catatonic {
            d.task = Task::Idle;
        }
        d.think("came back to the world", -40, tick);
        let (id, name, spouse) = (d.id, d.name.clone(), d.spouse);
        let friend = self
            .dwarves
            .iter()
            .filter(|o| o.alive && o.id != id && (spouse == Some(o.id) || o.regard(id) >= 30))
            .max_by_key(|o| o.regard(id) + if spouse == Some(o.id) { 100 } else { 0 })
            .map(|o| o.name.clone());
        self.count("mended", 1);
        self.count("catatonia.mended", 1);
        let pos = self.dwarves[i].pos;
        match friend.filter(|_| coaxed > 0) {
            Some(f) => {
                self.msg(&format!("{} sat with {} until they spoke again.", f, name), (140, 190, 255));
                self.spotlight("Spoke again", &format!("{} sat with {} until they spoke again.", f, name), Some(pos), (140, 190, 255));
                let fort = self.fortress_name.clone();
                self.legend(format!("{} sat with {} at {} until they spoke again.", f, name, fort));
            }
            None => self.msg(&format!("{} has spoken again.", name), (140, 190, 255)),
        }
    }

    /// A finder: a spouse or friend (regard >= 30) goes and sits with a
    /// catatonic dwarf.
    pub(super) fn find_coax(&mut self, d: &mut Dwarf) -> bool {
        if d.insane || d.fit != Fit::None {
            return false;
        }
        let at: HashMap<Pos, u32> = self
            .dwarves
            .iter()
            .filter(|p| (d.spouse == Some(p.id) || d.regard(p.id) >= 30) && self.wants_coaxing(p))
            .map(|p| (p.pos, p.id))
            .collect();
        if at.is_empty() {
            return false;
        }
        let beside = |q: Pos| at.keys().any(|p| adjacent(*p, q));
        let Some((stand, path)) = path::search(&self.world, d.pos, SEARCH_LIMIT, Mover::Dwarf, beside) else { return false };
        let Some((_, &patient)) = at.iter().find(|(p, _)| adjacent(**p, stand)) else { return false };
        d.task = Task::Coax { patient };
        Self::set_path(d, path);
        true
    }

    /// Sitting with a catatonic friend: walk over, then stay a while.
    pub(super) fn do_coax(&mut self, d: &mut Dwarf, patient: u32) {
        let Some(p) = self.dwarves.iter().find(|p| p.id == patient && p.alive && matches!(p.fit, Fit::Catatonic { .. })) else {
            self.release_task(d);
            return;
        };
        let (ppos, pname) = (p.pos, p.name.clone());
        if !adjacent(ppos, d.pos) {
            if let Travel::Failed = self.travel(d, ppos) {
                self.release_task(d);
                d.idle_cd = 100;
            }
            return;
        }
        if !Self::labor(d, 200, 0) {
            return;
        }
        d.task = Task::Idle;
        let tick = self.tick;
        d.think(&format!("sat with {}, who said nothing", pname), 4, tick);
        d.satisfy(crate::dwarf::NeedKind::Socialize, 150);
        self.count("coax", 1);
        if let Some(p) = self.dwarves.iter_mut().find(|p| p.id == patient) {
            if let Fit::Catatonic { coaxed, .. } = &mut p.fit {
                *coaxed += 1;
            }
            p.adjust_regard(d.id, 5);
        }
    }

    /// The overseer's daily round: a sentimental overseer asks a catatonic
    /// dwarf's closest friend to leave their work and sit with them.
    pub(super) fn ov_coax(&mut self, o: &mut Overseer) {
        if !o.has(super::personality::Trait::Sentimental) || self.alert {
            return;
        }
        let tick = self.tick;
        let lonely: Vec<(u32, String, Option<u32>)> =
            self.dwarves.iter().filter(|p| self.wants_coaxing(p)).map(|p| (p.id, p.name.clone(), p.spouse)).collect();
        for (patient, pname, spouse) in lonely {
            let friend = self
                .dwarves
                .iter()
                .enumerate()
                .filter(|(_, f)| {
                    f.alive
                        && f.id != patient
                        && !f.insane
                        && f.fit == Fit::None
                        && !f.is_child()
                        && !f.task.is_need()
                        && f.mood.is_none()
                        && f.jailed_until == 0
                        && (spouse == Some(f.id) || f.regard(patient) >= 30)
                })
                .max_by_key(|(_, f)| f.regard(patient) + if spouse == Some(f.id) { 100 } else { 0 })
                .map(|(j, f)| (j, f.name.clone()));
            let Some((j, fname)) = friend else { continue };
            let mut f = std::mem::take(&mut self.dwarves[j]);
            self.release_task(&mut f);
            f.task = Task::Coax { patient };
            f.path.clear();
            self.dwarves[j] = f;
            self.count("coax.asked", 1);
            if !o.notes.iter().rev().take(8).any(|(_, n)| n.contains(&pname) && n.contains("sit with")) {
                Self::note_ov(o, tick, format!("Asked {} to sit with {}, who lies unspeaking.", fname, pname));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A dwarf gone mad: raving, or melancholic.
    fn mad(g: &mut Game, i: usize, melancholy: bool) -> u32 {
        let mut d = std::mem::take(&mut g.dwarves[i]);
        let how = if melancholy { "has been stricken by melancholy" } else { "has gone stark raving mad, unable to finish their work" };
        g.go_insane(&mut d, how);
        let id = d.id;
        g.dwarves[i] = d;
        id
    }

    #[test]
    fn raving_eases_into_grief_and_grief_lifts() {
        let mut g = Game::new(11);
        g.creatures.retain(|_, c| c.tame);
        let id = mad(&mut g, 0, false);
        let spouse = g.dwarves[1].id;
        g.dwarves[0].spouse = Some(spouse);
        g.dwarves[1].spouse = Some(id);
        assert_eq!(breakdown_mood(&g.dwarves[0]).map(|m| m.0), Some("Stark raving mad"));
        for _ in 0..20 {
            g.tick += DAY;
            g.shift_daily();
            // The old lifting, as psyche_tick does it once a day.
            let mut d = std::mem::take(&mut g.dwarves[0]);
            let t = g.tick;
            g.tick = t - t % DAY + d.id as u64 % DAY;
            g.psyche_tick(&mut d);
            g.tick = t;
            g.dwarves[0] = d;
        }
        let d = &g.dwarves[0];
        assert!(!d.insane || d.melancholy, "still raving after 20 days with a living spouse");
        assert!(g.log.iter().any(|l| l.text.contains("rage has burned out into grief")));
        assert_eq!(g.annals.counters.get("breakdown.eased"), Some(&1));
    }

    #[test]
    fn festering_grief_turns_to_raving() {
        let mut g = Game::new(11);
        g.creatures.retain(|_, c| c.tame);
        g.knobs.set.insert("breakdown.worsen".into(), 1.0);
        mad(&mut g, 0, true);
        g.dwarves[0].stress = 700;
        g.tick += 9 * DAY;
        g.shift_daily();
        assert!(g.dwarves[0].melancholy, "too soon to turn");
        g.tick += DAY;
        g.shift_daily();
        assert!(g.dwarves[0].insane && !g.dwarves[0].melancholy, "the grief should have turned to raving");
        assert_eq!(breakdown_mood(&g.dwarves[0]).map(|m| m.0), Some("Stark raving mad"));
        // A calmer one stays melancholic.
        mad(&mut g, 1, true);
        g.dwarves[1].stress = 100;
        g.tick += 20 * DAY;
        g.shift_daily();
        assert!(g.dwarves[1].melancholy);
    }

    /// Dwarf `i` on the brink of a second tantrum, with a hot temper.
    fn hothead(g: &mut Game, i: usize, at: Pos) -> u32 {
        let tick = g.tick.max(1);
        let d = &mut g.dwarves[i];
        d.personality.temper = 50;
        d.stress = 2000;
        d.last_tantrum = tick;
        d.pos = at;
        d.path.clear();
        d.thirst = 0;
        d.hunger = 0;
        d.fatigue = 0;
        d.id
    }

    #[test]
    fn a_berserk_dwarf_is_subdued() {
        let mut g = Game::new(11);
        g.creatures.retain(|_, c| c.tame);
        let c = g.open_patch(5, 3, 8);
        let id = hothead(&mut g, 0, c.offset(2, 1, 0));
        for (i, dx) in [(1, 0), (2, 4)] {
            let s = &mut g.dwarves[i];
            s.soldier = true;
            s.fight_xp = 1500;
            s.pos = c.offset(dx, 1, 0);
            s.path.clear();
        }
        let n = g.dwarves.iter().filter(|d| d.alive).count();
        let mut raged = false;
        let mut calm_at = None;
        for t in 0..400 {
            g.step();
            let d = g.dwarves.iter().find(|d| d.id == id).unwrap();
            if matches!(d.fit, Fit::Berserk { .. }) {
                raged = true;
            } else if raged {
                calm_at = Some(t);
                break;
            }
        }
        assert!(raged, "the dwarf never went berserk");
        let d = g.dwarves.iter().find(|d| d.id == id).unwrap();
        assert!(calm_at.is_some() || d.body.unconscious(), "still raging after 400 ticks");
        assert_eq!(g.dwarves.iter().filter(|d| d.alive).count(), n, "somebody died");
        assert!(g.log.iter().any(|l| l.text.contains("has gone berserk")));
        assert!(g.log.iter().any(|l| l.text.contains("wrestled") || l.text.contains("rage has passed")));
        assert!(g.crimes.iter().any(|c| c.culprit == id && c.kind == super::super::CrimeKind::Assault), "no assault on the record");
        assert_eq!(g.annals.counters.get("berserk"), Some(&1));
        assert!(!g.spotlights.is_empty());
    }

    #[test]
    fn a_first_tantrum_is_only_a_tantrum() {
        let mut g = Game::new(11);
        g.creatures.retain(|_, c| c.tame);
        let c = g.open_patch(5, 3, 8);
        let id = hothead(&mut g, 0, c);
        g.dwarves[0].last_tantrum = 0;
        for _ in 0..5 {
            g.step();
        }
        let d = g.dwarves.iter().find(|d| d.id == id).unwrap();
        assert!(d.tantrum > 0 && d.fit == Fit::None, "a first tantrum shouldn't be berserk");
        assert!(d.last_tantrum > 0);
        // A mild dwarf's second tantrum is a tantrum too.
        let id = hothead(&mut g, 1, c.offset(4, 0, 0));
        g.dwarves[1].personality.temper = 10;
        for _ in 0..5 {
            g.step();
        }
        let d = g.dwarves.iter().find(|d| d.id == id).unwrap();
        assert!(d.tantrum > 0 && d.fit == Fit::None);
    }

    #[test]
    fn bystanders_run_from_a_berserker() {
        let mut g = Game::new(11);
        g.creatures.retain(|_, c| c.tame);
        let c = g.open_patch(5, 3, 8);
        let a = hothead(&mut g, 0, c);
        g.dwarves[0].fit = Fit::Berserk { until: g.tick + 600, holds: 0 };
        g.dwarves[0].task = Task::Berserk;
        g.dwarves[1].pos = c.offset(3, 0, 0);
        g.dwarves[1].soldier = false;
        let b = g.dwarves[1].id;
        g.dwarves[1].relations.retain(|r| r.0 != a);
        for _ in 0..10 {
            g.step();
        }
        let d = g.dwarves.iter().find(|d| d.id == b).unwrap();
        assert!(matches!(d.task, Task::Wander { .. }), "the bystander should run: {:?}", d.task);
    }

    /// A working well near the wagon (as in water.rs's tests).
    fn built_well(g: &mut Game) -> Pos {
        let e = g.embark;
        for r in 2..20 {
            for dx in -r..=r {
                let p = e.offset(dx, r, 0);
                let (a, b) = (p.down(), p.down().down());
                let floor_ok = g.world.walkable(p) && g.world.t(p).building == 0 && !g.items.values().any(|it| it.pos == p);
                if floor_ok && g.world.solid(a) && g.world.solid(b) && g.world.walkable(p.offset(-1, 0, 0)) {
                    g.world.t_mut(p).kind = TileKind::Open;
                    g.world.t_mut(a).kind = TileKind::Open;
                    g.world.t_mut(a).water = 7;
                    g.water_cells = None;
                    assert!(g.place_building(BuildingKind::Well, p));
                    let id = g.world.t(p).building;
                    let b = g.buildings.get_mut(&id).unwrap();
                    b.built = true;
                    b.mat = Mat::Granite;
                    return p;
                }
            }
        }
        panic!("nowhere to dig a well");
    }

    #[test]
    fn a_catatonic_dwarf_is_coaxed_back_and_given_water() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        built_well(&mut g);
        let e = g.embark;
        g.spawn_item(ItemKind::Bucket, Mat::Wood, e);
        for d in g.dwarves.iter_mut() {
            d.labors[Labor::Medicine.idx()] = false;
        }
        g.dwarves[2].labors[Labor::Medicine.idx()] = true;
        // Dwarf 0 breaks down into catatonia; dwarf 1 is their friend.
        g.knobs.set.insert("catatonia.chance".into(), 1.0);
        let (id, friend) = (g.dwarves[0].id, g.dwarves[1].id);
        g.dwarves[1].relations.retain(|r| r.0 != id);
        g.dwarves[1].relations.push((id, 60));
        let mut d = std::mem::take(&mut g.dwarves[0]);
        assert!(g.goes_catatonic(&mut d));
        assert!(d.insane && !d.melancholy && d.task == Task::Catatonic);
        assert_eq!(breakdown_mood(&d).map(|m| m.0), Some("Catatonic"));
        d.thirst = THIRSTY / 2 + 50;
        let at = d.pos;
        g.dwarves[0] = d;
        let mut back = None;
        for t in 0..5 * DAY {
            g.step();
            let d = g.dwarves.iter_mut().find(|d| d.id == id).unwrap();
            d.hunger = 0;
            if d.fit == Fit::None {
                back = Some(t);
                break;
            }
            assert_eq!(d.pos, at, "a catatonic dwarf doesn't move");
            assert!(d.alive);
        }
        assert!(back.is_some(), "still catatonic after five days");
        let d = g.dwarves.iter().find(|d| d.id == id).unwrap();
        assert!(!d.insane, "back to themselves");
        let name = d.name.clone();
        assert!(g.log.iter().any(|l| l.text.contains("bringing water for") && l.text.contains(&name)), "no doctor brought water");
        let fname = g.dwarves.iter().find(|d| d.id == friend).unwrap().name.clone();
        assert!(g.log.iter().any(|l| l.text.contains("until they spoke again") && l.text.contains(&fname)), "the friend didn't bring them back");
        assert!(g.annals.counters.get("coax").copied().unwrap_or(0) >= 1);
        assert_eq!(g.annals.counters.get("catatonic"), Some(&1));
        assert_eq!(g.annals.counters.get("mended"), Some(&1));
    }

    #[test]
    fn a_sentimental_overseer_sends_a_friend() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        g.knobs.set.insert("catatonia.chance".into(), 1.0);
        let id = g.dwarves[0].id;
        g.dwarves[1].relations.retain(|r| r.0 != id);
        g.dwarves[1].relations.push((id, 40));
        let mut d = std::mem::take(&mut g.dwarves[0]);
        assert!(g.goes_catatonic(&mut d));
        g.dwarves[0] = d;
        g.alert = false;
        let mut o = Overseer::new(super::super::Style::Steady);
        o.traits = vec![super::super::personality::Trait::Cautious];
        g.ov_coax(&mut o);
        let sent = |g: &Game| g.dwarves.iter().any(|f| f.task == Task::Coax { patient: id });
        assert!(!sent(&g), "only a sentimental overseer");
        o.traits = vec![super::super::personality::Trait::Sentimental];
        g.ov_coax(&mut o);
        assert!(sent(&g), "nobody was sent");
        assert!(o.notes.iter().any(|(_, n)| n.contains("sit with")));
    }

    #[test]
    fn a_desperate_catatonic_dwarf_stirs_to_drink_and_throws_no_tantrum() {
        let mut g = Game::new(11);
        g.creatures.retain(|_, c| c.tame);
        g.knobs.set.insert("catatonia.chance".into(), 1.0);
        let mut d = std::mem::take(&mut g.dwarves[0]);
        assert!(g.goes_catatonic(&mut d));
        d.thirst = THIRSTY + 7000;
        d.hunger = 0;
        d.tantrum = 900;
        d.stress = 3000;
        let id = d.id;
        g.dwarves[0] = d;
        for _ in 0..600 {
            g.step();
            let d = g.dwarves.iter().find(|d| d.id == id).unwrap();
            assert_eq!(d.tantrum, 0, "a mad dwarf keeps a tantrum going");
        }
        let d = g.dwarves.iter().find(|d| d.id == id).unwrap();
        assert!(d.alive && d.thirst < 1000, "never drank (thirst {}, task {:?})", d.thirst, d.task);
        assert!(matches!(d.fit, Fit::Catatonic { .. }) || !d.insane);
    }
}
