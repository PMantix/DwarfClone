//! Heroes: great deeds earn a dwarf an epithet ("Kelda the Dragonslayer"),
//! a line in the legends and the fortress's admiration; a hero's death
//! shakes everyone.

use super::*;
use crate::creature::Creature;

/// How grand each kind of deed is: a dwarf's epithet only ever gets grander.
pub(super) const RANK_FEAT: u8 = 1;
const RANK_KILLS: u8 = 2;
pub(super) const RANK_BEAST: u8 = 3;

impl Game {
    /// `d` struck the blow that killed `c`.
    pub(super) fn credit_kill(&mut self, d: &mut Dwarf, c: &Creature) {
        d.kills += 1;
        self.story_kill(d, c);
        let what = c.title();
        if c.species == crate::creature::Species::Undead && c.general.is_some() {
            self.earn_epithet(d, "Deathbane", RANK_BEAST + 1, &format!("destroyed {}", what));
        } else if c.breathes_fire {
            self.earn_epithet(d, "Dragonslayer", RANK_BEAST + 2, &format!("slew {}", what));
        } else if let Some(f) = c.figure {
            let kind = self.overworld.figures.get(f as usize).map_or("beast".to_string(), |x| x.kind.clone());
            self.earn_epithet(d, &format!("{}bane", cap_word(&kind)), RANK_BEAST + 1, &format!("slew {}", what));
        } else if c.species == crate::creature::Species::CaveHorror {
            self.earn_epithet(d, "Horrorbane", RANK_BEAST, &format!("slew {}", what));
        } else if c.general.is_some() {
            self.earn_epithet(d, "Warbreaker", RANK_BEAST, &format!("cut down {}, who led the attack", what));
        } else if d.kills == 10 {
            self.earn_epithet(d, "Grim", RANK_KILLS, "slew ten foes");
        } else if d.kills == 25 {
            self.earn_epithet(d, "Red-Handed", RANK_KILLS + 1, "slew twenty-five foes");
        }
    }

    /// Give `d` a name to be remembered by, if it's grander than the one
    /// they have.
    pub(super) fn earn_epithet(&mut self, d: &mut Dwarf, title: &str, rank: u8, deed: &str) {
        if rank <= d.epithet_rank {
            return;
        }
        let before = d.called();
        d.epithet = Some(title.to_string());
        d.epithet_rank = rank;
        d.deed = Some(deed.to_string());
        let now = d.called();
        self.saga_epithet(d.id, &before, &now, deed, rank);
        self.year_log.hero = Some(now.clone());
        self.milestone("hero", &format!("The fortress's first hero: {}", now));
        let tick = self.tick;
        d.think(&format!("was hailed as {}", now), -40, tick);
        self.msg(&format!("{} {} and will be known as {}!", before, deed, now), (255, 215, 60));
        let fort = self.fortress_name.clone();
        self.legend(format!("{} of {} {}, and was known after as {}.", before, fort, deed, now));
        let id = d.id;
        for o in self.dwarves.iter_mut().filter(|o| o.alive && o.id != id) {
            o.think(&format!("admired the deeds of {}", now), -5, tick);
        }
    }

    /// Smaller feats at festivals.
    pub(super) fn festival_feat(&mut self, d: &mut Dwarf, wrestled: bool) {
        if wrestled {
            d.bouts_won += 1;
            if d.bouts_won == 3 {
                self.earn_epithet(d, "Ironarm", RANK_FEAT, "won three arm-wrestling bouts");
            }
        } else {
            d.contests_won += 1;
            if d.contests_won == 2 {
                self.earn_epithet(d, "Deepcup", RANK_FEAT, "won two drinking contests");
            }
        }
    }

    /// A hero has died: the fortress mourns, and the legends remember.
    pub(super) fn hero_fell(&mut self, d: &Dwarf, cause: &str) {
        if d.epithet.is_none() {
            return;
        }
        let name = d.called();
        let tick = self.tick;
        for o in self.dwarves.iter_mut().filter(|o| o.alive && o.id != d.id) {
            o.think(&format!("mourned the hero {}", name), 25, tick);
        }
        let fort = self.fortress_name.clone();
        self.legend(format!("{}, who {}, {} at {}.", name, d.deed.clone().unwrap_or_default(), cause, fort));
        self.msg(&format!("The hero {} is dead. The fortress mourns.", name), (255, 120, 90));
        self.spotlight("A hero falls", &format!("{}, who {}, {}. The fortress mourns.", name, d.deed.clone().unwrap_or_default(), cause), Some(d.pos), (255, 120, 90));
    }
}

pub(super) fn cap_word(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::Species;

    #[test]
    fn slaying_a_dragon_makes_a_hero_and_heroes_are_mourned() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let mut d = std::mem::take(&mut g.dwarves[0]);
        let plain = d.called();
        assert_eq!(plain, d.name);
        let id = g.spawn_creature(Species::Megabeast, d.pos, DAY);
        let mut c = g.creatures.remove(&id).unwrap();
        c.label = Some("Emberwing the dragon".into());
        c.breathes_fire = true;
        g.credit_kill(&mut d, &c);
        assert_eq!(d.epithet.as_deref(), Some("Dragonslayer"));
        let first = d.name.split(' ').next().unwrap().to_string();
        assert_eq!(d.called(), format!("{} the Dragonslayer", first));
        assert!(g.log.iter().any(|l| l.text.contains("will be known as")));
        assert!(g.overworld.events.iter().any(|e| e.text.contains("the Dragonslayer")));
        assert!(g.dwarves.iter().skip(1).filter(|o| o.alive).all(|o| o.thoughts.iter().any(|t| t.text.starts_with("admired the deeds"))));
        // A lesser deed doesn't replace a greater one.
        for _ in 0..3 {
            g.festival_feat(&mut d, true);
        }
        assert_eq!(d.epithet.as_deref(), Some("Dragonslayer"));
        g.dwarves[0] = d;
        // When the hero dies, everyone grieves.
        let mut hero = std::mem::take(&mut g.dwarves[0]);
        g.kill(&mut hero, "was struck down");
        g.dwarves[0] = hero;
        assert!(g.dwarves.iter().skip(1).filter(|o| o.alive).all(|o| o.thoughts.iter().any(|t| t.text.starts_with("mourned the hero"))));
        assert!(g.overworld.events.iter().any(|e| e.text.contains("was struck down at")));
    }

    #[test]
    fn ten_kills_make_a_dwarf_grim() {
        let mut g = Game::new(81);
        let mut d = std::mem::take(&mut g.dwarves[0]);
        let id = g.spawn_creature(Species::Wolf, d.pos, DAY);
        let c = g.creatures.remove(&id).unwrap();
        for _ in 0..9 {
            g.credit_kill(&mut d, &c);
        }
        assert!(d.epithet.is_none());
        g.credit_kill(&mut d, &c);
        assert_eq!(d.epithet.as_deref(), Some("Grim"));
        g.dwarves[0] = d;
    }
}
