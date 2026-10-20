//! A ruin, or a living dark fortress: the old site's buildings with their
//! walls broken (burnt, if a dragon took it), the history its people
//! carved into the hall, what they left behind by their numbers, the
//! bones of those who died defending it (a fallen ruler named), any work
//! of legend made there that nobody carried off, and whoever moved in
//! since: the dead on evil land, goblins in a goblin hold, bandits near a
//! road, or the land's hunters denned in the shell.

use super::town::{build_mat, engrave, lay_out};
use super::{approach, arm, civ_race, hashed_name, loot, lore, occupant, soldier, Ctx, Made};
use crate::creature::Species;
use crate::game::Game;
use crate::geom::Pos;
use crate::items::{ItemKind, ARTIFACT};
use crate::material::Mat;
use crate::overworld::{Race, Site, SiteKind};
use crate::rng::Rng;
use crate::world::TileKind;

/// What a people left lying about, by race.
fn leavings(race: Race) -> &'static [(ItemKind, Mat)] {
    match race {
        Race::Dwarf => &[(ItemKind::Axe, Mat::Iron), (ItemKind::Pick, Mat::Iron), (ItemKind::Craft, Mat::Gold), (ItemKind::Gem, Mat::Amethyst), (ItemKind::Helm, Mat::Iron), (ItemKind::Bar, Mat::Copper), (ItemKind::Breastplate, Mat::Copper)],
        Race::Human => &[(ItemKind::Sword, Mat::Copper), (ItemKind::Spear, Mat::Iron), (ItemKind::Shield, Mat::Wood), (ItemKind::Craft, Mat::Bronze), (ItemKind::Helm, Mat::Bronze), (ItemKind::Bar, Mat::Iron)],
        Race::Elf => &[(ItemKind::Spear, Mat::Wood), (ItemKind::Crossbow, Mat::Wood), (ItemKind::Craft, Mat::Wood), (ItemKind::Gem, Mat::Amethyst), (ItemKind::Shield, Mat::Wood)],
        Race::Goblin => &[(ItemKind::Spear, Mat::Iron), (ItemKind::Craft, Mat::Bone), (ItemKind::Helm, Mat::Iron), (ItemKind::Bar, Mat::Iron), (ItemKind::Craft, Mat::Gold)],
    }
}

/// A ruined site (`broken`), or a dark fortress still standing, every
/// soul in it hostile.
pub fn make(g: &mut Game, ctx: &Ctx, site: &Site, broken: bool) -> Made {
    let mut made = Made::default();
    let (title, story) = if broken {
        lore::ruin_story(&g.overworld, site)
    } else {
        let (t, mut v) = lore::town_story(&g.overworld, site, super::super::peoples::Stance::Hostile);
        v.pop();
        v.push("Every soul in it would see you dead.".to_string());
        (t, v)
    };
    made.title = title;
    made.story = story;
    made.hostile = true;
    let mut rng = Rng::new(ctx.seed ^ 0x2015);
    let race = civ_race(&g.overworld, site.civ).unwrap_or(Race::Human);
    let fall = lore::fall_of(&g.overworld, site);
    let burnt = matches!(&fall, Some((lore::Fell::Beast { kind, .. }, _, _)) if kind == "dragon");
    let center = g.embark;
    let mat = build_mat(site.civ.map(|_| race), site.kind);
    let plan = lay_out(g, center, site.kind, mat, &mut rng);
    let events: Vec<usize> = lore::about(&g.overworld, &site.name).into_iter().map(|(i, _, _)| i).collect();
    if let Some(h) = plan.hall {
        engrave(g, &h, &events);
    }
    let rooms: Vec<Pos> = plan.hall.iter().chain(plan.houses.iter()).flat_map(|r| r.inside()).collect();
    let hall: Vec<Pos> = plan.hall.map_or_else(|| rooms.clone(), |h| h.inside());
    // Broken walls, rubble, and ash where it burned.
    if broken {
        let gone = if burnt { 0.6 } else { 0.4 };
        for r in plan.hall.iter().chain(plan.houses.iter()) {
            for p in r.border() {
                if g.world.t(p).kind == TileKind::Wall && rng.chance(gone) {
                    let t = g.world.t_mut(p);
                    t.kind = TileKind::Floor;
                    t.smooth = 0;
                    t.engraving = 0;
                    if rng.chance(0.3) {
                        super::scenery(g, &mut made, ItemKind::Boulder, mat, p);
                    }
                }
            }
        }
        if burnt {
            for _ in 0..rooms.len() / 6 {
                let p = *rng.pick(&rooms);
                super::scenery(g, &mut made, ItemKind::Ash, Mat::Wood, p);
            }
        }
        g.dig_version += 1;
    }
    made.arrive = Some(approach(&g.world, center, 10, 16, &mut rng));
    let spots = if rooms.is_empty() { vec![center] } else { rooms.clone() };
    // What they left, by their numbers.
    let things = leavings(race);
    let n = (2 + site.pop / 25).clamp(2, 10);
    for _ in 0..n {
        let (kind, m) = *rng.pick(things);
        let p = *rng.pick(&spots);
        let q = rng.range(0, 3) as u8;
        loot(g, &mut made, kind, m, p, q, None);
    }
    // Its works of legend, unless a beast carried them off to its lair
    // or the sackers took them (one in two).
    let carried = match &fall {
        Some((lore::Fell::Beast { name, .. }, _, _)) => lore::beast_lair(&g.overworld, name).is_some(),
        _ => false,
    };
    for (year, _, art, what) in lore::artifacts_of(&g.overworld, site) {
        let sacked = matches!(fall, Some((lore::Fell::Army { .. }, _, _))) && crate::game::names::hash(&[ctx.seed, year as u64]) % 2 == 0;
        if carried || sacked || !broken {
            continue;
        }
        let (kind, m) = lore::art_item(&what);
        let p = *rng.pick(&hall);
        loot(g, &mut made, kind, m, p, ARTIFACT, Some(art));
    }
    // The bones of those who fell defending it.
    if broken {
        let year = fall.as_ref().map(|f| f.1);
        if let Some((_, y, Some(ruler))) = &fall {
            let p = *rng.pick(&hall);
            loot(g, &mut made, ItemKind::BodyPart, Mat::Bone, p, 0, Some(format!("the bones of {}, who fell defending {} {}", ruler, site.name, lore::in_year(*y))));
        }
        for i in 0..rng.range(2, 6) {
            let p = *rng.pick(&spots);
            let name = hashed_name(ctx.seed, 300 + i as u64, race);
            let when = year.map_or(String::new(), |y| format!(" {}", lore::in_year(y)));
            loot(g, &mut made, ItemKind::BodyPart, Mat::Bone, p, 0, Some(format!("the bones of {}, who died at {}{}", name, site.name, when)));
        }
    }
    // Who lives here now.
    let (sp, k, label): (Species, i32, Option<&str>) = if !broken {
        (soldier(race), rng.range(6, 11), None)
    } else if site.kind == SiteKind::Lair || g.evil {
        (Species::Undead, rng.range(3, 7), None)
    } else if site.kind == SiteKind::DarkFortress {
        (Species::Goblin, rng.range(3, 7), None)
    } else if g.overworld.sites.iter().any(|s| s.id != site.id && !s.ruined && s.civ.is_some() && (s.x - site.x).abs().max((s.y - site.y).abs()) <= 3) {
        (Species::Human, rng.range(3, 6), Some("bandit"))
    } else {
        (g.fauna().1, rng.range(2, 5), None)
    };
    let den = hall.get(hall.len() / 2).copied().unwrap_or(center);
    for i in 0..k {
        let p = g.spot_near(den, 3);
        let label = label.map(|l| format!("{}, {}", hashed_name(ctx.seed, 500 + i as u64, Race::Human), l));
        let id = occupant(g, &mut made, sp, p, label, true);
        if sp.person() {
            arm(g, id, if i % 2 == 0 { ItemKind::Spear } else { ItemKind::Sword }, Mat::Iron);
        }
    }
    made
}

#[cfg(test)]
mod tests {
    use super::super::super::peoples::Stance;
    use super::super::super::HERO_ID;
    use super::super::town::tests::visit;
    use super::*;

    #[test]
    fn a_ruin_has_walls_loot_and_tenants() {
        let ow = super::super::town::tests::world();
        let s = ow.sites.iter().find(|s| s.ruined && s.kind != SiteKind::Lair).expect("a ruin in world 5").clone();
        let a = visit(s.id, Race::Dwarf, None, 9);
        let v = a.visit.as_ref().unwrap();
        let g = &v.g;
        let walls = g.world.tiles.iter().filter(|t| t.built_from.is_some() && t.kind == TileKind::Wall).count();
        assert!(walls > 10, "{} walls stand", walls);
        assert!(g.items.values().filter(|it| it.carried_by.is_none() && it.kind != ItemKind::Boulder && it.kind != ItemKind::Ash).count() >= 4, "loot and bones");
        let tenants: Vec<_> = g.creatures.values().filter(|c| c.alive && c.siege && c.target == Some(HERO_ID)).collect();
        assert!(tenants.len() >= 2, "someone lives here, and hunts the hero");
        // It tells its past.
        let (title, story) = lore::story(&g.overworld, v.key, Stance::Hostile, &g.fortress_name);
        assert!(title.contains(&s.name), "{}", title);
        assert!(!story.is_empty());
        assert!(g.log.iter().any(|e| e.text.contains(&s.name)), "the arrival tells it");
    }
}
