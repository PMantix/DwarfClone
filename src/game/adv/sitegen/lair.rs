//! A lair: a den dug under the highest ground one can walk to, the figure
//! itself waiting in it (the world's own `Figure`, so its death goes into
//! the legends), its hoard by the kills the legends count, the works of
//! legend it carried off from the places it laid waste, and the bones of
//! those it killed, named from the legends: figures whose death names it,
//! and the folk of the places it wasted or was driven from (A4, A13).
//!
//! A lair whose figure is dead lies empty, its hoard unguarded, unless the
//! land's hunters have moved in. A lair with no beast (a villain's tower)
//! is a ruin.

use super::{approach, hashed_name, loot, lore, occupant, open_ground, reachable, Ctx, Made};
use crate::creature::Species;
use crate::game::Game;
use crate::geom::Pos;
use crate::items::{ItemKind, ARTIFACT};
use crate::material::Mat;
use crate::overworld::{Race, Site};
use crate::rng::Rng;
use crate::world::TileKind;

/// The den: its floor, the deepest spot first.
fn dig_den(g: &mut Game, mouth: Pos, rng: &mut Rng) -> Vec<Pos> {
    let w = &g.world;
    let z = (mouth.z - rng.range(3, 5)).max(1);
    // In under the high ground, away from the map's edge.
    let dirs = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    let (dx, dy) = *dirs
        .iter()
        .max_by_key(|(dx, dy)| {
            let q = (mouth.x + dx * 5, mouth.y + dy * 5);
            let inb = q.0 > 5 && q.1 > 5 && q.0 < w.w - 6 && q.1 < w.h - 6;
            (inb, if inb { w.surface_z(q.0, q.1) } else { 0 })
        })
        .unwrap_or(&(1, 0));
    let r = rng.range(2, 4);
    let centre = Pos::new((mouth.x + dx * (r + 2)).clamp(r + 2, w.w - r - 3), (mouth.y + dy * (r + 2)).clamp(r + 2, w.h - r - 3), z);
    let mut floor = Vec::new();
    for oy in -r - 1..=r + 1 {
        for ox in -r - 1..=r + 1 {
            let d = ox * ox + oy * oy;
            if d <= r * r || d <= (r + 1) * (r + 1) && rng.chance(0.35) {
                floor.push(centre.offset(ox, oy, 0));
            }
        }
    }
    floor.sort_by_key(|p| (p.x - centre.x).pow(2) + (p.y - centre.y).pow(2));
    // The way down: a stair at the mouth, a passage to the den.
    let foot = Pos::new(mouth.x, mouth.y, z);
    let mut way = Vec::new();
    let mut p = foot;
    while p.x != centre.x || p.y != centre.y {
        way.push(p);
        p = p.offset((centre.x - p.x).signum(), (centre.y - p.y).signum(), 0);
    }
    for &q in floor.iter().chain(way.iter()) {
        carve(g, q, TileKind::Floor);
    }
    for zz in z..=mouth.z {
        let kind = if zz == mouth.z {
            TileKind::StairDown
        } else if zz == z {
            TileKind::StairUp
        } else {
            TileKind::StairUD
        };
        carve(g, Pos::new(mouth.x, mouth.y, zz), kind);
    }
    g.dig_version += 1;
    floor
}

/// Hollow out one tile, dry, with solid ground under it.
fn carve(g: &mut Game, p: Pos, kind: TileKind) {
    if !g.world.in_bounds(p) || p.z < 1 {
        return;
    }
    let below = p.down();
    let rock = g.world.t(below).mat;
    if !g.world.solid(below) && !g.world.t(below).kind.has_floor() && g.world.t(below).kind != TileKind::StairUD && g.world.t(below).kind != TileKind::StairUp {
        let t = g.world.t_mut(below);
        t.kind = TileKind::Wall;
        t.water = 0;
    }
    let t = g.world.t_mut(p);
    if t.kind == TileKind::Wall && t.mat == Mat::Air {
        t.mat = rock;
    }
    t.kind = kind;
    t.water = 0;
    t.magma = 0;
    t.aquifer = false;
    t.revealed = true;
    t.built_from = None;
    for (ox, oy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        let q = p.offset(ox, oy, 0);
        if g.world.in_bounds(q) {
            g.world.t_mut(q).aquifer = false;
        }
    }
}

/// A beast's lair.
pub fn make(g: &mut Game, ctx: &Ctx, site: &Site) -> Made {
    let fig = g.overworld.figures.iter().find(|f| f.lair == Some(site.id)).cloned();
    let Some(fig) = fig else { return super::ruin::make(g, ctx, site, true) };
    let mut made = Made::default();
    let (title, story) = lore::lair_story(&g.overworld, site, Some(&fig));
    made.title = title;
    made.story = story;
    let mut rng = Rng::new(ctx.seed ^ 0x1A1E);
    // The highest ground one can walk to: the den is under it.
    let start = g.embark;
    let w = &g.world;
    let ground: Vec<Pos> = reachable(w, start, 3000).into_iter().filter(|&p| open_ground(w, p) && p.x > 8 && p.y > 8 && p.x < w.w - 9 && p.y < w.h - 9).collect();
    let mouth = ground.iter().copied().max_by_key(|p| (p.z, -((p.x - w.w / 2).abs() + (p.y - w.h / 2).abs()))).unwrap_or(start);
    let den = dig_den(g, mouth, &mut rng);
    made.arrive = Some(approach(&g.world, mouth, 8, 14, &mut rng));
    let deep = den.first().copied().unwrap_or(mouth);
    let label = format!("{} the {}", fig.name, fig.kind);
    if fig.alive() {
        made.hostile = true;
        let id = occupant(g, &mut made, Species::Megabeast, deep, Some(label.clone()), true);
        if let Some(c) = g.creatures.get_mut(&id) {
            c.figure = Some(fig.id);
            c.breathes_fire = fig.kind == "dragon";
            c.desc = made.story.first().cloned();
        }
    } else {
        // The land's hunters have moved into the empty den.
        let sp = g.fauna().1;
        let mut r2 = Rng::new(ctx.seed ^ 0xD3);
        for _ in 0..r2.range(1, 4) {
            let p = *r2.pick(&den);
            occupant(g, &mut made, sp, p, None, true);
        }
    }
    // The hoard, by the kills the legends count.
    let n = (3 + fig.kills / 4).min(16) as i32;
    const HOARD: [(ItemKind, Mat); 6] = [(ItemKind::Craft, Mat::Gold), (ItemKind::Bar, Mat::Gold), (ItemKind::Gem, Mat::Amethyst), (ItemKind::Craft, Mat::Bronze), (ItemKind::Bar, Mat::Copper), (ItemKind::Helm, Mat::Bronze)];
    let heap: Vec<Pos> = den.iter().copied().take(den.len().max(2) / 2 + 1).collect();
    for _ in 0..n {
        let (kind, m) = *rng.pick(&HOARD);
        let p = *rng.pick(&heap);
        let q = rng.range(1, 5) as u8;
        loot(g, &mut made, kind, m, p, q, None);
    }
    // The works of legend from the places it laid waste.
    for (_, sname) in lore::wasted_by(&g.overworld, &fig.name) {
        let Some(s) = g.overworld.sites.iter().find(|s| s.name == sname).cloned() else { continue };
        for (_, _, art, what) in lore::artifacts_of(&g.overworld, &s) {
            let (kind, m) = lore::art_item(&what);
            loot(g, &mut made, kind, m, deep, ARTIFACT, Some(art));
        }
    }
    // The bones of its victims, named from the legends.
    let mut bones: Vec<String> = Vec::new();
    for f in g.overworld.figures.iter().filter(|f| f.cause.as_deref().map_or(false, |c| c.contains(&fig.name))) {
        bones.push(format!("the bones of {}, {}", f.name, f.cause.clone().unwrap_or_default()));
    }
    let off = " and was driven off.";
    let head = format!("{} the {} ", fig.name, fig.kind);
    let raids: Vec<(i32, String, bool)> = g
        .overworld
        .events
        .iter()
        .filter_map(|e| {
            let rest = e.text.strip_prefix(&head)?;
            if let Some(s) = rest.strip_prefix("laid waste to ") {
                return Some((e.year, s.trim_end_matches('.').to_string(), true));
            }
            let s = rest.strip_prefix("attacked ")?.strip_suffix(off)?;
            Some((e.year, s.to_string(), false))
        })
        .collect();
    for (k, (year, sname, wasted)) in raids.iter().enumerate() {
        let race = g.overworld.sites.iter().find(|s| &s.name == sname).and_then(|s| super::civ_race(&g.overworld, s.civ)).unwrap_or(Race::Human);
        for j in 0..if *wasted { 2 } else { 1 } {
            let name = hashed_name(ctx.seed, (k * 4 + j) as u64, race);
            let how = if *wasted { "when it laid waste to" } else { "defending" };
            bones.push(format!("the bones of {}, who died {} {} {}", name, how, sname, lore::in_year(*year)));
        }
    }
    let nameless = (fig.kills / 6).min(8);
    for (i, label) in bones.into_iter().take(12).enumerate() {
        let p = den[(i * 7 + 3) % den.len()];
        loot(g, &mut made, ItemKind::BodyPart, Mat::Bone, p, 0, Some(label));
    }
    for i in 0..nameless {
        let p = den[(i as usize * 5 + 1) % den.len()];
        let label = ["the gnawed bones of a traveller", "the gnawed bones of a hunter", "a cracked skull"][i as usize % 3];
        loot(g, &mut made, ItemKind::BodyPart, Mat::Bone, p, 0, Some(label.to_string()));
    }
    made
}

#[cfg(test)]
mod tests {
    use super::super::super::{SiteKey, HERO_ID};
    use super::super::town::tests::{visit, world};
    use super::*;

    #[test]
    fn the_dragon_waits_with_its_hoard_and_its_victims() {
        let ow = world();
        // The beast with the most kills that still keeps its lair.
        let f = ow.figures.iter().filter(|f| f.alive() && f.lair.is_some()).max_by_key(|f| f.kills).expect("a living beast").clone();
        let lair = f.lair.unwrap();
        let a = visit(lair, Race::Dwarf, None, 5);
        let v = a.visit.as_ref().unwrap();
        let g = &v.g;
        let beast = g.creatures.values().find(|c| c.figure == Some(f.id)).expect("the beast is home");
        assert!(beast.siege && beast.target == Some(HERO_ID), "and it hunts the hero");
        assert!(!g.world.outside(beast.pos), "deep in its den");
        let hoard = g.items.values().filter(|it| it.carried_by.is_none() && matches!(it.kind, ItemKind::Craft | ItemKind::Bar | ItemKind::Gem | ItemKind::Helm)).count();
        assert!(hoard >= 3, "a hoard of {}", hoard);
        let named: Vec<String> = g.items.values().filter(|it| it.kind == ItemKind::BodyPart).filter_map(|it| it.label.clone()).filter(|l| l.contains(" who died ") || l.contains(", slain")).collect();
        let history = ow.events.iter().any(|e| e.text.starts_with(&format!("{} the {} ", f.name, f.kind)) && (e.text.contains("laid waste") || e.text.contains("driven off")));
        if history {
            assert!(!named.is_empty(), "the bones name its victims");
        }
        // The arrival tells who lives here.
        assert!(g.log.iter().any(|e| e.text.contains(&f.name)), "{:?}", g.log.iter().map(|e| &e.text).collect::<Vec<_>>());
        assert_eq!(v.key, SiteKey::Site(lair));
    }
}
