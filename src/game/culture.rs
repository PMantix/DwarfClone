//! A culture of their own: the fortress plays its people's instruments,
//! dances its dances and recites its poetry (the forms themselves are the
//! world's, `arts.rs`). Craftsdwarves make the instruments; whoever is in
//! the tavern with one to hand strikes up a tune, and at a festival a
//! musician plays while the fortress dances.
//!
//! Everything here draws on a random stream of its own, so the rest of
//! the fortress's story doesn't shift for it.

use super::overseer::Overseer;
use super::*;
use crate::arts::{CivArts, FormKind, Instrument};
use crate::dwarf::NeedKind;

/// The fortress's arts and what's become of them.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Culture {
    /// Its own random stream (seeded on first use).
    #[serde(default)]
    pub(super) rng: Option<Rng>,
    /// A performance going on now, for the director: where, what, until when.
    #[serde(default)]
    pub stage: Option<(Pos, String, u64)>,
    /// When a tune in the tavern may next be announced (once a day at most).
    #[serde(default)]
    next_tune: u64,
    /// The festival (by its end tick) that has already had its music.
    #[serde(default)]
    festival_music: u64,
    /// Instruments made so far (to take turns among the people's kinds).
    #[serde(default)]
    pub instruments_made: u32,
    /// The works composed in the fortress (`works.rs`).
    #[serde(default)]
    pub works: Vec<super::works::Work>,
    /// The world's events the culture wrote itself (compositions, new forms).
    #[serde(default)]
    pub own: Vec<usize>,
    /// The day last seen to (daily work runs once a day).
    #[serde(default)]
    day: u64,
    /// How many of the world's events have been looked at for witnesses.
    #[serde(default)]
    seen: usize,
    /// The fortress's legends and who was there: (event index, dwarves alive then).
    #[serde(default)]
    pub memories: Vec<(usize, Vec<u32>)>,
    /// What the fortress's art shows, by item or building id (`depict.rs`).
    #[serde(default)]
    pub art: BTreeMap<u32, super::depict::Depiction>,
}

impl Game {
    /// A number in 0..1 from the culture's own stream.
    pub(super) fn c_roll(&mut self) -> f64 {
        self.c_rng().f64()
    }

    /// An index in 0..n from the culture's own stream.
    pub(super) fn c_pick(&mut self, n: usize) -> usize {
        self.c_rng().range(0, n.max(1) as i32) as usize
    }

    pub(super) fn c_rng(&mut self) -> &mut Rng {
        if self.culture.rng.is_none() {
            let seed = self.overworld.seed ^ (self.site.0 as u64) << 16 ^ (self.site.1 as u64) << 32 ^ 0xC017_0BE5;
            let seed = self.fortress_name.bytes().fold(seed, |h, b| h.rotate_left(5) ^ b as u64);
            self.culture.rng = Some(Rng::new(seed));
        }
        self.culture.rng.as_mut().unwrap()
    }

    /// The arts of the fortress's people (a stand-in of their own for a
    /// fortress with no civilization behind it).
    pub fn our_arts(&self) -> CivArts {
        let a = self.civ.map(|c| self.overworld.civ_arts(c)).unwrap_or_default();
        if !a.instruments.is_empty() {
            return a;
        }
        let mut rng = Rng::new(self.fortress_name.bytes().fold(7, |h, b| h.wrapping_mul(31) ^ b as u64));
        let civ = self.civ.unwrap_or(u32::MAX);
        let mut a = CivArts::default();
        a.instruments.push(crate::arts::new_instrument(&mut rng, Race::Dwarf, &[]));
        for k in [FormKind::Poetry, FormKind::Music, FormKind::Dance] {
            let f = crate::arts::new_form(&mut rng, Race::Dwarf, civ, k, &a.instruments);
            a.forms.push(f);
        }
        a
    }

    /// The kind of instrument an item is, by its name.
    pub fn instrument_of(&self, it: &Item) -> Option<Instrument> {
        if it.kind != ItemKind::Instrument {
            return None;
        }
        let name = it.label.as_deref()?;
        if let Some(i) = self.our_arts().instruments.into_iter().find(|i| i.name == name) {
            return Some(i);
        }
        self.overworld.civs.iter().flat_map(|c| crate::arts::native_arts(&self.overworld, c.id).instruments).find(|i| i.name == name)
    }

    /// What the look panel says of an item's art: an instrument's making
    /// and sound.
    pub fn art_lines(&self, id: u32) -> Vec<String> {
        let mut v = Vec::new();
        if let Some(i) = self.items.get(&id).and_then(|it| self.instrument_of(it)) {
            v.push(i.describe());
        }
        v.extend(self.depiction_lines(id));
        v
    }

    /// Something has just been made at a workshop: an instrument takes the
    /// name of one of its people's kinds.
    pub(super) fn culture_crafted(&mut self, d: &Dwarf, ids: &[u32]) {
        for &id in ids {
            self.depict_made(d, id);
            if self.items.get(&id).map_or(true, |it| it.kind != ItemKind::Instrument || it.label.is_some()) {
                continue;
            }
            let arts = self.our_arts();
            let n = self.culture.instruments_made as usize;
            self.culture.instruments_made += 1;
            let Some(i) = arts.instruments.get(n % arts.instruments.len().max(1)) else { continue };
            self.items.get_mut(&id).unwrap().label = Some(i.name.clone());
            if n == 0 {
                self.msg(&format!("The fortress has its first {}: {}", i.name, i.describe()), (200, 180, 255));
            }
        }
    }

    /// Instruments lying free for anyone to play.
    fn free_instruments(&self) -> Vec<u32> {
        self.items.values().filter(|it| it.kind == ItemKind::Instrument && it.free() && it.label.is_some()).map(|it| it.id).collect()
    }

    /// Now and then (every 100 ticks): music in the tavern and at a festival.
    pub(super) fn culture_tick(&mut self) {
        if self.culture.stage.as_ref().map_or(false, |s| self.tick >= s.2) {
            self.culture.stage = None;
        }
        self.remember();
        let day = self.tick / DAY + 1;
        if day != self.culture.day {
            self.culture.day = day;
            self.works_daily();
        }
        self.festival_music();
        self.tavern_music();
        self.admire_art();
    }

    /// New legends of the fortress, and who was alive in it to see them.
    pub(super) fn remember(&mut self) {
        let n = self.overworld.events.len();
        if self.culture.seen >= n {
            self.culture.seen = self.culture.seen.min(n);
            return;
        }
        let fort = self.fortress_name.clone();
        let alive: Vec<u32> = self.dwarves.iter().filter(|d| d.alive && !d.ghost).map(|d| d.id).collect();
        for i in self.culture.seen..n {
            if self.overworld.events[i].text.contains(&fort) && !self.culture.own.contains(&i) {
                self.culture.memories.push((i, alive.clone()));
            }
        }
        self.culture.seen = n;
    }

    /// The fortress's legends a dwarf lived through.
    pub fn lived_through(&self, id: u32) -> Vec<usize> {
        self.culture.memories.iter().filter(|m| m.1.contains(&id)).map(|m| m.0).collect()
    }

    /// Someone in the tavern with an instrument to hand strikes up a tune;
    /// everyone there enjoys it.
    pub(super) fn tavern_music(&mut self) {
        let instruments = self.free_instruments();
        if instruments.is_empty() || self.festival.is_some() {
            return;
        }
        let here: Vec<usize> = (0..self.dwarves.len())
            .filter(|&i| {
                let d = &self.dwarves[i];
                d.alive && !d.asleep && !d.ghost && !d.insane && d.tantrum == 0 && !d.is_child() && self.zone_kind_at(d.pos) == Some(ZoneKind::Tavern)
            })
            .collect();
        if here.is_empty() || self.c_roll() >= self.knob("culture.tune", 0.35) {
            return;
        }
        // The most artistic of them plays, more often than not.
        let mut by_art = here.clone();
        by_art.sort_by_key(|&i| (-self.dwarves[i].personality.artistry, self.dwarves[i].id));
        let player = if self.c_roll() < 0.6 { by_art[0] } else { here[self.c_pick(here.len())] };
        let inst = instruments[self.c_pick(instruments.len())];
        let Some(kind) = self.items.get(&inst).and_then(|it| self.instrument_of(it)) else { return };
        let tick = self.tick;
        let pname = self.dwarves[player].name.clone();
        // One of the fortress's own works, now and then.
        let work = if self.c_roll() < 0.5 { self.pick_work(None) } else { None };
        let piece = work.map_or(String::new(), |w| format!(" \"{}\"", self.culture.works[w].title));
        for &i in &here {
            let d = &mut self.dwarves[i];
            if i == player {
                d.think(&format!("played the {} in the tavern", kind.name), -5, tick);
                d.satisfy(NeedKind::Party, 250);
            } else if !d.thoughts.iter().any(|t| t.text.starts_with("enjoyed music") && tick < t.tick + DAY) {
                d.think(&format!("enjoyed music on the {}", kind.name), -4, tick);
                d.satisfy(NeedKind::Party, 150);
            }
        }
        self.count("culture.tunes", 1);
        if let Some(w) = work {
            self.performed(w);
        }
        let at = self.dwarves[player].pos;
        self.culture.stage = Some((at, format!("{} playing{} on the {} in the tavern", pname, piece, kind.name), tick + DAY / 8));
        if tick >= self.culture.next_tune {
            self.culture.next_tune = tick + DAY;
            self.msg(&format!("{} plays{} on the {} in the tavern: {}.", pname, piece, kind.name, kind.sound), (200, 180, 255));
        }
    }

    /// Once a festival is under way: a musician plays and the fortress
    /// dances its people's dance (or, with no instrument, sings its poetry).
    pub(super) fn festival_music(&mut self) {
        let Some(f) = self.festival.as_ref() else { return };
        if self.culture.festival_music == f.until || f.attended.len() < 2 {
            return;
        }
        let (until, kind, attended, venue) = (f.until, f.kind, f.attended.clone(), f.venue.clone());
        self.culture.festival_music = until;
        let arts = self.our_arts();
        let instruments = self.free_instruments();
        let players: Vec<u32> = attended.iter().copied().filter(|id| self.dwarves.iter().any(|d| d.id == *id && d.alive && !d.is_child())).collect();
        let Some(&player) = players.iter().max_by_key(|id| self.dwarves.iter().find(|d| d.id == **id).map_or(-99, |d| d.personality.artistry)) else { return };
        let pname = self.dwarves.iter().find(|d| d.id == player).map(|d| d.name.clone()).unwrap_or_default();
        let fest = kind.name();
        // A dance composed in the fortress, if there is one.
        if let Some(w) = self.pick_work(Some(FormKind::Dance)) {
            let wk = self.culture.works[w].clone();
            let tick = self.tick;
            for d in self.dwarves.iter_mut().filter(|d| d.alive && attended.contains(&d.id)) {
                d.think(&format!("danced \"{}\" at the {}", wk.title, fest), -6, tick);
                d.satisfy(NeedKind::Party, 200);
            }
            self.performed(w);
            self.count("culture.dances", 1);
            if let Some(at) = self.dwarves.iter().find(|d| d.id == player).map(|d| d.pos).or(venue.first().copied()) {
                self.culture.stage = Some((at, format!("the {}: \"{}\"", fest, wk.title), until));
            }
            self.msg(&format!("At the {}, {} leads the fortress in \"{}\", a {} by {}.", fest, pname, wk.title, wk.form, wk.author), (255, 215, 60));
            return;
        }
        let (text, thought) = match (instruments.first().and_then(|id| self.items.get(id)).and_then(|it| self.instrument_of(it)), arts.of_kind(FormKind::Dance).first()) {
            (Some(i), Some(dance)) => (
                format!("At the {}, {} strikes up the {} and the fortress dances the {}, {}.", fest, pname, i.name, dance.name, dance.desc),
                format!("danced the {} at the {}", dance.name, fest),
            ),
            (None, Some(dance)) => (
                format!("At the {}, the fortress claps out the time and dances the {}, {}.", fest, dance.name, dance.desc),
                format!("danced the {} at the {}", dance.name, fest),
            ),
            _ => match arts.of_kind(FormKind::Poetry).first() {
                Some(p) => (format!("At the {}, the fortress recites the {}, {}.", fest, p.name, p.desc), format!("recited the {} at the {}", p.name, fest)),
                None => return,
            },
        };
        let tick = self.tick;
        for d in self.dwarves.iter_mut().filter(|d| d.alive && attended.contains(&d.id)) {
            d.think(&thought, -5, tick);
            d.satisfy(NeedKind::Party, 200);
        }
        self.count("culture.dances", 1);
        let at = self.dwarves.iter().find(|d| d.id == player).map(|d| d.pos).or(venue.first().copied());
        if let Some(at) = at {
            self.culture.stage = Some((at, format!("the {}: {} dancing", fest, attended.len()), until));
        }
        self.msg(&text, (255, 215, 60));
    }

    /// The overseer, daily: once there's a tavern and a craftsdwarf's
    /// workshop, it keeps a few of its people's instruments in stock.
    pub(super) fn ov_culture(&mut self, o: &mut Overseer) {
        let tavern = self.zones.values().any(|z| z.kind == ZoneKind::Tavern);
        let shop = self.buildings.values().any(|b| b.built && b.kind == BuildingKind::Craftsdwarf);
        if !tavern || !shop || self.stock_of(ItemKind::Log) < 2 {
            return;
        }
        let keep = (1 + self.alive_count() as u32 / 12).min(4);
        let new = !self.work_orders.iter().any(|w| w.recipe == Recipe::Instrument);
        self.add_work_order(Recipe::Instrument, keep);
        if new {
            let arts = self.our_arts();
            let names: Vec<String> = arts.instruments.iter().map(|i| format!("the {} ({})", i.name, i.kind)).collect();
            Self::note_ov(o, self.tick, format!("Ordered instruments for the tavern: {}.", names.join(", ")));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::festivals::FestivalKind;

    fn tavern(g: &mut Game) -> Vec<Pos> {
        let e = g.embark;
        assert!(g.add_zone(e.offset(-3, -3, 0), e.offset(3, 3, 0), ZoneKind::Tavern) > 0);
        g.zones.values().filter(|z| z.kind == ZoneKind::Tavern).flat_map(|z| z.tiles.iter().copied()).filter(|&p| g.world.walkable(p)).collect()
    }

    #[test]
    fn a_craftsdwarf_makes_an_instrument_of_the_fortresss_people() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let arts = g.our_arts();
        assert!(!arts.instruments.is_empty());
        let e = g.embark;
        for _ in 0..4 {
            g.spawn_item(ItemKind::Log, Mat::Wood, e);
            g.spawn_item(ItemKind::Boulder, Mat::Granite, e);
        }
        let shop = (2..25)
            .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).flat_map(move |dx| (-1..=1).map(move |dz| (dx, dy, dz)))))
            .find_map(|(dx, dy, dz)| {
                let p = e.offset(dx, dy, dz);
                (g.can_place(BuildingKind::Craftsdwarf, p) && g.place_building(BuildingKind::Craftsdwarf, p)).then(|| g.world.t(p).building)
            })
            .expect("no room for a workshop");
        for d in g.dwarves.iter_mut() {
            d.labors[Labor::Crafting.idx()] = true;
        }
        for _ in 0..DAY * 3 {
            g.step();
            if g.buildings[&shop].built {
                break;
            }
        }
        assert!(g.buildings[&shop].built);
        g.add_order(shop, Recipe::Instrument);
        for _ in 0..DAY * 3 {
            g.step();
            if g.items.values().any(|it| it.kind == ItemKind::Instrument) {
                break;
            }
        }
        let it = g.items.values().find(|it| it.kind == ItemKind::Instrument).expect("no instrument made").clone();
        let kind = g.instrument_of(&it).expect("an instrument of a known kind");
        assert_eq!(kind, arts.instruments[0]);
        assert_eq!(it.name().split(' ').last(), Some(kind.name.as_str()));
        let lines = g.art_lines(it.id);
        assert!(lines[0].contains(&kind.sound), "{:?}", lines);
        assert!(g.log.iter().any(|l| l.text.contains("its first")));
        println!("{}: {}", it.name(), lines[0]);
    }

    #[test]
    fn music_in_the_tavern_and_a_dance_at_the_feast() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let tiles = tavern(&mut g);
        let e = g.embark;
        let inst = g.spawn_item(ItemKind::Instrument, Mat::Wood, e);
        let name = g.our_arts().instruments[0].name.clone();
        g.items.get_mut(&inst).unwrap().label = Some(name.clone());
        for (i, d) in g.dwarves.iter_mut().enumerate() {
            d.pos = tiles[i % tiles.len()];
            d.asleep = false;
        }
        for _ in 0..40 {
            g.tavern_music();
        }
        assert!(g.log.iter().any(|l| l.text.contains(&format!("on the {} in the tavern", name))));
        assert!(g.dwarves.iter().any(|d| d.thoughts.iter().any(|t| t.text == format!("enjoyed music on the {}", name))));
        assert!(g.culture.stage.as_ref().map_or(false, |s| s.1.contains("playing")), "the director has a stage to visit");
        // A feast: the musician plays and everyone dances the people's dance.
        let dance = g.our_arts().of_kind(FormKind::Dance)[0].name.clone();
        g.start_festival(FestivalKind::Harvest);
        let ids: Vec<u32> = g.dwarves.iter().map(|d| d.id).collect();
        g.festival.as_mut().unwrap().attended = ids;
        g.festival_music();
        let line = g.log.iter().find(|l| l.text.contains("dances the")).expect("no dance").text.clone();
        assert!(line.contains(&dance) && line.contains(&name), "{}", line);
        assert!(g.dwarves.iter().all(|d| d.thoughts.iter().any(|t| t.text == format!("danced the {} at the harvest feast", dance))));
        // Only once a festival.
        let n = g.log.len();
        g.festival_music();
        assert_eq!(g.log.len(), n);
        println!("{}", line);
    }

    #[test]
    fn the_fortress_plays_and_dances_its_own_works() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let tiles = tavern(&mut g);
        let e = g.embark;
        let inst = g.spawn_item(ItemKind::Instrument, Mat::Wood, e);
        g.items.get_mut(&inst).unwrap().label = Some(g.our_arts().instruments[0].name.clone());
        for (i, d) in g.dwarves.iter_mut().enumerate() {
            d.pos = tiles[i % tiles.len()];
            d.asleep = false;
        }
        let author = g.dwarves[0].id;
        let year = g.start_year;
        let work = |title: &str, kind: FormKind| super::super::works::Work {
            title: title.into(),
            form: "zanost".into(),
            kind,
            author: "Urist".into(),
            author_id: author,
            year,
            about: "how it began".into(),
            performed: 0,
        };
        g.culture.works.push(work("The Air of Stone", FormKind::Music));
        g.culture.works.push(work("The Reel of Stone", FormKind::Dance));
        for _ in 0..60 {
            g.tavern_music();
        }
        assert!(g.culture.works[0].performed > 0, "the tune was never played");
        assert_eq!(g.culture.works[1].performed, 0, "a dance isn't played in the tavern");
        assert!(g.log.iter().any(|l| l.text.contains("plays \"The Air of Stone\" on the")));
        assert!(g.dwarves[0].thoughts.iter().any(|t| t.text.starts_with("was glad to hear their")));
        g.start_festival(FestivalKind::Harvest);
        let ids: Vec<u32> = g.dwarves.iter().map(|d| d.id).collect();
        g.festival.as_mut().unwrap().attended = ids;
        g.festival_music();
        assert_eq!(g.culture.works[1].performed, 1);
        assert!(g.log.iter().any(|l| l.text.contains("leads the fortress in \"The Reel of Stone\"")));
    }

    #[test]
    fn the_overseer_orders_instruments_for_its_tavern() {
        let mut g = Game::new(81);
        g.set_overseer(Some(Style::Steady));
        let mut o = g.overseer.take().unwrap();
        g.ov_culture(&mut o);
        assert!(!g.work_orders.iter().any(|w| w.recipe == Recipe::Instrument), "no tavern yet");
        tavern(&mut g);
        let e = g.embark;
        for _ in 0..4 {
            g.spawn_item(ItemKind::Log, Mat::Wood, e);
        }
        let p = (2..25)
            .flat_map(|r| (-r..=r).flat_map(move |dy| (-r..=r).map(move |dx| (dx, dy))))
            .map(|(dx, dy)| e.offset(dx, dy, 0))
            .find(|&p| g.can_place(BuildingKind::Craftsdwarf, p))
            .unwrap();
        assert!(g.place_building(BuildingKind::Craftsdwarf, p));
        let id = g.world.t(p).building;
        g.buildings.get_mut(&id).unwrap().built = true;
        g.ov_culture(&mut o);
        assert!(g.work_orders.iter().any(|w| w.recipe == Recipe::Instrument && w.keep >= 1));
        assert!(o.notes.iter().any(|n| n.1.starts_with("Ordered instruments for the tavern")));
    }
}
