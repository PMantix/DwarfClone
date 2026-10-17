//! Arms and kit: what a bolt of each metal does, and whether a soldier is
//! ready for a fight.

use super::*;

impl Game {
    /// What one bolt does before skill and armour: wooden bolts (or none
    /// known) 9, bone 9.5, copper 10, bronze 11, iron 12, steel 14.
    pub(super) fn bolt_damage(mat: Option<Mat>) -> f32 {
        match mat {
            Some(Mat::Steel) => 14.0,
            Some(Mat::Iron) => 12.0,
            Some(Mat::Bronze) => 11.0,
            Some(Mat::Copper) => 10.0,
            Some(Mat::Bone) | Some(Mat::Gold) => 9.5,
            _ => 9.0,
        }
    }

    /// A work order for `metal` forged crossbows or bolt bundles. Stock is
    /// counted by kind whatever it's made of, so the wooden ones on hand are
    /// added to the mark: otherwise a stack of wooden bolts would keep the
    /// forge idle.
    pub(super) fn metal_arms_order(&self, recipe: Recipe, metal: u32) -> (Recipe, u32) {
        let kind = recipe.product().0;
        let held: HashSet<u32> = self.dwarves.iter().filter(|d| d.alive).filter_map(|d| d.held).collect();
        let wooden = self.items.values().filter(|it| it.kind == kind && it.mat == Mat::Wood && !held.contains(&it.id)).count() as u32;
        (recipe, metal + wooden)
    }
}
