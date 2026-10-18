//! Where new overseer behaviour plugs in. Each item keeps its logic in its
//! own file and adds one line to the hook it needs here: the daily round,
//! the alert check (every ten ticks, so keep it cheap), the brain's senses,
//! or the end of a siege.

use super::ambitions::{Ambition, Sense};
use super::overseer::Overseer;
use super::*;

impl Game {
    /// Daily, at the end of the overseer's round (`ov_daily`): one line per item.
    pub(super) fn ov_daily_hooks(&mut self, o: &mut Overseer) {
        // M1 (survival):
        self.ov_captives(o);
        self.ov_coax(o);
        self.ov_readiness(o);
        self.ov_water(o);
        self.ov_larder(o);
        self.ov_hospital(o);
        self.ov_lessons(o);
        self.ov_shelter(o);
        // The inside of the fort grows up (architect.rs, maturity.rs).
        self.ov_interior(o);
        // The outside and the grand works:
        self.ov_fortify(o);
        // Myth and magic: temples dedicated, a temple for a devout fort.
        self.ov_faith(o);
        self.ov_omens(o);
        self.ov_wizard(o);
        let _ = o;
    }

    /// Every ten ticks, at the end of the alert check (`ov_alert`): one line
    /// per item, and cheap.
    pub(super) fn ov_alert_hooks(&mut self, o: &mut Overseer) {
        // M1 (survival):
        self.ov_berserk(o);
        self.ov_cistern_gate(o);
        self.ov_rescue(o);
        self.ov_hunts(o);
        self.ov_towers_alert(o);
        self.ov_shelter_alert(o);
        // G01: war-game a coming siege in the arena.
        self.ov_wargame(o);
        // Myth and magic: a thief with a relic has the squad after it.
        self.ov_relics(o);
        // The thing in the night: hunt it or hide (haunt.rs).
        self.ov_haunt(o);
        let _ = o;
    }

    /// Senses beyond the brain's own: each is (sense, how strongly it fires
    /// 0..1, per-ambition weights). The brain links the sense to each listed
    /// ambition with weight × strength, so a weight of 1.3 is as strong a pull
    /// as `Besieged` has on Fortify. A sense not listed fires at 0.
    pub(super) fn extra_senses(&self, o: &Overseer) -> Vec<(Sense, f64, Vec<(Ambition, f64)>)> {
        let mut senses = Vec::new();
        // M1 (survival): Lesson, Prowlers, Unarmed, Wounded.
        senses.extend(self.prowler_sense(o));
        senses.push((Sense::Wounded, self.wounded_sense(), vec![(Ambition::Provision, 0.6), (Ambition::Venture, -1.5)]));
        senses.extend(self.unarmed_sense(o));
        senses.extend(self.lesson_sense(o));
        // Myth and magic: an ill omen readies the fort.
        senses.push((Sense::Omens, self.omen_sense(), vec![(Ambition::Fortify, 0.8), (Ambition::Muster, 0.8), (Ambition::Venture, -1.0)]));
        let _ = o;
        senses
    }

    /// A siege has just ended (`annals.sieges[i]` now has its outcome).
    pub(super) fn on_siege_end(&mut self, i: usize) {
        // M1 (survival):
        self.captives_after_siege(i);
        self.learn_from_siege(i);
        self.shelter_after_siege(i);
        // Stories: the fight gets a name.
        self.name_siege(i);
        // ...and a broken one a victory feast.
        self.victory_feast(i);
        self.judge_wargame(i);
    }
}
