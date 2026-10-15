//! Visitors to the tavern: bards who sing the fortress's own legends, and
//! wandering warriors who may ask to stay.

use super::*;
use crate::creature::{Creature, Species};
use crate::dwarf::NeedKind;

pub const BARD: u8 = 1;
pub const WARRIOR: u8 = 2;
/// A warrior who has made up their mind (to stay or go).
const DECIDED: u8 = 3;
/// A warrior who has joined: the visitor is gone, the dwarf has arrived.
pub const JOINED: u8 = 4;

const SONGS: [&str; 6] = ["The Ballad of", "The Lay of", "A Song for", "The Saga of", "A Drinking Song of", "The Lament of"];

impl Game {
    fn tavern_tiles(&self) -> Vec<Pos> {
        self.zones
            .values()
            .filter(|z| z.kind == ZoneKind::Tavern)
            .flat_map(|z| z.tiles.iter().copied())
            .filter(|&p| self.world.walkable(p))
            .collect()
    }

    /// Each season but winter: with a tavern, a bard or a wandering warrior may come.
    pub(super) fn maybe_visitor(&mut self, season: u64) {
        if season == 3 || self.tavern_tiles().is_empty() || !self.rng.chance(0.5) {
            return;
        }
        let Some(entry) = self.edge_entry() else { return };
        let kind = if self.rng.chance(0.6) { BARD } else { WARRIOR };
        self.spawn_visitor(entry, kind);
    }

    pub(super) fn spawn_visitor(&mut self, at: Pos, kind: u8) -> u32 {
        // Bards come from any people at peace with us; warriors are dwarves.
        let mut races = vec![Race::Dwarf];
        if kind == BARD {
            races.extend(self.neighbors.iter().filter(|n| !self.at_war_with(n)).map(|n| n.race));
        }
        let race = *self.rng.pick(&races);
        let name = crate::overworld::person_name(&mut self.rng, race);
        let stay = if kind == BARD { DAY * 4 } else { DAY * 3 };
        let id = self.spawn_creature(Species::Visitor, at, self.tick + stay);
        let tick = self.tick;
        let c = self.creatures.get_mut(&id).unwrap();
        c.visitor = kind;
        c.born = tick;
        let singular = match race {
            Race::Dwarf => "dwarf",
            Race::Human => "human",
            Race::Elf => "elf",
            Race::Goblin => "goblin",
        };
        c.label = Some(name.clone());
        c.desc = Some(if kind == BARD { format!("a {} bard", singular) } else { "a wandering dwarf warrior".to_string() });
        let text = if kind == BARD {
            format!("{}, a {} bard, has come to sing in your tavern.", name, singular)
        } else {
            format!("{}, a wandering warrior, has come to rest in your tavern.", name)
        };
        self.msg(&text, (170, 220, 255));
        id
    }

    /// A visitor heads for the tavern and stays there: a bard sings, a
    /// warrior makes up their mind.
    pub(super) fn visitor_think(&mut self, c: &mut Creature) {
        let tavern = self.tavern_tiles();
        if tavern.is_empty() {
            c.leaving = true;
            return;
        }
        let inside = self.zone_kind_at(c.pos) == Some(ZoneKind::Tavern);
        if !inside || self.rng.chance(0.08) {
            if c.path.is_empty() {
                let q = tavern[self.rng.range(0, tavern.len() as i32) as usize];
                if let Some(p) = path::astar(&self.world, c.pos, q, SEARCH_LIMIT, Mover::Dwarf) {
                    c.path = p.into_iter().rev().collect();
                } else {
                    c.leaving = true;
                }
            }
            return;
        }
        match c.visitor {
            BARD if (self.tick + c.id as u64) % (DAY / 3) < 15 => self.sing(c),
            WARRIOR if self.tick >= c.born + DAY * 2 => {
                c.visitor = DECIDED;
                if self.rng.chance(0.5) {
                    self.warrior_joins(c);
                } else {
                    self.msg(&format!("{} has moved on.", c.title()), (170, 220, 255));
                    c.leaving = true;
                }
            }
            _ => {}
        }
    }

    /// A song from the legends, the fortress's own most of all.
    fn sing(&mut self, c: &Creature) {
        let fort = self.fortress_name.clone();
        let ours: Vec<String> = self.overworld.events.iter().filter(|e| e.text.contains(&fort)).map(|e| e.text.clone()).collect();
        let event = if !ours.is_empty() && self.rng.chance(0.7) {
            self.rng.pick(&ours).clone()
        } else if !self.overworld.events.is_empty() {
            let i = self.rng.range(0, self.overworld.events.len() as i32) as usize;
            self.overworld.events[i].text.clone()
        } else {
            return;
        };
        let words: Vec<&str> = event.split(' ').collect();
        let subject: String = if words[0] == "The" {
            if event.contains(&fort) {
                fort.clone()
            } else {
                format!("the {}", words.get(1).copied().unwrap_or(""))
            }
        } else {
            words.iter().take(2).copied().collect::<Vec<_>>().join(" ")
        };
        let subject = subject.trim_end_matches(|ch: char| !ch.is_alphanumeric()).to_string();
        let title = format!("{} {}", self.rng.pick(&SONGS), subject);
        let mut how = event.trim_end_matches('.').to_string();
        if how.starts_with("The ") {
            how.replace_range(..1, "t");
        }
        self.msg(&format!("{} the bard sings \"{}\" in the tavern: how {}.", c.title(), title, how), (200, 180, 255));
        let tick = self.tick;
        for i in 0..self.dwarves.len() {
            let d = &self.dwarves[i];
            if !d.alive || d.asleep || self.zone_kind_at(d.pos) != Some(ZoneKind::Tavern) {
                continue;
            }
            let own = event.contains(&d.name) || event.contains(&d.called());
            let d = &mut self.dwarves[i];
            if own {
                d.think("heard a song of their own deeds", -15, tick);
            } else {
                d.think("enjoyed a bard's song", -6, tick);
            }
            d.satisfy(NeedKind::Party, 300);
        }
    }

    /// The warrior asks to stay, and is welcomed: a seasoned fighter joins.
    fn warrior_joins(&mut self, c: &mut Creature) {
        let id = self.new_id();
        let mut d = dwarf::new_dwarf(id, c.pos, &mut self.rng, &[]);
        if let Some(n) = &c.label {
            d.name = n.clone();
        }
        d.fight_xp = 1600 + self.rng.range(0, 800) as u32;
        d.epithet = Some("Wanderer".into());
        d.epithet_rank = super::heroes::RANK_FEAT;
        d.deed = Some("came from afar and stayed to defend the fortress".into());
        self.dress(&mut d, 400);
        let name = d.name.clone();
        let fort = self.fortress_name.clone();
        self.dwarves.push(d);
        c.visitor = JOINED;
        self.year_log.arrivals += 1;
        self.msg(&format!("{}, the wandering warrior, has asked to stay in {} and been welcomed!", name, fort), (120, 255, 120));
        self.legend(format!("{}, a wandering warrior, settled in {}.", name, fort));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bard_sings_the_forts_legends_in_the_tavern() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let e = g.embark;
        let n = g.add_zone(e.offset(-3, -3, 0), e.offset(3, 3, 0), ZoneKind::Tavern);
        assert!(n > 0);
        assert!(g.overworld.events.iter().any(|ev| ev.text.contains(&g.fortress_name)), "the founding should be legend");
        let bard = g.spawn_visitor(e, BARD);
        // Everyone into the tavern.
        let tiles = g.tavern_tiles();
        let mut sang = false;
        for _ in 0..DAY {
            for (i, d) in g.dwarves.iter_mut().enumerate() {
                d.pos = tiles[i % tiles.len()];
                d.path.clear();
                d.asleep = false;
            }
            g.step();
            if g.log.iter().any(|l| l.text.contains(" sings \"")) {
                sang = true;
                break;
            }
        }
        assert!(sang, "the bard never sang");
        println!("{}", g.log.iter().find(|l| l.text.contains(" sings \"")).unwrap().text);
        assert!(g.creatures.contains_key(&bard));
        assert!(g.dwarves.iter().any(|d| d.thoughts.iter().any(|t| t.text == "enjoyed a bard's song" || t.text == "heard a song of their own deeds")));
    }

    #[test]
    fn a_wandering_warrior_may_join() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let e = g.embark;
        g.add_zone(e.offset(-3, -3, 0), e.offset(3, 3, 0), ZoneKind::Tavern);
        let before = g.dwarves.len();
        let mut joined = false;
        for _ in 0..20 {
            let id = g.spawn_visitor(e, WARRIOR);
            let mut c = g.creatures.remove(&id).unwrap();
            c.pos = g.tavern_tiles()[0];
            c.born = 0;
            g.tick = DAY * 3;
            g.visitor_think(&mut c);
            if c.visitor == JOINED {
                joined = true;
                break;
            }
            assert!(c.leaving, "a warrior who won't stay moves on");
        }
        assert!(joined, "no warrior ever stayed");
        assert_eq!(g.dwarves.len(), before + 1);
        let d = g.dwarves.last().unwrap();
        assert_eq!(d.epithet.as_deref(), Some("Wanderer"));
        assert!(d.fight_xp >= 1600);
    }
}
