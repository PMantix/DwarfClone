use super::*;
use crate::dwarf::{Mood, MoodNeed};
use crate::items::ARTIFACT;

fn run(g: &mut Game, ticks: u64) {
    for _ in 0..ticks {
        g.step();
    }
}

fn place_near(g: &mut Game, kind: BuildingKind) -> u32 {
    let e = g.embark;
    for r in 1..25 {
        for dy in -r..=r {
            for dx in -r..=r {
                for dz in -1..=1 {
                    let p = e.offset(dx, dy, dz);
                    if g.can_place(kind, p) && g.place_building(kind, p) {
                        return g.world.t(p).building;
                    }
                }
            }
        }
    }
    panic!("no room for {:?}", kind);
}

#[test]
fn enclosed_rooms_are_detected_and_assigned() {
    let mut g = Game::new(90);
    let e = g.embark;
    // Hollow out a 4x3 chamber deep under the wagon, sealed by rock.
    let solid = |g: &Game, c: Pos| (-1..=3).all(|dy| (-1..=4).all(|dx| g.world.get(c.offset(dx, dy, 0)).map_or(false, |t| t.kind == TileKind::Wall && t.water == 0)));
    let c = (0..30)
        .flat_map(|r| [e.offset(r, 0, 0), e.offset(-r, 0, 0), e.offset(0, r, 0), e.offset(0, -r, 0)])
        .map(|p| Pos::new(p.x, p.y, g.world.surface_z(p.x, p.y) - 6))
        .find(|&c| solid(&g, c))
        .expect("solid rock somewhere near");
    for dy in 0..3 {
        for dx in 0..4 {
            let t = g.world.t_mut(c.offset(dx, dy, 0));
            t.kind = TileKind::Floor;
            t.revealed = true;
        }
    }
    let (tiles, bare) = g.room_at(c).expect("room not detected");
    assert_eq!(tiles.len(), 12);
    // A bed inside raises the value; an opening to a corridor breaks the room.
    assert!(g.place_building(BuildingKind::Bed, c));
    let bed = g.world.t(c).building;
    g.buildings.get_mut(&bed).unwrap().built = true;
    g.buildings.get_mut(&bed).unwrap().mat = Mat::Wood;
    let (_, furnished) = g.room_at(c).unwrap();
    assert!(furnished > bare);
    g.assign_rooms();
    assert!(g.buildings[&bed].owner.is_some(), "bed not assigned");
    // The noble should get it.
    let owner = g.buildings[&bed].owner.unwrap();
    assert!(g.dwarves.iter().any(|d| d.id == owner && d.noble.is_some()));
    let leak = c.offset(4, 1, 0);
    let t = g.world.t_mut(leak);
    t.kind = TileKind::StairUD;
    assert!(g.room_at(c).is_none(), "stairs should break the room");
}

#[test]
fn dwarves_dine_at_tables_and_make_friends() {
    let mut g = Game::new(91);
    g.creatures.clear();
    let e = g.embark;
    let t = g.spawn_item(ItemKind::Table, Mat::Granite, e);
    let c = g.spawn_item(ItemKind::Chair, Mat::Granite, e);
    let _ = (t, c);
    let table = place_near(&mut g, BuildingKind::Table);
    let tpos = g.buildings[&table].pos;
    let chair_pos = DIRS8
        .iter()
        .map(|&(dx, dy)| tpos.offset(dx, dy, 0))
        .find(|&p| g.can_place(BuildingKind::Chair, p))
        .unwrap();
    assert!(g.place_building(BuildingKind::Chair, chair_pos));
    run(&mut g, DAY);
    assert!(g.buildings.values().all(|b| b.built), "table/chair not built");
    for d in g.dwarves.iter_mut() {
        d.hunger = HUNGRY + 1;
    }
    run(&mut g, 2 * DAY);
    let at_table = g.dwarves.iter().filter(|d| d.thoughts.iter().any(|t| t.text == "ate at a table")).count();
    let friends = g.dwarves.iter().map(|d| d.relations.len()).sum::<usize>();
    eprintln!("ate at table: {}, relations: {}", at_table, friends);
    assert!(at_table >= 1, "nobody used the table");
    run(&mut g, 8 * DAY);
    let friends = g.dwarves.iter().map(|d| d.relations.len()).sum::<usize>();
    assert!(friends > 0, "nobody talked to anybody");
}

#[test]
fn strange_mood_makes_an_artifact() {
    let mut g = Game::new(92);
    g.creatures.clear();
    let e = g.embark;
    for _ in 0..4 {
        g.spawn_item(ItemKind::Log, Mat::Wood, e);
    }
    g.spawn_item(ItemKind::Bone, Mat::Bone, e);
    g.spawn_item(ItemKind::Boulder, Mat::Amethyst, e);
    let shop = place_near(&mut g, BuildingKind::Carpenter);
    run(&mut g, DAY);
    assert!(g.buildings[&shop].built);
    let i = g.dwarves.iter().position(|d| d.noble.is_none()).unwrap();
    g.dwarves[i].mood = Some(Mood {
        labor: Labor::Carpentry,
        shop: None,
        needs: vec![MoodNeed::Log, MoodNeed::Log, MoodNeed::Bone],
        items: Vec::new(),
        delivered: 0,
        started: g.tick,
        work_left: 0,
    });
    run(&mut g, 4 * DAY);
    let art = g.items.values().find(|it| it.quality == ARTIFACT);
    eprintln!("artifact: {:?}", art.map(|a| a.name()));
    assert!(art.is_some(), "no artifact: {:?}", g.log.iter().rev().take(5).map(|l| &l.text).collect::<Vec<_>>());
    assert!(g.dwarves[i].mood.is_none());
    assert!(g.wealth() > 1000);
}

#[test]
fn mandates_resolve_and_misery_breaks_dwarves() {
    let mut g = Game::new(93);
    g.creatures.clear();
    let noble = g.dwarves.iter().find(|d| d.noble.is_some()).unwrap().id;
    g.mandate = Some(Mandate { noble, kind: ItemKind::Barrel, mat: None, count: 1, baseline: 0, deadline: g.tick + SEASON });
    g.check_mandate();
    assert!(g.mandate.is_none(), "embark barrels should satisfy a 1-barrel mandate");

    let i = g.dwarves.iter().position(|d| d.noble.is_none()).unwrap();
    g.dwarves[i].stress = 5000;
    g.dwarves[i].personality.temper = -50;
    run(&mut g, 3 * DAY);
    assert!(g.dwarves[i].insane, "misery didn't break the dwarf");
}

#[test]
fn melancholy_can_lift_with_a_friend_alive() {
    let mut g = Game::new(11);
    g.creatures.retain(|_, c| c.tame);
    let friend = g.dwarves[1].id;
    let mut d = std::mem::take(&mut g.dwarves[0]);
    g.go_insane(&mut d, "has been stricken by melancholy");
    assert!(d.insane && d.melancholy);
    d.relations.push((friend, 60));
    d.stress = 0;
    let mut days = 0;
    while d.insane && days < 120 {
        for _ in 0..DAY {
            g.tick += 1;
            g.psyche_tick(&mut d);
            d.stress = d.stress.min(50);
        }
        days += 1;
    }
    assert!(!d.insane, "still melancholic after {} days", days);
    g.dwarves[0] = d;
    assert!(g.log.iter().any(|l| l.text.contains("come back out of melancholy")));
    // A failed mood's madness doesn't lift.
    let mut e = std::mem::take(&mut g.dwarves[2]);
    g.go_insane(&mut e, "has gone stark raving mad, unable to finish their work");
    assert!(e.insane && !e.melancholy);
}

#[test]
fn a_raving_dwarf_still_drinks_when_desperate() {
    let mut g = Game::new(11);
    g.creatures.retain(|_, c| c.tame);
    g.items.retain(|_, it| it.kind != ItemKind::Drink);
    let mut d = std::mem::take(&mut g.dwarves[0]);
    g.go_insane(&mut d, "has gone stark raving mad, unable to finish their work");
    assert!(d.insane && !d.melancholy);
    d.thirst = THIRSTY + 7000;
    d.hunger = 0;
    let at = d.pos;
    let id = d.id;
    g.dwarves[0] = d;
    // A drink ten tiles off, and nothing else to drink.
    let spot = (0..30).flat_map(|r| [at.offset(10, r, 0), at.offset(-10, r, 0), at.offset(10, -r, 0), at.offset(-10, -r, 0)])
        .find(|&p| g.world.walkable(p) && g.world.t(p).water == 0)
        .expect("somewhere to put a drink");
    g.spawn_item(ItemKind::Drink, Mat::Stoutcap, spot);
    run(&mut g, 600);
    let d = g.dwarves.iter().find(|d| d.id == id).unwrap();
    assert!(d.alive, "the raving dwarf died");
    assert!(d.insane && !d.melancholy, "still raving");
    assert!(d.thirst < 1000, "never drank (thirst {}, task {:?})", d.thirst, d.task);
}
