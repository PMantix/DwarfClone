//! Labor assignment: single toggles, whole columns, work details and specialists.

use super::Game;
use crate::dwarf::{Labor, LABORS};
use serde::{Deserialize, Serialize};

/// A named set of labors that can be handed to a dwarf in one go.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkDetail {
    pub name: String,
    pub labors: Vec<Labor>,
}

pub fn default_details() -> Vec<WorkDetail> {
    use Labor::*;
    let d = |name: &str, labors: &[Labor]| WorkDetail { name: name.into(), labors: labors.to_vec() };
    vec![
        d("Miner", &[Mining, Hauling]),
        d("Woodworker", &[Woodcutting, Carpentry, Hauling]),
        d("Stoneworker", &[Masonry, Crafting, Building, Hauling]),
        d("Farmer", &[Farming, Brewing, Cooking, Butchery, Soapmaking, Hauling]),
        d("Fisher", &[Fishing, Cooking, Hauling]),
        d("Leatherworker", &[Butchery, Leatherworking, Hauling]),
        d("Clothier", &[Crafting, Clothesmaking, Hauling]),
        d("Metalworker", &[Smelting, Smithing, Hauling]),
        d("Hauler", &[Hauling, Building]),
    ]
}

impl Game {
    /// Turn one labor on or off. Turning on a tool labor drops its rival
    /// (a dwarf carries a pick or an axe, not both); that labor is returned.
    pub fn set_labor(&mut self, dwarf_id: u32, labor: Labor, on: bool) -> Option<Labor> {
        let i = self.dwarves.iter().position(|d| d.id == dwarf_id)?;
        let mut dropped = None;
        let mut labors = self.dwarves[i].labors;
        labors[labor.idx()] = on;
        if on {
            if let Some(rival) = labor.exclusive_with() {
                if labors[rival.idx()] {
                    labors[rival.idx()] = false;
                    dropped = Some(rival);
                }
            }
        }
        self.set_labors(i, labors);
        dropped
    }

    pub fn toggle_labor(&mut self, dwarf_id: u32, labor: Labor) -> Option<Labor> {
        let on = self.dwarves.iter().find(|d| d.id == dwarf_id).map_or(false, |d| d.has(labor));
        self.set_labor(dwarf_id, labor, !on)
    }

    /// Everybody does this, or (if everybody already does) nobody does.
    /// Dwarves holding the rival tool labor are left out when switching on.
    /// Returns whether the labor was switched on.
    pub fn toggle_labor_all(&mut self, labor: Labor) -> bool {
        let rival = labor.exclusive_with();
        let eligible = |d: &crate::dwarf::Dwarf| d.alive && rival.map_or(true, |r| !d.has(r));
        let on = self.dwarves.iter().any(|d| eligible(d) && !d.has(labor));
        for i in 0..self.dwarves.len() {
            let d = &self.dwarves[i];
            if !d.alive || (on && !eligible(d)) {
                continue;
            }
            let mut labors = d.labors;
            labors[labor.idx()] = on;
            self.set_labors(i, labors);
        }
        on
    }

    /// Replace a dwarf's labors with a work detail's.
    pub fn apply_detail(&mut self, dwarf_id: u32, detail: usize) -> bool {
        let Some(wd) = self.work_details.get(detail) else { return false };
        let Some(i) = self.dwarves.iter().position(|d| d.id == dwarf_id) else { return false };
        let mut labors = [false; LABORS.len()];
        for l in &wd.labors {
            labors[l.idx()] = true;
        }
        self.set_labors(i, labors);
        true
    }

    /// Save a dwarf's current labors as a new work detail. Returns its name.
    pub fn save_detail(&mut self, dwarf_id: u32) -> Option<String> {
        let d = self.dwarves.iter().find(|d| d.id == dwarf_id)?;
        let labors: Vec<Labor> = LABORS.iter().copied().filter(|&l| d.has(l)).collect();
        if self.work_details.iter().any(|w| w.labors == labors) {
            return None;
        }
        let base = d.profession().to_string();
        let n = self.work_details.iter().filter(|w| w.name.starts_with(&base)).count();
        let name = if n == 0 { base } else { format!("{} {}", base, n + 1) };
        self.work_details.push(WorkDetail { name: name.clone(), labors });
        Some(name)
    }

    pub fn delete_detail(&mut self, detail: usize) -> Option<String> {
        (detail < self.work_details.len()).then(|| self.work_details.remove(detail).name)
    }

    pub fn toggle_specialist(&mut self, dwarf_id: u32) -> bool {
        let Some(d) = self.dwarves.iter_mut().find(|d| d.id == dwarf_id) else { return false };
        d.specialist = !d.specialist;
        d.specialist
    }

    /// Set the labor array of dwarf `i`, dropping their current job if it
    /// needed a labor they no longer have.
    fn set_labors(&mut self, i: usize, labors: [bool; LABORS.len()]) {
        let lost = LABORS.iter().any(|l| self.dwarves[i].labors[l.idx()] && !labors[l.idx()]);
        self.dwarves[i].labors = labors;
        if lost {
            let mut d = std::mem::take(&mut self.dwarves[i]);
            if !d.task.is_need() && d.mood.is_none() {
                self.release_task(&mut d);
            }
            self.dwarves[i] = d;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{ItemKind, DAY};

    #[test]
    fn column_toggle_and_tool_exclusivity() {
        let mut g = Game::new(5);
        let ids: Vec<u32> = g.dwarves.iter().map(|d| d.id).collect();

        // Everybody hauls already: a column toggle switches it off for all, then on again.
        assert!(!g.toggle_labor_all(Labor::Hauling));
        assert!(g.dwarves.iter().all(|d| !d.has(Labor::Hauling)));
        assert!(g.toggle_labor_all(Labor::Hauling));
        assert!(g.dwarves.iter().all(|d| d.has(Labor::Hauling)));

        // Woodcutting for everyone skips the miners rather than taking their picks away.
        let miners = g.dwarves.iter().filter(|d| d.has(Labor::Mining)).count();
        assert!(miners > 0);
        g.toggle_labor_all(Labor::Woodcutting);
        for d in &g.dwarves {
            assert!(!(d.has(Labor::Mining) && d.has(Labor::Woodcutting)), "{} has both tool labors", d.name);
        }
        assert_eq!(g.dwarves.iter().filter(|d| d.has(Labor::Mining)).count(), miners);

        // Turning mining on for a woodcutter drops woodcutting.
        let wc = g.dwarves.iter().find(|d| d.has(Labor::Woodcutting)).unwrap().id;
        assert_eq!(g.set_labor(wc, Labor::Mining, true), Some(Labor::Woodcutting));
        let d = g.dwarves.iter().find(|d| d.id == wc).unwrap();
        assert!(d.has(Labor::Mining) && !d.has(Labor::Woodcutting));

        // Work details replace the labor set; saving makes a new one.
        assert!(g.apply_detail(ids[0], 3)); // Farmer
        let d = &g.dwarves[0];
        assert!(d.has(Labor::Farming) && d.has(Labor::Cooking) && !d.has(Labor::Mining));
        g.set_labor(ids[0], Labor::Smithing, true);
        let n = g.work_details.len();
        assert!(g.save_detail(ids[0]).is_some());
        assert_eq!(g.work_details.len(), n + 1);
        assert!(g.save_detail(ids[0]).is_none(), "identical detail saved twice");
        assert!(g.delete_detail(n).is_some());
        assert_eq!(g.work_details.len(), n);
    }

    #[test]
    fn specialists_never_haul() {
        let mut g = Game::new(8);
        let e = g.embark;
        g.add_zone(e.offset(-6, 3, -1), e.offset(-1, 7, 1), crate::game::ZoneKind::Stockpile(crate::items::StockFilter::All));
        for i in 0..g.dwarves.len() {
            let id = g.dwarves[i].id;
            g.toggle_specialist(id);
        }
        // Items lying outside the stockpile; new ones (seeds, empty barrels) can appear anywhere.
        let loose: Vec<u32> = g.items.values().filter(|it| !g.stockpile_accepts(it.pos, it.kind, it.mat)).map(|it| it.id).collect();
        for _ in 0..DAY {
            g.step();
            assert!(
                g.dwarves.iter().all(|d| !matches!(d.task, crate::dwarf::Task::Haul { .. })),
                "a specialist took a hauling job"
            );
        }
        // Sanity check that there was something to haul.
        assert!(g.items.values().any(|it| it.kind == ItemKind::Drink && !g.stockpile_accepts(it.pos, it.kind, it.mat)));
        let moved = loose.iter().filter_map(|id| g.items.get(id)).filter(|it| it.carried_by.is_none() && g.stockpile_accepts(it.pos, it.kind, it.mat)).count();
        assert_eq!(moved, 0, "something was hauled to the stockpile");
    }
}
