//! Fame, the hero's saga, death and the world's memory (A10, A14). W0: the
//! fort's fall leaves its mark on the world (`mark_fallen`, called once from
//! `sagas_fall`), and the embark hook for a dead hero's remains is in place
//! (`place_remains`, empty until A14).

use super::super::*;
use super::Adventure;
use crate::items::ARTIFACT;
use crate::overworld::FortMark;

impl Game {
    /// The fort has fallen: the world remembers where, when, to what, and
    /// what it held. Draws no dice (it runs inside the fort's last tick).
    pub(crate) fn mark_fallen(&mut self, killer: Option<String>) {
        let artifacts: Vec<String> = self.items.values().filter(|it| it.quality >= ARTIFACT).map(|it| it.label.clone().unwrap_or_else(|| it.name())).collect();
        let mark = FortMark {
            name: self.fortress_name.clone(),
            site: self.site,
            civ: self.civ,
            founded: self.start_year,
            fell: Some(self.start_year + (self.tick / YEAR) as i32),
            killer,
            artifacts,
            save_file: crate::saves::fort_file(&self.fortress_name),
            peak_pop: self.dwarves.len(),
            journal: Vec::new(),
            occupant: None,
            robbed: Vec::new(),
        };
        let ow = &mut self.overworld;
        match ow.forts.iter().position(|f| f.save_file == mark.save_file && f.site == mark.site) {
            Some(i) => ow.forts[i] = mark,
            None => ow.forts.push(mark),
        }
    }

    /// A fort embarked where an adventurer fell finds their bones and gear
    /// (A14 fills it; called from `embark_prepared`).
    pub(crate) fn place_remains(&mut self) {}
}

impl Adventure {
    /// A deed: renown, a saga chapter, and at weight 3 a legend (A10).
    pub fn deed(&mut self, _text: &str, _weight: i32) {}

    /// The hero is dead: their figure, remains, saga and legend (A10).
    pub fn hero_died(&mut self, _cause: &str) {}

    /// The hero settles down in a friendly town (A10).
    pub fn retire(&mut self) {}
}
