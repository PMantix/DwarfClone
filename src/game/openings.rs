//! Openings (MIL-04, second half): a foe who lies senseless is finished
//! rather than left to crawl off, a besieger who breaks takes a parting blow
//! as it turns, and besiegers leave the fallen for the standing, which is
//! what gives a downed dwarf the time to be carried off.
//!
//! The blow itself is one `if` in `combat.rs: strike`; the rest lives here.

use super::combat::{adjacent, near, Fighter};
use super::*;
use crate::body::{Body, Part};
use crate::creature::Creature;

/// How close a standing dwarf must be for a besieger to leave a fallen one for them.
const LEAVE_FOR: i32 = 6;

impl Game {
    /// Where a blow at one who lies senseless lands: the head or the chest.
    pub(super) fn opening_part(&mut self) -> Part {
        if self.rng.chance(self.knob("opening.head", 0.5)) {
            Part::Head
        } else {
            Part::UpperBody
        }
    }

    /// The log line for a blow on one who lies senseless, and the tally.
    pub(super) fn finishing_words(&mut self, att: &Fighter, def: &Fighter, part: Part, body: &Body, suffix: &str) -> String {
        self.count("openings", 1);
        let who = cap_first(&att.name);
        if body.dead {
            self.count("finished", 1);
            format!("{} {} {} where they lie, in the {}, and finishes them.", who, att.verb, def.name, part.name())
        } else {
            format!("{} {} {} where they lie, in the {}{}", who, att.verb, def.name, part.name(), suffix)
        }
    }

    /// A hostile creature lying senseless beside a dwarf: a soldier finishes
    /// it before turning to the next.
    pub(super) fn downed_foe_beside(&self, d: &Dwarf) -> Option<u32> {
        self.creatures
            .values()
            .filter(|c| c.alive && c.body.unconscious() && (c.hostile() || c.siege) && adjacent(c.pos, d.pos))
            .map(|c| c.id)
            .min()
    }

    fn dwarf_downed(&self, id: u32) -> bool {
        self.dwarves.iter().any(|d| d.id == id && d.alive && d.body.unconscious())
    }

    /// A dwarf on their feet within `r` that the creature can get to.
    pub(super) fn standing_dwarf_near(&self, c: &Creature, r: i32) -> bool {
        self.dwarves
            .iter()
            .filter(|d| d.alive && !d.body.unconscious() && near(d.pos, c.pos, r))
            .any(|d| adjacent(d.pos, c.pos) || self.can_reach(c, d.pos, (r * r * 6) as usize))
    }

    /// A besieger whose quarry lies senseless leaves them for a dwarf still
    /// standing close by (the fallen are finished only when nobody is).
    /// Returns true when it let go of its target.
    pub(super) fn leave_the_fallen(&mut self, c: &mut Creature) -> bool {
        if !c.siege || !c.target.map_or(false, |t| self.dwarf_downed(t)) || !self.standing_dwarf_near(c, LEAVE_FOR) {
            return false;
        }
        c.target = None;
        c.path.clear();
        self.count("left_fallen", 1);
        true
    }

    /// Whom a besieger goes for: the nearest dwarf by path, but one still on
    /// their feet over one lying senseless while any stands within reach.
    pub(super) fn siege_quarry(&self, c: &Creature, dwarves: &[(u32, Pos)]) -> Option<(u32, Vec<Pos>)> {
        let mover = c.species.info().mover;
        let downed: HashSet<u32> = self.dwarves.iter().filter(|d| d.alive && d.body.unconscious()).map(|d| d.id).collect();
        let standing: HashMap<Pos, u32> = dwarves.iter().filter(|(id, _)| !downed.contains(id)).map(|&(id, p)| (p, id)).collect();
        // (One standing over a fallen dwarf is the one found on that tile.)
        let mut all: HashMap<Pos, u32> = dwarves.iter().filter(|(id, _)| downed.contains(id)).map(|&(id, p)| (p, id)).collect();
        all.extend(standing.iter().map(|(&p, &id)| (p, id)));
        let search = |at: &HashMap<Pos, u32>| {
            path::search(&self.world, c.pos, SEARCH_LIMIT, mover, |q| at.contains_key(&q)).map(|(p, path)| (at[&p], path))
        };
        if !dwarves.iter().any(|(id, _)| downed.contains(id)) {
            return search(&all);
        }
        if standing.keys().any(|&p| near(p, c.pos, LEAVE_FOR)) {
            if let Some(found) = search(&standing) {
                return Some(found);
            }
        }
        search(&all)
    }

    /// The army breaks: each besieger with a soldier beside it takes a
    /// parting blow as it turns.
    pub(super) fn parting_swings_on_rout(&mut self) {
        let ids: Vec<u32> = self.creatures.values().filter(|c| c.siege && c.leaving && c.alive).map(|c| c.id).collect();
        for id in ids {
            let Some(mut c) = self.creatures.remove(&id) else { continue };
            if self.parting_swing(&mut c) {
                self.creatures.insert(id, c);
            }
        }
    }

    /// A besieger that breaks turns its back: a soldier beside it gets one
    /// free blow as it goes. Returns false if that blow killed it (it has
    /// then been handed to `creature_died`).
    pub(super) fn parting_swing(&mut self, c: &mut Creature) -> bool {
        let Some(di) = self.dwarves.iter().position(|d| d.alive && d.soldier && !d.body.unconscious() && adjacent(d.pos, c.pos)) else {
            return true;
        };
        let mut d = std::mem::take(&mut self.dwarves[di]);
        let att = self.dwarf_fighter(&d);
        let def = self.creature_fighter(c);
        let hp0: i32 = c.body.hp.iter().sum();
        self.msg(&format!("{} catches {} as it turns to run.", d.called(), c.title()), (235, 225, 130));
        let msg = self.strike(&att, &def, &mut c.body);
        let hurt = c.body.hp.iter().sum::<i32>() < hp0;
        self.note_blow(super::annals::BY_SOLDIER, hurt);
        self.count("parting_swings", 1);
        d.fight_xp += 8;
        if let Some(m) = msg {
            self.msg(&m, (235, 225, 130));
        }
        let dead = c.body.dead;
        if dead {
            self.credit_kill(&mut d, c);
        }
        self.dwarves[di] = d;
        if dead {
            self.creature_died(c.clone());
            return false;
        }
        true
    }
}

fn cap_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}
