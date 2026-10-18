//! Art that tells: statues, crafts and engravings show real events and
//! figures from the fortress's history and the world's legends. The look
//! panel says what a piece shows, and a dwarf who lived through what it
//! shows stops to admire it.
//!
//! Every roll is on the culture's own stream (`culture.rs`).

use super::*;

/// What a piece of art shows.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Depiction {
    /// The legend it shows (an index into the world's events), if any.
    #[serde(default)]
    pub event: Option<usize>,
    /// The dwarf of the fortress it shows, if any.
    #[serde(default)]
    pub dwarf: Option<u32>,
    /// What it shows, for the look panel ("how the goblins came against Ironhold").
    pub text: String,
    /// Who made it.
    pub maker: String,
}

/// Item kinds that can show something.
pub fn tells(k: ItemKind) -> bool {
    matches!(k, ItemKind::Statue | ItemKind::Craft)
}

impl Game {
    /// A piece of art has just been made: perhaps it shows something real.
    pub(super) fn depict_made(&mut self, d: &Dwarf, id: u32) {
        let Some(kind) = self.items.get(&id).map(|it| it.kind).filter(|k| tells(*k)) else { return };
        let p = if kind == ItemKind::Statue { self.knob("culture.statue_tells", 0.85) } else { self.knob("culture.craft_tells", 0.4) };
        if self.culture.art.contains_key(&id) || self.c_roll() >= p {
            return;
        }
        let fort = self.fortress_name.clone();
        // A hero of the fortress, now and then; most often a legend the maker lived through.
        let heroes: Vec<(u32, String)> = self
            .dwarves
            .iter()
            .filter(|h| h.epithet.is_some() || (!h.alive && h.died.is_some() && d.relations.iter().any(|r| r.0 == h.id && r.1 > 0)))
            .map(|h| (h.id, h.called()))
            .collect();
        let lived = self.lived_through(d.id);
        let ours = self.fort_events();
        let dep = if !heroes.is_empty() && self.c_roll() < 0.25 {
            let (hid, name) = heroes[self.c_pick(heroes.len())].clone();
            let h = self.dwarves.iter().find(|h| h.id == hid).unwrap();
            let what = match (&h.deed, h.alive) {
                (Some(deed), _) => format!("{}, who {}", name, deed),
                (None, false) => format!("{}, mourned by the fortress", name),
                (None, true) => name.clone(),
            };
            Depiction { event: None, dwarf: Some(hid), text: format!("an image of {}", what), maker: d.name.clone() }
        } else {
            let pool = if !lived.is_empty() && self.c_roll() < 0.75 { lived } else { ours };
            let e = if !pool.is_empty() {
                pool[self.c_pick(pool.len())]
            } else if !self.overworld.events.is_empty() {
                self.c_pick(self.overworld.events.len())
            } else {
                return;
            };
            let (_, how) = super::works::subject_of(&self.overworld.events[e].text, &fort);
            Depiction { event: Some(e), dwarf: None, text: format!("an image of how {}", how), maker: d.name.clone() }
        };
        self.count("culture.depictions", 1);
        if kind == ItemKind::Statue {
            let what = self.items[&id].name();
            self.msg(&format!("{} has carved a {} showing {}.", d.name, what, dep.text.trim_start_matches("an image of ")), (200, 180, 255));
        }
        self.culture.art.insert(id, dep);
    }

    /// A piece of art becomes something else (an item set up as a building,
    /// or a building taken down): what it shows goes with it.
    pub(super) fn art_moved(&mut self, from: u32, to: u32) {
        if let Some(dep) = self.culture.art.remove(&from) {
            self.culture.art.insert(to, dep);
        }
    }

    /// The look panel's lines on what a piece shows.
    pub fn depiction_lines(&self, id: u32) -> Vec<String> {
        let Some(dep) = self.culture.art.get(&id) else { return Vec::new() };
        let year = dep.event.and_then(|e| self.overworld.events.get(e)).map_or(String::new(), |e| format!(" (year {})", e.year));
        let mut v = vec![format!("It shows {}{}. Made by {}.", dep.text, year, dep.maker)];
        let n = self.dwarves.iter().filter(|d| d.alive && dep.event.map_or(false, |e| self.lived_through(d.id).contains(&e))).count();
        if n > 0 {
            v.push(format!("{} of the fortress lived through it.", n));
        }
        v
    }

    /// Every now and then: a dwarf beside a piece of art about something
    /// they lived through (or showing them) stops to admire it.
    pub(super) fn admire_art(&mut self) {
        // Where the telling art is: items lying free and buildings standing.
        let mut pieces: Vec<(Pos, Option<usize>, Option<u32>, String)> = Vec::new();
        for (&id, dep) in &self.culture.art {
            let at = match (self.items.get(&id), self.buildings.get(&id)) {
                (Some(it), _) if it.carried_by.is_none() && it.container.is_none() => Some((it.pos, it.kind.plural().trim_end_matches('s').to_string())),
                (_, Some(b)) if b.built => Some((b.center(), b.kind.name().to_lowercase())),
                _ => None,
            };
            if let Some((p, what)) = at {
                pieces.push((p, dep.event, dep.dwarf, what));
            }
        }
        let tick = self.tick;
        for i in 0..self.dwarves.len() {
            let d = &self.dwarves[i];
            if !d.alive || d.asleep || d.ghost || d.insane || d.is_child() || d.thoughts.iter().any(|t| t.text.starts_with("admired ") && tick < t.tick + DAY) {
                continue;
            }
            let lived = self.lived_through(d.id);
            let (pos, id) = (d.pos, d.id);
            let near = |p: Pos| p.z == pos.z && (p.x - pos.x).abs() <= 3 && (p.y - pos.y).abs() <= 3;
            let mut found: Option<(String, bool)> = None;
            for (p, ev, who, what) in &pieces {
                if !near(*p) {
                    continue;
                }
                if *who == Some(id) {
                    found = Some((format!("admired a {} of themselves", what), true));
                    break;
                }
                if ev.map_or(false, |e| lived.contains(&e)) {
                    found = Some((format!("admired a {} of something they lived through", what), false));
                }
            }
            // An engraving close by.
            if found.is_none() {
                'eng: for dy in -2..=2 {
                    for dx in -2..=2 {
                        let q = pos.offset(dx, dy, 0);
                        let Some(t) = self.world.get(q) else { continue };
                        if t.smooth >= super::detail::ENGRAVED && t.engraving > 0 && lived.contains(&(t.engraving as usize - 1)) {
                            found = Some(("admired an engraving of something they lived through".to_string(), false));
                            break 'eng;
                        }
                    }
                }
            }
            if let Some((text, own)) = found {
                let d = &mut self.dwarves[i];
                d.think(&text, if own { -8 } else { -4 }, tick);
                self.count("culture.admired", 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_statue_shows_a_legend_its_maker_lived_through_and_they_admire_it() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let fort = g.fortress_name.clone();
        g.legend(format!("A great cave spider was slain at {}.", fort));
        g.remember();
        let ev = g.overworld.events.len() - 1;
        let maker = g.dwarves[0].clone();
        assert!(g.lived_through(maker.id).contains(&ev));
        // A newcomer arriving after never lived through it.
        let late = g.dwarves.len() - 1;
        let lid = g.dwarves[late].id;
        g.culture.memories.iter_mut().for_each(|m| m.1.retain(|&id| id != lid));
        g.knobs.set.insert("culture.statue_tells".into(), 1.0);
        let mut shown = false;
        for _ in 0..20 {
            let id = g.spawn_item(ItemKind::Statue, Mat::Granite, g.embark);
            g.depict_made(&maker, id);
            if g.culture.art.get(&id).map_or(false, |d| d.event == Some(ev)) {
                shown = true;
                let lines = g.depiction_lines(id);
                assert!(lines[0].contains("cave spider was slain") && lines[0].contains(&maker.name), "{:?}", lines);
                // Set up as a statue, it still shows it.
                let p = g.embark.offset(1, 0, 0);
                let b = g.place_building(BuildingKind::Statue, p).then(|| g.world.t(p).building);
                if let Some(b) = b {
                    g.art_moved(id, b);
                    g.items.remove(&id);
                    g.buildings.get_mut(&b).unwrap().built = true;
                    assert_eq!(g.depiction_lines(b), lines);
                }
                break;
            }
        }
        assert!(shown, "no statue ever showed the legend");
        // Everyone stands beside it; only those who were there admire it.
        let at = g.culture.art.keys().map(|&k| g.items.get(&k).map(|i| i.pos).or(g.buildings.get(&k).map(|b| b.center())).unwrap()).next().unwrap();
        for d in g.dwarves.iter_mut() {
            d.pos = at;
            d.asleep = false;
        }
        g.admire_art();
        assert!(g.dwarves[0].thoughts.iter().any(|t| t.text.contains("something they lived through")));
        assert!(!g.dwarves[late].thoughts.iter().any(|t| t.text.starts_with("admired")), "the newcomer wasn't there");
        // Once a day.
        let n = g.annals.counters.get("culture.admired").copied().unwrap_or(0);
        g.admire_art();
        assert_eq!(g.annals.counters.get("culture.admired").copied().unwrap_or(0), n);
    }

    #[test]
    fn an_engraving_of_a_siege_is_admired_by_those_who_held_the_walls() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let fort = g.fortress_name.clone();
        g.legend(format!("Snodub led the Vile Host in a siege of {}.", fort));
        g.remember();
        let ev = g.overworld.events.len() - 1;
        let p = g.dwarves[0].pos.offset(1, 0, 0);
        {
            let t = g.world.t_mut(p);
            t.smooth = super::super::detail::ENGRAVED + 2;
            t.engraving = ev as u32 + 1;
        }
        g.dwarves[0].asleep = false;
        g.admire_art();
        assert!(g.dwarves[0].thoughts.iter().any(|t| t.text == "admired an engraving of something they lived through"));
    }

    #[test]
    fn a_craft_may_show_a_hero_of_the_fortress() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        g.dwarves[1].epithet = Some("Spiderbane".into());
        g.dwarves[1].deed = Some("slew a great cave spider".into());
        g.knobs.set.insert("culture.craft_tells".into(), 1.0);
        let maker = g.dwarves[0].clone();
        let hero = g.dwarves[1].id;
        let mut seen = false;
        for _ in 0..40 {
            let id = g.spawn_item(ItemKind::Craft, Mat::Granite, g.embark);
            g.depict_made(&maker, id);
            if let Some(d) = g.culture.art.get(&id).filter(|d| d.dwarf == Some(hero)) {
                assert!(d.text.contains("slew a great cave spider"), "{}", d.text);
                seen = true;
                break;
            }
        }
        assert!(seen);
    }
}
