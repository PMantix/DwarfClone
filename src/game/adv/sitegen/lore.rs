//! Every place has a past: what the legends say happened at a site, told
//! as the arrival card and the first lines of the log ("The burnt halls of
//! Kogsak, where Ashmaw the dragon slew Urist Kadol in the year 87"). It
//! reads `Overworld.events`, which history writes as plain sentences, by
//! the shapes those sentences have (overworld.rs `simulate_year`, `battle`).

use super::super::peoples::Stance;
use super::super::SiteKey;
use crate::items::ItemKind;
use crate::material::Mat;
use crate::overworld::{Figure, Overworld, Race, Site, SiteKind};

/// "an elk", "a dragon".
pub fn a_or_an(name: &str) -> String {
    if name.starts_with("the ") {
        return name.to_string();
    }
    let vowel = name.chars().next().map_or(false, |c| "aeiouAEIOU".contains(c));
    format!("{} {}", if vowel { "an" } else { "a" }, name)
}

/// "in the year 87".
pub fn in_year(y: i32) -> String {
    if y <= 0 {
        "before the count of years".to_string()
    } else {
        format!("in the year {}", y)
    }
}

/// "twice", "five times".
fn times(n: usize) -> String {
    match n {
        2 => "twice".into(),
        3 => "three times".into(),
        4 => "four times".into(),
        5 => "five times".into(),
        n => format!("{} times", n),
    }
}

/// The events that name `name`, oldest first, with their indices.
pub fn about<'a>(ow: &'a Overworld, name: &str) -> Vec<(usize, i32, &'a str)> {
    if name.is_empty() {
        return Vec::new();
    }
    ow.events.iter().enumerate().filter(|(_, e)| e.text.contains(name)).map(|(i, e)| (i, e.year, e.text.as_str())).collect()
}

/// Who brought a site down.
#[derive(Clone, Debug, PartialEq)]
pub enum Fell {
    /// A beast of legend: its name and kind.
    Beast { name: String, kind: String },
    /// An army: "the goblins of the Vile Host".
    Army { who: String },
}

/// How a site fell: by whom, in what year, and the ruler who died
/// defending it, if one did.
pub fn fall_of(ow: &Overworld, s: &Site) -> Option<(Fell, i32, Option<String>)> {
    let waste = format!(" laid waste to {}.", s.name);
    let razed = format!(" razed {}.", s.name);
    let (fell, year) = ow.events.iter().rev().find_map(|e| {
        if let Some(who) = e.text.strip_suffix(&waste) {
            let (name, kind) = who.rsplit_once(" the ")?;
            return Some((Fell::Beast { name: name.to_string(), kind: kind.to_string() }, e.year));
        }
        let head = e.text.strip_suffix(&razed)?;
        let who = head.split_once(": the ").map_or(head, |(_, w)| w);
        Some((Fell::Army { who: format!("the {}", who) }, e.year))
    })?;
    let defend = format!(" fell defending {}.", s.name);
    let ruler = ow.events.iter().find(|e| e.year == year && e.text.ends_with(&defend)).and_then(|e| e.text.strip_suffix(&defend)).map(|r| r.to_string());
    Some((fell, year, ruler))
}

/// A work of history made at a site: (year, maker, its name, what it is).
pub fn artifacts_of(ow: &Overworld, s: &Site) -> Vec<(i32, String, String, String)> {
    let tail = format!(", in {}.", s.name);
    ow.events
        .iter()
        .filter_map(|e| {
            let head = e.text.strip_suffix(&tail)?;
            let (maker, rest) = head.split_once(" created ")?;
            let (art, what) = rest.rsplit_once(", a ")?;
            Some((e.year, maker.to_string(), art.to_string(), what.to_string()))
        })
        .collect()
}

/// What an artifact of the legends ("steel axe", "gold crown") is made as.
pub fn art_item(what: &str) -> (ItemKind, Mat) {
    let mat = if what.contains("steel") {
        Mat::Steel
    } else if what.contains("iron") {
        Mat::Iron
    } else if what.contains("gold") {
        Mat::Gold
    } else if what.contains("marble") {
        Mat::Marble
    } else if what.contains("amethyst") {
        Mat::Amethyst
    } else {
        Mat::Bronze
    };
    let kind = if what.contains("axe") {
        ItemKind::Axe
    } else if what.contains("shield") {
        ItemKind::Shield
    } else if what.contains("statue") {
        ItemKind::Statue
    } else {
        ItemKind::Craft
    };
    (kind, mat)
}

/// "dwarven", "elven".
pub fn race_adj(r: Race) -> &'static str {
    match r {
        Race::Dwarf => "dwarven",
        Race::Human => "human",
        Race::Elf => "elven",
        Race::Goblin => "goblin",
    }
}

/// What a site's buildings are called, by its kind.
fn buildings(kind: SiteKind) -> &'static str {
    match kind {
        SiteKind::Fortress => "halls",
        SiteKind::Town => "streets",
        SiteKind::Hamlet => "huts",
        SiteKind::Retreat => "groves",
        SiteKind::DarkFortress => "towers",
        SiteKind::Lair => "tower",
    }
}

/// When and by whom it was founded.
fn founding(ow: &Overworld, s: &Site) -> Option<String> {
    let tail = format!(" founded the {} of {}.", s.kind.name(), s.name);
    if let Some(e) = ow.events.iter().find(|e| e.text.ends_with(&tail)) {
        let who = e.text.strip_suffix(&tail).unwrap_or("");
        return Some(format!("{} founded it {}.", who, in_year(e.year)));
    }
    (s.founded > 0).then(|| format!("It was founded {}.", in_year(s.founded)))
}

/// The other things history tells of a place: beasts driven from its
/// gates, battles at its walls, the works made there.
fn deeds_at(ow: &Overworld, s: &Site, skip_year: Option<i32>) -> Vec<String> {
    let off = format!(" attacked {} and was driven off.", s.name);
    let gates = " broke upon the gates of the mountainhome.";
    let repulsed = " were repulsed.";
    // Who came against it, and in which years (told once each).
    let mut foes: Vec<(String, bool, Vec<i32>)> = Vec::new();
    for e in ow.events.iter() {
        if Some(e.year) == skip_year && e.text.contains(&s.name) {
            continue;
        }
        let (who, beast) = if let Some(b) = e.text.strip_suffix(&off) {
            (b.to_string(), true)
        } else if e.text.contains(&format!("of {}:", s.name)) && (e.text.ends_with(gates) || e.text.ends_with(repulsed)) {
            let w = e.text.split_once(": the ").map_or("its foes", |(_, w)| w);
            (format!("the {}", w.trim_end_matches(gates).trim_end_matches(repulsed).trim()), false)
        } else {
            continue;
        };
        match foes.iter_mut().find(|f| f.0 == who) {
            Some(f) => f.2.push(e.year),
            None => foes.push((who, beast, vec![e.year])),
        }
    }
    let mut v = Vec::new();
    for (who, beast, years) in foes {
        let what = if beast { "was driven from its gates" } else { "were thrown back from its walls" };
        v.push(match years.as_slice() {
            [y] => format!("{} {} {}.", cap(&who), what, in_year(*y)),
            [a, .., b] => format!("{} {} {}, between the years {} and {}.", cap(&who), what, times(years.len()), a, b),
            [] => continue,
        });
    }
    for (y, maker, art, what) in artifacts_of(ow, s) {
        v.push(format!("{} made {}, a {}, here {}.", maker, art, what, in_year(y)));
    }
    v
}

/// The arrival card of a ruin: its title and its story, the headline first.
pub fn ruin_story(ow: &Overworld, s: &Site) -> (String, Vec<String>) {
    let fall = fall_of(ow, s);
    let adj = match &fall {
        Some((Fell::Beast { kind, .. }, _, _)) if kind == "dragon" => "burnt",
        Some((Fell::Beast { .. }, _, _)) => "broken",
        Some((Fell::Army { .. }, _, _)) => "razed",
        None if s.kind == SiteKind::Lair => "black",
        None => "silent",
    };
    let title = format!("The {} {} of {}", adj, buildings(s.kind), s.name);
    let lower = format!("the {} {} of {}", adj, buildings(s.kind), s.name);
    let mut v = Vec::new();
    let head = match &fall {
        Some((Fell::Beast { name, kind }, y, Some(r))) => format!("{}, where {} the {} slew {} {}.", cap(&lower), name, kind, r, in_year(*y)),
        Some((Fell::Beast { name, kind }, y, None)) => format!("{}, laid waste by {} the {} {}.", cap(&lower), name, kind, in_year(*y)),
        Some((Fell::Army { who }, y, Some(r))) => format!("{}, where {} fell when {} sacked it {}.", cap(&lower), r, who, in_year(*y)),
        Some((Fell::Army { who }, y, None)) => format!("{}, razed by {} {}.", cap(&lower), who, in_year(*y)),
        None => format!("{}. No song tells how it fell.", cap(&lower)),
    };
    v.push(head);
    if let Some(f) = founding(ow, s) {
        v.push(f);
    }
    v.extend(deeds_at(ow, s, fall.as_ref().map(|f| f.1)));
    if let Some((Fell::Beast { name, .. }, _, _)) = &fall {
        if !artifacts_of(ow, s).is_empty() && beast_lair(ow, name).is_some() {
            v.push(format!("What treasure it held, {} carried off to its lair.", name));
        }
    }
    (title, v)
}

/// The figure named `name` that keeps a lair, and the lair's site id.
pub fn beast_lair(ow: &Overworld, name: &str) -> Option<(u32, u32)> {
    ow.figures.iter().find(|f| f.name == name && f.lair.is_some()).map(|f| (f.id, f.lair.unwrap_or(0)))
}

/// The sites a beast laid waste to, and the years.
pub fn wasted_by(ow: &Overworld, name: &str) -> Vec<(i32, String)> {
    let head = format!("{} the ", name);
    ow.events
        .iter()
        .filter(|e| e.text.starts_with(&head) && e.text.contains(" laid waste to "))
        .filter_map(|e| e.text.split_once(" laid waste to ").map(|(_, s)| (e.year, s.trim_end_matches('.').to_string())))
        .collect()
}

/// The arrival card of a beast's lair.
pub fn lair_story(ow: &Overworld, s: &Site, f: Option<&Figure>) -> (String, Vec<String>) {
    let Some(f) = f else { return ruin_story(ow, s) };
    let who = format!("{} the {}", f.name, f.kind);
    let mut v = Vec::new();
    let wasted = wasted_by(ow, &f.name);
    if let (Some(y), Some(cause)) = (f.died, f.cause.as_ref()) {
        v.push(format!("The empty lair of {}, {} {}.", who, cause, in_year(y)));
    } else if let Some((y, site)) = wasted.last() {
        v.push(format!("The lair of {}, who laid waste to {} {}.", who, site, in_year(*y)));
    } else {
        v.push(format!("The lair of {}.", who));
    }
    let title = if f.died.is_some() { format!("The empty lair of {}", who) } else { format!("The lair of {}", who) };
    for (_, y, text) in about(ow, &f.name).into_iter().rev().take(4).collect::<Vec<_>>().into_iter().rev() {
        v.push(format!("{}, {}", cap(&in_year(y)), lower_first(text)));
    }
    if f.kills > 0 && f.died.is_none() {
        v.push(format!("The legends count {} slain by it.", f.kills));
    }
    (title, v)
}

/// The arrival card of a living site, and how its folk take the hero.
pub fn town_story(ow: &Overworld, s: &Site, stance: Stance) -> (String, Vec<String>) {
    let civ = s.civ.and_then(|c| ow.civs.get(c as usize));
    let adj = civ.map_or("", |c| race_adj(c.race));
    let title = format!("{}, {} {}", s.name, a_or_an(adj), s.kind.name());
    let mut v = Vec::new();
    v.push(match civ {
        Some(c) => format!("{}, {} {} of the {}, home to {} souls.", s.name, a_or_an(adj), s.kind.name(), c.name, s.pop),
        None => format!("{}, a {} of {} souls.", s.name, s.kind.name(), s.pop),
    });
    if let Some(f) = founding(ow, s) {
        v.push(f);
    }
    let deeds = deeds_at(ow, s, None);
    let n = deeds.len();
    v.extend(deeds.into_iter().skip(n.saturating_sub(3)));
    v.push(
        match stance {
            Stance::Home => "You are home.",
            Stance::Friendly => "The folk here nod to you as you pass.",
            Stance::Wary => "The folk here watch you, and say little.",
            Stance::Hostile => "The guards know you for an enemy!",
        }
        .to_string(),
    );
    (title, v)
}

/// The arrival card of the open land: its region, and what is near.
pub fn wild_story(ow: &Overworld, x: i32, y: i32, name: &str) -> (String, Vec<String>) {
    let mut v = Vec::new();
    if ow.w <= 1 || x < 0 || y < 0 || x >= ow.w || y >= ow.h {
        return (cap(name), vec![format!("{}.", cap(name))]);
    }
    let biome = ow.cell(x, y).biome.name();
    v.push(format!("{}: {}{}.", cap(name), biome, if ow.cell(x, y).evil { ", and something is wrong with the land" } else { "" }));
    for f in ow.beasts_near(x, y, 4).into_iter().take(1).filter_map(|f| ow.figures.get(f as usize)) {
        v.push(format!("They say {} the {} hunts these lands.", f.name, f.kind));
    }
    for s in ow.sites.iter().filter(|s| s.ruined && (s.x - x).abs().max((s.y - y).abs()) <= 2).take(1) {
        v.push(format!("The ruins of {} lie near.", s.name));
    }
    (cap(name), v)
}

/// What the legends say of a site, for the look panel and the arrival
/// card: (title, lines).
pub fn story(ow: &Overworld, key: SiteKey, stance: Stance, name: &str) -> (String, Vec<String>) {
    match key {
        SiteKey::Wild(x, y) => wild_story(ow, x, y, name),
        SiteKey::Fort(_) => (cap(name), Vec::new()),
        SiteKey::Site(id) => {
            let Some(s) = ow.sites.iter().find(|s| s.id == id) else { return (cap(name), Vec::new()) };
            match s.kind {
                SiteKind::Lair => lair_story(ow, s, ow.figures.iter().find(|f| f.lair == Some(s.id))),
                _ if s.ruined => ruin_story(ow, s),
                _ => town_story(ow, s, stance),
            }
        }
    }
}

pub fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn lower_first(s: &str) -> String {
    // Names stay as they are; only "The" at the start is lowered.
    match s.strip_prefix("The ") {
        Some(rest) => format!("the {}", rest),
        None => s.to_string(),
    }
}
