//! Moving the hero (and companions) between `Game`s. Item ids belong to one
//! game (`next_id`), so they are never kept across visits: on the way in each
//! carried item gets a fresh id in the visit's `items`, and what the hero
//! holds and wears is pointed at the new ids; on the way out the items go
//! back into the pack, keeping the ids they had (which only the pack and
//! the hero's held and worn lists use until the next visit).
//!
//! While inside a site the `Hero` left behind is a shell: its `dwarf` and
//! `pack` are in the visit, its coins, race, home and the rest stay put.

use super::super::*;
use super::Hero;
use std::collections::HashMap;

/// Put the hero into `g` at `at`: their dwarf joins `g.dwarves` (the index
/// is returned) and their pack joins `g.items`, carried, under fresh ids.
pub fn carry_in(g: &mut Game, h: &mut Hero, at: Pos) -> usize {
    let mut d = std::mem::take(&mut h.dwarf);
    let pack = std::mem::take(&mut h.pack);
    let mut ids: HashMap<u32, u32> = HashMap::new();
    for mut it in pack {
        let id = g.new_id();
        ids.insert(it.id, id);
        it.id = id;
        it.pos = at;
        it.carried_by = Some(d.id);
        it.claimed_by = None;
        it.container = None;
        g.items.insert(id, it);
    }
    let map = |v: &mut Vec<u32>| *v = v.iter().filter_map(|i| ids.get(i).copied()).collect();
    d.held = d.held.and_then(|i| ids.get(&i).copied());
    map(&mut d.armor);
    map(&mut d.clothes);
    d.pos = at;
    d.path.clear();
    d.task = Task::Idle;
    d.carrying = None;
    d.working = false;
    d.attack_cd = 0;
    d.move_cd = 0;
    g.dwarves.push(d);
    g.dwarves.len() - 1
}

/// Take the hero (`g.dwarves[i]`) out of `g` and back into `h`: the dwarf
/// leaves `g.dwarves` (so a game kept or written back never holds them),
/// and everything they carry leaves `g.items` for the pack.
pub fn carry_out(g: &mut Game, i: usize, h: &mut Hero) {
    let d = g.dwarves.remove(i);
    let mine: Vec<u32> = g.items.values().filter(|it| it.carried_by == Some(d.id)).map(|it| it.id).collect();
    h.pack = mine.into_iter().filter_map(|id| g.items.remove(&id)).collect();
    h.dwarf = d;
}

#[cfg(test)]
mod tests {
    use super::super::tests::test_hero;
    use super::*;
    use crate::items::ItemKind;
    use crate::material::Mat;

    /// (kind, mat, label, held, worn) for each thing the hero has, sorted.
    fn kit(g: &Game, i: usize) -> Vec<(ItemKind, Mat, Option<String>, bool, bool)> {
        let d = &g.dwarves[i];
        let mut v: Vec<_> = g
            .items
            .values()
            .filter(|it| it.carried_by == Some(d.id))
            .map(|it| (it.kind, it.mat, it.label.clone(), d.held == Some(it.id), d.armor.contains(&it.id)))
            .collect();
        v.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));
        v
    }

    #[test]
    fn gear_crosses_between_games() {
        let mut a = Game::new(5);
        let mut b = Game::new(6);
        // Give b some ids in use, so the new ones differ.
        for _ in 0..50 {
            b.new_id();
        }
        let mut h = test_hero(3);
        let at = a.embark;
        let i = carry_in(&mut a, &mut h, at);
        assert!(h.pack.is_empty());
        let p = a.dwarves[i].pos;
        let axe = a.spawn_item(ItemKind::Axe, Mat::Iron, p);
        let helm = a.spawn_item(ItemKind::Helm, Mat::Copper, p);
        let gem = a.spawn_item(ItemKind::Craft, Mat::Gold, p);
        a.items.get_mut(&gem).unwrap().label = Some("the Ember of Dusk".into());
        a.items.get_mut(&gem).unwrap().quality = crate::items::ARTIFACT;
        for id in [axe, helm, gem] {
            a.items.get_mut(&id).unwrap().carried_by = Some(a.dwarves[i].id);
        }
        a.dwarves[i].held = Some(axe);
        a.dwarves[i].armor.push(helm);
        let before = kit(&a, i);
        assert_eq!(before.len(), 3);
        let n = a.items.len();
        carry_out(&mut a, i, &mut h);
        assert_eq!(a.items.len(), n - 3, "the gear left with the hero");
        assert!(a.dwarves.iter().all(|d| d.id != h.dwarf.id), "and so did the hero");
        assert_eq!(h.pack.len(), 3);
        let at = b.embark;
        let j = carry_in(&mut b, &mut h, at);
        assert_eq!(kit(&b, j), before, "same kinds, materials, labels, held and worn");
        let held = b.dwarves[j].held.unwrap();
        assert_ne!(held, axe, "a fresh id");
        assert_eq!(b.items[&held].kind, ItemKind::Axe);
        assert!(b.items.values().filter(|it| it.carried_by == Some(b.dwarves[j].id)).all(|it| it.pos == b.dwarves[j].pos));
    }
}
