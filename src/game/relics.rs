//! Artifact powers (MYT-22): every artifact made in a strange mood (and
//! every relic of legend found in the rock) has one modest power that
//! works: a hammer that never tires its maker, a cup that calms tantrums,
//! a helm that turns blows aside, a lantern that keeps cave things away, a
//! ring that brings luck in trade, a bed that mends the wounded, a blade
//! that puts foes to flight. Word of them gets about, and they are coveted.

use super::*;
use crate::creature::{Creature, Species};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Power {
    /// Its maker never tires.
    Tireless,
    /// Tantrums near it die away.
    Calm,
    /// Its wearer turns blows aside.
    Ward,
    /// Cave things keep away from it.
    Lantern,
    /// Merchants give way in a bargain.
    Luck,
    /// The wounded near it mend.
    Healing,
    /// Foes flee before whoever wields it.
    Dread,
}

impl Power {
    /// "never tires the hand that made it".
    pub fn blurb(self) -> &'static str {
        match self {
            Power::Tireless => "never tires the hand that made it or holds it",
            Power::Calm => "calms the angry who come near it",
            Power::Ward => "turns aside the blows aimed at its wearer",
            Power::Lantern => "keeps the things of the caverns away",
            Power::Luck => "brings luck in trade",
            Power::Healing => "mends the wounded who rest near it",
            Power::Dread => "puts its wielder's foes to flight",
        }
    }
}

/// An artifact with a power.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Relic {
    pub item: u32,
    /// The artifact's own name.
    pub name: String,
    /// "cup", "hammer", "lantern"...
    pub form: String,
    pub power: Power,
    /// The dwarf who made it, if one did.
    pub maker: Option<u32>,
    pub made: u64,
    /// Who has coveted it.
    #[serde(default)]
    pub coveted_by: Vec<u32>,
    /// How often its power has shown.
    #[serde(default)]
    pub uses: u32,
    /// When word of it last drew a thief.
    #[serde(default)]
    pub sought: u64,
}

impl Relic {
    /// "Kogan Ashtomb, a cup that calms the angry who come near it".
    pub fn told(&self) -> String {
        format!("{}, {} {} that {}", self.name, article(&self.form), self.form, self.power.blurb())
    }
}

fn article(w: &str) -> &'static str {
    if w.starts_with(|c: char| "aeiou".contains(c)) {
        "an"
    } else {
        "a"
    }
}

/// The power an artifact of this kind gets, and what it is.
fn power_for(kind: ItemKind, roll: u64) -> (Power, &'static str) {
    use ItemKind::*;
    match kind {
        Hammer => (Power::Tireless, "hammer"),
        Axe => (Power::Dread, "axe"),
        Sword => (Power::Dread, "sword"),
        Helm => (Power::Ward, "helm"),
        Breastplate => (Power::Ward, "breastplate"),
        Bed => (Power::Healing, "bed"),
        Table => (Power::Calm, "table"),
        Chair => (Power::Calm, "throne"),
        Door => (Power::Calm, "door"),
        _ => [(Power::Calm, "cup"), (Power::Lantern, "lantern"), (Power::Luck, "ring"), (Power::Healing, "amulet")][(roll % 4) as usize],
    }
}

/// How far a relic's power reaches.
const CALM_R: i32 = 10;
const HEAL_R: i32 = 5;
const LANTERN_R: i32 = 10;

fn reach(a: Pos, b: Pos, r: i32) -> bool {
    (a.z - b.z).abs() <= 1 && (a.x - b.x).abs() <= r && (a.y - b.y).abs() <= r
}

fn cave_thing(c: &Creature) -> bool {
    !c.tame && c.alive && matches!(c.species, Species::CaveHorror | Species::GiantSpider | Species::Troglodyte)
}

impl Game {
    /// A new artifact gets its power. `maker` made it in a strange mood.
    pub(super) fn empower_artifact(&mut self, item: u32, maker: Option<(u32, String)>) {
        let Some((kind, label)) = self.items.get(&item).map(|it| (it.kind, it.label.clone())) else { return };
        if self.magic.relics.iter().any(|r| r.item == item) {
            return;
        }
        let roll = self.mrng().next_u64();
        let (power, form) = power_for(kind, roll);
        let name = label.unwrap_or_else(|| "an artifact".into());
        let relic = Relic { item, name, form: form.into(), power, maker: maker.as_ref().map(|m| m.0), made: self.tick, coveted_by: Vec::new(), uses: 0, sought: self.tick };
        let told = relic.told();
        self.msg(&format!("{} is no ordinary {}: it {}.", relic.name, form, power.blurb()), (255, 215, 60));
        let fort = self.fortress_name.clone();
        match &maker {
            Some((_, who)) => self.legend(format!("{} made {} in {}.", who, told, fort)),
            None => self.legend(format!("{} came to light in {}.", told, fort)),
        }
        // The card for the artifact tells of its power too.
        if let Some(s) = self.spotlights.iter_mut().rev().find(|s| s.title == "An artifact") {
            s.text.push_str(&format!(" It is {} {} that {}.", article(form), form, power.blurb()));
        }
        self.magic.relics.push(relic);
        self.count("magic.relics", 1);
    }

    /// Where a relic is: in someone's hands, or where it lies.
    pub(super) fn relic_pos(&self, item: u32) -> Option<Pos> {
        let it = self.items.get(&item)?;
        if let Some(by) = it.carried_by {
            if let Some(d) = self.dwarves.iter().find(|d| d.id == by && d.alive) {
                return Some(d.pos);
            }
            if let Some(c) = self.creatures.get(&by) {
                return Some(c.pos);
            }
        }
        if let Some(d) = self.dwarves.iter().find(|d| d.alive && (d.held == Some(item) || d.armor.contains(&item))) {
            return Some(d.pos);
        }
        Some(it.pos)
    }

    /// A ring of luck in the fortress: the merchants give way.
    pub fn trade_luck(&self) -> bool {
        self.magic.relics.iter().any(|r| r.power == Power::Luck && self.items.get(&r.item).map_or(false, |it| it.carried_by.map_or(true, |c| !self.creatures.contains_key(&c))))
    }

    /// Every 20 ticks: the relics' powers at work.
    pub(super) fn relics_tick(&mut self) {
        if self.magic.relics.is_empty() {
            return;
        }
        // A relic gone (stolen off the map, burnt) is mourned once.
        let gone: Vec<Relic> = self.magic.relics.iter().filter(|r| !self.items.contains_key(&r.item)).cloned().collect();
        for r in gone {
            self.magic.relics.retain(|x| x.item != r.item);
            self.msg(&format!("{} is lost to the fortress.", r.name), (230, 160, 120));
            let fort = self.fortress_name.clone();
            self.legend(format!("{} was lost from {}.", r.told(), fort));
        }
        let tick = self.tick;
        let relics = self.magic.relics.clone();
        for r in relics {
            let Some(at) = self.relic_pos(r.item) else { continue };
            let mut used = false;
            match r.power {
                Power::Tireless => {
                    // Its maker, and whoever has it in hand.
                    for d in self.dwarves.iter_mut().filter(|d| d.alive && (Some(d.id) == r.maker || d.held == Some(r.item))) {
                        used |= d.fatigue > 0;
                        d.fatigue = d.fatigue.saturating_sub(20);
                    }
                }
                Power::Calm => {
                    let calmed: Vec<usize> = (0..self.dwarves.len()).filter(|&i| self.dwarves[i].alive && self.dwarves[i].tantrum > 0 && reach(self.dwarves[i].pos, at, CALM_R)).collect();
                    for i in calmed {
                        let d = &mut self.dwarves[i];
                        d.tantrum = 0;
                        d.stress -= 40;
                        d.think(&format!("was calmed by {}", r.name), -10, tick);
                        let name = d.name.clone();
                        self.msg(&format!("{}'s fury drains away before {}.", name, r.name), (190, 220, 255));
                        used = true;
                    }
                }
                Power::Healing if tick % 100 < 20 => {
                    for d in self.dwarves.iter_mut().filter(|d| d.alive && !d.ghost && reach(d.pos, at, HEAL_R) && !d.body.healed()) {
                        d.body.heal(2);
                        used = true;
                    }
                }
                Power::Lantern => {
                    // Already shrinking away (on its way, or lately found no
                    // way out): no need to search it a path again so soon.
                    let fleeing = |c: &crate::creature::Creature| c.wary_until > tick && c.target.is_none() && c.stalk.is_none() && (!c.path.is_empty() || c.wary_until > tick + DAY / 2 - 200);
                    let near: Vec<u32> = self.creatures.values().filter(|c| cave_thing(c) && reach(c.pos, at, LANTERN_R) && !fleeing(c)).map(|c| c.id).collect();
                    for id in near {
                        let Some(mut c) = self.creatures.remove(&id) else { continue };
                        c.target = None;
                        c.stalk = None;
                        c.wary_until = tick + DAY / 2;
                        let away = |q: Pos| (q.x - at.x).abs().max((q.y - at.y).abs()) >= LANTERN_R + 4 || (q.z - at.z).abs() > 1;
                        if let Some((_, path)) = path::search(&self.world, c.pos, 900, c.mover(), away) {
                            c.path = path.into_iter().rev().collect();
                        }
                        let what = c.title();
                        self.creatures.insert(id, c);
                        self.warn(&format!("lantern{}", id), &format!("{} shrinks from the light of {}.", combat::cap(&what), r.name), (255, 235, 160));
                        used = true;
                    }
                }
                Power::Dread => {
                    let Some(w) = self.dwarves.iter().find(|d| d.alive && d.held == Some(r.item)) else { continue };
                    let (wp, wname) = (w.pos, w.name.clone());
                    let near: Vec<u32> = self
                        .creatures
                        .values()
                        .filter(|c| c.alive && c.hostile() && !c.legendary() && !c.leaving && reach(c.pos, wp, 2))
                        .map(|c| c.id)
                        .collect();
                    for id in near {
                        if self.mrng().chance(0.75) {
                            continue;
                        }
                        let c = self.creatures.get_mut(&id).unwrap();
                        c.leaving = true;
                        c.target = None;
                        c.path.clear();
                        let what = c.title();
                        self.msg(&format!("{} flees in terror before {} and {}!", combat::cap(&what), wname.split(' ').next().unwrap_or(""), r.name), (190, 220, 255));
                        used = true;
                    }
                }
                _ => {}
            }
            if used {
                if let Some(x) = self.magic.relics.iter_mut().find(|x| x.item == r.item) {
                    x.uses += 1;
                }
            }
        }
    }

    /// When a creature strikes dwarf `di`: a warding relic they wear may
    /// turn the blow aside (true: no blow lands).
    pub(super) fn blow_turned(&mut self, c: &Creature, di: usize) -> bool {
        let d = &self.dwarves[di];
        let Some(r) = self.magic.relics.iter().find(|r| r.power == Power::Ward && d.armor.contains(&r.item)) else { return false };
        let (name, rname, item) = (d.name.clone(), r.name.clone(), r.item);
        if !self.mrng().chance(self.knob("magic.ward", 0.35)) {
            return false;
        }
        let first = name.split(' ').next().unwrap_or("").to_string();
        self.warn(&format!("ward{}", item), &format!("{} turns aside the {}'s blow from {}.", rname, c.name(), first), (190, 220, 255));
        if let Some(x) = self.magic.relics.iter_mut().find(|x| x.item == item) {
            x.uses += 1;
        }
        true
    }

    /// Each season: word of the relics gets about. A dwarf may covet one,
    /// and a relic lying about may draw a thief.
    pub(super) fn relics_season(&mut self) {
        let tick = self.tick;
        let relics: Vec<Relic> = self.magic.relics.iter().filter(|r| self.items.contains_key(&r.item) && tick >= r.made + SEASON).cloned().collect();
        for r in relics {
            let mut rng = self.mrng();
            if rng.chance(self.knob("magic.covet", 0.35)) {
                // The greedy and the artful covet most.
                let cands: Vec<usize> = (0..self.dwarves.len())
                    .filter(|&i| {
                        let d = &self.dwarves[i];
                        d.alive && !d.ghost && !d.is_child() && Some(d.id) != r.maker && !r.coveted_by.contains(&d.id) && d.personality.artistry >= 10
                    })
                    .collect();
                if !cands.is_empty() {
                    let i = cands[rng.range(0, cands.len() as i32) as usize];
                    let d = &mut self.dwarves[i];
                    d.think(&format!("covets {}", r.name), 12, tick);
                    let (id, name) = (d.id, d.name.clone());
                    if let Some(x) = self.magic.relics.iter_mut().find(|x| x.item == r.item) {
                        x.coveted_by.push(id);
                    }
                    self.gossip(format!("{} can't stop talking about {}, {} {} that {}.", name, r.name, article(&r.form), r.form, r.power.blurb()));
                }
            }
            // A relic left lying where anyone could pick it up draws a thief, once a year or so.
            let free = self.items.get(&r.item).map_or(false, |it| it.free());
            if free && tick >= r.sought + YEAR && rng.chance(self.knob("magic.thief", 0.5)) {
                if let Some(entry) = self.edge_entry() {
                    self.spawn_thief(entry);
                    if let Some(x) = self.magic.relics.iter_mut().find(|x| x.item == r.item) {
                        x.sought = tick;
                    }
                    self.msg(&format!("Word of {} has spread beyond the mountain, and someone covets it.", r.name), (230, 160, 120));
                    self.count("magic.relic_thief", 1);
                }
            }
        }
    }

    /// Alert check for the overseer: a thief making off with a relic has
    /// the squad sent after it.
    pub(super) fn ov_relics(&mut self, o: &mut super::overseer::Overseer) {
        let Some((cid, name)) = self
            .creatures
            .values()
            .filter(|c| c.alive && !c.sneaking)
            .find_map(|c| c.loot.and_then(|l| self.magic.relics.iter().find(|r| r.item == l)).map(|r| (c.id, r.name.clone())))
        else {
            return;
        };
        let Some(s) = self.squads.iter_mut().find(|s| !s.members.is_empty() && s.order != SquadOrder::Kill(cid)) else { return };
        s.order = SquadOrder::Kill(cid);
        let squad = s.name.clone();
        Self::note_ov(o, self.tick, format!("A thief is making off with {}! Sent {} after it.", name, squad));
    }

    /// For the look panel: an item's power, maker and admirers.
    pub fn relic_lines(&self, item: u32) -> Vec<String> {
        let Some(r) = self.magic.relics.iter().find(|r| r.item == item) else { return Vec::new() };
        let mut v = vec![format!("A {} that {}.", r.form, r.power.blurb())];
        let year = self.start_year + (r.made / YEAR) as i32;
        match r.maker.and_then(|m| self.dwarves.iter().find(|d| d.id == m)) {
            Some(d) => v.push(format!("Made by {} in a strange mood, in the year {}.", d.name, year)),
            None => v.push(format!("A relic of legend, found in the year {}.", year)),
        }
        if r.uses > 0 {
            v.push(format!("Its power has shown {} time{}.", r.uses, if r.uses == 1 { "" } else { "s" }));
        }
        let who: Vec<String> = r.coveted_by.iter().filter_map(|id| self.dwarves.iter().find(|d| d.id == *id && d.alive)).map(|d| d.name.clone()).collect();
        if !who.is_empty() {
            v.push(format!("Coveted by {}.", who.join(", ")));
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(g: &mut Game, kind: ItemKind, at: Pos, maker: Option<usize>) -> u32 {
        let id = g.spawn_item(kind, crate::material::Mat::Granite, at);
        let it = g.items.get_mut(&id).unwrap();
        it.quality = crate::items::ARTIFACT;
        it.label = Some("Kogan Ashtomb".into());
        let m = maker.map(|i| (g.dwarves[i].id, g.dwarves[i].name.clone()));
        g.empower_artifact(id, m);
        id
    }

    fn quiet() -> Game {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        g
    }

    #[test]
    fn each_artifact_gets_a_power_told_in_the_legends_and_the_look_panel() {
        let mut g = quiet();
        let e = g.embark;
        let id = artifact(&mut g, ItemKind::Hammer, e, Some(0));
        let r = g.magic.relics.iter().find(|r| r.item == id).unwrap().clone();
        assert_eq!((r.power, r.form.as_str()), (Power::Tireless, "hammer"));
        assert!(g.log.iter().any(|l| l.text == "Kogan Ashtomb is no ordinary hammer: it never tires the hand that made it or holds it."));
        assert!(g.overworld.events.iter().any(|ev| ev.text.contains("Kogan Ashtomb, a hammer that never tires")));
        let lines = g.relic_lines(id);
        assert_eq!(lines[0], "A hammer that never tires the hand that made it or holds it.");
        assert!(lines[1].starts_with(&format!("Made by {}", g.dwarves[0].name)));
        // Its maker never tires.
        g.dwarves[0].fatigue = 5000;
        for _ in 0..10 {
            g.relics_tick();
        }
        assert!(g.dwarves[0].fatigue <= 4800);
        // The same artifact never gets two.
        g.empower_artifact(id, None);
        assert_eq!(g.magic.relics.len(), 1);
    }

    #[test]
    fn a_calming_cup_ends_a_tantrum_nearby() {
        let mut g = quiet();
        let e = g.embark;
        let id = artifact(&mut g, ItemKind::Table, e, None);
        assert_eq!(g.magic.relics[0].power, Power::Calm);
        g.dwarves[1].pos = e;
        g.dwarves[1].tantrum = 900;
        g.relics_tick();
        assert_eq!(g.dwarves[1].tantrum, 0);
        assert!(g.log.iter().any(|l| l.text.contains("fury drains away before Kogan Ashtomb")));
        assert!(g.relic_lines(id).iter().any(|l| l.starts_with("Its power has shown")));
    }

    #[test]
    fn a_warding_helm_turns_blows_aside() {
        let mut g = quiet();
        let e = g.embark;
        let id = artifact(&mut g, ItemKind::Helm, e, None);
        assert_eq!(g.magic.relics[0].power, Power::Ward);
        g.dwarves[0].armor.push(id);
        let gob = g.spawn_creature(Species::Goblin, e, DAY);
        let c = g.creatures[&gob].clone();
        let turned = (0..60).filter(|_| g.blow_turned(&c, 0)).count();
        assert!(turned > 5 && turned < 45, "turned {} of 60", turned);
        // Without the helm, nothing is turned.
        g.dwarves[0].armor.clear();
        assert!((0..30).all(|_| !g.blow_turned(&c, 0)));
    }

    #[test]
    fn a_lantern_drives_cave_things_off_and_a_ring_brings_luck_in_trade() {
        let mut g = quiet();
        let e = g.embark;
        // Crafts come as cups, lanterns, rings or amulets.
        let mut forms = Vec::new();
        for _ in 0..24 {
            let id = artifact(&mut g, ItemKind::Craft, e, None);
            let r = g.magic.relics.iter().find(|r| r.item == id).unwrap();
            forms.push(r.form.clone());
        }
        for f in ["cup", "lantern", "ring", "amulet"] {
            assert!(forms.iter().any(|x| x == f), "no {} among {:?}", f, forms);
        }
        assert!(g.trade_luck());
        assert_eq!(g.trade_margin(), 0, "a ring of luck: the merchants want nothing on top");
        let spider = g.spawn_creature(Species::GiantSpider, e.offset(2, 0, 0), DAY);
        g.creatures.get_mut(&spider).unwrap().target = Some(g.dwarves[0].id);
        g.relics_tick();
        let c = &g.creatures[&spider];
        assert!(c.target.is_none() && c.wary_until > g.tick);
        assert!(g.log.iter().any(|l| l.text.contains("shrinks from the light of")));
    }

    #[test]
    fn a_dread_blade_puts_foes_to_flight_and_relics_are_coveted() {
        let mut g = quiet();
        let e = g.embark;
        let id = artifact(&mut g, ItemKind::Sword, e, Some(0));
        g.dwarves[1].held = Some(id);
        g.dwarves[1].pos = e;
        let gob = g.spawn_creature(Species::Goblin, e.offset(1, 0, 0), DAY);
        for _ in 0..30 {
            g.relics_tick();
        }
        assert!(g.creatures[&gob].leaving, "the goblin stood its ground");
        // A season on, word gets about.
        for d in g.dwarves.iter_mut() {
            d.personality.artistry = 40;
        }
        g.tick += YEAR + SEASON;
        for _ in 0..12 {
            g.relics_season();
        }
        assert!(!g.magic.relics[0].coveted_by.is_empty(), "nobody coveted it");
        assert!(g.dwarves.iter().any(|d| d.thoughts.iter().any(|t| t.text == "covets Kogan Ashtomb")));
        assert!(g.relic_lines(id).iter().any(|l| l.starts_with("Coveted by")));
        // Stolen away, it is mourned.
        g.items.remove(&id);
        g.relics_tick();
        assert!(g.magic.relics.is_empty());
        assert!(g.log.iter().any(|l| l.text == "Kogan Ashtomb is lost to the fortress."));
    }
}
