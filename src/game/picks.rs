//! Dig speed by pick and rock (O10, DIG-06): a steel pick bites granite
//! twice as fast as copper, and a miner trades up when a better pick lies in
//! the stores. The overseer forges picks once iron is to hand.

use super::*;

impl Game {
    /// The edge of the tool a dwarf holds, if it is of the given kind (a
    /// bare hand otherwise).
    pub(super) fn tool_edge(&self, d: &Dwarf, kind: ItemKind) -> u32 {
        d.held.and_then(|id| self.items.get(&id)).filter(|it| it.kind == kind).map_or(Mat::Air.edge(), |it| it.mat.edge())
    }

    /// Novice ticks for one dig, felling or channel: the rock's hardness
    /// against the pick's edge, a tree against the axe's.
    pub(super) fn dig_base(&self, d: &Dwarf, target: Pos, desig: Desig) -> u32 {
        if desig == Desig::Chop {
            // A copper axe fells a tree in 80.
            return (80 * Mat::Copper.edge() / self.tool_edge(d, ItemKind::Axe)).max(10);
        }
        self.world.t(target).mat.dig_ticks_with(self.tool_edge(d, ItemKind::Pick))
    }

    /// A miner holding a dull pick goes for a sharper one lying free in the
    /// stores. Some(true) when it set off to fetch it.
    pub(super) fn better_pick(&mut self, d: &mut Dwarf, chop: bool) -> Option<bool> {
        if chop {
            return None;
        }
        let held = d.held.and_then(|id| self.items.get(&id)).filter(|it| it.kind == ItemKind::Pick).map(|it| it.mat.edge())?;
        let free = |it: &Item| it.kind == ItemKind::Pick && it.mat.edge() > held && it.carried_by.is_none() && it.claimed_by.is_none();
        // A cheap look first: a path search only when one exists at all.
        if held >= Mat::Steel.edge() || !self.items.values().any(|it| free(it)) {
            return None;
        }
        let (item, path) = self.find_item(d.pos, |it| it.kind == ItemKind::Pick && it.mat.edge() > held)?;
        self.claim_item(item, d.id);
        d.task = Task::Equip { item };
        Self::set_path(d, path);
        Some(true)
    }

    /// The overseer's pick order (`ov_orders`, once a forge stands): while a
    /// miner digs with copper and no sharper pick waits in the stores, one
    /// more pick, but only with iron or steel bars to make it from.
    pub(super) fn pick_order(&self) -> (Recipe, u32) {
        let held: HashSet<u32> = self.dwarves.iter().filter(|d| d.alive).filter_map(|d| d.held).collect();
        let sharp = Mat::Iron.edge();
        let spare = self.items.values().filter(|it| it.kind == ItemKind::Pick && !held.contains(&it.id)).count() as u32;
        let spare_sharp = self.items.values().any(|it| it.kind == ItemKind::Pick && it.mat.edge() >= sharp && !held.contains(&it.id));
        let dull = self.dwarves.iter().any(|d| d.alive && d.has(Labor::Mining) && self.tool_edge(d, ItemKind::Pick) < sharp);
        let bars = self.items.values().any(|it| it.kind == ItemKind::Bar && matches!(it.mat, Mat::Iron | Mat::Steel) && it.free());
        (Recipe::ForgePick, spare + (dull && !spare_sharp && bars) as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_steel_pick_digs_granite_faster_than_copper() {
        let mut g = Game::new(5);
        let e = g.embark;
        let at = e.offset(0, 0, -3);
        g.world.t_mut(at).mat = Mat::Granite;
        let mut d = g.dwarves[0].clone();
        let mut with = |g: &mut Game, mat: Mat| {
            let id = g.spawn_item(ItemKind::Pick, mat, e);
            d.held = Some(id);
            g.dig_base(&d, at, Desig::Dig)
        };
        let copper = with(&mut g, Mat::Copper);
        let steel = with(&mut g, Mat::Steel);
        let iron = with(&mut g, Mat::Iron);
        assert_eq!(copper, 140, "a copper pick on granite takes as long as digging always did");
        assert!(steel * 2 <= copper + 1 && iron < copper && steel < iron, "{} {} {}", copper, iron, steel);
        // Soft rock is quicker whatever the pick.
        g.world.t_mut(at).mat = Mat::Sandstone;
        assert!(with(&mut g, Mat::Copper) < copper);
        g.world.t_mut(at).mat = Mat::Obsidian;
        assert!(with(&mut g, Mat::Copper) > copper);
    }

    #[test]
    fn a_miner_trades_a_copper_pick_for_steel() {
        let mut g = Game::new(5);
        let e = g.embark;
        let mut d = g.dwarves[0].clone();
        let copper = g.spawn_item(ItemKind::Pick, Mat::Copper, d.pos);
        g.items.get_mut(&copper).unwrap().carried_by = Some(d.id);
        d.held = Some(copper);
        assert_eq!(g.better_pick(&mut d, false), None, "nothing better to fetch");
        let steel = g.spawn_item(ItemKind::Pick, Mat::Steel, d.pos);
        assert_eq!(g.better_pick(&mut d, false), Some(true));
        assert_eq!(d.task, Task::Equip { item: steel });
        assert_eq!(g.better_pick(&mut d, true), None, "felling wants an axe, not a pick");
        let _ = e;
    }
}
