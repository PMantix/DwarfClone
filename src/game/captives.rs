//! Captives: besiegers taken alive in cage traps. Each keeps a name and the
//! civilization (and siege) it came with; caging them counts toward breaking
//! the siege, and the overseer decides what becomes of them.

use super::overseer::Overseer;
use super::personality::Trait;
use super::*;
use crate::creature::Species;
use serde::{Deserialize, Serialize};

/// Someone in one of the fortress's cages.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Captive {
    /// The cage item they're in.
    pub cage: u32,
    pub species: Species,
    pub name: String,
    /// The civilization that sent them (index into the world's civs).
    pub civ: Option<u32>,
    /// The siege they came with (index into `annals.sieges`).
    pub siege: Option<usize>,
    /// When the trap shut on them.
    pub caught: u64,
    pub fate: Fate,
    /// They led the siege.
    #[serde(default)]
    pub general: bool,
    /// The historical figure they are, if one.
    #[serde(default)]
    pub figure: Option<u32>,
    /// Questioned, and what they let slip hasn't come true yet.
    #[serde(default)]
    pub told: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fate {
    Held,
    /// Held, and already made to talk.
    Questioned,
    Ransomed,
    Freed,
    Executed,
    Escaped,
}

impl Fate {
    pub fn name(self) -> &'static str {
        match self {
            Fate::Held => "held",
            Fate::Questioned => "questioned",
            Fate::Ransomed => "ransomed",
            Fate::Freed => "set free",
            Fate::Executed => "put to death",
            Fate::Escaped => "escaped",
        }
    }
}

impl Captive {
    /// Still in the cage.
    pub fn caged(&self) -> bool {
        matches!(self.fate, Fate::Held | Fate::Questioned)
    }

    /// "Snaga Ushat the goblin".
    pub fn title(&self) -> String {
        format!("{} the {}", self.name, self.species.info().name)
    }
}

/// The people a species belongs to, for naming them.
fn race_of(s: Species) -> Option<Race> {
    match s {
        Species::Goblin => Some(Race::Goblin),
        Species::Human | Species::Merchant | Species::Visitor => Some(Race::Human),
        Species::Elf => Some(Race::Elf),
        Species::Raider => Some(Race::Dwarf),
        _ => None,
    }
}

impl Game {
    /// A cage trap has shut on `c` (a besieger or one of the peoples), now in
    /// the cage item `cage`: write down who they are, and see whether the
    /// siege still has the heart to go on.
    pub(super) fn take_captive(&mut self, c: &Creature, cage: u32) {
        let siege = if c.siege { self.annals.sieges.iter().rposition(|s| s.ended.is_none()) } else { None };
        let civ = siege.and_then(|i| self.annals.sieges[i].civ);
        let general = c.general.is_some();
        let name = match (&c.label, race_of(c.species)) {
            (Some(l), _) => l.clone(),
            (None, Some(r)) => crate::overworld::person_name(&mut self.rng, r),
            (None, None) => c.title(),
        };
        let figure = c.general.filter(|&f| f != u32::MAX).or(c.figure);
        let captive = Captive { cage, species: c.species, name, civ, siege, caught: self.tick, fate: Fate::Held, general, figure, told: false };
        let title = captive.title();
        self.captives.push(captive);
        self.count("captives", 1);
        if let Some(i) = siege {
            self.annals.sieges[i].foes_caught += 1;
        }
        if general {
            let text = format!("The general {} is in a cage!", title);
            self.msg(&text, (255, 215, 60));
            self.spotlight("A general caught", &text, None, (255, 215, 60));
            let fort = self.fortress_name.clone();
            self.legend(format!("{} was caught in a cage trap at {}.", title, fort));
        } else {
            self.msg(&format!("The caged one is called {}.", title), (160, 230, 160));
        }
        if c.siege && !c.legendary() {
            // Losing the general breaks them, as when the general falls.
            if general && !self.siege_routed {
                self.rout();
            }
            self.check_rout();
        }
    }

    /// Once a day the overseer decides what becomes of each captive who has
    /// been in a cage a couple of days, while no siege is on: by its traits it
    /// has them questioned, set free, put to death, or ransomed to a caravan
    /// of their people; otherwise they're kept.
    pub(super) fn ov_captives(&mut self, o: &mut Overseer) {
        if self.captives.is_empty() || self.annals.sieges.iter().any(|s| s.ended.is_none()) {
            return;
        }
        let wait = (self.knob("captives.wait_days", 2.0) * DAY as f64) as u64;
        let due: Vec<usize> = (0..self.captives.len()).filter(|&i| self.captives[i].caged() && self.tick >= self.captives[i].caught + wait).collect();
        for i in due {
            // A cage that's gone (sold, lost) takes its captive with it.
            if !self.items.get(&self.captives[i].cage).map_or(false, |it| it.kind == ItemKind::Cage && it.label.is_some()) {
                self.captives[i].fate = Fate::Escaped;
                continue;
            }
            let c = &self.captives[i];
            let civ_alive = c.civ.map_or(false, |v| self.overworld.civs.get(v as usize).map_or(false, |x| x.alive));
            let merciful = o.has(Trait::Sentimental) || o.has(Trait::Cheerful);
            let harsh = !o.has(Trait::Sentimental) && (o.has(Trait::Bold) || o.has(Trait::Vain) || o.style == Style::Reckless || (c.general && o.style == Style::Warlord));
            let curious = o.has(Trait::Cautious) || o.style == Style::Steady || !(merciful || harsh);
            let ransom = self.caravan.as_ref().map_or(false, |car| Some(car.civ) == c.civ || c.general) && self.depot().is_some();
            if c.fate == Fate::Held && civ_alive && curious {
                self.question(o, i);
            } else if merciful {
                self.set_free(o, i);
            } else if harsh {
                self.execute(o, i);
            } else if ransom {
                self.ransom(o, i);
            }
        }
    }

    /// Made to talk: they let slip that their people will come again, so the
    /// next siege is seen a season ahead.
    fn question(&mut self, o: &mut Overseer, i: usize) {
        let tick = self.tick;
        let c = &mut self.captives[i];
        c.fate = Fate::Questioned;
        c.told = true;
        let title = c.title();
        let civ = c.civ.map(|v| self.overworld.civ(v).name.clone()).unwrap_or_else(|| "enemy".into());
        self.siege_warned = true;
        self.count("questioned", 1);
        self.msg(&format!("{} has been questioned, and let slip that the {} will march on us again.", title, civ), (200, 200, 255));
        Self::note_ov(o, tick, format!("Had the captive {} questioned: the {} are coming back.", title, civ));
    }

    fn set_free(&mut self, o: &mut Overseer, i: usize) {
        let tick = self.tick;
        let (title, civ, _) = self.empty_cage(i, Fate::Freed);
        if let Some(v) = civ {
            *self.annals.mercies.entry(v).or_default() += 1;
        }
        self.count("freed", 1);
        let fort = self.fortress_name.clone();
        self.msg(&format!("The overseer has set {} free.", title), (160, 230, 160));
        self.legend(format!("{} was set free from a cage at {}.", title, fort));
        Self::note_ov(o, tick, format!("Set the captive {} free.", title));
    }

    /// Sold back to their people's caravan for gold.
    fn ransom(&mut self, o: &mut Overseer, i: usize) {
        let tick = self.tick;
        let general = self.captives[i].general;
        let (title, _, cage) = self.empty_cage(i, Fate::Ransomed);
        let at = self.depot().map(|d| d.pos).or_else(|| self.items.get(&cage).map(|it| it.pos)).unwrap_or(self.embark);
        let worth = (self.knob("captives.ransom", 300.0) * if general { 3.0 } else { 1.0 }) as u32;
        let mut paid = 0;
        for _ in 0..40 {
            if paid >= worth {
                break;
            }
            let bar = self.spawn_item(ItemKind::Bar, Mat::Gold, at);
            paid += self.items.get(&bar).map_or(worth, |it| it.value().max(1));
        }
        self.count("ransomed", 1);
        let fort = self.fortress_name.clone();
        let who = self.caravan.as_ref().map_or("the caravan".to_string(), |c| format!("the caravan from the {}", c.civ_name));
        self.msg(&format!("{} has been ransomed to {} for gold worth ☼{}.", title, who, paid), (240, 220, 120));
        self.legend(format!("{} was ransomed from a cage at {}.", title, fort));
        Self::note_ov(o, tick, format!("Ransomed the captive {} for ☼{} in gold.", title, paid));
    }

    /// Put to death. Mostly out of sight: those who knew think on it, and the
    /// chronicle has a line. Now and then a hard overseer has it done in the
    /// open, before the whole fort, and some are shaken while others are
    /// grimly satisfied. Their people will come in greater numbers to avenge it.
    fn execute(&mut self, o: &mut Overseer, i: usize) {
        let tick = self.tick;
        let figure = self.captives[i].figure;
        let (title, civ, cage) = self.empty_cage(i, Fate::Executed);
        if let Some(v) = civ {
            let g = self.knob("captives.grudge", 2.0) as u32;
            *self.annals.grudges.entry(v).or_default() += g;
        }
        self.count("executed", 1);
        let fort = self.fortress_name.clone();
        if let Some(f) = figure {
            let year = self.start_year + (tick / YEAR) as i32;
            self.overworld.kill_figure(f, year, format!("put to death at {}", fort));
        }
        self.legend(format!("{} was put to death at {}.", title, fort));
        Self::note_ov(o, tick, format!("Had the captive {} put to death.", title));
        let hard = !o.has(Trait::Sentimental) && (o.has(Trait::Bold) || matches!(o.style, Style::Reckless | Style::Warlord));
        let open = hard && self.rng.chance(self.knob("captives.open_execution", 0.25));
        let at = self.items.get(&cage).map(|it| it.pos);
        if open {
            let text = format!("On the overseer's order, {} has been put to death before the whole fortress.", title);
            self.msg(&text, (230, 120, 120));
            self.spotlight("An execution", &text, at, (230, 120, 120));
            self.count("executed.open", 1);
            for d in self.dwarves.iter_mut().filter(|d| d.alive) {
                if d.personality.anxiety > 10 {
                    d.think(&format!("watched {} put to death", title), 6, tick);
                } else {
                    d.think(&format!("watched {} get what was coming", title), -4, tick);
                }
            }
        } else {
            self.msg(&format!("{} is no longer in the cages.", title), (200, 170, 170));
            // Those who knew: the soldiers, and anyone near the cage.
            let near_cage = |p: Pos| at.map_or(false, |a| a.z == p.z && (a.x - p.x).abs() <= 8 && (a.y - p.y).abs() <= 8);
            for d in self.dwarves.iter_mut().filter(|d| d.alive && (d.soldier || near_cage(d.pos))) {
                let stress = if d.personality.anxiety > 25 { 3 } else { -3 };
                d.think(&format!("knew {} was put to death", title), stress, tick);
            }
        }
    }

    /// The captive's fate is settled and their cage is empty again:
    /// (title, civ, cage).
    fn empty_cage(&mut self, i: usize, fate: Fate) -> (String, Option<u32>, u32) {
        let c = &mut self.captives[i];
        c.fate = fate;
        c.told = false;
        let out = (c.title(), c.civ, c.cage);
        if let Some(it) = self.items.get_mut(&out.2) {
            it.label = None;
        }
        out
    }

    /// What the scouts add when `civ` marches: what a prisoner let slip, and
    /// the captive it comes to avenge or remembers being spared. The grudge
    /// is paid by this siege.
    pub(super) fn captive_words(&mut self, civ: u32, warned: bool) -> String {
        let mut s = String::new();
        if warned {
            if let Some(c) = self.captives.iter().rev().find(|c| c.told) {
                s.push_str(&format!(" We knew they would come, as the prisoner {} let slip.", c.name));
            }
        }
        for c in self.captives.iter_mut() {
            c.told = false;
        }
        let last = |g: &Game, f: Fate| g.captives.iter().rev().find(|c| c.civ == Some(civ) && c.fate == f).map(|c| c.name.clone());
        if self.annals.grudges.remove(&civ).is_some() {
            if let Some(name) = last(self, Fate::Executed) {
                s.push_str(&format!(" They come to avenge {}.", name));
            }
        }
        if self.annals.mercies.remove(&civ).is_some() {
            if let Some(name) = last(self, Fate::Freed) {
                s.push_str(&format!(" Fewer come: they remember {} was spared.", name));
            }
        }
        s
    }

    /// A besieger (in its think, every 15 ticks or so) goes to free one of
    /// its own from a cage it can reach, and works at the bars for about 250
    /// ticks; the captive comes out to fight again. Returns true while it's
    /// busy at that. It turns on a dwarf who comes close instead.
    pub(super) fn rescue_target(&mut self, c: &mut Creature, dwarves: &[(u32, Pos)]) -> bool {
        if c.legendary() || c.leaving || dwarves.iter().any(|(_, p)| super::combat::near(*p, c.pos, 2)) {
            return false;
        }
        let Some(civ) = self.annals.sieges.iter().rev().find(|s| s.ended.is_none()).and_then(|s| s.civ) else { return false };
        let cages: Vec<(usize, Pos)> = self
            .captives
            .iter()
            .enumerate()
            .filter(|(_, k)| k.caged() && k.civ == Some(civ))
            .filter_map(|(i, k)| self.items.get(&k.cage).filter(|it| it.label.is_some() && it.carried_by.is_none()).map(|it| (i, it.pos)))
            .collect();
        if cages.is_empty() {
            return false;
        }
        let by = |q: Pos, p: Pos| q == p || super::combat::adjacent(q, p);
        // At the bars: work them loose.
        if let Some(&(i, at)) = cages.iter().find(|(_, p)| by(c.pos, *p)) {
            c.path.clear();
            c.move_cd = 16;
            c.bash_work += 15;
            if c.bash_work >= 250 {
                c.bash_work = 0;
                self.free_captive(c, i, at);
            }
            return true;
        }
        // On the way already.
        if c.path.first().map_or(false, |&dest| cages.iter().any(|(_, p)| by(dest, *p))) {
            return true;
        }
        let mover = c.species.info().mover;
        if let Some((_, path)) = path::search(&self.world, c.pos, SEARCH_LIMIT, mover, |q| cages.iter().any(|(_, p)| by(q, *p))) {
            c.path = path.into_iter().rev().collect();
            c.target = None;
            c.bash = None;
            c.bash_work = 0;
            return true;
        }
        false
    }

    /// `c` has broken captive `i` out of the cage at `at`: they rejoin the siege.
    fn free_captive(&mut self, c: &Creature, i: usize, at: Pos) {
        let (title, _, _) = self.empty_cage(i, Fate::Escaped);
        let k = self.captives[i].clone();
        let id = self.spawn_creature(k.species, at, c.leave_tick.saturating_sub(self.tick));
        if let Some(n) = self.creatures.get_mut(&id) {
            n.siege = true;
            n.label = Some(k.name.clone());
        }
        self.count("escaped", 1);
        let text = format!("The {} has freed {} from the cage!", c.name(), title);
        self.msg(&text, (255, 120, 80));
        if k.general {
            self.spotlight("A general set loose", &text, Some(at), (255, 120, 80));
        }
    }

    /// A siege is over: those of it still in our cages go into the records.
    pub(super) fn captives_after_siege(&mut self, i: usize) {
        let names: Vec<String> = self.captives.iter().filter(|c| c.siege == Some(i) && c.caged()).map(|c| c.name.clone()).collect();
        if names.is_empty() || self.alive_count() == 0 {
            return;
        }
        let foe = self.annals.sieges[i].foe.clone();
        let fort = self.fortress_name.clone();
        let text = if names.len() == 1 {
            format!("{} of the {} was left behind in the cages of {}.", names[0], foe, fort)
        } else {
            format!("{} of the {} were left behind in the cages of {}.", names.len(), foe, fort)
        };
        self.msg(&text, (160, 230, 160));
        self.legend(text);
    }

    /// The captives still in their cages.
    pub fn caged(&self) -> impl Iterator<Item = &Captive> {
        self.captives.iter().filter(|c| c.caged())
    }

    /// For the Military panel: a blank line, "In cages:", then one line per
    /// captive ("Snaga Ushat the goblin (held)"); nothing when the cages are empty.
    pub fn cage_lines(&self) -> Vec<String> {
        let caged: Vec<&Captive> = self.caged().collect();
        if caged.is_empty() {
            return Vec::new();
        }
        let mut v = vec![String::new(), "In cages:".to_string()];
        for c in caged.iter().take(6) {
            v.push(format!(" {}{} ({})", c.title(), if c.general { ", general" } else { "" }, c.fate.name()));
        }
        if caged.len() > 6 {
            v.push(format!(" and {} more", caged.len() - 6));
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buildings::BuildingKind;

    /// A siege of `n` goblins at the map's edge, and a loaded cage trap.
    pub(super) fn siege_and_trap(seed: u64, n: u32) -> (Game, u32) {
        let mut g = Game::new(seed);
        g.creatures.retain(|_, c| c.tame);
        let mut foe = g.neighbors.iter().find(|n| g.at_war_with(n)).cloned().or_else(|| g.neighbors.first().cloned()).expect("a neighbour");
        foe.race = Race::Goblin;
        g.lay_siege_of(&foe, n, None);
        let at = g.open_patch(3, 3, 6).offset(1, 1, 0);
        assert!(g.place_building(BuildingKind::CageTrap, at));
        let tid = *g.buildings.keys().max().unwrap();
        let b = g.buildings.get_mut(&tid).unwrap();
        b.built = true;
        b.load = Some((ItemKind::Cage, Mat::Wood, 0));
        (g, tid)
    }

    /// Re-arm the trap by hand (in play a mechanic brings a new cage).
    pub(super) fn rearm(g: &mut Game, tid: u32) {
        g.buildings.get_mut(&tid).unwrap().built = true;
    }

    #[test]
    fn a_siege_half_caged_routs() {
        let (mut g, tid) = siege_and_trap(21, 4);
        let civ = g.annals.sieges[0].civ;
        assert!(civ.is_some(), "the siege knows who sent it");
        let rank: Vec<u32> = g.creatures.values().filter(|c| c.siege && c.general.is_none()).map(|c| c.id).collect();
        assert_eq!(rank.len(), 3);
        g.spring_trap(tid, rank[0]);
        assert!(!g.siege_routed, "one in a cage isn't enough");
        rearm(&mut g, tid);
        g.spring_trap(tid, rank[1]);
        assert!(g.siege_routed, "half of them caged: the rest should flee");
        assert_eq!(g.captives.len(), 2);
        for c in &g.captives {
            assert!(c.name.contains(' ') && c.name != "goblin", "a captive has a name: {}", c.name);
            assert_eq!((c.civ, c.siege, c.fate), (civ, Some(0), Fate::Held));
            assert!(g.items.get(&c.cage).map_or(false, |it| it.kind == ItemKind::Cage && it.label.is_some()));
        }
        assert_eq!(g.annals.sieges[0].foes_caught, 2);
        assert_eq!(g.annals.counters.get("captives"), Some(&2));
        assert!(g.log.iter().any(|l| l.text.contains("caught")));
        assert!(g.log.iter().any(|l| l.text.contains(&g.captives[0].name)), "the log names them");
        // And the Military panel lists them.
        let lines = g.cage_lines();
        assert_eq!(lines[1], "In cages:");
        assert!(lines[2].contains(&g.captives[0].name) && lines[2].ends_with("(held)"), "{:?}", lines);
        // The siege ends with those who ran: routed, not withdrawn.
        g.creatures.retain(|_, c| !c.siege);
        g.annals_tick();
        assert_eq!(g.annals.sieges[0].outcome, "routed");
        assert!(g.overworld.events.iter().any(|e| e.text.contains("2 of the goblins were left behind in the cages")));
    }

    #[test]
    fn caging_the_general_breaks_the_army() {
        let (mut g, tid) = siege_and_trap(21, 6);
        let gen = g.creatures.values().find(|c| c.general.is_some()).unwrap().id;
        let name = g.creatures[&gen].label.clone().unwrap();
        g.spring_trap(tid, gen);
        assert!(g.siege_routed);
        assert!(g.captives[0].general && g.captives[0].name == name);
        assert!(g.log.iter().any(|l| l.text.contains("is in a cage")));
    }

    /// One goblin caged, the rest of the siege gone, and three days gone by:
    /// the overseer may now decide. Returns the civ that sent them.
    fn one_caged(seed: u64) -> (Game, u32) {
        let (mut g, tid) = siege_and_trap(seed, 4);
        let gob = g.creatures.values().find(|c| c.siege && c.general.is_none()).unwrap().id;
        g.spring_trap(tid, gob);
        g.creatures.retain(|_, c| !c.siege);
        g.annals_tick();
        assert!(g.annals.sieges[0].ended.is_some());
        g.tick += 3 * DAY;
        let civ = g.captives[0].civ.unwrap();
        (g, civ)
    }

    fn overseer(style: Style, traits: [Trait; 2]) -> Overseer {
        let mut o = Overseer::new(style);
        o.name = "Urist".into();
        o.traits = traits.to_vec();
        o
    }

    #[test]
    fn a_bold_overseer_executes_and_their_people_come_for_vengeance() {
        let (mut g, civ) = one_caged(21);
        g.knobs.set.insert("captives.open_execution".into(), 0.0);
        let mut o = overseer(Style::Builder, [Trait::Bold, Trait::Greedy]);
        // Not before the second day in the cage.
        let caught = g.captives[0].caught;
        g.tick = caught + DAY;
        g.ov_captives(&mut o);
        assert_eq!(g.captives[0].fate, Fate::Held);
        g.tick = caught + 3 * DAY;
        // Without the grudge, for comparison (same dice).
        let rng = g.rng.clone();
        let plain = g.siege_strength(civ);
        g.rng = rng.clone();
        g.ov_captives(&mut o);
        let c = g.captives[0].clone();
        assert_eq!(c.fate, Fate::Executed);
        assert!(g.items[&c.cage].label.is_none(), "the cage is empty again");
        assert_eq!(g.annals.grudges.get(&civ), Some(&2));
        assert_eq!(g.annals.counters.get("executed"), Some(&1));
        assert!(g.overworld.events.iter().any(|e| e.text.contains(&c.name) && e.text.contains("put to death")), "a chronicle line");
        assert!(o.notes.iter().any(|(_, n)| n.contains("put to death")));
        // Off-screen: no card, but the soldiers or those nearby know.
        assert!(g.spotlights.iter().all(|s| s.title != "An execution"));
        g.rng = rng;
        assert_eq!(g.siege_strength(civ), plain + 2, "the next siege of that civ is two bigger");
        // The scouts say why, and the grudge is paid.
        let words = g.captive_words(civ, false);
        assert!(words.contains("avenge") && words.contains(&c.name), "{}", words);
        assert!(g.annals.grudges.get(&civ).is_none());
    }

    #[test]
    fn now_and_then_a_hard_overseer_does_it_in_the_open() {
        let (mut g, _) = one_caged(21);
        g.knobs.set.insert("captives.open_execution".into(), 1.0);
        for d in g.dwarves.iter_mut() {
            d.thoughts.clear();
        }
        g.dwarves[0].personality.anxiety = 40;
        g.dwarves[1].personality.anxiety = -40;
        let mut o = overseer(Style::Warlord, [Trait::Bold, Trait::Stubborn]);
        g.ov_captives(&mut o);
        assert_eq!(g.captives[0].fate, Fate::Executed);
        assert!(g.spotlights.iter().any(|s| s.title == "An execution"), "a card");
        let felt = |i: usize| g.dwarves[i].thoughts.iter().find(|t| t.text.contains(&g.captives[0].name)).map(|t| t.stress);
        assert!(felt(0).unwrap() > 0, "the anxious one is shaken");
        assert!(felt(1).unwrap() < 0, "the hard one is grimly satisfied");
        // A sentimental overseer never makes a show of it (it sets them free).
        let (mut g, _) = one_caged(21);
        g.knobs.set.insert("captives.open_execution".into(), 1.0);
        let mut o = overseer(Style::Warlord, [Trait::Bold, Trait::Sentimental]);
        g.ov_captives(&mut o);
        assert_eq!(g.captives[0].fate, Fate::Freed);
        assert!(g.spotlights.iter().all(|s| s.title != "An execution"));
    }

    #[test]
    fn a_cautious_overseer_questions_and_the_next_siege_is_foretold() {
        let (mut g, _) = one_caged(21);
        let mut o = overseer(Style::Steady, [Trait::Cautious, Trait::Greedy]);
        g.ov_captives(&mut o);
        let name = g.captives[0].name.clone();
        assert_eq!(g.captives[0].fate, Fate::Questioned);
        assert!(g.siege_warned, "what they let slip warns of the next siege");
        assert_eq!(g.annals.counters.get("questioned"), Some(&1));
        assert!(g.cage_lines().iter().any(|l| l.contains(&name) && l.contains("(questioned)")));
        // Once only: the next day they're kept, still in the cage.
        g.tick += DAY;
        g.siege_warned = false;
        g.ov_captives(&mut o);
        assert_eq!(g.captives[0].fate, Fate::Questioned);
        assert!(!g.siege_warned);
        // The scouts see the next one coming, and say how we knew.
        g.siege_warned = true;
        g.tick = g.tick.max(SEASON * 2 + 1);
        g.maybe_siege();
        assert!(g.incoming.is_some(), "a warned siege comes for certain");
        assert!(g.log.iter().any(|l| l.text.contains("Scouts report") && l.text.contains("let slip") && l.text.contains(&name)));
    }

    #[test]
    fn a_sentimental_overseer_sets_them_free_and_fewer_come() {
        let (mut g, civ) = one_caged(21);
        let mut o = overseer(Style::Builder, [Trait::Sentimental, Trait::Pious]);
        let rng = g.rng.clone();
        let plain = g.siege_strength(civ);
        g.rng = rng.clone();
        g.ov_captives(&mut o);
        assert_eq!(g.captives[0].fate, Fate::Freed);
        assert_eq!(g.annals.counters.get("freed"), Some(&1));
        g.rng = rng;
        assert_eq!(g.siege_strength(civ), (plain - 1).max(1));
        assert!(g.captive_words(civ, false).contains("spared"));
    }

    #[test]
    fn a_general_is_ransomed_to_a_caravan() {
        let (mut g, tid) = siege_and_trap(21, 4);
        let gen = g.creatures.values().find(|c| c.general.is_some()).unwrap().id;
        g.spring_trap(tid, gen);
        g.creatures.retain(|_, c| !c.siege);
        g.annals_tick();
        g.tick += 3 * DAY;
        let mut o = overseer(Style::Delver, [Trait::Greedy, Trait::Stubborn]);
        // No caravan: kept (it can't question them twice).
        g.captives[0].fate = Fate::Questioned;
        g.ov_captives(&mut o);
        assert_eq!(g.captives[0].fate, Fate::Questioned);
        // Any caravan will pay for a general.
        let at = g.open_patch(5, 5, 8);
        assert!(g.place_building(BuildingKind::Depot, at));
        let dep = *g.buildings.keys().max().unwrap();
        g.buildings.get_mut(&dep).unwrap().built = true;
        let home = g.civ.unwrap_or(0);
        g.caravan = Some(Caravan { civ: home, race: Race::Dwarf, civ_name: "Hosts".into(), goods: Vec::new(), merchants: Vec::new(), depot: dep, arrived: true, leave_tick: g.tick + DAY });
        let gold0 = g.items.values().filter(|it| it.kind == ItemKind::Bar && it.mat == Mat::Gold).count();
        g.ov_captives(&mut o);
        assert_eq!(g.captives[0].fate, Fate::Ransomed);
        let gold: u32 = g.items.values().filter(|it| it.kind == ItemKind::Bar && it.mat == Mat::Gold).map(|it| it.value()).sum();
        assert!(gold >= 900 && g.items.values().filter(|it| it.kind == ItemKind::Bar && it.mat == Mat::Gold).count() > gold0, "a general's ransom: ☼{}", gold);
    }

    #[test]
    fn besiegers_free_their_own() {
        let (mut g, tid) = siege_and_trap(21, 4);
        let gob = g.creatures.values().find(|c| c.siege && c.general.is_none()).unwrap().id;
        g.spring_trap(tid, gob);
        let cage = g.captives[0].cage;
        let at = g.items[&cage].pos;
        // One goblin left, a little way off, and nobody else about.
        let keep = g.creatures.values().find(|c| c.siege).unwrap().id;
        g.creatures.retain(|id, _| *id == keep);
        let start = (2..8).map(|r| at.offset(r, 0, 0)).find(|p| g.world.passable(*p, crate::world::Mover::Goblin)).unwrap_or(at);
        g.creatures.get_mut(&keep).unwrap().pos = start;
        let saved: Vec<Dwarf> = std::mem::take(&mut g.dwarves);
        for _ in 0..400 {
            g.update_creatures();
            g.tick += 1;
        }
        g.dwarves = saved;
        let k = &g.captives[0];
        assert_eq!(k.fate, Fate::Escaped, "the goblin should have broken them out");
        assert!(g.items[&cage].label.is_none());
        assert_eq!(g.creatures.values().filter(|c| c.siege && c.alive).count(), 2, "two besiegers now");
        assert!(g.creatures.values().any(|c| c.label.as_deref() == Some(k.name.as_str())));
        assert!(g.log.iter().any(|l| l.text.contains("from the cage")));
        assert!(g.cage_lines().is_empty());
    }

    #[test]
    fn animals_in_cages_are_not_captives() {
        let (mut g, tid) = siege_and_trap(21, 4);
        let at = g.buildings[&tid].pos;
        let wolf = g.spawn_creature(Species::Wolf, at, u64::MAX);
        g.spring_trap(tid, wolf);
        assert!(g.captives.is_empty());
        assert!(!g.siege_routed);
    }
}
