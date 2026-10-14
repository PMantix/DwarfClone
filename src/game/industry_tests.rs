use super::*;

fn run(g: &mut Game, ticks: u64) {
    for _ in 0..ticks {
        g.step();
    }
}

fn place_near(g: &mut Game, kind: BuildingKind) -> u32 {
    let e = g.embark;
    for r in 2..25 {
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

fn count(g: &Game, k: ItemKind, m: Option<Mat>) -> usize {
    g.items.values().filter(|it| it.kind == k && m.map_or(true, |m| it.mat == m)).count()
}

#[test]
fn production_chains() {
    let mut g = Game::new(77);
    g.creatures.clear();
    let e = g.embark;
    for _ in 0..8 {
        g.spawn_item(ItemKind::Boulder, Mat::Granite, e);
    }
    for _ in 0..3 {
        g.spawn_item(ItemKind::Boulder, Mat::Hematite, e);
    }
    for _ in 0..5 {
        g.spawn_item(ItemKind::Log, Mat::Wood, e);
    }
    g.spawn_item(ItemKind::Boulder, Mat::Limestone, e);
    let c = g.spawn_item(ItemKind::Carcass, Mat::Flesh, e);
    g.items.get_mut(&c).unwrap().label = Some("deer".into());

    let furnace = place_near(&mut g, BuildingKind::WoodFurnace);
    let smelter = place_near(&mut g, BuildingKind::Smelter);
    let forge = place_near(&mut g, BuildingKind::Forge);
    let kitchen = place_near(&mut g, BuildingKind::Kitchen);
    let butcher = place_near(&mut g, BuildingKind::Butcher);
    let crafts = place_near(&mut g, BuildingKind::Craftsdwarf);
    run(&mut g, 3 * DAY);
    for b in [furnace, smelter, forge, kitchen, butcher, crafts] {
        assert!(g.buildings[&b].built, "{} not built", g.buildings[&b].kind.name());
    }
    for _ in 0..4 {
        g.add_order(furnace, Recipe::Charcoal);
    }
    g.add_order(smelter, Recipe::SmeltOre);
    g.add_order(smelter, Recipe::SmeltOre);
    g.add_order(smelter, Recipe::MakeSteel);
    g.add_order(forge, Recipe::ForgePick);
    g.add_order(kitchen, Recipe::Cook);
    g.add_order(butcher, Recipe::Butcher);
    g.add_order(crafts, Recipe::StoneCraft);
    run(&mut g, 6 * DAY);

    let picks_iron = count(&g, ItemKind::Pick, Some(Mat::Iron)) + count(&g, ItemKind::Pick, Some(Mat::Steel));
    let steel = count(&g, ItemKind::Bar, Some(Mat::Steel)) + count(&g, ItemKind::Pick, Some(Mat::Steel));
    let meat = count(&g, ItemKind::Meat, None);
    let crafts_made = count(&g, ItemKind::Craft, None);
    let made_meal = g.log.iter().any(|l| l.text.contains("prepared meal"));
    eprintln!("iron picks {} steel {} meat {} crafts {} meal {}", picks_iron, steel, meat, crafts_made, made_meal);
    for l in g.log.iter().filter(|l| l.text.contains("needs")).take(8) {
        eprintln!("  {}", l.text);
    }
    assert!(picks_iron >= 1, "no iron pick forged");
    assert_eq!(steel, 1, "no steel");
    assert!(meat >= 1 || g.log.iter().any(|l| l.text.contains("meat")), "no butchering");
    assert!(crafts_made >= 1, "no crafts");
    assert!(made_meal, "no cooking");
    // All the workshops' orders are done and nothing is left claimed.
    for b in [furnace, smelter, forge, kitchen, butcher, crafts] {
        assert!(g.buildings[&b].orders.is_empty(), "{} has orders left", g.buildings[&b].kind.name());
    }
}

#[test]
fn goods_go_into_barrels_and_seeds_get_planted() {
    let mut g = Game::new(78);
    g.creatures.clear();
    let e = g.embark;
    let n = g.add_zone(e.offset(-4, 3, -1), e.offset(0, 6, 1), ZoneKind::Stockpile(StockFilter::Food));
    assert!(n > 4);
    let mut farm = 0;
    for (dx, dy) in [(-10, -10), (2, -10), (-10, 10), (2, 10)] {
        if farm < 8 {
            farm += g.add_zone(e.offset(dx, dy, -2), e.offset(dx + 4, dy + 3, 2), ZoneKind::Farm);
        }
    }
    assert!(farm > 0);
    let seeds_before = count(&g, ItemKind::Seed, None);
    run(&mut g, 4 * DAY);
    let boxed = g.items.values().filter(|it| it.container.is_some()).count();
    let seeds_after = count(&g, ItemKind::Seed, None);
    eprintln!("boxed {} seeds {} -> {}", boxed, seeds_before, seeds_after);
    assert!(boxed >= 5, "barrels not used");
    assert!(seeds_after < seeds_before, "no seeds planted");
    let full = g.items.values().filter(|it| it.kind.is_container()).map(|b| g.contents(b.id)).max().unwrap_or(0);
    assert!(full <= CONTAINER_CAP);
}



#[test]
fn hides_are_tanned_and_worked_into_leather_armor() {
    let mut g = Game::new(77);
    g.creatures.retain(|_, c| c.tame);
    let e = g.embark;
    for _ in 0..4 {
        g.spawn_item(ItemKind::Boulder, Mat::Granite, e);
    }
    g.spawn_item(ItemKind::Log, Mat::Wood, e);
    for _ in 0..2 {
        let c = g.spawn_item(ItemKind::Carcass, Mat::Flesh, e);
        g.items.get_mut(&c).unwrap().label = Some("deer".into());
    }
    for d in g.dwarves.iter_mut() {
        d.labors[Labor::Leatherworking.idx()] = true;
        d.labors[Labor::Butchery.idx()] = true;
    }
    let butcher = place_near(&mut g, BuildingKind::Butcher);
    let tannery = place_near(&mut g, BuildingKind::Tannery);
    let works = place_near(&mut g, BuildingKind::Leatherworks);
    run(&mut g, 3 * DAY);
    for b in [butcher, tannery, works] {
        assert!(g.buildings[&b].built, "{} not built", g.buildings[&b].kind.name());
    }
    g.add_order(butcher, Recipe::Butcher);
    g.add_order(butcher, Recipe::Butcher);
    run(&mut g, DAY);
    assert_eq!(count(&g, ItemKind::Hide, None), 2, "the butcher should leave hides, not leather");
    assert_eq!(count(&g, ItemKind::Leather, None), 0);
    g.add_order(tannery, Recipe::Tan);
    g.add_order(tannery, Recipe::Tan);
    run(&mut g, 2 * DAY);
    assert_eq!(count(&g, ItemKind::Leather, None), 2, "hides weren't tanned");
    assert!(g.items.values().any(|it| it.kind == ItemKind::Leather && it.label.as_deref() == Some("deer")));
    g.add_order(works, Recipe::LeatherArmor);
    run(&mut g, 2 * DAY);
    assert_eq!(count(&g, ItemKind::Breastplate, Some(Mat::Leather)), 1, "no leather armor");
    assert_eq!(count(&g, ItemKind::Leather, None), 0, "leather armor takes two pieces");
}

#[test]
fn thread_is_woven_into_cloth_and_sewn_into_clothes() {
    let mut g = Game::new(77);
    g.creatures.retain(|_, c| c.tame);
    let e = g.embark;
    for _ in 0..4 {
        g.spawn_item(ItemKind::Boulder, Mat::Granite, e);
    }
    for _ in 0..4 {
        g.spawn_item(ItemKind::Thread, Mat::Stoutcap, e);
    }
    let hide = g.spawn_item(ItemKind::Leather, Mat::Leather, e);
    g.items.get_mut(&hide).unwrap().label = Some("goat".into());
    for d in g.dwarves.iter_mut() {
        d.labors[Labor::Clothesmaking.idx()] = true;
        d.labors[Labor::Leatherworking.idx()] = true;
    }
    let loom = place_near(&mut g, BuildingKind::Loom);
    let clothier = place_near(&mut g, BuildingKind::Clothier);
    let works = place_near(&mut g, BuildingKind::Leatherworks);
    run(&mut g, 3 * DAY);
    for b in [loom, clothier, works] {
        assert!(g.buildings[&b].built, "{} not built", g.buildings[&b].kind.name());
    }
    g.add_order(loom, Recipe::Weave);
    g.add_order(loom, Recipe::Weave);
    g.add_order(works, Recipe::LeatherShoes);
    run(&mut g, 2 * DAY);
    // Everyone already wears shirts, trousers and shoes; count only new ones.
    let loose = |g: &Game, k: ItemKind, m: Mat| g.items.values().filter(|it| it.kind == k && it.mat == m && it.carried_by.is_none()).count();
    assert_eq!(count(&g, ItemKind::Cloth, Some(Mat::Stoutcap)), 2, "thread wasn't woven");
    assert_eq!(loose(&g, ItemKind::Shoes, Mat::Leather), 1, "no leather shoes");
    g.add_order(clothier, Recipe::ClothShirt);
    g.add_order(clothier, Recipe::ClothTrousers);
    run(&mut g, 2 * DAY);
    assert_eq!(loose(&g, ItemKind::Shirt, Mat::Stoutcap), 1);
    assert_eq!(loose(&g, ItemKind::Trousers, Mat::Stoutcap), 1);
}

#[test]
fn soap_from_ash_and_tallow_lets_doctors_clear_infection() {
    let mut g = Game::new(77);
    g.creatures.retain(|_, c| c.tame);
    let e = g.embark;
    for _ in 0..4 {
        g.spawn_item(ItemKind::Boulder, Mat::Granite, e);
    }
    g.spawn_item(ItemKind::Log, Mat::Wood, e);
    let c = g.spawn_item(ItemKind::Carcass, Mat::Flesh, e);
    g.items.get_mut(&c).unwrap().label = Some("goat".into());
    for d in g.dwarves.iter_mut() {
        d.labors[Labor::Soapmaking.idx()] = true;
        d.labors[Labor::Smelting.idx()] = true;
        d.labors[Labor::Butchery.idx()] = true;
        d.labors[Labor::Medicine.idx()] = false;
    }
    let furnace = place_near(&mut g, BuildingKind::WoodFurnace);
    let butcher = place_near(&mut g, BuildingKind::Butcher);
    let soaper = place_near(&mut g, BuildingKind::Soapmaker);
    run(&mut g, 3 * DAY);
    for b in [furnace, butcher, soaper] {
        assert!(g.buildings[&b].built, "{} not built", g.buildings[&b].kind.name());
    }
    g.add_order(furnace, Recipe::Ash);
    g.add_order(butcher, Recipe::Butcher);
    run(&mut g, DAY);
    assert_eq!(count(&g, ItemKind::Ash, None), 1, "no ash");
    assert_eq!(count(&g, ItemKind::Tallow, None), 1, "no tallow");
    g.add_order(soaper, Recipe::MakeSoap);
    run(&mut g, DAY);
    assert_eq!(count(&g, ItemKind::Soap, None), 2, "no soap");
    assert!(g.items.values().any(|it| it.kind == ItemKind::Soap && it.name() == "goat soap"));

    // A patient with a festering cut, and one doctor.
    let doc = g.dwarves[1].id;
    g.set_labor(doc, Labor::Medicine, true);
    let id = g.dwarves[0].id;
    {
        let d = &mut g.dwarves[0];
        d.body.wound(crate::body::Part::LeftArm, 4, true);
        d.body.bleeding = 0;
        d.body.infection = 300;
        d.task = Task::Rest { bed: None, stand: d.pos };
        d.asleep = true;
    }
    for _ in 0..DAY {
        g.step();
        let p = g.dwarves.iter().find(|d| d.id == id).unwrap();
        if !p.body.dirty {
            break;
        }
    }
    let p = g.dwarves.iter().find(|d| d.id == id).unwrap();
    assert!(!p.body.dirty, "the wound was never cleaned");
    assert_eq!(p.body.infection, 0, "soap should clear the infection");
    assert_eq!(count(&g, ItemKind::Soap, None), 1, "cleaning should use up a bar of soap");
}

#[test]
fn crops_grow_by_season_and_place() {
    let sun = Mat::Sunberry.crop().unwrap();
    assert!(sun.grows(true, 0) && !sun.grows(false, 0) && !sun.grows(true, 3));
    let grain = Mat::Hollowgrain.crop().unwrap();
    assert!(grain.grows(false, 2) && !grain.grows(true, 2) && !grain.grows(false, 0));
    assert!(Mat::Stoutcap.crop().unwrap().grows(true, 3), "stoutcaps grow anywhere, all year");
    let it = |m| Item { id: 0, kind: ItemKind::Plant, mat: m, pos: Pos::default(), carried_by: None, claimed_by: None, label: None, container: None, quality: 0, to_depot: false, dump: false, wear: 0, forbidden: false };
    assert!(it(Mat::Sunberry).is_food() && !it(Mat::Goldoat).is_food(), "grain isn't eaten raw");
    assert_eq!(it(Mat::Sunberry).name(), "sunberries");
}

#[test]
fn a_surface_farm_set_to_sunberries_grows_them_and_they_brew_into_wine() {
    let mut g = Game::new(77);
    g.creatures.retain(|_, c| c.tame);
    let e = g.embark;
    g.items.retain(|_, it| it.kind != ItemKind::Seed);
    for _ in 0..6 {
        g.spawn_item(ItemKind::Seed, Mat::Sunberry, e);
        g.spawn_item(ItemKind::Seed, Mat::Stoutcap, e);
    }
    let mut farm = 0;
    for (dx, dy) in [(-8, -8), (3, -8), (-8, 3), (3, 3)] {
        if g.add_zone(e.offset(dx, dy, -1), e.offset(dx + 3, dy + 3, 1), ZoneKind::Farm) > 0 {
            farm = *g.zones.keys().last().unwrap();
            break;
        }
    }
    assert!(farm > 0, "no farmable soil");
    assert!(g.world.outside(g.zones[&farm].tiles[0]));
    // Cycle the farm to sunberries.
    while g.zones[&farm].crop != Some(Mat::Sunberry) {
        g.cycle_farm_crop(farm).unwrap();
    }
    for d in g.dwarves.iter_mut() {
        d.labors[Labor::Farming.idx()] = true;
    }
    run(&mut g, DAY);
    let tiles = g.zones[&farm].tiles.clone();
    let planted: Vec<Option<Mat>> = tiles.iter().filter(|p| g.world.t(**p).farm > 0).map(|p| g.world.t(*p).crop).collect();
    assert!(!planted.is_empty(), "nothing planted");
    assert!(planted.iter().all(|&c| c == Some(Mat::Sunberry)), "planted {:?}", planted);
    for p in &tiles {
        if g.world.t(*p).farm > 0 {
            g.world.t_mut(*p).farm = RIPE;
        }
    }
    run(&mut g, DAY);
    assert!(count(&g, ItemKind::Plant, Some(Mat::Sunberry)) > 0, "no sunberries harvested");
    let still = place_near(&mut g, BuildingKind::Still);
    g.spawn_item(ItemKind::Boulder, Mat::Granite, e);
    run(&mut g, 2 * DAY);
    assert!(g.buildings[&still].built);
    g.items.retain(|_, it| !(it.kind == ItemKind::Plant && it.mat != Mat::Sunberry));
    g.add_order(still, Recipe::Brew);
    run(&mut g, DAY);
    assert!(g.items.values().any(|it| it.kind == ItemKind::Drink && it.name() == "barrel of sunberry wine"), "no wine");
}

#[test]
fn grain_is_milled_into_flour_and_baked_into_more_meals() {
    let mut g = Game::new(77);
    g.creatures.retain(|_, c| c.tame);
    let e = g.embark;
    for _ in 0..4 {
        g.spawn_item(ItemKind::Boulder, Mat::Granite, e);
    }
    // Only grain in the larder, so the cook can't pick anything else.
    g.items.retain(|_, it| !it.kind.is_food());
    g.spawn_item(ItemKind::Plant, Mat::Goldoat, e);
    for d in g.dwarves.iter_mut() {
        d.labors[Labor::Farming.idx()] = true;
        d.labors[Labor::Cooking.idx()] = true;
        d.hunger = 0;
    }
    let mill = place_near(&mut g, BuildingKind::Millstone);
    let kitchen = place_near(&mut g, BuildingKind::Kitchen);
    run(&mut g, 2 * DAY);
    assert!(g.buildings[&mill].built && g.buildings[&kitchen].built);
    g.add_order(mill, Recipe::Mill);
    run(&mut g, DAY);
    assert_eq!(count(&g, ItemKind::Flour, Some(Mat::Goldoat)), 2, "no flour");
    assert!(g.items.values().any(|it| it.kind == ItemKind::Seed && it.mat == Mat::Goldoat), "milling should leave seed");
    assert!(!g.items.values().find(|it| it.kind == ItemKind::Flour).unwrap().is_food(), "flour isn't eaten raw");
    let meals = count(&g, ItemKind::Meal, None);
    g.add_order(kitchen, Recipe::Cook);
    run(&mut g, DAY);
    assert_eq!(count(&g, ItemKind::Meal, None), meals + 3, "flour should make three meals");
}
