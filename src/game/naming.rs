//! Halls named for heroes and deeds (O03 step 4, ENG-16). A great hall the
//! overseer finishes is named for the fort's greatest hero ("the Hall of
//! Kelda the Dragonslayer"), or a noble, or a name of its own; a great
//! deed done inside it renames it, if the deed is heavier than the one it
//! was named for: a siege leader slain there ("the Hall of the Unbeaten"),
//! a beast slain ("the Hall of the Dragon's Fall"), three dead in a day
//! ("the Weeping Hall"), a wedding ("the Hall of Vows"). The gate takes a
//! name from the first siege that breaks on it. Every renaming is
//! announced, written in the legends and the year's chronicle, and the
//! look panel tells the name and why (`stories.rs: story_place_at`).
//!
//! The names live in `places.rs` (`name_place`); the overseer's room
//! titles ("the Great Dining Hall", `architect.rs: place_name`) are left
//! as they are, since battle names are built from them. Names come from
//! hashes, never the game's random numbers.

use super::chronicle::YearLog;
use super::overseer::{Overseer, RoomUse};
use super::places::PlaceKind;
use super::stories::{first_name, hash};
use super::*;

/// What was done at a place, for `deed_at`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeedKind {
    /// A siege broke there.
    SiegeHeld,
    /// The leader of a siege was slain there.
    LeaderSlain,
    /// A beast of the legends was slain there.
    BeastSlain,
    /// Many of ours were slaughtered there.
    Massacre,
    /// Two dwarves were wed there.
    Wedding,
    /// One of ours died there (three in a day name it).
    Death,
    /// It was finished: a great hall wants a name.
    Work,
}

/// A hall a name can hold to: its box, what to call it ("Hall",
/// "Temple") and whether it is one of the fort's great halls.
struct Hall {
    a: Pos,
    b: Pos,
    noun: &'static str,
    great: bool,
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

fn list(names: &[String]) -> String {
    match names.len() {
        0 => String::new(),
        1 => names[0].clone(),
        n => format!("{} and {}", names[..n - 1].join(", "), names[n - 1]),
    }
}

fn in_box(a: Pos, b: Pos, p: Pos) -> bool {
    p.x >= a.x.min(b.x) && p.x <= a.x.max(b.x) && p.y >= a.y.min(b.y) && p.y <= a.y.max(b.y) && p.z >= a.z.min(b.z) && p.z <= a.z.max(b.z)
}

fn room_noun(k: RoomUse) -> Option<(&'static str, bool)> {
    Some(match k {
        RoomUse::Dining | RoomUse::Meeting | RoomUse::Grand => ("Hall", true),
        RoomUse::Temple => ("Temple", true),
        RoomUse::Library => ("Library", true),
        RoomUse::Tomb => ("Tomb", false),
        RoomUse::Tavern | RoomUse::Parlour | RoomUse::Barracks | RoomUse::Hospital | RoomUse::Workshops | RoomUse::Industry => ("Hall", false),
        _ => return None,
    })
}

fn zone_noun(k: ZoneKind) -> Option<(&'static str, bool)> {
    Some(match k {
        ZoneKind::Meeting | ZoneKind::Dining => ("Hall", true),
        ZoneKind::Temple => ("Temple", true),
        ZoneKind::Tomb => ("Tomb", false),
        ZoneKind::Tavern | ZoneKind::Barracks | ZoneKind::Hospital => ("Hall", false),
        _ => return None,
    })
}

impl Game {
    /// The hall at `p`: one of the overseer's fitted rooms, else a zone
    /// the player laid out (a meeting hall, a temple...).
    fn hall_at(&self, o: Option<&Overseer>, p: Pos) -> Option<Hall> {
        if let Some(o) = o {
            let room = o.rooms.iter().filter(|r| r.stage == 1 && in_box(r.a, r.b, p) && (r.tiles.is_empty() || r.tiles.contains(&p))).find_map(|r| room_noun(r.kind).map(|n| (r, n)));
            if let Some((r, (noun, great))) = room {
                return Some(Hall { a: r.a, b: r.b, noun, great: great && (r.kind != RoomUse::Dining || o.interior.great == Some(r.a)) });
            }
        }
        let z = self.zone_at(p)?;
        let (noun, great) = zone_noun(z.kind)?;
        if z.tiles.len() < 9 {
            return None;
        }
        let (mut a, mut b) = (z.tiles[0], z.tiles[0]);
        for t in &z.tiles {
            a = Pos::new(a.x.min(t.x), a.y.min(t.y), a.z.min(t.z));
            b = Pos::new(b.x.max(t.x), b.y.max(t.y), b.z.max(t.z));
        }
        Some(Hall { a, b, noun, great })
    }

    /// Who a great hall is named for: the fort's greatest hero (living
    /// before fallen) not yet honoured with a hall, as known ("Kelda the
    /// Dragonslayer"), and what they did.
    fn hall_hero(&self) -> Option<(String, String)> {
        let honoured = |n: &str| self.stories.places.iter().any(|pl| pl.kind == PlaceKind::Hall && pl.name.ends_with(n));
        self.dwarves
            .iter()
            .filter(|d| d.epithet.is_some() && !honoured(&d.called()))
            .max_by_key(|d| (d.alive, d.epithet_rank, d.kills, u32::MAX - d.id))
            .map(|d| (d.called(), d.deed.clone().map_or_else(|| "the greatest of the fort's heroes".to_string(), |x| format!("who {}", x))))
    }

    /// The killing blow struck at `at` just now: who struck it, and what fell.
    fn blow_at(&self, at: Pos) -> Option<(u32, String, Option<String>)> {
        self.stories.marks.iter().rev().take_while(|m| m.tick == self.tick).find(|m| !m.died && m.at.z == at.z && (m.at.x - at.x).abs() <= 1 && (m.at.y - at.y).abs() <= 1).map(|m| (m.dwarf, m.name.clone(), m.foe.clone()))
    }

    /// Something was done at `at` (by `who`, the first the main one): the
    /// hall there may take a name from it. A deed outside any hall names
    /// nothing, except a siege held at the gate.
    pub(super) fn deed_at(&mut self, at: Pos, kind: DeedKind, who: &[u32]) {
        let Some(hall) = self.hall_at(self.overseer.as_ref(), at) else {
            if kind == DeedKind::SiegeHeld {
                self.name_gate(at, who);
            }
            return;
        };
        self.deed_in(hall, at, kind, who, None);
    }

    /// Daily (`ov_daily_hooks`), once the fort is settled: a great hall
    /// the overseer has finished, and not yet named, is named (one a day).
    pub(super) fn ov_name_halls(&mut self, o: &mut Overseer) {
        if o.interior.tier < 1 {
            return;
        }
        let named = |g: &Game, r: &super::overseer::Room| g.stories.places.iter().any(|pl| pl.b.is_some() && in_box(r.a, r.b, pl.at) && pl.kind == PlaceKind::Hall);
        let Some(r) = o.rooms.iter().find(|r| r.stage == 1 && room_noun(r.kind).map_or(false, |n| n.1) && (r.kind != RoomUse::Dining || o.interior.great == Some(r.a)) && !named(self, r)) else { return };
        let at = r.tiles.first().copied().unwrap_or(r.a);
        let Some(hall) = self.hall_at(Some(o), at) else { return };
        self.deed_in(hall, at, DeedKind::Work, &[], Some(o));
    }

    /// Where a wedding is held: the temple, else the great hall, if any.
    pub(super) fn wedding_hall(&self) -> Option<Pos> {
        if let Some(o) = self.overseer.as_ref() {
            let fitted = |k: RoomUse| o.rooms.iter().find(|r| r.stage == 1 && r.kind == k && (k != RoomUse::Dining || o.interior.great == Some(r.a)));
            if let Some(r) = fitted(RoomUse::Temple).or_else(|| fitted(RoomUse::Grand)).or_else(|| fitted(RoomUse::Dining)).or_else(|| fitted(RoomUse::Meeting)).or_else(|| fitted(RoomUse::Tavern)) {
                return Some(r.tiles.first().copied().unwrap_or(r.a));
            }
        }
        let zone = |k: ZoneKind| self.zones.values().filter(|z| z.kind == k && z.tiles.len() >= 9).min_by_key(|z| z.id).map(|z| z.tiles[z.tiles.len() / 2]);
        zone(ZoneKind::Temple).or_else(|| zone(ZoneKind::Meeting)).or_else(|| zone(ZoneKind::Tavern))
    }

    /// `deed_at` in a known hall (`o`: the overseer, when it is out on its round).
    fn deed_in(&mut self, hall: Hall, at: Pos, kind: DeedKind, who: &[u32], o: Option<&mut Overseer>) {
        let n = hall.noun;
        let called = |g: &Game, id: u32| g.dwarves.iter().find(|d| d.id == id).map(|d| d.called());
        let fort = self.fortress_name.clone();
        let siege = self.annals.sieges.iter().rposition(|s| s.ended.is_none()).or(self.annals.sieges.len().checked_sub(1)).map(|i| self.annals.sieges[i].clone());
        let blow = self.blow_at(at);
        let striker = who.first().and_then(|&id| called(self, id)).or_else(|| blow.as_ref().map(|b| b.1.clone()));
        let mut who: Vec<u32> = who.to_vec();
        if who.is_empty() {
            who.extend(blow.as_ref().map(|b| b.0));
        }
        let h = hash(&fort, (at.x as u64) << 32 ^ (at.y as u64) << 16 ^ at.z as u64);
        let (names, why, weight): (Vec<String>, String, u8) = match kind {
            DeedKind::LeaderSlain => {
                let Some(s) = siege else { return };
                let why = match striker {
                    Some(w) => format!("where {} slew {}, who led the {} against {}", w, s.leader, s.foe, fort),
                    None => format!("where {}, who led the {} against {}, fell", s.leader, s.foe, fort),
                };
                (vec![format!("the {} of the Unbeaten", n), format!("the {} of the Broken Host", n), format!("the {} of the Fallen Banner", n)], why, 7)
            }
            DeedKind::BeastSlain => {
                let beast = blow.as_ref().and_then(|b| b.2.clone()).unwrap_or_else(|| "beast".into());
                let b = cap(&beast);
                let why = match striker {
                    Some(w) => format!("where {} slew the {}", w, beast),
                    None => format!("where the {} was slain", beast),
                };
                (vec![format!("the {} of the {}'s Fall", n, b), format!("the {}bane {}", b, n), format!("the {} of the Slain {}", n, b)], why, 7)
            }
            DeedKind::Death | DeedKind::Massacre => {
                let tick = self.tick;
                let dead: Vec<String> = self.stories.marks.iter().filter(|m| m.died && m.tick + DAY >= tick && in_box(hall.a, hall.b, m.at)).map(|m| m.name.clone()).collect();
                if dead.is_empty() || (kind == DeedKind::Death && dead.len() < 3) {
                    return;
                }
                who = self.stories.marks.iter().filter(|m| m.died && m.tick + DAY >= tick && in_box(hall.a, hall.b, m.at)).map(|m| m.dwarf).collect();
                (vec![format!("the Weeping {}", n), format!("the {} of Tears", n), format!("the Silent {}", n)], format!("where {} died in one day", list(&dead)), 6)
            }
            DeedKind::Wedding => {
                let couple: Vec<String> = who.iter().filter_map(|&id| called(self, id)).collect();
                if couple.len() < 2 {
                    return;
                }
                (vec![format!("the {} of Vows", n), format!("the {} of Rings", n), format!("the Wedding {}", n)], format!("where {} were wed", list(&couple)), 2)
            }
            DeedKind::SiegeHeld => {
                let Some(s) = siege else { return };
                let why = format!("where the {} under {} broke against {}", s.foe, s.leader, fort);
                (vec![format!("the {} of the Held Line", n), format!("the Unyielding {}", n), format!("the {} of the Last Stand", n)], why, 5)
            }
            DeedKind::Work => {
                if !hall.great {
                    return;
                }
                if let Some((hero, deed)) = self.hall_hero() {
                    (vec![format!("the {} of {}", n, hero)], format!("for {}, {}", hero, deed), 3)
                } else if let Some(d) = self
                    .dwarves
                    .iter()
                    .filter(|d| d.alive && d.noble.is_some() && !self.stories.places.iter().any(|pl| pl.deed.contains(&d.name)))
                    .max_by_key(|d| (d.noble.map(|x| x as u8), u32::MAX - d.id))
                {
                    // A noble (not yet honoured): "the Hall of Borra", for the expedition leader.
                    let name = format!("the {} of {}", n, first_name(&d.name));
                    (vec![name], format!("for the {} {}, who ruled the fort when it was finished", d.noble.map_or("", |x| x.title()).to_lowercase(), d.name), 2)
                } else {
                    let name = super::names::coin(super::names::hash(&[h, hall.a.x as u64, hall.a.y as u64]), super::names::NameTheme::Hall);
                    (vec![name.replace(" Hall", &format!(" {}", n)).replace("Hall ", &format!("{} ", n))], "so named when it was finished".to_string(), 1)
                }
            }
        };
        // The first of the names no other place carries.
        let Some(name) = names.into_iter().find(|nm| !self.stories.places.iter().any(|pl| pl.name == *nm && !(pl.at == hall.a && pl.b == Some(hall.b)))) else { return };
        let mid = Pos::new((hall.a.x + hall.b.x) / 2, (hall.a.y + hall.b.y) / 2, hall.a.z);
        self.proclaim(PlaceKind::Hall, hall.a, hall.b, mid, name, why, weight, &who, kind == DeedKind::Work, o);
    }

    /// The first siege to break at the gate names it.
    fn name_gate(&mut self, at: Pos, who: &[u32]) {
        let Some(gate) = self.overseer.as_ref().and_then(|o| o.top.into_iter().chain(o.base).find(|g| g.z == at.z && (g.x - at.x).abs() <= 6 && (g.y - at.y).abs() <= 6)) else { return };
        if self.stories.places.iter().any(|pl| pl.kind == PlaceKind::Gate) {
            return;
        }
        let Some(s) = self.annals.sieges.last().cloned() else { return };
        let h = hash(&self.fortress_name, s.tick);
        let lead = first_name(&s.leader).to_string();
        let forms = [format!("the Gate of {}'s Ruin", lead), "the Unbroken Gate".to_string(), "the Gate of the Long Watch".to_string()];
        let name = forms[(h % 3) as usize].clone();
        let why = format!("where the {} under {} broke against {}", s.foe, s.leader, self.fortress_name);
        let (a, b) = (gate.offset(-2, -2, 0), gate.offset(2, 2, 0));
        self.proclaim(PlaceKind::Gate, a, b, gate, name, why, 4, who, false, None);
    }

    /// Name (or rename) the place and, if the name took, tell the fort:
    /// the announcement, the overseer's notes, the legends, the year's
    /// chronicle and the fort's memory; a great renaming stops the game.
    #[allow(clippy::too_many_arguments)]
    fn proclaim(&mut self, kind: PlaceKind, a: Pos, b: Pos, mid: Pos, name: String, why: String, weight: u8, who: &[u32], finished: bool, o: Option<&mut Overseer>) {
        let old = self.stories.places.iter().find(|pl| pl.b.map_or(false, |pb| (pl.at, pb) == (a, b))).map(|pl| pl.name.clone());
        if old.as_deref() == Some(name.as_str()) {
            return;
        }
        let i = self.name_place(kind, a, b, name.clone(), why.clone(), weight, false);
        if self.stories.places[i].name != name {
            return;
        }
        // What it was called until now.
        let was = old.or_else(|| self.place_name(mid).filter(|t| t.starts_with("the "))).unwrap_or_else(|| match kind {
            PlaceKind::Gate => "the gate".to_string(),
            _ => "the hall".to_string(),
        });
        let fort = self.fortress_name.clone();
        let (text, chron) = if finished {
            (format!("{} is named {}, {}.", cap(&was), name, why), format!("{} was named {}, {}.", cap(&was), name, why))
        } else {
            (format!("{} is called {} now, {}.", cap(&was), name, why), format!("{} was named {}, {}.", cap(&was), name, why))
        };
        self.msg_at(&text, (230, 200, 150), Some(mid), super::logkind::LogKind::Legend);
        self.legend(format!("In {}, {} was named {}, {}.", fort, was, name, why));
        self.year_log.named.push(chron);
        self.record(super::memory::MemoryKind::Named, who, Some(mid), format!("{} was named {}, {}.", cap(&was), name, why), weight.max(2));
        self.count("m2.places_named", 1);
        let note = if finished { format!("Named {} {}.", was, name) } else { format!("They will call {} {} now.", was, name) };
        match o {
            Some(o) => Self::note_ov(o, self.tick, note),
            None => {
                if let Some(mut o) = self.overseer.take() {
                    Self::note_ov(&mut o, self.tick, note);
                    self.overseer = Some(o);
                }
            }
        }
        if weight >= 6 {
            self.spotlight("A hall is renamed", &text, Some(mid), (230, 200, 150));
        }
    }

    /// A siege is over (`on_siege_end`): if it broke on us, the hall where
    /// it broke takes the name of the stand, and the gate its first name.
    pub(super) fn name_for_siege(&mut self, i: usize) {
        let Some(s) = self.annals.sieges.get(i).cloned() else { return };
        let fought = s.foes_slain + s.foes_caught > 0 || s.outcome == "routed";
        let named = self.stories.episodes.iter().rev().find(|e| e.siege == Some(i)).map(|e| (e.at, e.kind));
        // A slaughter leaves a weeping hall where it was thickest.
        if let Some((at, super::stories::EpisodeKind::Massacre)) = named {
            self.deed_at(at, DeedKind::Massacre, &[]);
        }
        if s.outcome == "the fortress fell" || !fought {
            return;
        }
        let episode = self.stories.episodes.iter().rev().find(|e| e.siege == Some(i)).map(|e| (e.at, e.veterans.clone()));
        let (at, who) = match (episode, s.broke_at) {
            (Some((at, v)), _) => (Some(at), v),
            (None, at) => (at, Vec::new()),
        };
        if let Some(at) = at {
            self.deed_at(at, DeedKind::SiegeHeld, &who);
        }
        if let Some(g) = self.overseer.as_ref().and_then(|o| o.top) {
            self.deed_at(g, DeedKind::SiegeHeld, &who);
        }
    }

    /// The chronicle's lines for the places named this year.
    pub(super) fn lines_named(&self, log: &YearLog) -> Vec<String> {
        log.named.iter().take(6).cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A meeting hall laid out by the player, 7 by 5, on open floor.
    fn hall(g: &mut Game) -> (Pos, Pos) {
        let a = g.embark.offset(-3, -2, 0);
        let b = a.offset(6, 4, 0);
        for p in box_iter(a, b) {
            let t = g.world.t_mut(p);
            t.kind = TileKind::Floor;
            t.revealed = true;
            t.zone = 0;
            t.building = 0;
        }
        assert_eq!(g.add_zone(a, b, ZoneKind::Meeting), 35);
        (a, b)
    }

    #[test]
    fn a_hall_is_named_for_a_hero_then_renamed_for_a_great_deed() {
        let mut g = Game::new(5);
        g.stories.places.clear();
        let (a, _) = hall(&mut g);
        let mid = a.offset(3, 2, 0);
        // Finished, with a hero in the fort: it is named for her.
        let hero = g.dwarves[0].id;
        g.dwarves[0].epithet = Some("Dragonslayer".into());
        g.dwarves[0].deed = Some("slew the dragon Ushat".into());
        let called = g.dwarves[0].called();
        g.deed_at(mid, DeedKind::Work, &[]);
        let look = g.story_place_at(mid).expect("the look panel tells the name");
        assert!(look.starts_with(&format!("The Hall of {}", called)) && look.contains("slew the dragon"), "{}", look);
        // A wedding is lighter: the hero keeps the hall.
        let (x, y) = (g.dwarves[1].id, g.dwarves[2].id);
        g.deed_at(mid, DeedKind::Wedding, &[x, y]);
        assert_eq!(g.place_at(mid).map(|p| p.name.clone()), Some(format!("the Hall of {}", called)));
        // A death outside any hall names nothing; two deaths in the hall aren't enough.
        g.deed_at(g.embark.offset(30, 30, 0), DeedKind::LeaderSlain, &[hero]);
        assert_eq!(g.stories.places.len(), 1);
        // The siege's leader falls in it: the Hall of the Unbeaten.
        g.note_siege("goblins", 12, "Snodub Gutripper");
        g.deed_at(mid, DeedKind::LeaderSlain, &[hero]);
        let pl = g.place_at(mid).expect("named").clone();
        assert_eq!(pl.name, "the Hall of the Unbeaten");
        assert!(pl.deed.contains("slew Snodub Gutripper") && pl.deed.contains(&called), "{}", pl.deed);
        assert!(g.log.iter().any(|l| l.text.contains("is called the Hall of the Unbeaten now")), "the renaming is announced");
        assert!(g.memory.events.iter().any(|m| m.kind == super::super::memory::MemoryKind::Named && m.who.contains(&hero)));
        assert!(g.spotlights.iter().any(|s| s.title == "A hall is renamed"));
        // ...and it goes in the year's chronicle.
        assert_eq!(g.year_log.named.len(), 2);
        g.tick = YEAR + 5;
        g.write_chronicle_for_test();
        let c = g.chronicles.last().expect("a chronicle");
        assert!(c.lines.iter().any(|l| l.starts_with(&format!("The Hall of {} was named the Hall of the Unbeaten, where", called))), "{:?}", c.lines);
    }

    #[test]
    fn the_overseer_names_its_great_halls() {
        let mut g = Game::new(5003);
        g.set_overseer(Some(super::super::overseer::Style::Builder));
        while g.tick < 2 * YEAR && g.alive_count() > 0 && !g.stories.places.iter().any(|p| p.kind == PlaceKind::Hall) {
            g.step();
        }
        let pl = g.stories.places.iter().find(|p| p.kind == PlaceKind::Hall).expect("a settled fort names a great hall").clone();
        assert!(pl.name.starts_with("the ") && !pl.deed.is_empty(), "{:?}", pl);
        let o = g.overseer.as_ref().unwrap();
        assert!(o.notes.iter().any(|n| n.1.contains(&pl.name)), "the overseer notes the name");
        assert!(g.year_log.named.iter().chain(g.chronicles.iter().flat_map(|c| c.lines.iter())).any(|l| l.contains(&pl.name)), "the chronicle tells it");
        eprintln!("day {}: {} ({})", g.tick / DAY, pl.name, pl.deed);
    }

    #[test]
    fn three_deaths_in_a_day_make_a_weeping_hall_and_a_wedding_names_a_plain_one() {
        let mut g = Game::new(5);
        g.stories.places.clear();
        let (a, b) = hall(&mut g);
        let mid = a.offset(3, 2, 0);
        let ids: Vec<u32> = g.dwarves.iter().take(4).map(|d| d.id).collect();
        // A wedding names an unnamed hall.
        g.deed_at(mid, DeedKind::Wedding, &ids[..2]);
        assert_eq!(g.place_at(mid).map(|p| p.name.as_str()), Some("the Hall of Vows"));
        // Deaths in it: two are grief, the third a name.
        for (n, &id) in ids[1..].iter().enumerate() {
            let i = g.dwarves.iter().position(|d| d.id == id).unwrap();
            let mut d = std::mem::take(&mut g.dwarves[i]);
            d.pos = a.offset(n as i32, 1, 0);
            g.kill_for_test(&mut d, "has been struck down");
            g.dwarves[i] = d;
            let weeping = g.place_at(mid).map_or(false, |p| p.name == "the Weeping Hall");
            assert_eq!(weeping, n == 2, "after {} deaths", n + 1);
        }
        let pl = g.place_at(b).unwrap();
        assert!(pl.deed.starts_with("where ") && pl.deed.ends_with("died in one day") && pl.weight == 6, "{}", pl.deed);
        // A second hall can't be the Weeping Hall too.
        assert!(g.stories.places.iter().filter(|p| p.name == "the Weeping Hall").count() == 1);
    }
}
