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
        let _ = o;
        senses
    }

    /// A siege has just ended (`annals.sieges[i]` now has its outcome).
    pub(super) fn on_siege_end(&mut self, i: usize) {
        // M1 (survival):
        self.captives_after_siege(i);
        self.learn_from_siege(i);
    }
}
