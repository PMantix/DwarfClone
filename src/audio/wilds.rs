//! What an adventurer hears: in a site, the place's own sound over the
//! ambience round them (a lair breathes and drips, a ruin whistles and
//! creaks, a town talks and hammers, the wild sounds of its land by day and
//! by night); on the world map, the road (the land's wind and creatures,
//! the weather, footsteps as they go) and a sparse travelling theme that
//! darkens at night and in foul weather; and a fight's music when foes are
//! near. Everything here only reads the adventure.

use super::music::Mood;
use super::scene;
use super::Scene;
use crate::game::adv::{Adventure, SiteKey, View};
use crate::game::{clock, Weather, DAY, SEASON, YEAR};
use crate::geom::Pos;
use crate::overworld::{Biome, SiteKind};

/// The kind of place, for its sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// A beast's den.
    Lair,
    /// Broken walls: a ruined site, or a fallen fort (`true`).
    Ruin(bool),
    /// People at work: a town, a hamlet, a fortress.
    Town,
    /// The open land.
    Wild(Biome),
}

/// What the adventure's ears remember between frames.
#[derive(Clone, Debug, Default)]
pub struct Ears {
    /// Where they last listened (a site's key, or the road), to start
    /// afresh in a new place.
    pub place: Option<u64>,
    /// The world cell they were last in, for footsteps on the road.
    pub road: Option<(i32, i32)>,
}

/// How a land sounds: (birds, insects, wind, frogs, owls), each 0..1.
fn land(b: Biome) -> (f32, f32, f32, f32, f32) {
    match b {
        Biome::Ocean => (0.1, 0.0, 0.8, 0.0, 0.0),
        Biome::Mountain => (0.3, 0.1, 0.8, 0.0, 0.2),
        Biome::Hills => (0.7, 0.5, 0.5, 0.0, 0.4),
        Biome::Forest => (1.0, 0.6, 0.3, 0.1, 1.0),
        Biome::Grassland => (0.7, 0.9, 0.5, 0.0, 0.2),
        Biome::Desert => (0.1, 0.3, 0.7, 0.0, 0.0),
        Biome::Tundra => (0.2, 0.1, 0.8, 0.0, 0.1),
        Biome::Swamp => (0.6, 1.0, 0.2, 1.0, 0.6),
        Biome::Jungle => (1.0, 1.0, 0.2, 0.7, 0.5),
        Biome::Taiga => (0.6, 0.3, 0.5, 0.1, 1.0),
        Biome::Savanna => (0.7, 0.9, 0.5, 0.0, 0.2),
        Biome::Badlands => (0.1, 0.2, 0.8, 0.0, 0.0),
        Biome::Glacier => (0.0, 0.0, 1.0, 0.0, 0.0),
        Biome::Shrubland => (0.6, 0.8, 0.5, 0.0, 0.3),
        Biome::Marsh => (0.6, 0.9, 0.4, 1.0, 0.4),
        Biome::Lake => (0.5, 0.7, 0.4, 0.8, 0.3),
    }
}

/// The place the hero is in, or None on the road.
pub fn place(a: &Adventure) -> Option<Place> {
    let v = a.visit.as_ref()?;
    Some(match v.key {
        SiteKey::Wild(x, y) => Place::Wild(a.ow().cell(x, y).biome),
        SiteKey::Fort(_) => Place::Ruin(true),
        SiteKey::Site(id) => match a.ow().sites.iter().find(|s| s.id == id) {
            Some(s) if s.ruined => Place::Ruin(false),
            Some(s) if s.kind == SiteKind::Lair => Place::Lair,
            Some(_) => Place::Town,
            None => Place::Wild(a.ow().cell(a.at.0, a.at.1).biome),
        },
    })
}

/// A number for where the hero is listening: each site its own, the road one.
pub fn place_tag(a: &Adventure) -> u64 {
    match a.visit.as_ref().map(|v| v.key) {
        None => 1,
        Some(SiteKey::Wild(x, y)) => 2 + (((x as u32 as u64) << 20) ^ (y as u32 as u64)) * 4,
        Some(SiteKey::Site(id)) => 3 + id as u64 * 4,
        Some(SiteKey::Fort(i)) => 4 + i as u64 * 4,
    }
}

/// 0 by day, 1 by night, 2 in foul weather: the colour of the road's music.
pub fn tint(tick: u64, w: Weather) -> u8 {
    if matches!(w, Weather::Storm | Weather::Blizzard | Weather::BloodRain) {
        2
    } else if clock::is_night(tick) {
        1
    } else {
        0
    }
}

/// The land's creatures and wind at `tick`, `open` (0..1) to the sky.
fn wild_life(s: &mut Scene, b: Biome, tick: u64, w: Weather, open: f32) {
    let (birds, insects, wind, frogs, owls) = land(b);
    let h = clock::hour(tick);
    let season = ((tick % YEAR) / SEASON).min(3) as usize;
    let fair = matches!(w, Weather::Clear | Weather::Snow);
    let night = clock::is_night(tick);
    let warm = [0.8, 1.0, 0.6, 0.0][season];
    s.wind = s.wind.max(open * (0.15 + 0.4 * wind));
    s.birds = if fair && (6..18).contains(&h) && season != 3 { open * birds * [1.0, 0.8, 0.5, 0.0][season] } else { 0.0 };
    s.insects = if fair && (h >= 18 || h < 4) { open * insects * warm } else { 0.0 };
    s.frogs = if season != 3 && !matches!(w, Weather::Blizzard | Weather::Snow) { open * frogs * warm * if night { 1.0 } else { 0.35 } } else { 0.0 };
    s.owls = if night && fair { open * owls } else { 0.0 };
}

/// The weather under the open sky.
fn weather(s: &mut Scene, w: Weather, open: f32) {
    let wet = matches!(w, Weather::Rain | Weather::Storm | Weather::BloodRain);
    s.wind = s.wind.max(open * match w {
        Weather::Storm => 1.0,
        Weather::Blizzard => 1.0,
        Weather::Snow => 0.5,
        _ if wet => 0.5,
        _ => 0.0,
    });
    s.rain = if wet { open * if w == Weather::Storm { 1.0 } else { 0.7 } } else { 0.0 };
    s.thunder = if w == Weather::Storm { open } else { 0.0 };
}

/// What the hero hears round them now (the mood is left for the caller).
pub fn ambience(a: &Adventure) -> Scene {
    let tick = a.now();
    let Some(v) = a.visit.as_ref() else {
        // The road: the open land of the cell they're in, its weather.
        let mut s = Scene { season: ((tick % YEAR) / SEASON).min(3) as u8, tint: tint(tick, a.weather), ..Scene::default() };
        let b = a.ow().cell(a.at.0, a.at.1).biome;
        wild_life(&mut s, b, tick, a.weather, 1.0);
        weather(&mut s, a.weather, 1.0);
        return s;
    };
    let g = &v.g;
    let cam = g.dwarves.get(v.hero).map_or(Pos::new(0, 0, 0), |d| d.pos);
    let mut s = scene::ambience(g, cam);
    s.tint = tint(tick, g.weather);
    let open = if g.world.in_bounds(cam) && g.world.outside(cam) { 1.0 } else { 0.0 };
    let day = !clock::is_night(tick);
    match place(a) {
        Some(Place::Lair) => {
            s.breath = 0.8;
            s.drips = s.drips.max(0.5);
            s.hum = s.hum.max(0.25);
            s.birds *= 0.2;
        }
        Some(Place::Ruin(fallen)) => {
            s.gaps = 0.4 + 0.4 * open + if matches!(g.weather, Weather::Storm | Weather::Blizzard) { 0.2 } else { 0.0 };
            s.creaks = if fallen { 0.8 } else { 0.6 };
            s.birds *= 0.4;
            s.drips = s.drips.max(0.2);
        }
        Some(Place::Town) => {
            s.voices = if day { 0.7 } else { 0.15 };
            s.hammer = if day && (7..19).contains(&clock::hour(tick)) { 0.7 } else { 0.0 };
            s.birds *= 0.6;
        }
        Some(Place::Wild(b)) => {
            let roof = if open > 0.0 { 1.0 } else { 0.25 };
            wild_life(&mut s, b, tick, g.weather, roof);
        }
        None => {}
    }
    s
}

/// Foes close enough to fight: a hostile thing in sight of the hero (or
/// just round a corner), or one that struck them a moment ago.
pub fn foes_near(a: &Adventure) -> bool {
    let Some(v) = a.visit.as_ref() else { return false };
    let g = &v.g;
    let Some(d) = g.dwarves.get(v.hero) else { return false };
    if d.attacker.map_or(false, |(_, t)| t > 0 && g.tick < t + DAY / 24) {
        return true;
    }
    let foe = |c: &crate::creature::Creature| c.alive && (c.hostile() || c.siege || c.target == Some(d.id)) && !c.leaving && c.pos.z == d.pos.z;
    let gap = |p: Pos| (p.x - d.pos.x).abs().max((p.y - d.pos.y).abs());
    // Heard close by, if not seen; seen a little further off.
    g.creatures.values().any(|c| foe(c) && gap(c.pos) <= 4) || g.in_sight(v.hero).iter().filter_map(|id| g.creatures.get(id)).any(|c| foe(c) && gap(c.pos) <= 12)
}

/// The adventure's music: a fight when foes are near (or met on the road),
/// the road's theme while travelling and in the wild, the fort's calm in a
/// town, a lament in a fallen fort, silence (only the place) in a lair.
pub fn mood(a: &Adventure, view: &View, meeting: bool) -> Mood {
    match view {
        View::Create => return Mood::Title,
        View::Dead => return Mood::Mourning,
        _ => {}
    }
    if a.over.is_some() {
        return Mood::Mourning;
    }
    if meeting || foes_near(a) {
        return Mood::Skirmish;
    }
    match place(a) {
        None | Some(Place::Wild(_)) => Mood::Travel,
        Some(Place::Town) => Mood::Calm,
        Some(Place::Ruin(true)) => Mood::Mourning,
        Some(Place::Ruin(false) | Place::Lair) => Mood::Silent,
    }
}

/// The hero walked on to another cell of the world map since last asked.
pub fn stepped(e: &mut Ears, a: &Adventure) -> bool {
    if a.visit.is_some() {
        e.road = None;
        return false;
    }
    let moved = e.road.map_or(false, |r| r != a.at);
    e.road = Some(a.at);
    moved
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::adv::{quick_start, SiteKey};
    use crate::game::Game;

    fn adventure(seed: u64) -> Adventure {
        quick_start(Game::new(seed).overworld, seed)
    }

    #[test]
    fn the_road_sounds_of_its_land_and_weather_and_has_its_theme() {
        let mut a = adventure(21);
        a.leave();
        assert!(a.visit.is_none());
        let view = View::Travel;
        assert_eq!(mood(&a, &view, false), Mood::Travel);
        assert_eq!(mood(&a, &view, true), Mood::Skirmish, "met on the road");
        a.weather = Weather::Clear;
        a.clock = a.clock - a.clock % DAY + 10 * clock::HOUR;
        let day = ambience(&a);
        assert_eq!(day.tint, 0);
        assert!(day.wind > 0.1, "{:?}", day);
        a.clock += 12 * clock::HOUR;
        let night = ambience(&a);
        assert_eq!(night.tint, 1);
        assert_eq!(night.birds, 0.0);
        a.weather = Weather::Storm;
        let storm = ambience(&a);
        assert_eq!(storm.tint, 2);
        assert!(storm.rain > 0.5 && storm.thunder > 0.5 && storm.wind > night.wind, "{:?}", storm);
        a.weather = Weather::Blizzard;
        let snow = ambience(&a);
        assert!(snow.wind > 0.9 && snow.rain == 0.0 && snow.frogs == 0.0 && snow.tint == 2);
        // Footsteps when the hero moves on.
        let mut e = Ears::default();
        assert!(!stepped(&mut e, &a), "nothing is news at first");
        assert!(!stepped(&mut e, &a));
        a.at.0 += 1;
        assert!(stepped(&mut e, &a));
    }

    #[test]
    fn each_land_is_heard_its_own_way() {
        let mut s = Scene::default();
        // A forest by day sings; a glacier only blows.
        let noon = DAY / 2 + SEASON;
        wild_life(&mut s, Biome::Forest, noon, Weather::Clear, 1.0);
        assert!(s.birds > 0.5);
        let mut g = Scene::default();
        wild_life(&mut g, Biome::Glacier, noon, Weather::Clear, 1.0);
        assert!(g.birds == 0.0 && g.wind > s.wind);
        // A summer night in the swamp: frogs and insects; in the forest, an owl.
        let night = SEASON + 23 * clock::HOUR;
        let mut w = Scene::default();
        wild_life(&mut w, Biome::Swamp, night, Weather::Clear, 1.0);
        assert!(w.frogs > 0.5 && w.insects > 0.5 && w.birds == 0.0, "{:?}", w);
        let mut f = Scene::default();
        wild_life(&mut f, Biome::Forest, night, Weather::Clear, 1.0);
        assert!(f.owls > 0.5);
    }

    #[test]
    fn sites_sound_of_what_they_are() {
        let mut a = adventure(22);
        // The quick start is at the hero's home: a town.
        if matches!(a.visit.as_ref().map(|v| v.key), Some(SiteKey::Site(_))) && place(&a) == Some(Place::Town) {
            let v = a.visit.as_mut().unwrap();
            v.g.tick = v.g.tick - v.g.tick % DAY + 10 * clock::HOUR;
            let s = ambience(&a);
            assert!(s.voices > 0.5 && s.hammer > 0.0, "{:?}", s);
            assert_eq!(mood(&a, &View::Local, false), Mood::Calm);
        }
        let tag = place_tag(&a);
        // A lair breathes; a ruin whistles and creaks.
        let lair = a.ow().sites.iter().find(|s| s.kind == SiteKind::Lair && !s.ruined).map(|s| s.id);
        let ruin = a.ow().sites.iter().find(|s| s.ruined).map(|s| s.id);
        if let Some(id) = lair {
            a.leave();
            if a.enter(SiteKey::Site(id)).is_ok() {
                assert_eq!(place(&a), Some(Place::Lair));
                let s = ambience(&a);
                assert!(s.breath > 0.5 && s.drips > 0.3, "{:?}", s);
                assert_ne!(place_tag(&a), tag);
            }
        }
        if let Some(id) = ruin {
            a.leave();
            if a.enter(SiteKey::Site(id)).is_ok() {
                assert_eq!(place(&a), Some(Place::Ruin(false)));
                let s = ambience(&a);
                assert!(s.gaps > 0.3 && s.creaks > 0.3, "{:?}", s);
            }
        }
        // In the wild: its land, and the road's theme.
        a.leave();
        let at = a.at;
        if a.enter(SiteKey::Wild(at.0, at.1)).is_ok() {
            assert!(matches!(place(&a), Some(Place::Wild(_))));
            assert!(matches!(mood(&a, &View::Local, false), Mood::Travel | Mood::Skirmish));
        }
    }

    #[test]
    fn a_foe_in_sight_is_a_fight() {
        let mut a = adventure(23);
        let at = a.at;
        a.leave();
        a.enter(SiteKey::Wild(at.0, at.1)).unwrap();
        let v = a.visit.as_mut().unwrap();
        // Clear the field first: nothing hostile about.
        v.g.creatures.retain(|_, c| !c.hostile() && !c.siege);
        assert!(!foes_near(&a));
        let v = a.visit.as_mut().unwrap();
        let h = v.hero;
        let id = v.g.raider_for(h);
        assert!(v.g.creatures.contains_key(&id));
        assert!(foes_near(&a));
        assert_eq!(mood(&a, &View::Local, false), Mood::Skirmish);
    }
}
