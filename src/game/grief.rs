//! Grief and vengeance (C02): a death ripples through the fort.
//!
//! - *Grief needs a body* (MND-21): those who saw a dwarf die grieve at
//!   once, and their kin and friends with them. A death nobody saw (a lone
//!   woodcutter, a miner in the dark) leaves the kin only worried until
//!   someone comes across the body, or until, ten days on, the dwarf is
//!   presumed dead and mourned a little less sharply.
//! - *Grief wandering* (MND-11): for a season, the spouse, children,
//!   parents and close friends of the dead go, one idle hour a day, to sit
//!   by the grave (or where they fell), and it comforts them.
//! - *Vengeance vow* (MND-05): when a creature killed them, those closest
//!   may swear vengeance on its kind. The overseer drafts the vowed; a
//!   vowed soldier picks that kind out of a crowd of foes, and an armed
//!   vowed civilian charges it instead of fleeing. A vow kept is a legend
//!   and a name ("the Avenger"); a vow two years unkept is a shame.
//! - *Traumatic triggers* (MND-07): a mourner who passes where the dead
//!   fell is reminded of it.
//!
//! Who swears comes from a hash of the mourner and the dead, never the
//! game's dice.

use super::overseer::Overseer;
use super::*;
use serde::{Deserialize, Serialize};

fn dist2(a: Pos, b: Pos) -> i32 {
    (a.x - b.x).pow(2) + (a.y - b.y).pow(2) + (a.z - b.z).pow(2)
}

/// A vow of vengeance on a kind of creature (or the very one).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vow {
    pub who: u32,
    /// The kind of creature, by its name ("goblin").
    pub foe: String,
    /// The very creature that struck the blow, if it's known.
    #[serde(default)]
    pub foe_id: Option<u32>,
    pub for_whom: u32,
    pub for_name: String,
    pub sworn: u64,
    /// The overseer has put them in a squad for it.
    #[serde(default)]
    pub drafted: bool,
    /// When they last saw the foe come again.
    #[serde(default)]
    pub roused: u64,
}

/// A thought held back until the body is found.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Sorrow {
    pub who: u32,
    pub text: String,
    pub stress: i32,
    /// Kin or a close friend: they will mourn, and may swear a vow.
    pub close: bool,
    /// Kin, not a friend.
    #[serde(default)]
    pub kin: bool,
}

/// A dwarf who died with nobody there to see.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Missing {
    pub dead: u32,
    pub name: String,
    pub since: u64,
    pub at: Pos,
    pub corpse: Option<u32>,
    pub grief: Vec<Sorrow>,
    pub killer: Option<String>,
    pub killer_id: Option<u32>,
}

/// One dwarf mourning another, for a season.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Mourner {
    pub who: u32,
    pub dead: u32,
    pub name: String,
    pub until: u64,
    /// Last set out for the grave, and where.
    pub visited: u64,
    pub going: Option<Pos>,
    pub visits: u16,
    /// Last reminded by passing where the dead fell.
    pub reminded: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GriefBook {
    #[serde(default)]
    pub missing: Vec<Missing>,
    #[serde(default)]
    pub mourners: Vec<Mourner>,
    #[serde(default)]
    pub vows: Vec<Vow>,
    /// Where dwarves died: (place, who, when), the last thirty.
    #[serde(default)]
    pub dark_places: Vec<(Pos, u32, u64)>,
}

/// How far a death can be seen from.
const SIGHT: i32 = 8;
/// After this long unfound, a dwarf is presumed dead.
const PRESUMED: u64 = DAY * 10;
/// A vow unkept this long is broken.
const VOW_YEARS: u64 = YEAR * 2;

/// "goblin" → "goblins", "wolf" → "wolves".
pub fn plural(s: &str) -> String {
    if s.ends_with("lf") {
        format!("{}ves", &s[..s.len() - 1])
    } else if s.ends_with('y') && !s.ends_with("ey") {
        format!("{}ies", &s[..s.len() - 1])
    } else if s.ends_with('s') || s.ends_with('x') || s.ends_with("ch") {
        format!("{}es", s)
    } else {
        format!("{}s", s)
    }
}

fn first(name: &str) -> &str {
    name.split(' ').next().unwrap_or(name)
}

impl Game {
    /// From `kill`: who grieves, now or once the body is found.
    pub(super) fn grieve(&mut self, d: &Dwarf, cause: &str) {
        let tick = self.tick;
        let (dead, at, name) = (d.id, d.pos, d.name.clone());
        let (spouse, parents) = (d.spouse, d.parents);
        let killer = self.annals.deaths.last().filter(|n| n.name == d.name && n.tick == tick).and_then(|n| n.killer.clone());
        let killer_id = d.attacker.filter(|a| killer.is_some() && tick < a.1 + DAY).map(|a| a.0);
        let seen: Vec<u32> = self
            .dwarves
            .iter()
            .filter(|o| o.alive && o.id != dead && !o.asleep && o.pos.z == at.z && (o.pos.x - at.x).abs() <= SIGHT && (o.pos.y - at.y).abs() <= SIGHT)
            .map(|o| o.id)
            .collect();
        // Dying in one's bed of old age, or a babe in arms, is found at once.
        let quiet = cause.contains("old age") || d.is_child() || d.ghost;
        let mut grief = Vec::new();
        for o in self.dwarves.iter().filter(|o| o.alive && o.id != dead) {
            let regard = o.regard(dead);
            let s = |text: String, stress: i32, close: bool, kin: bool| Sorrow { who: o.id, text, stress, close, kin };
            grief.push(if spouse == Some(o.id) {
                s(format!("lost their spouse {}", name), 140, true, true)
            } else if parents.map_or(false, |(m, f)| m == o.id || f == o.id) {
                s(format!("lost their child {}", name), 150, true, true)
            } else if o.family_of(dead) {
                s(format!("lost their parent {}", name), 110, true, true)
            } else if regard >= 30 {
                s(format!("lost their dear friend {}", name), 90, regard >= 50, false)
            } else if regard <= -40 {
                s(format!("felt satisfied at the death of {}", name), -10, false, false)
            } else if seen.contains(&o.id) {
                s(format!("saw {} die", name), 25, false, false)
            } else {
                continue;
            });
        }
        let gb = &mut self.society.grief;
        gb.dark_places.push((at, dead, tick));
        if gb.dark_places.len() > 30 {
            gb.dark_places.remove(0);
        }
        if !seen.is_empty() || quiet {
            self.deliver_grief(dead, &name, grief, 100, killer, killer_id);
            return;
        }
        // Nobody saw: the kin only wonder, for now.
        let corpse = self.items.values().filter(|it| it.kind == ItemKind::Corpse && it.pos == at && it.label.as_deref() == Some(name.as_str())).map(|it| it.id).max();
        for s in grief.iter().filter(|s| s.close) {
            if let Some(o) = self.dwarves.iter_mut().find(|o| o.id == s.who) {
                o.think(&format!("worried that {} has not come back", first(&name)), 10, tick);
            }
        }
        self.society.grief.missing.push(Missing { dead, name, since: tick, at, corpse, grief, killer, killer_id });
    }

    /// The news of a death reaches those it touches (`pct` of the full grief).
    fn deliver_grief(&mut self, dead: u32, name: &str, grief: Vec<Sorrow>, pct: i32, killer: Option<String>, killer_id: Option<u32>) {
        let tick = self.tick;
        for s in grief {
            let Some(i) = self.dwarves.iter().position(|o| o.id == s.who && o.alive) else { continue };
            self.dwarves[i].think(&s.text, s.stress * pct / 100, tick);
            if !s.close {
                continue;
            }
            let gb = &mut self.society.grief;
            gb.mourners.retain(|m| !(m.who == s.who && m.dead == dead));
            gb.mourners.push(Mourner { who: s.who, dead, name: name.to_string(), until: tick + SEASON, ..Default::default() });
            if let Some(k) = &killer {
                self.maybe_swear(i, dead, name, k, killer_id, s.kin);
            }
        }
    }

    /// The `i`th dwarf may swear vengeance for `dead` on the `foe`.
    fn maybe_swear(&mut self, i: usize, dead: u32, name: &str, foe: &str, foe_id: Option<u32>, kin: bool) {
        let o = &self.dwarves[i];
        let temper = o.personality.temper;
        if o.is_child() || o.insane || temper <= -10 || self.society.grief.vows.iter().any(|v| v.who == o.id) {
            return;
        }
        let odds = 30 + temper + if kin { 20 } else { 0 };
        if (super::names::hash(&[o.id as u64, dead as u64, 0x766f77]) % 100) as i32 >= odds {
            return;
        }
        let tick = self.tick;
        let (who, pos, called) = (o.id, o.pos, o.called());
        let foes = plural(foe);
        self.society.grief.vows.push(Vow { who, foe: foe.to_string(), foe_id, for_whom: dead, for_name: name.to_string(), sworn: tick, ..Default::default() });
        self.dwarves[i].think(&format!("swore to avenge {} on the {}", first(name), foes), -5, tick);
        let text = format!("{} swears vengeance on the {} that killed {}!", called, foes, first(name));
        self.msg_at(&text, (255, 120, 80), Some(pos), logkind::LogKind::Death);
        self.spotlight("A vow of vengeance", &format!("Over the body of {}, {} has sworn to slay the {} who did it.", name, called, foes), Some(pos), (255, 120, 80));
        self.record(super::memory::MemoryKind::Feat, &[who, dead], Some(pos), format!("{} swore vengeance on the {} for {}.", called, foes, name), 5);
        self.gossip(text);
        self.count("soc.vow_sworn", 1);
    }

    /// Every fifty ticks: bodies found, graves visited, old wounds reopened,
    /// and the vowed see their foe come again.
    pub(super) fn grief_tick(&mut self) {
        let tick = self.tick;
        // The missing: found, or given up for dead.
        let mut i = 0;
        while i < self.society.grief.missing.len() {
            let m = &self.society.grief.missing[i];
            let body = m.corpse.and_then(|c| self.items.get(&c));
            let at = body.map_or(m.at, |it| it.pos);
            let finder = match body {
                None => Some(None),
                Some(it) if it.carried_by.is_some() => Some(it.carried_by),
                Some(_) => self
                    .dwarves
                    .iter()
                    .find(|o| o.alive && o.id != m.dead && !o.asleep && o.pos.z == at.z && (o.pos.x - at.x).abs() <= 2 && (o.pos.y - at.y).abs() <= 2)
                    .map(|o| Some(o.id)),
            };
            if finder.is_none() && tick < m.since + PRESUMED {
                i += 1;
                continue;
            }
            let m = self.society.grief.missing.remove(i);
            let first_name = first(&m.name).to_string();
            match finder {
                Some(f) => {
                    let who = f.and_then(|f| self.dwarves.iter_mut().find(|o| o.id == f && o.alive));
                    let text = match who {
                        Some(o) => {
                            o.think(&format!("found the body of {}", first_name), 15, tick);
                            format!("{} found the body of {}.", o.called(), m.name)
                        }
                        None => format!("The remains of {} have been found.", m.name),
                    };
                    self.msg_at(&text, (200, 160, 160), Some(at), logkind::LogKind::Death);
                    self.gossip(text);
                    self.count("soc.found_body", 1);
                    self.deliver_grief(m.dead, &m.name, m.grief, 100, m.killer, m.killer_id);
                }
                None => {
                    self.msg(&format!("{} has not come back, and is presumed dead.", m.name), (200, 160, 160));
                    self.count("soc.presumed_dead", 1);
                    self.deliver_grief(m.dead, &m.name, m.grief, 70, m.killer, m.killer_id);
                }
            }
        }
        // Mourners at the graveside, or reminded where the dead fell.
        for k in 0..self.society.grief.mourners.len() {
            let m = self.society.grief.mourners[k].clone();
            if tick >= m.until {
                continue;
            }
            let Some(i) = self.dwarves.iter().position(|o| o.id == m.who && o.alive) else { continue };
            let pos = self.dwarves[i].pos;
            if let Some(p) = m.going {
                if p.z == pos.z && (p.x - pos.x).abs() <= 1 && (p.y - pos.y).abs() <= 1 {
                    let grave = self.buildings.values().any(|b| b.occupant == Some(m.dead) && b.pos == p);
                    let text = if grave { format!("sat by {}'s grave", first(&m.name)) } else { format!("stood where {} fell", first(&m.name)) };
                    self.dwarves[i].think(&text, -6, tick);
                    let gm = &mut self.society.grief.mourners[k];
                    gm.going = None;
                    gm.visits += 1;
                    if gm.visits == 1 {
                        let called = self.dwarves[i].called();
                        self.gossip(format!("{} {}, a long while.", called, text));
                    }
                } else if tick > m.visited + DAY / 2 {
                    self.society.grief.mourners[k].going = None;
                }
                continue;
            }
            if tick < m.reminded + DAY {
                continue;
            }
            let near = self.society.grief.dark_places.iter().any(|&(p, who, _)| who == m.dead && p.z == pos.z && (p.x - pos.x).abs() <= 2 && (p.y - pos.y).abs() <= 2);
            if near {
                self.dwarves[i].think(&format!("was reminded of {}'s death", first(&m.name)), 6, tick);
                self.society.grief.mourners[k].reminded = tick;
            }
        }
        // The foe comes again.
        if self.raided() {
            for k in 0..self.society.grief.vows.len() {
                let v = self.society.grief.vows[k].clone();
                if tick < v.roused + DAY * 5 || !self.creatures.values().any(|c| c.alive && c.hostile() && c.name() == v.foe) {
                    continue;
                }
                let Some(o) = self.dwarves.iter().find(|o| o.id == v.who && o.alive && !o.insane) else { continue };
                let (called, pos) = (o.called(), o.pos);
                self.society.grief.vows[k].roused = tick;
                self.msg_at(&format!("{} remembers {}: the {} have come again.", called, first(&v.for_name), plural(&v.foe)), (255, 120, 80), Some(pos), logkind::LogKind::Death);
            }
        }
    }

    /// Daily: mourning ends, vows long unkept are broken.
    pub(super) fn grief_daily(&mut self) {
        let tick = self.tick;
        let alive = |g: &Game, id: u32| g.dwarves.iter().any(|o| o.id == id && o.alive);
        let gb = &self.society.grief;
        let mourners: Vec<Mourner> = gb.mourners.iter().filter(|m| tick < m.until && alive(self, m.who)).cloned().collect();
        self.society.grief.mourners = mourners;
        self.society.grief.dark_places.retain(|p| tick < p.2 + YEAR);
        let vows = std::mem::take(&mut self.society.grief.vows);
        for v in vows {
            let Some(i) = self.dwarves.iter().position(|o| o.id == v.who && o.alive) else { continue };
            if tick < v.sworn + VOW_YEARS {
                self.society.grief.vows.push(v);
                continue;
            }
            self.dwarves[i].think(&format!("never kept their vow to avenge {}", first(&v.for_name)), 30, tick);
            let called = self.dwarves[i].called();
            self.gossip(format!("{} swore to avenge {}, and never did. They don't speak of it.", called, first(&v.for_name)));
            self.count("soc.vow_broken", 1);
        }
    }

    /// The leisure hook: a mourner who hasn't been today goes to the grave,
    /// or to where the dead fell.
    pub(super) fn grief_visit(&mut self, d: &mut Dwarf) -> bool {
        let tick = self.tick;
        let Some(k) = self.society.grief.mourners.iter().position(|m| m.who == d.id && tick < m.until && m.going.is_none() && tick >= m.visited + DAY) else { return false };
        if self.raided() {
            return false;
        }
        let dead = self.society.grief.mourners[k].dead;
        let grave = self.buildings.values().filter(|b| b.occupant == Some(dead) && matches!(b.kind, BuildingKind::Coffin | BuildingKind::Slab)).map(|b| b.pos).next();
        let fell = self.society.grief.dark_places.iter().rev().find(|p| p.1 == dead).map(|p| p.0);
        let target = grave.or(fell).filter(|&p| p != d.pos);
        let m = &mut self.society.grief.mourners[k];
        m.visited = tick;
        let Some(target) = target else { return false };
        m.going = Some(target);
        d.task = Task::Wander { target };
        true
    }

    /// A vowed civilian with a weapon in hand charges the foe they swore
    /// against rather than fleeing it. True if they did.
    pub(super) fn vow_charge(&mut self, d: &mut Dwarf, threats: &[(u32, Pos)]) -> bool {
        if self.society.grief.vows.is_empty() || d.held.and_then(|h| self.items.get(&h)).map_or(true, |it| it.kind.weapon_damage().is_none()) {
            return false;
        }
        let Some(target) = self.vow_target(d, threats) else { return false };
        if d.task != (Task::Attack { target }) {
            self.release_task(d);
            d.task = Task::Attack { target };
            let v = self.society.grief.vows.iter().find(|v| v.who == d.id).unwrap();
            let text = format!("{} charges the {}, crying out for {}!", d.called(), v.foe, first(&v.for_name));
            self.warn(&format!("vow{}", d.id), &text, (255, 120, 80));
        }
        true
    }

    /// Among `threats`, the foe `d` swore vengeance on: the very one, else
    /// the nearest of its kind.
    pub(super) fn vow_target(&self, d: &Dwarf, threats: &[(u32, Pos)]) -> Option<u32> {
        let v = self.society.grief.vows.iter().find(|v| v.who == d.id)?;
        if let Some(t) = threats.iter().find(|t| Some(t.0) == v.foe_id) {
            return Some(t.0);
        }
        threats.iter().filter(|t| self.creatures.get(&t.0).map_or(false, |c| c.name() == v.foe)).min_by_key(|t| (dist2(t.1, d.pos), t.0)).map(|t| t.0)
    }

    /// From `credit_kill`: a vow kept.
    pub(super) fn vow_kill(&mut self, d: &mut Dwarf, c: &Creature) {
        let Some(k) = self.society.grief.vows.iter().position(|v| v.who == d.id && (v.foe_id == Some(c.id) || v.foe == c.name())) else { return };
        let v = self.society.grief.vows.remove(k);
        let tick = self.tick;
        let before = d.called();
        d.think(&format!("avenged {}", first(&v.for_name)), -60, tick);
        self.society.grief.mourners.retain(|m| !(m.who == d.id && m.dead == v.for_whom));
        let text = format!("{} has avenged {}, slaying {}!", before, v.for_name, c.title());
        self.msg_at(&text, (255, 215, 60), Some(d.pos), logkind::LogKind::Death);
        self.spotlight("Vengeance", &format!("{} swore to avenge {} on the {}, and has kept the vow.", before, v.for_name, plural(&v.foe)), Some(d.pos), (255, 215, 60));
        self.gossip(text);
        let fort = self.fortress_name.clone();
        self.legend(format!("{} of {} avenged {}, slaying {}.", before, fort, v.for_name, c.title()));
        self.record(super::memory::MemoryKind::Feat, &[d.id, v.for_whom], Some(d.pos), format!("{} avenged {}, slaying {}.", before, v.for_name, c.title()), 6);
        self.earn_epithet(d, "Avenger", super::heroes::RANK_FEAT + 1, &format!("avenged {}", v.for_name));
        self.count("soc.vow_kept", 1);
    }

    /// The overseer's round: the vowed get a place in a squad.
    pub(super) fn ov_vows(&mut self, o: &mut Overseer) {
        if self.squads.is_empty() {
            return;
        }
        for k in 0..self.society.grief.vows.len() {
            let v = self.society.grief.vows[k].clone();
            if v.drafted {
                continue;
            }
            let Some(d) = self.dwarves.iter().find(|d| d.id == v.who && d.alive) else { continue };
            self.society.grief.vows[k].drafted = true;
            if d.is_child() || d.insane || d.noble.is_some() || d.soldier || self.position_of(d.id).is_some() || self.squad_of(d.id).is_some() {
                continue;
            }
            let called = d.called();
            let s = (0..self.squads.len()).min_by_key(|&i| self.squads[i].members.len()).unwrap_or(0);
            self.set_squad_member(s, v.who, true);
            let squad = self.squads[s].name.clone();
            Self::note_as(o, self.tick, super::personality::NoteKind::Plain, format!("{} swore to avenge {}. I gave them a place in {}.", called, first(&v.for_name), squad));
            self.count("soc.vow_drafted", 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::Species;

    /// A quiet fort: no wild animals, everyone awake and apart.
    fn quiet(seed: u64) -> Game {
        let mut g = Game::new(seed);
        g.creatures.retain(|_, c| c.tame);
        g
    }

    #[test]
    fn grief_waits_for_the_body() {
        let mut g = quiet(41);
        let (a, b) = (g.dwarves[0].id, g.dwarves[1].id);
        g.dwarves[0].spouse = Some(b);
        g.dwarves[1].spouse = Some(a);
        // Everyone else far off, on another level.
        let lone = g.dwarves[0].pos;
        for d in g.dwarves.iter_mut().skip(1) {
            d.pos = lone.offset(0, 0, -5);
        }
        let mut d = std::mem::take(&mut g.dwarves[0]);
        g.kill(&mut d, "was struck down");
        g.dwarves[0] = d;
        assert!(!g.dwarves[1].thoughts.iter().any(|t| t.text.contains("lost their spouse")), "grieved without a body");
        assert!(g.dwarves[1].thoughts.iter().any(|t| t.text.starts_with("worried that")));
        assert!(!g.dwarves[2].thoughts.iter().any(|t| t.text.contains("death")), "a stranger far off knew");
        assert_eq!(g.society.grief.missing.len(), 1);
        // Someone walks onto the body.
        g.dwarves[2].pos = lone;
        g.grief_tick();
        assert!(g.society.grief.missing.is_empty());
        assert!(g.dwarves[2].thoughts.iter().any(|t| t.text.starts_with("found the body")));
        assert!(g.dwarves[1].thoughts.iter().any(|t| t.text.contains("lost their spouse")), "the widow wasn't told");
        assert!(g.society.grief.mourners.iter().any(|m| m.who == b && m.dead == a));
        assert_eq!(g.annals.counters.get("soc.found_body"), Some(&1));
    }

    #[test]
    fn the_widow_visits_where_he_fell() {
        let mut g = quiet(42);
        let (a, b) = (g.dwarves[0].id, g.dwarves[1].id);
        g.dwarves[0].spouse = Some(b);
        g.dwarves[1].spouse = Some(a);
        let at = g.dwarves[0].pos;
        g.dwarves[1].pos = at.offset(1, 0, 0);
        let mut d = std::mem::take(&mut g.dwarves[0]);
        g.kill(&mut d, "was struck down");
        g.dwarves[0] = d;
        assert!(g.dwarves[1].thoughts.iter().any(|t| t.text.contains("lost their spouse")), "grief seen is grief at once");
        // Off somewhere, then idle the next day: she goes back.
        g.tick += DAY;
        let mut w = std::mem::take(&mut g.dwarves[1]);
        w.pos = at.offset(4, 0, 0);
        assert!(g.grief_visit(&mut w));
        assert_eq!(w.task, Task::Wander { target: at });
        w.pos = at;
        g.dwarves[1] = w;
        g.grief_tick();
        assert!(g.dwarves[1].thoughts.iter().any(|t| t.text.contains("stood where")), "{:?}", g.dwarves[1].thoughts);
        assert!(g.gossip.iter().any(|(_, t)| t.contains("stood where")));
    }

    /// The "done when": a dwarf whose spouse was killed by a goblin swears
    /// vengeance, is drafted, and in the next siege picks the goblin out.
    #[test]
    fn a_widow_swears_vengeance_and_seeks_the_killer() {
        let mut g = quiet(43);
        g.set_overseer(Some(Style::Steady));
        let (a, b) = (g.dwarves[0].id, g.dwarves[1].id);
        g.dwarves[0].spouse = Some(b);
        g.dwarves[1].spouse = Some(a);
        g.dwarves[1].personality.temper = 50;
        let at = g.dwarves[0].pos;
        g.dwarves[1].pos = at.offset(2, 0, 0);
        let gob = g.spawn_creature(Species::Goblin, at.offset(1, 0, 0), u64::MAX);
        g.annals.wounded_by.push((a, "goblin".into(), g.tick));
        g.dwarves[0].attacker = Some((gob, g.tick));
        let mut d = std::mem::take(&mut g.dwarves[0]);
        g.kill(&mut d, "was struck down");
        g.dwarves[0] = d;
        assert_eq!(g.society.grief.vows.len(), 1, "no vow");
        assert_eq!(g.society.grief.vows[0].foe, "goblin");
        assert!(g.log.iter().any(|l| l.text.contains("swears vengeance on the goblins")));
        // The overseer drafts her.
        g.new_squad();
        let mut o = g.overseer.take().unwrap();
        g.ov_vows(&mut o);
        g.overseer = Some(o);
        assert!(g.squad_of(b).is_some() && g.dwarves[1].soldier, "not drafted");
        // Next siege: a wolf is nearer, but she goes for the goblin.
        let wid = g.dwarves[1].pos;
        let wolf = g.spawn_creature(Species::Wolf, wid.offset(0, 1, 0), u64::MAX);
        let threats = vec![(wolf, wid.offset(0, 1, 0)), (gob, at.offset(1, 0, 0))];
        assert_eq!(g.vow_target(&g.dwarves[1], &threats), Some(gob));
        // And she kills it.
        let c = g.creatures.get(&gob).unwrap().clone();
        let mut w = std::mem::take(&mut g.dwarves[1]);
        g.credit_kill(&mut w, &c);
        assert_eq!(w.epithet.as_deref(), Some("Avenger"));
        g.dwarves[1] = w;
        assert!(g.society.grief.vows.is_empty());
        assert_eq!(g.annals.counters.get("soc.vow_kept"), Some(&1));
        assert!(g.overworld.events.iter().any(|e| e.text.contains("avenged")), "no legend");
    }

    #[test]
    fn an_armed_vowed_civilian_charges_instead_of_fleeing() {
        let mut g = quiet(44);
        let id = g.dwarves[0].id;
        g.society.grief.vows.push(Vow { who: id, foe: "goblin".into(), for_whom: 999, for_name: "Ulmek Ironfist".into(), ..Default::default() });
        let p = g.dwarves[0].pos;
        let axe = g.spawn_item(ItemKind::Axe, Mat::Iron, p);
        g.items.get_mut(&axe).unwrap().carried_by = Some(id);
        let gob = g.spawn_creature(Species::Goblin, p.offset(2, 0, 0), u64::MAX);
        let mut d = std::mem::take(&mut g.dwarves[0]);
        d.soldier = false;
        d.held = None;
        let threats = vec![(gob, p.offset(2, 0, 0))];
        assert!(!g.vow_charge(&mut d, &threats), "charged with bare hands");
        d.held = Some(axe);
        assert!(g.vow_charge(&mut d, &threats));
        assert_eq!(d.task, Task::Attack { target: gob });
        g.dwarves[0] = d;
    }
}
