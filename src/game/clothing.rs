//! Clothes: every dwarf wears a shirt, trousers and shoes that slowly wear
//! out. Tattered clothes are an embarrassment; fresh ones come from stock.

use super::*;
use crate::items::{FALLS_APART, TATTERED, WORN};

impl Game {
    /// Dress a newcomer in stoutcap cloth and leather shoes, already somewhat
    /// worn (up to `max_wear`).
    pub(super) fn dress(&mut self, d: &mut Dwarf, max_wear: u16) {
        for (kind, mat) in [(ItemKind::Shirt, Mat::Stoutcap), (ItemKind::Trousers, Mat::Stoutcap), (ItemKind::Shoes, Mat::Leather)] {
            let wear = self.rng.range(0, max_wear as i32 + 1) as u16;
            let id = self.spawn_item(kind, mat, d.pos);
            let it = self.items.get_mut(&id).unwrap();
            it.carried_by = Some(d.id);
            it.wear = wear;
            d.clothes.push(id);
        }
    }

    /// Once a day: clothes wear, the worst fall apart, rags embarrass, and
    /// anyone with something worn or missing looks out for a replacement.
    pub(super) fn clothes_daily(&mut self) {
        let tick = self.tick;
        for i in 0..self.dwarves.len() {
            if !self.dwarves[i].alive {
                continue;
            }
            let mut d = std::mem::take(&mut self.dwarves[i]);
            let mut fell = Vec::new();
            let mut tattered = false;
            d.clothes.retain(|id| self.items.contains_key(id));
            for &id in &d.clothes {
                let it = self.items.get_mut(&id).unwrap();
                it.wear += 1;
                if it.wear >= FALLS_APART {
                    fell.push(id);
                } else if it.wear >= TATTERED {
                    tattered = true;
                }
            }
            for id in fell {
                d.clothes.retain(|&c| c != id);
                if let Some(it) = self.items.remove(&id) {
                    let what = Item { wear: 0, ..it }.name();
                    self.msg(&format!("{}'s {} have worn through and been thrown away.", d.name, what), (170, 170, 170));
                }
            }
            if tattered {
                d.think("is embarrassed to be wearing rags", 3, tick);
            }
            d.want_clothes = self.items.values().any(|it| self.would_change_into(&d, it));
            self.dwarves[i] = d;
        }
    }

    /// Whether `it` is a spare piece of clothing `d` would rather wear than
    /// what they have on (or haven't).
    fn would_change_into(&self, d: &Dwarf, it: &Item) -> bool {
        let Some(slot) = it.kind.clothing_slot() else { return false };
        if it.carried_by.is_some() || it.claimed_by.is_some() || it.wear >= WORN {
            return false;
        }
        let current = d.clothes.iter().filter_map(|id| self.items.get(id)).find(|c| c.kind.clothing_slot() == Some(slot));
        current.map_or(true, |c| c.wear >= WORN)
    }

    /// Fetch a fresh piece of clothing to change into.
    pub(super) fn find_clothes(&mut self, d: &mut Dwarf) -> bool {
        d.want_clothes = false;
        let Some((item, path)) = self.find_item(d.pos, |it| self.would_change_into(d, it)) else { return false };
        self.claim_item(item, d.id);
        d.task = Task::Equip { item };
        Self::set_path(d, path);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn worn(g: &Game, i: usize, kind: ItemKind) -> Option<&Item> {
        g.dwarves[i].clothes.iter().filter_map(|id| g.items.get(id)).find(|it| it.kind == kind)
    }

    #[test]
    fn the_expedition_arrives_dressed() {
        let g = Game::new(81);
        for (i, d) in g.dwarves.iter().enumerate() {
            assert_eq!(d.clothes.len(), 3, "{} isn't fully dressed", d.name);
            for k in [ItemKind::Shirt, ItemKind::Trousers, ItemKind::Shoes] {
                let it = worn(&g, i, k).expect("missing a piece");
                assert_eq!(it.carried_by, Some(d.id));
            }
        }
    }

    #[test]
    fn rags_embarrass_until_the_dwarf_changes_into_fresh_clothes() {
        let mut g = Game::new(81);
        g.creatures.retain(|_, c| c.tame);
        let old = worn(&g, 0, ItemKind::Shirt).unwrap().id;
        g.items.get_mut(&old).unwrap().wear = TATTERED;
        g.clothes_daily();
        let text = "is embarrassed to be wearing rags";
        assert!(g.dwarves[0].thoughts.iter().any(|t| t.text == text));
        assert!(!g.dwarves[0].want_clothes, "nothing in stock to change into yet");
        let e = g.embark;
        let fresh = g.spawn_item(ItemKind::Shirt, Mat::Stoutcap, e);
        g.clothes_daily();
        assert!(g.dwarves[0].want_clothes);
        let id = g.dwarves[0].id;
        for _ in 0..DAY {
            g.step();
            if g.dwarves.iter().any(|d| d.id == id && d.clothes.contains(&fresh)) {
                break;
            }
        }
        let d = g.dwarves.iter().find(|d| d.id == id).unwrap();
        assert!(d.clothes.contains(&fresh), "never changed shirts");
        assert!(!d.clothes.contains(&old));
        let rag = &g.items[&old];
        assert!(rag.carried_by.is_none() && rag.name().starts_with("tattered"), "old shirt: {}", rag.name());
    }

    #[test]
    fn clothes_fall_apart_in_the_end() {
        let mut g = Game::new(81);
        let shoes = worn(&g, 0, ItemKind::Shoes).unwrap().id;
        g.items.get_mut(&shoes).unwrap().wear = FALLS_APART - 1;
        g.clothes_daily();
        assert!(!g.items.contains_key(&shoes));
        assert_eq!(g.dwarves[0].clothes.len(), 2);
    }
}
