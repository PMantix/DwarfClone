//! The resident wizard (MYT-45, MYT-48): now and then a wandering wizard
//! comes to the gate and asks to settle. The overseer decides by its
//! traits (the pious, vain, greedy and bold take them in; the cautious
//! send them away); a player's fort welcomes them. A wizard wants a
//! laboratory (the top of the wizard's tower, `fortify.rs`), rare
//! materials (gems, metal bars) and an assistant. In return they hurl
//! witchfire at the walking dead, the husks and the horrors of the deep,
//! banish ghosts and read omens; and now and then something goes wrong in
//! the laboratory.

use super::overseer::Overseer;
use super::personality::Trait;
use super::*;
use crate::creature::{Creature, Species};
use crate::dwarf::{Need, NeedKind};
use serde::{Deserialize, Serialize};

/// What a wizard asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Want {
    Gem,
    Bars,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Wizard {
    /// The dwarf they became on joining.
    pub dwarf: u32,
    pub name: String,
    /// "the Grey".
    pub title: String,
    pub arrived: u64,
    /// The laboratory zone they work in (0: none yet).
    pub lab: u32,
    pub assistant: Option<u32>,
    /// -100 (miserable) .. 100 (delighted).
    pub content: i32,
    /// When they last asked for materials, and for what.
    pub asked: u64,
    pub want: Option<Want>,
    /// When they last cast a spell, and what it was.
    pub cast: u64,
    pub spell: String,
    pub spells: u32,
    pub accidents: u32,
}

const TITLES: [&str; 6] = ["the Grey", "of the Seventh Flame", "Starreader", "the Unquiet", "of the Deep Lamp", "Ashcloak"];

/// What a wizard's spells are good against.
fn uncanny(c: &Creature) -> bool {
    c.alive && !c.tame && (matches!(c.species, Species::Undead | Species::Husk | Species::CaveHorror) || (c.figure.is_some() && c.hostile()))
}

fn gem(it: &Item) -> bool {
    it.kind == ItemKind::Gem || (it.kind == ItemKind::Boulder && it.mat.is_gem())
}

impl Game {
    fn wizard_dwarf(&self) -> Option<&Dwarf> {
        let w = self.magic.wizard.as_ref()?;
        self.dwarves.iter().find(|d| d.id == w.dwarf && d.alive)
    }

    /// The wizard wants a laboratory and the fort has none (the overseer
    /// raises a wizard's tower for them).
    pub(super) fn wizard_wants_lab(&self) -> bool {
        self.magic.wizard.as_ref().map_or(false, |w| w.lab == 0) && !(self.works.lab != 0 && self.zones.contains_key(&self.works.lab))
    }

    /// Spring, summer and autumn: a wandering wizard may come asking to settle.
    pub(super) fn wizard_season(&mut self) {
        let season = (self.tick / SEASON) % 4;
        if season == 3 || self.magic.wizard.is_some() || self.tick < YEAR || self.alive_count() < 10 {
            return;
        }
        if self.magic.wizard_refused > 0 && self.tick < self.magic.wizard_refused + YEAR {
            return;
        }
        let lab = self.works.lab != 0 && self.zones.contains_key(&self.works.lab);
        let mut rng = self.mrng();
        if !rng.chance(if lab { self.knob("magic.wizard_lab", 0.6) } else { self.knob("magic.wizard", 0.2) }) {
            return;
        }
        let name = crate::overworld::person_name(&mut rng, Race::Dwarf);
        let title = TITLES[rng.range(0, TITLES.len() as i32) as usize].to_string();
        self.wizard_asks(name, title, lab);
    }

    /// A wizard at the gate asks to settle: the overseer decides by its
    /// traits; a player's fort takes them in.
    pub(super) fn wizard_asks(&mut self, name: String, title: String, lab: bool) {
        let fort = self.fortress_name.clone();
        let called = format!("{} {}", name, title);
        self.msg(&format!("A wandering wizard, {}, has come to the gate of {} and asks to settle.", called, fort), (200, 170, 255));
        let tick = self.tick;
        let verdict = self.overseer.as_ref().map(|o| {
            if o.has(Trait::Cautious) {
                (false, "Sent the wizard on their way: no fire-throwing strangers behind our gate.")
            } else if o.has(Trait::Pious) {
                (true, "Took the wizard in: who knows what the gods sent them for.")
            } else if o.has(Trait::Vain) {
                (true, "Took the wizard in: every great hold has its wizard.")
            } else if o.has(Trait::Greedy) {
                (true, "Took the wizard in: there may be gold in their secrets.")
            } else if o.has(Trait::Bold) || o.style == Style::Builder {
                (true, "Took the wizard in, and the risk with them.")
            } else if lab {
                (true, "Took the wizard in: the laboratory stood empty anyway.")
            } else {
                (false, "Turned the wizard away: we have no laboratory, and no use for one.")
            }
        });
        if let (Some((yes, why)), Some(o)) = (verdict, self.overseer.as_mut()) {
            Self::note_ov(o, tick, format!("{} ({})", why, called));
            if !yes {
                self.magic.wizard_refused = tick;
                self.msg(&format!("{} has been turned away from {}.", called, fort), (200, 170, 255));
                return;
            }
        }
        // They settle: a dwarf of the fort from now on.
        let id = self.new_id();
        let at = self.edge_entry().unwrap_or(self.embark);
        let mut rng = self.mrng();
        let mut d = dwarf::new_dwarf(id, at, &mut rng, &[]);
        d.name = name.clone();
        d.epithet = Some("Wizard".into());
        d.epithet_rank = super::heroes::RANK_FEAT;
        d.deed = Some(format!("came to {} to study the hidden arts", fort));
        // Learning is their life: they go up to the laboratory whenever they can.
        d.needs.retain(|n| n.kind != NeedKind::Learn);
        d.needs.push(Need { kind: NeedKind::Learn, strength: 3, level: 0, low: false });
        self.dress(&mut d, 300);
        self.dwarves.push(d);
        self.year_log.arrivals += 1;
        self.magic.wizard = Some(Wizard {
            dwarf: id,
            name: name.clone(),
            title: title.clone(),
            arrived: tick,
            lab: 0,
            assistant: None,
            content: 20,
            asked: tick,
            want: None,
            cast: 0,
            spell: String::new(),
            spells: 0,
            accidents: 0,
        });
        let text = format!("{} has settled in {}. The wizard will want a laboratory, rare materials and an assistant, and will stand with us against the uncanny.", called, fort);
        self.msg(&text, (200, 170, 255));
        self.spotlight("A wizard", &text, Some(at), (200, 170, 255));
        self.legend(format!("{}, a wandering wizard, settled in {}.", called, fort));
        self.count("magic.wizards", 1);
    }

    /// Daily: the wizard's laboratory, materials and assistant, and the
    /// accidents that come of them.
    pub(super) fn wizard_daily(&mut self) {
        let Some(w) = self.magic.wizard.clone() else { return };
        let fort = self.fortress_name.clone();
        let called = format!("{} {}", w.name, w.title);
        let Some(wi) = self.dwarves.iter().position(|d| d.id == w.dwarf && d.alive) else {
            self.magic.wizard = None;
            self.legend(format!("{}, the wizard of {}, is no more.", called, fort));
            return;
        };
        let tick = self.tick;
        let mut w = w;
        // A laboratory.
        let lab_ok = |g: &Game, z: u32| z != 0 && g.zones.get(&z).map_or(false, |z| z.kind == ZoneKind::Laboratory);
        if !lab_ok(self, w.lab) {
            w.lab = 0;
            let free = self.zones.values().filter(|z| z.kind == ZoneKind::Laboratory).map(|z| z.id).min();
            if let Some(z) = free {
                w.lab = z;
                w.content = (w.content + 30).min(100);
                let place = self.work_at(self.zones[&z].tiles[0]).unwrap_or_else(|| "the laboratory".into());
                self.msg(&format!("{} has claimed {} and set up their instruments.", called, place), (200, 170, 255));
                self.legend(format!("{} took {} in {} for a laboratory.", called, place, fort));
            } else if (tick / DAY) % 7 == 0 {
                w.content -= 5;
                self.dwarves[wi].think("has no laboratory to work in", 15, tick);
                self.msg(&format!("{} asks for a laboratory: a high room in a tower would do.", called), (200, 170, 255));
            }
        }
        // Rare materials, every half season.
        if tick >= w.asked + SEASON / 2 {
            w.asked = tick;
            let want = if self.mrng().chance(0.5) { Want::Gem } else { Want::Bars };
            let found = self.items.values().filter(|it| it.free() && !it.forbidden && if want == Want::Gem { gem(it) } else { it.kind == ItemKind::Bar }).map(|it| it.id).min();
            match found {
                Some(id) => {
                    let what = self.items[&id].name();
                    self.items.remove(&id);
                    w.content = (w.content + 15).min(100);
                    w.want = None;
                    self.msg(&format!("{} took {} up to the laboratory for their studies.", called, what), (200, 170, 255));
                }
                None => {
                    w.content -= 15;
                    w.want = Some(want);
                    self.trouble(if want == Want::Gem { troubles::Lack::Gem } else { troubles::Lack::Bars });
                    let what = if want == Want::Gem { "gems" } else { "metal bars" };
                    self.msg(&format!("{} asks for {} for their studies, and there are none to give.", called, what), (200, 170, 255));
                    self.dwarves[wi].think(&format!("was refused {} for their studies", what), 10, tick);
                }
            }
        }
        // An assistant.
        if !w.assistant.map_or(false, |a| self.dwarves.iter().any(|d| d.id == a && d.alive && !d.insane)) {
            w.assistant = None;
            let pick = self
                .dwarves
                .iter()
                .filter(|d| d.alive && !d.insane && !d.ghost && !d.is_child() && !d.soldier && d.noble.is_none() && d.id != w.dwarf && d.mood.is_none())
                .max_by_key(|d| (d.need(NeedKind::Learn).map_or(0, |n| n.strength), std::cmp::Reverse(d.id)))
                .map(|d| (d.id, d.name.clone()));
            if let Some((a, aname)) = pick {
                w.assistant = Some(a);
                w.content = (w.content + 10).min(100);
                self.msg(&format!("{} has become the assistant of {}.", aname, called), (200, 170, 255));
                if let Some(o) = self.overseer.as_mut() {
                    Self::note_ov(o, tick, format!("Gave {} {} for an assistant: the keenest to learn of us all.", called, aname));
                }
            }
        }
        if let Some(a) = w.assistant {
            if (tick / DAY) % 5 == 0 {
                if let Some(d) = self.dwarves.iter_mut().find(|d| d.id == a) {
                    d.think("learned a little of the hidden arts", -6, tick);
                    d.satisfy(NeedKind::Learn, 400);
                }
            }
        }
        // Content or not.
        w.content = (w.content - 1).clamp(-100, 100);
        if w.content <= -50 && (tick / DAY) % 7 == 3 {
            self.dwarves[wi].think("is unhappy with how the fortress treats a wizard", 20, tick);
        }
        // Something goes wrong in the laboratory.
        let in_lab = w.lab != 0 && self.world.get(self.dwarves[wi].pos).map_or(false, |t| t.zone == w.lab);
        let mut rng = self.mrng();
        if in_lab && rng.chance(self.knob("magic.accident", 0.05)) {
            w.accidents += 1;
            let at = self.dwarves[wi].pos;
            let victim = w.assistant.and_then(|a| self.dwarves.iter().position(|d| d.id == a && d.alive)).unwrap_or(wi);
            let vname = self.dwarves[victim].name.split(' ').next().unwrap_or("").to_string();
            let what = match rng.range(0, 3) {
                0 => {
                    self.puff(at, 6);
                    if !self.ignite(at) {
                        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                            if self.ignite(at.offset(dx, dy, 0)) {
                                break;
                            }
                        }
                    }
                    format!("A spell went awry in {}'s laboratory: fire and smoke!", w.name)
                }
                1 => {
                    self.dwarves[victim].think("had their beard turned blue in a laboratory accident", 12, tick);
                    self.gossip(format!("{}'s beard has been bright blue since the wizard's last experiment.", vname));
                    format!("A cloud of violet smoke rolled out of {}'s laboratory, and {}'s beard came out of it blue.", w.name, vname)
                }
                _ => {
                    let d = &mut self.dwarves[victim];
                    let part = d.body.pick_part(&mut rng);
                    d.body.wound(part, 3, false);
                    d.think("was singed by a wild spark", 10, tick);
                    format!("A wild spark leapt from {}'s workbench and singed {}.", w.name, vname)
                }
            };
            self.msg(&what, (255, 160, 90));
            self.magic.year.push(what);
            self.count("magic.accidents", 1);
        }
        self.magic.wizard = Some(w);
    }

    /// Every 20 ticks: the wizard stands against the uncanny nearby, a
    /// spell at a time, and lays ghosts to rest.
    pub(super) fn wizard_tick(&mut self) {
        let Some(w) = self.magic.wizard.as_ref() else { return };
        let Some(d) = self.wizard_dwarf() else { return };
        if d.asleep || d.insane || d.body.unconscious() || self.tick < w.cast + self.knob("magic.spell_cd", 150.0) as u64 {
            return;
        }
        let (at, first) = (d.pos, d.name.split(' ').next().unwrap_or("").to_string());
        let reach = |p: Pos, r: i32| (p.z - at.z).abs() <= 1 && (p.x - at.x).abs() <= r && (p.y - at.y).abs() <= r;
        // A ghost near is laid to rest.
        if let Some(i) = (0..self.dwarves.len()).find(|&i| self.dwarves[i].ghost && reach(self.dwarves[i].pos, 8)) {
            let ghost = self.dwarves[i].name.clone();
            self.lay_to_rest(i, "has been laid to rest by a wizard", false);
            let text = format!("{} the Wizard speaks a word of power, and the ghost of {} is banished.", first, ghost);
            self.cast(text);
            return;
        }
        let foe = self.creatures.values().filter(|c| uncanny(c) && reach(c.pos, 12)).min_by_key(|c| (c.pos.x - at.x).abs() + (c.pos.y - at.y).abs()).map(|c| c.id);
        let Some(cid) = foe else { return };
        let mut rng = self.mrng();
        let dmg = self.knob("magic.witchfire", 9.0) as i32;
        let c = self.creatures.get_mut(&cid).unwrap();
        let part = c.body.pick_part(&mut rng);
        c.body.wound(part, dmg, false);
        let what = c.title();
        let text = format!("{} the Wizard hurls witchfire at {}!", first, what);
        self.cast(text);
        self.count("magic.spells", 1);
    }

    fn cast(&mut self, text: String) {
        let tick = self.tick;
        self.msg(&text, (200, 170, 255));
        if let Some(w) = self.magic.wizard.as_mut() {
            w.cast = tick;
            w.spell = text;
            w.spells += 1;
        }
    }

    /// The wizard reads an omen aloud.
    pub(super) fn wizard_reads(&mut self, kind: super::omens::OmenKind, due: u64) {
        let Some(d) = self.wizard_dwarf() else { return };
        let first = d.name.split(' ').next().unwrap_or("").to_string();
        let days = (due.saturating_sub(self.tick) + DAY - 1) / DAY;
        self.msg(&format!("{} the Wizard reads the omen: {}, within {} days.", first, kind.meaning(), days), (200, 170, 255));
    }

    /// Daily for the overseer: a wizard without a laboratory is noted.
    pub(super) fn ov_wizard(&mut self, o: &mut Overseer) {
        if self.wizard_wants_lab() && !o.did("wizardlab") {
            o.done.push("wizardlab".into());
            let who = self.magic.wizard.as_ref().map_or(String::new(), |w| format!("{} {}", w.name, w.title));
            Self::note_ov(o, self.tick, format!("{} wants a laboratory; a tower for the wizard goes into the plans.", who));
        }
    }

    /// For the look panel: the wizard and their assistant.
    pub fn wizard_lines(&self, dwarf: u32) -> Vec<String> {
        let Some(w) = self.magic.wizard.as_ref() else { return Vec::new() };
        let mut v = Vec::new();
        if dwarf == w.dwarf {
            let mood = match w.content {
                c if c >= 40 => "content",
                c if c >= 0 => "getting by",
                c if c >= -50 => "discontented",
                _ => "miserable",
            };
            v.push(format!("The fortress's wizard, {} ({}).", w.title, mood));
            let lab = if w.lab != 0 { "has a laboratory" } else { "wants a laboratory" };
            let assistant = w.assistant.and_then(|a| self.dwarves.iter().find(|d| d.id == a)).map_or("no assistant".to_string(), |d| format!("{} assists", d.name));
            v.push(format!("{}; {}; {} spells cast.", cap(lab), assistant, w.spells));
            if let Some(want) = w.want {
                v.push(format!("Wants {} for their studies.", if want == Want::Gem { "gems" } else { "metal bars" }));
            }
        } else if w.assistant == Some(dwarf) {
            v.push(format!("Assistant to {} {}.", w.name, w.title));
        }
        v
    }

    /// For the director: the wizard at a spell, or at study.
    pub fn wizard_sight(&self) -> Option<(u32, String, bool)> {
        let w = self.magic.wizard.as_ref()?;
        let d = self.wizard_dwarf()?;
        if w.cast > 0 && self.tick < w.cast + DAY / 8 {
            return Some((d.id, w.spell.trim_end_matches(['!', '.']).to_string(), true));
        }
        if w.lab != 0 && self.world.get(d.pos).map_or(false, |t| t.zone == w.lab) {
            return Some((d.id, format!("{} the Wizard at study in the laboratory", d.name.split(' ').next().unwrap_or("")), false));
        }
        None
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn big_fort() -> Game {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        while g.alive_count() < 10 {
            let id = g.new_id();
            let d = dwarf::new_dwarf(id, g.embark, &mut g.rng, &[]);
            g.dwarves.push(d);
        }
        g
    }

    #[test]
    fn the_overseer_takes_a_wizard_in_or_turns_them_away_by_its_traits() {
        let mut g = big_fort();
        g.set_overseer(Some(Style::Steady));
        g.overseer.as_mut().unwrap().traits = vec![Trait::Cautious];
        let n = g.dwarves.len();
        g.wizard_asks("Urist Lamp".into(), "the Grey".into(), false);
        assert!(g.magic.wizard.is_none() && g.dwarves.len() == n);
        assert!(g.overseer.as_ref().unwrap().notes.iter().any(|x| x.1.starts_with("Sent the wizard on their way")));
        assert!(g.magic.wizard_refused > 0 || g.tick == 0);
        g.overseer.as_mut().unwrap().traits = vec![Trait::Pious];
        g.wizard_asks("Urist Lamp".into(), "the Grey".into(), false);
        let w = g.magic.wizard.clone().expect("the pious take them in");
        let d = g.dwarves.iter().find(|d| d.id == w.dwarf).unwrap();
        assert_eq!(d.called(), "Urist the Wizard");
        assert!(g.spotlights.iter().any(|s| s.title == "A wizard"));
        assert!(g.overworld.events.iter().any(|e| e.text.contains("a wandering wizard, settled in")));
        assert!(g.wizard_lines(w.dwarf)[0].starts_with("The fortress's wizard"));
    }

    #[test]
    fn the_wizard_wants_a_laboratory_materials_and_an_assistant() {
        let mut g = big_fort();
        g.wizard_asks("Urist Lamp".into(), "the Grey".into(), false);
        assert!(g.wizard_wants_lab(), "no laboratory yet");
        // Nothing to give: the shortage goes to the overseer's troubles.
        g.items.retain(|_, it| !gem(it) && it.kind != ItemKind::Bar);
        g.tick = SEASON;
        g.wizard_daily();
        let w = g.magic.wizard.clone().unwrap();
        assert!(w.assistant.is_some(), "no assistant chosen");
        assert!(g.wizard_lines(w.assistant.unwrap())[0].starts_with("Assistant to Urist Lamp"));
        assert!(w.want.is_some(), "asked for nothing");
        assert!(g.troubles.iter().any(|t| matches!(t.0, troubles::Lack::Gem | troubles::Lack::Bars)));
        // A laboratory: it is claimed.
        let e = g.embark;
        g.add_zone(e.offset(-2, -2, 0), e.offset(2, 2, 0), ZoneKind::Laboratory);
        g.tick += DAY;
        g.wizard_daily();
        let w = g.magic.wizard.clone().unwrap();
        assert!(w.lab != 0 && !g.wizard_wants_lab());
        assert!(g.log.iter().any(|l| l.text.contains("has claimed")));
        // Gems in store are taken up to it.
        g.spawn_item(ItemKind::Gem, crate::material::Mat::Granite, e);
        g.spawn_item(ItemKind::Bar, crate::material::Mat::Copper, e);
        g.tick += SEASON;
        g.wizard_daily();
        assert!(g.log.iter().any(|l| l.text.contains("up to the laboratory for their studies")));
        assert!(g.magic.wizard.as_ref().unwrap().want.is_none());
    }

    #[test]
    fn the_wizard_hurls_witchfire_at_the_dead_and_lays_ghosts() {
        let mut g = big_fort();
        g.wizard_asks("Urist Lamp".into(), "the Grey".into(), false);
        let wid = g.magic.wizard.as_ref().unwrap().dwarf;
        let e = g.embark;
        let wi = g.dwarves.iter().position(|d| d.id == wid).unwrap();
        g.dwarves[wi].pos = e;
        g.dwarves[wi].asleep = false;
        let dead = g.spawn_creature(Species::Undead, e.offset(4, 0, 0), u64::MAX);
        let hp0: i32 = g.creatures[&dead].body.hp.iter().sum();
        g.tick = 1000;
        g.wizard_tick();
        let hp1: i32 = g.creatures[&dead].body.hp.iter().sum();
        assert!(hp1 < hp0, "no witchfire");
        assert!(g.log.iter().any(|l| l.text == "Urist the Wizard hurls witchfire at the walking corpse!" || l.text.starts_with("Urist the Wizard hurls witchfire")));
        assert!(g.wizard_sight().unwrap().2, "the director wants to see it");
        let seen = crate::director::candidates(&g);
        assert!(seen.iter().any(|s| s.kind == crate::director::Kind::Magic && s.what.contains("hurls witchfire")), "the camera never looks");
        // Not again at once.
        g.wizard_tick();
        assert_eq!(g.magic.wizard.as_ref().unwrap().spells, 1);
        // A ghost is laid.
        g.creatures.remove(&dead);
        let gi = (0..g.dwarves.len()).find(|&i| i != wi).unwrap();
        g.dwarves[gi].alive = false;
        g.dwarves[gi].ghost = true;
        g.dwarves[gi].pos = e.offset(2, 0, 0);
        g.tick += 1000;
        g.wizard_tick();
        assert!(!g.dwarves[gi].ghost && g.dwarves[gi].rested);
        assert!(g.log.iter().any(|l| l.text.contains("is banished")));
    }

    #[test]
    fn a_wizard_comes_now_and_then_reads_omens_and_gets_a_tower() {
        let mut g = big_fort();
        g.set_overseer(Some(Style::Steady));
        g.overseer.as_mut().unwrap().traits = vec![Trait::Pious];
        let mut came = None;
        for i in 0..24 {
            g.tick = YEAR + i * SEASON;
            g.wizard_season();
            if g.magic.wizard.is_some() {
                came = Some(i);
                break;
            }
        }
        assert!(came.is_some(), "no wizard in six years");
        assert!(g.log.iter().any(|l| l.text.contains("asks to settle")));
        // The wizard reads an omen aloud.
        g.see_omen(super::super::omens::OmenKind::BloodMoon, g.tick + 3 * DAY);
        assert!(g.log.iter().any(|l| l.text.contains("the Wizard reads the omen: the dead will walk, within 3 days")));
        // No laboratory: the overseer notes it, and wants a tower for them.
        let mut o = g.overseer.take().unwrap();
        o.traits.clear();
        g.ov_wizard(&mut o);
        assert!(o.notes.iter().any(|n| n.1.contains("wants a laboratory")));
        assert!(g.wizard_due(&o), "no tower for the wizard");
        g.overseer = Some(o);
        // The status screen tells of the omen and the wizard.
        let status = g.magic_status_lines();
        assert!(status.iter().any(|l| l.starts_with("An omen hangs over the fort: a blood moon")), "{:?}", status);
        assert!(status.iter().any(|l| l.starts_with("The wizard ") && l.contains("wanting a laboratory")), "{:?}", status);
        assert!(status.iter().any(|l| l.starts_with("Most here worship")), "{:?}", status);
    }

    #[test]
    fn the_laboratory_has_its_accidents() {
        let mut g = big_fort();
        g.wizard_asks("Urist Lamp".into(), "the Grey".into(), false);
        let e = g.embark;
        g.add_zone(e.offset(-2, -2, 0), e.offset(2, 2, 0), ZoneKind::Laboratory);
        g.wizard_daily();
        let wid = g.magic.wizard.as_ref().unwrap().dwarf;
        let lab = g.magic.wizard.as_ref().unwrap().lab;
        let tile = g.zones[&lab].tiles[0];
        for _ in 0..200 {
            g.tick += DAY;
            let wi = g.dwarves.iter().position(|d| d.id == wid).unwrap();
            g.dwarves[wi].pos = tile;
            g.wizard_daily();
        }
        let w = g.magic.wizard.as_ref().unwrap();
        assert!(w.accidents >= 2 && w.accidents <= 40, "{} accidents in 200 days", w.accidents);
        assert!(g.magic.year.iter().any(|l| l.contains("laboratory") || l.contains("workbench")));
    }
}
