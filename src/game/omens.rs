//! Taboos and omens (MYT-05, MYT-13): each god forbids something (felling
//! living trees, spilling blood in the temple, eating fish). Breaking it
//! scandalises the devout, and a god angered enough curses the culprit
//! with an ill-omened season until they atone at the god's own temple.
//! Omens (birds fleeing, an eclipse, a blood moon, weeping stone, a
//! shooting star) foretell what is coming, and it comes.

use super::overseer::Overseer;
use super::*;
use crate::worldgen::myth::Taboo;
use serde::{Deserialize, Serialize};

/// A sign in the sky or the stone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OmenKind {
    /// Birds flee the land: an army is coming.
    FleeingBirds,
    /// The sun goes dark at noon: a great beast is coming.
    Eclipse,
    /// A blood moon: the dead will walk.
    BloodMoon,
    /// The stone of the deep halls weeps: something will come up from below.
    WeepingStone,
    /// A shooting star: someone will be touched by a strange mood.
    ShootingStar,
    /// A god's anger: a curse has fallen.
    Wrath,
}

impl OmenKind {
    /// "a blood moon rose over Boatmurdered".
    pub fn sight(self, fort: &str) -> String {
        match self {
            OmenKind::FleeingBirds => format!("the birds fled the land round {} in great flocks, all one way", fort),
            OmenKind::Eclipse => "the sun went dark at noon".into(),
            OmenKind::BloodMoon => format!("a blood moon rose over {}", fort),
            OmenKind::WeepingStone => "the stone of the deep halls wept".into(),
            OmenKind::ShootingStar => "a star fell burning across the sky".into(),
            OmenKind::Wrath => "a cold wind blew through the temple".into(),
        }
    }

    /// What it foretells, plainly.
    pub fn meaning(self) -> &'static str {
        match self {
            OmenKind::FleeingBirds => "an army is on its way",
            OmenKind::Eclipse => "a great beast is coming",
            OmenKind::BloodMoon => "the dead will walk",
            OmenKind::WeepingStone => "something will come up from the deep",
            OmenKind::ShootingStar => "one of us will be touched by the gods",
            OmenKind::Wrath => "a god is angry",
        }
    }

    /// A bad sign (the overseer readies for it).
    pub fn ill(self) -> bool {
        matches!(self, OmenKind::FleeingBirds | OmenKind::Eclipse | OmenKind::BloodMoon | OmenKind::WeepingStone)
    }

    fn short(self) -> &'static str {
        match self {
            OmenKind::FleeingBirds => "the fleeing birds",
            OmenKind::Eclipse => "the eclipse",
            OmenKind::BloodMoon => "the blood moon",
            OmenKind::WeepingStone => "the weeping stone",
            OmenKind::ShootingStar => "the falling star",
            OmenKind::Wrath => "the god's anger",
        }
    }
}

/// An omen seen, and when what it foretells is due.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Omen {
    pub kind: OmenKind,
    pub seen: u64,
    pub due: u64,
    /// The overseer has taken note of it.
    #[serde(default)]
    pub noted: bool,
}

/// A god's curse on a dwarf: an ill-starred season, until they atone.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Curse {
    pub dwarf: u32,
    pub god: u32,
    pub until: u64,
    pub why: Taboo,
}

fn deed(t: Taboo) -> &'static str {
    match t {
        Taboo::FellingTrees => "felled a living tree",
        Taboo::BloodInTemple => "spilled blood in the temple",
        Taboo::EatingFish => "ate a fish",
    }
}

impl Game {
    /// Someone has broken a taboo (`who` did it, if anyone). The gods of the
    /// fortress who forbid it grow angry; their devout are scandalised.
    /// `who` is passed in rather than looked up: the culprit is usually out
    /// of `self.dwarves` for their own turn (`update_dwarf`).
    pub(super) fn taboo_broken(&mut self, taboo: Taboo, who: Option<&Dwarf>) {
        let who: Option<(u32, String, Pos)> = who.filter(|d| d.alive).map(|d| (d.id, d.name.clone(), d.pos));
        let culprit_id = who.as_ref().map(|w| w.0);
        let gods: Vec<u32> = self
            .fort_gods()
            .into_iter()
            .filter(|&g| self.god(g).map_or(false, |god| god.taboo == taboo))
            .filter(|&g| self.magic.patrons.iter().any(|(d, p)| p.0 == g && (Some(*d) == culprit_id || self.dwarves.iter().any(|x| x.id == *d && x.alive))))
            .collect();
        if gods.is_empty() {
            return;
        }
        self.count("magic.taboo", 1);
        let tick = self.tick;
        let weight = match taboo {
            Taboo::FellingTrees => 1,
            Taboo::EatingFish => 2,
            Taboo::BloodInTemple => 4,
        };
        let culprit = who.as_ref().map_or("Someone".to_string(), |w| w.1.split(' ').next().unwrap_or("Someone").to_string());
        for g in gods {
            let god = self.god(g).unwrap().clone();
            let scandal = format!("was scandalised that {} {}, which {} forbids", culprit, deed(taboo), god.name);
            for d in self.dwarves.iter_mut().filter(|d| d.alive && Some(d.id) != culprit_id) {
                let Some(&(pg, dev)) = self.magic.patrons.get(&d.id) else { continue };
                if pg != g || dev < 2 || d.thoughts.iter().any(|t| t.text.starts_with("was scandalised") && tick < t.tick + DAY) {
                    continue;
                }
                d.think(&scandal, 5 * dev as i32, tick);
            }
            self.warn(&format!("taboo{}", g), &format!("{} {}, which {} forbids. The devout are scandalised.", culprit, deed(taboo), god.called()), (230, 190, 120));
            let wrath = self.magic.wrath.entry(g).or_insert(0);
            *wrath += weight;
            if *wrath >= self.knob("magic.wrath", 12.0) as u32 {
                self.magic.wrath.insert(g, 0);
                self.divine_curse(g, taboo, who.clone());
            }
        }
    }

    /// A woodcutter has felled a living tree. A day of felling is one
    /// offence (noticed once every two days), not one per tree, or a
    /// woodcutting fort would anger a god of the forest without end.
    pub(super) fn tree_felled(&mut self, d: &Dwarf, _at: Pos) {
        if self.magic.felled_seen > 0 && self.tick < self.magic.felled_seen + DAY * 2 {
            return;
        }
        self.magic.felled_seen = self.tick.max(1);
        self.taboo_broken(Taboo::FellingTrees, Some(d));
    }

    /// For the overseer: how few logs in store before it sends the
    /// woodcutters out. A chief god who forbids felling makes it frugal.
    pub(super) fn ov_wood_floor(&self) -> usize {
        match self.chief_god().and_then(|g| self.god(g)) {
            Some(god) if god.taboo == Taboo::FellingTrees => 2,
            _ => 6,
        }
    }

    /// Blood on a temple floor (from `bleed_on`): noticed once a half day.
    pub(super) fn blood_on_holy_ground(&mut self, p: Pos) {
        if self.zone_kind_at(p) != Some(ZoneKind::Temple) || (self.magic.blood_seen > 0 && self.tick < self.magic.blood_seen + DAY / 2) {
            return;
        }
        self.magic.blood_seen = self.tick.max(1);
        self.taboo_broken(Taboo::BloodInTemple, None);
    }

    /// A god's patience runs out: the culprit (else whoever last did it) is
    /// cursed for a season, and everyone feels it.
    fn divine_curse(&mut self, g: u32, taboo: Taboo, who: Option<(u32, String, Pos)>) {
        let god = self.god(g).unwrap().clone();
        let tick = self.tick;
        let fort = self.fortress_name.clone();
        let sight = OmenKind::Wrath.sight(&fort);
        match who {
            Some((id, name, at)) => {
                self.magic.curses.retain(|c| c.dwarf != id);
                self.magic.curses.push(Curse { dwarf: id, god: g, until: tick + SEASON, why: taboo });
                let text = format!("{} is angered! {} {}, and {} has cursed {} for it. Only prayer at {}'s temple will lift it.", god.called(), cap(&sight), god.name, god.name, name, god.name);
                self.msg(&text, (255, 140, 90));
                self.spotlight("The wrath of a god", &text, Some(at), (255, 140, 90));
                self.magic.year.push(format!("{} cursed {} for {}.", god.called(), name, taboo.name()));
                self.legend(format!("{} cursed {} of {} for {}.", god.called(), name, fort, taboo.name()));
                self.count("magic.curses", 1);
                self.saga_curse(id, &name, &god.called(), taboo.name());
            }
            None => {
                self.msg(&format!("{} is angered by {} in {}: {}.", god.called(), taboo.name(), fort, sight), (255, 140, 90));
                self.magic.year.push(format!("{} was angered by {}.", god.called(), taboo.name()));
            }
        }
    }

    /// Daily: curses bite, and pass when their season is out; anger cools.
    pub(super) fn curses_daily(&mut self) {
        let tick = self.tick;
        if (tick / DAY) % 7 == 0 {
            for w in self.magic.wrath.values_mut() {
                *w = w.saturating_sub(2);
            }
        }
        let curses = self.magic.curses.clone();
        for c in curses {
            let Some(i) = self.dwarves.iter().position(|d| d.id == c.dwarf && d.alive) else {
                self.magic.curses.retain(|x| x.dwarf != c.dwarf);
                continue;
            };
            let god = self.god(c.god).map_or("the gods".to_string(), |g| g.name.clone());
            if tick >= c.until {
                self.magic.curses.retain(|x| x.dwarf != c.dwarf);
                let name = self.dwarves[i].name.clone();
                self.msg(&format!("The curse of {} on {} has run its course.", god, name), (190, 220, 255));
                continue;
            }
            // Ill luck: troubled sleep and a heavy heart.
            let tired = self.knob("magic.curse_fatigue", 600.0) as u32;
            let d = &mut self.dwarves[i];
            d.think(&format!("feels the anger of {}", god), 6, tick);
            d.fatigue += tired;
        }
    }

    /// At prayer before the god who cursed them: the curse is lifted.
    pub(super) fn atone(&mut self, d: &mut Dwarf) {
        let Some(c) = self.magic.curses.iter().find(|c| c.dwarf == d.id).cloned() else { return };
        let here = self.world.get(d.pos).map(|t| t.zone).and_then(|z| self.magic.shrines.get(&z)).copied();
        if here != Some(c.god) {
            return;
        }
        self.magic.curses.retain(|x| x.dwarf != d.id);
        let god = self.god(c.god).map_or("the gods".to_string(), |g| g.called());
        d.think(&format!("atoned before {}", god), -30, self.tick);
        let name = d.name.clone();
        self.msg(&format!("{} has atoned before {}, and the curse is lifted.", name, god), (190, 220, 255));
        self.saga_atoned(d.id, &name, &god);
        self.magic.year.push(format!("{} atoned before {}.", name, god));
    }

    /// Each season: perhaps an omen, of something that can come.
    pub(super) fn omens_season(&mut self) {
        if self.magic.omen.is_some() || self.tick < SEASON || self.alive_count() == 0 {
            return;
        }
        let mut rng = self.mrng();
        if !rng.chance(self.knob("magic.omen", 0.3)) {
            return;
        }
        let mut can = Vec::new();
        if self.neighbors.iter().any(|n| self.at_war_with(n)) && self.incoming.is_none() && self.tick >= SEASON * 2 {
            can.push(OmenKind::FleeingBirds);
        }
        let (sx, sy) = self.site;
        if self.tick >= YEAR && !self.overworld.beasts_near(sx, sy, 30).is_empty() && !self.creatures.values().any(|c| c.figure.is_some()) {
            can.push(OmenKind::Eclipse);
        }
        if self.necromancer.as_ref().map_or(false, |n| n.alive) && self.tick >= YEAR {
            can.push(OmenKind::BloodMoon);
        }
        if self.tick >= SEASON * 2 && self.cavern_floors().iter().any(|&p| self.world.t(p).revealed) && !self.creatures.values().any(|c| c.species == crate::creature::Species::CaveHorror) {
            can.push(OmenKind::WeepingStone);
        }
        if self.alive_count() >= 10 && !self.dwarves.iter().any(|d| d.mood.is_some()) {
            can.push(OmenKind::ShootingStar);
        }
        if can.is_empty() {
            return;
        }
        let kind = can[rng.range(0, can.len() as i32) as usize];
        let due = self.tick + DAY * rng.range(3, 9) as u64;
        self.see_omen(kind, due);
    }

    /// An omen is seen: everyone notices, the anxious fret, the devout take
    /// it as a sign. An army foretold comes for certain.
    pub(super) fn see_omen(&mut self, kind: OmenKind, due: u64) {
        let tick = self.tick;
        let fort = self.fortress_name.clone();
        self.magic.omen = Some(Omen { kind, seen: tick, due, noted: false });
        let sight = kind.sight(&fort);
        self.msg(&format!("An omen: {}. The dwarves mutter that {}.", cap(&sight), kind.meaning()), (255, 200, 140));
        self.saga_omen(kind);
        let season = SEASONS[((tick / SEASON) % 4) as usize].to_lowercase();
        self.magic.year.push(format!("In the {}, {}.", season, sight));
        self.count("magic.omens", 1);
        if kind == OmenKind::FleeingBirds {
            self.siege_warned = true;
        }
        self.wizard_reads(kind, due);
        let chief = self.chief_god().and_then(|g| self.god(g)).map(|g| g.name.clone());
        for d in self.dwarves.iter_mut().filter(|d| d.alive && !d.is_child()) {
            let dev = self.magic.patrons.get(&d.id).map_or(0, |p| p.1);
            if kind.ill() && d.personality.anxiety >= 15 {
                d.think(&format!("was unsettled by {}", kind.short()), 10, tick);
            } else if dev >= 3 {
                let who = chief.clone().unwrap_or_else(|| "the gods".into());
                d.think(&format!("took {} as a sign from {}", kind.short(), who), if kind.ill() { 4 } else { -10 }, tick);
            }
        }
    }

    /// Daily: an omen whose day has come comes true (or, if it cannot,
    /// comes to nothing).
    pub(super) fn omens_daily(&mut self) {
        let Some(o) = self.magic.omen.clone() else { return };
        if self.tick < o.due {
            return;
        }
        // The army foretold is still to march (the scouts' warning stands):
        // the omen waits on it rather than come to nothing just before.
        if o.kind == OmenKind::FleeingBirds && self.siege_warned && self.incoming.is_none() && self.tick < o.seen + YEAR {
            if let Some(om) = self.magic.omen.as_mut() {
                om.due = self.tick + DAY * 7;
            }
            return;
        }
        self.magic.omen = None;
        let mut rng = self.mrng();
        let came = match o.kind {
            // The army comes through the scouts as ever (`siege_warned`).
            OmenKind::FleeingBirds => self.incoming.is_some() || self.creatures.values().any(|c| c.siege),
            OmenKind::Eclipse => {
                let (sx, sy) = self.site;
                match self.overworld.beasts_near(sx, sy, 30).first().copied() {
                    Some(f) if !self.creatures.values().any(|c| c.figure.is_some()) => self.spawn_megabeast(f).is_some(),
                    _ => false,
                }
            }
            OmenKind::BloodMoon => {
                let before = self.creatures.len();
                self.raise_dead();
                self.creatures.len() > before
            }
            OmenKind::WeepingStone => {
                let floors: Vec<Pos> = self.cavern_floors();
                let found = floors.iter().any(|&p| self.world.t(p).revealed);
                if found && !floors.is_empty() {
                    let p = floors[rng.range(0, floors.len() as i32) as usize];
                    self.spawn_cave_horror(p);
                    true
                } else {
                    false
                }
            }
            OmenKind::ShootingStar => {
                let cands: Vec<usize> = (0..self.dwarves.len())
                    .filter(|&i| {
                        let d = &self.dwarves[i];
                        d.alive && !d.insane && !d.soldier && d.noble.is_none() && !d.is_child() && d.mood.is_none()
                    })
                    .collect();
                if cands.is_empty() || self.dwarves.iter().any(|d| d.mood.is_some()) {
                    false
                } else {
                    self.start_mood(cands[rng.range(0, cands.len() as i32) as usize]);
                    true
                }
            }
            OmenKind::Wrath => true,
        };
        self.saga_omen_kept(o.kind, came);
        if came {
            self.msg(&format!("The omen has come true: {}.", o.kind.meaning()), (255, 200, 140));
            if let Some(l) = self.magic.year.iter_mut().rev().find(|l| l.contains(&o.kind.sight(&self.fortress_name))) {
                l.pop();
                l.push_str(&format!(", and it came true: {}.", o.kind.meaning()));
            }
            self.count("magic.omens_true", 1);
        } else {
            self.msg(&format!("The omen of {} came to nothing, this time.", o.kind.short()), (190, 190, 200));
        }
    }

    /// The overseer reads an ill omen as a warning (a brain sense), and
    /// says what it makes of it.
    pub(super) fn omen_sense(&self) -> f64 {
        match &self.magic.omen {
            Some(o) if o.kind.ill() => 1.0,
            _ => 0.0,
        }
    }

    /// Daily for the overseer: it takes note of an omen, and a god who
    /// forbids eating fish has the fish in store kept from the table, if
    /// the larder can spare them.
    pub(super) fn ov_omens(&mut self, o: &mut Overseer) {
        let tick = self.tick;
        if let Some(om) = self.magic.omen.as_mut().filter(|om| !om.noted) {
            om.noted = true;
            let kind = om.kind;
            let what = match kind {
                OmenKind::FleeingBirds => "the squad to its drill and the gate in mind",
                OmenKind::Eclipse => "the squad close to home",
                OmenKind::BloodMoon => "an eye on the unburied dead",
                OmenKind::WeepingStone => "a watch on the caverns",
                OmenKind::ShootingStar => "stone, wood and gems in store for whoever is touched",
                OmenKind::Wrath => "the temple in mind",
            };
            let pious = o.has(super::personality::Trait::Pious);
            let lead = if pious { "A sign!" } else { "The dwarves are talking:" };
            Self::note_ov(o, tick, format!("{} {}; {}. Keeping {}.", lead, cap(&kind.sight(&self.fortress_name)), kind.meaning(), what));
        }
        let Some(chief) = self.chief_god() else { return };
        let Some(god) = self.god(chief).cloned() else { return };
        if god.taboo == Taboo::FellingTrees && !o.did("treetaboo") {
            o.done.push("treetaboo".into());
            Self::note_ov(o, tick, format!("{} forbids felling living trees: we cut only the wood we cannot do without.", god.called()));
        }
        if god.taboo != Taboo::EatingFish || self.food_per_head() < 4.0 {
            return;
        }
        let fish: Vec<u32> = self.items.values().filter(|it| it.kind == ItemKind::Fish && !it.forbidden && it.free()).map(|it| it.id).collect();
        if fish.is_empty() {
            return;
        }
        for id in &fish {
            self.items.get_mut(id).unwrap().forbidden = true;
        }
        if !o.did("fishtaboo") {
            o.done.push("fishtaboo".into());
            Self::note_ov(o, tick, format!("{} forbids eating fish, and most of us follow {}: the fish stay off the table.", god.called(), if god.female { "her" } else { "him" }));
        }
    }

    /// For the look panel: a dwarf's curse.
    pub fn curse_line(&self, dwarf: u32) -> Option<String> {
        let c = self.magic.curses.iter().find(|c| c.dwarf == dwarf)?;
        let god = self.god(c.god)?;
        Some(format!("Cursed by {} for {}.", god.called(), c.why.name()))
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn faithful() -> Game {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        g.faith_daily();
        g
    }

    /// Everyone devoutly worships the god who forbids `t` (one is made to).
    fn all_follow(g: &mut Game, t: Taboo) -> u32 {
        let gid = g.fort_gods()[0];
        g.overworld.gods[gid as usize].taboo = t;
        for p in g.magic.patrons.values_mut() {
            *p = (gid, 3);
        }
        gid
    }

    #[test]
    fn breaking_a_taboo_scandalises_the_devout_and_angers_the_god() {
        let mut g = faithful();
        let gid = all_follow(&mut g, Taboo::EatingFish);
        let who = g.dwarves[0].clone();
        g.taboo_broken(Taboo::EatingFish, Some(&who));
        assert!(g.dwarves[1].thoughts.iter().any(|t| t.text.starts_with("was scandalised that") && t.text.contains("ate a fish")));
        assert!(!g.dwarves[0].thoughts.iter().any(|t| t.text.starts_with("was scandalised")), "the culprit isn't scandalised at themself");
        assert_eq!(g.magic.wrath[&gid], 2);
        // A different taboo means nothing to this god.
        g.taboo_broken(Taboo::FellingTrees, Some(&who));
        assert_eq!(g.magic.wrath[&gid], 2);
        // Again and again: the god curses the culprit.
        for _ in 0..5 {
            g.taboo_broken(Taboo::EatingFish, Some(&who));
        }
        assert_eq!(g.magic.curses.len(), 1);
        assert_eq!(g.magic.curses[0].dwarf, who.id);
        assert!(g.curse_line(who.id).unwrap().contains("eating fish"));
        assert!(g.spotlights.iter().any(|s| s.title == "The wrath of a god"));
        let f0 = g.dwarves[0].fatigue;
        g.curses_daily();
        assert!(g.dwarves[0].fatigue > f0 && g.dwarves[0].thoughts.iter().any(|t| t.text.starts_with("feels the anger of")));
        // Prayer at the god's own temple lifts it.
        let e = g.embark;
        g.add_zone(e.offset(-3, -3, 0), e.offset(3, 3, 0), ZoneKind::Temple);
        g.faith_daily();
        assert_eq!(g.magic.shrines.values().next(), Some(&gid));
        let mut d = std::mem::take(&mut g.dwarves[0]);
        d.pos = g.zones.values().find(|z| z.kind == ZoneKind::Temple).unwrap().tiles[0];
        g.atone(&mut d);
        g.dwarves[0] = d;
        assert!(g.magic.curses.is_empty());
        assert!(g.log.iter().any(|l| l.text.contains("has atoned before")));
    }

    #[test]
    fn a_woodcutter_at_work_is_cursed_by_a_god_of_the_forest() {
        // Through the dwarf's own turn (taken out of `dwarves` the while).
        let mut g = faithful();
        let gid = all_follow(&mut g, Taboo::FellingTrees);
        g.magic.wrath.insert(gid, g.knob("magic.wrath", 12.0) as u32 - 1);
        let ids: Vec<u32> = g.dwarves.iter().map(|d| d.id).collect();
        for id in ids {
            g.set_labor(id, crate::dwarf::Labor::Woodcutting, true);
        }
        let e = g.embark;
        assert!(g.designate(e.offset(-30, -30, -2), e.offset(30, 30, 2), Desig::Chop) > 0, "no trees to fell");
        let mut t = 0;
        while g.magic.curses.is_empty() && t < DAY * 3 {
            g.step();
            t += 1;
        }
        assert_eq!(g.magic.curses.len(), 1, "no curse in {} ticks", t);
        let c = g.magic.curses[0].clone();
        assert_eq!((c.god, c.why), (gid, Taboo::FellingTrees));
        assert!(g.dwarves.iter().any(|d| d.id == c.dwarf && d.alive));
        assert!(g.curse_line(c.dwarf).is_some());
        assert!(g.spotlights.iter().any(|s| s.title == "The wrath of a god"));
        // The overseer's woodcutters go out only when wood is short.
        assert_eq!(g.ov_wood_floor(), 2);
    }

    #[test]
    fn felling_trees_and_blood_in_the_temple_are_noticed() {
        let mut g = faithful();
        let gid = all_follow(&mut g, Taboo::FellingTrees);
        // A woodcutter fells a tree.
        let tree = (0..g.world.h).flat_map(|y| (0..g.world.w).map(move |x| (x, y))).find_map(|(x, y)| {
            let z = g.world.surface_z(x, y);
            let p = Pos::new(x, y, z);
            (g.world.kind(p) == Some(crate::world::TileKind::Tree)).then_some(p)
        });
        let tree = tree.expect("no tree on the map");
        let mut d = std::mem::take(&mut g.dwarves[0]);
        g.tree_felled(&d, tree);
        d.pos = tree;
        g.dwarves[0] = d;
        assert_eq!(g.magic.wrath[&gid], 1);
        // Blood on a temple floor is worse.
        all_follow(&mut g, Taboo::BloodInTemple);
        let e = g.embark;
        g.add_zone(e.offset(-3, -3, 0), e.offset(3, 3, 0), ZoneKind::Temple);
        let p = g.zones.values().find(|z| z.kind == ZoneKind::Temple).unwrap().tiles[0];
        g.bleed_on(p, None, 3);
        assert_eq!(g.magic.wrath[&gid], 5);
        // A trail of it is one offence, not many.
        g.bleed_on(p, None, 3);
        assert_eq!(g.magic.wrath[&gid], 5);
    }

    #[test]
    fn an_omen_is_noticed_and_comes_true() {
        let mut g = faithful();
        while g.alive_count() < 10 {
            let id = g.new_id();
            let d = crate::dwarf::new_dwarf(id, g.embark, &mut g.rng, &[]);
            g.dwarves.push(d);
        }
        for d in g.dwarves.iter_mut() {
            d.personality.anxiety = 30;
        }
        g.tick = SEASON * 3;
        g.see_omen(OmenKind::ShootingStar, g.tick + DAY);
        assert!(g.log.iter().any(|l| l.text.starts_with("An omen: A star fell burning")));
        assert_eq!(g.magic.year.len(), 1);
        g.omens_daily();
        assert!(g.magic.omen.is_some(), "not due yet");
        g.tick += DAY;
        g.omens_daily();
        assert!(g.magic.omen.is_none());
        assert!(g.dwarves.iter().any(|d| d.mood.is_some()), "the star foretold a mood");
        assert!(g.magic.year[0].contains("came true"), "{}", g.magic.year[0]);
        // An ill omen unsettles the anxious, and the overseer reads it.
        g.set_overseer(Some(Style::Steady));
        g.see_omen(OmenKind::FleeingBirds, g.tick + DAY);
        assert!(g.siege_warned, "an army is coming for certain");
        assert!(g.dwarves.iter().any(|d| d.thoughts.iter().any(|t| t.text == "was unsettled by the fleeing birds")));
        assert_eq!(g.omen_sense(), 1.0);
        let mut o = g.overseer.take().unwrap();
        g.ov_omens(&mut o);
        assert!(o.notes.iter().any(|n| n.1.contains("an army is on its way")), "{:?}", o.notes);
        let b = g.brain(&o);
        assert_eq!(b.senses.iter().find(|s| s.0 == Sense::Omens).unwrap().1, 1.0);
        g.overseer = Some(o);
        // The year's omens go into the chronicle.
        g.write_chronicle();
        let c = g.chronicles.last().unwrap();
        assert!(c.lines.iter().any(|l| l.contains("the birds fled the land")), "{:?}", c.lines);
        assert!(g.magic.year.is_empty());
    }

    #[test]
    fn the_fleeing_birds_wait_on_the_army_they_foretell() {
        let mut g = faithful();
        g.tick = SEASON * 3;
        g.see_omen(OmenKind::FleeingBirds, g.tick + DAY);
        g.tick += DAY;
        g.omens_daily();
        let o = g.magic.omen.clone().expect("the army has yet to march: the omen stands");
        assert!(o.due > g.tick);
        assert!(!g.log.iter().any(|l| l.text.contains("came to nothing")));
        // Once the warning is spent and no army came, it comes to nothing.
        g.siege_warned = false;
        g.tick = o.due;
        g.omens_daily();
        assert!(g.magic.omen.is_none());
    }

    #[test]
    fn seasons_bring_omens_now_and_then() {
        let mut g = faithful();
        while g.alive_count() < 10 {
            let id = g.new_id();
            let d = crate::dwarf::new_dwarf(id, g.embark, &mut g.rng, &[]);
            g.dwarves.push(d);
        }
        g.tick = YEAR;
        let mut seen = 0;
        for i in 0..40 {
            g.tick = YEAR + i * SEASON;
            g.omens_season();
            if g.magic.omen.take().is_some() {
                seen += 1;
            }
        }
        assert!((3..=25).contains(&seen), "{} omens in 40 seasons", seen);
    }

    #[test]
    fn a_god_who_forbids_fish_keeps_it_off_the_overseers_table() {
        let mut g = faithful();
        all_follow(&mut g, Taboo::EatingFish);
        g.set_overseer(Some(Style::Steady));
        let e = g.embark;
        for _ in 0..40 {
            g.spawn_item(ItemKind::Meal, crate::material::Mat::Stoutcap, e);
        }
        let fish = g.spawn_item(ItemKind::Fish, crate::material::Mat::Stoutcap, e);
        let mut o = g.overseer.take().unwrap();
        g.ov_omens(&mut o);
        assert!(g.items[&fish].forbidden);
        assert!(o.notes.iter().any(|n| n.1.contains("forbids eating fish")));
        g.overseer = Some(o);
    }
}
