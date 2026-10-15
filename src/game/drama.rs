//! Fortress drama: pranks, love triangles, insults between rivals, and
//! duels at dawn. The gossip page keeps track of it all.

use super::*;
use serde::{Deserialize, Serialize};

const PRANKS: [&str; 8] = [
    "hid {}'s boots in a barrel",
    "filled {}'s mug with mud",
    "glued {}'s beard to the table while they slept",
    "swapped {}'s pick for a spoon",
    "told {} the ale had run out",
    "put a cave spider in {}'s bed",
    "painted a goblin face on {}'s door",
    "replaced {}'s seat with a stoutcap",
];

const INSULTS: [&str; 10] = [
    "a beardless pebble-licker",
    "an elf in dwarf's boots",
    "slower than a dead troglodyte",
    "a disgrace to their clan",
    "softer than tallow",
    "a goblin's mother",
    "unfit to haul refuse",
    "the worst brewer under the mountain",
    "a coward who hides behind the doors",
    "a stoutcap with legs",
];

/// A duel waiting for dawn.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Duel {
    pub a: u32,
    pub b: u32,
    pub at: u64,
}

impl Game {
    pub(super) fn gossip(&mut self, text: String) {
        self.gossip.push((self.tick, text));
        if self.gossip.len() > 80 {
            self.gossip.remove(0);
        }
    }

    /// Adults who are here, awake to the world, and themselves.
    fn players(&self) -> Vec<usize> {
        (0..self.dwarves.len())
            .filter(|&i| {
                let d = &self.dwarves[i];
                d.alive && !d.insane && !d.is_child() && d.jailed_until == 0
            })
            .collect()
    }

    /// Once a day: pranks, jealousy, insults and challenges.
    pub(super) fn drama_daily(&mut self) {
        let tick = self.tick;
        let who = self.players();
        if who.len() < 2 {
            return;
        }
        // Pranks, by the cheerful.
        for &i in &who {
            let d = &self.dwarves[i];
            if d.personality.sociability <= 20 || d.stress > 100 || !self.rng.chance(0.06) {
                continue;
            }
            let j = who[self.rng.range(0, who.len() as i32) as usize];
            if j == i {
                continue;
            }
            let (pname, tname, temper) = (self.dwarves[i].called(), self.dwarves[j].called(), self.dwarves[j].personality.temper);
            let deed = self.rng.pick(&PRANKS).replace("{}", &tname);
            self.dwarves[i].think("played a prank", -4, tick);
            if temper > 20 {
                self.dwarves[j].think("was the butt of a prank", 6, tick);
                let pid = self.dwarves[i].id;
                self.dwarves[j].adjust_regard(pid, -10);
                self.gossip(format!("{} {}. {} was not amused.", pname, deed, tname));
            } else {
                self.dwarves[j].think("laughed at a prank played on them", -2, tick);
                let pid = self.dwarves[i].id;
                self.dwarves[j].adjust_regard(pid, 3);
                self.gossip(format!("{} {}. Everyone laughed, {} too.", pname, deed, tname));
            }
        }
        // Two admirers of one unwed dwarf grow jealous of each other.
        for &z in &who {
            if self.dwarves[z].spouse.is_some() || !self.rng.chance(0.15) {
                continue;
            }
            let zid = self.dwarves[z].id;
            let fans: Vec<usize> = who.iter().copied().filter(|&i| i != z && self.dwarves[i].spouse.is_none() && self.dwarves[i].regard(zid) >= 60).collect();
            if fans.len() < 2 {
                continue;
            }
            let (a, b) = (fans[0], fans[1]);
            let (aid, bid) = (self.dwarves[a].id, self.dwarves[b].id);
            if self.dwarves[a].regard(bid) <= -40 {
                continue;
            }
            self.dwarves[a].adjust_regard(bid, -30);
            self.dwarves[b].adjust_regard(aid, -30);
            self.dwarves[a].think("grew jealous of a rival in love", 10, tick);
            self.dwarves[b].think("grew jealous of a rival in love", 10, tick);
            let (an, bn, zn) = (self.dwarves[a].called(), self.dwarves[b].called(), self.dwarves[z].called());
            self.gossip(format!("{} and {} both have eyes for {}. Sharp words were exchanged.", an, bn, zn));
        }
        // Rivals trade insults, and the hot-headed throw down a challenge.
        let pending: Vec<u32> = self.duels.iter().flat_map(|d| [d.a, d.b]).collect();
        for &i in &who {
            let iid = self.dwarves[i].id;
            let Some(&j) = who.iter().find(|&&j| j != i && self.dwarves[i].regard(self.dwarves[j].id) <= -40 && self.dwarves[j].regard(iid) <= -40) else { continue };
            if !self.rng.chance(0.2) {
                continue;
            }
            let jid = self.dwarves[j].id;
            let (an, bn) = (self.dwarves[i].called(), self.dwarves[j].called());
            let insult = *self.rng.pick(&INSULTS);
            self.dwarves[i].adjust_regard(jid, -5);
            self.dwarves[j].adjust_regard(iid, -10);
            self.dwarves[j].think(&format!("was insulted by {}", an), 8, tick);
            self.gossip(format!("{} called {} {}.", an, bn, insult));
            let hot = self.dwarves[i].personality.temper > 0 || self.dwarves[j].personality.temper > 0;
            let fresh = !pending.contains(&iid) && !pending.contains(&jid) && self.duels.iter().all(|d| d.a != iid && d.b != iid);
            if hot && fresh && self.dwarves[i].regard(jid) <= -60 && (self.dwarves[i].last_duel == 0 || tick >= self.dwarves[i].last_duel + SEASON) {
                let at = (tick / DAY + 1) * DAY + DAY / 10;
                self.duels.push(Duel { a: iid, b: jid, at });
                self.msg(&format!("{} has challenged {} to a duel at dawn!", an, bn), (255, 170, 90));
                self.gossip(format!("{} has challenged {} to a duel at dawn.", an, bn));
            }
        }
    }

    /// Duels whose dawn has come are fought.
    pub(super) fn duels_tick(&mut self) {
        let tick = self.tick;
        let due: Vec<Duel> = self.duels.iter().filter(|d| tick >= d.at).cloned().collect();
        self.duels.retain(|d| tick < d.at);
        for duel in due {
            let (Some(a), Some(b)) = (
                self.dwarves.iter().position(|d| d.id == duel.a && d.alive && !d.insane),
                self.dwarves.iter().position(|d| d.id == duel.b && d.alive && !d.insane),
            ) else {
                continue;
            };
            self.fight_duel(a, b);
        }
    }

    fn fight_duel(&mut self, a: usize, b: usize) {
        let tick = self.tick;
        let score = |g: &mut Game, i: usize| crate::dwarf::skill_level(g.dwarves[i].fight_xp) as i32 * 4 + g.dwarves[i].personality.temper / 10 + g.rng.range(0, 12);
        let (sa, sb) = (score(self, a), score(self, b));
        let (w, l) = if sa >= sb { (a, b) } else { (b, a) };
        let (wn, ln) = (self.dwarves[w].called(), self.dwarves[l].called());
        let (wid, lid) = (self.dwarves[w].id, self.dwarves[l].id);
        for _ in 0..2 {
            let part = self.dwarves[l].body.pick_part(&mut self.rng);
            self.dwarves[l].body.wound(part, 2, false);
        }
        if self.rng.chance(0.5) {
            let part = self.dwarves[w].body.pick_part(&mut self.rng);
            self.dwarves[w].body.wound(part, 2, false);
        }
        self.dwarves[w].fight_xp += 60;
        self.dwarves[l].fight_xp += 30;
        self.dwarves[w].last_duel = tick;
        self.dwarves[l].last_duel = tick;
        self.dwarves[w].think(&format!("won a duel against {}", ln), -20, tick);
        self.dwarves[l].think(&format!("lost a duel to {}", wn), 20, tick);
        let mut text = format!("At dawn, {} and {} fought their duel. {} won; {} yielded, bloodied.", wn, ln, wn, ln);
        // Sometimes blood settles it.
        if self.rng.chance(0.5) {
            self.dwarves[w].relations.retain(|r| r.0 != lid);
            self.dwarves[l].relations.retain(|r| r.0 != wid);
            text.push_str(" They shook hands, and the grudge is over.");
        }
        self.msg(&text, (255, 170, 90));
        self.gossip(text);
        let mut d = std::mem::take(&mut self.dwarves[w]);
        self.duels_won(&mut d);
        self.dwarves[w] = d;
    }

    /// Three duels won earn a dwarf a name.
    fn duels_won(&mut self, d: &mut Dwarf) {
        d.duels_won += 1;
        if d.duels_won == 3 {
            self.earn_epithet(d, "Unbeaten", super::heroes::RANK_FEAT + 1, "won three duels");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitter_rivals_insult_each_other_and_settle_it_at_dawn() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let (a, b) = (g.dwarves[0].id, g.dwarves[1].id);
        g.dwarves[0].adjust_regard(b, -90);
        g.dwarves[1].adjust_regard(a, -90);
        g.dwarves[0].personality.temper = 40;
        for _ in 0..40 {
            g.drama_daily();
            if !g.duels.is_empty() {
                break;
            }
        }
        assert!(g.gossip.iter().any(|(_, t)| t.contains(" called ")), "no insults");
        assert_eq!(g.duels.len(), 1, "no challenge");
        assert!(g.log.iter().any(|l| l.text.contains("duel at dawn")));
        g.tick = g.duels[0].at;
        g.duels_tick();
        assert!(g.duels.is_empty());
        assert!(g.log.iter().any(|l| l.text.starts_with("At dawn,")));
        assert!(g.dwarves.iter().take(2).any(|d| d.thoughts.iter().any(|t| t.text.starts_with("won a duel"))));
    }

    #[test]
    fn pranks_and_jealousy_make_gossip() {
        let mut g = Game::new(81);
        for d in g.dwarves.iter_mut() {
            d.personality.sociability = 40;
            d.stress = 0;
        }
        let z = g.dwarves[2].id;
        g.dwarves[0].adjust_regard(z, 80);
        g.dwarves[1].adjust_regard(z, 80);
        for d in g.dwarves.iter_mut() {
            d.spouse = None;
        }
        for _ in 0..60 {
            g.drama_daily();
        }
        assert!(g.gossip.iter().any(|(_, t)| t.contains("laughed") || t.contains("not amused")), "no pranks");
        assert!(g.gossip.iter().any(|(_, t)| t.contains("both have eyes for")), "no jealousy");
    }
}
