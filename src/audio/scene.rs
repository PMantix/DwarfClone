//! What the camera hears: the ambience around it (sky, weather, caverns,
//! fire, magma, water), the mood of the fortress for the music, and the
//! effects worth playing from what's happening on screen. Everything here
//! only reads the game.

use super::music::Mood;
use super::{Scene, SfxKind, Tone};
use crate::buildings::BuildingKind;
use crate::creature::Species;
use crate::dwarf::Task;
use crate::game::sounds::Sound;
use crate::game::{FestivalKind, Game, Weather, ZoneKind, DAY, SEASON, YEAR};
use crate::geom::Pos;
use crate::world::TileKind;
use std::collections::{HashMap, HashSet};

/// The season, 0 spring .. 3 winter.
pub fn season(g: &Game) -> u8 {
    ((g.tick % YEAR) / SEASON).min(3) as u8
}

/// The fortress's mood, most pressing first: war, then feasting, then
/// grief, else peace.
pub fn fortress_mood(g: &Game, cam: Pos) -> Mood {
    if g.fallen {
        return Mood::Mourning;
    }
    let siege = g.incoming.is_some() || g.creatures.values().any(|c| c.alive && c.siege && !c.leaving);
    if siege {
        return Mood::War;
    }
    let occasion = g.stories.occasion.as_ref().map_or(false, |o| g.tick < o.until);
    if g.festival.is_some() || occasion || merry_tavern(g, cam) {
        return Mood::Festive;
    }
    let grief = g.stories.marks.iter().any(|m| m.died && g.tick < m.tick + 2 * DAY);
    if grief {
        return Mood::Mourning;
    }
    Mood::Calm
}

/// The camera is in a tavern with company.
fn merry_tavern(g: &Game, cam: Pos) -> bool {
    let Some(t) = g.world.get(cam) else { return false };
    if t.zone == 0 || g.zones.get(&t.zone).map_or(true, |z| z.kind != ZoneKind::Tavern) {
        return false;
    }
    let zone = t.zone;
    g.dwarves.iter().filter(|d| d.alive && g.world.get(d.pos).map_or(false, |t| t.zone == zone)).count() >= 3
}

/// The ambience around the camera at `cam` (and the season); the mood is
/// left for the caller.
pub fn ambience(g: &Game, cam: Pos) -> Scene {
    let w = &g.world;
    let mut s = Scene { season: season(g), ..Scene::default() };
    if w.w <= 1 || w.h <= 1 {
        return s;
    }
    let cam = Pos::new(cam.x.clamp(0, w.w - 1), cam.y.clamp(0, w.h - 1), cam.z.clamp(0, w.d - 1));
    // How much of the view is open to the sky.
    let (mut open, mut seen) = (0, 0);
    for dy in (-9..=9).step_by(3) {
        for dx in (-18..=18).step_by(3) {
            let p = Pos::new(cam.x + dx, cam.y + dy, cam.z);
            if let Some(t) = w.get(p) {
                seen += 1;
                if t.kind != TileKind::Wall && w.outside(p) {
                    open += 1;
                }
            }
        }
    }
    let outdoor = if seen == 0 { 0.0 } else { open as f32 / seen as f32 };
    let depth = (w.surface_z(cam.x, cam.y) - cam.z).max(0);
    // Just below the surface the weather is still heard, muffled.
    let near = outdoor.max(match depth {
        0..=1 => 0.35,
        2..=4 => 0.15,
        _ => 0.0,
    });
    let wet = matches!(g.weather, Weather::Rain | Weather::Storm | Weather::BloodRain);
    let stormy = matches!(g.weather, Weather::Storm | Weather::Blizzard);
    let snowy = matches!(g.weather, Weather::Snow | Weather::Blizzard);
    let winter = s.season == 3;
    s.wind = near * (0.3 + if stormy { 0.7 } else if wet || snowy { 0.3 } else { 0.0 } + if winter { 0.1 } else { 0.0 });
    s.rain = if wet { near * if g.weather == Weather::Storm { 1.0 } else { 0.7 } } else { 0.0 };
    s.thunder = if g.weather == Weather::Storm {
        if outdoor > 0.2 {
            1.0
        } else if depth < 8 {
            0.4
        } else {
            0.0
        }
    } else {
        0.0
    };
    let day = (g.tick % DAY) as f32 / DAY as f32;
    let fair = !wet && !stormy;
    if fair && !winter && (0.03..0.62).contains(&day) {
        s.birds = outdoor * [1.0, 0.7, 0.4, 0.0][s.season as usize];
    }
    if fair && (0.6..0.97).contains(&day) {
        s.insects = outdoor * [0.3, 1.0, 0.3, 0.0][s.season as usize];
    }
    let cavern = w.caverns.iter().any(|&(lo, hi)| cam.z >= lo - 1 && cam.z <= hi + 1);
    let under = outdoor < 0.1 && depth > 0;
    if cavern {
        s.hum = 0.9;
        s.drips = 1.0;
    } else if under {
        s.hum = (depth as f32 / 30.0).min(0.5);
        s.drips = if depth > 4 { 0.25 } else { 0.0 };
    }
    // Fire, magma and water nearby, nearer counting for more.
    let (rx, ry) = (16, 10);
    let (mut fire, mut magma, mut water) = (0.0f32, 0.0f32, 0.0f32);
    for dz in -3..=1 {
        let z = cam.z + dz;
        if z < 0 || z >= w.d {
            continue;
        }
        for dy in (-ry..=ry).step_by(2) {
            for dx in (-rx..=rx).step_by(2) {
                let Some(t) = w.get(Pos::new(cam.x + dx, cam.y + dy, z)) else { continue };
                if t.fire == 0 && t.magma == 0 && (t.water == 0 || t.frozen) {
                    continue;
                }
                let d = ((dx * dx) as f32 / (rx * rx) as f32 + (dy * dy) as f32 / (ry * ry) as f32).sqrt();
                let near = (1.0 - d).max(0.0) * if dz == 0 { 1.0 } else { 0.5 };
                if t.fire > 0 && dz >= -1 {
                    fire += near * 0.4;
                }
                if t.magma > 0 {
                    magma += near * 0.08;
                }
                if t.water > 0 && !t.frozen && dz >= -1 {
                    water += near * 0.05;
                }
            }
        }
    }
    s.fire = fire.min(1.0);
    s.magma = magma.min(1.0);
    s.water = water.min(1.0);
    s
}

/// The effects for a big moment's card.
pub fn spotlight_cues(title: &str, color: (u8, u8, u8)) -> Vec<SfxKind> {
    match title {
        "An artifact" | "A monument" | "A great work" | "A dream come true" => vec![SfxKind::Chime],
        "A dragon!" | "A beast of legend!" | "From the deep" | "It comes" => vec![SfxKind::Roar, SfxKind::Stinger(Tone::Dark)],
        "Siege!" | "An army on the march" | "Raiders sighted" | "The dead walk" => vec![SfxKind::Stinger(Tone::War)],
        "The siege is broken" | "Home in triumph" | "Their leader falls" | "The thing in the night is slain" => vec![SfxKind::Cheer, SfxKind::Stinger(Tone::Bright)],
        "A hero falls" | "A ghost" | "Cave-in!" | "An execution" => vec![SfxKind::Stinger(Tone::Grief)],
        _ => {
            let (r, g, _) = color;
            vec![SfxKind::Stinger(if r > 200 && g < 140 { Tone::Dark } else { Tone::Bright })]
        }
    }
}

/// An effect to play: what, how loud (after distance), and where.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cue {
    pub kind: SfxKind,
    pub gain: f32,
    pub pan: f32,
}

/// How loud and where a sound at `p` is to a camera at `cam` seeing
/// `half` tiles either way; None when out of earshot.
pub fn place(cam: Pos, p: Pos, half: (i32, i32)) -> Option<(f32, f32)> {
    let dz = (p.z - cam.z).abs();
    if dz > 1 {
        return None;
    }
    let (hx, hy) = (half.0.max(4), half.1.max(3));
    let (dx, dy) = (p.x - cam.x, p.y - cam.y);
    if dx.abs() > hx + 4 || dy.abs() > hy + 4 {
        return None;
    }
    let d = ((dx * dx) as f32 / (hx * hx) as f32 + (dy * dy) as f32 / (hy * hy) as f32).sqrt();
    let gain = (1.0 - 0.6 * d).clamp(0.2, 1.0) * if dz == 1 { 0.45 } else { 1.0 };
    Some((gain, (dx as f32 / hx as f32).clamp(-1.0, 1.0) * 0.8))
}

/// What the app has seen of the game, to notice what changed.
#[derive(Default)]
pub struct Watch {
    started: bool,
    frame: u64,
    rng: u32,
    alert: bool,
    civ_alert: bool,
    festival: Option<FestivalKind>,
    occasion: bool,
    doors: HashMap<u32, u8>,
    beasts: HashSet<u32>,
    /// The newest announcement already heard.
    log_tick: u64,
}

/// Creatures whose coming is worth a roar.
fn beastly(s: Species) -> bool {
    matches!(s, Species::Megabeast | Species::CaveHorror | Species::Bear | Species::PolarBear | Species::Lion | Species::Jaguar | Species::Alligator | Species::Spawn)
}

impl Watch {
    fn roll(&mut self) -> f32 {
        let mut x = self.rng.max(1);
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Look at the game this frame and add the effects to play to `out`.
    /// `running`: the game isn't paused (work makes no noise when it is).
    pub fn look(&mut self, g: &Game, cam: Pos, half: (i32, i32), running: bool, out: &mut Vec<Cue>) {
        self.frame += 1;
        if !self.started {
            // What's already there when we start listening isn't news.
            self.started = true;
            self.rng = (g.tick as u32) | 1;
            self.alert = g.alert;
            self.civ_alert = g.civ_alert;
            self.festival = g.festival.as_ref().map(|f| f.kind);
            self.occasion = g.stories.occasion.is_some();
            self.beasts = g.creatures.values().filter(|c| beastly(c.species)).map(|c| c.id).collect();
            self.doors = doors(g).collect();
            self.log_tick = g.log.last().map_or(0, |e| e.tick);
            return;
        }
        // Lightning close by: a thunderclap.
        for e in g.log.iter().rev().take_while(|e| e.tick > self.log_tick) {
            if e.text.starts_with("Lightning strikes") {
                out.push(Cue { kind: SfxKind::Thunder, gain: 0.8, pan: 0.0 });
                break;
            }
        }
        self.log_tick = self.log_tick.max(g.log.last().map_or(0, |e| e.tick));
        // Blows struck in sight.
        let mut fights = 0;
        for &(p, s) in &g.sounds {
            if fights >= 2 {
                break;
            }
            if let Some((gain, pan)) = place(cam, p, half) {
                out.push(Cue { kind: if s == Sound::Blow { SfxKind::Thud } else { SfxKind::Clash }, gain, pan });
                fights += 1;
            }
        }
        // The alarm.
        if (g.alert && !self.alert) || (g.civ_alert && !self.civ_alert) {
            out.push(Cue { kind: SfxKind::Bell, gain: 0.8, pan: 0.0 });
        }
        self.alert = g.alert;
        self.civ_alert = g.civ_alert;
        // A feast begins: a victory or a wedding is cheered.
        let fest = g.festival.as_ref().map(|f| f.kind);
        if fest != self.festival && matches!(fest, Some(FestivalKind::Victory | FestivalKind::Wedding)) {
            out.push(Cue { kind: SfxKind::Cheer, gain: if fest == Some(FestivalKind::Victory) { 0.9 } else { 0.6 }, pan: 0.0 });
        }
        self.festival = fest;
        let occ = g.stories.occasion.is_some();
        if occ && !self.occasion && fest.is_none() {
            out.push(Cue { kind: SfxKind::Cheer, gain: 0.6, pan: 0.0 });
        }
        self.occasion = occ;
        if self.frame % 3 == 0 && running {
            self.work(g, cam, half, out);
        }
        if self.frame % 10 == 0 {
            // Doors and bridges that moved.
            let now: Vec<(u32, u8)> = doors(g).collect();
            for &(id, state) in &now {
                let before = self.doors.get(&id).copied();
                if before.map_or(false, |b| b != state) {
                    if let Some((gain, pan)) = g.buildings.get(&id).and_then(|b| place(cam, b.pos, half)) {
                        out.push(Cue { kind: SfxKind::Clank, gain, pan });
                    }
                }
            }
            self.doors = now.into_iter().collect();
            // A beast arrives: it roars (heard from further off).
            for c in g.creatures.values() {
                if c.alive && beastly(c.species) && c.hostile() && self.beasts.insert(c.id) {
                    let wide = (half.0 * 2, half.1 * 2);
                    let (gain, pan) = place(cam, c.pos, wide).unwrap_or((0.35, 0.0));
                    out.push(Cue { kind: SfxKind::Roar, gain: gain.max(0.35), pan });
                }
            }
        }
    }

    /// Picks, hammers and saws in sight.
    fn work(&mut self, g: &Game, cam: Pos, half: (i32, i32), out: &mut Vec<Cue>) {
        let mut dig: Option<(f32, f32, bool)> = None;
        let mut forge: Option<(f32, f32)> = None;
        let mut knock: Option<(f32, f32)> = None;
        let mut n = 0u32;
        for d in g.dwarves.iter().filter(|d| d.alive) {
            match &d.task {
                Task::Dig { target, stand } if d.pos == *stand => {
                    if let Some(p) = place(cam, d.pos, half) {
                        n += 1;
                        let wood = g.world.get(*target).map_or(false, |t| t.kind == TileKind::Tree);
                        // Any one of the diggers in sight.
                        if dig.is_none() || self.roll() < 1.0 / n as f32 {
                            dig = Some((p.0, p.1, wood));
                        }
                    }
                }
                Task::Craft { bld, items, delivered } if *delivered >= items.len() => {
                    let Some(b) = g.buildings.get(bld) else { continue };
                    let Some(p) = place(cam, d.pos, half) else { continue };
                    match b.kind {
                        BuildingKind::Forge | BuildingKind::MagmaForge => forge = Some(p),
                        BuildingKind::Mason | BuildingKind::Carpenter | BuildingKind::Craftsdwarf | BuildingKind::Mechanic => knock = Some(p),
                        _ => {}
                    }
                }
                Task::BuildTile { stand: Some(s), .. } if d.pos == *s => {
                    if let Some(p) = place(cam, d.pos, half) {
                        knock = knock.or(Some(p));
                    }
                }
                _ => {}
            }
        }
        if let Some((gain, pan, wood)) = dig {
            if self.roll() < 0.35 {
                out.push(Cue { kind: if wood { SfxKind::Knock } else { SfxKind::Pick }, gain: gain * 0.9, pan });
            }
        }
        if let Some((gain, pan)) = forge {
            if self.roll() < 0.3 {
                out.push(Cue { kind: SfxKind::Anvil, gain: gain * 0.8, pan });
            }
        }
        if let Some((gain, pan)) = knock {
            if self.roll() < 0.2 {
                out.push(Cue { kind: SfxKind::Knock, gain: gain * 0.8, pan });
            }
        }
    }
}

/// Every door, bridge and floodgate, and how it stands.
fn doors(g: &Game) -> impl Iterator<Item = (u32, u8)> + '_ {
    g.buildings
        .values()
        .filter(|b| b.built && matches!(b.kind, BuildingKind::Door | BuildingKind::Bridge | BuildingKind::Floodgate))
        .filter_map(|b| g.world.get(b.pos).map(|t| (b.id, t.door)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mood_follows_the_fortress() {
        let mut g = Game::new(5);
        let cam = g.embark;
        assert_eq!(fortress_mood(&g, cam), Mood::Calm);
        // A festival: the jig.
        g.festival = Some(crate::game::Festival { kind: FestivalKind::Harvest, until: g.tick + 100, venue: Vec::new(), attended: Vec::new(), mugs: Vec::new(), told: 0 });
        assert_eq!(fortress_mood(&g, cam), Mood::Festive);
        // A siege coming outranks it: war drums.
        g.incoming = Some((g.tick + 1000, g.neighbors.first().cloned().expect("a neighbour"), 10, "Snodub".into()));
        assert_eq!(fortress_mood(&g, cam), Mood::War);
        g.incoming = None;
        g.festival = None;
        // A death: a lament for two days, then peace.
        g.stories.marks.push(crate::game::stories::Mark { tick: g.tick, at: cam, dwarf: 1, name: "Urist".into(), died: true, foe: None, siege: None });
        assert_eq!(fortress_mood(&g, cam), Mood::Mourning);
        g.tick += 3 * DAY;
        assert_eq!(fortress_mood(&g, cam), Mood::Calm);
        g.fallen = true;
        assert_eq!(fortress_mood(&g, cam), Mood::Mourning);
    }

    #[test]
    fn invaders_on_the_map_are_war() {
        let mut g = Game::new(6);
        let cam = g.embark;
        if let Some(c) = g.creatures.values_mut().next() {
            c.siege = true;
            c.alive = true;
            c.leaving = false;
            assert_eq!(fortress_mood(&g, cam), Mood::War);
        }
    }

    #[test]
    fn the_surface_has_wind_and_weather_and_the_deep_has_drips() {
        let mut g = Game::new(7);
        let top = g.embark;
        g.weather = Weather::Clear;
        g.tick = g.tick - g.tick % DAY + DAY / 4;
        let s = ambience(&g, top);
        assert!(s.wind > 0.2, "wind outside: {:?}", s);
        assert_eq!(s.rain, 0.0);
        assert_eq!(s.drips, 0.0);
        if season(&g) != 3 {
            assert!(s.birds > 0.0, "birds by day: {:?}", s);
        }
        g.weather = Weather::Storm;
        let st = ambience(&g, top);
        assert!(st.wind > s.wind && st.rain > 0.5 && st.thunder > 0.5, "a storm: {:?}", st);
        assert_eq!(st.birds, 0.0);
        // Deep below: no sky, a hum and drips.
        let deep = Pos::new(top.x, top.y, 1);
        let d = ambience(&g, deep);
        assert!(d.wind < 0.05 && d.rain < 0.05 && d.birds == 0.0, "underground: {:?}", d);
        assert!(d.hum > 0.0, "underground hum: {:?}", d);
        // In a cavern, if the map has one.
        if let Some(&(lo, hi)) = g.world.caverns.first() {
            let c = ambience(&g, Pos::new(top.x, top.y, (lo + hi) / 2));
            assert!(c.drips > 0.5 && c.hum > 0.5, "cavern: {:?}", c);
        }
    }

    #[test]
    fn fire_and_water_are_heard_near() {
        let mut g = Game::new(8);
        let p = g.embark;
        assert_eq!(ambience(&g, p).fire, 0.0);
        for dx in 0..3 {
            let q = Pos::new(p.x + dx, p.y, p.z);
            if g.world.in_bounds(q) {
                g.world.t_mut(q).fire = 10;
            }
        }
        assert!(ambience(&g, p).fire > 0.5);
        let far = Pos::new(p.x + 60, p.y, p.z);
        if g.world.in_bounds(far) {
            assert!(ambience(&g, far).fire < 0.01);
        }
    }

    #[test]
    fn insects_sing_at_summer_dusk() {
        let mut g = Game::new(9);
        g.weather = Weather::Clear;
        let top = g.embark;
        let year = g.tick - g.tick % YEAR;
        g.tick = year + SEASON + 10 * DAY + DAY * 3 / 4;
        let s = ambience(&g, top);
        assert!(s.insects > 0.3 && s.birds == 0.0, "summer dusk: {:?}", s);
    }

    #[test]
    fn effects_are_placed_by_distance_and_level() {
        let cam = Pos::new(50, 50, 10);
        let (g0, p0) = place(cam, cam, (30, 15)).unwrap();
        assert!(g0 > 0.9 && p0.abs() < 0.01);
        let (g1, p1) = place(cam, Pos::new(75, 50, 10), (30, 15)).unwrap();
        assert!(g1 < g0 && p1 > 0.4);
        assert!(place(cam, Pos::new(50, 50, 13), (30, 15)).is_none(), "three levels down: unheard");
        assert!(place(cam, Pos::new(150, 50, 10), (30, 15)).is_none(), "off screen: unheard");
    }

    #[test]
    fn the_watch_hears_alarms_blows_and_beasts_once() {
        let mut g = Game::new(10);
        let cam = g.embark;
        g.alert = false;
        g.civ_alert = false;
        let mut w = Watch::default();
        let mut out = Vec::new();
        w.look(&g, cam, (30, 15), true, &mut out);
        assert!(out.is_empty(), "nothing is news at first");
        g.alert = true;
        w.look(&g, cam, (30, 15), true, &mut out);
        assert!(out.iter().any(|c| c.kind == SfxKind::Bell));
        out.clear();
        w.look(&g, cam, (30, 15), true, &mut out);
        assert!(!out.iter().any(|c| c.kind == SfxKind::Bell), "the bell rings once");
        g.sounds.push((cam, Sound::Blow));
        g.sounds.push((cam, Sound::Parry));
        g.sounds.push((Pos::new(cam.x, cam.y, cam.z + 5), Sound::Blow));
        out.clear();
        w.look(&g, cam, (30, 15), true, &mut out);
        assert_eq!(out.iter().filter(|c| matches!(c.kind, SfxKind::Thud | SfxKind::Clash)).count(), 2, "{:?}", out);
        assert!(spotlight_cues("An artifact", (255, 215, 60)).contains(&SfxKind::Chime));
        assert!(spotlight_cues("Siege!", (255, 70, 70)).contains(&SfxKind::Stinger(Tone::War)));
    }

    #[test]
    fn diggers_in_sight_make_picks_but_not_while_paused() {
        let mut g = Game::new(11);
        let cam = g.dwarves[0].pos;
        let target = Pos::new(cam.x + 1, cam.y, cam.z);
        g.dwarves[0].task = Task::Dig { target, stand: cam };
        let mut w = Watch::default();
        let mut out = Vec::new();
        for _ in 0..90 {
            w.look(&g, cam, (30, 15), false, &mut out);
        }
        assert!(out.is_empty(), "paused: {:?}", out);
        for _ in 0..90 {
            w.look(&g, cam, (30, 15), true, &mut out);
        }
        assert!(out.iter().any(|c| matches!(c.kind, SfxKind::Pick | SfxKind::Knock)), "digging is heard");
    }
}
